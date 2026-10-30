//! Actual public SVG resource preparation (no framebuffer readback/visual oracle),
//! compound passive title, and independent sibling-editor composition continuity.
#![cfg(all(target_os = "macos", feature = "svg-icons"))]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Disclosure, Stack},
    layout_view::DisclosureView,
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};
const DISCLOSURE: ComponentId = ComponentId(1);
const TRIGGER: ComponentId = ComponentId(2);
const TITLE: ComponentId = ComponentId(3);
const TITLE_TEXT: ComponentId = ComponentId(4);
const TITLE_ICON: ComponentId = ComponentId(5);
const BODY: ComponentId = ComponentId(20);
const SIBLING: ComponentId = ComponentId(50);
fn disclosure(h: &mut ContentHost) -> &mut DisclosureContent {
    h.get_mut(DISCLOSURE).unwrap().component_mut().unwrap()
}
fn fixture_order(sibling_first: bool) -> ContentHost {
    let title = Content::new(
        TITLE,
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: 4.,
            },
            flexible_child: None,
            children: vec![
                Content::new(TITLE_ICON, IconContent::new("tekes-settings").unwrap()),
                Content::new(TITLE_TEXT, TextContent::new("复合标题 中文")),
            ],
        },
    );
    let d = Content::new(
        DISCLOSURE,
        DisclosureContent::new(
            TRIGGER,
            title,
            Content::new(BODY, TextFieldContent::new("Body", "body retained")),
            true,
        ),
    );
    let sibling = Content::new(
        SIBLING,
        TextFieldContent::new("Independent editor", "sibling"),
    );
    let children = if sibling_first {
        vec![sibling, d]
    } else {
        vec![d, sibling]
    };
    let mut h = ContentHost::new(Content::new(
        ComponentId(100),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: Some(DISCLOSURE),
            children,
        },
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn fixture() -> ContentHost {
    fixture_order(true)
}
fn paint(
    h: &mut ContentHost,
    text: &mut coretext::Text,
    gpu: &mut tekes_ui_render::Renderer,
    cache: &mut SvgIconCache,
) -> Result<Scene, CompositionError> {
    let frame = Rect::new(0., 0., 400., 320.);
    let mut scene = Scene::new();
    text.atlas.begin_frame();
    let mut icons = cache.renderer(gpu, 1.);
    h.render(
        &mut scene,
        &mut WithIconRenderer::new(text, &mut icons),
        frame,
        frame,
        Theme::LIGHT.into(),
    )?;
    Ok(scene)
}
fn text() -> coretext::Text {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    text.backing = 1.;
    text
}
#[test]
fn canonical_disclosure_content_uses_real_svg_cache_for_every_direction() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    let mut text = text();
    let mut h = fixture();
    for (open, rtl, canonical) in [
        (true, false, "tekes-expand"),
        (false, false, "tekes-forward"),
        (false, true, "tekes-back"),
    ] {
        assert!(tekes_ui_icons::Glyph::named(canonical).is_some());
        disclosure(&mut h).set_expanded(open);
        disclosure(&mut h).set_rtl(rtl);
        let scene = paint(&mut h, &mut text, &mut gpu, &mut cache)
            .expect("Disclosure default must resolve in the original SvgIconCache");
        assert_eq!(
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Image(_)))
                .count(),
            2,
            "original title icon plus original indicator"
        );
        assert!(gpu.image_count() >= 2);
    }
}
#[test]
fn canonical_legacy_disclosure_view_uses_same_real_svg_resources() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    let mut text = text();
    let mut view = DisclosureView::default();
    let frame = Rect::new(0., 0., 240., 40.);
    for open in [false, true] {
        let model = Disclosure {
            open,
            disabled: false,
        };
        let mut scene = Scene::new();
        text.atlas.begin_frame();
        view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            Some(frame),
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
        );
        let mut icons = cache.renderer(&mut gpu, 1.);
        view.render_icon(
            &mut scene,
            &mut icons,
            &model,
            Some(frame),
            Theme::LIGHT.into(),
            16.,
        );
        assert_eq!(
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Image(_)))
                .count(),
            1
        );
    }
}
#[test]
fn compound_title_has_one_owner_and_external_fold_preserves_sibling_preedit_and_body_state() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    let mut text = text();
    let mut h = fixture();
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    let ax = h.semantics();
    for id in [TITLE, TITLE_TEXT, TITLE_ICON] {
        assert!(!ax.iter().any(|n| n.id == id));
    }
    assert_eq!(
        ax.iter().filter(|n| n.semantics.expanded.is_some()).count(),
        1
    );
    assert_eq!(
        ax.iter().find(|n| n.id == TRIGGER).unwrap().semantics.name,
        "复合标题 中文"
    );
    assert_eq!(
        h.focus_targets()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![SIBLING, TRIGGER, BODY]
    );
    let p = h.get(TITLE_ICON).unwrap().frame().center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    assert_eq!(h.pointer_capture().unwrap().0, TRIGGER);
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, TRIGGER);
    assert_eq!(records[0].action.source, ContentActionSource::Pointer);
    let Some(ContentActionPayload::DisclosureExpansion(intent)) = &records[0].payload else {
        panic!("original immutable intent absent")
    };
    assert!(disclosure(&mut h).confirm(intent, false));
    assert!(
        h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.source, ContentActionSource::Accessibility);
    assert_eq!(records[0].action.target, TRIGGER);
    let Some(ContentActionPayload::DisclosureExpansion(intent)) = &records[0].payload else {
        panic!("original immutable intent absent")
    };
    assert!(disclosure(&mut h).confirm(intent, false));
    assert!(h.focus(Some(BODY), true));
    h.event(&ContentEvent::Commit {
        text: " persisted".into(),
        replacement: None,
    });
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    let body_mount = h.semantic_mount(BODY);
    let trigger_mount = h.semantic_mount(TRIGGER);
    let body_before = h.semantic_text_input_snapshot(BODY).unwrap();
    assert!(body_before.marked.is_none());
    assert!(h.focus(Some(SIBLING), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    let sibling_mount = h.semantic_mount(SIBLING);
    let sibling_before = h.text_input_snapshot().unwrap();
    assert!(sibling_before.marked.is_some());
    for expanded in [false, true] {
        assert!(
            disclosure(&mut h).set_expanded(expanded),
            "another editor composition must not block this body"
        );
        paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
        assert_eq!(h.focused(), Some(SIBLING));
        assert_eq!(h.semantic_mount(SIBLING), sibling_mount);
        assert_eq!(h.text_input_snapshot().unwrap(), sibling_before);
        let input = &h
            .get(BODY)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        assert_eq!(input.text, body_before.text);
        assert_eq!(input.selection(), body_before.selection);
        assert_eq!(input.marked, body_before.marked);
        assert_eq!(h.semantic_mount(TRIGGER), trigger_mount);
    }
    assert_eq!(h.semantic_mount(BODY), body_mount);
    assert_eq!(h.semantic_text_input_snapshot(BODY).unwrap(), body_before);
    assert!(
        h.drain_action_records().is_empty(),
        "external confirmation cannot manufacture actions"
    );
}

#[test]
fn sibling_after_fold_keeps_editor_state_and_republishes_exact_translated_caret_geometry() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    let mut text = text();
    let mut h = fixture_order(false);
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    let body_mount = h.semantic_mount(BODY);
    let sibling_mount = h.semantic_mount(SIBLING);
    assert!(h.focus(Some(SIBLING), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    let before = h.text_input_snapshot().unwrap();
    assert!(before.marked.is_some());
    let frame = h.get(SIBLING).unwrap().frame();
    assert!(disclosure(&mut h).set_expanded(false));
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    let after = h.text_input_snapshot().unwrap();
    let moved = h.get(SIBLING).unwrap().frame();
    let dx = moved.min_x() - frame.min_x();
    let dy = moved.min_y() - frame.min_y();
    assert_eq!(moved.size, frame.size);
    assert_eq!(dx, 0.);
    assert!(
        dy < 0.,
        "fold must really move the following original editor upward"
    );
    assert_eq!(after.binding, before.binding);
    assert_eq!(after.text, before.text);
    assert_eq!(after.caret_index, before.caret_index);
    assert_eq!(after.selection, before.selection);
    assert_eq!(after.marked, before.marked);
    assert_eq!(after.read_only, before.read_only);
    let translate =
        |r: Rect| Rect::new(r.min_x() + dx, r.min_y() + dy, r.size.width, r.size.height);
    assert_eq!(after.caret, before.caret.map(translate));
    assert_eq!(after.clip, before.clip.map(translate));
    assert_eq!(
        after.carets,
        before
            .carets
            .iter()
            .map(|(index, r)| (*index, translate(*r)))
            .collect::<Vec<_>>()
    );
    assert_eq!(h.focused(), Some(SIBLING));
    assert_eq!(h.semantic_mount(SIBLING), sibling_mount);
    assert!(disclosure(&mut h).set_expanded(true));
    paint(&mut h, &mut text, &mut gpu, &mut cache).unwrap();
    assert_eq!(h.text_input_snapshot().unwrap(), before);
    assert_eq!(h.semantic_mount(BODY), body_mount);
    assert_eq!(h.focused(), Some(SIBLING));
    assert!(h.drain_action_records().is_empty());
}
