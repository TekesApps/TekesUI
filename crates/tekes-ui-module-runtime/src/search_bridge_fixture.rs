//! Test-only semantic fixture. Delegates the original field; no Search adapter,
//! command/query model, clear behavior or alternate text owner is implemented.
use tekes_ui_controls::{ButtonTextRenderer, TextSurface, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Rect, Scene, Size};
#[derive(Debug)]
pub struct SearchOwner(pub TextFieldContent);
impl Component for SearchOwner {
    fn semantic_frame(&self, f: Rect) -> Rect {
        self.0.semantic_frame(f)
    }
    fn action_kinds(&self, r: &ContentResponse) -> Vec<ContentActionKind> {
        self.0.action_kinds(r)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.0.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.0.arrange(t, f, c);
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.0.prepare_text_input(t, f, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.0.paint(s, t, f, c, theme);
    }
    fn requires_text_services(&self) -> bool {
        self.0.requires_text_services()
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.0.text_surface()
    }
    fn focusable(&self) -> bool {
        self.0.focusable()
    }
    fn set_focus(&mut self, f: bool, k: bool) {
        self.0.set_focus(f, k);
    }
    fn composing(&self) -> bool {
        self.0.composing()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = self.0.semantics();
        s.role = ContentRole::SearchInput;
        s
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.0.event(e)
    }
    fn key_event(&mut self, e: &ContentKeyEvent) -> ContentResponse {
        self.0.key_event(e)
    }
    fn pointer_event(&mut self, e: &PointerEvent) -> ContentResponse {
        self.0.pointer_event(e)
    }
    fn accessibility_action(
        &mut self,
        p: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        self.0.accessibility_action(p, a)
    }
}
