//! One content-driven Button over the original action state machine.
//!
//! Removed component entry points deliberately have no compatibility aliases:
//! ```compile_fail
//! use tekes_ui_controls::composition::CompoundButton;
//! ```
//! ```compile_fail
//! use tekes_ui_controls::composition::ButtonContent;
//! ```
//! ```compile_fail
//! use tekes_ui_controls::extended::IconButton;
//! ```
//! ```compile_fail
//! use tekes_ui_controls::layout_view::IconButtonView;
//! ```
use super::*;
use crate::ButtonModel;
use tekes_ui_scene::{CornerRadii, ImageId, ImageQuad};
use tekes_ui_tokens::proposal::layout::MainAxisDistribution;

/// Presentation data only. Every shape retains the same original action owner.
#[derive(Debug)]
pub enum ButtonContents {
    Text(String),
    Image(ImageId),
    ImageText(ImageId, String),
    Custom(Box<Content>),
}
impl From<String> for ButtonContents {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<&str> for ButtonContents {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<&String> for ButtonContents {
    fn from(v: &String) -> Self {
        Self::Text(v.clone())
    }
}
impl From<ImageId> for ButtonContents {
    fn from(v: ImageId) -> Self {
        Self::Image(v)
    }
}
impl<T: Into<String>> From<(ImageId, T)> for ButtonContents {
    fn from((image, text): (ImageId, T)) -> Self {
        Self::ImageText(image, text.into())
    }
}
impl From<Content> for ButtonContents {
    fn from(v: Content) -> Self {
        Self::Custom(Box::new(v))
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonImagePosition {
    #[default]
    Leading,
    Trailing,
    Above,
    Below,
}
/// Pointer timing only. Keyboard and AX retain the original once-only actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonActivationPhase {
    #[default]
    Release,
    Press,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CapsuleSegmentSpec {
    pub radii: CornerRadii,
    pub leading_inset: f32,
    pub trailing_inset: f32,
}

/// Text, image, image/text or original decorative Content; one action and AX identity.
/// Pure images/custom content require `with_accessible_name` before mounting.
/// Registered image resources remain host-owned; regular geometry comes from DS.
#[derive(Debug)]
pub struct Button {
    pub(crate) button: ButtonModel,
    /// Visible text; a separate accessible name never replaces this source.
    pub(crate) label: String,
    pub(crate) variant: ButtonVariant,
    pub(crate) activations: usize,
    pub(crate) pointer_activation_phase: ButtonActivationPhase,
    pub(crate) image_position: ButtonImagePosition,
    pub(crate) content_alignment: MainAxisDistribution,
    /// Optional leading content inset; the original action frame and hit size
    /// remain unchanged. None uses the standard Button inset.
    pub(crate) leading_content_inset: Option<f32>,
    accessible_name: Option<String>,
    image: Option<ImageId>,
    icon: Option<IconContent>,
    content: Vec<Content>,
    custom: bool,
    measured: Size,
    help: Option<String>,
    help_revision: u64,
    toolbar_presentation: bool,
    transparent_chrome: bool,
    toolbar_icon_slot: bool,
    capsule_segment: Option<CapsuleSegmentSpec>,
}
impl Button {
    pub fn new(contents: impl Into<ButtonContents>) -> Self {
        let mut button = Self {
            button: ButtonModel::default(),
            label: String::new(),
            variant: ButtonVariant::Quiet,
            activations: 0,
            pointer_activation_phase: ButtonActivationPhase::Release,
            image_position: ButtonImagePosition::default(),
            content_alignment: MainAxisDistribution::Center,
            leading_content_inset: None,
            accessible_name: None,
            image: None,
            icon: None,
            content: Vec::new(),
            custom: false,
            measured: Size::default(),
            help: None,
            help_revision: 0,
            toolbar_presentation: false,
            transparent_chrome: false,
            toolbar_icon_slot: false,
            capsule_segment: None,
        };
        button.set_contents(contents);
        button
    }
    pub(crate) fn set_toolbar_presentation(&mut self, toolbar: bool) {
        self.toolbar_presentation = toolbar;
    }
    pub(super) fn set_capsule_segment(&mut self, spec: Option<CapsuleSegmentSpec>) {
        if self.capsule_segment == spec {
            return;
        }
        let insets_before = self.button.horizontal_inset_override;
        let insets_after = spec
            .filter(|_| !self.toolbar_icon_slot)
            .map(|s| (s.leading_inset, s.trailing_inset));
        self.capsule_segment = spec;
        self.button.set_transparent_rest(spec.is_some());
        self.button.set_corner_radii(spec.map(|s| s.radii));
        self.button.set_horizontal_inset_override(insets_after);
        if insets_before != insets_after {
            self.measured = Size::default();
        }
    }
    #[cfg(test)]
    pub(super) fn uses_capsule_segment(&self) -> bool {
        self.capsule_segment.is_some()
    }
    pub(super) fn is_visually_active_in_capsule(&self) -> bool {
        !self.button.state.disabled
            && !self.button.state.loading
            && (self.button.state.hovered
                || self.button.state.pressed
                || self.button.state.selected
                || (self.button.state.focused && self.button.state.focus_visible))
    }
    pub(crate) fn set_toolbar_template_icon(&mut self, name: &str) {
        if self.content.first().is_some_and(|content| {
            content
                .component::<IconContent>()
                .is_some_and(|icon| icon.glyph().name == name)
        }) {
            return;
        }
        self.set_contents(Content::new(
            ComponentId(0),
            IconContent::new(name).expect("Toolbar validates canonical template artwork"),
        ));
    }
    // ToolbarControl retains its original Button owners rather than mounting
    // another action tree. Paint only this passive child after its owner chrome
    // has advanced the original hover/press channels.
    pub(crate) fn paint_toolbar_template_icon(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) -> Option<IconRenderError> {
        let child_theme = action_child_theme(self, theme);
        self.content.first_mut().and_then(|content| {
            content.paint(scene, text, child_theme);
            content
                .component::<IconContent>()
                .and_then(Component::icon_error)
        })
    }
    fn presentation_variant(&self) -> ButtonVariant {
        if self.toolbar_presentation && self.variant != ButtonVariant::Primary {
            ButtonVariant::Toolbar
        } else {
            self.variant
        }
    }
    /// Construct a button displaying a canonical vector glyph alongside a text label,
    /// without mounting a separate child `ComponentId`.
    pub fn icon_label(icon_name: &str, label: impl Into<String>) -> Result<Self, IconRenderError> {
        let mut button = Self::new("");
        button.set_icon_label(icon_name, label)?;
        Ok(button)
    }

    /// Replace this button's presentation with a canonical icon glyph and label.
    pub fn set_icon_label(
        &mut self,
        icon_name: &str,
        label: impl Into<String>,
    ) -> Result<(), IconRenderError> {
        self.image = None;
        self.content.clear();
        self.custom = false;
        if let Some(icon) = &mut self.icon {
            icon.set_glyph(icon_name)?;
        } else {
            self.icon = Some(IconContent::new(icon_name)?);
        }
        self.label = label.into();
        self.measured = Size::default();
        Ok(())
    }

    /// Align the painted icon ink to its leading slot edge when paired with a label.
    pub fn with_icon_ink_leading(mut self, enabled: bool) -> Self {
        self.set_icon_ink_leading(enabled);
        self
    }

    /// Update leading ink alignment on an icon-label button.
    pub fn set_icon_ink_leading(&mut self, enabled: bool) {
        if let Some(icon) = &mut self.icon {
            icon.set_ink_leading(enabled);
        }
    }

    /// Borrow the embedded icon content on an icon-label button, if present.
    pub fn icon(&self) -> Option<&IconContent> {
        self.icon.as_ref()
    }

    /// Replacing presentation never creates another action or resets its counter.
    pub fn set_contents(&mut self, contents: impl Into<ButtonContents>) {
        self.label.clear();
        self.image = None;
        self.icon = None;
        self.content.clear();
        self.custom = false;
        match contents.into() {
            ButtonContents::Text(text) => self.label = text,
            ButtonContents::Image(image) => self.image = Some(image),
            ButtonContents::ImageText(image, text) => {
                self.image = Some(image);
                self.label = text;
            }
            ButtonContents::Custom(content) => {
                self.custom = true;
                self.content.push(*content);
            }
        }
        self.measured = Size::default();
    }
    pub fn with_accessible_name(mut self, name: impl Into<String>) -> Self {
        self.set_accessible_name(Some(name.into()));
        self
    }
    pub fn set_accessible_name(&mut self, name: Option<String>) {
        self.accessible_name = name;
    }
    /// Use fill-only chrome for selected states while retaining the original
    /// ButtonModel colors, motion, geometry and focus treatment.
    pub fn set_selection_fill_only(&mut self, fill_only: bool) {
        self.button.set_selection_fill_only(fill_only);
    }
    /// Omit the Button background while preserving its interaction and focus
    /// chrome. Decorative content remains painted by its original child.
    pub fn set_transparent_chrome(&mut self, transparent: bool) {
        self.transparent_chrome = transparent;
    }
    /// Use the original ToolbarControl icon-button chrome metrics. Decorative
    /// content can add its measured width (for example a menu chevron).
    pub fn set_toolbar_icon_slot(&mut self, icon_slot: bool) {
        self.toolbar_icon_slot = icon_slot;
        self.button.frame_policy = if icon_slot {
            crate::ButtonFramePolicy::Exact
        } else {
            crate::ButtonFramePolicy::Minimum
        };
        let insets = self
            .capsule_segment
            .filter(|_| !icon_slot)
            .map(|s| (s.leading_inset, s.trailing_inset));
        self.button.set_horizontal_inset_override(insets);
        self.measured = Size::default();
    }
    pub fn with_tone(mut self, tone: crate::paint::ButtonTone) -> Self {
        self.set_tone(tone);
        self
    }
    pub fn set_tone(&mut self, tone: crate::paint::ButtonTone) {
        self.button.set_tone(tone);
    }
    pub fn tone(&self) -> crate::paint::ButtonTone {
        self.button.tone()
    }
    pub(super) fn uses_toolbar_icon_slot(&self) -> bool {
        self.toolbar_icon_slot
    }
    // MenuContent supplies the exact child size published during arrange;
    // unlike Button::paint, that measurement belongs to MenuContent's snapshot.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_toolbar_icon_frame(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        content_size: Size,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: ButtonVariant,
    ) -> crate::ButtonLayout {
        let focus_visible = self.button.state.focus_visible;
        let suppress_focus_ink =
            (self.toolbar_presentation || self.toolbar_icon_slot) && self.capsule_segment.is_none();
        if suppress_focus_ink {
            self.button.state.focus_visible = false;
        }
        let layout = if self.transparent_chrome {
            self.button.render_transparent_content_frame(
                scene,
                &mut TextAdapter(text),
                content_size,
                frame,
                clip,
                palette,
                variant,
            )
        } else {
            self.button.render_content_frame(
                scene,
                &mut TextAdapter(text),
                content_size,
                frame,
                clip,
                palette,
                variant,
            )
        };
        if suppress_focus_ink && self.button.state.focused {
            self.button.state.focus_visible = focus_visible;
        }
        layout
    }
    pub(super) fn toolbar_icon_size(
        &self,
        text: &mut dyn ButtonTextRenderer,
        measured: Size,
        available: Size,
    ) -> Size {
        let box_size = crate::toolbar::metrics(text, proposal::TextRole::UiBody).box_size;
        Size::new(
            (box_size + (measured.width - Self::icon_side(text)).max(0.)).min(available.width),
            box_size.max(measured.height),
        )
    }
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.set_help(Some(help.into()));
        self
    }
    pub fn set_help(&mut self, help: Option<String>) {
        let help = help.filter(|text| !text.trim().is_empty());
        if self.help != help {
            self.help = help;
            self.help_revision = self
                .help_revision
                .checked_add(1)
                .expect("button help revision exhausted");
        }
    }
    pub(super) fn supports_label_layout(&self) -> bool {
        !self.custom
    }
    pub fn help_text(&self) -> Option<&str> {
        self.help.as_deref()
    }
    pub fn accessible_name(&self) -> &str {
        self.name()
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn set_label(&mut self, label: impl Into<String>) {
        if self.custom {
            self.set_contents(label.into());
        } else {
            self.label = label.into();
            self.measured = Size::default();
        }
    }
    pub fn variant(&self) -> ButtonVariant {
        self.variant
    }
    pub fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn set_variant(&mut self, variant: ButtonVariant) {
        self.variant = variant;
    }
    pub fn activations(&self) -> usize {
        self.activations
    }
    pub fn model(&self) -> &ButtonModel {
        &self.button
    }
    pub fn state(&self) -> crate::ControlState {
        self.button.state
    }
    pub fn is_disabled(&self) -> bool {
        self.button.state.disabled
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.set_disabled(disabled);
        self
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.button.state.disabled = disabled;
        if disabled {
            self.button.blur();
        }
    }
    pub fn is_loading(&self) -> bool {
        self.button.state.loading
    }
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.set_loading(loading);
        self
    }
    pub fn set_loading(&mut self, loading: bool) {
        self.button.state.loading = loading;
        if loading {
            self.button.blur();
        }
    }
    pub fn is_selected(&self) -> bool {
        self.button.state.selected
    }
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.button.state.selected = selected;
        self
    }
    pub fn set_selected(&mut self, selected: bool) {
        self.button.state.selected = selected;
    }
    pub fn is_hovered(&self) -> bool {
        self.button.state.hovered
    }
    pub fn is_pressed(&self) -> bool {
        self.button.state.pressed
    }
    pub fn is_focused(&self) -> bool {
        self.button.state.focused
    }
    pub fn focus_visible(&self) -> bool {
        self.button.state.focus_visible
    }
    pub fn cancel(&mut self) {
        self.button.cancel();
    }
    pub fn pointer_activation_phase(&self) -> ButtonActivationPhase {
        self.pointer_activation_phase
    }
    pub fn with_pointer_activation_phase(mut self, phase: ButtonActivationPhase) -> Self {
        self.pointer_activation_phase = phase;
        self
    }
    pub fn set_pointer_activation_phase(&mut self, phase: ButtonActivationPhase) {
        self.pointer_activation_phase = phase;
    }
    pub fn image_position(&self) -> ButtonImagePosition {
        self.image_position
    }
    pub fn with_image_position(mut self, position: ButtonImagePosition) -> Self {
        self.set_image_position(position);
        self
    }
    pub fn set_image_position(&mut self, position: ButtonImagePosition) {
        self.image_position = position;
        self.measured = Size::default();
    }
    pub fn content_alignment(&self) -> MainAxisDistribution {
        self.content_alignment
    }
    pub fn with_content_alignment(mut self, alignment: MainAxisDistribution) -> Self {
        self.content_alignment = alignment;
        self
    }
    pub fn set_content_alignment(&mut self, alignment: MainAxisDistribution) {
        self.content_alignment = alignment;
    }
    pub fn leading_content_inset(&self) -> Option<f32> {
        self.leading_content_inset
    }
    pub fn with_leading_content_inset(mut self, inset: Option<f32>) -> Self {
        self.set_leading_content_inset(inset);
        self
    }
    pub fn set_leading_content_inset(&mut self, inset: Option<f32>) {
        self.leading_content_inset = inset;
        self.measured = Size::default();
    }
    pub fn frame_policy(&self) -> crate::ButtonFramePolicy {
        self.button.frame_policy
    }
    pub fn with_frame_policy(mut self, policy: crate::ButtonFramePolicy) -> Self {
        self.button.frame_policy = policy;
        self
    }
    pub fn set_frame_policy(&mut self, policy: crate::ButtonFramePolicy) {
        self.button.frame_policy = policy;
    }
    pub fn chrome_height(&self) -> Option<f32> {
        self.button.chrome_height
    }
    pub fn with_chrome_height(mut self, height: Option<f32>) -> Self {
        self.button.chrome_height = height;
        self
    }
    pub fn set_chrome_height(&mut self, height: Option<f32>) {
        self.button.chrome_height = height;
    }
    fn name(&self) -> &str {
        self.accessible_name.as_deref().unwrap_or(&self.label)
    }
    fn scale(text: &mut dyn ButtonTextRenderer) -> f32 {
        let scale = text.resolved_font_size(proposal::TextRole::UiBody)
            / proposal::TextRole::UiBody.style().size;
        if scale.is_finite() && scale > 0. {
            scale
        } else {
            1.
        }
    }
    fn icon_side(text: &mut dyn ButtonTextRenderer) -> f32 {
        proposal::metric("size", "size-icon-standard").expect("regular icon DS token")
            * Self::scale(text)
    }
    fn gap(text: &mut dyn ButtonTextRenderer) -> f32 {
        proposal::metric("spacing", "space-tight").expect("button content gap DS token")
            * Self::scale(text)
    }
    fn measure_body(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> Size {
        if let Some(content) = self.content.first_mut() {
            return content.measure_intrinsic(text, available).size;
        }
        let has_icon = self.image.is_some() || self.icon.is_some();
        let side = if let Some(icon) = &mut self.icon {
            let m = icon.measure(
                text,
                Size::new(available.width.max(0.), available.height.max(0.)),
            );
            m.size.width.max(m.size.height)
        } else if self.image.is_some() {
            Self::icon_side(text).min(available.width.max(0.))
        } else {
            0.
        };
        let gap = if has_icon && !self.label.is_empty() {
            Self::gap(text)
        } else {
            0.
        };
        let vertical = matches!(
            self.image_position,
            ButtonImagePosition::Above | ButtonImagePosition::Below
        );
        let label = if self.label.is_empty() {
            Size::default()
        } else {
            text.measure(
                &self.label,
                proposal::TextRole::UiBody,
                if vertical {
                    available.width
                } else {
                    (available.width - side - gap).max(0.)
                },
            )
        };
        if vertical {
            Size::new(side.max(label.width), side + gap + label.height)
        } else {
            Size::new(side + gap + label.width, side.max(label.height))
        }
    }
    fn place_icon_and_label(
        &self,
        text: &mut dyn ButtonTextRenderer,
        body: Rect,
        side: f32,
    ) -> (Rect, Rect) {
        let has_icon = self.image.is_some() || self.icon.is_some();
        let gap = if has_icon && !self.label.is_empty() {
            Self::gap(text)
        } else {
            0.
        };
        let vertical = matches!(
            self.image_position,
            ButtonImagePosition::Above | ButtonImagePosition::Below
        );
        let label_size = if self.label.is_empty() {
            Size::default()
        } else {
            text.measure(
                &self.label,
                proposal::TextRole::UiBody,
                if vertical {
                    body.size.width
                } else {
                    (body.size.width - side - gap).max(0.)
                },
            )
        };
        let leading = matches!(
            self.image_position,
            ButtonImagePosition::Leading | ButtonImagePosition::Above
        );
        let sizes = if leading {
            [Size::new(side, side), label_size]
        } else {
            [label_size, Size::new(side, side)]
        };
        let placed = layout_measured_children(
            body,
            if vertical {
                Axis::Vertical
            } else {
                Axis::Horizontal
            },
            &sizes.map(MeasuredChild::new),
            gap,
            proposal::layout::BUTTON_CONTENT,
            AlignmentOverride {
                main_axis: Some(MainAxisDistribution::Center),
                cross_axis: Some(proposal::layout::CrossAxisAlignment::Center),
            },
        );
        if leading {
            (placed[0].frame, placed[1].frame)
        } else {
            (placed[1].frame, placed[0].frame)
        }
    }
    fn body_frame(&self, layout: crate::ButtonLayout) -> Rect {
        action_content_frame(self, layout, self.measured)
    }
    fn measure_button(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        let outer = Rect::from_origin_size(Point::default(), available);
        let empty = crate::ButtonLayout::resolve_with_text_and_insets(
            text,
            outer,
            None,
            Size::default(),
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::default(),
            self.button.frame_policy,
            self.button.horizontal_inset_override,
        );
        let body_max_w = if let Some(inset) = self.leading_content_inset {
            let leading = (outer.min_x() + inset.max(0.)).min(empty.content.max_x());
            (empty.content.max_x() - leading).max(0.)
        } else {
            empty.content.size.width
        };
        // Empty-label layout supplies horizontal chrome, not a zero-height
        // constraint for arbitrary content. The actual receiving height is the
        // proposal; the measured body then determines the final Button slot.
        self.measured = self.measure_body(text, Size::new(body_max_w, available.height));
        if self.toolbar_icon_slot {
            return MeasuredChild::new(self.toolbar_icon_size(text, self.measured, available));
        }
        if self.button.frame_policy == crate::ButtonFramePolicy::Exact {
            return MeasuredChild::new(if let Some(inset) = self.leading_content_inset {
                Size::new(
                    (inset.max(0.) + self.measured.width).min(available.width),
                    self.measured.height,
                )
            } else {
                self.measured
            });
        }
        let insets = self
            .leading_content_inset
            .map(|inset| {
                let trailing = self.button.horizontal_inset_override.map_or_else(
                    || ButtonSize::Body.scaled_horizontal_inset(text, proposal::TextRole::UiBody),
                    |(_, trailing)| trailing.max(0.),
                );
                (inset.max(0.), trailing)
            })
            .or(self.button.horizontal_inset_override);
        MeasuredChild::new(ButtonModel::measure_content_with_insets(
            text,
            self.measured,
            available.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            insets,
        ))
    }
}
impl Button {
    pub(super) fn prepare_button_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        if self.help.is_some()
            && self.help_revision == u64::MAX
            && hooks
                .iter()
                .any(|h| h.kind == PreparedHookKind::Lifecycle(Lifecycle::Unmounted))
        {
            return Err(PreparedEffectsError::CounterCapacity);
        }
        Ok(PreparedEffects::local(self, hooks))
    }

    pub(super) fn prepare_button_action(
        &self,
        request: ComponentActionRequest<'_>,
    ) -> ActionPreparation {
        let owned = action_preparation::OwnedActionRequest::read(request);
        if self
            .decide_action(&owned)
            .is_some_and(|decision| decision.response.activated && self.activations == usize::MAX)
        {
            return action_preparation::rejected();
        }
        action_preparation::prepare_original(
            self,
            request,
            Self::decide_action,
            Self::apply_action,
            |decision| (decision.response, decision.focus),
        )
    }
}
impl Component for Button {
    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("component", "Button"),
            InspectionProperty::new("label", &self.label),
            InspectionProperty::new("variant", format!("{:?}", self.variant)),
            InspectionProperty::new("state", format!("{:?}", self.button.state)),
            InspectionProperty::new(
                "activation_phase",
                format!("{:?}", self.pointer_activation_phase),
            ),
            InspectionProperty::new("activations", self.activations),
            InspectionProperty::new(
                "leading_content_inset",
                format!("{:?}", self.leading_content_inset),
            ),
        ])
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }

    fn focus_boundary_visible(&self) -> bool {
        (self.capsule_segment.is_some() || !(self.toolbar_presentation || self.toolbar_icon_slot))
            && self.button.state.focused
            && self.button.state.focus_visible
    }
    fn set_clip_region(&mut self, region: Option<tekes_ui_scene::ClipRegion>) {
        self.button.set_clip_region(region);
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        if self.content.is_empty() {
            CurveSubtreeCapability::Local
        } else {
            CurveSubtreeCapability::Recursive
        }
    }

    fn focus_decoration_above_siblings(&self) -> bool {
        (self.capsule_segment.is_some()
            || (!(self.toolbar_presentation || self.toolbar_icon_slot)
                && self.button.frame_policy == crate::ButtonFramePolicy::Exact))
            && self.button.state.focused
            && self.button.state.focus_visible
            && !self.button.state.disabled
            && !self.button.state.loading
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let Some(d) = self.decide_action(&action_preparation::OwnedActionRequest::Accessibility(
            part,
            action.clone(),
        )) else {
            return ContentResponse::default();
        };
        let r = d.response;
        self.apply_action(d);
        r
    }

    fn requires_icon_services(&self) -> bool {
        self.icon.is_some()
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.icon.as_ref().and_then(Component::icon_error)
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.set_help(None);
        }
        if let Some(icon) = &mut self.icon {
            icon.lifecycle(event);
        }
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.content.len() != usize::from(self.custom) {
            return Err(CompositionError::InvalidSlotCount);
        }
        if (self.image.is_some() || self.icon.is_some() || !self.content.is_empty())
            && self.name().trim().is_empty()
        {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.content
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.content)
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        action_child_theme(self, theme)
    }
    fn help(&self) -> Option<(&str, u64)> {
        self.help.as_deref().map(|text| (text, self.help_revision))
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.measure_button(text, available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.measure_button(text, frame.size);
        let layout = resolve_action_slot(self, self.measured, frame, clip, text);
        let body = self.body_frame(layout);
        if let Some(content) = self.content.first_mut() {
            content.arrange(text, body, body.intersection(&clip).unwrap_or_default());
        } else if self.icon.is_some() {
            let side = Self::icon_side(text)
                .min(body.size.width)
                .min(body.size.height);
            let (icon_slot, _) = self.place_icon_and_label(text, body, side);
            let rect = icon_frame(icon_slot, side, text.backing_scale());
            if let Some(icon) = &mut self.icon {
                icon.arrange(text, rect, body.intersection(&clip).unwrap_or_default());
            }
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if !crate::widgets::valid_frame(frame) || !crate::widgets::valid_clip(clip) {
            self.button.clear_layout();
            return;
        }
        let variant = self.presentation_variant();
        if self.toolbar_presentation || variant == ButtonVariant::Toolbar {
            self.button.set_surface("toolbar");
        } else if theme.surface_declared || self.transparent_chrome {
            self.button.set_surface(theme.surface());
        } else {
            // Reused owners return to the original variant default after a
            // declaration is cleared; a previous explicit Surface cannot leak.
            self.button.clear_surface();
        }
        // Plain labels keep their existing production Button rendering and label service.
        if self.image.is_none()
            && self.icon.is_none()
            && self.content.is_empty()
            && !self.transparent_chrome
            && self.leading_content_inset.is_none()
        {
            let align = if self.content_alignment == MainAxisDistribution::Start {
                ButtonTextAlign::Leading
            } else {
                ButtonTextAlign::Center
            };
            if self.content_alignment != MainAxisDistribution::End {
                let focus_visible = self.button.state.focus_visible;
                let suppress_focus_ink = (self.toolbar_presentation || self.toolbar_icon_slot)
                    && self.capsule_segment.is_none();
                if suppress_focus_ink {
                    self.button.state.focus_visible = false;
                }
                self.button.render(
                    scene,
                    &mut TextAdapter(text),
                    &self.label,
                    frame,
                    Some(clip),
                    theme.palette,
                    variant,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                    align,
                );
                if suppress_focus_ink && self.button.state.focused {
                    self.button.state.focus_visible = focus_visible;
                }
                return;
            }
        }
        self.measure_button(text, frame.size);
        let layout = self.render_toolbar_icon_frame(
            scene,
            text,
            self.measured,
            frame,
            Some(clip),
            theme.palette,
            variant,
        );
        if !self.content.is_empty() {
            return;
        } // Host paints the original decorative subtree once.
        let Some(visible) = layout.visible else {
            return;
        };
        let body = self.body_frame(layout);
        let has_icon = self.image.is_some() || self.icon.is_some();
        let side = if has_icon {
            Self::icon_side(text)
                .min(body.size.width)
                .min(body.size.height)
        } else {
            0.
        };
        let (icon, label) = self.place_icon_and_label(text, body, side);
        scene.push_clip(visible);
        if let Some(image) = self.image {
            scene.push_image(ImageQuad {
                rect: icon_frame(icon, side, text.backing_scale()),
                image,
                opacity: if self.button.state.disabled {
                    proposal::metric("opacity", "alpha-wash").expect("DS subdued image opacity")
                } else {
                    1.
                },
            });
        }
        let child_theme = action_child_theme(self, theme);
        if self.icon.is_some() {
            let disabled = self.button.state.disabled;
            let rect = icon_frame(icon, side, text.backing_scale());
            if let Some(icon_content) = &mut self.icon {
                icon_content.set_disabled(disabled);
                icon_content.paint(scene, text, rect, visible, child_theme);
            }
        }
        if !self.label.is_empty() {
            text.draw(
                scene,
                &self.label,
                proposal::TextRole::UiBody,
                label,
                ButtonTextAlign::Center,
                child_theme.foreground.unwrap(),
            );
        }
        scene.pop_clip();
    }
    fn focusable(&self) -> bool {
        !self.button.state.disabled && !self.button.state.loading
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.button.state.focused = true;
            self.button.state.focus_visible = visible
        } else {
            self.button.blur()
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        // Unavailable actions do not consume keyboard input. Pointer motion
        // still reaches the original model so re-enabling cannot retain hover.
        if !self.focusable()
            && matches!(
                event,
                ContentEvent::Key { .. } | ContentEvent::TextCommand(_)
            )
        {
            self.button.synchronize_activation();
            return ContentResponse::default();
        }
        if matches!(event, ContentEvent::PointerDown(_)) {
            let Some(d) = self.decide_action(&action_preparation::OwnedActionRequest::Pointer(
                event.clone(),
            )) else {
                return ContentResponse::default();
            };
            let r = d.response;
            self.apply_action(d);
            return r;
        }
        let mut r = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(p) => {
                self.button.pointer_move_at(*p);
                r.changed = true
            }
            ContentEvent::PointerUp(p) => {
                r.activated = self.button.pointer_up_at(*p);
                r.consumed = true;
                r.changed = true
            }
            ContentEvent::TextCommand(
                TextEditCommand::InsertNewline | TextEditCommand::InsertLineBreak,
            ) => {
                r.activated = self.button.key(ControlKey::Enter, true);
                r.consumed = true;
                r.changed = true;
            }
            ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            } => {
                let was_pressed = self.button.state.pressed;
                self.button.cancel();
                r.changed = was_pressed;
            }
            ContentEvent::Key { key, down } => {
                r.activated = self.button.key(*key, *down);
                r.consumed = matches!(key, ControlKey::Space | ControlKey::Enter);
                r.changed = r.consumed
            }
            ContentEvent::Cancel => self.button.cancel(),
            _ => {}
        }
        if r.activated {
            self.activations += 1
        }
        r
    }

    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Button, self.name());
        s.disabled = self.button.state.disabled || self.button.state.loading;
        s.selected = self.button.state.selected;
        s
    }
}

pub(super) fn action_content_frame(
    action: &Button,
    layout: crate::ButtonLayout,
    measured: Size,
) -> Rect {
    let bounds = if let Some(inset) = action.leading_content_inset {
        let leading = (layout.frame.min_x() + inset).min(layout.content.max_x());
        Rect::new(
            leading,
            layout.content.min_y(),
            (layout.content.max_x() - leading).max(0.),
            layout.content.size.height,
        )
    } else {
        layout.content
    };
    let width = measured.width.min(bounds.size.width).max(0.);
    let height = measured.height.min(bounds.size.height).max(0.);
    let offset = match action.content_alignment {
        MainAxisDistribution::Start => 0.,
        MainAxisDistribution::Center | MainAxisDistribution::SpaceBetween => {
            (bounds.size.width - width) * 0.5
        }
        MainAxisDistribution::End => bounds.size.width - width,
    };
    Rect::new(
        bounds.min_x() + offset,
        bounds.center().y - height * 0.5,
        width,
        height,
    )
}
pub(super) fn resolve_action_slot(
    action: &Button,
    measured: Size,
    frame: Rect,
    clip: Rect,
    text: &mut dyn ButtonTextRenderer,
) -> crate::ButtonLayout {
    crate::ButtonLayout::resolve_with_text_and_insets(
        text,
        frame,
        Some(clip),
        measured,
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Leading,
        action.button.frame_policy,
        action.button.horizontal_inset_override,
    )
}
pub(super) fn measure_action_slot(
    action: &Button,
    content: &mut Content,
    measured: &mut Size,
    text: &mut dyn ButtonTextRenderer,
    available: Size,
) -> MeasuredChild {
    if action.toolbar_icon_slot {
        *measured = content.measure_intrinsic(text, available).size;
        return MeasuredChild::new(action.toolbar_icon_size(text, *measured, available));
    }
    if action.button.frame_policy == crate::ButtonFramePolicy::Exact {
        *measured = content.measure(text, available).size;
        return MeasuredChild::new(*measured);
    }
    let frame = Rect::from_origin_size(Point::default(), available);
    let empty = crate::ButtonLayout::resolve_with_text(
        text,
        frame,
        None,
        Size::default(),
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Leading,
        crate::ButtonFramePolicy::Minimum,
    );
    *measured = content
        .measure(text, Size::new(empty.content.size.width, available.height))
        .size;
    let layout = resolve_action_slot(
        action,
        *measured,
        Rect::new(0., 0., available.width, 0.),
        frame,
        text,
    );
    MeasuredChild::new(Size::new(
        (measured.width + available.width - empty.content.size.width).min(available.width),
        layout.frame.size.height,
    ))
}
pub(super) fn action_child_theme(action: &Button, theme: ContentTheme) -> ContentTheme {
    // The original model already advanced its hover/press channels during paint.
    // Decorative text uses those same complete endpoints and inherited Surface.
    let (text, icon) = if action.toolbar_presentation
        || action.transparent_chrome
        || action.variant == ButtonVariant::Toolbar
        || theme.surface_declared
    {
        let colors = action.button.surface_colors(
            theme.palette,
            action.presentation_variant(),
            if action.toolbar_presentation || action.variant == ButtonVariant::Toolbar {
                "toolbar"
            } else {
                theme.surface()
            },
        );
        (colors.text, colors.icon)
    } else {
        // Same undeclared palette/variant foreground as the original label and
        // decorative Content path. Explicit boundaries use generated endpoints.
        let role = if action.button.state.disabled {
            "tertiary-text"
        } else if action.variant == ButtonVariant::Primary {
            "action-primary-foreground"
        } else {
            "primary-text"
        };
        let ink = theme
            .palette
            .color(role)
            .expect("original Button foreground role");
        (ink, ink)
    };
    ContentTheme {
        foreground: Some(text),
        icon_foreground: Some(icon),
        ..theme
    }
}

// Rasterized artwork must retain its sampling phase as a segment translates.
// Only paint origin is aligned; typography-derived extent and original action
// layout/hit/hover geometry stay unchanged, including fractional font scales.
fn icon_frame(frame: Rect, side: f32, backing_scale: f32) -> Rect {
    let scale = if backing_scale.is_finite() && backing_scale > 0. {
        backing_scale
    } else {
        1.
    };
    let snap = |value: f32| (value * scale).round() / scale;
    Rect::new(
        snap(frame.center().x - side * 0.5),
        snap(frame.center().y - side * 0.5),
        side,
        side,
    )
}

#[derive(Clone, Debug, PartialEq)]
struct PreparedButtonAction {
    button: crate::widgets::ButtonActionDecision,
    activations: usize,
    response: ContentResponse,
    focus: PreparedActionFocus,
}
impl action_preparation::OriginalActionDecision for PreparedButtonAction {}
impl Button {
    fn decide_action(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedButtonAction> {
        let (button, is_pointer) = match request {
            action_preparation::OwnedActionRequest::Pointer(event) => {
                let event = event.routing_event();
                let ContentEvent::PointerDown(point) = event.as_ref() else {
                    return None;
                };
                (
                    self.button.prepare_content_down(
                        *point,
                        self.pointer_activation_phase == ButtonActivationPhase::Press,
                    ),
                    true,
                )
            }
            action_preparation::OwnedActionRequest::Accessibility(
                None,
                AccessibilityAction::Press,
            ) => {
                if !self.focusable() {
                    return None;
                }
                (self.button.prepare_accessibility_press(), false)
            }
            _ => return None,
        };
        let response = ContentResponse {
            consumed: if is_pointer {
                button.consumed
            } else {
                button.activated
            },
            changed: if is_pointer {
                button.consumed
            } else {
                button.activated
            },
            capture_pointer: is_pointer && button.consumed,
            activated: button.activated,
            ..Default::default()
        };
        Some(PreparedButtonAction {
            button,
            activations: self.activations,
            focus: if response.consumed {
                PreparedActionFocus::Owner
            } else {
                PreparedActionFocus::None
            },
            response,
        })
    }

    fn apply_action(&mut self, decision: PreparedButtonAction) {
        let activated = decision.button.activated;
        let next_activations = if activated {
            let Some(next) = self.activations.checked_add(1) else {
                return;
            };
            Some(next)
        } else {
            None
        };
        self.button.apply_content_action(decision.button);
        if let Some(next) = next_activations {
            self.activations = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_key_observation_cancels_space_before_reenable() {
        for loading in [false, true] {
            for text_command in [false, true] {
                let mut button = Button::new("Apply");
                button.set_focus(true, true);
                assert!(
                    button
                        .event(&ContentEvent::Key {
                            key: ControlKey::Space,
                            down: true
                        })
                        .consumed
                );
                assert!(button.button.state.pressed);
                button.button.state.disabled = !loading;
                button.button.state.loading = loading;
                let observed = if text_command {
                    ContentEvent::TextCommand(TextEditCommand::InsertNewline)
                } else {
                    ContentEvent::Key {
                        key: ControlKey::Enter,
                        down: true,
                    }
                };
                assert_eq!(button.event(&observed), ContentResponse::default());
                assert!(!button.button.state.pressed);
                button.button.state.disabled = false;
                button.button.state.loading = false;
                assert!(
                    !button
                        .event(&ContentEvent::Key {
                            key: ControlKey::Space,
                            down: false
                        })
                        .activated
                );
                assert_eq!(button.activations, 0);
                button.event(&ContentEvent::Key {
                    key: ControlKey::Space,
                    down: true,
                });
                assert!(
                    button
                        .event(&ContentEvent::Key {
                            key: ControlKey::Space,
                            down: false
                        })
                        .activated
                );
                assert_eq!(button.activations, 1);
            }
        }
    }
    #[derive(Default)]
    struct Text(Vec<Rgba>);
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(40_f32.min(width), role.style().line_height)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            ink: Rgba,
        ) {
            self.0.push(ink);
        }
    }
    const ROOT: ComponentId = ComponentId(83900);
    const ACTION: ComponentId = ComponentId(83901);
    fn frame() -> Rect {
        Rect::new(10., 20., 160., 60.)
    }
    fn colors(scene: &Scene, text: &Text) -> (Rgba, Rgba) {
        let fill = scene
            .primitives()
            .iter()
            .find_map(|p| match p {
                tekes_ui_scene::Primitive::Quad(q) => Some(q.fill),
                _ => None,
            })
            .expect("actual original Button fill");
        assert_eq!(text.0.len(), 1);
        (fill, text.0[0])
    }
    fn original_colors(palette: Palette, variant: ButtonVariant) -> (Rgba, Rgba) {
        let mut original = ButtonModel::default();
        let mut text = Text::default();
        let mut scene = Scene::new();
        original.render(
            &mut scene,
            &mut text,
            "Apply",
            frame(),
            Some(frame()),
            palette,
            variant,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
        colors(&scene, &text)
    }
    fn paint_colors(host: &mut ContentHost, palette: Palette) -> (Rgba, Rgba) {
        let mut scene = Scene::new();
        let mut text = Text::default();
        host.render(&mut scene, &mut text, frame(), frame(), palette)
            .unwrap();
        colors(&scene, &text)
    }
    fn host(custom: bool, variant: ButtonVariant) -> ContentHost {
        let mut button = if custom {
            Button::new(Content::new(ComponentId(83902), TextContent::new("Apply")))
                .with_accessible_name("Apply")
        } else {
            Button::new("Apply")
        };
        button.variant = variant;
        ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                children: vec![Content::new(ACTION, button)],
                flexible_child: None,
            },
        ))
        .unwrap()
    }
    fn declared_colors(palette: Palette, surface: &str) -> (Rgba, Rgba) {
        let colors = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            surface,
            "action-quiet",
            "rest",
        )
        .unwrap();
        (colors.background, colors.text)
    }

    #[test]
    fn undeclared_button_variants_preserve_original_label_and_custom_colors() {
        for theme in tekes_ui_tokens::Theme::ALL {
            let palette: Palette = theme.into();
            for variant in [
                ButtonVariant::Quiet,
                ButtonVariant::Panel,
                ButtonVariant::Primary,
            ] {
                for custom in [false, true] {
                    assert_eq!(
                        paint_colors(&mut host(custom, variant), palette),
                        original_colors(palette, variant),
                        "{theme:?}/{variant:?}/custom={custom}"
                    );
                }
            }
        }
    }

    #[test]
    fn explicit_surface_and_clear_use_the_same_paint_and_child_foreground_path() {
        let palette: Palette = tekes_ui_tokens::Theme::LIGHT.into();
        for custom in [false, true] {
            let mut host = host(custom, ButtonVariant::Panel);
            let original = original_colors(palette, ButtonVariant::Panel);
            assert_eq!(paint_colors(&mut host, palette), original);
            host.get_mut(ROOT)
                .unwrap()
                .set_surface(Some("sidebar"))
                .unwrap();
            assert_eq!(
                paint_colors(&mut host, palette),
                declared_colors(palette, "sidebar")
            );
            host.get_mut(ROOT)
                .unwrap()
                .set_surface(Some("canvas"))
                .unwrap();
            assert_eq!(
                paint_colors(&mut host, palette),
                declared_colors(palette, "canvas")
            );
            host.get_mut(ROOT).unwrap().set_surface(None).unwrap();
            assert_eq!(paint_colors(&mut host, palette), original);
            host.get_mut(ACTION)
                .unwrap()
                .set_surface(Some("code"))
                .unwrap();
            assert_eq!(
                paint_colors(&mut host, palette),
                declared_colors(palette, "code")
            );
            host.get_mut(ACTION).unwrap().set_surface(None).unwrap();
            assert_eq!(paint_colors(&mut host, palette), original);
            assert!(
                host.set_inherited_surface("canvas").unwrap(),
                "same name is a new explicit declaration"
            );
            assert!(!host.set_inherited_surface("canvas").unwrap());
            assert_eq!(
                paint_colors(&mut host, palette),
                declared_colors(palette, "canvas")
            );
        }
    }

    #[test]
    fn icon_label_buttons_coexist_and_paint_icon_with_button_ink() {
        use std::cell::Cell;
        use std::rc::Rc;

        #[derive(Default)]
        struct IconText {
            draws: Vec<(String, Rgba)>,
            tints: Vec<Rgba>,
            fail_prepare: bool,
        }
        impl crate::layout_view::IconRenderer for IconText {
            fn leading_ink_inset(&mut self, _: &str) -> Result<f32, IconRenderError> {
                Ok(1.)
            }
            fn prepare_icon(
                &mut self,
                name: &str,
                _: f32,
                _: f32,
                tint: Rgba,
            ) -> Result<IconImage, IconRenderError> {
                if self.fail_prepare {
                    return Err(IconRenderError::Upload);
                }
                self.tints.push(tint);
                let _ = name;
                Ok(IconImage::new(tekes_ui_scene::ImageId(7), Rc::new(())))
            }
            fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
        }
        impl ButtonTextRenderer for IconText {
            fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
                Some(self)
            }
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                value: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                ink: Rgba,
            ) {
                self.draws.push((value.into(), ink));
            }
        }

        let mut primary = Button::icon_label("tekes-new", "Create").unwrap();
        primary.variant = ButtonVariant::Primary;
        primary.image_position = ButtonImagePosition::Above;
        let mut back = Button::icon_label("tekes-back", "返回")
            .unwrap()
            .with_icon_ink_leading(true);
        back.button.state.disabled = true;

        assert!(primary.requires_icon_services());
        assert_eq!(
            primary.curve_subtree_capability(),
            CurveSubtreeCapability::Local
        );
        assert_eq!(
            back.curve_subtree_capability(),
            CurveSubtreeCapability::Local
        );

        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Horizontal,
                    gap: 8.,
                },
                children: vec![
                    Content::new(ComponentId(83910), primary),
                    Content::new(ComponentId(83911), back),
                ],
                flexible_child: None,
            },
        ))
        .unwrap();
        let mut text = IconText::default();
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut text,
            frame(),
            frame(),
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        assert_eq!(text.draws.len(), 2);
        assert_eq!(text.tints.len(), 2);
        assert_eq!(text.draws[0].1, text.tints[0]);
        assert_eq!(text.draws[1].1, text.tints[1]);

        text.fail_prepare = true;
        let err = host
            .render(
                &mut Scene::new(),
                &mut text,
                frame(),
                frame(),
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap_err();
        assert_eq!(
            err,
            CompositionError::IconResource {
                owner: ComponentId(83910),
                reason: IconRenderError::Upload,
            }
        );
        let replaced = host
            .get_mut(ComponentId(83910))
            .unwrap()
            .component_mut::<Button>()
            .unwrap();
        replaced.set_contents("Plain");
        assert!(!replaced.requires_icon_services());
        assert_eq!(replaced.icon_error(), None);
        let _ = Cell::new(0);
    }

    #[test]
    fn controlled_button_setters_cancel_interaction_and_invalidate_measured_cache() {
        let mut host = host(false, ButtonVariant::Primary);
        let mut text = Text::default();
        host.render(
            &mut Scene::new(),
            &mut text,
            frame(),
            frame(),
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let center = host.get(ACTION).unwrap().frame().center();
        assert!(host.event(&ContentEvent::PointerMove(center)).changed);
        assert!(host.event(&ContentEvent::PointerDown(center)).consumed);
        {
            let button = host
                .get_mut(ACTION)
                .unwrap()
                .component_mut::<Button>()
                .unwrap();
            assert!(button.is_hovered() && button.is_pressed());
            button.set_disabled(true);
            assert!(button.is_disabled());
            assert!(!button.is_hovered() && !button.is_pressed());
            button.set_disabled(false);
        }

        assert!(host.event(&ContentEvent::PointerMove(center)).changed);
        {
            let button = host
                .get_mut(ACTION)
                .unwrap()
                .component_mut::<Button>()
                .unwrap();
            assert!(button.is_hovered());
            button.set_loading(true);
            assert!(button.is_loading());
            assert!(!button.is_hovered() && !button.is_pressed());
        }

        struct LabelText;
        impl ButtonTextRenderer for LabelText {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
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
                _: Rgba,
            ) {
            }
        }
        let mut label_text = LabelText;
        let mut resizable = Button::new("Ok");
        let short = resizable
            .measure(&mut label_text, Size::new(500., 80.))
            .size
            .width;
        resizable.set_label("Longer confirmation label");
        assert_eq!(resizable.measured, Size::default());
        let long = resizable
            .measure(&mut label_text, Size::new(500., 80.))
            .size
            .width;
        assert!(long > short);

        let mut bare = Button::new("Open").with_frame_policy(crate::ButtonFramePolicy::Exact);
        let bare_w = bare
            .measure(&mut label_text, Size::new(300., 40.))
            .size
            .width;
        let mut exact_leading = Button::new("Open")
            .with_frame_policy(crate::ButtonFramePolicy::Exact)
            .with_leading_content_inset(Some(22.0));
        let exact_w = exact_leading
            .measure(&mut label_text, Size::new(300., 40.))
            .size
            .width;
        assert_eq!(exact_w, 22.0 + bare_w);

        let mut min_default = Button::new("Open");
        let default_w = min_default
            .measure(&mut label_text, Size::new(300., 40.))
            .size
            .width;
        let mut min_leading = Button::new("Open").with_leading_content_inset(Some(28.0));
        let leading_w = min_leading
            .measure(&mut label_text, Size::new(300., 40.))
            .size
            .width;
        let regular_inset =
            ButtonSize::Body.scaled_horizontal_inset(&mut label_text, proposal::TextRole::UiBody);
        assert_eq!(leading_w, default_w - regular_inset + 28.0);
    }
}
