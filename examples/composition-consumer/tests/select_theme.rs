#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ButtonLayout, ControlKey, MenuEvent, Select, layout_view::SelectView};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
const LABELS: [&str; 3] = ["Automatic 自动", "Compact 紧凑", "Comfortable 舒适"];
fn paint(
    view: &mut SelectView,
    text: &mut coretext::Text,
    model: &mut Select,
    theme: Theme,
) -> Scene {
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        text,
        model,
        &LABELS,
        Rect::new(24., 24., 240., 44.),
        Rect::new(0., 0., 600., 500.),
        None,
        theme.into(),
        proposal::TextRole::UiBody,
        false,
    );
    assert!(scene.composition_error().is_none());
    scene
}
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn label(scene: &Scene, layout: ButtonLayout, fill: Rgba, ink: Rgba, theme: Theme, enabled: bool) {
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == layout.frame => Some((i, q)),
            _ => None,
        })
        .expect("own actual Select trigger/option ground");
    assert_eq!(ground.fill, fill);
    let labels: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(r)
                if !r.glyphs.is_empty()
                    && r.glyphs.iter().all(|g| layout.content.contains(g.origin)) =>
            {
                Some(r)
            }
            _ => None,
        })
        .collect();
    assert!(
        !labels.is_empty(),
        "actual CoreText selected-title/option text"
    );
    assert!(labels.iter().all(|r| r.color == ink));
    if enabled {
        assert_eq!((labels[0].color.a, ground.fill.a), (1., 1.));
        assert!(
            contrast(labels[0].color, ground.fill) >= if theme.is_increased() { 7. } else { 4.5 }
        );
    }
}
#[test]
fn retained_select_themes_preserve_trigger_popup_value_and_real_input() {
    let mut model = Select::new(3);
    assert!(model.choose(1));
    let mut view = SelectView::new(3);
    view.set_reduced_motion(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut original_trigger = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for state in ["rest", "hover", "pressed", "disabled", "rest"] {
            view.cancel(&mut model);
            view.set_disabled(state == "disabled", &mut model);
            paint(&mut view, &mut text, &mut model, theme);
            view.pointer_move(Point::new(400., 400.), &mut model);
            if state != "disabled" {
                view.focus(true);
            }
            let point = view.layout().trigger.frame.center();
            if matches!(state, "hover" | "pressed") {
                view.pointer_move(point, &mut model);
            }
            if state == "pressed" {
                assert!(view.pointer_down(point, &mut model));
            }
            let scene = paint(&mut view, &mut text, &mut model, theme);
            let layout = view.layout().trigger;
            if let Some(original) = original_trigger {
                assert_eq!(layout, original);
            } else {
                original_trigger = Some(layout);
            }
            label(
                &scene,
                layout,
                palette
                    .color(match state {
                        "hover" => "action-primary-hover",
                        "pressed" => "action-primary-pressed",
                        "disabled" => "surface-2",
                        _ => "action-primary-ground",
                    })
                    .unwrap(),
                palette
                    .color(if state == "disabled" {
                        "tertiary-text"
                    } else {
                        "action-primary-foreground"
                    })
                    .unwrap(),
                theme,
                state != "disabled",
            );
            assert!(!model.menu.open);
            assert_eq!(model.selected, 1);
            if state == "rest" {
                assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.fill.a==0.&&q.border_color==palette.color("border-focused").unwrap())));
            }
            if state == "disabled" {
                assert!(!view.pointer_down(point, &mut model));
                assert_eq!(view.pointer_up(point, &mut model), MenuEvent::None);
                assert_eq!(
                    view.key(ControlKey::Enter, true, &mut model),
                    MenuEvent::None
                );
                assert!(!model.menu.open);
            }
        }
    }
    view.cancel(&mut model);
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    assert!(model.menu.open);
    let mut original_popup = None;
    // The same popup stays logically open across Theme changes; value and active row differ.
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        paint(&mut view, &mut text, &mut model, theme);
        let popup = view.layout().popup.clone().unwrap();
        view.pointer_move(popup.popup.origin, &mut model);
        view.key(ControlKey::Home, true, &mut model);
        for hovered in [false, true] {
            if hovered {
                view.pointer_move(popup.items[2].frame.center(), &mut model);
            }
            let scene = paint(&mut view, &mut text, &mut model, theme);
            let layout = view.layout();
            let popup = layout.popup.as_ref().unwrap();
            let active = if hovered { 2 } else { 0 };
            assert!(model.menu.open);
            assert_eq!(model.selected, 1, "highlight is not commit");
            assert_eq!(model.menu.items.selected, active);
            assert_eq!(popup.active_item, active);
            let geometry = (
                popup.popup,
                popup.items.iter().map(|l| l.frame).collect::<Vec<_>>(),
            );
            if let Some(original) = &original_popup {
                assert_eq!(&geometry, original);
            } else {
                original_popup = Some(geometry);
            }
            let surface = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == popup.popup => Some(q),
                    _ => None,
                })
                .unwrap();
            assert_eq!(surface.fill, palette.color("surface-popover").unwrap());
            label(
                &scene,
                layout.trigger,
                palette.color("action-primary-ground").unwrap(),
                palette.color("action-primary-foreground").unwrap(),
                theme,
                true,
            );
            label(
                &scene,
                popup.items[active],
                palette
                    .color(if hovered {
                        "row-selection-hover"
                    } else {
                        "row-selection"
                    })
                    .unwrap(),
                palette.color("primary-text").unwrap(),
                theme,
                true,
            );
            let ring = scene.primitives().iter().find_map(|p| match p {
                Primitive::Quad(q)
                    if q.fill.a == 0.
                        && q.rect.center() == popup.items[active].frame.center()
                        && q.border_color == palette.color("border-focused").unwrap() =>
                {
                    Some(q)
                }
                _ => None,
            });
            assert_eq!(ring.is_some(), !hovered);
            if let Some(ring) = ring {
                assert!(
                    contrast(ring.border_color, surface.fill) >= 3.,
                    "necessary keyboard focus indicator"
                );
            }
        }
    }
    assert!(matches!(
        view.key(ControlKey::Enter, true, &mut model),
        MenuEvent::Activated { index: 2, .. }
    ));
    assert_eq!(model.selected, 2);
    assert!(!model.menu.open);
    paint(&mut view, &mut text, &mut model, Theme::DARK);
    assert!(!view.is_presented());
    let point = view.layout().trigger.frame.center();
    assert_eq!(view.pointer_up(point, &mut model), MenuEvent::None);
    assert!(!model.menu.open, "release without press does not open");
    assert!(view.pointer_down(point, &mut model));
    view.pointer_up(point, &mut model);
    assert!(model.menu.open);
    paint(&mut view, &mut text, &mut model, Theme::DARK);
    assert!(
        view.pointer_down(point, &mut model),
        "outside popup press dismisses"
    );
    view.pointer_up(point, &mut model);
    assert!(!model.menu.open, "dismiss release cannot reopen trigger");
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    paint(&mut view, &mut text, &mut model, Theme::DARK);
    view.key(ControlKey::Home, true, &mut model);
    assert!(matches!(
        view.key(ControlKey::Escape, true, &mut model),
        MenuEvent::Dismissed { .. }
    ));
    assert_eq!(
        model.selected, 2,
        "Escape preserves committed option identity"
    );
    view.set_disabled(true, &mut model);
    view.set_disabled(false, &mut model);
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    paint(&mut view, &mut text, &mut model, Theme::DARK);
    assert!(model.menu.open);
    view.set_window_active(false, &mut model);
    view.set_window_active(true, &mut model);
    paint(&mut view, &mut text, &mut model, Theme::DARK);
    assert!(!model.menu.open && !view.is_presented());
    assert_eq!(model.selected, 2);
}
