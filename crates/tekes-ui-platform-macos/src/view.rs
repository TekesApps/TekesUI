//! 自定义 NSView：背衬层是 CAMetalLayer，负责输入事件的规整、尺寸与缩放同步、按需重绘。
//!
//! 设计见 docs/platform-boundary.md 的“帧节奏”与“实时缩放”两节。要点：
//! - 空闲时没有任何定时器或 display link；只有 AppKit 因 `setNeedsDisplay` 调 `updateLayer`
//!   或因尺寸变化调 `setFrameSize:` 时才投递事件。
//! - 尺寸变化在 `setFrameSize:` 里同步投递 `Resized` 和 `Redraw`，不等下一个显示周期。

use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::{AllocAnyThread, DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSEvent, NSTrackingArea, NSTrackingAreaOptions, NSView, NSViewLayerContentsRedrawPolicy,
};
use objc2_core_foundation::CGSize;
use objc2_core_graphics::{CGColorSpace, kCGColorSpaceSRGB};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CAMetalLayer, kCAGravityTopLeft};

use crate::event::{
    Event, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind, Point,
    ScrollEvent, ScrollPhase, Size, Vec2,
};
use crate::shared::{GeometryState, Shared};

#[derive(Debug)]
pub(crate) struct ViewIvars {
    shared: Rc<Shared>,
    layer: RefCell<Option<Retained<CAMetalLayer>>>,
    tracking_area: RefCell<Option<Retained<NSTrackingArea>>>,
}

define_class!(
    // SAFETY: 父类 NSView 没有额外的子类化要求，TekesView 不实现 Drop。
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ViewIvars]
    #[derive(Debug)]
    pub(crate) struct TekesView;

    // SAFETY: NSObjectProtocol 没有安全要求。
    unsafe impl NSObjectProtocol for TekesView {}

    impl TekesView {
        // ---- 视图属性 ----

        /// 左上角为原点，与引擎一致。AppKit 会据此把 convertPoint 的结果翻转好，
        /// 鼠标位置不需要手动 `height - y`。
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// 整个视图不透明，让窗口服务器不必混合下层。
        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            true
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        /// 窗口未激活时的第一次点击也送达视图，而不只是激活窗口。
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        // ---- 层 ----

        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            true
        }

        #[unsafe(method_id(makeBackingLayer))]
        fn make_backing_layer(&self) -> Retained<CALayer> {
            let layer = CAMetalLayer::new();
            // 渲染层按 gamma 空间混合，与 AppKit 一致，所以层的色彩空间标成 sRGB，
            // 不让系统把显示器的色域再转换一次。
            let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
            layer.setColorspace(srgb.as_deref());
            layer.setOpaque(true);
            // 新 drawable 还没来时，旧内容按原尺寸钉在左上角，不拉伸。
            layer.setContentsGravity(unsafe { kCAGravityTopLeft });
            let scale = self.scale_factor();
            layer.setContentsScale(scale);
            let b = self.bounds().size;
            let (w, h) = Size {
                width: b.width,
                height: b.height,
            }
            .to_drawable(scale);
            layer.setDrawableSize(CGSize::new(w as f64, h as f64));
            *self.ivars().layer.borrow_mut() = Some(layer.clone());
            layer.into_super()
        }

        /// 只有 `request_redraw` 引起的 `setNeedsDisplay`，或 AppKit 自己要求（首次显示、
        /// 窗口从遮挡恢复）才会走到这里。
        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            let shared = &self.ivars().shared;
            if !shared.ready.get() {
                return;
            }
            shared.redraw_pending.set(false);
            shared.dispatch(Event::Redraw);
        }

        // ---- 尺寸与缩放 ----

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), setFrameSize: size] };
            self.sync_geometry(false);
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), viewDidChangeBackingProperties] };
            self.sync_geometry(false);
        }

        #[unsafe(method(viewDidMoveToWindow))]
        fn view_did_move_to_window(&self) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), viewDidMoveToWindow] };
            self.sync_geometry(false);
        }

        // ---- 外观 ----

        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn view_did_change_effective_appearance(&self) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] };
            self.ivars().shared.check_appearance(false);
        }

        // ---- 跟踪区域：mouseMoved / mouseEntered / mouseExited ----

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), updateTrackingAreas] };
            let ivars = self.ivars();
            if let Some(old) = ivars.tracking_area.borrow_mut().take() {
                self.removeTrackingArea(&old);
            }
            // InVisibleRect：区域自动跟随可见矩形，尺寸变化时无需重建。
            // ActiveInActiveApp：应用处于前台时就有 hover，不要求窗口是 key window。
            let options = NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveInActiveApp
                | NSTrackingAreaOptions::InVisibleRect;
            // SAFETY: owner 是 self，options 合法，userInfo 为 None。
            let area = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    self.bounds(),
                    options,
                    Some(self),
                    None,
                )
            };
            self.addTrackingArea(&area);
            *ivars.tracking_area.borrow_mut() = Some(area);
        }

        // ---- 鼠标 ----

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, e: &NSEvent) {
            self.button_event(e, true);
        }
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, e: &NSEvent) {
            self.button_event(e, false);
        }
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, e: &NSEvent) {
            self.button_event(e, true);
        }
        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, e: &NSEvent) {
            self.button_event(e, false);
        }
        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, e: &NSEvent) {
            self.button_event(e, true);
        }
        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, e: &NSEvent) {
            self.button_event(e, false);
        }
        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, e: &NSEvent) {
            self.drag_event(e);
        }
        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, e: &NSEvent) {
            self.drag_event(e);
        }
        #[unsafe(method(otherMouseDragged:))]
        fn other_mouse_dragged(&self, e: &NSEvent) {
            self.drag_event(e);
        }
        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, e: &NSEvent) {
            self.mouse(e, MouseEventKind::Moved);
        }
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, e: &NSEvent) {
            self.mouse(e, MouseEventKind::Entered);
        }
        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, e: &NSEvent) {
            self.mouse(e, MouseEventKind::Exited);
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, e: &NSEvent) {
            let ev = ScrollEvent {
                delta: Vec2 {
                    x: e.scrollingDeltaX(),
                    y: e.scrollingDeltaY(),
                },
                precise: e.hasPreciseScrollingDeltas(),
                phase: ScrollPhase::from_ns_phase(e.phase().bits()),
                momentum_phase: ScrollPhase::from_ns_phase(e.momentumPhase().bits()),
                inverted: e.isDirectionInvertedFromDevice(),
                position: self.view_point(e),
                modifiers: modifiers_of(e),
            };
            self.ivars().shared.dispatch(Event::Scroll(ev));
        }

        // ---- 键盘 ----
        //
        // 暂不实现 NSTextInputClient（F5）。这里也不调用 interpretKeyEvents:，
        // 所以输入法和死键不会工作；也不调用 super，避免未处理的按键触发系统提示音。

        #[unsafe(method(keyDown:))]
        fn key_down(&self, e: &NSEvent) {
            self.key_event(e, KeyEventKind::Down);
        }
        #[unsafe(method(keyUp:))]
        fn key_up(&self, e: &NSEvent) {
            self.key_event(e, KeyEventKind::Up);
        }
        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, e: &NSEvent) {
            self.ivars()
                .shared
                .dispatch(Event::ModifiersChanged(modifiers_of(e)));
        }
    }
);

fn modifiers_of(e: &NSEvent) -> Modifiers {
    Modifiers::from_ns_flags(e.modifierFlags().bits())
}

impl TekesView {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        shared: Rc<Shared>,
        frame: NSRect,
    ) -> Retained<TekesView> {
        let this = Self::alloc(mtm).set_ivars(ViewIvars {
            shared,
            layer: RefCell::new(None),
            tracking_area: RefCell::new(None),
        });
        // SAFETY: NSView 的指定初始化方法 initWithFrame: 签名正确。
        let view: Retained<TekesView> = unsafe { msg_send![super(this), initWithFrame: frame] };
        // 必须在 wantsLayer 之前设置：我们自己决定重绘时机。
        // OnSetNeedsDisplay：只有 setNeedsDisplay 才触发 updateLayer。缩放时的重绘由
        // setFrameSize: 同步投递，如果用默认的 DuringViewResize，AppKit 会在同步重绘之后
        // 再触发一次 updateLayer，同一次缩放画两帧。
        view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::OnSetNeedsDisplay);
        view.setWantsLayer(true);
        view
    }

    /// 确保背衬层已经创建并返回它。
    pub(crate) fn metal_layer(&self) -> Retained<CAMetalLayer> {
        if self.ivars().layer.borrow().is_none() {
            // 访问 layer 会让 AppKit 调 makeBackingLayer。
            let _ = self.layer();
        }
        self.ivars()
            .layer
            .borrow()
            .clone()
            .expect("wantsLayer 之后 makeBackingLayer 一定被调用过")
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        match self.window() {
            Some(w) => w.backingScaleFactor(),
            // 还没挂到窗口时，用主屏幕的缩放因子做初值，挂上去后会再同步。
            None => objc2_app_kit::NSScreen::mainScreen(self.mtm())
                .map(|s| s.backingScaleFactor())
                .unwrap_or(1.0),
        }
    }

    pub(crate) fn size_points(&self) -> Size {
        let s = self.bounds().size;
        Size {
            width: s.width,
            height: s.height,
        }
    }

    pub(crate) fn drawable_size_px(&self) -> (u32, u32) {
        let d = self.metal_layer().drawableSize();
        (d.width as u32, d.height as u32)
    }

    /// NSEvent 的窗口坐标 → 视图坐标（左上角为原点，因为 isFlipped）。
    fn view_point(&self, e: &NSEvent) -> Point {
        let p = self.convertPoint_fromView(e.locationInWindow(), None);
        Point { x: p.x, y: p.y }
    }

    fn mouse(&self, e: &NSEvent, kind: MouseEventKind) {
        let ev = MouseEvent {
            kind,
            position: self.view_point(e),
            modifiers: modifiers_of(e),
        };
        self.ivars().shared.dispatch(Event::Mouse(ev));
    }

    fn button_event(&self, e: &NSEvent, down: bool) {
        let button = MouseButton::from_button_number(e.buttonNumber());
        let click_count = e.clickCount().max(0) as u32;
        let kind = if down {
            MouseEventKind::Down {
                button,
                click_count,
            }
        } else {
            MouseEventKind::Up {
                button,
                click_count,
            }
        };
        self.mouse(e, kind);
    }

    fn drag_event(&self, e: &NSEvent) {
        let button = MouseButton::from_button_number(e.buttonNumber());
        self.mouse(e, MouseEventKind::Dragged { button });
    }

    fn key_event(&self, e: &NSEvent, kind: KeyEventKind) {
        let ev = KeyEvent {
            kind,
            key_code: e.keyCode(),
            characters: e.characters().map(|s| s.to_string()).unwrap_or_default(),
            characters_ignoring_modifiers: e
                .charactersIgnoringModifiers()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            modifiers: modifiers_of(e),
            is_repeat: kind == KeyEventKind::Down && e.isARepeat(),
        };
        self.ivars().shared.dispatch(Event::Key(ev));
    }

    /// 把层的 contentsScale 与 drawableSize 对齐到视图当前的 bounds 和窗口缩放因子。
    /// 几何信息相对上一次投递有变化（或 `force`）时，同步投递 `Resized`，紧接着同步投递 `Redraw`。
    ///
    /// 层的更新总是做，投递才需要 `ready`：`Created` 之前 AppKit 的初始回调只更新层。
    pub(crate) fn sync_geometry(&self, force: bool) {
        let shared = &self.ivars().shared;
        let scale = self.scale_factor();
        let size = self.size_points();
        let drawable = size.to_drawable(scale);
        let layer = self.metal_layer();
        if layer.contentsScale() != scale {
            layer.setContentsScale(scale);
        }
        let current = layer.drawableSize();
        if current.width as u32 != drawable.0 || current.height as u32 != drawable.1 {
            layer.setDrawableSize(CGSize::new(drawable.0 as f64, drawable.1 as f64));
        }
        if !shared.ready.get() {
            return;
        }
        let state = GeometryState {
            size,
            scale,
            drawable,
        };
        if !force && shared.last_geometry.get() == Some(state) {
            return;
        }
        shared.last_geometry.set(Some(state));
        shared.dispatch(Event::Resized {
            size,
            scale,
            drawable_size: drawable,
        });
        // 同步重绘：不等下一个显示周期，否则实时缩放时窗口已经变了而内容还是旧尺寸。
        // 这一帧已经满足了此前挂起的重绘请求，取消 AppKit 那边的 needsDisplay，避免再来一帧。
        if shared.redraw_pending.replace(false) {
            self.setNeedsDisplay(false);
        }
        shared.dispatch(Event::Redraw);
    }
}
