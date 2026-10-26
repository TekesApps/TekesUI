use std::time::Duration;
use tekes_ui_controls::{
    Button, ButtonFramePolicy, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    ControlMotionContext, composition::*,
};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text {
    scale: f32,
    now: u64,
    drawn: Vec<(String, Rect)>,
}
impl Text {
    fn new(scale: f32) -> Self {
        Self {
            scale,
            ..Default::default()
        }
    }
}
impl ButtonTextRenderer for Text {
    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        r.style().size * self.scale
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * r.style().size * self.scale * 0.5).min(w),
            r.style().line_height * self.scale,
        )
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: Duration::from_millis(self.now),
            reduced_motion: true,
        })
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        r: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        self.drawn.push((s.into(), r));
    }
}
fn button(kind: usize) -> Button {
    match kind {
        0 => Button::new("Save"),
        1 => Button::new(ImageId(1)).with_accessible_name("Save"),
        2 => Button::new((ImageId(1), "Save")),
        _ => Button::new(Content::new(ComponentId(2), TextContent::new("Save")))
            .with_accessible_name("Save"),
    }
    .with_help("Save the current document")
}
fn host(kind: usize) -> ContentHost {
    let mut h = ContentHost::new(Content::new(ComponentId(1), button(kind))).unwrap();
    h.set_action_events(true);
    h
}
fn render(h: &mut ContentHost, t: &mut Text, theme: Theme) -> Scene {
    let frame = Rect::new(20., 30., 320. * t.scale, 100. * t.scale);
    let mut s = Scene::new();
    t.drawn.clear();
    h.render(
        &mut s,
        t,
        frame,
        Rect::new(0., 0., 1000., 1000.),
        theme.into(),
    )
    .unwrap();
    s
}
fn count(h: &ContentHost) -> usize {
    h.get(ComponentId(1))
        .unwrap()
        .component::<Button>()
        .unwrap()
        .activations
}
fn key(h: &mut ContentHost, key: ControlKey, down: bool) {
    h.event(&ContentEvent::Key { key, down });
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.6, "{a} != {b}");
}
#[test]
fn every_content_shape_has_one_original_action_and_one_accessibility_identity() {
    for kind in 0..4 {
        let mut h = host(kind);
        render(&mut h, &mut Text::new(1.), Theme::LIGHT);
        let nodes = h.semantics();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].semantics.name, "Save");
        assert_eq!(
            nodes[0].semantics.description.as_deref(),
            Some("Save the current document")
        );
        assert_eq!(h.focus_targets().len(), 1);
        assert!(
            !h.accessibility_action(ComponentId(2), None, AccessibilityAction::Press)
                .activated
        );
        let p = nodes[0].frame.center();
        h.event(&ContentEvent::PointerDown(p));
        h.event(&ContentEvent::PointerUp(p));
        assert_eq!(count(&h), 1);
        assert_eq!(h.drain_actions().len(), 1);
        h.focus(Some(ComponentId(1)), true);
        assert_eq!(h.focused(), Some(ComponentId(1)));
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(count(&h), 2);
        assert_eq!(h.drain_actions().len(), 1);
        assert!(
            h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press)
                .activated
        );
        assert_eq!(count(&h), 3);
        assert_eq!(h.drain_actions().len(), 1);
    }
}
#[test]
fn cancellation_capture_and_disabled_loading_are_identical_for_all_contents() {
    for kind in 0..4 {
        for loading in [false, true] {
            let mut h = host(kind);
            let mut t = Text::new(1.);
            render(&mut h, &mut t, Theme::LIGHT);
            let p = h.semantics()[0].frame.center();
            h.event(&ContentEvent::PointerMove(p));
            assert!(
                h.get(ComponentId(1))
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .button
                    .state
                    .hovered
            );
            assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
            key(&mut h, ControlKey::Escape, true);
            h.event(&ContentEvent::PointerUp(p));
            assert_eq!(count(&h), 0);
            h.focus(Some(ComponentId(1)), true);
            key(&mut h, ControlKey::Space, true);
            key(&mut h, ControlKey::Escape, true);
            key(&mut h, ControlKey::Space, false);
            assert_eq!(count(&h), 0);
            h.event(&ContentEvent::PointerDown(p));
            let b = h
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<Button>()
                .unwrap();
            b.button.state.disabled = !loading;
            b.button.state.loading = loading;
            render(&mut h, &mut t, Theme::LIGHT);
            h.event(&ContentEvent::PointerUp(p));
            assert!(
                !h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press)
                    .activated
            );
            key(&mut h, ControlKey::Enter, true);
            assert_eq!(count(&h), 0);
            assert!(h.drain_actions().is_empty());
        }
    }
}
#[test]
fn entire_content_group_is_centered_at_all_ds_scales_and_themes() {
    for kind in 0..4 {
        for scale in [0.75, 1., 1.25, 2.] {
            for theme in Theme::ALL {
                let mut h = host(kind);
                let mut t = Text::new(scale);
                let s = render(&mut h, &mut t, theme);
                let f = h.semantics()[0].frame;
                let mut bounds: Option<Rect> = None;
                for p in s.primitives() {
                    if let Primitive::Image(q) = p {
                        bounds = Some(q.rect);
                    }
                }
                for (_, r) in &t.drawn {
                    bounds = Some(match bounds {
                        None => *r,
                        Some(b) => Rect::new(
                            b.min_x().min(r.min_x()),
                            b.min_y().min(r.min_y()),
                            b.max_x().max(r.max_x()) - b.min_x().min(r.min_x()),
                            b.max_y().max(r.max_y()) - b.min_y().min(r.min_y()),
                        ),
                    });
                }
                let b = bounds.unwrap();
                near(b.center().x, f.center().x);
                near(b.center().y, f.center().y);
                if kind == 1 || kind == 2 {
                    let image = s
                        .primitives()
                        .iter()
                        .find_map(|p| {
                            if let Primitive::Image(q) = p {
                                Some(q)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    near(
                        image.rect.size.height,
                        proposal::metric("size", "size-icon-standard").unwrap() * scale,
                    );
                }
            }
        }
    }
}
#[test]
fn image_position_and_alignment_use_the_same_button_without_rebuilding_action_owner() {
    use proposal::layout::MainAxisDistribution;
    let mut h = host(2);
    let mut t = Text::new(1.);
    for position in [
        ButtonImagePosition::Leading,
        ButtonImagePosition::Trailing,
        ButtonImagePosition::Above,
        ButtonImagePosition::Below,
    ] {
        h.get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .image_position = position;
        let s = render(&mut h, &mut t, Theme::LIGHT);
        let image = s
            .primitives()
            .iter()
            .find_map(|p| {
                if let Primitive::Image(q) = p {
                    Some(q.rect)
                } else {
                    None
                }
            })
            .unwrap();
        let label = t.drawn[0].1;
        match position {
            ButtonImagePosition::Leading => assert!(image.max_x() <= label.min_x()),
            ButtonImagePosition::Trailing => assert!(image.min_x() >= label.max_x()),
            ButtonImagePosition::Above => assert!(image.max_y() <= label.min_y()),
            ButtonImagePosition::Below => assert!(image.min_y() >= label.max_y()),
        }
    }
    let mut centers = Vec::new();
    for alignment in [
        MainAxisDistribution::Start,
        MainAxisDistribution::Center,
        MainAxisDistribution::End,
    ] {
        let b = h
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<Button>()
            .unwrap();
        b.image_position = ButtonImagePosition::Leading;
        b.content_alignment = alignment;
        render(&mut h, &mut t, Theme::LIGHT);
        centers.push(t.drawn[0].1.center().x);
    }
    assert!(centers[0] < centers[1] && centers[1] < centers[2]);
}
#[test]
fn icon_only_and_custom_content_require_a_name_and_interactive_children_are_rejected() {
    for b in [
        Button::new(ImageId(1)),
        Button::new(Content::new(
            ComponentId(2),
            TextContent::new("Visual text"),
        )),
    ] {
        assert!(matches!(
            ContentHost::new(Content::new(ComponentId(1), b)),
            Err(CompositionError::EmptyAccessibleName)
        ));
    }
    let b = Button::new(Content::new(ComponentId(2), Button::new("Nested action")))
        .with_accessible_name("Invalid");
    assert!(matches!(
        ContentHost::new(Content::new(ComponentId(1), b)),
        Err(CompositionError::NestedInteractive { .. })
    ));
}
#[test]
fn button_help_reuses_original_dwell_revision_and_dismissal_pipeline() {
    for kind in 0..4 {
        let mut h = host(kind);
        let mut t = Text::new(1.);
        render(&mut h, &mut t, Theme::LIGHT);
        let p = h.semantics()[0].frame.center();
        h.event(&ContentEvent::PointerMove(p));
        render(&mut h, &mut t, Theme::LIGHT);
        assert!(h.help_tooltip().is_none());
        t.now = 500;
        render(&mut h, &mut t, Theme::LIGHT);
        assert_eq!(h.help_tooltip().unwrap().1, "Save the current document");
        assert_eq!(h.semantics().len(), 1);
        h.get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .set_help(Some("Updated explanation".into()));
        render(&mut h, &mut t, Theme::LIGHT);
        assert!(h.help_tooltip().is_none());
        t.now = 1000;
        render(&mut h, &mut t, Theme::LIGHT);
        assert_eq!(h.help_tooltip().unwrap().1, "Updated explanation");
        h.event(&ContentEvent::PointerDown(p));
        h.event(&ContentEvent::PointerUp(p));
        render(&mut h, &mut t, Theme::LIGHT);
        assert!(h.help_tooltip().is_none());
        assert_eq!(count(&h), 1);
    }
}
#[test]
fn exact_toolbar_buttons_and_content_replacement_preserve_the_original_action() {
    let mut h = host(0);
    let mut t = Text::new(1.);
    render(&mut h, &mut t, Theme::LIGHT);
    h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press);
    h.drain_actions();
    let b = h
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Button>()
        .unwrap();
    b.button.frame_policy = ButtonFramePolicy::Exact;
    b.set_contents(ImageId(1));
    b.set_accessible_name(Some("Save".into()));
    render(&mut h, &mut t, Theme::LIGHT);
    assert_eq!(count(&h), 1);
    assert_eq!(h.semantics().len(), 1);
    let b = h
        .get(ComponentId(1))
        .unwrap()
        .component::<Button>()
        .unwrap();
    assert_eq!(
        b.button.layout().unwrap().frame,
        h.get(ComponentId(1)).unwrap().frame()
    );
    h.set_window_active(false);
    assert!(
        !h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press)
            .activated
    );
    h.set_window_active(true);
    assert!(
        h.accessibility_action(ComponentId(1), None, AccessibilityAction::Press)
            .activated
    );
    assert_eq!(count(&h), 2);
}
