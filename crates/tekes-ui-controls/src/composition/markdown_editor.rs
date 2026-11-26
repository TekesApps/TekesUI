//! Editable Live-Preview Markdown projection adapter owning canonical LF source
//! and canonical undo/redo over one retained `TextFieldContent` / `TextSurface`.
use super::markdown_projection::{
    MarkdownInlineKind, MarkdownProjection, clamp_grapheme_utf16, normalize_markdown_source,
};
use super::rich_content::{
    CachedRichPaint, RichRenderer, ShapedBlockSpec, block_geometry_for_ranges, link_semantic_parts,
    order_document_carets, paint_code_line_numbers, paint_code_surfaces, paint_link_underlines,
    surface_link_rects,
    syntax::{self, CodeSpan},
};
use super::rich_document::utf16_to_byte;
use super::*;
use crate::{
    ExternalSelection, ExternalUndo, ExternalValueConflict, ExternalValueResult, ScrollPolicy,
    TextEditCommand, TextSurface, TextSurfacePresentation,
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

const MAX_UNDO: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownEditorVersion {
    identity: String,
    revision: u64,
    source_epoch: u64,
    binding: Vec<u8>,
}

impl MarkdownEditorVersion {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn source_epoch(&self) -> u64 {
        self.source_epoch
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownEditorError {
    InvalidIdentity,
    WrongDocument,
    StaleVersion,
    PartCapacity,
    Conflict(ExternalValueConflict),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceSnapshot {
    source: String,
    anchor: usize,
    caret: usize,
}

#[derive(Debug)]
pub struct MarkdownEditorContent {
    field: TextFieldContent,
    code_spans: Vec<CodeSpan>,
    paint_cache: Option<CachedRichPaint>,
    projection: MarkdownProjection,
    source: String,
    source_anchor: usize,
    source_caret: usize,
    undo: Vec<SourceSnapshot>,
    redo: Vec<SourceSnapshot>,
    identity: String,
    revision: u64,
    source_epoch: u64,
    last_acknowledged: (u64, String),
    links: Vec<RichLinkAction>,
    link_part_base: usize,
    next_link_part: usize,
}

impl MarkdownEditorContent {
    pub fn new(
        identity: impl Into<String>,
        revision: u64,
        source: impl Into<String>,
    ) -> Result<Self, MarkdownEditorError> {
        let identity = identity.into();
        if identity.trim().is_empty() || revision == 0 {
            return Err(MarkdownEditorError::InvalidIdentity);
        }
        let source = normalize_markdown_source(&source.into());
        let projection = MarkdownProjection::from_markdown(&source, None);
        let code_spans = build_editor_code_spans(&projection);

        let mut field = TextFieldContent::new("Markdown", projection.text());
        field.surface = TextSurface::new(projection.text(), true, false);
        field.surface.presentation = TextSurfacePresentation::Content;
        field.surface.set_role(proposal::TextRole::UiBody);
        field
            .surface
            .scroll
            .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Disabled);
        field.surface.scroll.set_scrollbar_visibility(false, false);
        field.show_metadata = false;
        field.clear_enabled = false;
        field.tab_policy = TextTabPolicy::Insert;
        field.viewport = TextViewport::Grow {
            min: 0.,
            max: f32::MAX,
        };

        let next_link_part = projection.links().len();
        Ok(Self {
            field,
            code_spans,
            paint_cache: None,
            projection,
            last_acknowledged: (revision, source.clone()),
            source,
            source_anchor: 0,
            source_caret: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            identity,
            revision,
            source_epoch: 0,
            links: Vec::new(),
            link_part_base: 0,
            next_link_part,
        })
    }

    pub fn with_viewport(mut self, viewport: TextViewport) -> Self {
        let scrollable = match viewport {
            TextViewport::Grow { max, .. } => max.is_finite() && max < f32::MAX,
            TextViewport::Fixed(_) | TextViewport::Fill | TextViewport::Form => true,
        };
        self.field.viewport = viewport;
        if scrollable {
            self.field
                .surface
                .scroll
                .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
            self.field
                .surface
                .scroll
                .set_scrollbar_visibility(false, true);
        } else {
            self.field
                .surface
                .scroll
                .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Disabled);
            self.field
                .surface
                .scroll
                .set_scrollbar_visibility(false, false);
        }
        self
    }

    pub fn with_tab_policy(mut self, policy: TextTabPolicy) -> Self {
        self.field.tab_policy = policy;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.field.field.label = label.into();
        self
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.field
            .set_availability(read_only, self.field.surface.input.disabled);
    }

    pub fn is_read_only(&self) -> bool {
        self.field.surface.input.read_only
    }

    pub fn is_composing(&self) -> bool {
        self.field.surface.input.marked.is_some()
    }

    pub fn is_projection_frozen(&self) -> bool {
        self.field.surface.input.marked.is_some() || self.field.editor_captured
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn display_text(&self) -> &str {
        self.field.text()
    }

    pub fn source_selection(&self) -> (usize, usize) {
        (self.source_anchor, self.source_caret)
    }

    pub fn select_source_range(&mut self, anchor: usize, caret: usize) {
        let clamped_a = clamp_grapheme_utf16(&self.source, anchor);
        let clamped_c = clamp_grapheme_utf16(&self.source, caret);
        self.source_anchor = clamped_a;
        self.source_caret = clamped_c;
        if !self.is_projection_frozen() {
            let expanded = self.focused_expansion();
            self.apply_projection_refresh(expanded, false);
        } else if let Some(disp) = self
            .projection
            .source_to_display(clamped_a.min(clamped_c)..clamped_a.max(clamped_c))
        {
            let (disp_a, disp_c) = if clamped_a <= clamped_c {
                (disp.start, disp.end)
            } else {
                (disp.end, disp.start)
            };
            self.field
                .surface
                .input
                .sync_projection(self.projection.text(), disp_a, disp_c);
        }
    }

    pub fn projection(&self) -> &MarkdownProjection {
        &self.projection
    }

    pub fn version(&self) -> MarkdownEditorVersion {
        MarkdownEditorVersion {
            identity: self.identity.clone(),
            revision: self.revision,
            source_epoch: self.source_epoch,
            binding: ContentTextInput::local_binding(&self.field.surface),
        }
    }

    pub fn block_geometry(&self) -> Vec<RichBlockGeometry> {
        let ranges = self.block_ranges();
        block_geometry_for_ranges(&self.field.surface, &ranges)
    }

    pub fn has_link_actions(&self) -> bool {
        !self.links.is_empty()
    }

    pub fn drain_link_actions(&mut self) -> Vec<RichLinkAction> {
        std::mem::take(&mut self.links)
    }

    /// Preflight check for controlled or streaming external source updates.
    /// Does not mutate state.
    pub fn check_external_source(
        &self,
        expected: &MarkdownEditorVersion,
        payload: &str,
        new_revision: u64,
        selection: ExternalSelection,
        _undo: ExternalUndo,
    ) -> Result<ExternalValueResult, MarkdownEditorError> {
        self.prepare_external_source(expected, payload, new_revision, selection)
            .map(|prepared| match prepared {
                Ok(Some(update)) => ExternalValueResult::Applied {
                    changed: update.changed,
                    document_changed: false,
                },
                Ok(None) => ExternalValueResult::Unchanged,
                Err(conflict) => ExternalValueResult::Conflict(conflict),
            })
    }

    /// Apply a controlled or streaming external Markdown source update at canonical
    /// LF coordinates while honoring local composition and unacknowledged edits.
    pub fn apply_external_source(
        &mut self,
        expected: &MarkdownEditorVersion,
        payload: &str,
        new_revision: u64,
        selection: ExternalSelection,
        undo: ExternalUndo,
    ) -> Result<ExternalValueResult, MarkdownEditorError> {
        let update =
            match self.prepare_external_source(expected, payload, new_revision, selection)? {
                Ok(Some(update)) => update,
                Ok(None) => return Ok(ExternalValueResult::Unchanged),
                Err(conflict) => return Ok(ExternalValueResult::Conflict(conflict)),
            };

        let next_projection = MarkdownProjection::from_markdown(
            &update.next_source,
            if self.is_projection_frozen() {
                None
            } else {
                self.field.has_focus().then_some(
                    update.next_anchor.min(update.next_caret)
                        ..update.next_anchor.max(update.next_caret),
                )
            },
        );
        let next_link_part = self
            .next_link_part
            .checked_add(next_projection.links().len())
            .ok_or(MarkdownEditorError::PartCapacity)?;

        match undo {
            ExternalUndo::Preserve => {}
            ExternalUndo::Transaction if update.changed => {
                self.push_undo(SourceSnapshot {
                    source: self.source.clone(),
                    anchor: self.source_anchor,
                    caret: self.source_caret,
                });
            }
            ExternalUndo::Transaction => {}
            ExternalUndo::Reset => {
                self.undo.clear();
                self.redo.clear();
            }
        }

        self.source = update.next_source;
        self.source_anchor = update.next_anchor;
        self.source_caret = update.next_caret;
        self.revision = new_revision;
        if update.changed {
            self.source_epoch += 1;
        }
        self.last_acknowledged = (new_revision, self.source.clone());

        if next_projection.links() != self.projection.links() {
            self.link_part_base = self.next_link_part;
            self.next_link_part = next_link_part;
        }
        self.install_projection(next_projection, false);

        Ok(ExternalValueResult::Applied {
            changed: update.changed,
            document_changed: false,
        })
    }

    pub(super) fn owner_field(&self) -> &TextFieldContent {
        &self.field
    }

    pub(super) fn owner_field_mut(&mut self) -> &mut TextFieldContent {
        &mut self.field
    }

    fn prepare_external_source(
        &self,
        expected: &MarkdownEditorVersion,
        payload: &str,
        new_revision: u64,
        selection: ExternalSelection,
    ) -> Result<Result<Option<PreparedExternalSource>, ExternalValueConflict>, MarkdownEditorError>
    {
        if expected.identity != self.identity {
            return Err(MarkdownEditorError::WrongDocument);
        }
        let next_source = normalize_markdown_source(payload);
        if new_revision == self.revision && next_source == self.last_acknowledged.1 {
            return Ok(Ok(None));
        }
        if new_revision <= self.revision || expected.revision != self.revision {
            return Err(MarkdownEditorError::StaleVersion);
        }
        if self.is_composing() {
            return Ok(Err(ExternalValueConflict::Composition));
        }
        if expected.source_epoch != self.source_epoch
            || expected.binding != ContentTextInput::local_binding(&self.field.surface)
            || self.source != self.last_acknowledged.1
        {
            return Ok(Err(ExternalValueConflict::StaleBaseline));
        }

        let next_u16_len = next_source.encode_utf16().count();
        let (next_anchor, next_caret) = match selection {
            ExternalSelection::Reset => (0, 0),
            ExternalSelection::Set(range) => {
                if range.start > range.end || range.end > next_u16_len {
                    return Ok(Err(ExternalValueConflict::Selection));
                }
                let a = clamp_grapheme_utf16(&next_source, range.start);
                let c = if range.is_empty() {
                    a
                } else {
                    ceil_grapheme_utf16(&next_source, range.end)
                };
                (a, c)
            }
            ExternalSelection::Preserve => (
                preserve_source_offset(&self.source, &next_source, self.source_anchor),
                preserve_source_offset(&self.source, &next_source, self.source_caret),
            ),
        };
        let changed = next_source != self.source;
        Ok(Ok(Some(PreparedExternalSource {
            next_source,
            next_anchor,
            next_caret,
            changed,
        })))
    }

    fn focused_expansion(&self) -> Option<Range<usize>> {
        self.field.has_focus().then_some(
            self.source_anchor.min(self.source_caret)..self.source_anchor.max(self.source_caret),
        )
    }

    fn push_undo(&mut self, snapshot: SourceSnapshot) {
        if self.undo.len() >= MAX_UNDO {
            self.undo.remove(0);
        }
        self.undo.push(snapshot);
        self.redo.clear();
    }

    fn apply_history_step(&mut self, redo: bool) -> bool {
        let target = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        let Some(snapshot) = target else {
            return false;
        };
        let current = SourceSnapshot {
            source: self.source.clone(),
            anchor: self.source_anchor,
            caret: self.source_caret,
        };
        if redo {
            if self.undo.len() >= MAX_UNDO {
                self.undo.remove(0);
            }
            self.undo.push(current);
        } else {
            if self.redo.len() >= MAX_UNDO {
                self.redo.remove(0);
            }
            self.redo.push(current);
        }
        self.source = snapshot.source;
        self.source_anchor = clamp_grapheme_utf16(&self.source, snapshot.anchor);
        self.source_caret = clamp_grapheme_utf16(&self.source, snapshot.caret);
        self.source_epoch += 1;
        let expanded = if self.is_projection_frozen() {
            None
        } else {
            self.focused_expansion()
        };
        self.apply_projection_refresh(expanded, true);
        true
    }

    fn finish_delegated_input(
        &mut self,
        was_frozen: bool,
        before_rev: (u64, u64),
        response: &mut ContentResponse,
        reveal_on_change: bool,
    ) {
        if self.is_composing() {
            return;
        }
        if self.field.surface.input.edit_revision() != before_rev {
            if self.reconcile_after_interaction(reveal_on_change) {
                response.changed = true;
            }
        } else if was_frozen && !self.is_projection_frozen() {
            let expanded = self.focused_expansion();
            if self.apply_projection_refresh(expanded, reveal_on_change) {
                response.changed = true;
            }
        }
    }

    fn reconcile_after_interaction(&mut self, reveal_on_change: bool) -> bool {
        if self.is_composing() {
            return false;
        }
        let disp_text = &self.field.surface.input.text;
        let disp_anchor = self.field.surface.input.anchor;
        let disp_caret = self.field.surface.input.caret;
        let (next_source, next_anchor, next_caret) =
            self.projection
                .apply_display_state(disp_text, disp_anchor, disp_caret);

        let text_changed = next_source != self.source;
        if text_changed {
            let prev = SourceSnapshot {
                source: self.source.clone(),
                anchor: self.source_anchor,
                caret: self.source_caret,
            };
            self.push_undo(prev);
            self.source = next_source;
            self.source_epoch += 1;
        }
        self.source_anchor = next_anchor;
        self.source_caret = next_caret;

        if self.is_projection_frozen() && !text_changed {
            return false;
        }

        let expanded = self.focused_expansion();
        self.apply_projection_refresh(expanded, reveal_on_change)
    }

    fn apply_projection_refresh(
        &mut self,
        expanded: Option<Range<usize>>,
        reveal_on_change: bool,
    ) -> bool {
        let next_proj = MarkdownProjection::from_markdown(&self.source, expanded);
        let disp_range =
            self.projection_source_to_display(&next_proj, self.source_anchor, self.source_caret);
        let needs_sync = next_proj != self.projection
            || self.field.surface.input.text != next_proj.text()
            || self.field.surface.input.anchor != disp_range.0
            || self.field.surface.input.caret != disp_range.1;

        if !needs_sync {
            return false;
        }

        if next_proj.links() != self.projection.links() {
            self.link_part_base = self.next_link_part;
            self.next_link_part = self.next_link_part.saturating_add(next_proj.links().len());
        }
        let blocks_or_text_changed = next_proj.text() != self.projection.text()
            || next_proj.blocks() != self.projection.blocks()
            || next_proj.inlines() != self.projection.inlines();

        self.projection = next_proj;
        self.code_spans = build_editor_code_spans(&self.projection);
        self.paint_cache = None;
        self.field.surface.input.sync_projection(
            self.projection.text(),
            disp_range.0,
            disp_range.1,
        );
        if blocks_or_text_changed {
            self.field.surface.invalidate_text_layout();
        }
        if reveal_on_change {
            self.field.surface.reveal();
        }
        true
    }

    fn install_projection(&mut self, next_proj: MarkdownProjection, reveal_on_change: bool) {
        let (disp_a, disp_c) =
            self.projection_source_to_display(&next_proj, self.source_anchor, self.source_caret);
        self.projection = next_proj;
        self.code_spans = build_editor_code_spans(&self.projection);
        self.paint_cache = None;
        self.field
            .surface
            .input
            .sync_projection(self.projection.text(), disp_a, disp_c);
        self.field.surface.invalidate_text_layout();
        if reveal_on_change {
            self.field.surface.reveal();
        }
    }

    fn projection_source_to_display(
        &self,
        proj: &MarkdownProjection,
        anchor: usize,
        caret: usize,
    ) -> (usize, usize) {
        if anchor == caret {
            let at = proj
                .source_to_display(anchor..anchor)
                .map_or(0, |r| r.start);
            return (at, at);
        }
        let min = anchor.min(caret);
        let max = anchor.max(caret);
        let mapped = proj.source_to_display(min..max).unwrap_or_else(|| {
            let a = proj.source_to_display(min..min).map_or(0, |r| r.start);
            let b = proj.source_to_display(max..max).map_or(a, |r| r.start);
            a.min(b)..a.max(b)
        });
        if anchor <= caret {
            (mapped.start, mapped.end)
        } else {
            (mapped.end, mapped.start)
        }
    }

    fn emit_link(&mut self, index: usize, source: ContentActionSource) -> bool {
        let Some(link) = self.projection.links().get(index) else {
            return false;
        };
        self.links.push(RichLinkAction {
            document: self.identity.clone(),
            revision: self.revision,
            target: link.target().to_owned(),
            display: link.display(),
            source_range: link.source(),
            source,
        });
        true
    }

    fn link_rects(&self, index: usize) -> Vec<Rect> {
        self.projection
            .links()
            .get(index)
            .map_or_else(Vec::new, |link| {
                surface_link_rects(&self.field.surface, &link.display())
            })
    }

    fn block_ranges(&self) -> Vec<(Range<usize>, RichBlockKind)> {
        self.projection
            .blocks()
            .iter()
            .map(|b| (b.display(), b.kind()))
            .collect()
    }

    fn shaped_specs(&self) -> Vec<ShapedBlockSpec> {
        let blocks = self.projection.blocks();
        if blocks.is_empty() {
            return Vec::new();
        }
        let mut specs = Vec::with_capacity(blocks.len() + 1);
        if blocks[0].display().start > 0 {
            specs.push(ShapedBlockSpec {
                display: 0..0,
                role: proposal::TextRole::UiBody,
                kind: RichBlockKind::Paragraph,
                gap_after: false,
            });
        }
        for block in blocks {
            specs.push(ShapedBlockSpec {
                display: block.display(),
                role: block.role(),
                kind: block.kind(),
                gap_after: true,
            });
        }
        specs
    }
}

struct PreparedExternalSource {
    next_source: String,
    next_anchor: usize,
    next_caret: usize,
    changed: bool,
}

fn build_editor_code_spans(projection: &MarkdownProjection) -> Vec<CodeSpan> {
    let mut spans = Vec::new();
    let display = projection.text();
    for block in projection.blocks() {
        if block.kind() != RichBlockKind::Code {
            continue;
        }
        for fence in block.fence_display() {
            if !fence.is_empty() {
                spans.push(CodeSpan {
                    display: fence.clone(),
                    token: "c-code-comment",
                });
            }
        }
        let body = block.body_display();
        if !body.is_empty()
            && let Some(lang) = block.language().filter(|l| !l.is_empty())
        {
            let start_b = utf16_to_byte(display, body.start);
            let end_b = utf16_to_byte(display, body.end);
            if let Ok(parsed) = syntax::parse_code_slice(lang, &display[start_b..end_b], body.start)
            {
                spans.extend(parsed);
            }
        }
    }
    for inline in projection.inlines() {
        if inline.kind() == MarkdownInlineKind::Code && !inline.display().is_empty() {
            spans.push(CodeSpan {
                display: inline.display(),
                token: "c-code-string",
            });
        }
    }
    spans.sort_by_key(|s| (s.display.start, s.display.end));
    let mut clean: Vec<CodeSpan> = Vec::with_capacity(spans.len());
    for mut span in spans {
        if let Some(prev) = clean.last()
            && span.display.start < prev.display.end
        {
            span.display.start = prev.display.end;
        }
        if span.display.start < span.display.end {
            clean.push(span);
        }
    }
    clean
}

fn ceil_grapheme_utf16(text: &str, target: usize) -> usize {
    let mut acc = 0usize;
    for grapheme in text.graphemes(true) {
        if acc >= target {
            return acc;
        }
        acc += grapheme.encode_utf16().count();
    }
    acc
}

fn preserve_source_offset(old_text: &str, new_text: &str, offset: usize) -> usize {
    let old: Vec<_> = old_text.graphemes(true).collect();
    let new: Vec<_> = new_text.graphemes(true).collect();
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let prefix_u16: usize = old[..prefix].iter().map(|g| g.encode_utf16().count()).sum();
    let old_u16_len = old_text.encode_utf16().count();
    let new_u16_len = new_text.encode_utf16().count();
    let old_end = old_u16_len
        - old[old.len() - suffix..]
            .iter()
            .map(|g| g.encode_utf16().count())
            .sum::<usize>();
    let new_end = new_u16_len
        - new[new.len() - suffix..]
            .iter()
            .map(|g| g.encode_utf16().count())
            .sum::<usize>();
    let clamped = offset.min(old_u16_len);
    let mapped = if clamped < prefix_u16 {
        clamped
    } else if clamped >= old_end {
        new_end.saturating_add(clamped - old_end)
    } else {
        new_end
    };
    clamp_grapheme_utf16(new_text, mapped)
}

impl Component for MarkdownEditorContent {
    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let expanded = self
            .projection
            .blocks()
            .iter()
            .filter(|b| b.is_expanded())
            .count();
        Some(vec![
            InspectionProperty::new("component", "MarkdownEditorContent"),
            InspectionProperty::new("identity", self.identity.clone()),
            InspectionProperty::new("revision", self.revision.to_string()),
            InspectionProperty::new("source_epoch", self.source_epoch.to_string()),
            InspectionProperty::new("read_only", self.is_read_only().to_string()),
            InspectionProperty::new("composing", self.is_composing().to_string()),
            InspectionProperty::new("projection_frozen", self.is_projection_frozen().to_string()),
            InspectionProperty::new("block_count", self.projection.blocks().len().to_string()),
            InspectionProperty::new("expanded_blocks", expanded.to_string()),
        ])
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        self.field.curve_subtree_capability()
    }
    fn has_stable_hooks(&self) -> bool {
        self.field.has_stable_hooks()
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        self.field.supports_prepared_lifecycle()
    }
    fn preserves_activation_focus(&self) -> bool {
        self.field.preserves_activation_focus()
    }
    fn requires_text_services(&self) -> bool {
        true
    }
    fn composing(&self) -> bool {
        self.is_composing()
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        Some(&self.field.surface)
    }
    fn text_input_snapshot(&self) -> Option<ContentTextInput> {
        let mut input = ContentTextInput::from_surface(&self.field.surface);
        order_document_carets(&mut input.carets);
        Some(input)
    }
    fn semantic_text_layout(&self) -> Option<ContentTextLayout> {
        let mut layout = ContentTextLayout::from_surface(&self.field.surface)?;
        order_document_carets(&mut layout.carets);
        order_document_carets(&mut layout.input.carets);
        Some(layout)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let specs = self.shaped_specs();
        self.field.measure(
            &mut RichRenderer::new(t, &specs, &[], &self.code_spans, None, None),
            a,
        )
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let specs = self.shaped_specs();
        self.field.arrange(
            &mut RichRenderer::new(t, &specs, &[], &self.code_spans, None, None),
            f,
            c,
        )
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let specs = self.shaped_specs();
        self.field.prepare_text_input(
            &mut RichRenderer::new(t, &specs, &[], &self.code_spans, None, None),
            f,
            c,
        );
        if let Some(paragraph) = self.field.surface.paragraph.as_ref() {
            if self
                .paint_cache
                .as_ref()
                .is_none_or(|cache| !cache.matches(paragraph))
            {
                let ranges = self.block_ranges();
                self.paint_cache = Some(CachedRichPaint::build_ranges(
                    paragraph,
                    &ranges,
                    &self.code_spans,
                ));
            }
        } else {
            self.paint_cache = None;
        }
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.prepare_text_input(t, f, c);
        paint_code_surfaces(s, t, theme, &self.block_geometry(), |index| {
            self.projection.blocks()[index].kind()
        });
        let block_ranges = self.block_ranges();
        let specs = self.shaped_specs();
        self.field.paint(
            s,
            &mut RichRenderer::new(
                t,
                &specs,
                &block_ranges,
                &self.code_spans,
                self.paint_cache.as_ref(),
                Some(theme.palette),
            ),
            f,
            c,
            theme,
        );
        let numbered: Vec<_> = self
            .projection
            .blocks()
            .iter()
            .filter(|b| b.kind() == RichBlockKind::Code)
            .map(|b| (b.body_display(), b.kind()))
            .collect();
        paint_code_line_numbers(s, t, &self.field.surface, c, theme, &numbered);
        let link_displays: Vec<_> = self
            .projection
            .links()
            .iter()
            .map(RichLink::display)
            .collect();
        paint_link_underlines(s, &self.field.surface, c, theme, link_displays.iter());
    }
    fn focusable(&self) -> bool {
        self.field.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let was_composing = self.is_composing();
        self.field.set_focus(focused, visible);
        if was_composing {
            let _ = self.reconcile_after_interaction(false);
        } else if !self.is_projection_frozen() {
            let expanded = self.focused_expansion();
            let _ = self.apply_projection_refresh(expanded, false);
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.field.window_active_changed(active);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.field.presentation_runnable_changed(runnable);
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        let was_frozen = self.is_projection_frozen();
        let before_rev = self.field.surface.input.edit_revision();
        self.field.lifecycle(event);
        let mut ignored = ContentResponse::default();
        self.finish_delegated_input(was_frozen, before_rev, &mut ignored, false);
    }
    fn semantics(&self) -> ContentSemantics {
        self.field.semantics()
    }
    fn semantic_frame(&self, f: Rect) -> Rect {
        self.field.semantic_frame(f)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        link_semantic_parts(
            &self.field.surface,
            self.projection.links(),
            self.link_part_base,
        )
    }
    fn semantic_part_preserves_activation_focus(&self, _: usize) -> bool {
        true
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if let Some(part) = part {
            let Some(index) = part
                .checked_sub(self.link_part_base)
                .filter(|index| *index < self.projection.links().len())
            else {
                return ContentResponse::default();
            };
            if matches!(action, AccessibilityAction::Press) && !self.link_rects(index).is_empty() {
                return ContentResponse::consumed(
                    self.emit_link(index, ContentActionSource::Accessibility),
                );
            }
            return ContentResponse::default();
        }
        let was_frozen = self.is_projection_frozen();
        let before_rev = self.field.surface.input.edit_revision();
        let mut r = self.field.accessibility_action(None, action);
        let reveal = r.changed;
        self.finish_delegated_input(was_frozen, before_rev, &mut r, reveal);
        r
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if key.down
            && !key.repeat
            && key.key == Some(ControlKey::Enter)
            && key.modifiers.command
            && !self.is_composing()
        {
            let caret = self.field.surface.input.caret;
            if let Some(index) = self
                .projection
                .links()
                .iter()
                .position(|link| link.display().contains(&caret))
            {
                return ContentResponse::consumed(
                    self.emit_link(index, ContentActionSource::Keyboard),
                );
            }
        }
        let was_frozen = self.is_projection_frozen();
        let before_rev = self.field.surface.input.edit_revision();
        let mut r = self.field.key_event(key);
        let reveal = r.changed;
        self.finish_delegated_input(was_frozen, before_rev, &mut r, reveal);
        r
    }
    fn pointer_event(&mut self, p: &PointerEvent) -> ContentResponse {
        if p.is_primary()
            && p.phase == PointerPhase::Down
            && p.modifiers.command
            && let Some(index) =
                self.projection
                    .links()
                    .iter()
                    .enumerate()
                    .find_map(|(index, _)| {
                        self.link_rects(index)
                            .iter()
                            .any(|r| r.contains(p.position))
                            .then_some(index)
                    })
        {
            return ContentResponse::consumed(self.emit_link(index, ContentActionSource::Pointer));
        }
        let was_frozen = self.is_projection_frozen();
        let before_rev = self.field.surface.input.edit_revision();
        let mut r = self.field.pointer_event(p);
        let reveal = r.changed;
        self.finish_delegated_input(was_frozen, before_rev, &mut r, reveal);
        r
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::Pointer(pointer) => return self.pointer_event(pointer),
            ContentEvent::PointerDown(position)
            | ContentEvent::PointerUp(position)
            | ContentEvent::PointerMove(position) => {
                return self.pointer_event(&PointerEvent {
                    position: *position,
                    phase: match event {
                        ContentEvent::PointerDown(_) => PointerPhase::Down,
                        ContentEvent::PointerUp(_) => PointerPhase::Up,
                        _ => PointerPhase::Move,
                    },
                    button: Some(PointerButton::Primary),
                    modifiers: InputModifiers::default(),
                    click_count: 1,
                });
            }
            ContentEvent::TextCommand(TextEditCommand::SelectAll) => {
                if self.field.surface.input.disabled || self.is_composing() {
                    return ContentResponse::consumed(false);
                }
                let before = (
                    self.source_anchor,
                    self.source_caret,
                    self.field.surface.input.selection(),
                );
                let end = self.source.encode_utf16().count();
                self.select_source_range(0, end);
                let changed = (
                    self.source_anchor,
                    self.source_caret,
                    self.field.surface.input.selection(),
                ) != before;
                if changed {
                    self.field.surface.reveal();
                }
                return ContentResponse::consumed(changed);
            }
            ContentEvent::TextCommand(TextEditCommand::Undo | TextEditCommand::Redo) => {
                if self.field.surface.input.read_only || self.field.surface.input.disabled {
                    return ContentResponse::consumed(false);
                }
                if self.is_composing() {
                    let mut r = self.field.event(event);
                    if self.reconcile_after_interaction(true) {
                        r.changed = true;
                    }
                    return r;
                }
                let is_redo = matches!(event, ContentEvent::TextCommand(TextEditCommand::Redo));
                return ContentResponse::consumed(self.apply_history_step(is_redo));
            }
            ContentEvent::Scroll { .. } => return self.field.event(event),
            _ => {}
        }
        let was_frozen = self.is_projection_frozen();
        let before_rev = self.field.surface.input.edit_revision();
        let mut r = self.field.event(event);
        let reveal = r.changed;
        self.finish_delegated_input(was_frozen, before_rev, &mut r, reveal);
        r
    }
}
