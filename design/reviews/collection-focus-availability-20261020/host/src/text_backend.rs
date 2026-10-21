use std::rc::Rc;
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, TextSurfaceHost, TextSurfaceTextRenderer};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size as SSize};
use tekes_ui_text::{coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper}, model::{Paragraph, ShapeOptions}, push::push_paragraph_visible, style::TextStyle, traits::Shaper};
use tekes_ui_tokens::proposal;
struct TextBackend {
    shaper: CoreTextShaper,
    rasterizer: CoreTextRasterizer,
    scale: f32,
    type_scale: f32,
    atlas: GlyphAtlas,
    started: std::time::Instant,
    clock: Option<std::time::Duration>,
    frame_requested: bool,
    reduced_motion: bool,
    caret_on: bool,
}
impl TextBackend {
    fn shape(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        let m = role.style();
        let style = if proposal::text_family(role.id()) == Some("mono") {
            TextStyle::named("Menlo", m.size * self.type_scale)
        } else {
            TextStyle::system(m.size * self.type_scale)
        }
        .with_weight(m.weight);
        Rc::new(self.shaper.shape(
            text,
            &style,
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
    fn push(
        &mut self,
        s: &mut Scene,
        p: &Paragraph,
        o: Point,
        c: tekes_ui_tokens::Rgba,
        clip: Rect,
    ) {
        push_paragraph_visible(
            s,
            &mut self.atlas,
            &mut self.rasterizer,
            p,
            o,
            self.scale,
            c,
            clip.min_y()..clip.max_y(),
        );
    }
}
impl TextSurfaceTextRenderer for TextBackend {
    fn push_visible(
        &mut self,
        s: &mut Scene,
        p: &Paragraph,
        o: Point,
        c: tekes_ui_tokens::Rgba,
        clip: Rect,
    ) {
        self.push(s, p, o, c, clip)
    }
}
impl TextSurfaceHost for TextBackend {
    fn shape_surface(
        &mut self,
        s: &str,
        r: proposal::TextRole,
        w: Option<f32>,
    ) -> Rc<Paragraph> {
        self.shape(s, r, w)
    }
    fn type_scale(&self) -> f32 {
        self.type_scale
    }
    fn caret_visible(&self) -> bool {
        self.caret_on
    }
}
impl ButtonTextRenderer for TextBackend {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> SSize {
        let p = self.shape(s, r, Some(w));
        SSize::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        s: &mut Scene,
        text: &str,
        r: proposal::TextRole,
        rect: Rect,
        a: ButtonTextAlign,
        color: tekes_ui_tokens::Rgba,
    ) {
        let p = self.shape(text, r, Some(rect.size.width));
        let x = match a {
            ButtonTextAlign::Leading => rect.min_x(),
            ButtonTextAlign::Center => rect.min_x() + (rect.size.width - p.width as f32) * 0.5,
        };
        let y = rect.min_y() + (rect.size.height - p.height as f32) * 0.5;
        self.push(s, &p, Point::new(x.round(), y.round()), color, rect)
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: self.clock.unwrap_or_else(|| self.started.elapsed()),
            reduced_motion: self.reduced_motion,
        })
    }
    fn request_animation_frame(&mut self) {
        self.frame_requested = true;
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
}

impl TextBackend {
    fn new(scale: f32, type_scale: f32) -> Self {
        let fonts=CoreTextFonts::new();
        Self { shaper:CoreTextShaper::new(fonts.clone()), rasterizer:CoreTextRasterizer::new(fonts), scale, type_scale, atlas:GlyphAtlas::default(), started:std::time::Instant::now(), clock:None, frame_requested:false, reduced_motion:true, caret_on:true }
    }
}
