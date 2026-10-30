//! Opt-in native selector probe. Opens one real window; no physical input or GPU submission.
//! Run only in an allocated native window. This is not a VoiceOver speech test.
#[cfg(all(target_os = "macos", feature = "content-accessibility"))]
#[path = "../tests/support/ax_modal_fixture.rs"]
mod fixture;

#[cfg(all(target_os = "macos", feature = "content-accessibility"))]
mod native {
    use super::fixture::*;
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{MainThreadMarker, NSArray, NSNumber, NSString};
    use std::{cell::RefCell, rc::Rc};
    use tekes_ui_controls::composition::{ContentHost, SwitchContent};
    use tekes_ui_platform_macos::{
        AccessibilityAction, App, AppConfig, Event, MenuBar, Size, Window, WindowConfig,
        WindowHandler,
        content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot,
            WindowAccessibilityPublication, content_key,
        },
    };
    const TITLE: &str = "AX modal lifecycle probe";

    fn children(node: &AnyObject) -> Retained<NSArray<AnyObject>> {
        unsafe { msg_send![node, accessibilityChildren] }
    }
    fn named(node: &AnyObject, label: &str) -> Option<Retained<AnyObject>> {
        for child in children(node).iter() {
            let name: Retained<NSString> = unsafe { msg_send![&*child, accessibilityLabel] };
            if name.to_string() == label {
                return Some(child.retain());
            }
            if let Some(found) = named(&child, label) {
                return Some(found);
            }
        }
        None
    }
    fn focused(view: &AnyObject) -> Retained<AnyObject> {
        unsafe { msg_send![view, accessibilityFocusedUIElement] }
    }
    fn press(node: &AnyObject) {
        let accepted: bool = unsafe { msg_send![node, accessibilityPerformPress] };
        assert!(
            accepted,
            "current native selector must enter the original handler"
        );
    }
    fn value(node: &AnyObject) -> i32 {
        let value: Retained<NSNumber> = unsafe { msg_send![node, accessibilityValue] };
        value.intValue()
    }

    #[derive(Default)]
    struct Saved {
        view: Option<Retained<AnyObject>>,
        element: Option<Retained<AnyObject>>,
        publication: Option<WindowAccessibilityPublication>,
        actions: usize,
        completed: bool,
    }
    struct Handler {
        host: ContentHost,
        saved: Rc<RefCell<Saved>>,
    }
    impl Handler {
        fn publish(&mut self, window: &Window) -> Retained<AnyObject> {
            paint(&mut self.host);
            assert!(window.set_content_accessibility_snapshot(
                ContentAccessibilitySnapshot::from_host(&self.host).unwrap()
            ));
            let mtm = MainThreadMarker::new().unwrap();
            let windows = NSApplication::sharedApplication(mtm).windows();
            let window = windows
                .iter()
                .find(|w| w.title().to_string() == TITLE)
                .expect("probe's actual window");
            let screen = window.screen().expect("probe window's actual screen");
            let key = NSString::from_str("NSScreenNumber");
            let id = screen
                .deviceDescription()
                .objectForKey(&key)
                .expect("screen device ID")
                .downcast::<NSNumber>()
                .expect("screen device ID is a number");
            println!("AX_MODAL_DISPLAY_ID={}", id.unsignedIntValue());
            window.contentView().unwrap().into()
        }
    }
    impl WindowHandler for Handler {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::Created => {
                    assert!(self.host.focus(Some(INVOKER), true));
                    let view = self.publish(window);
                    assert!(named(&view, "Modal scope").is_none());
                    press(&named(&view, "Open modal").unwrap());
                }
                Event::Accessibility { target, action } => {
                    assert!(matches!(action, AccessibilityAction::Press));
                    self.saved.borrow_mut().actions += 1;
                    let publication = window.content_accessibility_publication().unwrap();
                    let response = publication.dispatch(
                        ContentAccessibilityOwner(0),
                        &mut self.host,
                        target,
                        action,
                    );
                    assert!(response.consumed);
                    if Some(target) == content_key(&self.host, INVOKER, None) {
                        assert!(response.activated);
                        self.host.show_dialog(DIALOG).unwrap();
                        let view = self.publish(window);
                        assert!(named(&view, "Open modal").is_none());
                        let dialog = named(&view, "Modal scope").unwrap();
                        let role: Retained<NSString> =
                            unsafe { msg_send![&*dialog, accessibilityRole] };
                        assert_eq!(role.to_string(), "AXGroup");
                        let subrole: Retained<NSString> =
                            unsafe { msg_send![&*dialog, accessibilitySubrole] };
                        assert_eq!(subrole.to_string(), "AXDialog");
                        let choice = named(&view, "Modal choice").unwrap();
                        assert_eq!(value(&choice), 0);
                        press(&choice);
                    } else if Some(target) == content_key(&self.host, CHOICE, None) {
                        assert!(response.changed);
                        assert!(
                            self.host
                                .get(CHOICE)
                                .unwrap()
                                .component::<SwitchContent>()
                                .unwrap()
                                .model
                                .checked
                        );
                        let view = self.publish(window);
                        assert_eq!(value(&named(&view, "Modal choice").unwrap()), 1);
                        press(&named(&view, "Dismiss modal").unwrap());
                    } else {
                        assert_eq!(Some(target), content_key(&self.host, CANCEL, None));
                        assert!(response.activated);
                        assert!(self.host.close_dialog());
                        let view = self.publish(window);
                        assert!(named(&view, "Modal scope").is_none());
                        let invoker = named(&view, "Open modal").unwrap();
                        assert!(std::ptr::eq(&*focused(&view), &*invoker));
                        let publication = window.content_accessibility_publication().unwrap();
                        self.saved.borrow_mut().view = Some(view.clone());
                        self.saved.borrow_mut().element = Some(invoker.clone());
                        self.saved.borrow_mut().publication = Some(publication.clone());
                        window.close(); // Actual NSWindow -> AppDelegate.windowWillClose boundary.
                        assert!(
                            !publication.is_current(),
                            "close must revoke saved whole-batch authority before App.run finishes"
                        );
                        assert!(window.accessibility_nodes().is_empty());
                        assert!(children(&view).is_empty());
                        assert!(
                            std::ptr::eq(&*focused(&view), &*view),
                            "closed view must not return its old focused element"
                        );
                        let live: bool = unsafe { msg_send![&*invoker, isAccessibilityElement] };
                        let accepted: bool =
                            unsafe { msg_send![&*invoker, accessibilityPerformPress] };
                        assert!(!live && !accepted);
                        self.saved.borrow_mut().completed = true;
                    }
                }
                _ => {}
            }
        }
    }
    pub fn run() {
        let saved = Rc::new(RefCell::new(Saved::default()));
        App::new(AppConfig::default()).run(
            WindowConfig {
                title: TITLE.into(),
                size: Size {
                    width: 800.,
                    height: 600.,
                },
                ..Default::default()
            },
            MenuBar::default(),
            Handler {
                host: host(),
                saved: saved.clone(),
            },
        );
        // Retained native clients stay inert after App.run's Shared::finish too.
        let saved = saved.borrow();
        assert!(saved.completed);
        assert_eq!(saved.actions, 3);
        let view = saved.view.as_ref().unwrap();
        assert!(children(view).is_empty());
        assert!(std::ptr::eq(&*focused(view), &**view));
        let element = saved.element.as_ref().unwrap();
        let live: bool = unsafe { msg_send![&**element, isAccessibilityElement] };
        assert!(!live);
        assert!(!saved.publication.as_ref().unwrap().is_current());
        println!(
            "AX_MODAL_LIFECYCLE_NATIVE_OK windows=1 native_presses=3 physical_input=0 gpu_submissions=0 voiceover_test=false"
        );
    }
}
#[cfg(all(target_os = "macos", feature = "content-accessibility"))]
fn main() {
    native::run();
}
#[cfg(not(all(target_os = "macos", feature = "content-accessibility")))]
fn main() {
    panic!("AX modal lifecycle requires macOS and content-accessibility");
}
