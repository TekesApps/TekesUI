//! Native NSScrollView oracle, built only as a measurement example.
//! Public event adapter preserves a real local event's window/location; no global
//! synthetic events, swizzling, private selectors, or native Gallery control.
//! Reference-only opt-in: TEKES_SCROLL_REFERENCE_CENTERED_BASELINE=1 selects
//! 共同垂直居中排版策略AppKit参照; default native AppKit layout remains unchanged.
//! This policy changes native cached baselines from resolved font metrics, never
//! fixture expected baselines. Newline locations are reported separately from
//! nominal baselines; neither nominal blank baselines nor control glyphs are ink.
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
        MainThreadMarker, NSDictionary, NSObject, NSObjectProtocol, NSPoint, NSRange, NSRect, NSRunLoop,
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
    // Opt-in, body-only AppKit reference. No frame/present/GPU claims are made.
    struct Reference {
        fixture: Value,
        gallery: Value,
        seconds: f64,
        baseline_diagnostic: bool,
        centered_baseline: bool,
        baseline_delegate: Option<Retained<CenteredBaseline>>,
        speed: f64,
        scale: f64,
        width: f64,
        height: f64,
        document_height: f64,
        font_name: String,
        font_size: f64,
        line_height: f64,
        first: Option<f64>,
        last_target: Option<f64>,
        cycle: Option<u64>,
        stage: u8,
        measure: Option<(std::time::Instant, f64)>,
        last_cpu: Option<(std::time::Instant, f64)>,
        cpu_rates: Vec<[f64; 2]>,
        rows: Vec<[f64; 3]>,
        inputs: Vec<[f64; 6]>,
        measure_wall: f64,
        measure_cpu: f64,
        idle: Option<(std::time::Instant, f64)>,
        idle_callbacks: usize,
        invalid_visibility: usize,
        admission: Option<Value>,
        failure: Option<String>,
        written: bool,
    }
    static REFERENCE_EXIT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(2);

    fn cpu_seconds() -> f64 {
        #[repr(C)]
        struct Timespec {
            sec: i64,
            nsec: i64,
        }
        unsafe extern "C" {
            fn clock_gettime(clock: u32, ts: *mut Timespec) -> i32;
        }
        let mut ts = Timespec { sec: 0, nsec: 0 };
        // Same macOS process user+system clock as Gallery; fail instead of zeroing errors.
        assert_eq!(
            unsafe { clock_gettime(12, &mut ts) },
            0,
            "process CPU clock"
        );
        ts.sec as f64 + ts.nsec as f64 * 1e-9
    }

    impl Reference {
        fn from_env() -> Result<Option<Self>, String> {
            let Some(path) = std::env::var_os("TEKES_SCROLL_REFERENCE") else {
                return Ok(None);
            };
            let gallery: Value =
                serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let baseline_diagnostic =
                match std::env::var("TEKES_SCROLL_REFERENCE_BASELINE_DIAGNOSTIC") {
                    Err(std::env::VarError::NotPresent) => false,
                    Ok(value) if value == "0" => false,
                    Ok(value) if value == "1" => true,
                    _ => {
                        return Err(
                            "TEKES_SCROLL_REFERENCE_BASELINE_DIAGNOSTIC must be 0 or 1".into()
                        );
                    }
                };
            let centered_baseline = match std::env::var("TEKES_SCROLL_REFERENCE_CENTERED_BASELINE") {
                Err(std::env::VarError::NotPresent) => false,
                Ok(value) if value == "0" => false,
                Ok(value) if value == "1" => true,
                _ => return Err("TEKES_SCROLL_REFERENCE_CENTERED_BASELINE must be 0 or 1".into()),
            };
            let fixture = gallery["reference_fixture"].clone();
            let number = |v: &Value| {
                v.as_f64()
                    .filter(|n| n.is_finite() && *n > 0.0)
                    .ok_or_else(|| "missing/nonpositive reference metric".to_string())
            };
            if gallery["completed"] != true
                || gallery["visibility"]["occluded_callbacks"] != 0
                || gallery["visibility"]["non_key_callbacks"] != 0
                || gallery["reference_scroll"]["samples"]
                    .as_array()
                    .is_none_or(|rows| rows.is_empty())
                || fixture["schema"] != "tekesui-scroll-reference-fixture/1"
                || gallery["config"]["content"] != "scroll-area"
                || fixture["text"].as_str()
                    != Some(
                        include_str!("../../../fixtures/text/gallery-reading-en.txt").trim_end(),
                    )
                || fixture["normalization"] != "gallery-reading-en.txt.trim_end()"
                || fixture["read_only"] != true
                || fixture["multiline"] != true
            {
                return Err("requires an actual read-only Gallery ScrollArea fixture report with identical text".into());
            }
            let fonts = fixture["fonts"]
                .as_array()
                .ok_or("missing resolved fonts")?;
            if fonts.len() != 1 || fonts[0]["is_color"] != false {
                return Err("this bounded mode requires one resolved non-color font; fallback-font matching is unimplemented".into());
            }
            let font_name = fonts[0]["post_script_name"]
                .as_str()
                .ok_or("missing resolved font name")?
                .to_owned();
            let font_size = number(&fonts[0]["size_pt"])?;
            let width = number(&fixture["inner_rect_pt"][2])?;
            let height = number(&fixture["inner_rect_pt"][3])?;
            let wrap = number(&fixture["wrap_width_pt"])?;
            let document_height = number(&fixture["paragraph_height_pt"])?;
            let scale = number(&gallery["config"]["scale"])?;
            let speed = number(&gallery["config"]["speed_pt_s"])?;
            let seconds = std::env::var("TEKES_SCROLL_SECONDS")
                .unwrap_or_else(|_| "60".into())
                .parse::<f64>()
                .map_err(|e| e.to_string())?;
            if !seconds.is_finite()
                || seconds <= 0.0
                || gallery["config"]["seconds"].as_f64() != Some(seconds)
                || gallery["config"]["warmup_s"].as_f64() != Some(1.5)
                || (width - wrap).abs() > 0.5 / scale
                || (document_height - height - number(&fixture["scroll_limit_pt"])?).abs() > 1.0
            {
                return Err("invalid duration or nonmatching viewport/wrap/extent".into());
            }
            let lines = fixture["lines"]
                .as_array()
                .filter(|l| !l.is_empty())
                .ok_or("missing measured lines")?;
            if lines.iter().any(|line| {
                ["top_pt", "height_pt", "baseline_pt"].iter().any(|key| {
                    line[*key]
                        .as_f64()
                        .is_none_or(|value| !value.is_finite() || value < 0.0)
                }) || line["utf16"][0]
                    .as_u64()
                    .zip(line["utf16"][1].as_u64())
                    .is_none_or(|(start, end)| {
                        start > end
                            || end > fixture["text"].as_str().unwrap().encode_utf16().count() as u64
                    })
            }) {
                return Err("invalid measured line metadata".into());
            }
            let line_height = number(&lines[0]["height_pt"])?;
            if lines.iter().any(|l| {
                l["height_pt"]
                    .as_f64()
                    .is_none_or(|h| (h - line_height).abs() > 0.001)
            }) {
                return Err(
                    "nonuniform measured line heights are not supported by this mode".into(),
                );
            }
            let out = std::env::var_os("TEKES_SCROLL_ORACLE_OUT")
                .ok_or("explicit fresh TEKES_SCROLL_ORACLE_OUT required")?;
            if std::path::Path::new(&out).exists() {
                return Err("refusing to overwrite reference output".into());
            }
            Ok(Some(Self {
                fixture,
                gallery,
                seconds,
                baseline_diagnostic,
                centered_baseline,
                baseline_delegate: None,
                speed,
                scale,
                width,
                height,
                document_height,
                font_name,
                font_size,
                line_height,
                first: None,
                last_target: None,
                cycle: None,
                stage: 0,
                measure: None,
                last_cpu: None,
                cpu_rates: Vec::new(),
                rows: Vec::new(),
                inputs: Vec::new(),
                measure_wall: 0.0,
                measure_cpu: 0.0,
                idle: None,
                idle_callbacks: 0,
                invalid_visibility: 0,
                admission: None,
                failure: None,
                written: false,
            }))
        }
    }

    fn configure_reference(
        doc: &NumberedPaper,
        scroll: &MeasuredScroll,
        window: &NSWindow,
    ) -> Result<Value, String> {
        CAPTURE.with_borrow_mut(|c| {
            let r = c.reference.as_mut().unwrap();
            if (window.backingScaleFactor()-r.scale).abs() > 0.001 {
                return Err("backing scale differs from Gallery".into());
            }
            // Match Gallery TextEngine::prepare_proposal, not an NSFont name alias.
            let role = tekes_ui_tokens::proposal::TextRole::ComposerBody;
            if r.fixture["role"].as_str() != Some(role.id()) {
                return Err("CoreText system-font reference requires the measured composer-body role".into());
            }
            let style = tekes_ui_text::style::TextStyle::system(r.font_size as f32)
                .with_weight(role.style().weight);
            let ct_font = tekes_ui_text::coretext::CoreTextFonts::new().create_font(&style);
            // SAFETY: read-only queries on the owned CTFont returned above.
            let (ct_name, ct_size) = unsafe {
                (ct_font.post_script_name().to_string(), ct_font.size())
            };
            if ct_name != r.font_name || ct_size.to_bits() != r.font_size.to_bits() {
                let appkit_system = NSFont::systemFontOfSize(r.font_size);
                return Err(format!(
                    "font identity mismatch: expected CoreText PS={:?} size={:?}; Gallery API created PS={:?} size={:?}; AppKit systemFont name={:?} size={:?}; no font substitution applied",
                    r.font_name, r.font_size, ct_name, ct_size,
                    appkit_system.fontName().to_string(), appkit_system.pointSize()
                ));
            }
            // SAFETY: Apple documents CTFontRef/NSFont as toll-free bridged on macOS:
            // https://developer.apple.com/library/archive/documentation/StringsTextFonts/Conceptual/CoreText_Programming/Overview/Overview.html
            // Retain this exact CTFont as NSFont while its CF owner is alive. The
            // new ObjC retain is balanced by Retained; no ownership is transferred.
            let font: Retained<NSFont> = unsafe {
                Retained::retain(std::ptr::from_ref(&*ct_font).cast::<NSFont>().cast_mut())
            }.expect("owned nonnull CTFont has an NSFont toll-free bridge");
            if font.pointSize().to_bits() != ct_size.to_bits() {
                return Err(format!("CTFont/NSFont bridge size mismatch: PS={ct_name:?} CT={ct_size:?} NS={:?} NSFont name={:?}",font.pointSize(),font.fontName().to_string()));
            }
            doc.setFont(Some(&font));
            doc.setSelectable(true);
            doc.setTextContainerInset(NSSize::ZERO);
            let style = NSMutableParagraphStyle::new();
            style.setMinimumLineHeight(r.line_height);
            style.setMaximumLineHeight(r.line_height);
            doc.setDefaultParagraphStyle(Some(&style));
            // Public NSLayoutManager/NSTextContainer APIs via their exact ObjC ABI;
            // these classes are not enabled as typed bindings in this example's Cargo features.
            let (manager, container): (Retained<objc2::runtime::AnyObject>, Retained<objc2::runtime::AnyObject>) = unsafe {
                (msg_send![doc, layoutManager], msg_send![doc, textContainer])
            };
            // NSFont uses a negative descender; Gallery's descent is positive.
            // These are queries on the exact CTFont/NSFont object above, not fixture baselines.
            let ascent = font.ascender();
            let descent = -font.descender();
            let leading = font.leading();
            if ![ascent, descent, leading].iter().all(|v| v.is_finite()) || ascent <= 0.0 || descent < 0.0 {
                return Err("invalid resolved NSFont vertical metrics".into());
            }
            if r.centered_baseline {
                let delegate = CenteredBaseline::alloc(MainThreadMarker::new().unwrap()).set_ivars(BaselinePolicy {
                    ascent, descent, calls: RefCell::new(Vec::new()),
                });
                let delegate: Retained<CenteredBaseline> = unsafe { msg_send![super(delegate), init] };
                unsafe { let (): () = msg_send![&*manager, setDelegate: &*delegate]; }
                // NSLayoutManager's delegate is weak. Keep it alive through scrolling/finish.
                r.baseline_delegate = Some(delegate);
            }
            doc.setString(&NSString::from_str(r.fixture["text"].as_str().unwrap()));
            unsafe {
                let (): () = msg_send![&*container, setLineFragmentPadding: 0.0_f64];
                let (): () = msg_send![&*container, setWidthTracksTextView: false];
                let (): () = msg_send![&*container, setContainerSize: NSSize::new(r.fixture["wrap_width_pt"].as_f64().unwrap(), r.document_height+r.height)];
                let (): () = msg_send![&*manager, ensureLayoutForTextContainer: &*container];
            }
            let used: NSRect = unsafe { msg_send![&*manager, usedRectForTextContainer: &*container] };
            let count: usize = unsafe { msg_send![&*manager, numberOfGlyphs] };
            let mut index = 0;
            let mut native_lines = Vec::new();
            let utf16: Vec<u16> = r.fixture["text"].as_str().unwrap().encode_utf16().collect();
            while index < count {
                let mut range = NSRange::new(0, 0);
                let rect: NSRect = unsafe { msg_send![&*manager, lineFragmentRectForGlyphAtIndex: index, effectiveRange: &mut range] };
                let chars: NSRange = unsafe { msg_send![&*manager, characterRangeForGlyphRange: range, actualGlyphRange: std::ptr::null_mut::<NSRange>()] };
                let baseline: NSPoint = unsafe { msg_send![&*manager, locationForGlyphAtIndex: index] };
                let newline_only = utf16[chars.location..chars.location+chars.length].iter()
                    .all(|c| *c == u16::from(b'\n') || *c == u16::from(b'\r'));
                let nominal = (rect.size.height-ascent-descent)*0.5+ascent;
                // Newline control glyph locations are not visible ink baselines.
                // Nonempty lines always use the real, post-layout glyph location.
                let comparison_baseline = if newline_only { nominal } else { baseline.y };
                if r.centered_baseline {
                    let calls = r.baseline_delegate.as_ref().unwrap().ivars().calls.borrow();
                    if !calls.iter().any(|call| call["glyph_range"] == json!([range.location, range.location+range.length])
                        && call["top_pt"] == rect.origin.y && call["height_pt"] == rect.size.height) {
                        return Err(format!("centered baseline delegate did not observe line at glyph {index}"));
                    }
                }
                native_lines.push(json!({"utf16":[chars.location, chars.location+chars.length],
                    "top_pt":rect.origin.y,"height_pt":rect.size.height,
                    "baseline_pt":rect.origin.y+comparison_baseline,
                    "baseline_semantics":if newline_only {"nominal font-metric baseline; no visible ink"} else {"first visible line glyph baseline"},
                    "newline_only":newline_only,
                    "nominal_baseline_pt":rect.origin.y+nominal,
                    "control_glyph_location_pt":if newline_only {Some(rect.origin.y+baseline.y)} else {None},
                    "first_glyph_location_pt":rect.origin.y+baseline.y}));
                if range.length == 0 { return Err("AppKit returned empty glyph line".into()); }
                index = range.location+range.length;
            }
            let uses_font_leading: bool = unsafe { msg_send![&*manager, usesFontLeading] };
            let (ct_ascent, ct_descent, ct_leading) = unsafe {
                (ct_font.ascent(), ct_font.descent(), ct_font.leading())
            };
            let baseline_policy = json!({
                "id":if r.centered_baseline {"shared-font-metric-vertical-center-appkit-reference/v1"} else {"appkit-default/v1"},
                "name":if r.centered_baseline {"共同垂直居中排版策略AppKit参照"} else {"AppKit default"},
                "font_post_script_name":ct_name,"font_size_pt":ct_size,
                "nsfont_ascender_pt":ascent,"nsfont_positive_descent_pt":descent,"nsfont_leading_pt":leading,
                "uses_font_leading":uses_font_leading,
                "coretext_ascent_pt":ct_ascent,"coretext_descent_pt":ct_descent,"coretext_leading_pt":ct_leading,
                "formula":"(actual lineFragment height - NSFont ascender - positive descent)/2 + ascender",
                "expected_baselines_used_to_set_layout":false,
                "delegate_calls":r.baseline_delegate.as_ref().map(|d|d.ivars().calls.borrow().clone()),
                "blank_comparison":"nominal font-metric baseline, not control glyph location or visible ink"
            });
            let expected = r.fixture["lines"].as_array().unwrap();
            let tolerance = 0.5/r.scale;
            let baseline_differences: Vec<Value> = native_lines.iter().zip(expected).enumerate()
                .filter_map(|(index,(actual,expected))| {
                    let delta = actual["baseline_pt"].as_f64().unwrap()-expected["baseline_pt"].as_f64().unwrap();
                    (delta.abs()>tolerance).then(||json!({"index":index,"utf16":expected["utf16"],
                        "actual_baseline_pt":actual["baseline_pt"],"expected_baseline_pt":expected["baseline_pt"],
                        "delta_pt":delta}))
                }).collect();
            let non_baseline_mismatch = native_lines.len() != expected.len() || native_lines.iter().zip(expected).any(|(a,b)|
                a["utf16"] != b["utf16"] || ["top_pt","height_pt"].iter().any(|k|
                    (a[*k].as_f64().unwrap()-b[*k].as_f64().unwrap()).abs() > tolerance))
                || (used.size.height-r.document_height).abs() > 1.0
                || (scroll.contentView().bounds().size.width-r.width).abs() > tolerance
                || (scroll.contentView().bounds().size.height-r.height).abs() > tolerance;
            if non_baseline_mismatch || ((!r.baseline_diagnostic || r.centered_baseline) && !baseline_differences.is_empty()) {
                let first_line_difference = native_lines.iter().zip(expected).enumerate().find(|(_, (a, b))| {
                    a["utf16"] != b["utf16"] || ["top_pt", "height_pt", "baseline_pt"].iter().any(|k|
                        (a[*k].as_f64().unwrap()-b[*k].as_f64().unwrap()).abs() > tolerance)
                }).map(|(index, (actual, expected))| json!({"index":index,"actual":actual,"expected":expected}));
                return Err(format!("AppKit layout mismatch: {}", json!({
                    "font_post_script_name":ct_name,"font_point_size":ct_size,"appkit_font_name":font.fontName().to_string(),
                    "actual_viewport_pt":[scroll.contentView().bounds().size.width,scroll.contentView().bounds().size.height],
                    "expected_viewport_pt":[r.width,r.height],"actual_used_height_pt":used.size.height,
                    "expected_height_pt":r.document_height,"geometry_tolerance_pt":tolerance,
                    "first_line_difference":first_line_difference,"actual_lines":native_lines,"baseline_policy":baseline_policy,
                    "baseline_diagnostic_requested":r.baseline_diagnostic,"non_baseline_mismatch":non_baseline_mismatch,
                    "baseline_differences":baseline_differences,
                    "expected_lines_source":"fixture.lines in the same report"
                })));

            }
            let exact_layout_passed = baseline_differences.is_empty();
            let comparison_scope = if exact_layout_passed && r.centered_baseline { "body input and line geometry under explicit shared vertical-center policy; blank baselines nominal; Gallery chrome excluded" }
                else if exact_layout_passed { "body input and line geometry; Gallery chrome excluded" }
                else { "partial body CPU/scroll state; baseline mismatch retained; no exact layout acceptance" };
            Ok(json!({"status":if exact_layout_passed && r.centered_baseline {"MATCHED_BODY_INPUT_AND_LINE_GEOMETRY_SHARED_VERTICAL_CENTER_POLICY"} else if exact_layout_passed {"MATCHED_BODY_INPUT_AND_LINE_GEOMETRY"} else {"PARTIAL_BODY_CPU_SCROLL_STATE_DIAGNOSTIC"},
                "scope":if exact_layout_passed {"text body only; Gallery chrome excluded"} else {comparison_scope},"comparison_scope":comparison_scope,"exact_layout_passed":exact_layout_passed,
                "baseline_diagnostic_requested":r.baseline_diagnostic,"baseline_differences":baseline_differences,
                "font_post_script_name":ct_name,"font_point_size":ct_size,
                "appkit_font_name":font.fontName().to_string(),
                "font_source":"Gallery CoreTextFonts::create_font(system + ComposerBody weight); same object toll-free bridged",
                "lines":native_lines,"baseline_policy":baseline_policy,
                "used_height_pt":used.size.height,"viewport_pt":[r.width,r.height],"backing_scale":window.backingScaleFactor(),
                "native_resolved_glyph_fallback":"UNKNOWN; exact base font and measured line geometry matched",
                "actual_presentation":"UNKNOWN","gpu_frame_cost":"UNKNOWN"}))
        })
    }

    fn reference_event(
        scroll: &MeasuredScroll,
        seed: &Retained<NSEvent>,
        dy: f64,
        phase: usize,
        momentum: usize,
    ) {
        let event = GestureEvent::alloc(MainThreadMarker::new().unwrap()).set_ivars(Gesture {
            seed: seed.clone(),
            dy,
            phase,
            momentum,
        });
        let event: Retained<GestureEvent> = unsafe { msg_send![super(event), init] };
        scroll.scrollWheel(&event);
    }

    fn advance_reference(scroll: &MeasuredScroll, link: &CADisplayLink) {
        let target = link.targetTimestamp();
        let plan = CAPTURE.with_borrow_mut(|c| {
            c.frames += 1;
            let seed = c.seed.clone()?;
            let r = c.reference.as_mut().unwrap();
            if r.idle.is_some() { r.idle_callbacks += 1; return None; }
            let first = *r.first.get_or_insert(target);
            let elapsed = target-first;
            let dt = r.last_target.map_or(link.duration(), |t| (target-t).max(0.0));
            r.last_target = Some(target);
            if elapsed >= 1.5 && r.measure.is_none() {
                let now = std::time::Instant::now(); let cpu = cpu_seconds();
                r.measure = Some((now,cpu)); r.last_cpu = Some((now,cpu));
            }
            if elapsed >= 1.5+r.seconds {
                let (at,cpu) = r.measure.unwrap();
                r.measure_wall = at.elapsed().as_secs_f64();
                r.measure_cpu = cpu_seconds()-cpu;
                r.idle = Some((std::time::Instant::now(),cpu_seconds()));
                if let Some(link) = c.link.take() { link.invalidate(); }
                // One-shot idle wake; no display link/timer polling during idle.
                unsafe {
                    let _: Retained<objc2::runtime::AnyObject> = msg_send![objc2::class!(NSTimer),
                        scheduledTimerWithTimeInterval: 5.0_f64, target: scroll, selector: sel!(oracleFinish:),
                        userInfo: std::ptr::null::<objc2::runtime::AnyObject>(), repeats: false];
                }
                return None;
            }
            let cycle = (elapsed/2.0).floor() as u64;
            let t = elapsed%2.0;
            let direction = if cycle.is_multiple_of(2) {1.0} else {-1.0};
            let reset = if r.cycle != Some(cycle) { r.cycle=Some(cycle); r.stage=0;
                Some(if direction>0.0 {(r.document_height-r.height-200.0).max(0.0)} else {200.0}) } else {None};
            let mut events = Vec::with_capacity(3);
            if reset.is_some() { events.push((0.0,1,0)); }
            if t < 0.35 { events.push((-direction*r.speed*dt,4,0)); }
            else if t < 1.0 {
                if r.stage==0 {events.push((0.0,8,0));r.stage=1;}
                events.push((-direction*r.speed*(-8.0*(t-0.35)).exp()*dt,0,4));
            } else if r.stage==1 { events.push((0.0,0,8));r.stage=2; }
            Some((seed,reset,events,elapsed))
        });
        let Some((seed, reset, events, elapsed)) = plan else {
            return;
        };
        if let Some(y) = reset {
            scroll.contentView().scrollToPoint(NSPoint::new(0.0, y));
        }
        for (dy, phase, momentum) in events {
            reference_event(scroll, &seed, dy, phase, momentum);
        }
        CAPTURE.with_borrow_mut(|c| {
            let r = c.reference.as_mut().unwrap();
            if elapsed >= 1.5 {
                r.rows.push([
                    elapsed - 1.5,
                    target,
                    scroll.contentView().bounds().origin.y,
                ]);
                let now = std::time::Instant::now();
                let (last, old_cpu) = r.last_cpu.unwrap();
                if now.duration_since(last).as_secs_f64() >= 0.25 {
                    let cpu = cpu_seconds();
                    r.cpu_rates.push([
                        r.measure.unwrap().0.elapsed().as_secs_f64(),
                        (cpu - old_cpu) / now.duration_since(last).as_secs_f64() * 100.0,
                    ]);
                    r.last_cpu = Some((now, cpu));
                }
                if let Some(w) = &c.native_window
                    && (!w.isKeyWindow()
                        || !w
                            .occlusionState()
                            .contains(objc2_app_kit::NSWindowOcclusionState::Visible))
                {
                    r.invalid_visibility += 1;
                }
            }
        });
    }

    fn finish_reference(r: &mut Reference) {
        if r.written {
            return;
        }
        let idle = r.idle.map(|(at, cpu)| {
            json!({"wait_s":at.elapsed().as_secs_f64(),
            "process_cpu_percent":(cpu_seconds()-cpu)/at.elapsed().as_secs_f64()*100.0,
            "display_link_callbacks":r.idle_callbacks})
        });
        let completed = r.failure.is_none()
            && r.admission.is_some()
            && r.invalid_visibility == 0
            && r.measure_wall >= r.seconds - 0.02
            && idle
                .as_ref()
                .is_some_and(|v| v["wait_s"].as_f64().unwrap() >= 5.0)
            && r.idle_callbacks == 0;
        let report = json!({"schema":"tekesui-appkit-scroll-reference/1","completed":completed,
            "failure":r.failure,"fixture":r.fixture,"admission":r.admission,
            "baseline_diagnostic_requested":r.baseline_diagnostic,
            "exact_layout_passed":r.admission.as_ref().map(|a|&a["exact_layout_passed"]),
            "comparison_scope":r.admission.as_ref().map(|a|&a["comparison_scope"]),
            "gallery_machine":r.gallery["machine"],"gallery_display":r.gallery["display"],
            "identity_note":"program/source/profile and actual machine/OS/display binding require the root execution receipt; embedded Gallery metadata describes the input, not this process",
            "seconds":r.seconds,"speed_pt_s":r.speed,"warmup_s":1.5,
            "workload":"same Gallery2s alternating top/bottom .35s finger/.65s momentum/1s settle formula; public local NSEvent adapter",
            "process":{"cpu_percent_while_scrolling":if r.measure_wall>0.0 {Some(r.measure_cpu/r.measure_wall*100.0)}else{None},
                "measure_wall_s":r.measure_wall,"cpu_rate_unit":"100% = one logical CPU core; process user+system clock; sampled every >=250ms",
                "cpu_rate_samples_elapsed_s_percent":r.cpu_rates},
            "scroll_columns":["elapsed_measure_s","callback_target_s","native_y"],"scroll_samples":r.rows,
            "input_columns":["time","dx","dy","phase","momentum","precise"],"inputs":r.inputs,
            "visibility_invalid_callbacks":r.invalid_visibility,"idle_after":idle,
            "frame_cpu_ms":null,"frame_gpu_ms":null,"actual_presentation":null,
            "unmeasured":["AppKit per-frame total CPU","exclusive GPU cost","actual target presentation/gaps","physical trackpad parity","whole Gallery chrome"]});
        let path = std::env::var_os("TEKES_SCROLL_ORACLE_OUT").unwrap();
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        r.written = true;
        if completed {
            REFERENCE_EXIT.store(0, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[derive(Default)]
    struct Capture {
        model: ScrollArea,
        reference: Option<Reference>,
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
    struct BaselinePolicy {
        ascent: f64,
        descent: f64,
        calls: RefCell<Vec<Value>>,
    }
    // Public NSLayoutManagerDelegate optional method. Change the native layout cache,
    // not merely the report. No fixture or expected line metadata is accessible here.
    // https://developer.apple.com/documentation/appkit/nslayoutmanagerdelegate/layoutmanager(_:shouldsetlinefragmentrect:linefragmentusedrect:baselineoffset:in:forglyphrange:)
    define_class!(
        #[unsafe(super=NSObject)]
        #[thread_kind=MainThreadOnly]
        #[ivars=BaselinePolicy]
        struct CenteredBaseline;
        unsafe impl NSObjectProtocol for CenteredBaseline {}
        impl CenteredBaseline {
            #[unsafe(method(layoutManager:shouldSetLineFragmentRect:lineFragmentUsedRect:baselineOffset:inTextContainer:forGlyphRange:))]
            fn center_line(&self, _manager: &objc2::runtime::AnyObject,
                rect: std::ptr::NonNull<NSRect>, _used: std::ptr::NonNull<NSRect>,
                mut offset: std::ptr::NonNull<f64>, _container: &objc2::runtime::AnyObject,
                range: NSRange) -> bool {
                // AppKit supplies live in/out pointers for this synchronous delegate call.
                let rect = unsafe { rect.as_ref() };
                let before = unsafe { *offset.as_ref() };
                let after = (rect.size.height-self.ivars().ascent-self.ivars().descent)*0.5+self.ivars().ascent;
                unsafe { *offset.as_mut() = after; }
                self.ivars().calls.borrow_mut().push(json!({
                    "glyph_range":[range.location,range.location+range.length],
                    "top_pt":rect.origin.y,"height_pt":rect.size.height,
                    "baseline_offset_before_pt":before,"baseline_offset_after_pt":after,
                }));
                true
            }
        }
    );
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
      if CAPTURE.with_borrow(|c| c.reference.is_some()) { return; }
      NSColor::lightGrayColor().setStroke();let path=NSBezierPath::bezierPath();path.setLineWidth(0.5);
      let first=(rect.origin.y/20.0).floor() as i32;let last=((rect.origin.y+rect.size.height)/20.0).ceil() as i32;
      for i in first..=last{path.moveToPoint(NSPoint::new(0.0,i as f64*20.0));path.lineToPoint(NSPoint::new(600.0,i as f64*20.0));}path.stroke();
     }
    });
    define_class!(#[unsafe(super=NSScrollView)] #[thread_kind=MainThreadOnly] struct MeasuredScroll;
    unsafe impl NSObjectProtocol for MeasuredScroll {}
    impl MeasuredScroll {
     #[unsafe(method(oracleTick:))] fn tick(&self,link:&CADisplayLink){advance(self,link);}
     #[unsafe(method(oracleFinish:))] fn reference_finish(&self,_timer:&objc2::runtime::AnyObject){
      finish(); if let Some(window)=self.window(){window.performClose(None);}
     }
     #[unsafe(method(scrollWheel:))] fn scroll_wheel(&self,e:&NSEvent) {
      if (std::env::var_os("TEKES_SCROLL_SCRIPT").is_some() || CAPTURE.with_borrow(|c| c.reference.is_some())) && e.downcast_ref::<GestureEvent>().is_none() {
          CAPTURE.with_borrow_mut(|c|{if c.seed.is_none(){c.seed=unsafe{Retained::retain(e as *const _ as *mut NSEvent)};}});
          return; // isolate the scripted sample from additional physical input
      }
      let (x,y,phase,mom,precise)=(e.scrollingDeltaX(),e.scrollingDeltaY(),e.phase().bits(),e.momentumPhase().bits(),e.hasPreciseScrollingDeltas());
      let time=CACurrentMediaTime();
      CAPTURE.with_borrow_mut(|c| {if c.seed.is_none(){c.seed=unsafe{Retained::retain(e as *const _ as *mut NSEvent)};}
       if let Some(r)=&mut c.reference {r.inputs.push([time,x,y,phase as f64,mom as f64,if precise {1.0} else {0.0}]);}
       else {c.model.wheel_at(tekes_ui_scene::Point::new(-x as f32,-y as f32),precise,phase==1,phase==8||mom==8||phase==16||mom==16,mom!=0,time);c.inputs.push(json!([time,x,y,phase,mom,precise]));}});
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
   if let Some(reference)=&mut c.reference {finish_reference(reference); return;}
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
        if CAPTURE.with_borrow(|c| c.reference.is_some()) {
            advance_reference(scroll, link);
            return;
        }
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
                    let dimensions = CAPTURE.with_borrow(|c| {
                        c.reference
                            .as_ref()
                            .map(|r| (r.width, r.height, r.document_height))
                    });
                    let (body_width, body_height, document_height) =
                        dimensions.unwrap_or((600.0, 400.0, 3000.0));
                    // A separate ordinary AppKit window avoids mixing a native drawing oracle
                    // into the production layer-hosted Metal content view.
                    let native_window = unsafe {
                        NSWindow::initWithContentRect_styleMask_backing_defer(
                            NSWindow::alloc(mtm),
                            NSRect::new(
                                NSPoint::ZERO,
                                NSSize::new(body_width + 80.0, body_height + 80.0),
                            ),
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
                        NSRect::new(
                            NSPoint::ZERO,
                            NSSize::new(body_width + 80.0, body_height + 80.0),
                        ),
                    );
                    native_window.setContentView(Some(&root));
                    native_window.makeKeyAndOrderFront(None);
                    let scroll: Retained<MeasuredScroll> = unsafe {
                        msg_send![MeasuredScroll::alloc(mtm),initWithFrame:NSRect::new(NSPoint::new(30.0,30.0),NSSize::new(body_width,body_height))]
                    };
                    scroll.setHasVerticalScroller(true);
                    scroll.setScrollerStyle(NSScrollerStyle::Overlay);
                    scroll.setVerticalScrollElasticity(NSScrollElasticity::Allowed);
                    scroll.setHorizontalScrollElasticity(NSScrollElasticity::None);
                    scroll.setDrawsBackground(true);
                    scroll.setBackgroundColor(&NSColor::whiteColor());
                    let doc: Retained<NumberedPaper> = unsafe {
                        msg_send![NumberedPaper::alloc(mtm),initWithFrame:NSRect::new(NSPoint::ZERO,NSSize::new(body_width,document_height))]
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
                    if dimensions.is_some() {
                        match configure_reference(&doc, &scroll, &native_window) {
                            Ok(admission) => CAPTURE.with_borrow_mut(|c| {
                                c.reference.as_mut().unwrap().admission = Some(admission)
                            }),
                            Err(error) => {
                                CAPTURE.with_borrow_mut(|c| {
                                    c.reference.as_mut().unwrap().failure = Some(error)
                                });
                                // Created is dispatched before platform setup makes the
                                // anchor visible. Closing here would let setup reopen it.
                                CAPTURE.with_borrow_mut(|c| {
                                    c.native_window = Some(native_window);
                                    c.anchor_window = Some(anchor_window);
                                });
                                finish();
                                unsafe {
                                    let _: Retained<objc2::runtime::AnyObject> = msg_send![objc2::class!(NSTimer),
                                        scheduledTimerWithTimeInterval: 0.001_f64, target: &*scroll, selector: sel!(oracleFinish:),
                                        userInfo: std::ptr::null::<objc2::runtime::AnyObject>(), repeats: false];
                                }
                                return;
                            }
                        }
                    }
                    let hud: Option<Retained<Telemetry>> = if dimensions.is_none() {
                        Some(unsafe {
                            msg_send![Telemetry::alloc(mtm),initWithFrame:NSRect::new(NSPoint::new(30.0,435.0),NSSize::new(620.0,40.0))]
                        })
                    } else {
                        None
                    };
                    if let Some(hud) = &hud {
                        hud.setClipsToBounds(true);
                        root.addSubview(hud);
                    }
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
                        c.hud_view = hud;
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
        let reference = Reference::from_env().unwrap_or_else(|error| {
            eprintln!("AppKit reference: {error}");
            std::process::exit(2)
        });
        let reference_mode = reference.is_some();
        CAPTURE.with_borrow_mut(|c| c.reference = reference);
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
        if reference_mode {
            std::process::exit(REFERENCE_EXIT.load(std::sync::atomic::Ordering::Relaxed));
        }
    }
}
