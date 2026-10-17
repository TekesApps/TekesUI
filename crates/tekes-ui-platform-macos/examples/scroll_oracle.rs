//! Native NSScrollView oracle, built only as a measurement example.
//! Public event adapter preserves a real local event's window/location; no global
//! synthetic events, swizzling, private selectors, or native Gallery control.
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    mac::run();
}
#[cfg(target_os = "macos")]
mod mac {
    use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained, sel};
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSBezierPath, NSColor, NSEvent, NSFont,
        NSForegroundColorAttributeName, NSMutableParagraphStyle, NSScrollElasticity, NSScrollView,
        NSScrollerStyle, NSStringDrawing, NSTextView, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{
        MainThreadMarker, NSDictionary, NSObjectProtocol, NSPoint, NSRect, NSRunLoop,
        NSRunLoopCommonModes, NSSize, NSString,
    };
    use objc2_quartz_core::{CACurrentMediaTime, CADisplayLink, CAFrameRateRange};
    use serde_json::{Value, json};
    use std::cell::RefCell;
    use tekes_ui_controls::ScrollArea;
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler, WindowPlacement,
    };
    use tekes_ui_scene::Size;
    fn label(text: &str, point: NSPoint) {
        let color = NSColor::labelColor();
        let value: &objc2::runtime::AnyObject = &color;
        let attrs =
            NSDictionary::from_slices(&[unsafe { NSForegroundColorAttributeName }], &[value]);
        unsafe {
            NSString::from_str(text).drawAtPoint_withAttributes(point, Some(&attrs));
        }
    }
    #[derive(Default)]
    struct Capture {
        model: ScrollArea,
        inputs: Vec<Value>,
        seed: Option<Retained<NSEvent>>,
        script_n: usize,
        hud: String,
        rows: Vec<Value>,
        last: Option<f64>,
        frames: usize,
        start: f64,
        hud_view: Option<Retained<Telemetry>>,
        link: Option<Retained<CADisplayLink>>,
        native_window: Option<Retained<NSWindow>>,
        anchor_window: Option<Retained<NSWindow>>,
    }
    thread_local! {static CAPTURE:RefCell<Capture>=RefCell::new(Capture::default());}
    struct Gesture {
        seed: Retained<NSEvent>,
        dy: f64,
        phase: usize,
        momentum: usize,
    }
    // A public NSEvent adapter preserves the real event's window/location/flags.
    // It does not reify private NSEvent storage; in-bounds release behavior must
    // be checked against physical input before claiming full native parity.
    // Used only inside this calibration process, never posted to another app.
    define_class!(#[unsafe(super=NSEvent)] #[thread_kind=MainThreadOnly] #[ivars=Gesture] struct GestureEvent;
    unsafe impl NSObjectProtocol for GestureEvent {}
    impl GestureEvent {
      #[unsafe(method(type))] fn event_type(&self)->objc2_app_kit::NSEventType {objc2_app_kit::NSEventType::ScrollWheel}
      #[unsafe(method_id(window))] fn event_window(&self)->Option<Retained<objc2_app_kit::NSWindow>> {self.ivars().seed.window(MainThreadMarker::new().unwrap())}
      #[unsafe(method(windowNumber))] fn event_window_number(&self)->isize {self.ivars().seed.windowNumber()}
      #[unsafe(method(timestamp))] fn event_time(&self)->f64 {CACurrentMediaTime()}
      #[unsafe(method(locationInWindow))] fn location(&self)->NSPoint {self.ivars().seed.locationInWindow()}
      #[unsafe(method(modifierFlags))] fn flags(&self)->objc2_app_kit::NSEventModifierFlags {self.ivars().seed.modifierFlags()}
      #[unsafe(method(scrollingDeltaY))] fn scroll_y(&self)->f64 {self.ivars().dy}
      #[unsafe(method(scrollingDeltaX))] fn scroll_x(&self)->f64 {0.0}
      #[unsafe(method(deltaY))] fn delta_y(&self)->f64 {self.ivars().dy}
      #[unsafe(method(deltaX))] fn delta_x(&self)->f64 {0.0}
      #[unsafe(method(hasPreciseScrollingDeltas))] fn precise(&self)->bool {true}
      #[unsafe(method(phase))] fn phase(&self)->objc2_app_kit::NSEventPhase {objc2_app_kit::NSEventPhase::from_bits_retain(self.ivars().phase)}
      #[unsafe(method(momentumPhase))] fn momentum(&self)->objc2_app_kit::NSEventPhase {objc2_app_kit::NSEventPhase::from_bits_retain(self.ivars().momentum)}
    });
    // Native text supplies readable labels; the overlay marks exact 20pt rows.
    define_class!(#[unsafe(super=NSTextView)] #[thread_kind=MainThreadOnly] struct NumberedPaper;
    unsafe impl NSObjectProtocol for NumberedPaper {}
    impl NumberedPaper {
     #[unsafe(method(drawRect:))] fn draw(&self,rect:NSRect){
      let ():()=unsafe{msg_send![super(self),drawRect:rect]};
      NSColor::lightGrayColor().setStroke();let path=NSBezierPath::bezierPath();path.setLineWidth(0.5);
      let first=(rect.origin.y/20.0).floor() as i32;let last=((rect.origin.y+rect.size.height)/20.0).ceil() as i32;
      for i in first..=last{path.moveToPoint(NSPoint::new(0.0,i as f64*20.0));path.lineToPoint(NSPoint::new(600.0,i as f64*20.0));}path.stroke();
     }
    });
    define_class!(#[unsafe(super=NSScrollView)] #[thread_kind=MainThreadOnly] struct MeasuredScroll;
    unsafe impl NSObjectProtocol for MeasuredScroll {}
    impl MeasuredScroll {
     #[unsafe(method(oracleTick:))] fn tick(&self,link:&CADisplayLink){advance(self,link);}
     #[unsafe(method(scrollWheel:))] fn scroll_wheel(&self,e:&NSEvent) {
      if std::env::var_os("TEKES_SCROLL_SCRIPT").is_some() && e.downcast_ref::<GestureEvent>().is_none() {
          CAPTURE.with_borrow_mut(|c|{if c.seed.is_none(){c.seed=unsafe{Retained::retain(e as *const _ as *mut NSEvent)};}});
          return; // isolate the scripted sample from additional physical input
      }
      let (x,y,phase,mom,precise)=(e.scrollingDeltaX(),e.scrollingDeltaY(),e.phase().bits(),e.momentumPhase().bits(),e.hasPreciseScrollingDeltas());
      let time=CACurrentMediaTime();
      CAPTURE.with_borrow_mut(|c| {if c.seed.is_none(){c.seed=unsafe{Retained::retain(e as *const _ as *mut NSEvent)};} c.model.wheel_at(tekes_ui_scene::Point::new(-x as f32,-y as f32),precise,phase==1,phase==8||mom==8||phase==16||mom==16,mom!=0,time);c.inputs.push(json!([time,x,y,phase,mom,precise]));});
      // SAFETY: exact NSResponder scrollWheel: ABI; retain AppKit's original behavior.
      let ():()=unsafe{msg_send![super(self),scrollWheel:e]};
     }
    });
    define_class!(#[unsafe(super=NSView)] #[thread_kind=MainThreadOnly] struct Telemetry;
    unsafe impl NSObjectProtocol for Telemetry {}
    impl Telemetry {
     #[unsafe(method(isFlipped))] fn flipped(&self)->bool {true}
     #[unsafe(method(drawRect:))] fn draw(&self,_rect:NSRect) {
      NSColor::windowBackgroundColor().setFill();NSBezierPath::fillRect(self.bounds());
      CAPTURE.with_borrow(|c|{label(&c.hud,NSPoint::new(0.0,2.0));});
     }
    });

    fn finish() {
        CAPTURE.with_borrow_mut(|c|{
   if let Some(link)=c.link.take(){link.invalidate();}
   let path=std::env::var("TEKES_SCROLL_ORACLE_OUT").unwrap_or_else(|_|concat!(env!("CARGO_MANIFEST_DIR"),"/../../target/scroll-repair/native-capture.json").into());
   std::fs::write(path,serde_json::to_vec_pretty(&json!({"viewport":[600,400],"document":[600,3000],"start":c.start,
    "sample_columns":["sample_time","target_time","native_y","rust_y","native_knob_proportion"],"input_columns":["time","dx","dy","phase","momentum","precise"],"samples":c.rows,"inputs":c.inputs})).unwrap()).unwrap();
  });
    }
    fn advance(scroll: &MeasuredScroll, link: &CADisplayLink) {
        CAPTURE.with_borrow(|c| {
            if c.frames == 0 {
                if let Some(anchor) = &c.anchor_window {
                    anchor.orderOut(None);
                }
                if let Some(window) = &c.native_window {
                    window.makeKeyAndOrderFront(None);
                }
            }
        });
        let target = link.targetTimestamp();
        let (dt, seed, n, start) = CAPTURE.with_borrow_mut(|c| {
            let dt = c.last.map_or(link.duration(), |t| target - t);
            c.last = Some(target);
            let n = c.script_n;
            if c.seed.is_some() {
                c.script_n += 1;
            }
            (dt, c.seed.clone(), n, c.start)
        });
        if std::env::var_os("TEKES_SCROLL_SCRIPT").is_some()
            && let Some(seed) = seed
        {
            if n >= 180 {
                finish();
                if let Some(window) = scroll.window() {
                    window.performClose(None);
                }
                return;
            }
            if n == 0 {
                scroll.contentView().scrollToPoint(NSPoint::new(0.0, start));
                CAPTURE.with_borrow_mut(|c| {
                    c.model.cancel();
                    c.model.scroll_by(0.0, start as f32 - c.model.offset.y);
                });
            }
            let step = std::env::var("TEKES_SCROLL_STEP")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(20.0);
            let case = std::env::var("TEKES_SCROLL_CASE").unwrap_or_else(|_| "top".into());
            let (dy, phase, momentum) = match case.as_str() {
                "edge-reverse" => {
                    if n == 0 {
                        (0.0, 1, 0)
                    } else if n == 1 {
                        (step, 4, 0)
                    } else if n <= 16 {
                        (-step, 4, 0)
                    } else if n == 17 {
                        (0.0, 8, 0)
                    } else {
                        (0.0, 0, 0)
                    }
                }
                "reverse" | "reverse-momentum" => {
                    if n == 0 {
                        (0.0, 1, 0)
                    } else if n <= 12 {
                        (step, 4, 0)
                    } else if n <= 32 {
                        (-step, 4, 0)
                    } else if n == 33 {
                        (0.0, 8, 0)
                    } else if case == "reverse-momentum" && n <= 64 {
                        (
                            -16.0 * (-0.08 * (n as f64 - 34.0)).exp(),
                            0,
                            if n == 34 { 1 } else { 4 },
                        )
                    } else if case == "reverse-momentum" && n == 65 {
                        (0.0, 0, 8)
                    } else {
                        (0.0, 0, 0)
                    }
                }
                "momentum" => {
                    if n == 0 {
                        (0.0, 1, 0)
                    } else if n <= 10 {
                        (-step, 4, 0)
                    } else if n == 11 {
                        (0.0, 8, 0)
                    } else if n <= 42 {
                        (
                            -16.0 * (-0.08 * (n as f64 - 12.0)).exp(),
                            0,
                            if n == 12 { 1 } else { 4 },
                        )
                    } else if n == 43 {
                        (0.0, 0, 8)
                    } else {
                        (0.0, 0, 0)
                    }
                }
                _ => {
                    if n == 0 {
                        (0.0, 1, 0)
                    } else if n <= 20 {
                        (step, 4, 0)
                    } else if n == 21 {
                        (0.0, 8, 0)
                    } else {
                        (0.0, 0, 0)
                    }
                }
            };
            if phase != 0 || momentum != 0 {
                let this =
                    GestureEvent::alloc(MainThreadMarker::new().unwrap()).set_ivars(Gesture {
                        seed,
                        dy,
                        phase,
                        momentum,
                    });
                let event: Retained<GestureEvent> = unsafe { msg_send![super(this), init] };
                scroll.scrollWheel(&event);
            }
        }
        let time = CACurrentMediaTime();
        let y = scroll.contentView().bounds().origin.y;
        let done=CAPTURE.with_borrow_mut(|c|{
   c.model.tick(dt as f32);
   let previous=c.rows.get(c.rows.len().saturating_sub(2));
   let v=previous.map_or(0.0,|r|(y-r[2].as_f64().unwrap())/(time-r[0].as_f64().unwrap()).max(0.0001));
   let rv=previous.map_or(0.0,|r|(f64::from(c.model.visual_offset().y)-r[3].as_f64().unwrap())/(time-r[0].as_f64().unwrap()).max(0.0001));
   let phase=c.inputs.last().map(|r|format!("{}/{}",r[3],r[4])).unwrap_or_default();
   c.hud=format!("NS y={y:7.2} pt    v={v:8.1} pt/s    dt={:.2} ms\nRust y={:7.2} pt    v={rv:8.1} pt/s    phase={phase}",dt*1000.0,c.model.visual_offset().y);
   c.rows.push(json!([time,target,y,c.model.visual_offset().y,scroll.verticalScroller().map(|s|s.knobProportion())]));c.frames+=1;
   if let Some(hud)=&c.hud_view{hud.setNeedsDisplay(true);}
   c.frames>=36000
  });
        if done {
            finish();
            if let Some(window) = scroll.window() {
                window.performClose(None);
            }
        }
    }
    struct Oracle;
    impl WindowHandler for Oracle {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::Created => {
                    let mtm = MainThreadMarker::new().unwrap();
                    let anchor_window = NSApplication::sharedApplication(mtm)
                        .windows()
                        .firstObject()
                        .unwrap();
                    // A separate ordinary AppKit window avoids mixing a native drawing oracle
                    // into the production layer-hosted Metal content view.
                    let native_window = unsafe {
                        NSWindow::initWithContentRect_styleMask_backing_defer(
                            NSWindow::alloc(mtm),
                            NSRect::new(NSPoint::ZERO, NSSize::new(680.0, 480.0)),
                            NSWindowStyleMask::Titled
                                | NSWindowStyleMask::Closable
                                | NSWindowStyleMask::Miniaturizable,
                            NSBackingStoreType::Buffered,
                            false,
                        )
                    };
                    unsafe {
                        native_window.setReleasedWhenClosed(false);
                    }
                    native_window.setTitle(&NSString::from_str("NSScrollView calibration"));
                    native_window.setFrameOrigin(anchor_window.frame().origin);
                    native_window.setDelegate(anchor_window.delegate().as_deref());
                    let root = NSView::initWithFrame(
                        NSView::alloc(mtm),
                        NSRect::new(NSPoint::ZERO, NSSize::new(680.0, 480.0)),
                    );
                    native_window.setContentView(Some(&root));
                    native_window.makeKeyAndOrderFront(None);
                    let scroll: Retained<MeasuredScroll> = unsafe {
                        msg_send![MeasuredScroll::alloc(mtm),initWithFrame:NSRect::new(NSPoint::new(30.0,30.0),NSSize::new(600.0,400.0))]
                    };
                    scroll.setHasVerticalScroller(true);
                    scroll.setScrollerStyle(NSScrollerStyle::Overlay);
                    scroll.setVerticalScrollElasticity(NSScrollElasticity::Allowed);
                    scroll.setHorizontalScrollElasticity(NSScrollElasticity::None);
                    scroll.setDrawsBackground(true);
                    scroll.setBackgroundColor(&NSColor::whiteColor());
                    let doc: Retained<NumberedPaper> = unsafe {
                        msg_send![NumberedPaper::alloc(mtm),initWithFrame:NSRect::new(NSPoint::ZERO,NSSize::new(600.0,3000.0))]
                    };
                    doc.setEditable(false);
                    doc.setSelectable(false);
                    doc.setRichText(false);
                    doc.setVerticallyResizable(false);
                    doc.setDrawsBackground(true);
                    doc.setBackgroundColor(&NSColor::whiteColor());
                    doc.setTextColor(Some(&NSColor::blackColor()));
                    doc.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(14.0, 0.0)));
                    doc.setTextContainerInset(NSSize::new(12.0, 0.0));
                    let style = NSMutableParagraphStyle::new();
                    style.setMinimumLineHeight(20.0);
                    style.setMaximumLineHeight(20.0);
                    doc.setDefaultParagraphStyle(Some(&style));
                    doc.setString(&NSString::from_str(
                        &(0..150)
                            .map(|i| format!("{i:03}    y = {} pt", i * 20))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ));
                    scroll.setDocumentView(Some(&doc));
                    root.addSubview(&scroll);
                    doc.setNeedsDisplay(true);
                    let hud: Retained<Telemetry> = unsafe {
                        msg_send![Telemetry::alloc(mtm),initWithFrame:NSRect::new(NSPoint::new(30.0,435.0),NSSize::new(620.0,40.0))]
                    };
                    hud.setClipsToBounds(true);
                    root.addSubview(&hud);
                    let start = std::env::var("TEKES_SCROLL_START")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    scroll.contentView().scrollToPoint(NSPoint::new(0.0, start));
                    let link = unsafe {
                        scroll.displayLinkWithTarget_selector(&scroll, sel!(oracleTick:))
                    };
                    link.setPreferredFrameRateRange(CAFrameRateRange::new(120.0, 120.0, 120.0));
                    unsafe {
                        link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
                    }
                    CAPTURE.with_borrow_mut(|c| {
                        c.start = start;
                        c.model
                            .layout(Size::new(600.0, 400.0), Size::new(600.0, 3000.0));
                        c.model.scroll_by(0.0, start as f32);
                        c.hud_view = Some(hud);
                        c.link = Some(link);
                        c.native_window = Some(native_window);
                        c.anchor_window = Some(anchor_window);
                    });
                }
                Event::CloseRequested => {
                    finish();
                    let native = CAPTURE.with_borrow(|c| c.native_window.clone());
                    if let Some(native) = native {
                        native.close();
                    }
                    window.close();
                }
                _ => {}
            }
        }
    }
    impl Drop for Oracle {
        fn drop(&mut self) {
            finish();
        }
    }
    pub fn run() {
        App::new(AppConfig {
            name: Some("TekesUI Scroll Calibration".into()),
            ..Default::default()
        })
        .run(
            WindowConfig {
                title: "Calibration anchor".into(),
                size: tekes_ui_platform_macos::Size {
                    width: 680.0,
                    height: 480.0,
                },
                placement: WindowPlacement::Auto {
                    need_120hz: true,
                    size_check: false,
                },
                ..Default::default()
            },
            MenuBar::default(),
            Oracle,
        );
    }
}
