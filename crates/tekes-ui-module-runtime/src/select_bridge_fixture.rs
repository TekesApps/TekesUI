//! Test-only bridge witness. Original Select/View are the only selection/menu owners.
//! Prepares geometry and opens with original pointer input before mounting. This
//! deliberately does not implement #89's adapter, trigger AX open or update seams.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, MenuEvent, Select, composition::*, layout::MeasuredChild,
    layout_view::SelectView,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Debug)]
pub struct SelectOwner {
    pub model: Select,
    pub view: SelectView,
    pub disabled: bool,
    pub invalid: bool,
}
pub struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
impl SelectOwner {
    pub fn prepared() -> Self {
        let mut s = Self {
            model: Select::new(2),
            view: SelectView::new(2),
            disabled: false,
            invalid: true,
        };
        s.render_original();
        let p = Point::new(30., 20.);
        assert!(s.view.pointer_down(p, &mut s.model));
        assert_eq!(s.view.pointer_up(p, &mut s.model), MenuEvent::None);
        assert!(s.model.menu.open);
        s.render_original();
        s
    }
    fn render_original(&mut self) {
        self.view.set_reduced_motion(true);
        self.view.render(
            &mut Scene::new(),
            &mut Text,
            &mut self.model,
            &["One", "Two"],
            Rect::new(0., 0., 200., 40.),
            Rect::new(0., 0., 300., 240.),
            Some(Rect::new(0., 0., 300., 240.)),
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        );
    }
}
impl Component for SelectOwner {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(200., 40.))
    }
    fn focusable(&self) -> bool {
        !self.disabled
    }
    fn set_focus(&mut self, f: bool, k: bool) {
        if f {
            self.view.focus(k)
        } else {
            self.view.blur(&mut self.model)
        }
    }
    fn window_active_changed(&mut self, a: bool) {
        self.view.set_window_active(a, &mut self.model)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if matches!(e, ContentEvent::Cancel) {
            self.view.cancel(&mut self.model);
        }
        ContentResponse::default()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Select, "Mode");
        s.value = Some(["One", "Two"][self.model.selected.min(1)].into());
        s.expanded = Some(self.model.menu.open);
        s.disabled = self.disabled;
        s.invalid = self.invalid;
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.model.menu.open {
            return vec![];
        }
        self.view
            .layout()
            .popup
            .as_ref()
            .into_iter()
            .flat_map(|p| p.items.iter().enumerate())
            .map(|(i, row)| {
                let mut s = ContentSemantics::new(ContentRole::MenuItem, ["One", "Two"][i]);
                s.selected = self.model.selected == i;
                SemanticPart {
                    index: i,
                    frame: row.frame,
                    focused: self.model.menu.items.selected == i,
                    semantics: s,
                }
            })
            .collect()
    }
    fn accessibility_action(
        &mut self,
        p: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        let Some(i) = p else {
            return ContentResponse::default();
        };
        let consumed = match a {
            AccessibilityAction::Focus => self.view.focus_option(i, &mut self.model),
            AccessibilityAction::Press => matches!(
                self.view.activate_option(i, &mut self.model),
                MenuEvent::Activated { .. }
            ),
            _ => false,
        };
        ContentResponse {
            consumed,
            changed: consumed,
            activated: consumed && matches!(a, AccessibilityAction::Press),
            ..Default::default()
        }
    }
}
