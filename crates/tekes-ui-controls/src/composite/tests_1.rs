use super::*;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Palette;
use tekes_ui_tokens::{Rgba, Theme, proposal};

pub(super) struct FixedText;

pub(super) struct TallText;

pub(super) struct ScaledText(pub(super) f32);

pub(super) struct WrappingText {
    wraps_below: f32,
}

pub(super) struct IntrinsicWrappingText {
    wraps_below: f32,
}

impl ButtonTextRenderer for WrappingText {
    fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        let line_count = if max_width < self.wraps_below {
            2.0
        } else {
            1.0
        };
        Size::new(max_width.max(0.0), line_count * 20.0)
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}

impl ButtonTextRenderer for IntrinsicWrappingText {
    fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        let natural_width = text.chars().count() as f32 * 7.0;
        Size::new(
            natural_width.min(max_width.max(0.0)),
            if max_width < self.wraps_below {
                40.0
            } else {
                20.0
            },
        )
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}

#[derive(Default)]
pub(super) struct RecordingText {
    draws: Vec<(String, Rect, ButtonTextAlign)>,
    scale: f32,
}

impl ButtonTextRenderer for RecordingText {
    fn backing_scale(&self) -> f32 {
        if self.scale > 0.0 { self.scale } else { 1.0 }
    }

    fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 7.0).min(max_width.max(0.0)),
            18.0,
        )
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        text: &str,
        _role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        _ink: Rgba,
    ) {
        self.draws.push((text.to_owned(), rect, align));
    }
}

impl ButtonTextRenderer for ScaledText {
    fn backing_scale(&self) -> f32 {
        self.0
    }

    fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 7.0).min(max_width.max(0.0)),
            18.0,
        )
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}

impl ButtonTextRenderer for TallText {
    fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 7.0).min(max_width.max(0.0)),
            100.0,
        )
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}

impl ButtonTextRenderer for FixedText {
    fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        let width = (text.chars().count() as f32 * 7.0).min(max_width.max(0.0));
        Size::new(width, 18.0)
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}

pub(super) fn palette() -> Palette {
    Palette::from(Theme::default())
}

pub(super) fn render_menu(view: &mut MenuView, labels: &[&str]) -> MenuLayout {
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut FixedText,
        labels,
        Rect::new(12.0, 12.0, 120.0, 34.0),
        Rect::new(0.0, 0.0, 320.0, 260.0),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    )
    .expect("open menu layout")
}

#[test]
fn select_prepared_open_is_pure_and_generation_exhaustion_is_checked() {
    let mut view = MenuView::new_external();
    let mut model = Menu::new(3);
    let kind = MenuPreparedKind::Open {
        generation: 0,
        selected: 99,
    };
    let prepared = view.prepare_select_action(&model, kind.clone()).unwrap();
    assert_eq!(
        view.prepare_select_action(&model, kind),
        Some(prepared.clone())
    );
    assert!(!model.open);
    assert_eq!(view.open_generation(), 0);
    assert_eq!(prepared.event, MenuEvent::None);
    view.apply_select_action(&mut model, prepared);
    assert!(model.open);
    assert_eq!(model.items.selected, 2);
    assert_eq!(view.open_generation(), 1);
    assert_eq!(view.runtime.return_focus, Some(0));
    let noop = view
        .prepare_select_action(
            &model,
            MenuPreparedKind::Open {
                generation: 0,
                selected: 0,
            },
        )
        .unwrap();
    view.apply_select_action(&mut model, noop);
    assert_eq!(model.items.selected, 2);
    assert_eq!(view.open_generation(), 1);
    model.open = false;
    view.select_test_generation(u64::MAX);
    assert!(
        view.prepare_select_action(
            &model,
            MenuPreparedKind::Open {
                generation: 0,
                selected: 0
            }
        )
        .is_none()
    );
    assert!(!model.open);
    view.select_test_generation(0);
    view.set_window_active_with_model(&mut model, false);
    assert!(
        view.prepare_select_action(
            &model,
            MenuPreparedKind::Open {
                generation: 0,
                selected: 0
            }
        )
        .is_none()
    );
}

#[test]
fn select_prepared_focus_uses_original_scroll_and_snapshot_staleness() {
    let mut view = MenuView::new_external();
    let mut model = Menu::new(20);
    view.show_with_model(&mut model, 42);
    view.render_with_model(
        &mut model,
        &mut Scene::new(),
        &mut FixedText,
        &["Choice"; 20],
        Rect::new(12., 12., 120., 34.),
        Rect::new(0., 0., 320., 260.),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    )
    .unwrap();
    let prepared = view
        .prepare_select_action(
            &model,
            MenuPreparedKind::Focus {
                index: 19,
                offset: 0.,
            },
        )
        .unwrap();
    assert_eq!(model.items.selected, 0);
    assert_eq!(view.runtime.scroll.offset.y, 0.);
    assert_eq!(prepared.event, MenuEvent::None);
    // A changed original arm must invalidate the outer caller's equality check.
    view.runtime.buttons[0].pointer_down();
    assert_ne!(
        view.prepare_select_action(
            &model,
            MenuPreparedKind::Focus {
                index: 19,
                offset: 0.
            }
        ),
        Some(prepared.clone())
    );
    view.runtime.buttons[0].cancel();
    let current = view
        .prepare_select_action(
            &model,
            MenuPreparedKind::Focus {
                index: 19,
                offset: 0.,
            },
        )
        .unwrap();
    let expected_offset = view.runtime.reveal_offset(19);
    view.apply_select_action(&mut model, current);
    assert_eq!(model.items.selected, 19);
    assert!(view.runtime.focus_visible);
    assert!(view.runtime.scroll.offset.y > 0.);
    assert_eq!(view.runtime.scroll.offset.y, expected_offset);
}

#[test]
fn select_prepared_activation_rejects_disabled_submenu_and_preserves_original_event() {
    let mut view = MenuView::new_external();
    let mut model = Menu::new(3);
    view.set_row_styles(vec![
        MenuRowStyle::default(),
        MenuRowStyle {
            submenu: true,
            ..Default::default()
        },
        MenuRowStyle {
            disabled: true,
            ..Default::default()
        },
    ]);
    view.show_with_model(&mut model, 42);
    view.render_with_model(
        &mut model,
        &mut Scene::new(),
        &mut FixedText,
        &["One", "Submenu", "Disabled"],
        Rect::new(12., 12., 120., 34.),
        Rect::new(0., 0., 320., 260.),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    )
    .unwrap();
    assert!(
        view.prepare_select_action(&model, MenuPreparedKind::Activate { index: 1 })
            .is_none()
    );
    assert!(
        view.prepare_select_action(&model, MenuPreparedKind::Activate { index: 2 })
            .is_none()
    );
    let action = view
        .prepare_select_action(&model, MenuPreparedKind::Activate { index: 0 })
        .unwrap();
    assert_eq!(
        action.event,
        MenuEvent::Activated {
            index: 0,
            restore_focus: Some(42)
        }
    );
    assert!(model.open);
    assert_eq!(model.last_action, None);
    view.apply_select_action(&mut model, action);
    assert!(!model.open);
    assert_eq!(model.last_action, Some(0));
    assert_eq!(view.runtime.return_focus, None);
    assert!(
        view.prepare_select_action(&model, MenuPreparedKind::Activate { index: 0 })
            .is_none()
    );
}

#[test]
fn select_replacement_and_prepared_close_retire_original_arms_and_geometry() {
    let mut view = MenuView::new_external();
    let mut model = Menu::new(2);
    view.show_with_model(&mut model, 42);
    let close = view
        .prepare_select_action(&model, MenuPreparedKind::Close)
        .unwrap();
    assert_eq!(
        close.event,
        MenuEvent::Dismissed {
            restore_focus: Some(42)
        }
    );
    view.apply_select_action(&mut model, close);
    assert!(!model.open);
    assert!(view.runtime.placement.items.is_empty());
    view.runtime.buttons.resize_with(2, ButtonModel::default);
    view.runtime.buttons[0].pointer_down();
    assert!(view.pointer_armed());
    view.runtime.scroll_gesture_captured = true;
    view.replace_items_with_model(&mut model, 1);
    assert_eq!(model.items.count, 1);
    assert!(!model.open);
    assert_eq!(model.last_action, None);
    assert!(!view.pointer_armed());
    assert!(!view.scroll_gesture_captured());
    assert!(!view.is_presented());
    assert!(view.runtime.placement.items.is_empty());
    assert_eq!(view.open_generation(), 1);
}

#[test]
fn checked_menu_open_generation_is_pure_and_reuses_original_external_open() {
    let mut view = MenuView::new_external();
    let mut model = Menu::new(2);
    assert_eq!(view.next_open_generation(), Some(1));
    assert_eq!(view.next_open_generation(), Some(1));
    assert!(!model.open);
    assert_eq!(view.open_generation(), 0);
    assert_eq!(view.runtime.return_focus, None);
    view.apply_open_generation(&mut model, 42, 1);
    assert!(model.open);
    assert_eq!(view.open_generation(), 1);
    assert_eq!(view.runtime.return_focus, Some(42));
    assert!(!view.runtime.focus_visible);
    assert!(view.runtime.placement.items.is_empty());
    model.open = false;
    view.show_with_model(&mut model, 43);
    assert!(model.open);
    assert_eq!(view.open_generation(), 2);
    assert_eq!(view.runtime.return_focus, Some(43));
    assert_eq!(view.next_open_generation(), Some(3));
}

#[test]
fn checked_menu_generation_exhaustion_has_no_open_or_focus_effects() {
    let mut view = MenuView::new_external();
    let mut model = Menu::new(2);
    view.runtime.open_generation = u64::MAX;
    assert_eq!(view.next_open_generation(), None);
    assert!(!model.open);
    assert_eq!(view.runtime.return_focus, None);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            view.show_with_model(&mut model, 42);
        }))
        .is_err()
    );
    assert!(!model.open);
    assert_eq!(view.open_generation(), u64::MAX);
    assert_eq!(view.runtime.return_focus, None);
}

#[test]
fn checked_menu_release_is_read_only_and_commits_original_action_once() {
    let mut view = MenuView::new(2);
    view.show(42);
    let layout = render_menu(&mut view, &["One", "Two"]);
    let point = layout.items[1].frame.center();
    assert_eq!(view.pointer_release_index(&view.model, point), None);
    assert_eq!(view.pointer_down_at(point), MenuEvent::None);
    assert!(view.pointer_armed());
    let generation = view.open_generation();
    assert_eq!(view.pointer_release_index(&view.model, point), Some(1));
    assert_eq!(view.pointer_release_index(&view.model, point), Some(1));
    assert!(view.pointer_armed());
    assert!(view.is_open());
    assert_eq!(view.open_generation(), generation);
    assert_eq!(view.menu().last_action, None);
    assert_eq!(
        view.pointer_up_at(point),
        MenuEvent::Activated {
            index: 1,
            restore_focus: Some(42)
        }
    );
    assert_eq!(view.pointer_release_index(&view.model, point), None);
    assert_eq!(view.pointer_up_at(point), MenuEvent::None);
    assert_eq!(view.menu().last_action, Some(1));
    view.show(43);
    assert!(!view.pointer_armed());
    assert_eq!(view.pointer_release_index(&view.model, point), None);
}

#[test]
fn checked_menu_cancel_retires_pointer_and_scroll_but_preserves_space_arm() {
    let mut view = MenuView::new(2);
    view.show(42);
    let layout = render_menu(&mut view, &["One", "Two"]);
    let point = layout.items[0].frame.center();
    view.pointer_down_at(point);
    assert!(view.pointer_armed());
    view.runtime.scroll_gesture_captured = true;
    view.cancel_pointer_arm();
    assert!(!view.pointer_armed());
    assert!(!view.scroll_gesture_captured());
    assert_eq!(view.pointer_release_index(&view.model, point), None);
    assert_eq!(view.pointer_up_at(point), MenuEvent::None);
    assert!(view.is_open());
    // Menu's public Space-down activates eagerly (covered by the original
    // menu_keyboard_focus_syncs_before_enter_and_escape_works_before_render).
    // Establish the original Button's held Space arm to test pointer-only
    // cancellation; do not invent a second Menu keyboard policy.
    assert!(!view.runtime.buttons[0].key(ControlKey::Space, true));
    assert!(view.runtime.buttons[0].state.pressed);
    assert!(!view.pointer_armed());
    view.cancel_pointer_arm();
    assert!(view.runtime.buttons[0].state.pressed);
    assert!(!view.pointer_armed());
    assert_eq!(
        view.key(ControlKey::Space, false),
        MenuEvent::Activated {
            index: 0,
            restore_focus: Some(42)
        }
    );
    assert_eq!(view.key(ControlKey::Space, false), MenuEvent::None);
}

#[test]
fn checked_menu_release_rejects_disabled_and_inactive_original_rows() {
    let mut view = MenuView::new(2);
    view.show(42);
    let layout = render_menu(&mut view, &["One", "Two"]);
    let point = layout.items[0].frame.center();
    view.pointer_down_at(point);
    view.set_row_styles(vec![MenuRowStyle {
        disabled: true,
        ..Default::default()
    }]);
    assert_eq!(view.pointer_release_index(&view.model, point), None);
    assert_eq!(view.pointer_up_at(point), MenuEvent::None);
    view.set_row_styles(Vec::new());
    view.pointer_down_at(point);
    assert_eq!(view.pointer_release_index(&view.model, point), Some(0));
    view.set_window_active(false);
    assert!(!view.pointer_armed());
    assert_eq!(view.pointer_release_index(&view.model, point), None);
    assert_eq!(view.pointer_up_at(point), MenuEvent::None);
}

#[test]
fn menu_surface_uses_ancestor_clip_and_rows_keep_popup_clip() {
    for ancestor in [
        Rect::new(-20., -20., 400., 300.),
        Rect::new(30., 20., 90., 100.),
    ] {
        let mut view = MenuView::new(2);
        view.show(0);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut FixedText,
                &["First", "Second"],
                Rect::new(20., 20., 100., 32.),
                Rect::new(0., 0., 320., 240.),
                Some(ancestor),
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("menu layout");
        let visible = layout.visible.expect("menu visible");
        let primitives = scene.primitives();
        assert!(
            matches!(primitives.first(), Some(tekes_ui_scene::Primitive::PushClip(r)) if *r == ancestor)
        );
        let surface_index = primitives
            .iter()
            .position(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::Quad(q) if q.rect == layout.popup)
            })
            .expect("popup surface remains present after shadow layers");
        let pop_index = primitives[surface_index + 1..]
            .iter()
            .position(|primitive| matches!(primitive, tekes_ui_scene::Primitive::PopClip))
            .map(|offset| surface_index + 1 + offset)
            .expect("popup surface clip is closed");
        assert!(primitives[pop_index + 1..].iter().any(
            |primitive| matches!(primitive, tekes_ui_scene::Primitive::PushClip(r) if *r == visible)
        ));
    }
}

#[test]
fn keyboard_menu_reveals_focus_ring_inside_popup_padding() {
    let labels = [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
    ];
    let mut view = MenuView::new(labels.len());
    view.show(1);
    let mut scene = Scene::new();
    let _ = render_menu(&mut view, &labels);
    assert_eq!(view.key(ControlKey::End, true), MenuEvent::None);
    let layout = view
        .render(
            &mut scene,
            &mut FixedText,
            &labels,
            Rect::new(12.0, 12.0, 120.0, 34.0),
            Rect::new(0.0, 0.0, 120.0, 260.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open keyboard menu layout");
    let focused = layout.items.last().expect("last row");
    let ring_width = proposal::metric("stroke", "c-focus-width").unwrap();
    let ring_gap = proposal::metric("spacing", "c-focus-offset").unwrap();
    let ring_outset = ring_width + ring_gap;
    assert!(layout.items.windows(2).all(|rows| {
        (rows[1].frame.min_y() - rows[0].frame.max_y() - menu_row_gap()).abs() < 0.001
    }));
    assert!(layout.scroll_offset > 0.0);
    assert!(focused.focus_bounds().min_y() >= layout.popup.min_y() + 1.0);
    assert!(focused.focus_bounds().max_y() <= layout.popup.max_y() - 1.0);
    let last_row_quad = scene
        .primitives()
        .iter()
        .rposition(|primitive| {
            matches!(primitive, tekes_ui_scene::Primitive::Quad(q)
                if q.rect.size.width == focused.frame.size.width)
        })
        .expect("row quads present");
    let focus_quad = scene
        .primitives()
        .iter()
        .position(|primitive| {
            matches!(primitive, tekes_ui_scene::Primitive::Quad(q)
                if (q.rect.min_y() - (focused.frame.min_y() - ring_outset)).abs() < 0.01
                    && q.rect.size.width > focused.frame.size.width)
        })
        .expect("keyboard-selected menu row paints its focus ring");
    assert!(
        focus_quad > last_row_quad,
        "menu keyboard focus ring must paint after all sibling rows"
    );
}

#[test]
fn menu_rows_use_selected_fill_without_strokes_and_keep_popup_and_keyboard_focus() {
    for theme in Theme::ALL {
        for scale in [1., 2.] {
            for phase in 0..4 {
                let palette = Palette::from(theme);
                let mut text = ScaledText(scale);
                let mut view = MenuView::new(2);
                view.show(42);
                let mut scene = Scene::new();
                let anchor = Rect::new(20., 20., 100., 32.);
                let bounds = Rect::new(0., 0., 400., 320.);
                let layout = view
                    .render(
                        &mut scene,
                        &mut text,
                        &["First", "Second"],
                        anchor,
                        bounds,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    )
                    .unwrap();
                match phase {
                    0 => {
                        view.key(ControlKey::Down, true);
                    }
                    1 => view.pointer_move_at(layout.items[1].frame.center()),
                    2 => {
                        view.pointer_move_at(layout.items[1].frame.center());
                        view.pointer_down_at(layout.items[1].frame.center());
                    }
                    _ => view.set_disabled(true),
                }
                let mut scene = Scene::new();
                let layout = view
                    .render(
                        &mut scene,
                        &mut text,
                        &["First", "Second"],
                        anchor,
                        bounds,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    )
                    .unwrap();
                let quads: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|primitive| match primitive {
                        tekes_ui_scene::Primitive::Quad(quad) => Some(quad),
                        _ => None,
                    })
                    .collect();
                let popup = quads.iter().find(|quad| quad.rect == layout.popup).unwrap();
                assert_eq!(
                    popup.border_width,
                    paint::decorative_border(palette, scale).0
                );
                for item in &layout.items {
                    let row: Vec<_> = quads
                        .iter()
                        .filter(|quad| quad.rect == item.frame)
                        .collect();
                    assert_eq!(row.len(), 1, "one original menu row ground");
                    assert_eq!(
                        row[0].border_width, 0.,
                        "menu item strokes are absent in every state"
                    );
                }
                let selected = &layout.items[layout.active_item];
                let row = quads
                    .iter()
                    .find(|quad| quad.rect == selected.frame)
                    .unwrap();
                let fill = match phase {
                    1 => "row-selection-hover",
                    2 => "button-pressed",
                    _ => "row-selection",
                };
                assert_eq!(row.fill, palette.color(fill).unwrap());
                if phase == 0 {
                    let outset = proposal::metric("stroke", "c-focus-width").unwrap()
                        + proposal::metric("spacing", "c-focus-offset").unwrap();
                    assert_eq!(
                        quads
                            .iter()
                            .filter(|quad| quad.rect == selected.frame.inset(-outset)
                                && quad.border_color == palette.color("border-focused").unwrap())
                            .count(),
                        1,
                        "keyboard highlight keeps one independent focus ring"
                    );
                }
            }
        }
    }
}

#[test]
fn menu_row_measurement_uses_actual_focus_safe_inner_width() {
    let inset = menu_row_horizontal_inset(&mut FixedText, proposal::TextRole::UiBody);
    let current_label_width = 120.0 - menu_popup_inset() * 2.0 - inset * 2.0;
    let mut view = MenuView::new(1);
    view.show(1);
    let mut scene = Scene::new();
    let layout = view
        .render(
            &mut scene,
            &mut WrappingText {
                wraps_below: current_label_width + 0.5,
            },
            &["abcdefghijklmnopq"],
            Rect::new(12.0, 12.0, 120.0, 34.0),
            Rect::new(0.0, 0.0, 120.0, 260.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open menu layout");
    assert_eq!(layout.items[0].frame.size.height, 52.0);
    assert_eq!(layout.items[0].content.size.width, current_label_width);
}

#[test]
fn short_menu_shrinks_to_its_widest_button_row_and_focus_gutters() {
    let labels = ["System", "Light", "Dark"];
    let max_width = 420.0;
    let inset = menu_popup_inset();
    let max_inner_width = (MENU_MAX_WIDTH - inset * 2.0).max(0.0);
    let mut text = FixedText;
    let horiz = menu_row_horizontal_inset(&mut text, proposal::TextRole::UiBody);
    let preferred_row_width = labels
        .iter()
        .map(|label| {
            ButtonModel::measure_with_insets(
                &mut text,
                label,
                max_inner_width,
                menu_size_role(false),
                proposal::TextRole::UiBody,
                Some((horiz, horiz)),
            )
            .width
        })
        .fold(0.0, f32::max);
    let expected_width = preferred_row_width + inset * 2.0;
    let measured = MenuView::measure(
        &mut FixedText,
        &labels,
        max_width,
        proposal::TextRole::UiBody,
        false,
    );

    assert_eq!(measured.width, expected_width);
    assert!(measured.width < MENU_MAX_WIDTH);

    let mut view = MenuView::new(labels.len());
    view.show(0);
    let layout = view
        .render(
            &mut Scene::new(),
            &mut FixedText,
            &labels,
            Rect::new(20.0, 20.0, 80.0, 34.0),
            Rect::new(0.0, 0.0, max_width, 300.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open menu layout");
    assert_eq!(layout.popup.size.width, measured.width);
}

#[test]
fn menu_natural_width_includes_checked_and_submenu_indicator_columns() {
    let labels = ["Choice"];
    let width = |style: Option<MenuRowStyle>| {
        let mut view = MenuView::new(1);
        if let Some(style) = style {
            view.set_row_styles(vec![style]);
        }
        view.show(0);
        view.render(
            &mut Scene::new(),
            &mut FixedText,
            &labels,
            Rect::new(20.0, 20.0, 80.0, 34.0),
            Rect::new(0.0, 0.0, 360.0, 300.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open menu layout")
        .popup
        .size
        .width
    };
    let plain = width(None);
    let checked = width(Some(MenuRowStyle {
        checked: Some(false),
        ..Default::default()
    }));
    let submenu = width(Some(MenuRowStyle {
        submenu: true,
        ..Default::default()
    }));
    let both = width(Some(MenuRowStyle {
        checked: Some(false),
        submenu: true,
        ..Default::default()
    }));

    assert!(checked > plain);
    assert!(submenu > plain);
    assert!(both > checked && both > submenu);
    assert!(both < MENU_MAX_WIDTH);
}

#[test]
fn menu_header_keeps_its_allocated_width_when_rows_are_short() {
    let inset = menu_popup_inset();
    assert_eq!(
        MenuView::header_content_width(&mut FixedText, 420.0),
        MENU_MAX_WIDTH - inset * 2.0,
        "header width is a content-space upper bound, including for empty rows"
    );

    let header = Size::new(180.0, 32.0);
    let anchor = Rect::new(20.0, 20.0, 80.0, 34.0);
    let viewport = Rect::new(0.0, 0.0, 360.0, 320.0);
    let frame = MenuView::header_frame(
        &mut FixedText,
        &["A"],
        header,
        anchor,
        viewport,
        proposal::TextRole::UiBody,
        false,
    );
    assert_eq!(frame.size.width, header.width);

    let mut view = MenuView::new_external();
    let mut model = Menu::new(1);
    model.open = true;
    let mut scene = Scene::new();
    let mut painted_header = None;
    let mut paint_header = |_: &mut Scene, _: &mut dyn ButtonTextRenderer, frame: Rect| {
        painted_header = Some(frame);
    };
    let layout = view
        .render_with_header(
            &mut model,
            &["A"],
            MenuRenderContext {
                scene: &mut scene,
                text: &mut FixedText,
                anchor,
                visible: viewport,
                clip: Some(viewport),
                palette: palette(),
                role: proposal::TextRole::UiBody,
                compact: false,
            },
            header,
            &mut paint_header,
        )
        .expect("header menu layout");
    assert_eq!(layout.popup.size.width, header.width + inset * 2.0);
    assert_eq!(painted_header.unwrap().size.width, header.width);
}

#[test]
fn wider_header_remeasures_long_menu_row_at_final_popup_width() {
    let label = "Long option label for Theme";
    let wraps_below = 200.0;
    let header = Size::new(250.0, 32.0);
    let anchor = Rect::new(20.0, 20.0, 80.0, 34.0);
    let viewport = Rect::new(0.0, 0.0, 360.0, 320.0);
    let inset = menu_popup_inset();

    let mut natural_text = IntrinsicWrappingText { wraps_below };
    let natural = MenuView::measure(
        &mut natural_text,
        &[label],
        viewport.size.width,
        proposal::TextRole::UiBody,
        false,
    );
    let horiz = menu_row_horizontal_inset(&mut natural_text, proposal::TextRole::UiBody);
    assert!(natural.width - inset * 2.0 < header.width);
    assert!(natural.width - inset * 2.0 - horiz * 2.0 < wraps_below);
    let natural_row_height = ButtonModel::measure_with_insets(
        &mut natural_text,
        label,
        natural.width - inset * 2.0,
        menu_size_role(false),
        proposal::TextRole::UiBody,
        Some((horiz, horiz)),
    )
    .height
    .max(menu_min_row_height(false));

    let mut text = IntrinsicWrappingText { wraps_below };
    let mut view = MenuView::new_external();
    let mut model = Menu::new(1);
    model.open = true;
    let mut scene = Scene::new();
    let mut paint_header = |_: &mut Scene, _: &mut dyn ButtonTextRenderer, _: Rect| {};
    let layout = view
        .render_with_header(
            &mut model,
            &[label],
            MenuRenderContext {
                scene: &mut scene,
                text: &mut text,
                anchor,
                visible: viewport,
                clip: Some(viewport),
                palette: palette(),
                role: proposal::TextRole::UiBody,
                compact: false,
            },
            header,
            &mut paint_header,
        )
        .expect("header menu layout");

    let final_inner_width = (layout.popup.size.width - inset * 2.0).max(0.0);
    let expected_height = ButtonModel::measure_with_insets(
        &mut text,
        label,
        final_inner_width,
        menu_size_role(false),
        proposal::TextRole::UiBody,
        Some((horiz, horiz)),
    )
    .height
    .max(menu_min_row_height(false));
    assert!(
        natural_row_height > expected_height,
        "label wraps at natural width"
    );
    assert_eq!(layout.popup.size.width, header.width + inset * 2.0);
    assert_eq!(layout.items[0].frame.size.height, expected_height);
    assert_eq!(layout.items[0].frame.size.height, 32.0);
}

#[test]
fn tabs_focus_ring_uses_ancestor_clip_not_tabs_frame() {
    let mut tabs = TabsView::new(1);
    assert!(tabs.focus(0, true));
    let ancestor = Rect::new(-10.0, 0.0, 140.0, 80.0);
    let mut scene = Scene::new();
    let layout = tabs.render(
        &mut scene,
        &mut FixedText,
        &["Tab"],
        Rect::new(10.0, 20.0, 100.0, 30.0),
        Some(ancestor),
        palette(),
        proposal::TextRole::UiBody,
        false,
    );
    let item = layout.items.first().unwrap();
    let outset = proposal::metric("stroke", "c-focus-width").unwrap()
        + proposal::metric("spacing", "c-focus-offset").unwrap();
    let expected = item.frame.inset(-outset);
    assert!(
        scene.primitives().iter().any(|primitive| {
            matches!(primitive, tekes_ui_scene::Primitive::Quad(q) if q.rect == expected)
        }),
        "ring extends beyond the tab's own frame"
    );
    assert!(
        scene.primitives().iter().any(|primitive| {
            matches!(primitive, tekes_ui_scene::Primitive::PushClip(clip) if *clip == ancestor)
        }),
        "only the real ancestor viewport clips the ring"
    );
}

#[test]
fn dialog_actions_use_design_system_centered_content_alignment() {
    let mut dialog = DialogView::default();
    dialog.show(11);
    assert_eq!(
        dialog.key(ControlKey::Tab { reverse: false }, true),
        DialogEvent::Blocked
    );
    let mut text = RecordingText::default();
    let mut scene = Scene::new();
    let layout = dialog
        .render(
            &mut scene,
            &mut text,
            "Delete this item?",
            Rect::new(0.0, 0.0, 600.0, 420.0),
            Some(Rect::new(0.0, 0.0, 600.0, 420.0)),
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open dialog layout");
    assert_eq!(layout.focus, 1);
    let logical_focus_outset = proposal::metric("stroke", "c-focus-width").unwrap()
        + proposal::metric("spacing", "c-focus-offset").unwrap();
    let action_gap = layout.buttons[1].frame.min_x() - layout.buttons[0].frame.max_x();
    assert_eq!(action_gap, tekes_ui_tokens::Spacing::Regular.value());
    assert!(
        action_gap + f32::EPSILON >= 2.0 * logical_focus_outset,
        "dialog action focus rings need non-overlapping clearance"
    );
    for action in ["Cancel", "Confirm"] {
        assert_eq!(
            text.draws
                .iter()
                .find(|(label, _, _)| label == action)
                .unwrap()
                .2,
            ButtonTextAlign::default(),
            "dialog action {action} follows Button token alignment"
        );
    }
    let focused = layout.buttons[1];
    let outset = proposal::metric("stroke", "c-focus-width").unwrap()
        + proposal::metric("spacing", "c-focus-offset").unwrap();
    let ring = focused.frame.inset(-outset);
    assert!(
        scene.primitives().iter().any(|primitive| {
            matches!(primitive, tekes_ui_scene::Primitive::Quad(q) if q.rect == ring)
        }),
        "dialog button focus ring is drawn outside the action frame"
    );
}

#[test]
fn tabs_focus_arrow_and_space_use_the_new_focus_without_a_render() {
    let mut view = TabsView::new(3);
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut FixedText,
        &["One", "Two", "Three"],
        Rect::new(0.0, 0.0, 300.0, 34.0),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    );
    let lane_gap = layout.items[1].frame.min_x() - layout.items[0].frame.max_x();
    assert!(lane_gap + f32::EPSILON >= focus_reveal_outset(1.0).max(4.0));
    assert!(view.focus(0, true));
    assert_eq!(view.key(ControlKey::Right, true), Some(1));
    assert_eq!(view.selected(), 1);
    assert_eq!(view.key(ControlKey::Space, true), None);
    assert_eq!(view.key(ControlKey::Space, false), Some(1));
    assert!(layout.items[1].visible.is_some());
}

#[test]
fn tabs_cancel_space_when_focus_moves_before_key_up() {
    let mut view = TabsView::new(2);
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut FixedText,
        &["One", "Two"],
        Rect::new(0.0, 0.0, 200.0, 34.0),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    );
    view.focus(0, true);
    view.key(ControlKey::Space, true);
    view.key(ControlKey::Right, true);
    assert_eq!(view.key(ControlKey::Space, false), None);
    assert_eq!(view.selected(), 1);

    view.key(ControlKey::Space, true);
    view.clear_focus();
    assert_eq!(view.key(ControlKey::Space, false), None);
    assert_eq!(view.selected(), 1);
}

#[test]
fn menu_geometry_converges_row_heights_zero_gap_separator_and_concentric_radii() {
    struct CanonicalText(f32);
    impl ButtonTextRenderer for CanonicalText {
        fn backing_scale(&self) -> f32 {
            self.0
        }
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            let width = (text.chars().count() as f32 * 7.0).min(max_width.max(0.0));
            Size::new(width, role.style().line_height)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }
    let labels = ["Cut", "Copy", "Divider", "Paste"];
    for scale in [1.0, 2.0] {
        for (compact, expected_height) in [(false, 28.0), (true, 24.0)] {
            let mut view = MenuView::new(labels.len());
            view.set_row_styles(vec![
                MenuRowStyle::default(),
                MenuRowStyle::default(),
                MenuRowStyle {
                    separator: true,
                    ..Default::default()
                },
                MenuRowStyle::default(),
            ]);
            view.show(1);
            let mut scene = Scene::new();
            let layout = view
                .render(
                    &mut scene,
                    &mut CanonicalText(scale),
                    &labels,
                    Rect::new(12.0, 12.0, 120.0, 32.0),
                    Rect::new(0.0, 0.0, 320.0, 320.0),
                    None,
                    palette(),
                    proposal::TextRole::UiBody,
                    compact,
                )
                .expect("menu layout");
            assert_eq!(layout.items[0].frame.size.height, expected_height);
            assert_eq!(layout.items[1].frame.size.height, expected_height);
            assert_eq!(layout.items[2].frame.size.height, 9.0);
            assert_eq!(layout.items[3].frame.size.height, expected_height);
            assert_eq!(layout.items[0].frame.min_y() - layout.popup.min_y(), 6.0);
            assert_eq!(
                layout.items[1].frame.min_y() - layout.items[0].frame.max_y(),
                0.0
            );
            assert_eq!(
                layout.items[2].frame.min_y() - layout.items[1].frame.max_y(),
                0.0
            );
            assert_eq!(
                layout.items[3].frame.min_y() - layout.items[2].frame.max_y(),
                0.0
            );
            let row_frame = layout.items[0].frame;
            assert!(scene.primitives().iter().any(|primitive| {
                matches!(
                    primitive,
                    tekes_ui_scene::Primitive::Quad(q)
                        if q.rect == row_frame && q.corner_radii == tekes_ui_scene::CornerRadii::uniform(6.0)
                )
            }));
        }
    }
}

#[test]
fn select_combobox_and_command_palette_popup_widths_and_compact_apis() {
    use crate::composition::{
        Button, ComboBoxContent, CommandPaletteContent, ComponentId, Content, MenuContent,
        SelectContent, TextFieldContent,
    };
    use crate::layout_view::{ComboBoxView, CommandPaletteView, SelectView};
    struct CanonicalText;
    impl ButtonTextRenderer for CanonicalText {
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            let width = (text.chars().count() as f32 * 7.0).min(max_width.max(0.0));
            Size::new(width, role.style().line_height)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }

    let mut plain = MenuContent::new("File", vec!["Open".into()]).with_compact(true);
    assert!(plain.compact());
    plain.set_compact(false);
    assert!(!plain.compact());

    let mut select = SelectContent::new("Choice", vec!["Alpha".into(), "Beta".into()]);
    assert!(!select.compact());
    select.set_compact(true);
    assert!(select.compact());

    let mut combo =
        ComboBoxContent::new("Combo", vec!["One".into(), "Two".into()]).with_compact(true);
    assert!(combo.compact());
    combo.set_compact(false);
    assert!(!combo.compact());

    let mut cmd = CommandPaletteContent::new(
        Content::new(ComponentId(101), Button::new("Run")),
        Content::new(ComponentId(102), TextFieldContent::new("query", "")),
        crate::extended::CommandPalette {
            combo: crate::extended::ComboBox {
                items: vec!["Long Command Name".into()],
                ..Default::default()
            },
        },
    )
    .with_compact(true);
    assert!(cmd.compact());
    cmd.set_compact(false);
    assert!(!cmd.compact());

    // Select with wide anchor matches anchor width even beyond MENU_MAX_WIDTH.
    let mut select_view = SelectView::new(2);
    let mut select_model = crate::Select::new(2);
    select_model.menu.open = true;
    let mut scene = Scene::new();
    let wide_trigger = Rect::new(20.0, 20.0, 340.0, 32.0);
    let viewport = Rect::new(0.0, 0.0, 800.0, 400.0);
    select_view.render_trigger(
        &mut scene,
        &mut CanonicalText,
        &mut select_model,
        &["A", "B"],
        wide_trigger,
        Some(viewport),
        palette(),
        proposal::TextRole::UiBody,
    );
    let select_popup = select_view
        .render_options(
            &mut scene,
            &mut CanonicalText,
            &mut select_model,
            &["A", "B"],
            viewport,
            Some(viewport),
            palette(),
            proposal::TextRole::UiBody,
            true,
        )
        .expect("wide select popup");
    assert_eq!(select_popup.popup.size.width, 340.0);
    assert_eq!(select_popup.items[0].frame.size.height, 24.0);

    // ComboBox with wide anchor matches anchor width and supports compact rows.
    let mut combo_view = ComboBoxView::default();
    let mut combo_model = crate::extended::ComboBox {
        items: vec!["Alpha".into(), "Beta".into()],
        open: true,
        ..Default::default()
    };
    let combo_popup = combo_view
        .render(
            &mut scene,
            &mut CanonicalText,
            &mut combo_model,
            wide_trigger,
            viewport,
            Some(viewport),
            palette(),
            proposal::TextRole::UiBody,
            true,
        )
        .expect("wide combobox popup");
    assert_eq!(combo_popup.popup.size.width, 340.0);
    assert_eq!(combo_popup.items[0].frame.size.height, 24.0);

    // CommandPalette allows widths above MENU_MAX_WIDTH up to 560pt.
    let cmd_width = MenuView::command_header_content_width(&mut CanonicalText, 800.0);
    assert_eq!(cmd_width, 560.0 - 12.0);
    let mut cmd_view = CommandPaletteView::default();
    let mut cmd_model = crate::extended::CommandPalette {
        combo: crate::extended::ComboBox {
            items: vec!["Run Build".into(), "Run Tests".into()],
            open: true,
            ..Default::default()
        },
    };
    let wide_header = cmd_view.header_frame(
        &mut CanonicalText,
        &cmd_model,
        Size::new(400.0, 32.0),
        Rect::new(20.0, 20.0, 120.0, 32.0),
        viewport,
        true,
    );
    assert_eq!(wide_header.size.width, 400.0);
    assert!(wide_header.size.width > MENU_MAX_WIDTH);
    let mut paint_header = |_: &mut Scene, _: &mut dyn ButtonTextRenderer, _: Rect| {};
    let cmd_popup = cmd_view
        .render_with_header(
            MenuRenderContext {
                scene: &mut scene,
                text: &mut CanonicalText,
                anchor: Rect::new(20.0, 20.0, 120.0, 32.0),
                visible: viewport,
                clip: Some(viewport),
                palette: palette(),
                role: proposal::TextRole::UiBody,
                compact: true,
            },
            &mut cmd_model,
            (Size::new(400.0, 32.0), &mut paint_header),
        )
        .expect("wide command palette popup");
    assert_eq!(cmd_popup.popup.size.width, 400.0 + 12.0);
    assert_eq!(cmd_popup.items[0].frame.size.height, 24.0);
}
