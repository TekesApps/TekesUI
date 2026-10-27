#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey, ControlState, RadioGroupLayout, RadioGroupView, SelectionEvent,
    extended::RadioGroup,
};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme};
fn assert_radio(
    scene: &Scene,
    geometry: (Rect, Rect),
    palette: Palette,
    state: ControlState,
    ground: &str,
    backing: f32,
    clear: Rgba,
) {
    let (indicator, label) = geometry;
    let (index, outer) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == indicator => Some((i, *q)),
            _ => None,
        })
        .expect("actual radio indicator own background");
    assert_eq!(outer.fill, palette.color(ground).unwrap());
    assert_eq!(
        outer.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(indicator.size.width * 0.5)
    );
    assert_eq!(
        outer.border_color,
        palette
            .color(if state.disabled {
                "disabled-text"
            } else {
                "border"
            })
            .unwrap()
    );
    let marks: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q)
                if q.fill.a > 0.
                    && q.rect.min_x() >= indicator.min_x()
                    && q.rect.min_y() >= indicator.min_y()
                    && q.rect.max_x() <= indicator.max_x()
                    && q.rect.max_y() <= indicator.max_y() =>
            {
                Some(q.fill)
            }
            _ => None,
        })
        .collect();
    if state.selected {
        assert_eq!(marks.len(), 1, "selected radio emits exactly its own dot");
        assert_eq!(
            marks[0],
            palette
                .color(if state.disabled {
                    "disabled-text"
                } else {
                    "c-radio-checked-mark"
                })
                .unwrap()
        );
        if !state.disabled {
            assert!(
                contrast(marks[0], outer.fill) >= 3.,
                "dot/own indicator ground contrast"
            );
        }
    } else {
        assert!(marks.is_empty(), "unselected radio has no selection dot");
    }
    // Labels inherit actual painted parent ground, or the real renderer clear.
    let parent = scene.primitives()[..index]
        .iter()
        .rev()
        .find_map(|p| match p {
            Primitive::Quad(q) if q.fill.a == 1. && q.rect.intersection(&label) == Some(label) => {
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
        "actual CoreText radio label glyphs after this indicator"
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
    view: &mut RadioGroupView,
    model: &RadioGroup,
    text: &mut coretext::Text,
    theme: Theme,
) -> (Scene, RadioGroupLayout) {
    let palette: Palette = theme.into();
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 420., 240.),
        palette.color("surface-1").unwrap(),
    ));
    let layout = view.render(
        &mut scene,
        text,
        model,
        Rect::new(20., 20., 340., 0.),
        Some(Rect::new(0., 0., 420., 240.)),
        palette,
        &["Personal", "Shared", "Managed"],
    );
    (scene, layout)
}
#[test]
fn radio_group_same_instance_repaints_theme_and_preserves_selection_availability_and_roving_focus()
{
    let mut model = RadioGroup::new(3);
    model.set_enabled(1, false);
    let mut view = RadioGroupView::new();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    view.focus(&model, Some(0), true);
    let mut original = None;
    for theme in Theme::ALL {
        let palette: Palette = theme.into();
        let (scene, layout) = draw(&mut view, &model, &mut text, theme);
        assert_eq!(model.selected, Some(0));
        assert_eq!(view.focused(), Some(0));
        assert!(!model.is_enabled(1));
        if let Some(before) = &original {
            assert_eq!(&layout, before);
        } else {
            original = Some(layout.clone());
        }
        for i in 0..3 {
            assert_radio(
                &scene,
                (layout.indicators[i], layout.labels[i]),
                palette,
                ControlState {
                    selected: i == 0,
                    disabled: i == 1,
                    ..Default::default()
                },
                if i == 0 {
                    "c-radio-checked-ground"
                } else if i == 1 {
                    "surface-2"
                } else {
                    "surface-0"
                },
                text.backing,
                palette.color("surface-1").unwrap(),
            );
        }
        assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
            && q.rect.center()==layout.indicators[0].center() && q.border_color==palette.color("border-focused").unwrap())));
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    let clear = palette.color("surface-1").unwrap();
    let (_, layout) = draw(&mut view, &model, &mut text, theme);
    let unselected = layout.rows[2].frame().center();
    view.pointer_move(unselected);
    let (scene, layout) = draw(&mut view, &model, &mut text, theme);
    assert_radio(
        &scene,
        (layout.indicators[2], layout.labels[2]),
        palette,
        ControlState::default(),
        "button-hover",
        text.backing,
        clear,
    );
    assert_eq!(
        view.pointer_down(unselected, &model),
        SelectionEvent::Handled
    );
    let (scene, layout) = draw(&mut view, &model, &mut text, theme);
    assert_radio(
        &scene,
        (layout.indicators[2], layout.labels[2]),
        palette,
        ControlState::default(),
        "button-pressed",
        text.backing,
        clear,
    );
    assert_eq!(
        view.pointer_up(Point::new(400., 220.), &mut model),
        SelectionEvent::Handled
    );
    assert_eq!(model.selected, Some(0));
    let point = layout.rows[0].frame().center();
    view.pointer_move(point);
    let (scene, layout) = draw(&mut view, &model, &mut text, theme);
    assert_radio(
        &scene,
        (layout.indicators[0], layout.labels[0]),
        palette,
        ControlState {
            selected: true,
            ..Default::default()
        },
        "control-checked-hover",
        text.backing,
        clear,
    );
    assert_eq!(view.pointer_down(point, &model), SelectionEvent::Handled);
    let (scene, layout) = draw(&mut view, &model, &mut text, theme);
    assert_radio(
        &scene,
        (layout.indicators[0], layout.labels[0]),
        palette,
        ControlState {
            selected: true,
            ..Default::default()
        },
        "control-checked-pressed",
        text.backing,
        clear,
    );
    assert_eq!(
        view.pointer_up(Point::new(400., 220.), &mut model),
        SelectionEvent::Handled
    );
    assert_eq!(model.selected, Some(0));
    view.pointer_move(Point::new(400., 220.));
    view.focus(&model, Some(0), true);
    assert_eq!(
        view.key(ControlKey::Right, true, &mut model),
        SelectionEvent::Changed
    );
    assert_eq!(model.selected, Some(2));
    assert_eq!(view.focused(), Some(2));
    let (scene, layout) = draw(&mut view, &model, &mut text, theme);
    assert_radio(
        &scene,
        (layout.indicators[2], layout.labels[2]),
        palette,
        ControlState {
            selected: true,
            ..Default::default()
        },
        "c-radio-checked-ground",
        text.backing,
        clear,
    );
    let disabled_point = layout.rows[1].frame().center();
    assert_eq!(
        view.pointer_down(disabled_point, &model),
        SelectionEvent::Handled
    );
    assert_eq!(
        view.pointer_up(disabled_point, &mut model),
        SelectionEvent::Ignored
    );
    assert_eq!(model.selected, Some(2));
    view.set_disabled(true);
    let (scene, layout) = draw(&mut view, &model, &mut text, theme);
    for i in 0..3 {
        assert_radio(
            &scene,
            (layout.indicators[i], layout.labels[i]),
            palette,
            ControlState {
                selected: i == 2,
                disabled: true,
                ..Default::default()
            },
            "surface-2",
            text.backing,
            clear,
        );
    }
    assert_eq!(view.focused(), None);
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.))
    );
    assert_eq!(
        view.key(ControlKey::Left, true, &mut model),
        SelectionEvent::Ignored
    );
    assert_eq!(model.selected, Some(2));
    view.set_disabled(false);
    draw(&mut view, &model, &mut text, theme);
    let point = view.layout().rows[0].frame().center();
    assert_eq!(view.pointer_down(point, &model), SelectionEvent::Handled);
    assert_eq!(view.pointer_up(point, &mut model), SelectionEvent::Changed);
    assert_eq!(model.selected, Some(0));
}
