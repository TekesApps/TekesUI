use super::{MainThreadMarker, NSApplication, NSPoint, NSSize, NSWindow};

/// #116 retained native objects; no physical IME, window or VoiceOver acceptance.
#[doc(hidden)]
pub fn combo_text_role_contract() -> usize {
    use crate::accessibility::NativeAccessibilityElement;
    use crate::shared::Shared;
    use crate::{
        AccessibilityKey, AccessibilityNode, AccessibilityRole, AccessibilityTextLayout, Event,
        TextInputState, Window, WindowHandler,
    };
    use objc2::rc::Retained;
    use objc2::{msg_send, sel};
    use objc2_foundation::{NSRange, NSString};
    struct Recorder(std::rc::Rc<std::cell::RefCell<Vec<crate::AccessibilityAction>>>);
    impl WindowHandler for Recorder {
        fn handle(&mut self, e: Event, _: &mut Window) {
            if let Event::Accessibility { action, .. } = e {
                self.0.borrow_mut().push(action);
            }
        }
    }
    let mtm = MainThreadMarker::new().expect("main-thread ComboBox probe");
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let shared = Shared::new(mtm, Box::new(Recorder(events.clone())));
    // Real native setter delivery needs the original Window/Shared objects.
    // This window remains unshown and no global input or event loop is used.
    use objc2::MainThreadOnly;
    use objc2_app_kit::{NSApplicationActivationPolicy, NSBackingStoreType, NSWindowStyleMask};
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    let rect = objc2_foundation::NSRect::new(NSPoint::new(0., 0.), NSSize::new(240., 80.));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    let view = crate::view::TekesView::new(mtm, shared.clone(), rect);
    window.setContentView(Some(&view));
    shared
        .objects
        .set(crate::shared::Objects {
            window: window.clone(),
            view,
        })
        .ok()
        .unwrap();
    shared.ready.set(true);
    assert!(!window.isVisible());
    let key = AccessibilityKey {
        component: 116,
        part: None,
    };
    let element = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, element.clone());
    let mut checks = 0;
    for role in [AccessibilityRole::ComboBox] {
        for enabled in [false, true] {
            for read_only in [false, true] {
                let node = AccessibilityNode {
                    key,
                    parent: None,
                    role,
                    frame: crate::Rect {
                        origin: crate::Point::default(),
                        size: crate::Size {
                            width: 200.,
                            height: 40.,
                        },
                    },
                    label: "Query".into(),
                    value: Some("A😀B".into()),
                    description: None,
                    enabled,
                    focused: true,
                    selected: false,
                    expanded: Some(true),
                    invalid: false,
                    required: false,
                    text_selection: Some(1..3),
                    read_only,
                };
                assert!(crate::accessibility::validate_accessibility_nodes(
                    std::slice::from_ref(&node)
                ));
                *shared.accessibility_nodes.borrow_mut() = vec![node];
                let carets = vec![
                    (
                        0,
                        crate::Point { x: 0., y: 0. },
                        crate::Size {
                            width: 1.,
                            height: 20.,
                        },
                    ),
                    (
                        1,
                        crate::Point { x: 8., y: 0. },
                        crate::Size {
                            width: 1.,
                            height: 20.,
                        },
                    ),
                    (
                        3,
                        crate::Point { x: 16., y: 0. },
                        crate::Size {
                            width: 1.,
                            height: 20.,
                        },
                    ),
                    (
                        4,
                        crate::Point { x: 24., y: 0. },
                        crate::Size {
                            width: 1.,
                            height: 20.,
                        },
                    ),
                ];
                let layout = AccessibilityTextLayout {
                    state: TextInputState {
                        text: "A😀B".into(),
                        caret_index: 3,
                        selection: 1..3,
                        marked: None,
                        read_only,
                        caret_origin: crate::Point { x: 16., y: 0. },
                        caret_size: crate::Size {
                            width: 1.,
                            height: 20.,
                        },
                        carets,
                    },
                    lines: std::iter::once(0..4).collect(),
                };
                assert!(layout.is_valid());
                shared
                    .accessibility_text_layouts
                    .borrow_mut()
                    .insert(key, layout);
                let expanded: bool = unsafe { msg_send![&*element, isAccessibilityExpanded] };
                assert!(expanded);
                let allowed: bool = unsafe {
                    msg_send![&*element,isAccessibilitySelectorAllowed:sel!(setAccessibilityExpanded:)]
                };
                assert_eq!(allowed, enabled);
                let disclosed: bool = unsafe {
                    msg_send![&*element,isAccessibilitySelectorAllowed:sel!(setAccessibilityDisclosed:)]
                };
                assert!(!disclosed);
                checks += 3;
                let native_role: Retained<NSString> =
                    unsafe { msg_send![&*element, accessibilityRole] };
                let expected = "AXComboBox";
                assert_eq!(native_role.to_string(), expected);
                let subrole: Retained<NSString> =
                    unsafe { msg_send![&*element, accessibilitySubrole] };
                assert_eq!(subrole.to_string(), "");
                assert_eq!(&*native_role, unsafe {
                    objc2_app_kit::NSAccessibilityComboBoxRole
                });
                for selector in [
                    sel!(setAccessibilityValue:),
                    sel!(setAccessibilitySelectedText:),
                ] {
                    let allowed: bool =
                        unsafe { msg_send![&*element,isAccessibilitySelectorAllowed:selector] };
                    assert_eq!(allowed, enabled && !read_only, "{role:?} {selector:?}");
                    checks += 1;
                }
                for selector in [
                    sel!(setAccessibilitySelectedTextRange:),
                    sel!(setAccessibilitySelectedTextRanges:),
                    sel!(setAccessibilityFocused:),
                ] {
                    let allowed: bool =
                        unsafe { msg_send![&*element,isAccessibilitySelectorAllowed:selector] };
                    assert_eq!(allowed, enabled, "{role:?} {selector:?}");
                    checks += 1;
                }
                for selector in [
                    sel!(accessibilityValue),
                    sel!(accessibilitySelectedText),
                    sel!(accessibilitySelectedTextRange),
                    sel!(accessibilitySelectedTextRanges),
                    sel!(accessibilityNumberOfCharacters),
                    sel!(accessibilityStringForRange:),
                    sel!(accessibilityAttributedStringForRange:),
                    sel!(accessibilityVisibleCharacterRange),
                    sel!(accessibilityRangeForIndex:),
                    sel!(accessibilityRangeForPosition:),
                    sel!(accessibilityLineForIndex:),
                    sel!(accessibilityRangeForLine:),
                    sel!(accessibilityInsertionPointLineNumber),
                    sel!(accessibilityFrameForRange:),
                ] {
                    let allowed: bool =
                        unsafe { msg_send![&*element,isAccessibilitySelectorAllowed:selector] };
                    assert!(allowed, "{role:?} {selector:?}");
                    checks += 1;
                }
                let value: Retained<NSString> = unsafe { msg_send![&*element, accessibilityValue] };
                assert_eq!(value.to_string(), "A😀B");
                let count: isize = unsafe { msg_send![&*element, accessibilityNumberOfCharacters] };
                assert_eq!(count, 4);
                let selection: NSRange =
                    unsafe { msg_send![&*element, accessibilitySelectedTextRange] };
                assert_eq!(selection, NSRange::new(1, 2));
                let selected: Retained<NSString> =
                    unsafe { msg_send![&*element, accessibilitySelectedText] };
                assert_eq!(selected.to_string(), "😀");
                let string: Retained<NSString> =
                    unsafe { msg_send![&*element,accessibilityStringForRange:NSRange::new(1,2)] };
                assert_eq!(string.to_string(), "😀");
                let split: Option<Retained<NSString>> =
                    unsafe { msg_send![&*element,accessibilityStringForRange:NSRange::new(1,1)] };
                assert!(split.is_none());
                let line: isize = unsafe { msg_send![&*element,accessibilityLineForIndex:3isize] };
                assert_eq!(line, 0);
                let range: NSRange =
                    unsafe { msg_send![&*element,accessibilityRangeForLine:0isize] };
                assert_eq!(range, NSRange::new(0, 4));
                let visible: NSRange =
                    unsafe { msg_send![&*element, accessibilityVisibleCharacterRange] };
                assert_eq!(visible, NSRange::new(0, 4));
                events.borrow_mut().clear();
                let new_value = NSString::from_str("write");
                unsafe {
                    let _: () = msg_send![&*element,setAccessibilityValue:&*new_value];
                    let _: () = msg_send![&*element,setAccessibilitySelectedText:&*new_value];
                    let _: () =
                        msg_send![&*element,setAccessibilitySelectedTextRange:NSRange::new(1,2)];
                    let _: () = msg_send![&*element,setAccessibilityFocused:true];
                    let _: () = msg_send![&*element,setAccessibilityExpanded:false];
                }
                let mut expected = Vec::new();
                if enabled {
                    if !read_only {
                        expected.push(crate::AccessibilityAction::SetValue("write".into()));
                        expected.push(crate::AccessibilityAction::ReplaceSelectedText(
                            "write".into(),
                        ));
                    }
                    expected.push(crate::AccessibilityAction::SetSelection(1..3));
                    expected.push(crate::AccessibilityAction::Focus);
                    expected.push(crate::AccessibilityAction::SetExpanded(false));
                }
                assert_eq!(*events.borrow(), expected);
                checks += 14;
            }
        }
    }
    // Retained native elements lose access when the original cache binding changes.
    let replacement = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, replacement);
    let stale: Option<Retained<NSString>> =
        unsafe { msg_send![&*element,accessibilityStringForRange:NSRange::new(0,1)] };
    assert!(stale.is_none());
    events.borrow_mut().clear();
    unsafe {
        let _: () = msg_send![&*element,setAccessibilitySelectedTextRange:NSRange::new(0,1)];
    }
    assert!(events.borrow().is_empty());
    shared.closed.set(true);
    window.close();
    checks + 2
}
