//! One readonly text owner. Block typography is composed from the original host
//! shaper into the one Paragraph retained by TextSurface; no second hit engine.
use super::*;
use crate::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueConflict, ExternalValueResult,
    TextSurface, TextSurfaceHost, TextSurfaceTextRenderer,
};
use std::{ops::Range, rc::Rc};
use tekes_ui_text::Paragraph;

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
        self.document = document;
        self.link_part_base = self.next_link_part;
        self.next_link_part = next_link_part;
        self.revision = revision;
        Ok(self.version())
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
        let surface = &self.field.surface;
        let Some(paragraph) = surface
            .paragraph
            .as_ref()
            .filter(|_| surface.shaped == surface.input.text)
        else {
            return Vec::new();
        };
        let origin = surface.origin();
        self.document
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                let lines: Vec<_> = paragraph
                    .lines
                    .iter()
                    .filter(|line| {
                        line.utf16.start < block.display.end && line.utf16.end > block.display.start
                            || block.display.is_empty() && line.utf16.start == block.display.start
                    })
                    .collect();
                let first = lines.first()?;
                let last = lines.last()?;
                let frame = Rect::new(
                    origin.x,
                    origin.y + first.top as f32,
                    surface.inner.size.width,
                    (last.top + last.height - first.top) as f32,
                );
                Some(RichBlockGeometry {
                    index,
                    display: block.display.clone(),
                    frame,
                    visible: surface
                        .visible_frame()
                        .and_then(|clip| clip.intersection(&frame)),
                })
            })
            .collect()
    }
    fn link_rects(&self, index: usize) -> Vec<Rect> {
        let surface = &self.field.surface;
        let Some(link) = self.document.links().get(index) else {
            return Vec::new();
        };
        let Some(p) = surface
            .paragraph
            .as_ref()
            .filter(|_| surface.shaped == surface.input.text)
        else {
            return Vec::new();
        };
        let origin = surface.origin();
        p.selection_rects(
            link.display.clone(),
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
}

impl Component for RichContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        self.field.curve_subtree_capability()
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        self.field.table_transaction_compatibility()
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        self.field.prepared_effects_work(n)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        self.field.prepare_effects(hooks)
    }
    fn lifecycle_prepared(&mut self, event: Lifecycle, context: &mut PreparedLifecycleContext) {
        self.field.lifecycle_prepared(event, context);
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
        self.field
            .measure(&mut RichRenderer::new(t, &self.document, None), a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.field
            .arrange(&mut RichRenderer::new(t, &self.document, None), f, c)
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.field
            .prepare_text_input(&mut RichRenderer::new(t, &self.document, None), f, c)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.field.paint(
            s,
            &mut RichRenderer::new(t, &self.document, Some(theme.palette)),
            f,
            c,
            theme,
        );
        if let Some(paragraph) = self.field.surface.paragraph.as_ref() {
            let origin = self.field.surface.origin();
            let caption = proposal::TextRole::UiCaption;
            let gutter = t
                .measure(&line_number_sample(&self.document), caption, f32::MAX)
                .width;
            let mut number = 1;
            for block in self.document.blocks() {
                if !matches!(block.kind, RichBlockKind::Code | RichBlockKind::Diff { .. }) {
                    continue;
                }
                if block.kind == RichBlockKind::Code {
                    number = 1;
                }
                let mut first = true;
                for line in paragraph.lines.iter().filter(|line| {
                    line.utf16.start < block.display.end && line.utf16.end > block.display.start
                        || block.display.is_empty() && line.utf16.start == block.display.start
                }) {
                    if first {
                        let frame = Rect::new(
                            origin.x,
                            origin.y + line.top as f32,
                            gutter,
                            line.height as f32,
                        );
                        if let Some(visible) = frame.intersection(&c) {
                            s.push_clip(visible);
                            t.draw(
                                s,
                                &number.to_string(),
                                caption,
                                frame,
                                ButtonTextAlign::Leading,
                                theme
                                    .palette
                                    .color("secondary-text")
                                    .expect("semantic secondary text"),
                            );
                            s.pop_clip();
                        }
                        number += 1;
                    }
                    first = line.has_newline;
                }
            }
        }
        for index in 0..self.document.links().len() {
            for rect in self.link_rects(index) {
                s.push_clip(c);
                crate::extended::link_underline(
                    s,
                    rect,
                    theme.palette,
                    crate::ControlState::default(),
                );
                s.pop_clip();
            }
        }
    }
    fn focusable(&self) -> bool {
        self.field.focusable()
    }
    fn set_focus(&mut self, f: bool, k: bool) {
        self.field.set_focus(f, k)
    }
    fn semantics(&self) -> ContentSemantics {
        self.field.semantics()
    }
    fn semantic_frame(&self, f: Rect) -> Rect {
        self.field.semantic_frame(f)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.document
            .links()
            .iter()
            .enumerate()
            .flat_map(|(index, link)| {
                let mut semantics = ContentSemantics::new(ContentRole::Link, &link.target);
                semantics.value = Some(link.target.clone());
                self.link_rects(index)
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
                        index: self.link_part_base + index,
                        frame,
                        focused: false,
                        semantics,
                    })
                    .into_iter()
            })
            .collect()
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

struct RichRenderer<'a> {
    host: &'a mut dyn ButtonTextRenderer,
    document: &'a RichDocument,
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
        if self.document.blocks().is_empty() {
            return host.shape_surface(text, role, width);
        }
        Rc::new(shape_document(host, self.document, width))
    }
}
impl RichRenderer<'_> {
    fn new<'a>(
        host: &'a mut dyn ButtonTextRenderer,
        document: &'a RichDocument,
        palette: Option<Palette>,
    ) -> RichRenderer<'a> {
        let (type_scale, caret_visible) = host
            .text_surface_host()
            .map_or((1., true), |text| (text.type_scale(), text.caret_visible()));
        RichRenderer {
            host,
            document,
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
            if self.document.blocks().iter().all(|block| {
                !matches!(
                    block.kind,
                    RichBlockKind::Diff {
                        kind: crate::extended::DiffKind::Added | crate::extended::DiffKind::Removed
                    }
                )
            }) {
                host.push_visible(s, p, origin, ink, clip);
                return;
            }
            for block in self.document.blocks() {
                let color = match block.kind {
                    RichBlockKind::Diff {
                        kind: crate::extended::DiffKind::Added,
                    } => palette.color("status-succeeded").unwrap(),
                    RichBlockKind::Diff {
                        kind: crate::extended::DiffKind::Removed,
                    } => palette.color("status-failed").unwrap(),
                    _ => ink,
                };
                let part = Paragraph {
                    lines: p
                        .lines
                        .iter()
                        .filter(|line| {
                            line.utf16.start < block.display.end
                                && line.utf16.end > block.display.start
                        })
                        .cloned()
                        .collect(),
                    height: p.height,
                    width: p.width,
                    default_line_height: p.default_line_height,
                    fonts: p.fonts.clone(),
                    utf16_len: p.utf16_len,
                };
                host.push_visible(s, &part, origin, color, clip);
            }
        } else {
            host.push_visible(s, p, origin, ink, clip);
        }
    }
}

fn shape_document(
    host: &mut dyn TextSurfaceHost,
    document: &RichDocument,
    width: Option<f32>,
) -> Paragraph {
    let units: Vec<_> = document.text().encode_utf16().collect();
    let mut result = Paragraph {
        lines: vec![],
        height: 0.,
        width: 0.,
        default_line_height: 0.,
        fonts: vec![],
        utf16_len: units.len(),
    };
    let mut byte_offset = 0;
    let mut y = 0.;
    for (index, block) in document.blocks().iter().enumerate() {
        let end = document
            .blocks()
            .get(index + 1)
            .map_or(result.utf16_len, |next| next.display.start);
        let value = String::from_utf16(&units[block.display.start..end])
            .expect("validated block boundaries");
        let gutter = if matches!(block.kind, RichBlockKind::Code | RichBlockKind::Diff { .. }) {
            host.shape_surface(
                &line_number_sample(document),
                proposal::TextRole::UiCaption,
                None,
            )
            .width
                + proposal::metric("spacing", "space-relaxed").unwrap_or(12.) as f64
        } else {
            0.
        };
        let paragraph = host.shape_surface(
            &value,
            block.role,
            width.map(|w| (w - gutter as f32).max(0.)),
        );
        result.default_line_height = result
            .default_line_height
            .max(paragraph.default_line_height);
        for font in &paragraph.fonts {
            if result.font(font.key).is_none() {
                result.fonts.push(font.clone());
            }
        }
        for line in &paragraph.lines {
            // A block-final hard break is already represented by the preceding
            // line. Its synthetic trailing empty line belongs to the next block.
            if index + 1 < document.blocks().len()
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
            line.width += gutter;
            line.width_trimmed += gutter;
            for (caret, x) in &mut line.carets {
                *caret += block.display.start;
                *x += gutter;
            }
            for run in &mut line.runs {
                run.utf16.start += block.display.start;
                run.utf16.end += block.display.start;
                for glyph in &mut run.glyphs {
                    glyph.x += gutter;
                    glyph.utf16_index += block.display.start;
                    glyph.cluster_utf16.start += block.display.start;
                    glyph.cluster_utf16.end += block.display.start;
                }
            }
            result.width = result.width.max(line.width);
            result.lines.push(line);
        }
        y = result.lines.last().map_or(y, |line| line.top + line.height);
        if index + 1 < document.blocks().len() {
            y += proposal::metric("spacing", "space-relaxed").unwrap_or(12.) as f64;
        }
        byte_offset += value.len();
    }
    result.height = y.max(result.default_line_height);
    result
}

// Actual host typography measures the decoration; it never enters copy/ranges.
fn line_number_sample(document: &RichDocument) -> String {
    "0".repeat(document.text().lines().count().max(1).to_string().len())
}

// A line-end and the next line's start can share a UTF16 index. The native
// firstRect start must use Paragraph::caret_rect's downstream-line affinity,
// while retaining both original coordinates for a preceding line's range end.
fn order_document_carets(carets: &mut [(usize, Rect)]) {
    carets.sort_by(|(ai, a), (bi, b)| ai.cmp(bi).then_with(|| b.min_y().total_cmp(&a.min_y())));
}
