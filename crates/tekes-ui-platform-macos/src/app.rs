//! NSApplication、应用委托与 `run()`。

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
thread_local! { static WINDOWS:RefCell<Vec<Weak<Shared>>>=const {RefCell::new(Vec::new())}; }
thread_local! {static WINDOW_OWNERS:RefCell<Vec<WindowHandle>>=const {RefCell::new(Vec::new())};}
thread_local! { static RETIRE_PENDING: Cell<bool> = const { Cell::new(false) }; }

unsafe extern "C" {
    static _dispatch_main_q: u8;
    fn dispatch_async_f(
        queue: *const std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: extern "C" fn(*mut std::ffi::c_void),
    );
}

extern "C" fn retire_closed_windows_callback(_ctx: *mut std::ffi::c_void) {
    crate::ffi_boundary("retire_closed_windows_callback", || {
        RETIRE_PENDING.with(|pending| pending.set(false));
        retire_closed_windows();
    });
}

pub(crate) fn retire_closed_windows() {
    // Retire closed windows outside a registry borrow; native release may reenter.
    let retired = WINDOW_OWNERS.with(|windows| {
        let handles = std::mem::take(&mut *windows.borrow_mut());
        let (live, closed): (Vec<_>, Vec<_>) = handles.into_iter().partition(|h| !h.is_closed());
        *windows.borrow_mut() = live;
        closed
    });
    if !retired.is_empty() {
        objc2::rc::autoreleasepool(|_| drop(retired));
    }
    let _ = windows();
}

fn schedule_retire_closed_windows() {
    RETIRE_PENDING.with(|pending| {
        if pending.replace(true) {
            return;
        }
        // SAFETY: `_dispatch_main_q` is the main dispatch queue; `retire_closed_windows_callback`
        // runs on the main thread and touches only main-thread thread-locals.
        unsafe {
            dispatch_async_f(
                (&raw const _dispatch_main_q).cast(),
                std::ptr::null_mut(),
                retire_closed_windows_callback,
            );
        }
    });
}
pub(crate) fn windows() -> Vec<Rc<Shared>> {
    WINDOWS.with(|w| {
        let mut w = w.borrow_mut();
        w.retain(|s| s.upgrade().is_some_and(|s| !s.closed.get()));
        w.iter().filter_map(Weak::upgrade).collect()
    })
}

#[cfg(feature = "selftest")]
pub(crate) fn primary_window_shared() -> Option<Rc<Shared>> {
    windows().into_iter().find(|s| s.primary.get())
}

pub(crate) fn active_window_shared(mtm: MainThreadMarker) -> Option<Rc<Shared>> {
    let live: Vec<_> = windows()
        .into_iter()
        .filter(|s| !s.is_inactive_surface())
        .collect();
    if live.is_empty() {
        return None;
    }
    let app = NSApplication::sharedApplication(mtm);
    if let Some(found) = live
        .iter()
        .find(|s| s.objects().is_some_and(|o| o.window.isKeyWindow()))
    {
        return Some(found.clone());
    }
    if let Some(main_win) = app.mainWindow()
        && let Some(found) = live.iter().find(|s| {
            s.objects()
                .is_some_and(|o| std::ptr::eq(&*o.window, &*main_win) && o.window.isVisible())
        })
    {
        return Some(found.clone());
    }
    for ordered in app.orderedWindows().iter() {
        if !ordered.isVisible() {
            continue;
        }
        if let Some(found) = live.iter().find(|s| {
            s.objects()
                .is_some_and(|o| std::ptr::eq(&*o.window, &*ordered))
        }) {
            return Some(found.clone());
        }
    }
    live.into_iter()
        .find(|s| s.objects().is_some_and(|o| o.window.isVisible()))
}

pub(crate) fn sync_close_menu_item(mtm: MainThreadMarker) {
    let title = active_window_shared(mtm)
        .and_then(|s| s.close_menu_title())
        .unwrap_or("Close Window");
    menu_build::set_close_menu_title(mtm, title);
}

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAnimationContext, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSApplicationTerminateReply, NSBackingStoreType, NSButton, NSEvent, NSEventModifierFlags,
    NSEventType, NSMenuItem, NSMenuItemValidation, NSToolbar, NSViewFrameDidChangeNotification,
    NSWindow, NSWindowAnimationBehavior, NSWindowButton, NSWindowCollectionBehavior,
    NSWindowDelegate, NSWindowOcclusionState, NSWindowOrderingMode, NSWindowStyleMask,
    NSWindowTitleVisibility, NSWindowToolbarStyle, NSWorkspace,
    NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSPoint,
    NSProcessInfo, NSRect, NSSize, NSString,
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
    toolbar_observers: RefCell<Vec<(Retained<NSButton>, bool)>>,
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
            crate::ffi_boundary("AppDelegate::did_finish_launching", || {
                let _ = notification;
                self.launch();
            });
        }

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::did_become_active", || {
                for shared in windows() {
                    if shared.is_inactive_surface() {
                        continue;
                    }
                    let active = shared.objects().is_some_and(|o| o.window.isKeyWindow());
                    shared.dispatch(Event::Focus(active));
                }
            });
        }

        #[unsafe(method(applicationDidResignActive:))]
        fn did_resign_active(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::did_resign_active", || {
                for shared in windows() {
                    if shared.is_inactive_surface() {
                        continue;
                    }
                    shared.dispatch(Event::Focus(false));
                }
            });
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &NSApplication, has_visible_windows: bool) -> bool {
            crate::ffi_boundary("AppDelegate::should_handle_reopen", || {
                if !has_visible_windows {
                    self.reopen_main_window();
                }
                true
            })
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _app: &NSApplication) -> bool {
            crate::ffi_boundary(
                "AppDelegate::should_terminate_after_last_window_closed",
                || self.ivars().app_config.quit_after_last_window_closed,
            )
        }

        /// 不让 AppKit 调 `exit()`，而是停掉事件循环，让 `run()` 正常返回。
        /// 这样调用方（示例、测试）可以在 `run()` 之后收尾，处理器也能正常 Drop。
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, app: &NSApplication) -> NSApplicationTerminateReply {
            crate::ffi_boundary("AppDelegate::should_terminate", || {
                let blocked: Vec<_> = windows()
                    .into_iter()
                    .filter(|s| !s.allows_app_quit())
                    .collect();
                if !blocked.is_empty() {
                    for shared in blocked {
                        if shared.dormant.get() && shared.primary.get() {
                            self.reopen_main_window();
                        }
                        shared.dispatch(Event::QuitRequested);
                        if let Some(o) = shared.objects() {
                            o.window.makeKeyAndOrderFront(None);
                        }
                    }
                    return NSApplicationTerminateReply::TerminateCancel;
                }
                stop_run_loop(app);
                NSApplicationTerminateReply::TerminateCancel
            })
        }
    }

    // SAFETY: NSMenuItemValidation 没有安全要求，签名与协议一致。
    unsafe impl NSMenuItemValidation for AppDelegate {
        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, menu_item: &NSMenuItem) -> bool {
            crate::ffi_boundary("AppDelegate::validate_menu_item", || {
                let mtm = self.mtm();
                let Some(action) = menu_item.action() else {
                    return true;
                };
                if action == menu_build::reopen_action() {
                    return !self.ivars().shared.closed.get();
                }
                if action == menu_build::close_shortcut_action() {
                    if let Some(target) = active_window_shared(mtm) {
                        let title = target.close_menu_title().unwrap_or("Close Window");
                        if menu_item.title().to_string() != title {
                            menu_item.setTitle(&NSString::from_str(title));
                        }
                        return true;
                    }
                    if menu_item.title().to_string() != "Close Window" {
                        menu_item.setTitle(&NSString::from_str("Close Window"));
                    }
                    return false;
                }
                if action == menu_build::command_action() {
                    return active_window_shared(mtm).is_some();
                }
                true
            })
        }
    }

    // SAFETY: NSWindowDelegate 没有安全要求，签名与协议一致。
    unsafe impl NSWindowDelegate for AppDelegate {
        /// 红色按钮、⌘W（performClose:）都会来这里。平台层不替引擎决定：
        /// 投递 `CloseRequested` 并拒绝关闭，引擎要关就调 `Window::close`。
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            crate::ffi_boundary("AppDelegate::window_should_close", || {
                self.ivars().shared.dispatch(Event::CloseRequested);
                false
            })
        }

        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::window_will_close", || {
                // 之后不再投递事件。是否结束应用由 applicationShouldTerminateAfterLastWindowClosed: 决定。
                self.stop_observing_toolbar_controls();
                let shared = &self.ivars().shared;
                restore_windows(shared);
                shared.titlebar_drag_origin.borrow_mut().take();
                shared.animating.set(false);
                shared.discard_metal_frames();
                if let Some(o) = shared.objects() {
                    o.view.invalidate_display_link();
                    o.view.cancel_pending_performs();
                }
                if shared.reopenable.get() && !shared.closed.get() {
                    if !shared.dormant.get() {
                        shared.dispatch(Event::Focus(false));
                    }
                    shared.dormant.set(true);
                    shared.revoke_accessibility();
                    shared.text_input.borrow_mut().take();
                    shared.redraw_pending.set(false);
                    shared.redraw_scheduled.set(false);
                    shared.discard_dormant_events();
                } else {
                    crate::wake::native::unregister(&shared.wake_state);
                    shared.closed.set(true);
                    shared.revoke_accessibility();
                    shared.text_input.borrow_mut().take();
                    schedule_retire_closed_windows();
                }
                sync_close_menu_item(shared.mtm);
            });
        }

        /// 窗口重新可见（最小化后恢复、被盖住后露出来）时请求重绘。
        /// 不可见期间渲染层的取 drawable 会返回 Occluded 并跳过帧，不补这一帧会停在旧画面。
        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn window_did_change_occlusion_state(&self, notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::window_did_change_occlusion_state", || {
                let shared = &self.ivars().shared;
                if shared.is_inactive_surface() {
                    return;
                }
                let visible = notification
                    .object()
                    .and_then(|o| o.downcast::<NSWindow>().ok())
                    .is_some_and(|w| w.occlusionState().contains(NSWindowOcclusionState::Visible));
                if visible {
                    shared.request_redraw();
                }
            });
        }

        // Window movement changes screen-space candidate geometry without
        // changing the original text state or the view-local caret rectangle.
        #[unsafe(method(windowDidMove:))]
        fn window_did_move(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::window_did_move", || {
                if let Some(objects) = self.ivars().shared.objects()
                    && let Some(context) = objects.view.inputContext()
                {
                    context.invalidateCharacterCoordinates();
                }
            });
        }

        // Apply initial and changed Toolbar measurements after native layout.
        // Subsequent button-frame resets are corrected synchronously below.
        #[unsafe(method(windowDidUpdate:))]
        fn window_did_update(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::window_did_update", || {
                let shared = &self.ivars().shared;
                if shared.is_inactive_surface() || shared.toolbar_row_height.get().is_none() {
                    return;
                }
                if let Some(objects) = shared.objects() {
                    objects.window.layoutIfNeeded();
                    shared.align_toolbar_controls();
                }
            });
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::window_did_become_key", || {
                let shared = &self.ivars().shared;
                if shared.is_inactive_surface() {
                    return;
                }
                let active = NSApplication::sharedApplication(shared.mtm).isActive();
                shared.dispatch(Event::Focus(active));
                sync_close_menu_item(shared.mtm);
            });
        }

        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::window_did_resign_key", || {
                let shared = &self.ivars().shared;
                if !shared.is_inactive_surface() {
                    shared.dispatch(Event::Focus(false));
                    sync_close_menu_item(shared.mtm);
                }
            });
        }
    }

    impl AppDelegate {
        #[unsafe(method(toolbarControlFrameChanged:))]
        fn toolbar_control_frame_changed(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::toolbar_control_frame_changed", || {
                self.ivars().shared.align_toolbar_controls();
            });
        }

        #[unsafe(method(tekesReopenMainWindow:))]
        fn reopen_menu_selected(&self, _sender: Option<&AnyObject>) {
            crate::ffi_boundary("AppDelegate::reopen_menu_selected", || {
                self.reopen_main_window();
            });
        }

        #[unsafe(method(tekesCloseShortcut:))]
        fn close_shortcut_selected(&self, _sender: Option<&AnyObject>) {
            crate::ffi_boundary("AppDelegate::close_shortcut_selected", || {
                if let Some(target) = active_window_shared(self.mtm()) {
                    target.dispatch_close_shortcut();
                }
            });
        }

        /// 所有引擎命令菜单项的 action，`tag` 是 `CommandId`。
        #[unsafe(method(menuItemSelected:))]
        fn menu_item_selected(&self, sender: &NSMenuItem) {
            crate::ffi_boundary("AppDelegate::menu_item_selected", || {
                let id = sender.tag();
                if let Ok(id) = u32::try_from(id)
                    && let Some(owner) = active_window_shared(self.mtm())
                {
                    owner.dispatch(Event::Command(CommandId(id)));
                }
            });
        }

        /// NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification。
        #[unsafe(method(accessibilityDisplayOptionsChanged:))]
        fn accessibility_display_options_changed(&self, _notification: &NSNotification) {
            crate::ffi_boundary("AppDelegate::accessibility_display_options_changed", || {
                self.ivars().shared.check_appearance(false);
            });
        }
    }
);

impl AppDelegate {
    pub(crate) fn new(
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
            toolbar_observers: RefCell::new(Vec::new()),
        });
        // SAFETY: NSObject 的 init 签名正确。
        unsafe { msg_send![super(this), init] }
    }

    pub(crate) fn observe_toolbar_controls(&self) {
        self.stop_observing_toolbar_controls();
        let shared = &self.ivars().shared;
        let Some(objects) = shared.objects() else {
            return;
        };
        if !objects
            .window
            .styleMask()
            .contains(NSWindowStyleMask::FullSizeContentView)
        {
            return;
        }
        for kind in [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ] {
            let Some(button) = objects.window.standardWindowButton(kind) else {
                continue;
            };
            let posts = button.postsFrameChangedNotifications();
            self.ivars()
                .toolbar_observers
                .borrow_mut()
                .push((button.clone(), posts));
            // SAFETY: main-thread delegate is retained by the window owner;
            // registration is scoped to this exact native button and removed
            // before close/owner destruction.
            unsafe {
                NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                    self,
                    sel!(toolbarControlFrameChanged:),
                    Some(NSViewFrameDidChangeNotification),
                    Some(&button),
                );
            }
            button.setPostsFrameChangedNotifications(true);
        }
    }

    pub(crate) fn teardown_window(&self, shared: &Shared) {
        // SAFETY: the retained delegate remains alive through observer removal.
        unsafe {
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .removeObserver(self);
        }
        self.stop_observing_toolbar_controls();
        if let Some(objects) = shared.objects() {
            let _ = objects.window.makeFirstResponder(None);
            objects.window.setDelegate(None);
            if !shared.closed.get() || objects.window.isVisible() {
                objects
                    .window
                    .setAnimationBehavior(NSWindowAnimationBehavior::None);
                objects.window.close();
            }
        }
        shared.finish();
    }

    pub(crate) fn stop_observing_toolbar_controls(&self) {
        let buttons = std::mem::take(&mut *self.ivars().toolbar_observers.borrow_mut());
        for (button, posts) in buttons {
            // SAFETY: the observer and observed button are both still retained.
            unsafe {
                NSNotificationCenter::defaultCenter().removeObserver_name_object(
                    self,
                    Some(NSViewFrameDidChangeNotification),
                    Some(&button),
                );
            }
            button.setPostsFrameChangedNotifications(posts);
        }
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
        // Let the window distribute the MouseMoved events requested by the
        // content view's full-viewport hover tracking area.
        window.setAcceptsMouseMovedEvents(true);
        // SAFETY: 窗口不是由窗口控制器创建的，必须关掉关闭时自动 release。
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(&wc.title));
        if wc.unified_titlebar {
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            window.setTitlebarAppearsTransparent(true);
            window.setToolbarStyle(NSWindowToolbarStyle::Unified);
            // AppKit owns the unified chrome and genuine traffic lights. Items are
            // rendered by TekesUI in the extended content view, not duplicated here.
            let toolbar = NSToolbar::initWithIdentifier(
                NSToolbar::alloc(mtm),
                &NSString::from_str("TekesUI.UnifiedChrome"),
            );
            window.setToolbar(Some(&toolbar));
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
        if let Some(anchor) = exclusive_anchor.and_then(|s| s.objects()) {
            window.setAnimationBehavior(NSWindowAnimationBehavior::None);
            window.setFrame_display(anchor.window.frame(), false);
        }
        shared.set_objects(Objects {
            window: window.clone(),
            view: view.clone(),
        });
        self.observe_toolbar_controls();
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
        sync_close_menu_item(mtm);
    }

    pub(crate) fn reopen_main_window(&self) -> bool {
        let mtm = self.mtm();
        let shared = &self.ivars().shared;
        if shared.closed.get() {
            return false;
        }
        let Some(objects) = shared.objects() else {
            return false;
        };
        for candidate in windows() {
            if Rc::ptr_eq(&candidate, shared) || candidate.is_inactive_surface() {
                continue;
            }
            let hides_primary = candidate
                .exclusive_restore
                .borrow()
                .iter()
                .any(|weak| weak.upgrade().is_some_and(|w| Rc::ptr_eq(&w, shared)));
            if hides_primary && let Some(cover) = candidate.objects() {
                if cover.window.isMiniaturized() {
                    cover.window.deminiaturize(None);
                }
                cover.window.makeKeyAndOrderFront(None);
                cover.window.makeFirstResponder(Some(&cover.view));
                let app = NSApplication::sharedApplication(mtm);
                #[allow(deprecated)]
                app.activateIgnoringOtherApps(true);
                candidate.request_redraw();
                sync_close_menu_item(mtm);
                return true;
            }
        }
        let app = NSApplication::sharedApplication(mtm);
        let was_dormant = shared.dormant.replace(false);
        if was_dormant {
            self.observe_toolbar_controls();
        } else if objects.window.isMiniaturized() {
            objects.window.deminiaturize(None);
        }
        objects.window.makeKeyAndOrderFront(None);
        objects.window.makeFirstResponder(Some(&objects.view));
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        shared.align_toolbar_controls();
        if was_dormant {
            shared.last_geometry.set(None);
            shared.check_appearance(false);
            objects.view.sync_geometry(true);
            if shared.pending_wake_while_dormant.replace(false) {
                shared.dispatch(Event::Wake);
            }
        }
        shared.request_redraw();
        sync_close_menu_item(mtm);
        true
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
        shared.primary.set(true);
        shared
            .reopenable
            .set(!self.config.quit_after_last_window_closed);
        let delegate = AppDelegate::new(mtm, shared.clone(), self.config, window_config, menu_bar);
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        app.run();

        // run() 返回之后：先销毁辅助窗口，再对主窗口执行与 WindowOwner::drop 对齐的收尾
        // （移除观察者、解绑 delegate、关闭 NSWindow、清空待执行回调并丢弃处理器），
        // 保证 delegate 析构晚于 window.delegate() 解绑且不泄漏主窗口。
        objc2::rc::autoreleasepool(|_| {
            app.setDelegate(None);
            menu_build::detach_target(mtm, delegate.as_ref());
            let handles = WINDOW_OWNERS.with(|w| std::mem::take(&mut *w.borrow_mut()));
            drop(handles);
            delegate.teardown_window(&shared);
            for window in windows() {
                window.finish();
            }
            let _ = windows();
            drop(delegate);
            drop(shared);
        });
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
            && let Some(objects) = self.shared.objects()
        {
            objects.window.makeKeyAndOrderFront(None);
            self.shared.request_redraw();
        }
    }
}
impl Drop for WindowOwner {
    fn drop(&mut self) {
        self.delegate.teardown_window(&self.shared);
    }
}
pub(crate) fn open_window(
    mtm: MainThreadMarker,
    config: WindowConfig,
    handler: impl WindowHandler,
    exclusive_anchor: Option<&Rc<Shared>>,
) -> WindowHandle {
    retire_closed_windows();
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
    let Some(objects) = child.objects() else {
        return;
    };
    if let Some(original) = anchor.objects() {
        objects
            .window
            .setFrame_display(original.window.frame(), true);
    }
    // Re-presenting a live cover must retain windows hidden by its first show.
    let mut hidden = child.exclusive_restore.borrow().clone();
    let mut key = None;
    for shared in windows() {
        if Rc::ptr_eq(&shared, child) || shared.is_inactive_surface() {
            continue;
        }
        if let Some(o) = shared.objects()
            && o.window.isVisible()
            && !o.window.isMiniaturized()
        {
            if o.window.isKeyWindow() || Rc::ptr_eq(&shared, anchor) {
                key = Some(Rc::downgrade(&shared));
            }
            if !hidden
                .iter()
                .any(|saved| saved.ptr_eq(&Rc::downgrade(&shared)))
            {
                hidden.push(Rc::downgrade(&shared));
            }
        }
    }
    // Show the initialized cover first, then hide its backdrop in the same
    // nonanimated AppKit group. Moving Settings must never reveal Main.
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(0.);
    NSAnimationContext::currentContext().setAllowsImplicitAnimation(false);
    objects.window.makeKeyAndOrderFront(None);
    for shared in hidden.iter().filter_map(|w| w.upgrade()) {
        if let Some(o) = shared.objects() {
            let animation = o.window.animationBehavior();
            o.window
                .setAnimationBehavior(NSWindowAnimationBehavior::None);
            o.window.orderOut(None);
            o.window.setAnimationBehavior(animation);
        }
    }
    NSAnimationContext::endGrouping();
    *child.exclusive_restore.borrow_mut() = hidden;
    let mut original_key = child.exclusive_key.borrow_mut();
    if original_key.is_none() {
        *original_key = key;
    }
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
    // Returning from Settings keeps the current cover's frame on its original
    // Main window, including a move or resize made while Settings was visible.
    if let Some(anchor) = key.as_ref().filter(|shared| !shared.closed.get())
        && let Some(original) = anchor.objects()
        && let Some(cover) = owner.objects()
    {
        original
            .window
            .setFrame_display(cover.window.frame(), false);
    }
    for shared in hidden
        .into_iter()
        .filter_map(|w| w.upgrade())
        .filter(|s| !s.closed.get())
    {
        if let Some(o) = shared.objects() {
            let animation = o.window.animationBehavior();
            o.window
                .setAnimationBehavior(NSWindowAnimationBehavior::None);
            if let Some(cover) = owner.objects().filter(|o| o.window.isVisible()) {
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
        && let Some(o) = shared.objects()
    {
        o.window.makeKeyWindow();
    }
    NSAnimationContext::endGrouping();
}
