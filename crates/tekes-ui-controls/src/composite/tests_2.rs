use super::menu::menu_popup_geometry;
use super::tests_1::{FixedText, ScaledText, TallText, palette, render_menu};
use super::*;
use tekes_ui_scene::{Point, Quad, Scene};
use tekes_ui_tokens::Palette;
use tekes_ui_tokens::{Theme, proposal};

#[test]
fn menu_keyboard_focus_syncs_before_enter_and_escape_works_before_render() {
    let mut view = MenuView::new(3);
    view.show(7);
    let layout = render_menu(&mut view, &["One", "Two", "Three"]);
    assert_eq!(view.key(ControlKey::Down, true), MenuEvent::None);
    assert_eq!(
        view.key(ControlKey::Enter, true),
        MenuEvent::Activated {
            index: 1,
            restore_focus: Some(7),
        }
    );
    assert!(layout.items[1].visible.is_some());

    let mut view = MenuView::new(2);
    view.show(4);
    assert_eq!(
        view.key(ControlKey::Escape, true),
        MenuEvent::Dismissed {
            restore_focus: Some(4),
        }
    );
    assert!(!view.is_open());

    let mut view = MenuView::new(2);
    view.show(6);
    render_menu(&mut view, &["One", "Two"]);
    assert_eq!(
        view.key(ControlKey::Space, true),
        MenuEvent::Activated {
            index: 0,
            restore_focus: Some(6),
        }
    );
    assert_eq!(view.key(ControlKey::Space, false), MenuEvent::None);
    assert_eq!(view.menu().last_action, Some(0));
}

#[test]
fn menu_keyboard_active_row_uses_button_focus_ring() {
    let mut view = MenuView::new(3);
    view.show(0);
    render_menu(&mut view, &["One", "Two", "Three"]);
    assert_eq!(view.key(ControlKey::Down, true), MenuEvent::None);
    render_menu(&mut view, &["One", "Two", "Three"]);

    let active = &view.runtime.buttons[1].state;
    assert!(active.selected);
    assert!(active.focused);
    assert!(active.focus_visible);
    assert!(!view.runtime.buttons[0].state.focus_visible);
}

#[test]
fn disabled_menu_still_closes_with_escape_or_outside_pointer() {
    let mut view = MenuView::new(2);
    view.show(3);
    render_menu(&mut view, &["One", "Two"]);
    view.set_disabled(true);
    assert_eq!(view.key(ControlKey::Enter, true), MenuEvent::None);
    assert_eq!(
        view.key(ControlKey::Escape, true),
        MenuEvent::Dismissed {
            restore_focus: Some(3),
        }
    );

    let mut view = MenuView::new(2);
    view.show(4);
    render_menu(&mut view, &["One", "Two"]);
    view.set_disabled(true);
    assert_eq!(
        view.pointer_down_at(Point::new(1.0, 1.0)),
        MenuEvent::Dismissed {
            restore_focus: Some(4),
        }
    );
}

#[test]
fn menu_hover_changes_active_action_and_precise_scroll_capture_survives_outside_point() {
    let mut view = MenuView::new(8);
    view.show(2);
    let layout = render_menu(
        &mut view,
        &[
            "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
        ],
    );
    let point = Point::new(
        layout.items[2].frame.center().x,
        layout.items[2].frame.center().y,
    );
    view.pointer_move_at(point);
    assert_eq!(view.menu().items.selected, 2);

    let inside = Point::new(layout.popup.center().x, layout.popup.center().y);
    assert_eq!(
        view.wheel_event(MenuWheelInput {
            point: inside,
            delta: Point::new(0.0, -18.0),
            precise: true,
            begin: true,
            time: 1.0,
            ..MenuWheelInput::default()
        }),
        MenuWheelEvent::Consumed { scrolled: true }
    );
    assert_eq!(
        view.wheel_event(MenuWheelInput {
            point: Point::new(400.0, 400.0),
            delta: Point::new(0.0, -12.0),
            precise: true,
            end: true,
            momentum: true,
            time: 1.02,
            ..MenuWheelInput::default()
        }),
        MenuWheelEvent::Consumed { scrolled: true }
    );
    assert_eq!(
        view.wheel_event(MenuWheelInput {
            point: Point::new(400.0, 400.0),
            precise: true,
            time: 1.04,
            ..MenuWheelInput::default()
        }),
        MenuWheelEvent::Ignored
    );
}

#[test]
fn menu_wheel_edge_rebound_advances_and_settles_through_owned_clock() {
    let labels = [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven",
        "Twelve",
    ];
    for delta in [-400.0, 400.0] {
        let mut view = MenuView::new(labels.len());
        view.show(1);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12., 12., 120., 34.),
                Rect::new(0., 0., 320., 180.),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        let inside = layout.popup.center();
        assert_eq!(
            view.wheel_event(MenuWheelInput {
                point: inside,
                delta: Point::new(0., delta),
                precise: true,
                begin: true,
                time: 1.0,
                ..MenuWheelInput::default()
            }),
            MenuWheelEvent::Consumed { scrolled: true }
        );
        assert_eq!(
            view.wheel_event(MenuWheelInput {
                point: Point::new(400., 400.),
                precise: true,
                end: true,
                momentum: true,
                time: 1.02,
                ..MenuWheelInput::default()
            }),
            MenuWheelEvent::Consumed { scrolled: false }
        );
        assert!(view.needs_animation(), "edge {delta} starts rebound");
        let bouncing = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12., 12., 120., 34.),
                Rect::new(0., 0., 320., 180.),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        assert_ne!(bouncing.scroll_offset, 0.0, "edge {delta} visibly bounces");

        for _ in 0..240 {
            if !view.needs_animation() {
                break;
            }
            view.tick(1.0 / 120.0);
        }
        assert!(!view.needs_animation(), "edge {delta} settles");
        let settled = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12., 12., 120., 34.),
                Rect::new(0., 0., 320., 180.),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        let popup_inset = focus_reveal_outset(1.0);
        let document_height = labels.len() as f32 * 36.0
            + labels.len().saturating_sub(1) as f32 * popup_inset.max(4.0);
        let expected_rest = if delta < 0.0 {
            0.0
        } else {
            (document_height - (layout.popup.size.height - popup_inset * 2.0)).max(0.0)
        };
        assert!(
            (settled.scroll_offset - expected_rest).abs() < 0.01,
            "edge {delta} settles at scroll limit {expected_rest}, got {}",
            settled.scroll_offset
        );
    }
}

#[test]
fn menu_reduced_motion_clears_open_menu_bounce_and_stops_frames() {
    let labels = [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
    ];
    let mut view = MenuView::new(labels.len());
    view.show(1);
    let mut scene = Scene::new();
    let layout = view
        .render(
            &mut scene,
            &mut FixedText,
            &labels,
            Rect::new(12., 12., 120., 34.),
            Rect::new(0., 0., 320., 180.),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .unwrap();
    let inside = layout.popup.center();
    view.wheel_event(MenuWheelInput {
        point: inside,
        delta: Point::new(0., -300.),
        precise: true,
        begin: true,
        time: 1.0,
        ..MenuWheelInput::default()
    });
    view.wheel_event(MenuWheelInput {
        point: Point::new(400., 400.),
        precise: true,
        end: true,
        momentum: true,
        time: 1.02,
        ..MenuWheelInput::default()
    });
    assert!(view.needs_animation());
    assert!(view.set_reduced_motion(true));
    assert!(!view.needs_animation());
    let cleared = view
        .render(
            &mut scene,
            &mut FixedText,
            &labels,
            Rect::new(12., 12., 120., 34.),
            Rect::new(0., 0., 320., 180.),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .unwrap();
    assert_eq!(cleared.scroll_offset, 0.0);

    // The preference remains active for a later edge gesture while the
    // menu stays open: wheel input is consumed, but no elastic frame runs.
    view.wheel_event(MenuWheelInput {
        point: cleared.popup.center(),
        delta: Point::new(0., -300.),
        precise: true,
        begin: true,
        time: 2.0,
        ..MenuWheelInput::default()
    });
    view.wheel_event(MenuWheelInput {
        point: Point::new(400., 400.),
        precise: true,
        end: true,
        momentum: true,
        time: 2.02,
        ..MenuWheelInput::default()
    });
    assert!(!view.needs_animation());
    let still = view
        .render(
            &mut scene,
            &mut FixedText,
            &labels,
            Rect::new(12., 12., 120., 34.),
            Rect::new(0., 0., 320., 180.),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .unwrap();
    assert_eq!(still.scroll_offset, 0.0);
}

#[test]
fn menu_hover_target_is_the_action_enter_activates() {
    let mut view = MenuView::new(3);
    view.show(1);
    let layout = render_menu(&mut view, &["One", "Two", "Three"]);
    let row = layout.items[2].frame;
    view.pointer_move_at(Point::new(row.center().x, row.center().y));
    assert_eq!(
        view.key(ControlKey::Enter, true),
        MenuEvent::Activated {
            index: 2,
            restore_focus: Some(1),
        }
    );
}

#[test]
fn dialog_tab_then_enter_confirms_without_render_and_escape_closes_unrendered_dialog() {
    let mut view = DialogView::default();
    view.show(9);
    let mut scene = Scene::new();
    let layout = view
        .render(
            &mut scene,
            &mut FixedText,
            "Confirm action?",
            Rect::new(0.0, 0.0, 600.0, 400.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open dialog layout");
    assert_eq!(
        view.key(ControlKey::Tab { reverse: false }, true),
        DialogEvent::Blocked
    );
    assert_eq!(
        view.key(ControlKey::Enter, true),
        DialogEvent::Confirmed {
            restore_focus: Some(9),
        }
    );
    assert!(layout.buttons.iter().all(|button| button.visible.is_some()));

    let mut view = DialogView::default();
    view.show(5);
    assert_eq!(
        view.key(ControlKey::Escape, true),
        DialogEvent::Cancelled {
            restore_focus: Some(5),
        }
    );
    assert!(!view.is_open());
}

#[test]
fn menu_and_dialog_surfaces_use_semantic_roles_and_renderer_scale() {
    for theme in Theme::ALL {
        for scale in [1.0, 2.0] {
            let palette = Palette::from(theme);
            let mut menu = MenuView::new(2);
            menu.show(1);
            let mut text = ScaledText(scale);
            let mut scene = Scene::new();
            let menu_layout = menu
                .render(
                    &mut scene,
                    &mut text,
                    &["First", "Second"],
                    Rect::new(20.0, 20.0, 100.0, 32.0),
                    Rect::new(0.0, 0.0, 320.0, 240.0),
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                    false,
                )
                .expect("open menu layout");
            let menu_quad = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == menu_layout.popup => Some(q),
                    _ => None,
                })
                .expect("menu surface quad");
            assert_eq!(menu_quad.fill, palette.color("surface-popover").unwrap());
            assert_eq!(
                menu_quad.corner_radii,
                tekes_ui_scene::CornerRadii::uniform(
                    proposal::metric("radius", "radius-floating").unwrap(),
                )
            );
            assert_eq!(
                menu_quad.border_width,
                paint::decorative_border(palette, scale).0
            );

            let mut dialog = DialogView::default();
            dialog.show(2);
            let mut dialog_scene = Scene::new();
            let dialog_layout = dialog
                .render(
                    &mut dialog_scene,
                    &mut text,
                    "Confirm?",
                    Rect::new(0.0, 0.0, 600.0, 420.0),
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                    false,
                )
                .expect("open dialog layout");
            let dialog_quad = dialog_scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == dialog_layout.modal => Some(q),
                    _ => None,
                })
                .expect("dialog surface quad");
            assert_eq!(dialog_quad.fill, palette.color("surface-popover").unwrap());
            assert_eq!(
                dialog_quad.corner_radii,
                tekes_ui_scene::CornerRadii::uniform(
                    proposal::metric("radius", "c-dialog-radius").unwrap(),
                )
            );
            assert_eq!(
                dialog_quad.border_width,
                paint::decorative_border(palette, scale).0
            );
        }
    }
}

#[test]
fn dialog_inactive_input_is_ignored_but_explicit_cancel_closes() {
    let mut view = DialogView::default();
    view.show(3);
    assert!(view.blocks_background());
    view.deactivate();
    assert!(!view.blocks_background());
    assert_eq!(view.key(ControlKey::Escape, true), DialogEvent::None);
    assert_eq!(
        view.cancel(),
        DialogEvent::Cancelled {
            restore_focus: Some(3),
        }
    );
    assert!(!view.is_open());
}

#[test]
fn disabled_dialog_still_closes_with_escape_but_rejects_activation() {
    let mut view = DialogView::default();
    view.show(8);
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut FixedText,
        "Confirm?",
        Rect::new(0.0, 0.0, 600.0, 400.0),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    );
    view.set_disabled(true);
    assert_eq!(view.key(ControlKey::Enter, true), DialogEvent::None);
    assert!(view.is_open());
    assert_eq!(
        view.key(ControlKey::Escape, true),
        DialogEvent::Cancelled {
            restore_focus: Some(8),
        }
    );
    assert!(!view.is_open());
}

#[test]
fn open_dialog_blocks_host_background_scroll_outside_modal_preview() {
    let mut view = DialogView::default();
    view.show(3);
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut FixedText,
        "Confirm?",
        Rect::new(100.0, 100.0, 400.0, 300.0),
        None,
        palette(),
        proposal::TextRole::UiBody,
        false,
    );

    let header_point = Point::new(20.0, 24.0);
    assert_eq!(view.wheel_at(header_point), DialogEvent::None);
    assert!(view.blocks_background());
    assert_eq!(
        view.wheel_at(Point::new(120.0, 120.0)),
        DialogEvent::Blocked
    );
}

#[test]
fn short_dialog_stacks_actions_without_overlapping_or_leaving_modal() {
    let mut view = DialogView::default();
    view.show(0);
    let mut scene = Scene::new();
    let layout = view
        .render(
            &mut scene,
            &mut FixedText,
            "Confirm?",
            Rect::new(0.0, 0.0, 180.0, 120.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open dialog layout");
    let [cancel, confirm] = layout.buttons;
    assert!(cancel.frame.max_y() <= confirm.frame.min_y());
    assert!(cancel.frame.min_y() >= layout.modal.min_y());
    assert!(confirm.frame.max_y() <= layout.modal.max_y());
}

#[test]
fn large_dialog_text_keeps_its_measure_and_actions_clip_to_separate_slots() {
    let mut view = DialogView::default();
    view.show(0);
    let mut scene = Scene::new();
    let layout = view
        .render(
            &mut scene,
            &mut TallText,
            "A very long title that needs several lines",
            Rect::new(0.0, 0.0, 80.0, 120.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open dialog layout");

    assert_eq!(layout.title.size.height, 100.0);
    let [cancel, confirm] = layout.buttons;
    let cancel_visible = cancel.visible.expect("cancel has a visible slot");
    let confirm_visible = confirm.visible.expect("confirm has a visible slot");
    assert!(
        cancel_visible.intersection(&confirm_visible).is_none(),
        "cancel={cancel_visible:?} confirm={confirm_visible:?}"
    );
    assert!(!cancel.hit_test(confirm_visible.center()));
    assert!(!confirm.hit_test(cancel_visible.center()));
}

#[cfg(test)]
mod overlay_presence_tests {
    use super::*;
    use crate::{
        ControlMotionContext,
        extended::Popover,
        feedback_view::{OverlayEvent, PopoverView},
    };
    use std::time::Duration;
    use tekes_ui_tokens::{Rgba, Theme};

    struct ClockText {
        millis: u64,
        reduced: bool,
        requested: bool,
    }
    impl ClockText {
        fn new() -> Self {
            Self {
                millis: 0,
                reduced: false,
                requested: false,
            }
        }
    }
    impl ButtonTextRenderer for ClockText {
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: Duration::from_millis(self.millis),
                reduced_motion: self.reduced,
            })
        }
        fn request_animation_frame(&mut self) {
            self.requested = true;
        }
        fn measure(&mut self, label: &str, _: proposal::TextRole, max: f32) -> Size {
            Size::new((label.len() as f32 * 7.0).min(max), 18.0)
        }
        fn draw(
            &mut self,
            scene: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            frame: Rect,
            _: ButtonTextAlign,
            color: Rgba,
        ) {
            scene.push_quad(Quad::new(frame, color));
        }
    }
    fn viewport() -> Rect {
        Rect::new(0.0, 0.0, 600.0, 400.0)
    }
    fn menu(view: &mut MenuView, text: &mut ClockText, scene: &mut Scene) -> Option<MenuLayout> {
        view.render(
            scene,
            text,
            &["Open", "Copy"],
            Rect::new(20.0, 20.0, 120.0, 36.0),
            viewport(),
            Some(viewport()),
            Palette::from(Theme::default()),
            proposal::TextRole::UiBody,
            false,
        )
    }
    fn dialog(
        view: &mut DialogView,
        text: &mut ClockText,
        scene: &mut Scene,
        clip: Option<Rect>,
    ) -> Option<DialogLayout> {
        view.render(
            scene,
            text,
            "Confirm changes?",
            viewport(),
            clip,
            Palette::from(Theme::default()),
            proposal::TextRole::UiBody,
            false,
        )
    }
    fn popover(
        view: &mut PopoverView,
        model: &Popover,
        text: &mut ClockText,
        scene: &mut Scene,
        palette: Palette,
    ) {
        view.render_with_text_content(
            scene,
            text,
            model,
            Rect::new(20.0, 20.0, 120.0, 36.0),
            Size::new(220.0, 120.0),
            viewport(),
            Some(viewport()),
            palette,
            proposal::TextRole::UiBody,
            "content-v1",
            |scene, _, bounds| scene.push_quad(Quad::new(bounds, Rgba::new(1.0, 0.0, 0.0, 1.0))),
        );
    }
    #[test]
    fn window_deactivation_clears_popover_presence_without_replaying_exit() {
        for closing in [false, true] {
            let mut view = PopoverView::default();
            let mut model = Popover::default();
            let mut text = ClockText::new();
            let mut scene = Scene::new();
            view.show(&mut model, 7);
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            text.millis = 40;
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            if closing {
                view.close(&mut model);
            }
            assert!(view.needs_animation());
            view.set_window_active(false, &mut model);
            assert!(!view.needs_animation());
            assert!(!view.is_presented());
            assert!(!model.open);
            text.requested = false;
            scene.clear();
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            assert!(scene.is_empty());
            assert!(!text.requested);
            view.set_window_active(true, &mut model);
            text.millis = 60;
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            assert!(scene.is_empty());
            assert!(!text.requested);
        }
    }
    #[test]
    fn filtered_menu_dismissal_keeps_exit_but_cancel_discards_presentation() {
        let mut view = crate::layout_view::FilteredMenuView::new();
        let mut text = ClockText::new();
        let mut scene = Scene::new();
        let items = vec!["Open".to_owned(), "Copy".to_owned()];
        let paint = |view: &mut crate::layout_view::FilteredMenuView,
                     text: &mut ClockText,
                     scene: &mut Scene,
                     open| {
            view.render(
                scene,
                text,
                &items,
                &[7, 8],
                Some(0),
                open,
                Rect::new(20., 20., 120., 36.),
                viewport(),
                Some(viewport()),
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            );
        };
        paint(&mut view, &mut text, &mut scene, true);
        text.millis = 40;
        paint(&mut view, &mut text, &mut scene, true);
        view.dismiss();
        assert!(!view.is_visible());
        assert!(view.layout().is_none());
        assert!(
            view.is_presented(),
            "ordinary filtered dismissal must not mean window deactivation"
        );
        scene.clear();
        paint(&mut view, &mut text, &mut scene, false);
        assert!(!scene.is_empty());
        assert_eq!(view.key(ControlKey::Enter, true), MenuEvent::None);
        view.cancel();
        assert!(!view.is_presented());
        assert!(!view.needs_animation());
    }
    #[test]
    fn ordinary_menu_deactivation_keeps_exit_until_window_is_inactive() {
        let mut view = MenuView::new(2);
        let mut text = ClockText::new();
        let mut scene = Scene::new();
        view.show(7);
        menu(&mut view, &mut text, &mut scene);
        text.millis = 40;
        menu(&mut view, &mut text, &mut scene);
        view.deactivate();
        assert!(!view.is_open());
        assert!(
            view.is_presented(),
            "ordinary focus return retains exit paint"
        );
        assert!(view.needs_animation());
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        assert!(!scene.is_empty());
        view.set_window_active(false);
        assert!(!view.is_presented());
        assert!(!view.needs_animation());
    }
    #[test]
    fn window_deactivation_stops_menu_presence_without_render_or_replay() {
        for closing in [false, true] {
            let mut view = MenuView::new(2);
            let mut text = ClockText::new();
            let mut scene = Scene::new();
            view.show(7);
            menu(&mut view, &mut text, &mut scene);
            text.millis = 40;
            menu(&mut view, &mut text, &mut scene);
            if closing {
                view.key(ControlKey::Escape, true);
            }
            assert!(view.needs_animation());
            view.set_window_active(false);
            assert!(
                !view.needs_animation(),
                "inactive menu stops pending presence, closing={closing}"
            );
            assert!(!view.is_presented());
            text.requested = false;
            scene.clear();
            menu(&mut view, &mut text, &mut scene);
            assert!(scene.is_empty());
            assert!(!text.requested);
            view.set_window_active(true);
            text.millis = 60;
            menu(&mut view, &mut text, &mut scene);
            assert!(!view.needs_animation());
            assert!(scene.is_empty());
        }
    }
    #[test]
    fn window_deactivation_settles_dialog_presence_to_its_logical_state() {
        for closing in [false, true] {
            let mut view = DialogView::default();
            let mut text = ClockText::new();
            let mut scene = Scene::new();
            view.show(7);
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            text.millis = 40;
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            if closing {
                view.close();
            }
            assert!(view.needs_animation());
            view.set_window_active(false);
            assert!(
                !view.needs_animation(),
                "inactive dialog stops pending presence, closing={closing}"
            );
            assert_eq!(view.dialog().open, !closing);
            text.requested = false;
            scene.clear();
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            assert_eq!(scene.is_empty(), closing);
            assert!(!text.requested);
            view.set_window_active(true);
            text.millis = 60;
            text.requested = false;
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            assert!(!view.needs_animation());
            assert!(!text.requested);
        }
    }
    #[test]
    fn menu_exit_keeps_only_paint_and_reopen_continues_current_progress() {
        let mut text = ClockText::new();
        let mut view = MenuView::new(2);
        let mut scene = Scene::new();
        view.show(8);
        assert!(menu(&mut view, &mut text, &mut scene).is_some());
        assert!(text.requested && view.needs_animation());
        text.millis = 200;
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        let resting = scene.clone();
        assert!(!resting.is_empty());
        assert!(matches!(
            view.key(ControlKey::Escape, true),
            MenuEvent::Dismissed {
                restore_focus: Some(8)
            }
        ));
        assert!(!view.is_open());
        scene.clear();
        assert!(menu(&mut view, &mut text, &mut scene).is_none());
        assert_eq!(scene, resting);
        assert!(view.is_presented());
        assert_eq!(
            view.pointer_down_at(Point::new(30.0, 80.0)),
            MenuEvent::None
        );
        text.millis = 240;
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        let exiting = scene.clone();
        assert!(!exiting.is_empty());
        assert_ne!(exiting, resting);
        view.show(9);
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        assert_eq!(scene, exiting);
        text.millis = 500;
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        assert_eq!(scene, resting);
        view.clear_presentation();
        assert!(!view.is_presented());
    }
    #[test]
    fn dialog_dismissal_restores_focus_once_and_reduced_motion_finishes_exit() {
        let mut text = ClockText::new();
        let mut view = DialogView::default();
        let mut scene = Scene::new();
        view.show(3);
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        text.millis = 200;
        scene.clear();
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        assert!(matches!(
            view.close(),
            DialogEvent::Cancelled {
                restore_focus: Some(3)
            }
        ));
        assert_eq!(view.close(), DialogEvent::None);
        assert!(!view.blocks_background());
        assert_eq!(view.key(ControlKey::Enter, true), DialogEvent::None);
        scene.clear();
        assert!(dialog(&mut view, &mut text, &mut scene, Some(viewport())).is_none());
        assert!(!scene.is_empty());
        assert!(view.needs_animation());
        text.reduced = true;
        scene.clear();
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        assert!(scene.is_empty());
        assert!(!view.is_presented());
        assert!(!view.needs_animation());
    }
    #[test]
    fn dialog_exit_is_cancelled_when_current_ancestor_clip_changes() {
        let mut text = ClockText::new();
        let mut view = DialogView::default();
        let mut scene = Scene::new();
        view.show(3);
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        text.millis = 200;
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        view.close();
        scene.clear();
        dialog(&mut view, &mut text, &mut scene, Some(Rect::default()));
        assert!(scene.is_empty());
        assert!(!view.is_presented());
    }
    #[test]
    fn popover_retains_content_but_no_focus_or_hits_and_cancels_stale_brand() {
        let mut text = ClockText::new();
        let mut view = PopoverView::default();
        let mut model = Popover::default();
        let mut scene = Scene::new();
        let palette = Palette::from(Theme::default());
        view.show(&mut model, 4);
        popover(&mut view, &model, &mut text, &mut scene, palette);
        text.millis = 200;
        scene.clear();
        popover(&mut view, &model, &mut text, &mut scene, palette);
        let resting = scene.clone();
        assert!(matches!(
            view.close(&mut model),
            OverlayEvent::Dismissed {
                restore_focus: Some(4)
            }
        ));
        assert!(view.focus_target().is_none());
        assert!(!view.pointer_down(&mut model, Point::new(30.0, 80.0)));
        scene.clear();
        popover(&mut view, &model, &mut text, &mut scene, palette);
        assert_eq!(scene, resting);
        assert!(view.is_presented());
        scene.clear();
        popover(
            &mut view,
            &model,
            &mut text,
            &mut scene,
            Palette::new(palette.theme, 1).expect("registered Gallery brand"),
        );
        assert!(scene.is_empty());
        assert!(!view.is_presented());
    }

    #[test]
    fn tabs_invalid_and_hidden_layout_cancel_focus_and_capture() {
        let mut view = TabsView::new(2);
        let mut scene = Scene::new();
        let mut text = ClockText::new();
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(20., 20., 240., 40.);
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let point = view.placement.items[0].frame.center();
        assert_eq!(view.pointer_down_at(point), Some(0));
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            Rect::new(f32::NAN, 0., 240., 40.),
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(view.pointer_up_at(point), None);
        assert_eq!(view.pointer_down_at(point), None);
        assert_eq!(view.key(ControlKey::Enter, true), None);
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(0, true);
        view.key(ControlKey::Space, true);
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            frame,
            Some(Rect::new(500., 500., 20., 20.)),
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(view.key(ControlKey::Space, false), None);
        assert_eq!(view.key(ControlKey::Right, true), None);
        assert_eq!(view.selected(), 0);
    }

    #[test]
    fn menu_popup_geometry_handles_negative_or_nan_viewport_without_panicking() {
        for vh in [-20.0, f32::NAN] {
            let (popup, _, _) = menu_popup_geometry(
                Rect::new(10.0, 20.0, 120.0, 28.0),
                Size::new(120.0, 60.0),
                Rect::new(0.0, 0.0, 320.0, vh),
                4.0,
                &[28.0, 28.0],
                false,
                None,
            );
            assert!(popup.size.height.is_finite() || vh.is_nan());
        }
    }

    #[test]
    fn dialog_view_follows_parent_width_unless_max_width_configured() {
        let mut text = ClockText::new();
        let preview = Rect::new(0., 0., 800., 500.);
        let mut scene = Scene::new();
        let palette = Palette::from(Theme::LIGHT);

        let mut unconstrained = DialogView::default();
        unconstrained.show(1);
        let wide = unconstrained
            .render(
                &mut scene,
                &mut text,
                "Confirm",
                preview,
                None,
                palette,
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        assert_eq!(wide.modal.size.width, 784.0);

        let mut capped = DialogView::default().with_max_width(Some(460.));
        capped.show(1);
        let bounded = capped
            .render(
                &mut scene,
                &mut text,
                "Confirm",
                preview,
                None,
                palette,
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        assert_eq!(bounded.modal.size.width, 460.0);
    }
}
