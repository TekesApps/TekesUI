//! #123: production BreadcrumbView, real CoreText glyphs and bounded Theme changes.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ControlKey, extended::Breadcrumb, layout_view::BreadcrumbView};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal, surface_palette::resolve_surface};

fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let v = c.to_linear();
        0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn breadcrumb_same_instance_theme_states_keep_navigation_and_readable_actual_glyphs() {
    let model = Breadcrumb {
        items: vec!["Home".into(), "Docs".into(), "Current".into()],
    };
    let labels = ["Home", "Docs", "Current"];
    let frame = Rect::new(16., 16., 620., 34.);
    let outside = Point::new(-100., -100.);
    let mut view = BreadcrumbView::default();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let mut observed = Vec::new();
    // Shared Button/Palette brand coverage is reused; this component varies its own states.
    let brand = 0;
    for theme in Theme::ALL {
        let palette = Palette::new(theme, brand).unwrap();
        view.set_disabled(false);
        view.blur();
        view.pointer_move(outside);
        let layout = view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &labels,
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        let point = layout.items[0].frame().center();
        let current = layout.items[2].frame().center();
        assert_eq!(view.pointer_down(current), None);
        assert_eq!(view.activate(2, &model), None);
        for state in [
            "rest",
            "hover",
            "pressed",
            "focus",
            "hover-focus",
            "pressed-focus",
            "disabled",
        ] {
            view.set_disabled(false);
            view.blur();
            view.pointer_move(outside);
            match state {
                "hover" => view.pointer_move(point),
                "pressed" => {
                    assert_eq!(view.pointer_down(point), Some(0));
                }
                "focus" => view.focus(0),
                "hover-focus" => {
                    view.pointer_move(point);
                    view.focus(0);
                }
                "pressed-focus" => {
                    assert_eq!(view.pointer_down(point), Some(0));
                    view.focus(0);
                }
                "disabled" => view.set_disabled(true),
                _ => {}
            }
            let mut scene = Scene::new();
            #[cfg(feature = "token-audit")]
            let (_, reads) = tekes_ui_tokens::audit::capture(|| {
                view.render(
                    &mut scene,
                    &mut text,
                    &model,
                    &labels,
                    frame,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                );
            });
            #[cfg(not(feature = "token-audit"))]
            view.render(
                &mut scene,
                &mut text,
                &model,
                &labels,
                frame,
                None,
                palette,
                proposal::TextRole::UiBody,
            );
            let fills: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q) if q.rect == layout.items[0].frame() => Some(q.fill),
                    _ => None,
                })
                .collect();
            let endpoint = match state {
                "hover" | "hover-focus" => "hover",
                "pressed" | "pressed-focus" => "pressed",
                "disabled" => "disabled",
                _ => "rest",
            };
            let colors = resolve_surface(palette, "canvas", "action-quiet", endpoint).unwrap();
            let current_colors =
                resolve_surface(palette, "canvas", "action-quiet", "selected-disabled").unwrap();
            let fill_role = match state {
                "hover" | "hover-focus" => "button-hover",
                "pressed" | "pressed-focus" => "button-pressed",
                _ => "document-canvas",
            };
            assert_eq!(
                fills.first(),
                Some(&colors.background),
                "{theme:?} {brand} {state}"
            );
            let active_ink = colors.text;
            let current_ink = current_colors.text;
            let threshold = if theme.is_increased() { 7. } else { 4.5 };
            assert!(
                contrast(active_ink, fills[0]) >= threshold,
                "ancestor {theme:?} {state}"
            );
            let current_fill = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == layout.items[2].frame() => Some(q.fill),
                    _ => None,
                })
                .expect("current-page production background");
            assert_eq!(current_fill, current_colors.background);
            assert!(
                contrast(current_ink, current_fill) >= threshold,
                "current {theme:?}"
            );
            let focused = matches!(state, "focus" | "hover-focus" | "pressed-focus");
            let ring_count = scene.primitives().iter().filter(|p| matches!(p, Primitive::Quad(q) if q.rect != layout.items[0].frame() && q.rect != layout.items[1].frame() && q.rect != layout.items[2].frame())).count();
            assert_eq!(
                ring_count > 0,
                focused,
                "{state} keyboard focus must paint separately"
            );
            if focused {
                assert!(scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.border_width > 0. && q.border_color == palette.color("border-focused").unwrap())), "actual focus border token");
            }
            for (index, expected) in [(0, active_ink), (2, current_ink)] {
                let actual: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if run.glyphs.iter().any(|g| {
                                layout.items[index].frame().contains(g.origin)
                                    && (index == 2
                                        || g.origin.x < layout.items[index].frame().max_x() - 22.)
                            }) =>
                        {
                            Some(run.color)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!actual.is_empty(), "item {index} has real CoreText glyphs");
                assert!(actual.iter().all(|ink| *ink == expected));
            }
            #[cfg(feature = "token-audit")]
            {
                assert!(reads.iter().any(|(_, id)| id == fill_role));
                assert!(reads.iter().any(|(_, id)| id == "tertiary-text"));
            }
            if state == "disabled" {
                assert_eq!(view.pointer_down(point), None);
                assert_eq!(view.activate(0, &model), None);
            } else if state.starts_with("pressed") {
                assert_eq!(view.pointer_up(point, &model), Some(0));
                assert_eq!(view.pointer_up(point, &model), None);
            } else if focused {
                assert_eq!(
                    view.key_event(ControlKey::Enter, true, &model),
                    (true, Some(0))
                );
                assert_eq!(
                    view.key_event(ControlKey::Space, true, &model),
                    (false, None)
                );
            }
            if state == "rest" {
                observed.push((theme, fills[0], active_ink));
            }
            assert_eq!(model.items, labels.map(str::to_owned));
        }
    }
    assert!(
        observed
            .windows(2)
            .any(|w| w[0].1 != w[1].1 && w[0].2 != w[1].2),
        "same retained View repaints Theme colors"
    );
}
