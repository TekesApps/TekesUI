//! One readonly text owner. Block typography is composed from the original host
//! shaper into the one Paragraph retained by TextSurface; no second hit engine.
use super::*;
use crate::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueConflict, ExternalValueResult,
    TextSurface, TextSurfaceHost, TextSurfaceTextRenderer,
};
use std::{
    ops::Range,
    rc::{Rc, Weak},
};
use tekes_ui_text::Paragraph;

pub(super) mod syntax;
use syntax::CodeSpan;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RichInk {
    Default,
    DiffAdded,
    DiffRemoved,
    CodeToken(&'static str),
}

impl RichInk {
    const COUNT: usize = 7;

    const ALL: [Self; Self::COUNT] = [
        Self::Default,
        Self::DiffAdded,
        Self::DiffRemoved,
        Self::CodeToken("c-code-keyword"),
        Self::CodeToken("c-code-string"),
        Self::CodeToken("c-code-number"),
        Self::CodeToken("c-code-comment"),
    ];

    fn from_diff(kind: RichBlockKind) -> Self {
        match kind {
            RichBlockKind::Diff {
                kind: crate::extended::DiffKind::Added,
            } => Self::DiffAdded,
            RichBlockKind::Diff {
                kind: crate::extended::DiffKind::Removed,
            } => Self::DiffRemoved,
            _ => Self::Default,
        }
    }

    fn slot_index(self) -> usize {
        match self {
            Self::Default => 0,
            Self::DiffAdded => 1,
            Self::DiffRemoved => 2,
            Self::CodeToken("c-code-keyword") => 3,
            Self::CodeToken("c-code-string") => 4,
            Self::CodeToken("c-code-number") => 5,
            Self::CodeToken("c-code-comment") => 6,
            Self::CodeToken(_) => 0,
        }
    }

    fn resolve(self, palette: Palette, default_ink: Rgba) -> Rgba {
        match self {
            Self::Default => default_ink,
            Self::DiffAdded => palette.color("status-succeeded").unwrap(),
            Self::DiffRemoved => palette.color("status-failed").unwrap(),
            Self::CodeToken(key) => palette.color(key).expect("shared code token"),
        }
    }
}

#[derive(Debug)]
pub(super) struct CachedRichPaint {
    source: Weak<Paragraph>,
    plain: bool,
    batches: Vec<(RichInk, Paragraph)>,
}

impl CachedRichPaint {
    pub(super) fn matches(&self, paragraph: &Paragraph) -> bool {
        self.source.strong_count() > 0 && std::ptr::eq(self.source.as_ptr(), paragraph)
    }

    pub(super) fn build_ranges(
        paragraph: &Rc<Paragraph>,
        blocks: &[(Range<usize>, RichBlockKind)],
        code_spans: &[CodeSpan],
    ) -> Self {
        Self::build_ranges_with_weak(Rc::downgrade(paragraph), paragraph, blocks, code_spans)
    }

    pub(super) fn build_ranges_with_weak(
        source: Weak<Paragraph>,
        paragraph: &Paragraph,
        blocks: &[(Range<usize>, RichBlockKind)],
        code_spans: &[CodeSpan],
    ) -> Self {
        let has_diff_ink = blocks.iter().any(|(_, kind)| {
            matches!(
                kind,
                RichBlockKind::Diff {
                    kind: crate::extended::DiffKind::Added | crate::extended::DiffKind::Removed
                }
            )
        });
        if code_spans.is_empty() && !has_diff_ink {
            return Self {
                source,
                plain: true,
                batches: Vec::new(),
            };
        }

        let mut buckets: [Vec<tekes_ui_text::Line>; RichInk::COUNT] = Default::default();
        let mut block_idx = 0usize;
        let mut span_idx = 0usize;
        let mut cached_block_idx = usize::MAX;
        let mut block_span_range = 0..0usize;

        for line in &paragraph.lines {
            while block_idx < blocks.len() && blocks[block_idx].0.end <= line.utf16.start {
                block_idx += 1;
            }
            let Some((block_display, block_kind)) = blocks.get(block_idx) else {
                buckets[RichInk::Default.slot_index()].push(line.clone());
                continue;
            };
            if !(line.utf16.start < block_display.end && line.utf16.end > block_display.start) {
                buckets[RichInk::Default.slot_index()].push(line.clone());
                continue;
            }
            if cached_block_idx != block_idx {
                cached_block_idx = block_idx;
                while span_idx < code_spans.len()
                    && code_spans[span_idx].display.end <= block_display.start
                {
                    span_idx += 1;
                }
                let mut end = span_idx;
                while end < code_spans.len() && code_spans[end].display.start < block_display.end {
                    end += 1;
                }
                block_span_range = span_idx..end;
                span_idx = end;
            }
            let base_ink = RichInk::from_diff(*block_kind);
            let block_spans = &code_spans[block_span_range.clone()];
            if block_spans.is_empty() {
                buckets[base_ink.slot_index()].push(line.clone());
                continue;
            }

            let mut line_runs: [Vec<tekes_ui_text::ShapedRun>; 5] = Default::default();
            for run in &line.runs {
                let mut split_glyphs: [Vec<tekes_ui_text::ShapedGlyph>; 5] = Default::default();
                for glyph in &run.glyphs {
                    let cluster_start = glyph.cluster_utf16.start;
                    let found =
                        block_spans.partition_point(|span| span.display.end <= cluster_start);
                    let token_slot = block_spans
                        .get(found)
                        .filter(|span| span.display.contains(&cluster_start))
                        .map_or(0, |span| match span.token {
                            "c-code-keyword" => 1,
                            "c-code-string" => 2,
                            "c-code-number" => 3,
                            "c-code-comment" => 4,
                            _ => 0,
                        });
                    split_glyphs[token_slot].push(glyph.clone());
                }
                for (slot, glyphs) in split_glyphs.into_iter().enumerate() {
                    if !glyphs.is_empty() {
                        line_runs[slot].push(tekes_ui_text::ShapedRun {
                            font: run.font,
                            utf16: run.utf16.clone(),
                            glyphs,
                        });
                    }
                }
            }
            for (slot, runs) in line_runs.into_iter().enumerate() {
                if runs.is_empty() {
                    continue;
                }
                let target_ink = match slot {
                    0 => base_ink,
                    1 => RichInk::CodeToken("c-code-keyword"),
                    2 => RichInk::CodeToken("c-code-string"),
                    3 => RichInk::CodeToken("c-code-number"),
                    4 => RichInk::CodeToken("c-code-comment"),
                    _ => unreachable!(),
                };
                buckets[target_ink.slot_index()].push(tekes_ui_text::Line {
                    index: line.index,
                    utf16: line.utf16.clone(),
                    bytes: line.bytes.clone(),
                    ascent: line.ascent,
                    descent: line.descent,
                    leading: line.leading,
                    height: line.height,
                    top: line.top,
                    baseline_y: line.baseline_y,
                    width: line.width,
                    width_trimmed: line.width_trimmed,
                    runs,
                    carets: line.carets.clone(),
                    graphemes: line.graphemes.clone(),
                    has_newline: line.has_newline,
                });
            }
        }

        let batches = RichInk::ALL
            .into_iter()
            .zip(buckets)
            .filter(|(_, lines)| !lines.is_empty())
            .map(|(ink, lines)| {
                (
                    ink,
                    Paragraph {
                        lines,
                        height: paragraph.height,
                        width: paragraph.width,
                        default_line_height: paragraph.default_line_height,
                        fonts: paragraph.fonts.clone(),
                        utf16_len: paragraph.utf16_len,
                    },
                )
            })
            .collect();

        Self {
            source,
            plain: false,
            batches,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichContentVersion {
    identity: String,
    revision: u64,
    binding: Vec<u8>,
}
impl RichContentVersion {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RichContentError {
    InvalidIdentity,
    WrongDocument,
    StaleVersion,
    PartCapacity,
    /// A recognized code language could not be parsed. No update was applied.
    Syntax(String),
    Input(ExternalValueConflict),
}
#[derive(Clone, Debug, PartialEq)]
pub struct RichBlockGeometry {
    pub index: usize,
    pub display: Range<usize>,
    pub frame: Rect,
    pub visible: Option<Rect>,
}
/// An owned callback value from the original document, never a request to open
/// a URL or access the system. Drain once after dispatch; the host decides policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichLinkAction {
    pub document: String,
    pub revision: u64,
    pub target: String,
    pub display: Range<usize>,
    pub source_range: Range<usize>,
    pub source: ContentActionSource,
}

#[derive(Debug)]
pub struct RichContent {
    field: TextFieldContent,
    code_spans: Vec<CodeSpan>,
    paint_cache: Option<CachedRichPaint>,
    document: RichDocument,
    identity: String,
    revision: u64,
    links: Vec<RichLinkAction>,
    link_part_base: usize,
    next_link_part: usize,
}
impl RichContent {
    pub fn new(
        identity: impl Into<String>,
        revision: u64,
        document: RichDocument,
    ) -> Result<Self, RichContentError> {
        let identity = identity.into();
        if identity.trim().is_empty() || revision == 0 {
            return Err(RichContentError::InvalidIdentity);
        }
        let code_spans = syntax::parse(&document).map_err(RichContentError::Syntax)?;
        let mut field = TextFieldContent::read_only_content(
            "Document",
            document.text(),
            proposal::TextRole::UiBody,
        );
        let value = DraftRevision {
            identity: identity.clone(),
            version: revision,
            payload: document.text().into(),
        };
        let baseline = field.surface.input.external_baseline();
        if let ExternalValueResult::Conflict(error) = field.surface.apply_external_value(
            &baseline,
            &value,
            ExternalSelection::Preserve,
            ExternalUndo::Reset,
        ) {
            return Err(RichContentError::Input(error));
        }
        let next_link_part = document.links().len();
        Ok(Self {
            field,
            code_spans,
            paint_cache: None,
            document,
            identity,
            revision,
            links: Vec::new(),
            link_part_base: 0,
            next_link_part,
        })
    }
    pub fn document(&self) -> &RichDocument {
        &self.document
    }
    pub fn version(&self) -> RichContentVersion {
        RichContentVersion {
            identity: self.identity.clone(),
            revision: self.revision,
            binding: ContentTextInput::local_binding(&self.field.surface),
        }
    }
    /// Synchronous expected-version update. Original grapheme-prefix/suffix
    /// mapping preserves directed selection; removed text collapses to the end
    /// of its replacement. A document switch requires a new component.
    pub fn update(
        &mut self,
        expected: &RichContentVersion,
        revision: u64,
        document: RichDocument,
    ) -> Result<RichContentVersion, RichContentError> {
        if expected.identity != self.identity {
            return Err(RichContentError::WrongDocument);
        }
        if *expected != self.version() || revision <= self.revision {
            return Err(RichContentError::StaleVersion);
        }
        let code_spans = syntax::parse(&document).map_err(RichContentError::Syntax)?;
        let next_link_part = self
            .next_link_part
            .checked_add(document.links().len())
            .ok_or(RichContentError::PartCapacity)?;
        let baseline = self.field.surface.input.external_baseline();
        let value = DraftRevision {
            identity: self.identity.clone(),
            version: revision,
            payload: document.text().into(),
        };
        if let ExternalValueResult::Conflict(error) = self.field.surface.apply_external_value(
            &baseline,
            &value,
            ExternalSelection::Preserve,
            ExternalUndo::Reset,
        ) {
            return Err(RichContentError::Input(error));
        }
        self.field.surface.invalidate_text_layout();
        self.paint_cache = None;
        self.document = document;
        self.code_spans = code_spans;
        self.link_part_base = self.next_link_part;
        self.next_link_part = next_link_part;
        self.revision = revision;
        Ok(self.version())
    }
    pub fn has_link_actions(&self) -> bool {
        !self.links.is_empty()
    }
    pub fn drain_link_actions(&mut self) -> Vec<RichLinkAction> {
        std::mem::take(&mut self.links)
    }
    fn emit_link(&mut self, index: usize, source: ContentActionSource) -> bool {
        let Some(link) = self.document.links().get(index) else {
            return false;
        };
        self.links.push(RichLinkAction {
            document: self.identity.clone(),
            revision: self.revision,
            target: link.target.clone(),
            display: link.display.clone(),
            source_range: link.source.clone(),
            source,
        });
        true
    }
    /// Original paragraph geometry, in host coordinates. Use this with the
    /// existing Content reading transaction; no consumer pixel compensation.
    pub fn block_geometry(&self) -> Vec<RichBlockGeometry> {
        let ranges: Vec<_> = self
            .document
            .blocks()
            .iter()
            .map(|b| (b.display.clone(), b.kind))
            .collect();
        block_geometry_for_ranges(&self.field.surface, &ranges)
    }
    fn link_rects(&self, index: usize) -> Vec<Rect> {
        self.document
            .links()
            .get(index)
            .map_or_else(Vec::new, |link| {
                surface_link_rects(&self.field.surface, &link.display)
            })
    }
}

impl RichContent {
    pub(super) fn owner_field(&self) -> &TextFieldContent {
        &self.field
    }
    pub(super) fn owner_field_mut(&mut self) -> &mut TextFieldContent {
        &mut self.field
    }
}
impl Component for RichContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        self.field.curve_subtree_capability()
    }
    fn has_stable_hooks(&self) -> bool {
        self.field.has_stable_hooks()
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        self.field.supports_prepared_lifecycle()
    }
    fn requires_text_services(&self) -> bool {
        true
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
        let specs = document_specs(&self.document);
        self.field.measure(
            &mut RichRenderer::new(t, &specs, &[], &self.code_spans, None, None),
            a,
        )
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let specs = document_specs(&self.document);
        self.field.arrange(
            &mut RichRenderer::new(t, &specs, &[], &self.code_spans, None, None),
            f,
            c,
        )
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let specs = document_specs(&self.document);
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
                let ranges: Vec<_> = self
                    .document
                    .blocks()
                    .iter()
                    .map(|b| (b.display.clone(), b.kind))
                    .collect();
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
        // Prepare the same retained layout before painting surfaces. TextSurface
        // paints selection before glyphs, so backgrounds must precede its paint.
        self.prepare_text_input(t, f, c);
        paint_code_surfaces(s, t, theme, &self.block_geometry(), |index| {
            self.document.blocks()[index].kind
        });
        let block_ranges: Vec<_> = self
            .document
            .blocks()
            .iter()
            .map(|b| (b.display.clone(), b.kind))
            .collect();
        let specs = document_specs(&self.document);
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
            .document
            .blocks()
            .iter()
            .filter(|b| matches!(b.kind, RichBlockKind::Code | RichBlockKind::Diff { .. }))
            .map(|b| (b.display.clone(), b.kind))
            .collect();
        paint_code_line_numbers(s, t, &self.field.surface, c, theme, &numbered);
        paint_link_underlines(
            s,
            &self.field.surface,
            c,
            theme,
            self.document.links().iter().map(|link| &link.display),
        );
    }
    fn focusable(&self) -> bool {
        self.field.focusable()
    }
    fn set_focus(&mut self, f: bool, k: bool) {
        self.field.set_focus(f, k)
    }
    fn window_active_changed(&mut self, active: bool) {
        self.field.window_active_changed(active);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.field.presentation_runnable_changed(runnable);
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.field.lifecycle(event);
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
            self.document.links(),
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
                .filter(|index| *index < self.document.links().len())
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
        self.field.accessibility_action(None, action)
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if key.down && !key.repeat && key.key == Some(ControlKey::Enter) && key.modifiers.command {
            let caret = self.field.surface.input.caret;
            if let Some(index) = self
                .document
                .links()
                .iter()
                .position(|link| link.display.contains(&caret))
            {
                return ContentResponse::consumed(
                    self.emit_link(index, ContentActionSource::Keyboard),
                );
            }
        }
        self.field.key_event(key)
    }
    fn pointer_event(&mut self, p: &PointerEvent) -> ContentResponse {
        // Cmd-click opens a host callback; plain pointer gestures keep the
        // original text selection/capture behavior, including cross-block drag.
        if p.is_primary()
            && p.phase == PointerPhase::Down
            && p.modifiers.command
            && let Some(index) = self
                .document
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
        self.field.pointer_event(p)
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.field.event(event)
    }
}

pub(super) struct RichRenderer<'a> {
    host: &'a mut dyn ButtonTextRenderer,
    specs: &'a [ShapedBlockSpec],
    block_ranges: &'a [(Range<usize>, RichBlockKind)],
    code_spans: &'a [CodeSpan],
    paint_cache: Option<&'a CachedRichPaint>,
    palette: Option<Palette>,
    type_scale: f32,
    caret_visible: bool,
}
impl ButtonTextRenderer for RichRenderer<'_> {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        self.host.measure(text, role, width)
    }
    fn draw(
        &mut self,
        s: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.host.draw(s, text, role, rect, align, ink)
    }
    fn backing_scale(&self) -> f32 {
        self.host.backing_scale()
    }
    fn motion_context(&self) -> Option<crate::ControlMotionContext> {
        self.host.motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        self.host.animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        self.host.request_animation_frame()
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        if self.host.text_surface_host().is_some() {
            Some(self)
        } else {
            None
        }
    }
}
impl TextSurfaceHost for RichRenderer<'_> {
    fn type_scale(&self) -> f32 {
        self.type_scale
    }
    fn caret_visible(&self) -> bool {
        self.caret_visible
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        let host = self.host.text_surface_host().expect("checked text service");
        if self.specs.is_empty() {
            return host.shape_surface(text, role, width);
        }
        Rc::new(shape_block_sequence(host, text, self.specs, width))
    }
}
impl RichRenderer<'_> {
    pub(super) fn new<'a>(
        host: &'a mut dyn ButtonTextRenderer,
        specs: &'a [ShapedBlockSpec],
        block_ranges: &'a [(Range<usize>, RichBlockKind)],
        code_spans: &'a [CodeSpan],
        paint_cache: Option<&'a CachedRichPaint>,
        palette: Option<Palette>,
    ) -> RichRenderer<'a> {
        let (type_scale, caret_visible) = host
            .text_surface_host()
            .map_or((1., true), |text| (text.type_scale(), text.caret_visible()));
        RichRenderer {
            host,
            specs,
            block_ranges,
            code_spans,
            paint_cache,
            palette,
            type_scale,
            caret_visible,
        }
    }
}
impl TextSurfaceTextRenderer for RichRenderer<'_> {
    fn push_visible(&mut self, s: &mut Scene, p: &Paragraph, origin: Point, ink: Rgba, clip: Rect) {
        let host = self.host.text_surface_host().expect("checked text service");
        // Recolor the actual retained runs; never shape a second paint layout.
        if let Some(palette) = self.palette {
            let fallback;
            let cache = if let Some(cached) = self.paint_cache.filter(|c| c.matches(p)) {
                cached
            } else {
                fallback = CachedRichPaint::build_ranges_with_weak(
                    Weak::new(),
                    p,
                    self.block_ranges,
                    self.code_spans,
                );
                &fallback
            };
            if cache.plain {
                host.push_visible(s, p, origin, ink, clip);
                return;
            }
            for (rich_ink, batch) in &cache.batches {
                host.push_visible(s, batch, origin, rich_ink.resolve(palette, ink), clip);
            }
        } else {
            host.push_visible(s, p, origin, ink, clip);
        }
    }
}

pub(super) fn block_lines<'a>(
    lines: &'a [tekes_ui_text::Line],
    display: &Range<usize>,
) -> &'a [tekes_ui_text::Line] {
    let start = lines.partition_point(|line| {
        line.utf16.end < display.start
            || (line.utf16.end == display.start
                && !(display.is_empty() && line.utf16.start == display.start))
    });
    let len = lines[start..].partition_point(|line| {
        (line.utf16.start < display.end && line.utf16.end > display.start)
            || (display.is_empty() && line.utf16.start == display.start)
    });
    &lines[start..start + len]
}

#[derive(Clone, Debug)]
pub(super) struct ShapedBlockSpec {
    pub display: Range<usize>,
    pub role: proposal::TextRole,
    pub kind: RichBlockKind,
    pub gap_after: bool,
}

fn document_specs(document: &RichDocument) -> Vec<ShapedBlockSpec> {
    document
        .blocks()
        .iter()
        .map(|block| ShapedBlockSpec {
            display: block.display.clone(),
            role: block.role,
            kind: block.kind,
            gap_after: true,
        })
        .collect()
}

pub(super) fn shape_block_sequence(
    host: &mut dyn TextSurfaceHost,
    text: &str,
    specs: &[ShapedBlockSpec],
    width: Option<f32>,
) -> Paragraph {
    let units: Vec<_> = text.encode_utf16().collect();
    let mut result = Paragraph {
        lines: vec![],
        height: 0.,
        width: 0.,
        default_line_height: 0.,
        fonts: vec![],
        utf16_len: units.len(),
    };
    let has_gutter = specs
        .iter()
        .any(|block| matches!(block.kind, RichBlockKind::Code | RichBlockKind::Diff { .. }));
    let code_gutter = if has_gutter {
        host.shape_surface(
            &line_number_sample_str(text),
            proposal::TextRole::UiCaption,
            None,
        )
        .width
            + proposal::metric("spacing", "space-relaxed").unwrap_or(12.) as f64
    } else {
        0.
    };
    let mut byte_offset = 0;
    let mut y = 0.;
    for (index, block) in specs.iter().enumerate() {
        let end = specs
            .get(index + 1)
            .map_or(result.utf16_len, |next| next.display.start);
        let value = String::from_utf16(&units[block.display.start..end])
            .expect("validated block boundaries");
        let gutter = if matches!(block.kind, RichBlockKind::Code | RichBlockKind::Diff { .. }) {
            code_gutter
        } else {
            0.
        };
        let inset = code_inset(block.kind) as f64;
        y += inset;
        let text_offset = gutter + inset;
        let paragraph = host.shape_surface(
            &value,
            block.role,
            width.map(|w| (w - gutter as f32 - 2. * inset as f32).max(0.)),
        );
        result.default_line_height = result
            .default_line_height
            .max(paragraph.default_line_height);
        for font in &paragraph.fonts {
            if result.font(font.key).is_none() {
                result.fonts.push(font.clone());
            }
        }
        let mut pushed_for_block = 0usize;
        for line in &paragraph.lines {
            // A block-final hard break is already represented by the preceding
            // line. Its synthetic trailing empty line belongs to the next block.
            if index + 1 < specs.len()
                && line.utf16.is_empty()
                && line.utf16.start == end - block.display.start
            {
                continue;
            }
            let mut line = line.clone();
            line.index = result.lines.len();
            line.top += y;
            line.baseline_y += y;
            line.utf16.start += block.display.start;
            line.utf16.end += block.display.start;
            line.bytes.start += byte_offset;
            line.bytes.end += byte_offset;
            line.width += text_offset;
            line.width_trimmed += text_offset;
            for (caret, x) in &mut line.carets {
                *caret += block.display.start;
                *x += text_offset;
            }
            for grapheme in &mut line.graphemes {
                grapheme.start += block.display.start;
                grapheme.end += block.display.start;
                grapheme.x_start += text_offset;
                grapheme.x_end += text_offset;
            }
            for run in &mut line.runs {
                run.utf16.start += block.display.start;
                run.utf16.end += block.display.start;
                for glyph in &mut run.glyphs {
                    glyph.x += text_offset;
                    glyph.utf16_index += block.display.start;
                    glyph.cluster_utf16.start += block.display.start;
                    glyph.cluster_utf16.end += block.display.start;
                }
            }
            result.width = result.width.max(line.width + inset);
            result.lines.push(line);
            pushed_for_block += 1;
        }
        if block.display.is_empty() && pushed_for_block == 0 {
            let line_height = paragraph
                .lines
                .first()
                .map_or(paragraph.default_line_height, |l| l.height)
                .max(f64::from(
                    block.role.style().line_height * host.type_scale(),
                ));
            let ascent = paragraph
                .lines
                .first()
                .map_or(line_height * 0.8, |l| l.ascent);
            let descent = paragraph
                .lines
                .first()
                .map_or(line_height - ascent, |l| l.descent);
            result.lines.push(tekes_ui_text::Line {
                index: result.lines.len(),
                utf16: block.display.start..block.display.start,
                bytes: byte_offset..byte_offset,
                ascent,
                descent,
                leading: (line_height - ascent - descent).max(0.),
                height: line_height,
                top: y,
                baseline_y: y + ascent,
                width: text_offset,
                width_trimmed: text_offset,
                runs: Vec::new(),
                carets: vec![(block.display.start, text_offset)],
                graphemes: Vec::new(),
                has_newline: false,
            });
            result.width = result.width.max(text_offset + inset);
        }
        y = result.lines.last().map_or(y, |line| line.top + line.height) + inset;
        if index + 1 < specs.len() && block.gap_after {
            let is_diff_pair = matches!(block.kind, RichBlockKind::Diff { .. })
                && matches!(specs[index + 1].kind, RichBlockKind::Diff { .. });
            y += if is_diff_pair {
                tekes_ui_tokens::Spacing::Micro.value() as f64 * f64::from(host.type_scale())
            } else {
                proposal::metric("spacing", "space-relaxed").unwrap_or(12.) as f64
            };
        }
        byte_offset += value.len();
    }
    result.height = y.max(result.default_line_height);
    result
}

// Match the existing CodeBlockView surface inset; it is part of text geometry.
pub(super) fn code_inset(kind: RichBlockKind) -> f32 {
    if kind == RichBlockKind::Code {
        proposal::metric("spacing", "space-relaxed").expect("code surface inset")
    } else {
        0.
    }
}

// Actual host typography measures the decoration; it never enters copy/ranges.
fn line_number_sample_str(text: &str) -> String {
    "0".repeat(text.lines().count().max(1).to_string().len())
}

// A line-end and the next line's start can share a UTF16 index. The native
// firstRect start must use Paragraph::caret_rect's downstream-line affinity,
// while retaining both original coordinates for a preceding line's range end.
pub(super) fn order_document_carets(carets: &mut [(usize, Rect)]) {
    carets.sort_by(|(ai, a), (bi, b)| ai.cmp(bi).then_with(|| b.min_y().total_cmp(&a.min_y())));
}

pub(super) fn block_geometry_for_ranges(
    surface: &TextSurface,
    ranges: &[(Range<usize>, RichBlockKind)],
) -> Vec<RichBlockGeometry> {
    let Some(paragraph) = surface
        .paragraph
        .as_ref()
        .filter(|_| surface.shaped == surface.input.text)
    else {
        return Vec::new();
    };
    let origin = surface.origin();
    ranges
        .iter()
        .enumerate()
        .filter_map(|(index, (display, kind))| {
            let lines = block_lines(&paragraph.lines, display);
            let first = lines.first()?;
            let last = lines.last()?;
            let inset = code_inset(*kind);
            let frame = Rect::new(
                origin.x,
                origin.y + first.top as f32 - inset,
                surface.inner.size.width,
                (last.top + last.height - first.top) as f32 + 2. * inset,
            );
            Some(RichBlockGeometry {
                index,
                display: display.clone(),
                frame,
                visible: surface
                    .visible_frame()
                    .and_then(|clip| clip.intersection(&frame)),
            })
        })
        .collect()
}

pub(super) fn surface_link_rects(surface: &TextSurface, display: &Range<usize>) -> Vec<Rect> {
    let Some(p) = surface
        .paragraph
        .as_ref()
        .filter(|_| surface.shaped == surface.input.text)
    else {
        return Vec::new();
    };
    let origin = surface.origin();
    p.selection_rects(
        display.clone(),
        Rect::new(0., 0., surface.inner.size.width, p.height as f32),
    )
    .into_iter()
    .filter_map(|r| {
        surface.visible_frame().and_then(|clip| {
            clip.intersection(&Rect::new(
                r.min_x() + origin.x,
                r.min_y() + origin.y,
                r.size.width,
                r.size.height,
            ))
        })
    })
    .collect()
}

pub(super) fn paint_code_surfaces(
    s: &mut Scene,
    t: &mut dyn ButtonTextRenderer,
    theme: ContentTheme,
    geometry: &[RichBlockGeometry],
    kind_at: impl Fn(usize) -> RichBlockKind,
) {
    for block in geometry {
        if kind_at(block.index) == RichBlockKind::Code
            && let Some(visible) = block.visible
        {
            s.push_clip(visible);
            crate::paint::surface_kind_at_scale(
                s,
                block.frame,
                theme.palette,
                crate::paint::SurfaceKind::Code,
                t.backing_scale(),
            );
            s.pop_clip();
        }
    }
}

pub(super) fn paint_code_line_numbers(
    s: &mut Scene,
    t: &mut dyn ButtonTextRenderer,
    surface: &TextSurface,
    clip: Rect,
    theme: ContentTheme,
    blocks: &[(Range<usize>, RichBlockKind)],
) {
    if blocks.is_empty() {
        return;
    }
    let Some(content_clip) = surface
        .visible_frame()
        .and_then(|vf| vf.intersection(&clip))
    else {
        return;
    };
    let Some(paragraph) = surface.paragraph.as_ref() else {
        return;
    };
    let origin = surface.origin();
    let caption = proposal::TextRole::UiCaption;
    let sample = line_number_sample_str(&surface.input.text);
    let gutter = if let Some(host) = t.text_surface_host() {
        host.shape_surface(&sample, caption, None).width as f32
    } else {
        t.measure(&sample, caption, f32::MAX).width
    };
    let gutter_ink = tekes_ui_tokens::surface_palette::resolve_surface(
        theme.palette,
        "code",
        "action-quiet",
        "disabled",
    )
    .expect("code gutter Surface ink")
    .text;
    let mut number = 1usize;
    for (display, kind) in blocks {
        if !matches!(kind, RichBlockKind::Code | RichBlockKind::Diff { .. }) {
            continue;
        }
        if *kind == RichBlockKind::Code {
            number = 1;
        }
        if display.is_empty() {
            continue;
        }
        let mut first = true;
        for line in block_lines(&paragraph.lines, display) {
            if first {
                let frame = Rect::new(
                    origin.x + code_inset(*kind),
                    origin.y + line.top as f32,
                    gutter,
                    line.height as f32,
                );
                if let Some(visible) = frame.intersection(&content_clip) {
                    let label = number.to_string();
                    s.push_clip(visible);
                    if let Some(host) = t.text_surface_host() {
                        let num_para = host.shape_surface(&label, caption, None);
                        let num_origin = num_para.origin_in(frame, true);
                        host.push_visible(s, &num_para, num_origin, gutter_ink, visible);
                    } else {
                        t.draw(
                            s,
                            &label,
                            caption,
                            frame,
                            ButtonTextAlign::Leading,
                            gutter_ink,
                        );
                    }
                    s.pop_clip();
                }
                number += 1;
            }
            first = line.has_newline;
        }
    }
}

pub(super) fn paint_link_underlines<'a>(
    s: &mut Scene,
    surface: &TextSurface,
    clip: Rect,
    theme: ContentTheme,
    displays: impl IntoIterator<Item = &'a Range<usize>>,
) {
    let mut style: Option<(Rgba, f32)> = None;
    for display in displays {
        for rect in surface_link_rects(surface, display) {
            let (ink, rule) = *style.get_or_insert_with(|| {
                let ink = tekes_ui_tokens::surface_palette::resolve_surface(
                    theme.palette,
                    theme.surface(),
                    "link",
                    "rest",
                )
                .expect("validated inline Link Surface")
                .text;
                let rule =
                    proposal::metric("stroke", "stroke-rule").expect("Link underline stroke");
                (ink, rule)
            });
            s.push_clip(clip);
            let thickness = rule.min(rect.size.height);
            s.push_quad(tekes_ui_scene::Quad::new(
                Rect::new(
                    rect.min_x(),
                    rect.max_y() - thickness,
                    rect.size.width,
                    thickness,
                ),
                ink,
            ));
            s.pop_clip();
        }
    }
}

pub(super) fn link_semantic_parts(
    surface: &TextSurface,
    links: &[RichLink],
    base: usize,
) -> Vec<SemanticPart> {
    links
        .iter()
        .enumerate()
        .flat_map(|(index, link)| {
            let mut semantics = ContentSemantics::new(ContentRole::Link, &link.target);
            semantics.value = Some(link.target.clone());
            surface_link_rects(surface, &link.display)
                .into_iter()
                .reduce(|a, b| {
                    Rect::new(
                        a.min_x().min(b.min_x()),
                        a.min_y().min(b.min_y()),
                        a.max_x().max(b.max_x()) - a.min_x().min(b.min_x()),
                        a.max_y().max(b.max_y()) - a.min_y().min(b.min_y()),
                    )
                })
                .map(|frame| SemanticPart {
                    index: base + index,
                    frame,
                    focused: false,
                    semantics,
                })
                .into_iter()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::FontKey;
    use tekes_ui_text::{FontInfo, Line, ShapedGlyph, ShapedRun};
    use tekes_ui_tokens::{Palette, Theme};

    struct RecordingTextHost {
        type_scale: f32,
        shaped_inputs: Vec<(String, proposal::TextRole)>,
        pushed: Vec<(Point, Rgba, Rect, usize)>,
        fallback_draws: Vec<(String, Rect)>,
    }

    impl RecordingTextHost {
        fn new(type_scale: f32) -> Self {
            Self {
                type_scale,
                shaped_inputs: Vec::new(),
                pushed: Vec::new(),
                fallback_draws: Vec::new(),
            }
        }
    }

    impl ButtonTextRenderer for RecordingTextHost {
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
            Size::new(
                (s.chars().count() as f32 * 8. * self.type_scale).min(w),
                r.style().line_height * self.type_scale,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            s: &str,
            _: proposal::TextRole,
            r: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
            self.fallback_draws.push((s.to_owned(), r));
        }
    }

    impl TextSurfaceHost for RecordingTextHost {
        fn type_scale(&self) -> f32 {
            self.type_scale
        }
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            _width: Option<f32>,
        ) -> Rc<Paragraph> {
            self.shaped_inputs.push((text.to_owned(), role));
            let line_height = f64::from(role.style().line_height * self.type_scale);
            let ascent = line_height * 0.8;
            let descent = line_height - ascent;
            let font_key = FontKey(1);
            let mut lines = Vec::new();
            let mut y = 0.0;
            let mut utf16 = 0usize;
            let mut byte = 0usize;
            let mut max_w = 0.0_f64;

            for segment in text.split_inclusive('\n') {
                let has_nl = segment.ends_with('\n');
                let len_utf16 = segment.encode_utf16().count();
                let end_utf16 = utf16 + len_utf16;
                let end_byte = byte + segment.len();
                let mut glyphs = Vec::new();
                let mut carets = Vec::new();
                let mut x = 0.0;
                let mut cur_u16 = utf16;
                for ch in segment.chars() {
                    carets.push((cur_u16, x));
                    if ch != '\n' {
                        glyphs.push(ShapedGlyph {
                            id: ch as u16,
                            x,
                            y: 0.0,
                            utf16_index: cur_u16,
                            cluster_utf16: cur_u16..cur_u16 + ch.len_utf16(),
                        });
                        x += 8.0 * f64::from(self.type_scale);
                    }
                    cur_u16 += ch.len_utf16();
                }
                carets.push((end_utf16, x));
                max_w = max_w.max(x);
                lines.push(Line {
                    index: lines.len(),
                    utf16: utf16..end_utf16,
                    bytes: byte..end_byte,
                    ascent,
                    descent,
                    leading: 0.0,
                    height: line_height,
                    top: y,
                    baseline_y: y + ascent,
                    width: x,
                    width_trimmed: x,
                    runs: if glyphs.is_empty() {
                        Vec::new()
                    } else {
                        vec![ShapedRun {
                            font: font_key,
                            utf16: utf16..end_utf16,
                            glyphs,
                        }]
                    },
                    carets,
                    graphemes: Vec::new(),
                    has_newline: has_nl,
                });
                y += line_height;
                utf16 = end_utf16;
                byte = end_byte;
            }
            if text.ends_with('\n') {
                lines.push(Line {
                    index: lines.len(),
                    utf16: utf16..utf16,
                    bytes: byte..byte,
                    ascent,
                    descent,
                    leading: 0.0,
                    height: line_height,
                    top: y,
                    baseline_y: y + ascent,
                    width: 0.0,
                    width_trimmed: 0.0,
                    runs: Vec::new(),
                    carets: vec![(utf16, 0.0)],
                    graphemes: Vec::new(),
                    has_newline: false,
                });
                y += line_height;
            }

            Rc::new(Paragraph {
                lines,
                height: y.max(line_height),
                width: max_w,
                default_line_height: line_height,
                fonts: vec![FontInfo {
                    key: font_key,
                    post_script_name: "TestFont".into(),
                    size: f64::from(role.style().size * self.type_scale),
                    is_color: false,
                }],
                utf16_len: text.encode_utf16().count(),
            })
        }
    }

    impl TextSurfaceTextRenderer for RecordingTextHost {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            paragraph: &Paragraph,
            origin: Point,
            color: Rgba,
            clip: Rect,
        ) {
            let glyphs = paragraph
                .lines
                .iter()
                .flat_map(|l| l.runs.iter())
                .map(|r| r.glyphs.len())
                .sum();
            self.pushed.push((origin, color, clip, glyphs));
        }
    }

    #[test]
    fn code_line_numbers_clip_to_visible_frame_and_follow_type_scale() {
        let code = (1..=12)
            .map(|i| format!("let v{i} = {i};"))
            .collect::<Vec<_>>()
            .join("\n");
        let doc = RichDocument::from_markdown(&format!("```rust\n{code}\n```"));
        let mut rich = RichContent::new("scrolled", 1, doc).unwrap();
        rich.field.viewport = TextViewport::Fixed(80.);
        rich.field
            .surface
            .scroll
            .set_scroll_policy(crate::ScrollPolicy::Disabled, crate::ScrollPolicy::Auto);
        let mut host = RecordingTextHost::new(2.0);
        let frame = Rect::new(10., 100., 280., 80.);
        // Ancestor clip is intentionally larger than frame to verify line
        // numbers clip against `visible_frame()` rather than `c` alone.
        let ancestor = Rect::new(0., 0., 400., 400.);
        rich.arrange(&mut host, frame, ancestor);
        assert_eq!(rich.field.surface.visible_frame(), Some(frame));
        // Line numbers are shaped via `shape_document` only once during shape,
        // and scrolled lines above `frame.min_y()` must be clipped to `frame`.
        let gutter_shapes = host
            .shaped_inputs
            .iter()
            .filter(|(s, r)| s == "00" && *r == proposal::TextRole::UiCaption)
            .count();
        assert_eq!(gutter_shapes, 1);

        rich.field.surface.scroll.scroll_by(0., 80.);
        host.pushed.clear();
        host.shaped_inputs.clear();
        let mut scene = Scene::new();
        rich.paint(
            &mut scene,
            &mut host,
            frame,
            ancestor,
            Palette::from(Theme::LIGHT).into(),
        );
        assert!(host.fallback_draws.is_empty());
        let caption_draws: Vec<_> = host
            .pushed
            .iter()
            .filter(|(_, _, clip, glyphs)| *glyphs > 0 && clip.size.width < 100.)
            .collect();
        assert!(!caption_draws.is_empty());
        let expected_gutter_ink = tekes_ui_tokens::surface_palette::resolve_surface(
            Palette::from(Theme::LIGHT),
            "code",
            "action-quiet",
            "disabled",
        )
        .unwrap()
        .text;
        for (_, color, clip, _) in caption_draws {
            assert_eq!(*color, expected_gutter_ink);
            assert!(
                clip.min_y() >= frame.min_y() && clip.max_y() <= frame.max_y(),
                "line number clip {clip:?} escaped RichContent frame {frame:?}"
            );
        }
    }

    #[test]
    fn empty_code_blocks_match_between_middle_and_end_and_paint_cache_reuses_batches() {
        let mut host = RecordingTextHost::new(1.0);
        let middle_doc = RichDocument::from_markdown("```rust\n```\n\nafter");
        let end_doc = RichDocument::from_markdown("before\n\n```rust\n```");
        let mut mid = RichContent::new("mid", 1, middle_doc).unwrap();
        let mut end = RichContent::new("end", 1, end_doc).unwrap();
        let frame = Rect::new(0., 0., 240., 200.);
        mid.arrange(&mut host, frame, frame);
        end.arrange(&mut host, frame, frame);
        let mid_geom = mid.block_geometry();
        let end_geom = end.block_geometry();
        assert_eq!(mid_geom.len(), 2);
        assert_eq!(end_geom.len(), 2);
        assert!(
            (mid_geom[0].frame.size.height - end_geom[1].frame.size.height).abs() < 0.01,
            "middle {:?} vs end {:?}",
            mid_geom[0].frame,
            end_geom[1].frame
        );
        host.shaped_inputs.clear();
        mid.paint(
            &mut Scene::new(),
            &mut host,
            frame,
            frame,
            Palette::from(Theme::LIGHT).into(),
        );
        end.paint(
            &mut Scene::new(),
            &mut host,
            frame,
            frame,
            Palette::from(Theme::LIGHT).into(),
        );
        assert!(
            !host.shaped_inputs.iter().any(|(s, _)| s == "1"),
            "empty code blocks must not paint fake line number 1"
        );

        let mixed =
            RichDocument::from_markdown("```rust\nlet x = \"hi\"; // note\nlet y = 1;\n```");
        let mut cached = RichContent::new("cached", 1, mixed).unwrap();
        cached.arrange(&mut host, frame, frame);
        cached.paint(
            &mut Scene::new(),
            &mut host,
            frame,
            frame,
            Palette::from(Theme::LIGHT).into(),
        );
        let first_cache_ptr = cached.paint_cache.as_ref().unwrap().batches.as_ptr();
        let first_batches = cached.paint_cache.as_ref().unwrap().batches.len();
        #[cfg(feature = "syntax-highlighting")]
        assert!(first_batches >= 5);
        #[cfg(not(feature = "syntax-highlighting"))]
        assert!(cached.paint_cache.as_ref().unwrap().plain && first_batches == 0);
        cached.paint(
            &mut Scene::new(),
            &mut host,
            frame,
            frame,
            Palette::from(Theme::DARK).into(),
        );
        assert_eq!(
            cached.paint_cache.as_ref().unwrap().batches.as_ptr(),
            first_cache_ptr
        );
    }

    #[test]
    fn diff_blocks_use_micro_line_gap_and_code_blocks_use_relaxed_inset() {
        for scale in [1.0, 2.0] {
            let mut host = RecordingTextHost::new(scale);
            let diff = RichDocument::from_diff(&crate::extended::DiffView::between(
                "same\nold",
                "same\nnew",
            ));
            let mut rich = RichContent::new("diff", 1, diff).unwrap();
            let frame = Rect::new(0., 0., 320., 240.);
            rich.arrange(&mut host, frame, frame);
            let geom = rich.block_geometry();
            assert!(geom.len() >= 2);
            let gap = geom[1].frame.min_y() - geom[0].frame.max_y();
            assert!(
                (gap - 2.0 * scale).abs() < 0.01,
                "expected 2.0 * {scale} inter-diff gap, got {gap}"
            );
        }
        assert_eq!(code_inset(RichBlockKind::Code), 12.0);
    }
}
