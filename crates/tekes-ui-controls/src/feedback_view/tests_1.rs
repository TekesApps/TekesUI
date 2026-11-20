use super::*;
use crate::extended::{
    Banner, EmptyState, NoticeTone, Popover, Progress, StatusBadge, Toast, Tooltip,
};
use crate::{Field, NumberField};
use tekes_ui_scene::Primitive;
use tekes_ui_scene::Size;

pub(super) struct FixedText;
pub(super) struct ScaledText(pub(super) f32);

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
        _ink: tekes_ui_tokens::Rgba,
    ) {
    }
}

impl ButtonTextRenderer for FixedText {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        let width = (text.chars().count() as f32 * 8.).min(max_width.max(0.));
        Size::new(width, role.style().line_height)
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
pub(super) struct TallText;
impl ButtonTextRenderer for TallText {
    fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 8.).min(max_width.max(0.)),
            48.,
        )
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
#[derive(Default)]
pub(super) struct RecordingText(
    pub(super) Vec<(String, proposal::TextRole, Rect, ButtonTextAlign)>,
);
impl ButtonTextRenderer for RecordingText {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 8.).min(max_width.max(0.)),
            role.style().line_height,
        )
    }
    fn draw(
        &mut self,
        _scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        _ink: tekes_ui_tokens::Rgba,
    ) {
        self.0.push((text.to_owned(), role, rect, align));
    }
}
#[derive(Default)]
pub(super) struct WrappingText {
    pub(super) measures: Vec<(String, proposal::TextRole, f32)>,
    pub(super) draws: Vec<(String, proposal::TextRole, Rect, ButtonTextAlign)>,
}
impl ButtonTextRenderer for WrappingText {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        self.measures.push((text.to_owned(), role, max_width));
        let line_height = role.style().line_height.max(1.);
        let width = (text.chars().count() as f32 * 8.).min(max_width.max(0.));
        let lines = if max_width > 0. {
            ((text.chars().count() as f32 * 8.) / max_width)
                .ceil()
                .max(1.)
        } else {
            1.
        };
        Size::new(width, lines * line_height)
    }
    fn draw(
        &mut self,
        _scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        _ink: tekes_ui_tokens::Rgba,
    ) {
        self.draws.push((text.to_owned(), role, rect, align));
    }
}
pub(super) fn palette() -> Palette {
    Palette::from(tekes_ui_tokens::Theme::LIGHT)
}
pub(super) fn role() -> proposal::TextRole {
    proposal::TextRole::UiBody
}

pub(super) fn assert_background_clip(
    scene: &Scene,
    frame: Rect,
    ancestor: Rect,
    visible: Rect,
    has_content: bool,
) {
    let primitives = scene.primitives();
    assert!(matches!(primitives.first(), Some(Primitive::PushClip(r)) if *r == ancestor));
    let background = primitives
        .iter()
        .position(|p| matches!(p, Primitive::Quad(q) if q.rect == frame))
        .expect("surface quad");
    assert!(
        primitives[1..background]
            .iter()
            .all(|p| matches!(p, Primitive::Shadow(_)))
    );
    assert!(matches!(
        primitives.get(background + 1),
        Some(Primitive::PopClip)
    ));
    if has_content {
        assert!(
            primitives
                .iter()
                .any(|p| matches!(p, Primitive::PushClip(r) if *r == visible))
        );
    }
}

#[test]
fn surface_panel_clips_and_fully_clipped_frame_has_no_visible_geometry() {
    let mut view = SurfaceView::default();
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        Rect::new(0., 0., 100., 60.),
        Some(Rect::new(120., 0., 40., 40.)),
        palette(),
        true,
    );
    assert_eq!(layout.visible, None);
    assert!(scene.is_empty());
    assert!(!layout.hit_test(Point::new(1., 1.)));
}

#[test]
fn surface_background_uses_parent_clip_but_keeps_visible_geometry() {
    let frame = Rect::new(40., 30., 120., 70.);
    for (ancestor, visible) in [
        (Rect::new(0., 0., 320., 240.), frame),
        (
            Rect::new(80., 40., 120., 90.),
            Rect::new(80., 40., 80., 60.),
        ),
    ] {
        let mut view = SurfaceView::default();
        let mut scene = Scene::new();
        let layout = view.render(&mut scene, frame, Some(ancestor), palette(), true);
        assert_eq!(layout.visible, Some(visible));
        assert_background_clip(&scene, frame, ancestor, visible, false);
    }
}

#[test]
fn surface_render_scale_changes_only_decorative_ink_not_geometry() {
    let frame = Rect::new(10.0, 20.0, 180.0, 88.0);
    let clip = Rect::new(24.0, 10.0, 140.0, 70.0);
    for theme in tekes_ui_tokens::Theme::ALL {
        let palette = Palette::from(theme);
        let mut layouts = Vec::new();
        for scale in [1.0, 2.0] {
            let mut view = SurfaceView::default();
            let mut scene = Scene::new();
            let layout = view.render_at_scale(&mut scene, frame, Some(clip), palette, true, scale);
            assert_eq!(layout.frame, frame);
            assert_eq!(layout.content, safe_inset(frame, 16.0, 16.0));
            assert_eq!(layout.visible, frame.intersection(&clip));
            let panel = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == frame => Some(q),
                    _ => None,
                })
                .expect("panel quad");
            assert_eq!(panel.rect, frame);
            assert_eq!(panel.fill, palette.color("surface-panel").unwrap());
            assert_eq!(
                panel.border_color,
                paint::decorative_border(palette, scale).1
            );
            layouts.push(layout);
        }
        assert_eq!(layouts[0], layouts[1]);
    }
}
#[test]
fn field_provides_nonoverlapping_label_slot_and_error_message_geometry() {
    let mut view = FieldView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let field = Field {
        label: "Workspace".into(),
        help: "Use a short name".into(),
        error: Some("Already exists".into()),
        required: true,
    };
    let l = view.render(
        &mut scene,
        &mut text,
        &field,
        Rect::new(10., 20., 240., 120.),
        None,
        palette(),
        role(),
    );
    assert!(l.slot.min_y() >= l.label.max_y());
    assert!(l.message.min_y() >= l.slot.max_y());
    assert!(l.has_error);
    assert!(l.required_mark.is_some());
}
#[test]
fn field_slot_grows_for_scaled_text_surface_line_metrics() {
    let mut view = FieldView::default();
    let mut text = TallText;
    let mut scene = Scene::new();
    let l = view.render(
        &mut scene,
        &mut text,
        &Field {
            label: "Name".into(),
            ..Field::default()
        },
        Rect::new(0., 0., 240., 120.),
        None,
        palette(),
        role(),
    );
    assert!(l.slot.size.height >= 48. + 2. * proposal::SCROLL_SINGLE_LINE_PADDING_Y);
}
#[test]
fn field_emphasized_form_presentation_restores_label_and_spacing() {
    let mut view = FieldView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let frame = Rect::new(10., 20., 320., 140.);
    let layout = view.render_with_presentation(
        &mut scene,
        &mut text,
        &Field {
            label: "Workspace name".into(),
            help: "Use a clear name for your workspace.".into(),
            required: false,
            ..Field::default()
        },
        frame,
        None,
        palette(),
        role(),
        FieldPresentation::emphasized_form(),
    );
    assert_eq!(layout.label.min_y(), frame.min_y());
    assert_eq!(layout.label.size.height, 36.);
    assert_eq!(layout.slot.min_y(), layout.label.max_y() + 4.);
    assert_eq!(layout.message.min_y(), layout.slot.max_y() + 6.);
    assert!(layout.slot.size.height >= 36.);
    assert_eq!(layout.message.size.height, 36.);
    assert!(layout.required_mark.is_none());
}
#[test]
fn toolbar_pointer_and_keyboard_activation_are_owned_by_child_buttons() {
    let mut view = ToolbarView::new(2, 4.);
    view.focus(Some(1), true);
    let mut scene = Scene::new();
    let mut text = FixedText;
    let r = Rect::new(0., 0., 180., 36.);
    let layouts = view.render(
        &mut scene,
        &mut text,
        &["Add", "Remove"],
        r,
        None,
        palette(),
        role(),
    );
    assert_eq!(layouts.len(), 2);
    assert_eq!(view.key(ControlKey::Enter, true), Some(1));
    let point = Point::new(layouts[0].frame.center().x, layouts[0].frame.center().y);
    assert_eq!(view.pointer_down(point), Some(0));
    assert_eq!(view.pointer_up(point), Some(0));
    view.set_disabled(true);
    assert_eq!(view.key(ControlKey::Enter, true), None);
}
#[test]
fn toolbar_restores_primary_presentation_for_first_action() {
    let mut view = ToolbarView::new(3, 4.);
    let mut text = FixedText;
    let mut scene = Scene::new();
    let layouts = view.render(
        &mut scene,
        &mut text,
        &["Create", "Rename", "Remove"],
        Rect::new(0., 0., 528., 36.),
        None,
        palette(),
        role(),
    );
    let primary_fill = palette().color("action-primary-ground").unwrap();
    assert!(scene.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Quad(q) if q.rect == layouts[0].frame && q.fill == primary_fill)
    }));
}
#[test]
fn number_field_centers_value_in_its_full_slot() {
    let mut view = NumberFieldView::default();
    let mut text = RecordingText::default();
    let mut scene = Scene::new();
    let model = NumberField::new(8., 1., 32., 1.).unwrap();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(10., 20., 198., 36.),
        None,
        palette(),
        role(),
    );
    assert!(text.0.iter().any(|(drawn, text_role, rect, align)| {
        drawn == "8"
            && *text_role == proposal::TextRole::UiHeading
            && *rect == layout.value
            && *align == ButtonTextAlign::Center
    }));
    for stepper in ["−", "+"] {
        assert!(text.0.iter().any(|(drawn, _, _, align)| {
            drawn == stepper && *align == button_text_align(proposal::layout::BUTTON_CONTENT)
        }));
    }
}
#[test]
fn number_field_hover_and_press_keep_hit_slots_without_rectangular_backgrounds() {
    let mut model = NumberField::new(100., 50., 300., 10.).unwrap();
    let mut view = NumberFieldView::default();
    let mut text = FixedText;
    let frame = Rect::new(10., 20., 198., 36.);
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let point = layout.increment.frame.center();
    view.pointer_move(point);
    assert_eq!(view.pointer_down(point), Some(true));
    let mut pressed = Scene::new();
    let current = view.render(
        &mut pressed,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    assert_eq!(current.increment.frame, layout.increment.frame);
    assert_eq!(current.decrement.frame, layout.decrement.frame);
    for scene in [&scene, &pressed] {
        assert!(
            !scene
                .primitives()
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Quad(_))),
            "step symbols must not paint rectangular resting/hover/pressed chrome"
        );
    }
    assert_eq!(
        view.pointer_up(point, &mut model),
        NumberFieldEvent::Changed
    );
    assert_eq!(model.value(), 110.);
}

#[test]
fn number_field_respects_model_limits_and_cancels_press_when_disabled() {
    let mut model = NumberField::new(2., 1., 3., 1.).unwrap();
    let mut view = NumberFieldView::default();
    let mut scene = Scene::new();
    let mut text = FixedText;
    let l = view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(0., 0., 140., 36.),
        None,
        palette(),
        role(),
    );
    assert!(
        view.pointer_down(Point::new(
            l.increment.frame.center().x,
            l.increment.frame.center().y
        ))
        .is_some()
    );
    view.set_disabled(true);
    assert_eq!(
        view.pointer_up(
            Point::new(l.increment.frame.center().x, l.increment.frame.center().y),
            &mut model
        ),
        NumberFieldEvent::None
    );
    assert_eq!(model.value(), 2.);
    view.set_disabled(false);
    let l = view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(0., 0., 140., 36.),
        None,
        palette(),
        role(),
    );
    let p = Point::new(l.increment.frame.center().x, l.increment.frame.center().y);
    assert!(view.pointer_down(p).is_some());
    assert_eq!(view.pointer_up(p, &mut model), NumberFieldEvent::Changed);
    assert_eq!(model.value(), 3.);
    assert!(!model.can_increment(true));
}

#[test]
fn number_field_focus_routes_enter_and_space_to_the_focused_stepper_button() {
    let mut model = NumberField::new(2., 0., 4., 1.).unwrap();
    let mut view = NumberFieldView::default();
    view.focus(false, true);
    assert_eq!(view.focused_index(), Some(false));
    assert_eq!(
        view.key_event(&mut model, ControlKey::Enter, true),
        NumberFieldEvent::Changed
    );
    assert_eq!(model.value(), 1.);
    assert_eq!(
        view.key_event(&mut model, ControlKey::Space, true),
        NumberFieldEvent::None
    );
    view.blur();
    assert_eq!(
        view.key_event(&mut model, ControlKey::Space, false),
        NumberFieldEvent::None
    );
    assert_eq!(view.focused_index(), None);
    assert_eq!(model.value(), 1.);
}
#[test]
fn overlays_keep_offscreen_anchor_geometry_inside_effective_viewport() {
    let viewport = Rect::new(40., 30., 200., 160.);
    let anchor = Rect::new(-80., -100., 20., 20.);
    let mut scene = Scene::new();
    let mut text = FixedText;
    let mut tooltip = TooltipView::default();
    let tip = tooltip.render(
        &mut scene,
        &mut text,
        &Tooltip {
            visible: true,
            text: "detail".into(),
        },
        anchor,
        Size::new(120., 42.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(tip.frame.min_y() >= viewport.min_y());
    assert!(tip.frame.min_x() >= viewport.min_x());
    assert_eq!(tip.visible, Some(tip.frame));
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 9);
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        anchor,
        Size::new(150., 100.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(layout.frame.min_y() >= viewport.min_y());
    assert_eq!(layout.visible, Some(layout.frame));
}

#[test]
fn popover_dismiss_wraps_at_its_rendered_width_and_keeps_content_separate() {
    let mut scene = Scene::new();
    let mut text = WrappingText::default();
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 9);
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(20., 20., 10., 10.),
        Size::new(64., 180.),
        Rect::new(0., 0., 300., 300.),
        None,
        palette(),
        role(),
    );
    let dismiss = view.dismiss_layout.unwrap();
    let drawn = text
        .draws
        .iter()
        .find(|draw| draw.0 == "Dismiss")
        .unwrap()
        .2;
    let natural = text.measure("Dismiss", role(), drawn.size.width);
    assert!(drawn.size.height >= natural.height);
    assert!(dismiss.frame.max_y() <= layout.frame.max_y());
    assert!(layout.content.max_y() <= dismiss.frame.min_y());
    view.focus_dismiss(true, true);
    assert_eq!(
        view.key_event(&mut model, ControlKey::Enter, true),
        OverlayEvent::Dismissed {
            restore_focus: Some(9)
        }
    );
}

#[test]
fn tooltip_and_popover_clamp_to_viewport_and_outside_escape_restore_focus() {
    let viewport = Rect::new(0., 0., 240., 160.);
    let anchor = Rect::new(210., 135., 20., 20.);
    let mut text = FixedText;
    let mut scene = Scene::new();
    let mut tooltip = TooltipView::default();
    let model = Tooltip {
        visible: true,
        text: "Helpful detail".into(),
    };
    let tip = tooltip.render(
        &mut scene,
        &mut text,
        &model,
        anchor,
        Size::new(120., 42.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(tip.frame.max_x() <= viewport.max_x());
    assert!(tip.frame.max_y() <= viewport.max_y());
    let mut popover = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut popover, 7);
    let p = view.render(
        &mut scene,
        &mut text,
        &popover,
        anchor,
        Size::new(150., 90.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(p.frame.max_x() <= viewport.max_x());
    assert!(p.frame.max_y() <= viewport.max_y());
    assert_eq!(
        view.key(&mut popover, ControlKey::Escape),
        OverlayEvent::Dismissed {
            restore_focus: Some(7)
        }
    );
    view.show(&mut popover, 8);
    view.render(
        &mut scene,
        &mut text,
        &popover,
        anchor,
        Size::new(150., 90.),
        viewport,
        None,
        palette(),
        role(),
    );
    let outside = Point::new(3., 3.);
    assert!(view.pointer_down(&mut popover, outside));
    assert_eq!(
        view.pointer_up(&mut popover, outside),
        OverlayEvent::Dismissed {
            restore_focus: Some(8)
        }
    );
}
#[test]
fn toast_fixture_time_banner_dismiss_and_progress_are_clipped() {
    let mut toast = Toast::new("Saved", NoticeTone::Success, 30);
    let mut view = ToastView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let size = ToastView::measure(&mut text, &toast.message, 180.0, role());
    let r = Rect::new(10., 10., size.width, size.height);
    view.render(
        &mut scene,
        &mut text,
        &toast,
        r,
        Some(Rect::new(0., 0., 180., 120.)),
        palette(),
        role(),
    );
    let toast_dismiss = view.dismiss_layout.unwrap().frame.center();
    assert_eq!(view.focus_target(), view.dismiss_layout.unwrap().visible);
    assert!(view.pointer_down(toast_dismiss));
    assert!(!view.pointer_up(&mut toast, Point::new(300., 300.)));
    assert!(!view.dismiss.state.pressed);
    assert!(toast.visible);
    assert!(view.pointer_down(toast_dismiss));
    assert!(view.pointer_up(&mut toast, toast_dismiss));
    assert!(!toast.visible);
    // Duration fixture tests use a fresh toast after the action test.
    let mut toast = Toast::new("Saved", NoticeTone::Success, 30);
    view.render(
        &mut scene,
        &mut text,
        &toast,
        r,
        Some(Rect::new(0., 0., 180., 120.)),
        palette(),
        role(),
    );
    assert!(!view.advance(&mut toast, 10));
    assert_eq!(toast.remaining_ms(), 20);
    assert!(view.advance(&mut toast, 20));
    let mut banner = Banner {
        message: "Offline".into(),
        tone: NoticeTone::Warning,
        dismissible: true,
        visible: true,
    };
    let mut b = BannerView::default();
    b.render(&mut scene, &mut text, &banner, r, None, palette(), role());
    let dismiss = b.dismiss_layout.unwrap().frame.center();
    assert_eq!(b.focus_target(), b.dismiss_layout.unwrap().visible);
    assert!(!b.pointer_up(&mut banner, dismiss));
    assert!(b.pointer_down(dismiss));
    assert!(!b.pointer_up(&mut banner, Point::new(300., 300.)));
    assert!(!b.dismiss_button.state.pressed);
    assert!(banner.visible);
    assert!(b.pointer_down(dismiss));
    assert!(b.pointer_up(&mut banner, dismiss));
    assert!(!banner.visible);
    let mut progress = ProgressView::default();
    progress.render(
        &mut scene,
        Progress::new(0.5),
        r,
        Some(Rect::new(500., 500., 10., 10.)),
        palette(),
    );
    assert!(progress.layout().visible.is_none());
    let mut badge = StatusBadgeView::default();
    badge.render(&mut scene, StatusBadge::Running, r, None, palette());
    let mut skeleton = SkeletonView::default();
    skeleton.render(&mut scene, r, None, palette());
    let mut empty = EmptyStateView::default();
    let state = EmptyState {
        title: "No items".into(),
        detail: "Try another query".into(),
        action: Some("Clear".into()),
    };
    empty.render(&mut scene, &mut text, &state, r, None, palette(), role());
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Quad(_)))
    );
}
#[test]
fn banner_uses_available_height_for_wrapped_message_and_large_dismiss() {
    let mut view = BannerView::default();
    let model = Banner {
        visible: true,
        message: "A long banner message that wraps across many narrow lines in the available area"
            .into(),
        tone: NoticeTone::Info,
        dismissible: true,
    };
    let mut scene = Scene::new();
    let mut text = WrappingText::default();
    view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(0., 0., 300., 250.),
        None,
        palette(),
        role(),
    );
    let message = text
        .draws
        .iter()
        .find(|draw| draw.0 == model.message)
        .unwrap()
        .2;
    let measured = text.measure(&model.message, role(), message.size.width);
    assert!(measured.height > 54.);
    assert!(message.size.height >= measured.height);
    let mut tall_text = TallText;
    view.render(
        &mut scene,
        &mut tall_text,
        &model,
        Rect::new(0., 0., 300., 250.),
        None,
        palette(),
        role(),
    );
    let dismiss = view.dismiss_layout.unwrap();
    assert!(dismiss.frame.size.height >= 48.);
    assert!(dismiss.frame.max_y() <= 250.);
}

#[test]
fn narrow_banner_wraps_action_below_message_without_overlap() {
    let mut view = BannerView::default();
    let model = Banner {
        visible: true,
        message: "A long notice that needs several lines in a narrow window".into(),
        tone: NoticeTone::Info,
        dismissible: true,
    };
    let mut text = WrappingText::default();
    let measured = BannerView::measure(&mut text, &model.message, 220.0, true, role());
    assert!(measured.height > 74.0);
    let frame = Rect::new(10.0, 20.0, measured.width, measured.height);
    let mut scene = Scene::new();
    view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let message = text
        .draws
        .iter()
        .find(|draw| draw.0 == model.message)
        .unwrap()
        .2;
    let dismiss = view.dismiss_layout.unwrap().frame;
    assert!(message.max_x() <= frame.max_x());
    assert!(message.max_y() <= dismiss.min_y());
    assert!(dismiss.max_x() <= frame.max_x());
    assert!(dismiss.max_y() <= frame.max_y());
}

#[test]
fn owned_feedback_buttons_use_the_shared_default_text_alignment() {
    assert_eq!(ButtonTextAlign::default(), ButtonTextAlign::Center);
    let mut violations = Vec::new();
    for case in ["toolbar", "popover", "toast", "banner", "empty-state"] {
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        let frame = Rect::new(20., 30., 640., 240.);
        match case {
            "toolbar" => {
                ToolbarView::default().render(
                    &mut scene,
                    &mut text,
                    &["Save", "Cancel"],
                    frame,
                    None,
                    palette(),
                    role(),
                );
            }
            "popover" => {
                let mut model = Popover::default();
                let mut view = PopoverView::default();
                view.show(&mut model, 7);
                view.render(
                    &mut scene,
                    &mut text,
                    &model,
                    Rect::new(20., 20., 20., 20.),
                    Size::new(260., 180.),
                    Rect::new(0., 0., 1000., 600.),
                    None,
                    palette(),
                    role(),
                );
            }
            "toast" => {
                ToastView::default().render(
                    &mut scene,
                    &mut text,
                    &Toast::new("Notice message", NoticeTone::Success, 5000),
                    frame,
                    None,
                    palette(),
                    role(),
                );
            }
            "banner" => {
                BannerView::default().render(
                    &mut scene,
                    &mut text,
                    &Banner {
                        message: "Notice message".into(),
                        tone: NoticeTone::Info,
                        dismissible: true,
                        visible: true,
                    },
                    frame,
                    None,
                    palette(),
                    role(),
                );
            }
            "empty-state" => {
                EmptyStateView::default().render(
                    &mut scene,
                    &mut text,
                    &EmptyState {
                        title: "Empty".into(),
                        detail: "Body".into(),
                        action: Some("Create document".into()),
                    },
                    frame,
                    None,
                    palette(),
                    role(),
                );
            }
            _ => unreachable!(),
        }
        let actions: Vec<_> = text
            .0
            .iter()
            .filter(|(s, _, _, _)| {
                ["Save", "Cancel", "Dismiss", "Create document"].contains(&s.as_str())
            })
            .collect();
        assert_eq!(actions.len(), if case == "toolbar" { 2 } else { 1 });
        for (label, _, _, align) in actions {
            if *align != ButtonTextAlign::default() {
                violations.push(format!("{case}: {label} => {align:?}"));
            }
        }
        for (label, _, _, align) in &text.0 {
            if ["Notice message", "Empty", "Body"].contains(&label.as_str()) {
                assert_eq!(
                    *align,
                    ButtonTextAlign::Leading,
                    "body text retains its own semantic alignment"
                );
            }
        }
    }
    assert!(
        violations.is_empty(),
        "owned actions bypassed shared ButtonModel default: {violations:?}"
    );
}

#[test]
fn inactive_banner_keeps_passive_paint_without_focus_or_activation() {
    let mut view = BannerView::default();
    let mut model = Banner {
        message: "Your workspace has unsaved changes.".into(),
        tone: NoticeTone::Info,
        dismissible: true,
        visible: true,
    };
    view.set_window_active(false);
    let mut scene = Scene::new();
    let frame = Rect::new(20., 30., 640., 74.);
    view.render(
        &mut scene,
        &mut FixedText,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(
        !scene.is_empty(),
        "inactive windows retain passive notice paint"
    );
    assert!(view.logical_focus_target().is_none());
    assert!(view.focus_target().is_none());
    assert!(!view.pointer_down(frame.center()));
    view.focus_dismiss(true, true);
    assert!(!view.key(&mut model, ControlKey::Enter, true));
    assert!(model.visible);
    view.set_window_active(true);
    view.render(
        &mut Scene::new(),
        &mut FixedText,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    view.focus_dismiss(true, true);
    assert!(view.key(&mut model, ControlKey::Enter, true));
    assert!(!model.visible);
}

#[test]
fn notices_align_status_marker_to_actual_first_line_at_each_text_scale() {
    struct NoticeText {
        first_line: f32,
        rows: f32,
        draws: Vec<(String, Rect)>,
    }
    impl ButtonTextRenderer for NoticeText {
        fn measure(&mut self, text: &str, _: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 8.).min(width.max(0.)),
                self.first_line * self.rows,
            )
        }
        fn first_line_height(&mut self, _: &str, _: proposal::TextRole, _: f32) -> f32 {
            self.first_line
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            text: &str,
            _: proposal::TextRole,
            rect: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
            self.draws.push((text.to_owned(), rect));
        }
    }
    for first_line in [18., 27., 36.] {
        for (width, rows) in [(640., 1.), (220., 3.)] {
            for banner in [false, true] {
                let mut text = NoticeText {
                    first_line,
                    rows,
                    draws: Vec::new(),
                };
                let mut scene = Scene::new();
                let message = "Saved 文档 changes across wrapped lines";
                let size = if banner {
                    BannerView::measure(&mut text, message, width, true, role())
                } else {
                    ToastView::measure(&mut text, message, width, role())
                };
                let frame = Rect::new(20., 30., size.width, size.height);
                if banner {
                    BannerView::default().render(
                        &mut scene,
                        &mut text,
                        &Banner {
                            message: message.into(),
                            tone: NoticeTone::Info,
                            dismissible: true,
                            visible: true,
                        },
                        frame,
                        None,
                        palette(),
                        role(),
                    );
                } else {
                    ToastView::default().render(
                        &mut scene,
                        &mut text,
                        &Toast::new(message, NoticeTone::Success, 5000),
                        frame,
                        None,
                        palette(),
                        role(),
                    );
                }
                let message_rect = text.draws.iter().find(|(s, _)| s == message).unwrap().1;
                let diameter = if banner { 12. } else { 10. };
                let marker = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q)
                            if q.rect.size.width == diameter && q.rect.size.height == diameter =>
                        {
                            Some(q.rect)
                        }
                        _ => None,
                    })
                    .expect("actual status marker");
                assert!(
                    (marker.min_y() + marker.size.height / 2.
                        - message_rect.min_y()
                        - first_line / 2.)
                        .abs()
                        < 0.01,
                    "banner={banner}, first line={first_line}, rows={rows}: marker={marker:?}, text={message_rect:?}"
                );
                assert!(marker.max_x() < message_rect.min_x());
                assert!(marker.min_y() >= frame.min_y());
                assert!(marker.max_y() <= frame.max_y());
            }
        }
    }
}

#[test]
fn banner_uses_symmetric_padding_and_quiet_dismiss_with_first_line_marker() {
    let mut view = BannerView::default();
    let model = Banner {
        message: "Your workspace has unsaved changes.".into(),
        tone: NoticeTone::Info,
        dismissible: true,
        visible: true,
    };
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    let pad = tekes_ui_tokens::Spacing::Relaxed.value();
    let gap = tekes_ui_tokens::Spacing::Regular.value();
    let frame = Rect::new(10., 12., 640., 74.);
    view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let dismiss = view.dismiss_layout.unwrap();
    assert_eq!(
        dismiss.frame,
        Rect::new(frame.max_x() - pad - 92., 32., 92., 34.)
    );
    assert!(
        ((dismiss.frame.min_y() - frame.min_y()) - (frame.max_y() - dismiss.frame.max_y())).abs()
            <= 0.01
    );
    let message = text
        .0
        .iter()
        .find(|(drawn, _, _, _)| drawn == &model.message)
        .unwrap();
    assert_eq!(message.2.min_x(), frame.min_x() + pad + 12. + gap);
    assert_eq!(message.2.min_y(), frame.min_y() + pad);
    assert_eq!(message.2.size.height, role().style().line_height);
    assert!(scene.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Quad(q) if q.rect == Rect::new(frame.min_x() + pad, frame.min_y() + pad + (role().style().line_height - 12.) * 0.5, 12., 12.))
    }));
    let quiet_fill = tekes_ui_tokens::surface_palette::resolve_surface(
        palette(),
        "panel",
        "action-quiet",
        "rest",
    )
    .unwrap()
    .background;
    assert!(scene.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Quad(q) if q.rect == dismiss.frame && q.fill == quiet_fill)
    }));
}

#[test]
fn number_field_has_symmetric_stepper_gaps() {
    let mut view = NumberFieldView::default();
    let model = NumberField::new(10., 0., 100., 1.).unwrap();
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    let frame = Rect::new(12., 16., 198., 36.);
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let left_gap = layout.value.min_x() - layout.decrement.frame.max_x();
    let right_gap = layout.increment.frame.min_x() - layout.value.max_x();
    assert!(
        (left_gap - right_gap).abs() <= 0.01,
        "left_gap={left_gap} vs right_gap={right_gap}"
    );
    assert!(
        (left_gap - tekes_ui_tokens::Spacing::Relaxed.value()).abs() <= 0.01,
        "left_gap={left_gap}"
    );
}
