#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;

use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Theme, proposal};

const ROOT: ComponentId = ComponentId(83100);
const FIELD: ComponentId = ComponentId(83101);

fn text(scale: f32) -> coretext::Text {
    let mut text = coretext::Text::new(scale);
    text.services = true;
    text
}
fn host() -> ContentHost {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack { axis: Axis::Horizontal, gap: 0. },
            children: vec![Content::new(FIELD, TextFieldContent::inline("Search", "draft 😀", 12.))],
            flexible_child: None,
        },
    ))
    .unwrap()
}
fn render(host: &mut ContentHost, text: &mut coretext::Text, width: f32) -> Scene {
    let frame = Rect::new(20., 30., width, 80.);
    let mut scene = Scene::new();
    host.render(&mut scene, text, frame, frame, Theme::LIGHT.into()).unwrap();
    scene
}

#[test]
fn inline_width_follows_real_typography_and_allocation_without_form_metadata() {
    for scale in [0.75, 1., 1.5, 2.] {
        for available in [48., 200., 600.] {
            let mut text = text(scale);
            let cap = text.resolved_font_size(proposal::TextRole::UiBody) * 12.;
            let mut inline = TextFieldContent::inline("Search", "draft 😀", 12.);
            let mut ordinary = TextFieldContent::new("Search", "draft 😀");
            let inline_size = inline.measure(&mut text, Size::new(available, 200.)).size;
            let ordinary_size = ordinary.measure(&mut text, Size::new(available, 200.)).size;
            assert_eq!(inline_size.width, available.min(cap));
            assert_eq!(ordinary_size.width, available);
            assert!(ordinary.show_metadata);
            assert!(!inline.show_metadata);
            assert!(inline_size.height > 0. && inline_size.height < ordinary_size.height);
            assert!(!inline.surface.input.multiline);

            let mut host = host();
            let scene = render(&mut host, &mut text, available);
            let surface = host.text_surface(FIELD).unwrap();
            assert_eq!(surface.outer.size.width, available.min(cap));
            assert_eq!(surface.type_scale, scale);
            assert!(surface.paragraph.is_some());
            assert!(!scene.primitives().is_empty());
            let node = host.semantics().into_iter().find(|node| node.id == FIELD).unwrap();
            assert_eq!(node.semantics.role, ContentRole::TextInput);
            assert_eq!(node.semantics.name, "Search");
            assert_eq!(node.semantics.value.as_deref(), Some("draft 😀"));
        }
    }
}

#[test]
fn inline_retains_original_editor_preedit_binding_and_disabled_input_contract() {
    let mut host = host();
    let mut text = text(1.);
    render(&mut host, &mut text, 400.);
    assert!(host.focus(Some(FIELD), false));
    let binding = host.focused_text_binding().unwrap();
    let mount = host.semantic_mount(FIELD).unwrap();
    assert!(host.event(&ContentEvent::Preedit {
        text: "拼音".into(), selection: 2..2,
        clauses: std::iter::once(0..2).collect(), replacement: Some(0..8),
    }).consumed);
    let before = host.focused_text().unwrap().input.text.clone();
    let marked = host.focused_text().unwrap().input.marked.clone();
    assert_eq!(marked, Some(0..2));
    text.scale = 1.5;
    render(&mut host, &mut text, 180.);
    assert_eq!(host.focused(), Some(FIELD));
    assert_eq!(host.focused_text_binding(), Some(binding));
    assert_eq!(host.semantic_mount(FIELD), Some(mount));
    assert_eq!(host.focused_text().unwrap().input.text, before);
    assert_eq!(host.focused_text().unwrap().input.marked, marked);
    assert!(host.event(&ContentEvent::Commit { text: "中文".into(), replacement: None }).consumed);
    render(&mut host, &mut text, 180.);
    let layout = host.semantic_text_layout_snapshot(FIELD, None).unwrap();
    assert_eq!(layout.input.text, "中文");
    assert_eq!(layout.input.marked, None);
    assert!(!layout.lines.is_empty() && !layout.carets.is_empty());

    host.get_mut(FIELD).unwrap().component_mut::<TextFieldContent>().unwrap().surface.input.disabled = true;
    assert!(!host.event(&ContentEvent::Commit { text: "late".into(), replacement: None }).consumed);
    assert!(!host.accessibility_action(FIELD, None, AccessibilityAction::SetValue("late".into())).consumed);
    assert_eq!(host.get(FIELD).unwrap().component::<TextFieldContent>().unwrap().surface.input.text, "中文");
    host.set_window_active(false);
    assert_eq!(host.semantic_mount(FIELD), None);
    assert_eq!(host.semantic_text_layout_snapshot(FIELD, None), None);
}

#[test]
fn invalid_inline_widths_use_existing_zero_width_measurement_without_changing_form_default() {
    let mut text = text(1.);
    for width_em in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1., 0.] {
        let mut field = TextFieldContent::inline("Search", "draft", width_em);
        let size = field.measure(&mut text, Size::new(200., 100.)).size;
        assert_eq!(size.width, 0.);
        assert!(size.height.is_finite() && size.height > 0.);
    }
    let mut ordinary = TextFieldContent::new("Search", "draft");
    assert_eq!(ordinary.measure(&mut text, Size::new(200., 100.)).size.width, 200.);
    assert!(ordinary.show_metadata);
}
