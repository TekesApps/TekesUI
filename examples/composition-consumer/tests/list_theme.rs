#![cfg(target_os = "macos")]
//! Retained selectable ListView Theme consumption; heterogeneous ListContent is separate.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey,
    collections_view::{CollectionEvent, ListView},
    extended::List,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

#[test]
fn retained_list_theme_preserves_selection_and_real_text_through_hover_focus_and_disabled() {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut model = List::new(3);
    assert!(model.select(1));
    let mut view = ListView::new();
    let labels = ["Workspace 工作区", "Documents 文档", "Archive 归档"].map(str::to_owned);
    let frame = Rect::new(24., 24., 320., 220.);
    let clip = Rect::new(0., 0., 400., 300.);
    let mut original_rows = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for (hovered, disabled) in [(false, false), (true, false), (false, true), (false, false)] {
            view.set_disabled(disabled);
            let mut scene = Scene::new();
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                &labels,
                frame,
                Some(clip),
                palette,
            );
            let selected = layout.items[1].frame;
            if !disabled {
                view.focus(1, true);
                view.pointer_move(if hovered {
                    selected.center()
                } else {
                    Point::new(frame.max_x() + 1., frame.max_y() + 1.)
                });
            }
            scene.clear();
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                &labels,
                frame,
                Some(clip),
                palette,
            );
            assert!(scene.composition_error().is_none());
            assert_eq!(
                model.selected,
                Some(1),
                "Theme, hover and availability do not select another row"
            );
            let rows: Vec<_> = layout.items.iter().map(|row| row.frame).collect();
            if let Some(original) = &original_rows {
                assert_eq!(&rows, original);
            } else {
                original_rows = Some(rows);
            }
            let (surface_index, surface) = scene
                .primitives()
                .iter()
                .enumerate()
                .find_map(|(index, p)| match p {
                    Primitive::Quad(q) if q.rect == selected => Some((index, q)),
                    _ => None,
                })
                .expect("original selected-row ground");
            let ground_role = if disabled {
                "surface-2"
            } else if hovered {
                "row-selection-hover"
            } else {
                "row-selection"
            };
            assert_eq!(surface.fill, palette.color(ground_role).unwrap());
            let copies: Vec<_> = scene.primitives()[surface_index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if !run.glyphs.is_empty()
                            && run.glyphs.iter().all(|g| selected.contains(g.origin)) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !copies.is_empty(),
                "real CoreText row labels survive Theme changes"
            );
            let ink = palette
                .color(if disabled {
                    "tertiary-text"
                } else {
                    "primary-text"
                })
                .unwrap();
            assert!(copies.iter().all(|run| run.color == ink));
            if !disabled {
                assert_eq!((copies[0].color.a, surface.fill.a), (1., 1.));
                let luminance = |color: tekes_ui_tokens::Rgba| {
                    let c = color.to_linear();
                    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                };
                let (a, b) = (luminance(copies[0].color), luminance(surface.fill));
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05)
                        >= if theme.is_increased() { 7. } else { 4.5 }
                );
            }
            let focus = scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.fill.a == 0. && q.border_color == palette.color("border-focused").unwrap()
                    && q.rect.intersection(&selected).is_some()
            ));
            assert_eq!(focus, !disabled);
            if disabled {
                assert_eq!(
                    view.key(ControlKey::Down, true, &mut model),
                    CollectionEvent::Ignored
                );
                assert_eq!(
                    view.pointer_down(selected.center(), &mut model),
                    CollectionEvent::Ignored
                );
                assert_eq!(model.selected, Some(1));
            }
        }
    }
    for (key, selected) in [
        (ControlKey::Down, 2),
        (ControlKey::Home, 0),
        (ControlKey::End, 2),
        (ControlKey::Up, 1),
    ] {
        assert_eq!(view.key(key, true, &mut model), CollectionEvent::Changed);
        assert_eq!(model.selected, Some(selected));
    }
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Activated(1)
    );
}
