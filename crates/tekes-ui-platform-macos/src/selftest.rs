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
    Miniaturize,
    Deminiaturize,
    /// 用 `screencapture -o -l <窗口号> -x <path>` 截取窗口（设备像素，不含阴影）。
    /// 截图在单独的线程里做，不阻塞主线程。
    Capture {
        path: String,
    },
    /// 只打一行日志，用来在日志里标记阶段。
    Log(String),
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
        Action::Resize { w: nw, h: nh } => w.setContentSize(NSSize::new(nw, nh)),
        Action::Miniaturize => w.miniaturize(None),
        Action::Deminiaturize => w.deminiaturize(None),
        Action::Log(_) => {}
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
