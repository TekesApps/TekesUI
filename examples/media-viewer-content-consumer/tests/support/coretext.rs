use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, TextSurfaceHost,
    TextSurfaceTextRenderer,
};
use tekes_ui_macos_bridge::coretext_host::CoreTextHost;
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{LabelLayout, LabelOptions, Paragraph};
use tekes_ui_tokens::{Rgba, proposal};

pub struct Text {
    host: CoreTextHost,
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
    pub fn new(scale: f32) -> Self {
        Self {
            host: CoreTextHost::new()
                .with_type_scale(scale)
                .with_backing_scale(2.),
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

    fn sync_host(&mut self) {
        self.host.type_scale = self.scale;
        self.host.scale = self.backing;
        self.host.services_enabled = self.services;
        self.host.motion_context = Some(ControlMotionContext {
            now: self.now,
            reduced_motion: self.reduced_motion,
        });
    }
}

impl ButtonTextRenderer for Text {
    fn request_animation_frame(&mut self) {
        self.requested = true;
    }

    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        if self.services { Some(self) } else { None }
    }

    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.services.then_some(ControlMotionContext {
            now: self.now,
            reduced_motion: self.reduced_motion,
        })
    }

    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        self.sync_host();
        self.host.measure(s, r, w)
    }

    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        self.sync_host();
        self.host.resolved_font_size(r)
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
        self.sync_host();
        std::mem::swap(&mut self.atlas, &mut self.host.atlas);
        self.host.draw(scene, s, r, frame, align, ink);
        std::mem::swap(&mut self.atlas, &mut self.host.atlas);
    }
}

impl TextSurfaceHost for Text {
    fn type_scale(&self) -> f32 {
        self.scale
    }

    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        self.surface_shape_calls.push((s.to_owned(), w));
        self.sync_host();
        self.host.shape_surface(s, r, w)
    }

    fn shape_label(
        &mut self,
        s: &str,
        r: proposal::TextRole,
        w: f32,
        options: LabelOptions,
    ) -> Option<Rc<LabelLayout>> {
        self.sync_host();
        self.host.shape_label(s, r, w, options)
    }
}

impl TextSurfaceTextRenderer for Text {
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
        self.sync_host();
        std::mem::swap(&mut self.atlas, &mut self.host.atlas);
        self.host.push_visible(scene, p, origin, ink, clip);
        std::mem::swap(&mut self.atlas, &mut self.host.atlas);
    }
}
