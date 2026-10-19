//! Exact selection/query states through original Views and the existing TextSurface.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, Select, TextSurface, TextSurfaceHost,
    TextSurfaceTextRenderer,
    extended::ComboBox,
    layout_view::{ComboBoxView, SelectView},
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
fn existing_choice_states_preserve_values_and_use_actual_production_tokens() {
    let mut text = Text {
        shaper: CoreTextShaper::new(CoreTextFonts::new()),
        labels: Vec::new(),
    };
    let mut cases = Vec::new();
    let frame = Rect::new(20., 20., 240., 44.);
    let viewport = Rect::new(0., 0., 400., 500.);
    let all_labels = ["Automatic", "Compact", "Comfortable"];
    for theme in Theme::ALL {
        for brand in 0..3 {
            for component in ["tui.select", "tui.combo-box"] {
                for state in [
                    "rest", "hover", "pressed", "focus", "disabled", "selected", "open", "empty",
                    "error",
                ] {
                    if component == "tui.combo-box" && state == "error" {
                        continue;
                    }
                    let labels: &[&str] = if component == "tui.select" && state == "empty" {
                        &[]
                    } else {
                        &all_labels
                    };
                    let palette = Palette { theme, brand };
                    let expected;
                    let ((scene, pointer), reads) = if component == "tui.select" {
                        let mut model = Select::new(labels.len());
                        let mut view = SelectView::default();
                        view.set_reduced_motion(true);
                        if state == "selected" {
                            assert!(model.choose(1));
                        }
                        view.set_disabled(state == "disabled", &mut model);
                        view.set_error(state == "error");
                        view.render(
                            &mut Scene::new(),
                            &mut text,
                            &mut model,
                            labels,
                            frame,
                            viewport,
                            None,
                            palette,
                            proposal::TextRole::UiBody,
                            false,
                        );
                        match state {
                            "hover" => view.pointer_move(frame.center(), &mut model),
                            "pressed" => {
                                assert!(view.pointer_down(frame.center(), &mut model));
                            }
                            "focus" => view.focus(true),
                            "open" => {
                                view.focus(true);
                                view.key(ControlKey::Enter, true, &mut model);
                                assert!(model.menu.open);
                            }
                            _ => {}
                        }
                        text.labels.clear();
                        let value = model.selected;
                        let result = observe(|| {
                            let mut scene = Scene::new();
                            let layout = view.render(
                                &mut scene,
                                &mut text,
                                &mut model,
                                labels,
                                frame,
                                viewport,
                                None,
                                palette,
                                proposal::TextRole::UiBody,
                                false,
                            );
                            assert_eq!(layout.trigger.frame, frame);
                            assert_eq!(layout.popup.is_some(), state == "open");
                            if let Some(popup) = layout.popup {
                                assert_eq!(popup.items.len(), labels.len());
                            }
                            (scene, tekes_ui_controls::ControlState::default())
                        });
                        assert_eq!(model.selected, value);
                        assert_eq!(
                            text.labels.first().map(String::as_str).unwrap_or(""),
                            labels.get(value).copied().unwrap_or("")
                        );
                        if state == "empty" {
                            assert!(!view.pointer_down(frame.center(), &mut model));
                            view.focus(true);
                            view.key(ControlKey::Enter, true, &mut model);
                            assert!(!model.menu.open);
                            assert!(!model.choose(0));
                        }
                        if state == "disabled" {
                            assert!(!view.pointer_down(frame.center(), &mut model));
                            view.key(ControlKey::Enter, true, &mut model);
                            assert!(!model.menu.open);
                        }
                        if state == "pressed" {
                            view.pointer_up(frame.center(), &mut model);
                            assert!(model.menu.open);
                            assert_eq!(model.selected, 0);
                        }
                        if state == "open" {
                            view.key(ControlKey::Down, true, &mut model);
                            view.key(ControlKey::Enter, true, &mut model);
                            assert_eq!(model.selected, 1);
                            assert!(!model.menu.open);
                        }
                        expected = match state {
                            "hover" => "action-primary-hover",
                            "pressed" => "action-primary-pressed",
                            "disabled" | "empty" => "surface-2",
                            _ => "action-primary-ground",
                        };
                        result
                    } else {
                        let mut model = ComboBox {
                            items: labels.iter().map(|s| (*s).into()).collect(),
                            ..Default::default()
                        };
                        if state == "selected" {
                            model.set_query(labels[1]);
                            assert_eq!(model.key(ControlKey::Enter), Some(1));
                        }
                        if state == "open" {
                            model.open = true;
                            model.active = Some(0);
                        }
                        if state == "empty" {
                            model.set_query("no matching options");
                            assert!(model.filtered().is_empty());
                        }
                        model.disabled = state == "disabled";
                        let mut surface = TextSurface::new(&model.query, false, false);
                        surface.input.disabled = model.disabled;
                        surface.layout_with(frame, None, 1., 1., |s, w| {
                            text.shape_surface(s, proposal::TextRole::UiBody, w)
                        });
                        let point = surface.inner.center();
                        if state == "pressed" {
                            assert!(surface.pointer_down(point, false, 1, 24.).is_some());
                        }
                        let pointer = surface.pointer_state(
                            if matches!(state, "hover" | "pressed" | "disabled") {
                                Some(point)
                            } else {
                                None
                            },
                        );
                        assert_eq!(pointer.hovered, matches!(state, "hover" | "pressed"));
                        assert_eq!(pointer.pressed, state == "pressed");
                        let mut view = ComboBoxView::default();
                        view.options.set_reduced_motion(true);
                        let draft = surface.input.text.clone();
                        let value = model.selected;
                        let result = observe(|| {
                            let mut scene = Scene::new();
                            surface.draw(
                                &mut text,
                                &mut scene,
                                palette,
                                matches!(state, "focus" | "pressed" | "open" | "empty"),
                                false,
                            );
                            let layout = view.render(
                                &mut scene,
                                &mut text,
                                &mut model,
                                frame,
                                viewport,
                                None,
                                palette,
                                proposal::TextRole::UiBody,
                                false,
                            );
                            assert_eq!(layout.is_some(), state == "open");
                            (scene, pointer)
                        });
                        assert_eq!(surface.visible_frame(), Some(frame));
                        assert_eq!(surface.input.text, draft);
                        assert_eq!(model.query, draft);
                        assert_eq!(model.selected, value);
                        assert_eq!(model.open, matches!(state, "open" | "empty"));
                        if state == "selected" {
                            assert_eq!(model.selected, Some(1));
                            assert_eq!(surface.input.text, labels[1]);
                        }
                        if state == "disabled" {
                            assert!(!surface.input.insert("x", None));
                            model.set_query("ignored");
                            assert_eq!(model.query, draft);
                        }
                        if state == "open" {
                            assert_eq!(view.key(ControlKey::Down, true, &mut model), None);
                            assert_eq!(view.key(ControlKey::Enter, true, &mut model), Some(1));
                            assert_eq!(model.query, labels[1]);
                        }
                        if state == "empty" {
                            assert_eq!(view.key(ControlKey::Enter, true, &mut model), None);
                            assert!(model.open);
                            assert_eq!(model.query, draft);
                        }
                        expected = "transcript-input";
                        result
                    };
                    // Expected token reads occur after the production capture, never in its lookup list.
                    let base = scene
                        .primitives()
                        .iter()
                        .find_map(|p| match p {
                            Primitive::Quad(q) if q.rect == frame => Some(q),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        base.fill,
                        palette.color(expected).unwrap(),
                        "{component}/{state}/{theme:?}/{brand}"
                    );
                    if component == "tui.select" && state == "error" {
                        let boundary = proposal::control_boundary("error", theme).unwrap();
                        assert_eq!(base.border_color, palette.color(boundary).unwrap());
                    }
                    if component == "tui.combo-box" {
                        let boundary = proposal::control_boundary("required-input", theme).unwrap();
                        assert_eq!(base.border_color, palette.color(boundary).unwrap());
                        if state != "open" {
                            assert!(
                                !reads.iter().any(|(family, token)| family == "color"
                                    && (token.starts_with("action-primary-")
                                        || token == "button-hover"
                                        || token == "button-pressed")),
                                "closed query must not borrow Button skin dependencies"
                            );
                        }
                    }
                    let mut case = serde_json::json!({"component":component,"choice_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads});
                    if component == "tui.combo-box" {
                        case["pointer_hovered"] = pointer.hovered.into();
                        case["pointer_pressed"] = pointer.pressed.into();
                    }
                    cases.push(case);
                }
            }
        }
    }
    assert_eq!(cases.len(), 204);
    if let Some(path) = std::env::var_os("TEKESUI_CHOICE_SELECTION_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("explicit token-audit feature required");
        }
        #[cfg(feature="token-audit")] std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
