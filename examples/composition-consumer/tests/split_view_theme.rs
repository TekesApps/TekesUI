//! #121: original owned SplitViewContent, divider capture and pane Theme propagation.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::{
        Button, ComponentId, Content, ContentEvent, ContentHost, SplitViewConfig,
        SplitViewConstruction, SplitViewContent, SplitViewSlotIds, TextContent,
    },
    extended::Axis,
};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal, surface_palette::resolve_surface};
const ROOT: ComponentId = ComponentId(121_030);
fn split(host: &ContentHost) -> &SplitViewContent {
    host.get(ROOT)
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
}
#[test]
fn split_view_retained_themes_preserve_drag_capture_geometry_and_pane_context() {
    const DIVIDER: ComponentId = ComponentId(121_033);
    const LABEL: ComponentId = ComponentId(121_034);
    const ACTION: ComponentId = ComponentId(121_035);
    const BUDGET: usize = 64 * 1024 * 1024;
    let content = SplitViewContent::try_new(SplitViewConstruction {
        config: SplitViewConfig::default(),
        slots: SplitViewSlotIds {
            leading: ComponentId(121_031),
            trailing: ComponentId(121_032),
            divider: DIVIDER,
        },
        leading: Content::new(LABEL, TextContent::new("Master pane")),
        trailing: Content::new(ACTION, Button::new("Open detail")),
        budget: BUDGET,
    })
    .unwrap();
    let mut host = ContentHost::try_new_prepared(
        Content::new(ROOT, content).with_surface("panel").unwrap(),
        BUDGET,
    )
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let frame = Rect::new(16.25, 20.5, 440., 240.);
    let clip = Rect::new(20., 20., 430., 240.);
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let expected = host.split_view_expected(ROOT).unwrap();
        host.update_split_view(
            ROOT,
            expected,
            SplitViewConfig {
                axis,
                ..SplitViewConfig::default()
            },
            BUDGET,
        )
        .unwrap();
        host.render_checked(
            &mut Scene::new(),
            &mut text,
            frame,
            clip,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.focus(Some(DIVIDER), true));
        let initial = split(&host).layout().unwrap();
        let point = initial.divider.frame().center();
        let response = host.event(&ContentEvent::PointerDown(point));
        assert!(response.consumed && response.capture_pointer);
        // Move into the passive leading pane; the action leaf keeps its original rest state.
        let moved = match axis {
            Axis::Horizontal => Point::new(point.x - 24., point.y),
            Axis::Vertical => Point::new(point.x, point.y - 24.),
        };
        host.event(&ContentEvent::PointerMove(moved));
        let ratio = split(&host).config().unwrap().ratio;
        assert!(ratio < 0.5);
        let capture = host.pointer_capture().expect("real divider capture");
        assert_eq!(capture.0, DIVIDER);
        let focus = host.focused();
        let mount = host.semantic_mount(ACTION).unwrap();
        let mut original_layout = None;
        let mut observed = Vec::new();
        for theme in Theme::ALL {
            let palette = Palette::new(theme, 0).unwrap();
            let parent = resolve_surface(palette, "panel", "content", "rest")
                .unwrap()
                .background;
            let action_packet = resolve_surface(palette, "panel", "action-quiet", "rest").unwrap();
            text.atlas.begin_frame();
            let mut scene = Scene::new();
            scene.push_quad(Quad::new(frame, parent));
            host.render_checked(&mut scene, &mut text, frame, clip, palette)
                .unwrap();
            let layout = split(&host).layout().unwrap();
            if let Some(original) = original_layout {
                assert_eq!(layout, original);
            } else {
                original_layout = Some(layout);
            }
            assert_eq!(split(&host).config().unwrap().ratio, ratio);
            assert_eq!(host.pointer_capture(), Some(capture));
            assert_eq!(host.focused(), focus);
            assert_eq!(host.semantic_mount(ACTION), Some(mount));
            assert_eq!(layout.frame.visible_frame(), frame.intersection(&clip));
            let divider = layout.divider.frame();
            let width = if theme.is_increased() {
                proposal::metric("stroke", "stroke-rule").unwrap()
            } else {
                1. / text.backing
            };
            let ink_frame = match axis {
                Axis::Horizontal => Rect::new(
                    divider.center().x - width * 0.5,
                    divider.min_y(),
                    width,
                    divider.size.height,
                ),
                Axis::Vertical => Rect::new(
                    divider.min_x(),
                    divider.center().y - width * 0.5,
                    divider.size.width,
                    width,
                ),
            };
            let ink = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == ink_frame => Some(q.fill),
                    _ => None,
                })
                .expect("original at-scale divider ink");
            assert_eq!(ink, palette.color("separator").unwrap());
            assert_eq!(
                layout.divider_hit,
                divider.inset(-4.).intersection(&clip),
                "logical drag hit survives decorative width changes"
            );
            let label = host.get(LABEL).unwrap();
            let action = host
                .get(ACTION)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .layout()
                .unwrap();
            let fill = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == action.frame => Some(q.fill),
                    _ => None,
                })
                .unwrap();
            assert_eq!(fill, action_packet.background);
            for (region, expected) in [
                (label.frame(), palette.color("primary-text").unwrap()),
                (action.content, action_packet.text),
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
                assert!(!inks.is_empty(), "real owned-pane glyphs");
                assert!(
                    inks.iter().all(|ink| *ink == expected),
                    "{theme:?} {axis:?}: {inks:?} expected {expected:?}"
                );
            }
            observed.push(ink);
        }
        assert!(observed.windows(2).any(|w| w[0] != w[1]));
        host.event(&ContentEvent::Cancel);
        assert!(host.pointer_capture().is_none());
        assert_eq!(
            split(&host).config().unwrap().ratio,
            0.5,
            "original Cancel restores pre-Theme drag start"
        );
    }
}
