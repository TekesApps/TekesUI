#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey, ControlState,
    extended::SegmentedControl,
    layout_view::{SegmentedControlView, SegmentedLayout},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
fn assert_segment(
    scene: &Scene,
    layout: &SegmentedLayout,
    index: usize,
    palette: Palette,
    state: ControlState,
    ground: &str,
    backing: f32,
) {
    let item = &layout.items[index];
    let (paint_index, quad) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == item.frame => Some((i, *q)),
            _ => None,
        })
        .expect("actual segment own Button ground");
    assert_eq!(quad.fill, palette.color(ground).unwrap());
    assert_eq!(quad.fill.a, 1.);
    assert_eq!(
        quad.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(
            proposal::metric("radius", "c-button-md-radius").unwrap()
        )
    );
    if state.selected && !state.disabled {
        assert_eq!(
            quad.border_width,
            proposal::metric("stroke", "stroke-rule").unwrap()
        );
        assert_eq!(quad.border_color, palette.color("border").unwrap());
        assert!(
            contrast(quad.border_color, quad.fill) >= 3.,
            "selection boundary/own ground"
        );
        let peer = layout
            .items
            .iter()
            .enumerate()
            .find(|(i, _)| *i != index)
            .unwrap()
            .1;
        let peer_ground = scene
            .primitives()
            .iter()
            .rev()
            .find_map(|p| match p {
                Primitive::Quad(q) if q.rect == peer.frame => Some(q.fill),
                _ => None,
            })
            .unwrap();
        assert!(
            contrast(quad.border_color, peer_ground) >= 3.,
            "selection boundary/actual neighboring ground"
        );
    } else {
        assert_eq!(quad.border_width, 0.);
    }
    let inks: Vec<_> = scene.primitives()[paint_index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|g| {
                    let a = g.glyph;
                    item.content
                        .intersection(&Rect::new(
                            g.origin.x + a.left as f32 / backing,
                            g.origin.y + a.top as f32 / backing,
                            a.width as f32 / backing,
                            a.height as f32 / backing,
                        ))
                        .is_some()
                }) =>
            {
                Some(run.color)
            }
            _ => None,
        })
        .collect();
    assert!(
        !inks.is_empty(),
        "actual CoreText text after this segment's own paint"
    );
    for ink in inks {
        assert_eq!(
            ink,
            palette
                .color(if state.disabled {
                    "tertiary-text"
                } else {
                    "primary-text"
                })
                .unwrap()
        );
        if !state.disabled {
            assert!(
                contrast(ink, quad.fill)
                    >= if palette.theme.is_increased() {
                        7.
                    } else {
                        4.5
                    },
                "actual text/own segment ground"
            );
        }
    }
}
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let l = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (l(a), l(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn draw(
    view: &mut SegmentedControlView,
    model: &mut SegmentedControl,
    text: &mut coretext::Text,
    theme: Theme,
) -> (Scene, SegmentedLayout) {
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        text,
        model,
        &["Day", "Week", "Month"],
        Rect::new(20., 30., 480., 38.),
        Some(Rect::new(0., 0., 540., 140.)),
        theme.into(),
        proposal::TextRole::UiBody,
        false,
    );
    (scene, layout)
}
#[test]
fn segmented_control_same_instance_repaints_themes_and_preserves_selection_focus_and_cancel() {
    let mut model = SegmentedControl::new(3);
    model.set_enabled(1, false);
    let mut view = SegmentedControlView::new(3);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut original = None;
    for theme in Theme::ALL {
        let (scene, layout) = draw(&mut view, &mut model, &mut text, theme);
        let palette: Palette = theme.into();
        assert_eq!(model.selected, 0);
        assert!(!model.is_enabled(1));
        assert_eq!(view.focused(), None);
        if let Some(before) = &original {
            assert_eq!(&layout, before);
        } else {
            original = Some(layout.clone());
        }
        for i in 0..3 {
            assert_segment(
                &scene,
                &layout,
                i,
                palette,
                ControlState {
                    selected: i == 0,
                    disabled: i == 1,
                    ..Default::default()
                },
                if i == 0 { "row-selection" } else { "surface-2" },
                text.backing,
            );
        }
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    let (_, layout) = draw(&mut view, &mut model, &mut text, theme);
    let point = layout.items[2].frame.center();
    view.pointer_move(point);
    let (scene, layout) = draw(&mut view, &mut model, &mut text, theme);
    assert_eq!(view.focused(), None);
    assert_segment(
        &scene,
        &layout,
        2,
        palette,
        ControlState::default(),
        "button-hover",
        text.backing,
    );
    assert_eq!(view.pointer_down(point), Some(2));
    let (scene, layout) = draw(&mut view, &mut model, &mut text, theme);
    assert_segment(
        &scene,
        &layout,
        2,
        palette,
        ControlState::default(),
        "button-pressed",
        text.backing,
    );
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.))
    );
    view.pointer_move(Point::new(530., 120.));
    let (scene, layout) = draw(&mut view, &mut model, &mut text, theme);
    assert_segment(
        &scene,
        &layout,
        2,
        palette,
        ControlState::default(),
        "surface-2",
        text.backing,
    );
    assert_eq!(view.pointer_up(Point::new(530., 120.), &mut model), None);
    assert_eq!(model.selected, 0);
    view.pointer_move(Point::new(530., 120.));
    view.focus(0);
    assert!(view.key(ControlKey::Right, true, &mut model));
    assert_eq!(model.selected, 2);
    assert_eq!(view.focused(), Some(2));
    let (scene, layout) = draw(&mut view, &mut model, &mut text, theme);
    assert_segment(
        &scene,
        &layout,
        2,
        palette,
        ControlState {
            selected: true,
            ..Default::default()
        },
        "row-selection",
        text.backing,
    );
    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
        && q.rect.center()==layout.items[2].frame.center() && q.border_color==palette.color("border-focused").unwrap())));
    view.focus_with_visibility(0, true);
    let down = view.key_event(ControlKey::Space, true, &mut model);
    assert!(down.consumed);
    assert!(!down.changed);
    assert_eq!(model.selected, 2);
    let up = view.key_event(ControlKey::Space, false, &mut model);
    assert!(up.consumed);
    assert!(up.changed);
    assert_eq!(model.selected, 0);
    assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
    let (_, layout) = draw(&mut view, &mut model, &mut text, theme);
    let disabled = layout.items[1].frame.center();
    assert_eq!(view.pointer_down(disabled), None);
    assert_eq!(view.pointer_up(disabled, &mut model), None);
    assert_eq!(model.selected, 0);
    assert_eq!(view.pointer_down(layout.items[2].frame.center()), Some(2));
    view.set_disabled(true);
    let (scene, layout) = draw(&mut view, &mut model, &mut text, theme);
    for i in 0..3 {
        assert_segment(
            &scene,
            &layout,
            i,
            palette,
            ControlState {
                selected: i == 0,
                disabled: true,
                ..Default::default()
            },
            "surface-2",
            text.backing,
        );
    }
    assert_eq!(view.focused(), None);
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.))
    );
    assert_eq!(view.pointer_up(point, &mut model), None);
    assert!(!view.key_event(ControlKey::Right, true, &mut model).consumed);
    assert_eq!(model.selected, 0);
    view.set_disabled(false);
    draw(&mut view, &mut model, &mut text, theme);
    assert_eq!(view.pointer_down(point), Some(2));
    assert_eq!(view.pointer_up(point, &mut model), Some(2));
    assert_eq!(model.selected, 2);
}
