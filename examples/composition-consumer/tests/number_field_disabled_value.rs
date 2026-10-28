//! #129: public numeric synchronization keeps the original disabled editor atomic.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::Theme;
const NUMBER: ComponentId = ComponentId(129);
fn field(host: &ContentHost) -> &NumberFieldContent {
    host.get(NUMBER)
        .unwrap()
        .component::<NumberFieldContent>()
        .unwrap()
}
fn field_mut(host: &mut ContentHost) -> &mut NumberFieldContent {
    host.get_mut(NUMBER)
        .unwrap()
        .component_mut::<NumberFieldContent>()
        .unwrap()
}
fn render(host: &mut ContentHost) -> Scene {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let mut scene = Scene::new();
    let frame = Rect::new(10., 20., 320., 80.);
    host.render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())
        .unwrap();
    scene
}
fn fixture() -> ContentHost {
    let number = NumberFieldContent::new(
        "Percentage",
        10.,
        0.,
        100.,
        1.,
        DecimalNumberFormat { suffix: "%".into() },
    )
    .unwrap();
    let mut host = ContentHost::new(Content::new(NUMBER, number)).unwrap();
    render(&mut host);
    assert!(host.focus(Some(NUMBER), true));
    host
}
fn replace(host: &mut ContentHost, value: &str) {
    assert!(
        host.accessibility_action(NUMBER, None, AccessibilityAction::SetValue(value.into()))
            .changed
    );
}
#[test]
fn disabled_programmatic_value_is_atomic_and_reenabled_step_notifies_once() {
    let mut host = fixture();
    field_mut(&mut host).set_disabled(true);
    for value in [20., 30., 20.] {
        assert_eq!(field_mut(&mut host).set_value(value), Ok(true));
        let expected = format!("{value}%");
        let number = field(&host);
        assert_eq!(number.value(), value);
        assert_eq!(number.edit_state().text, expected);
        assert!(!number.edit_state().dirty);
        assert_eq!(number.validation(), NumberValidation::Valid(value));
        assert_eq!(number.semantics().value.as_deref(), Some(expected.as_str()));
        assert!(number.semantics().disabled && !number.semantics().invalid);
        let semantic = host
            .semantics()
            .into_iter()
            .find(|node| node.id == NUMBER)
            .unwrap();
        assert_eq!(semantic.semantics.value.as_deref(), Some(expected.as_str()));
        assert!(semantic.semantics.disabled);
        assert!(
            number
                .semantic_parts()
                .iter()
                .all(|part| part.semantics.disabled)
        );
        let surface = host.text_surface(NUMBER).unwrap();
        assert_eq!(surface.input.text, expected);
        assert!(surface.input.disabled);
        let end = expected.encode_utf16().count();
        assert_eq!(surface.input.selection(), end..end);
        assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
    }
    let baseline = host.text_surface(NUMBER).unwrap().input.external_baseline();
    assert_eq!(field_mut(&mut host).set_value(20.), Ok(false));
    assert_eq!(
        host.text_surface(NUMBER).unwrap().input.external_baseline(),
        baseline
    );
    assert!(
        !host
            .event(&ContentEvent::Commit {
                text: "blocked".into(),
                replacement: None
            })
            .changed
    );
    assert!(
        !host
            .accessibility_action(NUMBER, None, AccessibilityAction::SetValue("77".into()))
            .changed
    );
    assert!(!field_mut(&mut host).step(true));
    let scene = render(&mut host);
    assert_eq!(field(&host).value(), 20.);
    assert_eq!(field(&host).edit_state().text, "20%");
    let frame = host.text_surface(NUMBER).unwrap().outer;
    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Glyphs(run) if run.glyphs.iter().any(|g|frame.contains(g.origin)))),"actual original CoreText editor still paints its synchronized display");
    field_mut(&mut host).set_disabled(false);
    host.focus(Some(NUMBER), true);
    assert_eq!(host.focused(), Some(NUMBER));
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Up,
            down: true
        })
        .changed
    );
    assert_eq!(field(&host).value(), 21.);
    assert_eq!(field(&host).edit_state().text, "21%");
    assert_eq!(
        field_mut(&mut host)
            .drain_value_changes()
            .collect::<Vec<_>>(),
        vec![NumberValueChange {
            previous: 20.,
            value: 21.,
            reason: NumberCommitReason::Step
        }]
    );
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
}
#[test]
fn dirty_refusal_cancel_and_undo_keep_the_same_original_numeric_transactions() {
    let mut host = fixture();
    replace(&mut host, "12");
    field_mut(&mut host).set_disabled(true);
    let before = field(&host).edit_state();
    assert_eq!(
        field_mut(&mut host).set_value(20.),
        Err(NumberValueError::Editing)
    );
    assert_eq!(field_mut(&mut host).set_value(10.), Ok(false));
    assert_eq!(field(&host).edit_state(), before);
    assert!(!field_mut(&mut host).cancel_edit());
    assert!(!field_mut(&mut host).commit(NumberCommitReason::Submit));
    assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
    field_mut(&mut host).set_disabled(false);
    assert!(field_mut(&mut host).cancel_edit());
    assert_eq!(field(&host).edit_state().text, "10%");
    host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(field(&host).edit_state().text, "12");
    assert_eq!(
        field_mut(&mut host).set_value(20.),
        Err(NumberValueError::Editing)
    );
    host.event(&ContentEvent::TextCommand("redo:".into()));
    assert_eq!(field(&host).edit_state().text, "10%");
    assert_eq!(field_mut(&mut host).set_value(20.), Ok(true));
    assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
    host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(field(&host).value(), 20.);
    assert_eq!(field(&host).edit_state().text, "10%");
    assert!(field(&host).edit_state().dirty);
    assert!(field_mut(&mut host).commit(NumberCommitReason::Submit));
    assert_eq!(
        field_mut(&mut host)
            .drain_value_changes()
            .collect::<Vec<_>>(),
        vec![NumberValueChange {
            previous: 20.,
            value: 10.,
            reason: NumberCommitReason::Submit
        }]
    );
    host.event(&ContentEvent::TextCommand("redo:".into()));
    assert_eq!(field(&host).edit_state().text, "20%");
    assert_eq!(field(&host).value(), 10.);
    assert!(field_mut(&mut host).cancel_edit());
    assert_eq!(field(&host).edit_state().text, "10%");
    assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
}
#[test]
fn marked_refusal_preserves_preedit_until_the_original_cancel_path() {
    let mut host = fixture();
    assert!(
        host.event(&ContentEvent::Preedit {
            text: "12".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: Some(0..3)
        })
        .changed
    );
    assert!(field(&host).composing());
    field_mut(&mut host).set_disabled(true);
    let before = field(&host).edit_state();
    let marked = host.text_surface(NUMBER).unwrap().input.marked.clone();
    assert_eq!(
        field_mut(&mut host).set_value(20.),
        Err(NumberValueError::Editing)
    );
    assert_eq!(field_mut(&mut host).set_value(10.), Ok(false));
    assert_eq!(field(&host).edit_state(), before);
    assert_eq!(host.text_surface(NUMBER).unwrap().input.marked, marked);
    assert!(!field_mut(&mut host).cancel_edit());
    assert!(!field_mut(&mut host).commit(NumberCommitReason::Submit));
    field_mut(&mut host).set_disabled(false);
    host.event(&ContentEvent::TextCommand("cancelOperation:".into()));
    assert!(!field(&host).composing());
    assert_eq!(field(&host).edit_state().text, "10%");
    assert_eq!(field_mut(&mut host).set_value(20.), Ok(true));
    assert_eq!(field(&host).edit_state().text, "20%");
    assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
}
#[test]
fn disabled_boundaries_noops_and_invalid_formats_preserve_atomic_state() {
    let mut host = fixture();
    field_mut(&mut host).set_disabled(true);
    for value in [0., 100.] {
        assert_eq!(field_mut(&mut host).set_value(value), Ok(true));
        assert_eq!(field(&host).value(), value);
        assert_eq!(field(&host).edit_state().text, format!("{value}%"));
        assert_eq!(field(&host).semantics().value, Some(format!("{value}%")));
        assert_eq!(field(&host).validation(), NumberValidation::Valid(value));
        assert!(!field(&host).edit_state().dirty);
        assert_eq!(field_mut(&mut host).set_value(value), Ok(false));
        let before = field(&host).edit_state();
        let baseline = host.text_surface(NUMBER).unwrap().input.external_baseline();
        let semantics = field(&host).semantics();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 101.] {
            assert_eq!(
                field_mut(&mut host).set_value(invalid),
                Err(NumberValueError::InvalidValue)
            );
            assert_eq!(field(&host).edit_state(), before);
            assert_eq!(
                host.text_surface(NUMBER).unwrap().input.external_baseline(),
                baseline
            );
            assert_eq!(field(&host).semantics(), semantics);
        }
        assert_eq!(field_mut(&mut host).drain_value_changes().count(), 0);
    }
    #[derive(Debug)]
    struct OneDecimal;
    impl NumberFormat for OneDecimal {
        fn format(&self, value: f64) -> String {
            format!("{value:.1}")
        }
        fn parse(&self, text: &str) -> Result<f64, NumberParseError> {
            text.parse().map_err(|_| NumberParseError::Invalid)
        }
    }
    let mut number = NumberFieldContent::new("Fixed", 10., 0., 100., 0.5, OneDecimal).unwrap();
    number.set_disabled(true);
    let before = number.edit_state();
    assert_eq!(
        number.set_value(20.25),
        Err(NumberValueError::FormatMismatch)
    );
    assert_eq!(number.edit_state(), before);
    assert_eq!(number.semantics().value.as_deref(), Some("10.0"));
    assert_eq!(number.set_value(20.5), Ok(true));
    assert_eq!(number.value(), 20.5);
    assert_eq!(number.edit_state().text, "20.5");
    assert_eq!(number.validation(), NumberValidation::Valid(20.5));
    assert_eq!(number.drain_value_changes().count(), 0);
}

#[test]
fn formatter_changed_by_single_line_canonicalization_is_rejected_before_mutation() {
    #[derive(Debug)]
    struct ControlSuffix;
    impl NumberFormat for ControlSuffix {
        fn format(&self, value: f64) -> String {
            if value == 10. {
                "10%".into()
            } else {
                format!("{value}\n%")
            }
        }
        fn parse(&self, text: &str) -> Result<f64, NumberParseError> {
            text.strip_suffix('%')
                .ok_or(NumberParseError::Invalid)?
                .trim()
                .parse()
                .map_err(|_| NumberParseError::Invalid)
        }
    }
    let codec = ControlSuffix;
    assert_eq!(codec.parse(&codec.format(20.)), Ok(20.));
    let mut number =
        NumberFieldContent::new("Controlled format", 10., 0., 100., 1., codec).unwrap();
    for disabled in [false, true] {
        number.set_disabled(disabled);
        let before = number.edit_state();
        let semantics = number.semantics();
        let baseline = number.text_surface().unwrap().input.external_baseline();
        assert_eq!(number.set_value(10.), Ok(false));
        assert_eq!(number.set_value(20.), Err(NumberValueError::FormatMismatch));
        assert_eq!(number.edit_state(), before);
        assert_eq!(number.semantics(), semantics);
        assert_eq!(
            number.text_surface().unwrap().input.external_baseline(),
            baseline
        );
        assert_eq!(number.drain_value_changes().count(), 0);
    }
}
