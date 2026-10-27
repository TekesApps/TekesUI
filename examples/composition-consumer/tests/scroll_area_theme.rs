//! #121: original ScrollArea/ScrollContent preserves position while forwarding Theme.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ScrollArea,
    composition::{
        Button, ComponentId, Content, ContentHost, ScrollContent, StackContent, TextContent,
    },
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
#[test]
fn scroll_area_retained_themes_keep_offset_viewport_clip_and_real_leaf_context() {
    const SCROLLER: ComponentId = ComponentId(121_040);
    const ACTION: ComponentId = ComponentId(121_043);
    const HEADING: ComponentId = ComponentId(121_042);
    let body = Content::new(
        ComponentId(121_041),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(HEADING, TextContent::new("Account\nPreferences\nSecurity")),
                Content::new(ACTION, Button::new("Open detail")),
                Content::new(
                    ComponentId(121_044),
                    TextContent::new("Document row\n".repeat(20)),
                ),
            ],
        },
    );
    let root = Content::new(SCROLLER, ScrollContent::new(ScrollArea::default(), body))
        .with_surface("panel")
        .unwrap();
    let mut host = ContentHost::new(root).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let frame = Rect::new(20.25, 30.5, 360., 120.);
    let clip = Rect::new(20.25, 30.5, 350., 110.);
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(
        host.get_mut(SCROLLER)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .scroll_by(0., 36.)
    );
    let offset = host
        .get(SCROLLER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    let mount = host.semantic_mount(ACTION).unwrap();
    let mut original = None;
    let mut observed = Vec::new();
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
        let scroll = &host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll;
        assert_eq!(scroll.offset, offset);
        assert_eq!(scroll.overscroll, Point::default());
        assert!(offset.y > 0. && offset.y < scroll.limit().y);
        let viewport_clip = scroll
            .geometry(frame, Some(clip))
            .visible_viewport_rect()
            .unwrap();
        let heading = host.get(HEADING).unwrap();
        let action = host.get(ACTION).unwrap();
        let geometry = (
            heading.frame(),
            heading.visible(),
            action.frame(),
            action.visible(),
            scroll.limit(),
        );
        if let Some(original) = original {
            assert_eq!(geometry, original);
        } else {
            original = Some(geometry);
        }
        assert_eq!(
            heading.visible(),
            heading.frame().intersection(&viewport_clip)
        );
        assert_eq!(
            action.visible(),
            action.frame().intersection(&viewport_clip)
        );
        assert!(heading.visible().is_some() && action.visible().is_some());
        assert_eq!(host.semantic_mount(ACTION), Some(mount));
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::PushClip(r) if *r == viewport_clip)),
            "real viewport clips descendant paint"
        );
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
            .unwrap();
        assert_eq!(fill, packet.background);
        for (region, expected) in [
            (heading.frame(), palette.color("primary-text").unwrap()),
            (button.content, packet.text),
        ] {
            let inks: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| region.contains(g.origin)) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(!inks.is_empty(), "real scrolled Text/Button glyphs");
            assert!(
                inks.iter().all(|ink| *ink == expected),
                "{theme:?} {region:?}: {inks:?} expected={expected:?}"
            );
        }
        observed.push(fill);
    }
    assert!(
        observed.windows(2).any(|w| w[0] != w[1]),
        "same scroll owner forwards new Theme without a position reset"
    );
}
