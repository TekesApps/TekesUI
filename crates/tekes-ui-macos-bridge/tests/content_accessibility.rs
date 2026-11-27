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

#[test]
fn repeated_no_edit_text_key_preserves_ax_publication_without_redraw_and_invalidates_on_real_changes()
 {
    use std::time::Duration;
    use tekes_ui_controls::{ControlMotionContext, ScrollArea};
    use tekes_ui_platform_macos::Point as NativePoint;
    use tekes_ui_scene::Point;

    #[derive(Default)]
    struct ClockText {
        now: u64,
    }
    impl ButtonTextRenderer for ClockText {
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
            Text.measure(s, role, width)
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
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: Duration::from_millis(self.now),
                reduced_motion: false,
            })
        }
    }
    impl TextSurfaceTextRenderer for ClockText {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &tekes_ui_text::Paragraph,
            _: Point,
            _: Rgba,
            _: Rect,
        ) {
        }
    }
    impl TextSurfaceHost for ClockText {
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
            Text.shape_surface(text, role, width)
        }
    }

    const ROOT: ComponentId = ComponentId(16000);
    const SCROLL: ComponentId = ComponentId(16001);
    const LIST: ComponentId = ComponentId(16002);
    const FIRST: ComponentId = ComponentId(16003);
    const COMPOSER: ComponentId = ComponentId(16020);
    const MUTABLE: ComponentId = ComponentId(16021);

    let build_host = |read_only: bool| -> ContentHost {
        let rows = (0..4)
            .map(|i| {
                Content::new(
                    ComponentId(FIRST.0 + i),
                    Button::new(format!("Session {}", i + 1)),
                )
                .with_help(format!("Help item {}", i + 1))
            })
            .collect();
        let scroll = ScrollContent::new(
            ScrollArea::default(),
            Content::new(LIST, ListContent::new("Search results", rows)),
        )
        .with_viewport_height(Some(120.));
        let field = TextFieldContent::new("Composer", "Keep A😀B").with_read_only(read_only);
        let mut h = ContentHost::new(Content::new(
            ROOT,
            StackContent::vertical(
                tekes_ui_tokens::Spacing::Regular.value(),
                vec![Content::new(SCROLL, scroll), Content::new(COMPOSER, field)],
            ),
        ))
        .unwrap();
        h.set_action_events(true);
        h
    };
    let draw = |h: &mut ContentHost, t: &mut ClockText, w: f32, ht: f32| {
        let f = Rect::new(0., 0., w, ht);
        h.render(&mut Scene::new(), t, f, f, Theme::LIGHT.into())
            .unwrap();
    };
    let publish = |h: &ContentHost| {
        WindowAccessibilityPublication::prepare(vec![(
            ContentAccessibilityOwner(1),
            ContentAccessibilitySnapshot::from_host(h).expect("AX snapshot"),
        )])
        .expect("AX publication")
    };
    let send_chain = |h: &mut ContentHost, t: &mut ClockText, id: u64, consumed: bool| -> bool {
        let key = ContentKeyEvent {
            key: None,
            down: true,
            repeat: false,
            modifiers: InputModifiers {
                command: !consumed,
                ..Default::default()
            },
            native_code: 105,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        };
        let events = [
            ContentEvent::TextKeyStart {
                id,
                key: key.clone(),
                composing: false,
            },
            ContentEvent::TextKeyEnd {
                id,
                input_context_consumed: consumed,
            },
            ContentEvent::KeyInput(ContentKeyEvent { down: false, ..key }),
        ];
        let mut pending = false;
        for (idx, event) in events.iter().enumerate() {
            let response = h.event_with_text(event, t).unwrap();
            let req = h.has_visual_changes()
                || response.changed
                || if idx == 2 {
                    response.consumed
                } else {
                    response.activated
                };
            pending |= req;
        }
        pending
    };

    for input_context_consumed in [false, true] {
        let mut h = build_host(true);
        let mut t = ClockText::default();
        h.focus(Some(COMPOSER), false);
        draw(&mut h, &mut t, 400., 260.);
        let btn_pt = h.get(FIRST).unwrap().visible().unwrap().center();
        h.event(&ContentEvent::PointerMove(btn_pt));
        draw(&mut h, &mut t, 400., 260.);
        t.now += 500;
        draw(&mut h, &mut t, 400., 260.);
        assert_eq!(h.help_tooltip().map(|tip| tip.0), Some(FIRST));
        let mut publication = publish(&h);

        // Chain 1 hides tooltip -> 1 redraw -> republish -> AXPress fires once.
        assert!(send_chain(&mut h, &mut t, 1, input_context_consumed));
        assert!(!publication.is_current());
        draw(&mut h, &mut t, 400., 260.);
        publication = publish(&h);
        let node_count = publication.nodes().len();
        assert!(node_count >= 1);
        let button_key = publication
            .hit_test(NativePoint {
                x: f64::from(btn_pt.x),
                y: f64::from(btn_pt.y),
            })
            .expect("published button key");
        assert!(h.drain_action_records().is_empty());
        let first_press = publication.dispatch(
            ContentAccessibilityOwner(1),
            &mut h,
            button_key,
            AccessibilityAction::Press,
        );
        assert!(first_press.activated);
        assert_eq!(h.drain_action_records().len(), 1);

        // Restore read-only focus and publish with tooltip already hidden.
        h.focus(Some(COMPOSER), false);
        draw(&mut h, &mut t, 400., 260.);
        assert!(h.help_tooltip().is_none());
        publication = publish(&h);

        // Chain 2: repeat same input -> 0 redraws, AX remains current, button Press succeeds once without redraw/republish/focus switch.
        assert!(!send_chain(&mut h, &mut t, 2, input_context_consumed));
        assert!(!h.has_visual_changes());
        assert!(publication.is_current());
        assert_eq!(publication.nodes().len(), node_count);
        let repeat_press = publication.dispatch(
            ContentAccessibilityOwner(1),
            &mut h,
            button_key,
            AccessibilityAction::Press,
        );
        assert!(repeat_press.consumed && repeat_press.activated);
        let actions = h.drain_action_records();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action.target, FIRST);
    }

    // Real edit, layout change, unmount, and permission change must still invalidate stale AX publications.
    let mut h = build_host(false);
    let mut t = ClockText::default();
    h.focus(Some(COMPOSER), false);
    draw(&mut h, &mut t, 400., 260.);
    let p_edit = publish(&h);
    assert!(
        h.event(&ContentEvent::Commit {
            text: "x".into(),
            replacement: None,
        })
        .changed
    );
    assert!(!p_edit.is_current());
    draw(&mut h, &mut t, 400., 260.);

    let p_layout = publish(&h);
    draw(&mut h, &mut t, 420., 280.);
    assert!(!p_layout.is_current());

    let p_perm = publish(&h);
    let btn_pt = h.get(FIRST).unwrap().visible().unwrap().center();
    let btn_key = p_perm
        .hit_test(NativePoint {
            x: f64::from(btn_pt.x),
            y: f64::from(btn_pt.y),
        })
        .unwrap();
    h.get_mut(FIRST)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .set_disabled(true);
    assert!(!p_perm.is_current());
    assert!(
        !p_perm
            .dispatch(
                ContentAccessibilityOwner(1),
                &mut h,
                btn_key,
                AccessibilityAction::Press
            )
            .consumed
    );

    draw(&mut h, &mut t, 420., 280.);
    let p_unmount = publish(&h);
    let rows_list = h
        .get_mut(LIST)
        .unwrap()
        .component_mut::<ListContent>()
        .unwrap();
    *rows_list.items_mut() = vec![Content::new(MUTABLE, Button::new("Replacement"))];
    assert!(!p_unmount.is_current());
}
