#![cfg(feature = "token-audit")]
use tekes_ui_composition_consumer::scenarios::{self, ids};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, composition::*,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    millis: u64,
    reduced: bool,
}
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: std::time::Duration::from_millis(self.millis),
            reduced_motion: self.reduced,
        })
    }
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

#[test]
fn actual_menu_content_token_reads_are_bounded_and_state_checked() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for width in [360., 720.] {
                for state in [
                    "rest",
                    "focus",
                    "hover",
                    "pressed",
                    "open",
                    "disabled",
                    "loading",
                    "selected",
                    "remounted",
                    "exit",
                    "tree-root",
                    "tree-submenu",
                ] {
                    let mut host = scenarios::heterogeneous_list().unwrap().host;
                    let mut text = Text {
                        millis: 0,
                        reduced: state != "exit",
                    };
                    let frame = Rect::new(0., 0., width, 640.);
                    let palette = tekes_ui_tokens::Palette::new(theme, brand).expect("registered Gallery brand");
                    host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                        .unwrap();
                    if state.starts_with("tree-") {
                        host.get_mut(ids::LIST_MENU)
                            .unwrap()
                            .component_mut::<MenuContent>()
                            .unwrap()
                            .set_entries(vec![
                                MenuEntry::submenu(
                                    "group",
                                    "Group",
                                    vec![
                                        MenuEntry::Item {
                                            value: "a".into(),
                                            label: "A".into(),
                                            checked: Some(true),
                                            disabled: false,
                                        },
                                        MenuEntry::Item {
                                            value: "b".into(),
                                            label: "B".into(),
                                            checked: Some(false),
                                            disabled: true,
                                        },
                                        MenuEntry::Separator,
                                        MenuEntry::item("c", "C"),
                                    ],
                                ),
                                MenuEntry::Separator,
                                MenuEntry::Item {
                                    value: "disabled".into(),
                                    label: "Disabled".into(),
                                    checked: None,
                                    disabled: true,
                                },
                                MenuEntry::Item {
                                    value: "checked".into(),
                                    label: "Checked".into(),
                                    checked: Some(true),
                                    disabled: false,
                                },
                            ])
                            .unwrap();
                    }
                    let point = host.get(ids::LIST_MENU).unwrap().frame().center();
                    match state {
                        "focus" => {
                            host.focus(Some(ids::LIST_MENU), true);
                        }
                        "hover" => {
                            host.event(&ContentEvent::PointerMove(point));
                        }
                        "pressed" => {
                            host.event(&ContentEvent::PointerDown(point));
                        }
                        "rest" => {}
                        _ => {
                            host.accessibility_action(
                                ids::LIST_MENU,
                                None,
                                AccessibilityAction::Press,
                            );
                            host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                                .unwrap();
                            text.millis = 200;
                            host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                                .unwrap();
                            if state == "tree-submenu" {
                                let part = host
                                    .get(ids::LIST_MENU)
                                    .unwrap()
                                    .component::<MenuContent>()
                                    .unwrap()
                                    .semantic_parts()
                                    .into_iter()
                                    .find(|part| part.semantics.name == "Group")
                                    .unwrap()
                                    .index;
                                assert!(
                                    host.accessibility_action(
                                        ids::LIST_MENU,
                                        Some(part),
                                        AccessibilityAction::Press
                                    )
                                    .consumed
                                );
                                host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                                    .unwrap();
                            }
                            if state == "disabled" || state == "loading" {
                                let menu = host
                                    .get_mut(ids::LIST_MENU)
                                    .unwrap()
                                    .component_mut::<MenuContent>()
                                    .unwrap();
                                menu.trigger.button.state.disabled = state == "disabled";
                                menu.trigger.button.state.loading = state == "loading";
                            } else if matches!(state, "selected" | "remounted" | "exit") {
                                assert!(
                                    host.accessibility_action(
                                        ids::LIST_MENU,
                                        Some(1),
                                        AccessibilityAction::Press
                                    )
                                    .activated
                                );
                                if state == "remounted" {
                                    let menu = host.remove(ids::LIST_MENU).unwrap();
                                    host.insert(ids::LIST, menu).unwrap();
                                }
                            }
                        }
                    }
                    text.millis = 220;
                    host.render(&mut Scene::new(), &mut text, frame, frame, palette)
                        .unwrap();
                    let menu_node = host.get_mut(ids::LIST_MENU).unwrap();
                    let anchor = menu_node.frame();
                    let menu = menu_node.component_mut::<MenuContent>().unwrap();
                    assert_eq!(
                        menu.overlay_open(),
                        matches!(state, "open" | "tree-root" | "tree-submenu")
                    );
                    assert_eq!(
                        menu.semantics().disabled,
                        matches!(state, "disabled" | "loading")
                    );
                    assert_eq!(
                        menu.trigger.button.state.focused,
                        matches!(
                            state,
                            "focus"
                                | "pressed"
                                | "open"
                                | "selected"
                                | "exit"
                                | "tree-root"
                                | "tree-submenu"
                        ),
                        "state={state}"
                    );
                    if state == "hover" {
                        assert!(menu.trigger.button.state.hovered);
                    }
                    if state == "pressed" {
                        assert!(menu.trigger.button.state.pressed);
                    }
                    if matches!(state, "selected" | "remounted" | "exit") {
                        assert_eq!(menu.selected, Some(1));
                    }
                    if state == "remounted" {
                        assert!(!menu.overlay_presented());
                    }
                    if state == "exit" {
                        assert!(menu.overlay_presented());
                    }
                    if state.starts_with("tree-") {
                        let parts = menu.semantic_parts();
                        assert!(
                            parts
                                .iter()
                                .any(|p| p.semantics.name == "Checked" && p.semantics.selected)
                        );
                        assert!(
                            parts
                                .iter()
                                .any(|p| p.semantics.name == "Disabled" && p.semantics.disabled)
                        );
                        assert_eq!(
                            parts
                                .iter()
                                .any(|p| p.semantics.name == "A" && p.semantics.selected),
                            state == "tree-submenu"
                        );
                    }
                    let (_, reads) = tekes_ui_tokens::audit::capture(|| {
                        let mut scene = Scene::new();
                        menu.paint(&mut scene, &mut text, anchor, frame, palette.into());
                        if menu.overlay_presented() {
                            menu.paint_overlay(
                                &mut scene,
                                &mut text,
                                anchor,
                                frame,
                                palette.into(),
                            );
                        }
                        assert!(!scene.is_empty());
                    });
                    assert!(!reads.is_empty());
                    cases.push(serde_json::json!({"component":"tui.menu","menu_content_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"width":width,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 288);
    if let Ok(path) = std::env::var("TEKESUI_MENU_CONTENT_AUDIT_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Actual MenuContent prepared in independent heterogeneous List; capture only its production ButtonContent/MenuView paint, actual CoreText shape and no-op glyph painter.12 bounded states including controlled nested/checked/disabled/separator declarations, four themes/three brands/two widths. No other List child token reads attributed to Menu.","excluded":["complete alias/dependency closure","constants/nonlookup paths","arbitrary decorative trigger child closures","native/VoiceOver","performance"],"cases":cases})).unwrap()).unwrap();
    }
}
