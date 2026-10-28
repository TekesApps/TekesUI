//! Original public IconContent, Button and resource capability; no private Gallery.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{
    Rgba, Theme, proposal,
    surface_palette::{resolve_surface, surface_names},
};
#[derive(Default)]
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
#[derive(Default)]
struct Icons {
    calls: Vec<(String, f32, f32, Rgba)>,
    fail: bool,
}
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("Content must use checked resources, not legacy void drawing")
    }
    fn prepare_icon(
        &mut self,
        n: &str,
        s: f32,
        b: f32,
        c: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        self.calls.push((n.into(), s, b, c));
        if self.fail {
            return Err(IconRenderError::Upload);
        }
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
}
fn tree() -> ContentHost {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), IconContent::new("tekes-settings").unwrap()),
                Content::new(
                    ComponentId(3),
                    Button::new(Content::new(
                        ComponentId(4),
                        IconContent::new("tekes-back").unwrap(),
                    ))
                    .with_accessible_name("Back"),
                ),
            ],
        },
    ))
    .unwrap();
    host.set_action_events(true);
    host
}
#[test]
fn canonical_icon_inherits_icon_channel_and_action_is_one_original_owner() {
    let frame = Rect::new(0., 0., 200., 200.);
    for mode in Theme::ALL {
        for surface in surface_names() {
            let p = mode.into();
            let mut host = tree();
            host.set_inherited_surface(surface).unwrap();
            let mut icons = Icons::default();
            let mut scene = Scene::new();
            host.render(
                &mut scene,
                &mut WithIconRenderer::new(&mut Text, &mut icons),
                frame,
                frame,
                p,
            )
            .unwrap();
            assert_eq!(icons.calls.len(), 2);
            assert_eq!(
                icons.calls[0].3,
                resolve_surface(p, surface, "content", "rest").unwrap().icon
            );
            assert_eq!(
                icons.calls[1].3,
                resolve_surface(p, surface, "action-quiet", "rest")
                    .unwrap()
                    .icon
            );
            assert_eq!(icons.calls[0].0, "tekes-settings");
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|v| matches!(v,Primitive::Image(i)if i.image==ImageId(99)))
            );
            let ax = host.semantics();
            assert!(
                !ax.iter()
                    .any(|n| n.id == ComponentId(2) || n.id == ComponentId(4))
            );
            assert_eq!(ax.iter().filter(|n| n.semantics.name == "Back").count(), 1);
            assert_eq!(host.focus_targets().len(), 1);
            let center = host.get(ComponentId(3)).unwrap().frame().center();
            assert!(
                host.event(&ContentEvent::PointerDown(center))
                    .capture_pointer
            );
            assert!(host.event(&ContentEvent::PointerUp(center)).activated);
            assert!(!host.event(&ContentEvent::PointerUp(center)).activated);
            let action = host.drain_action_records();
            assert_eq!(action.len(), 1);
            assert_eq!(action[0].action.target, ComponentId(3));
        }
    }
}
#[test]
fn missing_capability_and_resource_failure_are_explicit_and_same_owner_can_recover() {
    let mut host = tree();
    let frame = Rect::new(0., 0., 200., 200.);
    let mut scene = Scene::new();
    assert_eq!(
        host.render(&mut scene, &mut Text, frame, frame, Theme::LIGHT.into()),
        Err(CompositionError::MissingIconServices(ComponentId(2)))
    );
    struct Legacy;
    impl IconRenderer for Legacy {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("cannot silently fall back to unchecked drawing")
        }
    }
    assert_eq!(
        host.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut Text, &mut Legacy),
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::IconResource {
            owner: ComponentId(2),
            reason: IconRenderError::UnsupportedRenderer
        })
    );
    let mut icons = Icons {
        fail: true,
        ..Default::default()
    };
    assert_eq!(
        host.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut Text, &mut icons),
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::IconResource {
            owner: ComponentId(2),
            reason: IconRenderError::Upload
        })
    );
    let mount = host.semantic_mount(ComponentId(2));
    icons.fail = false;
    host.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut Text, &mut icons),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(host.semantic_mount(ComponentId(2)), mount);
    let icon = host
        .get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap();
    assert!(icon.set_glyph("missing-glyph").is_err());
    assert_eq!(icon.glyph().name, "tekes-settings");
    icon.set_visible(false);
    host.remove(ComponentId(3)).unwrap();
    host.render(&mut scene, &mut Text, frame, frame, Theme::LIGHT.into())
        .unwrap();
}
#[test]
fn named_passive_icon_has_full_name_but_no_focus_or_action() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        IconContent::new("tekes-settings")
            .unwrap()
            .with_accessible_name("Settings status"),
    ))
    .unwrap();
    let f = Rect::new(0., 0., 32., 32.);
    host.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(&mut Text, &mut Icons::default()),
        f,
        f,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(host.semantics()[0].semantics.name, "Settings status");
    assert!(host.focus_targets().is_empty());
    assert!(!host.event(&ContentEvent::PointerDown(f.center())).consumed);
}
#[cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[test]
fn failed_icon_and_background_glyph_update_preserve_original_composer_marked() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), TextFieldContent::new("Composer", "draft😀")),
                Content::new(ComponentId(3), IconContent::new("tekes-settings").unwrap()),
            ],
        },
    ))
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut icons = Icons::default();
    let f = Rect::new(0., 0., 300., 300.);
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut text, &mut icons),
        f,
        f,
        Theme::LIGHT.into(),
    )
    .unwrap();
    host.focus(Some(ComponentId(2)), true);
    host.event_with_text(
        &ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let capture = host.capture_focus();
    let binding = host.focused_text_binding();
    let before = {
        let s = &host
            .get(ComponentId(2))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        (s.text.clone(), s.selection(), s.marked.clone())
    };
    icons.fail = true;
    assert!(
        host.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut text, &mut icons),
            f,
            f,
            Theme::LIGHT.into()
        )
        .is_err()
    );
    host.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_glyph("tekes-back")
        .unwrap();
    icons.fail = false;
    host.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut text, &mut icons),
        f,
        f,
        Theme::DARK.into(),
    )
    .unwrap();
    let s = &host
        .get(ComponentId(2))
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    assert_eq!((s.text.clone(), s.selection(), s.marked.clone()), before);
    assert_eq!(host.capture_focus(), capture);
    assert_eq!(host.focused_text_binding(), binding);
    assert!(host.drain_actions().is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn independent_scene_run_stop_and_glyph_button_states_keep_original_composer_owner() {
    use tekes_ui_composition_consumer::icon_content::*;
    let mut h = build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = false;
    let mut icons = Icons::default();
    let f = Rect::new(0., 0., 700., 500.);
    let mut scene = Scene::new();
    h.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut text, &mut icons),
        f,
        f,
        Theme::LIGHT.into(),
    )
    .unwrap();
    h.focus(Some(COMPOSER), true);
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let binding = h.focused_text_binding();
    let mount = h.semantic_mount(BACK);
    let before = {
        let i = &h
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        (i.text.clone(), i.selection(), i.marked.clone())
    };
    for running in [true, false, true] {
        assert!(
            h.accessibility_action(RUN, None, AccessibilityAction::Press)
                .activated
        );
        let records = process_actions(&mut h);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action.target, RUN);
        assert_eq!(
            h.get(ICON)
                .unwrap()
                .component::<IconContent>()
                .unwrap()
                .is_indeterminate(),
            running
        );
        text.now += std::time::Duration::from_millis(100);
        scene.clear();
        h.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut text, &mut icons),
            f,
            f,
            Theme::DARK.into(),
        )
        .unwrap();
        let i = &h
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        assert_eq!((i.text.clone(), i.selection(), i.marked.clone()), before);
        assert_eq!(binding, h.focused_text_binding());
        assert_eq!(mount, h.semantic_mount(BACK));
    }
    for unavailable in ["disabled", "loading"] {
        let point = h.get(BACK).unwrap().frame().center();
        assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
        let b = h.get_mut(BACK).unwrap().component_mut::<Button>().unwrap();
        b.button.state.selected = true;
        if unavailable == "disabled" {
            b.button.state.disabled = true;
        } else {
            b.button.state.loading = true;
        }
        scene.clear();
        h.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut text, &mut icons),
            f,
            f,
            Theme::DARK.into(),
        )
        .unwrap();
        assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
        assert!(
            !h.accessibility_action(BACK, None, AccessibilityAction::Press)
                .activated
        );
        assert!(process_actions(&mut h).is_empty());
        let b = h.get_mut(BACK).unwrap().component_mut::<Button>().unwrap();
        b.button.state.disabled = false;
        b.button.state.loading = false;
    }
    assert!(
        h.accessibility_action(BACK, None, AccessibilityAction::Press)
            .activated
    );
    assert_eq!(process_actions(&mut h).len(), 1);
    assert!(process_actions(&mut h).is_empty());
    let i = &h
        .get(COMPOSER)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    assert_eq!((i.text.clone(), i.selection(), i.marked.clone()), before);
    assert_eq!(binding, h.focused_text_binding());
    assert_eq!(mount, h.semantic_mount(BACK));
}

#[cfg(target_os = "macos")]
#[test]
fn allocated_slot_does_not_stretch_resolved_icon_metrics() {
    for scale in [1., 1.5] {
        for width in [7., 80.] {
            let mut h = ContentHost::new(Content::new(
                ComponentId(1),
                IconContent::new("tekes-settings").unwrap(),
            ))
            .unwrap();
            let mut text = coretext::Text::new(scale);
            text.services = true;
            let mut icons = Icons::default();
            let frame = Rect::new(0., 0., width, 80.);
            h.render(
                &mut Scene::new(),
                &mut WithIconRenderer::new(&mut text, &mut icons),
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
            let resolved =
                (tekes_ui_tokens::proposal::metric("size", "size-icon-standard").unwrap() * scale)
                    .min(width);
            let layout = h
                .get(ComponentId(1))
                .unwrap()
                .component::<IconContent>()
                .unwrap()
                .layout();
            assert_eq!(
                layout.artboard.size,
                Size::new(resolved, resolved),
                "large parent allocation must preserve DS artwork size"
            );
            assert_eq!(
                icons.calls[0].1, resolved,
                "raster size and measured natural size agree"
            );
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn business_status_declarations_and_passive_semantic_updates_keep_original_marked() {
    use tekes_ui_composition_consumer::icon_content::*;
    let mut h = build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = false;
    let mut icons = Icons::default();
    let f = Rect::new(0., 0., 700., 600.);
    let mut scene = Scene::new();
    h.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut text, &mut icons),
        f,
        f,
        Theme::LIGHT.into(),
    )
    .unwrap();
    h.focus(Some(COMPOSER), true);
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: None,
        },
        &mut text,
    )
    .unwrap();
    let binding = h.focused_text_binding();
    let mount = h.semantic_mount(ICON);
    let before = {
        let i = &h
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        (i.text.clone(), i.selection(), i.marked.clone())
    };
    for (id, name) in [
        (TONE, "Info"),
        (TONE, "Running"),
        (UNAVAILABLE, "Unavailable"),
        (TONE, "Succeeded"),
        (UNAVAILABLE, "Available"),
        (TONE, "Failed"),
        (TONE, "Default"),
    ] {
        assert!(
            h.accessibility_action(id, None, AccessibilityAction::Press)
                .activated
        );
        let records = process_actions(&mut h);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action.target, id);
        assert!(process_actions(&mut h).is_empty());
        h.get_mut(ICON)
            .unwrap()
            .component_mut::<IconContent>()
            .unwrap()
            .set_accessible_name(Some(name.into()))
            .unwrap();
        scene.clear();
        text.now += std::time::Duration::from_millis(100);
        h.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut text, &mut icons),
            f,
            f,
            Theme::DARK.into(),
        )
        .unwrap();
        assert_eq!(mount, h.semantic_mount(ICON));
        assert_eq!(binding, h.focused_text_binding());
        let i = &h
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        assert_eq!((i.text.clone(), i.selection(), i.marked.clone()), before);
    }
}
