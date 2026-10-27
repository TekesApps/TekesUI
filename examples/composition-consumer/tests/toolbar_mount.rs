#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;
use std::time::Duration;
use tekes_ui_composition_consumer::toolbar_mount::*;
use tekes_ui_controls::{ControlKey, ToolbarControl, composition::*};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
fn text() -> Text {
    let mut t = Text::new(1.);
    t.services = true;
    t
}
fn draw(host: &mut ToolbarHost<Source>, text: &mut Text) -> Scene {
    let mut s = Scene::new();
    host.render(&mut s, text, Theme::LIGHT.into()).unwrap();
    s
}
fn part(h: &ToolbarHost<Source>, name: &str) -> (ToolbarAccessibilityTarget, Rect) {
    for node in h.semantics() {
        for (target, p) in node.parts {
            if p.semantics.name == name {
                return (target, p.frame);
            }
        }
    }
    panic!("missing part {name}");
}
fn key(h: &mut ToolbarHost<Source>, key: ControlKey, down: bool) {
    h.event(&ContentEvent::Key { key, down });
}
#[test]
fn independent_undo_redo_group_gets_hover_hint_click_and_original_module_binding() {
    let mut h = build(900.).unwrap();
    let mut t = text();
    let before = draw(&mut h, &mut t);
    let (_, hit) = part(&h, "Undo");
    h.event(&ContentEvent::PointerMove(hit.center()));
    let hovered = draw(&mut h, &mut t);
    assert_ne!(
        before, hovered,
        "single event automatically reaches the added group"
    );
    t.now = Duration::from_secs(1);
    draw(&mut h, &mut t);
    let (owner, hint, layout) = h.help_tooltip().unwrap();
    assert_eq!(owner, EDIT_CONTROL);
    assert_eq!(hint, "Undo settings edit");
    assert!(layout.visible.is_some());
    h.event(&ContentEvent::PointerDown(hit.center()));
    h.event(&ContentEvent::PointerUp(hit.center()));
    let cmds = h.drain_commands();
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0].source,
        Source {
            module: "settings",
            generation: 9
        }
    );
    assert_eq!(cmds[0].command, 100);
    assert_eq!(cmds[0].group, EDIT);
    assert_eq!(cmds[0].window, 7);
    assert!(h.drain_commands().is_empty());
}
#[test]
fn keyboard_ax_and_menu_scope_are_shared_across_main_and_settings() {
    let mut h = build(900.).unwrap();
    let mut t = text();
    draw(&mut h, &mut t);
    key(&mut h, ControlKey::Tab { reverse: false }, true);
    key(&mut h, ControlKey::Enter, true);
    assert_eq!(h.drain_commands()[0].source.module, "navigation");
    key(&mut h, ControlKey::Tab { reverse: false }, true);
    key(&mut h, ControlKey::Right, true);
    key(&mut h, ControlKey::Enter, true);
    assert_eq!(h.drain_commands()[0].command, 101);
    let (undo, _) = part(&h, "Undo");
    assert!(
        h.accessibility_action(undo, AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.drain_commands()[0].command, 100);
    let (old_home, _) = part(&h, "Home");
    h.set_regions(7, Rect::new(0., 0., 340., 360.), regions(340.))
        .unwrap();
    draw(&mut h, &mut t);
    let more = h
        .semantics()
        .into_iter()
        .find(|n| n.node.id == EDIT_CONTROL)
        .unwrap()
        .parts
        .into_iter()
        .find(|(_, p)| p.semantics.role == ContentRole::MenuButton)
        .unwrap()
        .0;
    assert!(
        h.accessibility_action(more, AccessibilityAction::Press)
            .consumed
    );
    draw(&mut h, &mut t);
    assert!(
        !h.accessibility_action(old_home, AccessibilityAction::Press)
            .activated
    );
    assert!(h.semantics().iter().all(|n| n.node.id == EDIT_CONTROL));
    key(&mut h, ControlKey::Down, true);
    key(&mut h, ControlKey::Enter, true);
    let cmd = h.drain_commands();
    assert_eq!(cmd.len(), 1);
    assert_eq!(cmd[0].source.module, "settings");
}
#[test]
fn capture_cancels_on_resize_zoom_hide_remove_and_deactivation_and_stale_ax_is_rejected() {
    for change in 0..6 {
        let mut h = build(900.).unwrap();
        let mut t = text();
        draw(&mut h, &mut t);
        let (target, r) = part(&h, "Undo");
        h.event(&ContentEvent::PointerDown(r.center()));
        match change {
            0 => h
                .set_regions(7, Rect::new(0., 0., 800., 360.), regions(800.))
                .unwrap(),
            1 => t.scale = 1.5,
            2 => h.set_visible(EDIT, false).unwrap(),
            3 => {
                h.remove(EDIT).unwrap();
            }
            4 => h.set_window_active(false),
            _ => {
                h.update_component::<ToolbarControl>(EDIT, EDIT_CONTROL, |control| {
                    control.set_actions(vec![])
                })
                .unwrap();
            }
        }
        draw(&mut h, &mut t);
        h.event(&ContentEvent::PointerUp(r.center()));
        assert!(h.drain_commands().is_empty(), "change {change}");
        assert!(
            !h.accessibility_action(target, AccessibilityAction::Press)
                .activated,
            "old target {change}"
        );
        assert!(h.help_tooltip().is_none());
    }
}
#[test]
fn moving_original_group_keeps_hints_and_arbitrary_progress_has_original_semantics() {
    let mut h = build(900.).unwrap();
    let mut t = text();
    draw(&mut h, &mut t);
    assert!(
        h.semantics()
            .iter()
            .any(|n| n.node.semantics.name == "Import progress"
                && n.node.semantics.value.as_deref() == Some("42%"))
    );
    let original_mount = h.content_host().semantic_mount(EDIT_CONTROL);
    let old_geometry = h.content_host().geometry_snapshot().unwrap();
    let old_publication = tekes_ui_platform_macos::content_accessibility::WindowAccessibilityPublication::prepare(vec![(
        tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(1),
        tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot::from_host(h.content_host()).unwrap(),
    )]).unwrap();
    h.move_group(EDIT, MAIN, ToolbarPlacement::PrimaryAction, 3)
        .unwrap();
    assert!(!old_geometry.is_current());
    assert!(!old_publication.is_current());
    assert_eq!(h.content_host().semantic_mount(EDIT_CONTROL), original_mount);
    assert!(h.content_host().geometry_snapshot().is_none());
    draw(&mut h, &mut t);
    assert_eq!(h.content_host().semantic_mount(EDIT_CONTROL), original_mount);
    let (_, r) = part(&h, "Undo");
    assert!(r.max_x() < 495.);
    h.event(&ContentEvent::PointerMove(r.center()));
    draw(&mut h, &mut t);
    t.now = Duration::from_secs(1);
    draw(&mut h, &mut t);
    assert_eq!(h.help_tooltip().unwrap().1, "Undo settings edit");
    h.set_visible(EDIT, false).unwrap();
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_none());
    h.set_visible(EDIT, true).unwrap();
    draw(&mut h, &mut t);
    let (_, r) = part(&h, "Undo");
    h.event(&ContentEvent::PointerMove(r.center()));
    draw(&mut h, &mut t);
    t.now = Duration::from_secs(2);
    draw(&mut h, &mut t);
    assert!(h.help_tooltip().is_some());
}

#[test]
fn declaration_replacement_revokes_old_generation_without_retargeting_pending_commands() {
    let mut h = build(900.).unwrap();
    let mut t = text();
    draw(&mut h, &mut t);
    let (old, undo) = part(&h, "Undo");
    h.event(&ContentEvent::PointerDown(undo.center()));
    h.event(&ContentEvent::PointerUp(undo.center()));
    let mut updated = undo_redo();
    for binding in &mut updated.bindings {
        binding.source.generation = 10;
    }
    h.register(updated).unwrap();
    draw(&mut h, &mut t);
    assert!(
        h.drain_commands().is_empty(),
        "old undrained generation is revoked"
    );
    assert!(
        !h.accessibility_action(old, AccessibilityAction::Press)
            .consumed
    );
    let (fresh, _) = part(&h, "Undo");
    assert!(
        h.accessibility_action(fresh, AccessibilityAction::Press)
            .activated
    );
    let commands = h.drain_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].source.generation, 10);
    assert_eq!(commands[0].source.module, "settings");
}
#[test]
fn outside_menu_input_is_consumed_and_cannot_activate_a_background_group() {
    let mut h = build(340.).unwrap();
    let mut t = text();
    draw(&mut h, &mut t);
    let find_more = |h: &ToolbarHost<Source>, id| {
        h.semantics()
            .into_iter()
            .find(|n| n.node.id == id)
            .unwrap()
            .parts
            .into_iter()
            .find(|(_, p)| p.semantics.role == ContentRole::MenuButton)
            .unwrap()
    };
    let (other, other_frame) = find_more(&h, ComponentId(21011));
    let (more, _) = find_more(&h, EDIT_CONTROL);
    h.accessibility_action(more, AccessibilityAction::Press);
    draw(&mut h, &mut t);
    assert!(
        !h.accessibility_action(other, AccessibilityAction::Press)
            .consumed,
        "fresh background AX rejected by scope"
    );
    assert!(
        h.event(&ContentEvent::PointerDown(other_frame.frame.center()))
            .consumed
    );
    assert!(!h.event_bubbled());
    h.event(&ContentEvent::PointerUp(other_frame.frame.center()));
    assert!(h.drain_commands().is_empty());
    draw(&mut h, &mut t);
    assert!(
        h.semantics()
            .iter()
            .any(|n| n.node.id == ComponentId(21011))
    );
}
#[test]
fn declared_regions_support_independent_rows_and_native_safe_area_without_changing_placement() {
    let mut h = build(900.).unwrap();
    let mut t = text();
    draw(&mut h, &mut t);
    let (old, old_hit) = part(&h, "Undo");
    let old_mount = h.content_host().semantic_mount(EDIT_CONTROL);
    let mut allocations = regions(900.);
    allocations[1].available = Rect::new(100., 120., 700., 84.);
    allocations[1].native_safe_area.left = 30.;
    h.set_regions(7, Rect::new(0., 0., 900., 360.), allocations)
        .unwrap();
    assert!(
        h.semantics().is_empty(),
        "no stale geometry before republish"
    );
    draw(&mut h, &mut t);
    let (_, new_hit) = part(&h, "Undo");
    assert!(new_hit.min_y() > old_hit.max_y());
    assert!(new_hit.min_x() >= 130.);
    assert_eq!(h.content_host().semantic_mount(EDIT_CONTROL), old_mount);
    assert!(
        !h.accessibility_action(old, AccessibilityAction::Press)
            .consumed
    );
}
