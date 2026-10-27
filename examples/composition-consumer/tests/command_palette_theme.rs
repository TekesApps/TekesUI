#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_composition_consumer::command_status::{COMMAND, Scenario};
use tekes_ui_controls::{
    ControlKey,
    composition::{AccessibilityAction, ContentEvent},
    extended::{ComboBox, CommandPalette},
    layout_view::CommandPaletteView,
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

fn assert_text_ground(
    scene: &Scene,
    frame: Rect,
    palette: Palette,
    ground: &str,
    ink: &str,
    active: bool,
) -> Quad {
    let (background_index, quad) = scene
        .primitives()
        .iter()
        .enumerate()
        .find_map(|(index, p)| match p {
            Primitive::Quad(q) if q.rect == frame => Some((index, *q)),
            _ => None,
        })
        .expect("actual component background quad");
    assert_eq!(quad.fill, palette.color(ground).unwrap());
    assert_eq!(quad.fill.a, 1.);
    let inks: Vec<_> = scene.primitives()[background_index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|g| {
                    let glyph = g.glyph;
                    frame
                        .intersection(&Rect::new(
                            g.origin.x + glyph.left as f32 / 2.,
                            g.origin.y + glyph.top as f32 / 2.,
                            glyph.width as f32 / 2.,
                            glyph.height as f32 / 2.,
                        ))
                        .is_some()
                }) =>
            {
                Some(run.color)
            }
            _ => None,
        })
        .collect();
    assert!(!inks.is_empty(), "actual CoreText glyphs in {frame:?}");
    for actual in inks {
        assert_eq!(actual, palette.color(ink).unwrap());
        if active {
            let luminance = |c: Rgba| {
                let c = c.to_linear();
                0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
            };
            let (a, b) = (luminance(actual), luminance(quad.fill));
            let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
            let minimum = if palette.theme.is_increased() {
                7.
            } else {
                4.5
            };
            assert!(
                ratio >= minimum,
                "{ground}/{:?}: {ratio} < {minimum}",
                palette.theme
            );
        }
    }
    quad
}

fn draw(s: &mut Scenario, text: &mut coretext::Text, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    s.render(
        &mut scene,
        text,
        Rect::new(20., 20., 400., 420.),
        Rect::new(0., 0., 480., 520.),
        theme.into(),
    )
    .unwrap();
    scene
}

#[test]
fn command_palette_same_instance_preserves_query_and_active_candidate_across_themes() {
    let mut scenario = Scenario::new(
        ["Open file", "Open settings", "Close window"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
    )
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true; // real reduced-motion context, semantic endpoints only
    draw(&mut scenario, &mut text, Theme::LIGHT);
    scenario.host.focus(Some(COMMAND), true);
    assert!(
        scenario
            .host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .changed
    );
    draw(&mut scenario, &mut text, Theme::LIGHT);
    assert!(
        scenario
            .host
            .event(&ContentEvent::Commit {
                text: "Open".into(),
                replacement: None
            })
            .consumed
    );
    draw(&mut scenario, &mut text, Theme::LIGHT);
    scenario.host.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    scenario
        .host
        .accessibility_action(COMMAND, None, AccessibilityAction::SetSelection(2..2));
    let initial_query = scenario.query_frame().unwrap();
    let mut selected_fills = Vec::new();
    for theme in Theme::ALL {
        let scene = draw(&mut scenario, &mut text, theme);
        assert!(scenario.open());
        assert_eq!(scenario.query(), "Open");
        assert_eq!(scenario.active(), Some(1));
        assert_eq!(scenario.last_executed(), None);
        assert_eq!(scenario.query_frame(), Some(initial_query));
        assert_eq!(
            scenario.host.focused_text().unwrap().input.selection(),
            2..2
        );
        let palette: Palette = theme.into();
        let field = assert_text_ground(
            &scene,
            initial_query,
            palette,
            "transcript-input",
            "primary-text",
            true,
        );
        assert_eq!(
            field.corner_radii,
            tekes_ui_scene::CornerRadii::uniform(
                proposal::metric("radius", "radius-input").unwrap()
            )
        );
        assert_eq!(
            field.border_color,
            palette
                .color(proposal::control_boundary("required-input", theme).unwrap())
                .unwrap()
        );
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Quad(q)
            if q.fill.a == 0. && q.border_width > 0. && q.rect.center() == initial_query.center()
            && q.border_color == palette.color("border-focused").unwrap()))
        );
        let node = scenario
            .host
            .semantics()
            .into_iter()
            .find(|n| n.id == COMMAND)
            .unwrap();
        assert_eq!(node.parts.len(), 2);
        assert_eq!(
            node.parts.iter().map(|p| p.index).collect::<Vec<_>>(),
            [0, 1]
        );
        for part in &node.parts {
            assert_eq!(part.focused, part.index == 1);
            let ground = if part.index == 1 {
                "row-selection"
            } else {
                "surface-2"
            };
            let q = assert_text_ground(&scene, part.frame, palette, ground, "primary-text", true);
            if part.index == 1 {
                selected_fills.push(q.fill);
            }
        }
    }
    assert_ne!(selected_fills[0], selected_fills[1]);
    assert_ne!(selected_fills[2], selected_fills[3]);
    assert!(
        scenario
            .host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
    );
    assert_eq!(scenario.last_executed(), Some("Open settings"));
    assert!(!scenario.open());
    assert!(scenario.host.focused_text().is_none());
    assert_eq!(scenario.host.focused(), Some(COMMAND));
    draw(&mut scenario, &mut text, Theme::DARK_INCREASED);
    scenario.host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    draw(&mut scenario, &mut text, Theme::DARK_INCREASED);
    scenario.host.event(&ContentEvent::Commit {
        text: "unmatched command".into(),
        replacement: None,
    });
    draw(&mut scenario, &mut text, Theme::DARK_INCREASED);
    assert!(scenario.open());
    assert_eq!(scenario.active(), None);
    assert!(
        !scenario
            .host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
    );
    assert_eq!(scenario.last_executed(), Some("Open settings"));
}

#[test]
fn command_palette_disabled_closes_its_popup_and_suppresses_trigger_focus_in_all_themes() {
    let mut view = CommandPaletteView::default();
    view.options.set_reduced_motion(true);
    let mut model = CommandPalette {
        combo: ComboBox {
            items: vec!["Open file".into()],
            ..Default::default()
        },
    };
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(20., 20., 240., 40.);
    let bounds = Rect::new(0., 0., 400., 400.);
    for theme in Theme::ALL {
        let palette: Palette = theme.into();
        model.combo.disabled = false;
        view.focus_trigger(true);
        let mut scene = Scene::new();
        let layout = view.render_trigger(
            &mut scene,
            &mut text,
            &mut model,
            "Find a command",
            frame,
            Some(bounds),
            palette,
            proposal::TextRole::UiBody,
        );
        assert_text_ground(
            &scene,
            layout.frame,
            palette,
            "action-primary-ground",
            "action-primary-foreground",
            true,
        );
        assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q)
            if q.fill.a == 0. && q.border_width > 0. && q.border_color == palette.color("border-focused").unwrap())));
        assert!(view.open_from(&mut model, 7));
        assert!(
            view.render(
                &mut Scene::new(),
                &mut text,
                &mut model,
                frame,
                bounds,
                Some(bounds),
                palette,
                proposal::TextRole::UiBody,
                false
            )
            .is_some()
        );
        model.combo.disabled = true;
        view.focus_trigger(true);
        scene.clear();
        let layout = view.render_trigger(
            &mut scene,
            &mut text,
            &mut model,
            "Find a command",
            frame,
            Some(bounds),
            palette,
            proposal::TextRole::UiBody,
        );
        assert!(!model.combo.open);
        assert_text_ground(
            &scene,
            layout.frame,
            palette,
            "surface-2",
            "tertiary-text",
            false,
        );
        assert!(
            !scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Quad(q)
            if q.fill.a == 0. && q.border_width > 0.))
        );
        assert!(
            !view
                .trigger_key(ControlKey::Enter, true, &mut model)
                .consumed
        );
        assert!(!view.pointer_down_trigger(frame.center()));
        assert!(
            view.render(
                &mut Scene::new(),
                &mut text,
                &mut model,
                frame,
                bounds,
                Some(bounds),
                palette,
                proposal::TextRole::UiBody,
                false
            )
            .is_none()
        );
    }
}
