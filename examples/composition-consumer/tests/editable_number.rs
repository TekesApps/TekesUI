use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text(f32, u64);
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: std::time::Duration::from_millis(self.1),
            reduced_motion: true,
        })
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        let paragraph =
            <Self as tekes_ui_controls::TextSurfaceHost>::shape_surface(self, s, r, Some(w));
        Size::new(paragraph.width as f32, paragraph.height as f32)
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
    fn type_scale(&self) -> f32 {
        self.0
    }
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
            &tekes_ui_text::style::TextStyle::system(role.style().size * self.0),
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

use tekes_ui_composition_consumer::editable_number::*;
use tekes_ui_controls::{ControlKey, TextSurfaceAlignment};
fn render(host: &mut ContentHost, width: f32) {
    render_scaled(host, width, 1.);
}
fn render_scaled(host: &mut ContentHost, width: f32, scale: f32) {
    let f = Rect::new(10., 20., width, 400.);
    host.render(
        &mut Scene::new(),
        &mut Text(scale, 0),
        f,
        f,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn field(host: &ContentHost) -> &NumberFieldContent {
    host.get(PERCENT)
        .unwrap()
        .component::<NumberFieldContent>()
        .unwrap()
}
fn field_mut(host: &mut ContentHost) -> &mut NumberFieldContent {
    host.get_mut(PERCENT)
        .unwrap()
        .component_mut::<NumberFieldContent>()
        .unwrap()
}
fn fixture() -> ContentHost {
    let mut h = build().unwrap();
    render(&mut h, 300.);
    assert!(h.focus(Some(PERCENT), true));
    h
}
fn replace(h: &mut ContentHost, text: &str) {
    h.accessibility_action(PERCENT, None, AccessibilityAction::SetValue(text.into()));
}
fn key(h: &mut ContentHost, key: ControlKey) {
    h.event(&ContentEvent::Key { key, down: true });
}
#[test]
fn paired_formats_and_custom_parser() {
    let percent = DecimalNumberFormat { suffix: "%".into() };
    for value in [0., -0., 0.125, -125., 1e20, 300.] {
        assert_eq!(percent.parse(&percent.format(value)), Ok(value));
    }
    assert_eq!(percent.parse("125"), Ok(125.));
    assert_eq!(percent.parse(" 125% "), Ok(125.));
    for bad in [
        "NaN",
        "inf",
        "125%%",
        "1,2",
        "abc",
        "1e2",
        "1e",
        "1e-",
        "一百二十五",
        "１２５",
        "125px",
        "125％",
        "12+5",
        "1.2.5",
    ] {
        assert_eq!(percent.parse(bad), Err(NumberParseError::Invalid));
    }
    for text in ["", "-", "+", ".", "+.", "-."] {
        assert_eq!(percent.parse(text), Err(NumberParseError::Incomplete));
    }
    #[derive(Debug)]
    struct Fixed;
    impl NumberFormat for Fixed {
        fn format(&self, v: f64) -> String {
            format!("{v:.2}")
        }
        fn parse(&self, s: &str) -> Result<f64, NumberParseError> {
            s.parse().map_err(|_| NumberParseError::Invalid)
        }
    }
    let mut f = NumberFieldContent::new("Fixed", 1.25, 0., 5., 0.25, Fixed).unwrap();
    f.accessibility_action(None, &AccessibilityAction::SetValue("1.234".into()));
    assert_eq!(f.validation(), NumberValidation::FormatMismatch);
    assert!(!f.commit(NumberCommitReason::Submit));
    assert!(NumberFieldContent::new("Bad", 301., 50., 300., 10., percent).is_none());
}
#[test]
fn drafts_submit_step_and_cancel_have_separate_values_and_once_only_changes() {
    let mut h = fixture();
    replace(&mut h, "125");
    assert_eq!(field(&h).value(), 100.);
    assert_eq!(field(&h).edit_state().text, "125");
    assert_eq!(field(&h).validation(), NumberValidation::Valid(125.));
    assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
    key(&mut h, ControlKey::Enter);
    assert_eq!(field(&h).edit_state().text, "125%");
    assert_eq!(field(&h).value(), 125.);
    key(&mut h, ControlKey::Enter);
    let changes: Vec<_> = field_mut(&mut h).drain_value_changes().collect();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].reason, NumberCommitReason::Submit);
    key(&mut h, ControlKey::Up);
    assert_eq!(field(&h).value(), 135.);
    replace(&mut h, "125%");
    key(&mut h, ControlKey::Down);
    assert_eq!(field(&h).value(), 115.);
    replace(&mut h, "175");
    key(&mut h, ControlKey::Escape);
    assert_eq!(field(&h).edit_state().text, "115%");
}
#[test]
fn invalid_incomplete_and_range_drafts_survive_submit_step_and_blur() {
    for (text, validation) in [
        ("-", NumberValidation::Incomplete),
        ("oops", NumberValidation::Invalid),
        ("301%", NumberValidation::OutOfRange),
        ("49", NumberValidation::OutOfRange),
    ] {
        let mut h = fixture();
        replace(&mut h, text);
        key(&mut h, ControlKey::Enter);
        key(&mut h, ControlKey::Up);
        h.focus(Some(INTEGER), true);
        assert_eq!(field(&h).value(), 100.);
        assert_eq!(field(&h).edit_state().text, text);
        assert_eq!(field(&h).validation(), validation);
        assert!(
            field(&h)
                .semantic_parts()
                .iter()
                .all(|p| p.semantics.disabled)
        );
        assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
    }
    let mut h = fixture();
    replace(&mut h, "126");
    key(&mut h, ControlKey::Tab { reverse: false });
    assert_eq!(h.focused(), Some(INTEGER));
    assert_eq!(field(&h).value(), 126.);
    assert_eq!(
        field_mut(&mut h)
            .drain_value_changes()
            .next()
            .unwrap()
            .reason,
        NumberCommitReason::Blur
    );
    key(&mut h, ControlKey::Tab { reverse: true });
    assert_eq!(h.focused(), Some(PERCENT));
}
#[test]
fn pointer_buttons_boundaries_and_accessibility_share_value_and_geometry() {
    let mut h = fixture();
    replace(&mut h, "125");
    render(&mut h, 90.);
    let parts = field(&h).semantic_parts();
    let p = parts[1].frame.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(field(&h).value(), 135.);
    h.accessibility_action(PERCENT, Some(0), AccessibilityAction::Press);
    assert_eq!(field(&h).value(), 125.);
    replace(&mut h, "295");
    key(&mut h, ControlKey::Up);
    assert_eq!(field(&h).value(), 300.);
    assert!(!field(&h).can_step(true));
    assert!(field(&h).semantic_parts()[1].semantics.disabled);
    replace(&mut h, "50");
    key(&mut h, ControlKey::Enter);
    assert!(!field(&h).can_step(false));
    assert_eq!(field(&h).value_range(), Some(50.0..=300.0));
    let semantics = field(&h).semantics();
    assert_eq!(semantics.name, "Percentage");
    assert_eq!(semantics.value.as_deref(), Some("50%"));
    assert!(semantics.description.unwrap().contains("50% to 300%"));
    field_mut(&mut h).set_disabled(true);
    assert!(!field(&h).focusable());
    assert!(!field_mut(&mut h).step(true));
}
#[test]
fn ime_blocks_numeric_operations_and_native_key_chain_cannot_commit_candidate() {
    let mut h = fixture();
    h.event(&ContentEvent::Preedit {
        text: "125".into(),
        selection: 3..3,
        clauses: std::iter::once(0..3).collect(),
        replacement: Some(0..4),
    });
    assert!(field(&h).composing());
    key(&mut h, ControlKey::Enter);
    key(&mut h, ControlKey::Up);
    h.accessibility_action(PERCENT, None, AccessibilityAction::Increment);
    assert_eq!(field(&h).value(), 100.);
    assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
    key(&mut h, ControlKey::Escape);
    assert!(!field(&h).composing());
    assert_eq!(field(&h).edit_state().text, "100%");
    let enter = ContentKeyEvent {
        key: Some(ControlKey::Enter),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
    };
    h.event(&ContentEvent::TextKeyStart {
        id: 1,
        key: enter,
        composing: true,
    });
    h.event(&ContentEvent::Commit {
        text: "125".into(),
        replacement: Some(0..4),
    });
    h.event(&ContentEvent::TextCommand("insertNewline:".into()));
    h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(field(&h).value(), 100.);
    assert_eq!(field(&h).edit_state().text, "125");
    key(&mut h, ControlKey::Enter);
    assert_eq!(field(&h).value(), 125.);
}
#[test]
fn text_undo_redo_selection_and_alignment_keep_original_surface() {
    let mut h = fixture();
    replace(&mut h, "125");
    h.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(field(&h).edit_state().text, "100%");
    h.event(&ContentEvent::TextCommand("redo:".into()));
    assert_eq!(field(&h).edit_state().text, "125");
    h.accessibility_action(PERCENT, None, AccessibilityAction::SetSelection(0..2));
    assert_eq!(h.text_surface(PERCENT).unwrap().input.selected_text(), "12");
    h.accessibility_action(
        PERCENT,
        None,
        AccessibilityAction::ReplaceSelectedText("13".into()),
    );
    assert_eq!(field(&h).edit_state().text, "135");
    assert_eq!(field(&h).value(), 100.);
    for width in [42., 90., 300.] {
        field_mut(&mut h).set_alignment(TextSurfaceAlignment::Trailing);
        render(&mut h, width);
        let surface = h.text_surface(PERCENT).unwrap();
        let caret = surface.caret();
        assert!(caret.origin.x.is_finite());
        assert_eq!(
            field(&h).semantic_frame(h.get(PERCENT).unwrap().frame()),
            surface.outer
        );
        assert!(
            field(&h)
                .semantic_parts()
                .iter()
                .all(|p| p.frame.size.width <= width / 3. + 0.01)
        );
    }
}
#[test]
fn external_values_refuse_edit_conflicts_and_do_not_echo_persistence() {
    let mut h = fixture();
    assert_eq!(field_mut(&mut h).set_value(150.), Ok(true));
    assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
    replace(&mut h, "175");
    assert_eq!(field_mut(&mut h).set_value(150.), Ok(false));
    assert_eq!(field(&h).edit_state().text, "175");
    assert_eq!(
        field_mut(&mut h).set_value(200.),
        Err(NumberValueError::Editing)
    );
    assert_eq!(
        field_mut(&mut h).set_value(500.),
        Err(NumberValueError::InvalidValue)
    );
    assert!(field_mut(&mut h).cancel_edit());
    assert_eq!(field_mut(&mut h).set_value(200.), Ok(true));
    assert_eq!(field(&h).edit_state().text, "200%");
    assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
}
#[test]
fn extreme_finite_steps_saturate_at_boundary_instead_of_overflowing() {
    let mut f = NumberFieldContent::new(
        "Finite",
        1e308,
        -f64::MAX,
        f64::MAX,
        1e308,
        DecimalNumberFormat::default(),
    )
    .unwrap();
    assert!(f.step(true));
    assert_eq!(f.value(), f64::MAX);
    assert!(!f.can_step(true));
}
#[test]
fn candidate_confirming_arrow_does_not_step_but_plain_native_arrow_steps_once() {
    let mut h = fixture();
    let up = ContentKeyEvent {
        key: Some(ControlKey::Up),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 126,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
    };
    h.event(&ContentEvent::TextKeyStart {
        id: 1,
        key: up.clone(),
        composing: true,
    });
    h.event(&ContentEvent::Commit {
        text: "125".into(),
        replacement: Some(0..4),
    });
    h.event(&ContentEvent::TextCommand("moveUp:".into()));
    h.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    assert_eq!(field(&h).value(), 100.);
    h.event(&ContentEvent::TextKeyStart {
        id: 2,
        key: up.clone(),
        composing: false,
    });
    h.event(&ContentEvent::TextCommand("moveUp:".into()));
    h.event(&ContentEvent::KeyInput(up));
    h.event(&ContentEvent::TextKeyEnd {
        id: 2,
        input_context_consumed: false,
    });
    assert_eq!(field(&h).value(), 135.);
    assert_eq!(field_mut(&mut h).drain_value_changes().count(), 1);
}
#[test]
fn inline_field_and_buttons_share_vertical_center_in_both_presentations() {
    let mut h = fixture();
    for borderless in [false, true] {
        field_mut(&mut h).set_borderless(borderless);
        for align in [
            TextSurfaceAlignment::Leading,
            TextSurfaceAlignment::Center,
            TextSurfaceAlignment::Trailing,
        ] {
            field_mut(&mut h).set_alignment(align);
            render(&mut h, 120.);
            let surface = h.text_surface(PERCENT).unwrap();
            let center = surface.outer.center().y;
            assert!(
                field(&h)
                    .semantic_parts()
                    .iter()
                    .all(|p| (p.frame.center().y - center).abs() < 0.01)
            );
            let caret = surface.caret();
            assert_eq!(surface.hit(caret.center()), Some(surface.input.caret));
        }
    }
}

#[test]
fn scale_relayout_keeps_text_hits_actions_and_ax_frames_in_one_coordinate_space() {
    let mut h = fixture();
    for scale in [0.75, 1., 1.5, 2., 3.] {
        for width in [48., 120., 400.] {
            render_scaled(&mut h, width, scale);
            let surface = h.text_surface(PERCENT).unwrap();
            assert_eq!(surface.type_scale, scale);
            let caret = surface.caret();
            if surface.inner.contains(caret.center()) {
                assert_eq!(
                    surface.hit(caret.center()),
                    Some(surface.input.caret),
                    "scale={scale}, width={width}"
                );
            }
            assert!(surface.outer.size.width >= 0. && surface.inner.size.width >= 0.);
            assert!(caret.min_x().is_finite() && caret.min_y().is_finite());
            let parts = field(&h).semantic_parts();
            assert!(
                parts
                    .iter()
                    .all(|p| (p.frame.center().y - surface.outer.center().y).abs() < 0.01)
            );
            let p = parts[1].frame.center();
            h.event(&ContentEvent::PointerDown(p));
            h.event(&ContentEvent::PointerUp(p));
            assert!(field(&h).value().is_finite());
        }
    }
}
#[test]
fn blur_of_preedit_retains_draft_without_a_numeric_commit() {
    let mut h = fixture();
    h.event(&ContentEvent::Preedit {
        text: "125".into(),
        selection: 3..3,
        clauses: std::iter::once(0..3).collect(),
        replacement: Some(0..4),
    });
    h.focus(Some(INTEGER), true);
    assert!(!field(&h).composing());
    assert_eq!(field(&h).edit_state().text, "125");
    assert_eq!(field(&h).value(), 100.);
    assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
    h.focus(Some(PERCENT), true);
    key(&mut h, ControlKey::Enter);
    assert_eq!(field(&h).value(), 125.);
}
#[test]
fn cancel_and_outside_release_never_step() {
    let mut h = fixture();
    let p = field(&h).semantic_parts()[1].frame.center();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::Cancel);
    h.event(&ContentEvent::PointerUp(p));
    assert_eq!(field(&h).value(), 100.);
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(tekes_ui_scene::Point::new(0., 0.)));
    assert_eq!(field(&h).value(), 100.);
}

#[test]
fn only_numeric_text_and_the_declared_unit_can_be_committed() {
    let mut h = fixture();
    for text in [
        "125px",
        "125％",
        "一百二十五",
        "１２５%",
        "1e2",
        "abcE",
        "NaN%",
        "125%%",
    ] {
        replace(&mut h, text);
        assert_eq!(field(&h).validation(), NumberValidation::Invalid, "{text}");
        key(&mut h, ControlKey::Enter);
        key(&mut h, ControlKey::Up);
        h.accessibility_action(PERCENT, Some(1), AccessibilityAction::Press);
        h.focus(Some(INTEGER), true);
        assert_eq!(field(&h).edit_state().text, text);
        assert_eq!(field(&h).value(), 100.);
        assert_eq!(field_mut(&mut h).drain_value_changes().count(), 0);
        h.focus(Some(PERCENT), true);
    }
    for text in ["125", "125%", "+125%", "125.5%", " 125 % "] {
        replace(&mut h, text);
        assert!(
            matches!(field(&h).validation(), NumberValidation::Valid(_)),
            "{text}"
        );
        key(&mut h, ControlKey::Enter);
        assert!(field(&h).edit_state().text.ends_with('%'));
    }
}

#[test]
fn hints_on_numeric_buttons_use_their_own_geometry_outside_the_text_frame() {
    let mut h = fixture();
    let f = Rect::new(10., 20., 300., 400.);
    let mut t = Text(1., 0);
    for part in [0, 1] {
        let p = field(&h).semantic_parts()[part].frame.center();
        assert!(!h.text_surface(PERCENT).unwrap().outer.contains(p));
        h.event(&ContentEvent::PointerMove(p));
        h.render(&mut Scene::new(), &mut t, f, f, Theme::LIGHT.into())
            .unwrap();
        t.1 += 600;
        h.render(&mut Scene::new(), &mut t, f, f, Theme::LIGHT.into())
            .unwrap();
        assert_eq!(
            h.help_tooltip().map(|(_, text, _)| text),
            Some(if part == 0 { "Decrease" } else { "Increase" })
        );
        assert_eq!(h.focused(), Some(PERCENT));
        assert_eq!(field(&h).value(), 100.);
    }
}

#[test]
fn digit_capacity_fixes_only_input_width_without_limiting_edits() {
    let mut h = fixture();
    assert_eq!(field(&h).input_digits(), Some(3));
    let width = h.text_surface(PERCENT).unwrap().outer.size.width;
    for draft in ["50", "125%", "300", "1234567890%", "invalid"] {
        replace(&mut h, draft);
        render(&mut h, 400.);
        assert_eq!(h.text_surface(PERCENT).unwrap().outer.size.width, width);
        assert_eq!(field(&h).edit_state().text, draft);
    }
    assert!(!field_mut(&mut h).set_input_digits(0));
    assert!(!field_mut(&mut h).set_input_digits(1025));
    assert_eq!(field(&h).input_digits(), Some(3));
    field_mut(&mut h).set_input_digits(5);
    render(&mut h, 400.);
    assert!(h.text_surface(PERCENT).unwrap().outer.size.width > width);
    let five = h.text_surface(PERCENT).unwrap().outer.size.width;
    field_mut(&mut h).clear_input_digits();
    replace(&mut h, "50%");
    render(&mut h, 400.);
    assert!(h.text_surface(PERCENT).unwrap().outer.size.width < five);
}

#[test]
fn fixed_digit_width_measures_units_font_scale_and_border_insets() {
    let mut percent = NumberFieldContent::new(
        "Percent",
        100.,
        0.,
        300.,
        10.,
        DecimalNumberFormat { suffix: "%".into() },
    )
    .unwrap();
    let mut plain =
        NumberFieldContent::new("Plain", 100., 0., 300., 10., DecimalNumberFormat::default())
            .unwrap();
    percent.set_input_digits(3);
    plain.set_input_digits(3);
    let available = Size::new(800., 80.);
    let mut t = Text(1., 0);
    let width = percent.measure(&mut t, available).size.width;
    assert!(width > plain.measure(&mut t, available).size.width);
    percent.set_borderless(true);
    let borderless = percent.measure(&mut t, available).size.width;
    assert!(borderless < width);
    assert!(percent.measure(&mut Text(2., 0), available).size.width > borderless);
    assert!(
        percent
            .measure(&mut Text(1., 0), Size::new(30., 80.))
            .size
            .width
            <= 30.
    );
}

#[test]
fn default_auto_width_tracks_content_and_never_fills_extra_parent_space() {
    let mut h = fixture();
    field_mut(&mut h).clear_input_digits();
    assert_eq!(field(&h).input_digits(), None);
    replace(&mut h, "50%");
    render(&mut h, 400.);
    let short = h.text_surface(PERCENT).unwrap().outer.size.width;
    render(&mut h, 800.);
    assert_eq!(h.text_surface(PERCENT).unwrap().outer.size.width, short);
    replace(&mut h, "125%");
    render(&mut h, 800.);
    let longer = h.text_surface(PERCENT).unwrap().outer.size.width;
    assert!(longer > short && longer < 200.);
    replace(&mut h, "");
    render(&mut h, 800.);
    let empty = h.text_surface(PERCENT).unwrap().outer.size.width;
    assert!(empty > 0. && empty <= short);
    assert_eq!(field(&h).edit_state().text, "");
}
