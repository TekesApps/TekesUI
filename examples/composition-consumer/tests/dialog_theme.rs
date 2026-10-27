//! Bounded public Dialog Theme consumption; shared Button state coverage stays shared.
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_controls::{DialogEvent, DialogView};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};

#[test]
fn retained_public_dialog_repaints_theme_surface_copy_focus_and_disabled_actions() {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let preview = Rect::new(25., 35., 650., 500.);
    let mut view = DialogView::default();
    let mut paint = |view: &mut DialogView, theme| {
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            "Keep the workspace changes? 保留工作区更改",
            preview,
            Some(preview),
            Palette::from(theme),
            proposal::TextRole::UiBody,
            false,
        );
        assert!(scene.composition_error().is_none());
        (scene, layout)
    };
    assert!(paint(&mut view, Theme::LIGHT).0.primitives().is_empty());
    view.show(17);
    paint(&mut view, Theme::LIGHT);
    assert!(view.focus_button(1));
    let mut previous_ground = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for disabled in [false, true, false] {
            view.set_disabled(disabled);
            let (scene, layout) = paint(&mut view, theme);
            let layout = layout.expect("original open Dialog layout");
            assert!(layout.visible.is_some());
            assert!(view.is_open());
            assert_eq!(
                view.dialog().focus,
                1,
                "Theme and availability retain selection"
            );
            let (surface_index, surface) = scene
                .primitives()
                .iter()
                .enumerate()
                .find_map(|(index, p)| match p {
                    Primitive::Quad(q) if q.rect == layout.modal => Some((index, q)),
                    _ => None,
                })
                .expect("original modal surface");
            assert_eq!(surface.fill, palette.color("surface-popover").unwrap());
            assert_eq!(
                surface.corner_radii.top_left,
                proposal::metric("radius", "c-dialog-radius").unwrap()
            );
            let border_width = if theme.is_increased() {
                proposal::metric("stroke", "stroke-rule").unwrap()
            } else {
                0.5 // existing decorative hairline at the real fixture's 2x scale
            };
            let border_ink = palette.color("separator").unwrap();
            assert_eq!(
                (surface.border_width, surface.border_color),
                (border_width, border_ink)
            );
            assert!(scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.rect == preview && q.fill == palette.color("scrim").unwrap()
            )));
            // Inspect copy above the action row, independently of Cancel/Confirm ink.
            let copy = scene.primitives()[surface_index + 1..]
                .iter()
                .find_map(|p| match p {
                    Primitive::Glyphs(run)
                        if !run.glyphs.is_empty()
                            && run
                                .glyphs
                                .iter()
                                .all(|glyph| layout.title.contains(glyph.origin)) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .expect("actual CoreText Dialog copy glyphs");
            assert_eq!(copy.color, palette.color("primary-text").unwrap());
            assert_eq!((copy.color.a, surface.fill.a), (1., 1.));
            let luminance = |color: tekes_ui_tokens::Rgba| {
                let linear = color.to_linear();
                0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
            };
            let (a, b) = (luminance(copy.color), luminance(surface.fill));
            assert!(
                (a.max(b) + 0.05) / (a.min(b) + 0.05)
                    >= if theme.is_increased() { 7. } else { 4.5 }
            );
            let ring = scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.fill.a == 0. && q.border_color == palette.color("border-focused").unwrap()
                    && q.rect.intersection(&layout.buttons[1].frame).is_some()
            ));
            assert_eq!(
                ring, !disabled,
                "original retained Confirm focus decoration"
            );
            if disabled {
                assert_eq!(view.activate_button(1), DialogEvent::Blocked);
                assert!(!view.focus_button(0));
                assert!(view.is_open());
            }
        }
        let ground = palette.color("surface-popover").unwrap();
        if let Some(previous) = previous_ground
            && theme.is_dark()
        {
            assert_ne!(
                previous, ground,
                "appearance change repaints the retained modal"
            );
        }
        previous_ground = Some(ground);
    }
    assert_eq!(
        view.activate_button(0),
        DialogEvent::Cancelled {
            restore_focus: Some(17)
        }
    );
    let (scene, _) = paint(&mut view, Theme::DARK);
    assert!(!view.is_open());
    assert!(
        scene.primitives().is_empty(),
        "reduced-motion dismissal removes modal paint"
    );
    assert_eq!(view.activate_button(0), DialogEvent::Blocked);
}
