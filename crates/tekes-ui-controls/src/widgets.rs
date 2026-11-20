//! Platform-independent interaction contracts. Rendering and native event translation
//! are adapters; these models are shared by applications and Gallery stories.
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlMotionContext {
    pub now: Duration,
    pub reduced_motion: bool,
}

/// ButtonModel size roles resolve through the JSON Design System token table.
/// `Body` uses the Design System ordinary action-button metrics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
    Body,
}

impl ButtonSize {
    pub(crate) fn radius(self) -> f32 {
        let token = match self {
            Self::Xs => "c-button-xs-radius",
            Self::Sm => "c-button-sm-radius",
            Self::Md | Self::Body => "c-button-md-radius",
            Self::Lg => "c-button-lg-radius",
        };
        proposal::metric("radius", token).expect("ButtonModel component radius token")
    }
    fn minimum_height(self) -> f32 {
        match self {
            Self::Xs => proposal::metric("size", "size-button-xs"),
            Self::Sm => proposal::metric("size", "size-button-sm"),
            Self::Md => proposal::metric("size", "size-button-md"),
            Self::Lg => proposal::metric("size", "size-button-lg"),
            Self::Body => proposal::metric("size", "size-action-button"),
        }
        .expect("button minimum size token")
    }

    fn inset(self) -> f32 {
        let name = match self {
            Self::Xs => "space-button-xs-inset",
            Self::Sm => "space-button-sm-inset",
            Self::Md => "space-button-md-inset",
            Self::Lg => "space-button-lg-inset",
            Self::Body => "space-relaxed",
        };
        proposal::metric("spacing", name).expect("button inset token")
    }

    pub(crate) fn scaled_horizontal_inset(
        self,
        text: &mut (impl ButtonTextRenderer + ?Sized),
        role: proposal::TextRole,
    ) -> f32 {
        self.inset() * button_typography_scale(text, role)
    }

    fn vertical_inset(self, role: proposal::TextRole) -> f32 {
        ((self.minimum_height() - role.style().line_height) * 0.5).max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonTextAlign {
    Leading,
    Center,
}
impl Default for ButtonTextAlign {
    fn default() -> Self {
        match proposal::layout::BUTTON_CONTENT.text_alignment {
            proposal::layout::TextAlignment::Leading => Self::Leading,
            proposal::layout::TextAlignment::Center => Self::Center,
            proposal::layout::TextAlignment::Trailing => unreachable!(
                "Design compiler rejects unsupported trailing ButtonModel content alignment"
            ),
        }
    }
}

/// Text shaping and glyph output are supplied by the host; layout and paint
/// remain owned by the production ButtonModel component.
pub trait ButtonTextRenderer {
    /// Optional window resource service, shared by all original Content icons.
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        None
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size;
    /// Optional text capability. Existing label-only hosts remain compatible;
    /// composed text fields fail explicitly when the capability is absent.
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        None
    }
    /// Effective point size used by this text backend, including its shared
    /// typography/zoom context. Geometry consumers derive their ratios here;
    /// applications never need to set each control's dimensions separately.
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size
            * self
                .text_surface_host()
                .map_or(1., |host| host.type_scale())
    }
    /// Optional host clock and motion preference for animated production controls.
    fn motion_context(&self) -> Option<ControlMotionContext> {
        None
    }
    /// Window-level permission for a descendant to request another frame.
    fn animation_frames_allowed(&self) -> bool {
        true
    }
    /// Ask the host for another frame while a component-owned transition is active.
    fn request_animation_frame(&mut self) {}
    /// Backing scale used for scale-aware decorative geometry. Text adapters
    /// that do not know the renderer scale keep the compatible 1x default.
    fn backing_scale(&self) -> f32 {
        1.0
    }
    /// Height of the first shaped line. Existing adapters may rely on the
    /// fallback; text engines with shaping data should return the actual first
    /// line metric for the requested label and width.
    fn first_line_height(&mut self, _text: &str, role: proposal::TextRole, max_width: f32) -> f32 {
        self.measure("M", role, max_width).height
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    );
}

/// A borrowed renderer can be passed to the existing sized production View
/// APIs from a `Component` callback receiving `&mut dyn ButtonTextRenderer`.
/// Forward every host capability, including text services and motion preferences.
impl<T: ButtonTextRenderer + ?Sized> ButtonTextRenderer for &mut T {
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        (**self).icon_renderer()
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        (**self).measure(text, role, max_width)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        (**self).text_surface_host()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        (**self).resolved_font_size(role)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        (**self).motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        (**self).animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        (**self).request_animation_frame();
    }
    fn backing_scale(&self) -> f32 {
        (**self).backing_scale()
    }
    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> f32 {
        (**self).first_line_height(text, role, max_width)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        (**self).draw(scene, text, role, rect, align, ink);
    }
}

// Shared logical geometry follows the resolved text role. A renderer that does
// not opt into typography scaling retains the original DS metrics.
pub(super) fn button_typography_scale(
    text: &mut (impl ButtonTextRenderer + ?Sized),
    role: proposal::TextRole,
) -> f32 {
    let scale = text.resolved_font_size(role) / role.style().size;
    if scale.is_finite() && scale > 0. {
        scale
    } else {
        1.
    }
}

/// Resolved geometry shared by button paint, text placement and hit testing.
/// `content` is the constrained text slot; it is deliberately the same width
/// used for shaping so the host cannot accidentally wrap at a second width.
/// Exact is an explicit compact-action opt-in. Ordinary buttons retain their
/// token minimum and insets. Exact uses the caller-arranged frame for chrome,
/// decorative content and hit testing; focus still uses the shared ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonFramePolicy {
    #[default]
    Minimum,
    Exact,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonLayout {
    pub frame: Rect,
    pub content: Rect,
    /// Visible hit and text clip after intersecting the ancestor clip.
    pub visible: Option<Rect>,
    /// Shaped extent returned by the host text backend.
    pub label_size: Size,
}

impl ButtonLayout {
    /// Pure Design System baseline resolver for explicit geometry callers.
    /// ButtonModel measurement/painting and Content slots additionally use the
    /// renderer's resolved typography; Exact frames remain caller-owned.
    pub fn resolve(
        frame: Rect,
        clip: Option<Rect>,
        label_size: Size,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
    ) -> Self {
        Self::resolve_with_policy(
            frame,
            clip,
            label_size,
            size,
            role,
            align,
            ButtonFramePolicy::Minimum,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_with_policy(
        frame: Rect,
        clip: Option<Rect>,
        label_size: Size,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        policy: ButtonFramePolicy,
    ) -> Self {
        Self::resolve_scaled(frame, clip, label_size, size, role, align, policy, 1.)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_with_text(
        text: &mut (impl ButtonTextRenderer + ?Sized),
        frame: Rect,
        clip: Option<Rect>,
        label_size: Size,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        policy: ButtonFramePolicy,
    ) -> Self {
        let scale = button_typography_scale(text, role);
        Self::resolve_scaled(frame, clip, label_size, size, role, align, policy, scale)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_scaled(
        frame: Rect,
        clip: Option<Rect>,
        label_size: Size,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        policy: ButtonFramePolicy,
        scale: f32,
    ) -> Self {
        if policy == ButtonFramePolicy::Exact {
            return Self {
                frame,
                content: frame,
                visible: clip.map_or(Some(frame), |clip| frame.intersection(&clip)),
                label_size: Size::new(
                    finite_nonnegative(label_size.width),
                    finite_nonnegative(label_size.height),
                ),
            };
        }
        let horizontal_inset = size.inset() * scale;
        let vertical_inset = size.vertical_inset(role) * scale;
        let label_size = Size::new(
            finite_nonnegative(label_size.width),
            finite_nonnegative(label_size.height),
        );
        let label_height = finite_nonnegative(label_size.height);
        let min_height = size.minimum_height() * scale;
        let height = frame
            .size
            .height
            .max(min_height)
            .max(label_height + vertical_inset * 2.0);
        let frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, height);
        let content_width = (frame.size.width - horizontal_inset * 2.0).max(0.0);
        let content_height = label_height.min((frame.size.height - vertical_inset * 2.0).max(0.0));
        let content_x = match align {
            ButtonTextAlign::Leading | ButtonTextAlign::Center => frame.min_x() + horizontal_inset,
        };
        let content = Rect::new(
            content_x,
            frame.center().y - content_height * 0.5,
            content_width,
            content_height,
        );
        let visible = clip.map_or(Some(frame), |clip| frame.intersection(&clip));
        Self {
            frame,
            content,
            visible,
            label_size,
        }
    }

    fn empty() -> Self {
        Self {
            frame: Rect::default(),
            content: Rect::default(),
            visible: None,
            label_size: Size::default(),
        }
    }

    pub fn hit_test(&self, point: Point) -> bool {
        self.visible.is_some_and(|rect| rect.contains(point))
    }

    /// Bounds including the Design System focus ring, for host focus reveal.
    pub fn focus_bounds(&self) -> Rect {
        expand_for_focus(self.frame)
    }
}

pub(super) fn expand_for_focus(rect: Rect) -> Rect {
    let bleed = proposal::metric("stroke", "c-focus-width")
        .unwrap_or(0.0)
        .max(0.0)
        + proposal::metric("spacing", "c-focus-offset")
            .unwrap_or(0.0)
            .max(0.0);
    Rect::new(
        rect.min_x() - bleed,
        rect.min_y() - bleed,
        rect.size.width + bleed * 2.0,
        rect.size.height + bleed * 2.0,
    )
}

pub(super) fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn valid_frame(frame: Rect) -> bool {
    frame.min_x().is_finite()
        && frame.min_y().is_finite()
        && frame.size.width.is_finite()
        && frame.size.height.is_finite()
        && frame.max_x().is_finite()
        && frame.max_y().is_finite()
        && frame.size.width > 0.0
        && frame.size.height >= 0.0
}

pub(crate) fn valid_clip(clip: Rect) -> bool {
    clip.min_x().is_finite()
        && clip.min_y().is_finite()
        && clip.size.width.is_finite()
        && clip.size.height.is_finite()
        && clip.max_x().is_finite()
        && clip.max_y().is_finite()
        && clip.size.width > 0.0
        && clip.size.height > 0.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    Enter,
    Space,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
    Tab { reverse: bool },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub focus_visible: bool,
    pub disabled: bool,
    pub loading: bool,
    pub selected: bool,
    pub error: bool,
}

mod button_model;
mod models;
pub use button_model::ButtonModel;
pub(crate) use button_model::{ButtonActionDecision, ButtonActionSnapshot};
pub use models::{
    CheckState, Checkbox, CheckboxLayout, Dialog, Field, Menu, NumberField, Select, Tabs, Toolbar,
};
pub(crate) use models::{CheckboxActionDecision, CheckboxPrepared, SelectChoice};

#[cfg(test)]
mod tests_1;
#[cfg(test)]
mod tests_2;
