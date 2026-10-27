#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_controls::{ControlKey, TabsLayout, TabsView};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn render(tabs: &mut TabsView, text: &mut coretext::Text, theme: Theme) -> (Scene, TabsLayout) {
    let mut scene = Scene::new();
    let layout = tabs.render(
        &mut scene,
        text,
        &["General", "Security", "Advanced"],
        Rect::new(20., 30., 480., 40.),
        Some(Rect::new(0., 0., 540., 120.)),
        theme.into(),
        proposal::TextRole::UiBody,
        false,
    );
    (scene, layout)
}

fn assert_item(
    scene: &Scene,
    layout: &TabsLayout,
    index: usize,
    palette: Palette,
    ground: &str,
    ink: &str,
    enabled: bool,
) -> Quad {
    let item = &layout.items[index];
    let quad = scene
        .primitives()
        .iter()
        .find_map(|p| match p {
            Primitive::Quad(q) if q.rect == item.frame => Some(*q),
            _ => None,
        })
        .expect("actual tab Button background");
    assert_eq!(quad.fill, palette.color(ground).unwrap());
    assert_eq!(quad.fill.a, 1.);
    assert_eq!(
        quad.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(
            proposal::metric("radius", "c-button-md-radius").unwrap()
        )
    );
    let inks: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|positioned| {
                    let glyph = positioned.glyph;
                    let bounds = Rect::new(
                        positioned.origin.x + glyph.left as f32 / 2.,
                        positioned.origin.y + glyph.top as f32 / 2.,
                        glyph.width as f32 / 2.,
                        glyph.height as f32 / 2.,
                    );
                    item.content.intersection(&bounds).is_some()
                }) =>
            {
                Some(run.color)
            }
            _ => None,
        })
        .collect();
    assert!(!inks.is_empty(), "real CoreText glyphs for tab {index}");
    for actual in inks {
        assert_eq!(actual, palette.color(ink).unwrap());
        if enabled {
            let minimum = if palette.theme.is_increased() {
                7.
            } else {
                4.5
            };
            assert!(
                contrast(actual, quad.fill) >= minimum,
                "tab {index}/{:?}: {} < {minimum}",
                palette.theme,
                contrast(actual, quad.fill)
            );
        }
    }
    quad
}

#[test]
fn tabs_same_instance_repaints_selection_interaction_and_real_text_in_all_themes() {
    let mut tabs = TabsView::new(3);
    let mut text = coretext::Text::new(1.);
    text.services = true; // reduced motion gives deterministic semantic endpoints
    assert!(tabs.select(1));
    let (_, initial) = render(&mut tabs, &mut text, Theme::LIGHT);
    let target = initial.items[2].frame.center();
    let mut selected_fills = Vec::new();
    for theme in Theme::ALL {
        let palette: Palette = theme.into();
        tabs.pointer_move_at(tekes_ui_scene::Point::new(0., 0.));
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        assert_eq!(layout, initial, "theme changes preserve tab geometry");
        assert_eq!(tabs.selected(), 1, "theme changes preserve selection");
        let selected = assert_item(
            &scene,
            &layout,
            1,
            palette,
            "row-selection",
            "primary-text",
            true,
        );
        selected_fills.push(selected.fill);
        let rest = assert_item(
            &scene,
            &layout,
            2,
            palette,
            "surface-2",
            "primary-text",
            true,
        );
        assert_ne!(selected.fill, rest.fill);

        tabs.pointer_move_at(target);
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        let hover = assert_item(
            &scene,
            &layout,
            2,
            palette,
            "button-hover",
            "primary-text",
            true,
        );
        assert_ne!(hover.fill, rest.fill);
        assert_eq!(tabs.pointer_down_at(target), Some(2));
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        let pressed = assert_item(
            &scene,
            &layout,
            2,
            palette,
            "button-pressed",
            "primary-text",
            true,
        );
        assert_ne!(pressed.fill, hover.fill);
        assert_eq!(tabs.pointer_up_at(tekes_ui_scene::Point::new(0., 0.)), None);
        assert_eq!(tabs.selected(), 1, "release outside cancels selection");

        let selected_target = layout.items[1].frame.center();
        tabs.pointer_move_at(selected_target);
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        let selected_hover = assert_item(
            &scene,
            &layout,
            1,
            palette,
            "row-selection-hover",
            "primary-text",
            true,
        );
        assert_ne!(selected_hover.fill, selected.fill);
        assert_eq!(tabs.pointer_down_at(selected_target), Some(1));
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        // Quiet selected tabs retain their selected-hover ground while pressed.
        assert_item(
            &scene,
            &layout,
            1,
            palette,
            "row-selection-hover",
            "primary-text",
            true,
        );
        assert_eq!(tabs.pointer_up_at(selected_target), Some(1));
        tabs.pointer_move_at(tekes_ui_scene::Point::new(0., 0.));
        assert!(tabs.focus(1, true));
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        assert_item(
            &scene,
            &layout,
            1,
            palette,
            "row-selection",
            "primary-text",
            true,
        );
        assert!(scene.primitives().iter().any(|p| matches!(p,
            Primitive::Quad(q) if q.fill.a == 0. && q.border_width > 0.
                && q.border_color == palette.color("border-focused").unwrap())));
        tabs.set_disabled(true);
        let (scene, layout) = render(&mut tabs, &mut text, theme);
        for index in 0..3 {
            assert_item(
                &scene,
                &layout,
                index,
                palette,
                "surface-2",
                "tertiary-text",
                false,
            );
        }
        assert!(!scene.primitives().iter().any(|p| matches!(p,
            Primitive::Quad(q) if q.fill.a == 0. && q.border_width > 0.)));
        assert_eq!(tabs.pointer_down_at(target), None);
        assert_eq!(tabs.key(ControlKey::Right, true), None);
        assert_eq!(tabs.selected(), 1);
        tabs.set_disabled(false);
        tabs.clear_focus();
    }
    assert_ne!(selected_fills[0], selected_fills[1]);
    assert_ne!(selected_fills[2], selected_fills[3]);
    assert!(tabs.focus(1, true));
    assert_eq!(tabs.key(ControlKey::Right, true), Some(2));
    assert_eq!(tabs.selected(), 2);
}
