//! 自测脚本支持，仅供示例和 Gallery 的自测模式使用，不属于边界 API。
//!
//! 后台线程按时间表向主队列投递步骤，步骤在主线程里直接操作 AppKit，
//! 与 osascript / 真实输入走同一批 AppKit 入口，不合成全局鼠标事件。
//! 只在自测模式下使用：空闲行为不受影响。

use std::ffi::c_void;
use std::time::Duration;

use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSApplication, NSEvent,
    NSEventModifierFlags, NSEventType, NSWindow,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSSize, NSString};

unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, ctx: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// 一个自测步骤。
#[derive(Clone, Debug)]
pub enum Action {
    /// 改内容区尺寸，单位 pt。
    Resize {
        w: f64,
        h: f64,
    },
    /// Synthetic public AppKit live-resize callbacks; not a physical drag.
    BeginLiveResize,
    EndLiveResize,
    /// Enable presented receipts and reset the interval baseline.
    FrameMark(&'static str),
    /// Require real presented frames since the last mark, then reset it.
    FrameExpect {
        label: &'static str,
        min: u64,
        max: u64,
    },
    Miniaturize,
    Deminiaturize,
    /// 用 `screencapture -o -l <窗口号> -x <path>` 截取窗口（设备像素，不含阴影）。
    /// 截图在单独的线程里做，不阻塞主线程。
    Capture {
        path: String,
    },
    /// 只打一行日志，用来在日志里标记阶段。
    Log(String),
    /// Wake a benchmark's receipt poll independently of display-link callbacks.
    WakeRedraw,
    /// `performClose`：走 `CloseRequested` 路径。
    Close,
    /// 触发菜单项（`tag` 是 `CommandId` 的值）：与点菜单走同一个 `sendAction`，
    /// 菜单里没有这个命令时只打日志。
    MenuCommand(u32),
    /// 合成一个窗口内的鼠标移动（不带按键），坐标是内容区的 pt，左上角为原点。
    /// 事件直接送给内容视图的 `mouseMoved:`，和 NSTrackingArea 的路径一致；不产生全局鼠标事件。
    MouseMove {
        x: f64,
        y: f64,
    },
    /// 合成鼠标移出内容区（`mouseExited:`）。
    MouseExit,
    /// Text-service bridge smoke tests; preedit is synthetic, not a physical IME session.
    Text(String),
    Preedit(String),
    /// Exact UTF-16 ranges through NSTextInputClient, not a physical IME.
    TextRange {
        text: String,
        replacement: Option<std::ops::Range<usize>>,
    },
    PreeditRange {
        text: String,
        selection: std::ops::Range<usize>,
        replacement: Option<std::ops::Range<usize>>,
    },
    Unmark,
    ExpectTextState {
        text: String,
        selection: std::ops::Range<usize>,
        marked: Option<std::ops::Range<usize>>,
        query: std::ops::Range<usize>,
        actual: std::ops::Range<usize>,
    },
    ExpectNoText,
    ExpectTextRect {
        range: std::ops::Range<usize>,
        actual: std::ops::Range<usize>,
        nonzero_width: bool,
    },
    /// Invoke the real NSAccessibility selectors on a published native element.
    AXSelect {
        component: u64,
        range: std::ops::Range<usize>,
    },
    /// Invoke the published element's actual accessibilityPerformPress selector.
    AXPress {
        component: u64,
    },
    /// Invoke the native selector for a full projected key, including remote parts.
    AXPressKey {
        key: crate::AccessibilityKey,
    },
    AXReplace {
        component: u64,
        text: String,
    },
    ExpectAXText {
        component: u64,
        text: String,
        selection: std::ops::Range<usize>,
        selected: String,
        query: std::ops::Range<usize>,
        substring: Option<String>,
    },
    /// Read AppKit's advertised legacy AX attributes for a real published owner.
    /// This audits protocol reachability, not VoiceOver speech or interaction.
    AuditAXText(u64),
    ExpectAXMissing(u64),
    TextCommand(&'static str),
    ExpectText {
        text: String,
        marked: bool,
    },
    TypeAscii(String),
    /// 合成一次左键单击（按下加抬起），经 `NSApplication.sendEvent`。
    Click {
        x: f64,
        y: f64,
    },
    /// 合成一次按键（按下加抬起），无修饰键。`key_code` 是虚拟键码（↑ 126、↓ 125）。
    Key {
        key_code: u16,
    },
    /// Send a native key with its real character value through the input context.
    KeyCharacters {
        key_code: u16,
        chars: &'static str,
    },
    /// Send a line-based scroll-wheel NSEvent to this window at content coordinates.
    ScrollWheel {
        x: f64,
        y: f64,
        delta_x: i32,
        delta_y: i32,
    },
    /// 只改这一个窗口的外观（深色或跟随系统），不碰系统设置；
    /// 内容视图的 `effectiveAppearance` 随之变化，平台层会投递 `SystemAppearanceChanged`。
    WindowDark(bool),
    /// 合成一个本地 keyDown（不是全局 CGEvent），交给 `NSApp.mainMenu.performKeyEquivalent:`，
    /// 走菜单快捷键匹配；日志里打印是否被菜单接住。这条路径不经过系统全局热键，
    /// 所以只能证明菜单项配置正确，不能证明系统没有拦截。
    KeyEquivalent {
        chars: &'static str,
        key_code: u16,
        control: bool,
        alt: bool,
        shift: bool,
        command: bool,
    },
}

/// 内容区坐标（左上角原点）→ 窗口坐标（左下角原点）。
fn win_point(w: &NSWindow, x: f64, y_top: f64) -> NSPoint {
    let h = w
        .contentView()
        .map(|v| v.frame().size.height)
        .unwrap_or(0.0);
    NSPoint::new(x, h - y_top)
}

fn mouse_event(
    w: &NSWindow,
    ty: NSEventType,
    x: f64,
    y: f64,
    count: isize,
) -> Option<objc2::rc::Retained<NSEvent>> {
    NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        ty,
        win_point(w, x, y),
        NSEventModifierFlags::empty(),
        0.0,
        w.windowNumber(),
        None,
        0,
        count,
        1.0,
    )
}

fn key_event(
    w: &NSWindow,
    ty: NSEventType,
    key_code: u16,
    chars: &str,
) -> Option<objc2::rc::Retained<NSEvent>> {
    let s = NSString::from_str(chars);
    NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        ty,
        NSPoint::new(0.0, 0.0),
        NSEventModifierFlags::empty(),
        0.0,
        w.windowNumber(),
        None,
        &s,
        &s,
        false,
        key_code,
    )
}

thread_local! { static FRAME_MARK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

extern "C" fn run_step(ctx: *mut c_void) {
    // SAFETY: ctx 是 run_script 里 Box::into_raw 出来的 Action，每个只消费一次。
    let action = *unsafe { Box::from_raw(ctx.cast::<Action>()) };
    let mtm = MainThreadMarker::new().expect("主队列上必是主线程");
    let app = NSApplication::sharedApplication(mtm);
    let Some(w) = app.windows().firstObject() else {
        eprintln!("selftest: 没有窗口，跳过 {action:?}");
        return;
    };
    eprintln!("selftest: {action:?}");
    match action {
        Action::AXSelect { .. }
        | Action::AXReplace { .. }
        | Action::AXPress { .. }
        | Action::AXPressKey { .. }
        | Action::ExpectAXText { .. }
        | Action::AuditAXText(_)
        | Action::ExpectAXMissing(_) => {
            use objc2::{DefinedClass, msg_send};
            use objc2_foundation::NSRange;
            let content = w.contentView().expect("selftest content");
            let view = content
                .downcast_ref::<crate::view::TekesView>()
                .expect("TekesView");
            let key = match &action {
                Action::AXPressKey { key } => *key,
                Action::AXSelect { component, .. }
                | Action::AXReplace { component, .. }
                | Action::AXPress { component }
                | Action::ExpectAXText { component, .. }
                | Action::AuditAXText(component)
                | Action::ExpectAXMissing(component) => crate::AccessibilityKey {
                    component: *component,
                    part: None,
                },
                _ => unreachable!(),
            };
            let component = key.component;
            let element = view
                .ivars()
                .shared
                .accessibility_elements
                .borrow()
                .get(&key)
                .cloned();
            if matches!(action, Action::ExpectAXMissing(_)) {
                assert!(element.is_none(), "closed AX element remains published");
                eprintln!("SELFTEST PASS AX closed owner removed");
                return;
            }
            let element = element.unwrap_or_else(|| {
                let published: Vec<_> = view
                    .ivars()
                    .shared
                    .accessibility_nodes
                    .borrow()
                    .iter()
                    .map(|n| (n.key, n.label.clone(), n.frame))
                    .collect();
                panic!("native accessibility element {key:?} missing; published={published:?}");
            });
            match action {
                Action::AXPress { component }
                | Action::AXPressKey {
                    key: crate::AccessibilityKey { component, .. },
                } => unsafe {
                    use objc2::{runtime::Bool, sel};
                    let allowed: Bool = msg_send![&*element, isAccessibilitySelectorAllowed:sel!(accessibilityPerformPress)];
                    assert!(
                        allowed.as_bool(),
                        "AXPress is not allowed for component {component}"
                    );
                    let pressed: bool = msg_send![&*element, accessibilityPerformPress];
                    assert!(
                        pressed,
                        "AXPress was not accepted for component {component}"
                    );
                    eprintln!(
                        "SELFTEST PASS AXPress component={component} part={:?}",
                        key.part
                    );
                },
                Action::AuditAXText(_) => unsafe {
                    use objc2::{rc::Retained, sel};
                    use objc2_foundation::{NSArray, NSAttributedString, NSValue};
                    let role: Retained<NSString> = msg_send![&*element, accessibilityRole];
                    let selected: NSRange = msg_send![&*element, accessibilitySelectedTextRange];
                    let selections: Option<Retained<NSArray<NSValue>>> =
                        msg_send![&*element, accessibilitySelectedTextRanges];
                    let selections = selections.expect("text selection array");
                    assert_eq!(selections.count(), 1);
                    assert_eq!(selections.firstObject().unwrap().rangeValue(), selected);
                    let visible: NSRange = msg_send![&*element, accessibilityVisibleCharacterRange];
                    assert_ne!(visible.location, isize::MAX as usize);
                    let first: NSRange = msg_send![&*element, accessibilityRangeForLine:0isize];
                    assert_eq!(first.location, 0);
                    let character: NSRange =
                        msg_send![&*element, accessibilityRangeForIndex:0isize];
                    assert_eq!(character.location, 0);
                    let fragment: Option<Retained<NSAttributedString>> =
                        msg_send![&*element, accessibilityAttributedStringForRange:character];
                    assert!(fragment.is_some());
                    let frame: objc2_foundation::NSRect =
                        msg_send![&*element, accessibilityFrameForRange:character];
                    assert!(frame.size.height > 0.);
                    for selector in [
                        sel!(accessibilityRangeForIndex:),
                        sel!(accessibilityRangeForPosition:),
                        sel!(accessibilityVisibleCharacterRange),
                        sel!(accessibilitySelectedTextRanges),
                        sel!(setAccessibilitySelectedTextRanges:),
                        sel!(setAccessibilityFocused:),
                    ] {
                        let allowed: objc2::runtime::Bool =
                            msg_send![&*element, isAccessibilitySelectorAllowed:selector];
                        assert!(allowed.as_bool(), "text selector blocked: {selector:?}");
                    }
                    let ax_window: Option<Retained<objc2::runtime::AnyObject>> =
                        msg_send![&*element, accessibilityWindow];
                    let ax_top: Option<Retained<objc2::runtime::AnyObject>> =
                        msg_send![&*element, accessibilityTopLevelUIElement];
                    let expected = (&*w as *const NSWindow).cast::<objc2::runtime::AnyObject>();
                    let window_matches = ax_window
                        .as_ref()
                        .is_some_and(|v| std::ptr::eq(&**v, expected));
                    let top_matches = ax_top
                        .as_ref()
                        .is_some_and(|v| std::ptr::eq(&**v, expected));
                    eprintln!(
                        "AX_WINDOW_RELATION component={component} window_matches={window_matches} top_matches={top_matches}"
                    );
                    let focused: bool = msg_send![&*element, isAccessibilityFocused];
                    eprintln!(
                        "AX_MODERN_PASS component={component} role={role} focused={focused} selected={selected:?} visible={visible:?} first={first:?} character={character:?} frame={frame:?}; not VoiceOver speech acceptance"
                    );
                },
                Action::AXSelect { range, .. } => unsafe {
                    let _: () = msg_send![&*element, setAccessibilitySelectedTextRange: NSRange::new(range.start, range.end - range.start)];
                },
                Action::AXReplace { text, .. } => unsafe {
                    let _: () = msg_send![&*element, setAccessibilitySelectedText: &*NSString::from_str(&text)];
                },
                Action::ExpectAXText {
                    text,
                    selection,
                    selected,
                    query,
                    substring,
                    ..
                } => unsafe {
                    let count: isize = msg_send![&*element, accessibilityNumberOfCharacters];
                    let actual: NSRange = msg_send![&*element, accessibilitySelectedTextRange];
                    let value: objc2::rc::Retained<NSString> =
                        msg_send![&*element, accessibilityValue];
                    let selected_text: Option<objc2::rc::Retained<NSString>> =
                        msg_send![&*element, accessibilitySelectedText];
                    let sub: Option<objc2::rc::Retained<NSString>> = msg_send![&*element, accessibilityStringForRange: NSRange::new(query.start, query.end - query.start)];
                    assert_eq!(count as usize, text.encode_utf16().count());
                    assert_eq!(value.to_string(), text);
                    assert_eq!(
                        actual,
                        NSRange::new(selection.start, selection.end - selection.start)
                    );
                    assert_eq!(selected_text.map(|s| s.to_string()), Some(selected));
                    assert_eq!(sub.map(|s| s.to_string()), substring);
                    eprintln!(
                        "SELFTEST PASS AX text={text:?} selection={selection:?} query={query:?}"
                    );
                },
                _ => unreachable!(),
            }
        }
        Action::Resize { w: nw, h: nh } => w.setContentSize(NSSize::new(nw, nh)),
        Action::BeginLiveResize => w
            .contentView()
            .expect("selftest view")
            .viewWillStartLiveResize(),
        Action::EndLiveResize => w
            .contentView()
            .expect("selftest view")
            .viewDidEndLiveResize(),
        Action::FrameMark(label) => {
            let view = w.contentView().expect("selftest view");
            let view = view
                .downcast_ref::<crate::view::TekesView>()
                .expect("TekesView");
            let count = view.selftest_presented_count();
            FRAME_MARK.with(|mark| mark.set(count));
            eprintln!("frame-mark {label}: presented={count}");
        }
        Action::FrameExpect { label, min, max } => {
            let view = w.contentView().expect("selftest view");
            let view = view
                .downcast_ref::<crate::view::TekesView>()
                .expect("TekesView");
            let count = view.selftest_presented_count();
            let delta = FRAME_MARK.with(|mark| count.saturating_sub(mark.replace(count)));
            eprintln!("frame-expect {label}: presented_delta={delta} required={min}..={max}");
            if !(min..=max).contains(&delta) {
                std::process::exit(2);
            }
        }
        Action::Miniaturize => w.miniaturize(None),
        Action::Deminiaturize => w.deminiaturize(None),
        Action::Log(_) => {}
        Action::WakeRedraw => {
            if let Some(view) = w.contentView() {
                view.updateLayer();
            }
        }
        Action::TypeAscii(text) => {
            for c in text.chars() {
                let string = NSString::from_str(&c.to_string());
                if let Some(e)=NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(NSEventType::KeyDown,NSPoint::ZERO,NSEventModifierFlags::empty(),0.0,w.windowNumber(),None,&string,&string,false,0) {app.sendEvent(&e);}
            }
        }
        Action::Text(_)
        | Action::TextRange { .. }
        | Action::PreeditRange { .. }
        | Action::Unmark
        | Action::ExpectTextState { .. }
        | Action::ExpectNoText
        | Action::ExpectTextRect { .. }
        | Action::Preedit(_)
        | Action::TextCommand(_)
        | Action::ExpectText { .. } => {
            use objc2_app_kit::NSTextInputClient;
            use objc2_foundation::NSRange;
            let view = w.contentView().expect("selftest content view");
            let view = view
                .downcast_ref::<crate::view::TekesView>()
                .expect("TekesView");
            match action {
                Action::ExpectTextRect {
                    range,
                    actual,
                    nonzero_width,
                } => {
                    let mut covered = NSRange::new(isize::MAX as usize, 0);
                    let rect = unsafe {
                        view.firstRectForCharacterRange_actualRange(
                            NSRange::new(range.start, range.end - range.start),
                            &mut covered,
                        )
                    };
                    assert_eq!(
                        covered,
                        NSRange::new(actual.start, actual.end - actual.start)
                    );
                    assert!(
                        rect.origin.x.is_finite()
                            && rect.origin.y.is_finite()
                            && rect.size.height > 0.
                    );
                    assert_eq!(rect.size.width > 0., nonzero_width);
                    eprintln!(
                        "SELFTEST PASS native range rect={range:?} actual={actual:?} rect={rect:?}"
                    );
                }
                Action::TextRange { text, replacement } => unsafe {
                    let range = replacement.map_or(NSRange::new(isize::MAX as usize, 0), |r| {
                        NSRange::new(r.start, r.end.saturating_sub(r.start))
                    });
                    view.insertText_replacementRange(&NSString::from_str(&text), range);
                },
                Action::PreeditRange {
                    text,
                    selection,
                    replacement,
                } => unsafe {
                    let range = replacement.map_or(NSRange::new(isize::MAX as usize, 0), |r| {
                        NSRange::new(r.start, r.end.saturating_sub(r.start))
                    });
                    view.setMarkedText_selectedRange_replacementRange(
                        &NSString::from_str(&text),
                        NSRange::new(
                            selection.start,
                            selection.end.saturating_sub(selection.start),
                        ),
                        range,
                    );
                },
                Action::Unmark => view.unmarkText(),
                Action::ExpectNoText => {
                    let range = view.selectedRange();
                    assert_eq!(
                        range.location,
                        isize::MAX as usize,
                        "closed dialog must unpublish text input state"
                    );
                    assert!(!view.hasMarkedText());
                    eprintln!("SELFTEST PASS closed text owner is unpublished");
                }
                Action::ExpectTextState {
                    text,
                    selection,
                    marked,
                    query,
                    actual,
                } => {
                    let selected = view.selectedRange();
                    let native_marked = view.markedRange();
                    assert_eq!(
                        selected,
                        NSRange::new(selection.start, selection.end - selection.start)
                    );
                    assert_eq!(
                        native_marked,
                        marked
                            .as_ref()
                            .map_or(NSRange::new(isize::MAX as usize, 0), |r| NSRange::new(
                                r.start,
                                r.end - r.start
                            ))
                    );
                    let complete = unsafe {
                        view.attributedSubstringForProposedRange_actualRange(
                            NSRange::new(0, usize::MAX),
                            std::ptr::null_mut(),
                        )
                    }
                    .expect("active text query")
                    .string()
                    .to_string();
                    assert_eq!(complete, text);
                    let mut actual_range = NSRange::new(0, 0);
                    let partial = unsafe {
                        view.attributedSubstringForProposedRange_actualRange(
                            NSRange::new(query.start, query.end - query.start),
                            &mut actual_range,
                        )
                    }
                    .expect("query slice")
                    .string()
                    .to_string();
                    assert_eq!(
                        actual_range,
                        NSRange::new(actual.start, actual.end - actual.start)
                    );
                    assert_eq!(
                        partial,
                        String::from_utf16_lossy(
                            &text.encode_utf16().collect::<Vec<_>>()[actual.clone()]
                        )
                    );
                    let rect = unsafe {
                        view.firstRectForCharacterRange_actualRange(selected, std::ptr::null_mut())
                    };
                    assert!(
                        rect.origin.x.is_finite()
                            && rect.origin.y.is_finite()
                            && rect.size.height > 0.
                    );
                    let index = view.characterIndexForPoint(NSPoint::new(
                        rect.origin.x,
                        rect.origin.y + rect.size.height * 0.5,
                    ));
                    assert!(index <= text.encode_utf16().count());
                    eprintln!(
                        "SELFTEST PASS exact UTF16 text={text:?} selection={selection:?} marked={marked:?} query={query:?} actual={actual:?} candidate={rect:?} hit={index}"
                    );
                }
                Action::Text(text) => unsafe {
                    view.insertText_replacementRange(
                        &NSString::from_str(&text),
                        NSRange::new(isize::MAX as usize, 0),
                    );
                },
                Action::Preedit(text) => unsafe {
                    view.setMarkedText_selectedRange_replacementRange(
                        &NSString::from_str(&text),
                        NSRange::new(text.encode_utf16().count(), 0),
                        NSRange::new(isize::MAX as usize, 0),
                    );
                },
                Action::TextCommand(command) => unsafe {
                    view.doCommandBySelector(objc2::runtime::Sel::register(
                        std::ffi::CString::new(command).unwrap().as_c_str(),
                    ));
                },
                Action::ExpectText { text, marked } => {
                    let result = unsafe {
                        view.attributedSubstringForProposedRange_actualRange(
                            NSRange::new(0, usize::MAX),
                            std::ptr::null_mut(),
                        )
                    }
                    .map(|s| s.string().to_string());
                    if result.as_deref() != Some(text.as_str()) || view.hasMarkedText() != marked {
                        eprintln!(
                            "SELFTEST FAIL text={result:?} marked={}",
                            view.hasMarkedText()
                        );
                        std::process::exit(3);
                    }
                    let selected = view.selectedRange();
                    let rect = unsafe {
                        view.firstRectForCharacterRange_actualRange(selected, std::ptr::null_mut())
                    };
                    assert!(
                        rect.origin.x.is_finite()
                            && rect.origin.y.is_finite()
                            && rect.size.height > 0.0
                    );
                    eprintln!(
                        "SELFTEST PASS text query, selection {selected:?}, candidate rect {rect:?}"
                    );
                }
                _ => unreachable!(),
            }
        }
        Action::Close => w.performClose(None),
        Action::MenuCommand(tag) => {
            let mut found = false;
            if let Some(main) = app.mainMenu() {
                for top in main.itemArray().iter() {
                    let Some(sub) = top.submenu() else { continue };
                    for item in sub.itemArray().iter() {
                        if item.tag() == tag as isize
                            && let Some(action) = item.action()
                        {
                            // SAFETY: action/target 来自平台层自己建的菜单项。
                            unsafe {
                                app.sendAction_to_from(
                                    action,
                                    item.target().as_deref(),
                                    Some(&item),
                                );
                            }
                            found = true;
                        }
                    }
                }
            }
            if !found {
                eprintln!("selftest: 没有命令 {tag}");
            }
        }
        Action::MouseMove { x, y } => {
            if let (Some(v), Some(e)) = (
                w.contentView(),
                mouse_event(&w, NSEventType::MouseMoved, x, y, 0),
            ) {
                v.mouseMoved(&e);
            }
        }
        Action::MouseExit => {
            // SAFETY: 不带用户数据（null 指针）。
            let e = unsafe {
                NSEvent::enterExitEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_trackingNumber_userData(
                    NSEventType::MouseExited,
                    win_point(&w, -1.0, -1.0),
                    NSEventModifierFlags::empty(),
                    0.0,
                    w.windowNumber(),
                    None,
                    0,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if let (Some(v), Some(e)) = (w.contentView(), e) {
                v.mouseExited(&e);
            }
        }
        Action::Click { x, y } => {
            for ty in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
                if let Some(e) = mouse_event(&w, ty, x, y, 1) {
                    app.sendEvent(&e);
                }
            }
        }
        Action::Key { key_code } | Action::KeyCharacters { key_code, .. } => {
            let chars = match action {
                Action::KeyCharacters { chars, .. } => chars,
                _ => "",
            };
            for ty in [NSEventType::KeyDown, NSEventType::KeyUp] {
                if let Some(e) = key_event(&w, ty, key_code, chars) {
                    app.sendEvent(&e);
                }
            }
        }
        Action::ScrollWheel {
            x,
            y,
            delta_x,
            delta_y,
        } => {
            use objc2_core_foundation::CGPoint;
            use objc2_core_graphics::{CGEvent, CGScrollEventUnit};
            let cg = CGEvent::new_scroll_wheel_event2(
                None,
                CGScrollEventUnit::Line,
                2,
                delta_y,
                delta_x,
                0,
            )
            .expect("create scroll-wheel event");
            let point = win_point(&w, x, y);
            let primary_height = objc2_app_kit::NSScreen::screens(mtm)
                .firstObject()
                .expect("primary display for scroll coordinate conversion")
                .frame()
                .size
                .height;
            // A synthesized NSEvent has no NSWindow, so sendEvent cannot hit-test
            // it. Supply window coordinates and call the real content view's
            // native scrollWheel: entry, which performs the original conversion
            // and dispatches Event::Scroll. Do not call the Rust host directly.
            CGEvent::set_location(Some(&cg), CGPoint::new(point.x, primary_height - point.y));
            let event = NSEvent::eventWithCGEvent(&cg).expect("wrap native scroll event");
            let actual = event.locationInWindow();
            assert!(
                (actual.x - point.x).abs() < 1e-6 && (actual.y - point.y).abs() < 1e-6,
                "native synthesized scroll must carry the requested window coordinates"
            );
            eprintln!(
                "SELFTEST native scroll window={} point={actual:?} delta=({}, {})",
                w.windowNumber(),
                event.scrollingDeltaX(),
                event.scrollingDeltaY(),
            );
            w.contentView()
                .expect("real native content view")
                .scrollWheel(&event);
        }
        Action::KeyEquivalent {
            chars,
            key_code,
            control,
            alt,
            shift,
            command,
        } => {
            let mut flags = NSEventModifierFlags::empty();
            if control {
                flags |= NSEventModifierFlags::Control;
            }
            if alt {
                flags |= NSEventModifierFlags::Option;
            }
            if shift {
                flags |= NSEventModifierFlags::Shift;
            }
            if command {
                flags |= NSEventModifierFlags::Command;
            }
            let s = NSString::from_str(chars);
            let e = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::KeyDown,
                NSPoint::new(0.0, 0.0),
                flags,
                0.0,
                w.windowNumber(),
                None,
                &s,
                &s,
                false,
                key_code,
            );
            let handled = match (app.mainMenu(), e) {
                (Some(m), Some(e)) => m.performKeyEquivalent(&e),
                _ => false,
            };
            eprintln!(
                "selftest: 菜单快捷键 {chars:?} 修饰 C={control} O={alt} S={shift} D={command} -> handled={handled}"
            );
        }
        Action::WindowDark(on) => {
            let a = if on {
                // SAFETY: AppKit 导出的常量，进程内有效。
                NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua })
            } else {
                None
            };
            w.setAppearance(a.as_deref());
        }
        Action::Capture { path } => {
            let number = w.windowNumber();
            std::thread::spawn(move || {
                let status = std::process::Command::new("screencapture")
                    .args(["-o", "-x", "-l", &number.to_string(), &path])
                    .status();
                eprintln!("selftest: screencapture -> {path}: {status:?}");
            });
        }
    }
}

/// 在后台线程里按时间表（距调用时刻的毫秒数，需升序）执行步骤。
pub fn run_script(steps: Vec<(u64, Action)>) {
    std::thread::spawn(move || {
        let mut last = 0;
        for (t, action) in steps {
            std::thread::sleep(Duration::from_millis(t.saturating_sub(last)));
            last = t;
            let ctx = Box::into_raw(Box::new(action)).cast::<c_void>();
            // SAFETY: 主队列在进程生命周期内有效；run_step 在主线程消费 ctx。
            unsafe { dispatch_async_f(&raw const _dispatch_main_q, ctx, run_step) };
        }
    });
}

/// Main-thread native-object contract only; creates no application or window.
/// Uses the actual cached AX element class and its existing stale-object fence.
#[doc(hidden)]
pub fn required_metadata_contract() -> usize {
    use crate::accessibility::NativeAccessibilityElement;
    use crate::shared::Shared;
    use crate::{
        AccessibilityKey, AccessibilityNode, AccessibilityRole, Event, Window, WindowHandler,
    };
    use objc2::{msg_send, sel};
    struct Noop;
    impl WindowHandler for Noop {
        fn handle(&mut self, _: Event, _: &mut Window) {}
    }
    let mtm = MainThreadMarker::new().expect("run the native object probe on main thread");
    let shared = Shared::new(mtm, Box::new(Noop));
    let key = AccessibilityKey {
        component: 7,
        part: None,
    };
    let mut node = AccessibilityNode {
        key,
        parent: None,
        role: AccessibilityRole::TextField,
        frame: crate::Rect {
            origin: crate::Point::default(),
            size: crate::Size {
                width: 100.,
                height: 32.,
            },
        },
        label: "Required metadata probe".into(),
        value: Some(String::new()),
        description: None,
        enabled: true,
        focused: false,
        selected: false,
        expanded: None,
        invalid: false,
        required: false,
        text_selection: None,
        read_only: false,
    };
    let element = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, element.clone());
    let mut checks = 0;
    for role in [AccessibilityRole::TextField, AccessibilityRole::TextArea] {
        for required in [false, true] {
            for enabled in [false, true] {
                for readonly in [false, true] {
                    for invalid in [false, true] {
                        node.role = role;
                        node.required = required;
                        node.enabled = enabled;
                        node.read_only = readonly;
                        node.invalid = invalid;
                        *shared.accessibility_nodes.borrow_mut() = vec![node.clone()];
                        assert!(crate::accessibility::validate_accessibility_nodes(&[
                            node.clone()
                        ]));
                        let actual: bool = unsafe { msg_send![&*element, isAccessibilityRequired] };
                        assert_eq!(actual, required);
                        let readable: bool = unsafe {
                            msg_send![&*element,isAccessibilitySelectorAllowed:sel!(isAccessibilityRequired)]
                        };
                        assert!(readable);
                        let writable: bool = unsafe {
                            msg_send![&*element,isAccessibilitySelectorAllowed:sel!(setAccessibilityRequired:)]
                        };
                        assert!(!writable);
                        checks += 1;
                    }
                }
            }
        }
    }
    shared.accessibility_nodes.borrow_mut().clear();
    let absent: bool = unsafe { msg_send![&*element, isAccessibilityRequired] };
    assert!(!absent);
    checks += 1;
    node.required = true;
    *shared.accessibility_nodes.borrow_mut() = vec![node];
    let replacement = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, replacement.clone());
    let stale: bool = unsafe { msg_send![&*element, isAccessibilityRequired] };
    assert!(!stale);
    checks += 1;
    let current: bool = unsafe { msg_send![&*replacement, isAccessibilityRequired] };
    assert!(current);
    checks += 1;
    shared.closed.set(true);
    let closed: bool = unsafe { msg_send![&*replacement, isAccessibilityRequired] };
    assert!(!closed);
    checks += 1;
    checks
}

/// Main-thread Link role/action probe. No application, window, or desktop input.
#[doc(hidden)]
pub fn link_role_contract() -> usize {
    use crate::accessibility::NativeAccessibilityElement;
    use crate::shared::Shared;
    use crate::{
        AccessibilityKey, AccessibilityNode, AccessibilityRole, Event, Window, WindowHandler,
    };
    use objc2::rc::Retained;
    use objc2::{msg_send, sel};
    use objc2_foundation::NSString;
    struct Noop;
    impl WindowHandler for Noop {
        fn handle(&mut self, _: Event, _: &mut Window) {}
    }
    let mtm = MainThreadMarker::new().expect("main-thread Link probe");
    let shared = Shared::new(mtm, Box::new(Noop));
    let key = AccessibilityKey {
        component: 8,
        part: None,
    };
    let mut node = AccessibilityNode {
        key,
        parent: None,
        role: AccessibilityRole::Link,
        frame: crate::Rect {
            origin: crate::Point::default(),
            size: crate::Size {
                width: 100.,
                height: 32.,
            },
        },
        label: "Read documentation".into(),
        value: Some(String::new()),
        description: None,
        enabled: true,
        focused: false,
        selected: false,
        expanded: None,
        invalid: false,
        required: false,
        text_selection: None,
        read_only: false,
    };

    let element = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, element.clone());
    let mut checks = 0;
    for enabled in [false, true] {
        node.enabled = enabled;
        *shared.accessibility_nodes.borrow_mut() = vec![node.clone()];
        assert!(crate::accessibility::validate_accessibility_nodes(&[
            node.clone()
        ]));
        let role: Retained<NSString> = unsafe { msg_send![&*element, accessibilityRole] };
        assert_eq!(role.to_string(), "AXLink");
        let allowed: bool = unsafe {
            msg_send![&*element, isAccessibilitySelectorAllowed:sel!(accessibilityPerformPress)]
        };
        assert_eq!(allowed, enabled);
        let focus_allowed: bool = unsafe {
            msg_send![&*element, isAccessibilitySelectorAllowed:sel!(setAccessibilityFocused:)]
        };
        assert_eq!(focus_allowed, enabled);
        let value_allowed: bool = unsafe {
            msg_send![&*element, isAccessibilitySelectorAllowed:sel!(setAccessibilityValue:)]
        };
        assert!(!value_allowed);
        // Enabled dispatch requires a real Window; covered by Gallery consumer tests.
        // This object-only probe must not create a window or invoke that dispatch.
        if !enabled {
            let press: bool = unsafe { msg_send![&*element, accessibilityPerformPress] };
            assert!(!press);
            unsafe {
                let _: () = msg_send![&*element, setAccessibilityFocused:true];
            }
        }
        checks += 1;
    }
    shared.accessibility_nodes.borrow_mut().clear();
    let absent: bool = unsafe { msg_send![&*element, accessibilityPerformPress] };
    assert!(!absent);
    checks += 1;
    *shared.accessibility_nodes.borrow_mut() = vec![node];
    let replacement = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, replacement.clone());
    let stale: bool = unsafe { msg_send![&*element, accessibilityPerformPress] };
    assert!(!stale);
    checks += 1;
    let current: Retained<NSString> = unsafe { msg_send![&*replacement, accessibilityRole] };
    assert_eq!(current.to_string(), "AXLink");
    let current_allowed: bool = unsafe {
        msg_send![&*replacement, isAccessibilitySelectorAllowed:sel!(accessibilityPerformPress)]
    };
    assert!(current_allowed);
    checks += 1;
    shared.closed.set(true);
    let closed: bool = unsafe { msg_send![&*replacement, accessibilityPerformPress] };
    assert!(!closed);
    checks + 1
}

/// Hidden native-object contract only: no shown window, global input or drag.
#[doc(hidden)]
pub fn titlebar_contract() -> Vec<(bool, f64, f64)> {
    use crate::shared::{Objects, Shared};
    use crate::view::TekesView;
    use crate::{Event, Window, WindowHandler};
    use objc2::MainThreadOnly;
    use objc2_app_kit::{
        NSApplicationActivationPolicy, NSBackingStoreType, NSToolbar, NSWindowStyleMask,
        NSWindowTitleVisibility, NSWindowToolbarStyle,
    };
    use objc2_foundation::NSRect;
    use std::{cell::Cell, rc::Rc};
    struct Measure(Rc<Cell<f64>>, Rc<Cell<Option<f64>>>);
    impl WindowHandler for Measure {
        fn handle(&mut self, event: Event, window: &mut Window) {
            if event == Event::Created {
                let height = window.titlebar_content_height();
                self.0.set(height);
                let (safe_height, leading) = window.titlebar_safe_area();
                println!(
                    "TITLEBAR_NATIVE_SAFE_AREA content_height={height} reserved_height={safe_height} leading={leading}"
                );
                assert!(safe_height >= height && safe_height <= window.size().height);
                assert!(leading >= 0. && leading <= window.size().width);
                if height == 0. {
                    assert_eq!((safe_height, leading), (0., 0.));
                    assert_eq!(window.titlebar_control_center_y(), None);
                } else {
                    assert!(leading > 0.);
                    self.1.set(window.titlebar_control_center_y());
                }
            }
            if event == Event::Command(crate::CommandId(1000)) {
                window.set_cursor_regions(vec![
                    crate::CursorRegion {
                        origin: crate::Point { x: 180., y: 100. },
                        size: crate::Size {
                            width: 9.,
                            height: 100.,
                        },
                        icon: crate::CursorIcon::ResizeHorizontal,
                    },
                    crate::CursorRegion {
                        origin: crate::Point { x: 300., y: 200. },
                        size: crate::Size {
                            width: 100.,
                            height: 9.,
                        },
                        icon: crate::CursorIcon::ResizeVertical,
                    },
                ]);
            } else if let Event::Command(crate::CommandId(height)) = event {
                let center = window.set_toolbar_row_height(f64::from(height));
                self.1.set(center);
                for invalid in [f64::NAN, f64::INFINITY, -1., 0.] {
                    assert_eq!(window.set_toolbar_row_height(invalid), center);
                }
            }
            // A synthetic object callback is not a live native gesture.
            assert!(!window.try_begin_titlebar_drag());
        }
    }
    let mtm = MainThreadMarker::new().expect("main-thread native contract");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    let mut rows = vec![];
    for extended in [false, true] {
        let height = Rc::new(Cell::new(-1.));
        let center = Rc::new(Cell::new(None));
        let shared = Shared::new(mtm, Box::new(Measure(height.clone(), center.clone())));
        let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(720., 480.));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect,
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable
                    | if extended {
                        NSWindowStyleMask::FullSizeContentView
                    } else {
                        NSWindowStyleMask::empty()
                    },
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setToolbarStyle(NSWindowToolbarStyle::Unified);
        if extended {
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
        }
        window.setToolbar(Some(&NSToolbar::initWithIdentifier(
            NSToolbar::alloc(mtm),
            &NSString::from_str("TekesUI.Titlebar.Contract"),
        )));
        let view = TekesView::new(mtm, shared.clone(), rect);
        window.setContentView(Some(&view));
        view.layoutSubtreeIfNeeded();
        shared
            .objects
            .set(Objects {
                window: window.clone(),
                view: view.clone(),
            })
            .ok()
            .unwrap();
        shared.ready.set(true);
        shared.dispatch(Event::Created);
        if extended {
            let button = window
                .standardWindowButton(objc2_app_kit::NSWindowButton::CloseButton)
                .unwrap();
            let r = view.convertRect_fromView(button.bounds(), Some(&button));
            let expected_center = r.origin.y + r.size.height * 0.5;
            assert_eq!(center.get(), Some(expected_center));
            println!("TITLEBAR_NATIVE_CONTROL_CENTER y={expected_center}");
        }
        let expected = if extended {
            window.frame().size.height - window.contentLayoutRect().size.height
        } else {
            0.
        };
        assert!(
            (height.get() - expected).abs() < 0.001,
            "native content overlap"
        );
        assert!(!window.isVisible());
        rows.push((extended, height.get(), expected));
        // Public alignment API on original native objects. No windows shown.
        let originals: Vec<_> = [
            objc2_app_kit::NSWindowButton::CloseButton,
            objc2_app_kit::NSWindowButton::MiniaturizeButton,
            objc2_app_kit::NSWindowButton::ZoomButton,
        ]
        .into_iter()
        .filter_map(|kind| {
            window.standardWindowButton(kind).map(|button| {
                let frame = button.frame();
                (kind, button, frame)
            })
        })
        .collect();
        for row in [10, 42, 65, 84, 126] {
            shared.dispatch(Event::Command(crate::CommandId(row)));
            view.layoutSubtreeIfNeeded();
            window.setContentSize(NSSize::new(720. + f64::from(row), 480.));
            view.layoutSubtreeIfNeeded();
            for (kind, original, before) in &originals {
                let button = window.standardWindowButton(*kind).unwrap();
                assert!(std::ptr::eq(&*button, &**original));
                let after = button.frame();
                assert_eq!(after.origin.x, before.origin.x);
                assert_eq!(after.size, before.size);
                if extended {
                    let rect = view.convertRect_fromView(button.bounds(), Some(&button));
                    let expected = f64::from(row).max(before.size.height) * 0.5;
                    assert!((rect.origin.y + rect.size.height * 0.5 - expected).abs() < 0.001);
                    assert!(
                        button.visibleRect().size.height >= before.size.height - 0.001,
                        "native traffic light must remain visible at row {row}"
                    );
                    // NSView hitTest takes parent coordinates. The returned
                    // target must be the real original native NSButton.
                    let local = NSPoint::new(rect.origin.x + rect.size.width * 0.5, expected);
                    // SAFETY: the retained original view remains attached to its native window.
                    let parent = unsafe { view.superview() };
                    let point = view.convertPoint_toView(local, parent.as_deref());
                    let hit = view.hitTest(point).expect("native button hit");
                    let expected: &objc2_app_kit::NSView = &button;
                    assert!(std::ptr::eq(&*hit, expected));
                } else {
                    assert_eq!(after, *before);
                }
            }
        }
        println!(
            "TITLEBAR_ROW_ALIGNMENT extended={extended} rows=5 original_size_x_identity=true resized=true original_native_hit_targets=true"
        );
        shared.dispatch(Event::Command(crate::CommandId(1000)));
        for _ in 0..3 {
            view.updateTrackingAreas();
        }
        let areas = view.trackingAreas();
        assert_eq!(
            areas
                .iter()
                .filter(|area| area
                    .options()
                    .contains(objc2_app_kit::NSTrackingAreaOptions::CursorUpdate))
                .count(),
            3
        );
        for (x, y, expected) in [
            (184., 150., Some(crate::CursorIcon::ResizeHorizontal)),
            (350., 204., Some(crate::CursorIcon::ResizeVertical)),
            (250., 150., None),
        ] {
            let (icon, cursor) = view.cursor_for_point(crate::Point { x, y });
            assert_eq!(icon, expected);
            let event = mouse_event(&window, NSEventType::MouseMoved, x, y, 0).unwrap();
            view.cursorUpdate(&event);
            assert!(
                std::ptr::eq(&*objc2_app_kit::NSCursor::currentCursor(), &*cursor),
                "actual native cursor target"
            );
        }
        view.set_cursor_regions(Vec::new());
        assert_eq!(
            view.trackingAreas()
                .iter()
                .filter(|area| area
                    .options()
                    .contains(objc2_app_kit::NSTrackingAreaOptions::CursorUpdate))
                .count(),
            1
        );
        let event = mouse_event(&window, NSEventType::MouseMoved, 184., 150., 0).unwrap();
        view.cursorUpdate(&event);
        assert!(std::ptr::eq(
            &*objc2_app_kit::NSCursor::currentCursor(),
            &*objc2_app_kit::NSCursor::arrowCursor()
        ));
        println!(
            "NATIVE_CURSOR_CONTRACT extended={extended} interior_boundary_areas=2 rebuilds=3 native_horizontal_vertical_and_arrow=true removed_region_restores_arrow=true"
        );
        let down = mouse_event(&window, NSEventType::LeftMouseDown, 100., 20., 1).unwrap();
        view.mouseDown(&down);
        assert!(shared.titlebar_drag_origin.borrow().is_some());
        let drag = mouse_event(&window, NSEventType::LeftMouseDragged, 110., 20., 1).unwrap();
        view.mouseDragged(&drag);
        assert_eq!(
            shared
                .titlebar_drag_origin
                .borrow()
                .as_ref()
                .unwrap()
                .r#type(),
            NSEventType::LeftMouseDown
        );
        let right = mouse_event(&window, NSEventType::RightMouseDown, 100., 20., 1).unwrap();
        view.rightMouseDown(&right);
        assert_eq!(
            shared
                .titlebar_drag_origin
                .borrow()
                .as_ref()
                .unwrap()
                .r#type(),
            NSEventType::LeftMouseDown
        );
        let up = mouse_event(&window, NSEventType::LeftMouseUp, 110., 20., 1).unwrap();
        view.mouseUp(&up);
        assert!(shared.titlebar_drag_origin.borrow().is_none());
        view.mouseDragged(&drag);
        assert!(shared.titlebar_drag_origin.borrow().is_none());
        view.mouseDown(&down);
        shared.dispatch(Event::Focus(false));
        assert!(shared.titlebar_drag_origin.borrow().is_none());
        view.mouseDown(&down);
        shared.finish();
        assert!(shared.titlebar_drag_origin.borrow().is_none());
        window.close();
    }
    rows
}

/// Actual AppKit regression for re-presenting one live Exclusive cover. Only
/// this process's four temporary windows are shown, hidden and closed.
#[doc(hidden)]
pub fn exclusive_reentry_contract() {
    use crate::{App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler};
    use std::{cell::Cell, rc::Rc};
    struct Noop;
    impl WindowHandler for Noop {
        fn handle(&mut self, event: Event, window: &mut Window) {
            if event == Event::CloseRequested {
                window.close();
            }
        }
    }
    struct Probe {
        started: bool,
        complete: Rc<Cell<bool>>,
    }
    impl WindowHandler for Probe {
        fn handle(&mut self, event: Event, window: &mut Window) {
            if event == Event::CloseRequested {
                window.close();
                return;
            }
            if event != Event::Focus(true) || self.started {
                return;
            }
            let mtm = MainThreadMarker::new().expect("native focus callback is main-thread");
            let app = NSApplication::sharedApplication(mtm);
            // launch_window emits logical Focus before first presentation. Wait
            // for actual native activation instead of treating that as key focus.
            let Some(anchor_window) = app.keyWindow().filter(|w| {
                app.isRunning()
                    && app.isActive()
                    && w.windowNumber() == window.native_window_number()
            }) else {
                return;
            };
            self.started = true;
            assert!(app.isRunning() && app.isActive());
            assert_eq!(anchor_window.windowNumber(), window.native_window_number());
            let config = |name: &str| WindowConfig {
                title: format!("TekesUI Exclusive reentry {name}"),
                size: crate::Size {
                    width: 320.,
                    height: 200.,
                },
                ..Default::default()
            };
            let other = window.open_window(config("visible"), Noop);
            let hidden = window.open_window(config("originally-hidden"), Noop);
            let other_window = &other.shared.objects.get().unwrap().window;
            let hidden_window = &hidden.shared.objects.get().unwrap().window;
            hidden_window.orderOut(None);
            // Establish the real original key before opening Settings. No key
            // operation is performed by the test after closing the cover.
            anchor_window.makeKeyAndOrderFront(None);
            println!(
                "EXCLUSIVE_FOCUS stage=initial-anchor app_active={} app_running={} actual_key={:?} target={} target_key={}",
                app.isActive(),
                app.isRunning(),
                app.keyWindow().map(|w| w.windowNumber()),
                anchor_window.windowNumber(),
                anchor_window.isKeyWindow()
            );
            assert!(
                anchor_window.isKeyWindow(),
                "initial anchor must actually own native key focus"
            );
            assert!(anchor_window.isVisible() && other_window.isVisible());
            assert!(!hidden_window.isVisible());
            let cover = window.open_exclusive_window(config("cover"), Noop);
            let cover_window = &cover.shared.objects.get().unwrap().window;
            assert!(
                cover_window.isKeyWindow(),
                "Exclusive cover must actually own native key focus"
            );
            assert!(cover_window.isVisible());
            assert!(!anchor_window.isVisible() && !other_window.isVisible());
            assert_eq!(cover.shared.exclusive_restore.borrow().len(), 2);
            assert!(
                cover
                    .shared
                    .exclusive_key
                    .borrow()
                    .as_ref()
                    .and_then(std::rc::Weak::upgrade)
                    .is_some_and(|saved| saved
                        .objects
                        .get()
                        .is_some_and(|o| o.window.windowNumber() == window.native_window_number()))
            );
            // Re-show an already-recorded window to cover duplicate merging.
            other_window.orderFront(None);
            window.present_exclusive(&cover);
            assert_eq!(cover.shared.exclusive_restore.borrow().len(), 2);
            assert!(
                cover
                    .shared
                    .exclusive_key
                    .borrow()
                    .as_ref()
                    .and_then(std::rc::Weak::upgrade)
                    .is_some_and(|saved| saved
                        .objects
                        .get()
                        .is_some_and(|o| o.window.windowNumber() == window.native_window_number()))
            );
            assert!(cover_window.isVisible());
            assert!(!anchor_window.isVisible() && !other_window.isVisible());
            assert!(!hidden_window.isVisible());
            cover_window.close();
            println!(
                "EXCLUSIVE_FOCUS stage=after-close app_active={} app_running={} actual_key={:?} target={} target_key={}",
                app.isActive(),
                app.isRunning(),
                app.keyWindow().map(|w| w.windowNumber()),
                anchor_window.windowNumber(),
                anchor_window.isKeyWindow()
            );
            assert!(cover.is_closed());
            assert!(anchor_window.isVisible() && other_window.isVisible());
            assert!(!hidden_window.isVisible());
            assert!(
                anchor_window.isKeyWindow(),
                "original key window must be restored"
            );
            assert!(cover.shared.exclusive_restore.borrow().is_empty());
            assert!(cover.shared.exclusive_key.borrow().is_none());
            for handle in [&other, &hidden] {
                handle.shared.objects.get().unwrap().window.close();
                handle.shared.finish();
            }
            cover.shared.finish();
            self.complete.set(true);
            window.close();
        }
    }
    let complete = Rc::new(Cell::new(false));
    // Reuse the existing selftest scheduler: if native activation never occurs,
    // close this probe's only window and fail after the normal run loop returns.
    run_script(vec![(5_000, Action::Close)]);
    App::new(AppConfig {
        name: Some("TekesUI Exclusive reentry regression".into()),
        quit_after_last_window_closed: true,
    })
    .run(
        WindowConfig {
            title: "TekesUI Exclusive reentry anchor".into(),
            size: crate::Size {
                width: 320.,
                height: 200.,
            },
            ..Default::default()
        },
        MenuBar::default(),
        Probe {
            started: false,
            complete: complete.clone(),
        },
    );
    assert!(
        complete.get(),
        "native application never completed the focused Exclusive scenario"
    );
    println!(
        "EXCLUSIVE_REENTRY_NATIVE_OK windows=4 repeated_present=2 restore_visible=2 restore_originally_hidden=0 original_focus=true"
    );
}

/// Issue #120 actual AppKit object mapping for image role and alternate text.
/// Creates one retained NSObject subclass, without an application or window.
pub fn image_role_contract() -> usize {
    use crate::accessibility::NativeAccessibilityElement;
    use crate::shared::Shared;
    use crate::{
        AccessibilityKey, AccessibilityNode, AccessibilityRole, Event, Window, WindowHandler,
    };
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2_foundation::NSString;

    struct Noop;
    impl WindowHandler for Noop {
        fn handle(&mut self, _: Event, _: &mut Window) {}
    }

    let mtm = MainThreadMarker::new().expect("Image probe main thread");
    let shared = Shared::new(mtm, Box::new(Noop));
    let key = AccessibilityKey {
        component: 120,
        part: None,
    };
    let element = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, element.clone());
    let node = AccessibilityNode {
        key,
        parent: None,
        role: AccessibilityRole::Image,
        frame: crate::Rect {
            origin: crate::Point::default(),
            size: crate::Size {
                width: 24.,
                height: 16.,
            },
        },
        label: "Alt text".into(),
        value: None,
        description: None,
        enabled: false,
        focused: false,
        selected: false,
        expanded: None,
        invalid: false,
        required: false,
        text_selection: None,
        read_only: false,
    };
    assert!(crate::accessibility::validate_accessibility_nodes(
        std::slice::from_ref(&node)
    ));
    *shared.accessibility_nodes.borrow_mut() = vec![node];

    let role: Retained<NSString> = unsafe { msg_send![&*element, accessibilityRole] };
    assert_eq!(&*role, unsafe { objc2_app_kit::NSAccessibilityImageRole });
    let label: Retained<NSString> = unsafe { msg_send![&*element, accessibilityLabel] };
    assert_eq!(label.to_string(), "Alt text");
    let is_element: bool = unsafe { msg_send![&*element, isAccessibilityElement] };
    assert!(is_element);
    shared.finish();
    3
}

/// Issue #103 actual AppKit object mapping for the original Splitter role.
/// Queries a real NSObject subclass with a hidden real window dispatch owner.
/// Never shows or activates the window or asks VoiceOver to speak.
pub fn splitter_role_contract() -> usize {
    use crate::accessibility::NativeAccessibilityElement;
    use crate::shared::Shared;
    use crate::{
        AccessibilityAction, AccessibilityKey, AccessibilityNode, AccessibilityRange,
        AccessibilityRole, Event, Window, WindowHandler,
    };
    use objc2::rc::Retained;
    use objc2::{msg_send, sel};
    use objc2_foundation::{NSNumber, NSString};

    struct Recorder(std::rc::Rc<std::cell::RefCell<Vec<Event>>>);
    impl WindowHandler for Recorder {
        fn handle(&mut self, event: Event, _: &mut Window) {
            self.0.borrow_mut().push(event);
        }
    }

    let mtm = MainThreadMarker::new().expect("Splitter probe main thread");
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let shared = Shared::new(mtm, Box::new(Recorder(events.clone())));
    // Enabled native actions use the production WindowHandler dispatch, which
    // requires actual Window objects. Preserve that dependency without taking
    // desktop focus; the old object-only fixture panicked before dispatch.
    use objc2::MainThreadOnly;
    use objc2_app_kit::{NSApplicationActivationPolicy, NSBackingStoreType, NSWindowStyleMask};
    NSApplication::sharedApplication(mtm)
        .setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    let rect = objc2_foundation::NSRect::new(NSPoint::new(0., 0.), NSSize::new(160., 80.));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            NSWindowStyleMask::empty(),
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
        component: 103,
        part: Some(4),
    };
    let element = NativeAccessibilityElement::new(mtm, &shared, key);
    shared
        .accessibility_elements
        .borrow_mut()
        .insert(key, element.clone());
    let node = AccessibilityNode {
        key,
        parent: None,
        role: AccessibilityRole::Splitter(AccessibilityRange::new(0.1, 0.9).unwrap()),
        frame: crate::Rect {
            origin: crate::Point { x: 80.0, y: 8.0 },
            size: crate::Size {
                width: 1.0,
                height: 72.0,
            },
        },
        label: "Split pane divider".into(),
        value: Some("0.625".into()),
        description: None,
        enabled: true,
        focused: false,
        selected: false,
        expanded: None,
        invalid: false,
        required: false,
        text_selection: None,
        read_only: false,
    };
    assert!(crate::accessibility::validate_accessibility_nodes(
        std::slice::from_ref(&node)
    ));
    *shared.accessibility_nodes.borrow_mut() = vec![node];

    let role: Retained<NSString> = unsafe { msg_send![&*element, accessibilityRole] };
    assert_eq!(&*role, unsafe {
        objc2_app_kit::NSAccessibilitySplitterRole
    });
    let label: Retained<NSString> = unsafe { msg_send![&*element, accessibilityLabel] };
    assert_eq!(label.to_string(), "Split pane divider");
    let value: Retained<NSNumber> = unsafe { msg_send![&*element, accessibilityValue] };
    assert!((value.as_f64() - 0.625).abs() < f64::EPSILON);
    let minimum: Option<Retained<NSNumber>> =
        unsafe { msg_send![&*element, accessibilityMinValue] };
    let maximum: Option<Retained<NSNumber>> =
        unsafe { msg_send![&*element, accessibilityMaxValue] };
    assert!((minimum.unwrap().as_f64() - 0.1).abs() < f64::EPSILON);
    assert!((maximum.unwrap().as_f64() - 0.9).abs() < f64::EPSILON);
    let is_element: bool = unsafe { msg_send![&*element, isAccessibilityElement] };
    assert!(is_element);
    let increment_allowed: bool = unsafe {
        msg_send![&*element, isAccessibilitySelectorAllowed: sel!(accessibilityPerformIncrement)]
    };
    assert!(increment_allowed);
    let decrement_allowed: bool = unsafe {
        msg_send![&*element, isAccessibilitySelectorAllowed: sel!(accessibilityPerformDecrement)]
    };
    assert!(decrement_allowed);
    let incremented: bool = unsafe { msg_send![&*element, accessibilityPerformIncrement] };
    let decremented: bool = unsafe { msg_send![&*element, accessibilityPerformDecrement] };
    assert!(incremented && decremented);
    assert_eq!(
        *events.borrow(),
        vec![
            Event::Accessibility {
                target: key,
                action: AccessibilityAction::Increment,
            },
            Event::Accessibility {
                target: key,
                action: AccessibilityAction::Decrement,
            },
        ],
        "native actions must dispatch the exact splitter key in order"
    );

    let setter_allowed: bool = unsafe {
        msg_send![&*element, isAccessibilitySelectorAllowed: sel!(setAccessibilityValue:)]
    };
    assert!(setter_allowed);
    let next_value = NSNumber::new_f64(0.75);
    let _: () = unsafe { msg_send![&*element, setAccessibilityValue: &*next_value] };
    assert_eq!(
        events.borrow().last(),
        Some(&Event::Accessibility {
            target: key,
            action: AccessibilityAction::SetValue("0.75".into()),
        })
    );
    assert_eq!(events.borrow().len(), 3);

    let mut disabled = shared.accessibility_nodes.borrow()[0].clone();
    disabled.enabled = false;
    *shared.accessibility_nodes.borrow_mut() = vec![disabled];
    let increment_allowed: bool = unsafe {
        msg_send![&*element, isAccessibilitySelectorAllowed: sel!(accessibilityPerformIncrement)]
    };
    assert!(!increment_allowed);
    let incremented: bool = unsafe { msg_send![&*element, accessibilityPerformIncrement] };
    assert!(!incremented);

    let mut read_only = shared.accessibility_nodes.borrow()[0].clone();
    read_only.enabled = true;
    read_only.read_only = true;
    *shared.accessibility_nodes.borrow_mut() = vec![read_only];
    let decrement_allowed: bool = unsafe {
        msg_send![&*element, isAccessibilitySelectorAllowed: sel!(accessibilityPerformDecrement)]
    };
    assert!(!decrement_allowed);
    let decremented: bool = unsafe { msg_send![&*element, accessibilityPerformDecrement] };
    assert!(!decremented);
    assert_eq!(events.borrow().len(), 3, "refused actions emit no events");
    shared.finish();
    18
}
