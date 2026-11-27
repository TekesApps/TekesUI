#![cfg(target_os = "macos")]
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_macos_bridge::content_accessibility::*;
use tekes_ui_platform_macos::{AccessibilityAction, AccessibilityRole};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * role.style().size).min(width),
            role.style().line_height,
        )
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
impl TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut s = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(s.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
fn setup() -> ContentHost {
    let text = TextFieldContent::read_only_content(
        "History",
        "A😀B\nsecond line\nlast",
        proposal::TextRole::UiBody,
    );
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        ListContent::new("Root", vec![Content::new(ComponentId(2), text)]),
    ))
    .unwrap();
    let f = Rect::new(30., 40., 240., 160.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    h
}
#[test]
fn complete_original_lines_surrogates_and_unfocused_readonly_are_projected() {
    let h = setup();
    let key = content_key(&h, ComponentId(2), None).unwrap();
    let node = content_nodes(&h)
        .into_iter()
        .find(|n| n.key == key)
        .unwrap();
    assert_eq!(node.role, AccessibilityRole::TextArea);
    assert!(node.read_only && !node.focused);
    let layouts = content_text_layouts(&h);
    let l = &layouts[&key];
    assert_eq!(l.range_for_line(0), Some(0..5));
    assert_eq!(l.range_for_line(1), Some(5..17));
    assert_eq!(l.range_for_index(2), Some(1..3));
    assert_eq!(l.line_for_index(5), Some(1));
    let original = h
        .semantic_text_layout_snapshot(ComponentId(2), None)
        .unwrap();
    assert_eq!(l.lines, original.lines);
    assert_eq!(l.state.carets.len(), original.carets.len());
}
#[test]
fn query_is_readonly_and_inactive_removed_mounts_cannot_receive_actions() {
    let mut h = setup();
    let key = content_key(&h, ComponentId(2), None).unwrap();
    let before = (
        h.focused(),
        h.focus_epoch(),
        h.pointer_capture(),
        h.needs_render(),
    );
    let _ = content_nodes(&h);
    let _ = content_text_layouts(&h);
    assert_eq!(
        before,
        (
            h.focused(),
            h.focus_epoch(),
            h.pointer_capture(),
            h.needs_render()
        )
    );
    assert!(content_action(&mut h, key, AccessibilityAction::SetSelection(1..3)).consumed);
    assert_eq!(
        h.semantic_text_layout_snapshot(ComponentId(2), None)
            .unwrap()
            .input
            .selection,
        1..3
    );
    h.set_window_active(false);
    assert!(content_nodes(&h).is_empty() && content_text_layouts(&h).is_empty());
    assert!(!content_action(&mut h, key, AccessibilityAction::SetSelection(0..1)).consumed);
    h.set_window_active(true);
    h.remove(ComponentId(2)).unwrap();
    h.insert(
        ComponentId(1),
        Content::new(ComponentId(2), TextFieldContent::new("Replacement", "new")),
    )
    .unwrap();
    assert_ne!(content_key(&h, ComponentId(2), None), Some(key));
    assert!(!content_action(&mut h, key, AccessibilityAction::SetValue("stale".into())).consumed);
}
#[test]
fn stale_unshaped_text_is_not_published_as_old_native_lines() {
    let mut h = setup();
    let original = h
        .get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    original.replace_value(
        "same length different shape",
        tekes_ui_controls::ExternalSelection::Reset,
        tekes_ui_controls::ExternalUndo::Reset,
    );
    assert!(
        h.semantic_text_layout_snapshot(ComponentId(2), None)
            .is_none()
    );
    assert!(content_text_layouts(&h).is_empty());
}

#[test]
fn disabled_original_text_keeps_its_native_role_but_rejects_action() {
    let mut h = setup();
    h.get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .set_disabled(true);
    let key = content_key(&h, ComponentId(2), None).unwrap();
    let node = content_nodes(&h)
        .into_iter()
        .find(|n| n.key == key)
        .unwrap();
    assert_eq!(node.role, AccessibilityRole::TextArea);
    assert!(!node.enabled);
    assert!(content_text_layouts(&h).contains_key(&key));
    assert!(!content_action(&mut h, key, AccessibilityAction::SetSelection(1..3)).consumed);
}

#[test]
fn soft_wrap_and_offscreen_lines_keep_the_original_shaper_ranges() {
    let mut h = setup();
    let f = Rect::new(25., 35., 62., 40.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    let key = content_key(&h, ComponentId(2), None).unwrap();
    let original = h
        .semantic_text_layout_snapshot(ComponentId(2), None)
        .unwrap();
    let layouts = content_text_layouts(&h);
    let l = &layouts[&key];
    assert!(l.lines.len() > 3);
    assert_eq!(l.lines, original.lines);
    assert!(original.carets.len() > original.input.carets.len());
    assert_eq!(
        l.range_for_line(l.lines.len() - 1),
        original.lines.last().cloned()
    );
}

#[test]
fn scrolled_inspector_key_value_table_projects_consistently_without_quadratic_work() {
    let essay = "Morning light crossed the curtain and left a slowly moving edge on the floor. \
         The staircase, the small table, and every shelf held notes tucked between pages.\n"
        .repeat(36);
    assert!(essay.len() > 4500);
    let mut table_rows = Vec::new();
    for i in 0..35 {
        let value = match i {
            4 => format!("ContentSemantics {{ role: TextInput, text: {:?} }}", essay),
            12 => essay.clone(),
            _ => format!("Inspector row value #{i} with enough detail to wrap naturally"),
        };
        table_rows.push((
            Content::new(
                ComponentId(10_010 + i * 2),
                TextContent::new(format!("Field {i}")),
            ),
            Content::new(
                ComponentId(10_011 + i * 2),
                TextFieldContent::read_only_content(
                    format!("Field {i}"),
                    &value,
                    proposal::TextRole::UiBody,
                ),
            ),
        ));
    }
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        ScrollContent::new(
            tekes_ui_controls::ScrollArea::default(),
            Content::new(
                ComponentId(2),
                StackContent::vertical(
                    8.,
                    vec![Content::new(
                        ComponentId(3),
                        KeyValueTableContent::new(table_rows),
                    )],
                ),
            ),
        )
        .scrollbar(true),
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 360., 640.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let baseline = ContentAccessibilitySnapshot::from_host(&h).expect("initial AX snapshot");
    let baseline_keys: Vec<_> = baseline.nodes().iter().map(|n| n.key).collect();
    let baseline_layouts = baseline.text_layouts().clone();
    assert_eq!(baseline_layouts.len(), 35);

    let start = std::time::Instant::now();
    for step in 1..=10 {
        h.event(&ContentEvent::Scroll {
            position: frame.center(),
            delta: tekes_ui_scene::Point::new(0., 120.),
            precise: true,
            begin: step == 1,
            end: step == 10,
            momentum: false,
            time: step as f64 * 0.016,
        });
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let snap = ContentAccessibilitySnapshot::from_host(&h).expect("scrolled AX snapshot");
        let keys: Vec<_> = snap.nodes().iter().map(|n| n.key).collect();
        assert_eq!(keys, baseline_keys);
        assert_eq!(snap.text_layouts().len(), baseline_layouts.len());
        for (k, layout) in &baseline_layouts {
            assert_eq!(snap.text_layouts()[k].lines, layout.lines);
        }
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(1500),
        "10 scrolled Inspector AX snapshots took {elapsed:?}"
    );
    let essay_surface = h.text_surface(ComponentId(10_011 + 12 * 2)).unwrap();
    let visible_input =
        tekes_ui_controls::composition::ContentTextInput::from_surface(essay_surface);
    let full_layout = h
        .semantic_text_layout_snapshot(ComponentId(10_011 + 12 * 2), None)
        .unwrap();
    assert!(
        visible_input.carets.len() < full_layout.carets.len(),
        "clipped ContentTextInput must only project carets for visible lines ({} vs {})",
        visible_input.carets.len(),
        full_layout.carets.len()
    );
}

#[test]
fn disabled_datatable_keeps_header_column_and_cell_row_ax_parent_chains() {
    use tekes_ui_controls::extended::{
        KeyedTableColumn, KeyedTableRow, KeyedTableSnapshot, TableBudget, TableColumnKey,
        TableDatasetKey, TableRowKey,
    };
    let snapshot = KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: vec![KeyedTableColumn {
            key: TableColumnKey(1),
            name: "Col".into(),
        }],
        rows: vec![KeyedTableRow {
            key: TableRowKey(1),
            name: "Row".into(),
            cells: vec!["Val".into()],
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
        "DisabledChainTable",
        &snapshot,
        &mut displays,
        TableBudget {
            max_units: 1000,
            max_utf8_bytes: 100_000,
        },
    )
    .unwrap();
    let mut h = ContentHost::new(Content::new(ComponentId(10), table)).unwrap();
    let frame = Rect::new(0., 0., 240., 160.);

    for disabled in [false, true, false] {
        h.get_mut(ComponentId(10))
            .unwrap()
            .component_mut::<DataTableContent>()
            .unwrap()
            .set_disabled(disabled)
            .unwrap();
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let nodes = content_nodes(&h);
        let table_node = nodes
            .iter()
            .find(|n| matches!(n.role, AccessibilityRole::Table))
            .unwrap();
        let col_node = nodes
            .iter()
            .find(|n| matches!(n.role, AccessibilityRole::TableColumn { index: 0 }))
            .unwrap();
        let header_node = nodes
            .iter()
            .find(|n| {
                matches!(
                    n.role,
                    AccessibilityRole::TableColumnHeader { index: 0, .. }
                )
            })
            .unwrap();
        let row_node = nodes
            .iter()
            .find(|n| matches!(n.role, AccessibilityRole::TableRow { index: 0 }))
            .unwrap();
        let cell_node = nodes
            .iter()
            .find(|n| matches!(n.role, AccessibilityRole::TableCell { row: 0, column: 0 }))
            .unwrap();

        assert_eq!(col_node.parent, Some(table_node.key));
        assert_eq!(header_node.parent, Some(col_node.key));
        assert_eq!(row_node.parent, Some(table_node.key));
        assert_eq!(cell_node.parent, Some(row_node.key));
        assert_eq!(table_node.enabled, !disabled);
        assert_eq!(col_node.enabled, !disabled);
        assert_eq!(header_node.enabled, !disabled);
        assert_eq!(row_node.enabled, !disabled);
        assert_eq!(cell_node.enabled, !disabled);

        if disabled {
            assert!(!content_action(&mut h, row_node.key, AccessibilityAction::Press).consumed);
            assert!(!content_action(&mut h, header_node.key, AccessibilityAction::Press).consumed);
        } else {
            assert!(content_action(&mut h, row_node.key, AccessibilityAction::Press).consumed);
        }
        assert!(!content_action(&mut h, cell_node.key, AccessibilityAction::Press).consumed);
    }
}

#[test]
fn publication_pass_reuses_semantic_parts_for_valid_text_layouts_and_sliders() {
    use std::cell::Cell;
    use std::rc::Rc;
    use tekes_ui_controls::layout::MeasuredChild;

    #[derive(Debug)]
    struct CountingPartsOwner {
        kind_slider: bool,
        count: usize,
        calls: Rc<Cell<usize>>,
        constructed_parts: Rc<Cell<usize>>,
        frame: std::cell::Cell<Rect>,
    }
    impl Component for CountingPartsOwner {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(available)
        }
        fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, _: Rect) {
            self.frame.set(frame);
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::new(ContentRole::Group, "CountingPartsOwner")
        }
        fn semantic_parts(&self) -> Vec<SemanticPart> {
            self.calls.set(self.calls.get() + 1);
            self.constructed_parts
                .set(self.constructed_parts.get() + self.count);
            (0..self.count)
                .map(|i| SemanticPart {
                    index: i + 1,
                    frame: Rect::new(4., 4. + i as f32 * 2., 120., 18.),
                    focused: i == 0,
                    semantics: if self.kind_slider {
                        let mut s =
                            ContentSemantics::new(ContentRole::Slider, format!("Slider {i}"));
                        s.value = Some("25".into());
                        s
                    } else {
                        let mut s =
                            ContentSemantics::new(ContentRole::TextInput, format!("Editor {i}"));
                        s.value = Some(format!("value-{i}"));
                        s
                    },
                })
                .collect()
        }
        fn semantic_part_parent(&self, _: usize) -> Option<usize> {
            None
        }
        fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
            (self.kind_slider && (1..=self.count).contains(&part)).then_some(0.0..=100.0)
        }
        fn semantic_part_text_layout(&self, part: usize) -> Option<ContentTextLayout> {
            if self.kind_slider || !(1..=self.count).contains(&part) {
                return None;
            }
            let text = format!("value-{}", part - 1);
            let len = text.len();
            let line_rect = Rect::new(8., 6. + (part - 1) as f32 * 2., 60., 14.);
            Some(ContentTextLayout {
                input: ContentTextInput {
                    binding: part.to_le_bytes().to_vec(),
                    text,
                    caret_index: 0,
                    selection: 0..0,
                    marked: None,
                    read_only: false,
                    caret: Some(Rect::new(line_rect.min_x(), line_rect.min_y(), 1., 14.)),
                    clip: Some(self.frame.get()),
                    carets: vec![(0, Rect::new(line_rect.min_x(), line_rect.min_y(), 1., 14.))],
                },
                multiline: false,
                lines: std::iter::once(0..len).collect(),
                carets: vec![(0, Rect::new(line_rect.min_x(), line_rect.min_y(), 1., 14.))],
            })
        }
    }

    for kind_slider in [false, true] {
        for count in [16usize, 32, 64] {
            let calls = Rc::new(Cell::new(0));
            let constructed_parts = Rc::new(Cell::new(0));
            let owner = CountingPartsOwner {
                kind_slider,
                count,
                calls: calls.clone(),
                constructed_parts: constructed_parts.clone(),
                frame: std::cell::Cell::new(Rect::default()),
            };
            let mut host = ContentHost::new(Content::new(ComponentId(900), owner)).unwrap();
            let frame = Rect::new(0., 0., 320., 400.);
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();

            calls.set(0);
            constructed_parts.set(0);

            let snapshot =
                ContentAccessibilitySnapshot::from_host(&host).expect("valid AX snapshot");
            let pub_result = WindowAccessibilityPublication::prepare(vec![(
                ContentAccessibilityOwner(1),
                snapshot,
            )])
            .expect("valid WindowAccessibilityPublication");

            assert_eq!(
                calls.get(),
                1,
                "kind_slider={kind_slider} count={count}: expected 1 semantic_parts() call"
            );
            assert_eq!(
                constructed_parts.get(),
                count,
                "kind_slider={kind_slider} count={count}: expected {count} constructed parts"
            );
            let ax_nodes = pub_result.nodes();
            if kind_slider {
                let sliders = ax_nodes
                    .iter()
                    .filter(|n| matches!(n.role, AccessibilityRole::Slider(_)))
                    .count();
                assert_eq!(sliders, count);
            } else {
                assert_eq!(pub_result.text_layouts().unwrap().len(), count);
            }
        }
    }
}
