#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{CheckState, Checkbox, CheckboxLayout, ControlKey};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
fn assert_checkbox(
    scene: &Scene,
    geometry: (Rect, Rect),
    palette: Palette,
    checked: CheckState,
    ground: &str,
    disabled: bool,
    backing: f32,
) -> Rect {
    let (row, label) = geometry;
    let side = proposal::metric("size", "c-checkbox-size").unwrap();
    let (index, control) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q)
                if q.rect.min_x() == row.min_x()
                    && q.rect.size == tekes_ui_scene::Size::new(side, side)
                    && row.intersection(&q.rect) == Some(q.rect) =>
            {
                Some((i, *q))
            }
            _ => None,
        })
        .expect("actual Checkbox indicator background");
    assert_eq!(control.fill, palette.color(ground).unwrap());
    assert_eq!(
        control.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(
            proposal::metric("radius", "c-checkbox-radius").unwrap()
        )
    );
    assert_eq!(control.border_color, palette.color("border").unwrap());
    let parent = scene.primitives()[..index]
        .iter()
        .rev()
        .find_map(|p| match p {
            Primitive::Quad(q) if q.fill.a == 1. && q.rect.intersection(&label) == Some(label) => {
                Some(q.fill)
            }
            _ => None,
        })
        .expect("actual opaque parent ground under this Checkbox label");
    let marks: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q)
                if q.fill.a > 0. && control.rect.intersection(&q.rect) == Some(q.rect) =>
            {
                Some(q.fill)
            }
            _ => None,
        })
        .collect();
    if checked == CheckState::Unchecked {
        assert!(marks.is_empty());
    } else {
        assert!(!marks.is_empty(), "checked/mixed emits real mark quads");
        if checked == CheckState::Mixed {
            assert_eq!(marks.len(), 1);
        }
        let role = if disabled {
            "tertiary-text"
        } else {
            "control-checked-mark"
        };
        for ink in marks {
            assert_eq!(ink, palette.color(role).unwrap());
            if !disabled {
                assert!(contrast(ink, control.fill) >= 3., "actual mark/own ground");
            }
        }
    }
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
        "real CoreText label glyphs after this Checkbox paint"
    );
    for ink in inks {
        assert_eq!(
            ink,
            palette
                .color(if disabled {
                    "tertiary-text"
                } else {
                    "primary-text"
                })
                .unwrap()
        );
        if !disabled {
            assert!(
                contrast(ink, parent)
                    >= if palette.theme.is_increased() {
                        7.
                    } else {
                        4.5
                    },
                "actual label/parent ground"
            );
        }
    }
    control.rect
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
    checkbox: &mut Checkbox,
    text: &mut coretext::Text,
    theme: Theme,
) -> (Scene, CheckboxLayout) {
    let mut scene = Scene::new();
    let palette: Palette = theme.into();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 420., 140.),
        palette.color("surface-1").unwrap(),
    ));
    let layout = checkbox.render(
        &mut scene,
        text,
        "Receive project notifications",
        Rect::new(20., 30., 340., 40.),
        Some(Rect::new(0., 0., 420., 140.)),
        palette,
        proposal::TextRole::UiBody,
    );
    (scene, layout)
}
#[test]
fn checkbox_same_instance_repaints_all_check_states_and_preserves_original_activation() {
    let mut checkbox = Checkbox::default();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut original = None;
    for checked in [
        CheckState::Unchecked,
        CheckState::Checked,
        CheckState::Mixed,
    ] {
        checkbox.checked = checked;
        for theme in Theme::ALL {
            let (scene, layout) = draw(&mut checkbox, &mut text, theme);
            assert_eq!(checkbox.checked, checked);
            if let Some(before) = original {
                assert_eq!(layout, before);
            } else {
                original = Some(layout);
            }
            assert_checkbox(
                &scene,
                (layout.frame, layout.label),
                theme.into(),
                checked,
                if checked == CheckState::Unchecked {
                    "surface-2"
                } else {
                    "control-checked-ground"
                },
                false,
                text.backing,
            );
        }
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    let (_, layout) = draw(&mut checkbox, &mut text, theme);
    let point = layout.frame.center();
    checkbox.pointer_move_at(point);
    let (scene, layout) = draw(&mut checkbox, &mut text, theme);
    assert!(checkbox.button.state.hovered);
    assert_checkbox(
        &scene,
        (layout.frame, layout.label),
        palette,
        CheckState::Mixed,
        "control-checked-hover",
        false,
        text.backing,
    );
    assert!(checkbox.pointer_down_at(point));
    let (scene, layout) = draw(&mut checkbox, &mut text, theme);
    assert!(checkbox.button.state.pressed);
    assert_checkbox(
        &scene,
        (layout.frame, layout.label),
        palette,
        CheckState::Mixed,
        "control-checked-pressed",
        false,
        text.backing,
    );
    assert!(!checkbox.pointer_up_at(Point::new(400., 120.)));
    assert_eq!(checkbox.checked, CheckState::Mixed);
    checkbox.pointer_move_at(Point::new(400., 120.));
    checkbox.button.state.focused = true;
    checkbox.button.state.focus_visible = true;
    assert!(!checkbox.key(ControlKey::Enter, true));
    assert!(!checkbox.key(ControlKey::Enter, false));
    assert_eq!(checkbox.checked, CheckState::Mixed);
    assert!(!checkbox.key(ControlKey::Space, true));
    assert!(checkbox.key(ControlKey::Space, false));
    assert_eq!(checkbox.checked, CheckState::Checked);
    let (scene, layout) = draw(&mut checkbox, &mut text, theme);
    let control = assert_checkbox(
        &scene,
        (layout.frame, layout.label),
        palette,
        CheckState::Checked,
        "control-checked-ground",
        false,
        text.backing,
    );
    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.fill.a==0. && q.border_width>0.
        && q.rect.center()==control.center() && q.border_color==palette.color("border-focused").unwrap())));
    assert!(checkbox.pointer_down_at(point));
    checkbox.button.state.disabled = true;
    for checked in [
        CheckState::Unchecked,
        CheckState::Checked,
        CheckState::Mixed,
    ] {
        checkbox.checked = checked;
        let (scene, layout) = draw(&mut checkbox, &mut text, theme);
        assert_checkbox(
            &scene,
            (layout.frame, layout.label),
            palette,
            checked,
            "surface-2",
            true,
            text.backing,
        );
        assert!(
            !scene
                .primitives()
                .iter()
                .any(|p| matches!(p,Primitive::Quad(q) if q.fill.a==0. && q.border_width>0.))
        );
        assert!(!checkbox.pointer_up_at(point));
        assert!(!checkbox.key(ControlKey::Space, true));
        assert!(!checkbox.key(ControlKey::Space, false));
        assert!(!checkbox.toggle());
        assert_eq!(checkbox.checked, checked);
    }
    checkbox.button.state.disabled = false;
    draw(&mut checkbox, &mut text, theme);
    assert!(checkbox.pointer_down_at(point));
    assert!(checkbox.pointer_up_at(point));
    assert_eq!(
        checkbox.checked,
        CheckState::Checked,
        "matching pointer release toggles Mixed to Checked"
    );
}
