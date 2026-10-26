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
use tekes_ui_scene::{ImageId, ImageQuad};
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

/// Text, image, image/text or original decorative Content; one action and AX identity.
/// Pure images/custom content require `with_accessible_name` before mounting.
/// Registered image resources remain host-owned; regular geometry comes from DS.
#[derive(Debug)]
pub struct Button {
    pub button: ButtonModel,
    /// Visible text; a separate accessible name never replaces this source.
    pub label: String,
    pub variant: ButtonVariant,
    pub activations: usize,
    pub pointer_activation_phase: ButtonActivationPhase,
    pub image_position: ButtonImagePosition,
    pub content_alignment: MainAxisDistribution,
    accessible_name: Option<String>,
    image: Option<ImageId>,
    content: Vec<Content>,
    custom: bool,
    measured: Size,
    help: Option<String>,
    help_revision: u64,
    toolbar_presentation: bool,
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
            accessible_name: None,
            image: None,
            content: Vec::new(),
            custom: false,
            measured: Size::default(),
            help: None,
            help_revision: 0,
            toolbar_presentation: false,
        };
        button.set_contents(contents);
        button
    }
    pub(crate) fn set_toolbar_presentation(&mut self, toolbar: bool) {
        self.toolbar_presentation = toolbar;
    }
    fn presentation_variant(&self) -> ButtonVariant {
        if self.toolbar_presentation && self.variant != ButtonVariant::Primary {
            ButtonVariant::Toolbar
        } else { self.variant }
    }
    /// Replacing presentation never creates another action or resets its counter.
    pub fn set_contents(&mut self, contents: impl Into<ButtonContents>) {
        self.label.clear();
        self.image = None;
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
        let side = if self.image.is_some() {
            Self::icon_side(text).min(available.width.max(0.))
        } else {
            0.
        };
        let gap = if self.image.is_some() && !self.label.is_empty() {
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
    fn body_frame(&self, layout: crate::ButtonLayout) -> Rect {
        // Exact is a compact chrome allocation, not a request to stretch its content.
        let bounds = layout.content;
        let width = self.measured.width.min(bounds.size.width).max(0.);
        let height = self.measured.height.min(bounds.size.height).max(0.);
        let offset = match self.content_alignment {
            MainAxisDistribution::Start => 0.,
            MainAxisDistribution::End => (bounds.size.width - width).max(0.),
            _ => (bounds.size.width - width).max(0.) * 0.5,
        };
        Rect::new(
            bounds.min_x() + offset,
            bounds.center().y - height * 0.5,
            width,
            height,
        )
    }
    fn measure_button(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        let outer = Rect::from_origin_size(Point::default(), available);
        let empty = crate::ButtonLayout::resolve_with_text(
            text,
            outer,
            None,
            Size::default(),
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::default(),
            self.button.frame_policy,
        );
        // Empty-label layout supplies horizontal chrome, not a zero-height
        // constraint for arbitrary content. The actual receiving height is the
        // proposal; the measured body then determines the final Button slot.
        self.measured = self.measure_body(text, Size::new(empty.content.size.width, available.height));
        MeasuredChild::new(
            if self.button.frame_policy == crate::ButtonFramePolicy::Exact {
                self.measured
            } else {
                ButtonModel::measure_content(
                    text,
                    self.measured,
                    available.width,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
            },
        )
    }
}
impl Component for Button {
    fn table_transaction_compatibility(&self)->TableTransactionCompatibility{TableTransactionCompatibility::StableHooks}
    fn prepared_effects_work(&self,n:usize)->Result<usize,PreparedEffectsError>{PreparedEffects::local_work(n)}
    fn prepare_effects(&self,hooks:&[PreparedHook])->Result<PreparedEffects,PreparedEffectsError>{
        if self.help.is_some()&&self.help_revision==u64::MAX&&hooks.iter().any(|h|h.kind==PreparedHookKind::Lifecycle(Lifecycle::Unmounted)){return Err(PreparedEffectsError::CounterCapacity);}
        Ok(PreparedEffects::local(self,hooks))
    }


    fn focus_boundary_visible(&self)->bool {self.button.state.focused&&self.button.state.focus_visible}
    fn set_clip_region(&mut self,region:Option<tekes_ui_scene::ClipRegion>) {self.button.set_clip_region(region);}


    fn curve_subtree_capability(&self) -> CurveSubtreeCapability { if self.content.is_empty() { CurveSubtreeCapability::Local } else { CurveSubtreeCapability::Recursive } }
    fn action_transaction_compatibility(&self)->ActionTransactionCompatibility {ActionTransactionCompatibility::StableHooks}
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
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

    fn focus_decoration_above_siblings(&self) -> bool {
        self.button.frame_policy == crate::ButtonFramePolicy::Exact
            && self.button.state.focused
            && self.button.state.focus_visible
            && !self.button.state.disabled
            && !self.button.state.loading
    }
    fn accessibility_action(&mut self,part:Option<usize>,action:&AccessibilityAction)->ContentResponse {
        let Some(d)=self.decide_action(&action_preparation::OwnedActionRequest::Accessibility(part,action.clone()))else{return ContentResponse::default();};
        let r=d.response;self.apply_action(d);r
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.set_help(None);
        }
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.content.len() != usize::from(self.custom) {
            return Err(CompositionError::InvalidSlotCount);
        }
        if (self.image.is_some() || !self.content.is_empty()) && self.name().trim().is_empty() {
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
        } else if theme.surface_declared {
            self.button.set_surface(theme.surface());
        } else {
            // Reused owners return to the original variant default after a
            // declaration is cleared; a previous explicit Surface cannot leak.
            self.button.clear_surface();
        }
        // Plain labels keep their existing production Button rendering and label service.
        if self.image.is_none() && self.content.is_empty() {
            let align = if self.content_alignment == MainAxisDistribution::Start {
                ButtonTextAlign::Leading
            } else {
                ButtonTextAlign::Center
            };
            if self.content_alignment != MainAxisDistribution::End {
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
                return;
            }
        }
        self.measure_button(text, frame.size);
        let layout = self.button.render_content_frame(
            scene,
            &mut TextAdapter(text),
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
        let side = if self.image.is_some() {
            Self::icon_side(text)
                .min(body.size.width)
                .min(body.size.height)
        } else {
            0.
        };
        let gap = if self.image.is_some() && !self.label.is_empty() {
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
        let (icon, label) = if leading {
            (placed[0].frame, placed[1].frame)
        } else {
            (placed[1].frame, placed[0].frame)
        };
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
        if !self.label.is_empty() {
            text.draw(
                scene,
                &self.label,
                proposal::TextRole::UiBody,
                label,
                ButtonTextAlign::Center,
                action_child_theme(self, theme).foreground.unwrap(),
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
        if !self.focusable() && matches!(event, ContentEvent::Key { .. } | ContentEvent::TextCommand(_)) {
            self.button.synchronize_activation();
            return ContentResponse::default();
        }
        if matches!(event,ContentEvent::PointerDown(_)) {
            let Some(d)=self.decide_action(&action_preparation::OwnedActionRequest::Pointer(event.clone()))else{return ContentResponse::default();};
            let r=d.response;self.apply_action(d);return r;
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
            ContentEvent::TextCommand(command)
                if matches!(command.as_str(), "insertNewline:" | "insertLineBreak:") =>
            {
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

pub(super) fn resolve_action_slot(
    action: &Button,
    measured: Size,
    frame: Rect,
    clip: Rect,
    text: &mut dyn ButtonTextRenderer,
) -> crate::ButtonLayout {
    crate::ButtonLayout::resolve_with_text(
        text,
        frame,
        Some(clip),
        measured,
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Leading,
        action.button.frame_policy,
    )
}
pub(super) fn measure_action_slot(
    action: &Button,
    content: &mut Content,
    measured: &mut Size,
    text: &mut dyn ButtonTextRenderer,
    available: Size,
) -> MeasuredChild {
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
    let (text, icon) = if action.toolbar_presentation || action.variant == ButtonVariant::Toolbar || theme.surface_declared {
        let colors = action.button.surface_colors(theme.palette, action.presentation_variant(), if action.toolbar_presentation || action.variant == ButtonVariant::Toolbar { "toolbar" } else { theme.surface() });
        (colors.text, colors.icon)
    } else {
        // Same undeclared palette/variant foreground as the original label and
        // decorative Content path. Explicit boundaries use generated endpoints.
        let role = if action.button.state.disabled { "tertiary-text" }
            else if action.variant == ButtonVariant::Primary { "action-primary-foreground" }
            else { "primary-text" };
        let ink = theme.palette.color(role).expect("original Button foreground role");
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
                (self.button.prepare_content_down(
                    *point,
                    self.pointer_activation_phase == ButtonActivationPhase::Press,
                ), true)
            }
            action_preparation::OwnedActionRequest::Accessibility(None, AccessibilityAction::Press) => {
                if !self.focusable() {
                    return None;
                }
                (self.button.prepare_accessibility_press(), false)
            }
            _ => return None,
        };
        let response = ContentResponse {
            consumed: if is_pointer { button.consumed } else { button.activated },
            changed: if is_pointer { button.consumed } else { button.activated },
            capture_pointer: is_pointer && button.consumed,
            activated: button.activated,
            ..Default::default()
        };
        Some(PreparedButtonAction {
            button,
            activations: self.activations,
            focus: if response.consumed { PreparedActionFocus::Owner } else { PreparedActionFocus::None },
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
                assert!(button.event(&ContentEvent::Key { key: ControlKey::Space, down: true }).consumed);
                assert!(button.button.state.pressed);
                button.button.state.disabled = !loading;
                button.button.state.loading = loading;
                let observed = if text_command {
                    ContentEvent::TextCommand("insertNewline:".into())
                } else {
                    ContentEvent::Key { key: ControlKey::Enter, down: true }
                };
                assert_eq!(button.event(&observed), ContentResponse::default());
                assert!(!button.button.state.pressed);
                button.button.state.disabled = false;
                button.button.state.loading = false;
                assert!(!button.event(&ContentEvent::Key { key: ControlKey::Space, down: false }).activated);
                assert_eq!(button.activations, 0);
                button.event(&ContentEvent::Key { key: ControlKey::Space, down: true });
                assert!(button.event(&ContentEvent::Key { key: ControlKey::Space, down: false }).activated);
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
        fn draw(&mut self, _: &mut Scene, _: &str, _: proposal::TextRole, _: Rect, _: ButtonTextAlign, ink: Rgba) {
            self.0.push(ink);
        }
    }
    const ROOT: ComponentId = ComponentId(83900);
    const ACTION: ComponentId = ComponentId(83901);
    fn frame() -> Rect { Rect::new(10., 20., 160., 60.) }
    fn colors(scene: &Scene, text: &Text) -> (Rgba, Rgba) {
        let fill = scene.primitives().iter().find_map(|p| match p {
            tekes_ui_scene::Primitive::Quad(q) => Some(q.fill), _ => None,
        }).expect("actual original Button fill");
        assert_eq!(text.0.len(), 1);
        (fill, text.0[0])
    }
    fn original_colors(palette: Palette, variant: ButtonVariant) -> (Rgba, Rgba) {
        let mut original = ButtonModel::default();
        let mut text = Text::default();
        let mut scene = Scene::new();
        original.render(&mut scene, &mut text, "Apply", frame(), Some(frame()), palette,
            variant, ButtonSize::Body, proposal::TextRole::UiBody, ButtonTextAlign::Center);
        colors(&scene, &text)
    }
    fn paint_colors(host: &mut ContentHost, palette: Palette) -> (Rgba, Rgba) {
        let mut scene = Scene::new();
        let mut text = Text::default();
        host.render(&mut scene, &mut text, frame(), frame(), palette).unwrap();
        colors(&scene, &text)
    }
    fn host(custom: bool, variant: ButtonVariant) -> ContentHost {
        let mut button = if custom {
            Button::new(Content::new(ComponentId(83902), TextContent::new("Apply")))
                .with_accessible_name("Apply")
        } else { Button::new("Apply") };
        button.variant = variant;
        ContentHost::new(Content::new(ROOT, StackContent {
            model: Stack { axis: Axis::Vertical, gap: 0. },
            children: vec![Content::new(ACTION, button)], flexible_child: None,
        })).unwrap()
    }
    fn declared_colors(palette: Palette, surface: &str) -> (Rgba, Rgba) {
        let colors = tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, "action-quiet", "rest").unwrap();
        (colors.background, colors.text)
    }

    #[test]
    fn undeclared_button_variants_preserve_original_label_and_custom_colors() {
        for theme in tekes_ui_tokens::Theme::ALL {
            let palette: Palette = theme.into();
            for variant in [ButtonVariant::Quiet, ButtonVariant::Panel, ButtonVariant::Primary] {
                for custom in [false, true] {
                    assert_eq!(paint_colors(&mut host(custom, variant), palette), original_colors(palette, variant),
                        "{theme:?}/{variant:?}/custom={custom}");
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
            host.get_mut(ROOT).unwrap().set_surface(Some("sidebar")).unwrap();
            assert_eq!(paint_colors(&mut host, palette), declared_colors(palette, "sidebar"));
            host.get_mut(ROOT).unwrap().set_surface(Some("canvas")).unwrap();
            assert_eq!(paint_colors(&mut host, palette), declared_colors(palette, "canvas"));
            host.get_mut(ROOT).unwrap().set_surface(None).unwrap();
            assert_eq!(paint_colors(&mut host, palette), original);
            host.get_mut(ACTION).unwrap().set_surface(Some("code")).unwrap();
            assert_eq!(paint_colors(&mut host, palette), declared_colors(palette, "code"));
            host.get_mut(ACTION).unwrap().set_surface(None).unwrap();
            assert_eq!(paint_colors(&mut host, palette), original);
            assert!(host.set_inherited_surface("canvas").unwrap(), "same name is a new explicit declaration");
            assert!(!host.set_inherited_surface("canvas").unwrap());
            assert_eq!(paint_colors(&mut host, palette), declared_colors(palette, "canvas"));
        }
    }

}
