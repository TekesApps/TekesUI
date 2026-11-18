use super::tests_1::{FixedText, RecordingText, ScaledText, rect};
use super::*;
use crate::extended::{
    CodeBlock, DataTable, DiffKind, DiffView, List, Markdown, MarkdownBlock, Tree,
};
use tekes_ui_scene::{Primitive, Size};
use tekes_ui_tokens::{Rgba, Theme};

#[test]
fn list_down_and_up_without_render_keeps_capture_and_updates_model() {
    let mut view = ListView::new();
    let mut model = List::new(2);
    let labels = vec!["a".into(), "b".into()];
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut text,
        &model,
        &labels,
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    let point = Point::new(20., view.layout.items[1].frame.min_y() + 2.);
    assert_eq!(
        view.pointer_down(point, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Handled);
    assert_eq!(model.selected, Some(1));
}
#[test]
fn tree_expansion_rebuild_changes_visible_row_ids_not_node_identity() {
    let mut tree = Tree::default();
    let root = tree.add("root", None).unwrap();
    let child = tree.add("child", Some(root)).unwrap();
    let mut view = TreeView::new();
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut text,
        &tree,
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(
        view.layout.items.iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![root]
    );
    tree.toggle(root);
    view.render(
        &mut scene,
        &mut text,
        &tree,
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(
        view.layout.items.iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![root, child]
    );
    assert_eq!(view.layout.items[1].depth, 1);
}
#[test]
fn data_table_sorted_layout_routes_to_stable_source_row() {
    let mut table = DataTable::new(2, 1);
    let rows = vec![vec!["z".into()], vec!["a".into()]];
    let mut view = DataTableView::new();
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut text,
        &table,
        &["Name".into()],
        &rows,
        &[1, 0],
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    let point = Point::new(20., view.layout.rows[0].frame.min_y() + 2.);
    assert_eq!(
        view.pointer_down(point, &mut table),
        CollectionEvent::Changed
    );
    assert_eq!(table.selected_row, Some(1));
}
#[test]
fn window_deactivation_cancels_captured_list_row() {
    let mut view = ListView::new();
    let mut model = List::new(1);
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["a".into()],
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    let point = Point::new(20., 25.);
    view.pointer_down(point, &mut model);
    view.set_window_active(false);
    assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Ignored);
}

#[test]
fn table_skips_clipped_rows_and_disabled_table_cannot_sort() {
    let mut model = DataTable::new(2, 1);
    let rows = vec![vec!["first".into()], vec!["second".into()]];
    let mut view = DataTableView::new();
    let mut text = FixedText { height: 56. };
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        &["Name".into()],
        &rows,
        &[0, 1],
        rect(),
        Some(Rect::new(10., 20., 90., 32.)),
        Theme::LIGHT.into(),
    );
    assert!(layout.rows.iter().all(|row| row.visible.is_none()));
    view.set_disabled(true);
    assert_eq!(
        view.pointer_down(Point::new(20., 25.), &mut model),
        CollectionEvent::Ignored
    );
    assert_eq!(model.sort, None);
}

#[test]
fn chart_routes_only_visible_real_bar_geometry() {
    let mut model = BarChart::new(vec![
        crate::ChartDatum::new("visible", 3.),
        crate::ChartDatum::new("clipped", 7.),
    ]);
    let mut view = ChartView::new();
    let mut text = FixedText { height: 12. };
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &mut model,
        Rect::new(0., 0., 300., 140.),
        Some(Rect::new(0., 0., 60., 140.)),
        Theme::LIGHT.into(),
        1.,
    );
    assert!(layout.visible.is_some());
    assert!(
        layout
            .geometry
            .bars
            .iter()
            .all(|bar| bar.rect.min_x() >= 0.)
    );
    assert!(!view.pointer_move(Point::new(280., 80.), &mut model));
}

#[test]
fn markdown_and_code_layout_expand_to_host_measure_and_clip() {
    let markdown = Markdown::parse("# Heading\n\nA long paragraph");
    let mut view = MarkdownView::new();
    let mut text = FixedText { height: 64. };
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &markdown,
        rect(),
        Some(Rect::new(10., 20., 240., 40.)),
        Theme::LIGHT.into(),
        1.,
    );
    assert!(layout.blocks[0].frame.size.height >= 64.);
    assert!(layout.blocks[0].visible.is_some());
    assert!(layout.blocks[1].visible.is_none());

    let mut code_view = CodeBlockView::new();
    let code_layout = code_view.render(
        &mut scene,
        &mut text,
        &CodeBlock {
            language: "rs".into(),
            source: "line one\nline two".into(),
        },
        Rect::new(0., 0., 80., 30.),
        None,
        Theme::LIGHT.into(),
        1.5,
    );
    assert!(code_layout.content_height > 30.);
    assert_eq!(
        code_layout
            .blocks
            .iter()
            .filter(|line| line.visible.is_some())
            .count(),
        1
    );
}

#[test]
fn diff_presentation_keeps_stable_line_ids_when_clipped() {
    let diff = DiffView::between("same\nold", "same\nnew");
    let mut view = DiffViewRuntime::new();
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &diff,
        Rect::new(0., 0., 200., 100.),
        Some(Rect::new(0., 0., 200., 31.)),
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(
        layout.blocks.iter().map(|line| line.id).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        layout
            .blocks
            .iter()
            .filter(|line| line.visible.is_some())
            .count(),
        2
    );
}

#[test]
fn data_table_header_keyboard_focus_routes_to_sorting() {
    let mut model = DataTable::new(0, 2);
    let mut view = DataTableView::new();
    view.render(
        &mut Scene::new(),
        &mut FixedText { height: 14. },
        &model,
        &["Name".into(), "Kind".into()],
        &[],
        &[],
        Rect::new(0., 0., 300., 120.),
        None,
        Theme::LIGHT.into(),
    );
    view.focus_header(0, true);
    assert_eq!(
        view.key(ControlKey::Right, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(
        model.sort,
        Some((1, crate::extended::SortDirection::Ascending))
    );
}

#[test]
fn sorted_data_table_keys_follow_visual_order_without_intermediate_render() {
    let mut model = DataTable::new(5, 1);
    let mut view = DataTableView::new();
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    let rows = (0..5)
        .map(|index| vec![index.to_string()])
        .collect::<Vec<_>>();
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["Name".into()],
        &rows,
        &[3, 2, 1, 0, 4],
        Rect::new(0., 0., 300., 260.),
        None,
        Theme::LIGHT.into(),
    );
    model.select_row(3);
    view.focus_row(3, true);

    for (key, expected) in [
        (ControlKey::Down, 2),
        (ControlKey::Down, 1),
        (ControlKey::Up, 2),
        (ControlKey::Home, 3),
        (ControlKey::End, 4),
    ] {
        assert_eq!(
            view.key(key, true, &mut model),
            CollectionEvent::Changed,
            "{key:?} should move to source row {expected} in visual order"
        );
        assert_eq!(model.selected_row, Some(expected));
        assert_eq!(view.state.focused, Some(expected));
    }
}

#[test]
fn sorted_data_table_keys_traverse_logical_rows_beyond_the_visible_clip() {
    let mut model = DataTable::new(5, 1);
    let mut view = DataTableView::new();
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    let rows = (0..5)
        .map(|index| vec![index.to_string()])
        .collect::<Vec<_>>();
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["Name".into()],
        &rows,
        &[3, 2, 1, 0, 4],
        Rect::new(0., 0., 300., 260.),
        Some(Rect::new(0., 40., 300., 34.)),
        Theme::LIGHT.into(),
    );
    model.select_row(3);
    view.focus_row(3, true);
    assert_eq!(
        view.key(ControlKey::Down, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(model.selected_row, Some(2));
    assert_eq!(view.state.focused, Some(2));
}

#[test]
fn list_hover_exit_immediately_restores_transparent_rows_during_motion() {
    use crate::ControlMotionContext;
    use crate::extended::{
        KeyedList, KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey,
    };
    use std::time::Duration;

    struct MovingText(Duration);
    impl ButtonTextRenderer for MovingText {
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: self.0,
                reduced_motion: false,
            })
        }
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(40., 14.)
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
    for theme in [Theme::LIGHT, Theme::DARK] {
        for selected in [None, Some(3)] {
            let mut model = KeyedList::new(
                KeyedListSnapshot {
                    dataset: ListDatasetKey(1),
                    items: (0..4)
                        .map(|i| KeyedListItem {
                            key: ListItemKey(i),
                            name: format!("row {i}"),
                        })
                        .collect(),
                    display_items: (0..4).map(ListItemKey).collect(),
                },
                ListBudget {
                    max_items: 4,
                    max_display_items: 4,
                    max_utf8_bytes: 1000,
                    max_staging_units: 1000,
                },
            )
            .unwrap();
            if let Some(i) = selected {
                model.select_key(ListItemKey(i));
            }
            let mut view = ListView::new();
            let mut text = MovingText(Duration::ZERO);
            let frame = Rect::new(0., 0., 240., 400.);
            let layout = view.render_keyed(
                &mut Scene::new(),
                &mut text,
                &model,
                frame,
                Some(frame),
                theme.into(),
            );
            // Cross A -> B -> C while earlier hover animations are active,
            // then hover the selected row, leave the rows, and leave the host.
            for (step, current) in [Some(0), Some(1), Some(2), Some(3), None, None]
                .into_iter()
                .enumerate()
            {
                let point =
                    current
                        .map(|i| layout.items[i].frame.center())
                        .unwrap_or(if step == 4 {
                            Point::new(10., 390.)
                        } else {
                            Point::new(f32::MIN, f32::MIN)
                        });
                view.pointer_move_keyed(point, &model);
                for offset in [0, 10, 30] {
                    text.0 = Duration::from_millis(1 + step as u64 * 40 + offset);
                    let mut scene = Scene::new();
                    view.render_keyed(
                        &mut scene,
                        &mut text,
                        &model,
                        frame,
                        Some(frame),
                        theme.into(),
                    );
                    let painted = layout.items.iter().enumerate().filter_map(|(i, row)| {
                        scene.primitives().iter().any(|p| matches!(p,
                            Primitive::Quad(q) if q.fill.a > 0. && q.rect.contains(row.frame.center())
                        )).then_some(i)
                    }).collect::<Vec<_>>();
                    let expected = (0..4)
                        .filter(|i| Some(*i) == current || Some(*i as u64) == selected)
                        .collect::<Vec<_>>();
                    assert_eq!(painted, expected, "step {step}, offset {offset}, {theme:?}");
                }
            }
        }
    }
}

#[test]
fn list_hidden_or_disabled_state_does_not_keep_old_capture_or_hover() {
    let mut view = ListView::new();
    let mut model = List::new(1);
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["row".into()],
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    let point = Point::new(20., 25.);
    view.pointer_down(point, &mut model);
    view.pointer_move(point);
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["row".into()],
        Rect::new(0., 0., 0., 0.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Ignored);
    assert!(!view.pointer_move(point));

    view.set_disabled(true);
    assert!(!view.pointer_move(point));
}

#[test]
fn tree_and_table_hidden_or_disabled_views_cancel_old_press() {
    let mut tree = Tree::default();
    tree.add("root", None).unwrap();
    let mut tree_view = TreeView::new();
    let mut text = FixedText { height: 14. };
    let mut scene = Scene::new();
    tree_view.render(
        &mut scene,
        &mut text,
        &tree,
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    let tree_point = Point::new(20., 25.);
    assert_eq!(
        tree_view.pointer_down(tree_point, &mut tree),
        CollectionEvent::Changed
    );
    tree_view.set_disabled(true);
    assert_eq!(
        tree_view.pointer_up(tree_point, &mut tree),
        CollectionEvent::Ignored
    );

    let mut table = DataTable::new(1, 1);
    let mut table_view = DataTableView::new();
    table_view.render(
        &mut scene,
        &mut text,
        &table,
        &["Name".into()],
        &[vec!["row".into()]],
        &[0],
        rect(),
        None,
        Theme::LIGHT.into(),
    );
    let row_point = Point::new(20., 62.);
    assert_eq!(
        table_view.pointer_down(row_point, &mut table),
        CollectionEvent::Changed
    );
    table_view.render(
        &mut scene,
        &mut text,
        &table,
        &["Name".into()],
        &[vec!["row".into()]],
        &[0],
        Rect::new(0., 0., 0., 0.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(
        table_view.pointer_up(row_point, &mut table),
        CollectionEvent::Ignored
    );
}

#[test]
fn markdown_measures_bullet_prefix_and_diff_measures_wrapped_prefixed_lines() {
    let mut markdown = Markdown::default();
    markdown.blocks.push(MarkdownBlock::Bullet("item".into()));
    let mut markdown_view = MarkdownView::new();
    let mut text = RecordingText::default();
    let mut scene = Scene::new();
    markdown_view.render(
        &mut scene,
        &mut text,
        &markdown,
        Rect::new(0., 0., 80., 120.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert!(text.measured.iter().any(|(value, _)| value == "• item"));
    assert_eq!(text.drawn, ["• item"]);
    assert_eq!(markdown_view.layout.blocks[0].frame.min_y(), 0.);
    assert_eq!(markdown_view.layout.blocks[0].frame.min_x(), 0.);
    assert_eq!(markdown_view.layout.blocks[0].frame.size.height, 24.);
    assert_eq!(
        text.draw_rects[0].min_y(),
        markdown_view.layout.blocks[0].frame.min_y()
    );
    assert_eq!(text.draw_rects[0].size.height, 14.);

    markdown
        .blocks
        .push(MarkdownBlock::Paragraph("next".into()));
    markdown_view.render(
        &mut scene,
        &mut text,
        &markdown,
        Rect::new(0., 0., 80., 120.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(markdown_view.layout.blocks[1].frame.min_y(), 36.);

    let diff = DiffView {
        lines: vec![crate::extended::DiffLine {
            kind: DiffKind::Added,
            text: "a sufficiently long line".into(),
        }],
    };
    let mut diff_view = DiffViewRuntime::new();
    let mut text = RecordingText::default();
    let diff_layout = diff_view.render(
        &mut scene,
        &mut text,
        &diff,
        Rect::new(0., 0., 80., 120.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert!(
        text.measured
            .iter()
            .any(|(value, width)| { value == "+ a sufficiently long line" && *width == 64. })
    );
    assert_eq!(text.drawn, ["+ a sufficiently long line"]);
    assert!(diff_layout.blocks[0].frame.size.height >= 72.);
    assert_eq!(diff_layout.blocks[0].frame.min_x(), 8.);
    assert_eq!(diff_layout.blocks[0].frame.size.width, 64.);
}

#[test]
fn code_diff_and_chart_keep_gallery_text_insets_and_alignment() {
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    let mut code_view = CodeBlockView::new().with_max_width(Some(650.));
    let code = CodeBlock {
        language: "rs".into(),
        source: "let value = 1;".into(),
    };
    code_view.render(
        &mut scene,
        &mut text,
        &code,
        Rect::new(0., 0., 680., 100.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(text.draw_rects[0].min_x(), 14.);
    assert_eq!(text.draw_rects[0].min_y(), 14.);
    assert_eq!(text.draw_rects[0].size.width, 650.);

    let mut diff_view = DiffViewRuntime::new().with_max_width(Some(700.));
    let diff = DiffView::between("same", "same");
    diff_view.render(
        &mut scene,
        &mut text,
        &diff,
        Rect::new(0., 0., 800., 100.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(text.draw_rects[1].min_x(), 8.);
    assert_eq!(text.draw_rects[1].size.width, 700.);
    assert_eq!(text.draw_rects[1].size.height, 28.);
    assert_eq!(diff_view.layout.blocks[0].frame.size.height, 28.);

    for (scale, label, value) in [(1., "A", 4.), (2., "Long release label", 123456.)] {
        let mut chart_view = ChartView::new();
        let mut chart = BarChart::new(vec![crate::chart::ChartDatum::new(label, value)]);
        let mut chart_text = RecordingText::default();
        let chart_layout = chart_view.render(
            &mut scene,
            &mut chart_text,
            &mut chart,
            Rect::new(0., 0., 320., 220.),
            None,
            Theme::LIGHT.into(),
            scale,
        );
        let bar = &chart_layout.geometry.bars[0];
        assert_eq!(chart_text.alignments, [ButtonTextAlign::Center; 2]);
        assert_eq!(chart_text.draw_rects[0], bar.value_label);
        assert_eq!(chart_text.draw_rects[1], bar.category_label);
        for slot in [bar.value_label, bar.category_label] {
            assert!(slot.min_y() >= chart_layout.geometry.bounds.min_y());
            assert!(slot.max_y() <= chart_layout.geometry.bounds.max_y());
        }
        assert!(chart_layout.geometry.plot.min_y() > chart_layout.geometry.bounds.min_y());
        for ((origin, ink_width), slot) in chart_text
            .ink_origins
            .iter()
            .zip(&chart_text.ink_widths)
            .zip([bar.value_label, bar.category_label])
        {
            assert!((origin.x + ink_width * 0.5 - slot.center().x).abs() < 0.001);
        }
    }
}

#[test]
fn code_and_diff_wrapped_lines_reserve_both_horizontal_insets() {
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    let code_frame = Rect::new(20., 10., 120., 240.);
    let mut code_view = CodeBlockView::new();
    let code_layout = code_view.render(
        &mut scene,
        &mut text,
        &CodeBlock {
            language: "rs".into(),
            source: "a deliberately long line that wraps in a narrow code block".into(),
        },
        code_frame,
        None,
        Theme::LIGHT.into(),
        1.5,
    );
    assert!(text.measured.iter().any(|(_, width)| *width == 92.));
    assert_eq!(text.draw_rects[0].min_x(), code_frame.min_x() + 14.);
    assert_eq!(text.draw_rects[0].max_x(), code_frame.max_x() - 14.);
    assert!(code_layout.blocks[0].frame.max_x() <= code_frame.max_x() - 14.);

    text.measured.clear();
    text.draw_rects.clear();
    let diff_frame = Rect::new(30., 5., 100., 240.);
    let mut diff_view = DiffViewRuntime::new();
    let diff_layout = diff_view.render(
        &mut scene,
        &mut text,
        &DiffView::between(
            "a deliberately long existing line which wraps",
            "a deliberately long changed line which wraps",
        ),
        diff_frame,
        None,
        Theme::LIGHT.into(),
        1.5,
    );
    assert!(text.measured.iter().any(|(_, width)| *width == 84.));
    assert!(diff_layout.blocks.iter().all(|line| {
        line.frame.min_x() == diff_frame.min_x() + 8.
            && line.frame.max_x() == diff_frame.max_x() - 8.
    }));
}

#[test]
fn code_block_surface_uses_code_role_and_text_backend_backing_scale() {
    for theme in Theme::ALL {
        for backing_scale in [1.0, 2.0] {
            let palette = Palette::from(theme);
            let frame = Rect::new(14.0, 18.0, 220.0, 80.0);
            let mut text = ScaledText(backing_scale);
            let mut view = CodeBlockView::new();
            let mut scene = Scene::new();
            let layout = view.render(
                &mut scene,
                &mut text,
                &CodeBlock {
                    language: "rs".into(),
                    source: "let value = 1;".into(),
                },
                frame,
                Some(Rect::new(0.0, 0.0, 400.0, 200.0)),
                palette,
                1.5,
            );
            assert_eq!(layout.frame, frame);
            let code_quad = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    Primitive::Quad(q) if q.rect == frame => Some(q),
                    _ => None,
                })
                .expect("code block surface quad");
            assert_eq!(code_quad.fill, palette.color("code-surface").unwrap());
            assert_eq!(
                code_quad.corner_radii,
                tekes_ui_scene::CornerRadii::uniform(
                    proposal::metric("radius", "c-code-radius").unwrap()
                )
            );
            assert_eq!(
                code_quad.border_width,
                crate::paint::decorative_border(palette, backing_scale).0
            );
        }
    }
}

#[test]
fn code_block_background_uses_parent_clip_and_text_keeps_frame_clip() {
    let frame = Rect::new(14., 18., 220., 80.);
    for ancestor in [
        Rect::new(-20., -20., 400., 240.),
        Rect::new(60., 30., 140., 50.),
    ] {
        let mut text = ScaledText(2.0);
        let mut view = CodeBlockView::new();
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &CodeBlock {
                language: "rs".into(),
                source: "let value = 1;".into(),
            },
            frame,
            Some(ancestor),
            Theme::LIGHT.into(),
            1.0,
        );
        let visible = frame.intersection(&ancestor).expect("visible code block");
        assert_eq!(layout.visible, Some(visible));
        let primitives = scene.primitives();
        assert!(matches!(primitives.first(), Some(Primitive::PushClip(r)) if *r == ancestor));
        assert!(matches!(primitives.get(1), Some(Primitive::Quad(q)) if q.rect == frame));
        assert!(matches!(primitives.get(2), Some(Primitive::PopClip)));
        assert!(matches!(primitives.get(3), Some(Primitive::PushClip(r)) if *r == visible));
    }
}

#[test]
fn malformed_and_empty_code_block_bounds_emit_no_surface_or_text() {
    let model = CodeBlock {
        language: "rs".into(),
        source: "line".into(),
    };
    let mut view = CodeBlockView::new();
    let mut text = RecordingText::default();
    let mut scene = Scene::new();
    for frame in [
        Rect::new(0., 0., 0., 40.),
        Rect::new(f32::NAN, 0., 40., 40.),
        Rect::new(f32::MAX, 0., f32::MAX, 40.),
    ] {
        scene.clear();
        text.drawn.clear();
        view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            Theme::LIGHT.into(),
            1.,
        );
        assert!(scene.is_empty());
        assert!(text.drawn.is_empty());
    }
}

#[test]
fn document_views_follow_parent_width_and_default_zero_top_inset() {
    let mut scene = Scene::new();
    let mut text = RecordingText::default();

    let md = Markdown {
        blocks: vec![MarkdownBlock::Paragraph("Body".into())],
    };
    let mut md_default = MarkdownView::new();
    let default_layout = md_default.render(
        &mut scene,
        &mut text,
        &md,
        Rect::new(10., 20., 500., 200.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(default_layout.blocks[0].frame.min_y(), 20.0);

    let mut md_inset = MarkdownView::new().with_top_inset(4.0);
    let inset_layout = md_inset.render(
        &mut scene,
        &mut text,
        &md,
        Rect::new(10., 20., 500., 200.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(inset_layout.blocks[0].frame.min_y(), 24.0);

    let code = CodeBlock {
        language: "rs".into(),
        source: "let wide = true;".into(),
    };
    let mut code_view = CodeBlockView::new();
    let code_layout = code_view.render(
        &mut scene,
        &mut text,
        &code,
        Rect::new(0., 0., 900., 100.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(code_layout.blocks[0].frame.size.width, 900. - 28.);

    let diff = DiffView::between("same", "same");
    let mut diff_view = DiffViewRuntime::new();
    let diff_layout = diff_view.render(
        &mut scene,
        &mut text,
        &diff,
        Rect::new(0., 0., 960., 100.),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    assert_eq!(diff_layout.blocks[0].frame.size.width, 960. - 16.);
}
