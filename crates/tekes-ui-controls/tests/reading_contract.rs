//! Independent caller of the optional public reading policy, with real shaping.
use std::rc::Rc;
use tekes_ui_controls::{ReadingController, TextSurface};
use tekes_ui_scene::{Point, Rect};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::ShapeOptions,
    style::TextStyle,
    traits::Shaper,
};

fn layout(reading: &mut ReadingController, view: &mut TextSurface, width: f32, text_scale: f32) {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    reading.layout_with(
        view,
        Rect::new(20., 20., width, 180.),
        None,
        1.,
        text_scale,
        |s, w| {
            Rc::new(shaper.shape(
                s,
                &TextStyle::system(14. * text_scale),
                &ShapeOptions {
                    width: w.map(f64::from),
                    line_spacing: 0.,
                },
            ))
        },
    );
}
fn document() -> String {
    (0..100)
        .map(|i| {
            format!("Row {i}. 中文 e\u{301} 😀 Reading position remains stable across updates.\n")
        })
        .collect()
}
#[test]
fn history_append_reflow_and_type_scale_preserve_utf16_anchor_and_editor_state() {
    let mut view = TextSurface::new(&document(), true, true);
    let mut reading = ReadingController::default();
    layout(&mut reading, &mut view, 560., 1.);
    view.scroll.scroll_by(0., 713.25);
    view.input.select_range(0..10);
    layout(&mut reading, &mut view, 560., 1.);
    let anchor = view.reflow_anchor.unwrap();
    let selected = view.input.selection();
    view.input.text.push_str("\nNew streamed paragraph 中文😀.");
    for (width, scale) in [(280., 1.), (280., 1.5), (560., 1.), (300., 2.), (560., 1.)] {
        layout(&mut reading, &mut view, width, scale);
        let after = view.reflow_anchor.unwrap();
        assert_eq!(after.utf16, anchor.utf16);
        assert_eq!(after.within_line, anchor.within_line);
        assert_eq!(view.input.selection(), selected);
        assert!(!reading.is_following_latest());
        let relative = view.caret_doc(anchor.utf16).min_y() - view.scroll.offset.y;
        assert!((relative + anchor.within_line).abs() < 0.01);
    }
}
#[test]
fn follow_is_explicit_upward_intent_latches_before_append_and_manual_bottom_never_repins() {
    for reduced in [false, true] {
        let mut view = TextSurface::new(&document(), true, true);
        view.scroll.set_reduced_motion(reduced);
        let mut reading = ReadingController::default();
        layout(&mut reading, &mut view, 400., 1.);
        reading.follow_latest(&mut view);
        view.input.text.push_str("\nA new paragraph.");
        layout(&mut reading, &mut view, 320., 1.);
        assert_eq!(view.scroll.offset.y, view.scroll.limit().y);
        reading.observe_scroll(&mut view, Point::new(0., 1.), true);
        assert!(reading.is_following_latest());
        reading.observe_scroll(&mut view, Point::new(0., 1.), false);
        assert!(!reading.is_following_latest());
        let point = view.inner.center();
        view.wheel_at(
            point,
            Point::new(0., 1.),
            true,
            true,
            false,
            false,
            1.,
            false,
        );
        let history = view.scroll.offset.y;
        view.input.text.push_str("\nArrival before the next frame.");
        layout(&mut reading, &mut view, 320., 1.);
        assert!((view.scroll.offset.y - history).abs() < 0.001);
        view.scroll.scroll_by(0., f32::MAX);
        let manual_bottom = view.scroll.offset.y;
        view.input
            .text
            .push_str("\nTimer or network arrival does not opt in.");
        for _ in 0..3 {
            layout(&mut reading, &mut view, 320., 1.);
        }
        assert_eq!(view.scroll.offset.y, manual_bottom);
        assert!(view.scroll.offset.y < view.scroll.limit().y);
        reading.follow_latest(&mut view);
        layout(&mut reading, &mut view, 320., 1.);
        assert_eq!(view.scroll.offset.y, view.scroll.limit().y);
        assert!(!view.scroll.needs_animation());
    }
}
#[test]
fn explicit_latest_releases_pointer_capture_but_never_edits_marked_text_or_selection() {
    let mut view = TextSurface::new(&document(), true, false);
    let mut reading = ReadingController::default();
    layout(&mut reading, &mut view, 400., 1.);
    view.pointer_down(view.inner.center(), false, 1, 24.);
    assert!(view.pointer_state(None).pressed);
    view.input.preedit("输入", 2..2, None);
    let text = view.input.text.clone();
    let marked = view.input.marked.clone();
    let selection = view.input.selection();
    reading.follow_latest(&mut view);
    assert!(!view.pointer_state(None).pressed);
    layout(&mut reading, &mut view, 300., 1.5);
    assert_eq!(view.input.text, text);
    assert_eq!(view.input.marked, marked);
    assert_eq!(view.input.selection(), selection);
}

use tekes_ui_controls::{
    ReadingAnchorUpdate as A, ReadingSelectionUpdate as S, ReadingUpdateError as E,
    TextRangeMapping as M,
};
fn utf16(s: &str) -> usize {
    s.encode_utf16().count()
}
fn prefix(page: usize) -> String {
    (0..12)
        .map(|n| format!("Earlier {page}/{n}. 中文 😀 e\u{301}.\n"))
        .collect()
}

#[test]
fn versioned_prepend_restores_measured_position_and_selection_within_one_pixel() {
    for width in [280., 560.] {
        let old = document();
        let mut view = TextSurface::new("", true, true);
        let mut reading = ReadingController::default();
        let version = reading
            .set_document(&mut view, "document-A", 10, &old)
            .unwrap();
        layout(&mut reading, &mut view, width, 1.);
        view.scroll.scroll_by(0., 713.25);
        layout(&mut reading, &mut view, width, 1.);
        let anchor = view.reflow_anchor.unwrap();
        let old_y = view.caret_doc(anchor.utf16).min_y() - view.scroll.offset.y;
        let a = utf16(&old[..old.find("中文").unwrap()]);
        view.input.select_range(a..a + 2);
        let selected = view.input.selected_text();
        let before = prefix(1);
        let text = format!("{before}{old}");
        let result = reading
            .update_document(
                &mut view,
                &version,
                11,
                &text,
                &[M {
                    old: 0..utf16(&old),
                    new: utf16(&before)..utf16(&text),
                }],
            )
            .unwrap();
        assert_eq!(result.anchor, A::Mapped);
        assert_eq!(result.selection, S::Preserved);
        assert_eq!(view.input.selected_text(), selected);
        assert_eq!(view.input.anchor, a + utf16(&before));
        layout(&mut reading, &mut view, width, 1.);
        let mapped = anchor.utf16 + utf16(&before);
        let after_y = view.caret_doc(mapped).min_y() - view.scroll.offset.y;
        assert!((old_y - after_y).abs() <= 1., "{old_y} vs {after_y}");
        assert!(!reading.is_following_latest());
    }
}

#[test]
fn two_prepend_updates_before_layout_compose_with_tail_append_and_reflow() {
    let old = document();
    let mut view = TextSurface::new("", true, true);
    let mut reading = ReadingController::default();
    let v = reading.set_document(&mut view, "A", 1, &old).unwrap();
    layout(&mut reading, &mut view, 560., 1.);
    view.scroll.scroll_by(0., 713.25);
    layout(&mut reading, &mut view, 560., 1.);
    let anchor = view.reflow_anchor.unwrap();
    let p1 = prefix(1);
    let first = format!("{p1}{old}\nAppended same batch. 中文😀");
    let r1 = reading
        .update_document(
            &mut view,
            &v,
            2,
            &first,
            &[M {
                old: 0..utf16(&old),
                new: utf16(&p1)..utf16(&p1) + utf16(&old),
            }],
        )
        .unwrap();
    let p2 = prefix(2);
    let second = format!("{p2}{first}");
    let r2 = reading
        .update_document(
            &mut view,
            &r1.version,
            3,
            &second,
            &[M {
                old: 0..utf16(&first),
                new: utf16(&p2)..utf16(&second),
            }],
        )
        .unwrap();
    assert_eq!(r2.anchor, A::Mapped);
    let logical = anchor.utf16 + utf16(&p1) + utf16(&p2);
    for (width, scale) in [(280., 1.5), (560., 1.), (300., 2.), (560., 1.)] {
        layout(&mut reading, &mut view, width, scale);
        assert_eq!(view.reflow_anchor.unwrap().utf16, logical);
        let y = view.caret_doc(logical).min_y() - view.scroll.offset.y;
        assert!((y + anchor.within_line).abs() <= 1.);
        assert!(view.scroll.offset.y < view.scroll.limit().y);
    }
    reading.follow_latest(&mut view);
    let third = format!("{second}\nTail after explicit latest.");
    let r3 = reading
        .update_document(
            &mut view,
            &r2.version,
            4,
            &third,
            &[M {
                old: 0..utf16(&second),
                new: 0..utf16(&second),
            }],
        )
        .unwrap();
    assert_eq!(r3.anchor, A::FollowingLatest);
    layout(&mut reading, &mut view, 560., 1.);
    assert_eq!(view.scroll.offset.y, view.scroll.limit().y);
}

#[test]
fn identity_attachment_revision_and_invalid_mapping_reject_without_mutation() {
    let old = document();
    let mut view = TextSurface::new("", true, true);
    let mut reading = ReadingController::default();
    let a = reading.set_document(&mut view, "A", 1, &old).unwrap();
    reading
        .set_document(&mut view, "B", 1, "Other document")
        .unwrap();
    let new_a = reading.set_document(&mut view, "A", 1, &old).unwrap();
    assert_eq!(
        reading.update_document(&mut view, &a, 2, "late", &[]),
        Err(E::WrongDocument)
    );
    assert_eq!(
        reading.update_document(&mut view, &new_a, 1, "stale", &[]),
        Err(E::StaleRevision)
    );
    assert_eq!(
        reading.update_document(
            &mut view,
            &new_a,
            2,
            "wrong",
            &[M {
                old: 0..3,
                new: 0..3
            }]
        ),
        Err(E::InvalidMapping)
    );
    assert_eq!(view.input.text, old);
    let mut other = ReadingController::default();
    let mut other_view = TextSurface::new("", true, true);
    other.set_document(&mut other_view, "A", 1, &old).unwrap();
    assert_eq!(
        other.update_document(&mut other_view, &new_a, 2, "cross controller", &[]),
        Err(E::WrongDocument)
    );
    layout(&mut reading, &mut view, 400., 1.);
    let v2 = reading
        .update_document(
            &mut view,
            &new_a,
            2,
            &old,
            &[M {
                old: 0..utf16(&old),
                new: 0..utf16(&old),
            }],
        )
        .unwrap()
        .version;
    assert_eq!(
        reading.update_document(&mut view, &new_a, 3, "old callback", &[]),
        Err(E::StaleRevision)
    );
    view.input.text.push_str("external unversioned mutation");
    assert_eq!(
        reading.update_document(&mut view, &v2, 3, "bad refresh", &[]),
        Err(E::UnversionedChange)
    );
}

#[test]
fn deletion_fallback_and_selection_invalidation_are_explicit_not_a_different_selection() {
    let old = document();
    let mut view = TextSurface::new("", true, true);
    let mut reading = ReadingController::default();
    let version = reading.set_document(&mut view, "A", 1, &old).unwrap();
    layout(&mut reading, &mut view, 400., 1.);
    view.scroll.scroll_by(0., 713.25);
    layout(&mut reading, &mut view, 400., 1.);
    let anchor = view.reflow_anchor.unwrap();
    view.input.select_range(anchor.utf16..anchor.utf16 + 3);
    let first = &old[..old.find('\n').unwrap() + 1];
    let r = reading
        .update_document(
            &mut view,
            &version,
            2,
            first,
            &[M {
                old: 0..utf16(first),
                new: 0..utf16(first),
            }],
        )
        .unwrap();
    assert_eq!(r.anchor, A::FallbackStart);
    assert_eq!(r.selection, S::Invalidated);
    assert!(view.input.selection().is_empty());
    layout(&mut reading, &mut view, 400., 1.);
    assert_eq!(view.scroll.offset.y, 0.);
    // An insertion inside a selected span may map both endpoints, but must not
    // silently include new text in the user's old selection.
    let v = reading
        .set_document(&mut view, "C", 1, "中文😀 e\u{301} tail")
        .unwrap();
    view.input.select_all();
    let next = "中文 inserted 😀 e\u{301} tail";
    let r = reading
        .update_document(
            &mut view,
            &v,
            2,
            next,
            &[
                M {
                    old: 0..2,
                    new: 0..2,
                },
                M {
                    old: 2..utf16("中文😀 e\u{301} tail"),
                    new: utf16("中文 inserted ")..utf16(next),
                },
            ],
        )
        .unwrap();
    assert_eq!(r.selection, S::Invalidated);
    assert!(view.input.selection().is_empty());
}

#[test]
fn reading_updates_reject_active_composition_and_editable_surfaces_and_invalid_graphemes() {
    let mut view = TextSurface::new("Composer draft", true, false);
    let mut reading = ReadingController::default();
    assert_eq!(
        reading.set_document(&mut view, "A", 1, "replace"),
        Err(E::EditableSurface)
    );
    view.input.preedit("输入", 2..2, None);
    let composing = view.input.text.clone();
    let marked = view.input.marked.clone();
    assert_eq!(
        reading.set_document(&mut view, "A", 1, "replace"),
        Err(E::CompositionActive)
    );
    assert_eq!(view.input.text, composing);
    assert_eq!(view.input.marked, marked);
    let mut view = TextSurface::new("", true, true);
    let v = reading
        .set_document(&mut view, "A", 1, "😀e\u{301}")
        .unwrap();
    assert_eq!(
        reading.update_document(
            &mut view,
            &v,
            2,
            "😀e\u{301}",
            &[M {
                old: 1..4,
                new: 1..4
            }]
        ),
        Err(E::InvalidMapping)
    );
    assert_eq!(
        reading.update_document(
            &mut view,
            &v,
            2,
            "😀e\u{301}",
            &[M {
                old: 2..3,
                new: 2..3
            }]
        ),
        Err(E::InvalidMapping)
    );
}

#[test]
fn selection_requires_retained_interval_coverage_and_keeps_left_affinity_at_its_end() {
    let mut reading = ReadingController::default();
    let mut view = TextSurface::new("", true, true);
    let v = reading.set_document(&mut view, "A", 1, "abcDEF").unwrap();
    view.input.select_all();
    let r = reading
        .update_document(
            &mut view,
            &v,
            2,
            "abcDEF",
            &[
                M {
                    old: 0..2,
                    new: 0..2,
                },
                M {
                    old: 4..6,
                    new: 4..6,
                },
            ],
        )
        .unwrap();
    assert_eq!(
        r.selection,
        S::Invalidated,
        "identical bytes cannot pretend that unmapped selected content retained its identity"
    );
    let v = reading.set_document(&mut view, "A", 3, "abcDEF").unwrap();
    view.input.anchor = 3;
    view.input.caret = 0;
    let r = reading
        .update_document(
            &mut view,
            &v,
            4,
            "abc + DEF",
            &[
                M {
                    old: 0..3,
                    new: 0..3,
                },
                M {
                    old: 3..6,
                    new: 6..9,
                },
            ],
        )
        .unwrap();
    assert_eq!(r.selection, S::Preserved);
    assert_eq!(view.input.selected_text(), "abc");
    assert_eq!((view.input.anchor, view.input.caret), (3, 0));
}

#[test]
fn truncating_to_retained_anchor_uses_existing_legal_scroll_clamp() {
    let old = document();
    let mut reading = ReadingController::default();
    let mut view = TextSurface::new("", true, true);
    let v = reading.set_document(&mut view, "A", 1, &old).unwrap();
    layout(&mut reading, &mut view, 560., 1.);
    view.scroll.scroll_by(0., 713.25);
    layout(&mut reading, &mut view, 560., 1.);
    let anchor = view.reflow_anchor.unwrap();
    let start_byte = old
        .char_indices()
        .find(|(b, _)| utf16(&old[..*b]) == anchor.utf16)
        .unwrap()
        .0;
    let line = &old[start_byte..start_byte + old[start_byte..].find('\n').unwrap() + 1];
    let r = reading
        .update_document(
            &mut view,
            &v,
            2,
            line,
            &[M {
                old: anchor.utf16..anchor.utf16 + utf16(line),
                new: 0..utf16(line),
            }],
        )
        .unwrap();
    assert_eq!(r.anchor, A::Mapped);
    layout(&mut reading, &mut view, 560., 1.);
    assert_eq!(view.scroll.offset.y, 0.);
    assert_eq!(view.scroll.limit().y, 0.);
    assert_eq!(view.reflow_anchor.unwrap().utf16, 0);
    assert!(view.caret_doc(0).min_y() < view.inner.size.height);
}
