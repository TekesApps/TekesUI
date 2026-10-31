use std::rc::Rc;
use tekes_ui_composition_consumer::tree_content::*;
use tekes_ui_controls::{composition::*, layout_view::IconRenderer};
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_tokens::{Rgba, Theme};
#[path = "support/coretext.rs"]
mod coretext;
struct Icons;
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("checked resources required")
    }
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(1), Rc::new(())))
    }
}
#[test]
fn public_tree_text_icon_composer_keep_original_owner_and_preedit_barrier() {
    let mut h = build().unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut icons = Icons;
    let f = Rect::new(0., 0., 500., 700.);
    let render = |h: &mut ContentHost, text: &mut coretext::Text, icons: &mut Icons| {
        h.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(text, icons),
            f,
            f,
            Theme::LIGHT.into(),
        )
        .unwrap()
    };
    render(&mut h, &mut text, &mut icons);
    let tree = h.semantics().into_iter().find(|n| n.id == TREE).unwrap();
    assert_eq!(tree.semantics.role, ContentRole::Tree);
    assert_eq!(tree.parts.len(), 3);
    assert_eq!(
        h.semantics()
            .iter()
            .filter(|n| n.id == ComponentId(202))
            .count(),
        0,
        "decorative descendant suppressed"
    );
    let display_mount = h.semantic_mount(ComponentId(202));
    let child = tree.parts[1].index;
    assert!(
        h.accessibility_action(TREE, Some(child), AccessibilityAction::Press)
            .activated
    );
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    let i = match &records[0].payload {
        Some(ContentActionPayload::TreeIntent(i)) => i.clone(),
        _ => panic!("typed TreeIntent"),
    };
    assert!(h.focus(Some(COMPOSER), true));
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
    let before = h.semantic_text_input_snapshot(COMPOSER).unwrap();
    for action in [
        AccessibilityAction::Focus,
        AccessibilityAction::Press,
        AccessibilityAction::SetExpanded(false),
    ] {
        assert!(
            !h.accessibility_action(TREE, Some(tree.parts[0].index), action)
                .consumed
        );
    }
    reorder(&mut h).unwrap();
    h.set_inherited_surface("canvas").unwrap();
    render(&mut h, &mut text, &mut icons);
    assert_eq!(h.semantic_mount(ComponentId(202)), display_mount);
    assert!(
        h.get(TREE)
            .unwrap()
            .component::<TreeContent>()
            .unwrap()
            .intent_is_current(&i)
    );
    assert_eq!(h.focused_text_binding(), binding);
    let after = h.semantic_text_input_snapshot(COMPOSER).unwrap();
    assert_eq!(
        (after.text, after.selection, after.marked),
        (before.text, before.selection, before.marked)
    );
    assert_eq!(h.focused(), Some(COMPOSER));
    assert!(h.is_composing());
    assert!(h.drain_action_records().is_empty());
}
