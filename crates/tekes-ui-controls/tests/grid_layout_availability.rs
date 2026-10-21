use tekes_ui_controls::{
    extended::{Axis, Grid, Stack},
    layout_view::{GridView, StackView},
};
use tekes_ui_scene::{Point, Rect};
#[test]
fn grid_zero_area_cannot_publish_child_hit_frames_and_restore_works() {
    let mut view = GridView::new(Grid::new(2));
    let bounds = Rect::new(8., 8., 240., 120.);
    assert!(
        view.render(bounds, 4, 40., None)
            .items
            .iter()
            .any(|c| c.contains(Point::new(20., 20.)))
    );
    for empty in [Rect::new(8., 8., 240., 0.), Rect::new(8., 8., 0., 120.)] {
        let layout = view.render(empty, 4, 40., None);
        assert!(layout.visible.is_none());
        assert!(
            !layout.items.iter().any(|c| c.visible_frame().is_some()),
            "unavailable Grid must not publish paint/hit targets: {empty:?}"
        );
        assert!(
            !layout
                .items
                .iter()
                .any(|c| c.contains(Point::new(20., 20.)))
        )
    }
    assert!(
        view.render(bounds, 4, 40., None)
            .items
            .iter()
            .any(|c| c.contains(Point::new(20., 20.)))
    );
}
#[test]
fn positive_grid_keeps_natural_overflow_and_true_ancestor_clipping() {
    let mut view = GridView::new(Grid::new(1));
    let bounds = Rect::new(8., 8., 100., 20.);
    let clip = Rect::new(0., 0., 120., 60.);
    let layout = view.render(bounds, 3, 40., Some(clip));
    assert_eq!(layout.items.len(), 3);
    assert_eq!(layout.items[1].frame(), Rect::new(8., 48., 100., 40.));
    assert_eq!(
        layout.items[1].visible_frame(),
        Some(Rect::new(8., 48., 100., 12.))
    );
    assert!(layout.items[2].visible_frame().is_none());
}
#[test]
fn pure_grid_measure_geometry_and_existing_stack_zero_area_behavior_are_preserved() {
    let bounds = Rect::new(8., 8., 100., 0.);
    let grid = Grid::new(1);
    let measured = grid.layout(bounds, 2, 40.);
    assert_eq!(measured.len(), 2);
    assert_eq!(measured[1], Rect::new(8., 48., 100., 40.));
    let mut stack = StackView::new(Stack::new(Axis::Vertical));
    assert!(
        stack
            .render(bounds, 2, None)
            .items
            .iter()
            .all(|c| c.visible_frame().is_none())
    );
}
