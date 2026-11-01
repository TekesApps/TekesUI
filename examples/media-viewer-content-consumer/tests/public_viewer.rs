#![cfg(target_os = "macos")]
//! All behavior crosses the public ContentHost/ImageContent/Renderer boundary.
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme};
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;

const VIEW: ComponentId = ComponentId(20401);
const IMAGE: ComponentId = ComponentId(20402);
fn viewer(host: &mut ContentHost) -> &mut MediaViewerContent {
    host.get_mut(VIEW).unwrap().component_mut().unwrap()
}
fn loading() -> ContentHost {
    ContentHost::new(Content::new(
        VIEW,
        MediaViewerContent::new(
            IMAGE,
            ImageContent::loading(
                Size::new(200., 100.),
                ImageSemantics::Alternative("Four corner chart".into()),
            )
            .unwrap(),
            "Image viewer",
        )
        .unwrap(),
    ))
    .unwrap()
}
fn ready(renderer: &mut Renderer) -> ContentHost {
    let mut host = loading();
    let request = viewer(&mut host).begin_load().unwrap();
    let lease = renderer
        .register_bitmap_image(&[255, 0, 0, 255], 1, 1)
        .unwrap();
    assert!(
        viewer(&mut host)
            .complete(
                request,
                ImageLoadOutcome::Ready {
                    lease,
                    logical_size: Size::new(200., 100.),
                }
            )
            .unwrap()
    );
    host
}
fn scene(host: &mut ContentHost, frame: Rect, clip: Rect) -> Scene {
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut Text::new(1.),
        frame,
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
    scene
}
fn assert_transform(host: &mut ContentHost, q: f64, scale: f64, x: f32, y: f32) {
    let t = viewer(host).transform().expect("visible public geometry");
    assert!((t.relative_scale - q).abs() < 1e-9, "{t:?}");
    assert!((t.scale - scale).abs() < 1e-9, "{t:?}");
    assert!((t.translation.x - x).abs() < 0.001, "{t:?}");
    assert!((t.translation.y - y).abs() < 0.001, "{t:?}");
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn hand_calculated_anchor_resize_origin_and_empty_viewport() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = ready(&mut gpu);
    let frame = Rect::new(10., 20., 100., 100.);
    scene(&mut host, frame, frame);
    assert_transform(&mut host, 1., 0.5, 0., 25.);
    viewer(&mut host).drain_transform_changes();
    assert!(
        viewer(&mut host)
            .execute(MediaViewerCommand::ZoomAt {
                relative_scale: 2.,
                anchor: Point::new(40., 50.),
            })
            .unwrap()
    );
    // I=(200,100), old s=.5,T=(0,25): anchor contains P=(80,50).
    // s'=1 => T'=O-P=(-40,0); new center reads P=(90,50).
    assert_transform(&mut host, 2., 1., -40., 0.);
    assert_eq!(viewer(&mut host).drain_transform_changes().len(), 1);
    scene(&mut host, frame, frame);
    assert_eq!(
        host.get(IMAGE).unwrap().frame(),
        Rect::new(-30., 20., 200., 100.)
    );
    let wide = Rect::new(70., 90., 200., 100.);
    scene(&mut host, wide, wide);
    assert_transform(&mut host, 2., 2., -80., -50.);
    assert_eq!(
        host.get(IMAGE).unwrap().frame(),
        Rect::new(-10., 40., 400., 200.)
    );
    let empty = Rect::new(0., 0., 0., 0.);
    let suspended = scene(&mut host, empty, empty);
    assert!(
        !suspended
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::ControlledTransformedImage { .. }))
    );
    assert!(viewer(&mut host).transform().is_none());
    assert!(
        !viewer(&mut host)
            .execute(MediaViewerCommand::ZoomIn)
            .unwrap()
    );
    let small = Rect::new(5., 6., 20., 10.);
    scene(&mut host, small, small);
    assert_transform(&mut host, 2., 0.2, -8., -5.);
    // No clamp occurs in this vector: center P=(90,50) is preserved exactly.
    let t = viewer(&mut host).transform().unwrap();
    assert!(((10. - f64::from(t.translation.x)) / t.scale - 90.).abs() < 0.001);
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn replacement_tokens_failures_and_new_ready_keep_original_image_owner() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = ready(&mut gpu);
    let frame = Rect::new(0., 0., 100., 100.);
    scene(&mut host, frame, frame);
    viewer(&mut host)
        .execute(MediaViewerCommand::ZoomIn)
        .unwrap();
    let before = viewer(&mut host).transform();
    let a = viewer(&mut host).begin_load().unwrap();
    let b = viewer(&mut host).begin_load().unwrap();
    assert!(
        !viewer(&mut host)
            .complete(a, ImageLoadOutcome::Missing)
            .unwrap()
    );
    let mut other = loading();
    assert!(
        !viewer(&mut other)
            .complete(b.clone(), ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert!(
        !viewer(&mut host)
            .execute(MediaViewerCommand::ZoomIn)
            .unwrap()
    );
    assert_eq!(viewer(&mut host).transform(), before);
    assert!(
        viewer(&mut host)
            .complete(b, ImageLoadOutcome::Failed("replacement failed".into()))
            .unwrap()
    );
    assert_eq!(
        viewer(&mut host).image_status(),
        Some(ImageLoadStatus::Failed)
    );
    assert_eq!(viewer(&mut host).transform(), before);
    assert!(
        viewer(&mut host)
            .execute(MediaViewerCommand::ZoomIn)
            .unwrap()
    );
    let next = viewer(&mut host).begin_load().unwrap();
    let green = gpu.register_bitmap_image(&[0, 255, 0, 255], 1, 1).unwrap();
    assert!(
        viewer(&mut host)
            .complete(
                next,
                ImageLoadOutcome::Ready {
                    lease: green,
                    logical_size: Size::new(200., 100.),
                }
            )
            .unwrap()
    );
    assert_transform(&mut host, 1., 0.5, 0., 25.);
    assert_eq!(
        viewer(&mut host).transform().unwrap().mode,
        MediaViewerMode::Fit
    );
    let request = viewer(&mut host).begin_load().unwrap();
    assert!(
        viewer(&mut host)
            .complete(request, ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert!(
        !viewer(&mut host)
            .execute(MediaViewerCommand::ZoomIn)
            .unwrap()
    );
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn original_host_cancel_focus_and_resize_reject_old_drag() {
    // Observe through the immutable public API so reads retain current layout.
    let transform = |host: &ContentHost| {
        host.get(VIEW)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform()
    };
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = ready(&mut gpu);
    let frame = Rect::new(0., 0., 100., 100.);
    scene(&mut host, frame, frame);
    viewer(&mut host)
        .execute(MediaViewerCommand::ZoomAt {
            relative_scale: 4.,
            anchor: Point::new(50., 50.),
        })
        .unwrap();
    scene(&mut host, frame, frame);
    host.event(&ContentEvent::PointerDown(Point::new(50., 50.)));
    host.event(&ContentEvent::PointerMove(Point::new(40., 40.)));
    let displayed = transform(&host);
    host.event(&ContentEvent::Cancel);
    host.event(&ContentEvent::PointerMove(Point::new(20., 20.)));
    host.event(&ContentEvent::PointerUp(Point::new(20., 20.)));
    assert_eq!(
        transform(&host),
        displayed,
        "Cancel preserves last visible geometry"
    );
    host.focus(Some(VIEW), true);
    assert_eq!(host.focused(), Some(VIEW));
    let before = transform(&host).unwrap();
    viewer(&mut host).drain_transform_changes();
    scene(&mut host, frame, frame);
    host.event(&ContentEvent::Key {
        key: ControlKey::Right,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Right,
        down: false,
    });
    assert_eq!(
        transform(&host).unwrap().translation.x,
        before.translation.x - 32.
    );
    assert_eq!(viewer(&mut host).drain_transform_changes().len(), 1);
    scene(&mut host, frame, frame);
    host.event(&ContentEvent::PointerDown(Point::new(50., 50.)));
    let resized = Rect::new(0., 0., 120., 80.);
    scene(&mut host, resized, resized);
    let after_resize = transform(&host);
    host.event(&ContentEvent::PointerMove(Point::new(30., 30.)));
    host.event(&ContentEvent::PointerUp(Point::new(30., 30.)));
    assert_eq!(transform(&host), after_resize);
    viewer(&mut host).set_disabled(true);
    assert!(
        !viewer(&mut host)
            .execute(MediaViewerCommand::ZoomIn)
            .unwrap()
    );
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn real_image_subtree_clip_alt_and_shared_lease_survive_one_viewer_drop() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let lease = gpu.register_bitmap_image(&[255, 0, 0, 255], 1, 1).unwrap();
    let make = |name: &str| {
        ContentHost::new(Content::new(
            VIEW,
            MediaViewerContent::new(
                IMAGE,
                ImageContent::ready(
                    lease.clone(),
                    Size::new(200., 100.),
                    ImageSemantics::Alternative(name.into()),
                )
                .unwrap(),
                "Viewer",
            )
            .unwrap(),
        ))
        .unwrap()
    };
    let mut first = make("First chart");
    let mut second = make("Second chart");
    drop(lease);
    let frame = Rect::new(0., 0., 100., 100.);
    let clip = Rect::new(20., 30., 60., 40.);
    let old_scene = scene(&mut first, frame, clip);
    let semantics = first.semantics();
    assert_eq!(
        semantics
            .iter()
            .filter(|s| s.semantics.role == ContentRole::Image)
            .count(),
        1
    );
    assert!(semantics.iter().any(|s| s.semantics.name == "First chart"));
    assert_eq!(
        first.get(IMAGE).unwrap().frame(),
        Rect::new(0., 25., 100., 50.)
    );
    drop(first);
    let remaining = scene(&mut second, frame, clip);
    assert_eq!(gpu.image_count(), 1);
    let pixels = gpu
        .render_to_rgba8(
            &remaining,
            &mut GlyphAtlas::default(),
            (100, 100),
            1.,
            Rgba::gray(0.125),
        )
        .unwrap();
    let pixel = |x: usize, y: usize| &pixels[(y * 100 + x) * 4..(y * 100 + x + 1) * 4];
    assert!(pixel(50, 50)[0] > 200 && pixel(50, 50)[1] < 40);
    assert_eq!(
        pixel(19, 50),
        &[32, 32, 32, 255],
        "ancestor rectangular clip is preserved"
    );
    assert_eq!(pixel(50, 29), &[32, 32, 32, 255]);
    drop(second);
    drop(remaining);
    assert_eq!(
        gpu.image_count(),
        1,
        "retained old Scene still owns the shared lease"
    );
    drop(old_scene);
    gpu.render_to_rgba8(
        &Scene::new(),
        &mut GlyphAtlas::default(),
        (2, 2),
        1.,
        Rgba::gray(0.),
    )
    .unwrap();
    assert_eq!(gpu.image_count(), 0);
}

#[path = "support/viewer_fixture.rs"]
mod fixture;
#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn original_button_key_has_one_command_and_focus_stays_in_original_host() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = fixture::fixture(&mut gpu);
    let frame = Rect::new(0., 0., 320., 640.);
    scene(&mut host, frame, frame);
    let zoom = ComponentId(20412);
    assert!(host.focus_targets().iter().any(|(id, _)| *id == zoom));
    host.focus(Some(zoom), true);
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    let mut actions = fixture::Actions::default();
    viewer(&mut host).drain_transform_changes();
    assert_eq!(actions.dispatch(&mut host), (0, 0));
    assert_eq!(viewer(&mut host).transform().unwrap().relative_scale, 1.25);
    assert_eq!(viewer(&mut host).drain_transform_changes().len(), 1);
    assert_eq!(actions.dispatch(&mut host), (0, 0));
    assert!(viewer(&mut host).drain_transform_changes().is_empty());
    assert_eq!(host.focused(), Some(zoom));
    // Direct owner commands/draining via get_mut invalidate Host input geometry.
    // Acknowledge that update as the real host event loop does before new focus.
    scene(&mut host, frame, frame);
    assert!(host.focus(Some(fixture::CLOSE), true));
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    assert_eq!(actions.dispatch(&mut host), (1, 0));
    assert_eq!(actions.dispatch(&mut host), (0, 0));
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn close_reopen_rejects_old_request_and_restores_fit_without_consumer_state() {
    use tekes_ui_controls::extended::{Axis, Stack};
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let image = ImageContent::ready(
        gpu.register_bitmap_image(&[255, 0, 0, 255], 1, 1).unwrap(),
        Size::new(200., 100.),
        ImageSemantics::Alternative("Reopened chart".into()),
    )
    .unwrap();
    let root = ComponentId(20450);
    let mut host = ContentHost::new(Content::new(
        root,
        StackContent {
            model: Stack::new(Axis::Vertical),
            flexible_child: Some(VIEW),
            children: vec![Content::new(
                VIEW,
                MediaViewerContent::new(IMAGE, image, "Viewer").unwrap(),
            )],
        },
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 100., 100.);
    scene(&mut host, frame, frame);
    viewer(&mut host)
        .execute(MediaViewerCommand::ZoomIn)
        .unwrap();
    let old = viewer(&mut host).begin_load().unwrap();
    let closed = host.remove(VIEW).unwrap();
    assert!(!host.focus_targets().iter().any(|(id, _)| *id == VIEW));
    host.insert(root, closed).unwrap();
    assert!(
        !viewer(&mut host)
            .complete(old, ImageLoadOutcome::Missing)
            .unwrap()
    );
    scene(&mut host, frame, frame);
    assert_transform(&mut host, 1., 0.5, 0., 25.);
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn disabled_buttons_project_same_command_owner_and_never_activate() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = fixture::fixture(&mut gpu);
    let frame = Rect::new(0., 0., 320., 640.);
    scene(&mut host, frame, frame);
    fixture::refresh_buttons(&mut host);
    scene(&mut host, frame, frame);
    let out = ComponentId(20411);
    let semantics = host.semantics();
    assert!(
        semantics
            .iter()
            .find(|s| s.id == out)
            .unwrap()
            .semantics
            .disabled
    );
    let center = host.get(out).unwrap().frame().center();
    host.event(&ContentEvent::PointerDown(center));
    host.event(&ContentEvent::PointerUp(center));
    assert_eq!(
        host.get(out)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
    assert_eq!(fixture::Actions::default().dispatch(&mut host), (0, 0));
    assert_eq!(viewer(&mut host).transform().unwrap().relative_scale, 1.);
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn original_modal_close_and_escape_restore_invoker_and_cancel_drag() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = fixture::modal_fixture(&mut gpu);
    let frame = Rect::new(0., 0., 640., 800.);
    scene(&mut host, frame, frame);
    assert!(host.focus(Some(fixture::INVOKER), true));
    host.show_dialog(fixture::DIALOG).unwrap();
    scene(&mut host, frame, frame);
    assert!(
        host.focus_targets()
            .iter()
            .all(|(id, _)| *id != fixture::INVOKER)
    );
    host.focus(Some(fixture::CLOSE), true);
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    let mut actions = fixture::Actions::default();
    assert_eq!(actions.dispatch(&mut host), (1, 0));
    assert!(host.close_dialog());
    assert_eq!(host.focused(), Some(fixture::INVOKER));
    assert_eq!(actions.dispatch(&mut host), (0, 0));
    host.show_dialog(fixture::DIALOG).unwrap();
    scene(&mut host, frame, frame);
    viewer(&mut host)
        .execute(MediaViewerCommand::ZoomIn)
        .unwrap();
    scene(&mut host, frame, frame);
    let center = host.get(VIEW).unwrap().frame().center();
    host.event(&ContentEvent::PointerDown(center));
    let before = viewer(&mut host).transform();
    host.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert_eq!(host.focused(), Some(fixture::INVOKER));
    host.event(&ContentEvent::PointerMove(Point::new(
        center.x - 20.,
        center.y - 20.,
    )));
    host.event(&ContentEvent::PointerUp(center));
    assert_eq!(viewer(&mut host).transform(), before);
}

#[test]
#[ignore = "requires an allocated real Renderer/GPU window"]
fn original_tab_scope_and_window_inactivity_cancel_pointer_authority() {
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut host = fixture::fixture(&mut gpu);
    let frame = Rect::new(0., 0., 320., 640.);
    scene(&mut host, frame, frame);
    host.focus(Some(ComponentId(20412)), true);
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: false,
    });
    assert_eq!(
        host.focused(),
        Some(ComponentId(20413)),
        "original Host tab order reaches Fit"
    );
    viewer(&mut host)
        .execute(MediaViewerCommand::ZoomIn)
        .unwrap();
    scene(&mut host, frame, frame);
    let center = host.get(VIEW).unwrap().frame().center();
    host.event(&ContentEvent::PointerDown(center));
    host.event(&ContentEvent::PointerMove(Point::new(
        center.x - 10.,
        center.y,
    )));
    let displayed = viewer(&mut host).transform();
    host.set_window_active(false);
    host.set_window_active(true);
    host.event(&ContentEvent::PointerMove(Point::new(
        center.x - 30.,
        center.y,
    )));
    host.event(&ContentEvent::PointerUp(center));
    assert_eq!(viewer(&mut host).transform(), displayed);
    host.event(&ContentEvent::PointerDown(center));
    host.focus(Some(fixture::CLOSE), true);
    let unfocused = viewer(&mut host).transform();
    host.event(&ContentEvent::PointerMove(Point::new(
        center.x - 40.,
        center.y,
    )));
    host.event(&ContentEvent::PointerUp(center));
    assert_eq!(viewer(&mut host).transform(), unfocused);
}
