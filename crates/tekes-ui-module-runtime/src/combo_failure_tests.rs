use super::*;
use crate::{component::content as wire, native_content::semantic_test_snapshot};
use tekes_ui_controls::composition::*;
fn node(id: u64) -> wire::SemanticNode {
    let target = wire::Target {
        id,
        mount: id + 100,
    };
    let input = wire::TextState {
        owner: target,
        text: "A😀B".into(),
        caret_index: 3,
        selection: 1..3,
        marked: None,
        read_only: true,
        caret: [0., 0., 1., 20.],
        clip: [0., 0., 100., 20.],
        carets: vec![],
    };
    wire::SemanticNode {
        target,
        parent: None,
        frame: [0., 0., 200., 40.],
        visible: Some([0., 0., 200., 40.]),
        focused: false,
        semantics: wire::Semantics {
            role: wire::SemanticRole::ComboBox,
            name: "Query".into(),
            value: Some(input.text.clone()),
            disabled: false,
            selected: false,
            expanded: None,
            invalid: false,
            required: false,
            description: None,
        },
        preserves_activation_focus: false,
        parts: vec![],
        value_range: None,
        text_binding: Some(vec![1]),
        text: Some(input.clone()),
        text_layout: Some(wire::TextLayout {
            binding: vec![1],
            input,
            multiline: false,
            lines: std::iter::once(0..4).collect(),
            carets: vec![(0, [0., 0., 1., 20.]), (4, [30., 0., 1., 20.])],
        }),
    }
}
fn frame() -> Rect {
    Rect::new(0., 0., 200., 80.)
}
fn owner() -> ProcessContainer {
    ProcessContainer::new_v1("unstarted-search-contract", 1, || {}).unwrap()
}
fn current(c: &ProcessContainer) -> usize {
    c.semantic_projection.parts(frame(), frame(), false)[0].index
}
fn denied(c: &ProcessContainer, a: usize) {
    assert!(c.semantic_projection.needs_refresh());
    assert!(
        c.semantic_projection
            .parts(frame(), frame(), false)
            .is_empty()
    );
    assert!(c.semantic_projection.callback(a).is_none());
    assert!(
        c.semantic_projection
            .text_layout(a, frame(), frame())
            .is_none()
    );
}
#[test]
fn actual_accept_content_keeps_original_revoke_when_new_content_contains_malformed_semantics() {
    let mut c = owner();
    c.accept_content(semantic_test_snapshot(vec![node(1)]));
    let a = current(&c);
    assert!(c.semantic_projection.callback(a).is_some());
    assert!(
        c.semantic_projection
            .text_layout(a, frame(), frame())
            .is_some()
    );
    let mut invalid = node(3);
    invalid.frame[2] = -1.;
    let b = semantic_test_snapshot(vec![node(1), node(2), invalid]);
    // Preserve original owner behavior: accept the child content response,
    // but revoke invalid semantic authority. No valid prefix or old A is usable.
    assert!(c.accept_content(b.clone()).changed);
    assert!(c.error.is_some());
    denied(&c, a);
    assert_eq!(c.content_snapshot.as_ref().unwrap().reply(), b.reply());
    c.accept_content(semantic_test_snapshot(vec![node(1)]));
    let recovered = current(&c);
    assert!(recovered > a);
    assert!(c.semantic_projection.callback(a).is_none());
    assert!(c.semantic_projection.callback(recovered).is_some());
}
#[test]
fn actual_old_schema_decode_error_enters_original_request_owner_failure_fence() {
    let mut c = owner();
    c.accept_content(semantic_test_snapshot(vec![node(1)]));
    let a = current(&c);
    let encoded =
        serde_json::to_vec(semantic_test_snapshot(vec![node(1), node(2)]).reply()).unwrap();
    let error = serde_json::from_slice::<super::old_combo_schema::Reply>(&encoded).unwrap_err();
    assert!(error.to_string().contains("unknown variant `ComboBox`"));
    assert_eq!(c.content_failure(error.into()), ContentResponse::default());
    assert!(c.error.as_ref().unwrap().contains("ComboBox"));
    denied(&c, a);
    c.accept_content(semantic_test_snapshot(vec![node(1)]));
    let recovered = current(&c);
    assert!(recovered > a);
    assert!(c.semantic_projection.callback(a).is_none());
}
#[test]
fn actual_hide_deactivate_cancel_request_failure_and_unmount_keep_old_callbacks_revoked() {
    for mode in [
        "hide",
        "deactivate",
        "cancel",
        "disconnect-request-failure",
        "unmounted",
    ] {
        let mut c = owner();
        c.accept_content(semantic_test_snapshot(vec![node(1)]));
        let a = current(&c);
        match mode {
            "hide" => c.set_visible(false).unwrap(),
            "deactivate" => c.set_window_active(false),
            "cancel" => c.cancel(),
            // The current request owner uses content_failure; the historical
            // retire_process_state helper is not reintroduced for a test.
            "disconnect-request-failure" => {
                c.content_failure(anyhow::anyhow!("child disconnected"));
            }
            "unmounted" => c.lifecycle(Lifecycle::Unmounted),
            _ => unreachable!(),
        }
        denied(&c, a);
        if mode == "hide" {
            c.set_visible(true).unwrap();
        }
        if mode == "deactivate" {
            c.set_window_active(true);
        }
        denied(&c, a);
        c.accept_content(semantic_test_snapshot(vec![node(1)]));
        let recovered = current(&c);
        assert!(recovered > a, "{mode}");
        assert!(c.semantic_projection.callback(a).is_none());
        assert!(c.semantic_projection.callback(recovered).is_some());
    }
}
#[derive(Debug)]
struct LocalOnlyContainer(ProcessContainer);
// Test-only lifecycle harness avoids spawning a worker; failures below call
// the actual owner methods. It supplies no alternate failure/editor engine.
impl Component for LocalOnlyContainer {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width, 40.))
    }
}
#[test]
fn actual_owner_failures_leave_same_host_neighbor_composer_focus_marked_text_and_binding_untouched()
{
    let container = ComponentId(101);
    let composer = ComponentId(102);
    let mut host = ContentHost::new(Content::new(
        ComponentId(100),
        ListContent::new(
            "Root",
            vec![
                Content::new(container, LocalOnlyContainer(owner())),
                Content::new(composer, TextFieldContent::new("Composer", "draft")),
            ],
        ),
    ))
    .unwrap();
    let mut text = crate::content_accessibility::combo_coretext::Text::new(1.);
    text.services = true;
    let f = Rect::new(0., 0., 320., 180.);
    host.render(
        &mut Scene::new(),
        &mut text,
        f,
        f,
        tekes_ui_tokens::Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(host.focus(Some(composer), false));
    host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    let before = host.text_input_snapshot().unwrap();
    assert!(before.marked.is_some());
    let epoch = host.focus_epoch();
    let binding = host.native_key_binding();
    {
        let c = &mut host
            .get_mut(container)
            .unwrap()
            .component_mut::<LocalOnlyContainer>()
            .unwrap()
            .0;
        c.accept_content(semantic_test_snapshot(vec![node(1)]));
        let a = current(c);
        let mut invalid = node(3);
        invalid.frame[2] = -1.;
        c.accept_content(semantic_test_snapshot(vec![node(1), node(2), invalid]));
        denied(c, a);
        let encoded = serde_json::to_vec(semantic_test_snapshot(vec![node(1)]).reply()).unwrap();
        let error = serde_json::from_slice::<super::old_combo_schema::Reply>(&encoded).unwrap_err();
        c.content_failure(error.into());
        c.set_visible(false).unwrap();
        c.set_window_active(false);
        c.content_failure(anyhow::anyhow!("child disconnected"));
    }
    assert_eq!(host.focused(), Some(composer));
    assert_eq!(host.focus_epoch(), epoch);
    assert_eq!(host.native_key_binding(), binding);
    assert_eq!(host.text_input_snapshot().unwrap(), before);
}
