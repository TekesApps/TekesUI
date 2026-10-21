//! Current model state wins over a menu geometry retained by an earlier frame.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, MenuEvent,
    extended::{ComboBox, CommandPalette},
    layout_view::{ComboBoxView, CommandPaletteView},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Labels;
impl ButtonTextRenderer for Labels {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), r.style().line_height)
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
fn split_select_overlay_rejects_options_cleared_after_trigger_layout() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let mut view = SelectView::default();
    view.set_reduced_motion(true);
    let mut model = Select::new(2);
    model.menu.open = true;
    let mut scene = Scene::new();
    view.render_trigger(
        &mut scene,
        &mut Labels,
        &mut model,
        &["A", "B"],
        Rect::new(20., 20., 220., 44.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    let trigger_scene = scene.clone();
    model.menu.items.count = 0;
    assert!(
        view.render_options(
            &mut scene,
            &mut Labels,
            &mut model,
            &[],
            Rect::new(0., 0., 400., 500.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false
        )
        .is_none()
    );
    assert!(!model.menu.open);
    assert_eq!(
        scene, trigger_scene,
        "empty late data must not paint an empty popup"
    );
}
#[test]
fn select_combined_and_split_paint_keep_the_same_public_geometry_and_tokens() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    for theme in Theme::ALL {
        for brand in 0..3 {
            for open in [false, true] {
                let palette = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
                let mut combined = SelectView::default();
                let mut split = SelectView::default();
                combined.set_reduced_motion(true);
                split.set_reduced_motion(true);
                combined.set_error(true);
                split.set_error(true);
                combined.focus(true);
                split.focus(true);
                let mut a = Select::new(3);
                a.choose(1);
                a.menu.open = open;
                let mut b = a.clone();
                let frame = Rect::new(20., 20., 220., 44.);
                let viewport = Rect::new(0., 0., 400., 500.);
                let labels = ["Automatic", "Compact", "Comfortable"];
                let mut left = Scene::new();
                let mut right = Scene::new();
                let expected = combined.render(
                    &mut left,
                    &mut Labels,
                    &mut a,
                    &labels,
                    frame,
                    viewport,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                    false,
                );
                split.render_trigger(
                    &mut right,
                    &mut Labels,
                    &mut b,
                    &labels,
                    frame,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                );
                assert!(
                    split.layout().popup.is_none(),
                    "base pass does not paint an overlay"
                );
                split.render_options(
                    &mut right,
                    &mut Labels,
                    &mut b,
                    &labels,
                    viewport,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                    false,
                );
                assert_eq!(&expected, split.layout());
                assert_eq!(left, right);
                assert_eq!(a.selected, b.selected);
                assert_eq!(a.menu.open, b.menu.open);
            }
        }
    }
}

#[test]
fn empty_select_never_opens_and_count_drop_cancels_a_retained_trigger_press() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let mut model = Select::new(0);
    let mut view = SelectView::default();
    view.set_reduced_motion(true);
    let frame = Rect::new(20., 20., 220., 44.);
    let render = |view: &mut SelectView, model: &mut Select, labels: &[&str]| {
        view.render(
            &mut Scene::new(),
            &mut Labels,
            model,
            labels,
            frame,
            Rect::new(0., 0., 400., 500.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        )
    };
    render(&mut view, &mut model, &[]);
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    assert!(
        !model.menu.open,
        "an empty fixed-option selector has no menu to open"
    );
    assert!(!view.pointer_down(frame.center(), &mut model));
    view.pointer_up(frame.center(), &mut model);
    assert!(!model.menu.open);
    assert!(!model.choose(0));
    render(&mut view, &mut model, &["Automatic", "Compact"]);
    assert!(view.pointer_down(frame.center(), &mut model));
    model.menu.items.count = 0;
    view.pointer_up(frame.center(), &mut model);
    assert!(
        !model.menu.open,
        "latest empty model cancels a retained press before another frame"
    );
    render(&mut view, &mut model, &["Automatic", "Compact"]);
    view.pointer_up(frame.center(), &mut model);
    assert!(
        !model.menu.open,
        "restoring choices must not revive the old arm"
    );
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    assert!(model.menu.open);
    render(&mut view, &mut model, &[]);
    assert!(!model.menu.open);
    assert!(view.layout().popup.is_none());
    render(&mut view, &mut model, &["Automatic", "Compact"]);
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    assert!(model.menu.open, "fresh input after choices return works");
}

#[test]
fn repeated_combo_confirmation_reports_each_selection_even_when_value_is_unchanged() {
    let mut view = ComboBoxView::default();
    let mut model = model();
    for _ in 0..2 {
        model.set_query("Alpha");
        assert_eq!(view.key(ControlKey::Enter, true, &mut model), Some(0));
        assert_eq!(model.selected, Some(0));
        assert!(!model.open);
        assert_eq!(view.key(ControlKey::Enter, false, &mut model), None);
    }
    model.set_query("no results");
    assert_eq!(view.key(ControlKey::Enter, true, &mut model), None);
    assert!(model.open);
    assert_eq!(model.selected, Some(0));
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), None);
}
#[test]
fn repeated_command_confirmation_reports_each_activation_even_when_value_is_unchanged() {
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette { combo: model() };
    for _ in 0..2 {
        view.open(&mut model);
        assert_eq!(view.key(ControlKey::Enter, true, &mut model), Some(0));
        assert_eq!(model.combo.selected, Some(0));
        assert!(!model.combo.open);
        assert_eq!(view.key(ControlKey::Enter, false, &mut model), None);
    }
    view.open(&mut model);
    model.combo.set_query("no results");
    assert_eq!(view.key(ControlKey::Enter, true, &mut model), None);
    assert!(model.combo.open);
    assert_eq!(model.combo.selected, Some(0));
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), None);
}

#[test]
fn select_keyboard_close_restores_visible_trigger_focus_without_host_help() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::Palette;
    for theme in Theme::ALL {
        for brand in 0..3 {
            for close in [ControlKey::Enter, ControlKey::Escape] {
                let palette = Palette::new(theme, brand).unwrap();
                let mut model = Select::new(3);
                let mut view = SelectView::default();
                view.set_reduced_motion(true);
                let frame = Rect::new(20., 20., 220., 44.);
                let render = |view: &mut SelectView, model: &mut Select| {
                    let mut scene = Scene::new();
                    view.render(
                        &mut scene,
                        &mut Labels,
                        model,
                        &["Automatic", "Compact", "Comfortable"],
                        frame,
                        Rect::new(0., 0., 400., 500.),
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    scene
                };
                let ring = |scene: &Scene| {
                    scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.fill.a == 0. && q.border_color == palette.color("border-focused").unwrap()))
                };
                render(&mut view, &mut model);
                view.pointer_down(frame.center(), &mut model);
                view.pointer_up(frame.center(), &mut model);
                assert!(model.menu.open);
                assert!(
                    !ring(&render(&mut view, &mut model)),
                    "mouse-open trigger does not acquire a keyboard ring"
                );
                view.key(ControlKey::Down, true, &mut model);
                view.key(close, true, &mut model);
                assert!(!model.menu.open);
                assert_eq!(model.selected, usize::from(close == ControlKey::Enter));
                assert!(
                    ring(&render(&mut view, &mut model)),
                    "keyboard close restores the owned trigger focus presentation: {close:?}"
                );
                view.key(close, false, &mut model);
                assert!(!model.menu.open, "confirm release must not reopen");
                view.blur(&mut model);
                assert!(!ring(&render(&mut view, &mut model)));
            }
        }
    }
}

#[test]
fn select_trigger_closes_an_open_menu_without_rearming_it_on_the_same_click() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let mut model = Select::new(3);
    let mut view = SelectView::default();
    view.set_reduced_motion(true);
    let frame = Rect::new(20., 20., 220., 44.);
    let render = |view: &mut SelectView, model: &mut Select| {
        view.render(
            &mut Scene::new(),
            &mut Labels,
            model,
            &["Automatic", "Compact", "Comfortable"],
            frame,
            Rect::new(0., 0., 400., 500.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        )
    };
    render(&mut view, &mut model);
    let click = |view: &mut SelectView, model: &mut Select| {
        assert!(view.pointer_down(frame.center(), model));
        view.pointer_up(frame.center(), model);
    };
    click(&mut view, &mut model);
    assert!(model.menu.open);
    render(&mut view, &mut model);
    click(&mut view, &mut model);
    assert!(
        !model.menu.open,
        "dismissed trigger click must stay closed after release"
    );
    assert_eq!(model.selected, 0);
    render(&mut view, &mut model);
    click(&mut view, &mut model);
    assert!(model.menu.open, "a fresh click can reopen the select");
    render(&mut view, &mut model);
    let outside = Point::new(350., 450.);
    assert!(view.pointer_down(outside, &mut model));
    view.pointer_up(outside, &mut model);
    assert!(!model.menu.open);
    view.pointer_move(frame.center(), &mut model);
    assert!(!model.menu.open);
}

#[test]
fn command_trigger_hover_and_pointer_drag_use_existing_button_contract() {
    let frame = Rect::new(20., 20., 220., 44.);
    let inside = frame.center();
    let outside = Point::new(300., 100.);
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette::default();
    model.combo.query = "preserved".into();
    let render = |view: &mut CommandPaletteView, model: &mut CommandPalette| {
        let mut scene = Scene::new();
        view.render_trigger(
            &mut scene,
            &mut Labels,
            model,
            "Commands",
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
        );
        scene
    };
    let rest = render(&mut view, &mut model);
    view.pointer_move(inside, &mut model);
    let hover = render(&mut view, &mut model);
    assert_ne!(rest, hover, "closed trigger must receive hover");
    assert!(!model.combo.open);
    assert_eq!(model.combo.query, "preserved");
    assert!(view.pointer_down_trigger(inside));
    let pressed = render(&mut view, &mut model);
    assert_ne!(hover, pressed);
    view.pointer_move(outside, &mut model);
    let outside_scene = render(&mut view, &mut model);
    assert_eq!(
        rest, outside_scene,
        "dragging outside releases hover and pressed paint"
    );
    assert!(!view.pointer_up_trigger(outside, &mut model));
    assert!(!model.combo.open);
    view.pointer_move(inside, &mut model);
    assert_eq!(hover, render(&mut view, &mut model));
    view.pointer_move(outside, &mut model);
    assert_eq!(rest, render(&mut view, &mut model));
    model.combo.disabled = true;
    view.pointer_move(inside, &mut model);
    let disabled = render(&mut view, &mut model);
    view.pointer_move(outside, &mut model);
    assert_eq!(disabled, render(&mut view, &mut model));
    assert!(!view.pointer_down_trigger(inside));
    assert!(!model.combo.open);
    assert_eq!(model.combo.query, "preserved");
}

#[test]
fn trigger_focus_transfer_preserves_open_query_but_whole_component_blur_dismisses() {
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette { combo: model() };
    model.combo.query = "Al".into();
    command(&mut view, &mut model);
    let render = |view: &mut CommandPaletteView, model: &mut CommandPalette| {
        let mut scene = Scene::new();
        view.render_trigger(
            &mut scene,
            &mut Labels,
            model,
            "Commands",
            Rect::new(10., 10., 220., 36.),
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
        );
        scene
    };
    view.focus_trigger(true);
    let focused = render(&mut view, &mut model);
    view.blur_trigger();
    let transferred = render(&mut view, &mut model);
    assert_ne!(focused, transferred);
    assert!(model.combo.open);
    assert_eq!(model.combo.query, "Al");
    assert!(view.options.is_visible());
    assert!(
        !view
            .trigger_key(ControlKey::Enter, true, &mut model)
            .consumed
    );
    assert_eq!(model.combo.query, "Al");
    view.blur(&mut model);
    assert!(!model.combo.open);
    assert!(!view.options.is_visible());
}
fn model() -> ComboBox {
    ComboBox {
        items: vec!["Alpha".into(), "Beta".into()],
        open: true,
        active: Some(0),
        ..Default::default()
    }
}
fn combo(view: &mut ComboBoxView, m: &mut ComboBox) -> Point {
    view.render(
        &mut Scene::new(),
        &mut Labels,
        m,
        Rect::new(10., 10., 220., 36.),
        Rect::new(0., 0., 320., 300.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    )
    .unwrap()
    .items[0]
        .frame
        .center()
}
fn command(view: &mut CommandPaletteView, m: &mut CommandPalette) -> Point {
    view.render(
        &mut Scene::new(),
        &mut Labels,
        m,
        Rect::new(10., 10., 220., 36.),
        Rect::new(0., 0., 320., 300.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    )
    .unwrap()
    .items[0]
        .frame
        .center()
}
#[test]
fn choice_popup_release_rejects_latest_disabled_closed_or_changed_items_before_render() {
    for palette in [false, true] {
        for change in 0..5 {
            let mut c = ComboBoxView::default();
            let mut p = CommandPaletteView::default();
            let mut m = CommandPalette { combo: model() };
            let point = if palette {
                command(&mut p, &mut m)
            } else {
                combo(&mut c, &mut m.combo)
            };
            let down = if palette {
                p.pointer_down(point)
            } else {
                c.pointer_down(point)
            };
            assert!(!matches!(down, MenuEvent::Dismissed { .. }));
            match change {
                0 => m.combo.disabled = true,
                1 => m.combo.open = false,
                2 => m.combo.items.clear(),
                3 => m.combo.items[0] = "Replacement".into(),
                _ => m.combo.set_query("Beta"),
            };
            let before = m.combo.clone();
            let selected = if palette {
                p.pointer_up(point, &mut m)
            } else {
                c.pointer_up(point, &mut m.combo)
            };
            assert_eq!(selected, None, "palette={palette} change={change}");
            assert_eq!(m.combo.selected, before.selected);
            assert_eq!(m.combo.query, before.query);
        }
    }
}
#[test]
fn choice_observed_disabled_key_cancels_old_capture_before_reenable() {
    for palette in [false, true] {
        let mut c = ComboBoxView::default();
        let mut p = CommandPaletteView::default();
        let mut m = CommandPalette { combo: model() };
        let point = if palette {
            command(&mut p, &mut m)
        } else {
            combo(&mut c, &mut m.combo)
        };
        if palette {
            p.pointer_down(point);
        } else {
            c.pointer_down(point);
        }
        m.combo.disabled = true;
        if palette {
            p.key_event(ControlKey::Escape, true, &mut m);
        } else {
            c.key_event(ControlKey::Escape, true, &mut m.combo);
        }
        m.combo.disabled = false;
        let selected = if palette {
            p.pointer_up(point, &mut m)
        } else {
            c.pointer_up(point, &mut m.combo)
        };
        assert_eq!(selected, None);
        assert_eq!(m.combo.selected, None);
    }
}
#[test]
fn choice_changed_rendered_content_cancels_old_capture_but_fresh_click_selects() {
    for palette in [false, true] {
        let mut c = ComboBoxView::default();
        let mut p = CommandPaletteView::default();
        let mut m = CommandPalette { combo: model() };
        let point = if palette {
            command(&mut p, &mut m)
        } else {
            combo(&mut c, &mut m.combo)
        };
        if palette {
            p.pointer_down(point);
        } else {
            c.pointer_down(point);
        }
        m.combo.items[0] = "Replacement".into();
        let current = if palette {
            command(&mut p, &mut m)
        } else {
            combo(&mut c, &mut m.combo)
        };
        let selected = if palette {
            p.pointer_up(current, &mut m)
        } else {
            c.pointer_up(current, &mut m.combo)
        };
        assert_eq!(selected, None);
        if palette {
            p.pointer_down(current);
        } else {
            c.pointer_down(current);
        }
        let selected = if palette {
            p.pointer_up(current, &mut m)
        } else {
            c.pointer_up(current, &mut m.combo)
        };
        assert_eq!(selected, Some(0));
        assert_eq!(m.combo.query, "Replacement");
    }
}
#[test]
fn command_trigger_consults_current_disabled_without_another_render() {
    for keyboard in [false, true] {
        let mut view = CommandPaletteView::default();
        let mut m = CommandPalette { combo: model() };
        m.combo.open = false;
        let frame = Rect::new(0., 0., 240., 40.);
        view.render_trigger(
            &mut Scene::new(),
            &mut Labels,
            &mut m,
            "Commands",
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
        );
        view.focus_trigger(true);
        if keyboard {
            assert!(view.trigger_key(ControlKey::Space, true, &mut m).consumed);
        } else {
            assert!(view.pointer_down_trigger(frame.center()));
        }
        m.combo.disabled = true;
        let activated = if keyboard {
            view.trigger_key(ControlKey::Space, false, &mut m).changed
        } else {
            view.pointer_up_trigger(frame.center(), &mut m)
        };
        assert!(!activated);
        assert!(!m.combo.open);
        m.combo.disabled = false;
        assert!(!view.trigger_key(ControlKey::Space, false, &mut m).changed);
        assert!(!m.combo.open);
    }
}
#[test]
fn disabled_command_model_and_public_open_preserve_the_query() {
    for through_view in [false, true] {
        let mut m = CommandPalette { combo: model() };
        m.combo.open = false;
        m.combo.disabled = true;
        m.combo.query = "draft query".into();
        let mut view = CommandPaletteView::default();
        if through_view {
            view.open(&mut m);
        } else {
            m.open();
        }
        assert!(!m.combo.open);
        assert_eq!(m.combo.query, "draft query");
    }
}
#[test]
fn select_existing_availability_setter_cancels_old_trigger_release() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let mut m = Select::new(2);
    let mut v = SelectView::new(2);
    let frame = Rect::new(0., 0., 220., 36.);
    v.render(
        &mut Scene::new(),
        &mut Labels,
        &mut m,
        &["Alpha", "Beta"],
        frame,
        Rect::new(0., 0., 320., 300.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    assert!(v.pointer_down(frame.center(), &mut m));
    v.set_disabled(true, &mut m);
    v.set_disabled(false, &mut m);
    v.pointer_up(frame.center(), &mut m);
    assert!(!m.menu.open);
    assert_eq!(m.selected, 0);
    assert!(v.pointer_down(frame.center(), &mut m));
    v.pointer_up(frame.center(), &mut m);
    assert!(m.menu.open);
}

#[test]
fn empty_choice_candidates_keep_query_open_without_a_phantom_menu_surface() {
    for palette in [false, true] {
        let mut c = ComboBoxView::default();
        let mut p = CommandPaletteView::default();
        let mut m = CommandPalette { combo: model() };
        m.combo.set_query("no matches");
        let mut scene = Scene::new();
        let anchor = Rect::new(0., 0., 240., 64.);
        let viewport = Rect::new(0., 0., 360., 400.);
        let shown = if palette {
            p.render(
                &mut scene,
                &mut Labels,
                &mut m,
                anchor,
                viewport,
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            )
        } else {
            c.render(
                &mut scene,
                &mut Labels,
                &mut m.combo,
                anchor,
                viewport,
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            )
        };
        assert!(shown.is_none());
        assert!(m.combo.open);
        assert_eq!(m.combo.query, "no matches");
        assert_eq!(m.combo.selected, None);
        assert!(scene.primitives().is_empty());
    }
}
