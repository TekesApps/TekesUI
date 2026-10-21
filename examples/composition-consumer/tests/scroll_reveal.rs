//! Public viewport prerequisite for #28; the consumer supplies a target rectangle
//! only here. Stable Content item navigation will use the library's tree mapping.
use tekes_ui_controls::{
    Insets, ScrollArea, ScrollPolicy, ScrollRevealError, ScrollRevealOptions, ScrollRevealPlacement,
};
use tekes_ui_scene::{Point, Rect, Size};
fn viewport() -> ScrollArea {
    let mut scroll = ScrollArea::default();
    scroll.layout(Size::new(200., 100.), Size::new(1000., 1000.));
    scroll
}
fn options(placement: ScrollRevealPlacement) -> ScrollRevealOptions {
    ScrollRevealOptions {
        placement,
        margins: Insets::new(10., 15., 20., 5.),
        animated: false,
    }
}
#[test]
fn public_nearest_honors_the_comfort_zone_and_keeps_visible_or_oversized_items_stable() {
    let mut scroll = viewport();
    let opts = options(ScrollRevealPlacement::Nearest);
    let result = scroll
        .reveal_with(Rect::new(300., 500., 40., 20.), opts)
        .unwrap();
    assert!(result.changed);
    assert!(!result.clamped);
    assert!(!result.animated);
    assert_eq!(scroll.offset, Point::new(155., 440.));
    assert!(
        !scroll
            .reveal_with(Rect::new(300., 500., 40., 20.), opts)
            .unwrap()
            .changed
    );
    let before = scroll.offset;
    // A larger item already spans the whole usable viewport; moving would make
    // a different edge disappear without revealing more of the actual item.
    assert!(
        !scroll
            .reveal_with(Rect::new(100., 400., 400., 300.), opts)
            .unwrap()
            .changed
    );
    assert_eq!(scroll.offset, before);
}
#[test]
fn public_centered_uses_asymmetric_margins_and_reports_document_edge_clamping() {
    let mut scroll = viewport();
    let opts = options(ScrollRevealPlacement::Centered);
    let result = scroll
        .reveal_with(Rect::new(300., 500., 40., 20.), opts)
        .unwrap();
    assert_eq!(scroll.offset, Point::new(225., 465.));
    assert!(!result.clamped);
    let result = scroll
        .reveal_with(Rect::new(0., 0., 10., 10.), opts)
        .unwrap();
    assert_eq!(scroll.offset, Point::default());
    assert!(result.clamped);
    let result = scroll
        .reveal_with(Rect::new(990., 990., 10., 10.), opts)
        .unwrap();
    assert_eq!(scroll.offset, scroll.limit());
    assert!(result.clamped);
}
#[test]
fn public_animation_is_host_clocked_monotonic_and_retargets_from_the_current_position() {
    let mut scroll = viewport();
    let opts = ScrollRevealOptions::default();
    let r = scroll
        .reveal_with(Rect::new(20., 500., 20., 20.), opts)
        .unwrap();
    assert!(r.animated && scroll.needs_animation());
    assert_eq!(scroll.offset, Point::default());
    assert!(scroll.tick(0.02));
    let first = scroll.offset.y;
    assert!(first > 0. && first < 420.);
    assert!(!scroll.tick(f32::NAN));
    assert_eq!(scroll.offset.y, first);
    assert!(
        scroll
            .reveal_with(Rect::new(20., 800., 20., 20.), opts)
            .unwrap()
            .animated
    );
    assert_eq!(scroll.offset.y, first);
    assert!(scroll.tick(0.02));
    assert!(scroll.offset.y > first && scroll.offset.y < 720.);
    scroll.tick(1.);
    assert_eq!(scroll.offset.y, 720.);
    assert!(!scroll.needs_animation());
}
#[test]
fn public_reduced_motion_and_a_runtime_preference_change_finish_immediately() {
    let mut scroll = viewport();
    let opts = ScrollRevealOptions::default();
    scroll.set_reduced_motion(true);
    let r = scroll
        .reveal_with(Rect::new(20., 500., 20., 20.), opts)
        .unwrap();
    assert!(!r.animated);
    assert_eq!(scroll.offset.y, 420.);
    assert!(!scroll.needs_animation());
    scroll.set_reduced_motion(false);
    assert!(
        scroll
            .reveal_with(Rect::new(20., 800., 20., 20.), opts)
            .unwrap()
            .animated
    );
    scroll.tick(0.02);
    assert!(scroll.set_reduced_motion(true));
    assert_eq!(scroll.offset.y, 720.);
    assert!(!scroll.needs_animation());
}
#[test]
fn public_navigation_fences_old_momentum_until_fresh_user_input_then_user_input_wins() {
    for animated in [false, true] {
        let mut scroll = viewport();
        let opts = ScrollRevealOptions {
            animated,
            ..Default::default()
        };
        scroll
            .reveal_with(Rect::new(20., 500., 20., 20.), opts)
            .unwrap();
        let before = scroll.offset;
        assert!(!scroll.wheel_at(Point::new(0., 40.), true, false, false, true, 1.));
        assert_eq!(scroll.offset, before);
        // A new physical wheel delta may arrive without a phase-began callback.
        assert!(scroll.wheel_at(Point::new(0., 12.), true, false, true, false, 1.1));
        let user = scroll.offset;
        scroll.tick(1.);
        assert_eq!(scroll.offset, user);
        assert!(scroll.wheel_at(Point::new(0., 8.), true, false, true, true, 2.2));
        assert_eq!(scroll.offset.y, user.y + 8.);
    }
}
#[test]
fn public_invalid_requests_preserve_the_inflight_navigation_and_reject_nonfinite_geometry() {
    let mut scroll = viewport();
    let opts = ScrollRevealOptions::default();
    scroll
        .reveal_with(Rect::new(20., 500., 20., 20.), opts)
        .unwrap();
    for rect in [
        Rect::new(f32::NAN, 0., 1., 1.),
        Rect::new(0., 0., -1., 1.),
        Rect::new(0., f32::INFINITY, 1., 1.),
    ] {
        assert_eq!(
            scroll.reveal_with(rect, opts),
            Err(ScrollRevealError::InvalidRect)
        );
    }
    for margins in [
        Insets::new(-1., 0., 0., 0.),
        Insets::new(50., 0., 50., 0.),
        Insets::new(0., f32::NAN, 0., 0.),
    ] {
        assert_eq!(
            scroll.reveal_with(
                Rect::new(0., 0., 1., 1.),
                ScrollRevealOptions { margins, ..opts }
            ),
            Err(ScrollRevealError::InvalidMargins)
        );
    }
    assert!(scroll.needs_animation());
    scroll.tick(1.);
    assert_eq!(scroll.offset.y, 420.);
    assert_eq!(
        ScrollArea::default().reveal_with(Rect::new(0., 0., 1., 1.), opts),
        Err(ScrollRevealError::UnlaidOut)
    );
}
#[test]
fn public_axis_policy_lifecycle_and_geometry_changes_do_not_leave_a_stale_animation() {
    let mut scroll = viewport();
    scroll.set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
    let r = scroll
        .reveal_with(
            Rect::new(800., 500., 20., 20.),
            ScrollRevealOptions::default(),
        )
        .unwrap();
    assert!(r.clamped);
    scroll.tick(1.);
    assert_eq!(scroll.offset, Point::new(0., 420.));
    scroll
        .reveal_with(
            Rect::new(20., 800., 20., 20.),
            ScrollRevealOptions::default(),
        )
        .unwrap();
    scroll.cancel();
    let before = scroll.offset;
    scroll.tick(1.);
    assert_eq!(scroll.offset, before);
    scroll
        .reveal_with(
            Rect::new(20., 800., 20., 20.),
            ScrollRevealOptions::default(),
        )
        .unwrap();
    scroll.layout(Size::new(200., 100.), Size::new(200., 200.));
    scroll.tick(1.);
    assert_eq!(scroll.offset.y, 100.);
    assert!(!scroll.needs_animation());
}
#[test]
fn public_extreme_finite_geometry_and_elapsed_time_do_not_overflow_or_panic() {
    let mut scroll = viewport();
    let r = scroll
        .reveal_with(
            Rect::new(f32::MAX, f32::MAX, f32::MAX, f32::MAX),
            ScrollRevealOptions::default(),
        )
        .unwrap();
    assert!(r.clamped);
    scroll.tick(f32::MAX);
    assert_eq!(scroll.offset, scroll.limit());
    assert!(!scroll.needs_animation());
}
#[test]
fn public_start_selects_one_axis_and_moves_an_already_visible_target() {
    use tekes_ui_controls::extended::Axis;
    let mut s = viewport();
    s.scroll_by(120., 250.);
    let target = Rect::new(130., 280., 60., 20.);
    let r = s
        .reveal_with(
            target,
            options(ScrollRevealPlacement::Start(Axis::Vertical)),
        )
        .unwrap();
    assert!(r.changed);
    assert!(!r.clamped);
    assert_eq!(s.offset, Point::new(120., 270.));
    assert!(
        !s.reveal_with(
            target,
            options(ScrollRevealPlacement::Start(Axis::Vertical))
        )
        .unwrap()
        .changed
    );
    let r = s
        .reveal_with(
            target,
            options(ScrollRevealPlacement::Start(Axis::Horizontal)),
        )
        .unwrap();
    assert!(r.changed);
    assert_eq!(s.offset, Point::new(125., 270.));
}
#[test]
fn public_start_aligns_oversized_items_and_reports_only_selected_axis_clamps() {
    use tekes_ui_controls::extended::Axis;
    let mut s = viewport();
    s.scroll_by(100., 450.);
    let opts = options(ScrollRevealPlacement::Start(Axis::Vertical));
    s.reveal_with(Rect::new(800., 400., 400., 300.), opts)
        .unwrap();
    assert_eq!(s.offset, Point::new(100., 390.));
    assert!(
        s.reveal_with(Rect::new(9999., 0., 20., 200.), opts)
            .unwrap()
            .clamped
    );
    assert_eq!(s.offset, Point::new(100., 0.));
    assert!(
        s.reveal_with(Rect::new(9999., 999., 20., 200.), opts)
            .unwrap()
            .clamped
    );
    assert_eq!(s.offset, Point::new(100., 900.));
}
#[test]
fn public_start_motion_reduced_motion_and_wheel_keep_the_other_axis() {
    use tekes_ui_controls::extended::Axis;
    let mut s = viewport();
    s.scroll_by(120., 150.);
    let opts = ScrollRevealOptions {
        animated: true,
        ..options(ScrollRevealPlacement::Start(Axis::Vertical))
    };
    assert!(
        s.reveal_with(Rect::new(20., 700., 20., 300.), opts)
            .unwrap()
            .animated
    );
    s.tick(0.02);
    assert_eq!(s.offset.x, 120.);
    assert!(s.offset.y > 150. && s.offset.y < 690.);
    s.set_reduced_motion(true);
    assert_eq!(s.offset, Point::new(120., 690.));
    s.set_reduced_motion(false);
    s.reveal_with(Rect::new(20., 300., 20., 200.), opts)
        .unwrap();
    assert!(s.wheel_at(Point::new(0., 10.), true, false, true, false, 1.));
    let user = s.offset;
    s.tick(1.);
    assert_eq!(s.offset, user);
    assert_eq!(s.offset.x, 120.);
}
