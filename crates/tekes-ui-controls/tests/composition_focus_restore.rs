//! Caller restoration uses the same mounted object, never an ID replacement.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const ROOT: ComponentId = ComponentId(10);
const CALLER: ComponentId = ComponentId(11);
const DIALOG: ComponentId = ComponentId(12);
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), 18.)
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
fn host() -> ContentHost {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(CALLER, Button::new("Original caller")),
                Content::new(
                    DIALOG,
                    DialogContent::new(
                        "Dialog",
                        Content::new(ComponentId(13), TextContent::new("Header")),
                        Content::new(ComponentId(14), Button::new("Body action")),
                        Content::new(ComponentId(15), Button::new("Close")),
                    ),
                ),
            ],
        },
    ))
    .unwrap()
}
fn render(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 640., 400.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
#[test]
fn dialog_restoration_rejects_a_new_caller_with_the_same_component_id() {
    let mut h = host();
    render(&mut h);
    assert!(h.focus(Some(CALLER), true));
    h.show_dialog(DIALOG).unwrap();
    render(&mut h);
    let _original = h.remove(CALLER).unwrap();
    h.insert(
        ROOT,
        Content::new(CALLER, Button::new("Replacement caller")),
    )
    .unwrap();
    assert!(h.close_dialog());
    assert_eq!(
        h.focused(),
        None,
        "the old modal must not focus a replacement with the same ID"
    );
    render(&mut h);
    assert_eq!(h.focused(), None);
    assert!(!h.close_dialog());
    assert!(
        h.focus(Some(CALLER), true),
        "fresh focus of replacement remains available"
    );
}
#[test]
fn dialog_restoration_rejects_an_unmounted_and_remounted_caller() {
    let mut h = host();
    render(&mut h);
    h.focus(Some(CALLER), true);
    h.show_dialog(DIALOG).unwrap();
    render(&mut h);
    let original = h.remove(CALLER).unwrap();
    h.insert(ROOT, original).unwrap();
    assert!(h.close_dialog());
    assert_eq!(h.focused(), None, "remount requires fresh focus intent");
}
#[test]
fn dialog_restoration_keeps_the_same_mount_after_reorder_and_rejects_unavailable_callers() {
    for scenario in 0..4 {
        let mut h = host();
        render(&mut h);
        h.focus(Some(CALLER), true);
        h.show_dialog(DIALOG).unwrap();
        render(&mut h);
        match scenario {
            1 => {
                h.reorder(ROOT, &[DIALOG, CALLER]).unwrap();
            }
            2 => {
                h.get_mut(CALLER)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap()
                    .button
                    .state
                    .disabled = true;
            }
            3 => {
                h.remove(CALLER).unwrap();
            }
            _ => {}
        }
        assert!(h.close_dialog());
        assert_eq!(h.focused(), if scenario < 2 { Some(CALLER) } else { None });
        assert!(!h.close_dialog());
    }
}

#[test]
fn dialog_does_not_capture_an_already_ineligible_programmatic_caller() {
    let mut h = host();
    render(&mut h);
    h.focus(Some(CALLER), true);
    h.get_mut(CALLER)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = true;
    h.show_dialog(DIALOG).unwrap();
    h.get_mut(CALLER)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = false;
    assert!(h.close_dialog());
    assert_eq!(
        h.focused(),
        None,
        "reenabling cannot create return-focus intent absent at modal open"
    );
}

#[test]
fn stale_curved_pointer_input_does_not_block_modal_escape_and_original_caller_restore() {
    for lossless in [false, true] {
        let mut h = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack::new(Axis::Vertical),
                flexible_child: None,
                children: vec![
                    Content::new(CALLER, Button::new("Original caller")),
                    Content::new(
                        DIALOG,
                        DialogContent::new(
                            "Curved body dialog",
                            Content::new(ComponentId(13), TextContent::new("Header")),
                            Content::new(
                                ComponentId(16),
                                PanelContent::new(Content::new(
                                    ComponentId(14),
                                    Button::new("Body action"),
                                ))
                                .unwrap(),
                            ),
                            Content::new(ComponentId(15), Button::new("Close")),
                        ),
                    ),
                ],
            },
        ))
        .unwrap();
        render(&mut h);
        assert!(h.focus(Some(CALLER), true));
        let caller_mount = h.get(CALLER).unwrap().mount_identity();
        h.show_dialog(DIALOG).unwrap();
        render(&mut h);
        let point = h
            .get(ComponentId(14))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .button
            .layout()
            .unwrap()
            .frame
            .center();
        assert_eq!(h.pointer_hit_test(point), Some(ComponentId(14)));
        let owner = h.focused();
        // The public mutable accessor withdraws input authority even when its
        // caller makes no change. Do not render or force focus before Escape.
        let _ = h.get_mut(ComponentId(14)).unwrap();
        assert!(h.needs_render());
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let event = if lossless {
                ContentEvent::Pointer(PointerEvent {
                    position: point,
                    phase,
                    button: Some(PointerButton::Primary),
                    modifiers: PointerModifiers::default(),
                    click_count: 1,
                })
            } else if phase == PointerPhase::Down {
                ContentEvent::PointerDown(point)
            } else {
                ContentEvent::PointerUp(point)
            };
            let response = h.event(&event);
            assert!(!response.capture_pointer && !response.activated);
            assert!(h.pointer_capture().is_none());
            assert_eq!(h.focused(), owner);
        }
        assert_eq!(
            h.get(ComponentId(14))
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            0
        );
        assert!(
            h.event(&ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Escape,
                down: true,
            })
            .consumed
        );
        assert_eq!(h.focused(), Some(CALLER));
        assert_eq!(h.get(CALLER).unwrap().mount_identity(), caller_mount);
        assert!(
            !h.close_dialog(),
            "Escape already closed the original modal"
        );
    }
}
