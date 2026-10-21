//! Independent workspace: original SliderView, real events, primitive/token receipts.
use tekes_ui_controls::{ControlKey, SliderView, extended::Slider};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};
#[test]
fn production_slider_states_use_live_geometry_events_and_actual_tokens() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for initial in [0., 50., 100.] {
                for state in ["rest", "hover", "pressed", "focus", "disabled", "dragging"] {
                    let p = Palette::new(theme, brand).unwrap();
                    let mut model = Slider::new(initial, 0., 100., 5.).unwrap();
                    let mut view = SliderView::new();
                    let frame = Rect::new(16., 18., 220., 30.);
                    let probe =
                        view.render(&mut Scene::new(), model, frame, Some(frame.inset(-6.)), p);
                    match state {
                        "hover" => {
                            view.pointer_move(probe.thumb.center(), &mut model);
                        }
                        "pressed" => {
                            assert!(view.pointer_down(probe.thumb.center(), &mut model));
                        }
                        "dragging" => {
                            assert!(view.pointer_down(probe.thumb.center(), &mut model));
                            view.pointer_move(
                                Point::new(
                                    probe.track.min_x() + probe.track.size.width * 0.75,
                                    probe.track.center().y,
                                ),
                                &mut model,
                            );
                        }
                        "focus" => {
                            view.focus(true);
                        }
                        "disabled" => {
                            model.disabled = true;
                            view.focus(true);
                        }
                        _ => {}
                    }
                    let before = model.value();
                    #[cfg(feature = "token-audit")]
                    let ((scene, layout), reads) = tekes_ui_tokens::audit::capture(|| {
                        let mut scene = Scene::new();
                        let l = view.render(&mut scene, model, frame, Some(frame.inset(-6.)), p);
                        (scene, l)
                    });
                    #[cfg(not(feature = "token-audit"))]
                    let ((scene, layout), reads) = (
                        {
                            let mut scene = Scene::new();
                            let l =
                                view.render(&mut scene, model, frame, Some(frame.inset(-6.)), p);
                            (scene, l)
                        },
                        Vec::<(String, String)>::new(),
                    );
                    assert_eq!(model.value(), before);
                    assert_eq!(layout.frame, probe.frame);
                    assert_eq!(layout.track, probe.track);
                    assert_eq!(view.is_captured(), matches!(state, "pressed" | "dragging"));
                    assert_eq!(
                        view.is_focused(),
                        matches!(state, "pressed" | "dragging" | "focus")
                    );
                    if state == "dragging" {
                        assert_eq!(model.value(), 75.);
                    } else {
                        assert_eq!(model.value(), initial);
                    }
                    let quads = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| {
                            if let Primitive::Quad(q) = p {
                                Some(q)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(quads.len(), if state == "focus" { 4 } else { 3 });
                    assert_eq!(
                        quads[0].fill,
                        p.color(if state == "disabled" {
                            "surface-2"
                        } else {
                            "surface-3"
                        })
                        .unwrap()
                    );
                    assert_eq!(
                        quads[1].fill,
                        p.color(match state {
                            "disabled" => "disabled-text",
                            "hover" => "accent-hover",
                            "pressed" | "dragging" => "accent-pressed",
                            _ => "accent",
                        })
                        .unwrap()
                    );
                    assert_eq!(quads[2].rect, layout.thumb);
                    assert_eq!(
                        quads[2].border_color,
                        p.color(if state == "disabled" {
                            "disabled-text"
                        } else {
                            "border"
                        })
                        .unwrap()
                    );
                    assert!(
                        (layout.thumb.center().x
                            - (layout.track.min_x()
                                + layout.track.size.width * model.normalized_value() as f32))
                            .abs()
                            < 0.001
                    );
                    if state == "focus" {
                        assert_eq!(view.key(ControlKey::Home, true, &mut model), before != 0.);
                        assert_eq!(model.value(), 0.);
                        view.key(ControlKey::End, true, &mut model);
                        assert_eq!(model.value(), 100.);
                    }
                    if matches!(state, "pressed" | "dragging") {
                        assert!(view.pointer_up(Point::new(-100., -100.), &model));
                        assert!(!view.is_captured());
                        let value = model.value();
                        view.pointer_move(Point::new(-100., -100.), &mut model);
                        assert_eq!(model.value(), value);
                    }
                    if state == "disabled" {
                        assert!(!view.pointer_down(layout.thumb.center(), &mut model));
                        assert!(!view.key(ControlKey::Right, true, &mut model));
                        assert_eq!(model.value(), initial);
                    }
                    cases.push(serde_json::json!({"component":"tui.slider","range_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"initial":initial,"rendered_value":before,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 216);
    if let Some(path) = std::env::var_os("TEKESUI_SLIDER_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("token-audit required");
        }
        #[cfg(feature="token-audit")]std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases})).unwrap()).unwrap();
    }
}
