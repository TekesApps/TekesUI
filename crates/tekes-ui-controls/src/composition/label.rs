//! Per-node readonly label service adapter. No interaction or semantics ownership.
use super::*;
use std::rc::Rc;
use tekes_ui_text::{LabelLayout, LabelOptions, LabelOverflow};

#[derive(Debug)]
pub(super) struct LabelState {
    options: LabelOptions,
    cached: Option<(String, proposal::TextRole, f32, Rc<LabelLayout>)>,
    pub failed: bool,
}
impl LabelState {
    pub fn new(options: LabelOptions) -> Self {
        Self {
            options,
            cached: None,
            failed: false,
        }
    }
    pub fn reset(&mut self) {
        // A new measure pass may use a new host, font registry or type scale.
        self.cached = None;
        self.failed = false;
    }
}
pub(super) struct LabelRenderer<'a> {
    host: &'a mut dyn ButtonTextRenderer,
    state: &'a mut LabelState,
    clip: Rect,
}
impl<'a> LabelRenderer<'a> {
    pub fn new(
        host: &'a mut dyn ButtonTextRenderer,
        state: &'a mut LabelState,
        clip: Rect,
    ) -> Self {
        Self { host, state, clip }
    }
    pub(super) fn failed(&self) -> bool {
        self.state.failed
    }
    fn layout(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> Option<Rc<LabelLayout>> {
        if let Some((source, cached_role, cached_width, layout)) = &self.state.cached
            && source == text
            && *cached_role == role
            && *cached_width == width
        {
            return Some(layout.clone());
        }
        let result = self
            .host
            .text_surface_host()
            .and_then(|host| host.shape_label(text, role, width, self.state.options));
        if let Some(layout) = &result {
            self.state.cached = Some((text.to_owned(), role, width, layout.clone()));
        } else {
            self.state.failed = true;
            self.state.cached = None;
        }
        result
    }
}
impl ButtonTextRenderer for LabelRenderer<'_> {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        self.layout(text, role, width)
            .map(|layout| {
                // Reuse the existing label service's 1/64pt fit precision.
                // Intrinsic f64 advances must not round downward into an f32
                // constraint, wrap the final grapheme, and trigger ellipsis.
                let measured_width = ((layout.width() * 64.).ceil() / 64.) as f32;
                Size::new(measured_width.min(width), layout.height() as f32)
            })
            .unwrap_or_default()
    }
    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, width: f32) -> f32 {
        self.layout(text, role, width)
            .and_then(|layout| {
                layout
                    .paragraph()
                    .lines
                    .first()
                    .map(|line| line.height as f32)
            })
            .unwrap_or(0.)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        // Use the final measurement object, rather than reshaping at the glyph
        // width (which may be smaller than its original constraint).
        let measured = self
            .state
            .cached
            .as_ref()
            .filter(|(s, r, _, _)| s == text && *r == role)
            .map(|(_, _, _, p)| p.clone());
        let Some(layout) = measured.or_else(|| self.layout(text, role, rect.size.width)) else {
            return;
        };
        let x = match align {
            ButtonTextAlign::Leading => rect.min_x(),
            ButtonTextAlign::Center => {
                rect.min_x() + (rect.size.width - layout.width() as f32).max(0.) * 0.5
            }
        };
        let origin = Point::new(
            x.round(),
            (rect.min_y() + (rect.size.height - layout.height() as f32) * 0.5).round(),
        );
        if let Some(host) = self.host.text_surface_host() {
            let clip = if self.state.options.overflow == LabelOverflow::Clip {
                self.clip.intersection(&rect).unwrap_or_default()
            } else {
                // Ellipsis keeps complete shaped glyph bearings. The original
                // component's Scene clip still enforces its visible outer frame.
                self.clip
            };
            scene.push_clip(clip);
            host.push_visible(scene, layout.paragraph(), origin, ink, clip);
            scene.pop_clip();
        } else {
            self.state.failed = true;
        }
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.host.text_surface_host()
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.host.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.host.request_animation_frame();
    }
    fn backing_scale(&self) -> f32 {
        self.host.backing_scale()
    }
}
