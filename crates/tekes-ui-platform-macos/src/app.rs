//! NSApplication、应用委托与 `run()`。

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSApplicationTerminateReply, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType,
    NSMenuItem, NSWindow, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowStyleMask,
    NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSProcessInfo, NSRect,
    NSSize, NSString,
};

use crate::config::{AppConfig, WindowConfig};
use crate::event::Event;
use crate::menu::{CommandId, MenuBar};
use crate::menu_build;
use crate::shared::{Objects, Shared, WindowHandler};
use crate::view::TekesView;

#[derive(Debug)]
pub(crate) struct DelegateIvars {
    shared: Rc<Shared>,
    app_config: AppConfig,
    window_config: WindowConfig,
    menu_bar: MenuBar,
}

define_class!(
    // SAFETY: NSObject 没有子类化要求，AppDelegate 不实现 Drop。
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    #[derive(Debug)]
    pub(crate) struct AppDelegate;

    // SAFETY: NSObjectProtocol 没有安全要求。
    unsafe impl NSObjectProtocol for AppDelegate {}

    // SAFETY: NSApplicationDelegate 没有安全要求，下面的签名与协议一致。
    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let _ = notification;
            self.launch();
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _app: &NSApplication) -> bool {
            self.ivars().app_config.quit_after_last_window_closed
        }

        /// 不让 AppKit 调 `exit()`，而是停掉事件循环，让 `run()` 正常返回。
        /// 这样调用方（示例、测试）可以在 `run()` 之后收尾，处理器也能正常 Drop。
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, app: &NSApplication) -> NSApplicationTerminateReply {
            stop_run_loop(app);
            NSApplicationTerminateReply::TerminateCancel
        }
    }

    // SAFETY: NSWindowDelegate 没有安全要求，签名与协议一致。
    unsafe impl NSWindowDelegate for AppDelegate {
        /// 红色按钮、⌘W（performClose:）都会来这里。平台层不替引擎决定：
        /// 投递 `CloseRequested` 并拒绝关闭，引擎要关就调 `Window::close`。
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            self.ivars().shared.dispatch(Event::CloseRequested);
            false
        }

        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            // 之后不再投递事件。是否结束应用由 applicationShouldTerminateAfterLastWindowClosed: 决定。
            self.ivars().shared.closed.set(true);
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            self.ivars().shared.dispatch(Event::Focus(true));
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSNotification) {
            self.ivars().shared.dispatch(Event::Focus(false));
        }
    }

    impl AppDelegate {
        /// 所有引擎命令菜单项的 action，`tag` 是 `CommandId`。
        #[unsafe(method(menuItemSelected:))]
        fn menu_item_selected(&self, sender: &NSMenuItem) {
            let id = sender.tag();
            if let Ok(id) = u32::try_from(id) {
                self.ivars().shared.dispatch(Event::Command(CommandId(id)));
            }
        }

        /// NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification。
        #[unsafe(method(accessibilityDisplayOptionsChanged:))]
        fn accessibility_display_options_changed(&self, _notification: &NSNotification) {
            self.ivars().shared.check_appearance(false);
        }
    }
);

impl AppDelegate {
    fn new(
        mtm: MainThreadMarker,
        shared: Rc<Shared>,
        app_config: AppConfig,
        window_config: WindowConfig,
        menu_bar: MenuBar,
    ) -> Retained<AppDelegate> {
        let this = Self::alloc(mtm).set_ivars(DelegateIvars {
            shared,
            app_config,
            window_config,
            menu_bar,
        });
        // SAFETY: NSObject 的 init 签名正确。
        unsafe { msg_send![super(this), init] }
    }

    /// `applicationDidFinishLaunching:` 里做的全部事情：菜单、窗口、视图、起始事件。
    ///
    /// 起始事件顺序固定为：`Created`、`SystemAppearanceChanged`、`Resized`、`Redraw`，
    /// 全部在窗口显示之前同步投递，所以窗口第一次出现时第一帧已经画好。
    fn launch(&self) {
        let mtm = self.mtm();
        let ivars = self.ivars();
        let shared = &ivars.shared;
        let wc = &ivars.window_config;
        let app = NSApplication::sharedApplication(mtm);

        let app_name = ivars
            .app_config
            .name
            .clone()
            .unwrap_or_else(|| NSProcessInfo::processInfo().processName().to_string());
        let target: &AnyObject = self.as_ref();
        menu_build::install(mtm, &app_name, &ivars.menu_bar, target);

        let rect = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(wc.size.width, wc.size.height),
        );
        // SAFETY: 下面关闭 releasedWhenClosed，由 Retained 管理生命周期。
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect,
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: 窗口不是由窗口控制器创建的，必须关掉关闭时自动 release。
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(&wc.title));
        // 全屏菜单项 toggleFullScreen: 需要这个行为才有效。
        window.setCollectionBehavior(NSWindowCollectionBehavior::FullScreenPrimary);
        if let Some(min) = wc.min_size {
            window.setContentMinSize(NSSize::new(min.width, min.height));
        }
        window.setDelegate(Some(ProtocolObject::from_ref(self)));

        let view = TekesView::new(mtm, shared.clone(), rect);
        window.setContentView(Some(&view));
        window.makeFirstResponder(Some(&view));

        // 位置：有自动保存的位置就恢复，否则按配置居中。
        let mut restored = false;
        if let Some(name) = &wc.frame_autosave_name {
            let name = NSString::from_str(name);
            restored = window.setFrameUsingName(&name);
            window.setFrameAutosaveName(&name);
        }
        let mut report = None;
        if !restored {
            report = crate::screens::place(mtm, &window, wc.placement);
        }

        let _ = shared.objects.set(Objects {
            window: window.clone(),
            view: view.clone(),
        });
        // 强制创建背衬层，使 `Created` 时 `metal_layer()` 可用。
        let _ = view.metal_layer();

        // 工作区通知：增强对比度等辅助功能显示选项变化。
        // SAFETY: observer 是委托，selector 签名为 (NSNotification)，通知名是 AppKit 常量。
        unsafe {
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .addObserver_selector_name_object(
                    target,
                    sel!(accessibilityDisplayOptionsChanged:),
                    Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                    None,
                );
        }

        *shared.placement.borrow_mut() = report;
        shared.ready.set(true);
        shared.dispatch(Event::Created);
        shared.check_appearance(true);
        view.sync_geometry(true);

        window.makeKeyAndOrderFront(None);
        // 未打包的可执行文件需要显式激活，否则窗口出现在别的应用后面。
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
    }
}

/// 让 `[NSApp run]` 返回。`stop:` 只在处理完一个事件后才生效，所以补发一个空事件唤醒循环。
fn stop_run_loop(app: &NSApplication) {
    app.stop(None);
    let event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::ApplicationDefined,
        NSPoint::new(0.0, 0.0),
        NSEventModifierFlags::empty(),
        0.0,
        0,
        None,
        0,
        0,
        0,
    );
    if let Some(event) = event {
        app.postEvent_atStart(&event, true);
    }
}

/// 应用入口。只能在主线程创建。
#[derive(Debug)]
pub struct App {
    config: AppConfig,
    mtm: MainThreadMarker,
}

impl App {
    /// 创建应用。必须在主线程调用，否则 panic。
    pub fn new(config: AppConfig) -> App {
        let mtm = MainThreadMarker::new().expect("App 只能在主线程创建");
        App { config, mtm }
    }

    /// 创建窗口并进入事件循环，直到应用结束（⌘Q，或配置了最后一个窗口关闭即退出时关窗）。
    ///
    /// 事件循环空闲时进程阻塞在 `mach_msg`，不消耗 CPU。
    /// `menu_bar` 为引擎的菜单；标准应用菜单与窗口菜单自动添加。
    ///
    /// # Panics
    ///
    /// 菜单描述不合法（见 `MenuBar::validate`）时 panic。
    pub fn run(self, window_config: WindowConfig, menu_bar: MenuBar, handler: impl WindowHandler) {
        if let Err(e) = menu_bar.validate() {
            panic!("菜单描述不合法：{e}");
        }
        let mtm = self.mtm;
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        let shared = Shared::new(mtm, Box::new(handler));
        let delegate = AppDelegate::new(mtm, shared.clone(), self.config, window_config, menu_bar);
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        app.run();

        // run() 返回之后：丢弃处理器，断开委托。
        app.setDelegate(None);
        shared.finish();
    }
}
