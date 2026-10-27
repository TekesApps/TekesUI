//! Dense caller-owned layout using original CompoundButton actions and chrome.
use tekes_ui_controls::{ButtonFramePolicy, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
pub const ROOT: ComponentId = ComponentId(12000);
pub const RELEASE: ComponentId = ComponentId(12001);
pub const PRESS: ComponentId = ComponentId(12002);
pub const DISABLED: ComponentId = ComponentId(12003);
pub const NORMAL: ComponentId = ComponentId(12004);
#[derive(Debug)]
struct Tick;
impl Component for Tick {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        _: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(40., 2.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        let ink = theme
            .foreground
            .unwrap_or_else(|| theme.palette.color("primary-text").unwrap());
        scene.push_quad(Quad::new(
            Rect::new(frame.min_x() + 8., frame.center().y - 1., 24., 2.),
            ink,
        ));
    }
}
#[derive(Debug)]
struct Rows {
    children: Vec<Content>,
}
impl Component for Rows {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        for c in &mut self.children {
            c.measure(t, a);
        }
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(a.width, 120.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        for (i, child) in self.children.iter_mut().enumerate() {
            let (y, h) = if i < 3 {
                (i as f32 * 10., 10.)
            } else {
                (60., 36.)
            };
            child.arrange(t, Rect::new(f.min_x(), f.min_y() + y, f.size.width, h), c);
        }
    }
}
pub fn build() -> ContentHost {
    let mut children = Vec::new();
    for (id, name, phase) in [
        (RELEASE, "Release row", ButtonActivationPhase::Release),
        (PRESS, "Press row", ButtonActivationPhase::Press),
        (DISABLED, "Disabled row", ButtonActivationPhase::Press),
    ] {
        let mut action =
            Button::new(Content::new(ComponentId(id.0 + 100), Tick)).with_accessible_name(name);
        action.button.frame_policy = ButtonFramePolicy::Exact;
        action.pointer_activation_phase = phase;
        action.button.state.disabled = id == DISABLED;
        children.push(Content::new(id, action));
    }
    children.push(Content::new(NORMAL, Button::new("Ordinary action")));
    let mut host = ContentHost::new(Content::new(ROOT, Rows { children })).unwrap();
    host.set_action_events(true);
    host
}
