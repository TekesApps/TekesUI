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

fn key_event(w: &NSWindow, ty: NSEventType, key_code: u16) -> Option<objc2::rc::Retained<NSEvent>> {
    let s = NSString::from_str("");
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
        | Action::ExpectAXText { .. }
        | Action::AuditAXText(_)
        | Action::ExpectAXMissing(_) => {
            use objc2::{DefinedClass, msg_send};
            use objc2_foundation::NSRange;
            let content = w.contentView().expect("selftest content");
            let view = content
                .downcast_ref::<crate::view::TekesView>()
                .expect("TekesView");
            let component = match &action {
                Action::AXSelect { component, .. }
                | Action::AXReplace { component, .. }
                | Action::ExpectAXText { component, .. } => *component,
                Action::AuditAXText(component) | Action::ExpectAXMissing(component) => *component,
                _ => unreachable!(),
            };
            let element = view
                .ivars()
                .shared
                .accessibility_elements
                .borrow()
                .get(&crate::AccessibilityKey {
                    component,
                    part: None,
                })
                .cloned();
            if matches!(action, Action::ExpectAXMissing(_)) {
                assert!(element.is_none(), "closed AX element remains published");
                eprintln!("SELFTEST PASS AX closed owner removed");
                return;
            }
            let element = element.expect("native AX text element");
            match action {
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
        Action::Key { key_code } => {
            for ty in [NSEventType::KeyDown, NSEventType::KeyUp] {
                if let Some(e) = key_event(&w, ty, key_code) {
                    app.sendEvent(&e);
                }
            }
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
