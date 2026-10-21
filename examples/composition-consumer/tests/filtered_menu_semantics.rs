use std::time::Duration;
use tekes_ui_controls::extended::{ComboBox, CommandPalette};
use tekes_ui_controls::layout_view::{ComboBoxView, CommandPaletteView};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlMotionContext};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text {
    millis: u64,
    reduced: bool,
}
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: Duration::from_millis(self.millis),
            reduced_motion: self.reduced,
        })
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        r: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        scene.push_quad(Quad::new(r, c));
    }
}
fn paint(view: &mut ComboBoxView, model: &mut ComboBox) {
    view.render(
        &mut Scene::new(),
        &mut Text::default(),
        model,
        Rect::new(10., 10., 160., 36.),
        Rect::new(0., 0., 600., 500.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
}
fn model() -> ComboBox {
    let mut model = ComboBox {
        items: ["Alpha", "Beta", "Bravo", "Gamma"]
            .map(str::to_owned)
            .to_vec(),
        ..Default::default()
    };
    model.set_query("B");
    model
}
#[test]
fn filtered_semantics_accept_source_indices_and_share_commit_path() {
    let mut model = model();
    let mut view = ComboBoxView::default();
    assert!(!view.focus_option(2, &mut model));
    paint(&mut view, &mut model);
    assert_eq!(view.options.original_index(0), Some(1));
    assert_eq!(view.options.original_index(1), Some(2));
    assert!(!view.focus_option(0, &mut model));
    assert!(view.focus_option(2, &mut model));
    assert_eq!(model.active, Some(2));
    assert_eq!(model.selected, None);
    assert_eq!(view.activate_option(2, &mut model), Some(2));
    assert_eq!(model.selected, Some(2));
    assert_eq!(model.query, "Bravo");
    assert!(!model.open);
    assert_eq!(view.activate_option(2, &mut model), None);
}
#[test]
fn stale_query_labels_disabled_and_empty_candidates_cannot_retarget_actions() {
    let mut model = model();
    let mut view = ComboBoxView::default();
    paint(&mut view, &mut model);
    model.set_query("Gamma");
    assert!(!view.options.is_current(&model));
    assert_eq!(view.activate_option(1, &mut model), None);
    assert_eq!(model.selected, None);
    paint(&mut view, &mut model);
    model.items[3] = "Gamma changed".into();
    assert!(!view.focus_option(3, &mut model));
    paint(&mut view, &mut model);
    model.disabled = true;
    assert_eq!(view.activate_option(3, &mut model), None);
    model.disabled = false;
    model.set_query("absent");
    paint(&mut view, &mut model);
    assert!(!view.options.is_current(&model));
    assert_eq!(view.activate_option(0, &mut model), None);
}
#[test]
fn command_palette_uses_identical_source_index_contract_and_window_guard() {
    let mut model = CommandPalette { combo: model() };
    let mut view = CommandPaletteView::default();
    view.render(
        &mut Scene::new(),
        &mut Text::default(),
        &mut model,
        Rect::new(10., 10., 160., 36.),
        Rect::new(0., 0., 600., 500.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    assert!(view.focus_option(2, &mut model));
    assert_eq!(model.combo.selected, None);
    assert_eq!(view.activate_option(2, &mut model), Some(2));
    model.combo.set_query("B");
    view.render(
        &mut Scene::new(),
        &mut Text::default(),
        &mut model,
        Rect::new(10., 10., 160., 36.),
        Rect::new(0., 0., 600., 500.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    view.set_window_active(false, &mut model);
    assert!(!view.focus_option(1, &mut model));
    assert_eq!(view.activate_option(1, &mut model), None);
}
