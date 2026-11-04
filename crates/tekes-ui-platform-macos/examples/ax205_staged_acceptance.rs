//! Opt-in native selector probe. Opens one real window; no physical input or GPU submission.
//! Run only in an allocated native window. This is not a VoiceOver speech test.
#[cfg(all(target_os = "macos", feature = "content-accessibility"))]
#[path = "../tests/support/ax_modal_fixture.rs"]
mod fixture;

#[cfg(all(target_os = "macos", feature = "content-accessibility"))]
mod native {
    use super::fixture::*;
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject, sel};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{MainThreadMarker, NSArray, NSNumber, NSString};
    use std::{
        cell::RefCell,
        ffi::c_void,
        path::PathBuf,
        rc::Rc,
        time::{Duration, Instant},
    };
    use tekes_ui_controls::composition::{ContentHost, DialogContent, SwitchContent};
    use tekes_ui_platform_macos::{
        AccessibilityAction, App, AppConfig, Event, MenuBar, Size, Window, WindowConfig,
        WindowHandler,
        content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot,
            WindowAccessibilityPublication, content_key,
        },
    };
    const TITLE: &str = "AX external notification probe #205";
    const LAYOUT_IDENTIFIER: &str = "TekesUI.AX205.actual-layout-ancestor";
    fn channel() -> PathBuf {
        PathBuf::from(std::env::var("TEKESUI_AX205_CHANNEL").expect("allocated harness channel"))
    }
    fn marker(name: &str) {
        let path = channel().join(name);
        assert!(!path.exists(), "duplicate phase");
        std::fs::write(path, format!("pid={}\n", std::process::id())).unwrap();
        println!("AX205_PHASE={name}");
    }
    unsafe extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }
    extern "C" fn target_ready(_: *mut c_void) {
        marker("TARGET_READY");
    }
    extern "C" fn finish(_: *mut c_void) {
        marker("FINISHING");
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).terminate(None);
    }
    fn queue(work: extern "C" fn(*mut c_void)) {
        // No UI objects cross threads. libdispatch invokes the callback on main.
        unsafe {
            dispatch_async_f(&raw const _dispatch_main_q, std::ptr::null_mut(), work);
        }
    }
    fn wait_file(name: &str) {
        let deadline = Instant::now() + Duration::from_secs(40);
        while !channel().join(name).exists() {
            assert!(Instant::now() < deadline, "harness command timeout: {name}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

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
    fn value(node: &AnyObject) -> i32 {
        let value: Retained<NSNumber> = unsafe { msg_send![node, accessibilityValue] };
        value.intValue()
    }

    #[derive(Debug, PartialEq, Eq)]
    struct HostWitness {
        actions: usize,
        choice_checked: bool,
        dialog_open: bool,
        invoker_focused: bool,
    }
    fn host_witness(saved: &Saved, host: &ContentHost) -> HostWitness {
        HostWitness {
            actions: saved.actions,
            choice_checked: host
                .get(CHOICE)
                .unwrap()
                .component::<SwitchContent>()
                .unwrap()
                .model
                .checked,
            dialog_open: host
                .get(DIALOG)
                .unwrap()
                .component::<DialogContent>()
                .unwrap()
                .is_open(),
            invoker_focused: host.focused() == Some(INVOKER),
        }
    }
    fn write_witness(name: &str, witness: &HostWitness) {
        std::fs::write(channel().join(name), format!(
            "{{\"pid\":{},\"actions\":{},\"choice_checked\":{},\"dialog_open\":{},\"invoker_focused\":{}}}\n",
            std::process::id(), witness.actions, witness.choice_checked, witness.dialog_open, witness.invoker_focused
        )).unwrap();
    }
    thread_local! {
        static PROBE_SAVED: RefCell<Option<Rc<RefCell<Saved>>>> = const { RefCell::new(None) };
    }
    extern "C" fn stale_check(_: *mut c_void) {
        PROBE_SAVED.with(|slot| {
            let slot = slot.borrow();
            let mut saved = slot.as_ref().unwrap().borrow_mut();
            let host = saved.host.as_ref().unwrap().borrow();
            let after = host_witness(&saved, &host);
            assert_eq!(saved.before_close.as_ref().unwrap(), &after);
            assert!(saved.completed);
            assert!(!saved.publication.as_ref().unwrap().is_current());
            let view = saved.view.as_ref().unwrap();
            assert!(children(view).is_empty());
            assert!(std::ptr::eq(&*focused(view), &**view));
            let invoker = saved.element.as_ref().unwrap();
            let live: bool = unsafe { msg_send![&**invoker, isAccessibilityElement] };
            let accepted: bool = unsafe { msg_send![&**invoker, accessibilityPerformPress] };
            assert!(!live && !accepted);
            write_witness("POST_STALE_HOST.json", &after);
            drop(host);
            saved.stale_checked = true;
        });
        marker("STALE_CHECKED_READY");
    }
    #[derive(Default)]
    struct Saved {
        view: Option<Retained<AnyObject>>,
        element: Option<Retained<AnyObject>>,
        publication: Option<WindowAccessibilityPublication>,
        actions: usize,
        completed: bool,
        host: Option<Rc<RefCell<ContentHost>>>,
        before_close: Option<HostWitness>,
        stale_checked: bool,
        layout_target: Option<Retained<AnyObject>>,
    }
    enum EvidencePhase {
        Modal,
        Value,
        Dismiss,
    }
    struct Handler {
        host: Rc<RefCell<ContentHost>>,
        saved: Rc<RefCell<Saved>>,
        pending_evidence: Option<EvidencePhase>,
    }
    impl Handler {
        fn publish_snapshot(&mut self, window: &Window) {
            let mut host = self.host.borrow_mut();
            paint(&mut host);
            assert!(window.set_content_accessibility_snapshot(
                ContentAccessibilitySnapshot::from_host(&host).unwrap()
            ));
            drop(host);
        }
        fn publish(&mut self, window: &Window, phase: &str) -> Retained<AnyObject> {
            self.publish_snapshot(window);
            self.inspect_publication(phase)
        }
        fn inspect_publication(&mut self, phase: &str) -> Retained<AnyObject> {
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
            let view: Retained<AnyObject> = window.contentView().unwrap().into();
            self.record_layout_source(&view, &window, phase);
            view
        }
        fn record_layout_source(&mut self, view: &AnyObject, window: &AnyObject, phase: &str) {
            let view_before: bool = unsafe { msg_send![view, isAccessibilityElement] };
            let ancestor = unsafe { objc2_app_kit::NSAccessibilityUnignoredAncestor(view) }
                .expect("actual unignored ancestor; no fallback source");
            let ancestor_before: bool = unsafe { msg_send![&*ancestor, isAccessibilityElement] };
            let same_as_view = std::ptr::eq(view, &*ancestor);
            let same_as_window = std::ptr::eq(window, &*ancestor);
            let binding_mode = if same_as_view {
                "view"
            } else if same_as_window {
                "window"
            } else {
                "identifier"
            };
            let first = self.saved.borrow().layout_target.is_none();
            let original_identifier: Option<Retained<NSString>> =
                unsafe { msg_send![&*ancestor, accessibilityIdentifier] };
            let role_before: Option<Retained<NSString>> =
                unsafe { msg_send![&*ancestor, accessibilityRole] };
            let parent_before: Option<Retained<AnyObject>> =
                unsafe { msg_send![&*ancestor, accessibilityParent] };
            let tag_applied = first && binding_mode == "identifier";
            if tag_applied {
                assert!(
                    original_identifier
                        .as_ref()
                        .is_none_or(|x| x.to_string().is_empty()),
                    "refuse to overwrite a nonempty original ancestor identifier"
                );
                let supports: bool = unsafe {
                    msg_send![&*ancestor, respondsToSelector: sel!(setAccessibilityIdentifier:)]
                };
                assert!(
                    supports,
                    "actual other ancestor must expose identifier metadata"
                );
                unsafe {
                    let _: () = msg_send![&*ancestor, setAccessibilityIdentifier: &*NSString::from_str(LAYOUT_IDENTIFIER)];
                }
            }
            if first {
                self.saved.borrow_mut().layout_target = Some(ancestor.clone());
            } else {
                assert!(
                    std::ptr::eq(
                        &**self.saved.borrow().layout_target.as_ref().unwrap(),
                        &*ancestor
                    ),
                    "layout ancestor changed; do not rebind from observed events"
                );
            }
            let view_after: bool = unsafe { msg_send![view, isAccessibilityElement] };
            let ancestor_after: bool = unsafe { msg_send![&*ancestor, isAccessibilityElement] };
            let again = unsafe { objc2_app_kit::NSAccessibilityUnignoredAncestor(view) }.unwrap();
            assert_eq!(
                view_before, view_after,
                "metadata must not change participation"
            );
            assert_eq!(
                ancestor_before, ancestor_after,
                "metadata must not change participation"
            );
            assert!(
                std::ptr::eq(&*ancestor, &*again),
                "metadata must not change routing target"
            );
            let identifier: Option<Retained<NSString>> =
                unsafe { msg_send![&*ancestor, accessibilityIdentifier] };
            if binding_mode == "identifier" {
                assert_eq!(
                    identifier.as_ref().map(|x| x.to_string()).as_deref(),
                    Some(LAYOUT_IDENTIFIER)
                );
            } else {
                assert_eq!(
                    original_identifier.as_ref().map(|x| x.to_string()),
                    identifier.as_ref().map(|x| x.to_string())
                );
            }
            let role_after: Option<Retained<NSString>> =
                unsafe { msg_send![&*ancestor, accessibilityRole] };
            let parent_after: Option<Retained<AnyObject>> =
                unsafe { msg_send![&*ancestor, accessibilityParent] };
            assert_eq!(
                role_before.as_ref().map(|x| x.to_string()),
                role_after.as_ref().map(|x| x.to_string()),
                "identifier must not change role"
            );
            let same_parent = match (&parent_before, &parent_after) {
                (None, None) => true,
                (Some(a), Some(b)) => std::ptr::eq(&**a, &**b),
                _ => false,
            };
            assert!(same_parent, "identifier must not change parent identity");
            let record = serde_json::json!({
                "pid": std::process::id(), "phase": phase, "binding_mode": binding_mode,
                "native_source": "NSAccessibilityUnignoredAncestor(view)", "window_title": TITLE,
                "original_identifier": original_identifier.map(|x| x.to_string()),
                "native_identifier": identifier.map(|x| x.to_string()),
                "view_pointer": format!("{view:p}"), "ancestor_pointer": format!("{:p}", &*ancestor),
                "ancestor_same_as_view": same_as_view, "ancestor_same_as_window": same_as_window,
                "view_is_element_before": view_before, "view_is_element_after": view_after,
                "ancestor_is_element_before": ancestor_before, "ancestor_is_element_after": ancestor_after,
                "tag_applied": tag_applied,
                "ancestor_role_before": role_before.map(|x| x.to_string()),
                "ancestor_role_after": role_after.map(|x| x.to_string()),
                "ancestor_parent_before": parent_before.map(|x| format!("{:p}", &*x)),
                "ancestor_parent_after": parent_after.map(|x| format!("{:p}", &*x))
            });
            std::fs::write(
                channel().join(format!("LAYOUT_TARGET-{phase}.json")),
                serde_json::to_vec_pretty(&record).unwrap(),
            )
            .unwrap();
        }
    }
    impl WindowHandler for Handler {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::Created => {
                    assert!(self.host.borrow_mut().focus(Some(INVOKER), true));
                    let view = self.publish(window, "initial");
                    assert!(named(&view, "Modal scope").is_none());
                    let _invoker = named(&view, "Open modal").expect("initial native invoker");
                    queue(target_ready); // After Created returns, main remains responsive.
                    let wake = window.wake_proxy();
                    std::thread::spawn(move || {
                        wait_file("CLOSE_REQUEST");
                        let _ = wake.wake();
                        wait_file("STALE_CHECK_REQUEST");
                        queue(stale_check);
                        wait_file("FINISH_REQUEST");
                        queue(finish);
                    });
                }
                Event::Accessibility { target, action } => {
                    assert!(
                        self.pending_evidence.is_none(),
                        "previous phase evidence must complete"
                    );
                    assert!(channel().join("OBSERVERS_READY").is_file());
                    assert!(matches!(action, AccessibilityAction::Press));
                    self.saved.borrow_mut().actions += 1;
                    let publication = window.content_accessibility_publication().unwrap();
                    let response = publication.dispatch(
                        ContentAccessibilityOwner(0),
                        &mut self.host.borrow_mut(),
                        target,
                        action,
                    );
                    assert!(response.consumed);
                    if Some(target) == content_key(&self.host.borrow(), INVOKER, None) {
                        assert!(response.activated);
                        self.host.borrow_mut().show_dialog(DIALOG).unwrap();
                        self.publish_snapshot(window);
                        self.pending_evidence = Some(EvidencePhase::Modal);
                    } else if Some(target) == content_key(&self.host.borrow(), CHOICE, None) {
                        assert!(response.changed);
                        assert!(
                            self.host
                                .borrow()
                                .get(CHOICE)
                                .unwrap()
                                .component::<SwitchContent>()
                                .unwrap()
                                .model
                                .checked
                        );
                        self.publish_snapshot(window);
                        self.pending_evidence = Some(EvidencePhase::Value);
                    } else {
                        assert_eq!(Some(target), content_key(&self.host.borrow(), CANCEL, None));
                        assert!(response.activated);
                        assert!(self.host.borrow_mut().close_dialog());
                        self.publish_snapshot(window);
                        self.pending_evidence = Some(EvidencePhase::Dismiss);
                    }
                    // Queue native evidence only after this synchronous AX action returns.
                    // The observer still waits for the original phase marker before its next step.
                    assert!(matches!(
                        window.wake_proxy().wake(),
                        tekes_ui_platform_macos::WakeResult::Queued
                            | tekes_ui_platform_macos::WakeResult::Coalesced
                    ));
                }
                Event::Wake => {
                    if let Some(phase) = self.pending_evidence.take() {
                        match phase {
                            EvidencePhase::Modal => {
                                let view = self.inspect_publication("modal");
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
                                marker("MODAL_READY");
                            }
                            EvidencePhase::Value => {
                                let view = self.inspect_publication("value");
                                assert_eq!(value(&named(&view, "Modal choice").unwrap()), 1);
                                marker("VALUE_READY");
                            }
                            EvidencePhase::Dismiss => {
                                let view = self.inspect_publication("dismiss");
                                assert!(named(&view, "Modal scope").is_none());
                                let invoker = named(&view, "Open modal").unwrap();
                                assert!(std::ptr::eq(&*focused(&view), &*invoker));
                                let publication =
                                    window.content_accessibility_publication().unwrap();
                                self.saved.borrow_mut().view = Some(view.clone());
                                self.saved.borrow_mut().element = Some(invoker.clone());
                                self.saved.borrow_mut().publication = Some(publication.clone());
                                marker("DISMISSED_READY");
                            }
                        }
                        return;
                    }
                    assert!(channel().join("CLOSE_REQUEST").is_file());
                    assert_eq!(self.saved.borrow().actions, 3);
                    let view = self.saved.borrow().view.as_ref().unwrap().clone();
                    let invoker = self.saved.borrow().element.as_ref().unwrap().clone();
                    let publication = self.saved.borrow().publication.as_ref().unwrap().clone();
                    let before = host_witness(&self.saved.borrow(), &self.host.borrow());
                    assert_eq!(before.actions, 3);
                    assert!(before.choice_checked && !before.dialog_open && before.invoker_focused);
                    write_witness("PRE_CLOSE_HOST.json", &before);
                    self.saved.borrow_mut().before_close = Some(before);
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
                    let accepted: bool = unsafe { msg_send![&*invoker, accessibilityPerformPress] };
                    assert!(!live && !accepted);
                    self.saved.borrow_mut().completed = true;
                    marker("CLOSED_READY");
                }
                _ => {}
            }
        }
    }
    pub fn run() {
        let saved = Rc::new(RefCell::new(Saved::default()));
        let host = Rc::new(RefCell::new(host()));
        saved.borrow_mut().host = Some(host.clone());
        PROBE_SAVED.with(|slot| *slot.borrow_mut() = Some(saved.clone()));
        App::new(AppConfig {
            name: Some("AX205 staged acceptance".into()),
            quit_after_last_window_closed: false,
        })
        .run(
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
                host: host.clone(),
                saved: saved.clone(),
                pending_evidence: None,
            },
        );
        // Retained native clients stay inert after App.run's Shared::finish too.
        let saved = saved.borrow();
        assert!(saved.completed && saved.stale_checked);
        assert_eq!(saved.actions, 3);
        let view = saved.view.as_ref().unwrap();
        assert!(children(view).is_empty());
        assert!(std::ptr::eq(&*focused(view), &**view));
        let element = saved.element.as_ref().unwrap();
        let live: bool = unsafe { msg_send![&**element, isAccessibilityElement] };
        assert!(!live);
        assert!(!saved.publication.as_ref().unwrap().is_current());
        println!(
            "AX205_TARGET_V6_OK windows=1 external_presses=3 physical_input=0 gpu_submissions=0 voiceover_test=false"
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
