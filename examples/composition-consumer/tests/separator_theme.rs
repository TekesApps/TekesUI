//! #121: retained public Separator model/View; decorative rest-only Theme contract.
use tekes_ui_controls::{
    extended::{Axis, Separator},
    layout_view::SeparatorView,
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};

#[test]
fn separator_same_instance_themes_keep_direction_hairline_and_ancestor_clip() {
    let mut model = Separator::new(Axis::Horizontal);
    let mut view = SeparatorView::default();
    let mut colors = Vec::new();
    // Existing separator_scale tests cover brands, invalid scale and compatibility.
    // These cases add retained Theme changes for both owned direction branches.
    for theme in Theme::ALL {
        let palette = Palette::new(theme, 0).unwrap();
        let width = if theme.is_increased() {
            proposal::metric("stroke", "stroke-rule").unwrap()
        } else {
            0.5
        };
        for axis in [Axis::Horizontal, Axis::Vertical] {
            model.axis = axis;
            let (frame, clip) = match axis {
                Axis::Horizontal => (
                    Rect::new(10.25, 20.5, 100., 12.),
                    Rect::new(25., 15., 60., 30.),
                ),
                Axis::Vertical => (
                    Rect::new(10.25, 20.5, 12., 100.),
                    Rect::new(5., 35., 30., 60.),
                ),
            };
            let mut scene = Scene::new();
            let layout =
                view.render_at_scale(&mut scene, frame, model.axis, Some(clip), palette, 2.);
            assert_eq!(layout.frame, frame);
            assert_eq!(layout.visible, frame.intersection(&clip));
            assert_eq!(view.layout(), layout);
            let quads: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q) => Some(q),
                    _ => None,
                })
                .collect();
            assert_eq!(quads.len(), 1, "passive separator has only decorative ink");
            let quad = quads[0];
            assert_eq!(quad.fill, palette.color("separator").unwrap());
            assert_eq!(quad.border_width, 0.);
            let expected = match axis {
                Axis::Horizontal => Rect::new(
                    frame.min_x(),
                    frame.center().y - width * 0.5,
                    frame.size.width,
                    width,
                ),
                Axis::Vertical => Rect::new(
                    frame.center().x - width * 0.5,
                    frame.min_y(),
                    width,
                    frame.size.height,
                ),
            };
            assert_eq!(quad.rect, expected);
            assert_eq!(
                scene
                    .primitives()
                    .iter()
                    .filter(|p| matches!(p, Primitive::PushClip(r) if *r == clip))
                    .count(),
                1
            );
            assert_eq!(
                scene
                    .primitives()
                    .iter()
                    .filter(|p| matches!(p, Primitive::PopClip))
                    .count(),
                1
            );
            colors.push(quad.fill);
            let mut hidden = Scene::new();
            assert!(
                view.render_at_scale(
                    &mut hidden,
                    frame,
                    model.axis,
                    Some(Rect::new(500., 500., 10., 10.)),
                    palette,
                    2.
                )
                .visible
                .is_none()
            );
            assert!(
                hidden.is_empty(),
                "clipped separator emits no interactive or paint primitives"
            );
        }
    }
    assert!(
        colors.windows(2).any(|w| w[0] != w[1]),
        "retained View consumes each Theme palette"
    );
}
