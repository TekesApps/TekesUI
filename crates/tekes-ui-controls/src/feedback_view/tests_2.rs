use super::tests_1::{
    FixedText, RecordingText, ScaledText, TallText, WrappingText, assert_background_clip, palette,
    role,
};
use super::*;
use crate::extended::{Banner, EmptyState, NoticeTone, Popover, Toast, Tooltip};
use tekes_ui_scene::Primitive;
use tekes_ui_scene::{Quad, Size};

#[test]
fn toast_uses_symmetric_padding_and_centered_dismiss() {
    let mut view = ToastView::default();
    let model = Toast::new("Changes saved", NoticeTone::Success, 5000);
    let pad = tekes_ui_tokens::Spacing::Relaxed.value();
    let gap = tekes_ui_tokens::Spacing::Regular.value();
    let dot = proposal::metric("size", "size-dot").unwrap();
    let min_row = proposal::metric("size", "size-row-height").unwrap();
    let dismiss_w = ButtonModel::measure(
        &mut RecordingText::default(),
        "Dismiss",
        f32::MAX,
        ButtonSize::Body,
        role(),
    )
    .width;
    let leading_inset = pad + dot + gap;
    let content_width = 520. - leading_inset - gap - dismiss_w - pad;
    let measured = ToastView::measure(&mut RecordingText::default(), &model.message, 520., role());
    assert_eq!(measured, Size::new(520., pad * 2. + min_row));
    let frame = Rect::new(40., 30., measured.width, measured.height);
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    assert_eq!(
        layout.content,
        Rect::new(40. + leading_inset, 30. + pad, content_width, min_row)
    );
    let message = text
        .0
        .iter()
        .find(|(drawn, _, _, _)| drawn == &model.message)
        .unwrap();
    assert_eq!(
        message.2,
        Rect::new(
            40. + leading_inset,
            30. + pad,
            content_width,
            role().style().line_height
        )
    );
    let dismiss = view.dismiss_layout.unwrap().frame;
    assert_eq!(
        dismiss,
        Rect::new(
            frame.max_x() - pad - dismiss_w,
            30. + pad,
            dismiss_w,
            min_row
        )
    );
    assert!((dismiss.min_x() - layout.content.max_x() - gap).abs() <= 0.01);
    assert!(((frame.max_x() - dismiss.max_x()) - pad).abs() <= 0.01);
    assert!(text.0.iter().any(|(drawn, _, _, align)| {
        drawn == "Dismiss" && *align == ButtonTextAlign::default()
    }));
    assert!(scene.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Quad(q) if q.rect == Rect::new(frame.min_x() + pad, frame.min_y() + pad + (role().style().line_height - dot) * 0.5, dot, dot))
    }));
}

#[test]
fn toast_measure_matches_wrapped_message_and_dismiss_slot() {
    let mut view = ToastView::default();
    let model = Toast::new(
        "A long toast message that wraps in a narrow viewport.",
        NoticeTone::Info,
        5000,
    );
    let mut text = TallText;
    let measured = ToastView::measure(&mut text, &model.message, 196.0, role());
    assert!(measured.height > 62.0);
    let frame = Rect::new(20.0, 30.0, measured.width, measured.height);
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
    let dismiss = view.dismiss_layout.unwrap();
    assert_eq!(layout.frame.size, measured);
    assert_eq!(layout.content.size.height, 48.0);
    assert!(dismiss.frame.max_y() <= frame.max_y());
    assert!(dismiss.frame.size.height >= proposal::metric("size", "size-row-height").unwrap());
    assert!(layout.content.max_y() <= frame.max_y());
    if frame.size.width < 280.0 {
        assert!(layout.content.max_y() + 8.0 <= dismiss.frame.min_y());
    } else {
        assert!(layout.content.max_x() + 4.0 <= dismiss.frame.min_x());
    }
}

#[test]
fn narrow_toast_places_dismiss_after_wrapped_message() {
    let mut view = ToastView::default();
    let model = Toast::new(
        "A long toast message wrapping in a narrow window",
        NoticeTone::Info,
        5000,
    );
    let mut text = WrappingText::default();
    let measured = ToastView::measure(&mut text, &model.message, 220.0, role());
    assert!(measured.height > 62.0);
    let frame = Rect::new(12.0, 16.0, measured.width, measured.height);
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
    let dismiss = view.dismiss_layout.unwrap().frame;
    assert_eq!(layout.frame.size, measured);
    assert!(layout.content.max_x() <= frame.max_x());
    assert!(layout.content.max_y() + 8.0 <= dismiss.min_y());
    assert!(dismiss.max_x() <= frame.max_x());
    assert!(dismiss.max_y() <= frame.max_y());
}

#[test]
fn empty_state_text_uses_symmetric_token_spacing_without_dead_void() {
    let mut view = EmptyStateView::default();
    let model = EmptyState {
        title: "No documents yet".into(),
        detail: "Create a document to begin your workspace.".into(),
        action: Some("Create document".into()),
    };
    let pad = tekes_ui_tokens::Spacing::Regular.value();
    let gap = tekes_ui_tokens::Spacing::Regular.value();
    let frame = Rect::new(36., 28., 488., 164.);
    let mut scene = Scene::new();
    let mut text = RecordingText::default();
    view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let title = text
        .0
        .iter()
        .find(|(drawn, _, _, _)| drawn == &model.title)
        .unwrap();
    let detail = text
        .0
        .iter()
        .find(|(drawn, _, _, _)| drawn == &model.detail)
        .unwrap();
    assert_eq!(title.2.min_x(), frame.min_x() + pad);
    assert_eq!(title.2.min_y(), frame.min_y() + pad);
    assert_eq!(
        title.2.size.height,
        proposal::TextRole::UiHeading.style().line_height
    );
    assert_eq!(detail.2.min_x(), frame.min_x() + pad);
    assert_eq!(detail.2.min_y(), title.2.max_y() + gap);
    assert_eq!(detail.2.size.height, role().style().line_height);
    let measured = EmptyStateView::measure(&mut RecordingText::default(), &model, 488., role());
    assert!(measured.height < 130., "no 54pt detail void: {measured:?}");
}

#[test]
fn empty_state_action_stays_inside_tight_frame() {
    let mut view = EmptyStateView::default();
    let model = EmptyState {
        title: "No documents yet".into(),
        detail: "Create a document to begin your workspace.".into(),
        action: Some("Create document".into()),
    };
    let pad = tekes_ui_tokens::Spacing::Regular.value();
    let frame = Rect::new(36., 28., 488., 116.);
    let mut scene = Scene::new();
    let mut text = FixedText;
    view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let action = view.action_layout.unwrap();
    assert!(action.frame.max_y() <= frame.max_y() - pad + 0.01);
    assert!(action.visible.is_some());
}

#[test]
fn empty_state_wrap_measure_matches_draw_and_action_stays_after_text() {
    let mut view = EmptyStateView::default();
    let model = EmptyState {
        title: "A".repeat(110),
        detail: "Details".into(),
        action: Some("Create document".into()),
    };
    let pad = tekes_ui_tokens::Spacing::Regular.value();
    let frame = Rect::new(20., 16., 500., 148.);
    let title_width = (frame.size.width - pad * 2.).min(420.);
    let mut scene = Scene::new();
    let mut text = WrappingText::default();
    view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let title_measurements: Vec<_> = text
        .measures
        .iter()
        .filter(|(measured, text_role, _)| {
            measured == &model.title && *text_role == proposal::TextRole::UiHeading
        })
        .collect();
    assert!(!title_measurements.is_empty());
    assert!(
        title_measurements
            .iter()
            .all(|(_, _, width)| *width == title_width)
    );
    let title_draw = text
        .draws
        .iter()
        .find(|(drawn, _, _, _)| drawn == &model.title)
        .unwrap();
    assert_eq!(title_draw.2.size.width, title_width);
    assert_eq!(
        title_draw.2.size.height,
        3. * proposal::TextRole::UiHeading.style().line_height
    );
    let detail_draw = text
        .draws
        .iter()
        .find(|(drawn, _, _, _)| drawn == &model.detail)
        .unwrap();
    let action = view.action_layout.unwrap();
    assert!(
        action.frame.min_y()
            >= detail_draw.2.max_y() + tekes_ui_tokens::Spacing::Relaxed.value() - 0.01
    );
    let detail_slot = Rect::new(
        detail_draw.2.min_x(),
        detail_draw.2.min_y(),
        frame.size.width - pad * 2.,
        detail_draw.2.size.height,
    );
    assert!(intersect(action.frame, detail_slot).is_none());
}

#[test]
fn empty_state_follows_parent_detail_width_unless_max_detail_width_configured() {
    let model = EmptyState {
        title: "Empty".into(),
        detail: "A long explanation that can use all available horizontal space".into(),
        action: None,
    };
    let frame = Rect::new(0., 0., 800., 180.);
    let mut scene = Scene::new();
    let mut text = WrappingText::default();

    let mut unconstrained = EmptyStateView::default();
    unconstrained.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let wide_draw = text
        .draws
        .iter()
        .rfind(|(drawn, _, _, _)| drawn == &model.detail)
        .unwrap()
        .2;
    assert_eq!(wide_draw.size.width, 784.);

    text.draws.clear();
    let mut capped = EmptyStateView::default().with_max_detail_width(Some(460.));
    capped.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    let capped_draw = text
        .draws
        .iter()
        .rfind(|(drawn, _, _, _)| drawn == &model.detail)
        .unwrap()
        .2;
    assert_eq!(capped_draw.size.width, 460.);
}

#[test]
fn tooltip_render_consumes_theme_surface_and_text_roles() {
    struct InkText(Vec<tekes_ui_tokens::Rgba>);
    impl ButtonTextRenderer for InkText {
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 8.).min(max_width.max(0.)),
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
            ink: tekes_ui_tokens::Rgba,
        ) {
            self.0.push(ink);
        }
    }

    let viewport = Rect::new(0., 0., 320., 200.);
    let anchor = Rect::new(24., 24., 120., 36.);
    let model = Tooltip {
        visible: true,
        text: "Helpful detail".into(),
    };
    let mut view = TooltipView::default();
    for theme in tekes_ui_tokens::Theme::ALL {
        let palette = Palette::from(theme);
        let mut scene = Scene::new();
        let mut text = InkText(Vec::new());
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(180., 48.),
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        assert_eq!(layout.visible, Some(layout.frame), "{theme:?}");
        let frame_colors = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            "hint",
            "surface-frame",
            "rest",
        )
        .expect("validated Tooltip frame packet");
        let text_colors =
            tekes_ui_tokens::surface_palette::resolve_surface(palette, "hint", "content", "rest")
                .expect("validated Tooltip content packet");
        let shadows = scene
            .primitives()
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Shadow(_)))
            .count();
        if theme.is_increased() {
            assert_eq!(shadows, 0, "{theme:?} IC tooltip should remain shadow-free");
        } else {
            let expected_layers = proposal::elevation("popover", theme).len();
            assert_eq!(
                shadows, expected_layers,
                "{theme:?} ordinary tooltip should use popover elevation layers"
            );
        }
        let background = scene
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Quad(quad) if quad.rect == layout.frame => Some(quad),
                _ => None,
            })
            .expect("public Tooltip consumer paints its background");
        assert_eq!(background.fill, frame_colors.background, "{theme:?}");
        let expected_stroke = crate::paint::decorative_border(palette, text.backing_scale()).0;
        assert_eq!(background.border_width, expected_stroke, "{theme:?}");
        assert_eq!(
            Some(background.border_color),
            frame_colors.boundary,
            "{theme:?}"
        );
        let radius =
            proposal::metric("radius", "radius-floating").expect("generated Tooltip radius");
        assert_eq!(background.corner_radii.top_left, radius, "{theme:?}");
        assert_eq!(text.0, vec![text_colors.text], "{theme:?}");
    }
}
#[test]
fn tooltip_owns_hover_focus_escape_and_leave_state() {
    let anchor = Rect::new(20., 20., 120., 36.);
    let viewport = Rect::new(0., 0., 240., 160.);
    let mut view = TooltipView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let model = Tooltip {
        visible: false,
        text: "Helpful detail".into(),
    };
    assert!(
        view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(150., 48.),
            viewport,
            None,
            palette(),
            role(),
        )
        .visible
        .is_none()
    );
    view.pointer_move(anchor.center());
    assert!(
        view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(150., 48.),
            viewport,
            None,
            palette(),
            role(),
        )
        .visible
        .is_some()
    );
    assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
    assert!(
        view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(150., 48.),
            viewport,
            None,
            palette(),
            role(),
        )
        .visible
        .is_none()
    );
    view.pointer_move(Point::new(230., 150.));
    view.render(
        &mut scene,
        &mut text,
        &model,
        anchor,
        Size::new(150., 48.),
        viewport,
        None,
        palette(),
        role(),
    );
    view.pointer_move(anchor.center());
    assert!(
        view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(150., 48.),
            viewport,
            None,
            palette(),
            role(),
        )
        .visible
        .is_some()
    );
    view.set_focus(true, true);
    assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
    view.pointer_move(Point::new(230., 150.));
    view.set_focus(false, false);
    assert!(
        view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(150., 48.),
            viewport,
            None,
            palette(),
            role(),
        )
        .visible
        .is_none()
    );
}

#[test]
fn popover_content_slot_uses_component_bounds_and_deactivation_restores_focus() {
    let viewport = Rect::new(0., 0., 240., 160.);
    let anchor = Rect::new(210., 135., 20., 20.);
    let mut model = Popover::default();
    model.show(12);
    let mut view = PopoverView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let mut slot = None;
    let layout = view.render_with_content(
        &mut scene,
        &mut text,
        &model,
        anchor,
        Size::new(180., 96.),
        viewport,
        Some(Rect::new(0., 0., 230., 150.)),
        palette(),
        role(),
        |scene, content| {
            slot = Some(content);
            scene.push_quad(Quad::new(content, palette().color("surface-2").unwrap()));
        },
    );
    let content = slot.expect("open popover calls its content slot");
    assert_eq!(content, layout.content);
    assert!(
        layout
            .visible
            .is_some_and(|visible| visible.contains(content.center()))
    );
    assert!(layout.frame.max_x() <= 230.);
    assert!(layout.frame.max_y() <= 150.);
    let dismiss = view.dismiss_layout.unwrap().frame.center();
    assert!(view.pointer_down(&mut model, dismiss));
    assert_eq!(
        view.pointer_up(&mut model, dismiss),
        OverlayEvent::Dismissed {
            restore_focus: Some(12)
        }
    );
    view.show(&mut model, 13);
    assert!(model.open);
    assert_eq!(
        view.set_window_active(false, &mut model),
        OverlayEvent::Dismissed {
            restore_focus: Some(13)
        }
    );
    assert!(!model.open);
}

#[test]
fn popover_surface_uses_floating_role_and_text_backend_scale() {
    let mut model = Popover::default();
    let mut view = PopoverView::default();
    view.show(&mut model, 7);
    for theme in tekes_ui_tokens::Theme::ALL {
        for scale in [1.0, 2.0] {
            let palette = Palette::from(theme);
            let mut text = ScaledText(scale);
            let mut scene = Scene::new();
            let layout = view.render_with_content(
                &mut scene,
                &mut text,
                &model,
                Rect::new(30.0, 24.0, 80.0, 30.0),
                Size::new(180.0, 96.0),
                Rect::new(0.0, 0.0, 320.0, 240.0),
                None,
                palette,
                role(),
                |_, _| {},
            );
            let popover_quad = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    Primitive::Quad(q) if q.rect == layout.frame => Some(q),
                    _ => None,
                })
                .expect("popover surface quad");
            assert_eq!(popover_quad.fill, palette.color("surface-popover").unwrap());
            assert_eq!(
                popover_quad.corner_radii,
                tekes_ui_scene::CornerRadii::uniform(
                    proposal::metric("radius", "radius-floating").unwrap(),
                )
            );
            assert_eq!(
                popover_quad.border_width,
                paint::decorative_border(palette, scale).0
            );
        }
    }
}

#[test]
fn popover_background_uses_effective_parent_viewport_before_content_clip() {
    let anchor = Rect::new(90., 70., 30., 24.);
    let viewport = Rect::new(0., 0., 320., 240.);
    for (clip, parent) in [
        (Rect::new(-20., -20., 360., 280.), viewport),
        (
            Rect::new(20., 10., 240., 200.),
            Rect::new(20., 10., 240., 200.),
        ),
    ] {
        let mut model = Popover::default();
        model.show(1);
        let mut view = PopoverView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let layout = view.render_with_content(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(160., 100.),
            viewport,
            Some(clip),
            palette(),
            role(),
            |_, _| {},
        );
        let visible = layout.visible.expect("popover visible inside parent");
        assert_background_clip(&scene, layout.frame, parent, visible, true);
    }
}

#[test]
fn toast_and_banner_backgrounds_use_parent_clip_before_content_clip() {
    let frame = Rect::new(40., 30., 220., 74.);
    for ancestor in [
        Rect::new(0., 0., 400., 300.),
        Rect::new(80., 40., 140., 60.),
    ] {
        let expected_visible = frame.intersection(&ancestor).unwrap();
        let toast = Toast::new("Saved", NoticeTone::Success, 1_000);
        let mut toast_view = ToastView::default();
        let mut toast_text = FixedText;
        let mut toast_scene = Scene::new();
        let toast_layout = toast_view.render(
            &mut toast_scene,
            &mut toast_text,
            &toast,
            frame,
            Some(ancestor),
            palette(),
            role(),
        );
        assert_eq!(toast_layout.visible, Some(expected_visible));
        assert_background_clip(&toast_scene, frame, ancestor, expected_visible, true);

        let banner = Banner {
            message: "Offline".into(),
            tone: NoticeTone::Warning,
            dismissible: true,
            visible: true,
        };
        let mut banner_view = BannerView::default();
        let mut banner_text = FixedText;
        let mut banner_scene = Scene::new();
        let banner_layout = banner_view.render(
            &mut banner_scene,
            &mut banner_text,
            &banner,
            frame,
            Some(ancestor),
            palette(),
            role(),
        );
        assert_eq!(banner_layout.visible, Some(expected_visible));
        assert_background_clip(&banner_scene, frame, ancestor, expected_visible, true);
    }
}

#[test]
fn popover_measure_matches_rendered_content_slot() {
    let mut model = Popover::default();
    model.show(4);
    let mut view = PopoverView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let requested = Size::new(160., 100.);
    let size = PopoverView::measure(&mut text, requested, role());
    let layout = view.render_with_content(
        &mut scene,
        &mut text,
        &model,
        Rect::new(100., 100., 20., 20.),
        size,
        Rect::new(0., 0., 600., 600.),
        None,
        palette(),
        role(),
        |_, _| {},
    );
    assert_eq!(layout.content.size, requested);
}

#[test]
fn popover_fully_clipped_geometry_never_receives_input() {
    let mut model = Popover::default();
    model.show(3);
    let mut view = PopoverView::default();
    let mut text = FixedText;
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(10., 10., 20., 20.),
        Size::new(120., 80.),
        Rect::new(0., 0., 240., 160.),
        Some(Rect::new(300., 300., 20., 20.)),
        palette(),
        role(),
    );
    assert_eq!(layout.visible, None);
    assert!(
        view.logical_focus_target().is_some(),
        "valid disjoint clip preserves the mounted dismiss frame"
    );
    assert!(!layout.hit_test(Point::new(10., 10.)));
    assert!(!view.pointer_down(&mut model, Point::new(10., 10.)));
    assert_eq!(
        view.pointer_up(&mut model, Point::new(10., 10.)),
        OverlayEvent::None
    );
    assert!(model.open);
    view.focus_dismiss(true, true);
    let mut visible_scene = Scene::new();
    view.render(
        &mut visible_scene,
        &mut text,
        &model,
        Rect::new(10., 10., 20., 20.),
        Size::new(120., 80.),
        Rect::new(0., 0., 240., 160.),
        Some(Rect::new(0., 0., 240., 160.)),
        palette(),
        role(),
    );
    assert!(
        view.dismiss.state.focus_visible,
        "focus ring survives scroll reveal render"
    );
    assert_eq!(
        view.key(&mut model, ControlKey::Enter),
        OverlayEvent::Dismissed {
            restore_focus: Some(3)
        }
    );
}

#[test]
fn offscreen_feedback_actions_keep_logical_focus_until_scroll_reveals_them() {
    let frame = Rect::new(180., 24., 320., 110.);
    let offscreen = Rect::new(0., 0., 140., 180.);
    let visible = Rect::new(0., 0., 600., 240.);
    let mut text = FixedText;
    let mut scene = Scene::new();

    let mut toast = Toast::new("Saved", NoticeTone::Success, 5000);
    let mut toast_view = ToastView::default();
    let size = ToastView::measure(&mut text, &toast.message, frame.size.width, role());
    let frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, size.height);
    toast_view.render(
        &mut scene,
        &mut text,
        &toast,
        frame,
        Some(offscreen),
        palette(),
        role(),
    );
    assert!(
        toast_view.focus_target().is_none(),
        "pointer target stays clipped"
    );
    let toast_target = toast_view
        .logical_focus_target()
        .expect("mounted offscreen action");
    assert!(toast_target.min_x() >= frame.min_x());
    toast_view.focus_dismiss(true, true);
    assert!(toast_view.dismiss.state.focus_visible);
    toast_view.render(
        &mut scene,
        &mut text,
        &toast,
        frame,
        Some(visible),
        palette(),
        role(),
    );
    assert!(toast_view.focus_target().is_some());
    assert!(
        toast_view.dismiss.state.focus_visible,
        "focus ring survives reveal render"
    );
    assert!(!toast_view.key(&mut toast, ControlKey::Space, true));
    assert!(toast_view.key(&mut toast, ControlKey::Space, false));
    assert!(
        !toast.visible,
        "focused dismiss action activates with Space"
    );

    let mut banner = Banner {
        message: "Offline".into(),
        tone: NoticeTone::Warning,
        dismissible: true,
        visible: true,
    };
    let mut banner_view = BannerView::default();
    let size = BannerView::measure(&mut text, &banner.message, frame.size.width, true, role());
    let banner_frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, size.height);
    banner_view.render(
        &mut scene,
        &mut text,
        &banner,
        banner_frame,
        Some(offscreen),
        palette(),
        role(),
    );
    let banner_target = banner_view
        .logical_focus_target()
        .expect("mounted banner dismiss action");
    assert!(banner_view.focus_target().is_none());
    banner_view.focus_dismiss(true, true);
    banner_view.render(
        &mut scene,
        &mut text,
        &banner,
        banner_frame,
        Some(visible),
        palette(),
        role(),
    );
    assert!(banner_view.dismiss_button.state.focus_visible);
    assert!(!banner_view.key(&mut banner, ControlKey::Space, true));
    assert!(banner_view.key(&mut banner, ControlKey::Space, false));
    assert!(!banner.visible);
    assert!(banner_target.size.width > 0.);

    let mut empty_view = EmptyStateView::default();
    let empty = EmptyState {
        title: "No items".into(),
        detail: "Try again".into(),
        action: Some("Create".into()),
    };
    let empty_frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, 260.);
    empty_view.render(
        &mut scene,
        &mut text,
        &empty,
        empty_frame,
        Some(offscreen),
        palette(),
        role(),
    );
    assert!(empty_view.focus_target().is_none());
    assert!(empty_view.logical_focus_target().is_some());
    empty_view.focus_action(true, true);
    assert!(
        empty_view.action.state.focus_visible,
        "offscreen focus must be accepted from mounted logical action"
    );
    empty_view.render(
        &mut scene,
        &mut text,
        &empty,
        empty_frame,
        Some(visible),
        palette(),
        role(),
    );
    assert!(empty_view.action.state.focus_visible);
    assert!(!empty_view.key(ControlKey::Space, true));
    assert!(empty_view.key(ControlKey::Space, false));

    let no_action = EmptyState {
        action: None,
        ..empty.clone()
    };
    empty_view.render(
        &mut scene,
        &mut text,
        &no_action,
        empty_frame,
        Some(visible),
        palette(),
        role(),
    );
    assert!(
        empty_view.logical_focus_target().is_none(),
        "no action is not mounted"
    );

    let invalid_clips = [
        Rect::new(0., 0., 0., 100.),
        Rect::new(f32::NAN, 0., 100., 100.),
    ];
    for clip in invalid_clips {
        let visible_toast = Toast::new("Still visible", NoticeTone::Info, 5000);
        let mut toast_view = ToastView::default();
        toast_view.render(
            &mut scene,
            &mut text,
            &visible_toast,
            empty_frame,
            Some(clip),
            palette(),
            role(),
        );
        assert!(
            toast_view.logical_focus_target().is_none(),
            "invalid clip {clip:?}"
        );
        let mut banner_view = BannerView::default();
        let visible_banner = Banner {
            message: "Still visible".into(),
            tone: NoticeTone::Info,
            dismissible: true,
            visible: true,
        };
        banner_view.render(
            &mut scene,
            &mut text,
            &visible_banner,
            banner_frame,
            Some(clip),
            palette(),
            role(),
        );
        assert!(
            banner_view.logical_focus_target().is_none(),
            "invalid clip {clip:?}"
        );
    }
    for invalid_frame in [
        Rect::new(0., 0., 0., 100.),
        Rect::new(f32::NAN, 0., 100., 100.),
    ] {
        let visible_toast = Toast::new("Still visible", NoticeTone::Info, 5000);
        let mut toast_view = ToastView::default();
        toast_view.render(
            &mut scene,
            &mut text,
            &visible_toast,
            invalid_frame,
            Some(visible),
            palette(),
            role(),
        );
        assert!(
            toast_view.logical_focus_target().is_none(),
            "invalid toast frame {invalid_frame:?}"
        );
        let mut banner_view = BannerView::default();
        let visible_banner = Banner {
            message: "Still visible".into(),
            tone: NoticeTone::Info,
            dismissible: true,
            visible: true,
        };
        banner_view.render(
            &mut scene,
            &mut text,
            &visible_banner,
            invalid_frame,
            Some(visible),
            palette(),
            role(),
        );
        assert!(
            banner_view.logical_focus_target().is_none(),
            "invalid banner frame {invalid_frame:?}"
        );
        let mut empty_view = EmptyStateView::default();
        empty_view.render(
            &mut scene,
            &mut text,
            &empty,
            invalid_frame,
            Some(visible),
            palette(),
            role(),
        );
        assert!(
            empty_view.logical_focus_target().is_none(),
            "invalid empty-state frame {invalid_frame:?}"
        );
    }
    let mut hidden_toast = Toast::new("Hidden", NoticeTone::Info, 5000);
    hidden_toast.visible = false;
    let mut hidden_view = ToastView::default();
    hidden_view.render(
        &mut scene,
        &mut text,
        &hidden_toast,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(
        hidden_view.logical_focus_target().is_none(),
        "hidden toast is unmounted"
    );
    let mut disabled_empty = EmptyStateView::default();
    disabled_empty.set_disabled(true);
    disabled_empty.render(
        &mut scene,
        &mut text,
        &empty,
        frame,
        Some(visible),
        palette(),
        role(),
    );
    assert!(
        disabled_empty.logical_focus_target().is_none(),
        "disabled action is ineligible"
    );
}

#[test]
fn empty_state_action_requires_a_real_visible_pointer_press() {
    let mut view = EmptyStateView::default();
    let model = EmptyState {
        title: "No documents".into(),
        detail: "Create a document to get started.".into(),
        action: Some("Create document".into()),
    };
    let mut scene = Scene::new();
    let mut text = FixedText;
    let outer = Rect::new(10., 20., 520., 164.);
    let mut surface = SurfaceView::default();
    let content = surface
        .render(&mut scene, outer, None, palette(), true)
        .content;
    view.render(
        &mut scene,
        &mut text,
        &model,
        content,
        None,
        palette(),
        role(),
    );
    let title = view.layout.content;
    assert_eq!(title.min_x(), outer.min_x() + 24.);
    assert_eq!(title.min_y(), outer.min_y() + 24.);
    let point = view.action_layout.unwrap().frame.center();
    assert_eq!(
        view.action_layout.unwrap().frame.min_x(),
        outer.min_x() + 24.
    );
    assert_eq!(
        view.action_layout.unwrap().frame.min_y(),
        outer.min_y() + 82.
    );
    let primary_fill = palette().color("action-primary-ground").unwrap();
    assert!(scene.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Quad(q) if q.rect == view.action_layout.unwrap().frame && q.fill == primary_fill)
    }));
    view.pointer_move(point);
    assert!(view.action.state.hovered);
    assert_eq!(view.focus_target(), view.action_layout.unwrap().visible);
    assert!(!view.pointer_up(point));
    assert!(view.pointer_down(point));
    assert!(view.action.state.pressed);
    let outside = Point::new(350., 220.);
    view.pointer_move(outside);
    assert!(!view.action.state.pressed);
    assert!(!view.pointer_up(outside));
    assert!(!view.action.state.pressed);
    view.pointer_move(point);
    assert!(view.action.state.hovered);
    assert!(view.pointer_down(point));
    assert!(view.pointer_up(point));

    view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(0., 0., 260., 180.),
        Some(Rect::new(400., 400., 20., 20.)),
        palette(),
        role(),
    );
    assert!(view.action_layout.is_none());
    assert!(!view.pointer_down(point));
    assert!(!view.pointer_up(point));

    view.render(
        &mut scene,
        &mut text,
        &model,
        Rect::new(0., 0., 260., 180.),
        None,
        palette(),
        role(),
    );
    let point = view.action_layout.unwrap().frame.center();
    view.pointer_move(point);
    assert!(view.action.state.hovered);
    view.set_disabled(true);
    view.pointer_move(point);
    assert!(!view.action.state.hovered);
    assert!(!view.pointer_down(point));
    assert!(!view.pointer_up(point));
}

#[test]
fn popover_uses_symmetric_token_insets_and_empty_state_detail_uses_muted_ink() {
    let mut popover = PopoverView::default();
    let mut pmodel = Popover::default();
    popover.show(&mut pmodel, 1);
    let p_layout = popover.render(
        &mut Scene::new(),
        &mut FixedText,
        &pmodel,
        Rect::new(10., 10., 20., 20.),
        Size::new(260., 180.),
        Rect::new(0., 0., 800., 600.),
        None,
        palette(),
        role(),
    );
    let left_margin = p_layout.content.min_x() - p_layout.frame.min_x();
    let right_margin = p_layout.frame.max_x() - p_layout.content.max_x();
    let top_margin = p_layout.content.min_y() - p_layout.frame.min_y();
    let dismiss = popover.dismiss_layout.unwrap().frame;
    let action_left = dismiss.min_x() - p_layout.frame.min_x();
    let action_bottom = p_layout.frame.max_y() - dismiss.max_y();
    assert!((left_margin - right_margin).abs() <= 0.01);
    assert!((left_margin - top_margin).abs() <= 0.01);
    assert!((left_margin - action_left).abs() <= 0.01);
    assert!((top_margin - action_bottom).abs() <= 0.01);
    assert!(dismiss.max_x() <= p_layout.frame.max_x() - left_margin + 0.01);

    let mut empty = EmptyStateView::default();
    let emodel = EmptyState {
        title: "Nothing here".into(),
        detail: "Muted helper copy".into(),
        action: None,
    };
    struct InkText(Vec<(String, tekes_ui_tokens::Rgba)>);
    impl ButtonTextRenderer for InkText {
        fn measure(&mut self, value: &str, role: proposal::TextRole, max_width: f32) -> Size {
            FixedText.measure(value, role, max_width)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            value: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            ink: tekes_ui_tokens::Rgba,
        ) {
            self.0.push((value.to_owned(), ink));
        }
    }
    let mut text = InkText(Vec::new());
    empty.render(
        &mut Scene::new(),
        &mut text,
        &emodel,
        Rect::new(0., 0., 360., 160.),
        None,
        palette(),
        role(),
    );
    let primary =
        tekes_ui_tokens::surface_palette::resolve_surface(palette(), "canvas", "content", "rest")
            .unwrap()
            .text;
    let muted = tekes_ui_tokens::surface_palette::resolve_surface(
        palette(),
        "canvas",
        "data-category-label",
        "rest",
    )
    .unwrap()
    .text;
    let title_ink = text.0.iter().find(|(s, _)| s == &emodel.title).unwrap().1;
    let detail_ink = text.0.iter().find(|(s, _)| s == &emodel.detail).unwrap().1;
    assert_eq!(title_ink, primary);
    assert_eq!(detail_ink, muted);
}

#[test]
fn toast_warning_marker_matches_banner_warning_and_differs_from_error_across_all_themes() {
    for theme in tekes_ui_tokens::Theme::ALL {
        let palette = Palette::from(theme);
        for surface in tekes_ui_tokens::surface_palette::surface_names() {
            let toast_warn = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                surface,
                "notice-marker-toast-warning",
                "rest",
            )
            .unwrap()
            .marker
            .unwrap();
            let banner_warn = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                surface,
                "notice-marker-banner-warning",
                "rest",
            )
            .unwrap()
            .marker
            .unwrap();
            let toast_err = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                surface,
                "notice-marker-error",
                "rest",
            )
            .unwrap()
            .marker
            .unwrap();
            assert_eq!(toast_warn, banner_warn, "{theme:?}/{surface}");
            assert_ne!(toast_warn, toast_err, "{theme:?}/{surface}");
        }
    }
}

#[test]
fn toast_view_paints_floating_elevation_in_ordinary_modes_and_shadow_free_in_ic() {
    for theme in tekes_ui_tokens::Theme::ALL {
        let palette = Palette::from(theme);
        let mut view = ToastView::default();
        let model = Toast::new("Check configuration", NoticeTone::Warning, 5000);
        let mut text = FixedText;
        let frame = Rect::new(20., 20., 320., 64.);
        let mut scene = Scene::new();
        view.render(&mut scene, &mut text, &model, frame, None, palette, role());
        let shadows = scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Shadow(_)))
            .count();
        if theme.is_increased() {
            assert_eq!(shadows, 0, "{theme:?} IC toast should remain shadow-free");
        } else {
            let expected_layers = proposal::elevation("popover", theme).len();
            assert_eq!(
                shadows, expected_layers,
                "{theme:?} ordinary toast should use popover elevation layers"
            );
        }
        let surface_quad = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                Primitive::Quad(q) if q.rect == frame => Some(*q),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            surface_quad.corner_radii.top_left,
            proposal::metric("radius", "radius-floating").unwrap()
        );
    }
}

#[test]
fn toast_view_overlay_presence_animates_enter_exit_and_snaps_under_reduced_motion() {
    #[derive(Default)]
    struct ClockText {
        ms: u64,
        reduced: bool,
        requests: usize,
    }
    impl ButtonTextRenderer for ClockText {
        fn motion_context(&self) -> Option<crate::ControlMotionContext> {
            Some(crate::ControlMotionContext {
                now: std::time::Duration::from_millis(self.ms),
                reduced_motion: self.reduced,
            })
        }
        fn request_animation_frame(&mut self) {
            self.requests += 1;
        }
        fn measure(&mut self, value: &str, role: proposal::TextRole, max: f32) -> Size {
            FixedText.measure(value, role, max)
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

    let mut view = ToastView::default();
    let mut model = Toast::new("Saved", NoticeTone::Warning, 5000);
    let mut text = ClockText::default();
    let frame = Rect::new(20., 20., 300., 60.);

    // Frame 0: visible -> enters from 0 -> 1 via overlay_presence.
    let mut s0 = Scene::new();
    view.render(&mut s0, &mut text, &model, frame, None, palette(), role());
    assert!(view.needs_animation());
    assert!(text.requests > 0);

    // Mid-exit after dismissal while animated.
    text.ms = 200;
    let mut s_full = Scene::new();
    view.render(
        &mut s_full,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(!view.needs_animation());

    model.dismiss();
    text.ms = 250;
    text.requests = 0;
    let mut s_exit = Scene::new();
    view.render(
        &mut s_exit,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(view.is_presented());
    assert!(view.needs_animation());
    assert!(!s_exit.primitives().is_empty());

    // After duration completes, exit completes and stops painting.
    text.ms = 500;
    let mut s_done = Scene::new();
    view.render(
        &mut s_done,
        &mut text,
        &model,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(!view.is_presented());
    assert!(!view.needs_animation());
    assert!(s_done.primitives().is_empty());

    // Reduced motion snaps immediately with no animation frame requested.
    let mut rm_view = ToastView::default();
    let mut rm_model = Toast::new("Saved", NoticeTone::Warning, 5000);
    let mut rm_text = ClockText {
        ms: 0,
        reduced: true,
        requests: 0,
    };
    let mut s_rm = Scene::new();
    rm_view.render(
        &mut s_rm,
        &mut rm_text,
        &rm_model,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(!rm_view.needs_animation());
    assert_eq!(rm_text.requests, 0);
    rm_model.dismiss();
    rm_text.ms = 16;
    let mut s_rm_closed = Scene::new();
    rm_view.render(
        &mut s_rm_closed,
        &mut rm_text,
        &rm_model,
        frame,
        None,
        palette(),
        role(),
    );
    assert!(!rm_view.is_presented());
    assert!(s_rm_closed.primitives().is_empty());
}

#[test]
fn tooltip_view_overlay_presence_animates_enter_exit_and_snaps_under_reduced_motion() {
    #[derive(Default)]
    struct ClockText {
        ms: u64,
        reduced: bool,
        requests: usize,
    }
    impl ButtonTextRenderer for ClockText {
        fn motion_context(&self) -> Option<crate::ControlMotionContext> {
            Some(crate::ControlMotionContext {
                now: std::time::Duration::from_millis(self.ms),
                reduced_motion: self.reduced,
            })
        }
        fn request_animation_frame(&mut self) {
            self.requests += 1;
        }
        fn measure(&mut self, value: &str, role: proposal::TextRole, max: f32) -> Size {
            FixedText.measure(value, role, max)
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

    let anchor = Rect::new(24., 24., 120., 32.);
    let viewport = Rect::new(0., 0., 360., 220.);
    let mut view = TooltipView::default();
    let mut model = Tooltip {
        visible: true,
        text: "Helpful detail".into(),
    };
    let mut text = ClockText::default();

    let mut s0 = Scene::new();
    let l0 = view.render(
        &mut s0,
        &mut text,
        &model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(l0.visible.is_some());
    assert!(view.needs_animation());
    assert!(text.requests > 0);

    text.ms = 200;
    let mut s_full = Scene::new();
    view.render(
        &mut s_full,
        &mut text,
        &model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(!view.needs_animation());

    // Dismissing triggers exit fade while layout.visible is None and is_presented is true.
    model.visible = false;
    text.ms = 250;
    text.requests = 0;
    let mut s_exit = Scene::new();
    let l_exit = view.render(
        &mut s_exit,
        &mut text,
        &model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert_eq!(l_exit.visible, None);
    assert!(view.is_presented());
    assert!(view.needs_animation());
    assert!(text.requests > 0);
    assert!(!s_exit.primitives().is_empty());

    // After duration completes, exit settles cleanly.
    text.ms = 500;
    let mut s_done = Scene::new();
    view.render(
        &mut s_done,
        &mut text,
        &model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(!view.is_presented());
    assert!(!view.needs_animation());
    assert!(s_done.primitives().is_empty());

    // Reduced motion snaps enter & exit immediately with zero frame requests.
    let mut rm_view = TooltipView::default();
    let mut rm_model = Tooltip {
        visible: true,
        text: "Helpful detail".into(),
    };
    let mut rm_text = ClockText {
        ms: 0,
        reduced: true,
        requests: 0,
    };
    let mut s_rm = Scene::new();
    rm_view.render(
        &mut s_rm,
        &mut rm_text,
        &rm_model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(!rm_view.needs_animation());
    assert_eq!(rm_text.requests, 0);
    rm_model.visible = false;
    rm_text.ms = 16;
    let mut s_rm_closed = Scene::new();
    rm_view.render(
        &mut s_rm_closed,
        &mut rm_text,
        &rm_model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(!rm_view.is_presented());
    assert!(s_rm_closed.primitives().is_empty());

    // Window deactivation immediately clears presentation.
    let mut active_view = TooltipView::default();
    let active_model = Tooltip {
        visible: true,
        text: "Helpful detail".into(),
    };
    let mut active_text = ClockText::default();
    active_view.render(
        &mut Scene::new(),
        &mut active_text,
        &active_model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    active_text.ms = 200;
    active_view.render(
        &mut Scene::new(),
        &mut active_text,
        &active_model,
        anchor,
        Size::new(160., 44.),
        viewport,
        None,
        palette(),
        role(),
    );
    assert!(active_view.is_presented());
    active_view.set_window_active(false);
    assert!(!active_view.is_presented());
}

#[test]
fn dialog_view_default_buttons_use_popover_surface_contract() {
    for theme in tekes_ui_tokens::Theme::ALL {
        let palette = Palette::from(theme);
        let mut view = DialogView::default();
        view.show(1);
        let mut scene = Scene::new();
        let mut text = FixedText;
        let layout = view
            .render(
                &mut scene,
                &mut text,
                "Confirm changes",
                Rect::new(0., 0., 640., 480.),
                None,
                palette,
                role(),
                false,
            )
            .expect("open Dialog layout");
        let cancel_frame = layout.buttons[0].frame;
        let cancel_quad = scene
            .primitives()
            .iter()
            .rev()
            .find_map(|p| match p {
                Primitive::Quad(q) if q.rect == cancel_frame && q.fill.a > 0. => Some(*q),
                _ => None,
            })
            .expect("Dialog Cancel button background quad");
        let expected_cancel = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            "popover",
            "action-quiet",
            "rest",
        )
        .unwrap()
        .background;
        assert_eq!(cancel_quad.fill, expected_cancel, "{theme:?}");
        if theme != tekes_ui_tokens::Theme::LIGHT_INCREASED {
            assert_ne!(
                cancel_quad.fill,
                palette.color("surface-2").unwrap(),
                "{theme:?} Cancel button should inherit popover surface rather than surface-2"
            );
        }
    }
}

#[test]
fn tooltip_popover_and_empty_state_views_use_tight_tokens_and_natural_dismiss_width() {
    struct TinyText;
    impl ButtonTextRenderer for TinyText {
        fn measure(&mut self, value: &str, _: proposal::TextRole, max_width: f32) -> Size {
            Size::new((value.chars().count() as f32 * 6.0).min(max_width), 10.0)
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

    let tip = Tooltip {
        visible: true,
        text: "Hi".into(),
    };
    let fitted = TooltipView::fitted_size(
        &mut TinyText,
        &tip,
        Rect::new(0., 0., 200., 100.),
        proposal::TextRole::UiCaption,
    );
    assert_eq!(fitted.width, 12.0 + 8.0 * 2.0);
    assert_eq!(
        fitted.height,
        proposal::metric("size", "size-compact-row").unwrap()
    );
    let mut tip_view = TooltipView::default();
    let tip_layout = tip_view.render(
        &mut Scene::new(),
        &mut TinyText,
        &tip,
        Rect::new(20., 20., 40., 24.),
        fitted,
        Rect::new(0., 0., 300., 200.),
        None,
        palette(),
        proposal::TextRole::UiCaption,
    );
    assert_eq!(
        tip_layout.frame.min_y() - 44.0,
        tekes_ui_tokens::Spacing::Snug.value()
    );

    let pop_measured = PopoverView::measure(&mut TinyText, Size::new(60., 40.), role());
    let natural_dismiss =
        ButtonModel::measure(&mut TinyText, "Dismiss", f32::MAX, ButtonSize::Md, role()).width;
    assert_eq!(
        pop_measured.width,
        (60.0_f32 + 12.0 * 2.0).max(natural_dismiss + 12.0 * 2.0)
    );
    assert!(pop_measured.width < 120.0 + 12.0 * 2.0);
}
