//! Exact selection/query states through original Views and the existing TextSurface.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurface, TextSurfaceHost,
    TextSurfaceTextRenderer,
    extended::{ComboBox, CommandPalette},
    layout_view::CommandPaletteView,
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
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        let p = self.shape_surface(s, r, Some(w));
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
fn observe<T>(render: impl FnOnce() -> T) -> (T, Vec<(String, String)>) {
    #[cfg(feature = "token-audit")]
    {
        tekes_ui_tokens::audit::capture(render)
    }
    #[cfg(not(feature = "token-audit"))]
    {
        (render(), Vec::new())
    }
}
#[test]
fn command_results_use_the_production_query_and_menu_without_button_selected_paint() {
    let mut text = Text {
        shaper: CoreTextShaper::new(CoreTextFonts::new()),
        labels: Vec::new(),
    };
    let mut cases = Vec::new();
    let trigger = Rect::new(20., 20., 240., 44.);
    let input = Rect::new(20., 80., 320., 44.);
    let viewport = Rect::new(0., 0., 400., 500.);
    for theme in Theme::ALL {
        for brand in 0..3 {
            for state in ["selected", "open", "empty"] {
                let palette = Palette { theme, brand };
                let mut model = CommandPalette {
                    combo: ComboBox {
                        items: ["Open recent", "Open settings", "Create workspace"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                        ..Default::default()
                    },
                };
                let mut view = CommandPaletteView::default();
                view.options.set_reduced_motion(true);
                view.render_trigger(
                    &mut Scene::new(),
                    &mut text,
                    &mut model,
                    "Commands",
                    trigger,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                );
                view.focus_trigger(true);
                assert!(
                    view.trigger_key(ControlKey::Enter, true, &mut model)
                        .changed
                );
                assert!(model.combo.open);
                view.blur_trigger();
                if state == "selected" {
                    for execution in 0..2 {
                        if execution > 0 {
                            view.open(&mut model);
                        }
                        assert_eq!(view.key(ControlKey::Down, true, &mut model), None);
                        assert_eq!(
                            view.key(ControlKey::Enter, true, &mut model),
                            Some(1),
                            "each repeated command must reach its consumer"
                        );
                        assert_eq!(view.key(ControlKey::Enter, false, &mut model), None);
                        assert_eq!(model.combo.selected, Some(1));
                        assert_eq!(model.combo.query, "Open settings");
                        assert!(!model.combo.open);
                    }
                } else if state == "empty" {
                    model.combo.set_query("no such command");
                }
                let mut surface = TextSurface::new(&model.combo.query, false, false);
                surface.layout_with(input, None, 1., 1., |s, w| {
                    text.shape_surface(s, proposal::TextRole::UiBody, w)
                });
                let draft = surface.input.text.clone();
                text.labels.clear();
                let ((scene, popup), reads) = observe(|| {
                    let mut scene = Scene::new();
                    view.render_trigger(
                        &mut scene,
                        &mut text,
                        &mut model,
                        "Commands",
                        trigger,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    if model.combo.open {
                        surface.draw(&mut text, &mut scene, palette, true, false);
                    }
                    let popup = view.render(
                        &mut scene,
                        &mut text,
                        &mut model,
                        input,
                        viewport,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    (scene, popup)
                });
                assert_eq!(popup.is_some(), state == "open");
                if let Some(p) = popup {
                    assert_eq!(p.items.len(), 3);
                }
                assert_eq!(surface.input.text, draft);
                let trigger_quad = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect == trigger => Some(q),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(
                    trigger_quad.fill,
                    palette.color("action-primary-ground").unwrap(),
                    "command history is not Button.selected appearance"
                );
                if state == "empty" {
                    assert_eq!(view.key(ControlKey::Enter, true, &mut model), None);
                    assert!(model.combo.open);
                    assert_eq!(model.combo.selected, None);
                    assert_eq!(model.combo.query, draft);
                    assert!(model.combo.filtered().is_empty());
                }
                if state == "open" {
                    assert_eq!(model.combo.active, Some(0));
                    assert_eq!(model.combo.selected, None);
                }
                if state != "selected" {
                    let query_quad = scene
                        .primitives()
                        .iter()
                        .find_map(|p| match p {
                            Primitive::Quad(q) if q.rect == input => Some(q),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(query_quad.fill, palette.color("transcript-input").unwrap());
                    let boundary = proposal::control_boundary("required-input", theme).unwrap();
                    assert_eq!(query_quad.border_color, palette.color(boundary).unwrap());
                }
                cases.push(serde_json::json!({"component":"tui.command-palette","choice_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 36);
    if let Some(path) = std::env::var_os("TEKESUI_COMMAND_RESULTS_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("token-audit feature required");
        }
        #[cfg(feature="token-audit")] std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
