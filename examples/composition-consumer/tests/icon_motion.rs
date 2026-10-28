//! Original clocked motor, checked resource and one ContentHost scheduling gate.
use std::time::Duration;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, composition::*,
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    now: Duration,
    reduced: bool,
    clock: bool,
    allowed: bool,
    requested: bool,
}
impl Default for Text {
    fn default() -> Self {
        Self {
            now: Duration::ZERO,
            reduced: false,
            clock: true,
            allowed: true,
            requested: false,
        }
    }
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::new(20., 20.)
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
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.clock.then_some(ControlMotionContext {
            now: self.now,
            reduced_motion: self.reduced,
        })
    }
    fn animation_frames_allowed(&self) -> bool {
        self.allowed
    }
    fn request_animation_frame(&mut self) {
        self.requested = true;
    }
}
struct Icons {
    fail: bool,
}
impl IconRenderer for Icons {
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("checked resources only")
    }
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        if self.fail {
            return Err(IconRenderError::Upload);
        }
        Ok(IconImage::new(ImageId(1), std::rc::Rc::new(())))
    }
}
fn host() -> ContentHost {
    let mut icon = IconContent::new("tekes-back").unwrap();
    icon.set_indeterminate(true);
    ContentHost::new(Content::new(ComponentId(1), icon)).unwrap()
}
fn draw(h: &mut ContentHost, t: &mut Text, icons: &mut Icons) -> Result<Scene, CompositionError> {
    t.requested = false;
    let mut s = Scene::new();
    let f = Rect::new(0., 0., 32., 32.);
    h.render(
        &mut s,
        &mut WithIconRenderer::new(t, icons),
        f,
        f,
        Theme::LIGHT.into(),
    )?;
    Ok(s)
}
fn angle(s: &Scene) -> f32 {
    s.primitives()
        .iter()
        .find_map(|p| match p {
            Primitive::TransformedImage(i) => Some(i.rotation),
            Primitive::Image(_) => Some(0.),
            _ => None,
        })
        .unwrap()
}
#[test]
fn original_motor_uses_json_period_and_reduced_stop_hide_unmount_window_gates() {
    let mut h = host();
    let mut t = Text::default();
    let mut icons = Icons { fail: false };
    assert_eq!(angle(&draw(&mut h, &mut t, &mut icons).unwrap()), 0.);
    assert!(t.requested && h.animation_frame_requested());
    let mount = h.semantic_mount(ComponentId(1));
    t.now =
        Duration::from_secs_f64(f64::from(proposal::INDETERMINATE_ROTATION_PERIOD_SECONDS) / 4.);
    assert!(
        (angle(&draw(&mut h, &mut t, &mut icons).unwrap()) - std::f32::consts::FRAC_PI_2).abs()
            < 1e-5
    );
    assert!(t.requested);
    t.reduced = true;
    assert_eq!(angle(&draw(&mut h, &mut t, &mut icons).unwrap()), 0.);
    assert!(!t.requested && !h.animation_frame_requested());
    t.reduced = false;
    t.allowed = false;
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(!t.requested);
    t.allowed = true;
    h.set_window_active(false);
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(!t.requested && !h.animation_frame_requested());
    h.set_window_active(true);
    t.now += Duration::from_secs(1000);
    assert_eq!(angle(&draw(&mut h, &mut t, &mut icons).unwrap()), 0.);
    assert!(t.requested);
    assert_eq!(mount, h.semantic_mount(ComponentId(1)));
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_indeterminate(false);
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(!t.requested);
    let icon = h
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap();
    icon.set_indeterminate(true);
    icon.set_visible(false);
    let s = draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(s.is_empty());
    assert!(!t.requested);
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_visible(true);
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(t.requested);
    // Removing the actual animated descendant retires its work, not a business ticker.
    let root = Content::new(
        ComponentId(2),
        tekes_ui_controls::composition::StackContent {
            model: tekes_ui_controls::extended::Stack {
                axis: tekes_ui_controls::extended::Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![Content::new(ComponentId(3), {
                let mut i = IconContent::new("tekes-back").unwrap();
                i.set_indeterminate(true);
                i
            })],
        },
    );
    let mut h = ContentHost::new(root).unwrap();
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(t.requested);
    h.remove(ComponentId(3)).unwrap();
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(!t.requested);
}
#[test]
fn missing_clock_backward_time_and_upload_failure_never_schedule_failure_loops() {
    let mut h = host();
    let mut t = Text {
        clock: false,
        ..Default::default()
    };
    let mut icons = Icons { fail: false };
    assert!(matches!(
        draw(&mut h, &mut t, &mut icons),
        Err(CompositionError::IconResource {
            reason: IconRenderError::MissingMotionContext,
            ..
        })
    ));
    assert!(!t.requested && !h.animation_frame_requested());
    t.clock = true;
    t.now = Duration::from_secs(2);
    draw(&mut h, &mut t, &mut icons).unwrap();
    t.now = Duration::from_secs(1);
    assert!(matches!(
        draw(&mut h, &mut t, &mut icons),
        Err(CompositionError::IconResource {
            reason: IconRenderError::InvalidMotionTime,
            ..
        })
    ));
    assert!(!t.requested && !h.animation_frame_requested());
    icons.fail = true;
    assert!(draw(&mut h, &mut t, &mut icons).is_err());
    assert!(!t.requested && !h.animation_frame_requested());
    icons.fail = false;
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(t.requested);
}

#[test]
fn explicit_passive_disabled_stops_original_motor_and_reenable_starts_without_background_catchup() {
    let mut h = host();
    let mut t = Text::default();
    let mut icons = Icons { fail: false };
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_status_tone(Some(tekes_ui_controls::extended::StatusBadge::Running));
    draw(&mut h, &mut t, &mut icons).unwrap();
    assert!(t.requested);
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_disabled(true);
    t.now = Duration::from_secs(1000);
    assert_eq!(angle(&draw(&mut h, &mut t, &mut icons).unwrap()), 0.);
    assert!(!t.requested && !h.animation_frame_requested());
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<IconContent>()
        .unwrap()
        .set_disabled(false);
    t.now += Duration::from_secs(1000);
    assert_eq!(angle(&draw(&mut h, &mut t, &mut icons).unwrap()), 0.);
    assert!(t.requested);
}
