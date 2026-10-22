use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlMotionContext};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    push_paragraph,
};
use tekes_ui_tokens::{Rgba, proposal};
pub struct Text {
    pub atlas: GlyphAtlas,
    shaper: CoreTextShaper,
    raster: CoreTextRasterizer,
    pub scale: f32,
    type_scale: f32,
    motion_context: Option<ControlMotionContext>,
    animation_requested: bool,
}
impl Text {
    pub fn new() -> Self {
        let f = CoreTextFonts::new();
        Self {
            atlas: GlyphAtlas::default(),
            shaper: CoreTextShaper::new(f.clone()),
            raster: CoreTextRasterizer::new(f),
            scale: 1.,
            type_scale: 1.,
            motion_context: None,
            animation_requested: false,
        }
    }
    pub fn type_scale(&self) -> f32 {
        self.type_scale
    }
    pub fn begin_frame(&mut self, now: std::time::Duration, reduced_motion: bool) {
        self.motion_context = Some(ControlMotionContext {
            now,
            reduced_motion,
        });
        self.animation_requested = false;
    }
    pub fn animation_requested(&self) -> bool {
        self.animation_requested
    }
    pub fn set_body_size(&mut self, size: f32) -> bool {
        let scale = size / proposal::TextRole::UiBody.style().size;
        if !scale.is_finite() || scale <= 0. || self.type_scale == scale {
            return false;
        }
        self.type_scale = scale;
        true
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.motion_context
    }
    fn request_animation_frame(&mut self) {
        self.animation_requested = true;
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * self.type_scale
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, _: f32) -> Size {
        let style = role.style();
        let p = self.shaper.shape(
            text,
            &TextStyle::system(style.size * self.type_scale).with_weight(style.weight),
            &ShapeOptions::default(),
        );
        Size::new(p.width as f32, p.height as f32)
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
        let style = role.style();
        let p = self.shaper.shape(
            text,
            &TextStyle::system(style.size * self.type_scale).with_weight(style.weight),
            &ShapeOptions::default(),
        );
        let x = if align == ButtonTextAlign::Center {
            r.min_x() + (r.size.width - p.width as f32) * 0.5
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

impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &tekes_ui_text::Paragraph,
        origin: Point,
        color: Rgba,
        clip: Rect,
    ) {
        tekes_ui_text::push_paragraph_visible(
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_label(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        let metrics = role.style();
        Some(std::rc::Rc::new(
            self.shaper
                .shape_label(
                    value,
                    &TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight),
                    &ShapeOptions::with_width(f64::from(width)),
                    options,
                )
                .with_min_line_height(f64::from(metrics.line_height * self.type_scale)),
        ))
    }

    fn type_scale(&self) -> f32 {
        self.type_scale
    }
    fn shape_surface(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        let metrics = role.style();
        let mut paragraph = self.shaper.shape(
            value,
            &TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        );
        let mut top = 0.;
        for line in &mut paragraph.lines {
            let height = f64::from(metrics.line_height * self.type_scale).max(line.height);
            line.top = top;
            line.height = height;
            line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
            top += height;
        }
        paragraph.height = top.max(f64::from(metrics.line_height * self.type_scale));
        paragraph.default_line_height = f64::from(metrics.line_height * self.type_scale);
        std::rc::Rc::new(paragraph)
    }
}
