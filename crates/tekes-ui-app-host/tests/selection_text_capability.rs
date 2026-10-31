#![cfg(target_os = "macos")]
#[path = "../../../examples/composition-consumer/tests/support/coretext.rs"]
mod coretext;
// AppHost now consumes the platform snapshot directly (window_accessibility.rs).
use tekes_ui_controls::{ButtonTextRenderer, TextSurface, composition::*, layout::MeasuredChild};
use tekes_ui_platform_macos::{AccessibilityRole, content_accessibility as platform};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::Theme;

#[derive(Debug)]
struct SelectionField {
    field: TextFieldContent,
    role: ContentRole,
}
impl Component for SelectionField {
    fn semantic_frame(&self, f: Rect) -> Rect {
        self.field.semantic_frame(f)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.field.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.field.arrange(t, f, c);
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.field.prepare_text_input(t, f, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.field.paint(s, t, f, c, theme);
    }
    fn requires_text_services(&self) -> bool {
        self.field.requires_text_services()
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.field.text_surface()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(self.role, "Choice");
        if self.role == ContentRole::Checkbox {
            s.value = Some("mixed".into());
        }
        if matches!(self.role, ContentRole::RadioButton | ContentRole::Segment) {
            s.selected = true;
        }
        s
    }
}
#[derive(Debug)]
struct SelectionParent {
    role: ContentRole,
    children: Vec<Content>,
}
impl Component for SelectionParent {
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        for child in &mut self.children {
            child.arrange(t, f, c);
        }
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(self.role, "Choices")
    }
}

fn host(role: ContentRole) -> (ContentHost, ComponentId) {
    let id = ComponentId(107);
    let mut field = TextFieldContent::new("Choice", "A😀B");
    field.surface.input.select_range(1..3);
    let choice = Content::new(id, SelectionField { field, role });
    let root = match role {
        ContentRole::RadioButton => Content::new(
            ComponentId(106),
            SelectionParent {
                role: ContentRole::RadioGroup,
                children: vec![choice],
            },
        ),
        ContentRole::Segment => Content::new(
            ComponentId(105),
            SelectionParent {
                role: ContentRole::SegmentedGroup,
                children: vec![choice],
            },
        ),
        _ => choice,
    };
    (ContentHost::new(root).unwrap(), id)
}

#[test]
fn original_coretext_surface_cannot_turn_selection_roles_into_text_owners_in_public_projections() {
    for (role, expected) in [
        (ContentRole::Checkbox, AccessibilityRole::Checkbox),
        (ContentRole::RadioGroup, AccessibilityRole::RadioGroup),
        (ContentRole::RadioButton, AccessibilityRole::RadioButton),
        (ContentRole::SegmentedGroup, AccessibilityRole::RadioGroup),
        (ContentRole::Segment, AccessibilityRole::RadioButton),
    ] {
        let (mut h, id) = host(role);
        let frame = Rect::new(0., 0., 300., 100.);
        let mut text = coretext::Text::new(1.);
        text.services = true;
        h.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let original = h.get(id).unwrap().component::<SelectionField>().unwrap();
        assert!(
            original.field.text_surface().is_some(),
            "the same original owner retains a real TextSurface for {role:?}"
        );
        assert!(!original.field.surface.input.selection().is_empty());
        assert!(
            h.text_surface(id).is_none(),
            "ContentHost keeps selection roles out of text capability queries"
        );
        assert!(h.semantic_text_input_snapshot(id).is_none());
        assert!(h.semantic_text_layout_snapshot(id, None).is_none());

        let app_snapshot = platform::ContentAccessibilitySnapshot::from_host(&h).unwrap();
        let app_nodes = app_snapshot.nodes().to_vec();
        let native_nodes = platform::content_nodes(&h);
        assert_eq!(app_nodes, native_nodes);
        let key = platform::content_key(&h, id, None).unwrap();
        let node = app_nodes.iter().find(|n| n.key == key).unwrap();
        assert_eq!(node.role, expected);
        assert!(node.text_selection.is_none());
        assert!(platform::content_text_layouts(&h).is_empty());
        // AppHost's current public snapshot validates the same node boundary,
        // plus the original geometry lease and its text-layout identities.
        assert!(app_snapshot.is_valid());
        assert!(app_snapshot.text_layouts().is_empty());
    }
}
