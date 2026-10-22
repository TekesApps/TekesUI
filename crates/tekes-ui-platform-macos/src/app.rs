//! NSApplication、应用委托与 `run()`。

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
thread_local! { static WINDOWS:RefCell<Vec<Weak<Shared>>>=const {RefCell::new(Vec::new())}; }
thread_local! {static WINDOW_OWNERS:RefCell<Vec<WindowHandle>>=const {RefCell::new(Vec::new())};}
fn windows() -> Vec<Rc<Shared>> {
    WINDOWS.with(|w| {
        let mut w = w.borrow_mut();
        w.retain(|s| s.upgrade().is_some_and(|s| !s.closed.get()));
        w.iter().filter_map(Weak::upgrade).collect()
    })
}

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAnimationContext, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSApplicationTerminateReply, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType,
    NSMenuItem, NSWindow, NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowDelegate,
    NSWindowOcclusionState, NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility,
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

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &NSNotification) {
            for shared in windows() {
                let active=shared.objects.get().is_some_and(|o|o.window.isKeyWindow());
                shared.dispatch(Event::Focus(active));
            }
        }

        #[unsafe(method(applicationDidResignActive:))]
        fn did_resign_active(&self, _notification: &NSNotification) {
            for shared in windows() {shared.dispatch(Event::Focus(false));}
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _app: &NSApplication) -> bool {
            self.ivars().app_config.quit_after_last_window_closed
        }

        /// 不让 AppKit 调 `exit()`，而是停掉事件循环，让 `run()` 正常返回。
        /// 这样调用方（示例、测试）可以在 `run()` 之后收尾，处理器也能正常 Drop。
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, app: &NSApplication) -> NSApplicationTerminateReply {
            let blocked:Vec<_>=windows().into_iter().filter(|s|!s.allows_app_quit()).collect();
            if !blocked.is_empty() {
                for shared in blocked {
                    shared.dispatch(Event::QuitRequested);
                    if let Some(o)=shared.objects.get(){o.window.makeKeyAndOrderFront(None);}
                }
                return NSApplicationTerminateReply::TerminateCancel;
            }
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
            let shared = &self.ivars().shared;
            crate::wake::native::unregister(&shared.wake_state);
            shared.closed.set(true);
            restore_windows(shared);
            shared.titlebar_drag_origin.borrow_mut().take();
            shared.animating.set(false);
            if let Some(o) = shared.objects.get() {
                o.view.invalidate_display_link();
            }
        }

        /// 窗口重新可见（最小化后恢复、被盖住后露出来）时请求重绘。
        /// 不可见期间渲染层的取 drawable 会返回 Occluded 并跳过帧，不补这一帧会停在旧画面。
        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn window_did_change_occlusion_state(&self, notification: &NSNotification) {
            let visible = notification
                .object()
                .and_then(|o| o.downcast::<NSWindow>().ok())
                .is_some_and(|w| w.occlusionState().contains(NSWindowOcclusionState::Visible));
            if visible {
                self.ivars().shared.request_redraw();
            }
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            let shared = &self.ivars().shared;
            let active = NSApplication::sharedApplication(shared.mtm).isActive();
            shared.dispatch(Event::Focus(active));
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
                let owner=windows().into_iter().find(|s|s.objects.get().is_some_and(|o|o.window.isKeyWindow())).unwrap_or_else(||self.ivars().shared.clone());
                owner.dispatch(Event::Command(CommandId(id)));
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
        self.launch_window(true, None);
    }
    fn launch_window(&self, install_menu: bool, exclusive_anchor: Option<&Rc<Shared>>) {
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
        if install_menu {
            menu_build::install(mtm, &app_name, &ivars.menu_bar, target);
        }
        WINDOWS.with(|w| w.borrow_mut().push(Rc::downgrade(shared)));

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
                    | NSWindowStyleMask::Resizable
                    | if wc.unified_titlebar {
                        NSWindowStyleMask::FullSizeContentView
                    } else {
                        NSWindowStyleMask::empty()
                    },
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: 窗口不是由窗口控制器创建的，必须关掉关闭时自动 release。
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(&wc.title));
        if wc.unified_titlebar {
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
            // TekesUI owns the complete typography-scaled top row. An empty
            // NSToolbar would add a second AppKit height allocation (66pt).
            // Keep genuine native window controls in the full-size content view.
        }
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

        // Position before the first show: never expose a centered intermediate
        // Settings window and then visibly move it over its owning window.
        if let Some(anchor) = exclusive_anchor.and_then(|s| s.objects.get()) {
            window.setAnimationBehavior(NSWindowAnimationBehavior::None);
            window.setFrame_display(anchor.window.frame(), false);
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
        // Publish initial keyboard ownership before the first geometry/frame;
        // hosts must not assume a newly created background window is active.
        shared.dispatch(Event::Focus(app.isActive() && window.isKeyWindow()));
        shared.check_appearance(true);
        view.sync_geometry(true);

        if let Some(anchor) = exclusive_anchor {
            present_exclusive(shared, anchor);
        } else {
            window.makeKeyAndOrderFront(None);
        }
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
        for window in windows() {
            window.finish();
        }
        let handles = WINDOW_OWNERS.with(|w| std::mem::take(&mut *w.borrow_mut()));
        drop(handles);
        shared.finish();
    }
}

/// Owned auxiliary window in the existing NSApplication/event loop. No nested run().
#[derive(Clone, Debug)]
pub struct WindowHandle {
    _owner: Rc<WindowOwner>,
    pub(crate) shared: Rc<Shared>,
}
#[derive(Debug)]
struct WindowOwner {
    delegate: Retained<AppDelegate>,
    shared: Rc<Shared>,
}
impl WindowHandle {
    pub fn is_closed(&self) -> bool {
        self.shared.closed.get()
    }
    pub fn show(&self) {
        if !self.is_closed()
            && let Some(objects) = self.shared.objects.get()
        {
            objects.window.makeKeyAndOrderFront(None);
            self.shared.request_redraw();
        }
    }
}
impl Drop for WindowOwner {
    fn drop(&mut self) {
        // SAFETY: the retained delegate remains alive through observer removal.
        unsafe {
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .removeObserver(&self.delegate);
        }
        if let Some(objects) = self.shared.objects.get() {
            objects.window.setDelegate(None);
            objects.window.close();
        }
        self.shared.finish();
    }
}
pub(crate) fn open_window(
    mtm: MainThreadMarker,
    config: WindowConfig,
    handler: impl WindowHandler,
    exclusive_anchor: Option<&Rc<Shared>>,
) -> WindowHandle {
    // Retire closed windows outside a registry borrow; native release may reenter.
    let retired = WINDOW_OWNERS.with(|windows| {
        let handles = std::mem::take(&mut *windows.borrow_mut());
        let (live, closed): (Vec<_>, Vec<_>) = handles.into_iter().partition(|h| !h.is_closed());
        *windows.borrow_mut() = live;
        closed
    });
    drop(retired);
    let shared = Shared::new(mtm, Box::new(handler));
    let delegate = AppDelegate::new(
        mtm,
        shared.clone(),
        AppConfig::default(),
        config,
        MenuBar::default(),
    );
    delegate.launch_window(false, exclusive_anchor);
    let handle = WindowHandle {
        _owner: Rc::new(WindowOwner {
            delegate,
            shared: shared.clone(),
        }),
        shared,
    };
    // App lifetime ownership lets Independent windows survive their opener closing.
    WINDOW_OWNERS.with(|windows| windows.borrow_mut().push(handle.clone()));
    handle
}

pub(crate) fn present_exclusive(child: &Rc<Shared>, anchor: &Rc<Shared>) {
    let Some(objects) = child.objects.get() else {
        return;
    };
    if let Some(original) = anchor.objects.get() {
        objects
            .window
            .setFrame_display(original.window.frame(), true);
    }
    let mut hidden = Vec::new();
    let mut key = None;
    for shared in windows() {
        if Rc::ptr_eq(&shared, child) {
            continue;
        }
        if let Some(o) = shared.objects.get()
            && o.window.isVisible()
            && !o.window.isMiniaturized()
        {
            if o.window.isKeyWindow() || Rc::ptr_eq(&shared, anchor) {
                key = Some(Rc::downgrade(&shared));
            }
            hidden.push(Rc::downgrade(&shared));
        }
    }
    // Show the initialized cover first, then hide its backdrop in the same
    // nonanimated AppKit group. Moving Settings must never reveal Main.
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(0.);
    NSAnimationContext::currentContext().setAllowsImplicitAnimation(false);
    objects.window.makeKeyAndOrderFront(None);
    for shared in hidden.iter().filter_map(|w| w.upgrade()) {
        if let Some(o) = shared.objects.get() {
            let animation = o.window.animationBehavior();
            o.window
                .setAnimationBehavior(NSWindowAnimationBehavior::None);
            o.window.orderOut(None);
            o.window.setAnimationBehavior(animation);
        }
    }
    NSAnimationContext::endGrouping();
    *child.exclusive_restore.borrow_mut() = hidden;
    *child.exclusive_key.borrow_mut() = key;
}
pub(crate) fn restore_windows(owner: &Shared) {
    let hidden = std::mem::take(&mut *owner.exclusive_restore.borrow_mut());
    let key = owner
        .exclusive_key
        .borrow_mut()
        .take()
        .and_then(|w| w.upgrade());
    if hidden.is_empty() && key.is_none() {
        return;
    }
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(0.);
    NSAnimationContext::currentContext().setAllowsImplicitAnimation(false);
    for shared in hidden
        .into_iter()
        .filter_map(|w| w.upgrade())
        .filter(|s| !s.closed.get())
    {
        if let Some(o) = shared.objects.get() {
            let animation = o.window.animationBehavior();
            o.window
                .setAnimationBehavior(NSWindowAnimationBehavior::None);
            if let Some(cover) = owner.objects.get().filter(|o| o.window.isVisible()) {
                o.window.orderWindow_relativeTo(
                    NSWindowOrderingMode::Below,
                    cover.window.windowNumber(),
                );
            } else {
                o.window.orderFront(None);
            }
            o.window.setAnimationBehavior(animation);
        }
    }
    if let Some(shared) = key.filter(|s| !s.closed.get())
        && let Some(o) = shared.objects.get()
    {
        o.window.makeKeyWindow();
    }
    NSAnimationContext::endGrouping();
}
