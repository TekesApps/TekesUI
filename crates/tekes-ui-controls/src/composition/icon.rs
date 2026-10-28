//! Canonical passive artwork in the existing Content/rendering contracts.
use super::*;
use crate::extended::StatusBadge;
use crate::layout_view::{IconRenderer, IconView};
use std::rc::Rc;
use tekes_ui_scene::{ImageId, ImageQuad, TransformedImageQuad};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconRenderError {
    UnknownIcon,
    UnsupportedRenderer,
    InvalidSize,
    Raster,
    Upload,
    MissingMotionContext,
    InvalidMotionTime,
    UnsupportedTone,
}
/// A registered image and its original backend lifetime. Clones share that
/// registration; dropping the last lease retires it on its original renderer.
#[derive(Clone, Debug)]
pub struct IconImage {
    image: ImageId,
    _lease: Rc<dyn Debug>,
}
impl IconImage {
    pub fn new(image: ImageId, lease: Rc<dyn Debug>) -> Self {
        Self {
            image,
            _lease: lease,
        }
    }
    pub fn id(&self) -> ImageId {
        self.image
    }
}

#[derive(Debug)]
pub struct IconContent {
    glyph: tekes_ui_icons::Glyph,
    view: IconView,
    name: Option<String>,
    visible: bool,
    resource: Option<IconImage>,
    error: Option<IconRenderError>,
    indeterminate: bool,
    status_tone: Option<StatusBadge>,
    disabled: bool,
    rotation: Option<tekes_ui_motion::linear_rotation::LinearRotation>,
}
impl IconContent {
    pub fn new(canonical_name: &str) -> Result<Self, IconRenderError> {
        let glyph =
            tekes_ui_icons::Glyph::named(canonical_name).ok_or(IconRenderError::UnknownIcon)?;
        Ok(Self {
            glyph,
            view: IconView::default(),
            name: None,
            visible: true,
            resource: None,
            error: None,
            indeterminate: false,
            status_tone: None,
            disabled: false,
            rotation: None,
        })
    }
    pub fn glyph(&self) -> tekes_ui_icons::Glyph {
        self.glyph
    }
    pub fn set_glyph(&mut self, name: &str) -> Result<bool, IconRenderError> {
        let next = tekes_ui_icons::Glyph::named(name).ok_or(IconRenderError::UnknownIcon)?;
        if next == self.glyph {
            return Ok(false);
        }
        self.glyph = next;
        self.resource = None;
        self.error = None;
        Ok(true)
    }
    /// Defaults to decorative; a named passive icon has text semantics and no
    /// action/focus owner. An enclosing Button still owns the entire AX identity.
    pub fn with_accessible_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Passive semantic tone only; action-owned decorative ink remains authoritative.
    pub fn set_status_tone(&mut self, tone: Option<StatusBadge>) -> bool {
        let changed = self.status_tone != tone;
        self.status_tone = tone;
        changed
    }
    pub fn status_tone(&self) -> Option<StatusBadge> {
        self.status_tone
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.rotation = None;
        }
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    /// Update explicit passive semantics without replacing the original mount.
    /// None restores decorative content; invalid names never change the old node.
    pub fn set_accessible_name(&mut self, name: Option<String>) -> Result<bool, CompositionError> {
        if name.as_ref().is_some_and(|s| s.trim().is_empty()) {
            return Err(CompositionError::EmptyAccessibleName);
        }
        let changed = self.name != name;
        self.name = name;
        Ok(changed)
    }
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        if !visible {
            self.resource = None;
            self.error = None;
            self.rotation = None;
        }
    }
    /// The original canonical artwork rotates using the host clock and JSON
    /// period. Static is the default; this never creates an action or ticker.
    pub fn set_indeterminate(&mut self, active: bool) {
        if self.indeterminate != active {
            self.rotation = None;
        }
        self.indeterminate = active;
    }
    pub fn is_indeterminate(&self) -> bool {
        self.indeterminate
    }
    pub fn layout(&self) -> crate::layout_view::IconLayout {
        self.view.layout()
    }
    fn natural_side(text: &mut dyn ButtonTextRenderer) -> f32 {
        let role = proposal::TextRole::UiBody;
        let scale = text.resolved_font_size(role) / role.style().size;
        let scale = if scale.is_finite() && scale > 0. {
            scale
        } else {
            1.
        };
        proposal::metric("size", "size-icon-standard").expect("DS regular icon size") * scale
    }
}
impl Component for IconContent {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, calls: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(calls)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }

    fn active(&self) -> bool {
        self.visible
    }
    fn arrange(&mut self, _text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if frame.intersection(&clip).is_none() {
            self.resource = None;
            self.rotation = None;
        }
    }
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.error
    }
    fn semantic_hidden(&self) -> bool {
        self.name.is_none()
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.as_ref().is_some_and(|s| s.trim().is_empty()) {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.error = None;
        if !self.visible {
            return MeasuredChild::new(Size::default());
        }
        let side = Self::natural_side(text);
        let side = side.min(available.width).min(available.height).max(0.);
        MeasuredChild::new(Size::new(side, side))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.error = None;
        let mut angle = 0.;
        let mut animate = false;
        if self.indeterminate && !self.disabled && text.animation_frames_allowed() {
            let Some(motion) = text.motion_context() else {
                self.error = Some(IconRenderError::MissingMotionContext);
                self.resource = None;
                self.rotation = None;
                return;
            };
            if motion.reduced_motion {
                self.rotation = None;
            } else {
                if self.rotation.is_none() {
                    let speed = 1. / f64::from(proposal::INDETERMINATE_ROTATION_PERIOD_SECONDS);
                    let mut motor = tekes_ui_motion::linear_rotation::LinearRotation::new(speed)
                        .expect("validated DS period");
                    motor.resume(motion.now).expect("new monotonic motor");
                    self.rotation = Some(motor);
                }
                match self.rotation.as_mut().unwrap().advance(motion.now) {
                    Ok(sample) => {
                        angle = (sample.value.rem_euclid(1.) * std::f64::consts::TAU) as f32;
                        animate = true;
                    }
                    Err(_) => {
                        self.rotation = None;
                        self.resource = None;
                        self.error = Some(IconRenderError::InvalidMotionTime);
                        return;
                    }
                }
            }
        } else {
            self.rotation = None;
        }
        let requested = Self::natural_side(text);
        let backing = text.backing_scale();
        let tint = if let Some(ink) = theme.icon_foreground {
            ink
        } else {
            let usage = match self.status_tone {
                None => "content",
                Some(StatusBadge::Info) => "content-status-info",
                Some(StatusBadge::Running) => "content-status-running",
                Some(StatusBadge::Succeeded) => "content-status-succeeded",
                Some(StatusBadge::Failed) => "content-status-failed",
            };
            match tekes_ui_tokens::surface_palette::resolve_surface(
                theme.palette,
                theme.surface(),
                usage,
                if self.disabled { "disabled" } else { "rest" },
            ) {
                Ok(colors) => colors.icon,
                Err(_) => {
                    self.error = Some(IconRenderError::UnsupportedTone);
                    self.resource = None;
                    self.rotation = None;
                    return;
                }
            }
        };
        let Some(service) = text.icon_renderer() else {
            self.error = Some(IconRenderError::UnsupportedRenderer);
            self.resource = None;
            return;
        };
        struct Prepared<'a> {
            service: &'a mut dyn IconRenderer,
            resource: &'a mut Option<IconImage>,
            error: &'a mut Option<IconRenderError>,
            backing: f32,
            angle: f32,
        }
        impl IconRenderer for Prepared<'_> {
            fn draw_icon(&mut self, scene: &mut Scene, name: &str, frame: Rect, tint: Rgba) {
                match self
                    .service
                    .prepare_icon(name, frame.size.width, self.backing, tint)
                {
                    Ok(image) => {
                        let quad = ImageQuad {
                            rect: frame,
                            image: image.id(),
                            opacity: 1.,
                        };
                        if self.angle == 0. {
                            scene.push_image(quad);
                        } else {
                            scene.push_transformed_image(TransformedImageQuad::new(
                                quad, self.angle,
                            ));
                        }
                        *self.resource = Some(image);
                    }
                    Err(error) => {
                        *self.resource = None;
                        *self.error = Some(error);
                    }
                }
            }
        }
        let mut renderer = Prepared {
            service,
            resource: &mut self.resource,
            error: &mut self.error,
            backing,
            angle,
        };
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited Icon Surface");
        self.view.render_tinted(
            scene,
            &mut renderer,
            self.glyph.name,
            frame,
            requested,
            Some(clip),
            theme.palette,
            tint,
        );
        if self.error.is_none() && self.resource.is_some() && animate {
            text.request_animation_frame();
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.resource = None;
            self.error = None;
            self.rotation = None;
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Text, self.name.as_deref().unwrap_or(""));
        s.disabled = self.disabled;
        s
    }
}

/// Borrow the existing text and icon capabilities for one window frame. No
/// text/interaction manager or separate per-component event wiring is created.
pub struct WithIconRenderer<'a> {
    text: &'a mut dyn ButtonTextRenderer,
    icons: &'a mut dyn IconRenderer,
}
impl std::fmt::Debug for WithIconRenderer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithIconRenderer").finish_non_exhaustive()
    }
}
impl<'a> WithIconRenderer<'a> {
    pub fn new(text: &'a mut dyn ButtonTextRenderer, icons: &'a mut dyn IconRenderer) -> Self {
        Self { text, icons }
    }
}
impl ButtonTextRenderer for WithIconRenderer<'_> {
    fn icon_renderer(&mut self) -> Option<&mut dyn IconRenderer> {
        Some(self.icons)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        self.text.measure(s, r, w)
    }
    fn draw(
        &mut self,
        s: &mut Scene,
        t: &str,
        r: proposal::TextRole,
        f: Rect,
        a: ButtonTextAlign,
        i: Rgba,
    ) {
        self.text.draw(s, t, r, f, a, i)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.text.text_surface_host()
    }
    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        self.text.resolved_font_size(r)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.text.motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        self.text.animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        self.text.request_animation_frame()
    }
    fn backing_scale(&self) -> f32 {
        self.text.backing_scale()
    }
    fn first_line_height(&mut self, s: &str, r: proposal::TextRole, w: f32) -> f32 {
        self.text.first_line_height(s, r, w)
    }
}

#[cfg(feature = "svg-icons")]
mod svg;

#[cfg(feature = "svg-icons")]
pub use svg::{SvgIconCache, SvgIconRenderer};
