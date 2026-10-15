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
use std::collections::VecDeque;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSWindow;
use objc2_foundation::{MainThreadMarker, NSSize, NSString};

use crate::event::{Event, Size};
use crate::menu::CommandId;
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

pub(crate) struct Shared {
    pub mtm: MainThreadMarker,
    handler: RefCell<Option<Box<dyn WindowHandler>>>,
    queue: RefCell<VecDeque<Event>>,
    dispatching: Cell<bool>,
    pub redraw_pending: Cell<bool>,
    pub last_geometry: Cell<Option<GeometryState>>,
    pub last_appearance: Cell<Option<(bool, bool)>>,
    pub objects: OnceCell<Objects>,
    /// `Created` 已经投递。在此之前视图收到的回调（初始 setFrameSize 等）不投递事件。
    pub ready: Cell<bool>,
    pub closed: Cell<bool>,
    /// 统计：用于示例和测试。
    pub redraw_count: Cell<u64>,
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared")
            .field("ready", &self.ready.get())
            .field("closed", &self.closed.get())
            .finish_non_exhaustive()
    }
}

impl Shared {
    pub fn new(mtm: MainThreadMarker, handler: Box<dyn WindowHandler>) -> Rc<Shared> {
        Rc::new(Shared {
            mtm,
            handler: RefCell::new(Some(handler)),
            queue: RefCell::new(VecDeque::new()),
            dispatching: Cell::new(false),
            redraw_pending: Cell::new(false),
            last_geometry: Cell::new(None),
            last_appearance: Cell::new(None),
            objects: OnceCell::new(),
            ready: Cell::new(false),
            closed: Cell::new(false),
            redraw_count: Cell::new(0),
        })
    }

    /// 投递一个事件。可以在任何时刻、包括 `handle` 运行期间调用。
    pub fn dispatch(self: &Rc<Self>, event: Event) {
        if self.closed.get() && !matches!(event, Event::CloseRequested) {
            return;
        }
        {
            let mut q = self.queue.borrow_mut();
            if event == Event::Redraw && q.back() == Some(&Event::Redraw) {
                return;
            }
            q.push_back(event);
        }
        if self.dispatching.replace(true) {
            // 外层已经有分发循环在跑，它会在当前 handle 返回后取到这个事件。
            return;
        }
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(ev) = next else { break };
            let taken = self.handler.borrow_mut().take();
            let Some(mut handler) = taken else { break };
            if ev == Event::Redraw {
                self.redraw_count.set(self.redraw_count.get() + 1);
            }
            let mut window = Window::new(self.clone());
            handler.handle(ev, &mut window);
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
        self.closed.set(true);
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
            o.view.setNeedsDisplay(true);
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

    pub fn set_title(&self, title: &str) {
        self.window.setTitle(&NSString::from_str(title));
    }

    /// 视图尺寸，单位 pt。
    pub fn size(&self) -> Size {
        self.view.size_points()
    }

    /// `backingScaleFactor`。
    pub fn scale(&self) -> f64 {
        self.view.scale_factor()
    }

    /// 当前 CAMetalLayer 的 drawableSize，单位像素。
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

    /// 关闭窗口。不经过 `CloseRequested`，直接关。
    pub fn close(&self) {
        self.window.close();
    }

    /// 改变内容区尺寸，单位 pt。会同步引发 `Resized` 与 `Redraw`（排队，在当前 `handle` 返回后投递）。
    pub fn set_content_size(&self, size: Size) {
        self.window
            .setContentSize(NSSize::new(size.width, size.height));
    }

    /// 修改菜单项的勾选状态。命令不存在时返回 false。
    pub fn set_command_checked(&self, command: CommandId, checked: bool) -> bool {
        crate::menu_build::set_command_checked(self.shared.mtm, command, checked)
    }

    /// 读取菜单项的勾选状态，命令不存在时返回 `None`。
    pub fn is_command_checked(&self, command: CommandId) -> Option<bool> {
        crate::menu_build::is_command_checked(self.shared.mtm, command)
    }

    /// 设置 CAMetalLayer 的 `presentsWithTransaction`。默认关闭。
    ///
    /// 打开后，实时缩放时窗口边框与内容在同一个 Core Animation 事务里更新，边缘不会抖动，
    /// 代价是呈现路径必须在主线程上 `waitUntilScheduled` 再 `present`，
    /// 否则会卡顿。是否启用留给渲染层集成时按实测决定。
    pub fn set_presents_with_transaction(&self, on: bool) {
        self.view.metal_layer().setPresentsWithTransaction(on);
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
