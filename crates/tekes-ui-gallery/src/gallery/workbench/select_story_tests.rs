//! Real Gallery/Workbench route; keys come from the original native projection.
use super::super::{Gallery, Page, static_text::StaticText};
use super::*;
use tekes_ui_controls::composition::ComponentId;
use tekes_ui_platform_macos::{
    AccessibilityAction as A, AccessibilityKey as K, AccessibilityRole as R, Modifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
fn gallery() -> (Gallery, usize) {
    let i = entries()
        .iter()
        .position(|e| e.renderer() == "select")
        .unwrap();
    let mut g = Gallery::with_static_text(StaticText::new(None));
    g.page = Page::Workbench(i);
    g.set_view_size(1200., 850.);
    paint(&mut g);
    (g, i)
}
fn paint(g: &mut Gallery) {
    g.build_scene(&mut Scene::new());
}
fn root(g: &Gallery) -> K {
    g.accessibility_nodes()
        .iter()
        .find(|n| n.role == R::PopUpButton)
        .unwrap()
        .key
}
fn option(g: &Gallery, index: usize) -> K {
    g.accessibility_nodes()
        .iter()
        .filter(|n| n.role == R::MenuItem)
        .nth(index)
        .unwrap()
        .key
}
fn group(g: &Gallery) -> K {
    g.accessibility_nodes()
        .iter()
        .find(|n| n.role == R::Menu)
        .unwrap()
        .key
}
fn press(g: &mut Gallery, k: K) {
    assert!(g.accessibility_action(k, &A::Press).redraw);
    paint(g);
}
#[test]
fn select_root_focus_uses_original_host_admission_without_preparing_an_action() {
    let (mut g, i) = gallery();
    let r = root(&g);
    let before = g.workbench.select_value_for_test(i);
    assert!(g.accessibility_action(r, &A::Focus).redraw);
    paint(&mut g);
    assert_eq!(
        g.workbench.state(i).select_story.host.focused(),
        Some(ComponentId(8901))
    );
    assert_eq!(g.workbench.select_value_for_test(i), before);
    assert!(!g.workbench.select_value_for_test(i).0);
    assert_eq!(
        g.accessibility_nodes().iter().filter(|n| n.focused).count(),
        1
    );
}
#[test]
fn select_native_actual_owner_opaque_reopen_and_same_value_queue_once() {
    let (mut g, i) = gallery();
    let r = root(&g);
    let initial = g
        .accessibility_nodes()
        .into_iter()
        .find(|n| n.key == r)
        .unwrap();
    assert_eq!(initial.role, R::PopUpButton);
    assert_eq!(initial.expanded, Some(false));
    press(&mut g, r);
    let nodes = g.accessibility_nodes();
    let items: Vec<_> = nodes.iter().filter(|n| n.role == R::MenuItem).collect();
    assert_eq!(items.len(), 3);
    let group_key = group(&g);
    let menu = nodes.iter().find(|n| n.key == group_key).unwrap();
    assert_eq!(menu.parent, Some(r));
    assert!(items.iter().all(|n| n.parent == Some(group_key)));
    for (n, expected) in items.iter().zip(["Automatic", "Compact", "Comfortable"]) {
        assert_eq!(n.label, expected);
    }
    assert_eq!(nodes.iter().filter(|n| n.focused).count(), 1);
    assert!(!g.accessibility_action(group_key, &A::Press).redraw);
    assert!(!g.accessibility_action(group_key, &A::Focus).redraw);
    let old = option(&g, 2);
    assert!(g.accessibility_action(old, &A::Focus).redraw);
    assert_eq!(g.workbench.select_value_for_test(i).1, 0);
    let before = g
        .workbench
        .state(i)
        .log
        .iter()
        .filter(|s| s.starts_with("select: ") && s.as_str() != "select: open")
        .count();
    press(&mut g, old);
    assert_eq!(g.workbench.select_value_for_test(i), (false, 2));
    assert_eq!(g.workbench.state(i).focus, Some(20));
    assert!(
        !g.accessibility_nodes()
            .iter()
            .any(|n| matches!(n.role, R::Menu | R::MenuItem))
    );
    assert_eq!(
        g.workbench
            .state(i)
            .log
            .iter()
            .filter(|s| s.starts_with("select: ") && s.as_str() != "select: open")
            .count(),
        before + 1
    );
    assert!(!g.accessibility_action(old, &A::Press).redraw);
    press(&mut g, r);
    let new = option(&g, 2);
    assert_ne!(old, new);
    let n = g
        .accessibility_nodes()
        .into_iter()
        .find(|n| n.key == new)
        .unwrap();
    assert!(n.selected && n.focused);
    press(&mut g, new);
    assert_eq!(
        g.workbench
            .state(i)
            .log
            .iter()
            .filter(|s| s.starts_with("select: ") && s.as_str() != "select: open")
            .count(),
        before + 2
    );
    assert!(
        g.workbench
            .state(i)
            .select_story
            .host
            .drain_action_records()
            .is_empty()
    );
}
#[test]
fn select_native_expanded_press_background_focus_and_disabled_empty_revoke() {
    let (mut g, i) = gallery();
    let r = root(&g);
    press(&mut g, r);
    let old = option(&g, 0);
    press(&mut g, r);
    assert!(!g.workbench.select_value_for_test(i).0);
    assert_eq!(
        g.accessibility_nodes()
            .into_iter()
            .find(|n| n.key == r)
            .unwrap()
            .expanded,
        Some(false)
    );
    assert!(!g.accessibility_action(old, &A::Press).redraw);
    press(&mut g, r);
    let background = K {
        component: crate::gallery::accessibility::STORY + i as u64,
        part: Some(1),
    };
    assert!(g.accessibility_action(background, &A::Focus).redraw);
    paint(&mut g);
    assert!(!g.workbench.select_value_for_test(i).0);
    let nodes = g.accessibility_nodes();
    assert_eq!(nodes.iter().filter(|n| n.focused).count(), 1);
    assert!(nodes.iter().any(|n| n.key == background && n.focused));
    press(&mut g, r);
    let old = option(&g, 1);
    g.workbench.state(i).disabled = true;
    paint(&mut g);
    assert!(!g.accessibility_action(old, &A::Press).redraw);
    g.workbench.state(i).disabled = false;
    g.workbench.state(i).select_empty = true;
    paint(&mut g);
    assert!(!g.accessibility_action(r, &A::Press).redraw);
    g.workbench.state(i).select_empty = false;
    paint(&mut g);
    press(&mut g, r);
    assert_ne!(option(&g, 1), old);
    assert!(!g.accessibility_action(old, &A::Press).redraw);
}
#[test]
fn select_actual_outer_focus_dismisses_and_old_legacy_positional_key_is_rejected() {
    for search in [true, false] {
        let (mut g, i) = gallery();
        let r = root(&g);
        press(&mut g, r);
        let old = option(&g, 0);
        let target = if search {
            K {
                component: crate::gallery::accessibility::SEARCH,
                part: Some(0),
            }
        } else {
            g.accessibility_nodes()
                .iter()
                .find(|n| {
                    n.key.component == crate::gallery::accessibility::NAV && n.key.part.is_some()
                })
                .unwrap()
                .key
        };
        assert!(g.accessibility_action(target, &A::Focus).redraw);
        paint(&mut g);
        assert!(!g.workbench.select_value_for_test(i).0);
        let nodes = g.accessibility_nodes();
        assert_eq!(nodes.iter().filter(|n| n.focused).count(), 1);
        assert!(nodes.iter().any(|n| n.key == target && n.focused));
        assert!(!g.accessibility_action(old, &A::Press).redraw);
        let legacy = K {
            component: crate::gallery::accessibility::STORY + i as u64,
            part: Some(60),
        };
        assert!(!g.accessibility_action(legacy, &A::Press).redraw);
    }
}
fn mouse(frame: tekes_ui_platform_macos::Rect, down: bool) -> Event {
    let p = tekes_ui_platform_macos::Point {
        x: frame.origin.x + frame.size.width / 2.,
        y: frame.origin.y + frame.size.height / 2.,
    };
    Event::Mouse(MouseEvent {
        position: tekes_ui_platform_macos::Point { x: p.x, y: p.y },
        modifiers: Modifiers::NONE,
        kind: if down {
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            }
        } else {
            MouseEventKind::Up {
                button: MouseButton::Left,
                click_count: 1,
            }
        },
    })
}
#[test]
fn select_actual_pointer_menu_confirmation_and_outside_release_do_not_double_deliver() {
    let (mut g, i) = gallery();
    let frame = g
        .accessibility_nodes()
        .iter()
        .find(|n| n.role == R::PopUpButton)
        .unwrap()
        .frame;
    g.handle_input(&mouse(frame, true));
    g.handle_input(&mouse(frame, false));
    paint(&mut g);
    assert!(g.workbench.select_value_for_test(i).0);
    let row = g
        .accessibility_nodes()
        .iter()
        .filter(|n| n.role == R::MenuItem)
        .nth(1)
        .unwrap()
        .frame;
    g.handle_input(&mouse(row, true));
    g.handle_input(&mouse(row, false));
    paint(&mut g);
    assert_eq!(g.workbench.select_value_for_test(i), (false, 1));
    assert_eq!(
        g.workbench
            .state(i)
            .log
            .iter()
            .filter(|s| s.as_str() == "select: 1")
            .count(),
        1
    );
}
