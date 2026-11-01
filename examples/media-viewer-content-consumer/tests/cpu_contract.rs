#![cfg(target_os = "macos")]
//! CPU-only public contract of real Loading/Missing ImageContent. This does not
//! stand in for Ready/lease/GPU acceptance and creates no Renderer.
use tekes_ui_controls::{
    ControlKey,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::Theme;
#[path = "support/coretext.rs"]
mod coretext;
const ROOT: ComponentId = ComponentId(20470);
const VIEW: ComponentId = ComponentId(20471);
const IMAGE: ComponentId = ComponentId(20472);
fn host() -> ContentHost {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack::new(Axis::Vertical),
            flexible_child: Some(VIEW),
            children: vec![Content::new(
                VIEW,
                MediaViewerContent::new(
                    IMAGE,
                    ImageContent::loading(
                        Size::new(200., 100.),
                        ImageSemantics::Alternative("Pending chart".into()),
                    )
                    .unwrap(),
                    "Pending viewer",
                )
                .unwrap(),
            )],
        },
    ))
    .unwrap()
}
fn viewer(host: &mut ContentHost) -> &mut MediaViewerContent {
    host.get_mut(VIEW).unwrap().component_mut().unwrap()
}
fn arrange(host: &mut ContentHost) -> Scene {
    let frame = Rect::new(0., 0., 100., 100.);
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut coretext::Text::new(1.),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    scene
}
#[test]
fn real_loading_and_missing_content_never_accept_transform_input_or_paint_image() {
    let mut host = host();
    let scene = arrange(&mut host);
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::ControlledTransformedImage { .. }))
    );
    let before = viewer(&mut host).transform();
    viewer(&mut host).drain_transform_changes();
    assert!(!viewer(&mut host).can_execute(MediaViewerCommand::ZoomIn));
    assert!(
        !viewer(&mut host)
            .execute(MediaViewerCommand::ZoomIn)
            .unwrap()
    );
    host.focus(Some(VIEW), true);
    host.event(&ContentEvent::PointerDown(Point::new(50., 50.)));
    host.event(&ContentEvent::PointerMove(Point::new(20., 20.)));
    host.event(&ContentEvent::Key {
        key: ControlKey::Right,
        down: true,
    });
    host.event(&ContentEvent::PointerUp(Point::new(20., 20.)));
    assert_eq!(viewer(&mut host).transform(), before);
    assert!(viewer(&mut host).drain_transform_changes().is_empty());
    let request = viewer(&mut host).begin_load().unwrap();
    assert!(
        viewer(&mut host)
            .complete(request, ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert_eq!(
        viewer(&mut host).image_status(),
        Some(ImageLoadStatus::Missing)
    );
    assert!(!viewer(&mut host).execute(MediaViewerCommand::Fit).unwrap());
    let scene = arrange(&mut host);
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::ControlledTransformedImage { .. }))
    );
    let semantics = host.semantics();
    assert_eq!(
        semantics
            .iter()
            .filter(|s| s.semantics.role == ContentRole::Image)
            .count(),
        1
    );
    assert!(semantics.iter().any(|s|s.semantics.role==ContentRole::Group && s.semantics.name=="Pending viewer"));
}
#[test]
fn real_public_completion_capability_rejects_cross_owner_replay_and_old_mount() {
    let mut first = host();
    let mut second = host();
    arrange(&mut first);
    arrange(&mut second);
    let a = viewer(&mut first).begin_load().unwrap();
    let b = viewer(&mut first).begin_load().unwrap();
    assert!(
        !viewer(&mut first)
            .complete(a, ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert!(
        !viewer(&mut second)
            .complete(b.clone(), ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert!(
        viewer(&mut first)
            .complete(b.clone(), ImageLoadOutcome::Failed("No source yet".into()))
            .unwrap()
    );
    assert_eq!(viewer(&mut first).failure_message(), Some("No source yet"));
    assert!(
        !viewer(&mut first)
            .complete(b, ImageLoadOutcome::Missing)
            .unwrap()
    );
    let old = viewer(&mut first).begin_load().unwrap();
    let detached = first.remove(VIEW).unwrap();
    first.insert(ROOT, detached).unwrap();
    assert!(
        !viewer(&mut first)
            .complete(old, ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert_eq!(
        viewer(&mut first).image_status(),
        Some(ImageLoadStatus::Loading)
    );
    assert_eq!(
        viewer(&mut second).image_status(),
        Some(ImageLoadStatus::Loading)
    );
}
