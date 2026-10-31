use super::*;

#[cfg(test)]
mod tree_protocol_depth_tests {
    use super::*;

    fn s(role: wire::SemanticRole) -> wire::Semantics {
        wire::Semantics {
            role,
            name: "Tree".into(),
            value: None,
            disabled: false,
            selected: false,
            expanded: None,
            invalid: false,
            required: false,
            description: None,
        }
    }

    fn node() -> wire::SemanticNode {
        wire::SemanticNode {
            target: wire::Target { id: 1, mount: 1 },
            parent: None,
            frame: [0., 0., 100., 100.],
            visible: Some([0., 0., 100., 100.]),
            focused: false,
            semantics: s(wire::SemanticRole::Tree),
            preserves_activation_focus: false,
            parts: vec![],
            value_range: None,
            text_binding: None,
            text: None,
            text_layout: None,
        }
    }

    fn part(index: usize, parent: Option<usize>, level: usize) -> wire::SemanticPart {
        wire::SemanticPart {
            index,
            parent,
            frame: [0., 0., 0., 0.],
            focused: false,
            semantics: s(wire::SemanticRole::TreeItem { level }),
            preserves_activation_focus: false,
            text_layout: None,
            value_range: None,
        }
    }

    #[test]
    fn tree_depth_must_follow_the_actual_parent_and_never_overflow() {
        let mut n = node();
        n.parts = vec![
            part(10, None, 0),
            part(20, Some(10), 1),
            part(30, Some(20), 2),
        ];
        assert!(validate(&[n.clone()]).is_ok());
        for invalid in [0, 1, 3, usize::MAX] {
            n.parts[2].semantics.role = wire::SemanticRole::TreeItem { level: invalid };
            assert!(validate(&[n.clone()]).is_err());
        }
        n.parts[2].semantics.role = wire::SemanticRole::TreeItem { level: 2 };
        n.parts[2].parent = Some(10);
        assert!(validate(&[n.clone()]).is_err());
        n.parts[2].parent = Some(999);
        assert!(validate(&[n.clone()]).is_err());
        n.parts = vec![part(10, None, usize::MAX), part(20, Some(10), 0)];
        assert!(validate(&[n]).is_err());
    }

    #[test]
    fn nested_process_projection_and_zero_area_parent_keep_tree_chain() {
        let mut n = node();
        n.semantics = s(wire::SemanticRole::Group);
        let mut root = part(1, None, 0);
        root.semantics = s(wire::SemanticRole::Tree);
        n.parts = vec![root, part(2, Some(1), 0), part(3, Some(2), 1)];
        assert!(validate(&[n.clone()]).is_ok());
        n.parts[1].parent = None;
        assert!(validate(&[n]).is_err());
    }
}

#[cfg(test)]
mod issue111_table_wire_tests {
    use super::*;

    fn node() -> wire::SemanticNode {
        use wire::SemanticRole as W;

        let sem = |role| wire::Semantics {
            role,
            name: "Table".into(),
            value: None,
            disabled: false,
            selected: false,
            expanded: None,
            invalid: false,
            required: false,
            description: None,
        };
        let part = |index, parent, role| wire::SemanticPart {
            index,
            parent,
            frame: [0., 0., 30., 20.],
            focused: false,
            semantics: sem(role),
            preserves_activation_focus: false,
            text_layout: None,
            value_range: None,
        };

        wire::SemanticNode {
            target: wire::Target { id: 1, mount: 1 },
            parent: None,
            frame: [0., 0., 100., 80.],
            visible: Some([0., 0., 100., 80.]),
            focused: false,
            semantics: sem(W::Table),
            preserves_activation_focus: false,
            text_binding: None,
            text: None,
            text_layout: None,
            value_range: None,
            parts: vec![
                part(10, None, W::TableColumn { index: 0 }),
                part(
                    11,
                    Some(10),
                    W::TableColumnHeader {
                        index: 0,
                        sort: None,
                    },
                ),
                part(12, None, W::TableRow { index: 0 }),
                part(13, Some(12), W::TableCell { row: 0, column: 0 }),
            ],
        }
    }

    #[test]
    fn receiver_accepts_table_and_rejects_broken_finite_structure() {
        use wire::SemanticRole as W;

        let good = node();
        validate(std::slice::from_ref(&good)).unwrap();
        assert_eq!(
            semantics(&good.parts[1].semantics).role,
            ContentRole::TableColumnHeader {
                index: 0,
                sort: None
            }
        );
        for mode in 0..7 {
            let mut bad = good.clone();
            match mode {
                0 => {
                    bad.parts.remove(1);
                }
                1 => bad.parts[1].parent = Some(99),
                2 => bad.parts[0].semantics.role = W::TableColumn { index: usize::MAX },
                3 => bad.parts[3].semantics.role = W::TableCell { row: 1, column: 0 },
                4 => bad.parts[3].semantics.role = W::TableRow { index: 0 },
                5 => {
                    bad.parts[2].frame = [0.; 4];
                    bad.parts[2].focused = true;
                }
                _ => bad.parts[1].index = bad.parts[0].index,
            }
            assert!(
                validate(&[bad]).is_err(),
                "invalid Table structure mode {mode}"
            );
        }
    }

    #[test]
    fn remote_table_focus_uses_final_parent_clip_and_keeps_identity_tree() {
        use crate::native_content::semantic_test_snapshot;

        let mut input = node();
        input.parts[1].focused = true;
        let mut remote = RemoteSemantics::default();
        remote.update(semantic_test_snapshot(vec![input])).unwrap();

        let frame = Rect::new(20.0, 30.0, 100.0, 80.0);
        let full = remote.parts(frame, frame, true);
        assert_eq!(full.len(), 5);
        let root = full
            .iter()
            .find(|part| part.semantics.role == ContentRole::Table)
            .unwrap();
        let column = full
            .iter()
            .find(|part| matches!(part.semantics.role, ContentRole::TableColumn { index: 0 }))
            .unwrap();
        let header = full
            .iter()
            .find(|part| {
                matches!(
                    part.semantics.role,
                    ContentRole::TableColumnHeader { index: 0, .. }
                )
            })
            .unwrap();
        let row = full
            .iter()
            .find(|part| matches!(part.semantics.role, ContentRole::TableRow { index: 0 }))
            .unwrap();
        let cell = full
            .iter()
            .find(|part| {
                matches!(
                    part.semantics.role,
                    ContentRole::TableCell { row: 0, column: 0 }
                )
            })
            .unwrap();
        assert!(!header.frame.is_empty() && header.focused);
        assert_eq!(remote.parent(column.index), Some(root.index));
        assert_eq!(remote.parent(header.index), Some(column.index));
        assert_eq!(remote.parent(row.index), Some(root.index));
        assert_eq!(remote.parent(cell.index), Some(row.index));
        let identities = full.iter().map(|part| part.index).collect::<Vec<_>>();

        let clipped = remote.parts(frame, Rect::new(20.0, 100.0, 100.0, 20.0), true);
        assert_eq!(clipped.len(), 5);
        assert_eq!(
            clipped.iter().map(|part| part.index).collect::<Vec<_>>(),
            identities
        );
        let clipped_header = clipped
            .iter()
            .find(|part| part.index == header.index)
            .unwrap();
        assert!(clipped_header.frame.is_empty());
        assert!(!clipped_header.focused);
        assert_eq!(remote.parent(clipped_header.index), Some(column.index));

        let revealed = remote.parts(frame, frame, true);
        let revealed_header = revealed
            .iter()
            .find(|part| part.index == header.index)
            .unwrap();
        assert!(!revealed_header.frame.is_empty() && revealed_header.focused);
        assert_eq!(remote.parent(revealed_header.index), Some(column.index));
        assert_eq!(
            revealed.iter().map(|part| part.index).collect::<Vec<_>>(),
            identities
        );
    }
}
