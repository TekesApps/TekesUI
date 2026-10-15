//! 打开一个窗口和一组菜单，把收到的每个事件打到 stderr，数 Redraw，
//! 并在每次重绘时按最后一次点击的位置改 CAMetalLayer 的 backgroundColor。
//!
//! 这个示例不渲染（渲染层在另一个 crate，下一阶段集成），用来验证平台层本身：
//! 空闲时事件流是否真的静止、缩放时是否同步重绘、菜单和输入是否规整。
//!
//! 运行：`cargo run -p tekes-ui-platform-macos --example window`
//!
//! 环境变量：
//! - `TEKES_SELFTEST=1`：自测模式。示例自己合成鼠标、键盘、滚轮事件，改窗口尺寸和外观，
//!   最后关窗退出，进程退出码 0 表示走完全部步骤。
//! - `TEKES_SELFTEST=hold`：同上，但做完后保留窗口不退出，便于外部截图和测空闲 CPU。
//! - `TEKES_LOG_MOVES=1`：同时记录鼠标移动（默认只统计，避免刷屏）。
//! - `TEKES_AUTOSAVE=1`：打开窗口 frame 自动保存。

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("这个示例只能在 macOS 上运行");
}

#[cfg(target_os = "macos")]
fn main() {
    mac::main();
}

#[cfg(target_os = "macos")]
mod mac {
    use std::time::Instant;

    use objc2_core_graphics::CGColor;
    use objc2_quartz_core::{CALayer, CATransaction};
    use tekes_ui_platform_macos::{
        App, AppConfig, CommandId, Event, KeyEquivalent, Menu, MenuBar, MenuCommand, MenuItem,
        Modifiers, MouseEventKind, Point, Window, WindowConfig, WindowHandler, WindowPlacement,
    };

    const CMD_LIGHT: CommandId = CommandId(1);
    const CMD_DARK: CommandId = CommandId(2);
    const CMD_CONTRAST: CommandId = CommandId(3);
    const CMD_REDRAW: CommandId = CommandId(10);
    const CMD_TWICE: CommandId = CommandId(11);
    const CMD_GROW: CommandId = CommandId(12);

    struct Logger {
        start: Instant,
        last_click: Point,
        redraws: u64,
        moves: u64,
        log_moves: bool,
        dark: bool,
        contrast: bool,
        /// 正在 handle 里。用来断言平台层没有重入调用 handle。
        busy: bool,
    }

    impl Logger {
        fn log(&self, what: &str) {
            eprintln!("[{:>8.3}] {what}", self.start.elapsed().as_secs_f64());
        }

        fn paint(&self, window: &Window) {
            let size = window.size();
            let r = (self.last_click.x / size.width.max(1.0)).clamp(0.0, 1.0);
            let g = (self.last_click.y / size.height.max(1.0)).clamp(0.0, 1.0);
            // 深色背景叠加一点区别，便于看出外观变化；增强对比度时亮度拉满。
            let b = if self.dark { 0.25 } else { 0.75 };
            let (r, g, b) = if self.contrast {
                (r, g, 1.0 - b)
            } else {
                (r, g, b)
            };
            let color = CGColor::new_srgb(r, g, b, 1.0);
            // SAFETY: metal_layer 在窗口存活期间指向有效的 CAMetalLayer（CALayer 的子类）。
            let layer = unsafe { window.metal_layer().cast::<CALayer>().as_ref() };
            // 关掉隐式动画，否则颜色会渐变 0.25 秒，空闲后看起来像还在动。
            CATransaction::begin();
            CATransaction::setDisableActions(true);
            layer.setBackgroundColor(Some(&color));
            CATransaction::commit();
        }

        fn sync_checks(&self, window: &Window) {
            window.set_command_checked(CMD_LIGHT, !self.dark);
            window.set_command_checked(CMD_DARK, self.dark);
            window.set_command_checked(CMD_CONTRAST, self.contrast);
        }
    }

    impl WindowHandler for Logger {
        fn handle(&mut self, event: Event, window: &mut Window) {
            assert!(!self.busy, "handle 被重入调用：平台层的排队机制失效");
            self.busy = true;
            self.handle_inner(event, window);
            self.busy = false;
        }
    }

    impl Logger {
        fn handle_inner(&mut self, event: Event, window: &mut Window) {
            match &event {
                Event::Redraw => {
                    self.redraws += 1;
                    let (w, h) = window.drawable_size();
                    self.log(&format!(
                        "Redraw #{} (platform count {}) drawable {w}x{h}",
                        self.redraws,
                        window.redraw_count()
                    ));
                    self.paint(window);
                    // 在处理 Redraw 时改标题：AppKit 可能同步回调，用来验证重入安全。
                    window.set_title(&format!(
                        "TekesUI window example — redraws: {}",
                        self.redraws
                    ));
                }
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Moved => {
                        self.moves += 1;
                        if self.log_moves {
                            self.log(&format!("{event:?}"));
                        }
                    }
                    MouseEventKind::Down { .. } => {
                        self.last_click = m.position;
                        self.log(&format!("{event:?}"));
                        window.request_redraw();
                    }
                    _ => self.log(&format!("{event:?}")),
                },
                Event::Created => {
                    self.log(&format!(
                        "Created: size {:?} scale {} drawable {:?} layer {:p} windowNumber {}",
                        window.size(),
                        window.scale(),
                        window.drawable_size(),
                        window.metal_layer(),
                        window.native_window_number()
                    ));
                    if let Some(r) = window.placement_report() {
                        for (i, s) in r.all_screens.iter().enumerate() {
                            self.log(&format!(
                                "  screen[{i}] {:?} builtin={} scale={} max_fps={} frame={:?}",
                                s.name, s.builtin, s.scale, s.max_fps, s.frame
                            ));
                        }
                        let s = &r.screen;
                        self.log(&format!(
                            "  placement: screen[{}] {:?} (builtin={}, scale={}, max_fps={}) layout={:?}；{}",
                            r.screen_index, s.name, s.builtin, s.scale, s.max_fps, r.layout, r.reason
                        ));
                    }
                    if let Some(s) = window.screen_info() {
                        self.log(&format!(
                            "  window is on {:?} builtin={} scale={} max_fps={}",
                            s.name, s.builtin, s.scale, s.max_fps
                        ));
                    }
                }
                Event::Command(id) => {
                    self.log(&format!("{event:?}"));
                    match *id {
                        CMD_LIGHT | CMD_DARK => {
                            // 单选勾选：只是演示菜单勾选，不改系统外观。
                            self.dark = *id == CMD_DARK;
                            self.sync_checks(window);
                            window.request_redraw();
                        }
                        CMD_CONTRAST => {
                            self.contrast = !self.contrast;
                            self.sync_checks(window);
                            window.request_redraw();
                        }
                        CMD_REDRAW => window.request_redraw(),
                        CMD_GROW => {
                            // 在 handle 里改窗口尺寸：AppKit 会同步回调 setFrameSize:，
                            // 平台层把 Resized/Redraw 排队，等本次 handle 返回后再投递。
                            let s = window.size();
                            self.log("  CMD_GROW：调用 set_content_size 之前");
                            window.set_content_size(tekes_ui_platform_macos::Size {
                                width: s.width + 20.0,
                                height: s.height + 10.0,
                            });
                            self.log("  CMD_GROW：set_content_size 已返回，Resized 应该排在后面");
                        }
                        CMD_TWICE => {
                            // 合并：连续两次请求只产生一次 Redraw。
                            window.request_redraw();
                            window.request_redraw();
                        }
                        _ => {}
                    }
                }
                Event::SystemAppearanceChanged {
                    dark,
                    increase_contrast,
                } => {
                    self.log(&format!("{event:?}"));
                    self.dark = *dark;
                    self.contrast = *increase_contrast;
                    self.sync_checks(window);
                    window.request_redraw();
                }
                Event::CloseRequested => {
                    self.log(&format!("{event:?} -> close"));
                    window.close();
                }
                _ => self.log(&format!("{event:?}")),
            }
        }
    }

    impl Drop for Logger {
        fn drop(&mut self) {
            self.log(&format!(
                "退出：Redraw 共 {} 次，mouseMoved 共 {} 次",
                self.redraws, self.moves
            ));
        }
    }

    fn menu_bar() -> MenuBar {
        let view = Menu::new(
            "View",
            vec![
                MenuItem::Command(
                    MenuCommand::new("Light", CMD_LIGHT)
                        .key(KeyEquivalent::command('1'))
                        .checked(true),
                ),
                MenuItem::Command(
                    MenuCommand::new("Dark", CMD_DARK).key(KeyEquivalent::command('2')),
                ),
                MenuItem::Separator,
                MenuItem::Command(MenuCommand::new("Increase Contrast", CMD_CONTRAST).key(
                    KeyEquivalent::new(
                        '3',
                        Modifiers {
                            command: true,
                            shift: true,
                            ..Modifiers::NONE
                        },
                    ),
                )),
            ],
        );
        let debug = Menu::new(
            "Debug",
            vec![
                MenuItem::Command(
                    MenuCommand::new("Request Redraw", CMD_REDRAW).key(KeyEquivalent::command('r')),
                ),
                MenuItem::Command(MenuCommand::new("Request Redraw Twice", CMD_TWICE)),
                MenuItem::Command(MenuCommand::new("Grow Window (reentrancy)", CMD_GROW)),
            ],
        );
        MenuBar {
            menus: vec![view, debug],
        }
    }

    pub fn main() {
        let selftest = std::env::var("TEKES_SELFTEST").ok();
        // 摆放规则（开发规则）：有外接显示器放外接屏；否则放内建屏右半边；
        // 自测会缩放窗口，属于尺寸类检查，免除右半边规则。可用环境变量覆盖，见 placement 模块。
        let window_config = WindowConfig {
            placement: WindowPlacement::Auto {
                need_120hz: false,
                size_check: selftest.is_some(),
            },
            title: "TekesUI window example".into(),
            min_size: Some(tekes_ui_platform_macos::Size {
                width: 320.0,
                height: 200.0,
            }),
            frame_autosave_name: std::env::var("TEKES_AUTOSAVE")
                .is_ok()
                .then(|| "TekesUIWindowExample".to_string()),
            ..WindowConfig::default()
        };
        let logger = Logger {
            start: Instant::now(),
            last_click: Point { x: 480.0, y: 320.0 },
            redraws: 0,
            moves: 0,
            log_moves: std::env::var("TEKES_LOG_MOVES").is_ok(),
            dark: false,
            contrast: false,
            busy: false,
        };
        eprintln!("pid {}", std::process::id());
        if let Some(mode) = selftest {
            selftest::spawn(mode == "hold");
        }
        App::new(AppConfig::default()).run(window_config, menu_bar(), logger);
        eprintln!("run() 已返回");
    }

    /// 自测：从后台线程按时间表向主队列投递步骤，步骤在主线程里直接操作 AppKit，
    /// 与 osascript/真实输入走同一批 AppKit 入口。仅在自测模式使用，不影响空闲行为。
    mod selftest {
        use std::ffi::c_void;
        use std::time::Duration;

        use objc2::rc::Retained;
        use objc2_app_kit::{
            NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSApplication,
            NSEvent, NSEventModifierFlags, NSEventType, NSScreen, NSWindow,
        };
        use objc2_core_graphics::{CGEvent, CGEventField, CGScrollEventUnit};
        use objc2_foundation::{MainThreadMarker, NSPoint, NSSize, NSString};

        unsafe extern "C" {
            static _dispatch_main_q: c_void;
            fn dispatch_async_f(
                queue: *const c_void,
                ctx: *mut c_void,
                work: extern "C" fn(*mut c_void),
            );
        }

        #[derive(Clone, Copy, Debug)]
        enum Step {
            Click {
                x: f64,
                y: f64,
                count: isize,
            },
            Drag {
                x: f64,
                y: f64,
            },
            Key {
                chars: &'static str,
                command: bool,
            },
            Scroll {
                dy: i32,
                phase: i64,
                momentum: i64,
            },
            Menu {
                tag: isize,
            },
            Resize {
                w: f64,
                h: f64,
            },
            Dark(bool),
            /// 给层塞一张 200x200 像素的图（上半红、下半蓝），验证 contentsGravity：
            /// 窗口放大后图应钉在左上角、不拉伸、上红下蓝。
            StaleImage,
            Close,
            #[allow(dead_code)]
            Note(&'static str),
        }

        extern "C" fn run_step(ctx: *mut c_void) {
            // SAFETY: ctx 是 spawn 里 Box::into_raw 出来的 Step，每个只消费一次。
            let step = *unsafe { Box::from_raw(ctx.cast::<Step>()) };
            let mtm = MainThreadMarker::new().expect("dispatch_async_f 到主队列，必在主线程");
            eprintln!("--- selftest: {step:?}");
            perform(mtm, step);
        }

        fn window(mtm: MainThreadMarker) -> Option<Retained<NSWindow>> {
            NSApplication::sharedApplication(mtm)
                .windows()
                .firstObject()
        }

        /// 内容区左上角坐标 → 窗口坐标（左下角为原点）。
        fn win_point(w: &NSWindow, x: f64, y_top: f64) -> NSPoint {
            let h = w
                .contentView()
                .map(|v| v.frame().size.height)
                .unwrap_or(0.0);
            NSPoint::new(x, h - y_top)
        }

        fn mouse(mtm: MainThreadMarker, ty: NSEventType, x: f64, y: f64, count: isize) {
            let Some(w) = window(mtm) else { return };
            let e = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                ty,
                win_point(&w, x, y),
                NSEventModifierFlags::empty(),
                0.0,
                w.windowNumber(),
                None,
                0,
                count,
                1.0,
            );
            if let Some(e) = e {
                NSApplication::sharedApplication(mtm).sendEvent(&e);
            }
        }

        fn key(mtm: MainThreadMarker, ty: NSEventType, chars: &str, command: bool) {
            let Some(w) = window(mtm) else { return };
            let flags = if command {
                NSEventModifierFlags::Command
            } else {
                NSEventModifierFlags::empty()
            };
            let s = NSString::from_str(chars);
            let e = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                ty, NSPoint::new(0.0, 0.0), flags, 0.0, w.windowNumber(), None, &s, &s, false, 0,
            );
            if let Some(e) = e {
                NSApplication::sharedApplication(mtm).sendEvent(&e);
            }
        }

        fn perform(mtm: MainThreadMarker, step: Step) {
            let app = NSApplication::sharedApplication(mtm);
            let Some(w) = window(mtm) else { return };
            match step {
                Step::Note(_) => {}
                Step::Click { x, y, count } => {
                    mouse(mtm, NSEventType::LeftMouseDown, x, y, count);
                    mouse(mtm, NSEventType::LeftMouseUp, x, y, count);
                }
                Step::Drag { x, y } => mouse(mtm, NSEventType::LeftMouseDragged, x, y, 1),
                Step::Key { chars, command } => {
                    key(mtm, NSEventType::KeyDown, chars, command);
                    key(mtm, NSEventType::KeyUp, chars, command);
                }
                Step::Scroll {
                    dy,
                    phase,
                    momentum,
                } => {
                    let ev = CGEvent::new_scroll_wheel_event2(
                        None,
                        CGScrollEventUnit::Pixel,
                        1,
                        dy,
                        0,
                        0,
                    );
                    if let Some(ev) = ev.as_deref() {
                        // 事件位置放在窗口内容区中央（CG 全局坐标，左上角为原点）。
                        let center = win_point(&w, 200.0, 150.0);
                        let screen = w.convertPointToScreen(center);
                        let primary_h = NSScreen::screens(mtm)
                            .firstObject()
                            .map(|s| s.frame().size.height)
                            .unwrap_or(0.0);
                        CGEvent::set_location(
                            Some(ev),
                            objc2_core_foundation::CGPoint::new(screen.x, primary_h - screen.y),
                        );
                        CGEvent::set_integer_value_field(
                            Some(ev),
                            CGEventField::ScrollWheelEventIsContinuous,
                            1,
                        );
                        CGEvent::set_integer_value_field(
                            Some(ev),
                            CGEventField::ScrollWheelEventScrollPhase,
                            phase,
                        );
                        CGEvent::set_integer_value_field(
                            Some(ev),
                            CGEventField::ScrollWheelEventMomentumPhase,
                            momentum,
                        );
                        let num = w.windowNumber() as i64;
                        CGEvent::set_integer_value_field(
                            Some(ev),
                            CGEventField::MouseEventWindowUnderMousePointer,
                            num,
                        );
                        CGEvent::set_integer_value_field(
                            Some(ev),
                            CGEventField::MouseEventWindowUnderMousePointerThatCanHandleThisEvent,
                            num,
                        );
                        if let Some(e) = NSEvent::eventWithCGEvent(ev) {
                            // 合成的滚轮 NSEvent 没有 window，走不了 sendEvent 的命中测试，
                            // 直接送给内容视图（位置字段因此不可信，这里只验证增量与阶段）。
                            if let Some(v) = w.contentView() {
                                v.scrollWheel(&e);
                            }
                        }
                    }
                }
                Step::Menu { tag } => {
                    if let Some(main) = app.mainMenu() {
                        for top in main.itemArray().iter() {
                            if let Some(sub) = top.submenu() {
                                for item in sub.itemArray().iter() {
                                    if item.tag() == tag && item.action().is_some() {
                                        // SAFETY: item 的 action/target 来自平台层。
                                        unsafe {
                                            app.sendAction_to_from(
                                                item.action().unwrap(),
                                                item.target().as_deref(),
                                                Some(&item),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Step::Resize { w: nw, h: nh } => w.setContentSize(NSSize::new(nw, nh)),
                Step::Dark(on) => {
                    // 只改这一个窗口的外观，不碰系统设置；视图的 effectiveAppearance 随之变化。
                    let a = if on {
                        NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua })
                    } else {
                        None
                    };
                    w.setAppearance(a.as_deref());
                }
                Step::StaleImage => stale_image(&w),
                Step::Close => w.performClose(None),
            }
        }

        unsafe extern "C" {
            fn CGBitmapContextCreate(
                data: *mut c_void,
                width: usize,
                height: usize,
                bits_per_component: usize,
                bytes_per_row: usize,
                space: *const c_void,
                bitmap_info: u32,
            ) -> *mut c_void;
        }

        fn stale_image(w: &NSWindow) {
            use objc2::runtime::AnyObject;
            use objc2_core_foundation::{CGPoint, CGRect, CGSize};
            use objc2_core_graphics::{
                CGBitmapContextCreateImage, CGColorSpace, CGContext, kCGColorSpaceSRGB,
            };
            let Some(layer) = w.contentView().and_then(|v| v.layer()) else {
                return;
            };
            let Some(space) = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB })) else {
                return;
            };
            // SAFETY: 参数合法；data 为空表示由 CG 分配；返回的 CGContext 在本函数内用完后泄漏一次，
            // 仅自测用，不影响结论。
            let ctx = unsafe {
                CGBitmapContextCreate(
                    std::ptr::null_mut(),
                    200,
                    200,
                    8,
                    0,
                    (&*space as *const CGColorSpace).cast(),
                    0x2002, // premultipliedFirst | byteOrder32Little（BGRA）
                )
            };
            // SAFETY: 上面创建成功的 CGContext。
            let Some(ctx) = (unsafe { ctx.cast::<CGContext>().as_ref() }) else {
                return;
            };
            // CG 坐标原点在左下：y 100..200 是图的上半。
            CGContext::set_rgb_fill_color(Some(ctx), 0.9, 0.1, 0.1, 1.0);
            CGContext::fill_rect(
                Some(ctx),
                CGRect::new(CGPoint::new(0.0, 100.0), CGSize::new(200.0, 100.0)),
            );
            CGContext::set_rgb_fill_color(Some(ctx), 0.1, 0.2, 0.9, 1.0);
            CGContext::fill_rect(
                Some(ctx),
                CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(200.0, 100.0)),
            );
            if let Some(img) = CGBitmapContextCreateImage(Some(ctx)) {
                let obj: &AnyObject = unsafe { &*(&*img as *const _ as *const AnyObject) };
                eprintln!(
                    "    layer.isGeometryFlipped={} contentsGravity={}",
                    layer.isGeometryFlipped(),
                    layer.contentsGravity()
                );
                // SAFETY: CGImage 是合法的 CALayer contents。
                unsafe { layer.setContents(Some(obj)) };
            }
        }

        pub fn spawn(hold: bool) {
            let mut steps: Vec<(u64, Step)> = vec![
                (800, Step::Note("基线：此前应只有启动时的 Redraw")),
                (
                    1200,
                    Step::Click {
                        x: 100.0,
                        y: 50.0,
                        count: 1,
                    },
                ),
                (
                    1500,
                    Step::Click {
                        x: 100.0,
                        y: 50.0,
                        count: 2,
                    },
                ),
                (1800, Step::Drag { x: 200.0, y: 150.0 }),
                (
                    2100,
                    Step::Key {
                        chars: "a",
                        command: false,
                    },
                ),
                (
                    2400,
                    Step::Key {
                        chars: "1",
                        command: true,
                    },
                ),
                // 滚轮阶段用 CGEvent 的取值：scroll 1 开始、2 继续、4 结束；momentum 1 开始、2 继续、3 结束。
                (
                    2700,
                    Step::Scroll {
                        dy: 12,
                        phase: 1,
                        momentum: 0,
                    },
                ),
                (
                    2800,
                    Step::Scroll {
                        dy: 8,
                        phase: 2,
                        momentum: 0,
                    },
                ),
                (
                    2900,
                    Step::Scroll {
                        dy: 0,
                        phase: 4,
                        momentum: 0,
                    },
                ),
                (
                    3000,
                    Step::Scroll {
                        dy: 5,
                        phase: 0,
                        momentum: 1,
                    },
                ),
                (
                    3100,
                    Step::Scroll {
                        dy: 3,
                        phase: 0,
                        momentum: 2,
                    },
                ),
                (
                    3200,
                    Step::Scroll {
                        dy: 0,
                        phase: 0,
                        momentum: 3,
                    },
                ),
                (3300, Step::Menu { tag: 11 }),
                (3500, Step::Menu { tag: 12 }),
                (3600, Step::StaleImage),
                (3700, Step::Resize { w: 700.0, h: 450.0 }),
            ];
            // 模拟实时缩放：每 20ms 一步，共 20 步。
            for i in 1..=20 {
                steps.push((
                    4200 + i * 20,
                    Step::Resize {
                        w: 700.0 + i as f64 * 5.0,
                        h: 450.0 + i as f64 * 2.0,
                    },
                ));
            }
            steps.push((5000, Step::Dark(true)));
            if !hold {
                steps.push((5500, Step::Dark(false)));
                steps.push((6000, Step::Close));
            }
            std::thread::spawn(move || {
                let mut last = 0;
                for (t, step) in steps {
                    std::thread::sleep(Duration::from_millis(t - last));
                    last = t;
                    let ctx = Box::into_raw(Box::new(step)).cast::<c_void>();
                    // SAFETY: 主队列在整个进程生命周期内有效；run_step 在主线程消费 ctx。
                    unsafe { dispatch_async_f(&raw const _dispatch_main_q, ctx, run_step) };
                }
            });
        }
    }
}
