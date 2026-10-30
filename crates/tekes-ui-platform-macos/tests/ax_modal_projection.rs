#![cfg(feature = "content-accessibility")]
#[path = "support/ax_modal_fixture.rs"]
mod fixture;
use fixture::*;
use tekes_ui_controls::composition::{ContentResponse, SwitchContent};
use tekes_ui_platform_macos::{
    AccessibilityAction, AccessibilityRole,
    content_accessibility::{
        ContentAccessibilityOwner, ContentAccessibilitySnapshot, WindowAccessibilityPublication,
        content_action, content_key, content_nodes,
    },
};

#[test]
fn original_modal_scope_withdraws_background_and_restores_original_invoker() {
    let mut host = host();
    assert!(host.focus(Some(INVOKER), true));
    paint(&mut host);
    let invoker = content_key(&host, INVOKER, None).unwrap();
    let initial = ContentAccessibilitySnapshot::from_host(&host).unwrap();
    assert!(!initial.nodes().iter().any(|n| n.label == "Modal scope"));
    let previous =
        WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(0), initial)])
            .unwrap();
    host.show_dialog(DIALOG).unwrap();
    paint(&mut host);
    let modal = ContentAccessibilitySnapshot::from_host(&host).unwrap();
    assert!(
        modal
            .nodes()
            .iter()
            .any(|n| n.role == AccessibilityRole::Dialog && n.label == "Modal scope")
    );
    assert!(
        !modal
            .nodes()
            .iter()
            .any(|n| n.key == invoker || n.label == "Window content")
    );
    assert!(!previous.is_current());
    assert_eq!(
        previous.dispatch(
            ContentAccessibilityOwner(0),
            &mut host,
            invoker,
            AccessibilityAction::Press
        ),
        ContentResponse::default()
    );
    assert_eq!(
        content_action(&mut host, invoker, AccessibilityAction::Press),
        ContentResponse::default()
    );
    let choice = content_key(&host, CHOICE, None).unwrap();
    let active =
        WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(0), modal)])
            .unwrap();
    assert!(
        active
            .dispatch(
                ContentAccessibilityOwner(0),
                &mut host,
                choice,
                AccessibilityAction::Press
            )
            .changed
    );
    assert!(
        host.get(CHOICE)
            .unwrap()
            .component::<SwitchContent>()
            .unwrap()
            .model
            .checked
    );
    paint(&mut host);
    assert!(host.close_dialog());
    paint(&mut host);
    assert!(!active.is_current());
    let restored = content_nodes(&host);
    assert!(!restored.iter().any(|n| n.label == "Modal scope"
        || n.label == "Modal choice"
        || n.label == "Dismiss modal"));
    assert!(restored.iter().any(|n| n.key == invoker && n.focused));
    assert_eq!(host.focused(), Some(INVOKER));
    host.show_dialog(DIALOG).unwrap();
    paint(&mut host);
    assert!(
        content_nodes(&host)
            .iter()
            .any(|n| n.key == choice && n.selected)
    );
}

#[test]
fn inactive_original_host_withdraws_the_complete_modal_projection() {
    let mut host = host();
    host.show_dialog(DIALOG).unwrap();
    paint(&mut host);
    let choice = content_key(&host, CHOICE, None).unwrap();
    let previous = ContentAccessibilitySnapshot::from_host(&host).unwrap();
    host.set_window_active(false);
    assert!(content_nodes(&host).is_empty());
    assert!(ContentAccessibilitySnapshot::from_host(&host).is_none());
    assert!(!previous.is_current());
    assert_eq!(
        content_action(&mut host, choice, AccessibilityAction::Press),
        ContentResponse::default()
    );
    host.set_window_active(true);
    paint(&mut host);
    assert!(
        content_nodes(&host)
            .iter()
            .any(|n| n.key == choice && !n.selected)
    );
}
