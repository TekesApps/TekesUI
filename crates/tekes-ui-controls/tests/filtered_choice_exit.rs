use std::time::Duration;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext,
    extended::{ComboBox, CommandPalette},
    layout_view::{ComboBoxView, CommandPaletteView},
};
use tekes_ui_scene::{Rect, Scene, Size};
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
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
fn frame() -> Rect {
    Rect::new(0., 0., 480., 360.)
}
fn model() -> ComboBox {
    ComboBox {
        items: vec![
            "Close".into(),
            "Open file".into(),
            "Other".into(),
            "Open settings".into(),
        ],
        query: "Open".into(),
        open: true,
        active: Some(3),
        ..Default::default()
    }
}
fn draw(view: &mut ComboBoxView, model: &mut ComboBox, text: &mut Text) -> Scene {
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        text,
        model,
        Rect::new(20., 20., 200., 36.),
        frame(),
        Some(frame()),
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    scene
}
#[test]
fn original_indices_and_exit_survive_all_normal_combo_close_paths() {
    for close in ["enter", "escape", "pointer", "ax"] {
        let mut view = ComboBoxView::default();
        let mut model = model();
        let mut text = Text::default();
        draw(&mut view, &mut model, &mut text);
        text.millis = 200;
        draw(&mut view, &mut model, &mut text);
        let point = view.options.layout().unwrap().items[1].frame.center();
        let selected = match close {
            "enter" => view.key(ControlKey::Enter, true, &mut model),
            "escape" => view.key(ControlKey::Escape, true, &mut model),
            "pointer" => {
                view.pointer_down(point);
                view.pointer_up(point, &mut model)
            }
            _ => view.activate_option(3, &mut model),
        };
        assert_eq!(selected, (close != "escape").then_some(3));
        assert!(!model.open);
        assert!(
            view.options.is_presented(),
            "normal {close} must retain presentation"
        );
        assert!(view.options.layout().is_none());
        assert!(!view.focus_option(1, &mut model));
        assert_eq!(view.activate_option(1, &mut model), None);
        assert!(
            view.options.is_presented(),
            "late inert actions must not erase exit paint"
        );
        let exit = draw(&mut view, &mut model, &mut text);
        assert!(
            !exit.is_empty(),
            "normal close may change query/filter but still rebuild outgoing rows"
        );
        assert!(view.options.layout().is_none());
        text.millis = 500;
        assert!(draw(&mut view, &mut model, &mut text).is_empty());
        assert!(!view.options.is_presented());
    }
}
#[test]
fn command_normal_close_retains_exit_but_window_deactivation_clears_it() {
    for close in ["enter", "escape", "pointer", "ax"] {
        let mut view = CommandPaletteView::default();
        let mut model = CommandPalette { combo: model() };
        let mut text = Text::default();
        let paint = |v: &mut CommandPaletteView, m: &mut CommandPalette, t: &mut Text| {
            let mut scene = Scene::new();
            v.render(
                &mut scene,
                t,
                m,
                Rect::new(20., 20., 200., 36.),
                frame(),
                Some(frame()),
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            );
            scene
        };
        paint(&mut view, &mut model, &mut text);
        text.millis = 200;
        paint(&mut view, &mut model, &mut text);
        let point = view.options.layout().unwrap().items[1].frame.center();
        let selected = match close {
            "enter" => view.key(ControlKey::Enter, true, &mut model),
            "escape" => view.key(ControlKey::Escape, true, &mut model),
            "pointer" => {
                view.pointer_down(point);
                view.pointer_up(point, &mut model)
            }
            _ => view.activate_option(3, &mut model),
        };
        assert_eq!(selected, (close != "escape").then_some(3));
        assert!(!model.combo.open);
        assert!(view.options.is_presented());
        assert!(!paint(&mut view, &mut model, &mut text).is_empty());
        view.set_window_active(false, &mut model);
        assert!(!view.options.is_presented());
        assert!(paint(&mut view, &mut model, &mut text).is_empty());
    }
}
#[test]
fn closed_source_replacement_discards_retained_rows_and_reduced_motion_settles() {
    for reduced in [false, true] {
        let mut view = ComboBoxView::default();
        let mut model = model();
        let mut text = Text {
            reduced,
            ..Default::default()
        };
        draw(&mut view, &mut model, &mut text);
        text.millis = 200;
        draw(&mut view, &mut model, &mut text);
        view.key(ControlKey::Enter, true, &mut model);
        if reduced {
            assert!(draw(&mut view, &mut model, &mut text).is_empty());
            assert!(!view.options.is_presented());
        } else {
            assert!(view.options.is_presented());
            model.items[3] = "Replacement".into();
            assert!(draw(&mut view, &mut model, &mut text).is_empty());
            assert!(!view.options.is_presented());
        }
    }
}
