#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey, ControlState, SelectionEvent, SwitchLayout, SwitchView, extended::Switch,
};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme};
fn assert_switch(
    scene: &Scene,
    geometry: (Rect, Rect),
    palette: Palette,
    state: ControlState,
    ground: &str,
    backing: f32,
    clear: Rgba,
) -> Rect {
    let (track, label) = geometry;
    let (index, outer) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == track => Some((i, *q)),
            _ => None,
        })
        .expect("actual Switch track background");
    assert_eq!(outer.fill, palette.color(ground).unwrap());
    let boundary = if state.disabled {
        "disabled-text"
    } else {
        "border"
    };
    assert_eq!(outer.border_color, palette.color(boundary).unwrap());
    let thumbs: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q)
                if q.fill.a > 0.
                    && q.rect.min_x() >= track.min_x()
                    && q.rect.max_x() <= track.max_x()
                    && q.rect.min_y() >= track.min_y()
                    && q.rect.max_y() <= track.max_y() =>
            {
                Some(*q)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        thumbs.len(),
        1,
        "exactly the own thumb after the actual track paint"
    );
    let thumb = thumbs[0];
    assert_eq!(
        thumb.fill,
        palette
            .color(if state.disabled {
                "disabled-text"
            } else if state.selected {
                "c-switch-on-thumb"
            } else {
                "surface-0"
            })
            .unwrap()
    );
    assert_eq!(thumb.border_color, palette.color(boundary).unwrap());
    assert!(
        if state.selected {
            thumb.rect.center().x > track.center().x
        } else {
            thumb.rect.center().x < track.center().x
        },
        "reduced-motion on/off endpoint"
    );
    if !state.disabled {
        if state.selected {
            assert!(
                contrast(thumb.fill, outer.fill) >= 3.,
                "actual on thumb/track contrast"
            );
        } else {
            assert!(
                contrast(thumb.fill, outer.fill).max(contrast(thumb.border_color, outer.fill))
                    >= 3.,
                "actual off thumb fill-or-boundary/track contrast"
            );
        }
    }
    let parent = scene.primitives()[..index]
        .iter()
        .rev()
        .find_map(|p| match p {
            Primitive::Quad(q)
                if q.fill.a == 1.
                    && q.rect.min_x() <= label.min_x()
                    && q.rect.max_x() >= label.max_x()
                    && q.rect.min_y() <= label.min_y()
                    && q.rect.max_y() >= label.max_y() =>
            {
                Some(q.fill)
            }
            _ => None,
        })
        .unwrap_or(clear);
    let inks: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|g| {
                    let a = g.glyph;
                    label
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
        "real CoreText Switch label glyphs after own track paint"
    );
    for ink in inks {
        assert_eq!(
            ink,
            palette
                .color(if state.disabled {
                    "disabled-text"
                } else {
                    "primary-text"
                })
                .unwrap()
        );
        if !state.disabled {
            assert!(
                contrast(ink, parent)
                    >= if palette.theme.is_increased() {
                        7.
                    } else {
                        4.5
                    },
                "actual label/parent ground contrast"
            );
        }
    }
    thumb.rect
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
    view: &mut SwitchView,
    model: Switch,
    text: &mut coretext::Text,
    theme: Theme,
) -> (Scene, SwitchLayout) {
    let palette: Palette = theme.into();
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 420., 150.),
        palette.color("surface-1").unwrap(),
    ));
    let layout = view.render_labeled(
        &mut scene,
        text,
        model,
        Rect::new(20., 30., 340., 32.),
        Some(Rect::new(0., 0., 420., 150.)),
        palette,
        "Notifications",
    );
    (scene, layout)
}
#[test]
fn switch_same_instance_repaints_on_off_themes_and_preserves_toggle_and_cancel_contracts() {
    let mut model = Switch::default();
    let mut view = SwitchView::new();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    for checked in [false, true] {
        model.checked = checked;
        let mut original = None;
        for theme in Theme::ALL {
            let palette: Palette = theme.into();
            let (scene, layout) = draw(&mut view, model, &mut text, theme);
            assert_eq!(model.checked, checked);
            assert!(!view.is_focused());
            let thumb = assert_switch(
                &scene,
                (layout.toggle.frame(), layout.label),
                palette,
                ControlState {
                    selected: checked,
                    ..Default::default()
                },
                if checked {
                    "c-switch-on-ground"
                } else {
                    "surface-3"
                },
                text.backing,
                palette.color("surface-1").unwrap(),
            );
            if let Some(before) = original {
                assert_eq!((layout, thumb), before);
            } else {
                original = Some((layout, thumb));
            }
        }
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    let clear = palette.color("surface-1").unwrap();
    for checked in [false, true] {
        model.checked = checked;
        view.blur();
        let (_, layout) = draw(&mut view, model, &mut text, theme);
        let point = layout.toggle.frame().center();
        view.pointer_move(point);
        let (scene, layout) = draw(&mut view, model, &mut text, theme);
        assert!(!view.is_focused(), "hover must work before focus");
        assert_switch(
            &scene,
            (layout.toggle.frame(), layout.label),
            palette,
            ControlState {
                selected: checked,
                ..Default::default()
            },
            if checked {
                "control-checked-hover"
            } else {
                "button-hover"
            },
            text.backing,
            clear,
        );
        assert!(view.pointer_down(point, &model));
        let (scene, layout) = draw(&mut view, model, &mut text, theme);
        assert_switch(
            &scene,
            (layout.toggle.frame(), layout.label),
            palette,
            ControlState {
                selected: checked,
                ..Default::default()
            },
            if checked {
                "control-checked-pressed"
            } else {
                "button-pressed"
            },
            text.backing,
            clear,
        );
        assert_eq!(
            view.pointer_up(Point::new(400., 120.), &mut model),
            SelectionEvent::Handled
        );
        assert_eq!(model.checked, checked);
        view.pointer_move(Point::new(400., 120.));
    }
    view.focus(true);
    let (scene, layout) = draw(&mut view, model, &mut text, theme);
    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
        && q.rect.center()==layout.toggle.frame().center() && q.border_color==palette.color("border-focused").unwrap())));
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        SelectionEvent::Ignored
    );
    assert_eq!(
        view.key(ControlKey::Space, true, &mut model),
        SelectionEvent::Handled
    );
    assert!(model.checked);
    assert_eq!(
        view.key(ControlKey::Space, false, &mut model),
        SelectionEvent::Changed
    );
    assert!(!model.checked);
    let point = layout.toggle.frame().center();
    assert!(view.pointer_down(point, &model));
    model.disabled = true;
    let (scene, layout) = draw(&mut view, model, &mut text, theme);
    assert_switch(
        &scene,
        (layout.toggle.frame(), layout.label),
        palette,
        ControlState {
            disabled: true,
            ..Default::default()
        },
        "surface-2",
        text.backing,
        clear,
    );
    assert!(!view.is_focused());
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.))
    );
    assert_eq!(view.pointer_up(point, &mut model), SelectionEvent::Ignored);
    assert!(!model.checked);
    assert_eq!(
        view.key(ControlKey::Space, true, &mut model),
        SelectionEvent::Ignored
    );
    assert!(!model.checked);
    model.disabled = false;
    draw(&mut view, model, &mut text, theme);
    assert!(view.pointer_down(point, &model));
    assert_eq!(view.pointer_up(point, &mut model), SelectionEvent::Changed);
    assert!(model.checked);
}
