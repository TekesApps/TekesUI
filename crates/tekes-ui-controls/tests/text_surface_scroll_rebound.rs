use std::{rc::Rc, time::Duration};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, ReadingController, ScrollArea,
    TextSurface, TextSurfaceHost, TextSurfacePresentation, TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{Line, Paragraph};
use tekes_ui_tokens::{Rgba, Theme, proposal};

const ROOT: ComponentId = ComponentId(99000);
const FIELD: ComponentId = ComponentId(99001);
const RICH: ComponentId = ComponentId(99002);

struct MotionText {
    now: Duration,
    reduced_motion: bool,
    animation_requested: bool,
}

impl MotionText {
    fn new(now: Duration, reduced_motion: bool) -> Self {
        Self {
            now,
            reduced_motion,
            animation_requested: false,
        }
    }
}

impl ButtonTextRenderer for MotionText {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }

    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: self.now,
            reduced_motion: self.reduced_motion,
        })
    }

    fn request_animation_frame(&mut self) {
        self.animation_requested = true;
    }

    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (text.len() as f32 * 8.0).min(width),
            role.style().line_height,
        )
    }

    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}

impl TextSurfaceHost for MotionText {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        _: Option<f32>,
    ) -> Rc<Paragraph> {
        let line_height = role.style().line_height.max(18.0) as f64;
        let mut lines = Vec::new();
        let mut byte_offset = 0;
        let mut utf16_offset = 0;
        for (index, raw_line) in text.split('\n').enumerate() {
            let byte_len = raw_line.len();
            let utf16_len = raw_line.encode_utf16().count();
            let has_newline = byte_offset + byte_len < text.len();
            let extra_bytes = usize::from(has_newline);
            let extra_utf16 = usize::from(has_newline);
            let top = index as f64 * line_height;
            lines.push(Line {
                index,
                utf16: utf16_offset..(utf16_offset + utf16_len + extra_utf16),
                bytes: byte_offset..(byte_offset + byte_len + extra_bytes),
                ascent: line_height * 0.8,
                descent: line_height * 0.2,
                leading: 0.0,
                height: line_height,
                top,
                baseline_y: top + line_height * 0.8,
                width: 240.0,
                width_trimmed: 240.0,
                runs: Vec::new(),
                carets: vec![(utf16_offset, 0.0), (utf16_offset + utf16_len, 240.0)],
                graphemes: Vec::new(),
                has_newline,
            });
            byte_offset += byte_len + extra_bytes;
            utf16_offset += utf16_len + extra_utf16;
        }
        let height = (lines.len() as f64 * line_height).max(line_height);
        Rc::new(Paragraph {
            lines,
            height,
            width: 240.0,
            default_line_height: line_height,
            fonts: Vec::new(),
            utf16_len: utf16_offset,
        })
    }
}

impl TextSurfaceTextRenderer for MotionText {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}

fn read_only_text_field() -> TextFieldContent {
    let body = "A day by the window.\n".repeat(48);
    let mut surface = TextSurface::new(&body, true, true);
    surface.set_presentation(TextSurfacePresentation::Content);
    let mut field = TextFieldContent::from_surface("Text · select and copy", surface)
        .with_show_metadata(false)
        .with_viewport(TextViewport::Fill);
    field.reading = Some(ReadingController::default());
    field
}

fn wheel(
    delta_y: f32,
    phase: ContentWheelPhase,
    momentum_phase: ContentWheelPhase,
    time: f64,
) -> ContentWheelEvent {
    ContentWheelEvent {
        position: Point::new(120.0, 120.0),
        delta: Point::new(0.0, delta_y),
        precise: true,
        phase,
        momentum_phase,
        time,
    }
}

fn render_with(host: &mut ContentHost, text: &mut MotionText) -> bool {
    text.animation_requested = false;
    let frame = Rect::new(0.0, 0.0, 360.0, 220.0);
    host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
        .unwrap();
    text.animation_requested
}

#[test]
fn read_only_text_rebounds_top_overscroll_and_accepts_subsequent_scroll() {
    use ContentWheelPhase as Phase;

    let mut host = ContentHost::new(Content::new(FIELD, read_only_text_field())).unwrap();
    let mut text = MotionText::new(Duration::ZERO, false);
    assert!(!render_with(&mut host, &mut text));

    // A long idle gap before scrolling must not be integrated as a jump on frame 1.
    text.now = Duration::from_secs(10);
    assert!(!render_with(&mut host, &mut text));

    let mut clock = 10.0;
    host.wheel_event(&wheel(0.0, Phase::Began, Phase::None, clock));
    host.wheel_event(&wheel(240.0, Phase::Changed, Phase::None, clock + 0.01));
    host.wheel_event(&wheel(0.0, Phase::Ended, Phase::None, clock + 0.02));
    assert!(host.text_surface(FIELD).unwrap().scroll().offset.y > 100.0);

    // Flick back up so trackpad momentum crosses offset 0 at 1:1 elasticity.
    clock = 10.1;
    host.wheel_event(&wheel(0.0, Phase::Began, Phase::None, clock));
    host.wheel_event(&wheel(-180.0, Phase::Changed, Phase::None, clock + 0.01));
    host.wheel_event(&wheel(0.0, Phase::Ended, Phase::None, clock + 0.02));
    host.wheel_event(&wheel(-120.0, Phase::None, Phase::Began, clock + 0.03));
    host.wheel_event(&wheel(-60.0, Phase::None, Phase::Changed, clock + 0.04));
    host.wheel_event(&wheel(0.0, Phase::None, Phase::Ended, clock + 0.05));

    {
        let s = host.text_surface(FIELD).unwrap();
        assert_eq!(s.scroll().offset.y, 0.0);
        assert!(
            s.scroll().overscroll.y <= -50.0,
            "expected large top elastic negative overscroll, got {:?}",
            s.scroll().overscroll
        );
        assert!(s.scroll().needs_animation());
    }

    // First post-overscroll frame must request animation without snapping instantly across idle time.
    text.now = Duration::from_secs_f64(10.05);
    assert!(render_with(&mut host, &mut text));
    assert!(
        host.text_surface(FIELD).unwrap().scroll().overscroll.y < -0.5,
        "first frame must preserve elastic stretch rather than snapping in 0ms"
    );

    let step = Duration::from_secs_f64(1.0 / 120.0);
    let mut frames = 0;
    while frames < 240 {
        frames += 1;
        text.now += step;
        let requested = render_with(&mut host, &mut text);
        if !requested {
            break;
        }
    }
    assert!(frames < 240, "scroll rebound did not settle within 2s");

    {
        let s = host.text_surface(FIELD).unwrap();
        assert_eq!(s.scroll().overscroll, Point::default());
        assert_eq!(s.scroll().offset.y, 0.0);
        assert_eq!(s.scroll().visual_offset().y, 0.0);
        assert!(!s.scroll().needs_animation());
        assert!(
            (s.origin().y - s.inner().min_y()).abs() < 0.01,
            "top blank gap remained after rebound: origin={}, inner_min_y={}",
            s.origin().y,
            s.inner().min_y()
        );
    }

    // Immediate downward gesture and momentum must scroll at full 1:1 rate, never stiffened/ignored.
    clock = text.now.as_secs_f64() + 0.05;
    host.wheel_event(&wheel(0.0, Phase::Began, Phase::None, clock));
    host.wheel_event(&wheel(40.0, Phase::Changed, Phase::None, clock + 0.01));
    let after_finger = host.text_surface(FIELD).unwrap().scroll().offset.y;
    assert!(
        after_finger >= 39.9,
        "downward finger scroll was stiffened/stuck at {after_finger}"
    );
    host.wheel_event(&wheel(0.0, Phase::Ended, Phase::None, clock + 0.02));
    host.wheel_event(&wheel(60.0, Phase::None, Phase::Changed, clock + 0.03));
    host.wheel_event(&wheel(0.0, Phase::None, Phase::Ended, clock + 0.04));
    let after_momentum = host.text_surface(FIELD).unwrap().scroll().offset.y;
    assert!(
        after_momentum >= 99.0,
        "subsequent momentum was dropped: after_finger={after_finger}, after_momentum={after_momentum}"
    );
}

#[test]
fn reduced_motion_prevents_overscroll_and_rebound_frames() {
    use ContentWheelPhase as Phase;

    let mut host = ContentHost::new(Content::new(FIELD, read_only_text_field())).unwrap();
    let mut text = MotionText::new(Duration::ZERO, true);
    assert!(!render_with(&mut host, &mut text));

    host.wheel_event(&wheel(0.0, Phase::Began, Phase::None, 0.1));
    host.wheel_event(&wheel(-120.0, Phase::Changed, Phase::None, 0.11));
    host.wheel_event(&wheel(0.0, Phase::Ended, Phase::None, 0.12));

    let s = host.text_surface(FIELD).unwrap();
    assert_eq!(s.scroll().overscroll, Point::default());
    assert_eq!(s.scroll().visual_offset(), Point::default());
    assert!(!s.scroll().needs_animation());
    assert!(!render_with(&mut host, &mut text));
}

#[test]
fn rich_content_and_scrolled_owner_forward_motion_context_cleanly() {
    let rich = RichContent::new(
        "manual",
        1,
        RichDocument::from_markdown("# Title\n\nBody link [a](a.md)"),
    )
    .unwrap();
    let mut host = ContentHost::new(Content::new(
        ROOT,
        ScrollContent::new(ScrollArea::default(), Content::new(RICH, rich)),
    ))
    .unwrap();
    let mut text = MotionText::new(Duration::ZERO, false);
    assert!(!render_with(&mut host, &mut text));
    text.now = Duration::from_secs(1);
    assert!(!render_with(&mut host, &mut text));
    host.set_window_active(false);
    host.set_window_active(true);
    assert!(!render_with(&mut host, &mut text));
}
