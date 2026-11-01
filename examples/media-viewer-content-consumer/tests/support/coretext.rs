use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    push::push_paragraph_visible,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Rgba, proposal};
pub struct Text {
    shaper: CoreTextShaper,
    rasterizer: CoreTextRasterizer,
    pub atlas: GlyphAtlas,
    pub scale: f32,
    pub now: std::time::Duration,
    pub services: bool,
    pub draw_text: bool,
    pub surface_shape_calls: Vec<(String, Option<f32>)>,
    pub backing: f32,
    pub requested: bool,
    pub reduced_motion: bool,
}
impl Text {
    fn shape(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Paragraph {
        let m = r.style();
        self.shaper.shape(
            s,
            &TextStyle::system(m.size * self.scale).with_weight(m.weight),
            &ShapeOptions {
                width: Some(w as f64),
                line_spacing: 0.,
            },
        )
    }
}
impl ButtonTextRenderer for Text {
    fn request_animation_frame(&mut self) {
        self.requested = true;
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        if self.services { Some(self) } else { None }
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.services
            .then_some(tekes_ui_controls::ControlMotionContext {
                now: self.now,
                reduced_motion: self.reduced_motion,
            })
    }

    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        let p = self.shape(s, r, w);
        Size::new(p.width as f32, p.height as f32)
    }
    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        r.style().size * self.scale
    }
    fn backing_scale(&self) -> f32 {
        self.backing
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        r: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        if !self.draw_text {
            return;
        }
        let p = self.shape(s, r, frame.size.width);
        let x = frame.min_x()
            + if align == ButtonTextAlign::Center {
                (frame.size.width - p.width as f32) * 0.5
            } else {
                0.
            };
        let y = frame.center().y - p.height as f32 * 0.5;
        push_paragraph_visible(
            scene,
            &mut self.atlas,
            &mut self.rasterizer,
            &p,
            Point::new(x, y),
            self.backing,
            ink,
            frame.min_y()..frame.max_y(),
        );
    }
}
impl Text {
    pub fn new(scale: f32) -> Self {
        let fonts = CoreTextFonts::new();
        Self {
            shaper: CoreTextShaper::new(fonts.clone()),
            rasterizer: CoreTextRasterizer::new(fonts),
            atlas: GlyphAtlas::default(),
            scale,
            now: std::time::Duration::ZERO,
            services: true,
            draw_text: true,
            surface_shape_calls: Vec::new(),
            backing: 2.,
            requested: false,
            reduced_motion: true,
        }
    }
}

impl tekes_ui_controls::TextSurfaceHost for Text {
    fn type_scale(&self) -> f32 {
        self.scale
    }
    fn shape_surface(
        &mut self,
        s: &str,
        r: proposal::TextRole,
        w: Option<f32>,
    ) -> std::rc::Rc<Paragraph> {
        self.surface_shape_calls.push((s.to_owned(), w));
        std::rc::Rc::new(self.shape(s, r, w.unwrap_or(f32::MAX)))
    }
    fn shape_label(
        &mut self,
        s: &str,
        r: proposal::TextRole,
        w: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        let m = r.style();
        Some(std::rc::Rc::new(
            self.shaper
                .shape_label(
                    s,
                    &TextStyle::system(m.size * self.scale).with_weight(m.weight),
                    &ShapeOptions::with_width(w as f64),
                    options,
                )
                .with_min_line_height((m.line_height * self.scale) as f64),
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        p: &Paragraph,
        origin: Point,
        ink: Rgba,
        clip: Rect,
    ) {
        if !self.draw_text {
            return;
        }
        push_paragraph_visible(
            scene,
            &mut self.atlas,
            &mut self.rasterizer,
            p,
            origin,
            self.backing,
            ink,
            clip.min_y()..clip.max_y(),
        );
    }
}
