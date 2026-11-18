use super::*;
use crate::ControlState;
use tekes_ui_scene::Point;
#[test]
fn stack_grid_and_split_respect_bounds() {
    let s = Stack {
        axis: Axis::Horizontal,
        gap: 10.,
    };
    assert_eq!(
        s.layout(Rect::new(0., 0., 110., 20.), 2),
        vec![Rect::new(0., 0., 50., 20.), Rect::new(60., 0., 50., 20.)]
    );
    let g = Grid::new(2);
    assert_eq!(g.layout(Rect::new(0., 0., 100., 100.), 3, 20.).len(), 3);
    let narrow = s.layout(Rect::new(0., 0., 5., 20.), 3);
    assert!(narrow.iter().all(|r| r.min_x() >= 0. && r.max_x() <= 5.));
    let mut huge = Stack {
        axis: Axis::Vertical,
        gap: 1000.,
    };
    let items = huge.layout(Rect::new(0., 0., 20., 5.), 3);
    assert!(items.iter().all(|r| r.min_y() >= 0. && r.max_y() <= 5.));
    huge.gap = f32::NAN;
    assert!(
        huge.layout(Rect::new(0., 0., 20., 5.), 2)
            .iter()
            .all(|r| r.size.height.is_finite())
    );
    let mut split = SplitView::new(9.);
    assert_eq!(split.ratio(), 1.);
    split.collapse(Some(Pane::Leading));
    assert_eq!(
        split
            .layout(Rect::new(0., 0., 101., 40.), Axis::Horizontal)
            .0
            .size
            .width,
        0.
    );
    split.divider = 10.;
    let tiny = split.layout(Rect::new(0., 0., 3., 4.), Axis::Horizontal);
    assert!(tiny.1.max_x() <= 3. && tiny.2.min_x() <= 3.);
}

#[test]
fn existing_control_layouts_keep_draw_frames_separate_from_visible_hits() {
    let clip = Rect::new(80., 20., 40., 60.);
    let slider = Slider::new(50., 0., 100., 5.).unwrap();
    let slider_frame = slider.layout(Rect::new(60., 30., 120., 36.), Some(clip));
    assert_eq!(slider_frame.frame(), Rect::new(60., 30., 120., 36.));
    assert_eq!(
        slider_frame.visible_frame(),
        Some(Rect::new(80., 30., 40., 36.))
    );
    assert!(slider_frame.contains(Point::new(90., 40.)));
    assert!(!slider_frame.contains(Point::new(70., 40.)));

    let radios = RadioGroup::new(3);
    let rows = radios.row_layout(Rect::new(60., 10., 180., f32::NAN), 36., 6., Some(clip));
    assert_eq!(rows[0].frame(), Rect::new(60., 10., 180., 36.));
    assert_eq!(rows[0].visible_frame(), Some(Rect::new(80., 20., 40., 26.)));
    assert_eq!(rows[1].frame(), Rect::new(60., 52., 180., 36.));
    assert_eq!(rows[1].visible_frame(), Some(Rect::new(80., 52., 40., 28.)));
    assert_eq!(rows[2].frame(), Rect::new(60., 94., 180., 36.));
    assert_eq!(rows[2].visible_frame(), None);
    assert!(
        radios
            .row_layout(Rect::new(f32::NAN, 10., 180., 0.), 36., 6., Some(clip))
            .is_empty()
    );
    assert_eq!(
        radios
            .row_layout(Rect::new(60., 10., 180., 0.), 36., 6., Some(clip))
            .len(),
        radios.len(),
        "a valid zero-height flow origin still lays out radio rows"
    );

    let split = SplitView::new(0.5);
    let split = split.layout_clipped(Rect::new(10., 10., 300., 80.), Axis::Horizontal, Some(clip));
    assert_eq!(split.leading.frame().size.width, 149.5);
    assert_eq!(
        split.leading.visible_frame(),
        Some(Rect::new(80., 20., 40., 60.))
    );
    assert_eq!(split.frame.visible_frame(), Some(clip));

    let switch = Switch::default().layout(Rect::new(60., 30., 48., 26.), Some(clip));
    assert_eq!(switch.frame(), Rect::new(60., 30., 48., 26.));
    assert_eq!(switch.visible_frame(), Some(Rect::new(80., 30., 28., 26.)));
}

#[test]
fn control_frame_focus_bounds_include_ring_gap_and_backing_pixel_guard() {
    let frame = Rect::new(20.0, 30.0, 40.0, 24.0);
    let control = ControlFrame::new(frame, Some(Rect::new(10.0, 10.0, 80.0, 80.0)));
    let focus_1x = control.focus_bounds(1.0);
    let focus_2x = control.focus_bounds(2.0);
    let focus_outset = proposal::metric("spacing", "c-focus-offset").unwrap()
        + proposal::metric("stroke", "c-focus-width").unwrap();
    assert_eq!(focus_1x, frame.inset(-(focus_outset + 1.0)));
    assert_eq!(focus_2x, frame.inset(-(focus_outset + 0.5)));
    assert_eq!(control.frame(), frame);
    assert_eq!(control.visible_frame(), Some(frame));
}

#[test]
fn clipped_control_frames_do_not_retain_hit_regions_after_resize_or_disable() {
    use crate::composition::Component;
    struct Text;
    impl crate::ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, role: proposal::TextRole, _: f32) -> tekes_ui_scene::Size {
            tekes_ui_scene::Size::new(0., role.style().line_height)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    let mut icon =
        crate::composition::Button::new(tekes_ui_scene::ImageId(1)).with_accessible_name("Add");
    let clip = Rect::new(0., 0., 80., 80.);
    let palette = Palette::from(tekes_ui_tokens::Theme::LIGHT);
    let frame = Rect::new(10., 10., 44., 40.);
    icon.arrange(&mut Text, frame, clip);
    icon.paint(&mut Scene::new(), &mut Text, frame, clip, palette.into());
    let original = icon.button.layout().unwrap();
    assert!(original.hit_test(Point::new(20., 20.)));
    let frame = Rect::new(110., 110., 44., 40.);
    icon.arrange(&mut Text, frame, clip);
    icon.paint(&mut Scene::new(), &mut Text, frame, clip, palette.into());
    let resized = icon.button.layout().unwrap();
    assert_eq!(resized.visible, None);
    assert!(!resized.hit_test(Point::new(20., 20.)));

    let mut switch_model = Switch {
        checked: false,
        disabled: true,
    };
    assert!(!switch_model.toggle());
    assert!(!switch_model.set(true));
    assert!(!switch_model.checked);

    let mut radio = RadioGroup::new(2);
    radio.set_enabled(0, false);
    assert!(!radio.select(0));
    assert_eq!(radio.selected, Some(1));
    assert!(!radio.key(ControlKey::Left));
    assert_eq!(radio.selected, Some(1));
}
#[test]
fn slider_compact_geometry_preserves_endpoints_and_state_ink() {
    let frame = Rect::new(12., 20., 160., 40.);
    for theme in tekes_ui_tokens::Theme::ALL {
        for (value, hovered, pressed, role) in [
            (0., false, false, "accent"),
            (50., true, false, "accent-hover"),
            (100., true, true, "accent-pressed"),
        ] {
            let mut scene = Scene::new();
            let geometry = slider(
                &mut scene,
                frame,
                theme,
                Slider::new(value, 0., 100., 1.).unwrap(),
                ControlState {
                    hovered,
                    pressed,
                    ..ControlState::default()
                },
            );
            assert_eq!(geometry.thumb.size, tekes_ui_scene::Size::new(16., 16.));
            assert_eq!(geometry.track.size.height, 2.);
            assert_eq!(geometry.thumb.center().y, frame.center().y);
            assert_eq!(geometry.track.center().y, frame.center().y);
            assert_eq!(
                geometry.thumb_center_x,
                geometry.track.min_x() + geometry.track.size.width * value as f32 / 100.
            );
            let tekes_ui_scene::Primitive::Quad(fill) = &scene.primitives()[1] else {
                panic!("slider fill quad")
            };
            assert_eq!(fill.fill, paint_color(theme.into(), role));
            assert_eq!(fill.rect.max_x(), geometry.thumb_center_x);
        }
    }
}

#[test]
fn slider_rejects_nonfinite_and_maps_drag_and_keys() {
    let mut s = Slider::new(5., 0., 10., 2.).unwrap();
    assert!(s.drag(75., Rect::new(0., 0., 100., 10.)));
    assert_eq!(s.value(), 8.);
    assert!(s.key(ControlKey::Home));
    assert_eq!(s.value(), 0.);
    assert!(!s.set(f64::NAN));
    assert!(Slider::new(0., 0., 1., 0.).is_none());
}
#[test]
fn radio_and_segmented_navigation_skip_disabled() {
    let mut r = RadioGroup::new(3);
    r.set_enabled(1, false);
    assert!(r.key(ControlKey::Right));
    assert_eq!(r.selected, Some(2));
    let mut s = SegmentedControl::new(3);
    s.set_enabled(1, false);
    assert!(s.key(ControlKey::Right));
    assert_eq!(s.selected, 2);
}
#[test]
fn tree_expansion_and_navigation_are_structural() {
    let mut t = Tree::default();
    let root = t.add("root", None).unwrap();
    let child = t.add("child", Some(root)).unwrap();
    assert_eq!(t.visible(), vec![root]);
    assert!(t.select(root));
    assert!(t.toggle(root));
    assert_eq!(t.visible(), vec![root, child]);
    assert!(t.key(ControlKey::Right));
    assert_eq!(t.selected, Some(child));
    assert!(t.key(ControlKey::Left));
    assert_eq!(t.selected, Some(root));
    assert!(t.key(ControlKey::End));
    assert_eq!(t.selected, Some(child));
    assert!(t.set_disabled(child, true));
    assert_eq!(t.selected, None);
    assert!(t.key(ControlKey::End));
    assert_eq!(t.selected, Some(root));
    assert_eq!(t.add("bad", Some(99)), None);
}
#[test]
fn combo_filters_and_commits_only_visible_items() {
    let mut c = ComboBox {
        items: vec!["Apple".into(), "Pear".into()],
        ..Default::default()
    };
    c.set_query("pea");
    assert_eq!(c.filtered(), vec![1]);
    assert_eq!(c.key(ControlKey::Enter), Some(1));
    assert_eq!(c.selected, Some(1));
    assert!(!c.open);
}
#[test]
fn markdown_subset_and_lcs_diff_are_explicit_and_ordered() {
    let m = Markdown::parse("# Title\n\nhello\nworld\n\n- item\n```rs\nx\n```");
    assert_eq!(
        m.blocks,
        vec![
            MarkdownBlock::Heading {
                level: 1,
                text: "Title".into()
            },
            MarkdownBlock::Paragraph("hello world".into()),
            MarkdownBlock::Bullet("item".into()),
            MarkdownBlock::Code {
                language: "rs".into(),
                source: "x".into()
            }
        ]
    );
    let d = DiffView::between("a\nb", "a\nc");
    assert_eq!(
        d.lines.iter().map(|l| l.kind).collect::<Vec<_>>(),
        vec![DiffKind::Equal, DiffKind::Removed, DiffKind::Added]
    );
}
#[test]
fn disabled_components_and_focus_return_have_contracts() {
    let mut d = Disclosure {
        open: false,
        disabled: true,
    };
    assert!(!d.toggle());
    let mut p = Popover::default();
    p.show(42);
    assert_eq!(p.close(), Some(42));
    assert_eq!(p.close(), None);
    let mut b = Banner {
        message: "x".into(),
        tone: NoticeTone::Info,
        dismissible: false,
        visible: true,
    };
    assert!(!b.dismiss());
}
#[test]
fn split_drag_deadzone_cancel_and_keyboard_restore() {
    let mut s = SplitView::new(0.5);
    let bounds = Rect::new(0., 0., 204., 40.);
    assert!(s.begin_drag(100., bounds, Axis::Horizontal));
    assert!(!s.drag_to(103., bounds, Axis::Horizontal));
    assert!(s.drag_to(120., bounds, Axis::Horizontal));
    assert!(s.ratio() > 0.5);
    assert!(s.drag_to(102., bounds, Axis::Horizontal));
    assert!(
        s.ratio() > 0.5,
        "the dead zone must stay latched after drag begins"
    );
    assert!(s.cancel_drag());
    assert_eq!(s.ratio(), 0.5);
    assert!(s.begin_drag(100., bounds, Axis::Horizontal));
    assert!(s.drag_to(120., bounds, Axis::Horizontal));
    s.end_drag();
    assert!(s.key(ControlKey::Left));
    assert!(!s.key(ControlKey::Escape));
    assert!(s.key_in(ControlKey::Home, bounds, Axis::Horizontal));
    let panes = s.layout(bounds, Axis::Horizontal);
    assert!(panes.0.size.width >= s.min_pane);
    assert!(panes.2.size.width >= s.min_pane);
}
#[test]
fn split_reset_restores_balanced_expanded_state_and_ends_gesture() {
    let mut split = SplitView::new(0.7);
    let bounds = Rect::new(0., 0., 200., 100.);
    split.collapse(Some(Pane::Leading));
    assert!(split.begin_drag(140., bounds, Axis::Horizontal));
    assert!(split.drag_to(160., bounds, Axis::Horizontal));
    assert!(split.key_in(ControlKey::Enter, bounds, Axis::Horizontal));
    assert_eq!(split.ratio(), 0.5);
    assert_eq!(split.collapsed, None);
    assert!(!split.drag_to(180., bounds, Axis::Horizontal));
    assert!(
        !split.cancel_drag(),
        "reset cannot later roll back to the old ratio"
    );
    split.set_ratio(0.8);
    assert!(split.key_in(ControlKey::Space, bounds, Axis::Vertical));
    assert_eq!(split.ratio(), 0.5);
}
#[test]
fn split_infeasible_minima_switches_single_visible_panel() {
    let mut split = SplitView::new(0.4);
    split.min_pane = 24.; // Explicit consumer requirement.
    let narrow = Rect::new(0., 0., 40., 80.); // 39 content points cannot hold two 24pt panes.
    assert!(split.is_constrained(narrow, Axis::Horizontal));
    assert!(!split.begin_drag(20., narrow, Axis::Horizontal));
    let (leading, divider, trailing) = split.layout(narrow, Axis::Horizontal);
    assert_eq!(leading, narrow);
    assert!(divider.is_empty());
    assert!(trailing.is_empty());
    assert!(split.key_in(ControlKey::Right, narrow, Axis::Horizontal));
    assert_eq!(split.visible_pane(), Pane::Trailing);
    let (leading, divider, trailing) = split.layout(narrow, Axis::Horizontal);
    assert!(leading.is_empty());
    assert!(divider.is_empty());
    assert_eq!(trailing, narrow);
    assert!(split.key_in(ControlKey::Left, narrow, Axis::Horizontal));
    assert_eq!(split.visible_pane(), Pane::Leading);
}
#[test]
fn split_cancel_restores_ratio_and_switchable_pane_after_latched_drag() {
    let mut split = SplitView::new(0.5);
    split.min_pane = 24.; // This fixture explicitly exercises constrained fallback.
    split.choose_visible(Pane::Trailing);
    let bounds = Rect::new(0., 0., 160., 50.);
    assert!(split.begin_drag(80., bounds, Axis::Horizontal));
    assert!(!split.drag_to(83., bounds, Axis::Horizontal));
    assert!(split.drag_to(84., bounds, Axis::Horizontal));
    assert_eq!(split.collapsed, None);
    assert!(split.drag_to(82., bounds, Axis::Horizontal)); // latch persists inside the dead zone.
    assert!(split.cancel_drag());
    assert_eq!(split.visible_pane(), Pane::Trailing);
    let (leading, _, trailing) = split.layout(Rect::new(0., 0., 40., 50.), Axis::Horizontal);
    assert!(leading.is_empty());
    assert_eq!(trailing.size.width, 40.);
}
#[test]
fn toast_uses_reproducible_and_paused_time() {
    let mut toast = Toast::new("saved", NoticeTone::Success, 10);
    toast.paused = true;
    assert!(!toast.advance(100));
    assert_eq!(toast.remaining_ms(), 10);
    toast.paused = false;
    assert!(!toast.advance(4));
    assert!(toast.advance(20));
    assert!(!toast.visible);
    toast.show("retry", NoticeTone::Info, 5);
    assert_eq!(toast.remaining_ms(), 5);
}
#[test]
fn progress_painter_clamps_negative_geometry_and_semantic_tokens_exist() {
    for theme in tekes_ui_tokens::Theme::ALL {
        let mut scene = Scene::new();
        let palette: Palette = theme.into();
        progress(
            &mut scene,
            Rect::new(0., 0., -4., -3.),
            theme,
            Progress::new(1.),
        );
        switch(&mut scene, Rect::new(0., 0., 32., 18.), theme, true, false);
        radio(&mut scene, Rect::new(0., 0., 20., 20.), theme, true, false);
        status_badge(
            &mut scene,
            Rect::new(0., 0., 80., 20.),
            theme,
            StatusBadge::Succeeded,
        );
        let ink = link_underline(
            &mut scene,
            Rect::new(0., 0., 80., 20.),
            theme,
            ControlState {
                focused: true,
                focus_visible: true,
                ..ControlState::default()
            },
        );
        assert_eq!(ink, palette.color("c-link-foreground").unwrap());
        assert!(scene.len() >= 9);
    }
}
#[test]
fn stateful_control_painters_use_brand_tokens_for_all_themes() {
    for theme in tekes_ui_tokens::Theme::ALL {
        for brand in [0, 1, 2] {
            let palette = Palette::new(theme, brand).expect("registered Gallery brand");
            let checked_focus = ControlState {
                selected: true,
                focused: true,
                focus_visible: true,
                ..ControlState::default()
            };
            let mut scene = Scene::new();
            switch_with_state(
                &mut scene,
                Rect::new(0., 0., 42., 24.),
                palette,
                checked_focus,
            );
            let stroke = proposal::metric("stroke", "stroke-rule").unwrap();
            for index in [0, 1] {
                let tekes_ui_scene::Primitive::Quad(quad) = &scene.primitives()[index] else {
                    panic!("switch paints quads")
                };
                assert_eq!(quad.border_width, stroke);
                assert_eq!(
                    quad.border_color,
                    tekes_ui_tokens::surface_palette::resolve_toggle_surface(
                        palette,
                        "canvas",
                        "selected",
                        tekes_ui_tokens::surface_palette::SurfaceOptions::default(),
                    )
                    .unwrap()
                    .track
                    .boundary
                    .unwrap()
                );
            }
            radio_with_state(
                &mut scene,
                Rect::new(50., 0., 24., 24.),
                palette,
                checked_focus,
            );
            let model = Slider::new(25., 0., 100., 1.).unwrap();
            assert_eq!(model.normalized_value(), 0.25);
            let geometry = slider(
                &mut scene,
                Rect::new(0., 40., 120., 24.),
                palette,
                model,
                ControlState {
                    hovered: true,
                    ..ControlState::default()
                },
            );
            assert!(geometry.thumb_center_x > geometry.track.min_x());
            assert!(geometry.thumb_center_x < geometry.track.max_x());
            let roles = [
                "c-switch-on-ground",
                "c-switch-on-thumb",
                "control-checked-hover",
                "control-checked-pressed",
                "c-radio-checked-ground",
                "c-radio-checked-mark",
                "c-progress-active",
                "border-focused",
            ];
            assert!(roles.iter().all(|role| palette.color(role).is_some()));
            let focus_rings = scene.primitives().iter().filter(|p| matches!(p,
                tekes_ui_scene::Primitive::Quad(q) if q.fill.a == 0.0
                    && q.border_color == palette.color("border-focused").unwrap()
                    && q.border_width == proposal::metric("stroke", "c-focus-width").unwrap()
            )).count();
            assert_eq!(
                focus_rings, 1,
                "Switch keeps its independent ring; Radio uses its indicator border"
            );
            assert!(scene.primitives().iter().any(|p| matches!(p,
                tekes_ui_scene::Primitive::Quad(q)
                    if q.rect == Rect::new(50., 0., 24., 24.)
                        && q.fill == palette.color("c-radio-checked-ground").unwrap()
                        && q.border_color == palette.color("border-focused").unwrap()
                        && q.border_width == proposal::metric("stroke", "c-focus-width").unwrap()
            )));

            for (state, role) in [
                (
                    ControlState {
                        selected: true,
                        hovered: true,
                        ..ControlState::default()
                    },
                    "control-checked-hover",
                ),
                (
                    ControlState {
                        selected: true,
                        pressed: true,
                        ..ControlState::default()
                    },
                    "control-checked-pressed",
                ),
            ] {
                let mut interactive = Scene::new();
                switch_with_state(
                    &mut interactive,
                    Rect::new(0., 0., 42., 24.),
                    palette,
                    state,
                );
                radio_with_state(
                    &mut interactive,
                    Rect::new(50., 0., 24., 24.),
                    palette,
                    state,
                );
                assert_eq!(
                    scene_quad_fill(&interactive, 0),
                    palette.color(role).unwrap()
                );
                assert_eq!(
                    scene_quad_fill(&interactive, 2),
                    palette.color(role).unwrap()
                );
            }

            let mut disabled = Scene::new();
            let state = ControlState {
                selected: true,
                disabled: true,
                focused: true,
                focus_visible: true,
                ..ControlState::default()
            };
            switch_with_state(&mut disabled, Rect::new(0., 0., 42., 24.), palette, state);
            radio_with_state(&mut disabled, Rect::new(50., 0., 24., 24.), palette, state);
            assert_eq!(
                disabled.len(),
                4,
                "disabled Switch/Radio have no focus rings"
            );
            assert_eq!(
                scene_quad_fill(&disabled, 3),
                palette.color("disabled-text").unwrap()
            );
            for index in [0, 1] {
                let tekes_ui_scene::Primitive::Quad(quad) = &disabled.primitives()[index] else {
                    panic!("switch paints quads")
                };
                assert_eq!(quad.border_width, stroke);
                assert_eq!(quad.border_color, palette.color("disabled-text").unwrap());
            }
        }
    }
}
#[test]
fn split_keyboard_restores_both_panes_after_constrained_resize() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let narrow = Rect::new(0., 0., 120., 120.);
        let wide = Rect::new(0., 0., 1000., 1000.);
        let mut split = SplitView::new(0.5);
        split.min_pane = 160.;
        assert!(split.key_in(ControlKey::Right, narrow, axis));
        assert!(split.collapsed.is_some());
        assert!(split.key_in(ControlKey::Home, wide, axis));
        assert_eq!(split.collapsed, None);
        let (leading, divider, trailing) = split.layout(wide, axis);
        assert!(leading.size.width > 0. && leading.size.height > 0.);
        assert!(divider.size.width > 0. && divider.size.height > 0.);
        assert!(trailing.size.width > 0. && trailing.size.height > 0.);
    }
}

fn scene_quad_fill(scene: &Scene, index: usize) -> Rgba {
    match &scene.primitives()[index] {
        tekes_ui_scene::Primitive::Quad(q) => q.fill,
        _ => panic!("expected a component quad"),
    }
}
#[test]
fn data_table_selects_rows_and_toggles_sort_direction() {
    let mut table = DataTable::new(3, 2);
    assert!(table.sort_by(0));
    assert_eq!(table.sort, Some((0, SortDirection::Ascending)));
    assert!(table.sort_by(0));
    assert_eq!(table.sort, Some((0, SortDirection::Descending)));
    assert!(table.key(ControlKey::End));
    assert_eq!(table.selected_row, Some(2));
    assert!(!table.sort_by(2));
}
