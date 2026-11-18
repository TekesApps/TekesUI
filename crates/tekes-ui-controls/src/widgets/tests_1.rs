use super::*;
use tekes_ui_scene::Primitive;
use tekes_ui_tokens::Palette;
use tekes_ui_tokens::{Theme, proposal};

pub(super) struct FixedText {
    pub(super) measured: Size,
    pub(super) drawn: Option<(Rect, ButtonTextAlign)>,
}

pub(super) struct CheckboxText {
    pub(super) measured: Size,
    pub(super) first_line: f32,
    pub(super) backing_scale: f32,
    pub(super) drawn: Option<(Rect, Rgba)>,
}

impl ButtonTextRenderer for CheckboxText {
    fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(self.measured.width.min(max_width), self.measured.height)
    }

    fn backing_scale(&self) -> f32 {
        self.backing_scale
    }

    fn first_line_height(
        &mut self,
        _text: &str,
        _role: proposal::TextRole,
        _max_width: f32,
    ) -> f32 {
        self.first_line
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        rect: Rect,
        _align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.drawn = Some((rect, ink));
    }
}

impl ButtonTextRenderer for FixedText {
    fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
        Size::new(self.measured.width.min(max_width), self.measured.height)
    }

    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        _ink: Rgba,
    ) {
        self.drawn = Some((rect, align));
    }
}

#[derive(Default)]
pub(super) struct MotionText {
    pub(super) context: Option<ControlMotionContext>,
    pub(super) requested_frames: usize,
}

impl ButtonTextRenderer for MotionText {
    fn measure(&mut self, _text: &str, _role: proposal::TextRole, _max_width: f32) -> Size {
        Size::new(48., 20.)
    }

    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.context
    }

    fn request_animation_frame(&mut self) {
        self.requested_frames += 1;
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

#[test]
fn button_component_transitions_are_interactive_interruptible_and_reduced_motion_aware() {
    let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
    let frame = Rect::new(0., 0., 120., 40.);
    let point = Point::new(60., 20.);
    let mut button = ButtonModel::default();
    let mut text = MotionText {
        context: Some(ControlMotionContext {
            now: Duration::ZERO,
            reduced_motion: false,
        }),
        ..Default::default()
    };
    let mut scene = Scene::new();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        palette,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    button.pointer_move_at(point);
    text.context = Some(ControlMotionContext {
        now: Duration::from_millis(1),
        reduced_motion: false,
    });
    scene.clear();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        palette,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    assert!(button.needs_animation());
    assert!(text.requested_frames > 0);
    button.tick(Duration::from_millis(100));
    let hover_midflight = button.hover_transition.value();
    assert!(hover_midflight > 0.0 && hover_midflight < 1.0);

    assert!(button.pointer_down_at(point));
    text.context = Some(ControlMotionContext {
        now: Duration::from_millis(100),
        reduced_motion: false,
    });
    scene.clear();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        palette,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    button.tick(Duration::from_millis(130));
    let press_midflight = button.press_transition.value();
    assert!(press_midflight > 0.0 && press_midflight < 1.0);
    assert!(
        button.pointer_up_at(point),
        "activation stays immediate while paint is transitioning"
    );

    button.set_reduced_motion(true, Duration::from_millis(130));
    assert!(!button.needs_animation());
    assert_eq!(button.press_transition.value(), 0.0);

    button.state.focused = true;
    button.state.focus_visible = true;
    assert!(button.pointer_down_at(point));
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        Some(Rect::new(500., 500., 1., 1.)),
        palette,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    assert!(
        !button.state.focused,
        "fully clipped controls release focus"
    );
    assert!(
        !button.pointer_up_at(point),
        "a hidden press capture cannot activate"
    );
}

#[test]
fn button_theme_and_palette_source_changes_snap_transitions_and_repaint_from_new_palette() {
    let light = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
    let dark = Palette::new(Theme::DARK, 0).expect("registered Gallery brand");
    let alternate_brand = Palette::new(Theme::DARK, 1).expect("registered Gallery brand");
    let frame = Rect::new(0., 0., 120., 40.);
    let point = Point::new(60., 20.);
    let mut button = ButtonModel::default();
    let mut text = MotionText {
        context: Some(ControlMotionContext {
            now: Duration::ZERO,
            reduced_motion: false,
        }),
        ..Default::default()
    };
    let mut scene = Scene::new();

    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        light,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    button.pointer_move_at(point);
    text.context = Some(ControlMotionContext {
        now: Duration::from_millis(1),
        reduced_motion: false,
    });
    scene.clear();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        light,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    button.tick(Duration::from_millis(100));
    assert!(button.hover_transition.value() > 0.0);
    assert!(button.hover_transition.value() < 1.0);

    text.context = Some(ControlMotionContext {
        now: Duration::from_millis(101),
        reduced_motion: false,
    });
    scene.clear();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        dark,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    assert_eq!(button.hover_transition.value(), 1.0);
    let hover = scene
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Quad(quad) => Some(quad),
            _ => None,
        })
        .expect("button background");
    assert!(!button.needs_animation());
    assert_eq!(hover.fill, dark.color("button-hover").expect("hover role"));

    assert!(button.pointer_down_at(point));
    text.context = Some(ControlMotionContext {
        now: Duration::from_millis(102),
        reduced_motion: false,
    });
    scene.clear();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        dark,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    button.tick(Duration::from_millis(130));
    assert!(button.press_transition.value() > 0.0);
    assert!(button.press_transition.value() < 1.0);

    text.context = Some(ControlMotionContext {
        now: Duration::from_millis(131),
        reduced_motion: false,
    });
    scene.clear();
    button.render(
        &mut scene,
        &mut text,
        "Run",
        frame,
        None,
        alternate_brand,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );
    assert_eq!(button.press_transition.value(), 1.0);
    assert_eq!(
        button.motion_palette,
        Some((Theme::DARK, alternate_brand.source()))
    );
    let pressed = scene
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Quad(quad) => Some(quad),
            _ => None,
        })
        .expect("button background");
    assert!(!button.needs_animation());
    assert_eq!(
        pressed.fill,
        alternate_brand
            .color("button-pressed")
            .expect("pressed role"),
    );
}

#[test]
fn invalid_button_layout_cannot_keep_keyboard_activation_or_motion() {
    let mut button = ButtonModel::default();
    button.state.focused = true;
    button.state.focus_visible = true;
    let mut text = FixedText {
        measured: Size::new(20., 16.),
        drawn: None,
    };
    button.render(
        &mut Scene::new(),
        &mut text,
        "Hidden",
        Rect::new(f32::NAN, 0., 80., 32.),
        None,
        tekes_ui_tokens::Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::default(),
    );
    assert!(!button.state.focused);
    assert!(!button.needs_animation());
    assert!(!button.key(ControlKey::Enter, true));
    assert!(!button.key(ControlKey::Space, true));
    assert!(!button.key(ControlKey::Space, false));
}

#[test]
fn button_sizes_consume_their_own_radius_roles_for_base_and_focus() {
    for (size, expected) in [
        (ButtonSize::Xs, 6.),
        (ButtonSize::Sm, 8.),
        (ButtonSize::Md, 8.),
        (ButtonSize::Lg, 12.),
        (ButtonSize::Body, 8.),
    ] {
        let mut text = FixedText {
            measured: Size::new(20., 16.),
            drawn: None,
        };
        let mut scene = Scene::new();
        let mut button = ButtonModel::default();
        button.state.focused = true;
        button.state.focus_visible = true;
        let layout = button.render(
            &mut scene,
            &mut text,
            "Label",
            Rect::new(20., 20., 180., 48.),
            None,
            tekes_ui_tokens::Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            size,
            proposal::TextRole::UiBody,
            ButtonTextAlign::default(),
        );
        let quads = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                tekes_ui_scene::Primitive::Quad(q) => Some(q),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(quads[0].rect, layout.frame);
        assert_eq!(
            quads[0].corner_radii,
            tekes_ui_scene::CornerRadii::uniform(expected)
        );
        assert_eq!(
            quads[1].corner_radii,
            tekes_ui_scene::CornerRadii::uniform(expected + 4.)
        );
        assert!(layout.hit_test(layout.frame.center()));
    }
}

#[test]
fn rendered_button_owns_large_text_layout_clipping_and_hit_geometry() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(88.0, 54.0),
        drawn: None,
    };
    let mut scene = Scene::new();
    let layout = button.render(
        &mut scene,
        &mut text,
        "Large label",
        Rect::new(10.0, 20.0, 140.0, 32.0),
        Some(Rect::new(0.0, 30.0, 300.0, 100.0)),
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Center,
    );

    assert!(
        layout.frame.size.height
            >= 54.0 + ButtonSize::Md.vertical_inset(proposal::TextRole::UiBody) * 2.0
    );
    assert_eq!(button.layout(), Some(layout));
    assert_eq!(
        layout.visible,
        Some(Rect::new(
            10.0,
            30.0,
            140.0,
            layout.frame.size.height - 10.0
        ))
    );
    assert!(layout.hit_test(Point::new(20.0, 31.0)));
    assert!(!layout.hit_test(Point::new(20.0, 29.0)));
    assert_eq!(text.drawn, Some((layout.content, ButtonTextAlign::Center)));
    assert!(
        scene
            .primitives()
            .iter()
            .any(|primitive| matches!(primitive, Primitive::PushClip(_)))
    );
}

#[test]
fn fractional_button_background_uses_only_the_ancestor_clip() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, proposal::TextRole::UiBody.style().line_height),
        drawn: None,
    };
    let parent = Rect::new(10.0, 10.0, 120.0, 80.0);
    let frame = Rect::new(24.1875, 24.3125, 80.0, 36.0);
    let mut scene = Scene::new();
    let layout = button.render(
        &mut scene,
        &mut text,
        "Action",
        frame,
        Some(parent),
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Leading,
    );

    assert_eq!(layout.frame, frame);
    assert_eq!(layout.visible, Some(frame));
    assert!(layout.hit_test(frame.center()));
    assert!(!layout.hit_test(Point::new(frame.min_x() - 0.01, frame.center().y)));
    assert_eq!(text.drawn, Some((layout.content, ButtonTextAlign::Leading)));
    let clips: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::PushClip(clip) => Some(*clip),
            _ => None,
        })
        .collect();
    assert_eq!(clips, [parent, frame]);
    assert!(matches!(
        scene.primitives().first(),
        Some(Primitive::PushClip(clip)) if *clip == parent
    ));
    assert!(matches!(
        scene.primitives().get(1),
        Some(Primitive::Quad(quad)) if quad.rect == frame
    ));
}

#[test]
fn button_background_remains_clipped_by_a_real_parent_viewport_edge() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, proposal::TextRole::UiBody.style().line_height),
        drawn: None,
    };
    let parent = Rect::new(30.0, 30.0, 50.0, 50.0);
    let frame = Rect::new(24.1875, 24.3125, 80.0, 36.0);
    let mut scene = Scene::new();
    let layout = button.render(
        &mut scene,
        &mut text,
        "Action",
        frame,
        Some(parent),
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Md,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Leading,
    );

    assert_eq!(layout.visible, Some(Rect::new(30.0, 30.0, 50.0, 30.3125)));
    assert!(!layout.hit_test(Point::new(29.99, 40.0)));
    let clips: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::PushClip(clip) => Some(*clip),
            _ => None,
        })
        .collect();
    assert_eq!(clips, [parent, layout.visible.unwrap()]);
    assert!(matches!(
        scene.primitives().first(),
        Some(Primitive::PushClip(clip)) if *clip == parent
    ));
    assert!(matches!(
        scene.primitives().get(1),
        Some(Primitive::Quad(quad)) if quad.rect == frame
    ));
}

#[test]
fn checkbox_indicator_uses_parent_clip_while_label_and_hits_keep_row_clip() {
    let mut checkbox = Checkbox::default();
    let mut text = CheckboxText {
        measured: Size::new(40.0, proposal::TextRole::UiBody.style().line_height),
        first_line: proposal::TextRole::UiBody.style().line_height,
        backing_scale: 1.0,
        drawn: None,
    };
    let parent = Rect::new(10.0, 10.0, 120.0, 80.0);
    let frame = Rect::new(24.1875, 24.3125, 80.0, 36.0);
    let mut scene = Scene::new();
    let layout = checkbox.render(
        &mut scene,
        &mut text,
        "Option",
        frame,
        Some(parent),
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );

    assert_eq!(layout.frame, frame);
    assert_eq!(layout.visible, Some(frame));
    assert!(layout.hit_test(frame.center()));
    assert!(!layout.hit_test(Point::new(frame.min_x() - 0.01, frame.center().y)));
    assert!(text.drawn.is_some_and(|(rect, _)| rect == layout.label));
    let clips: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::PushClip(clip) => Some(*clip),
            _ => None,
        })
        .collect();
    assert_eq!(clips, [parent, frame]);
    assert!(matches!(
        scene.primitives().first(),
        Some(Primitive::PushClip(clip)) if *clip == parent
    ));
    assert!(matches!(
        scene.primitives().get(1),
        Some(Primitive::Quad(quad)) if quad.rect == layout.control
    ));
}

#[test]
fn checkbox_indicator_clip_preserves_narrow_rows_with_or_without_ancestor() {
    for width in [0.0, 8.0, 16.0, 220.0] {
        for ancestor in [None, Some(Rect::new(10.0, 10.0, 300.0, 100.0))] {
            let frame = Rect::new(24.1875, 24.3125, width, 36.0);
            let mut checkbox = Checkbox::default();
            let mut text = CheckboxText {
                measured: Size::new(80.0, proposal::TextRole::UiBody.style().line_height),
                first_line: proposal::TextRole::UiBody.style().line_height,
                backing_scale: 1.0,
                drawn: None,
            };
            let mut scene = Scene::new();
            let layout = checkbox.render(
                &mut scene,
                &mut text,
                "Option",
                frame,
                ancestor,
                Theme::LIGHT,
                proposal::TextRole::UiBody,
            );

            if width == 0.0 {
                assert_eq!(layout.visible, None);
                assert!(scene.is_empty());
                assert_eq!(text.drawn, None);
                continue;
            }
            assert_eq!(layout.visible, Some(frame));
            assert!(layout.hit_test(frame.center()));
            let clips: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::PushClip(clip) => Some(*clip),
                    _ => None,
                })
                .collect();

            if width < 16.0 {
                // The fixed-size mark overflows a narrow row, so paint is
                // intentionally contained by the legacy visible row.
                assert!(matches!(
                    scene.primitives().first(),
                    Some(Primitive::PushClip(clip)) if *clip == frame
                ));
                assert_eq!(clips.first(), Some(&frame));
            } else {
                let expected_mark_clip = ancestor;
                if let Some(parent) = expected_mark_clip {
                    assert!(matches!(
                        scene.primitives().first(),
                        Some(Primitive::PushClip(clip)) if *clip == parent
                    ));
                    assert_eq!(clips.first(), Some(&parent));
                } else {
                    assert!(matches!(
                        scene.primitives().first(),
                        Some(Primitive::Quad(_))
                    ));
                    assert_eq!(clips.first(), (width > 16.0).then_some(&frame));
                }
                if width == 16.0 {
                    assert_eq!(text.drawn, None);
                } else {
                    assert!(text.drawn.is_some_and(|(rect, _)| rect == layout.label));
                    assert_eq!(clips.last(), Some(&frame));
                }
            }
        }
    }
}

#[test]
fn button_measure_keeps_legacy_height_at_default_text_and_expands_for_large_text() {
    let mut text = FixedText {
        measured: Size::new(80.0, proposal::TextRole::ButtonXs.style().line_height),
        drawn: None,
    };
    let default = ButtonModel::measure(
        &mut text,
        "Motion action",
        160.0,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
    );
    assert_eq!(
        default.height,
        ButtonSize::Xs.minimum_height().max(
            proposal::TextRole::ButtonXs.style().line_height
                + ButtonSize::Xs.vertical_inset(proposal::TextRole::ButtonXs) * 2.0
        )
    );
    text.measured.height = proposal::TextRole::ButtonXs.style().line_height * 1.5;
    let large = ButtonModel::measure(
        &mut text,
        "Motion action",
        160.0,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
    );
    assert!(large.height > default.height);
    assert!(large.width <= 160.0);
}

#[test]
fn invalid_geometry_and_empty_labels_never_emit_invalid_or_unclipped_text() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(f32::NAN, f32::INFINITY),
        drawn: None,
    };
    let mut scene = Scene::new();
    let invalid = button.render(
        &mut scene,
        &mut text,
        "Invalid",
        Rect::new(f32::NAN, 0.0, 0.0, 20.0),
        None,
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
        ButtonTextAlign::Center,
    );
    assert_eq!(invalid.visible, None);
    assert!(scene.is_empty());

    let empty = button.render(
        &mut scene,
        &mut text,
        "",
        Rect::new(10.0, 10.0, 60.0, 26.0),
        None,
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
        ButtonTextAlign::Center,
    );
    assert!(empty.frame.min_x().is_finite());
    assert!(empty.frame.size.width.is_finite());
    assert!(text.drawn.is_none());
}

#[test]
fn button_point_interaction_uses_clipped_placement_and_cancels_outside_release() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 16.0),
        drawn: None,
    };
    button.render(
        &mut Scene::new(),
        &mut text,
        "Action",
        Rect::new(10.0, 20.0, 100.0, 32.0),
        Some(Rect::new(10.0, 20.0, 50.0, 32.0)),
        Theme::LIGHT,
        crate::paint::ButtonVariant::Panel,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
        ButtonTextAlign::Leading,
    );
    assert!(!button.pointer_down_at(Point::new(70.0, 30.0)));
    assert!(button.pointer_down_at(Point::new(20.0, 30.0)));
    assert!(button.state.pressed);
    button.pointer_move_at(Point::new(70.0, 30.0));
    assert!(!button.state.pressed);
    assert!(!button.pointer_up_at(Point::new(70.0, 30.0)));
    assert!(!button.state.pressed);
    assert!(!button.pointer_up_at(Point::new(20.0, 30.0)));
    button.clear_layout();
    assert_eq!(button.layout(), None);
}
