//! #72 real public API consumer: Scene and programmatic CoreText/AX evidence.
//! GPU, native window, physical IME and VoiceOver are separate acceptance.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::{
    progress_content::{self as fixture, BAR, RESULT, ROW},
    scroll_updates::{COMPOSER, SCROLLER},
};
use tekes_ui_controls::{
    ControlKey,
    composition::*,
    extended::{Progress, progress, progress_on_surface},
    feedback_view::ProgressView,
};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Theme,
    surface_palette::{resolve_surface, surface_names},
};
fn quads(s: &Scene) -> Vec<Quad> {
    s.primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q) => Some(*q),
            _ => None,
        })
        .collect()
}
fn paint(h: &mut ContentHost, t: &mut coretext::Text, p: Palette) -> Scene {
    t.services = true;
    t.atlas.begin_frame();
    let mut s = Scene::new();
    let f = Rect::new(0., 0., 360., 400.);
    h.render(&mut s, t, f, f, p).unwrap();
    s
}
#[test]
fn original_model_and_legacy_signature_preserve_values_and_static_unknown_geometry() {
    let mut m = Progress::new(f32::NAN);
    assert_eq!(m.value(), 0.);
    assert!(!m.set(f32::INFINITY));
    assert!(m.set(4.));
    assert_eq!(m.value(), 1.);
    assert!(!m.set(2.));
    assert!(m.set(-3.));
    assert_eq!(m.value(), 0.);
    let frame = Rect::new(1.25, 2.5, 120., 12.);
    let p = Theme::LIGHT.into();
    for value in [0., 0.45, 1.] {
        for unknown in [false, true] {
            m.set(value);
            m.indeterminate = unknown;
            let mut s = Scene::new();
            progress(&mut s, frame, p, m);
            let mut direct = Scene::new();
            progress_on_surface(&mut direct, frame, p, m, "canvas").unwrap();
            assert_eq!(quads(&s), quads(&direct));
            assert_eq!(
                quads(&s)[1].rect.size.width,
                120. * if unknown { 0.3 } else { value }
            );
        }
    }
}
#[test]
fn view_and_ordinary_content_use_the_complete_packet_on_all_surfaces_modes_brands_and_allocations()
{
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                for backing in [1., 2.] {
                    for (w, h) in [(120., 12.), (0., 0.), (0.25, 0.5), (8., 12.)] {
                        let p = Palette::new(theme, brand).unwrap();
                        let f = Rect::new(12.25, 20.5, w, h);
                        let clip = Rect::new(0., 0., 300., 100.);
                        let model = Progress::new(0.45);
                        let mut host = ContentHost::new(
                            Content::new(BAR, ProgressContent::new("Progress", 0.45))
                                .with_surface(surface)
                                .unwrap(),
                        )
                        .unwrap();
                        let mut t = coretext::Text::new(1.);
                        t.backing = backing;
                        let mut actual = Scene::new();
                        host.render(&mut actual, &mut t, f, clip, p).unwrap();
                        let mut expected = Scene::new();
                        let mut view = ProgressView::default();
                        view.set_surface(surface).unwrap();
                        let layout = view.render(&mut expected, model, f, Some(clip), p);
                        assert_eq!(quads(&actual), quads(&expected));
                        assert_eq!(host.get(BAR).unwrap().frame(), f);
                        let ordinary = host
                            .get(BAR)
                            .unwrap()
                            .component::<ProgressContent>()
                            .unwrap()
                            .layout();
                        if w > 0. && h > 0. {
                            assert_eq!(ordinary, layout);
                        } else {
                            // Original ContentHost skips painting an empty leaf;
                            // the original standalone View still records its input frame.
                            assert!(ordinary.visible.is_none());
                            assert!(layout.visible.is_none());
                            assert!(quads(&actual).is_empty());
                        }
                        if w > 0. && h > 0. {
                            let packet =
                                resolve_surface(p, surface, "progress-track", "rest").unwrap();
                            let q = quads(&actual);
                            assert_eq!(q[0].fill, packet.background);
                            assert_eq!(q[1].fill, packet.marker.unwrap());
                            assert_eq!(q[1].rect.size.width, w * 0.45);
                            assert_eq!(q[2].border_color, packet.boundary.unwrap());
                            assert!(q[2].border_width > 0.);
                            assert_eq!(q[2].rect, f);
                            assert!(layout.hit_test(f.center()));
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn finite_parent_measurement_and_ancestor_clip_use_original_view() {
    let mut c = ProgressContent::new("P", 0.);
    let mut t = coretext::Text::new(1.);
    for (a, want) in [
        (Size::new(100., 23.), Size::new(100., 23.)),
        (Size::new(f32::INFINITY, f32::NAN), Size::default()),
        (Size::new(-2., -1.), Size::default()),
    ] {
        assert_eq!(c.measure(&mut t, a).size, want);
    }
    let f = Rect::new(0.25, 0.5, 120., 12.);
    let clip = Rect::new(10., 0., 60., 40.);
    let mut v = ProgressView::default();
    let mut s = Scene::new();
    let layout = v.render(&mut s, c.model, f, Some(clip), Theme::LIGHT.into());
    assert_eq!(layout.visible, f.intersection(&clip));
    assert!(!layout.hit_test(Point::new(1., 2.)));
    assert!(layout.hit_test(Point::new(20., 2.)));
    assert!(
        s.primitives()
            .iter()
            .any(|p| matches!(p,Primitive::PushClip(r) if Some(*r)==layout.visible))
    );
}
#[test]
fn surface_inheritance_override_clear_and_unknown_rejection_keep_mount_and_context() {
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let mount = s.host.semantic_mount(BAR);
    assert_eq!(s.host.inherited_surface(), "canvas");
    s.host.set_inherited_surface("sidebar").unwrap();
    s.host
        .get_mut(ROW)
        .unwrap()
        .set_surface(Some("panel"))
        .unwrap();
    s.host
        .get_mut(BAR)
        .unwrap()
        .set_surface(Some("code"))
        .unwrap();
    for expected in ["code", "panel", "sidebar"] {
        let p = Palette::new(Theme::DARK, 2).unwrap();
        let scene = paint(&mut s.host, &mut t, p);
        let f = s.host.get(BAR).unwrap().frame();
        let packet = resolve_surface(p, expected, "progress-track", "rest").unwrap();
        assert!(
            quads(&scene)
                .iter()
                .any(|q| q.rect == f && q.fill == packet.background)
        );
        assert_eq!(s.host.semantic_mount(BAR), mount);
        if expected == "code" {
            s.host.get_mut(BAR).unwrap().set_surface(None).unwrap();
        } else {
            s.host.get_mut(ROW).unwrap().set_surface(None).unwrap();
        }
    }
    assert!(s.host.set_inherited_surface("unknown").is_err());
    assert_eq!(s.host.inherited_surface(), "sidebar");
    assert!(
        s.host
            .get_mut(BAR)
            .unwrap()
            .set_surface(Some("unknown"))
            .is_err()
    );
    assert_eq!(s.host.get(BAR).unwrap().surface(), None);
    assert_eq!(s.host.semantic_mount(BAR), mount);
    let mut view = ProgressView::default();
    view.set_surface("panel").unwrap();
    assert!(view.set_surface("unknown").is_err());
    assert_eq!(view.surface(), "panel");
    let mut scene = Scene::new();
    assert!(
        progress_on_surface(
            &mut scene,
            Rect::new(0., 0., 50., 12.),
            Theme::LIGHT.into(),
            Progress::new(1.),
            "unknown"
        )
        .is_err()
    );
    assert!(scene.is_empty());
    assert!(ContentHost::new(Content::new(BAR, ProgressContent::new("  ", 0.))).is_err());
    assert!(
        Content::new(BAR, ProgressContent::new("P", 0.))
            .with_surface("unknown")
            .is_err()
    );
}
#[test]
fn visible_progress_reports_readonly_range_known_unknown_without_focus_or_actions() {
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let node = s
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == BAR)
        .unwrap();
    assert_eq!(node.semantics.role, ContentRole::ProgressIndicator);
    assert_eq!(node.semantics.name, "Transfer progress");
    assert_eq!(node.semantics.value.as_deref(), Some("45%"));
    assert!(node.semantics.description.is_some());
    assert!(node.visible.is_some());
    assert_eq!(s.host.value_range(BAR), Some(0.0..=100.0));
    assert!(!s.host.focus(Some(BAR), true));
    for action in [
        AccessibilityAction::Press,
        AccessibilityAction::Focus,
        AccessibilityAction::SetValue("0.5".into()),
        AccessibilityAction::Increment,
        AccessibilityAction::Decrement,
    ] {
        assert_eq!(
            s.host.accessibility_action(BAR, None, action),
            ContentResponse::default()
        );
    }
    let c = s
        .host
        .get_mut(BAR)
        .unwrap()
        .component_mut::<ProgressContent>()
        .unwrap();
    assert!(!c.focusable());
    assert!(!c.interactive());
    assert!(!c.requires_text_services());
    assert!(c.text_surface().is_none());
    for event in [
        ContentEvent::PointerDown(Point::new(1., 1.)),
        ContentEvent::PointerUp(Point::new(1., 1.)),
        ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        },
        ContentEvent::Scroll {
            position: Point::new(1., 1.),
            delta: Point::new(0., 2.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.,
        },
    ] {
        assert_eq!(c.event(&event), ContentResponse::default());
    }
    fixture::publish(&mut s.host, 0.45, true, "Loading");
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let node = s
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == BAR)
        .unwrap();
    assert_eq!(node.semantics.value.as_deref(), Some("进度未知"));
    assert_eq!(s.host.value_range(BAR), None);
    assert!(s.host.drain_action_records().is_empty());
}
#[test]
fn all_host_states_preserve_composer_marked_binding_focus_and_original_reading_anchor() {
    let (mut s, mut version) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    t.services = true;
    t.reduced_motion = true;
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    s.host
        .get_mut(SCROLLER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let pos = s.host.get(SCROLLER).unwrap().frame().center();
    assert!(
        s.host
            .event(&ContentEvent::Scroll {
                position: pos,
                delta: Point::new(0., -80.),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 1.
            })
            .consumed
    );
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let keys = s.item_keys();
    let anchor = *keys
        .iter()
        .find(|id| s.host.get(**id).unwrap().visible().is_some())
        .unwrap();
    let y = s.host.get(anchor).unwrap().frame().min_y();
    let anchor_mount = s.host.semantic_mount(anchor);
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    assert!(before.marked.is_some());
    let editor_mount = s.host.semantic_mount(COMPOSER);
    let bar_mount = s.host.semantic_mount(BAR);
    let binding = s.host.focused_text_binding();
    s.host.drain_action_records();
    // rest/loading/error/success/indeterminate remain distinct caller states.
    for (i, (value, unknown, result)) in [
        (0., false, "Rest"),
        (0.45, false, "Loading"),
        (0.45, false, "Error"),
        (1., false, "Success"),
        (0.45, true, "Loading, unknown"),
        (1., false, "Awaiting verification"),
    ]
    .into_iter()
    .enumerate()
    {
        let (v, ()) = s
            .host
            .update_scroll_reading_document(
                SCROLLER,
                &version,
                version.revision() + 1,
                keys.clone(),
                |h| {
                    fixture::publish(h, value, unknown, result);
                    h.set_inherited_surface(if i % 2 == 0 { "sidebar" } else { "canvas" })
                        .unwrap();
                },
            )
            .unwrap();
        version = v;
        let actual_scene = paint(
            &mut s.host,
            &mut t,
            Palette::new(
                if i % 2 == 0 {
                    Theme::DARK
                } else {
                    Theme::LIGHT
                },
                2,
            )
            .unwrap(),
        );
        let after = s.host.text_input_snapshot().unwrap();
        assert_eq!(after.binding, before.binding);
        assert_eq!(after.marked, before.marked);
        assert_eq!(after.selection, before.selection);
        assert_eq!(after.text, before.text);
        assert_eq!(after.caret, before.caret);
        assert_eq!(after.carets, before.carets);
        assert_eq!(s.host.focused_text_binding(), binding);
        assert_eq!(s.host.focused(), Some(COMPOSER));
        assert_eq!(s.host.semantic_mount(COMPOSER), editor_mount);
        assert_eq!(s.host.semantic_mount(BAR), bar_mount);
        assert_eq!(s.host.semantic_mount(anchor), anchor_mount);
        let bar_center = s.host.get(BAR).unwrap().frame().center();
        for event in [
            ContentEvent::PointerDown(bar_center),
            ContentEvent::PointerUp(bar_center),
            ContentEvent::Scroll {
                position: bar_center,
                delta: Point::new(0., 2.),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 10. + i as f64,
            },
        ] {
            assert_eq!(s.host.event(&event), ContentResponse::default());
        }
        assert_eq!(s.host.focused(), Some(COMPOSER));
        assert_eq!(s.host.focused_text_binding(), binding);
        let reading = s
            .host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .reading_result()
            .unwrap();
        assert_eq!(reading.version, version);
        assert_eq!(
            reading.anchor,
            ContentReadingAnchorUpdate::Preserved(anchor)
        );
        assert!((s.host.get(anchor).unwrap().frame().min_y() - y).abs() < 1.);
        assert_eq!(
            s.host
                .get(RESULT)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text,
            result
        );
        assert!(s.host.drain_action_records().is_empty());
        let mut expected = Scene::new();
        let mut view = ProgressView::default();
        view.set_surface(s.host.inherited_surface()).unwrap();
        let bar = s
            .host
            .get(BAR)
            .unwrap()
            .component::<ProgressContent>()
            .unwrap();
        let f = s.host.get(BAR).unwrap().frame();
        view.render(
            &mut expected,
            bar.model,
            f,
            Some(f),
            Palette::new(
                if i % 2 == 0 {
                    Theme::DARK
                } else {
                    Theme::LIGHT
                },
                2,
            )
            .unwrap(),
        );
        let q = quads(&expected);
        let actual = quads(&actual_scene);
        for expected_quad in &q {
            assert!(
                actual.contains(expected_quad),
                "host state {result} missed original Progress packet/geometry"
            );
        }
        assert_eq!(
            q[1].fill,
            resolve_surface(
                Palette::new(
                    if i % 2 == 0 {
                        Theme::DARK
                    } else {
                        Theme::LIGHT
                    },
                    2
                )
                .unwrap(),
                s.host.inherited_surface(),
                "progress-track",
                "rest"
            )
            .unwrap()
            .marker
            .unwrap()
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn platform_progress_projection_is_readonly_known_unknown_and_rejects_all_actions() {
    use tekes_ui_platform_macos::{
        AccessibilityAction as A, AccessibilityRange, AccessibilityRole,
        content_accessibility::{content_action, content_key, content_nodes},
    };
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    let mount = s.host.semantic_mount(BAR);
    for unknown in [false, true, false] {
        fixture::publish(&mut s.host, 0.45, unknown, "Loading");
        paint(&mut s.host, &mut t, Theme::LIGHT.into());
        let key = content_key(&s.host, BAR, None).unwrap();
        let n = content_nodes(&s.host)
            .into_iter()
            .find(|n| n.key == key)
            .unwrap();
        assert_eq!(s.host.semantic_mount(BAR), mount);
        assert_eq!(n.label, "Transfer progress");
        assert_eq!(
            n.role,
            AccessibilityRole::ProgressIndicator(
                (!unknown).then(|| AccessibilityRange::new(0., 100.).unwrap())
            )
        );
        assert_eq!(n.value.as_deref(), (!unknown).then_some("45"));
        assert!(!n.focused);
        assert!(n.read_only);
        for action in [
            A::Press,
            A::Focus,
            A::SetValue("0.5".into()),
            A::Increment,
            A::Decrement,
        ] {
            assert_eq!(
                content_action(&mut s.host, key, action),
                ContentResponse::default()
            );
        }
    }
}
#[test]
#[ignore = "explicit external Theme artifact acceptance; set TEKESUI_ISSUE72_THEME_IDS"]
fn external_theme_definition_preserves_palette_source_in_mounted_progress() {
    let ids = std::env::var("TEKESUI_ISSUE72_THEME_IDS").expect("two exact verified Theme IDs");
    let ids = ids.split(',').collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    let mount = s.host.semantic_mount(BAR);
    for id in ids {
        for theme in Theme::ALL {
            for surface in surface_names() {
                let p = Palette::for_theme_definition(theme, id)
                    .expect("Theme compiled from verified external artifact");
                assert_eq!(p.brand_index(), None);
                s.host.set_inherited_surface(surface).unwrap();
                let scene = paint(&mut s.host, &mut t, p);
                let packet = resolve_surface(p, surface, "progress-track", "rest").unwrap();
                let frame = s.host.get(BAR).unwrap().frame();
                let q = quads(&scene);
                assert!(
                    q.iter()
                        .any(|q| q.rect == frame && q.fill == packet.background)
                );
                assert!(q.iter().any(|q| q.rect.min_y() == frame.min_y()
                    && q.fill == packet.marker.unwrap()
                    && q.rect.size.width == frame.size.width * 0.45));
                assert!(q.iter().any(|q| q.rect == frame
                    && q.border_color == packet.boundary.unwrap()
                    && q.border_width > 0.));
                assert_eq!(s.host.semantic_mount(BAR), mount);
            }
        }
    }
}
