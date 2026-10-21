#![cfg(all(feature = "token-audit", target_os = "macos"))]
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::Tooltip,
    feedback_view::{TooltipEvent, TooltipView},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::ShapeOptions,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Text(CoreTextShaper);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        let m = role.style();
        let p = self.0.shape(
            s,
            &TextStyle::system(m.size).with_weight(m.weight),
            &ShapeOptions {
                width: Some(width as f64),
                line_spacing: 0.,
            },
        );
        Size::new(p.width as f32, p.height as f32)
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
fn anchor() -> Rect {
    Rect::new(20., 20., 100., 30.)
}
fn render(
    view: &mut TooltipView,
    model: &Tooltip,
    text: &mut Text,
    palette: Palette,
) -> (bool, usize) {
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        text,
        model,
        anchor(),
        Size::new(180., 52.),
        Rect::new(0., 0., 300., 180.),
        None,
        palette,
        proposal::TextRole::UiCaption,
    );
    (layout.visible.is_some(), scene.len())
}
#[test]
fn tooltip_rest_and_open_have_asserted_actual_reads_including_empty_closed_renders() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            let palette = Palette { theme, brand };
            for scenario in [
                "rest",
                "hover-open",
                "focus-open",
                "controlled-open",
                "escape-suppressed",
                "inactive",
                "late-hover-restored",
                "late-focus-restored",
                "controlled-resumed",
                "fresh-hover-restored",
                "fresh-focus-restored",
                "controlled-reopen",
                "new-hover-after-escape",
                "new-focus-after-escape",
                "duplicate-hover-after-escape",
                "duplicate-focus-after-escape",
                "hover-exit-retained-focus",
                "focus-exit-retained-hover",
                "new-controlled-after-escape",
                "duplicate-controlled-after-escape",
            ] {
                let mut text = Text(CoreTextShaper::new(CoreTextFonts::new()));
                let mut model = Tooltip {
                    visible: matches!(
                        scenario,
                        "controlled-open"
                            | "controlled-resumed"
                            | "controlled-reopen"
                            | "new-controlled-after-escape"
                            | "duplicate-controlled-after-escape"
                    ),
                    text: "Helpful detail · 操作说明".into(),
                };
                let mut view = TooltipView::default();
                render(&mut view, &model, &mut text, palette);
                match scenario {
                    "new-hover-after-escape"
                    | "new-focus-after-escape"
                    | "duplicate-hover-after-escape"
                    | "duplicate-focus-after-escape"
                    | "hover-exit-retained-focus"
                    | "focus-exit-retained-hover" => {
                        view.pointer_move(anchor().center());
                        view.set_focus(true, true);
                        assert!(render(&mut view, &model, &mut text, palette).0);
                        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
                        assert!(!render(&mut view, &model, &mut text, palette).0);
                        if matches!(
                            scenario,
                            "new-hover-after-escape" | "hover-exit-retained-focus"
                        ) {
                            view.pointer_move(tekes_ui_scene::Point::new(-10., -10.));
                            assert!(!render(&mut view, &model, &mut text, palette).0);
                            if scenario == "new-hover-after-escape" {
                                view.pointer_move(anchor().center());
                            }
                        } else if matches!(
                            scenario,
                            "new-focus-after-escape" | "focus-exit-retained-hover"
                        ) {
                            view.set_focus(false, false);
                            assert!(!render(&mut view, &model, &mut text, palette).0);
                            if scenario == "new-focus-after-escape" {
                                view.set_focus(true, true);
                            }
                        } else if scenario == "duplicate-hover-after-escape" {
                            view.pointer_move(anchor().center());
                        } else {
                            view.set_focus(true, true);
                        }
                    }
                    "new-controlled-after-escape" | "duplicate-controlled-after-escape" => {
                        view.pointer_move(anchor().center());
                        view.set_focus(true, true);
                        assert!(render(&mut view, &model, &mut text, palette).0);
                        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
                        assert!(!render(&mut view, &model, &mut text, palette).0);
                        if scenario == "new-controlled-after-escape" {
                            model.visible = false;
                            assert!(!render(&mut view, &model, &mut text, palette).0);
                            model.visible = true;
                        }
                    }
                    "hover-open" => view.pointer_move(anchor().center()),
                    "focus-open" => view.set_focus(true, true),
                    "escape-suppressed" => {
                        view.pointer_move(anchor().center());
                        render(&mut view, &model, &mut text, palette);
                        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
                    }
                    "inactive" => {
                        view.pointer_move(anchor().center());
                        render(&mut view, &model, &mut text, palette);
                        view.set_window_active(false);
                    }
                    "late-hover-restored" | "fresh-hover-restored" => {
                        view.set_window_active(false);
                        view.pointer_move(anchor().center());
                        view.set_window_active(true);
                        assert!(!render(&mut view, &model, &mut text, palette).0);
                        if scenario == "fresh-hover-restored" {
                            view.pointer_move(anchor().center());
                        }
                    }
                    "late-focus-restored" | "fresh-focus-restored" => {
                        view.set_window_active(false);
                        view.set_focus(true, true);
                        view.set_window_active(true);
                        assert!(!render(&mut view, &model, &mut text, palette).0);
                        if scenario == "fresh-focus-restored" {
                            view.set_focus(true, true);
                        }
                    }
                    "controlled-resumed" => {
                        view.set_window_active(false);
                        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::None);
                        view.set_window_active(true);
                    }
                    "controlled-reopen" => {
                        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
                        assert!(!render(&mut view, &model, &mut text, palette).0);
                        model.visible = false;
                        render(&mut view, &model, &mut text, palette);
                        model.visible = true;
                    }
                    _ => {}
                }
                let expected = matches!(
                    scenario,
                    "hover-open"
                        | "focus-open"
                        | "controlled-open"
                        | "controlled-resumed"
                        | "fresh-hover-restored"
                        | "fresh-focus-restored"
                        | "controlled-reopen"
                        | "new-hover-after-escape"
                        | "new-focus-after-escape"
                        | "new-controlled-after-escape"
                );
                let ((open, commands), reads) = tekes_ui_tokens::audit::capture(|| {
                    render(&mut view, &model, &mut text, palette)
                });
                assert_eq!(open, expected, "{scenario}");
                assert_eq!(commands == 0, !expected);
                assert_eq!(
                    reads.is_empty(),
                    !expected,
                    "closed component draws/reads nothing; no invented host tokens"
                );
                cases.push(serde_json::json!({"component":"tui.tooltip","content_state":if open{"open"}else{"rest"},"scenario":scenario,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads,"no_dynamic_lookups":!expected,"zero_lookup_reason":if expected{""}else{"Closed/inactive Tooltip emits no semantic paint or dynamic lookup; constants and caller background are excluded."}}));
            }
        }
    }
    assert_eq!(cases.len(), 240);
    if let Ok(path) = std::env::var("TEKESUI_TOOLTIP_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Public TooltipView:20asserted lifecycle scenarios, including fresh vs duplicate independent owner intent after Escape,4themes/3brands. Real CoreText measurement, no-op glyph sink. Closed zero-read cases are explicit, not inferred color dependencies.","excluded":["constants and nonlookup paths","caller background and trigger tokens","complete alias/branch closure","GPU pixels/native/VoiceOver/performance"],"cases":cases})).unwrap()).unwrap();
    }
}
