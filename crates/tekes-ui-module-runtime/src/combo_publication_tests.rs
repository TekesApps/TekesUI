use super::*;
use crate::native_content::semantic_test_snapshot;
fn node(id: u64) -> wire::SemanticNode {
    let target = wire::Target {
        id,
        mount: id + 100,
    };
    let input = wire::TextState {
        owner: target,
        text: "A😀B".into(),
        caret_index: 3,
        selection: 1..3,
        marked: None,
        read_only: true,
        caret: [0., 0., 1., 20.],
        clip: [0., 0., 100., 20.],
        carets: vec![],
    };
    wire::SemanticNode {
        target,
        parent: None,
        frame: [0., 0., 200., 40.],
        visible: Some([0., 0., 200., 40.]),
        focused: false,
        semantics: wire::Semantics {
            role: wire::SemanticRole::ComboBox,
            name: "Query".into(),
            value: Some(input.text.clone()),
            disabled: false,
            selected: false,
            expanded: None,
            invalid: false,
            required: false,
            description: None,
        },
        preserves_activation_focus: false,
        parts: vec![],
        value_range: None,
        text_binding: Some(vec![1]),
        text: Some(input.clone()),
        text_layout: Some(wire::TextLayout {
            binding: vec![1],
            input,
            multiline: false,
            lines: std::iter::once(0..4).collect(),
            carets: vec![(0, [0., 0., 1., 20.]), (4, [30., 0., 1., 20.])],
        }),
    }
}
fn frame() -> Rect {
    Rect::new(0., 0., 200., 80.)
}
fn index(r: &RemoteSemantics) -> usize {
    r.parts(frame(), frame(), false)[0].index
}
fn closed(r: &RemoteSemantics, old: usize) {
    assert!(r.needs_refresh());
    assert!(r.parts(frame(), frame(), false).is_empty());
    assert!(r.callback(old).is_none());
    assert!(r.text_layout(old, frame(), frame()).is_none());
    assert!(r.parent(old).is_none());
    assert!(!r.preserves_focus(old));
}
#[test]
fn malformed_b_revokes_a_without_publishing_valid_prefix_and_c_never_revives_old_targets() {
    let mut r = RemoteSemantics::default();
    r.update(semantic_test_snapshot(vec![node(1)])).unwrap();
    let a = index(&r);
    assert!(r.callback(a).is_some());
    assert!(r.text_layout(a, frame(), frame()).is_some());
    let next = r.next;
    let mut malformed = node(3);
    malformed.frame[2] = -1.;
    assert!(
        r.update(semantic_test_snapshot(vec![node(1), node(2), malformed]))
            .is_err()
    );
    closed(&r, a);
    assert!(r.entries.is_empty());
    assert!(r.snapshot.is_none());
    assert!(r.attachment.is_none());
    assert_eq!(r.next, next);
    // Even reusing the same remote target cannot revive the old projected ID.
    r.update(semantic_test_snapshot(vec![node(1)])).unwrap();
    let c = index(&r);
    assert!(c > a);
    assert!(r.callback(a).is_none());
    assert!(r.text_layout(a, frame(), frame()).is_none());
    assert!(r.callback(c).is_some());
    r.revoke();
    closed(&r, c);
    r.update(semantic_test_snapshot(vec![node(1)])).unwrap();
    assert!(index(&r) > c);
    assert!(r.callback(c).is_none());
}
#[test]
fn capacity_failure_is_dirty_without_prefix_wrap_or_partial_allocator_commit() {
    let mut r = RemoteSemantics::default();
    r.update(semantic_test_snapshot(vec![node(1)])).unwrap();
    let a = index(&r);
    // One candidate allocation succeeds locally; the next would overflow.
    r.next = usize::MAX - 1;
    let old_entries = format!("{:?}", r.entries);
    assert!(
        r.update(semantic_test_snapshot(vec![node(1), node(2), node(3)]))
            .is_err()
    );
    closed(&r, a);
    assert!(r.dirty);
    assert_eq!(r.next, usize::MAX - 1);
    assert_eq!(format!("{:?}", r.entries), old_entries);
    r.update(semantic_test_snapshot(vec![node(4)])).unwrap();
    let last = index(&r);
    assert_eq!(last, usize::MAX);
    assert!(r.callback(a).is_none());
    r.revoke();
    closed(&r, last);
    assert_eq!(r.next, usize::MAX);
    assert!(r.update(semantic_test_snapshot(vec![node(5)])).is_err());
    closed(&r, last);
    assert_eq!(r.next, usize::MAX);
}
#[test]
fn malformed_combo_multiline_rejects_without_relabeling_and_valid_layout_retains_owner() {
    let mut n = node(1);
    n.text_layout.as_mut().unwrap().multiline = true;
    assert!(validate(std::slice::from_ref(&n)).is_err());
    n.text_layout.as_mut().unwrap().multiline = false;
    validate(std::slice::from_ref(&n)).unwrap();
    let mut r = RemoteSemantics::default();
    r.update(semantic_test_snapshot(vec![n])).unwrap();
    let t = r.text_layout(index(&r), frame(), frame()).unwrap();
    assert_eq!(t.input.binding, vec![1]);
    assert_eq!(t.input.selection, 1..3);
    assert_eq!(t.input.text, "A😀B");
    assert!(t.input.read_only);
    assert!(!t.multiline);
}

#[test]
fn remote_combo_part_keeps_full_original_layout_and_option_parent_without_extra_editor() {
    let mut n = node(1);
    let layout = n.text_layout.take().unwrap();
    let mut combo = n.semantics.clone();
    combo.expanded = Some(true);
    n.semantics.role = wire::SemanticRole::Group;
    n.semantics.value = None;
    n.text = None;
    n.text_binding = None;
    let mut option = combo.clone();
    option.role = wire::SemanticRole::MenuItem;
    option.name = String::new();
    option.value = None;
    option.expanded = None;
    n.parts = vec![
        wire::SemanticPart {
            index: 4,
            parent: None,
            frame: n.frame,
            focused: false,
            semantics: combo,
            preserves_activation_focus: false,
            text_layout: Some(layout.clone()),
            value_range: None,
        },
        wire::SemanticPart {
            index: 5,
            parent: Some(4),
            frame: [0., 40., 100., 20.],
            focused: false,
            semantics: option,
            preserves_activation_focus: true,
            text_layout: None,
            value_range: None,
        },
    ];
    validate(std::slice::from_ref(&n)).unwrap();
    let mut r = RemoteSemantics::default();
    r.update(semantic_test_snapshot(vec![n.clone()])).unwrap();
    let parts = r.parts(frame(), frame(), false);
    let combo = parts
        .iter()
        .find(|p| p.semantics.role == ContentRole::ComboBox)
        .unwrap();
    let option = parts
        .iter()
        .find(|p| p.semantics.role == ContentRole::MenuItem)
        .unwrap();
    assert_eq!(r.parent(option.index), Some(combo.index));
    assert!(r.preserves_focus(option.index));
    assert!(r.text_layout(option.index, frame(), frame()).is_none());
    let t = r.text_layout(combo.index, frame(), frame()).unwrap();
    assert_eq!(t.input.binding, layout.binding);
    assert_eq!(t.lines, layout.lines);
    assert_eq!(t.input.selection, layout.input.selection);
    assert_eq!(t.input.marked, layout.input.marked);
    let old = combo.index;
    n.parts[0].text_layout.as_mut().unwrap().multiline = true;
    assert!(r.update(semantic_test_snapshot(vec![n])).is_err());
    closed(&r, old);
}
