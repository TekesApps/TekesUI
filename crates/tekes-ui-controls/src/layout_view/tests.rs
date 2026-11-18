use super::disclosure_link::ButtonRuntime;
use super::*;
use crate::ButtonTextAlign;
use crate::composite::MenuEvent;
use crate::extended::{Breadcrumb, ComboBox, CommandPalette, Disclosure, Link, SegmentedControl};
use crate::widgets::ControlMotionContext;
use std::time::Duration;
use tekes_ui_scene::Size;
use tekes_ui_tokens::Theme;

struct FixedText;
impl ButtonTextRenderer for FixedText {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 8.).min(max_width),
            role.style().line_height,
        )
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: tekes_ui_tokens::Rgba,
    ) {
    }
}

#[derive(Default)]
struct DisclosureMotionText {
    now: Duration,
    reduced: bool,
    frames: usize,
}
impl ButtonTextRenderer for DisclosureMotionText {
    fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(max_width, 20.)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: self.now,
            reduced_motion: self.reduced,
        })
    }
    fn request_animation_frame(&mut self) {
        self.frames += 1;
    }
    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: ButtonTextAlign,
        _ink: tekes_ui_tokens::Rgba,
    ) {
    }
}

#[test]
fn shared_button_runtime_deactivation_clears_owned_motion_and_stale_hover() {
    let mut runtime = ButtonRuntime::default();
    let mut text = DisclosureMotionText::default();
    let mut scene = Scene::new();
    let frame = Rect::new(20., 20., 160., 40.);
    let render =
        |runtime: &mut ButtonRuntime, text: &mut DisclosureMotionText, scene: &mut Scene| {
            runtime.render(
                scene,
                text,
                "Action",
                frame,
                None,
                Palette::from(Theme::LIGHT),
                proposal::TextRole::UiBody,
                ButtonVariant::Quiet,
                None,
            );
        };
    render(&mut runtime, &mut text, &mut scene);
    runtime.pointer_move(frame.center());
    render(&mut runtime, &mut text, &mut scene);
    assert!(runtime.button.needs_animation());
    text.now = Duration::from_millis(50);
    render(&mut runtime, &mut text, &mut scene);
    runtime.active(false);
    assert!(!runtime.button.needs_animation());
    assert!(!runtime.button.state.hovered);
    runtime.pointer_move(frame.center());
    assert!(!runtime.pointer_up(frame.center()));
    runtime.active(true);
    render(&mut runtime, &mut text, &mut scene);
    assert!(!runtime.button.needs_animation());
    assert!(!runtime.button.state.hovered);
}

#[test]
fn disclosure_body_layout_animates_reversibly_and_respects_motion_and_clip() {
    let mut view = DisclosureView::default();
    let mut model = Disclosure::default();
    let mut text = DisclosureMotionText::default();
    let body = Rect::new(10., 50., 180., 100.);
    let clip = Some(Rect::new(0., 0., 200., 80.));

    let closed = view.layout_body(&mut text, &model, body, clip);
    assert_eq!(closed.occupied_height, 0.);
    assert!(!closed.interactive);

    text.now = Duration::from_millis(1);
    model.open = true;
    let start = view.layout_body(&mut text, &model, body, clip);
    assert_eq!(start.occupied_height, 0.);
    assert!(!start.interactive);

    text.now = Duration::from_millis(151);
    let middle = view.layout_body(&mut text, &model, body, clip);
    assert!(middle.progress > 0. && middle.progress < 1.);
    assert!(middle.interactive);
    assert_eq!(middle.body_rect, body);
    assert_eq!(
        middle.visible_clip.unwrap().size.height,
        middle.occupied_height.min(30.)
    );
    assert!(
        text.frames > 0,
        "active reveal should request another host frame"
    );

    model.open = false;
    text.now = Duration::from_millis(152);
    let reversed = view.layout_body(&mut text, &model, body, clip);
    assert!(reversed.progress > 0.);
    assert!(
        !reversed.interactive,
        "closing content becomes inert immediately"
    );
    text.now = Duration::from_millis(180);
    let collapsing = view.layout_body(&mut text, &model, body, clip);
    assert!(collapsing.progress < reversed.progress);

    model.open = true;
    text.reduced = true;
    text.now = Duration::from_millis(181);
    let reduced = view.layout_body(&mut text, &model, body, clip);
    assert_eq!(reduced.progress, 1.);

    let hidden_clip = Some(Rect::new(400., 400., 20., 20.));
    model.open = false;
    text.reduced = false;
    text.now = Duration::from_millis(182);
    let hidden = view.layout_body(&mut text, &model, body, hidden_clip);
    assert_eq!(hidden.progress, 0.);
    assert!(hidden.visible_clip.is_none());
    assert!(!hidden.interactive);
}

#[test]
fn icon_artboard_uses_json_center_alignment_without_changing_fitted_size() {
    #[derive(Default)]
    struct Capture(Option<Rect>);
    impl IconRenderer for Capture {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, frame: Rect, _: tekes_ui_tokens::Rgba) {
            self.0 = Some(frame);
        }
    }

    let bounds = Rect::new(10., 20., 80., 64.);
    let side = 24.;
    let expected = layout_measured_children(
        bounds,
        Axis::Horizontal,
        &[MeasuredChild::new(Size::new(side, side))],
        0.,
        proposal::layout::alignment(proposal::layout::AlignmentPreset::IconArtboard),
        AlignmentOverride::default(),
    )[0]
    .frame;
    let mut renderer = Capture::default();
    let mut scene = Scene::default();
    let layout = IconView::default().render(
        &mut scene,
        &mut renderer,
        "plus",
        bounds,
        side,
        None,
        Palette::from(Theme::LIGHT),
    );
    assert_eq!(layout.artboard, expected);
    assert_eq!(layout.artboard.size, Size::new(side, side));
    assert_eq!(layout.artboard.center(), bounds.center());
    assert_eq!(renderer.0, Some(expected));
}

#[test]
fn stack_and_grid_views_preserve_frames_and_reject_overflow_geometry() {
    let clip = Rect::new(10., 10., 20., 20.);
    let mut stack = StackView::new(Stack {
        axis: Axis::Horizontal,
        gap: 4.,
    });
    let l = stack
        .render(Rect::new(0., 0., 100., 30.), 2, Some(clip))
        .clone();
    assert_eq!(l.items[0].frame(), Rect::new(0., 0., 48., 30.));
    assert_eq!(
        l.items[0].visible_frame(),
        Some(Rect::new(10., 10., 20., 20.))
    );
    let mut grid = GridView::new(Grid::new(2));
    assert_eq!(
        grid.render(Rect::new(0., 0., 80., 80.), 3, 20., None)
            .items
            .len(),
        3
    );
    assert!(
        stack
            .render(Rect::new(f32::MAX, 0., f32::MAX, 30.), 2, None)
            .items
            .is_empty()
    );
    assert!(
        grid.render(
            Rect::new(0., 0., 80., 80.),
            3,
            20.,
            Some(Rect::new(f32::MAX, 0., f32::MAX, 20.))
        )
        .visible
        .is_none()
    );
}

#[test]
fn disclosure_space_down_is_consumed_and_release_toggles_once() {
    let mut view = DisclosureView::default();
    let mut model = Disclosure::default();
    view.focus(true);
    let down = view.key_event(ControlKey::Space, true, &mut model);
    assert!(down.consumed);
    assert!(!down.changed);
    assert!(!model.open);
    let up = view.key_event(ControlKey::Space, false, &mut model);
    assert!(up.consumed);
    assert!(up.changed);
    assert!(model.open);
    assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
}

#[test]
fn disclosure_pointer_activation_requires_visible_matching_press() {
    let mut scene = Scene::default();
    let mut view = DisclosureView::default();
    let mut model = Disclosure::default();
    let frame = Rect::new(12., 20., 260., 40.);
    let palette = Palette::from(Theme::LIGHT);
    let full_clip = Some(Rect::new(0., 0., 400., 300.));
    let mut text = FixedText;
    view.render(
        &mut scene,
        &mut text,
        &model,
        "Details",
        frame,
        full_clip,
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(!view.pointer_up(Point::new(30., 30.), &mut model));
    assert!(!model.open);
    assert!(view.pointer_down(Point::new(30., 30.)));
    assert!(view.pointer_up(Point::new(30., 30.), &mut model));
    assert!(model.open);

    assert!(view.pointer_down(Point::new(30., 30.)));
    let hidden = view.render(
        &mut scene,
        &mut text,
        &model,
        "Details",
        frame,
        Some(Rect::new(500., 500., 20., 20.)),
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(hidden.visible.is_none());
    assert!(!view.pointer_up(Point::new(30., 30.), &mut model));
    assert!(model.open);

    model.disabled = true;
    view.render(
        &mut scene,
        &mut text,
        &model,
        "Details",
        frame,
        full_clip,
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(!view.pointer_down(Point::new(30., 30.)));
    assert!(!view.pointer_up(Point::new(30., 30.), &mut model));
    assert!(model.open);
}

#[test]
fn disclosure_icon_slot_keeps_label_geometry_stable_across_state_and_text_scale() {
    struct ScaledText(f32);
    impl ButtonTextRenderer for ScaledText {
        fn measure(&mut self, text: &str, _: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 8. * self.0).min(max_width),
                18. * self.0,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    #[derive(Default)]
    struct CaptureIcon {
        name: String,
        frame: Rect,
        tint: Option<tekes_ui_tokens::Rgba>,
    }
    impl IconRenderer for CaptureIcon {
        fn draw_icon(
            &mut self,
            _: &mut Scene,
            canonical_name: &str,
            frame: Rect,
            tint: tekes_ui_tokens::Rgba,
        ) {
            self.name = canonical_name.to_owned();
            self.frame = frame;
            self.tint = Some(tint);
        }
    }

    let mut scene = Scene::default();
    let mut view = DisclosureView::default();
    let mut model = Disclosure::default();
    let frame = Rect::new(12., 20., 260., 40.);
    let clip = Some(Rect::new(0., 0., 400., 300.));
    for scale in [1.0, 1.25, 1.5] {
        let mut text = ScaledText(scale);
        let closed = view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            clip,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
        );
        let closed_icon_frame = view.icon_frame();
        let mut icon = CaptureIcon::default();
        view.render_icon(
            &mut scene,
            &mut icon,
            &model,
            clip,
            Palette::from(Theme::LIGHT),
            16.,
        );
        assert_eq!(icon.name, "tekes-forward");
        assert_eq!(icon.frame.center(), closed_icon_frame.center());
        assert_eq!(icon.frame.center().y, closed.frame.center().y);
        assert_eq!(icon.tint, Palette::from(Theme::LIGHT).color("primary-text"));

        model.open = true;
        let opened = view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            clip,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
        );
        let opened_icon_frame = view.icon_frame();
        view.render_icon(
            &mut scene,
            &mut icon,
            &model,
            clip,
            Palette::from(Theme::LIGHT),
            16.,
        );
        assert_eq!(icon.name, "tekes-expand");
        assert_eq!(opened.content.min_x(), closed.content.min_x());
        assert_eq!(opened_icon_frame, closed_icon_frame);
        assert_eq!(icon.frame.center(), opened_icon_frame.center());
        assert_eq!(opened.content.center().y, opened.frame.center().y);
        assert_eq!(closed.content.center().y, closed.frame.center().y);
        model.open = false;
    }
    model.disabled = true;
    let mut text = FixedText;
    view.render(
        &mut scene,
        &mut text,
        &model,
        "Details",
        frame,
        clip,
        Palette::from(Theme::LIGHT),
        proposal::TextRole::UiBody,
    );
    let mut icon = CaptureIcon::default();
    view.render_icon(
        &mut scene,
        &mut icon,
        &model,
        clip,
        Palette::from(Theme::LIGHT),
        16.,
    );
    assert_eq!(
        icon.tint,
        Palette::from(Theme::LIGHT).color("tertiary-text")
    );
}

#[test]
fn combo_box_preserves_closed_open_and_disabled_model_semantics() {
    let mut view = ComboBoxView::default();
    let mut model = ComboBox {
        items: vec!["A".into(), "B".into()],
        active: Some(0),
        ..Default::default()
    };
    let opened = view.key_event(ControlKey::Enter, true, &mut model);
    assert!(opened.consumed && opened.changed);
    assert!(model.open);
    assert_eq!(model.selected, None);
    model.disabled = true;
    let before = (
        model.open,
        model.active,
        model.selected,
        model.query.clone(),
    );
    assert!(!view.key_event(ControlKey::Enter, true, &mut model).consumed);
    assert_eq!(
        before,
        (model.open, model.active, model.selected, model.query)
    );
}

#[test]
fn combo_box_keys_use_live_query_without_an_intervening_render() {
    let mut view = ComboBoxView::default();
    let mut model = ComboBox {
        items: vec!["Automatic".into(), "Compact".into(), "Comfortable".into()],
        ..Default::default()
    };
    model.set_query("Com");

    let down = view.key_event(ControlKey::Down, true, &mut model);
    assert!(down.consumed);
    assert_eq!(model.active, Some(2));
    let enter = view.key_event(ControlKey::Enter, true, &mut model);
    assert!(enter.consumed && enter.changed);
    assert_eq!(model.selected, Some(2));
    assert_eq!(model.query, "Comfortable");
    assert!(!model.open);
}

#[test]
fn command_palette_returns_original_invoker_once_for_all_close_paths() {
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette {
        combo: ComboBox {
            items: vec!["One".into(), "Two".into()],
            ..Default::default()
        },
    };
    for key in [ControlKey::Escape, ControlKey::Enter] {
        assert!(view.open_from(&mut model, 42));
        model.combo.set_query("T");
        assert!(!view.open_from(&mut model, 99));
        assert_eq!(model.combo.query, "T");
        assert_eq!(view.take_restore_focus(&model), None);
        assert!(view.key_event(key, true, &mut model).consumed);
        assert!(!model.combo.open);
        assert_eq!(view.take_restore_focus(&model), Some(42));
        assert_eq!(view.take_restore_focus(&model), None);
    }
    assert!(view.open_from(&mut model, 17));
    view.cancel(&mut model);
    assert_eq!(view.take_restore_focus(&model), Some(17));
}

#[test]
fn command_palette_does_not_replay_focus_after_scope_exit_or_legacy_reopen() {
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette::default();
    assert!(view.open_from(&mut model, 42));
    view.blur(&mut model);
    assert_eq!(view.take_restore_focus(&model), None);
    assert!(view.open_from(&mut model, 42));
    view.set_window_active(false, &mut model);
    assert_eq!(view.take_restore_focus(&model), None);
    assert!(!view.open_from(&mut model, 99));
    view.set_window_active(true, &mut model);
    assert!(view.open_from(&mut model, 42));
    view.cancel(&mut model);
    view.open(&mut model);
    view.cancel(&mut model);
    assert_eq!(view.take_restore_focus(&model), None);
    model.combo.disabled = true;
    assert!(!view.open_from(&mut model, 99));
    assert_eq!(view.take_restore_focus(&model), None);
}

#[test]
fn command_palette_keys_use_live_query_without_an_intervening_render() {
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette {
        combo: ComboBox {
            items: vec!["Automatic".into(), "Compact".into(), "Comfortable".into()],
            ..Default::default()
        },
    };
    model.open();
    model.combo.set_query("Com");

    let down = view.key_event(ControlKey::Down, true, &mut model);
    assert!(down.consumed);
    assert_eq!(model.combo.active, Some(2));
    let enter = view.key_event(ControlKey::Enter, true, &mut model);
    assert!(enter.consumed && enter.changed);
    assert_eq!(model.combo.selected, Some(2));
    assert_eq!(model.combo.query, "Comfortable");
    assert!(!model.combo.open);
}

#[test]
fn select_long_value_reserves_and_clips_before_popup_indicator() {
    #[derive(Default)]
    struct CaptureIcon {
        name: String,
        frame: Rect,
    }
    impl IconRenderer for CaptureIcon {
        fn draw_icon(&mut self, _: &mut Scene, name: &str, frame: Rect, _: tekes_ui_tokens::Rgba) {
            self.name = name.to_owned();
            self.frame = frame;
        }
    }
    #[derive(Default)]
    struct Text {
        icon: CaptureIcon,
        label: Option<Rect>,
        measured_width: f32,
    }
    impl ButtonTextRenderer for Text {
        fn icon_renderer(&mut self) -> Option<&mut dyn IconRenderer> {
            Some(&mut self.icon)
        }
        fn measure(&mut self, _: &str, _: proposal::TextRole, width: f32) -> Size {
            self.measured_width = width;
            Size::new(width, 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            rect: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
            self.label = Some(rect);
        }
    }
    let mut view = SelectView::new(1);
    let mut model = crate::Select::new(1);
    let mut text = Text::default();
    let mut scene = Scene::new();
    let layout = view.render_trigger(
        &mut scene,
        &mut text,
        &mut model,
        &["An unusually long selected value"],
        Rect::new(10., 20., 120., 40.),
        None,
        Palette::from(Theme::LIGHT),
        proposal::TextRole::UiBody,
    );
    let label = text.label.unwrap();
    assert_eq!(text.icon.name, "tekes-expand");
    assert!(text.icon.frame.size.width > 0.);
    assert_eq!(
        text.icon.frame.min_x() - label.max_x(),
        proposal::metric("spacing", "space-tight").unwrap()
    );
    assert_eq!(label.size.width, text.measured_width);
    assert!(text.icon.frame.max_x() <= layout.frame.max_x());
    assert!(scene.primitives().iter().any(|p| matches!(p,
        tekes_ui_scene::Primitive::PushClip(rect) if *rect == label)));
}

#[test]
fn select_quiet_trigger_preserves_open_menu_selection() {
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        let mut view = SelectView::new(2);
        let mut model = crate::Select::new(2);
        model.selected = 1;
        let frame = Rect::new(20., 20., 180., 40.);
        for open in [false, true] {
            model.menu.open = open;
            let mut scene = Scene::new();
            view.render_trigger(
                &mut scene,
                &mut FixedText,
                &mut model,
                &["Draft", "Published"],
                frame,
                None,
                palette,
                proposal::TextRole::UiBody,
            );
            let ground = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == frame => Some(q),
                    _ => None,
                })
                .expect("select trigger ground");
            assert_eq!(ground.fill, palette.color("surface-2").unwrap());
            assert_eq!(ground.border_width, 0.);
            assert_eq!(model.selected, 1);
            assert_eq!(model.menu.items.selected, 1);
            assert_eq!(model.menu.open, open);
        }
    }
}

#[test]
fn select_keeps_menu_cursor_across_render_close_and_reopen() {
    let mut view = SelectView::new(3);
    let mut model = crate::Select::new(3);
    let labels = ["One", "Two", "Three"];
    let mut scene = Scene::default();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(20., 20., 180., 40.);
    let viewport = Rect::new(0., 0., 240., 220.);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    view.focus(true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        MenuEvent::None
    );
    let _ = view.key(ControlKey::Enter, false, &mut model);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    let _ = view.key(ControlKey::Down, true, &mut model);
    assert_eq!(model.menu.items.selected, 1, "navigation before render");
    let _ = view.key(ControlKey::Down, false, &mut model);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    assert_eq!(model.menu.items.selected, 1, "navigation after render");
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        MenuEvent::Activated {
            index: 1,
            restore_focus: Some(0)
        }
    );
    assert_eq!(model.selected, 1);
    let _ = view.key(ControlKey::Enter, false, &mut model);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    let _ = view.key(ControlKey::Enter, true, &mut model);
    assert!(model.menu.open, "menu reopens after prior selection");
    let _ = view.key(ControlKey::Enter, false, &mut model);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    let _ = view.key(ControlKey::Down, true, &mut model);
    assert_eq!(
        model.menu.items.selected, 2,
        "reopened navigation before render"
    );
    let _ = view.key(ControlKey::Down, false, &mut model);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    assert_eq!(
        model.menu.items.selected, 2,
        "reopened navigation after render"
    );
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        MenuEvent::Activated {
            index: 2,
            restore_focus: Some(0)
        }
    );
    assert_eq!(model.selected, 2);
    let _ = view.key(ControlKey::Enter, false, &mut model);
}

#[test]
fn select_pointer_release_requires_a_matching_down_and_outside_down_dismisses() {
    let mut view = SelectView::new(3);
    let mut model = crate::Select::new(3);
    let labels = ["One", "Two", "Three"];
    let mut scene = Scene::default();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(20., 20., 180., 40.);
    let viewport = Rect::new(0., 0., 240., 220.);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    view.focus(true);
    let _ = view.key(ControlKey::Enter, true, &mut model);
    let _ = view.key(ControlKey::Enter, false, &mut model);
    let layout = view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    let item = layout.popup.as_ref().unwrap().items[1].frame.center();

    assert_eq!(view.pointer_up(item, &mut model), MenuEvent::None);
    assert!(model.menu.open, "an unpaired release must not dismiss");
    assert_eq!(model.selected, 0);

    let _ = view.pointer_down(item, &mut model);
    let outside = Point::new(230., 210.);
    view.pointer_move(outside, &mut model);
    assert_eq!(view.pointer_up(outside, &mut model), MenuEvent::None);
    assert!(model.menu.open, "dragging out does not commit or dismiss");
    assert_eq!(model.selected, 0);

    let _ = view.pointer_down(item, &mut model);
    assert!(matches!(
        view.pointer_up(item, &mut model),
        MenuEvent::Activated { index: 1, .. }
    ));
    assert_eq!(model.selected, 1);
    assert!(!model.menu.open);

    view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    let _ = view.key(ControlKey::Enter, true, &mut model);
    let _ = view.key(ControlKey::Enter, false, &mut model);
    assert!(
        model.menu.open,
        "selection can reopen after pointer capture"
    );
    let _ = view.pointer_down(outside, &mut model);
    assert!(!model.menu.open, "outside pointer-down dismisses the menu");
}

#[test]
fn select_render_does_not_reactivate_an_inactive_menu_runtime() {
    let mut view = SelectView::new(2);
    let mut model = crate::Select::new(2);
    let labels = ["One", "Two"];
    let mut scene = Scene::default();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(20., 20., 180., 40.);
    let viewport = Rect::new(0., 0., 240., 220.);
    view.set_window_active(false, &mut model);
    model.menu.open = true;
    let _ = view.render(
        &mut scene,
        &mut text,
        &mut model,
        &labels,
        frame,
        viewport,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    assert_eq!(
        view.key(ControlKey::Down, true, &mut model),
        MenuEvent::None
    );
    assert!(!model.menu.open, "inactive rendering cancels the menu");
}

#[test]
fn link_render_consumes_palette_inks_for_interaction_states_and_theme_changes() {
    for (state, role, focused) in [
        ("normal", "c-link-foreground", false),
        ("hover", "c-link-hover", false),
        ("pressed", "c-link-pressed", false),
        ("disabled", "disabled-text", false),
        ("focus-visible", "c-link-foreground", true),
    ] {
        let mut view = LinkView::default();
        let mut model = Link::default();
        let frame = Rect::new(8., 8., 120., 36.);
        model.button.state.disabled = state == "disabled";
        if focused {
            view.focus(true);
        }
        view.render(
            &mut Scene::new(),
            &mut FixedText,
            &mut model,
            "Open",
            frame,
            None,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
        );
        if state == "hover" {
            view.pointer_move(frame.center());
        } else if state == "pressed" {
            assert!(view.pointer_down(frame.center()));
        }
        // Keep the same model, view and held input while changing palettes.
        for theme in Theme::ALL {
            let palette = Palette::from(theme);
            let mut scene = Scene::new();
            view.render(
                &mut scene,
                &mut FixedText,
                &mut model,
                "Open",
                frame,
                None,
                palette,
                proposal::TextRole::UiBody,
            );
            let stroke = proposal::metric(
                "stroke",
                if focused {
                    "stroke-emphasis"
                } else {
                    "stroke-rule"
                },
            )
            .expect("original Link underline stroke");
            let underline = scene
                .primitives()
                .iter()
                .rev()
                .find_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(quad)
                        if quad.border_width == 0.
                            && quad.rect.size.height == stroke
                            && quad.rect.min_x() > frame.min_x()
                            && quad.rect.max_x() < frame.max_x()
                            && quad.rect.min_y() >= frame.min_y()
                            && quad.rect.max_y() < frame.max_y() =>
                    {
                        Some(quad.fill)
                    }
                    _ => None,
                })
                .expect("Link render emits its inset semantic underline");
            assert_eq!(
                underline,
                palette.color(role).expect("generated Link semantic color"),
                "{state} in {theme:?}"
            );
            let has_keyboard_focus = scene.primitives().iter().any(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::Quad(quad) if quad.border_width == 2.)
            });
            assert_eq!(has_keyboard_focus, focused, "{state} in {theme:?}");
        }
    }
}
#[test]
fn link_focus_ring_uses_ancestor_clip_while_text_keeps_its_frame_clip() {
    for theme in Theme::ALL {
        let mut view = LinkView::default();
        let mut model = Link::default();
        let frame = Rect::new(20.25, 20.5, 120., 36.);
        let ancestor = Rect::new(4., 4., 180., 90.);
        view.focus(true);
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut FixedText,
            &mut model,
            "Open",
            frame,
            Some(ancestor),
            Palette::from(theme),
            proposal::TextRole::UiBody,
        );
        let mut clips = Vec::new();
        let mut found = false;
        for primitive in scene.primitives() {
            match primitive {
                tekes_ui_scene::Primitive::PushClip(rect) => clips.push(*rect),
                tekes_ui_scene::Primitive::PopClip => {
                    clips.pop();
                }
                tekes_ui_scene::Primitive::Quad(q) if q.fill.a == 0. && q.border_width == 2. => {
                    assert_eq!(clips, vec![ancestor]);
                    assert_eq!(q.rect, frame.inset(-4.));
                    assert!(ancestor.contains(q.rect.origin));
                    found = true;
                }
                _ => {}
            }
        }
        assert!(found, "keyboard focus must survive the Link's own frame");
        assert_eq!(view.layout.frame, frame);
        assert_eq!(view.layout.visible, Some(frame));
    }
}

#[test]
fn link_hidden_by_clip_cancels_old_pointer_arm() {
    let mut view = LinkView::default();
    let mut model = Link::default();
    let mut scene = Scene::default();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(0., 0., 120., 32.);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        "Open",
        frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(view.pointer_down(Point::new(20., 16.)));
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        "Open",
        frame,
        Some(Rect::new(300., 300., 30., 30.)),
        palette,
        proposal::TextRole::UiBody,
    );
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        "Open",
        frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(!view.pointer_up(Point::new(20., 16.), &mut model));
}

#[test]
fn link_needs_down_and_cancels_when_released_outside() {
    let mut view = LinkView::default();
    let mut model = Link::default();
    let mut scene = Scene::default();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(0., 0., 120., 32.);
    view.render(
        &mut scene,
        &mut text,
        &mut model,
        "Open",
        frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    view.pointer_move(Point::new(20., 16.));
    assert!(!view.pointer_up(Point::new(20., 16.), &mut model));
    assert!(view.pointer_down(Point::new(20., 16.)));
    view.pointer_move(Point::new(200., 16.));
    assert!(!view.pointer_up(Point::new(200., 16.), &mut model));
}

#[test]
fn segmented_space_is_consumed_and_selects_once_on_release() {
    let mut view = SegmentedControlView::new(2);
    let mut model = SegmentedControl::new(2);
    view.focus_with_visibility(1, true);
    let down = view.key_event(ControlKey::Space, true, &mut model);
    assert!(down.consumed);
    assert!(!down.changed);
    let up = view.key_event(ControlKey::Space, false, &mut model);
    assert!(up.consumed);
    assert!(up.changed);
    assert_eq!(model.selected, 1);
    assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
}

#[test]
fn split_resize_target_follows_axis_clip_constraints_and_activation() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut view = SplitViewView::new(axis);
        let mut model = SplitView::new(0.5);
        let bounds = Rect::new(0., 0., 400., 400.);
        let clip = Rect::new(40., 40., 300., 300.);
        assert_eq!(view.resize_target(&model), None);
        let layout = view.render(
            &mut Scene::default(),
            &mut model,
            bounds,
            Some(clip),
            Theme::LIGHT.into(),
        );
        assert_eq!(
            view.resize_target(&model),
            layout.divider_hit.map(|r| (r, axis))
        );
        let (target, _) = view.resize_target(&model).unwrap();
        assert!(target.min_x() >= clip.min_x() && target.max_x() <= clip.max_x());
        assert!(target.min_y() >= clip.min_y() && target.max_y() <= clip.max_y());
        view.set_window_active(false, &mut model);
        assert_eq!(view.resize_target(&model), None);
        view.set_window_active(true, &mut model);
        assert!(view.resize_target(&model).is_some());
        model.min_pane = 300.;
        assert_eq!(view.resize_target(&model), None);
        model.min_pane = 0.;
        view.render(
            &mut Scene::default(),
            &mut model,
            Rect::default(),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(view.resize_target(&model), None);
    }
}

#[test]
fn split_view_runtime_cancel_restores_ratio_for_escape_and_invalid_layout() {
    let mut v = SplitViewView::new(Axis::Horizontal);
    let mut model = SplitView::new(0.5);
    let bounds = Rect::new(0., 0., 200., 100.);
    v.render(
        &mut Scene::default(),
        &mut model,
        bounds,
        None,
        Palette::from(Theme::LIGHT),
    );
    assert!(v.pointer_down(Point::new(100., 50.), &mut model));
    assert!(v.pointer_move(Point::new(140., 50.), &mut model));
    assert_ne!(model.ratio(), 0.5);
    assert!(v.key(ControlKey::Escape, &mut model));
    assert_eq!(model.ratio(), 0.5);
    assert!(!v.pointer_move(Point::new(140., 50.), &mut model));
    assert!(!v.pointer_up(&mut model));

    assert!(v.pointer_down(Point::new(100., 50.), &mut model));
    assert!(v.pointer_move(Point::new(140., 50.), &mut model));
    assert_ne!(model.ratio(), 0.5);
    v.render(
        &mut Scene::default(),
        &mut model,
        bounds,
        Some(Rect::new(300., 300., 10., 10.)),
        Palette::from(Theme::LIGHT),
    );
    assert_eq!(
        model.ratio(),
        0.5,
        "a fully clipped split cancels and rolls back"
    );
    assert!(!v.pointer_move(Point::new(150., 50.), &mut model));
    assert!(!v.pointer_up(&mut model));

    v.render(
        &mut Scene::default(),
        &mut model,
        bounds,
        None,
        Palette::from(Theme::LIGHT),
    );
    assert!(v.pointer_down(Point::new(100., 50.), &mut model));
    assert!(v.pointer_move(Point::new(140., 50.), &mut model));
    assert_ne!(model.ratio(), 0.5);
    v.render(
        &mut Scene::default(),
        &mut model,
        Rect::new(f32::NAN, 0., 200., 100.),
        None,
        Palette::from(Theme::LIGHT),
    );
    assert_eq!(model.ratio(), 0.5, "invalid bounds cancel and roll back");
    assert!(!v.pointer_move(Point::new(150., 50.), &mut model));
    assert!(!v.pointer_up(&mut model));
}

#[test]
fn split_double_click_reset_requires_live_divider_and_releases_drag() {
    let mut view = SplitViewView::new(Axis::Horizontal);
    let mut model = SplitView::new(0.7);
    view.render(
        &mut Scene::new(),
        &mut model,
        Rect::new(0., 0., 200., 100.),
        None,
        Palette::from(Theme::LIGHT),
    );
    let divider = view.layout().divider_hit.unwrap().center();
    assert!(!view.double_click(Point::new(10., 10.), &mut model));
    assert_eq!(model.ratio(), 0.7);
    assert!(view.pointer_down(divider, &mut model));
    assert!(view.pointer_move(Point::new(divider.x + 20., divider.y), &mut model));
    assert!(view.double_click(divider, &mut model));
    assert_eq!(model.ratio(), 0.5);
    assert!(!view.pointer_move(divider, &mut model));
    assert!(!view.pointer_up(&mut model));
    model.set_ratio(0.7);
    view.set_window_active(false, &mut model);
    assert!(!view.double_click(divider, &mut model));
    assert_eq!(model.ratio(), 0.7);
}

#[test]
fn split_view_runtime_focus_loss_cancels_and_releases_drag() {
    let mut v = SplitViewView::new(Axis::Horizontal);
    let mut model = SplitView::new(0.5);
    let bounds = Rect::new(0., 0., 200., 100.);
    v.render(
        &mut Scene::default(),
        &mut model,
        bounds,
        None,
        Palette::from(Theme::LIGHT),
    );
    assert!(v.pointer_down(Point::new(100., 50.), &mut model));
    assert!(v.pointer_move(Point::new(140., 50.), &mut model));
    assert_ne!(model.ratio(), 0.5);
    v.set_window_active(false, &mut model);
    assert_eq!(model.ratio(), 0.5);
    assert!(!v.pointer_move(Point::new(140., 50.), &mut model));
    assert!(!v.pointer_up(&mut model));
}

#[test]
fn navigation_invalid_layout_drops_old_button_hit_targets() {
    let mut scene = Scene::new();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(20., 20., 160., 40.);
    let invalid = Rect::new(f32::NAN, 20., 160., 40.);
    use crate::composition::{Component, ContentEvent};
    let mut icon =
        crate::composition::Button::new(tekes_ui_scene::ImageId(1)).with_accessible_name("Action");
    let clip = Rect::new(0., 0., 400., 300.);
    icon.arrange(&mut text, frame, clip);
    icon.paint(&mut scene, &mut text, frame, clip, palette.into());
    assert!(
        icon.event(&ContentEvent::PointerDown(frame.center()))
            .consumed
    );
    icon.arrange(&mut text, invalid, clip);
    icon.paint(&mut scene, &mut text, invalid, clip, palette.into());
    assert!(
        !icon
            .event(&ContentEvent::PointerUp(frame.center()))
            .activated
    );
    assert!(
        !icon
            .event(&ContentEvent::PointerDown(frame.center()))
            .consumed
    );
    icon.set_focus(true, true);
    // A hidden invalidated button must not retain an old pointer placement.
    assert!(icon.button.layout().is_none());

    let mut select = SelectView::new(2);
    let mut model = crate::Select::new(2);
    for bounds in [frame, invalid] {
        select.render(
            &mut scene,
            &mut text,
            &mut model,
            &["A", "B"],
            bounds,
            Rect::new(0., 0., 400., 300.),
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        if bounds == frame {
            select.focus(true);
        }
    }
    assert!(!select.pointer_down(frame.center(), &mut model));
    select.key(ControlKey::Enter, true, &mut model);
    assert!(!model.menu.open);
}

#[test]
fn breadcrumb_shrink_releases_press_and_invalid_roving_focus() {
    let mut view = BreadcrumbView::default();
    let mut model = Breadcrumb {
        items: vec!["A".into(), "B".into(), "C".into()],
    };
    let mut scene = Scene::new();
    let mut text = FixedText;
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(0., 0., 320., 40.);
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["A", "B", "C"],
        frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    let point = view.layout.items[1].frame().center();
    assert_eq!(view.pointer_down(point), Some(1));
    model.items.truncate(1);
    view.render(
        &mut scene,
        &mut text,
        &model,
        &["A"],
        frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    assert_eq!(view.pointer_up(point, &model), None);
    assert_eq!(
        view.key_event(ControlKey::Enter, true, &model),
        (false, None)
    );
    assert!(!view.buttons[0].state.focused);
}

#[test]
fn split_hidden_divider_does_not_accept_keyboard_reset() {
    let mut view = SplitViewView::new(Axis::Horizontal);
    let mut model = SplitView::new(0.7);
    view.render(
        &mut Scene::new(),
        &mut model,
        Rect::new(0., 0., 200., 100.),
        Some(Rect::new(300., 300., 20., 20.)),
        Palette::from(Theme::LIGHT),
    );
    assert!(!view.key(ControlKey::Enter, &mut model));
    assert_eq!(model.ratio(), 0.7);
}

#[test]
fn icon_button_preserves_model_loading_and_selected_state() {
    use crate::composition::{Component, ContentEvent, ContentResponse};
    let mut button =
        crate::composition::Button::new(tekes_ui_scene::ImageId(1)).with_accessible_name("Action");
    button.button.state.loading = true;
    button.button.state.selected = true;
    let frame = Rect::new(0., 0., 40., 40.);
    button.arrange(&mut FixedText, frame, frame);
    button.paint(
        &mut Scene::new(),
        &mut FixedText,
        frame,
        frame,
        Palette::from(Theme::LIGHT).into(),
    );
    assert!(button.button.state.loading && button.button.state.selected);
    assert!(
        !button
            .event(&ContentEvent::PointerDown(frame.center()))
            .consumed
    );
    button.set_focus(true, true);
    assert_eq!(
        button.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true
        }),
        ContentResponse::default()
    );
}

#[test]
fn link_deactivation_clears_separately_owned_hover() {
    let mut view = LinkView::default();
    let mut model = Link::default();
    let frame = Rect::new(0., 0., 160., 40.);
    let render = |view: &mut LinkView, model: &mut Link| {
        view.render(
            &mut Scene::new(),
            &mut FixedText,
            model,
            "Link",
            frame,
            None,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
        );
    };
    render(&mut view, &mut model);
    view.pointer_move(frame.center());
    render(&mut view, &mut model);
    assert!(model.button.state.hovered);
    view.set_window_active(false);
    view.set_window_active(true);
    render(&mut view, &mut model);
    assert!(!model.button.state.hovered);
}

#[test]
fn select_reopens_with_selected_item_revealed_in_unchanged_popup() {
    let mut view = SelectView::new(10);
    let mut model = crate::Select::new(10);
    let labels = ["A"; 10];
    let mut scene = Scene::new();
    let mut text = FixedText;
    let render = |view: &mut SelectView,
                  model: &mut crate::Select,
                  scene: &mut Scene,
                  text: &mut FixedText| {
        view.render(
            scene,
            text,
            model,
            &labels,
            Rect::new(20., 20., 160., 40.),
            Rect::new(0., 0., 300., 180.),
            None,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
            false,
        )
    };
    render(&mut view, &mut model, &mut scene, &mut text);
    view.focus(true);
    view.key(ControlKey::Enter, true, &mut model);
    render(&mut view, &mut model, &mut scene, &mut text);
    view.key(ControlKey::Escape, true, &mut model);
    model.selected = 9;
    render(&mut view, &mut model, &mut scene, &mut text);
    view.key(ControlKey::Enter, true, &mut model);
    let popup = render(&mut view, &mut model, &mut scene, &mut text)
        .popup
        .unwrap();
    assert!(popup.scroll_offset > 0.);
    assert!(popup.items[9].visible.is_some_and(has_area));
}
