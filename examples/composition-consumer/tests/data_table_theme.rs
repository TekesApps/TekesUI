#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey,
    collections_view::{CollectionEvent, DataTableView},
    extended::{DataTable, SortDirection},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

#[test]
fn retained_data_table_theme_preserves_sorted_identity_header_and_readonly_cell_ink() {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut model = DataTable::new(3, 2);
    assert!(model.select_row(1));
    assert!(model.sort_by(0));
    let headers = ["Name 名称", "Kind 类型"].map(str::to_owned);
    let rows = [
        vec!["Zulu 工作区".into(), "Folder".into()],
        vec!["Alpha 文档".into(), "Text".into()],
        vec!["Bravo 归档".into(), "Folder".into()],
    ];
    let mut order = [1, 2, 0];
    let mut view = DataTableView::new();
    let frame = Rect::new(24., 24., 480., 240.);
    let clip = Rect::new(0., 0., 560., 320.);
    let mut original_geometry = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for (hovered, header_focus, disabled) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (false, false, false),
        ] {
            view.set_disabled(disabled);
            let mut scene = Scene::new();
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                &headers,
                &rows,
                &order,
                frame,
                Some(clip),
                palette,
            );
            let selected = layout.rows.iter().find(|row| row.id == 1).unwrap().frame;
            if !disabled {
                if header_focus {
                    view.focus_header(0, true);
                } else {
                    view.focus_row(1, true);
                }
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
                &headers,
                &rows,
                &order,
                frame,
                Some(clip),
                palette,
            );
            assert!(scene.composition_error().is_none());
            assert_eq!(model.selected_row, Some(1));
            assert_eq!(model.sort, Some((0, SortDirection::Ascending)));
            assert_eq!(
                layout.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
                order.to_vec()
            );
            let geometry = (
                layout
                    .headers
                    .iter()
                    .map(|row| row.frame)
                    .collect::<Vec<_>>(),
                layout.rows.iter().map(|row| row.frame).collect::<Vec<_>>(),
            );
            if let Some(original) = &original_geometry {
                assert_eq!(&geometry, original);
            } else {
                original_geometry = Some(geometry);
            }
            let header = layout.headers[0].frame;
            for (bounds, is_header) in [(header, true), (selected, false)] {
                let (index, ground) = scene
                    .primitives()
                    .iter()
                    .enumerate()
                    .find_map(|(index, p)| match p {
                        Primitive::Quad(q) if q.rect == bounds => Some((index, q)),
                        _ => None,
                    })
                    .expect("own actual table header or row ground");
                let role = if is_header || disabled {
                    "surface-2"
                } else if hovered {
                    "row-selection-hover"
                } else {
                    "row-selection"
                };
                assert_eq!(ground.fill, palette.color(role).unwrap());
                let cells = if is_header {
                    vec![header]
                } else {
                    layout
                        .columns
                        .iter()
                        .map(|column| column.intersection(&selected).unwrap())
                        .collect()
                };
                for cell in cells {
                    let labels: Vec<_> = scene.primitives()[index + 1..]
                        .iter()
                        .filter_map(|p| match p {
                            Primitive::Glyphs(run)
                                if !run.glyphs.is_empty()
                                    && run.glyphs.iter().all(|g| cell.contains(g.origin)) =>
                            {
                                Some(run)
                            }
                            _ => None,
                        })
                        .collect();
                    assert!(!labels.is_empty(), "real CoreText header/cell glyphs");
                    let ink = palette
                        .color(if is_header && disabled {
                            "tertiary-text"
                        } else {
                            "primary-text"
                        })
                        .unwrap();
                    assert!(labels.iter().all(|run| run.color == ink));
                    // Readonly data remains readable even when row actions are disabled.
                    if !is_header || !disabled {
                        assert_eq!((labels[0].color.a, ground.fill.a), (1., 1.));
                        let luminance = |color: tekes_ui_tokens::Rgba| {
                            let c = color.to_linear();
                            0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                        };
                        let (a, b) = (luminance(labels[0].color), luminance(ground.fill));
                        assert!(
                            (a.max(b) + 0.05) / (a.min(b) + 0.05)
                                >= if theme.is_increased() { 7. } else { 4.5 }
                        );
                    }
                }
            }
            let focus_bounds = if header_focus { header } else { selected };
            let focus=scene.primitives().iter().any(|p|matches!(p,
                Primitive::Quad(q) if q.fill.a==0.&&q.border_color==palette.color("border-focused").unwrap()&&q.rect.intersection(&focus_bounds).is_some()
            ));
            assert_eq!(focus, !disabled);
            if disabled {
                assert_eq!(
                    view.key(ControlKey::Down, true, &mut model),
                    CollectionEvent::Ignored
                );
                assert_eq!(
                    view.pointer_down(header.center(), &mut model),
                    CollectionEvent::Ignored
                );
                assert_eq!(model.sort, Some((0, SortDirection::Ascending)));
                assert_eq!(model.selected_row, Some(1));
            }
        }
    }
    for (key, id) in [
        (ControlKey::Down, 2),
        (ControlKey::End, 0),
        (ControlKey::Home, 1),
    ] {
        assert_eq!(view.key(key, true, &mut model), CollectionEvent::Changed);
        assert_eq!(model.selected_row, Some(id));
    }
    view.focus_header(0, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(model.sort, Some((0, SortDirection::Descending)));
    assert_eq!(
        model.selected_row,
        Some(1),
        "sorting does not retarget original row identity"
    );
    // The caller owns sorted order; the original IDs, not display slots, enter the View.
    order.reverse();
    let layout = view.render(
        &mut Scene::new(),
        &mut text,
        &model,
        &headers,
        &rows,
        &order,
        frame,
        Some(clip),
        Theme::DARK.into(),
    );
    assert_eq!(
        layout.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![0, 2, 1]
    );
    view.focus_row(1, true);
    assert_eq!(
        view.key(ControlKey::Up, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(model.selected_row, Some(2));
    assert_eq!(
        view.key(ControlKey::Home, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(model.selected_row, Some(0));
    assert_eq!(
        view.key(ControlKey::End, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(model.selected_row, Some(1));
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Activated(1)
    );
}
