#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::scroll_updates::{COMPOSER, SCROLLER};
use tekes_ui_composition_consumer::status_badge_content::{self as fixture, BAR, ROW};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{StatusBadge, status_badge, status_badge_on_surface},
    feedback_view::StatusBadgeView,
};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Theme,
    surface_palette::{resolve_surface, surface_names},
};
const TONES: [StatusBadge; 4] = [
    StatusBadge::Info,
    StatusBadge::Running,
    StatusBadge::Succeeded,
    StatusBadge::Failed,
];
fn quads(s: &Scene) -> Vec<Quad> {
    s.primitives()
        .iter()
        .filter_map(|p| {
            if let Primitive::Quad(q) = p {
                Some(*q)
            } else {
                None
            }
        })
        .collect()
}
fn palettes(mode: Theme) -> Vec<Palette> {
    let mut palettes: Vec<_> = (0..3)
        .map(|brand| Palette::new(mode, brand).expect("registered Gallery brand"))
        .collect();
    palettes.extend(["blue", "blue-inputs"].into_iter().map(|id| {
        Palette::for_theme_definition(mode, id)
            .expect("required external Theme definition must exist")
    }));
    palettes
}
fn paint(h: &mut ContentHost, t: &mut coretext::Text, p: Palette) -> Scene {
    t.services = true;
    t.atlas.begin_frame();
    let mut s = Scene::new();
    let f = Rect::new(0., 0., 360., 400.);
    fixture::render_checked(h, &mut s, t, f, f, p).unwrap();
    s
}
#[test]
fn four_tones_complete_packets_original_geometry_and_label_measurement() {
    for mode in Theme::ALL {
        for p in palettes(mode) {
            for surface in surface_names() {
                for (tone, usage) in TONES.into_iter().zip([
                    "status-badge-info",
                    "status-badge-running",
                    "status-badge-succeeded",
                    "status-badge-failed",
                ]) {
                    let packet = resolve_surface(p, surface, usage, "rest").unwrap();
                    assert_eq!(packet.text, packet.icon);
                    assert_eq!(
                        (packet.boundary, packet.focus, packet.marker, packet.helper),
                        (None, None, None, None)
                    );
                    assert_eq!(packet.text, StatusBadgeView::label_color(p));
                    for invalid in ["hover", "pressed", "selected", "disabled", "focus"] {
                        assert!(resolve_surface(p, surface, usage, invalid).is_err());
                    }
                    for label in ["Info", "状态 · 未验证", "Long e\u{301} 👨‍👩‍👧‍👦 text that wraps"]
                    {
                        for (w, h) in [(128., 28.), (18.25, 48.5), (0., 0.), (0.25, 0.5)] {
                            let f = Rect::new(12.25, 16.5, w, h);
                            let clip = Rect::new(0., 0., 300., 150.);
                            let mut t = coretext::Text::new(1.5);
                            let measured =
                                StatusBadgeView::measure_label(&mut t, label, f.size).unwrap();
                            let original =
                                t.measure(label, tekes_ui_tokens::proposal::TextRole::UiCaption, w);
                            assert_eq!(
                                measured,
                                Size::new(
                                    original.width.min(w).max(0.),
                                    original.height.min(h).max(0.)
                                )
                            );
                            let mut expected = Scene::new();
                            let mut view = StatusBadgeView::default();
                            view.set_surface(surface).unwrap();
                            view.render_labelled(
                                &mut expected,
                                &mut t,
                                tone,
                                label,
                                f,
                                f,
                                Some(clip),
                                p,
                            )
                            .unwrap();
                            let mut host = ContentHost::new(
                                Content::new(BAR, StatusBadgeContent::new(tone, label).unwrap())
                                    .with_surface(surface)
                                    .unwrap(),
                            )
                            .unwrap();
                            let mut actual = Scene::new();
                            host.render(&mut actual, &mut t, f, clip, p).unwrap();
                            assert_eq!(quads(&expected), quads(&actual));
                            if w > 0. && h > 0. {
                                assert_eq!(
                                    host.get(BAR)
                                        .unwrap()
                                        .component::<StatusBadgeContent>()
                                        .unwrap()
                                        .layout(),
                                    view.layout()
                                );
                                assert_eq!(quads(&actual)[0].fill, packet.background);
                                assert_eq!(quads(&actual)[0].corner_radii, tekes_ui_scene::CornerRadii::uniform(h * 0.5));
                            }
                        }
                    }
                }
            }
        }
    }
    let f = Rect::new(0., 0., 128., 28.);
    for tone in TONES {
        let mut old = Scene::new();
        let mut new = Scene::new();
        status_badge(&mut old, f, Theme::LIGHT, tone);
        status_badge_on_surface(&mut new, f, Theme::LIGHT, tone, "canvas").unwrap();
        assert_eq!(quads(&old), quads(&new));
    }
}
#[test]
fn invalid_inputs_reject_before_retained_state_scene_mount_or_editor_changes() {
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    paint(&mut s.host, &mut t, Theme::LIGHT.into());
    s.host.focus(Some(COMPOSER), true);
    let before = s.host.text_input_snapshot().unwrap();
    let mount = s.host.semantic_mount(BAR);
    let frame = s.host.get(BAR).unwrap().frame();
    let layout = s
        .host
        .get(BAR)
        .unwrap()
        .component::<StatusBadgeContent>()
        .unwrap()
        .layout();
    for label in ["", "  "] {
        assert!(
            fixture::publish(
                &mut s.host,
                StatusBadge::Failed,
                label,
                None,
                Some("bad".into()),
                false
            )
            .is_err()
        );
    }
    assert!(
        fixture::publish(
            &mut s.host,
            StatusBadge::Failed,
            "Failed",
            Some("  ".into()),
            None,
            false
        )
        .is_err()
    );
    assert_eq!(
        s.host
            .get(BAR)
            .unwrap()
            .component::<StatusBadgeContent>()
            .unwrap()
            .model(),
        StatusBadge::Info
    );
    assert_eq!(
        s.host
            .get(BAR)
            .unwrap()
            .component::<StatusBadgeContent>()
            .unwrap()
            .label(),
        "Awaiting verification"
    );
    for f in [
        Rect::new(0., 0., f32::NAN, 28.),
        Rect::new(0., 0., 120., f32::INFINITY),
        Rect::new(0., 0., -1., 28.),
        Rect::new(f32::MAX, 0., f32::MAX, 28.),
    ] {
        let mut scene = Scene::new();
        assert!(
            fixture::render_checked(
                &mut s.host,
                &mut scene,
                &mut t,
                f,
                Rect::new(0., 0., 300., 300.),
                Theme::LIGHT.into()
            )
            .is_err()
        );
        assert!(scene.is_empty());
        let mut v = StatusBadgeView::default();
        v.render_labelled(
            &mut scene,
            &mut t,
            StatusBadge::Info,
            "Info",
            frame,
            frame,
            None,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let old = v.layout();
        let len = scene.primitives().len();
        assert!(
            v.render_labelled(
                &mut scene,
                &mut t,
                StatusBadge::Failed,
                "Error",
                f,
                f,
                None,
                Theme::LIGHT.into()
            )
            .is_err()
        );
        assert_eq!(scene.primitives().len(), len);
        assert_eq!(v.layout(), old);
        if !f.size.width.is_finite()
            || !f.size.height.is_finite()
            || f.size.width < 0.
            || f.size.height < 0.
        {
            assert!(StatusBadgeView::measure_label(&mut t, "I", f.size).is_err());
        }
        assert_eq!(s.host.semantic_mount(BAR), mount);
        assert_eq!(s.host.get(BAR).unwrap().frame(), frame);
        assert_eq!(
            s.host
                .get(BAR)
                .unwrap()
                .component::<StatusBadgeContent>()
                .unwrap()
                .layout(),
            layout
        );
        assert_eq!(s.host.text_input_snapshot().unwrap(), before);
    }
    let mut v = StatusBadgeView::default();
    v.set_surface("panel").unwrap();
    assert!(v.set_surface("unknown").is_err());
    assert_eq!(v.surface(), "panel");
    let mut scene = Scene::new();
    assert!(
        status_badge_on_surface(
            &mut scene,
            frame,
            Theme::LIGHT,
            StatusBadge::Info,
            "unknown"
        )
        .is_err()
    );
    assert!(scene.is_empty());
    assert!(
        s.host
            .get_mut(BAR)
            .unwrap()
            .set_surface(Some("unknown"))
            .is_err()
    );
    assert_eq!(s.host.get(BAR).unwrap().surface(), None);
}
#[test]
fn named_and_decorative_are_passive_text_without_action_focus_editor_or_disabled_visual() {
    let mut c = StatusBadgeContent::new(StatusBadge::Running, "Loading").unwrap();
    assert!(!c.focusable());
    assert!(!c.interactive());
    assert!(!c.requires_text_services());
    assert!(c.text_surface().is_none());
    assert!(!c.semantic_hidden());
    assert_eq!(c.semantics().role, ContentRole::Text);
    assert_eq!(c.semantics().name, "Loading");
    assert!(!c.semantics().disabled);
    c.update(
        StatusBadge::Failed,
        "错误",
        Some("Task failed".into()),
        Some("Try again later".into()),
        false,
    )
    .unwrap();
    assert_eq!(c.semantics().name, "Task failed");
    assert_eq!(
        c.semantics().description.as_deref(),
        Some("Try again later")
    );
    let mut host = ContentHost::new(Content::new(BAR, c)).unwrap();
    let mut t = coretext::Text::new(1.);
    paint(&mut host, &mut t, Theme::LIGHT.into());
    assert!(!host.focus(Some(BAR), true));
    assert!(host.focus_targets().is_empty());
    assert_eq!(host.value_range(BAR), None);
    for a in [
        AccessibilityAction::Press,
        AccessibilityAction::Focus,
        AccessibilityAction::SetValue("x".into()),
        AccessibilityAction::Increment,
        AccessibilityAction::Decrement,
    ] {
        assert_eq!(
            host.accessibility_action(BAR, None, a),
            ContentResponse::default()
        );
    }
    for event in [
        ContentEvent::PointerDown(Point::new(4., 4.)),
        ContentEvent::PointerUp(Point::new(4., 4.)),
        ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        },
        ContentEvent::Scroll {
            position: Point::new(4., 4.),
            delta: Point::new(0., 2.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.,
        },
    ] {
        assert_eq!(
            host.get_mut(BAR)
                .unwrap()
                .component_mut::<StatusBadgeContent>()
                .unwrap()
                .event(&event),
            ContentResponse::default()
        );
    }
    let mount = host.semantic_mount(BAR);
    host.get_mut(BAR)
        .unwrap()
        .component_mut::<StatusBadgeContent>()
        .unwrap()
        .update(StatusBadge::Info, "Info", None, None, true)
        .unwrap();
    paint(&mut host, &mut t, Theme::LIGHT.into());
    assert!(host.semantics().is_empty());
    assert_eq!(host.semantic_mount(BAR), mount);
    assert!(host.drain_action_records().is_empty());
    assert!(
        ContentHost::new(Content::new(
            BAR,
            StatusBadgeContent::decorative(StatusBadge::Info, "")
        ))
        .is_ok()
    );
}
#[test]
fn surface_inheritance_explicit_clear_and_solid_ink_ignore_decorative_overrides() {
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
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
    let mount = s.host.semantic_mount(BAR);
    for expected in ["code", "panel", "sidebar"] {
        let p = Theme::DARK.into();
        let actual = paint(&mut s.host, &mut t, p);
        let f = s.host.get(BAR).unwrap().frame();
        assert!(quads(&actual).iter().any(|q| {
            q.rect == f
                && q.fill
                    == resolve_surface(p, expected, "status-badge-info", "rest")
                        .unwrap()
                        .background
        }));
        assert_eq!(s.host.semantic_mount(BAR), mount);
        if expected == "code" {
            s.host.get_mut(BAR).unwrap().set_surface(None).unwrap();
        } else {
            s.host.get_mut(ROW).unwrap().set_surface(None).unwrap();
        }
    }
    // Direct original Component contract receives decorative ink as an action child.
    let mut c = StatusBadgeContent::new(StatusBadge::Info, "Info").unwrap();
    let mut theme = ContentTheme::from(Palette::from(Theme::LIGHT));
    theme.foreground = Some(tekes_ui_tokens::Rgba::new(1., 0., 0., 1.));
    // This main API exposes foreground only; historical icon_foreground coverage
    // is not claimed here. StatusBadge renders text and solid ink, not icons.
    let mut scene = Scene::new();
    let f = Rect::new(0., 0., 120., 28.);
    c.paint(&mut scene, &mut t, f, f, theme);
    for p in scene.primitives() {
        if let Primitive::Glyphs(run) = p {
            assert_eq!(run.color, StatusBadgeView::label_color(theme.palette));
        }
    }
}
#[test]
fn dynamic_badge_updates_preserve_composer_marked_selection_caret_send_gate_and_original_reading_anchor()
 {
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
    for (i, (tone, result)) in [
        (StatusBadge::Info, "Rest"),
        (StatusBadge::Running, "Loading"),
        (StatusBadge::Failed, "Error"),
        (StatusBadge::Succeeded, "Success"),
        (StatusBadge::Info, "Awaiting verification"),
        (StatusBadge::Info, "Cancelled"),
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
                    fixture::publish(
                        h,
                        tone,
                        result,
                        Some(format!("State: {result}")),
                        Some(format!("Description {i}")),
                        false,
                    )
                    .unwrap();
                    h.set_inherited_surface(if i % 2 == 0 { "sidebar" } else { "canvas" })
                        .unwrap();
                },
            )
            .unwrap();
        version = v;
        let _actual_scene = paint(
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
        assert_eq!(
            s.host
                .get(COMPOSER)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .submit_policy,
            TextSubmitPolicy::Enter
        );
        assert!(
            !s.host
                .event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true
                })
                .activated,
            "original marked-text send gate remains closed"
        );
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
        assert_eq!(
            s.host
                .get(COMPOSER)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .submit_policy,
            TextSubmitPolicy::Enter
        );
        assert!(
            !s.host
                .event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true
                })
                .activated,
            "original marked-text send gate remains closed"
        );
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
        assert!(s.host.drain_action_records().is_empty());
    }
}

#[test]
fn hidden_semantic_wrapper_preserves_visible_descendant_parentage() {
    #[derive(Debug)]
    struct HiddenWrapper(Vec<Content>);
    impl Component for HiddenWrapper {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(available)
        }
        fn semantic_hidden(&self) -> bool { true }
        fn children(&self) -> &[Content] { &self.0 }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> { Some(&mut self.0) }
    }
    let child = ComponentId(9002);
    let host = ContentHost::new(Content::new(
        ComponentId(9001),
        HiddenWrapper(vec![Content::new(child, TextContent::new("Visible status"))]),
    )).unwrap();
    let nodes = host.semantics();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, child);
    assert_eq!(nodes[0].parent, None);
}
