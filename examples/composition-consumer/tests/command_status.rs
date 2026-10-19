use std::rc::Rc;
use tekes_ui_composition_consumer::{
    command_status::{COMMAND, FEEDBACK, Scenario},
    content_status::Status,
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::{AccessibilityAction, ContentEvent, ContentResponse, ContentRole},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Text {
    shaper: CoreTextShaper,
    labels: Vec<String>,
}
impl Text {
    fn new() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            labels: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        let p = self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: Some(w.into()),
                line_spacing: 0.,
            },
        );
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        self.labels.push(s.into());
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
fn items() -> Vec<String> {
    ["Open file", "Open settings", "Close window"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}
fn draw(s: &mut Scenario, t: &mut Text, p: Palette) -> Scene {
    let mut scene = Scene::new();
    s.render(
        &mut scene,
        t,
        Rect::new(0., 0., 360., 400.),
        Rect::new(0., 0., 360., 400.),
        p,
    )
    .unwrap();
    scene
}
fn key(s: &mut Scenario, k: ControlKey) -> ContentResponse {
    s.host.event(&ContentEvent::Key { key: k, down: true })
}
fn commit(
    s: &mut Scenario,
    text: &str,
    replacement: Option<std::ops::Range<usize>>,
) -> ContentResponse {
    s.host.event(&ContentEvent::Commit {
        text: text.into(),
        replacement,
    })
}
fn open(s: &mut Scenario, t: &mut Text, p: Palette) {
    draw(s, t, p);
    s.host.focus(Some(COMMAND), true);
    assert!(key(s, ControlKey::Enter).consumed);
    assert!(s.open());
    draw(s, t, p);
}
#[test]
fn retained_command_refresh_keeps_query_ime_selection_and_restores_real_trigger() {
    let mut s = Scenario::new(items()).unwrap();
    let mut t = Text::new();
    let p = Theme::LIGHT.into();
    open(&mut s, &mut t, p);
    commit(&mut s, "Open", None);
    draw(&mut s, &mut t, p);
    key(&mut s, ControlKey::Down);
    assert_eq!(s.active(), Some(1));
    s.host
        .accessibility_action(COMMAND, None, AccessibilityAction::SetSelection(1..3));
    let range = s.host.focused_text().unwrap().input.selection();
    for status in [Status::Loading, Status::Error, Status::Ready] {
        s.set_status(status);
        draw(&mut s, &mut t, p);
        assert_eq!(s.query(), "Open");
        assert_eq!(s.active(), Some(1));
        assert_eq!(s.host.focused_text().unwrap().input.selection(), range);
        assert_eq!(s.host.focused(), Some(COMMAND));
        assert_eq!(s.last_executed(), None);
    }
    s.host.event(&ContentEvent::Preedit {
        text: "開く".into(),
        selection: 1..1,
        replacement: Some(0..4),
        clauses: std::iter::once(0..2).collect(),
    });
    let snapshot = {
        let i = &s.host.focused_text().unwrap().input;
        (i.text.clone(), i.marked.clone(), i.selection())
    };
    s.set_status(Status::Loading);
    draw(&mut s, &mut t, p);
    s.set_status(Status::Error);
    draw(&mut s, &mut t, p);
    assert_eq!(s.host.focused_text().unwrap().input.text, snapshot.0);
    assert_eq!(s.host.focused_text().unwrap().input.marked, snapshot.1);
    assert_eq!(s.host.focused_text().unwrap().input.selection(), snapshot.2);
    assert!(!s.set_items(vec!["new response".into()]));
    assert!(!key(&mut s, ControlKey::Enter).consumed);
    assert!(!key(&mut s, ControlKey::Down).consumed);
    assert!(key(&mut s, ControlKey::Escape).consumed);
    assert!(s.open());
    assert_eq!(s.query(), "Open");
    draw(&mut s, &mut t, p);
    assert!(key(&mut s, ControlKey::Enter).activated);
    assert_eq!(s.last_executed(), Some("Open file"));
    assert!(!s.open());
    assert!(s.host.focused_text().is_none());
    draw(&mut s, &mut t, p);
    let node = s
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == COMMAND)
        .unwrap();
    assert!(node.focused);
    assert_eq!(node.semantics.role, ContentRole::Button);
    assert_eq!(node.semantics.expanded, Some(false));
    assert!(key(&mut s, ControlKey::Space).consumed);
    assert!(!s.open());
    assert!(
        s.host
            .event(&ContentEvent::Key {
                key: ControlKey::Space,
                down: false
            })
            .changed
    );
    assert!(s.open());
    assert_eq!(s.query(), "");
    draw(&mut s, &mut t, p);
    assert!(key(&mut s, ControlKey::Enter).activated);
    assert_eq!(s.last_executed(), Some("Open file"));
    open(&mut s, &mut t, p);
    assert!(key(&mut s, ControlKey::Escape).consumed);
    assert!(!s.open());
    assert_eq!(s.host.focused(), Some(COMMAND));
}
#[test]
fn provider_replacement_cancels_stale_candidate_and_selection_drag_releases() {
    let mut s = Scenario::new(Vec::new()).unwrap();
    let mut t = Text::new();
    let p = Theme::LIGHT.into();
    s.set_status(Status::Loading);
    open(&mut s, &mut t, p);
    assert_eq!(s.active(), None);
    assert!(!key(&mut s, ControlKey::Enter).activated);
    assert!(s.open());
    assert!(s.set_items(items()));
    draw(&mut s, &mut t, p);
    assert_eq!(s.host.focused(), Some(COMMAND));
    let point = s.query_frame().unwrap().center();
    assert!(
        s.host
            .event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    draw(&mut s, &mut t, p);
    let row = s
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == COMMAND)
        .unwrap()
        .parts[0]
        .frame
        .center();
    s.host.event(&ContentEvent::PointerMove(row));
    assert!(!s.host.event(&ContentEvent::PointerUp(row)).activated);
    assert!(!s.host.focused_text().unwrap().pointer_state(None).pressed);
    s.host.event(&ContentEvent::PointerDown(row));
    assert!(s.set_items(vec![
        "Open settings".into(),
        "Open file".into(),
        "Close window".into()
    ]));
    assert!(!s.host.event(&ContentEvent::PointerUp(row)).activated);
    assert_eq!(s.last_executed(), None);
    draw(&mut s, &mut t, p);
    assert!(
        s.host
            .accessibility_action(COMMAND, Some(0), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(s.last_executed(), Some("Open settings"));
    assert!(s.set_items(vec!["Close window".into()]));
    assert_eq!(
        s.last_executed(),
        Some("Open settings"),
        "execution history is caller data, not live candidate index"
    );
    assert!(!s.set_items(vec!["Duplicate".into(), "Duplicate".into()]));
    let trigger = s.trigger_frame().unwrap().center();
    s.host.event(&ContentEvent::PointerDown(trigger));
    assert!(s.host.event(&ContentEvent::PointerUp(trigger)).changed);
    assert!(s.open());
    draw(&mut s, &mut t, p);
    s.host.event(&ContentEvent::Cancel);
    assert!(!s.open());
}
#[test]
fn command_loading_and_failure_keep_actual_query_tokens_for_cached_and_empty_data() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for cached in [false, true] {
                for status in [Status::Loading, Status::Error] {
                    let mut s = Scenario::new(if cached { items() } else { Vec::new() }).unwrap();
                    let mut t = Text::new();
                    let p = Palette::new(theme, brand).unwrap();
                    open(&mut s, &mut t, p);
                    commit(&mut s, "Open", None);
                    draw(&mut s, &mut t, p);
                    s.set_status(status);
                    t.labels.clear();
                    #[cfg(feature = "token-audit")]
                    let (scene, reads) =
                        tekes_ui_tokens::audit::capture(|| draw(&mut s, &mut t, p));
                    #[cfg(not(feature = "token-audit"))]
                    let (scene, reads) = (draw(&mut s, &mut t, p), Vec::<(String, String)>::new());
                    assert_eq!(s.query(), "Open");
                    assert_eq!(s.host.focused(), Some(COMMAND));
                    assert!(s.open());
                    let node = s
                        .host
                        .semantics()
                        .into_iter()
                        .find(|n| n.id == COMMAND)
                        .unwrap();
                    assert!(!node.semantics.invalid);
                    let q = scene
                        .primitives()
                        .iter()
                        .find_map(|p| match p {
                            Primitive::Quad(q) if q.rect == node.frame => Some(q),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(q.fill, p.color("transcript-input").unwrap());
                    assert_eq!(
                        q.border_color,
                        p.color(proposal::control_boundary("required-input", theme).unwrap())
                            .unwrap()
                    );
                    assert!(t.labels.iter().any(|s| s == status.label()));
                    let feedback = s
                        .host
                        .semantics()
                        .into_iter()
                        .find(|n| n.id == FEEDBACK)
                        .unwrap();
                    assert_eq!(feedback.semantics.invalid, status == Status::Error);
                    cases.push(serde_json::json!({"component":"tui.command-palette","choice_state":if status==Status::Loading{"loading"}else{"error"},"cached":cached,"theme":format!("{theme:?}"),"brand":brand,"state_asserted":true,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 48);
    if let Some(path) = std::env::var_os("TEKESUI_COMMAND_ASYNC_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("token-audit required");
        }
        #[cfg(feature="token-audit")]std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
