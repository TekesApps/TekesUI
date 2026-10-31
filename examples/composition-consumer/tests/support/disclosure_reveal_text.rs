//! CPU-only geometry/icon-service seam; no pixels/GPU/native evidence.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme};

struct Text {
    now: Option<std::time::Duration>,
}
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.now.map(|now| tekes_ui_controls::ControlMotionContext {
            now,
            reduced_motion: false,
        })
    }
    fn measure(&mut self, _: &str, _: tekes_ui_tokens::proposal::TextRole, width: f32) -> Size {
        Size::new(width.min(100.), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: tekes_ui_tokens::proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
struct Icons;
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("prepared-icon seam required")
    }
}
pub fn render(host: &mut ContentHost) {
    render_clock(host, None);
}
pub fn render_at(host: &mut ContentHost, now: std::time::Duration) {
    render_clock(host, Some(now));
}
fn render_clock(host: &mut ContentHost, now: Option<std::time::Duration>) {
    let frame = Rect::new(0., 0., 400., 300.);
    host.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(&mut Text { now }, &mut Icons),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
