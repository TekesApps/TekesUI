use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text(f32);
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn type_scale(&self) -> f32 {
        self.0
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size * self.0),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use std::time::Duration;
use tekes_ui_composition_consumer::reading_stream::*;
use tekes_ui_controls::ControlKey;
fn paint(host: &mut ContentHost, width: f32, scale: f32) {
    let frame = Rect::new(16., 16., width - 32., 608.);
    host.render(
        &mut Scene::new(),
        &mut Text(scale),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn activate(host: &mut ContentHost, id: ComponentId) {
    assert!(
        host.accessibility_action(id, None, AccessibilityAction::Press)
            .activated
    );
}
#[test]
fn real_reader_stream_pauses_inactive_and_keeps_history_draft_and_focus_through_reflow() {
    let (mut host, mut source) = build().unwrap();
    paint(&mut host, 720., 1.);
    activate(&mut host, LATEST);
    source.process_actions(&mut host, Duration::ZERO);
    paint(&mut host, 720., 1.);
    let surface = host.text_surface(TRANSCRIPT).unwrap();
    assert_eq!(surface.scroll.offset.y, surface.scroll.limit().y);
    let point = surface.inner.center();
    assert!(
        host.event(&ContentEvent::Scroll {
            position: point,
            delta: tekes_ui_scene::Point::new(0., -180.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    paint(&mut host, 720., 1.);
    let anchor = host
        .text_surface(TRANSCRIPT)
        .unwrap()
        .reflow_anchor
        .unwrap();
    let y = host
        .text_surface(TRANSCRIPT)
        .unwrap()
        .caret_doc(anchor.utf16)
        .min_y()
        - host.text_surface(TRANSCRIPT).unwrap().scroll.offset.y;
    activate(&mut host, STREAM);
    source.process_actions(&mut host, Duration::ZERO);
    assert!(host.focus(Some(COMPOSER), true));
    host.event(&ContentEvent::Commit {
        text: "newer retained 😀".into(),
        replacement: None,
    });
    let draft = host.text_surface(COMPOSER).unwrap().input.text.clone();
    let selection = host.text_surface(COMPOSER).unwrap().input.selection();
    assert!(!source.tick(&mut host, Duration::from_secs(2), false));
    assert_eq!(source.appended, 0);
    assert!(source.tick(&mut host, Duration::from_secs(3), true));
    assert_eq!(source.appended, 0);
    for (i, ms) in [3500, 3900, 4300, 4700].into_iter().enumerate() {
        source.tick(&mut host, Duration::from_millis(ms), true);
        let (width, scale) = if i % 2 == 0 { (480., 1.5) } else { (720., 1.) };
        paint(&mut host, width, scale);
        let surface = host.text_surface(TRANSCRIPT).unwrap();
        let after = surface.caret_doc(anchor.utf16).min_y() - surface.scroll.offset.y;
        assert!(
            (after - y).abs() < 1.,
            "native consumer reflow anchor drift: {after} vs {y}"
        );
        assert_eq!(source.appended, i + 1);
        assert_eq!(host.focused(), Some(COMPOSER));
        assert_eq!(host.text_surface(COMPOSER).unwrap().input.text, draft);
        assert_eq!(
            host.text_surface(COMPOSER).unwrap().input.selection(),
            selection
        );
    }
    assert_eq!(source.pending, 0);
    assert!(
        !host
            .get(TRANSCRIPT)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .reading
            .as_ref()
            .unwrap()
            .is_following_latest()
    );
    // An owned action drained later must not commandeer the now-active editor.
    activate(&mut host, LATEST);
    host.focus(Some(COMPOSER), true);
    source.process_actions(&mut host, Duration::from_secs(5));
    paint(&mut host, 480., 1.5);
    assert_eq!(host.focused(), Some(COMPOSER));
    let surface = host.text_surface(TRANSCRIPT).unwrap();
    assert_eq!(surface.scroll.offset.y, surface.scroll.limit().y);
}
#[test]
fn typed_actions_keep_multiple_arrivals_once_and_keyboard_latest_uses_actual_button() {
    let (mut host, mut source) = build().unwrap();
    paint(&mut host, 720., 1.);
    activate(&mut host, APPEND);
    activate(&mut host, APPEND);
    source.process_actions(&mut host, Duration::ZERO);
    assert_eq!(source.appended, 2);
    source.process_actions(&mut host, Duration::ZERO);
    assert_eq!(source.appended, 2);
    assert!(host.focus(Some(LATEST), true));
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true
        })
        .activated
    );
    assert!(
        !host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: false
            })
            .activated
    );
    source.process_actions(&mut host, Duration::ZERO);
    paint(&mut host, 720., 1.);
    assert_eq!(host.focused(), Some(LATEST));
    let f = host
        .get(TRANSCRIPT)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert!(f.reading.as_ref().unwrap().is_following_latest());
    assert_eq!(f.surface.scroll.offset.y, f.surface.scroll.limit().y);
    activate(&mut host, ZOOM);
    activate(&mut host, WIDTH);
    source.process_actions(&mut host, Duration::ZERO);
    assert!(source.large_text && source.narrow);
}
