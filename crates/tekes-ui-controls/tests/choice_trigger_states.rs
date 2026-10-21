//! Public trigger states, production paint and bounded token observations.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, extended::CommandPalette, layout_view::CommandPaletteView,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Labels;
impl ButtonTextRenderer for Labels {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * 8.).min(w),
            r.style().line_height,
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
#[test]
fn command_trigger_states_use_production_roles_without_changing_query_or_geometry() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for scenario in [
                "rest",
                "hover",
                "pressed",
                "focus",
                "disabled-hover",
                "drag-outside",
                "returned-rest",
            ] {
                let frame = Rect::new(20., 20., 220., 44.);
                let inside = frame.center();
                let outside = Point::new(300., 100.);
                let mut view = CommandPaletteView::default();
                let mut model = CommandPalette::default();
                model.combo.query = "preserved".into();
                let palette = Palette { theme, brand };
                let mut text = Labels;
                view.render_trigger(
                    &mut Scene::new(),
                    &mut text,
                    &mut model,
                    "Commands",
                    frame,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                );
                match scenario {
                    "hover" => view.pointer_move(inside, &mut model),
                    "pressed" => {
                        view.pointer_move(inside, &mut model);
                        assert!(view.pointer_down_trigger(inside));
                    }
                    "focus" => view.focus_trigger(true),
                    "disabled-hover" => {
                        model.combo.disabled = true;
                        view.pointer_move(inside, &mut model);
                    }
                    "drag-outside" => {
                        assert!(view.pointer_down_trigger(inside));
                        view.pointer_move(outside, &mut model);
                    }
                    "returned-rest" => {
                        view.pointer_move(inside, &mut model);
                        view.pointer_move(outside, &mut model);
                    }
                    _ => {}
                }
                let render = || {
                    let mut scene = Scene::new();
                    let layout = view.render_trigger(
                        &mut scene,
                        &mut text,
                        &mut model,
                        "Commands",
                        frame,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    (scene, layout)
                };
                #[cfg(feature = "token-audit")]
                let ((scene, layout), reads) = tekes_ui_tokens::audit::capture(render);
                #[cfg(not(feature = "token-audit"))]
                let ((scene, layout), reads) = {
                    let mut render = render;
                    (render(), Vec::<(String, String)>::new())
                };
                // Expected-role reads are outside the production observation window.
                let expected = match scenario {
                    "hover" => "action-primary-hover",
                    "pressed" => "action-primary-pressed",
                    "disabled-hover" => "surface-2",
                    _ => "action-primary-ground",
                };
                let quads: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| {
                        if let Primitive::Quad(q) = p {
                            Some(q)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(
                    quads[0].fill,
                    palette.color(expected).unwrap(),
                    "{theme:?}/{brand}/{scenario}"
                );
                assert_eq!(quads.len(), if scenario == "focus" { 2 } else { 1 });
                assert_eq!(layout.frame, frame);
                assert!(!model.combo.open);
                assert_eq!(model.combo.query, "preserved");
                if scenario == "drag-outside" {
                    assert!(!view.pointer_up_trigger(outside, &mut model));
                    assert!(!model.combo.open);
                }
                if scenario == "disabled-hover" {
                    assert!(!view.pointer_down_trigger(inside));
                }
                let state = match scenario {
                    "disabled-hover" => "disabled",
                    "drag-outside" | "returned-rest" => "rest",
                    other => other,
                };
                cases.push(serde_json::json!({"component":"tui.command-palette","part":"trigger","action_state":state,"scenario":scenario,"theme":format!("{theme:?}"),"brand":brand,"state_asserted":true,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 84);
    if let Some(path) = std::env::var_os("TEKESUI_CHOICE_TRIGGER_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("explicit token-audit feature required");
        }
        #[cfg(feature="token-audit")] std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
