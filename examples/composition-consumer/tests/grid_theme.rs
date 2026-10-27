//! #121: original measured GridContent retains children while forwarding Theme/Surface.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::{Button, ComponentId, Content, ContentHost, GridContent, TextContent},
    extended::Grid,
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
#[test]
fn grid_retained_themes_keep_measured_rows_clips_and_original_leaf_semantics() {
    const ROOT: ComponentId = ComponentId(121_020);
    const TITLE: ComponentId = ComponentId(121_021);
    const ACTION: ComponentId = ComponentId(121_022);
    const DETAIL: ComponentId = ComponentId(121_023);
    let root = Content::new(
        ROOT,
        GridContent {
            model: Grid {
                columns: 2,
                gap_x: 8.,
                gap_y: 8.,
            },
            children: vec![
                Content::new(TITLE, TextContent::new("Account")),
                Content::new(ACTION, Button::new("Open")),
                Content::new(DETAIL, TextContent::new("Preferences")),
            ],
        },
    )
    .with_surface("panel")
    .unwrap();
    let mut host = ContentHost::new(root).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let frame = Rect::new(20.25, 30.5, 400., 120.);
    let clip = Rect::new(25.25, 30.5, 380., 100.);
    let mut original = None;
    let mut original_mount = None;
    let mut observed = Vec::new();
    // Shared leaf state/contrast matrices are reused; this root only owns layout/propagation.
    for theme in Theme::ALL {
        let palette = Palette::new(theme, 0).unwrap();
        let packet = resolve_surface(palette, "panel", "action-quiet", "rest").unwrap();
        let ground = resolve_surface(palette, "panel", "content", "rest")
            .unwrap()
            .background;
        text.atlas.begin_frame();
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(frame, ground));
        host.render(&mut scene, &mut text, frame, clip, palette)
            .unwrap();
        let title = host.get(TITLE).unwrap();
        let action = host.get(ACTION).unwrap();
        let detail = host.get(DETAIL).unwrap();
        let geometry = [
            (title.frame(), title.visible()),
            (action.frame(), action.visible()),
            (detail.frame(), detail.visible()),
        ];
        if let Some(original) = original {
            assert_eq!(geometry, original);
        } else {
            original = Some(geometry);
        }
        for (child, visible) in geometry {
            assert_eq!(visible, child.intersection(&clip));
        }
        assert_eq!(title.frame().min_y(), action.frame().min_y());
        assert_eq!(
            title.frame().size.height,
            action.frame().size.height,
            "row uses the original maximum measured child height"
        );
        assert_eq!(title.frame().size.width, action.frame().size.width);
        assert_eq!(title.frame().min_x(), detail.frame().min_x());
        assert!(title.frame().max_x() <= action.frame().min_x());
        assert!(title.frame().max_y() <= detail.frame().min_y());
        let mount = host.semantic_mount(ACTION).unwrap();
        if let Some(original_mount) = original_mount {
            assert_eq!(mount, original_mount);
        } else {
            original_mount = Some(mount);
        }
        let button = action
            .component::<Button>()
            .unwrap()
            .button
            .layout()
            .unwrap();
        let fill = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Quad(q) if q.rect == button.frame => Some(q.fill),
                _ => None,
            })
            .expect("real Grid child Button background");
        assert_eq!(
            fill, packet.background,
            "Grid forwards declared panel Surface to the original Button"
        );
        for (child_frame, expected_ink) in [
            (title.frame(), palette.color("primary-text").unwrap()),
            (button.content, packet.text),
            (detail.frame(), palette.color("primary-text").unwrap()),
        ] {
            let inks: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| child_frame.contains(g.origin)) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(!inks.is_empty(), "actual CoreText child glyphs");
            assert!(
                inks.iter().all(|ink| *ink == expected_ink),
                "{theme:?}: frame={child_frame:?} actual={inks:?} expected={expected_ink:?}"
            );
        }
        assert_eq!(
            host.focus_targets()
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![ACTION],
            "Grid has no container focus/action target"
        );
        observed.push(fill);
    }
    assert!(
        observed.windows(2).any(|w| w[0] != w[1]),
        "same mounted Grid forwards changed Theme to child paint"
    );
}
