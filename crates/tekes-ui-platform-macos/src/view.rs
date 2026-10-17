//! 自定义 NSView：背衬层是 CAMetalLayer，负责输入事件的规整、尺寸与缩放同步、按需重绘。
//!
//! 设计见 docs/platform-boundary.md 的“帧节奏”与“实时缩放”两节。要点：
//! - 空闲时没有任何定时器或 display link；只有 AppKit 因 `setNeedsDisplay` 调 `updateLayer`
//!   或因尺寸变化调 `setFrameSize:` 时才投递事件。
//! - 尺寸变化在 `setFrameSize:` 里同步投递 `Resized` 和 `Redraw`，不等下一个显示周期。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AllocAnyThread, DefinedClass, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSEvent, NSTextInputClient, NSTrackingArea, NSTrackingAreaOptions, NSView,
    NSViewLayerContentsPlacement, NSViewLayerContentsRedrawPolicy,
};
use objc2_core_graphics::{CGColorSpace, kCGColorSpaceSRGB};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSObjectNSDelayedPerforming, NSObjectNSThreadPerformAdditions,
    NSObjectProtocol, NSRect, NSRunLoop, NSRunLoopCommonModes, NSSize,
};
use objc2_quartz_core::{
    CACurrentMediaTime, CADisplayLink, CAFrameRateRange, CALayer, CAMetalDisplayLink,
    CAMetalDisplayLinkDelegate, CAMetalDisplayLinkUpdate, CAMetalLayer,
};

use crate::event::{
    AnimationFrame, Event, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent,
    MouseEventKind, Point, ScrollEvent, ScrollPhase, Size, Vec2,
};
use crate::shared::{GeometryState, Shared};

#[derive(Debug)]
pub(crate) struct ViewIvars {
    shared: Rc<Shared>,
    layer: RefCell<Option<Retained<CAMetalLayer>>>,
    tracking_area: RefCell<Option<Retained<NSTrackingArea>>>,
    /// 按需创建的显示链接。创建后一直存在，不跑的时候是暂停状态（`isPaused`），
    /// 窗口关闭或应用结束时 invalidate（它强引用 target，也就是这个视图）。
    link: RefCell<Option<Retained<CADisplayLink>>>,
    metal_link: RefCell<Option<Retained<CAMetalDisplayLink>>>,
    metal_start_pending: Cell<bool>,
    live_resizing: Cell<bool>,
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

    // SAFETY: required NSTextInputClient methods use AppKit's exact ABI. Rust owns
    // document state; native callbacks only dispatch edits or query a published copy.
    #[allow(non_snake_case)]
    unsafe impl NSTextInputClient for TekesView {
        #[unsafe(method(insertText:replacementRange:))]
        unsafe fn insertText_replacementRange(&self,string:&objc2::runtime::AnyObject,replacement:objc2_foundation::NSRange) {
            self.ivars().shared.dispatch(Event::TextCommit{text:input_string(string),replacement:input_range(replacement)});
        }
        #[unsafe(method(doCommandBySelector:))]
        unsafe fn doCommandBySelector(&self,selector:objc2::runtime::Sel) {
            self.ivars().shared.dispatch(Event::TextCommand(selector.name().to_string_lossy().into_owned()));
        }
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        unsafe fn setMarkedText_selectedRange_replacementRange(&self,string:&objc2::runtime::AnyObject,selection:objc2_foundation::NSRange,replacement:objc2_foundation::NSRange) {
            self.ivars().shared.dispatch(Event::TextPreedit{text:input_string(string),clauses:input_clauses(string),selection:selection.location..selection.location.saturating_add(selection.length),replacement:input_range(replacement)});
        }
        #[unsafe(method(unmarkText))]
        fn unmarkText(&self) {self.ivars().shared.dispatch(Event::TextUnmark);}
        #[unsafe(method(selectedRange))]
        fn selectedRange(&self)->objc2_foundation::NSRange {self.ivars().shared.text_input.borrow().as_ref().map(|s|native_range(s.selection.clone())).unwrap_or(not_found())}
        #[unsafe(method(markedRange))]
        fn markedRange(&self)->objc2_foundation::NSRange {self.ivars().shared.text_input.borrow().as_ref().and_then(|s|s.marked.clone()).map(native_range).unwrap_or(not_found())}
        #[unsafe(method(hasMarkedText))]
        fn hasMarkedText(&self)->bool {self.ivars().shared.text_input.borrow().as_ref().is_some_and(|s|s.marked.is_some())}
        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        unsafe fn attributedSubstringForProposedRange_actualRange(&self,range:objc2_foundation::NSRange,actual:*mut objc2_foundation::NSRange)->Option<Retained<objc2_foundation::NSAttributedString>> {
            (|| {
            let state=self.ivars().shared.text_input.borrow();let state=state.as_ref()?;
            if range.location==isize::MAX as usize {return None;}
            let units:Vec<u16>=state.text.encode_utf16().collect();let mut start=range.location.min(units.len());let mut end=range.location.saturating_add(range.length).min(units.len());
            let low=|u:u16|(0xDC00..=0xDFFF).contains(&u);
            if start<units.len()&&start>0&&low(units[start]){start-=1;}
            if end<units.len()&&end>0&&low(units[end]){end+=1;}
            if start>end {return None;}
            if !actual.is_null(){unsafe {*actual=objc2_foundation::NSRange::new(start,end-start);}}
            Some(objc2_foundation::NSAttributedString::initWithString(objc2_foundation::NSAttributedString::alloc(),&objc2_foundation::NSString::from_str(&String::from_utf16_lossy(&units[start..end]))))
            })()
        }
        #[unsafe(method_id(validAttributesForMarkedText))]
        fn validAttributesForMarkedText(&self)->Retained<NSArray<objc2_foundation::NSAttributedStringKey>> {unsafe {NSArray::from_slice(&[objc2_app_kit::NSMarkedClauseSegmentAttributeName,objc2_app_kit::NSUnderlineStyleAttributeName])}}
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        unsafe fn firstRectForCharacterRange_actualRange(&self,_range:objc2_foundation::NSRange,actual:*mut objc2_foundation::NSRange)->NSRect {
            let state=self.ivars().shared.text_input.borrow();let Some(s)=state.as_ref() else {return NSRect::ZERO;};
            let (index,origin,size)=s.carets.iter().find(|v|v.0==_range.location).copied().unwrap_or((s.caret_index,s.caret_origin,s.caret_size));
            if !actual.is_null(){unsafe {*actual=objc2_foundation::NSRange::new(index,0);}}
            let rect=NSRect::new(objc2_foundation::NSPoint::new(origin.x,origin.y),NSSize::new(size.width,size.height));
            let rect=self.convertRect_toView(rect,None);self.window().map_or(rect,|w|w.convertRectToScreen(rect))
        }
        #[unsafe(method(characterIndexForPoint:))]
        fn characterIndexForPoint(&self,point:objc2_foundation::NSPoint)->usize {
            let p=self.window().map_or(point,|w|w.convertPointFromScreen(point));let p=self.convertPoint_fromView(p,None);
            self.ivars().shared.text_input.borrow().as_ref().map_or(0,|s|s.carets.iter().min_by(|a,b|{
                let distance=|v:&(usize,Point,Size)| ((p.y-(v.1.y+v.2.height*0.5)).abs()*1000.0)+(p.x-v.1.x).abs();
                distance(a).total_cmp(&distance(b))
            }).map_or(s.caret_index,|v|v.0))
        }
    }

    // SAFETY: QuartzCore's exact public delegate ABI. The display link is added
    // only to the main run loop; this MainThreadOnly view owns its weak delegate.
    #[allow(non_snake_case)]
    unsafe impl CAMetalDisplayLinkDelegate for TekesView {
        #[unsafe(method(metalDisplayLink:needsUpdate:))]
        fn metalDisplayLink_needsUpdate(&self, _link: &CAMetalDisplayLink, update: &CAMetalDisplayLinkUpdate) {
            let callback_time = CACurrentMediaTime();
            // A handler may stop/invalidate the link during this callback.
            let _lifetime_guard = _link.retain();
            let shared = &self.ivars().shared;
            shared.link_callbacks.set(shared.link_callbacks.get() + 1);
            let current = self.ivars().metal_link.borrow().clone();
            if !current.as_ref().is_some_and(|link| std::ptr::eq(&**link, _link)) { return; }
            if !shared.ready.get() || !shared.animating.get() || self.ivars().live_resizing.get() { return; }
            shared.animation_frames.set(shared.animation_frames.get() + 1);
            // Keep a missed callback from enlarging the nominal refresh budget.
            let hz = self.window().and_then(|w| w.screen()).map_or(60, |s| s.maximumFramesPerSecond()).clamp(1, 120);
            let interval = 1.0 / hz as f64;
            let target = update.targetPresentationTimestamp();
            let frame = AnimationFrame::new(target - interval, target, callback_time, interval)
                .with_submission_deadline(update.targetTimestamp());
            shared.dispatch_metal_frame(frame, self.metal_layer(), update.drawable());
        }
    }

    impl TekesView {
        #[unsafe(method(tekesStartMetalLink))]
        fn start_metal_link_deferred(&self) {
            self.ivars().metal_start_pending.set(false);
            let shared = &self.ivars().shared;
            if !shared.closed.get() && shared.animating.get() && !self.ivars().live_resizing.get() {
                if shared.can_dispatch_synchronously() {
                    self.start_metal_link_now();
                } else {
                    self.schedule_metal_start();
                }
            }
        }

        #[unsafe(method(tekesCaretBlink))]
        fn caret_blink(&self) {
            let active=self.ivars().shared.text_input.borrow().as_ref().is_some_and(|s|!s.read_only&&s.selection.is_empty());
            if active && !self.ivars().shared.closed.get() {
                self.ivars().shared.dispatch(Event::CaretBlink);
                self.schedule_caret_blink();
            } else {self.ivars().shared.caret_scheduled.set(false);}
        }

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
            let layer = crate::presentation::ObservedMetalLayer::new(self.ivars().shared.presentation.clone()).into_super();
            // 渲染层按 gamma 空间混合，与 AppKit 一致，所以层的色彩空间标成 sRGB，
            // 不让系统把显示器的色域再转换一次。
            let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
            layer.setColorspace(srgb.as_deref());
            layer.setOpaque(true);
            // drawableSize 归渲染器管（wgpu 配置 surface 时设置），平台层不碰；
            // 在渲染器接管之前，CAMetalLayer 自己按 bounds * contentsScale 算。
            layer.setContentsScale(self.scale_factor());
            *self.ivars().layer.borrow_mut() = Some(layer.clone());
            layer.into_super()
        }

        /// 实测（macOS 26）：背衬层是 CAMetalLayer 时，`setNeedsDisplay:` 会被转成层的 needsDisplay，
        /// 但 CAMetalLayer 从不执行 display，AppKit 也就不会调用 `updateLayer`，连同步的
        /// `display` 也不会。所以按需重绘不走这条路，而是 `request_redraw` 自己在主 runloop 上排一次
        /// 性的 `tekesPerformRedraw`（见下）。这里保留实现：万一 AppKit 因别的原因调用，
        /// 照样投递 `Redraw`。
        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            let shared = &self.ivars().shared;
            if !shared.ready.get() {
                return;
            }
            shared.redraw_pending.set(false);
            shared.dispatch(Event::Redraw);
        }

        /// `request_redraw` 排进主 runloop（common modes）的一次性回调，合并后只投递一次 `Redraw`。
        /// 没有定时器，也没有 display link：runloop 在没有排队回调时停在 `mach_msg`。
        #[unsafe(method(tekesPerformRedraw))]
        fn perform_redraw(&self) {
            let shared = &self.ivars().shared;
            // 已经被同步重绘（缩放路径）满足的请求，这里不再重复。
            if shared.ready.get() && shared.redraw_pending.replace(false) {
                shared.dispatch(Event::Redraw);
            }
        }

        /// 显示链接的回调。第一件事是读时钟：`callback_time` 要尽量接近回调真正发生的时刻。
        /// 动画模式关着时（暂停之前已经排队的最后一次回调）只计数，不投递。
        #[unsafe(method(tekesDisplayLinkFired:))]
        fn display_link_fired(&self, link: &CADisplayLink) {
            let callback_time = CACurrentMediaTime();
            let shared = &self.ivars().shared;
            shared.link_callbacks.set(shared.link_callbacks.get() + 1);
            // Pausing CAD may leave one callback queued. Outside live resize
            // only the Metal delegate may drive this opt-in renderer.
            if Self::uses_metal_display_link() && !self.ivars().live_resizing.get() { return; }
            if !shared.ready.get() || !shared.animating.get() {
                return;
            }
            shared
                .animation_frames
                .set(shared.animation_frames.get() + 1);
            let frame = AnimationFrame::new(
                link.timestamp(),
                link.targetTimestamp(),
                callback_time,
                link.duration(),
            );
            shared.dispatch(Event::AnimationFrame(frame));
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

        // ---- 实时缩放 ----
        //
        // 实时缩放期间打开 presentsWithTransaction：渲染层在主线程同步呈现
        // （commit + waitUntilScheduled + present），内容与窗口边框在同一个 Core Animation
        // 事务里更新，边缘不会错位。结束后关掉，回到普通的异步呈现。
        // wgpu 在每次取 drawable 时读取这个属性，所以渲染层不需要额外配合。

        #[unsafe(method(viewWillStartLiveResize))]
        fn view_will_start_live_resize(&self) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), viewWillStartLiveResize] };
            self.ivars().live_resizing.set(true);
            if Self::uses_metal_display_link() {
                self.stop_metal_link();
                // CADisplayLink supplies timing without reserving drawables;
                // keep animation moving while AppKit resizes/reconfigures.
                if self.ivars().shared.animating.get() { self.set_display_link_running(true); }
            }
            self.metal_layer().setPresentsWithTransaction(true);
        }

        #[unsafe(method(viewDidEndLiveResize))]
        fn view_did_end_live_resize(&self) {
            // SAFETY: 调用父类实现，签名一致。
            let () = unsafe { msg_send![super(self), viewDidEndLiveResize] };
            self.metal_layer().setPresentsWithTransaction(false);
            self.ivars().live_resizing.set(false);
            if Self::uses_metal_display_link() && self.ivars().shared.animating.get() {
                self.set_metal_display_link_running(true);
            }
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
                timestamp: e.timestamp(),
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
            let editing=self.ivars().shared.text_input.borrow().is_some();
            if editing && !modifiers_of(e).command {
                self.interpretKeyEvents(&NSArray::from_slice(&[e]));
            } else {self.key_event(e, KeyEventKind::Down);}
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

fn not_found() -> objc2_foundation::NSRange {
    objc2_foundation::NSRange::new(isize::MAX as usize, 0)
}
fn input_range(r: objc2_foundation::NSRange) -> Option<std::ops::Range<usize>> {
    (r.location != isize::MAX as usize).then(|| r.location..r.location.saturating_add(r.length))
}
fn native_range(r: std::ops::Range<usize>) -> objc2_foundation::NSRange {
    objc2_foundation::NSRange::new(r.start, r.end.saturating_sub(r.start))
}
fn input_clauses(value: &objc2::runtime::AnyObject) -> Vec<std::ops::Range<usize>> {
    let Some(text) = value.downcast_ref::<objc2_foundation::NSAttributedString>() else {
        return Vec::new();
    };
    let mut index = 0;
    let mut clauses = Vec::new();
    while index < text.length() {
        let mut range = objc2_foundation::NSRange::new(index, 1);
        // SAFETY: index is inside the attributed string; out-range pointer is valid.
        let attribute = unsafe {
            text.attribute_atIndex_effectiveRange(
                objc2_app_kit::NSMarkedClauseSegmentAttributeName,
                index,
                &mut range,
            )
        };
        let end = range
            .location
            .saturating_add(range.length)
            .min(text.length());
        if attribute.is_some() && end > range.location {
            clauses.push(range.location..end);
        }
        index = end.max(index + 1);
    }
    clauses
}

fn input_string(value: &objc2::runtime::AnyObject) -> String {
    if let Some(s) = value.downcast_ref::<objc2_foundation::NSString>() {
        s.to_string()
    } else if let Some(s) = value.downcast_ref::<objc2_foundation::NSAttributedString>() {
        s.string().to_string()
    } else {
        String::new()
    }
}

fn modifiers_of(e: &NSEvent) -> Modifiers {
    Modifiers::from_ns_flags(e.modifierFlags().bits())
}

impl TekesView {
    pub(crate) fn schedule_caret_blink(&self) {
        // SAFETY: main thread, selector has no arguments, common runloop modes.
        unsafe {
            self.performSelector_withObject_afterDelay_inModes(
                sel!(tekesCaretBlink),
                None,
                0.5,
                &NSArray::from_slice(&[NSRunLoopCommonModes]),
            );
        }
    }
    pub(crate) fn new(
        mtm: MainThreadMarker,
        shared: Rc<Shared>,
        frame: NSRect,
    ) -> Retained<TekesView> {
        let this = Self::alloc(mtm).set_ivars(ViewIvars {
            shared,
            layer: RefCell::new(None),
            tracking_area: RefCell::new(None),
            link: RefCell::new(None),
            metal_link: RefCell::new(None),
            metal_start_pending: Cell::new(false),
            live_resizing: Cell::new(false),
        });
        // SAFETY: NSView 的指定初始化方法 initWithFrame: 签名正确。
        let view: Retained<TekesView> = unsafe { msg_send![super(this), initWithFrame: frame] };
        // 注意：`setWantsLayer(true)` 会把重绘策略重置成 Never，所以策略必须在它之后设置
        // （实测）。OnSetNeedsDisplay：AppKit 不因缩放自动重绘层内容。缩放时的重绘由
        // `setFrameSize:` 同步投递；如果用 DuringViewResize，AppKit 会在缩放时再自己重绘一次，
        // 同一次缩放画两帧，还会与渲染层写入 drawable 的内容冲突。
        view.setWantsLayer(true);
        view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::OnSetNeedsDisplay);
        // 新 drawable 还没来时，旧内容按原尺寸钉在左上角，不拉伸。
        // 不能直接设层的 contentsGravity：AppKit 管着背衬层，会按视图的 layerContentsPlacement
        // 把它改回 resize（实测）。TopLeft 是对视图而言的“左上”，AppKit 会处理翻转。
        view.setLayerContentsPlacement(NSViewLayerContentsPlacement::TopLeft);
        view
    }

    /// 在主 runloop 上排一次 `tekesPerformRedraw`。同一时刻是否已排由调用方用 `redraw_pending` 保证。
    pub(crate) fn schedule_redraw(&self) {
        // SAFETY: selector 是上面定义的无参方法；modes 数组里是 AppKit 导出的全局常量。
        unsafe {
            let modes = NSArray::from_slice(&[NSRunLoopCommonModes]);
            self.performSelectorOnMainThread_withObject_waitUntilDone_modes(
                sel!(tekesPerformRedraw),
                None,
                false,
                Some(&modes),
            );
        }
    }

    /// 启动或暂停显示链接。第一次启动时创建：`NSView.displayLink(target:selector:)`（macOS 14），
    /// 自动跟随视图所在窗口的屏幕；加进主 runloop 的 common modes（实时缩放、菜单跟踪期间也能走）；
    /// 帧率范围 60–120，首选 120，ProMotion 上给到 120Hz，只有 60Hz 的屏幕自然落到 60。
    /// 返回启动之后链接是否在跑（系统不支持时为 false）；暂停永远返回 false。
    pub(crate) fn set_display_link_running(&self, run: bool) -> bool {
        if Self::uses_metal_display_link() && !self.ivars().live_resizing.get() {
            return self.set_metal_display_link_running(run);
        }
        let existing = self.ivars().link.borrow().clone();
        if !run {
            if let Some(link) = existing {
                link.setPaused(true);
            }
            return false;
        }
        let link = match existing {
            Some(link) => link,
            None => {
                if !self.respondsToSelector(sel!(displayLinkWithTarget:selector:)) {
                    return false;
                }
                // SAFETY: target 是 self，selector 是上面 define_class 里定义的、接受一个
                // CADisplayLink 参数的方法。
                let link = unsafe {
                    self.displayLinkWithTarget_selector(self, sel!(tekesDisplayLinkFired:))
                };
                link.setPreferredFrameRateRange(CAFrameRateRange::new(120.0, 120.0, 120.0));
                link.setPaused(true);
                // SAFETY: 主线程；common modes 是 Foundation 导出的全局常量。
                unsafe {
                    link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
                }
                *self.ivars().link.borrow_mut() = Some(link.clone());
                link
            }
        };
        link.setPaused(false);
        true
    }

    fn uses_metal_display_link() -> bool {
        std::env::var("TEKES_METAL_DISPLAY_LINK").as_deref() == Ok("1")
    }

    /// CAMetalDisplayLink owns layer drawable-count configuration while alive.
    /// wgpu reconfigures it on resize, so detach the link before that operation.
    fn stop_metal_link(&self) {
        let link = self.ivars().metal_link.borrow_mut().take();
        if let Some(link) = link {
            link.setPaused(true);
            link.invalidate();
            link.setDelegate(None);
        }
        self.ivars().shared.discard_metal_frames();
        self.ivars()
            .shared
            .presentation
            .metal_display_link
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Opt-in diagnostic driver. The ordinary production driver is unchanged.
    fn set_metal_display_link_running(&self, run: bool) -> bool {
        let cad = self.ivars().link.borrow().clone();
        if let Some(cad) = cad {
            cad.setPaused(true);
        }
        let existing = self.ivars().metal_link.borrow().clone();
        if !run {
            self.stop_metal_link();
            return false;
        }
        if self.ivars().live_resizing.get() {
            return true;
        }
        if existing.as_ref().is_some_and(|link| !link.isPaused()) {
            return true;
        }
        self.schedule_metal_start();
        true
    }

    fn schedule_metal_start(&self) {
        if !self.ivars().metal_start_pending.replace(true) {
            // SAFETY: exact zero-argument selector on this main-thread view.
            unsafe {
                self.performSelector_withObject_afterDelay_inModes(
                    sel!(tekesStartMetalLink),
                    None,
                    0.001,
                    &NSArray::from_slice(&[NSRunLoopCommonModes]),
                );
            }
        }
    }

    fn start_metal_link_now(&self) {
        let existing = self.ivars().metal_link.borrow().clone();
        let link = existing.unwrap_or_else(|| {
            let link = CAMetalDisplayLink::initWithMetalLayer(
                CAMetalDisplayLink::alloc(),
                &self.metal_layer(),
            );
            link.setPaused(true);
            link.setDelegate(Some(ProtocolObject::from_ref(self)));
            link.setPreferredFrameLatency(1.0);
            link.setPreferredFrameRateRange(CAFrameRateRange::new(120.0, 120.0, 120.0));
            *self.ivars().metal_link.borrow_mut() = Some(link.clone());
            // SAFETY: main-thread NSView; common modes continue during tracking.
            unsafe {
                link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
            }
            link
        });
        self.ivars()
            .shared
            .presentation
            .metal_display_link
            .store(true, std::sync::atomic::Ordering::Relaxed);
        link.setPaused(false);
    }

    /// 窗口关闭或应用结束时调用：invalidate 显示链接，断开它对视图的强引用。
    pub(crate) fn invalidate_display_link(&self) {
        self.stop_metal_link();
        let cad = self.ivars().link.borrow_mut().take();
        if let Some(link) = cad {
            link.invalidate();
        }
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

    /// Selftest-only observation; counts actual positive Metal presentation receipts.
    pub(crate) fn selftest_presented_count(&self) -> u64 {
        let observer = &self.ivars().shared.presentation;
        observer
            .enabled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        observer.frames.lock().map_or(0, |frames| {
            frames
                .iter()
                .filter(|frame| frame.presented_at > 0.0)
                .count() as u64
        })
    }

    pub(crate) fn is_live_resizing(&self) -> bool {
        self.ivars().live_resizing.get()
    }

    pub(crate) fn size_points(&self) -> Size {
        let s = self.bounds().size;
        Size {
            width: s.width,
            height: s.height,
        }
    }

    pub(crate) fn drawable_size_px(&self) -> (u32, u32) {
        self.size_points().to_drawable(self.scale_factor())
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

    /// 把层的 contentsScale 对齐到窗口缩放因子，算出新的 drawable 像素尺寸。
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
        let metal = Self::uses_metal_display_link();
        if metal {
            self.stop_metal_link();
        }
        shared.last_geometry.set(Some(state));
        shared.dispatch(Event::Resized {
            size,
            scale,
            drawable_size: drawable,
        });
        // 同步重绘：不等下一轮 runloop，否则实时缩放时窗口已经变了而内容还是旧尺寸。
        // 这一帧同时满足此前挂起的重绘请求；runloop 里已排的 `tekesPerformRedraw` 到点时
        // 看到标志已清，什么也不做。
        shared.redraw_pending.set(false);
        shared.dispatch(Event::Redraw);
        if metal && shared.animating.get() && !self.ivars().live_resizing.get() {
            self.schedule_metal_start();
        }
    }
}
