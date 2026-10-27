#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey,
    collections_view::{CollectionEvent, TreeView},
    extended::Tree,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

#[test]
fn retained_tree_theme_preserves_hierarchy_disabled_rows_and_original_navigation() {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut tree = Tree::default();
    let root = tree.add("Workspace 工作区", None).unwrap();
    let first = tree.add("Documents 文档", Some(root)).unwrap();
    let blocked = tree.add("Locked 锁定", Some(root)).unwrap();
    let last = tree.add("Archive 归档", Some(root)).unwrap();
    assert!(tree.set_disabled(blocked, true));
    assert!(tree.toggle(root));
    assert!(tree.select(root));
    assert!(!tree.select(blocked));
    let mut view = TreeView::new();
    let frame = Rect::new(24., 24., 360., 240.);
    let clip = Rect::new(0., 0., 440., 320.);
    let mut original_rows = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for (hovered, disabled) in [(false, false), (true, false), (false, true), (false, false)] {
            view.set_disabled(disabled);
            let mut scene = Scene::new();
            let layout = view.render(&mut scene, &mut text, &tree, frame, Some(clip), palette);
            let root_frame = layout.items[0].frame;
            if !disabled {
                view.focus(root, true);
                view.pointer_move(if hovered {
                    root_frame.center()
                } else {
                    Point::new(frame.max_x() + 1., frame.max_y() + 1.)
                });
            }
            scene.clear();
            let layout = view.render(&mut scene, &mut text, &tree, frame, Some(clip), palette);
            assert!(scene.composition_error().is_none());
            assert_eq!(tree.selected, Some(root));
            assert!(tree.nodes()[root].expanded);
            assert_eq!(
                layout
                    .items
                    .iter()
                    .map(|row| (row.id, row.depth))
                    .collect::<Vec<_>>(),
                vec![(root, 0), (first, 1), (blocked, 1), (last, 1)]
            );
            let rows: Vec<_> = layout.items.iter().map(|row| row.frame).collect();
            if let Some(original) = &original_rows {
                assert_eq!(&rows, original);
            } else {
                original_rows = Some(rows);
            }
            for id in [root, blocked] {
                let row = layout.items.iter().find(|row| row.id == id).unwrap().frame;
                let unavailable = disabled || id == blocked;
                let (index, ground) = scene
                    .primitives()
                    .iter()
                    .enumerate()
                    .find_map(|(index, p)| match p {
                        Primitive::Quad(q) if q.rect == row => Some((index, q)),
                        _ => None,
                    })
                    .expect("own real Tree row ground");
                let role = if unavailable {
                    "surface-2"
                } else if hovered {
                    "row-selection-hover"
                } else {
                    "row-selection"
                };
                assert_eq!(ground.fill, palette.color(role).unwrap());
                let labels: Vec<_> = scene.primitives()[index + 1..]
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if !run.glyphs.is_empty()
                                && run.glyphs.iter().all(|g| row.contains(g.origin)) =>
                        {
                            Some(run)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    !labels.is_empty(),
                    "real CoreText label for enabled/disabled Tree node"
                );
                assert!(labels.iter().all(|run| {
                    run.color
                        == palette
                            .color(if unavailable {
                                "tertiary-text"
                            } else {
                                "primary-text"
                            })
                            .unwrap()
                }));
                if !unavailable {
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
            let focused = scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.fill.a==0. && q.border_color==palette.color("border-focused").unwrap()
                    && q.rect.intersection(&root_frame).is_some()
            ));
            assert_eq!(focused, !disabled);
            if disabled {
                assert_eq!(
                    view.key(ControlKey::Right, true, &mut tree),
                    CollectionEvent::Ignored
                );
                assert_eq!(
                    view.pointer_down(root_frame.center(), &mut tree),
                    CollectionEvent::Ignored
                );
                assert_eq!(tree.selected, Some(root));
            }
        }
    }
    for (key, id) in [
        (ControlKey::Down, first),
        (ControlKey::Down, last),
        (ControlKey::Home, root),
        (ControlKey::Right, first),
        (ControlKey::Left, root),
    ] {
        assert_eq!(view.key(key, true, &mut tree), CollectionEvent::Changed);
        assert_eq!(tree.selected, Some(id));
    }
    assert_eq!(
        view.key(ControlKey::Left, true, &mut tree),
        CollectionEvent::Changed
    );
    let layout = view.render(
        &mut Scene::new(),
        &mut text,
        &tree,
        frame,
        Some(clip),
        Theme::LIGHT.into(),
    );
    assert_eq!(
        layout.items.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![root]
    );
    assert_eq!(
        view.key(ControlKey::Right, true, &mut tree),
        CollectionEvent::Changed
    );
    view.render(
        &mut Scene::new(),
        &mut text,
        &tree,
        frame,
        Some(clip),
        Theme::DARK.into(),
    );
    assert_eq!(
        view.key(ControlKey::End, true, &mut tree),
        CollectionEvent::Changed
    );
    assert_eq!(tree.selected, Some(last));
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut tree),
        CollectionEvent::Activated(last)
    );
}
