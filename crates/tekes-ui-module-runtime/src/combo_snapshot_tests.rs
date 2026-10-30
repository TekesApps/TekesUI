use super::*;
use tekes_ui_controls::composition::*;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;
#[test]
fn public_original_field_exports_combo_role_and_unchanged_owner_text_binding() {
    let id = ComponentId(101);
    let mut field = TextFieldContent::new("Query", "A😀B");
    field.surface.input.read_only = true;
    field.surface.input.select_range(1..3);
    let mut host = ContentHost::new(Content::new(
        id,
        super::combo_fixture::ComboOwner::new(field),
    ))
    .unwrap();
    let mut text = super::combo_coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(0., 0., 240., 80.);
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let before = host.semantic_text_input_snapshot(id).unwrap();
    let nodes = snapshot(&host).unwrap();
    assert_eq!(nodes.len(), 1);
    let node = &nodes[0];
    assert_eq!(node.semantics.role, wire::SemanticRole::ComboBox);
    assert_eq!(node.text_binding.as_ref(), Some(&before.binding));
    assert_eq!(node.target.mount, host.semantic_mount(id).unwrap());
    let t = node.text_layout.as_ref().unwrap();
    assert!(t.is_valid());
    assert!(!t.multiline);
    assert_eq!(t.input.selection, 1..3);
    assert_eq!(t.input.text, "A😀B");
    assert!(t.input.read_only);
    let decoded: Vec<wire::SemanticNode> =
        serde_json::from_slice(&serde_json::to_vec(&nodes).unwrap()).unwrap();
    assert_eq!(decoded, nodes);
    assert_eq!(host.semantic_text_input_snapshot(id).unwrap(), before);
}

#[test]
fn combo_wire_uses_actual_marked_draft_and_original_action_binding() {
    let id = ComponentId(116);
    let mut field = TextFieldContent::new("Combo", "A😀B");
    field.surface.input.select_range(1..3);
    let mut host = ContentHost::new(Content::new(
        id,
        super::combo_fixture::ComboOwner::new(field),
    ))
    .unwrap();
    let mut text = super::combo_coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 240., 80.);
    host.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    host.focus(Some(id), false);
    let key = host.native_key_binding();
    host.event(&ContentEvent::Preedit {
        text: "中😀".into(),
        selection: 1..3,
        clauses: std::iter::once(0..3).collect(),
        replacement: None,
    });
    host.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    let before = host.semantic_text_input_snapshot(id).unwrap();
    assert!(before.marked.is_some());
    let nodes = snapshot(&host).unwrap();
    let n = &nodes[0];
    assert_eq!(n.semantics.role, wire::SemanticRole::ComboBox);
    assert_eq!(n.semantics.value.as_deref(), Some(before.text.as_str()));
    let t = n.text_layout.as_ref().unwrap();
    assert_eq!(t.input.marked, before.marked);
    assert_eq!(t.input.selection, before.selection);
    assert_eq!(t.binding, before.binding);
    assert_eq!(n.text.as_ref().unwrap(), &t.input);
    assert_eq!(n.text_binding.as_ref(), Some(&t.binding));
    assert!(t.is_valid());
    host.event(&ContentEvent::Commit {
        text: "终😀".into(),
        replacement: None,
    });
    let response = action(
        &mut host,
        n.target,
        None,
        Some(&before.binding),
        &wire::AccessibilityAction::SetSelection(1..4),
    )
    .unwrap();
    assert!(response.consumed);
    assert_eq!(host.text_surface(id).unwrap().input.selection(), 1..4);
    assert_eq!(host.text_surface(id).unwrap().input.text, "A终😀B");
    assert_eq!(host.native_key_binding(), key);
    assert!(
        action(
            &mut host,
            n.target,
            None,
            Some(&[0]),
            &wire::AccessibilityAction::SetValue("stale".into())
        )
        .is_err()
    );
    assert_eq!(host.text_surface(id).unwrap().input.text, "A终😀B");
}

#[test]
fn set_expanded_wire_roundtrip_reaches_original_combo_without_toggling() {
    let id = ComponentId(117);
    let mut host = ContentHost::new(Content::new(
        id,
        ComboBoxContent::new("Options", vec!["A".into(), "B".into()]),
    ))
    .unwrap();
    let mut text = super::combo_coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(0., 0., 320., 240.);
    // Repeating an explicit state must not become a second unqualified Press.
    for expanded in [true, true, false, false] {
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let nodes = snapshot(&host).unwrap();
        let owner = nodes.iter().find(|n| n.target.id == id.0).unwrap();
        let request = wire::Input::Accessibility {
            target: owner.target,
            part: None,
            binding: owner.text_binding.clone(),
            action: wire::AccessibilityAction::SetExpanded(expanded),
        };
        let decoded: wire::Input =
            serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        let wire::Input::Accessibility {
            target,
            part,
            binding,
            action: verb,
        } = decoded
        else {
            panic!("expected original accessibility request");
        };
        let open_before = host
            .get(id)
            .unwrap()
            .component::<ComboBoxContent>()
            .unwrap()
            .model()
            .open;
        assert!(action(&mut host, target, part, Some(&[]), &verb).is_err());
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<ComboBoxContent>()
                .unwrap()
                .model()
                .open,
            open_before
        );
        assert!(
            action(&mut host, target, part, binding.as_deref(), &verb)
                .unwrap()
                .consumed
        );
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<ComboBoxContent>()
                .unwrap()
                .model()
                .open,
            expanded
        );
    }
}
