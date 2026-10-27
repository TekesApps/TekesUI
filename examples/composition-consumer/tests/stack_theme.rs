//! #121: retained measured StackContent forwards Theme/Surface to original children.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::{Button, ComponentId, Content, ContentHost, StackContent, TextContent},
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
#[test]
fn stack_retained_theme_changes_preserve_measured_children_and_forward_semantic_context() {
    const ROOT: ComponentId = ComponentId(121_010);
    const TEXT: ComponentId = ComponentId(121_011);
    const ACTION: ComponentId = ComponentId(121_012);
    let root = Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(TEXT, TextContent::new("Account")),
                Content::new(ACTION, Button::new("Open")),
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
    let mut observed = Vec::new();
    // Shared leaves retain their own state/contrast tests; only propagation is checked here.
    for axis in [Axis::Horizontal, Axis::Vertical] {
        host.get_mut(ROOT)
            .unwrap()
            .component_mut::<StackContent>()
            .unwrap()
            .model
            .axis = axis;
        let mut original = None;
        let mut original_mount = None;
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
            let label = host.get(TEXT).unwrap();
            let action = host.get(ACTION).unwrap();
            let geometry = (
                label.frame(),
                action.frame(),
                label.visible(),
                action.visible(),
            );
            if let Some(original) = original {
                assert_eq!(geometry, original);
            } else {
                original = Some(geometry);
            }
            assert_eq!(label.visible(), label.frame().intersection(&clip));
            assert_eq!(action.visible(), action.frame().intersection(&clip));
            match axis {
                Axis::Horizontal => assert!(label.frame().max_x() <= action.frame().min_x()),
                Axis::Vertical => assert!(label.frame().max_y() <= action.frame().min_y()),
            }
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
                .expect("original child Button background");
            assert_eq!(
                fill, packet.background,
                "Stack's declared panel Surface reaches its child"
            );
            for (child_frame, expected_ink) in [
                (label.frame(), palette.color("primary-text").unwrap()),
                (button.content, packet.text),
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
                assert!(!inks.is_empty(), "real CoreText child glyphs");
                assert!(
                    inks.iter().all(|ink| *ink == expected_ink),
                    "{theme:?} {axis:?}: child={child_frame:?} actual={inks:?} expected={expected_ink:?}"
                );
            }
            assert_eq!(
                host.focus_targets()
                    .iter()
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>(),
                vec![ACTION],
                "Stack introduces no focus target"
            );
            observed.push(palette.color("primary-text").unwrap());
        }
    }
    assert!(
        observed.windows(2).any(|w| w[0] != w[1]),
        "same Stack forwards changed Theme to real leaves"
    );
}
