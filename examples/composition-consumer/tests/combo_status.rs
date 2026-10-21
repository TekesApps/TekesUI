use std::rc::Rc;
use tekes_ui_composition_consumer::{
    combo_status::{FEEDBACK, QUERY, Scenario},
    content_status::Status,
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::{AccessibilityAction, ContentEvent, ContentResponse},
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
    ["Automatic", "Compact", "Comfortable"]
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
#[test]
fn combo_refresh_preserves_real_editor_ime_selection_focus_and_candidate_contract() {
    let mut s = Scenario::new(items()).unwrap();
    let mut t = Text::new();
    let p = Theme::LIGHT.into();
    draw(&mut s, &mut t, p);
    s.host.focus(Some(QUERY), true);
    commit(&mut s, "Com", None);
    draw(&mut s, &mut t, p);
    assert_eq!(s.active(), Some(1));
    s.set_status(Status::Loading);
    draw(&mut s, &mut t, p);
    key(&mut s, ControlKey::Down);
    assert_eq!(s.active(), Some(2));
    s.set_status(Status::Error);
    let scene = draw(&mut s, &mut t, p);
    assert_eq!(s.active(), Some(2));
    assert_eq!(s.host.focused(), Some(QUERY));
    assert_eq!(s.query(), "Com");
    let snapshot = s.host.semantics();
    let field = snapshot.iter().find(|n| n.id == QUERY).unwrap();
    let feedback = snapshot.iter().find(|n| n.id == FEEDBACK).unwrap();
    assert!(!field.semantics.invalid);
    assert!(feedback.semantics.invalid);
    let popup = s.popup_frame().unwrap();
    let base = scene
        .primitives()
        .iter()
        .position(|p| matches!(p,Primitive::Quad(q) if q.rect==feedback.frame))
        .unwrap();
    let overlay = scene
        .primitives()
        .iter()
        .position(|p| matches!(p,Primitive::Quad(q) if q.rect==popup))
        .unwrap();
    assert!(base < overlay);
    s.host.event(&ContentEvent::Preedit {
        text: "候補".into(),
        selection: 1..1,
        replacement: Some(0..3),
        clauses: std::iter::once(0..2).collect(),
    });
    let input = &s.host.focused_text().unwrap().input;
    let before = (input.text.clone(), input.marked.clone(), input.selection());
    s.set_status(Status::Loading);
    draw(&mut s, &mut t, p);
    s.set_status(Status::Error);
    draw(&mut s, &mut t, p);
    assert_eq!(s.host.focused_text().unwrap().input.text, before.0);
    assert_eq!(s.host.focused_text().unwrap().input.marked, before.1);
    assert_eq!(s.host.focused_text().unwrap().input.selection(), before.2);
    assert!(!s.set_items(vec!["new response".into()]));
    assert!(!key(&mut s, ControlKey::Enter).consumed);
    assert!(s.open());
    assert_eq!(s.selected(), None);
    assert!(!key(&mut s, ControlKey::Down).consumed);
    assert_eq!(s.host.focused_text().unwrap().input.marked, before.1);
    assert!(key(&mut s, ControlKey::Escape).consumed);
    assert!(s.host.focused_text().unwrap().input.marked.is_none());
    assert_eq!(s.query(), "Com");
    assert!(s.open(), "first Escape cancels preedit, not the ComboBox");
    draw(&mut s, &mut t, p);
    assert!(key(&mut s, ControlKey::Enter).activated);
    assert_eq!(s.query(), "Compact");
    assert_eq!(s.selected(), Some("Compact"));
    assert!(
        s.host
            .accessibility_action(QUERY, None, AccessibilityAction::SetSelection(1..3))
            .consumed
    );
    let selected_range = s.host.focused_text().unwrap().input.selection();
    s.set_status(Status::Ready);
    draw(&mut s, &mut t, p);
    assert_eq!(
        s.host.focused_text().unwrap().input.selection(),
        selected_range
    );
    assert!(s.set_items(vec![
        "Comfortable".into(),
        "Automatic".into(),
        "Compact".into()
    ]));
    assert_eq!(s.selected(), Some("Compact"));
    assert_eq!(s.query(), "Compact");
    assert_eq!(
        s.host.focused_text().unwrap().input.selection(),
        selected_range
    );
    commit(&mut s, "Com", Some(0..7));
    draw(&mut s, &mut t, p);
    let row = s
        .host
        .semantics()
        .iter()
        .find(|n| n.id == QUERY)
        .unwrap()
        .parts[0]
        .frame
        .center();
    s.host.event(&ContentEvent::PointerDown(row));
    assert!(s.set_items(items()));
    assert!(!s.host.event(&ContentEvent::PointerUp(row)).activated);
    assert_eq!(s.selected(), Some("Compact"));
    draw(&mut s, &mut t, p);
    let compact_key = s
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == QUERY)
        .unwrap()
        .parts
        .into_iter()
        .find(|p| p.semantics.name == "Compact")
        .unwrap()
        .index;
    assert_eq!(
        compact_key, 1,
        "semantic actions use original source indices, not filtered row zero"
    );
    assert!(
        s.host
            .accessibility_action(QUERY, Some(compact_key), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(s.selected(), Some("Compact"));
}
#[test]
fn combo_async_and_validation_states_use_original_field_and_actual_tokens() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for case in ["loading", "failure", "validation"] {
                let mut s = Scenario::new(items()).unwrap();
                let mut t = Text::new();
                let p = Palette::new(theme, brand).unwrap();
                draw(&mut s, &mut t, p);
                s.host.focus(Some(QUERY), true);
                commit(&mut s, "Com", None);
                draw(&mut s, &mut t, p);
                assert!(key(&mut s, ControlKey::Enter).activated);
                s.set_status(match case {
                    "loading" => Status::Loading,
                    "failure" => Status::Error,
                    _ => Status::Ready,
                });
                if case == "validation" {
                    s.set_validation_error(Some("Choose a permitted layout.".into()));
                }
                t.labels.clear();
                #[cfg(feature = "token-audit")]
                let (scene, reads) = tekes_ui_tokens::audit::capture(|| draw(&mut s, &mut t, p));
                #[cfg(not(feature = "token-audit"))]
                let (scene, reads) = (draw(&mut s, &mut t, p), Vec::<(String, String)>::new());
                assert_eq!(s.query(), "Compact");
                assert_eq!(s.selected(), Some("Compact"));
                assert_eq!(s.host.focused(), Some(QUERY));
                let node = s
                    .host
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == QUERY)
                    .unwrap();
                assert_eq!(node.semantics.invalid, case == "validation");
                let q = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect == node.frame => Some(q),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(q.fill, p.color("transcript-input").unwrap());
                let boundary = proposal::control_boundary(
                    if case == "validation" {
                        "error"
                    } else {
                        "required-input"
                    },
                    theme,
                )
                .unwrap();
                assert_eq!(q.border_color, p.color(boundary).unwrap());
                if case != "validation" {
                    assert!(t.labels.iter().any(|s| s == s_from_status(case)));
                }
                cases.push(serde_json::json!({"component":"tui.combo-box","choice_state":if case=="loading" {"loading"}else{"error"},"context":case,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 36);
    if let Some(path) = std::env::var_os("TEKESUI_COMBO_ASYNC_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("token-audit feature required");
        }
        #[cfg(feature="token-audit")] std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
fn s_from_status(case: &str) -> &str {
    if case == "loading" {
        Status::Loading.label()
    } else {
        Status::Error.label()
    }
}

#[test]
fn query_selection_drag_releases_before_candidate_overlay_can_route_pointer() {
    let mut s = Scenario::new(items()).unwrap();
    let mut t = Text::new();
    let p = Theme::LIGHT.into();
    draw(&mut s, &mut t, p);
    let point = s.host.get(QUERY).unwrap().frame().center();
    assert!(
        s.host
            .event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    draw(&mut s, &mut t, p);
    assert!(s.open());
    assert!(s.host.focused_text().unwrap().pointer_state(None).pressed);
    let candidate = s
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == QUERY)
        .unwrap()
        .parts[0]
        .frame
        .center();
    s.host.event(&ContentEvent::PointerMove(candidate));
    assert!(!s.host.event(&ContentEvent::PointerUp(candidate)).activated);
    assert!(!s.host.focused_text().unwrap().pointer_state(None).pressed);
    assert_eq!(s.selected(), None);
    assert!(s.open());
    draw(&mut s, &mut t, p);
    s.host.event(&ContentEvent::PointerDown(candidate));
    assert!(s.host.event(&ContentEvent::PointerUp(candidate)).activated);
    assert_eq!(s.selected(), Some("Automatic"));
}
