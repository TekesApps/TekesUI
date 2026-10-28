#![cfg(feature = "token-audit")]
use tekes_ui_composition_consumer::popover_content as p;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, composition::*, extended::Popover,
    feedback_view::PopoverView,
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
fn popover_shell_and_owned_subtree_have_separate_actual_token_observations() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for width in [360., 720.] {
                for state in [
                    "open",
                    "editor-focus",
                    "switch-focus",
                    "dismiss-focus",
                    "selected-switch",
                    "marked-editor",
                    "exit",
                    "reduced-open",
                ] {
                    let mut fixture = p::Scenario::new().unwrap();
                    let mut text = Text {
                        millis: 0,
                        reduced: state == "reduced-open",
                    };
                    let frame = Rect::new(0., 0., width, 640.);
                    let palette = tekes_ui_tokens::Palette::new(theme, brand)
                        .expect("registered Gallery brand");
                    fixture
                        .host
                        .render(&mut Scene::new(), &mut text, frame, frame, palette)
                        .unwrap();
                    fixture
                        .host
                        .accessibility_action(p::CALLER, None, AccessibilityAction::Press);
                    fixture.process();
                    fixture
                        .host
                        .render(&mut Scene::new(), &mut text, frame, frame, palette)
                        .unwrap();
                    text.millis = 200;
                    fixture
                        .host
                        .render(&mut Scene::new(), &mut text, frame, frame, palette)
                        .unwrap();
                    let anchor = fixture.host.get(p::CALLER).unwrap().frame();
                    let size = fixture
                        .host
                        .semantics()
                        .into_iter()
                        .find(|n| n.id == p::POPOVER)
                        .unwrap()
                        .frame
                        .size;
                    let body_size = fixture.host.get(p::BODY).unwrap().frame().size;
                    text.millis = 0;
                    let mut shell = PopoverView::default();
                    let mut model = Popover::default();
                    shell.show(&mut model, 1);
                    shell.render(
                        &mut Scene::new(),
                        &mut text,
                        &model,
                        anchor,
                        size,
                        frame,
                        Some(frame),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    text.millis = 200;
                    shell.render(
                        &mut Scene::new(),
                        &mut text,
                        &model,
                        anchor,
                        size,
                        frame,
                        Some(frame),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    match state {
                        "switch-focus" => {
                            fixture.host.focus(Some(p::SWITCH), true);
                        }
                        "dismiss-focus" => {
                            fixture.host.focus(Some(p::POPOVER), true);
                            shell.focus_dismiss(true, true);
                        }
                        "selected-switch" => {
                            fixture.host.accessibility_action(
                                p::SWITCH,
                                None,
                                AccessibilityAction::Press,
                            );
                            assert!(
                                fixture
                                    .host
                                    .get(p::SWITCH)
                                    .unwrap()
                                    .component::<SwitchContent>()
                                    .unwrap()
                                    .model
                                    .checked
                            );
                        }
                        "marked-editor" => {
                            fixture.host.event(&ContentEvent::Preedit {
                                text: "ni".into(),
                                selection: 2..2,
                                replacement: None,
                                clauses: vec![],
                            });
                            assert!(fixture.host.focused_text().unwrap().input.marked.is_some());
                        }
                        "exit" => {
                            fixture.escape();
                            shell.close(&mut model);
                        }
                        _ => {}
                    }
                    text.millis = 220;
                    let mut registered_scene = Scene::new();
                    fixture
                        .host
                        .render(&mut registered_scene, &mut text, frame, frame, palette)
                        .unwrap();
                    let popup = fixture.host.get(p::POPOVER).unwrap();
                    let popup_frame = popup.component::<PopoverContent>().unwrap().layout().frame;
                    assert!(
                        registered_scene.primitives().iter().any(|p| matches!(p,
                        tekes_ui_scene::Primitive::Quad(q) if q.rect == popup_frame)),
                        "registered host must actually draw the current open or exiting shell"
                    );
                    // Direct scoped observation must use the same declared Surface
                    // as the registered owner. Switching to canvas while closing
                    // correctly invalidates retained paint; that is not an exit bug.
                    let surface = popup
                        .surface()
                        .or_else(|| {
                            popup
                                .component::<PopoverContent>()
                                .unwrap()
                                .default_surface()
                        })
                        .unwrap();
                    let overlay_theme = ContentTheme::from(palette).with_surface(surface).unwrap();
                    assert_eq!(fixture.is_open(), state != "exit");
                    if state == "exit" {
                        assert!(fixture.is_presented());
                        assert!(fixture.host.text_surface(p::EDITOR).is_none());
                    }
                    if state == "switch-focus" {
                        assert_eq!(fixture.host.focused(), Some(p::SWITCH));
                    }
                    if state == "dismiss-focus" {
                        assert_eq!(fixture.host.focused(), Some(p::POPOVER));
                    }
                    if state == "editor-focus" {
                        assert_eq!(fixture.host.focused(), Some(p::EDITOR));
                    }
                    let (_, shell_reads) = tekes_ui_tokens::audit::capture(|| {
                        PopoverView::measure(&mut text, body_size, proposal::TextRole::UiBody);
                        shell.render(
                            &mut Scene::new(),
                            &mut text,
                            &model,
                            anchor,
                            size,
                            frame,
                            Some(frame),
                            palette,
                            proposal::TextRole::UiBody,
                        );
                    });
                    let (_, tree_reads) = tekes_ui_tokens::audit::capture(|| {
                        let mut scene = Scene::new();
                        // Audit this same registered shell/body only. ContentHost
                        // normally invokes this original component in opening order.
                        let frame = fixture.host.get(p::POPOVER).unwrap().frame();
                        fixture
                            .host
                            .get_mut(p::POPOVER)
                            .unwrap()
                            .component_mut::<PopoverContent>()
                            .unwrap()
                            .paint_overlay(&mut scene, &mut text, frame, frame, overlay_theme);
                        assert!(!scene.is_empty());
                    });
                    assert!(!shell_reads.is_empty() && !tree_reads.is_empty());
                    for (component, layer, reads) in [
                        ("tui.popover", "production-shell", shell_reads),
                        (
                            "consumer.popover-owned",
                            "owned-subtree-including-children",
                            tree_reads,
                        ),
                    ] {
                        cases.push(serde_json::json!({"component":component,"layer":layer,"popover_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"width":width,"lookups":reads}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 384);
    if let Ok(path) = std::env::var("TEKESUI_POPOVER_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Production PopoverView shell lookups separate from consumer-defined owned subtree including Caption/TextField/Switch.8bounded states,four themes/three brands/two widths; real CoreText/no-op glyph painter. No child reads attributed to primitive shell.","excluded":["complete alias/dependency closure","constants/nonlookup paths","unexecuted states/child families","native/VoiceOver","performance"],"cases":cases})).unwrap()).unwrap();
    }
}
