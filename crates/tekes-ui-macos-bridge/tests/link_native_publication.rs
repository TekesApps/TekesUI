#[cfg(all(target_os = "macos", feature = "selftest"))]
fn main() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, TextFieldContent};
    use tekes_ui_macos_bridge::content_accessibility::{
        ContentAccessibilityOwner, ContentAccessibilitySnapshot, WindowAccessibilityPublication,
    };
    use tekes_ui_macos_bridge::coretext_host::CoreTextHost;
    use tekes_ui_platform_macos::AccessibilityGeometryLease;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};

    let host = ContentHost::new(Content::new(
        ComponentId(1),
        TextFieldContent::new("Search", "a"),
    ))
    .unwrap();
    let text = CoreTextHost::new();
    let frame = Rect::new(0.0, 0.0, 200.0, 100.0);
    let snap = move |host: &mut ContentHost,
                     text: &mut CoreTextHost|
          -> Rc<dyn AccessibilityGeometryLease> {
        let mut scene = Scene::new();
        host.render(&mut scene, text, frame, frame, Palette::from(Theme::LIGHT))
            .unwrap();
        let snapshot = ContentAccessibilitySnapshot::from_host(host).unwrap();
        Rc::new(
            WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(0), snapshot)])
                .unwrap(),
        )
    };
    let harness = Rc::new(RefCell::new((host, text)));
    let first = {
        let mut guard = harness.borrow_mut();
        let (host, text) = &mut *guard;
        snap(host, text)
    };
    let harness2 = harness.clone();
    let pub_checks = tekes_ui_platform_macos::selftest::content_accessibility_publication_contract(
        first,
        move || {
            let mut guard = harness2.borrow_mut();
            let (host, text) = &mut *guard;
            host.focus(Some(ComponentId(1)), false);
            host.event_with_text(
                &tekes_ui_controls::composition::ContentEvent::Commit {
                    text: "b".into(),
                    replacement: None,
                },
                text,
            )
            .unwrap();
            snap(host, text)
        },
    );
    assert_eq!(pub_checks, 4);
}
#[cfg(not(all(target_os = "macos", feature = "selftest")))]
fn main() {
    println!("LINK_NATIVE_PUBLICATION_UNAVAILABLE");
}
