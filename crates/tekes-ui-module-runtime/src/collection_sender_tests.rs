use super::*;

mod tree_wire_tests {
    use super::*;
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer,
        composition::*,
        extended::{KeyedTreeNode, TreeDatasetKey, TreeNodeKey},
        layout_view::IconRenderer,
    };
    use tekes_ui_scene::{ImageId, Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
    struct Icons;
    impl IconRenderer for Icons {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
        fn prepare_icon(
            &mut self,
            _: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(ImageId(1), std::rc::Rc::new(())))
        }
    }
    struct Text(Icons);
    impl ButtonTextRenderer for Text {
        fn icon_renderer(&mut self) -> Option<&mut dyn IconRenderer> {
            Some(&mut self.0)
        }
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new((s.len() as f32 * 8.).min(w), 20.)
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
    #[test]
    fn tree_wire_snapshot_and_native_platform_disclosure() {
        let nodes = vec![(1, None), (2, Some(1))]
            .into_iter()
            .map(|(k, p)| TreeContentNode {
                row_id: ComponentId(100 + k),
                node: KeyedTreeNode {
                    key: TreeNodeKey(k),
                    parent: p.map(TreeNodeKey),
                    label: format!("node{k}"),
                    disabled: false,
                    initially_expanded: true,
                },
                display: TreeDisplayUpdate::Keep,
            })
            .collect();
        let mut h = ContentHost::new(Content::new(
            ComponentId(10),
            TreeContent::new("Tree", TreeDatasetKey(1), nodes).unwrap(),
        ))
        .unwrap();
        let f = Rect::new(0., 0., 320., 200.);
        h.render(
            &mut Scene::new(),
            &mut Text(Icons),
            f,
            f,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let snap = snapshot(&h).expect("public Tree must publish through module runtime");
        assert_eq!(snap[0].parts.len(), 2);
        let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
        assert!(
            native
                .iter()
                .any(|n| n.role == tekes_ui_platform_macos::AccessibilityRole::Tree)
        );
        let root = native.iter().find(|n| n.label == "node1").unwrap().key;
        assert!(
            tekes_ui_platform_macos::content_accessibility::content_action(
                &mut h,
                root,
                tekes_ui_platform_macos::AccessibilityAction::SetExpanded(false)
            )
            .changed
        );
        assert!(
            h.semantics()
                .iter()
                .find(|n| n.id == ComponentId(10))
                .unwrap()
                .parts
                .is_empty(),
            "mutation invalidates old geometry"
        );
    }
}

mod datatable_wire_tests {
    use super::*;
    use tekes_ui_controls::extended::SortDirection;
    #[test]
    fn sender_preserves_table_role_values_and_sort() {
        let cases = [
            (ContentRole::Table, wire::SemanticRole::Table),
            (
                ContentRole::TableColumn { index: 3 },
                wire::SemanticRole::TableColumn { index: 3 },
            ),
            (
                ContentRole::TableColumnHeader {
                    index: 3,
                    sort: Some(SortDirection::Descending),
                },
                wire::SemanticRole::TableColumnHeader {
                    index: 3,
                    sort: Some(wire::TableSortDirection::Descending),
                },
            ),
            (
                ContentRole::TableRow { index: 2 },
                wire::SemanticRole::TableRow { index: 2 },
            ),
            (
                ContentRole::TableCell { row: 2, column: 3 },
                wire::SemanticRole::TableCell { row: 2, column: 3 },
            ),
        ];
        for (role, expected) in cases {
            assert_eq!(
                semantics(ContentSemantics::new(role, "table part"))
                    .unwrap()
                    .role,
                expected
            );
        }
        for (sort, expected) in [
            (
                Some(SortDirection::Ascending),
                Some(wire::TableSortDirection::Ascending),
            ),
            (
                Some(SortDirection::Descending),
                Some(wire::TableSortDirection::Descending),
            ),
            (None, None),
        ] {
            assert_eq!(
                semantics(ContentSemantics::new(
                    ContentRole::TableColumnHeader { index: 0, sort },
                    "header"
                ))
                .unwrap()
                .role,
                wire::SemanticRole::TableColumnHeader {
                    index: 0,
                    sort: expected
                }
            );
        }
    }
}

mod datatable_content_snapshot_tests {
    use super::*;
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer,
        composition::*,
        extended::{
            KeyedTableColumn, KeyedTableRow, KeyedTableSnapshot, TableBudget, TableColumnKey,
            TableDatasetKey, TableRowKey,
        },
    };
    use tekes_ui_scene::{Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, text: &str, _: proposal::TextRole, width: f32) -> Size {
            Size::new((text.chars().count() as f32 * 8.).min(width), 20.)
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

    #[test]
    fn original_datatable_snapshot_roundtrips_all_structural_roles_without_text_owner() {
        let input = KeyedTableSnapshot {
            dataset: TableDatasetKey(1),
            columns: vec![KeyedTableColumn {
                key: TableColumnKey(1),
                name: "Name".into(),
            }],
            rows: vec![KeyedTableRow {
                key: TableRowKey(1),
                name: "Entity 1".into(),
                cells: vec!["Ready".into()],
            }],
            display_rows: vec![TableRowKey(1)],
        };
        let mut displays = TableDisplays {
            headers: vec![TableHeaderDisplay {
                key: TableColumnKey(1),
                display: TableDisplayUpdate::Keep,
            }],
            rows: vec![TableRowDisplay {
                key: TableRowKey(1),
                id: ComponentId(101),
                replace_wrapper: false,
                cells: vec![TableCellDisplay {
                    column: TableColumnKey(1),
                    display: TableDisplayUpdate::Keep,
                    accessory: TableDisplayUpdate::Keep,
                }],
            }],
        };
        let table = DataTableContent::new(
            "Public data",
            &input,
            &mut displays,
            TableBudget {
                max_units: 100_000,
                max_utf8_bytes: 100_000_000,
            },
        )
        .unwrap();
        let mut host = ContentHost::new(Content::new(ComponentId(10), table)).unwrap();
        let frame = Rect::new(0., 0., 320., 200.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();

        let actual = snapshot(&host).expect("original DataTableContent snapshot");
        let bytes = serde_json::to_vec(&actual).unwrap();
        let received: Vec<wire::SemanticNode> = serde_json::from_slice(&bytes).unwrap();
        crate::container_semantics::validate(&received).unwrap();
        assert_eq!(received.len(), 1);
        let table = &received[0];
        assert_eq!(table.semantics.role, wire::SemanticRole::Table);
        assert_eq!(table.parts.len(), 4);
        let roles = table
            .parts
            .iter()
            .map(|part| part.semantics.role)
            .collect::<Vec<_>>();
        assert_eq!(
            roles,
            vec![
                wire::SemanticRole::TableColumn { index: 0 },
                wire::SemanticRole::TableColumnHeader {
                    index: 0,
                    sort: None,
                },
                wire::SemanticRole::TableRow { index: 0 },
                wire::SemanticRole::TableCell { row: 0, column: 0 },
            ]
        );
        assert_eq!(table.parts[1].semantics.name, "Name");
        assert_eq!(table.parts[3].semantics.value.as_deref(), Some("Ready"));
        assert!(table.text.is_none());
        assert!(table.text_binding.is_none());
        assert!(table.text_layout.is_none());
        assert!(table.parts.iter().all(|part| part.text_layout.is_none()));

        let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&host);
        use tekes_ui_platform_macos::AccessibilityRole as NativeRole;
        let native_table = native
            .iter()
            .find(|node| node.role == NativeRole::Table)
            .unwrap();
        assert!(!native_table.focused);
        assert!(!native_table.read_only);
        let column = native
            .iter()
            .find(|node| matches!(node.role, NativeRole::TableColumn { .. }))
            .unwrap();
        let cell = native
            .iter()
            .find(|node| matches!(node.role, NativeRole::TableCell { .. }))
            .unwrap();
        let header = native
            .iter()
            .find(|node| matches!(node.role, NativeRole::TableColumnHeader { .. }))
            .unwrap();
        let row = native
            .iter()
            .find(|node| matches!(node.role, NativeRole::TableRow { .. }))
            .unwrap();
        assert!(column.read_only && !column.focused);
        assert!(cell.read_only && !cell.focused);
        assert!(!header.read_only);
        assert!(!row.read_only);

        let before = serde_json::to_vec(&snapshot(&host).unwrap()).unwrap();
        for key in [column.key, cell.key] {
            let response = tekes_ui_platform_macos::content_accessibility::content_action(
                &mut host,
                key,
                tekes_ui_platform_macos::AccessibilityAction::Press,
            );
            assert!(!response.consumed && !response.activated && !response.changed);
        }
        assert_eq!(
            serde_json::to_vec(&snapshot(&host).unwrap()).unwrap(),
            before
        );
    }
}
