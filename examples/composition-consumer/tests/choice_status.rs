use tekes_ui_composition_consumer::{
    choice_status::{CHOICE, FEEDBACK, Scenario},
    content_status::Status,
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::ContentEvent,
};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::ShapeOptions,
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
fn labels() -> Vec<String> {
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
        Rect::new(0., 0., 360., 300.),
        Rect::new(0., 0., 360., 300.),
        p,
    )
    .unwrap();
    scene
}
fn key(s: &mut Scenario, k: ControlKey) -> tekes_ui_controls::composition::ContentResponse {
    s.host.event(&ContentEvent::Key { key: k, down: true })
}
#[test]
fn select_background_refresh_retains_cached_choice_candidate_and_focus_and_uses_overlay_pass() {
    let mut s = Scenario::new(labels()).unwrap();
    let mut text = Text::new();
    let p = Theme::LIGHT.into();
    draw(&mut s, &mut text, p);
    assert!(s.host.focus(Some(CHOICE), true));
    key(&mut s, ControlKey::Enter);
    draw(&mut s, &mut text, p);
    assert!(s.open());
    s.set_status(Status::Loading);
    draw(&mut s, &mut text, p);
    assert!(s.open());
    assert_eq!(s.host.focused(), Some(CHOICE));
    key(&mut s, ControlKey::Down);
    draw(&mut s, &mut text, p);
    s.set_status(Status::Error);
    let scene = draw(&mut s, &mut text, p);
    assert!(s.open());
    assert_eq!(s.selected(), 0);
    assert_eq!(s.host.focused(), Some(CHOICE));
    let snapshot = s.host.semantics();
    let choice = snapshot.iter().find(|n| n.id == CHOICE).unwrap();
    let feedback = snapshot.iter().find(|n| n.id == FEEDBACK).unwrap();
    assert!(
        !choice.semantics.invalid,
        "async content failure is not field validation"
    );
    assert!(feedback.semantics.invalid);
    assert_eq!(choice.parts.iter().position(|p| p.focused), Some(1));
    let popup = s.popup_frame().unwrap();
    let banner = scene
        .primitives()
        .iter()
        .position(|p| matches!(p,Primitive::Quad(q) if q.rect==feedback.frame))
        .unwrap();
    let menu = scene
        .primitives()
        .iter()
        .position(|p| matches!(p,Primitive::Quad(q) if q.rect==popup))
        .unwrap();
    assert!(
        banner < menu,
        "public ContentHost paints menu over caller feedback"
    );
    assert!(key(&mut s, ControlKey::Enter).activated);
    draw(&mut s, &mut text, p);
    assert_eq!(s.selected(), 1);
    assert!(!s.open());
    s.set_status(Status::Ready);
    draw(&mut s, &mut text, p);
    assert_eq!(s.selected(), 1);
    assert_eq!(s.host.focused(), Some(CHOICE));
    s.set_options(Vec::new());
    s.set_status(Status::Loading);
    draw(&mut s, &mut text, p);
    assert!(!s.open());
    assert_eq!(s.host.focused(), None);
    assert!(
        s.host
            .semantics()
            .iter()
            .find(|n| n.id == CHOICE)
            .unwrap()
            .semantics
            .value
            .is_none()
    );
    assert!(s.set_options(vec![
        "Comfortable".into(),
        "Automatic".into(),
        "Compact".into()
    ]));
    s.set_status(Status::Ready);
    draw(&mut s, &mut text, p);
    assert_eq!(
        s.selected(),
        2,
        "same domain choice survives option reorder"
    );
    assert_eq!(s.host.focused(), None, "data arrival does not steal focus");
    assert!(!s.set_options(vec!["unrelated option".into()]));
    assert_eq!(s.selected(), 2, "rejected response preserves cached choice");
    assert_eq!(
        s.host
            .semantics()
            .iter()
            .find(|n| n.id == CHOICE)
            .unwrap()
            .semantics
            .value
            .as_deref(),
        Some("Compact")
    );
    s.host.focus(Some(CHOICE), true);
    key(&mut s, ControlKey::Enter);
    draw(&mut s, &mut text, p);
    let old_row = s
        .host
        .semantics()
        .iter()
        .find(|n| n.id == CHOICE)
        .unwrap()
        .parts[0]
        .frame
        .center();
    s.host.event(&ContentEvent::PointerDown(old_row));
    assert!(s.set_options(labels()));
    assert!(
        !s.open(),
        "provider data replacement cancels stale popup capture"
    );
    assert!(!s.host.event(&ContentEvent::PointerUp(old_row)).activated);
    assert_eq!(
        s.selected(),
        1,
        "retained Compact value maps to its new position"
    );
    assert_eq!(s.host.focused(), Some(CHOICE));
}
#[test]
fn select_async_states_observe_real_public_composition_tokens_in_four_themes_and_three_brands() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for status in [Status::Loading, Status::Error] {
                let mut s = Scenario::new(labels()).unwrap();
                let mut text = Text::new();
                let p = Palette::new(theme, brand).unwrap();
                draw(&mut s, &mut text, p);
                s.host.focus(Some(CHOICE), true);
                key(&mut s, ControlKey::Enter);
                draw(&mut s, &mut text, p);
                key(&mut s, ControlKey::Down);
                assert!(key(&mut s, ControlKey::Enter).activated);
                s.set_status(status);
                text.labels.clear();
                #[cfg(feature = "token-audit")]
                let (scene, reads) = tekes_ui_tokens::audit::capture(|| draw(&mut s, &mut text, p));
                #[cfg(not(feature = "token-audit"))]
                let (scene, reads) = (draw(&mut s, &mut text, p), Vec::<(String, String)>::new());
                assert_eq!(s.selected(), 1);
                assert_eq!(s.host.focused(), Some(CHOICE));
                assert!(!s.open());
                assert!(text.labels.iter().any(|s| s == status.label()));
                let node = s
                    .host
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == CHOICE)
                    .unwrap();
                assert_eq!(node.semantics.value.as_deref(), Some("Compact"));
                assert!(!node.semantics.invalid);
                assert!(!node.semantics.disabled);
                assert!(scene.primitives().iter().any(|v| matches!(v,Primitive::Quad(q) if q.rect==node.frame && q.fill==p.color("action-primary-ground").unwrap())));
                cases.push(serde_json::json!({"component":"tui.select","choice_state":if status==Status::Loading {"loading"}else{"error"},"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 24);
    if let Some(path) = std::env::var_os("TEKESUI_SELECT_ASYNC_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("token-audit feature required");
        }
        #[cfg(feature="token-audit")] std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
