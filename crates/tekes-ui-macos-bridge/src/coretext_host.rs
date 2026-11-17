//! Shared CoreText shaping, glyph-atlas rasterization, and composition text host adapter.

use std::{collections::HashMap, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, TextSurfaceHost,
    TextSurfaceTextRenderer,
};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{
    LabelLayout, LabelOptions, Paragraph, SemanticLabel, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper, PreparedText},
    push_paragraph, push_paragraph_visible,
};
use tekes_ui_tokens::{Rgba, proposal};

/// Opt-in per-draw CPU observations, reset by [`CoreTextHost::begin_frame`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ParagraphLayoutDiagnostic {
    pub requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub unique_entries: usize,
    pub prepare_count: u64,
    pub prepare_ns: u128,
    pub natural_count: u64,
    pub natural_ns: u128,
    pub bounded_count: u64,
    pub bounded_ns: u128,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ParagraphLayoutKey {
    text: String,
    role: proposal::TextRole,
    type_scale: u32,
    width: u32,
}

/// Resolve the system or monospace [`TextStyle`] for a token [`proposal::TextRole`] at `type_scale`.
pub fn role_style(role: proposal::TextRole, type_scale: f32) -> TextStyle {
    let metrics = role.style();
    let size = metrics.size * type_scale;
    let style = if proposal::text_family(role.id()) == Some("mono") {
        TextStyle::named("Menlo", size)
    } else {
        TextStyle::system(size)
    };
    style.with_weight(metrics.weight)
}

/// Return the widest non-trailing-whitespace line advance in `paragraph`.
pub fn trimmed_advance(paragraph: &Paragraph) -> f32 {
    paragraph
        .lines
        .iter()
        .map(|line| line.width_trimmed)
        .fold(0., f64::max) as f32
}

/// Normalize a shaped paragraph's line tops, heights, and baselines against `role` metrics.
pub fn normalize_role_paragraph(
    mut paragraph: Paragraph,
    role: proposal::TextRole,
    type_scale: f32,
) -> Paragraph {
    let metrics = role.style();
    let mut top = 0.;
    for line in &mut paragraph.lines {
        let height = f64::from(metrics.line_height * type_scale).max(line.height);
        line.top = top;
        line.height = height;
        line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
        top += height;
    }
    paragraph.height = top.max(f64::from(metrics.line_height * type_scale));
    paragraph.default_line_height = f64::from(metrics.line_height * type_scale);
    paragraph
}

/// Shape system UI text at an explicit final logical size and weight.
pub fn shape_system_text_with_weight(
    shaper: &mut CoreTextShaper,
    value: &str,
    final_logical_point_size: f32,
    weight: u16,
    width: Option<f32>,
) -> Option<Rc<Paragraph>> {
    Some(Rc::new(shaper.shape(
        value,
        &TextStyle::system(final_logical_point_size).with_weight(weight),
        &ShapeOptions {
            width: width.map(f64::from),
            line_spacing: 0.,
        },
    )))
}

/// Shape regular-weight system UI text at an explicit final logical size.
pub fn shape_system_text(
    shaper: &mut CoreTextShaper,
    value: &str,
    final_logical_point_size: f32,
    width: Option<f32>,
) -> Option<Rc<Paragraph>> {
    Some(Rc::new(shaper.shape(
        value,
        &TextStyle::system(final_logical_point_size),
        &ShapeOptions {
            width: width.map(f64::from),
            line_spacing: 0.,
        },
    )))
}

/// Shape a plain label at `role` and `type_scale`.
pub fn shape_label(
    shaper: &mut CoreTextShaper,
    value: &str,
    role: proposal::TextRole,
    type_scale: f32,
    width: f32,
    options: LabelOptions,
) -> Option<Rc<LabelLayout>> {
    let metrics = role.style();
    let style = role_style(role, type_scale);
    Some(Rc::new(
        shaper
            .shape_label(
                value,
                &style,
                &ShapeOptions::with_width(f64::from(width)),
                options,
            )
            .with_min_line_height(f64::from(metrics.line_height * type_scale)),
    ))
}

/// Shape a styled [`SemanticLabel`] at `role` and `type_scale`.
pub fn shape_semantic_label(
    shaper: &mut CoreTextShaper,
    value: &SemanticLabel,
    role: proposal::TextRole,
    type_scale: f32,
    width: f32,
    options: LabelOptions,
) -> Option<Rc<LabelLayout>> {
    let metrics = role.style();
    let style = role_style(role, type_scale);
    Some(Rc::new(
        shaper
            .shape_semantic_label(
                value,
                &style,
                &ShapeOptions::with_width(f64::from(width)),
                options,
            )
            .with_min_line_height(f64::from(metrics.line_height * type_scale)),
    ))
}

/// Shape and normalize a role-styled surface paragraph.
pub fn shape_surface(
    shaper: &mut CoreTextShaper,
    value: &str,
    role: proposal::TextRole,
    type_scale: f32,
    width: Option<f32>,
) -> Rc<Paragraph> {
    let style = role_style(role, type_scale);
    let paragraph = shaper.shape(
        value,
        &style,
        &ShapeOptions {
            width: width.map(f64::from),
            line_spacing: 0.,
        },
    );
    Rc::new(normalize_role_paragraph(paragraph, role, type_scale))
}

/// Prepare `text` once for multiple width-bounded reflows under `role` and `type_scale`.
pub fn prepare_role_text(
    shaper: &CoreTextShaper,
    text: &str,
    role: proposal::TextRole,
    type_scale: f32,
) -> PreparedText {
    let style = role_style(role, type_scale);
    shaper.prepare(text, &style)
}

/// Reflow already prepared text to an optional `width` and normalize line metrics for `role`.
pub fn reflow_role_text(
    shaper: &mut CoreTextShaper,
    prepared: &PreparedText,
    role: proposal::TextRole,
    type_scale: f32,
    width: Option<f32>,
) -> Paragraph {
    let paragraph = shaper.shape_prepared(
        prepared,
        &ShapeOptions {
            width: width.map(f64::from),
            line_spacing: 0.,
        },
    );
    normalize_role_paragraph(paragraph, role, type_scale)
}

/// Shared CoreText font registry, shaper, glyph rasterizer, and atlas host implementing
/// [`ButtonTextRenderer`], [`TextSurfaceHost`], and [`TextSurfaceTextRenderer`].
pub struct CoreTextHost {
    pub atlas: GlyphAtlas,
    pub shaper: CoreTextShaper,
    pub raster: CoreTextRasterizer,
    pub scale: f32,
    pub type_scale: f32,
    pub motion_context: Option<ControlMotionContext>,
    pub animation_requested: bool,
    pub services_enabled: bool,
    pub paragraph_diagnostic: Option<ParagraphLayoutDiagnostic>,
    frame_paragraphs: Option<HashMap<ParagraphLayoutKey, (Rc<Paragraph>, f32)>>,
}

impl std::fmt::Debug for CoreTextHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreTextHost")
            .field("scale", &self.scale)
            .field("type_scale", &self.type_scale)
            .field("services_enabled", &self.services_enabled)
            .finish_non_exhaustive()
    }
}

impl Default for CoreTextHost {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreTextHost {
    /// Construct a standard production host with `scale = 1.0`, `type_scale = 1.0`, and text
    /// services enabled.
    pub fn new() -> Self {
        let fonts = CoreTextFonts::new();
        Self {
            atlas: GlyphAtlas::default(),
            shaper: CoreTextShaper::new(fonts.clone()),
            raster: CoreTextRasterizer::new(fonts),
            scale: 1.,
            type_scale: 1.,
            motion_context: None,
            animation_requested: false,
            services_enabled: true,
            paragraph_diagnostic: None,
            frame_paragraphs: None,
        }
    }

    pub fn with_diagnostic(mut self, enabled: bool) -> Self {
        self.paragraph_diagnostic = enabled.then(ParagraphLayoutDiagnostic::default);
        self
    }

    pub fn with_type_scale(mut self, type_scale: f32) -> Self {
        self.type_scale = type_scale;
        self
    }

    pub fn with_backing_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_text_services(mut self, enabled: bool) -> Self {
        self.services_enabled = enabled;
        if enabled && self.motion_context.is_none() {
            self.motion_context = Some(ControlMotionContext {
                now: std::time::Duration::ZERO,
                reduced_motion: true,
            });
        }
        self
    }

    pub fn role_style(&self, role: proposal::TextRole) -> TextStyle {
        role_style(role, self.type_scale)
    }

    /// Shape or retrieve from the active frame cache the CoreText paragraph and trimmed/bounded slot.
    pub fn paragraph_layout(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> (Rc<Paragraph>, f32) {
        let key = self.frame_paragraphs.as_ref().map(|_| ParagraphLayoutKey {
            text: value.to_owned(),
            role,
            type_scale: self.type_scale.to_bits(),
            width: width.to_bits(),
        });
        if let Some(stats) = &mut self.paragraph_diagnostic {
            stats.requests += 1;
        }
        if let Some(layout) = key
            .as_ref()
            .and_then(|key| self.frame_paragraphs.as_ref()?.get(key))
        {
            if let Some(stats) = &mut self.paragraph_diagnostic {
                stats.hits += 1;
            }
            return layout.clone();
        }
        if let Some(stats) = &mut self.paragraph_diagnostic {
            stats.misses += 1;
        }
        let style = self.role_style(role);
        let begin = self.paragraph_diagnostic.map(|_| std::time::Instant::now());
        let prepared = self.shaper.prepare(value, &style);
        if let (Some(begin), Some(stats)) = (begin, &mut self.paragraph_diagnostic) {
            stats.prepare_count += 1;
            stats.prepare_ns += begin.elapsed().as_nanos();
        }
        let begin = self.paragraph_diagnostic.map(|_| std::time::Instant::now());
        let natural = self
            .shaper
            .shape_prepared(&prepared, &ShapeOptions::default());
        if let (Some(begin), Some(stats)) = (begin, &mut self.paragraph_diagnostic) {
            stats.natural_count += 1;
            stats.natural_ns += begin.elapsed().as_nanos();
        }
        let intrinsic = trimmed_advance(&natural);
        let (paragraph, slot) = if !width.is_finite() || intrinsic <= width.max(0.) + 1. / 64. {
            (natural, intrinsic)
        } else {
            let slot = width.max(1.);
            let begin = self.paragraph_diagnostic.map(|_| std::time::Instant::now());
            let bounded = self
                .shaper
                .shape_prepared(&prepared, &ShapeOptions::with_width(f64::from(slot)));
            if let (Some(begin), Some(stats)) = (begin, &mut self.paragraph_diagnostic) {
                stats.bounded_count += 1;
                stats.bounded_ns += begin.elapsed().as_nanos();
            }
            (bounded, slot)
        };
        let paragraph = Rc::new(paragraph);
        if let (Some(cache), Some(key)) = (&mut self.frame_paragraphs, key) {
            cache.insert(key, (paragraph.clone(), slot));
            if let Some(stats) = &mut self.paragraph_diagnostic {
                stats.unique_entries = cache.len();
            }
        }
        (paragraph, slot)
    }

    pub fn paragraph_layout_diagnostic(&self) -> Option<ParagraphLayoutDiagnostic> {
        self.paragraph_diagnostic
    }

    pub fn has_frame_paragraphs(&self) -> bool {
        self.frame_paragraphs.is_some()
    }

    pub fn frame_paragraph_count(&self) -> Option<usize> {
        self.frame_paragraphs.as_ref().map(HashMap::len)
    }

    pub fn begin_frame(&mut self, now: std::time::Duration, reduced_motion: bool) {
        self.motion_context = Some(ControlMotionContext {
            now,
            reduced_motion,
        });
        self.animation_requested = false;
        self.frame_paragraphs
            .get_or_insert_with(HashMap::new)
            .clear();
        if let Some(stats) = &mut self.paragraph_diagnostic {
            *stats = ParagraphLayoutDiagnostic::default();
        }
    }

    pub fn set_body_size(&mut self, size: f32) -> bool {
        let scale = size / proposal::TextRole::UiBody.style().size;
        if !scale.is_finite() || scale <= 0. || self.type_scale == scale {
            return false;
        }
        self.type_scale = scale;
        if let Some(cache) = &mut self.frame_paragraphs {
            cache.clear();
        }
        if let Some(stats) = &mut self.paragraph_diagnostic {
            stats.unique_entries = 0;
        }
        true
    }
}

impl ButtonTextRenderer for CoreTextHost {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        self.services_enabled.then_some(self)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.services_enabled
            .then_some(self.motion_context)
            .flatten()
    }
    fn request_animation_frame(&mut self) {
        self.animation_requested = true;
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * self.type_scale
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let (paragraph, slot) = self.paragraph_layout(text, role, width);
        Size::new(slot, paragraph.height as f32)
    }
    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, width: f32) -> f32 {
        let (paragraph, _) = self.paragraph_layout(text, role, width);
        paragraph
            .lines
            .first()
            .map_or(paragraph.default_line_height as f32, |line| {
                line.height as f32
            })
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        r: Rect,
        align: ButtonTextAlign,
        color: Rgba,
    ) {
        let (p, _) = self.paragraph_layout(text, role, r.size.width);
        let x = if align == ButtonTextAlign::Center {
            r.min_x() + (r.size.width - trimmed_advance(&p)) * 0.5
        } else {
            r.min_x()
        };
        let y = r.min_y() + (r.size.height - p.height as f32) * 0.5;
        scene.push_clip(r);
        push_paragraph(
            scene,
            &mut self.atlas,
            &mut self.raster,
            &p,
            Point::new(x.round(), y.round()),
            self.scale,
            color,
        );
        scene.pop_clip();
    }
}

impl TextSurfaceTextRenderer for CoreTextHost {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &Paragraph,
        origin: Point,
        color: Rgba,
        clip: Rect,
    ) {
        push_paragraph_visible(
            scene,
            &mut self.atlas,
            &mut self.raster,
            paragraph,
            origin,
            self.scale,
            color,
            clip.min_y()..clip.max_y(),
        );
    }
}

impl TextSurfaceHost for CoreTextHost {
    fn shape_system_text_with_weight(
        &mut self,
        value: &str,
        final_logical_point_size: f32,
        weight: u16,
        width: Option<f32>,
    ) -> Option<Rc<Paragraph>> {
        shape_system_text_with_weight(
            &mut self.shaper,
            value,
            final_logical_point_size,
            weight,
            width,
        )
    }
    fn shape_system_text(
        &mut self,
        value: &str,
        final_logical_point_size: f32,
        width: Option<f32>,
    ) -> Option<Rc<Paragraph>> {
        shape_system_text(&mut self.shaper, value, final_logical_point_size, width)
    }
    fn shape_label(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
        options: LabelOptions,
    ) -> Option<Rc<LabelLayout>> {
        shape_label(
            &mut self.shaper,
            value,
            role,
            self.type_scale,
            width,
            options,
        )
    }
    fn shape_semantic_label(
        &mut self,
        value: &SemanticLabel,
        role: proposal::TextRole,
        width: f32,
        options: LabelOptions,
    ) -> Option<Rc<LabelLayout>> {
        shape_semantic_label(
            &mut self.shaper,
            value,
            role,
            self.type_scale,
            width,
            options,
        )
    }
    fn type_scale(&self) -> f32 {
        self.type_scale
    }
    fn shape_surface(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        shape_surface(&mut self.shaper, value, role, self.type_scale, width)
    }
}
