use tekes_ui_controls::{ScrollArea, ScrollPolicy};
use tekes_ui_scene::{Point, Rect, Size};

#[test]
fn content_growth_and_reflow_keep_the_native_axis_lock_and_input_lifetime() {
    for viewport in [Size::new(300., 180.), Size::new(260., 160.)] {
        let mut scroll = ScrollArea::default();
        scroll.layout(Size::new(300., 180.), Size::new(600., 1600.));
        scroll.scroll_by(100., 200.);
        scroll.wheel_at(Point::new(0., 20.), true, true, false, false, 1.);
        assert!(scroll.needs_animation());
        scroll.layout(viewport, Size::new(620., 1750.));
        assert!(
            scroll.needs_animation(),
            "layout is not an input cancellation"
        );
        let before = scroll.offset;
        scroll.wheel_at(Point::new(12., 18.), true, false, false, false, 1.01);
        assert_eq!(
            scroll.offset.x, before.x,
            "native vertical lock survives reflow"
        );
        assert_eq!(scroll.offset.y, before.y + 18.);
        scroll.wheel_at(Point::new(0., 7.), true, false, false, true, 1.02);
        scroll.layout(viewport, Size::new(620., 1800.));
        assert!(scroll.needs_animation(), "layout must not end OS momentum");
        let before = scroll.offset.y;
        scroll.wheel_at(Point::new(0., 5.), true, false, false, true, 1.03);
        assert_eq!(scroll.offset.y, before + 5.);
    }
}

#[test]
fn document_updates_preserve_a_legal_elastic_return_instead_of_snapping() {
    let make = || {
        let mut scroll = ScrollArea::default();
        scroll.layout(Size::new(300., 180.), Size::new(600., 1600.));
        scroll.wheel_at(Point::new(0., -60.), true, true, true, false, 1.);
        assert!(scroll.overscroll.y < 0.);
        scroll
    };
    let mut updated = make();
    let mut reference = make();
    let before = updated.visual_offset();
    updated.layout(Size::new(300., 180.), Size::new(600., 1700.));
    assert_eq!(
        updated.visual_offset(),
        before,
        "append must not snap elastic return"
    );
    for _ in 0..5 {
        updated.tick(0.01);
        reference.tick(0.01);
        assert_eq!(updated.visual_offset(), reference.visual_offset());
    }
    updated.cancel();
    assert!(!updated.needs_animation());
    assert_eq!(updated.overscroll, Point::default());
}

#[test]
fn reflow_invalidates_only_obsolete_thumb_drag_and_unavailable_axes() {
    let mut scroll = ScrollArea::default();
    let viewport = Size::new(300., 180.);
    scroll.layout(viewport, Size::new(600., 1600.));
    scroll.scroll_by(100., 200.);
    let track = Rect::new(280., 0., 20., 180.);
    let thumb = scroll.thumb(true, track, 20.);
    assert!(scroll.pointer_down(thumb.center(), true, track, 20.));
    assert_eq!(scroll.dragging_axis(), Some(true));
    scroll.layout(viewport, Size::new(600., 1700.));
    assert_eq!(scroll.dragging_axis(), None);
    assert!(!scroll.pointer_drag(Point::new(290., 160.), track, 20.));
    scroll.wheel_at(Point::new(0., 20.), true, true, false, false, 2.);
    scroll.layout(viewport, Size::new(200., 100.));
    assert_eq!(scroll.offset, Point::default());
    assert_eq!(scroll.overscroll, Point::default());
    assert!(!scroll.needs_animation());
    assert!(!scroll.wheel_at(Point::new(5., 5.), true, false, false, true, 2.01));
    scroll.set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Enabled);
    scroll.wheel_at(Point::new(0., -20.), true, true, false, false, 3.);
    scroll.layout(viewport, Size::new(250., 120.));
    assert!(scroll.overscroll.y < 0.);
    assert_eq!(scroll.overscroll.x, 0.);
    scroll.cancel();
    assert!(!scroll.needs_animation());
}

#[test]
fn growth_releases_an_obsolete_edge_momentum_fence() {
    let mut scroll = ScrollArea::default();
    scroll.layout(Size::new(100., 100.), Size::new(400., 400.));
    scroll.scroll_by(0., 300.);
    scroll.wheel_at(Point::new(0., 20.), true, false, false, true, 1.);
    assert!(scroll.overscroll.y > 0.);
    scroll.layout(Size::new(100., 100.), Size::new(400., 800.));
    assert_eq!(scroll.overscroll.y, 0.);
    let before = scroll.offset.y;
    assert!(scroll.wheel_at(Point::new(0., 10.), true, false, false, true, 1.01));
    assert_eq!(
        scroll.offset.y,
        before + 10.,
        "old edge rebound cannot suppress valid arriving OS momentum"
    );
}
