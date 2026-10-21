use tekes_ui_composition_consumer::{
    controlled_reading::{PublicationError as E, publish},
    scroll_updates::{COMPOSER, SCROLLER, Scenario, TOP},
};
use tekes_ui_controls::{
    DraftRevision, ExternalSelection as S, ExternalUndo as U, ExternalValueConflict as C,
    TextSurface, composition::*,
};
fn draft(revision: u64, text: &str) -> DraftRevision {
    DraftRevision {
        identity: "draft-A".into(),
        version: revision,
        payload: text.into(),
    }
}
fn editor(s: &Scenario) -> &TextSurface {
    &s.host
        .get(COMPOSER)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
}
fn editor_mut(s: &mut Scenario) -> &mut TextSurface {
    &mut s
        .host
        .get_mut(COMPOSER)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
}
fn snapshot(s: &Scenario) -> String {
    format!(
        "{:?}/{:?}/{:?}/{:?}",
        editor(s),
        s.host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap(),
        s.item_keys(),
        s.host.focused()
    )
}
fn setup() -> (Scenario, tekes_ui_controls::ReadingDocumentVersion) {
    let mut s = Scenario::reading_document().unwrap();
    let surface = editor_mut(&mut s);
    let b = surface.input.external_baseline();
    surface.apply_external_value(&b, &draft(1, "local😀"), S::Reset, U::Reset);
    let v = s.attach_reading("transcript-A", 1).unwrap();
    s.host.focus(Some(COMPOSER), true);
    (s, v)
}
#[test]
fn invalid_editor_preflight_never_advances_reading_or_mutates_transcript() {
    for failure in ["stale", "composition", "revision", "foreign"] {
        let (mut s, v) = setup();
        let mut baseline = editor(&s).input.external_baseline();
        let mut next = draft(2, "remote");
        let conflict = match failure {
            "stale" => {
                editor_mut(&mut s).input.insert("!", None);
                C::StaleBaseline
            }
            "composition" => {
                editor_mut(&mut s).input.preedit("ni", 2..2, None);
                C::Composition
            }
            "revision" => {
                next = draft(1, "different");
                C::Revision
            }
            _ => {
                baseline = TextSurface::new("", true, false).input.external_baseline();
                C::ForeignEditor
            }
        };
        let before = snapshot(&s);
        assert_eq!(
            publish(&mut s, &v, &baseline, &next, ComponentId(18000)),
            Err(E::Editor(conflict))
        );
        assert_eq!(snapshot(&s), before);
        assert_eq!(
            s.host
                .get(SCROLLER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .reading_document_version(),
            Some(&v)
        );
    }
}
#[test]
fn reading_rejection_or_structural_failure_precedes_any_editor_apply() {
    let (mut s, old) = setup();
    let current = s.insert_versioned(&old, false).unwrap();
    let b = editor(&s).input.external_baseline();
    let before = snapshot(&s);
    assert!(matches!(
        publish(&mut s, &old, &b, &draft(2, "remote"), ComponentId(18000)),
        Err(E::Reading(_))
    ));
    assert_eq!(snapshot(&s), before);
    assert_eq!(
        publish(&mut s, &current, &b, &draft(2, "remote"), TOP),
        Err(E::Structure)
    );
    assert_eq!(snapshot(&s), before);
}
#[test]
fn complete_public_consumer_retains_original_editor_focus_selection_and_undo() {
    let (mut s, v) = setup();
    editor_mut(&mut s).input.select_range(5..7);
    let baseline = editor(&s).input.external_baseline();
    let v2 = publish(
        &mut s,
        &v,
        &baseline,
        &draft(2, "新local😀"),
        ComponentId(18000),
    )
    .unwrap();
    assert_eq!(v2.revision(), 2);
    assert_eq!(editor(&s).input.text, "新local😀");
    assert_eq!(editor(&s).input.selection(), 6..8);
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert!(s.item_keys().contains(&ComponentId(18000)));
    assert!(editor_mut(&mut s).input.undo());
    assert_eq!(editor(&s).input.text, "local😀");
    assert!(editor_mut(&mut s).input.redo());
    assert_eq!(editor(&s).input.text, "新local😀");
    let before = editor(&s).input.external_baseline();
    assert_ne!(
        editor(&s).check_external_value(&baseline, &draft(3, "next"), S::Reset, U::Reset),
        tekes_ui_controls::ExternalValueResult::Conflict(C::ForeignEditor)
    );
    assert_eq!(editor(&s).input.external_baseline(), before);
}

#[test]
fn duplicate_ack_can_append_transcript_without_disturbing_local_preedit() {
    let (mut s, version) = setup();
    let baseline = editor(&s).input.external_baseline();
    editor_mut(&mut s).input.preedit("ni", 2..2, None);
    editor_mut(&mut s).clauses = std::iter::once(0..2).collect();
    let before = format!("{:?}", editor(&s));
    let updated = publish(
        &mut s,
        &version,
        &baseline,
        &draft(1, "local😀"),
        ComponentId(18000),
    )
    .unwrap();
    assert_eq!(updated.revision(), 2);
    assert_eq!(format!("{:?}", editor(&s)), before);
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert!(s.item_keys().contains(&ComponentId(18000)));
}
