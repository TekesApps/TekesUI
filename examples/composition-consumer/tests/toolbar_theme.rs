#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{ToolbarAction, ToolbarControl, composition::*};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
const ROOT: ComponentId = ComponentId(29100);
fn draw(host: &mut ContentHost, text: &mut coretext::Text, theme: Theme, width: f32) -> Scene {
    let frame = Rect::new(24., 24., width, 360.);
    host.get_mut(ROOT)
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .set_viewport(frame);
    let mut scene = Scene::new();
    host.render(&mut scene, text, frame, frame, theme.into())
        .unwrap();
    assert!(scene.composition_error().is_none());
    scene
}
fn parts(host: &ContentHost) -> Vec<SemanticPart> {
    host.semantics()
        .into_iter()
        .find(|n| n.id == ROOT)
        .unwrap()
        .parts
}
#[test]
fn retained_toolbar_control_themes_keep_surface_packets_and_original_command_identity() {
    let mut bar = ToolbarControl::window();
    bar.set_actions(
        [(7, "Save", true), (42, "Pin", true), (99, "Locked", false)]
            .into_iter()
            .map(|(id, title, enabled)| ToolbarAction {
                id,
                title: title.into(),
                enabled,
            })
            .collect(),
    );
    bar.set_selected_ids(&[42]);
    bar.set_group_together(true);
    let mut host = ContentHost::new(Content::new(ROOT, bar)).unwrap();
    host.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut original_parts = None;
    let mut original_mount = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        draw(&mut host, &mut text, theme, 640.);
        let save = parts(&host)
            .into_iter()
            .find(|p| p.semantics.name == "Save")
            .unwrap();
        for state in ["rest", "hover", "pressed", "rest"] {
            host.event(&ContentEvent::Cancel);
            host.event(&ContentEvent::PointerMove(Point::new(650., 340.)));
            assert!(
                host.accessibility_action(ROOT, Some(save.index), AccessibilityAction::Focus)
                    .consumed
            );
            if state != "rest" {
                host.event(&ContentEvent::PointerMove(save.frame.center()));
            }
            if state == "pressed" {
                host.event(&ContentEvent::PointerDown(save.frame.center()));
            }
            let scene = draw(&mut host, &mut text, theme, 640.);
            let background = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q)
                        if q.rect.min_x() == 24.
                            && q.rect.min_y() == 24.
                            && q.rect.size.width == 640. =>
                    {
                        Some(q)
                    }
                    _ => None,
                })
                .expect("actual ToolbarControl row background");
            assert_eq!(background.fill, palette.color("surface-0").unwrap());
            let actions = parts(&host);
            let geometry: Vec<_> = actions.iter().map(|p| (p.index, p.frame)).collect();
            if let Some(original) = &original_parts {
                assert_eq!(&geometry, original);
            } else {
                original_parts = Some(geometry);
            }
            assert_eq!(actions.len(), 3);
            for action in actions {
                assert_eq!(action.semantics.role, ContentRole::Button);
                let endpoint = match action.semantics.name.as_str() {
                    "Save" => state,
                    "Pin" => "selected",
                    "Locked" => "disabled",
                    _ => panic!("original command only"),
                };
                assert_eq!(action.semantics.selected, action.semantics.name == "Pin");
                assert_eq!(action.semantics.disabled, action.semantics.name == "Locked");
                let packet = resolve_surface(palette, "toolbar", "action-quiet", endpoint).unwrap();
                let ground = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect == action.frame => Some(q),
                        _ => None,
                    })
                    .expect("original toolbar Button own ground");
                assert_eq!(ground.fill, packet.background);
                let labels: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(r)
                            if !r.glyphs.is_empty()
                                && r.glyphs.iter().all(|g| action.frame.contains(g.origin)) =>
                        {
                            Some(r)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!labels.is_empty(), "actual CoreText action title");
                assert!(labels.iter().all(|r| r.color == packet.text));
                if !action.semantics.disabled {
                    assert_eq!((labels[0].color.a, ground.fill.a), (1., 1.));
                    let luminance = |c: tekes_ui_tokens::Rgba| {
                        let c = c.to_linear();
                        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                    };
                    let (a, b) = (luminance(labels[0].color), luminance(ground.fill));
                    assert!(
                        (a.max(b) + 0.05) / (a.min(b) + 0.05)
                            >= if theme.is_increased() { 7. } else { 4.5 }
                    );
                }
            }
            if state == "rest" {
                assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.fill.a==0.&&q.border_color==palette.color("border-focused").unwrap())));
            }
            assert!(
                host.drain_action_records().is_empty(),
                "theme/focus/arming never invokes a command"
            );
        }
        host.event(&ContentEvent::Cancel);
        let actions = parts(&host);
        let locked = actions
            .iter()
            .find(|p| p.semantics.name == "Locked")
            .unwrap();
        assert!(
            !host
                .accessibility_action(ROOT, Some(locked.index), AccessibilityAction::Press)
                .activated
        );
        host.event(&ContentEvent::PointerDown(save.frame.center()));
        host.event(&ContentEvent::PointerUp(save.frame.center()));
        let records = host.drain_action_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action.target, ROOT);
        assert_eq!(
            records[0].payload,
            Some(ContentActionPayload::ToolbarCommand(7))
        );
        if let Some(mount) = original_mount {
            assert_eq!(records[0].mount, mount);
        } else {
            original_mount = Some(records[0].mount);
        }
        draw(&mut host, &mut text, theme, 70.);
        assert_eq!(
            host.get(ROOT)
                .unwrap()
                .component::<ToolbarControl>()
                .unwrap()
                .overflow_ids(),
            vec![7, 42, 99]
        );
        let more = parts(&host)
            .into_iter()
            .find(|p| p.semantics.role == ContentRole::MenuButton)
            .unwrap();
        assert!(
            host.accessibility_action(ROOT, Some(more.index), AccessibilityAction::Press)
                .consumed
        );
        draw(&mut host, &mut text, theme, 70.);
        let pin = parts(&host)
            .into_iter()
            .find(|p| p.semantics.name == "Pin")
            .unwrap();
        assert_eq!(pin.semantics.role, ContentRole::MenuItem);
        assert!(pin.semantics.selected);
        assert!(
            host.accessibility_action(ROOT, Some(pin.index), AccessibilityAction::Press)
                .activated
        );
        let records = host.drain_action_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].payload,
            Some(ContentActionPayload::ToolbarCommand(42))
        );
        assert_eq!(Some(records[0].mount), original_mount);
    }
}
