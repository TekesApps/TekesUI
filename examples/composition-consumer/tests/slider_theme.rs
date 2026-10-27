#![cfg(target_os = "macos")]
use tekes_ui_controls::{ControlKey, ControlState, SliderLayout, SliderView, extended::Slider};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme};
fn assert_slider(scene: &Scene, layout: SliderLayout, palette: Palette, state: ControlState) {
    let (index, track) = scene
        .primitives()
        .iter()
        .enumerate()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == layout.track => Some((i, *q)),
            _ => None,
        })
        .expect("actual Slider own track paint");
    assert_eq!(
        track.fill,
        palette
            .color(if state.disabled {
                "surface-2"
            } else {
                "surface-3"
            })
            .unwrap()
    );
    let fill = scene.primitives()[index + 1..]
        .iter()
        .find_map(|p| match p {
            Primitive::Quad(q)
                if q.rect.min_x() == layout.track.min_x()
                    && q.rect.min_y() == layout.track.min_y()
                    && q.rect.max_x() <= layout.track.max_x()
                    && q.rect.size.height == layout.track.size.height =>
            {
                Some(*q)
            }
            _ => None,
        })
        .expect("actual active value track after its own ground");
    assert_eq!(
        fill.fill,
        palette
            .color(if state.disabled {
                "disabled-text"
            } else if state.pressed {
                "accent-pressed"
            } else if state.hovered {
                "accent-hover"
            } else {
                "accent"
            })
            .unwrap()
    );
    let thumb = scene.primitives()[index + 1..]
        .iter()
        .find_map(|p| match p {
            Primitive::Quad(q) if q.rect == layout.thumb => Some(*q),
            _ => None,
        })
        .expect("actual thumb after own track paint");
    assert_eq!(
        thumb.fill,
        palette
            .color(if state.disabled {
                "surface-2"
            } else {
                "surface-0"
            })
            .unwrap()
    );
    assert_eq!(
        thumb.border_color,
        palette
            .color(if state.disabled {
                "disabled-text"
            } else {
                "border"
            })
            .unwrap()
    );
    if !state.disabled {
        assert!(
            contrast(fill.fill, track.fill) >= 3.,
            "actual active/rest track contrast"
        );
        for adjacent in [fill.fill, track.fill] {
            assert!(
                contrast(thumb.fill, adjacent).max(contrast(thumb.border_color, adjacent)) >= 3.,
                "actual thumb fill-or-required-boundary/adjacent track contrast"
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

fn draw(view: &mut SliderView, model: Slider, theme: Theme) -> (Scene, SliderLayout) {
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        model,
        Rect::new(20., 30., 400., 36.),
        Some(Rect::new(0., 0., 480., 160.)),
        theme.into(),
    );
    (scene, layout)
}
#[test]
fn slider_same_instance_repaints_themes_and_preserves_bounded_drag_keyboard_and_cancel() {
    let mut model = Slider::new(50., 0., 100., 5.).unwrap();
    let mut view = SliderView::new();
    let mut original = None;
    for theme in Theme::ALL {
        let (scene, layout) = draw(&mut view, model, theme);
        assert_eq!(model.value(), 50.);
        assert!(!view.is_focused());
        if let Some(before) = original {
            assert_eq!(layout, before);
        } else {
            original = Some(layout);
        }
        assert_slider(&scene, layout, theme.into(), ControlState::default());
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    let (_, layout) = draw(&mut view, model, theme);
    let point = layout.thumb.center();
    view.pointer_move(point, &mut model);
    let (scene, layout) = draw(&mut view, model, theme);
    assert!(!view.is_focused(), "hover requires no prior focus");
    assert_slider(
        &scene,
        layout,
        palette,
        ControlState {
            hovered: true,
            ..Default::default()
        },
    );
    assert!(view.pointer_down(point, &mut model));
    let (scene, layout) = draw(&mut view, model, theme);
    assert!(view.is_captured());
    assert_slider(
        &scene,
        layout,
        palette,
        ControlState {
            pressed: true,
            ..Default::default()
        },
    );
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.))
    );
    view.pointer_move(Point::new(layout.track.max_x() + 100., point.y), &mut model);
    assert_eq!(model.value(), 100.);
    assert!(view.cancel());
    assert!(!view.is_captured());
    assert!(!view.pointer_up(point, &model));
    view.pointer_move(Point::new(layout.track.min_x(), point.y), &mut model);
    assert_eq!(
        model.value(),
        100.,
        "cancel ends capture without rolling back the live value"
    );
    view.focus(true);
    assert!(view.key(ControlKey::Home, true, &mut model));
    assert_eq!(model.value(), 0.);
    assert!(view.key(ControlKey::Right, true, &mut model));
    assert_eq!(model.value(), 5.);
    assert!(view.key(ControlKey::End, true, &mut model));
    assert_eq!(model.value(), 100.);
    let (scene, layout) = draw(&mut view, model, theme);
    assert_slider(
        &scene,
        layout,
        palette,
        ControlState {
            hovered: true,
            ..Default::default()
        },
    );
    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
        && q.rect.center()==layout.thumb.center() && q.border_color==palette.color("border-focused").unwrap())));
    let point = layout.track.center();
    assert!(view.pointer_down(point, &mut model));
    assert_eq!(model.value(), 50.);
    model.disabled = true;
    let (scene, layout) = draw(&mut view, model, theme);
    assert_slider(
        &scene,
        layout,
        palette,
        ControlState {
            disabled: true,
            ..Default::default()
        },
    );
    assert!(!view.is_captured());
    assert!(!view.is_focused());
    assert!(!view.pointer_up(point, &model));
    assert!(!view.key(ControlKey::Right, true, &mut model));
    assert_eq!(model.value(), 50.);
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.))
    );
    model.disabled = false;
    draw(&mut view, model, theme);
    assert!(view.pointer_down(point, &mut model));
    view.set_window_active(false);
    assert!(!view.is_captured());
    assert!(!view.is_focused());
    assert!(!view.pointer_up(point, &model));
    view.set_window_active(true);
    draw(&mut view, model, theme);
    assert!(view.pointer_down(point, &mut model));
    view.pointer_move(Point::new(layout.track.min_x(), point.y), &mut model);
    assert_eq!(model.value(), 0.);
    assert!(view.pointer_up(point, &model));
    assert!(!view.is_captured());
}
