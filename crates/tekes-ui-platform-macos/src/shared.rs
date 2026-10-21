//! 引擎侧句柄 `Window`、`WindowHandler` trait，以及事件分发核心 `Shared`。
//!
//! # 重入
//!
//! 引擎在 `handle` 里调用 `Window` 的方法（`set_title`、`close`、`set_content_size` 等），
//! AppKit 往往在这些调用里同步回调我们的视图或委托（`setFrameSize:`、`windowWillClose:`……），
//! 回调又想投递事件。如果直接再调一次 `handle`，就会对同一个 `&mut handler` 产生两个可变引用，
//! 对 `RefCell` 则是二次借用而 panic。
//!
//! 规则：
//! 1. `RefCell` 的借用永远不跨越任何外部调用（AppKit 调用、`handle`）。取出、放回都是一条语句内完成。
//! 2. 处理器用“取走再放回”的方式使用：分发期间 `handler` 槽是 `None`。
//! 3. `dispatching` 标志标记有一个分发循环正在运行。重入的 `dispatch` 只把事件放进队列就返回，
//!    由外层循环在当前 `handle` 返回后按顺序投递。事件的顺序因此保持，且任何时刻最多一个 `handle` 在运行。
//! 4. 队列里相邻重复的 `Redraw` 合并。
//!
//! `Window` 本身只是一组 `Retained` 引用和一个 `Rc<Shared>`，每次分发时临时构造，
//! 不持有任何 `RefCell` 借用，所以 `handle` 里随便调它的方法都不会二次借用。

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventType, NSWindow, NSWindowButton};
use objc2_foundation::{MainThreadMarker, NSSize, NSString};

use crate::event::{Event, Size};
use crate::menu::CommandId;
use crate::placement::{PlacementReport, ScreenInfo};
use crate::view::TekesView;

/// 引擎实现的处理器。平台层在主线程上同步调用，一次只有一个调用在运行。
pub trait WindowHandler: 'static {
    fn handle(&mut self, event: Event, window: &mut Window);
}

/// 最近一次投递给引擎的几何信息，用来去重。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GeometryState {
    pub size: Size,
    pub scale: f64,
    pub drawable: (u32, u32),
}

pub(crate) struct Objects {
    pub window: Retained<NSWindow>,
    pub view: Retained<TekesView>,
}

struct ProvidedFrame {
    layer: Retained<objc2_quartz_core::CAMetalLayer>,
    drawable: Retained<objc2::runtime::ProtocolObject<dyn objc2_quartz_core::CAMetalDrawable>>,
}

struct QueuedEvent {
    event: Event,
    drawable: Option<ProvidedFrame>,
}

pub(crate) struct Shared {
    pub(crate) wake_state: std::sync::Arc<crate::wake::WakeState>,
    pub presentation: std::sync::Arc<crate::presentation::PresentationObserver>,
    pub caret_scheduled: Cell<bool>,
    pub text_input: RefCell<Option<crate::event::TextInputState>>,
    pub accessibility_nodes: RefCell<Vec<crate::accessibility::AccessibilityNode>>,
    pub accessibility_text_layouts: RefCell<
        HashMap<
            crate::accessibility::AccessibilityKey,
            crate::accessibility::AccessibilityTextLayout,
        >,
    >,
    pub accessibility_elements: RefCell<
        HashMap<
            crate::accessibility::AccessibilityKey,
            Retained<crate::accessibility::NativeAccessibilityElement>,
        >,
    >,
    pub mtm: MainThreadMarker,
    handler: RefCell<Option<Box<dyn WindowHandler>>>,
    queue: RefCell<VecDeque<QueuedEvent>>,
    dispatching: Cell<bool>,
    // AppKit requires the original native down, even when a component first
    // offers its unhandled drag later. Never retain a RefCell borrow over AppKit.
    pub(crate) titlebar_drag_origin: RefCell<Option<Retained<NSEvent>>>,
    pub redraw_pending: Cell<bool>,
    pub last_geometry: Cell<Option<GeometryState>>,
    pub last_motion: Cell<Option<bool>>,
    pub last_appearance: Cell<Option<(bool, bool)>>,
    pub objects: OnceCell<Objects>,
    /// 窗口摆放的选择结果（autosave 恢复了位置时为 None）。
    pub placement: RefCell<Option<PlacementReport>>,
    /// `Created` 已经投递。在此之前视图收到的回调（初始 setFrameSize 等）不投递事件。
    pub ready: Cell<bool>,
    pub closed: Cell<bool>,
    /// 统计：用于示例和测试。
    pub redraw_count: Cell<u64>,
    /// 持续动画模式是否开着（`Window::set_animating`）。
    pub animating: Cell<bool>,
    /// 显示链接回调被调用的总次数（不论动画模式开没开，用来验证关掉之后真的静默）。
    pub link_callbacks: Cell<u64>,
    pub metal_reentrant_queued: Cell<u64>,
    /// 其中作为 `AnimationFrame` 投递给引擎的次数。
    pub animation_frames: Cell<u64>,
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared")
            .field("ready", &self.ready.get())
            .field("closed", &self.closed.get())
            .finish_non_exhaustive()
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        crate::wake::native::unregister(&self.wake_state);
    }
}

impl Shared {
    pub fn new(mtm: MainThreadMarker, handler: Box<dyn WindowHandler>) -> Rc<Shared> {
        Rc::new(Shared {
            wake_state: Default::default(),
            presentation: Default::default(),
            caret_scheduled: Cell::new(false),
            text_input: RefCell::new(None),
            accessibility_nodes: RefCell::new(Vec::new()),
            accessibility_text_layouts: RefCell::new(HashMap::new()),
            accessibility_elements: RefCell::new(HashMap::new()),
            mtm,
            handler: RefCell::new(Some(handler)),
            queue: RefCell::new(VecDeque::new()),
            dispatching: Cell::new(false),
            titlebar_drag_origin: RefCell::new(None),
            redraw_pending: Cell::new(false),
            last_geometry: Cell::new(None),
            last_motion: Cell::new(None),
            last_appearance: Cell::new(None),
            objects: OnceCell::new(),
            placement: RefCell::new(None),
            ready: Cell::new(false),
            closed: Cell::new(false),
            redraw_count: Cell::new(0),
            animating: Cell::new(false),
            link_callbacks: Cell::new(0),
            metal_reentrant_queued: Cell::new(0),
            animation_frames: Cell::new(0),
        })
    }

    pub(crate) fn can_dispatch_synchronously(&self) -> bool {
        !self.dispatching.get()
    }

    /// Metal updates keep their drawable across reentrant AppKit callbacks. The
    /// thread-local handoff is installed only when this queued handler runs.
    pub(crate) fn dispatch_metal_frame(
        self: &Rc<Self>,
        frame: crate::AnimationFrame,
        layer: Retained<objc2_quartz_core::CAMetalLayer>,
        drawable: Retained<objc2::runtime::ProtocolObject<dyn objc2_quartz_core::CAMetalDrawable>>,
    ) {
        if self.dispatching.get() {
            self.metal_reentrant_queued
                .set(self.metal_reentrant_queued.get() + 1);
        }
        self.enqueue(QueuedEvent {
            event: Event::AnimationFrame(frame),
            drawable: Some(ProvidedFrame { layer, drawable }),
        });
    }

    /// Release queued drawables before reconfiguring their layer. Restore the
    /// ordinary event queue before dropping Objective-C objects (may reenter).
    pub(crate) fn discard_metal_frames(&self) {
        let mut pending = std::mem::take(&mut *self.queue.borrow_mut());
        let mut discarded = Vec::new();
        while let Some(event) = pending.pop_front() {
            if event.drawable.is_some() {
                discarded.push(event);
            } else {
                self.queue.borrow_mut().push_back(event);
            }
        }
        drop(discarded);
    }

    /// 投递一个事件。重入时在当前 handler 返回后按顺序处理。
    pub fn dispatch(self: &Rc<Self>, event: Event) {
        if matches!(event, Event::Focus(false) | Event::CloseRequested) {
            self.titlebar_drag_origin.borrow_mut().take();
        }
        self.enqueue(QueuedEvent {
            event,
            drawable: None,
        });
    }

    fn enqueue(self: &Rc<Self>, queued: QueuedEvent) {
        if self.closed.get() && queued.event != Event::CloseRequested {
            return;
        }
        {
            let mut queue = self.queue.borrow_mut();
            if queued.event == Event::Redraw
                && queue.back().is_some_and(|q| q.event == Event::Redraw)
            {
                return;
            }
            queue.push_back(queued);
        }
        if self.dispatching.replace(true) {
            return;
        }
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(QueuedEvent {
                mut event,
                drawable,
            }) = next
            else {
                break;
            };
            if drawable.is_some() && (self.closed.get() || !self.animating.get()) {
                continue;
            }
            if let Event::AnimationFrame(frame) = &mut event {
                frame.dispatch_time = objc2_quartz_core::CACurrentMediaTime();
            }
            let taken = self.handler.borrow_mut().take();
            let Some(mut handler) = taken else { break };
            if event == Event::Redraw {
                self.redraw_count.set(self.redraw_count.get() + 1);
            }
            let mut window = Window::new(self.clone());
            if let Some(frame) = drawable {
                crate::presentation::with_provided_drawable(&frame.layer, frame.drawable, || {
                    handler.handle(event, &mut window)
                });
            } else {
                handler.handle(event, &mut window);
            }
            *self.handler.borrow_mut() = Some(handler);
        }
        self.dispatching.set(false);
    }

    /// 重新读取系统外观，变化了（或 `force`）就投递 `SystemAppearanceChanged`。
    pub fn check_appearance(self: &Rc<Self>, force: bool) {
        if !self.ready.get() {
            return;
        }
        let Some(o) = self.objects.get() else { return };
        let reduce =
            objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        if force || self.last_motion.get() != Some(reduce) {
            self.last_motion.set(Some(reduce));
            self.dispatch(Event::SystemMotionChanged(reduce));
        }
        let now = crate::appearance::read(&o.view);
        if !force && self.last_appearance.get() == Some(now) {
            return;
        }
        self.last_appearance.set(Some(now));
        self.dispatch(Event::SystemAppearanceChanged {
            dark: now.0,
            increase_contrast: now.1,
        });
    }

    /// 应用结束时丢弃处理器，让它的 Drop 在 `run()` 返回之前执行。
    pub fn finish(&self) {
        crate::wake::native::unregister(&self.wake_state);
        self.closed.set(true);
        self.titlebar_drag_origin.borrow_mut().take();
        self.animating.set(false);
        self.discard_metal_frames();
        if let Some(o) = self.objects.get() {
            o.view.invalidate_display_link();
        }
        let handler = self.handler.borrow_mut().take();
        drop(handler);
    }

    /// 标记需要重绘（合并）。
    pub fn request_redraw(&self) {
        if self.closed.get() {
            return;
        }
        if !self.redraw_pending.replace(true)
            && let Some(o) = self.objects.get()
        {
            o.view.schedule_redraw();
        }
    }
}

/// 引擎持有的窗口句柄。只能在主线程使用，不是 `Send`。
///
/// 所有方法都可以在 `handle` 里随时调用。
#[derive(Debug)]
pub struct Window {
    shared: Rc<Shared>,
    window: Retained<NSWindow>,
    view: Retained<TekesView>,
}

impl Window {
    /// UI-thread creation of a background-safe notification proxy. Clones do
    /// not retain this window; queued work is discarded when it closes.
    pub fn wake_proxy(&self) -> crate::WindowWake {
        crate::wake::native::register(&self.shared)
    }
    /// Visibility at this instant; a fully covered/minimized window cannot be
    /// used as evidence of visible presentation cadence.
    pub fn is_occluded(&self) -> bool {
        !self
            .window
            .occlusionState()
            .contains(objc2_app_kit::NSWindowOcclusionState::Visible)
    }

    /// Publish hover shapes after layout. AppKit restores the default cursor on exit.
    pub fn set_cursor_regions(&self, regions: Vec<crate::CursorRegion>) {
        self.view.set_cursor_regions(regions);
    }

    pub fn is_key_window(&self) -> bool {
        self.window.isKeyWindow()
    }

    /// Effective keyboard scope, including application activation. A background
    /// application's retained key window alone does not establish active input.
    pub fn is_active(&self) -> bool {
        objc2_app_kit::NSApplication::sharedApplication(self.shared.mtm).isActive()
            && self.window.isKeyWindow()
    }

    /// Enable optional drawable receipts for benchmarking; disabled in normal UI.
    pub fn set_presentation_timing(&self, enabled: bool) {
        if enabled && let Ok(mut frames) = self.shared.presentation.frames.lock() {
            frames.reserve(16384);
        }
        self.shared
            .presentation
            .enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
    }
    /// Benchmark counters (provided drawable consumptions, ordinary acquisitions).
    pub fn drawable_acquisition_counts(&self) -> (u64, u64) {
        use std::sync::atomic::Ordering;
        (
            self.shared
                .presentation
                .provided_drawables
                .load(Ordering::Relaxed),
            self.shared
                .presentation
                .fallback_acquires
                .load(Ordering::Relaxed),
        )
    }

    pub fn metal_reentrant_queued(&self) -> u64 {
        self.shared.metal_reentrant_queued.get()
    }

    pub fn drawable_serial(&self) -> u64 {
        self.shared
            .presentation
            .serial
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn presented_frames(&self) -> Vec<crate::PresentedFrame> {
        self.shared
            .presentation
            .frames
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default()
    }
    /// Enable Started/Finished provenance around text-owner key-downs. Legacy
    /// callback events are still delivered exactly once. Default is disabled.
    /// Reject changes during a key span, so the FIFO markers stay balanced.
    pub fn set_text_key_events(&self, enabled: bool) -> bool {
        !self.shared.closed.get() && self.view.set_text_key_events(enabled)
    }
    pub fn set_text_input_state(&self, state: Option<crate::event::TextInputState>) {
        if self.shared.closed.get() {
            return;
        }
        let blink = state
            .as_ref()
            .is_some_and(|s| !s.read_only && s.selection.is_empty());
        let discard = crate::event::TextInputState::conversion_ended(
            self.shared.text_input.borrow().as_ref(),
            state.as_ref(),
        );
        *self.shared.text_input.borrow_mut() = state;
        if blink && !self.shared.caret_scheduled.replace(true) {
            self.view.schedule_caret_blink();
        }
        if let Some(context) = self.view.inputContext() {
            // Publish the cleared marked range before asking IMK to finish its
            // conversion. No RefCell borrow may survive a native callback.
            if discard {
                context.discardMarkedText();
            }
            context.invalidateCharacterCoordinates();
        }
    }
    /// Publish the host-owned accessibility tree. Node data stays in the host
    /// snapshot; native AX elements only expose it and route requested actions.
    pub fn set_accessibility_nodes(&self, nodes: Vec<crate::accessibility::AccessibilityNode>) {
        if self.shared.closed.get() {
            return;
        }
        let nodes = if crate::accessibility::validate_accessibility_nodes(&nodes) {
            nodes
        } else {
            Vec::new()
        };
        if *self.shared.accessibility_nodes.borrow() == nodes {
            return;
        }
        let previous = self.shared.accessibility_nodes.replace(nodes);
        self.view.accessibility_nodes_changed(&previous);
    }
    /// Replace per-owner shaped text layouts. Publish before the corresponding
    /// node snapshot so AX notifications observe matching text and geometry.
    /// This never changes keyboard focus, text selection, or IME ownership.
    /// Omitted keys are removed; invalid/duplicate keys fail closed as a batch.
    pub fn set_accessibility_text_layouts(
        &self,
        layouts: Vec<(
            crate::accessibility::AccessibilityKey,
            crate::accessibility::AccessibilityTextLayout,
        )>,
    ) {
        if self.shared.closed.get() {
            return;
        }
        let mut accepted = HashMap::new();
        for (key, layout) in layouts {
            if !layout.is_valid() || accepted.insert(key, layout).is_some() {
                accepted.clear();
                break;
            }
        }
        *self.shared.accessibility_text_layouts.borrow_mut() = accepted;
    }

    /// Owned readback of the accepted tree, after validation. Changing this
    /// copy never mutates native AX elements; publication still uses the setter.
    pub fn accessibility_nodes(&self) -> Vec<crate::accessibility::AccessibilityNode> {
        if self.shared.closed.get() {
            return Vec::new();
        }
        self.shared.accessibility_nodes.borrow().clone()
    }

    pub fn clipboard_text(&self) -> Option<String> {
        use objc2_app_kit::NSPasteboardTypeString;
        text_pasteboard()
            .stringForType(unsafe { NSPasteboardTypeString })
            .map(|s| s.to_string())
    }
    pub fn set_clipboard_text(&self, text: &str) {
        use objc2_app_kit::NSPasteboardTypeString;
        let pasteboard = text_pasteboard();
        pasteboard.clearContents();
        pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
    }

    fn new(shared: Rc<Shared>) -> Window {
        let o = shared
            .objects
            .get()
            .expect("事件在窗口创建之前被投递，这是平台层的 bug");
        Window {
            window: o.window.clone(),
            view: o.view.clone(),
            shared,
        }
    }

    /// 请求在下一个显示周期重绘。同一周期内多次调用合并为一次 `Event::Redraw`。
    /// 在处理 `Redraw` 的过程中调用，表示“下一帧还要画”。
    pub fn request_redraw(&self) {
        self.shared.request_redraw();
    }

    /// 打开或关闭持续动画模式。
    ///
    /// 开着的时候，显示链接（`CADisplayLink`，随窗口所在屏幕的刷新率，ProMotion 上最高 120Hz）
    /// 每次刷新回调一次，平台层投递一个 [`Event::AnimationFrame`]；引擎在里面推进动画并画一帧。
    /// 关掉之后链接暂停，不再有任何回调，进程回到按需状态（空闲 0 CPU）。
    /// 重复设置同一个值无效果。
    ///
    /// 返回设置之后动画模式是否真的开着：需要 macOS 14（`NSView.displayLink(target:selector:)`），
    /// 系统不支持时开不起来，返回 false。关闭永远返回 false。
    pub fn set_animating(&self, animating: bool) -> bool {
        if self.shared.closed.get() {
            return false;
        }
        if self.shared.animating.get() == animating {
            return animating;
        }
        let on = animating && self.view.set_display_link_running(true);
        if !animating {
            self.view.set_display_link_running(false);
        }
        self.shared.animating.set(on);
        on
    }

    /// Live resize needs immediate geometry redraws even when animation is on.
    pub fn is_live_resizing(&self) -> bool {
        self.view.is_live_resizing()
    }

    /// 持续动画模式现在是否开着。
    pub fn is_animating(&self) -> bool {
        self.shared.animating.get()
    }

    /// 显示链接回调被调用的总次数（含动画模式关着时的，理论上关着时不增加），诊断用。
    pub fn display_link_callbacks(&self) -> u64 {
        self.shared.link_callbacks.get()
    }

    /// 作为 `AnimationFrame` 投递给引擎的次数，诊断用。
    pub fn animation_frame_count(&self) -> u64 {
        self.shared.animation_frames.get()
    }

    pub fn set_title(&self, title: &str) {
        self.window.setTitle(&NSString::from_str(title));
    }

    /// 视图尺寸，单位 pt。
    pub fn size(&self) -> Size {
        self.view.size_points()
    }

    /// Native unified-titlebar exclusion in this view's top-left point space.
    /// Height follows AppKit layout; the leading extent follows visible traffic
    /// lights instead of hard-coded window-control coordinates.
    pub fn titlebar_safe_area(&self) -> (f64, f64) {
        let size = self.size();
        let mut leading: f64 = 0.;
        let mut height = self.titlebar_content_height();
        for kind in [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ] {
            if let Some(button) = self.window.standardWindowButton(kind) {
                if button.isHidden() {
                    continue;
                }
                let rect = self
                    .view
                    .convertRect_fromView(button.bounds(), Some(&button));
                // Ordinary content and fullscreen chrome may not overlap the
                // traffic lights at all. Only reserve their actual visible area.
                let right = rect.origin.x + rect.size.width;
                let bottom = rect.origin.y + rect.size.height;
                if right > 0.
                    && rect.origin.x < size.width
                    && bottom > 0.
                    && rect.origin.y < size.height
                {
                    leading = leading.max(right.clamp(0., size.width));
                    height = height.max(bottom.clamp(0., size.height));
                }
            }
        }
        (height, leading)
    }

    /// `backingScaleFactor`。
    pub fn scale(&self) -> f64 {
        self.view.scale_factor()
    }

    /// drawable 的像素尺寸，等于 `size() * scale()` 取整（与 `Resized` 里的值一致）。
    pub fn drawable_size(&self) -> (u32, u32) {
        self.view.drawable_size_px()
    }

    /// 指向 CAMetalLayer 的原始指针，供渲染层创建 wgpu surface。
    ///
    /// 指针在窗口存活期间有效。CAMetalLayer 由视图持有，渲染层不要 release 它。
    pub fn metal_layer(&self) -> NonNull<c_void> {
        let layer = self.view.metal_layer();
        let ptr: *const AnyObject = Retained::as_ptr(&layer).cast();
        NonNull::new(ptr.cast_mut().cast::<c_void>()).expect("Retained 不会是空指针")
    }

    /// Height of native titlebar/toolbar chrome overlapping this content view,
    /// in logical points. Ordinary non-extended content returns zero. Query the
    /// current window geometry; do not hardcode an OS-specific toolbar height.
    pub fn titlebar_content_height(&self) -> f64 {
        let layout = self
            .view
            .convertRect_fromView(self.window.contentLayoutRect(), None);
        let height = layout.origin.y;
        if height.is_finite() {
            height.clamp(0., self.size().height.max(0.))
        } else {
            0.
        }
    }

    /// Hand an eligible titlebar press to AppKit. May be called synchronously
    /// on down or after a component offers an unhandled drag. Cancel the old
    /// component interaction first: AppKit may not deliver a matching mouse-up.
    pub fn begin_titlebar_drag(&self) {
        let _ = self.try_begin_titlebar_drag();
    }

    /// Same handoff with an explicit result. Uses the retained original native
    /// down, never the dragged event, and consumes that origin at most once.
    pub fn try_begin_titlebar_drag(&self) -> bool {
        let Some(current) = self.window.currentEvent() else {
            return false;
        };
        if !self.is_active()
            || !matches!(
                current.r#type(),
                NSEventType::LeftMouseDown | NSEventType::LeftMouseDragged
            )
            || current.windowNumber() != self.window.windowNumber()
            || NSEvent::pressedMouseButtons() & 1 == 0
        {
            return false;
        }
        let Some(origin) = self.shared.titlebar_drag_origin.borrow_mut().take() else {
            return false;
        };
        if origin.r#type() != NSEventType::LeftMouseDown
            || origin.windowNumber() != self.window.windowNumber()
        {
            return false;
        }
        self.window.performWindowDragWithEvent(&origin);
        true
    }

    /// 关闭窗口。不经过 `CloseRequested`，直接关。
    pub fn close(&self) {
        self.window.close();
    }

    /// 改变内容区尺寸，单位 pt。会同步引发 `Resized` 与 `Redraw`（排队，在当前 `handle` 返回后投递）。
    pub fn set_content_size(&self, size: Size) {
        self.window
            .setContentSize(NSSize::new(size.width, size.height));
    }

    pub fn update_menu_titles(&self, bar: &crate::menu::MenuBar) {
        crate::menu_build::update_titles(self.shared.mtm, bar);
    }

    /// 修改菜单项的勾选状态。命令不存在时返回 false。
    pub fn set_command_checked(&self, command: CommandId, checked: bool) -> bool {
        crate::menu_build::set_command_checked(self.shared.mtm, command, checked)
    }

    /// 读取菜单项的勾选状态，命令不存在时返回 `None`。
    pub fn is_command_checked(&self, command: CommandId) -> Option<bool> {
        crate::menu_build::is_command_checked(self.shared.mtm, command)
    }

    /// 启动时窗口摆放的选择结果：选了哪块屏幕、为什么，以及当时看到的全部屏幕。
    /// 位置由 autosave 恢复时为 `None`。
    pub fn placement_report(&self) -> Option<PlacementReport> {
        self.shared.placement.borrow().clone()
    }

    /// 窗口当前所在屏幕的信息。
    pub fn screen_info(&self) -> Option<ScreenInfo> {
        crate::screens::current(&self.window)
    }

    /// `NSWindow.windowNumber`，仅用于诊断（例如 `screencapture -l`）。
    pub fn native_window_number(&self) -> isize {
        self.window.windowNumber()
    }

    /// 已经投递的 `Redraw` 数量，仅用于诊断。
    pub fn redraw_count(&self) -> u64 {
        self.shared.redraw_count.get()
    }
}

fn text_pasteboard() -> Retained<objc2_app_kit::NSPasteboard> {
    if std::env::var("TEKES_SELFTEST").as_deref() == Ok("text") {
        // Native text smoke tests must not replace the user's general clipboard.
        objc2_app_kit::NSPasteboard::pasteboardWithName(&NSString::from_str(&format!(
            "TekesUI.TextSelftest.{}",
            std::process::id()
        )))
    } else {
        objc2_app_kit::NSPasteboard::generalPasteboard()
    }
}
