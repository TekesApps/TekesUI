use tekes_ui_controls::{
    DraftRevision, ExternalSelection as S, ExternalUndo as U, ExternalValueConflict as C,
    ExternalValueResult as R, TextEditBaseline, TextInput, TextSurface,
};
fn value(key: &str, version: u64, payload: &str) -> DraftRevision {
    DraftRevision {
        identity: key.into(),
        version,
        payload: payload.into(),
    }
}
fn bind(input: &mut TextInput) {
    let b = input.external_baseline();
    input.apply_external_value(&b, &value("A", 1, "a😀b"), S::Reset, U::Reset);
}
fn parity(
    input: &mut TextInput,
    baseline: &TextEditBaseline,
    value: &DraftRevision,
    selection: S,
    undo: U,
    expected: R,
) {
    let before = format!("{input:?}");
    assert_eq!(
        input.check_external_value(baseline, value, selection.clone(), undo),
        expected
    );
    assert_eq!(
        format!("{input:?}"),
        before,
        "preflight must preserve ALL input state"
    );
    assert_eq!(
        input.apply_external_value(baseline, value, selection, undo),
        expected
    );
    if matches!(expected, R::Conflict(_) | R::Unchanged) {
        assert_eq!(format!("{input:?}"), before);
    }
}
#[test]
fn every_rejection_and_duplicate_ack_match_apply_without_mutation() {
    let mut a = TextInput::new("", true);
    let b = TextInput::new("", true);
    bind(&mut a);
    let current = a.external_baseline();
    parity(
        &mut a,
        &b.external_baseline(),
        &value("A", 2, "x"),
        S::Reset,
        U::Reset,
        R::Conflict(C::ForeignEditor),
    );
    for v in [value("", 2, "x"), value("A", 0, "x")] {
        parity(
            &mut a,
            &current,
            &v,
            S::Reset,
            U::Reset,
            R::Conflict(C::InvalidIdentity),
        );
    }
    for v in [value("A", 1, "different"), value("A", 0, "x")] {
        let expected = if v.version == 0 {
            C::InvalidIdentity
        } else {
            C::Revision
        };
        parity(
            &mut a,
            &current,
            &v,
            S::Reset,
            U::Reset,
            R::Conflict(expected),
        );
    }
    parity(
        &mut a,
        &current,
        &value("B", 1, "x"),
        S::Reset,
        U::Preserve,
        R::Conflict(C::DocumentResetRequired),
    );
    parity(
        &mut a,
        &current,
        &value("A", 2, "x"),
        S::Set(0..2),
        U::Reset,
        R::Conflict(C::Selection),
    );
    a.preedit("ni", 2..2, None);
    let composing = a.external_baseline();
    parity(
        &mut a,
        &composing,
        &value("A", 2, "x"),
        S::Reset,
        U::Reset,
        R::Conflict(C::Composition),
    );
    parity(
        &mut a,
        &current,
        &value("A", 1, "a😀b"),
        S::Set(100..101),
        U::Reset,
        R::Unchanged,
    );
    a.cancel_composition();
    a.insert("local", None);
    parity(
        &mut a,
        &current,
        &value("A", 2, "x"),
        S::Reset,
        U::Reset,
        R::Conflict(C::StaleBaseline),
    );
}
#[test]
fn older_publication_and_unchanged_payload_new_revision_preserve_exact_result_flags() {
    let mut input = TextInput::new("", true);
    bind(&mut input);
    let b = input.external_baseline();
    parity(
        &mut input,
        &b,
        &value("A", 3, "a😀b"),
        S::Preserve,
        U::Preserve,
        R::Applied {
            changed: false,
            document_changed: false,
        },
    );
    let b = input.external_baseline();
    parity(
        &mut input,
        &b,
        &value("A", 2, "older"),
        S::Preserve,
        U::Preserve,
        R::Conflict(C::Revision),
    );
    input.insert("!", None);
    let text = input.text.clone();
    let b = input.external_baseline();
    parity(
        &mut input,
        &b,
        &value("A", 4, &text),
        S::Preserve,
        U::Reset,
        R::Applied {
            changed: true,
            document_changed: false,
        },
    );
    assert!(!input.undo());
    let b = input.external_baseline();
    parity(
        &mut input,
        &b,
        &value("B", 1, &text),
        S::Preserve,
        U::Reset,
        R::Applied {
            changed: false,
            document_changed: true,
        },
    );
}

#[test]
fn successful_check_is_not_a_reservation_and_apply_revalidates_aba_documents_and_publications() {
    for intervening in ["edit", "aba", "document", "publication", "preedit"] {
        let mut a = TextInput::new("", true);
        bind(&mut a);
        let baseline = a.external_baseline();
        let pending = value("A", 2, "remote");
        assert!(matches!(
            a.check_external_value(&baseline, &pending, S::Preserve, U::Transaction),
            R::Applied { .. }
        ));
        let conflict = match intervening {
            "edit" => {
                a.insert("!", None);
                C::StaleBaseline
            }
            "aba" => {
                a.insert("!", None);
                assert!(a.undo());
                assert_eq!(a.text, "a😀b");
                C::StaleBaseline
            }
            "document" => {
                a.apply_external_value(&baseline, &value("B", 1, "other"), S::Reset, U::Reset);
                C::StaleBaseline
            }
            "publication" => {
                a.apply_external_value(
                    &baseline,
                    &value("A", 3, "newer"),
                    S::Preserve,
                    U::Preserve,
                );
                C::Revision
            }
            _ => {
                a.preedit("ni", 2..2, None);
                C::Composition
            }
        };
        parity(
            &mut a,
            &baseline,
            &pending,
            S::Preserve,
            U::Transaction,
            R::Conflict(conflict),
        );
    }
}
#[test]
fn successful_prediction_preserves_native_identity_and_actual_selection_undo_rules() {
    for multiline in [false, true] {
        for selection in [S::Preserve, S::Reset, S::Set(1..3)] {
            for undo in [U::Preserve, U::Transaction, U::Reset] {
                let mut input = TextInput::new("", multiline);
                bind(&mut input);
                input.select_range(1..3);
                input.insert("😀", None);
                let baseline = input.external_baseline();
                let next = value("A", 2, "新a😀b\r\n");
                let prediction =
                    input.check_external_value(&baseline, &next, selection.clone(), undo);
                assert!(matches!(prediction, R::Applied { .. }));
                parity(
                    &mut input,
                    &baseline,
                    &next,
                    selection.clone(),
                    undo,
                    prediction,
                );
                // Same editor baseline remains recognizable, although stale; a replacement
                // instance would return ForeignEditor instead.
                assert_ne!(
                    input.check_external_value(
                        &baseline,
                        &value("A", 3, "later"),
                        S::Reset,
                        U::Reset
                    ),
                    R::Conflict(C::ForeignEditor)
                );
                match undo {
                    U::Transaction => {
                        assert!(input.undo());
                        assert_eq!(input.text, "a😀b");
                        assert!(input.redo());
                    }
                    U::Reset => {
                        assert!(!input.undo());
                        assert!(!input.redo());
                    }
                    U::Preserve => {
                        assert!(input.undo());
                    }
                }
            }
        }
    }
}
#[test]
fn surface_check_preserves_preedit_clauses_scroll_navigation_and_history() {
    let mut surface = TextSurface::new("", true, false);
    bind(&mut surface.input);
    surface.input.select_range(1..3);
    surface.input.insert("!", None);
    surface.scroll.scroll_by(0., 50.);
    let b = surface.input.external_baseline();
    let before = format!("{surface:?}");
    let next = value("A", 2, "replacement");
    let predicted = surface.check_external_value(&b, &next, S::Preserve, U::Transaction);
    assert!(matches!(predicted, R::Applied { .. }));
    assert_eq!(format!("{surface:?}"), before);
    assert_eq!(
        surface.apply_external_value(&b, &next, S::Preserve, U::Transaction),
        predicted
    );
    assert!(surface.input.undo());
    surface.input.preedit("ni", 2..2, None);
    surface.clauses = std::iter::once(0..2).collect();
    let b = surface.input.external_baseline();
    let before = format!("{surface:?}");
    for _ in 0..2 {
        assert_eq!(
            surface.check_external_value(&b, &value("A", 3, "remote"), S::Reset, U::Reset),
            R::Conflict(C::Composition)
        );
        assert_eq!(format!("{surface:?}"), before);
    }
    surface.input.preedit("nih", 3..3, None);
    assert_eq!(
        surface.check_external_value(&b, &value("A", 3, "remote"), S::Reset, U::Reset),
        R::Conflict(C::Composition)
    );
}
