//! Delegation to one original CompoundButton, with business capture only.
use tekes_ui_controls::{
    ButtonFramePolicy, ButtonTextRenderer, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
pub const ROOT: ComponentId = ComponentId(15000);
pub const DIRECT: ComponentId = ComponentId(15001);
pub const WRAPPED_RELEASE: ComponentId = ComponentId(15002);
pub const WRAPPED_PRESS: ComponentId = ComponentId(15003);
pub const AFTER: ComponentId = ComponentId(15004);
pub const NORMAL: ComponentId = ComponentId(15005);
#[derive(Debug)]
struct Decoration;
impl Component for Decoration {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(40., 2.))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        f: Rect,
        _: Rect,
        t: ContentTheme,
    ) {
        let ink = t
            .foreground
            .unwrap_or_else(|| t.palette.color("primary-text").unwrap());
        s.push_quad(Quad::new(
            Rect::new(f.min_x() + 8., f.center().y - 1., 24., 2.),
            ink,
        ));
    }
}
#[derive(Debug)]
pub struct ForwardedAction {
    pub button: Button,
    pub captures: usize,
}
impl ForwardedAction {
    fn capture(&mut self, r: ContentResponse) -> ContentResponse {
        if r.activated {
            self.captures += 1;
        }
        r
    }
}
impl Component for ForwardedAction {
    fn focus_decoration_above_siblings(&self) -> bool {
        self.button.focus_decoration_above_siblings()
    }
    fn children(&self) -> &[Content] {
        self.button.children()
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.button.children_mut()
    }
    fn fixed_slots(&self) -> bool {
        self.button.fixed_slots()
    }
    fn validate(&self) -> Result<(), CompositionError> {
        self.button.validate()
    }
    fn child_theme(&self, t: ContentTheme) -> ContentTheme {
        self.button.child_theme(t)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.button.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.button.arrange(t, f, c)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        p: ContentTheme,
    ) {
        self.button.paint(s, t, f, c, p)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let r = self.button.event(e);
        self.capture(r)
    }
    fn accessibility_action(
        &mut self,
        p: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        let r = self.button.accessibility_action(p, a);
        self.capture(r)
    }
    fn focusable(&self) -> bool {
        self.button.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.button.set_focus(f, v)
    }
    fn semantics(&self) -> ContentSemantics {
        self.button.semantics()
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        self.button.lifecycle(e)
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
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        for c in &mut self.children {
            c.measure(t, a);
        }
        MeasuredChild::new(Size::new(a.width, 120.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        for (i, child) in self.children.iter_mut().enumerate() {
            let (y, h) = if i < 4 {
                (i as f32 * 10., 10.)
            } else {
                (70., 36.)
            };
            child.arrange(t, Rect::new(f.min_x(), f.min_y() + y, f.size.width, h), c);
        }
    }
}
pub fn action(h: &ContentHost, id: ComponentId) -> &Button {
    let n = h.get(id).unwrap();
    n.component::<Button>()
        .or_else(|| n.component::<ForwardedAction>().map(|a| &a.button))
        .unwrap()
}
pub fn action_mut(h: &mut ContentHost, id: ComponentId) -> &mut Button {
    let n = h.get_mut(id).unwrap();
    if n.component::<Button>().is_some() {
        n.component_mut().unwrap()
    } else {
        &mut n.component_mut::<ForwardedAction>().unwrap().button
    }
}
pub fn build() -> ContentHost {
    let mut children = Vec::new();
    for (id, label, wrapped, phase) in [
        (DIRECT, "Direct row", false, ButtonActivationPhase::Release),
        (
            WRAPPED_RELEASE,
            "Wrapped release row",
            true,
            ButtonActivationPhase::Release,
        ),
        (
            WRAPPED_PRESS,
            "Wrapped press row",
            true,
            ButtonActivationPhase::Press,
        ),
        (
            AFTER,
            "Following row",
            false,
            ButtonActivationPhase::Release,
        ),
    ] {
        let mut b =
            Button::new(Content::new(ComponentId(id.0 + 100), Decoration)).with_accessible_name(label);
        b.button.frame_policy = ButtonFramePolicy::Exact;
        b.pointer_activation_phase = phase;
        children.push(if wrapped {
            Content::new(
                id,
                ForwardedAction {
                    button: b,
                    captures: 0,
                },
            )
        } else {
            Content::new(id, b)
        });
    }
    children.push(Content::new(NORMAL, Button::new("Ordinary action")));
    let mut h = ContentHost::new(Content::new(ROOT, Rows { children })).unwrap();
    h.set_action_events(true);
    h
}
