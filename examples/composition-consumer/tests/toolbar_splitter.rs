use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text(f32);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * self.0 * 0.5).min(width),
            role.style().line_height * self.0 / proposal::TOOLBAR_REFERENCE_TITLE_SIZE,
        )
    }
    fn resolved_font_size(&mut self, _: proposal::TextRole) -> f32 {
        self.0
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
use tekes_ui_controls::{
    ControlKey, ToolbarAction, ToolbarControl,
    extended::{Axis, SplitView},
    layout_view::SplitViewView,
};
use tekes_ui_scene::{Point, Primitive, Quad};
fn quads(s: &Scene) -> Vec<Quad> {
    s.primitives()
        .iter()
        .filter_map(|p| {
            if let Primitive::Quad(q) = p {
                Some(*q)
            } else {
                None
            }
        })
        .collect()
}
fn frame() -> Rect {
    Rect::new(0., 0., 800., 400.)
}
fn band(height: f32) -> Rect {
    Rect::new(0., 0., 800., height)
}
fn setup(ratio: f32) -> (SplitView, SplitViewView) {
    let mut m = SplitView::new(ratio);
    m.min_pane = 40.;
    let mut v = SplitViewView::new(Axis::Horizontal);
    v.arrange(&mut m, frame(), None);
    (m, v)
}
#[test]
fn content_line_and_short_rounded_toolbar_mark_share_x_across_themes_and_scales() {
    for theme in Theme::ALL {
        for scale in [1., 1.25, 1.5, 2., 3.] {
            for height in [32., 42., 64.615, 84.] {
                for ratio in [1. / 3., 0.5, 2. / 3.] {
                    let (m, mut v) = setup(ratio);
                    v.set_toolbar_region(Some(band(height)), &[], &mut Text(13.));
                    let mut s = Scene::new();
                    v.paint(&mut s, &m, theme.into(), scale);
                    let q = quads(&s);
                    assert_eq!(q.len(), 2);
                    let content = q.iter().find(|q| q.rect.min_y() == height).unwrap();
                    let mark = q.iter().find(|q| q.rect.max_y() < height).unwrap();
                    assert!(mark.rect.min_y() > 0. && mark.rect.size.height < height);
                    assert!((mark.rect.center().x - content.rect.center().x).abs() < 0.001);
                    assert!(mark.corner_radii.top_left > 0.);
                    assert_eq!(mark.fill, content.fill);
                    let (width, color) = tekes_ui_controls::paint::decorative_border(theme, scale);
                    assert_eq!(mark.rect.size.width, width);
                    assert_eq!(mark.fill, color);
                }
            }
        }
    }
}
#[test]
fn original_toolbar_buttons_and_overflow_shield_full_hits_including_disabled() {
    for width in [100., 220., 420.] {
        for disabled in [false, true] {
            let mut toolbar = ToolbarControl::window();
            toolbar.set_actions(vec![
                ToolbarAction {
                    id: 7,
                    title: "Save document".into(),
                    enabled: !disabled,
                },
                ToolbarAction {
                    id: 8,
                    title: "Publish".into(),
                    enabled: true,
                },
            ]);
            let mut h = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
            let bounds = Rect::new(230., 0., width, 42.);
            h.render(
                &mut Scene::new(),
                &mut Text(13.),
                bounds,
                frame(),
                Theme::LIGHT.into(),
            )
            .unwrap();
            let regions = h.pointer_regions();
            assert!(!regions.is_empty());
            for control in regions {
                let x = control.center().x;
                let (mut m, mut v) = setup(x / (frame().size.width - 1.));
                v.set_toolbar_region(Some(band(42.)), &h.pointer_regions(), &mut Text(13.));
                let mut scene = Scene::new();
                v.paint(&mut scene, &m, Theme::LIGHT.into(), 2.);
                assert!(
                    quads(&scene).iter().all(|q| q.rect.min_y() >= 42.),
                    "divider crossed an original button or overflow trigger"
                );
                assert!(!v.pointer_down(control.center(), &mut m));
                assert!(
                    v.resize_regions(&m)
                        .iter()
                        .all(|(r, _)| r.intersection(&control).is_none())
                );
            }
        }
    }
}
#[test]
fn captured_drag_keeps_mark_in_blank_segment_seam_and_rechecks_occupied_geometry() {
    let (mut m, mut v) = setup(0.3);
    let control = Rect::new(480., 7., 60., 28.);
    v.set_toolbar_region(Some(band(42.)), &[control], &mut Text(13.));
    let start = Point::new(v.layout().divider.frame().center().x, 21.);
    assert!(v.pointer_down(start, &mut m));
    let mut during = Scene::new();
    v.paint(&mut during, &m, Theme::LIGHT.into(), 2.);
    assert_eq!(
        quads(&during).len(),
        2,
        "dragging retains the short seam marker"
    );
    assert!(v.pointer_move(Point::new(control.center().x, 21.), &mut m));
    v.arrange(&mut m, frame(), None);
    let mut scene = Scene::new();
    v.paint(&mut scene, &m, Theme::LIGHT.into(), 2.);
    assert!(quads(&scene).iter().all(|q| q.rect.min_y() >= 42.));
    assert!(v.pointer_up(&mut m));
    let mut scene = Scene::new();
    v.paint(&mut scene, &m, Theme::LIGHT.into(), 2.);
    assert!(quads(&scene).iter().all(|q| q.rect.min_y() >= 42.));
    v.set_toolbar_region(Some(band(42.)), &[], &mut Text(13.));
    let mut scene = Scene::new();
    v.paint(&mut scene, &m, Theme::LIGHT.into(), 2.);
    assert_eq!(quads(&scene).len(), 2);
    let start = Point::new(v.layout().divider.frame().center().x, 21.);
    let original = m.ratio();
    assert!(v.pointer_down(start, &mut m));
    v.pointer_move(Point::new(360., 21.), &mut m);
    assert!(v.key(ControlKey::Escape, &mut m));
    v.arrange(&mut m, frame(), None);
    assert_eq!(m.ratio(), original);
    let mut scene = Scene::new();
    v.paint(&mut scene, &m, Theme::LIGHT.into(), 2.);
    assert_eq!(quads(&scene).len(), 2);
}
#[test]
fn resize_cursor_and_pointer_partition_use_identical_control_exclusions() {
    let (mut m, mut v) = setup(0.5);
    let hit = v.layout().divider_hit.unwrap();
    let controls = [
        Rect::new(hit.min_x() - 4., 8., 8., 24.),
        Rect::new(hit.center().x, 0., 20., 38.),
    ];
    v.set_toolbar_region(Some(band(42.)), &controls, &mut Text(13.));
    for y in [0., 4., 10., 21., 35., 40., 60., 200.] {
        for x in [
            hit.min_x(),
            hit.min_x() + 1.,
            hit.center().x,
            hit.max_x() - 0.1,
        ] {
            let p = Point::new(x, y);
            let allowed = hit.contains(p) && !controls.iter().any(|r| r.contains(p));
            let cursor = v.resize_regions(&m).iter().any(|(r, _)| r.contains(p));
            assert_eq!(cursor, allowed, "{p:?}");
            let down = v.pointer_down(p, &mut m);
            assert_eq!(down, allowed);
            v.pointer_up(&mut m);
        }
    }
}
#[test]
fn collapsed_hidden_and_reexpanded_panes_never_keep_stale_marks_or_hits() {
    let (mut m, mut v) = setup(0.5);
    v.set_toolbar_region(Some(band(42.)), &[], &mut Text(13.));
    m.min_pane = 600.;
    v.arrange(&mut m, frame(), None);
    let mut s = Scene::new();
    v.paint(&mut s, &m, Theme::LIGHT.into(), 2.);
    assert!(quads(&s).is_empty());
    assert!(v.resize_regions(&m).is_empty());
    m.min_pane = 40.;
    v.arrange(&mut m, frame(), None);
    let mut s = Scene::new();
    v.paint(&mut s, &m, Theme::LIGHT.into(), 2.);
    assert_eq!(quads(&s).len(), 2);
    v.set_toolbar_region(None, &[], &mut Text(13.));
    let mut s = Scene::new();
    v.paint(&mut s, &m, Theme::LIGHT.into(), 2.);
    assert_eq!(quads(&s).len(), 1);
    assert_eq!(quads(&s)[0].rect.size.height, 400.);
}

#[test]
fn segmented_original_toolbars_reflow_with_two_and_three_container_boundaries() {
    for columns in [2, 3] {
        for font in [13., 20.] {
            for width in [600., 900., 1200.] {
                let whole = Rect::new(10., 20., width, 400.);
                let band = Rect::new(10., 20., width, 42. * font / 13.);
                let mut outer = SplitView::new(0.33);
                outer.min_pane = 40.;
                let mut inner = SplitView::new(0.67);
                inner.min_pane = 40.;
                let mut ov = SplitViewView::new(Axis::Horizontal);
                let mut iv = SplitViewView::new(Axis::Horizontal);
                let mut bars: Vec<_> = (0..columns)
                    .map(|i| {
                        ContentHost::new(Content::new(
                            ComponentId(1),
                            ToolbarContent::new(vec![Content::new(
                                ComponentId(2),
                                ToolbarGroup::new(
                                    if i == 0 {
                                        ToolbarPlacement::PrimaryAction
                                    } else {
                                        ToolbarPlacement::Navigation
                                    },
                                    vec![Content::new(
                                        ComponentId(3),
                                        Button::new("Action"),
                                    )],
                                ),
                            )]),
                        ))
                        .unwrap()
                    })
                    .collect();
                let mounts: Vec<_> = bars
                    .iter()
                    .map(|h| h.semantic_mount(ComponentId(3)))
                    .collect();
                for ratio in [0.25, 0.35, 0.5] {
                    outer.set_ratio(ratio);
                    let a = ov.arrange(&mut outer, whole, None);
                    let b = iv.arrange(&mut inner, whole, None);
                    let panes = if columns == 2 {
                        vec![a.leading.frame(), a.trailing.frame()]
                    } else {
                        vec![
                            a.leading.frame(),
                            Rect::new(
                                a.trailing.frame().min_x(),
                                whole.min_y(),
                                (b.leading.frame().max_x() - a.trailing.frame().min_x()).max(0.),
                                whole.size.height,
                            ),
                            b.trailing.frame(),
                        ]
                    };
                    let mut occupied = Vec::new();
                    for (host, pane) in bars.iter_mut().zip(panes) {
                        let segment = pane.intersection(&band).unwrap();
                        host.render(
                            &mut Scene::new(),
                            &mut Text(font),
                            segment,
                            segment,
                            Theme::LIGHT.into(),
                        )
                        .unwrap();
                        for control in host.pointer_regions() {
                            assert!(
                                control.min_x() >= segment.min_x()
                                    && control.max_x() <= segment.max_x() + 0.001
                            );
                            assert!(control.intersection(&a.divider.frame()).is_none());
                            if columns == 3 {
                                assert!(control.intersection(&b.divider.frame()).is_none());
                            }
                            occupied.push(control);
                        }
                    }
                    ov.set_toolbar_region(Some(band), &occupied, &mut Text(font));
                    iv.set_toolbar_region(Some(band), &occupied, &mut Text(font));
                    let seam = Point::new(a.divider.frame().center().x, band.center().y);
                    assert!(ov.pointer_down(seam, &mut outer));
                    ov.pointer_up(&mut outer);
                    assert_eq!(
                        bars.iter()
                            .map(|h| h.semantic_mount(ComponentId(3)))
                            .collect::<Vec<_>>(),
                        mounts
                    );
                    assert!(bars.iter_mut().all(|h| h.drain_actions().is_empty()));
                }
            }
        }
    }
}

struct RasterText {
    font: f32,
    backing: f32,
}
impl ButtonTextRenderer for RasterText {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Text(self.font).measure(s, r, w)
    }
    fn resolved_font_size(&mut self, _: proposal::TextRole) -> f32 {
        self.font
    }
    fn backing_scale(&self) -> f32 {
        self.backing
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
fn icon_and_overflow_paint_phase_is_stable_without_quantizing_original_action_hits() {
    use tekes_ui_controls::ToolbarIconAction;
    use tekes_ui_scene::ImageId;
    for font in [13., 17., 20.] {
        for backing in [1., 1.25, 1.5, 2., 3.] {
            for width in [48., 250.] {
                let mut toolbar = ToolbarControl::window();
                toolbar.set_icon_actions(
                    (1..=3)
                        .map(|id| ToolbarIconAction {
                            id,
                            image: ImageId(id as u32),
                            label: format!("Action {id}"),
                            enabled: true,
                        })
                        .collect(),
                );
                toolbar.set_overflow_icon(Some(ImageId(10)));
                let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
                for step in 0..9 {
                    let x = 30. + step as f32 / 8.;
                    let frame = Rect::new(x, 10., width * font / 13., 42. * font / 13.);
                    let mut scene = Scene::new();
                    host.render(
                        &mut scene,
                        &mut RasterText { font, backing },
                        frame,
                        Rect::new(0., 0., 1000., 400.),
                        Theme::LIGHT.into(),
                    )
                    .unwrap();
                    let images: Vec<_> = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| {
                            if let Primitive::Image(q) = p {
                                Some(q)
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert!(!images.is_empty());
                    for image in images {
                        for value in [image.rect.min_x(), image.rect.min_y()] {
                            assert!((value * backing - (value * backing).round()).abs() < 0.0001);
                        }
                        let nominal = proposal::metric("size", "size-icon-standard").unwrap()
                            * font
                            / proposal::TOOLBAR_REFERENCE_TITLE_SIZE;
                        assert!(
                            (image.rect.size.width - nominal).abs() < 0.001,
                            "font-derived artwork extent must not be rounded"
                        );
                    }
                    let expected = x + 7. * font / 13.;
                    let first = host.pointer_regions()[0];
                    if width > 100. {
                        assert!(
                            (first.min_x() - expected).abs() < 0.001,
                            "paint snapping changed hit/hover geometry"
                        );
                    }
                    let action = first.center();
                    assert!(host.event(&ContentEvent::PointerDown(action)).consumed);
                    host.event(&ContentEvent::PointerUp(action));
                    host.event(&ContentEvent::Cancel);
                }
            }
        }
    }
}

#[test]
fn seam_marker_uses_original_toolbar_icon_box_height_and_row_center_at_any_body_size() {
    use tekes_ui_controls::ToolbarIconAction;
    use tekes_ui_scene::ImageId;
    for font in [10., 13., 17., 20., 26.] {
        for extra_native_overlap in [0., 30.] {
            let mut text = Text(font);
            let mut toolbar = ToolbarControl::window();
            toolbar.set_icon_actions(vec![ToolbarIconAction {
                id: 1,
                image: ImageId(1),
                label: "Settings".into(),
                enabled: true,
            }]);
            let size = toolbar.measure(&mut text);
            let band = Rect::new(0., 0., 800., size.height + extra_native_overlap);
            let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
            let rect = Rect::new(10., 0., size.width, size.height);
            host.render(
                &mut Scene::new(),
                &mut text,
                rect,
                frame(),
                Theme::LIGHT.into(),
            )
            .unwrap();
            let button = host.pointer_regions()[0];
            let (mut model, mut view) = setup(0.5);
            view.set_toolbar_region(Some(band), &host.pointer_regions(), &mut text);
            for dragging in [false, true] {
                if dragging {
                    assert!(view.pointer_down(
                        Point::new(view.layout().divider.frame().center().x, band.center().y),
                        &mut model
                    ));
                }
                let mut scene = Scene::new();
                view.paint(&mut scene, &model, Theme::LIGHT.into(), 2.);
                let marker = quads(&scene)
                    .into_iter()
                    .find(|q| q.corner_radii.top_left > 0.)
                    .unwrap();
                assert!((marker.rect.size.height - button.size.height).abs() < 0.001);
                let row_height = ToolbarControl::window().row_height(&mut text);
                assert!(
                    (marker.rect.size.height / row_height - 2. / 3.).abs() < 0.0001,
                    "toolbar seam is two thirds of the natural row, held={dragging} font={font}"
                );
                assert!((marker.rect.center().y - button.center().y).abs() < 0.001);
                let icon = proposal::metric("size", "size-icon-standard").unwrap()
                    * proposal::REGULAR_ICON_VISIBLE_HEIGHT_RATIO
                    * font
                    / proposal::TOOLBAR_REFERENCE_TITLE_SIZE;
                assert!(
                    (marker.rect.size.height - icon * proposal::TOOLBAR_BOX_TO_VISIBLE_ICON).abs()
                        < 0.001
                );
            }
        }
    }
}

// The caller owns content minima, including zero. No percentage floor may
// override them on a large window; pointer and keyboard use the same contract.
#[test]
fn consumer_minimum_is_exact_without_a_hidden_percentage_floor() {
    assert_eq!(SplitView::new(0.5).min_pane, 0.);
    assert!(!SplitView::new(0.5).begin_drag(0., Rect::default(), Axis::Horizontal));
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for length in [800., 4000., 8000.] {
            for minimum in [0., 12., 24., 120.] {
                let bounds = match axis {
                    Axis::Horizontal => Rect::new(0., 0., length, 400.),
                    Axis::Vertical => Rect::new(0., 0., 400., length),
                };
                let mut model = SplitView::new(0.5);
                model.min_pane = minimum;
                let usable = length - model.divider;
                assert!(model.begin_drag(usable / 2., bounds, axis));
                model.drag_to(-length, bounds, axis);
                let first = model.layout(bounds, axis).0;
                let first_size = if axis == Axis::Horizontal {
                    first.size.width
                } else {
                    first.size.height
                };
                assert!(
                    (first_size - minimum).abs() < 0.001,
                    "{length} {minimum} {first_size}"
                );
                model.end_drag();
                model.key_in(ControlKey::End, bounds, axis);
                let last = model.layout(bounds, axis).2;
                let last_size = if axis == Axis::Horizontal {
                    last.size.width
                } else {
                    last.size.height
                };
                assert!((last_size - minimum).abs() < 0.001);
                model.key_in(ControlKey::Home, bounds, axis);
                assert!((model.ratio_in(bounds, axis) * usable - minimum).abs() < 0.001);
                // A resize changes the ratio needed for the same physical minimum.
                let larger = Rect::new(0., 0., bounds.size.width * 2., bounds.size.height * 2.);
                model.key_in(ControlKey::Home, larger, axis);
                let first = model.layout(larger, axis).0;
                let first_size = if axis == Axis::Horizontal {
                    first.size.width
                } else {
                    first.size.height
                };
                assert!((first_size - minimum).abs() < 0.001);
            }
        }
    }
}

#[test]
fn combined_consumer_aligns_real_navigation_box_mark_and_scaled_row() {
    use tekes_ui_composition_consumer::split_toolbar::{self, NAV, ROOT, SplitToolbar};
    for font in [8., 13., 20., 26.] {
        for width in [480., 720., 1000.] {
            for ratio in [0.2, 0.5, 0.8] {
                let mut host = split_toolbar::build().unwrap();
                let mut text = Text(font);
                let row = ToolbarControl::window().row_height(&mut text);
                let f = host
                    .get_mut(ROOT)
                    .unwrap()
                    .component_mut::<SplitToolbar>()
                    .unwrap();
                f.native_leading = 69.;
                f.native_center = Some(row * 0.5);
                f.model.set_ratio(ratio);
                let bounds = Rect::new(0., 0., width, 400.);
                let mut scene = Scene::new();
                host.render(&mut scene, &mut text, bounds, bounds, Theme::LIGHT.into())
                    .unwrap();
                let nav = host.get(NAV).unwrap();
                let hit = nav
                    .component::<ToolbarControl>()
                    .unwrap()
                    .pointer_regions(nav.frame(), nav.visible().unwrap())[0];
                assert!((hit.center().y - row * 0.5).abs() < 0.001);
                let divider = split_toolbar::divider_center(&host);
                assert!(hit.min_x() >= divider.x);
                let mark = quads(&scene)
                    .into_iter()
                    .find(|q| {
                        q.rect.size.width <= 1.
                            && q.corner_radii.top_left > 0.
                            && q.rect.max_y() < row
                    })
                    .unwrap();
                assert!((mark.rect.size.height / row - 2. / 3.).abs() < 0.0001);
                assert!((mark.rect.size.height - hit.size.height).abs() < 0.001);
                assert!((mark.rect.center().y - hit.center().y).abs() < 0.001);
                assert!((mark.rect.center().x - divider.x).abs() < 0.001);
            }
        }
    }
}

#[test]
fn combined_real_content_capture_moves_navigation_then_original_button_activates() {
    use tekes_ui_composition_consumer::split_toolbar::{self, NAV, ROOT, SplitToolbar};
    let mut host = split_toolbar::build().unwrap();
    let bounds = Rect::new(0., 0., 900., 500.);
    let mut text = Text(13.);
    let render = |h: &mut ContentHost, t: &mut Text| {
        let mut scene = Scene::new();
        h.render(&mut scene, t, bounds, bounds, Theme::LIGHT.into())
            .unwrap();
        scene
    };
    let hit = |h: &ContentHost| {
        let nav = h.get(NAV).unwrap();
        nav.component::<ToolbarControl>()
            .unwrap()
            .pointer_regions(nav.frame(), nav.visible().unwrap())[0]
    };
    render(&mut host, &mut text);
    let mounted = host.semantic_mount(NAV);
    let before = hit(&host);
    let initial = split_toolbar::divider_center(&host);
    let start = Point::new(initial.x, 21.);
    let down = host.event(&ContentEvent::PointerDown(start));
    assert!(down.consumed && down.capture_pointer);
    assert!(
        host.event(&ContentEvent::PointerMove(Point::new(
            start.x + 135.25,
            start.y
        )))
        .changed
    );
    let scene = render(&mut host, &mut text);
    let after = hit(&host);
    assert!((after.min_x() - before.min_x() - 135.25).abs() < 0.001);
    assert_eq!(host.semantic_mount(NAV), mounted);
    assert!(
        host.get(ROOT)
            .unwrap()
            .component::<SplitToolbar>()
            .unwrap()
            .held
    );
    let mark = quads(&scene)
        .into_iter()
        .find(|q| q.rect.size.width <= 1. && q.corner_radii.top_left > 0. && q.rect.max_y() < 42.)
        .unwrap();
    assert!((mark.rect.size.height - 28.).abs() < 0.001);
    host.event(&ContentEvent::PointerUp(Point::new(
        start.x + 135.25,
        start.y,
    )));
    assert!(
        !host
            .get(ROOT)
            .unwrap()
            .component::<SplitToolbar>()
            .unwrap()
            .held
    );
    let nav = hit(&host).center();
    let ratio = host
        .get(ROOT)
        .unwrap()
        .component::<SplitToolbar>()
        .unwrap()
        .model
        .ratio();
    host.event(&ContentEvent::PointerDown(nav));
    host.event(&ContentEvent::PointerUp(nav));
    assert_eq!(
        host.get_mut(NAV)
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap()
            .take_action(),
        Some(1)
    );
    assert_eq!(
        host.get(ROOT)
            .unwrap()
            .component::<SplitToolbar>()
            .unwrap()
            .model
            .ratio(),
        ratio
    );
    // Original capture cancels and rolls back, even across a translated child.
    let seam = split_toolbar::divider_center(&host);
    host.event(&ContentEvent::PointerDown(Point::new(seam.x, 200.)));
    host.event(&ContentEvent::PointerMove(Point::new(seam.x - 200., 200.)));
    render(&mut host, &mut text);
    host.event(&ContentEvent::Cancel);
    render(&mut host, &mut text);
    assert_eq!(
        host.get(ROOT)
            .unwrap()
            .component::<SplitToolbar>()
            .unwrap()
            .model
            .ratio(),
        ratio
    );
    // At a clamped edge the retained split capture must not bubble into window drag.
    let seam = split_toolbar::divider_center(&host);
    host.event(&ContentEvent::PointerDown(Point::new(seam.x, 200.)));
    host.event(&ContentEvent::PointerMove(Point::new(3000., 200.)));
    render(&mut host, &mut text);
    let edge = host.event(&ContentEvent::PointerMove(Point::new(6000., 200.)));
    assert!(edge.consumed && !host.event_bubbled());
    host.event(&ContentEvent::Cancel);
    render(&mut host, &mut text);
    // Typography reflow keeps the same mounted Navigation, not a copied icon.
    text.0 = 20.;
    render(&mut host, &mut text);
    assert_eq!(host.semantic_mount(NAV), mounted);
}
