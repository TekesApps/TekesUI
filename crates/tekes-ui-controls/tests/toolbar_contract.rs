//! Real public ContentHost consumer: no pre-click to seed a toolbar focus index.
use tekes_ui_controls::{composition::*, *};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::new(s.len() as f32 * 8., 16.)
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
fn render(host: &mut ContentHost, width: f32) {
    let frame = Rect::new(0., 0., width, 48.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn event(host: &mut ContentHost, key: ControlKey, down: bool) -> ContentResponse {
    host.event(&ContentEvent::KeyInput(ContentKeyEvent {
        key: Some(key),
        down,
        repeat: false,
        modifiers: Default::default(),
        native_code: 0,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
    }))
}
fn host() -> ContentHost {
    let mut toolbar = ToolbarControl::default();
    toolbar.set_viewport(Rect::new(0., 0., 600., 400.));
    toolbar.set_actions(vec![
        ToolbarAction {
            id: 1,
            title: "First".into(),
            enabled: true,
        },
        ToolbarAction {
            id: 2,
            title: "Disabled".into(),
            enabled: false,
        },
        ToolbarAction {
            id: 3,
            title: "Last".into(),
            enabled: true,
        },
    ]);
    let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
    host.set_action_events(true);
    render(&mut host, 600.);
    host
}
fn take(host: &mut ContentHost) -> Option<u64> {
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .take_action()
}
#[test]
fn tab_entry_space_and_roving_keys_work_without_pointer_and_skip_disabled() {
    let mut host = host();
    event(&mut host, ControlKey::Tab { reverse: false }, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
    event(&mut host, ControlKey::Right, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), Some(3));
    assert_eq!(host.drain_actions().len(), 1);
    event(&mut host, ControlKey::Home, true);
    event(&mut host, ControlKey::Enter, true);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
    event(&mut host, ControlKey::End, true);
    event(&mut host, ControlKey::Enter, true);
    assert_eq!(take(&mut host), Some(3));
    assert_eq!(host.drain_actions().len(), 1);
}
#[test]
fn changing_focus_cancels_old_space_arm_and_disabled_registration_is_not_activated() {
    let mut host = host();
    event(&mut host, ControlKey::Tab { reverse: false }, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Right, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), None);
    assert!(host.drain_actions().is_empty());
    let toolbar = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap();
    toolbar.set_actions(vec![
        ToolbarAction {
            id: 1,
            title: "First".into(),
            enabled: true,
        },
        ToolbarAction {
            id: 2,
            title: "Disabled".into(),
            enabled: false,
        },
        ToolbarAction {
            id: 3,
            title: "Last".into(),
            enabled: false,
        },
    ]);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
}
#[test]
fn narrow_overflow_is_keyboard_reachable_and_opening_is_not_an_action() {
    let mut host = host();
    render(&mut host, 60.);
    event(&mut host, ControlKey::Tab { reverse: false }, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap()
            .is_open()
    );
    assert!(host.drain_actions().is_empty());
    assert_eq!(take(&mut host), None);
    event(&mut host, ControlKey::Escape, true);
    assert!(
        !host
            .get(ComponentId(1))
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap()
            .is_open()
    );
    render(&mut host, 600.);
    event(&mut host, ControlKey::Enter, true);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
}

#[cfg(feature = "token-audit")]
#[test]
fn original_toolbar_component_states_observe_real_token_dependencies() {
    let mut cases = Vec::new();
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for state in ["rest", "hover", "pressed", "focus", "disabled", "overflow"] {
                let mut host = host();
                let width = if state == "overflow" { 60. } else { 600. };
                render(&mut host, width);
                match state {
                    "focus" => {
                        event(&mut host, ControlKey::Tab { reverse: false }, true);
                    }
                    "pressed" => {
                        event(&mut host, ControlKey::Tab { reverse: false }, true);
                        assert!(event(&mut host, ControlKey::Space, true).consumed);
                    }
                    "hover" => {
                        host.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
                            24., 24.,
                        )));
                    }
                    "disabled" => {
                        let t = host
                            .get_mut(ComponentId(1))
                            .unwrap()
                            .component_mut::<ToolbarControl>()
                            .unwrap();
                        t.set_actions(vec![
                            ToolbarAction {
                                id: 1,
                                title: "First".into(),
                                enabled: false,
                            },
                            ToolbarAction {
                                id: 2,
                                title: "Disabled".into(),
                                enabled: false,
                            },
                            ToolbarAction {
                                id: 3,
                                title: "Last".into(),
                                enabled: false,
                            },
                        ]);
                        assert!(!t.focusable());
                    }
                    "overflow" => {
                        event(&mut host, ControlKey::Tab { reverse: false }, true);
                        event(&mut host, ControlKey::Space, true);
                        event(&mut host, ControlKey::Space, false);
                        assert!(
                            host.get(ComponentId(1))
                                .unwrap()
                                .component::<ToolbarControl>()
                                .unwrap()
                                .is_open()
                        );
                    }
                    _ => {}
                }
                let frame = Rect::new(0., 0., width, 48.);
                let (_, reads) = tekes_ui_tokens::audit::capture(|| {
                    host.render(
                        &mut Scene::new(),
                        &mut Text,
                        frame,
                        frame,
                        tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                    )
                    .unwrap()
                });
                assert!(!reads.is_empty());
                if state == "pressed" {
                    event(&mut host, ControlKey::Space, false);
                    assert_eq!(take(&mut host), Some(1));
                    assert_eq!(host.drain_actions().len(), 1);
                } else {
                    assert!(host.drain_actions().is_empty());
                }
                cases.push(serde_json::json!({"component":"tui.toolbar","action_state":state,"state_asserted":true,"theme":theme_index,"brand":brand,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 72);
    if let Some(path) = std::env::var_os("TEKESUI_TOOLBAR_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases,"scope":"Actual ToolbarControl in original ContentHost: six states, four themes, three existing brands. Reuses ButtonContent and Menu; legacy ToolbarView is not exercised by this probe.","excluded":["legacy ToolbarView runtime","arbitrary caller Content slots","font backend and icon texture pixels","complete state combinations or dependency extraction","native/VoiceOver/Gold7 experience","performance"]})).unwrap()).unwrap();
    }
}
