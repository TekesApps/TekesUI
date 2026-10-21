use tekes_ui_controls::{
    extended::{self, Axis, SplitView},
    layout_view::{SeparatorView, SplitViewView},
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Contrast, Palette, Theme};

fn ink(scene: &Scene) -> Vec<(Rect, tekes_ui_tokens::Rgba)> {
    scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q) => Some((q.rect, q.fill)),
            _ => None,
        })
        .collect()
}

#[test]
fn public_separator_accepts_real_scale_without_changing_layout_or_color_role() {
    let frame = Rect::new(10.25, 20.5, 100., 12.);
    for theme in Theme::ALL {
        for brand in 0..3 {
            for axis in [Axis::Horizontal, Axis::Vertical] {
                for scale in [1., 1.5, 2., 0., -1., f32::NAN, f32::INFINITY] {
                    let palette = Palette { theme, brand };
                    let mut view = SeparatorView::default();
                    let mut scene = Scene::new();
                    let layout =
                        view.render_at_scale(&mut scene, frame, axis, None, palette, scale);
                    assert_eq!(layout.frame, frame);
                    assert_eq!(layout.visible, Some(frame));
                    let marks = ink(&scene);
                    assert_eq!(marks.len(), 1);
                    let (rect, color) = marks[0];
                    let expected = if theme.contrast == Contrast::Increased {
                        1.
                    } else if scale.is_finite() && scale > 0. {
                        1. / scale
                    } else {
                        1.
                    };
                    assert_eq!(
                        if axis == Axis::Horizontal {
                            rect.size.height
                        } else {
                            rect.size.width
                        },
                        expected
                    );
                    assert!((rect.center().x - frame.center().x).abs() < 0.00001);
                    assert!((rect.center().y - frame.center().y).abs() < 0.00001);
                    assert_eq!(color, palette.color("separator").unwrap());
                }
            }
        }
    }
}

#[test]
fn compatible_entry_points_keep_the_one_x_default() {
    let frame = Rect::new(0., 0., 100., 12.);
    for theme in Theme::ALL {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let palette = Palette::from(theme);
            let mut old = Scene::new();
            let mut explicit = Scene::new();
            extended::separator(&mut old, frame, palette, axis);
            extended::separator_at_scale(&mut explicit, frame, palette, axis, 1.);
            assert_eq!(ink(&old), ink(&explicit));
            let mut old_view = SeparatorView::default();
            let mut explicit_view = SeparatorView::default();
            let old_layout = old_view.render(&mut old, frame, axis, None, palette);
            let explicit_layout =
                explicit_view.render_at_scale(&mut explicit, frame, axis, None, palette, 1.);
            assert_eq!(old_layout, explicit_layout);
            assert_eq!(ink(&old), ink(&explicit));
        }
    }
}

#[test]
fn scaled_separator_keeps_ancestor_clip_and_rejects_unavailable_bounds() {
    let frame = Rect::new(10., 20., 100., 12.);
    for clip in [
        Rect::new(200., 200., 10., 10.),
        Rect::new(0., 0., 0., 10.),
        Rect::new(f32::NAN, 0., 10., 10.),
    ] {
        let mut view = SeparatorView::default();
        let mut scene = Scene::new();
        assert!(
            view.render_at_scale(
                &mut scene,
                frame,
                Axis::Horizontal,
                Some(clip),
                Theme::LIGHT.into(),
                2.
            )
            .visible
            .is_none()
        );
        assert!(scene.is_empty());
    }
    let mut view = SeparatorView::default();
    let mut scene = Scene::new();
    let clip = Rect::new(25., 20., 30., 12.);
    let layout = view.render_at_scale(
        &mut scene,
        frame,
        Axis::Horizontal,
        Some(clip),
        Theme::LIGHT.into(),
        2.,
    );
    assert_eq!(layout.visible, Some(clip));
    assert_eq!(ink(&scene)[0].0.size.height, 0.5);
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::PushClip(r) if *r == clip))
    );
}

#[test]
fn decorative_scale_path_does_not_thin_or_move_the_existing_split_view_divider() {
    let bounds = Rect::new(0., 0., 300., 100.);
    let mut model = SplitView::new(0.5);
    let mut view = SplitViewView::new(Axis::Horizontal);
    let mut scene = Scene::new();
    let expected = model.layout(bounds, Axis::Horizontal);
    let layout = view.render(&mut scene, &mut model, bounds, None, Theme::LIGHT.into());
    assert_eq!(layout.leading.frame(), expected.0);
    assert_eq!(layout.divider.frame(), expected.1);
    assert_eq!(layout.trailing.frame(), expected.2);
    assert_eq!(ink(&scene).len(), 1);
    assert_eq!(ink(&scene)[0].0.size.width, 1.);
}

#[test]
fn real_split_view_uses_hairline_ink_but_retains_divider_hit_and_drag_contract() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for theme in Theme::ALL {
            for scale in [1., 1.5, 2., 0., f32::NAN] {
                let bounds = Rect::new(10., 20., 300., 300.);
                let mut model = SplitView::new(0.5);
                let mut view = SplitViewView::new(axis);
                let expected = model.layout(bounds, axis);
                let mut scene = Scene::new();
                let layout = view.render_at_scale(
                    &mut scene,
                    &mut model,
                    bounds,
                    Some(bounds),
                    theme.into(),
                    scale,
                );
                assert_eq!(layout.leading.frame(), expected.0);
                assert_eq!(layout.divider.frame(), expected.1);
                assert_eq!(layout.trailing.frame(), expected.2);
                let line = ink(&scene)[0].0;
                let width = if axis == Axis::Horizontal {
                    line.size.width
                } else {
                    line.size.height
                };
                let effective = if scale.is_finite() && scale > 0. {
                    scale
                } else {
                    1.
                };
                assert_eq!(
                    width,
                    if theme.contrast == Contrast::Increased {
                        proposal_rule()
                    } else {
                        1. / effective
                    }
                );
                assert_eq!(
                    layout.divider_hit,
                    Some(expected.1.inset(-4.).intersection(&bounds).unwrap())
                );
                let start = layout.divider.frame().center();
                assert!(view.pointer_down(start, &mut model));
                let mut next = start;
                if axis == Axis::Horizontal {
                    next.x += 30.;
                } else {
                    next.y += 30.;
                }
                view.pointer_move(next, &mut model);
                assert!(model.ratio() > 0.5);
                view.cancel(&mut model);
                assert_eq!(model.ratio(), 0.5);
            }
        }
    }
}
fn proposal_rule() -> f32 {
    tekes_ui_tokens::proposal::metric("stroke", "stroke-rule").unwrap()
}
