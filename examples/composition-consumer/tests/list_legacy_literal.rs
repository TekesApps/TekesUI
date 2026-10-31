use tekes_ui_controls::{ControlKey, extended::List};
#[test]
fn independent_public_consumer_preserves_literal_destructure_new_and_legacy_navigation() {
    let mut literal = List {
        selected: Some(1),
        count: 3,
    };
    let copy = literal.clone();
    let List { selected, count } = copy;
    assert_eq!((selected, count), (Some(1), 3));
    assert!(literal.key(ControlKey::End));
    assert_eq!(
        literal,
        List {
            selected: Some(2),
            count: 3
        }
    );
    assert!(!literal.select(3));
    literal.count = 0;
    literal.selected = None;
    assert!(!literal.key(ControlKey::Home));
    let mut fresh = List::new(2);
    assert!(fresh.key(ControlKey::Down));
    assert_eq!(fresh.selected, Some(0));
}

#[test]
fn legacy_selectable_list_fields_and_constructor_remain_public() {
    use tekes_ui_controls::{collections_view::ListView, composition::SelectableListContent};

    let literal = SelectableListContent {
        model: List {
            selected: Some(0),
            count: 1,
        },
        view: ListView::new(),
        labels: vec!["One".into()],
        name: "Legacy".into(),
    };
    assert_eq!(literal.model.selected, Some(0));
    assert_eq!(literal.labels, ["One"]);
    let constructed = SelectableListContent::new("New", vec!["Two".into()]);
    assert_eq!(constructed.model.count, 1);
    assert_eq!(constructed.name, "New");
}
