//! Real shared editor lifecycle; no consumer-owned editor state machine.
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use tekes_ui_controls::{
    TextSurface,
    extended::{Axis, Stack},
};
const ROOT: ComponentId = ComponentId(9600);
const IDS: [ComponentId; 3] = [ComponentId(9601), ComponentId(9602), ComponentId(9603)];
fn build() -> ContentHost {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            flexible_child: None,
            children: IDS
                .into_iter()
                .enumerate()
                .map(|(i, id)| {
                    let mut field =
                        TextFieldContent::new(["TextField", "TextArea", "SearchField"][i], "A😀B");
                    if i == 1 {
                        field.surface = TextSurface::new("A😀B\nTail", true, false);
                        field.viewport = TextViewport::Fixed(96.);
                    }
                    Content::new(id, field)
                })
                .collect(),
        },
    ))
    .unwrap()
}
fn render(host: &mut ContentHost, theme: Theme) {
    let frame = Rect::new(8., 8., 340., 600.);
    host.render(&mut Scene::new(), &mut Text, frame, frame, theme.into())
        .unwrap();
}
#[test]
fn reordered_real_editor_keeps_focus_selection_and_live_preedit_in_each_alias() {
    for theme in Theme::ALL {
        for id in IDS {
            let mut host = build();
            render(&mut host, theme);
            assert!(host.focus(Some(id), true));
            assert!(
                host.accessibility_action(id, None, AccessibilityAction::SetSelection(1..3))
                    .consumed
            );
            host.event(&ContentEvent::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: None,
            });
            let surface = host.text_surface(id).unwrap();
            assert_eq!(surface.input.marked, Some(1..3));
            let before = (
                surface.input.text.clone(),
                surface.input.selection(),
                surface.input.marked.clone(),
                surface.clauses.clone(),
            );
            let identity = host
                .get(id)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap() as *const TextFieldContent;
            host.reorder(ROOT, &[IDS[2], IDS[1], IDS[0]]).unwrap();
            render(&mut host, theme);
            assert_eq!(host.focused(), Some(id));
            let after = host.text_surface(id).unwrap();
            assert_eq!(
                (
                    after.input.text.clone(),
                    after.input.selection(),
                    after.input.marked.clone(),
                    after.clauses.clone()
                ),
                before
            );
            assert_eq!(
                host.get(id)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap() as *const TextFieldContent,
                identity
            );
            host.event(&ContentEvent::Commit {
                text: "你".into(),
                replacement: None,
            });
            assert!(host.text_surface(id).unwrap().input.text.contains("你"));
        }
    }
}
#[test]
fn unmounted_composing_editor_finalizes_visible_draft_and_retains_undo_on_owned_reinsert() {
    for id in IDS {
        let mut host = build();
        render(&mut host, Theme::LIGHT);
        assert!(host.focus(Some(id), true));
        let original = host.text_surface(id).unwrap().input.text.clone();
        host.accessibility_action(id, None, AccessibilityAction::SetSelection(1..3));
        host.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        });
        let visible = host.text_surface(id).unwrap().input.text.clone();
        let identity = host
            .get(id)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap() as *const TextFieldContent;
        let detached = host.remove(id).unwrap();
        assert_eq!(host.focused(), None);
        let field = detached.component::<TextFieldContent>().unwrap();
        assert_eq!(field as *const TextFieldContent, identity);
        assert_eq!(field.surface.input.text, visible);
        assert!(field.surface.input.marked.is_none());
        assert!(field.surface.clauses.is_empty());
        assert!(
            !host
                .accessibility_action(id, None, AccessibilityAction::SetValue("late".into()))
                .consumed
        );
        assert!(
            !host
                .event(&ContentEvent::Commit {
                    text: "late".into(),
                    replacement: None
                })
                .consumed
        );
        host.insert(ROOT, detached).unwrap();
        render(&mut host, Theme::DARK);
        assert_eq!(host.focused(), None, "reinsert does not steal focus");
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap() as *const TextFieldContent,
            identity
        );
        assert_eq!(host.text_surface(id).unwrap().input.text, visible);
        assert!(host.focus(Some(id), true));
        host.event(&ContentEvent::TextCommand("undo:".into()));
        assert_eq!(host.text_surface(id).unwrap().input.text, original);
    }
}
#[test]
fn removing_captured_editable_or_readonly_real_editor_cancels_gesture_without_losing_state() {
    use tekes_ui_scene::Point;
    for id in IDS {
        for readonly in [false, true] {
            let mut host = build();
            host.get_mut(id)
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .read_only = readonly;
            render(&mut host, Theme::LIGHT);
            let s = host.text_surface(id).unwrap();
            let value = s.input.text.clone();
            let p = Point::new(s.inner.min_x() + 8., s.inner.min_y() + 8.);
            host.event(&ContentEvent::PointerDown(p));
            assert!(host.text_surface(id).unwrap().pointer_state(None).pressed);
            let selection = host.text_surface(id).unwrap().input.selection();
            let detached = host.remove(id).unwrap();
            let field = detached.component::<TextFieldContent>().unwrap();
            assert!(!field.surface.pointer_state(None).pressed);
            assert_eq!(field.surface.input.text, value);
            assert_eq!(field.surface.input.selection(), selection);
            assert_eq!(host.focused(), None);
            assert!(!host.event(&ContentEvent::PointerUp(p)).activated);
            host.insert(ROOT, detached).unwrap();
            render(&mut host, Theme::LIGHT);
            let s = host.text_surface(id).unwrap();
            assert_eq!(s.input.text, value);
            assert_eq!(s.input.selection(), selection);
            assert_eq!(s.input.read_only, readonly);
            assert!(!s.pointer_state(None).pressed);
        }
    }
}
