//! Anchored popup bounds must follow the caller's measured control geometry.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, Menu, MenuEvent, MenuView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Labels;
impl ButtonTextRenderer for Labels {
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
fn layout(
    anchor: Rect,
    viewport: Rect,
    count: usize,
) -> (MenuView, Menu, tekes_ui_controls::MenuLayout) {
    let mut v = MenuView::new_external();
    let mut m = Menu::new(count);
    m.open = true;
    let labels = vec!["Choice"; count];
    let l = v
        .render_with_model(
            &mut m,
            &mut Scene::new(),
            &mut Labels,
            &labels,
            anchor,
            viewport,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        )
        .unwrap();
    (v, m, l)
}
#[test]
fn menu_follows_the_bottom_of_measured_tall_anchors() {
    for height in [36., 64., 100.] {
        let anchor = Rect::new(20., 20., 180., height);
        let (_, _, l) = layout(anchor, Rect::new(0., 0., 360., 480.), 2);
        let gap = proposal::metric("spacing", "space-micro").unwrap();
        assert!(
            l.popup.min_y() >= anchor.max_y() + gap - 0.01,
            "{height}: {:?}",
            l.popup
        );
    }
}
#[test]
fn menu_flips_above_when_the_measured_popup_fits_there() {
    let anchor = Rect::new(20., 270., 180., 64.);
    let (_, _, l) = layout(anchor, Rect::new(0., 0., 360., 400.), 2);
    assert!(l.popup.max_y() <= anchor.min_y(), "{:?}", l.popup);
}
#[test]
fn scrollable_menu_uses_available_side_and_reveals_keyboard_target() {
    let viewport = Rect::new(0., 0., 360., 260.);
    let anchor = Rect::new(20., 90., 180., 64.);
    let (mut v, mut m, l) = layout(anchor, viewport, 12);
    assert!(
        l.popup.max_y() <= anchor.min_y() || l.popup.min_y() >= anchor.max_y(),
        "{:?}",
        l.popup
    );
    assert!(l.popup.min_y() >= viewport.min_y() && l.popup.max_y() <= viewport.max_y());
    v.key_with_model(&mut m, ControlKey::End, true);
    let labels = vec!["Choice"; 12];
    let l = v
        .render_with_model(
            &mut m,
            &mut Scene::new(),
            &mut Labels,
            &labels,
            anchor,
            viewport,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        )
        .unwrap();
    assert!(l.scroll_offset > 0.);
    assert!(l.items.last().unwrap().visible.is_some());
    assert!(matches!(
        v.key_with_model(&mut m, ControlKey::Enter, true),
        MenuEvent::Activated { index: 11, .. }
    ));
}
#[test]
fn menu_stays_bounded_when_no_side_can_show_a_complete_row() {
    let viewport = Rect::new(0., 0., 120., 40.);
    let (_, _, l) = layout(viewport, viewport, 3);
    assert!(l.popup.min_y() >= 0. && l.popup.max_y() <= 40.);
    assert!(l.popup.size.height.is_finite());
}

#[test]
fn public_popup_measurement_matches_unconstrained_production_layout() {
    let labels = ["Choice"; 4];
    let measured = MenuView::measure(
        &mut Labels,
        &labels,
        360.,
        proposal::TextRole::UiBody,
        false,
    );
    let (_, _, l) = layout(
        Rect::new(20., 20., 180., 64.),
        Rect::new(0., 0., 360., 800.),
        4,
    );
    assert_eq!(measured, l.popup.size);
}

#[test]
fn production_popup_geometry_and_lookup_observations_are_bounded() {
    use tekes_ui_controls::extended::{ComboBox, CommandPalette};
    use tekes_ui_controls::layout_view::{ComboBoxView, CommandPaletteView, SelectView};
    let mut cases = Vec::new();
    for component in [
        "tui.menu",
        "tui.select",
        "tui.combo-box",
        "tui.command-palette",
    ] {
        for case in ["below", "above", "constrained", "empty"] {
            for theme in Theme::ALL {
                for brand in 0..3 {
                    let run = || {
                        let mut text = Labels;
                        let mut scene = Scene::new();
                        let palette = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
                        let viewport = Rect::new(
                            0.,
                            0.,
                            360.,
                            if case == "constrained" { 260. } else { 480. },
                        );
                        let anchor = Rect::new(
                            20.,
                            if case == "above" {
                                360.
                            } else if case == "constrained" {
                                90.
                            } else {
                                20.
                            },
                            220.,
                            64.,
                        );
                        let labels = if case == "empty" {
                            Vec::new()
                        } else {
                            vec!["First choice", "Second choice", "Third choice"]
                        };
                        let popup = match component {
                            "tui.menu" => {
                                let mut v = MenuView::new_external();
                                let mut m = Menu::new(labels.len());
                                m.open = true;
                                v.render_with_model(
                                    &mut m,
                                    &mut scene,
                                    &mut text,
                                    &labels,
                                    anchor,
                                    viewport,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                )
                            }
                            "tui.select" => {
                                let mut v = SelectView::new(labels.len());
                                let mut m = tekes_ui_controls::Select::new(labels.len());
                                m.menu.open = true;
                                v.render(
                                    &mut scene,
                                    &mut text,
                                    &mut m,
                                    &labels,
                                    anchor,
                                    viewport,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                )
                                .popup
                            }
                            "tui.combo-box" => {
                                let mut v = ComboBoxView::default();
                                let mut m = ComboBox {
                                    items: labels.iter().map(|s| s.to_string()).collect(),
                                    open: true,
                                    active: Some(0),
                                    ..Default::default()
                                };
                                let result = v.render(
                                    &mut scene,
                                    &mut text,
                                    &mut m,
                                    anchor,
                                    viewport,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                );
                                assert!(m.open);
                                result
                            }
                            _ => {
                                let mut v = CommandPaletteView::default();
                                let mut m = CommandPalette {
                                    combo: ComboBox {
                                        items: labels.iter().map(|s| s.to_string()).collect(),
                                        open: true,
                                        active: Some(0),
                                        ..Default::default()
                                    },
                                };
                                let result = v.render(
                                    &mut scene,
                                    &mut text,
                                    &mut m,
                                    anchor,
                                    viewport,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                );
                                assert!(m.combo.open);
                                result
                            }
                        };
                        if case == "empty"
                            && matches!(
                                component,
                                "tui.select" | "tui.combo-box" | "tui.command-palette"
                            )
                        {
                            assert!(popup.is_none());
                            if component == "tui.select" {
                                assert!(scene.primitives().iter().any(|p| matches!(p, tekes_ui_scene::Primitive::Quad(q) if q.rect == anchor)), "empty Select keeps only its unavailable trigger, not a popup");
                            } else {
                                assert!(scene.primitives().is_empty());
                            }
                        } else {
                            let popup = popup.unwrap().popup;
                            assert!(
                                popup.min_y() >= anchor.max_y() || popup.max_y() <= anchor.min_y()
                            );
                            assert!(
                                popup.min_y() >= viewport.min_y()
                                    && popup.max_y() <= viewport.max_y()
                            );
                        }
                    };
                    #[cfg(feature = "token-audit")]
                    let (_, lookups) = tekes_ui_tokens::audit::capture(run);
                    #[cfg(not(feature = "token-audit"))]
                    let lookups: Vec<(String, String)> = {
                        run();
                        Vec::new()
                    };
                    cases.push(serde_json::json!({"component":component,"geometry_case":case,"state_asserted":false,"theme":format!("{theme:?}"),"brand":brand,"lookups":lookups}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 192);
    if let Ok(path) = std::env::var("TEKESUI_POPUP_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Public production popup geometry only; four existing consumers x four scenarios x four themes x three brands. No required-state coverage or native acceptance inferred.","excluded":["complete state profiles","native input/AX","font backend internals","unexecuted paths"],"cases":cases})).unwrap()).unwrap();
    }
}
