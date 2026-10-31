//! Cross-executable legacy layout/Scene probe; output is compared to frozen root.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, collections_view::ListView, extended::List,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text {
    draws: Vec<String>,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.len() as f32 * 8.).min(w),
            if s.is_empty() {
                0.
            } else {
                20. + 20. * s.matches('\n').count() as f32
            },
        )
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        r: proposal::TextRole,
        f: Rect,
        a: ButtonTextAlign,
        c: Rgba,
    ) {
        self.draws.push(format!("{s:?} {r:?} {f:?} {a:?} {c:?}"));
    }
}
#[test]
fn original_list_layout_scene_probe() {
    let mut records = Vec::new();
    for width in [0., 1., 10., 40., 90., 200., 500., 1000.] {
        for clip in [
            Rect::new(0., 0., 1200., 500.),
            Rect::new(0., 10., 1200., 15.),
            Rect::new(0., 500., 1200., 20.),
            Rect::new(0., 0., 0., 200.),
        ] {
            for theme in [Theme::LIGHT, Theme::DARK] {
                for state in 0..3 {
                    for labels in [
                        vec!["a".to_string(), "b".into(), "c".into()],
                        vec!["".to_string(), "long\nsecond\nthird".into(), "tail".into()],
                    ] {
                        let mut v = ListView::new();
                        let mut m = List::new(labels.len());
                        let mut text = Text::default();
                        let f = Rect::new(0., 0., width, 300.);
                        v.render(
                            &mut Scene::new(),
                            &mut text,
                            &m,
                            &labels,
                            f,
                            Some(clip),
                            theme.into(),
                        );
                        if state > 0 {
                            m.select(1);
                            v.focus(1, true);
                        }
                        if state == 2 {
                            v.set_disabled(true);
                        }
                        text.draws.clear();
                        let mut scene = Scene::new();
                        let l = v.render(
                            &mut scene,
                            &mut text,
                            &m,
                            &labels,
                            f,
                            Some(clip),
                            theme.into(),
                        );
                        records.push(format!(
                            "{:?}\n{l:?}\n{scene:?}\n{:?}",
                            ListView::measure(&mut text, &labels, width),
                            text.draws
                        ));
                    }
                }
            }
        }
    }
    assert_eq!(records.len(), 384);
    if let Ok(path) = std::env::var("ISSUE95_PROBE_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
    }
}
