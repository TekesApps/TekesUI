use super::*;
use crate::ButtonTextRenderer;
use crate::extended::Axis;
use std::{any::Any, collections::BTreeSet};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::proposal;

#[derive(Debug)]
pub struct Content {
    pub(super) id: ComponentId,
    pub(super) component: Box<dyn Component>,
    pub(super) frame: Rect,
    pub(super) clip: Rect,
    pub(super) curve_geometry: Option<panel::NodeGeometry>,
    pub(super) mount: u64,
    pub(super) preserve_activation_focus: bool,
    pub(super) bubble_handled_events: Option<bool>,
    pub(super) padding_focus: Option<PaddingFocusBinding>,
    pub(super) label: Option<label::LabelState>,
    pub(super) help: Option<String>,
    pub(super) help_revision: u64,
    pub(super) surface: Option<&'static str>,
    pub(super) secondary_foreground: bool,
    // Cached value last notified to this owner; notice and ordinary owner
    // notification semantics differ. This is not a generic activity fact.
    pub(super) activity_effective: bool,
    pub(super) activity_parent: bool,
    pub(super) composition_active: bool,
    pub(super) retained_body_paint: bool,
    pub(super) semantic_suppressed: bool,
    pub(super) stack_distribution: Option<proposal::layout::MainAxisDistribution>,
    pub(super) stack_padding: Option<crate::layout::Insets>,
    #[cfg(feature = "inspection")]
    pub(super) inspection_enabled: bool,
    #[cfg(feature = "inspection")]
    pub(super) inspection_paint: Option<InspectionPaint>,
}
impl Content {
    pub(super) fn active(&self) -> bool {
        self.composition_active
            && self.component.active()
            && (self.component.overlay_scope().is_none()
                || self.component.overlay_open() && !self.component.semantics().disabled)
    }
    pub fn new(id: ComponentId, component: impl Component) -> Self {
        Self::new_with_mount(id, component, actions::next_mount())
    }
    pub(super) fn new_with_mount(id: ComponentId, component: impl Component, mount: u64) -> Self {
        Self {
            id,
            component: Box::new(component),
            frame: Rect::default(),
            clip: Rect::default(),
            curve_geometry: None,
            mount,
            preserve_activation_focus: false,
            bubble_handled_events: None,
            padding_focus: None,
            label: None,
            help: None,
            help_revision: 0,
            surface: None,
            secondary_foreground: false,
            activity_effective: true,
            activity_parent: true,
            composition_active: true,
            retained_body_paint: false,
            semantic_suppressed: false,
            stack_distribution: None,
            stack_padding: None,
            #[cfg(feature = "inspection")]
            inspection_enabled: false,
            #[cfg(feature = "inspection")]
            inspection_paint: None,
        }
    }
    /// Hide a decorative slot from AX while preserving mount, layout and paint.
    pub(super) fn suppress_semantics(&mut self) {
        self.semantic_suppressed = true;
    }
    // The retained body stays mounted. Only ordinary paint may outlive activity.
    pub(super) fn set_composition_active(&mut self, active: bool) {
        if self.composition_active == active {
            return;
        }
        if !active {
            empty_state::hide_subtree(self);
            notice::hide_subtree(self);
        }
        self.composition_active = active;
        if !active {
            self.notify_presentation_runnable(false);
        }
        self.refresh_activity(self.activity_parent);
    }
    pub(super) fn retain_body_paint(&mut self, retained: bool) {
        self.retained_body_paint = retained;
    }
    pub(super) fn paint_available(&self) -> bool {
        self.active() || self.retained_body_paint && self.component.active()
    }
    pub(super) fn paint_services_available(&self) -> bool {
        self.paint_available()
            || self.composition_active
                && self.component.paints_children()
                && self.component.overlay_presented()
    }
    pub fn with_surface(
        mut self,
        name: &str,
    ) -> Result<Self, tekes_ui_tokens::surface_palette::SurfaceError> {
        self.set_surface(Some(name))?;
        Ok(self)
    }
    pub fn set_surface(
        &mut self,
        name: Option<&str>,
    ) -> Result<bool, tekes_ui_tokens::surface_palette::SurfaceError> {
        let next = name
            .map(tekes_ui_tokens::surface_palette::canonical_surface)
            .transpose()?;
        let changed = self.surface != next;
        self.surface = next;
        Ok(changed)
    }
    pub fn surface(&self) -> Option<&'static str> {
        self.surface
    }
    pub(crate) fn with_secondary_foreground(mut self) -> Self {
        self.secondary_foreground = true;
        self
    }
    pub(super) fn resolved_theme(&self, theme: ContentTheme) -> ContentTheme {
        let surface = self
            .surface
            .or_else(|| self.component.default_surface())
            .map(|name| {
                tekes_ui_tokens::surface_palette::canonical_surface(name)
                    .expect("component Surface validated before host paint")
            });
        let mut resolved = surface.map_or(theme, |surface| ContentTheme {
            surface,
            surface_declared: true,
            foreground: None,
            icon_foreground: None,
            ..theme
        });
        if self.secondary_foreground {
            resolved.foreground = Some(
                tekes_ui_tokens::surface_palette::resolve_surface(
                    resolved.palette,
                    resolved.surface(),
                    "data-category-label",
                    "rest",
                )
                .expect("secondary label Surface packet")
                .text,
            );
        }
        resolved
    }
    pub(super) fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        let mut child = self.component.child_theme(theme);
        if let Some(surface) = self.surface {
            child.surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)
                .expect("Content Surface validated before host paint");
            child.surface_declared = true;
        }
        child
    }
    pub(super) fn theme_for(&self, id: ComponentId, theme: ContentTheme) -> Option<ContentTheme> {
        let theme = self.resolved_theme(theme);
        if self.id == id {
            return Some(theme);
        }
        let child_theme = self.child_theme(theme);
        self.component
            .children()
            .iter()
            .find_map(|child| child.theme_for(id, child_theme))
    }
    /// Opt in for this TextContent or Button label. Original component,
    /// source, accessible name and interaction state remain owned by this node.
    /// For a custom Button, apply this to its decorative TextContent slot.
    pub fn with_label_layout(
        mut self,
        options: tekes_ui_text::LabelOptions,
    ) -> Result<Self, CompositionError> {
        if self.component::<TextContent>().is_none()
            && !self
                .component::<Button>()
                .is_some_and(Button::supports_label_layout)
        {
            return Err(CompositionError::UnsupportedLabelLayout(self.id));
        }
        self.label = Some(label::LabelState::new(options));
        Ok(self)
    }
    // Toolbar owns a single-line presentation policy, while every descendant
    // retains its source, semantics, mount identity and interaction state.
    pub(super) fn toolbar_label_layouts(&mut self) {
        if let Some(strip) = self.component_mut::<TabStripContent>() {
            strip.set_toolbar_presentation(true);
        }
        if let Some(scroll) = self.component_mut::<ScrollContent>() {
            scroll.toolbar_tabs = scroll.horizontal
                && scroll
                    .children
                    .iter()
                    .any(|c| c.component::<TabStripContent>().is_some());
        }
        if let Some(button) = self.component_mut::<Button>() {
            button.set_toolbar_presentation(true);
        }
        if let Some(toolbar) = self.component_mut::<crate::ToolbarControl>() {
            toolbar.set_embedded(true);
        }
        if self.component::<TextContent>().is_some()
            || self
                .component::<Button>()
                .is_some_and(Button::supports_label_layout)
        {
            let state = self.label.get_or_insert_with(|| {
                label::LabelState::new(tekes_ui_text::LabelOptions::single_line(
                    tekes_ui_text::LabelOverflow::TailEllipsis,
                ))
            });
            state.toolbar();
        }
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.toolbar_label_layouts();
            }
        }
    }
    pub(super) fn reset_label_layouts(&mut self) {
        if let Some(strip) = self.component_mut::<TabStripContent>() {
            strip.set_toolbar_presentation(false);
        }
        if let Some(scroll) = self.component_mut::<ScrollContent>() {
            scroll.toolbar_tabs = false;
        }
        if let Some(button) = self.component_mut::<Button>() {
            button.set_toolbar_presentation(false);
        }
        if let Some(toolbar) = self.component_mut::<crate::ToolbarControl>() {
            toolbar.set_embedded(false);
        }
        // Slot containers may only measure descendants during arrange. A new
        // host frame must not inspect a previous frame's failed capability or
        // replay a paragraph from a replaced font registry before that arrange.
        if let Some(label) = &mut self.label {
            label.reset();
        }
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.reset_label_layouts();
            }
        }
    }
    pub(super) fn label_failure(&self) -> Option<ComponentId> {
        if !self.paint_services_available() {
            return None;
        }
        if self.label.as_ref().is_some_and(|label| label.failed) {
            return Some(self.id);
        }
        self.component
            .children()
            .iter()
            .find_map(Content::label_failure)
    }
    pub fn id(&self) -> ComponentId {
        self.id
    }
    pub fn frame(&self) -> Rect {
        self.frame
    }
    pub fn visible(&self) -> Option<Rect> {
        self.frame.intersection(&self.clip)
    }
    pub(super) fn routes_pointer_children(&self) -> bool {
        !self.component.semantics().role.is_action_owner()
    }
    pub(super) fn pointer_target_at(&self, point: Point) -> bool {
        self.component.interactive()
            && self.curve_contains(point)
            && self
                .visible()
                .is_some_and(|visible| visible.contains(point))
    }
    /// Keep the current keyboard/IME owner on pointer or accessibility Press.
    /// This instance remains eligible for Tab and explicit accessibility Focus.
    /// Intended for auxiliary actions such as Return to latest; original
    /// component input/paint/state remains unchanged.
    pub fn preserving_focus(mut self) -> Self {
        self.preserve_activation_focus = true;
        self
    }
    /// Opt this instance into bubbling handled ordinary events to its ancestors.
    /// Does not alter action recording, focus ownership or pointer capture.
    /// For per-event policy, override Component::bubble_handled_event instead.
    pub fn bubbling_handled_events(mut self, enabled: bool) -> Self {
        self.bubble_handled_events = Some(enabled);
        self
    }
    pub(super) fn bubbles(&self, event: &ContentEvent, response: &ContentResponse) -> bool {
        !self.component.blocks_event_bubbling(event)
            && (!response.consumed
                || self
                    .bubble_handled_events
                    .unwrap_or_else(|| self.component.bubble_handled_event(event)))
    }
    pub fn mount_identity(&self) -> u64 {
        self.mount
    }
    pub fn component<T: Component>(&self) -> Option<&T> {
        (self.component.as_ref() as &dyn Any).downcast_ref()
    }
    pub fn component_mut<T: Component>(&mut self) -> Option<&mut T> {
        (self.component.as_mut() as &mut dyn Any).downcast_mut()
    }
    /// Update an explicitly declared Stack distribution without replacing its
    /// mounted content. Ordinary components and undeclared Stacks return None.
    pub fn stack_distribution_mut(
        &mut self,
    ) -> Option<&mut proposal::layout::MainAxisDistribution> {
        self.stack_distribution.as_mut()
    }
    pub fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.measure_content(text, available, false)
    }
    /// Same label and finite-geometry service as measure, for decorative slots.
    pub fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.measure_content(text, available, true)
    }
    pub(super) fn measure_content(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
        intrinsic: bool,
    ) -> MeasuredChild {
        let outer_available = clean_size(available);
        let padding_layout = self.stack_padding.map(|padding| {
            crate::layout::ContainerLayout::new(
                Rect::new(0., 0., outer_available.width, outer_available.height),
                crate::layout::Insets::ZERO,
                padding,
                None,
            )
        });
        let available = padding_layout.map_or(outer_available, |layout| layout.content().size);
        let mut measured = if let Some(label) = &mut self.label {
            label.reset();
            let mut text = label::LabelRenderer::new(text, label, self.clip);
            if intrinsic {
                self.component
                    .measure_intrinsic(&mut text, clean_size(available))
            } else {
                self.component.measure(&mut text, clean_size(available))
            }
        } else if intrinsic {
            self.component
                .measure_intrinsic(text, clean_size(available))
        } else {
            self.component.measure(text, clean_size(available))
        };
        measured.size = clean_size(measured.size);
        if !intrinsic && self.stack_distribution.is_some() {
            let available = clean_size(available);
            match self
                .component::<StackContent>()
                .expect("distribution is declared by StackContent")
                .model
                .axis
            {
                Axis::Horizontal => measured.size.width = measured.size.width.max(available.width),
                Axis::Vertical => measured.size.height = measured.size.height.max(available.height),
            }
        }
        if let Some(layout) = padding_layout {
            measured.size.width += outer_available.width - available.width;
            measured.size.height += outer_available.height - available.height;
            measured.first_baseline = measured
                .first_baseline
                .map(|baseline| baseline + layout.content().min_y());
        }
        measured.first_baseline = measured.first_baseline.filter(|v| v.is_finite());
        measured
    }
    pub fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.frame = clean_rect(frame);
        self.clip = clean_rect(clip);
        if self.stack_distribution.is_some() || self.stack_padding.is_some() {
            let distribution = self.stack_distribution;
            let frame = self.stack_padding.map_or(self.frame, |padding| {
                crate::layout::ContainerLayout::new(
                    self.frame,
                    crate::layout::Insets::ZERO,
                    padding,
                    Some(self.clip),
                )
                .content()
            });
            let clip = self.clip;
            self.component_mut::<StackContent>()
                .expect("distribution is declared by StackContent")
                .arrange_with_distribution(text, frame, clip, distribution);
        } else if let Some(label) = &mut self.label {
            let mut renderer = label::LabelRenderer::new(text, label, self.clip);
            self.component.measure(&mut renderer, self.frame.size);
            self.component.arrange(&mut renderer, self.frame, self.clip);
        } else {
            self.component.arrange(text, self.frame, self.clip);
        }
    }
    /// Paint this existing mounted subtree with its arranged frames and clips.
    /// This does not create another input/focus owner or perform layout.
    pub(super) fn paint_component(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        if let Some(label) = &mut self.label {
            self.component.paint(
                scene,
                &mut label::LabelRenderer::new(text, label, self.clip).with_theme(theme),
                self.frame,
                self.clip,
                theme,
            );
        } else {
            self.component
                .paint(scene, text, self.frame, self.clip, theme);
        }
    }
    pub fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        self.paint_except(scene, text, theme, None);
    }
    pub(super) fn paint_except(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
        skip: Option<ComponentId>,
    ) {
        if skip == Some(self.id) {
            return;
        }
        if !self.paint_available() {
            return;
        }
        let curve_scope = self.push_curve_scope(scene);
        let theme = self.resolved_theme(theme);
        // Offscreen ancestors may contain visible overflowing children. Walk
        // children even when their parent frame itself is outside the viewport.
        if self.visible().is_some() {
            scene.push_clip(self.clip);
            #[cfg(feature = "inspection")]
            if self.inspection_enabled {
                let start = scene.primitives().len();
                let (_, tokens) = tekes_ui_tokens::audit::capture_resolved(|| {
                    self.paint_component(scene, text, theme);
                });
                let quads = scene.primitives()[start..]
                    .iter()
                    .filter_map(|primitive| match primitive {
                        tekes_ui_scene::Primitive::Quad(quad)
                        | tekes_ui_scene::Primitive::KeyedQuad { quad, .. } => Some(*quad),
                        _ => None,
                    })
                    .collect();
                self.inspection_paint = Some(InspectionPaint {
                    theme,
                    tokens,
                    quads,
                    includes_owned_children: self.component.paints_children(),
                });
            } else {
                self.paint_component(scene, text, theme);
            }
            #[cfg(not(feature = "inspection"))]
            self.paint_component(scene, text, theme);
            scene.pop_clip();
        }
        if self.component.paints_children() {
            for _ in 0..curve_scope {
                scene.pop_clip();
            }
            return;
        }
        let child_theme = self.child_theme(theme);
        if let Some(children) = self.component.children_mut() {
            // Exact compact actions can be adjacent with no gap. Keep their
            // existing outsetting keyboard ring above sibling chrome by drawing
            // the focused branch last in this same ordinary paint traversal.
            // Layout, hit order, semantics and overlay ordering stay unchanged.
            for focused_layer in [false, true] {
                for child in children
                    .iter_mut()
                    .filter(|child| child.has_raised_focus_decoration() == focused_layer)
                {
                    child.paint_except(scene, text, child_theme, skip);
                }
            }
        }
        let descendant_focus = self
            .component
            .children()
            .iter()
            .any(Content::has_focus_boundary);
        let panel_clip = self.clip;
        if let Some(panel) = self.component_mut::<PanelContent>() {
            scene.push_clip(panel_clip);
            panel.paint_boundary(scene, theme, descendant_focus);
            scene.pop_clip();
        }
        for _ in 0..curve_scope {
            scene.pop_clip();
        }
    }
    pub(super) fn has_focus_boundary(&self) -> bool {
        self.active()
            && (self.component.focus_boundary_visible()
                || self
                    .component
                    .children()
                    .iter()
                    .any(Content::has_focus_boundary))
    }
    pub(super) fn has_raised_focus_decoration(&self) -> bool {
        if !self.active() {
            return false;
        }
        self.component.focus_decoration_above_siblings()
            || self
                .component
                .children()
                .iter()
                .any(Content::has_raised_focus_decoration)
    }
    /// Paint overlays belonging to this same mounted subtree. A component that
    /// opts into `paints_children` calls this at its desired compositing point.
    /// An inactive owner may finish retained overlay paint, but stays absent
    /// from the host's input, focus and semantic traversal.
    pub fn paint_overlays(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        // Registered Popovers have one host-owned overlay pass in opening order.
        if !self.composition_active || self.component::<PopoverContent>().is_some() {
            return;
        }
        let owned = self.component.paints_children();
        if !(self.active() || owned && self.component.overlay_presented()) {
            return;
        }
        let theme = self.resolved_theme(theme);
        let child_theme = self.child_theme(theme);
        if !owned && let Some(children) = self.component.children_mut() {
            for child in children {
                child.paint_overlays(scene, text, child_theme);
            }
        }
        if self.component.overlay_presented() {
            let count = self.push_curve_scope(scene);
            self.component
                .paint_overlay(scene, text, self.frame, self.clip, theme);
            for _ in 0..count {
                scene.pop_clip();
            }
        }
    }
    pub(super) fn overlay_id(&self) -> Option<ComponentId> {
        if !self.active() {
            return None;
        }
        self.component
            .children()
            .iter()
            .rev()
            .find_map(Content::overlay_id)
            .or_else(|| self.component.overlay_open().then_some(self.id))
    }
    pub(super) fn needs_icons(&self) -> Option<ComponentId> {
        if !self.paint_services_available() {
            return None;
        }
        if self.component.requires_icon_services() {
            Some(self.id)
        } else {
            self.component
                .children()
                .iter()
                .find_map(Content::needs_icons)
        }
    }
    pub(super) fn icon_failure(&self) -> Option<(ComponentId, IconRenderError)> {
        if !self.paint_services_available() {
            return None;
        }
        self.component
            .icon_error()
            .map(|error| (self.id, error))
            .or_else(|| {
                self.component
                    .children()
                    .iter()
                    .find_map(Content::icon_failure)
            })
    }
    pub(super) fn needs_text(&self) -> Option<ComponentId> {
        if !self.paint_services_available() {
            return None;
        }
        if self.component.requires_text_services() {
            Some(self.id)
        } else {
            self.component
                .children()
                .iter()
                .find_map(Content::needs_text)
        }
    }
    pub(super) fn find_active(&self, id: ComponentId) -> Option<&Content> {
        self.find_in_active_ancestors(id)
            .filter(|node| node.active())
    }
    // Exit presentation may retain an inactive Dialog owner, but never bypass
    // an inactive ancestor such as a collapsed or replaced content panel.
    pub(super) fn find_in_active_ancestors(&self, id: ComponentId) -> Option<&Content> {
        if self.id == id {
            return Some(self);
        }
        if !self.active() {
            return None;
        }
        self.component
            .children()
            .iter()
            .find_map(|c| c.find_in_active_ancestors(id))
    }
    pub(super) fn find(&self, id: ComponentId) -> Option<&Content> {
        if self.id == id {
            Some(self)
        } else {
            self.component.children().iter().find_map(|c| c.find(id))
        }
    }
    pub(super) fn find_mut(&mut self, id: ComponentId) -> Option<&mut Content> {
        if self.id == id {
            return Some(self);
        }
        self.component
            .children_mut()?
            .iter_mut()
            .find_map(|c| c.find_mut(id))
    }
    pub(super) fn notify_presentation_runnable(&mut self, ancestor_active: bool) {
        let active = ancestor_active && self.active();
        self.component
            .presentation_runnable_changed(active && self.visible().is_some());
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.notify_presentation_runnable(active);
            }
        }
    }
    // Withdraw only. Successful layout is the sole publisher of true.
    pub(super) fn reconcile_presentation_runnable(&mut self, ancestor_active: bool) {
        let active = ancestor_active && self.active();
        if !active || self.visible().is_none() {
            self.component.presentation_runnable_changed(false);
        }
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.reconcile_presentation_runnable(active);
            }
        }
    }
    pub(super) fn refresh_activity(&mut self, parent_active: bool) {
        self.activity_parent = parent_active;
        let active = parent_active && self.active();
        // Notice visibility owns its runnable lifecycle. Its window authority
        // follows ancestry so a completed Dismiss can still drain once.
        let notification_active = if self.component::<ToastContent>().is_some()
            || self.component::<BannerContent>().is_some()
        {
            parent_active
        } else {
            active
        };
        if self.activity_effective != notification_active {
            self.activity_effective = notification_active;
            self.component.window_active_changed(notification_active);
        }
        // Visit retained descendants even when this owner is inactive.
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.refresh_activity(active);
            }
        }
    }
    pub(super) fn notification_activity(&self, parent: bool) -> bool {
        if self.component::<ToastContent>().is_some() || self.component::<BannerContent>().is_some()
        {
            parent
        } else {
            parent && self.active()
        }
    }
    pub(super) fn window_active_state(&mut self, parent: bool) -> (bool, bool) {
        self.activity_parent = parent;
        let effective = self.notification_activity(parent);
        let changed = self.activity_effective != effective;
        self.activity_effective = effective;
        (effective, changed)
    }
    pub(super) fn lifecycle(&mut self, event: Lifecycle) {
        self.lifecycle_with(event, &mut actions::next_mount);
    }
    pub(super) fn lifecycle_reserved(&mut self, event: Lifecycle, range: &mut actions::MountRange) {
        self.lifecycle_with(event, &mut || range.take());
    }
    pub(super) fn lifecycle_start(&mut self, event: Lifecycle, mount: u64) {
        self.curve_geometry = None;
        self.component.set_clip_region(None);
        self.component.set_curve_geometry_context(None);
        self.mount = mount;
        if event == Lifecycle::Unmounted {
            self.set_help(None);
        }
    }
    pub(super) fn lifecycle_with(&mut self, event: Lifecycle, next: &mut impl FnMut() -> u64) {
        let split_was_intact = self
            .component::<SplitViewContent>()
            .is_some_and(|s| s.validate().is_ok());
        self.lifecycle_start(event, next());
        if event == Lifecycle::Unmounted {
            self.component.event(&ContentEvent::Cancel);
            self.component.set_focus(false, false);
        }
        self.component.lifecycle(event);
        if let Some(children) = self.component.children_mut() {
            for c in children {
                c.lifecycle_with(event, next)
            }
        }
        if split_was_intact {
            split_view::refresh_local(self);
        }
    }
    pub(super) fn interactive_id(&self) -> Option<ComponentId> {
        if self.component.interactive() {
            return Some(self.id);
        }
        self.component
            .children()
            .iter()
            .find_map(Content::interactive_id)
    }
    pub(super) fn action_ancestor(&self, target: ComponentId) -> Option<ComponentId> {
        self.find(target)?;
        if self.component.semantics().role.is_action_owner() {
            return Some(self.id);
        }
        self.component
            .children()
            .iter()
            .find_map(|c| c.action_ancestor(target))
    }
    pub(super) fn ids(&self, ids: &mut BTreeSet<ComponentId>) -> Result<(), CompositionError> {
        self.component.validate()?;
        if self
            .component
            .default_surface()
            .is_some_and(|name| tekes_ui_tokens::surface_palette::canonical_surface(name).is_err())
        {
            return Err(CompositionError::InvalidSurface);
        }
        if !ids.insert(self.id) {
            return Err(CompositionError::DuplicateId(self.id));
        }
        if self.component.semantics().role.is_action_owner()
            && let Some(child) = self
                .component
                .children()
                .iter()
                .find_map(Content::interactive_id)
        {
            return Err(CompositionError::NestedInteractive {
                action: self.id,
                child,
            });
        }
        for c in self.component.children() {
            c.ids(ids)?;
        }
        Ok(())
    }
    pub(super) fn focus_ids(&self, ids: &mut Vec<ComponentId>) {
        if !self.component.participates_in_focus_traversal() || !self.active() {
            return;
        }
        if self.component.focusable()
            && !self.frame.is_empty()
            && !self.component.semantics().disabled
        {
            ids.push(self.id)
        }
        for c in self.component.children() {
            c.focus_ids(ids)
        }
    }
    // A route follows the topmost real target and its ancestors, never siblings
    // behind an unhandled target. Keep processing and propagation independent.
    pub(super) fn route(
        &mut self,
        event: &ContentEvent,
        position: Point,
        actions: &mut Option<Vec<actions::PendingAction>>,
    ) -> Option<(ComponentId, ContentResponse, bool)> {
        if !self.active() || !self.curve_contains(position) {
            return None;
        }
        let action = self.component.semantics().role.is_action_owner();
        let child = if !action && let Some(children) = self.component.children_mut() {
            children
                .iter_mut()
                .rev()
                .find_map(|c| c.route(event, position, actions))
        } else {
            None
        };
        if let Some((id, mut response, false)) = child {
            response.changed |= self.observe_nested_reading(event, id, &response);
            return Some((id, response, false));
        }
        if child.is_some()
            || self.component.interactive() && self.visible().is_some_and(|r| r.contains(position))
        {
            let response = self.deliver_actions(event, false, actions);
            return Some(self.finish_bubble(event, child, response));
        }
        None
    }
    pub(super) fn finish_bubble(
        &self,
        event: &ContentEvent,
        child: Option<(ComponentId, ContentResponse, bool)>,
        response: ContentResponse,
    ) -> (ComponentId, ContentResponse, bool) {
        let bubbles = self.bubbles(event, &response);
        let Some((id, previous, _)) = child else {
            return (self.id, response, bubbles);
        };
        // A parent cannot take capture/focus from the original handled target.
        let owner = if previous.consumed || previous.capture_pointer {
            id
        } else {
            self.id
        };
        (
            owner,
            ContentResponse {
                consumed: previous.consumed || response.consumed,
                changed: previous.changed || response.changed,
                capture_pointer: previous.capture_pointer
                    || !previous.consumed && response.capture_pointer,
                activated: previous.activated || response.activated,
                text_key: previous.text_key.or(response.text_key),
            },
            bubbles,
        )
    }
    pub(super) fn route_from(
        &mut self,
        id: ComponentId,
        event: &ContentEvent,
        actions: &mut Option<Vec<actions::PendingAction>>,
    ) -> Option<(ComponentId, ContentResponse, bool)> {
        if !self.active() {
            return None;
        }
        if self.id == id {
            let response = self.deliver_actions(event, false, actions);
            return Some((id, response, self.bubbles(event, &response)));
        }
        let child = self
            .component
            .children_mut()?
            .iter_mut()
            .find_map(|c| c.route_from(id, event, actions))?;
        if !child.2 {
            return Some(child);
        }
        let response = self.deliver_actions(event, false, actions);
        Some(self.finish_bubble(event, Some(child), response))
    }
    pub(super) fn text_wheel_path(&self, target: ComponentId) -> Option<Vec<(ComponentId, u64)>> {
        if !self.active() {
            return None;
        }
        if self.id == target {
            return Some(vec![(self.id, self.mount)]);
        }
        let mut path = self
            .component
            .children()
            .iter()
            .find_map(|child| child.text_wheel_path(target))?;
        path.insert(0, (self.id, self.mount));
        Some(path)
    }
    pub(super) fn route_text_wheel_from(
        &mut self,
        id: ComponentId,
        event: &ContentEvent,
        actions: &mut Option<Vec<actions::PendingAction>>,
    ) -> Option<(ComponentId, ContentResponse, bool)> {
        if !self.active() {
            return None;
        }
        if self.id == id {
            let field = self.component_mut::<TextFieldContent>()?;
            let response = field.captured_wheel_event(event);
            let allowed = actions.is_some() && !self.component.semantics().disabled;
            self.record_actions(&response, event.action_source(), allowed, actions);
            return Some((id, response, self.bubbles(event, &response)));
        }
        let child = self
            .component
            .children_mut()?
            .iter_mut()
            .find_map(|child| child.route_text_wheel_from(id, event, actions))?;
        if !child.2 {
            return Some(child);
        }
        let response = self.deliver_actions(event, false, actions);
        Some(self.finish_bubble(event, Some(child), response))
    }
    // Hit classification is independent of event consumption: even a disabled
    // or currently unhandled real control shields its bounds from a parent's
    // decorative focus area. No coordinates are forwarded to the editor.
    pub(super) fn padding_focus_at(&self, position: Point) -> PaddingFocusHit {
        if !self.active() {
            return PaddingFocusHit::None;
        }
        if !self.component.semantics().disabled {
            for child in self.component.children().iter().rev() {
                match child.padding_focus_at(position) {
                    PaddingFocusHit::None => {}
                    hit => return hit,
                }
            }
        }
        if !self.visible().is_some_and(|r| r.contains(position)) {
            return PaddingFocusHit::None;
        }
        if self.component.interactive() || self.component.semantics().disabled {
            return PaddingFocusHit::Blocked;
        }
        if let Some(binding) = self.padding_focus {
            return self
                .padding_text(binding.target, binding.allow_read_only)
                .filter(|target| {
                    target.mount == binding.mount
                        && target
                            .component
                            .semantic_frame(target.frame)
                            .intersection(&target.clip)
                            .is_some()
                })
                .map_or(PaddingFocusHit::Blocked, |_| {
                    PaddingFocusHit::Focus(binding.target)
                });
        }
        PaddingFocusHit::None
    }
    pub(super) fn padding_text(&self, id: ComponentId, allow_read_only: bool) -> Option<&Content> {
        if !self.active() || self.component.semantics().disabled {
            return None;
        }
        if self.id == id {
            return (self.component.focusable()
                && self.component.text_surface().is_some_and(|s| {
                    !s.input.disabled && (allow_read_only || !s.input.read_only)
                }))
            .then_some(self);
        }
        self.component
            .children()
            .iter()
            .find_map(|c| c.padding_text(id, allow_read_only))
    }
    pub(super) fn move_pointer(
        &mut self,
        event: &ContentEvent,
        position: Point,
        actions: &mut Option<Vec<actions::PendingAction>>,
    ) -> (Option<(ComponentId, ContentResponse, bool)>, bool) {
        if !self.active() {
            return (None, false);
        }
        let mut hit = None;
        let mut changed = false;
        if let Some(children) = self.component.children_mut() {
            for child in children.iter_mut() {
                let (result, redraw) = child.move_pointer(event, position, actions);
                changed |= redraw;
                if result.is_some() {
                    hit = result;
                }
            }
        }
        let inside =
            self.visible().is_some_and(|r| r.contains(position)) && self.curve_contains(position);
        if self.component.interactive() || hit.is_some_and(|(_, _, bubbles)| bubbles) {
            // Outside delivery clears old hover but cannot consume another
            // branch's event. Stopped ancestors still receive an exit update.
            let stopped = hit.is_some_and(|(_, _, bubbles)| !bubbles);
            if !stopped || !inside {
                // Preserve the original captured point. Region-aware owners
                // receive the Host-derived inside bit through deliver_actions;
                // ordinary owners keep their existing Move semantics.
                let response = self.deliver_actions(event, false, actions);
                changed |= response.changed;
                if !stopped && (hit.is_some() || inside) {
                    hit = Some(self.finish_bubble(event, hit, response));
                }
            }
        }
        (hit, changed)
    }
    pub(super) fn reveal(&mut self, id: ComponentId, frame: Rect) -> Option<Rect> {
        if self.id == id {
            return Some(frame);
        }
        if let Some(children) = self.component.children_mut() {
            for c in children {
                if let Some(frame) = c.reveal(id, frame) {
                    return Some(self.component.reveal(frame));
                }
            }
        }
        None
    }
    pub(super) fn semantic_nodes(
        &self,
        parent: Option<ComponentId>,
        focused: Option<ComponentId>,
        out: &mut Vec<SemanticNode>,
    ) {
        if !self.active() || self.semantic_suppressed || self.component.semantic_subtree_hidden() {
            return;
        }
        let hidden = self.component.semantic_hidden();
        if !hidden {
            out.push(SemanticNode {
                id: self.id,
                parent,
                frame: self.component.semantic_frame(self.frame),
                visible: self
                    .curve_bounds(self.component.semantic_frame(self.frame))
                    .ok()
                    .flatten(),
                focused: focused == Some(self.id),
                semantics: self.help_semantics(),
                parts: self.component.semantic_parts(),
            });
        }
        // Ordinary action content is decorative to assistive technology;
        // the action supplies one explicit accessible name and one focus.
        if !self.component.semantics().role.is_action_owner() {
            for c in self.component.children() {
                c.semantic_nodes(if hidden { parent } else { Some(self.id) }, focused, out)
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionError {
    MountExhausted,
    InvalidEmptyState(ComponentId),
    InvalidNotice(ComponentId),
    MissingNoticeClock(ComponentId),
    NoticeClockWentBackwards(ComponentId),
    SceneFragment(tekes_ui_scene::FragmentError),
    CurveGeometry(tekes_ui_scene::ClipError),
    CurvePointerRegionsUnsupported,
    UnsupportedCurveChild(ComponentId),
    InvalidPanelPadding,
    InvalidSurface,
    DuplicateId(ComponentId),
    MissingId(ComponentId),
    NotContainer(ComponentId),
    InvalidOrder,
    EmptyAccessibleName,
    InvalidSlotCount,
    MissingTextServices(ComponentId),
    MissingIconServices(ComponentId),
    IconResource {
        owner: ComponentId,
        reason: IconRenderError,
    },
    MissingLabelServices(ComponentId),
    UnsupportedLabelLayout(ComponentId),
    InvalidPaddingFocus {
        owner: ComponentId,
        target: ComponentId,
    },
    NotDialog(ComponentId),
    NotPopover(ComponentId),
    PopoverUnavailable(ComponentId),
    InvalidPopoverAnchor {
        owner: ComponentId,
        anchor: ComponentId,
    },
    ModalAlreadyOpen,
    NestedInteractive {
        action: ComponentId,
        child: ComponentId,
    },
    ReadingTransactionInProgress,
}
/// Owner-local identity for the exact arranged focus evidence.
#[derive(Clone, Debug)]
pub(crate) struct RequiredFocusEvidence(std::sync::Arc<()>);
impl RequiredFocusEvidence {
    pub(crate) fn new() -> Self {
        Self(std::sync::Arc::new(()))
    }
    pub(crate) fn from_owner(token: std::sync::Arc<()>) -> Self {
        Self(token)
    }
}
impl Default for RequiredFocusEvidence {
    fn default() -> Self {
        Self::new()
    }
}
impl PartialEq for RequiredFocusEvidence {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
/// Read-only finite evidence for the original required-focus region.
#[derive(Clone, Debug, PartialEq)]
pub enum RequiredFocusSpace {
    Legacy,
    NotRequired,
    Unmeasured,
    Insufficient,
    Sufficient { witness: Rect },
}
#[derive(Clone, Debug, PartialEq)]
pub enum RequiredFocusResult {
    NotRequired,
    Unmeasured,
    Insufficient,
    Clipped { witness: Rect },
    Visible { witness: Rect },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct PaddingFocusBinding {
    pub(super) target: ComponentId,
    pub(super) mount: u64,
    pub(super) allow_read_only: bool,
}

pub(super) enum PaddingFocusHit {
    None,
    Blocked,
    Focus(ComponentId),
}
/// Local native wheel facts. Unlike ContentEvent's compatibility booleans,
/// these distinguish an independent precise packet from a gesture continuation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentWheelPhase {
    #[default]
    None,
    MayBegin,
    Began,
    Changed,
    Ended,
    Cancelled,
}
#[derive(Clone, Copy, Debug)]
pub struct ContentWheelEvent {
    pub position: Point,
    /// Logical document direction, in points for precise packets and lines otherwise.
    pub delta: Point,
    pub precise: bool,
    pub phase: ContentWheelPhase,
    pub momentum_phase: ContentWheelPhase,
    pub time: f64,
}
#[derive(Debug)]
pub(super) struct TextWheelLease {
    pub(super) target: ComponentId,
    pub(super) path: Vec<(ComponentId, u64)>,
    pub(super) focus_epoch: u64,
    pub(super) awaiting_momentum: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum TextWheelDispatch {
    Start,
    Continue,
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composition contract violation: {self:?}")
    }
}
impl std::error::Error for CompositionError {}
