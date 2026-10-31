use super::*;
#[path = "select_bridge_fixture.rs"]
mod fixture;
use tekes_ui_controls::composition::*;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;
#[test]
fn original_select_exports_wire_role_metadata_and_original_option_callbacks() {
    let id = ComponentId(105);
    let mut h = ContentHost::new(Content::new(id, fixture::SelectOwner::prepared())).unwrap();
    let f = Rect::new(0., 0., 300., 240.);
    h.render(
        &mut Scene::new(),
        &mut fixture::Text,
        f,
        f,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let before = (
        h.focused(),
        h.focus_epoch(),
        h.pointer_capture(),
        h.needs_render(),
    );
    let nodes = snapshot(&h).unwrap();
    let n = &nodes[0];
    assert_eq!(n.semantics.role, wire::SemanticRole::Select);
    assert_eq!(n.semantics.value.as_deref(), Some("One"));
    assert_eq!(n.semantics.expanded, Some(true));
    assert!(n.semantics.invalid);
    assert!(n.text.is_none() && n.text_layout.is_none() && n.text_binding.is_none());
    assert_eq!(n.parts.len(), 2);
    assert_eq!(n.parts[0].semantics.role, wire::SemanticRole::MenuItem);
    assert!(n.parts[0].semantics.selected);
    let encoded = serde_json::to_vec(&nodes).unwrap();
    assert_eq!(
        serde_json::from_slice::<Vec<wire::SemanticNode>>(&encoded).unwrap(),
        nodes
    );
    assert_eq!(
        before,
        (
            h.focused(),
            h.focus_epoch(),
            h.pointer_capture(),
            h.needs_render()
        )
    );
    assert!(
        action(
            &mut h,
            n.target,
            Some(1),
            None,
            &wire::AccessibilityAction::Focus
        )
        .unwrap()
        .consumed
    );
    assert_eq!(
        h.get(id)
            .unwrap()
            .component::<fixture::SelectOwner>()
            .unwrap()
            .model
            .selected,
        0
    );
    assert!(
        action(
            &mut h,
            n.target,
            Some(1),
            None,
            &wire::AccessibilityAction::Press
        )
        .unwrap()
        .activated
    );
    assert_eq!(
        h.get(id)
            .unwrap()
            .component::<fixture::SelectOwner>()
            .unwrap()
            .model
            .selected,
        1
    );
    assert!(
        action(
            &mut h,
            n.target,
            Some(1),
            None,
            &wire::AccessibilityAction::Press
        )
        .is_err()
    );
    assert!(
        !action(
            &mut h,
            n.target,
            None,
            None,
            &wire::AccessibilityAction::ReplaceSelectedText("bad".into())
        )
        .unwrap()
        .consumed
    );
}

#[test]
fn original_ax_gate_rejects_select_option_callbacks_before_mutation_while_neighbor_composes() {
    let select = ComponentId(105);
    let composer = ComponentId(106);
    let mut h = ContentHost::new(Content::new(
        ComponentId(100),
        ListContent::new(
            "Root",
            vec![
                Content::new(select, fixture::SelectOwner::prepared()),
                Content::new(composer, TextFieldContent::new("Composer", "draft")),
            ],
        ),
    ))
    .unwrap();
    let f = Rect::new(0., 0., 320., 240.);
    let mut text = super::combo_coretext::Text::new(1.);
    text.services = true;
    h.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    h.focus(Some(composer), true);
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    let before = h.text_input_snapshot().unwrap();
    assert!(before.marked.is_some());
    let binding = h.native_key_binding();
    let epoch = h.focus_epoch();
    let nodes = snapshot(&h).unwrap();
    let n = nodes.iter().find(|n| n.target.id == select.0).unwrap();
    for a in [
        wire::AccessibilityAction::Focus,
        wire::AccessibilityAction::Press,
        wire::AccessibilityAction::SetExpanded(true),
    ] {
        let state = format!(
            "{:?}",
            h.get(select)
                .unwrap()
                .component::<fixture::SelectOwner>()
                .unwrap()
        );
        assert!(
            !action(&mut h, n.target, Some(1), None, &a)
                .unwrap()
                .consumed
        );
        assert_eq!(
            format!(
                "{:?}",
                h.get(select)
                    .unwrap()
                    .component::<fixture::SelectOwner>()
                    .unwrap()
            ),
            state
        );
        assert_eq!(h.text_input_snapshot().unwrap(), before);
        assert_eq!(h.native_key_binding(), binding);
        assert_eq!(h.focus_epoch(), epoch);
    }
}

#[derive(Debug)]
struct MisboundPart(ContentTextLayout);
impl Component for MisboundPart {
    fn measure(
        &mut self,
        _: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        a: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        vec![SemanticPart {
            index: 1,
            frame: Rect::new(0., 0., 200., 40.),
            focused: false,
            semantics: ContentSemantics::new(ContentRole::Select, "Mode"),
        }]
    }
    fn semantic_part_text_layout(&self, p: usize) -> Option<ContentTextLayout> {
        (p == 1).then(|| self.0.clone())
    }
}
#[test]
fn sender_rejects_entire_select_snapshot_with_text_part_instead_of_exporting_binding() {
    let f = Rect::new(0., 0., 320., 240.);
    let mut text = super::combo_coretext::Text::new(1.);
    text.services = true;
    let id = ComponentId(1);
    let mut editor =
        ContentHost::new(Content::new(id, TextFieldContent::new("Editor", "One"))).unwrap();
    editor
        .render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    let layout = editor.semantic_text_layout_snapshot(id, None).unwrap();
    let mut h = ContentHost::new(Content::new(
        ComponentId(100),
        ListContent::new(
            "Root",
            vec![
                Content::new(
                    ComponentId(101),
                    tekes_ui_controls::Button::new("Valid prefix"),
                ),
                Content::new(ComponentId(105), MisboundPart(layout)),
            ],
        ),
    ))
    .unwrap();
    h.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    assert!(snapshot(&h).is_err());
}
