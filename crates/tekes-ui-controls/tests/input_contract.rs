//! Public TextSurface contract using the existing CoreText host and editor
//! transactions. Optional lookup observations are bounded, never native UAT.
use std::rc::Rc;
use tekes_ui_controls::{TextSurface, TextSurfaceHost, TextSurfaceTextRenderer};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct TextHost(CoreTextShaper);
impl TextSurfaceHost for TextHost {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        let style = TextStyle::system(role.style().size).with_weight(role.style().weight);
        Rc::new(self.0.shape(
            text,
            &style,
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for TextHost {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
fn observe<T>(f: impl FnOnce() -> T) -> (T, Vec<(String, String)>) {
    #[cfg(feature = "token-audit")]
    {
        tekes_ui_tokens::audit::capture(f)
    }
    #[cfg(not(feature = "token-audit"))]
    {
        (f(), Vec::new())
    }
}
fn layout(surface: &mut TextSurface, host: &mut TextHost, frame: Rect) {
    let role = surface.role();
    surface.layout_with(frame, None, 1., 1., |s, w| host.shape_surface(s, role, w));
}
#[test]
fn input_states_preserve_draft_selection_geometry_and_editability() {
    let mut host = TextHost(CoreTextShaper::new(CoreTextFonts::new()));
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            let palette = Palette::new(theme, brand).expect("registered Gallery brand");
            for component in ["tui.text-field", "tui.text-area", "tui.search-field"] {
                for state in [
                    "rest",
                    "focus",
                    "disabled",
                    "error",
                    "read-only",
                    "composing",
                    "selection",
                    "disabled-focus",
                    "read-only-selection",
                    "hover",
                    "pressed",
                    "read-only-hover",
                    "read-only-pressed",
                    "disabled-hover",
                    "disabled-pressed",
                    "error-pressed",
                    "composing-hover",
                ] {
                    let (pointer, reads) = observe(|| {
                        let disabled = state.starts_with("disabled");
                        let read_only = state.starts_with("read-only");
                        let focused = !matches!(
                            state,
                            "rest"
                                | "disabled"
                                | "hover"
                                | "read-only-hover"
                                | "disabled-hover"
                                | "disabled-pressed"
                        );
                        let mut surface = TextSurface::new(
                            "A😀e\u{301}中 Sample",
                            component == "tui.text-area",
                            read_only,
                        );
                        surface.input_mut().set_disabled(disabled);
                        surface.set_error(state.starts_with("error"));
                        let frame = Rect::new(
                            16.,
                            18.,
                            280.,
                            if component == "tui.text-area" {
                                140.
                            } else {
                                44.
                            },
                        );
                        layout(&mut surface, &mut host, frame);
                        if matches!(state, "selection" | "read-only-selection") {
                            assert!(surface.input_mut().select_range(2..4));
                            assert_eq!(surface.input().selection(), 1..5);
                            assert_eq!(surface.input().selected_text(), "😀e\u{301}");
                            assert!(!surface.selection_rects(1..5).is_empty());
                        }
                        if state.starts_with("composing") {
                            assert!(surface.input_mut().select_range(1..5));
                            assert!(surface.input_mut().preedit("输入", 2..2, None));
                            surface.set_clauses(vec![0..1, 1..2]);
                            assert_eq!(surface.marked(), Some(1..3));
                            assert!(!surface.input_mut().select_range(0..1));
                        }
                        layout(&mut surface, &mut host, frame);
                        let point = surface.inner().center();
                        let before_hover = (
                            surface.text().to_owned(),
                            surface.input().selection(),
                            surface.marked(),
                        );
                        let hover = surface.pointer_state(Some(point));
                        assert_eq!(hover.hovered, !disabled);
                        assert!(!hover.pressed);
                        assert_eq!(
                            (
                                surface.text().to_owned(),
                                surface.input().selection(),
                                surface.marked()
                            ),
                            before_hover
                        );
                        if state.ends_with("pressed") {
                            assert_eq!(
                                surface.pointer_down(point, false, 1, 24.).is_some(),
                                !disabled
                            );
                        }
                        let pointer = surface.pointer_state(Some(point));
                        assert_eq!(pointer.pressed, state.ends_with("pressed") && !disabled);
                        assert!(!surface.pointer_state(None).hovered);
                        assert_eq!(surface.pointer_state(None).pressed, pointer.pressed);
                        let draft = surface.text().to_owned();
                        let selection = surface.input().selection();
                        let mut blink_on = Scene::new();
                        let mut blink_off = Scene::new();
                        surface.draw(&mut host, &mut blink_on, palette, focused, true);
                        surface.draw(&mut host, &mut blink_off, palette, focused, false);
                        let caret_allowed =
                            focused && !disabled && !read_only && selection.is_empty();
                        assert_eq!(
                            blink_on.primitives().len(),
                            blink_off.primitives().len() + usize::from(caret_allowed),
                            "{component}/{state}"
                        );
                        let boundary = blink_on
                            .primitives()
                            .iter()
                            .find_map(|p| {
                                if let Primitive::Quad(q) = p {
                                    (q.rect == frame).then_some(q)
                                } else {
                                    None
                                }
                            })
                            .unwrap();
                        assert_eq!(
                            boundary.border_color,
                            palette
                                .color(if surface.error() {
                                    "status-failed"
                                } else if focused && !disabled {
                                    "border-focused"
                                } else {
                                    proposal::control_boundary("required-input", theme).unwrap()
                                })
                                .unwrap()
                        );
                        assert_eq!(
                            boundary.border_width,
                            proposal::metric(
                                "stroke",
                                if focused && !disabled {
                                    "c-focus-width"
                                } else {
                                    "stroke-rule"
                                }
                            )
                            .unwrap()
                        );
                        assert!(!blink_on.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.fill.a == 0. && q.border_width > 0. && q.rect.center() == frame.center())), "Field focus has no separate outer contour");
                        assert_eq!(surface.text(), draft);
                        assert_eq!(surface.input().selection(), selection);
                        assert_eq!(surface.visible_frame(), Some(frame));
                        if disabled || read_only {
                            assert!(!surface.input_mut().insert("x", None));
                            assert!(!surface.input_mut().preedit("pin", 3..3, None));
                            assert!(!surface.input_mut().delete(true, false));
                            assert_eq!(surface.text(), draft);
                        }
                        if state.starts_with("composing") {
                            assert!(surface.input_mut().preedit("", 0..0, None));
                            assert_eq!(surface.text(), "A😀e\u{301}中 Sample");
                            assert_eq!(surface.input().selection(), 1..5);
                            assert!(surface.input_mut().preedit("你", 1..1, None));
                            assert!(surface.input_mut().insert("你", None));
                            assert!(surface.input_mut().undo());
                            assert_eq!(surface.text(), "A😀e\u{301}中 Sample");
                        }
                        // Ending capture changes no input skin or text/selection;
                        // hover/pressed have no Button appearance or activation.
                        surface.pointer_up();
                        assert!(!surface.pointer_state(Some(point)).pressed);
                        let mut released = Scene::new();
                        surface.draw(&mut host, &mut released, palette, focused, true);
                        if !state.starts_with("composing") {
                            assert_eq!(released.primitives(), blink_on.primitives());
                        }
                        // Existing pointer selection/drag is reused, including read-only.
                        let accepted = surface.pointer_down(frame.center(), false, 1, 24.);
                        assert_eq!(accepted.is_some(), !disabled);
                        if !disabled {
                            surface.pointer_drag(
                                Point::new(frame.max_x() - 15., frame.center().y),
                                24.,
                            );
                        }
                        surface.cancel_interaction();
                        assert!(!surface.pointer_drag(frame.center(), 24.));
                        assert!(!surface.pointer_state(Some(point)).pressed);
                        pointer
                    });
                    cases.push(serde_json::json!({"component":component,"theme":format!("{theme:?}"),"brand":brand,"input_state":state,"pointer_hovered":pointer.hovered,"pointer_pressed":pointer.pressed,"lookups": reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 612);
    #[cfg(feature = "token-audit")]
    if let Ok(path) = std::env::var("TEKESUI_INPUT_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({
            "format":"tekesui-runtime-token-observation/1", "complete":false,
            "scope":"Public TextSurface layout/draw, CoreText shaping and existing edit/pointer transactions; 3 input aliases, 4 themes, 3 existing brands, 17 states/combinations.",
            "excluded":["compiled constants and aliases", "baked TextRole style accessors", "font backend internals", "unexecuted paths", "physical IME and accessibility"], "cases":cases
        })).unwrap()).unwrap();
    }
}

#[test]
fn pointer_snapshot_uses_visible_geometry_and_existing_selection_capture() {
    let contract: serde_json::Value =
        serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap();
    let contract = &contract["proposal"]["interaction"]["text_surface"];
    assert_eq!(contract["hover"]["presentation"], "preserve-input-surface");
    assert_eq!(
        contract["pressed"]["presentation"],
        "preserve-input-surface"
    );
    assert_eq!(contract["presentation"]["hover_pressed_overrides"], false);
    let mut host = TextHost(CoreTextShaper::new(CoreTextFonts::new()));
    for multiline in [false, true] {
        for read_only in [false, true] {
            let mut surface = TextSurface::new("A😀 e\u{301} 中文 sample", multiline, read_only);
            let frame = Rect::new(20., 20., 240., 80.);
            let clip = Rect::new(20., 20., 100., 80.);
            let role = surface.role();
            surface.layout_with(frame, Some(clip), 1., 1., |s, w| {
                host.shape_surface(s, role, w)
            });
            let inside = Point::new(40., 40.);
            let outside = Point::new(200., 40.);
            assert!(surface.pointer_state(Some(inside)).hovered);
            assert!(!surface.pointer_state(Some(outside)).hovered);
            assert!(surface.pointer_down(outside, false, 1, 24.).is_none());
            assert!(!surface.pointer_state(Some(outside)).pressed);
            let draft = surface.text().to_owned();
            assert!(surface.pointer_down(inside, false, 1, 24.).is_some());
            assert!(surface.pointer_state(None).pressed);
            surface.pointer_drag(outside, 24.);
            let selection = surface.input().selection();
            surface.pointer_up();
            assert!(!surface.pointer_state(Some(inside)).pressed);
            assert_eq!(surface.input().selection(), selection);
            assert_eq!(surface.text(), draft);
            surface.pointer_down(inside, false, 1, 24.);
            surface.input_mut().set_disabled(true);
            let state = surface.pointer_state(Some(inside));
            assert!(state.disabled);
            assert!(!state.hovered && !state.pressed);
            assert!(!surface.pointer_drag(outside, 24.));
            surface.input_mut().set_disabled(false);
            assert!(!surface.pointer_state(Some(inside)).pressed);
            surface.pointer_down(inside, false, 1, 24.);
            surface.cancel_interaction();
            assert!(!surface.pointer_state(None).pressed);
            assert!(!surface.pointer_drag(outside, 24.));
        }
    }
}

#[test]
fn ordinary_multiline_input_defaults_to_neutral_role_matching_composer_metrics() {
    #[path = "support/coretext.rs"]
    mod coretext;
    use tekes_ui_controls::composition::{
        AccessibilityAction, Component, ComponentId, Content, ContentHost, MarkdownEditorContent,
        TextFieldContent,
    };

    assert_eq!(proposal::TextRole::MultilineBody.id(), "multiline-body");
    let multiline_style = proposal::TextRole::MultilineBody.style();
    let composer_style = proposal::TextRole::ComposerBody.style();
    assert_eq!(multiline_style.size, 14.0);
    assert_eq!(multiline_style.line_height, 20.0);
    assert_eq!(multiline_style.weight, 400);
    assert_eq!(
        proposal::text_family(proposal::TextRole::MultilineBody.id()),
        Some("ui")
    );
    assert_eq!(
        multiline_style.size, composer_style.size,
        "neutral multiline role keeps 14 pt size"
    );
    assert_eq!(
        multiline_style.line_height, composer_style.line_height,
        "neutral multiline role keeps 20 pt line height"
    );
    assert_eq!(multiline_style.weight, composer_style.weight);

    // Ordinary public inputs and surface constructors.
    let single_field = TextFieldContent::new("Title", "Hello");
    assert_eq!(single_field.role(), proposal::TextRole::UiBody);

    let multiline_field = TextFieldContent::multiline("Notes", "Hello");
    assert_eq!(multiline_field.role(), proposal::TextRole::MultilineBody);

    let explicit_composer = TextFieldContent::multiline("Composer", "Hello")
        .with_role(proposal::TextRole::ComposerBody);
    assert_eq!(explicit_composer.role(), proposal::TextRole::ComposerBody);

    let ro_doc =
        TextFieldContent::read_only_content("Doc", "Read-only body", proposal::TextRole::DocBody);
    assert_eq!(ro_doc.role(), proposal::TextRole::DocBody);

    let md = MarkdownEditorContent::new("md-doc", 1, "Paragraph one").expect("markdown editor");
    assert_eq!(
        md.text_surface().expect("markdown text surface").role(),
        proposal::TextRole::UiBody
    );

    assert_eq!(
        TextSurface::new("a", false, false).role(),
        proposal::TextRole::UiBody
    );
    assert_eq!(
        TextSurface::new("a", true, false).role(),
        proposal::TextRole::MultilineBody
    );
    assert_eq!(
        TextSurface::new("a", true, true).role(),
        proposal::TextRole::MultilineBody
    );

    // Compare layout, type scaling, line wrapping, selection, and IME candidate/caret geometry
    // between default MultilineBody and explicit ComposerBody through ContentHost + CoreText.
    let sample =
        "Ordinary multiline input wraps smoothly across lines when narrowed; 中文输入候选定位。";
    let frame = Rect::new(0., 0., 200., 220.);
    let palette = Palette::from(Theme::LIGHT);

    for type_scale in [1.0_f32, 1.5_f32] {
        let mut text = coretext::Text::new(type_scale);
        text.services = true;

        let mut neutral_host = ContentHost::new(Content::new(
            ComponentId(1),
            TextFieldContent::multiline("Notes", sample),
        ))
        .unwrap();
        let mut composer_host = ContentHost::new(Content::new(
            ComponentId(2),
            TextFieldContent::multiline("Notes", sample)
                .with_role(proposal::TextRole::ComposerBody),
        ))
        .unwrap();

        let mut s1 = Scene::new();
        let mut s2 = Scene::new();
        neutral_host
            .render(&mut s1, &mut text, frame, frame, palette)
            .expect("render neutral multiline");
        composer_host
            .render(&mut s2, &mut text, frame, frame, palette)
            .expect("render explicit composer");

        {
            let surf_n = neutral_host.text_surface(ComponentId(1)).unwrap();
            let surf_c = composer_host.text_surface(ComponentId(2)).unwrap();
            assert_eq!(surf_n.role(), proposal::TextRole::MultilineBody);
            assert_eq!(surf_c.role(), proposal::TextRole::ComposerBody);

            let pn = surf_n.paragraph().expect("shaped neutral paragraph");
            let pc = surf_c.paragraph().expect("shaped composer paragraph");
            assert!(pn.lines.len() >= 2, "expected wrapped multiline layout");
            assert_eq!(pn.lines.len(), pc.lines.len());
            assert_eq!(pn.height.to_bits(), pc.height.to_bits());
            assert_eq!(pn.width.to_bits(), pc.width.to_bits());
            assert_eq!(surf_n.scroll().limit(), surf_c.scroll().limit());
            assert_eq!(
                surf_n.minimum_outer_height(type_scale).to_bits(),
                surf_c.minimum_outer_height(type_scale).to_bits()
            );
        }

        // Selection range & rects parity.
        let r1 = neutral_host.accessibility_action(
            ComponentId(1),
            None,
            AccessibilityAction::SetSelection(9..20),
        );
        let r2 = composer_host.accessibility_action(
            ComponentId(2),
            None,
            AccessibilityAction::SetSelection(9..20),
        );
        assert!(r1.consumed && r1.changed && r2.consumed && r2.changed);
        assert_eq!(
            neutral_host
                .text_surface(ComponentId(1))
                .unwrap()
                .selection_rects(9..20),
            composer_host
                .text_surface(ComponentId(2))
                .unwrap()
                .selection_rects(9..20)
        );

        // Active IME composition + candidate/marked geometry parity.
        let clauses = vec![0..2, 2..5];
        assert!(
            neutral_host
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap()
                .set_composition_preview("nihao", 2..5, None, clauses.clone())
        );
        assert!(
            composer_host
                .get_mut(ComponentId(2))
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap()
                .set_composition_preview("nihao", 2..5, None, clauses)
        );
        neutral_host
            .render(&mut Scene::new(), &mut text, frame, frame, palette)
            .expect("render neutral IME");
        composer_host
            .render(&mut Scene::new(), &mut text, frame, frame, palette)
            .expect("render composer IME");

        let snap_n = neutral_host
            .semantic_text_input_snapshot(ComponentId(1))
            .unwrap();
        let snap_c = composer_host
            .semantic_text_input_snapshot(ComponentId(2))
            .unwrap();
        assert_eq!(snap_n.marked, snap_c.marked);
        let surf_n = neutral_host.text_surface(ComponentId(1)).unwrap();
        let surf_c = composer_host.text_surface(ComponentId(2)).unwrap();
        let marked = surf_n.marked().expect("active marked range");
        assert_eq!(
            surf_n.selection_rects(marked.clone()),
            surf_c.selection_rects(marked)
        );
        assert_eq!(surf_n.caret(), surf_c.caret());
    }
}
