//! Window-level declaration/mount adapter over ONE original ContentHost.
//! No second hover, hint, focus, capture, menu, Button or AX state machine.
use super::*;
use crate::{Insets, ToolbarAction, ToolbarControl};
use std::collections::BTreeMap;
use tekes_ui_scene::ImageId;

/// Window-owned toolbar content is anchored independently of animated containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarWindowEdge {
    Leading,
    Trailing,
}
/// Registration ownership: Container follows its allocation and is removed with
/// it; Window retains a fixed edge allocation independent of panel visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarRegionScope {
    #[default]
    Container,
    Window(ToolbarWindowEdge),
}
/// A region is an allocation supplied by window/container composition, not a
/// semantic placement. Navigation/Principal/PrimaryAction stay local to it.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarRegion {
    pub id: ComponentId,
    pub available: Rect,
    pub native_safe_area: Insets,
    pub scope: ToolbarRegionScope,
}
impl ToolbarRegion {
    pub fn new(id: ComponentId, available: Rect) -> Self {
        Self {
            id,
            available,
            native_safe_area: Insets::ZERO,
            scope: ToolbarRegionScope::Container,
        }
    }
    /// `band` is the window toolbar band, not a moving panel frame. The shared
    /// host measures all original Content and reserves its space from siblings.
    pub fn window(id: ComponentId, band: Rect, edge: ToolbarWindowEdge) -> Self {
        Self {
            scope: ToolbarRegionScope::Window(edge),
            ..Self::new(id, band)
        }
    }
    fn usable(&self) -> Rect {
        let f = self.available;
        let left = self.native_safe_area.left.min(f.size.width);
        let top = self.native_safe_area.top.min(f.size.height);
        Rect::new(
            f.min_x() + left,
            f.min_y() + top,
            (f.size.width - left - self.native_safe_area.right).max(0.),
            (f.size.height - top - self.native_safe_area.bottom).max(0.),
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarGroupOverflow {
    /// All command buttons stay together, or share their original More menu.
    #[default]
    Together,
    /// A prefix stays visible; the remaining commands use that same menu.
    Split,
}
/// Declaration data for the original standard Button presenter in ToolbarControl.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarButtonDeclaration {
    pub id: u64,
    pub icon: Option<ImageId>,
    pub text: String,
    pub hint: Option<String>,
    pub enabled: bool,
}
/// Build an original command control. Bind this control ID + original command
/// IDs below; the host supplies input, overlays, help timing and window viewport.
pub fn toolbar_commands(
    id: ComponentId,
    buttons: Vec<ToolbarButtonDeclaration>,
    overflow: ToolbarGroupOverflow,
) -> Content {
    let mut control = ToolbarControl::window();
    control.set_group_together(overflow == ToolbarGroupOverflow::Together);
    control.set_actions(
        buttons
            .iter()
            .map(|b| ToolbarAction {
                id: b.id,
                title: b.text.clone(),
                enabled: b.enabled,
            })
            .collect(),
    );
    for b in buttons {
        control.set_action_icon(b.id, b.icon);
        control.set_action_help(b.id, b.hint);
    }
    Content::new(id, control)
}
/// `source` is the consumer's ORIGINAL module/session/generation binding. It is
/// captured at delivery, never inferred from placement or a navigation index.
#[derive(Clone, Debug)]
pub struct ToolbarCommandBinding<B> {
    pub target: ComponentId,
    /// Some for a ToolbarControl command; None for an original Content action.
    pub action: Option<u64>,
    pub command: u64,
    pub source: B,
}
#[derive(Debug)]
pub struct ToolbarRegistration<B> {
    pub region: ComponentId,
    pub order: i32,
    /// An original ToolbarGroup Content; arbitrary original descendants allowed.
    pub group: Content,
    pub bindings: Vec<ToolbarCommandBinding<B>>,
}
#[derive(Clone, Debug)]
pub struct ToolbarInvocation<B> {
    pub window: u64,
    pub group: ComponentId,
    pub command: u64,
    pub source: B,
    pub input: ContentActionSource,
}
/// A native adapter retains this exact target, not a naked reusable numeric ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarAccessibilityTarget {
    id: ComponentId,
    mount: u64,
    part: Option<usize>,
    revision: u64,
}
#[derive(Clone, Debug)]
pub struct ToolbarSemanticNode {
    pub target: ToolbarAccessibilityTarget,
    pub node: SemanticNode,
    pub parts: Vec<(ToolbarAccessibilityTarget, SemanticPart)>,
}
#[derive(Debug)]
struct Registration<B> {
    region: ComponentId,
    order: i32,
    bindings: Vec<ToolbarCommandBinding<B>>,
}
#[derive(Debug, Default)]
struct Regions {
    allocations: Vec<ToolbarRegion>,
    children: Vec<Content>,
}
impl Component for Regions {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        // Native exclusion constrains controls, not the window chrome beneath
        // the original traffic lights. Paint each full allocation before children.
        for region in &self.allocations {
            // Container allocations define the real ground, including their
            // split gaps. A fixed window registration must not repaint those
            // gaps merely because its positioning band spans the whole window.
            if matches!(region.scope, ToolbarRegionScope::Window(_))
                && self.allocations.iter().any(|r| {
                    r.scope == ToolbarRegionScope::Container
                        && r.available.min_y() < region.available.max_y()
                        && r.available.max_y() > region.available.min_y()
                })
            {
                continue;
            }
            crate::paint::toolbar_background(
                scene,
                region.available,
                Some(clip),
                region.available.size.height,
                theme.palette,
            );
        }
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        // Resolve persistent window allocations before any container. Window
        // controls never get reparented to an animated panel just to stay visible.
        let mut measured = Vec::new();
        for region in &self.allocations {
            if let ToolbarRegionScope::Window(edge) = region.scope {
                let usable = region.usable();
                let width = self
                    .children
                    .iter_mut()
                    .find(|c| c.id == region.id)
                    .expect("registered region")
                    .measure(text, usable.size)
                    .size
                    .width;
                measured.push((region.id, region.available, usable, edge, width));
            }
        }
        let mut fixed = Vec::new();
        for (index, &(id, band, usable, edge, natural)) in measured.iter().enumerate() {
            // Same-row edge contributions share one budget, including native
            // exclusions. In a narrow window their original overflow stays owner.
            let same_band = |b: Rect| b == band;
            let left = measured
                .iter()
                .filter(|(_, b, _, _, _)| same_band(*b))
                .map(|(_, _, u, _, _)| u.min_x())
                .fold(usable.min_x(), f32::max);
            let right = measured
                .iter()
                .filter(|(_, b, _, _, _)| same_band(*b))
                .map(|(_, _, u, _, _)| u.max_x())
                .fold(usable.max_x(), f32::min)
                .max(left);
            let total = measured
                .iter()
                .filter(|(_, b, _, _, _)| same_band(*b))
                .map(|(_, _, _, _, w)| w)
                .sum::<f32>();
            let factor = if total > 0. {
                ((right - left) / total).min(1.)
            } else {
                0.
            };
            let preceding = measured[..index]
                .iter()
                .filter(|(_, b, _, e, _)| same_band(*b) && *e == edge)
                .map(|(_, _, _, _, w)| w * factor)
                .sum::<f32>();
            let width = natural * factor;
            let x = match edge {
                ToolbarWindowEdge::Leading => left + preceding,
                ToolbarWindowEdge::Trailing => right - preceding - width,
            };
            fixed.push((
                id,
                edge,
                Rect::new(x, usable.min_y(), width, usable.size.height),
            ));
        }
        for child in &mut self.children {
            let region = self
                .allocations
                .iter()
                .find(|r| r.id == child.id)
                .expect("registered region");
            let usable = if let Some((_, _, frame)) =
                fixed.iter().find(|(id, _, _)| *id == child.id)
            {
                *frame
            } else {
                let base = region.usable();
                let mut left = base.min_x();
                let mut right = base.max_x();
                for (_, edge, frame) in &fixed {
                    if frame.size.width <= 0.
                        || frame.min_y() >= base.max_y()
                        || frame.max_y() <= base.min_y()
                    {
                        continue;
                    }
                    match edge {
                        ToolbarWindowEdge::Leading => left = left.max(frame.max_x()).min(right),
                        ToolbarWindowEdge::Trailing => right = right.min(frame.min_x()).max(left),
                    }
                }
                Rect::new(left, base.min_y(), (right - left).max(0.), base.size.height)
            };
            let row = crate::toolbar::metrics(text, proposal::TextRole::UiBody).row;
            let frame = Rect::new(
                usable.min_x(),
                usable.center().y - row * 0.5,
                usable.size.width,
                row,
            );
            let clip = usable.intersection(&clip).unwrap_or_default();
            child.arrange(text, frame, clip);
        }
    }
}
/// Mount all toolbar regions/groups into one original tree. Call event/render
/// once per window event/frame, regardless of how many modules declare groups.
#[derive(Debug)]
pub struct ToolbarHost<B> {
    host: ContentHost,
    root: ComponentId,
    window: u64,
    viewport: Rect,
    registrations: BTreeMap<ComponentId, Registration<B>>,
    invocations: Vec<ToolbarInvocation<B>>,
    ready: bool,
    active: bool,
    revision: u64,
    typography: Option<(f32, f32)>,
}
impl<B: Clone + Debug> ToolbarHost<B> {
    pub fn new(root: ComponentId, window: u64, viewport: Rect) -> Result<Self, CompositionError> {
        let mut host = ContentHost::new(Content::new(root, Regions::default()))?;
        host.set_action_events(true);
        host.set_help_overlay_deferred(true);
        Ok(Self {
            host,
            root,
            window,
            viewport,
            registrations: BTreeMap::new(),
            invocations: Vec::new(),
            ready: false,
            active: true,
            revision: 0,
            typography: None,
        })
    }
    fn invalidate(&mut self) {
        self.ready = false;
        self.revision = self
            .revision
            .checked_add(1)
            .expect("toolbar revision exhausted");
    }
    /// Context updates invalidate old geometry before another input/AX callback.
    /// Native controls are not moved or resized by this mount adapter.
    pub fn set_regions(
        &mut self,
        window: u64,
        viewport: Rect,
        regions: Vec<ToolbarRegion>,
    ) -> Result<(), CompositionError> {
        let mut ids = BTreeSet::new();
        for region in &regions {
            if region.id == self.root || !ids.insert(region.id) {
                return Err(CompositionError::DuplicateId(region.id));
            }
            if self.registrations.contains_key(&region.id) {
                return Err(CompositionError::DuplicateId(region.id));
            }
            let f = region.available;
            if [
                f.min_x(),
                f.min_y(),
                f.size.width,
                f.size.height,
                region.native_safe_area.top,
                region.native_safe_area.right,
                region.native_safe_area.bottom,
                region.native_safe_area.left,
            ]
            .iter()
            .any(|v| !v.is_finite())
                || f.size.width < 0.
                || f.size.height < 0.
                || [
                    region.native_safe_area.top,
                    region.native_safe_area.right,
                    region.native_safe_area.bottom,
                    region.native_safe_area.left,
                ]
                .iter()
                .any(|v| *v < 0.)
            {
                return Err(CompositionError::InvalidSlotCount);
            }
        }
        let current = &self
            .host
            .get(self.root)
            .unwrap()
            .component::<Regions>()
            .unwrap()
            .allocations;
        if self.window == window && self.viewport == viewport && *current == regions {
            return Ok(());
        }
        let old: Vec<_> = current.iter().map(|r| r.id).collect();
        let changed: Vec<_> = current
            .iter()
            .filter(|old| !regions.contains(old))
            .map(|r| r.id)
            .collect();
        // Validate collisions against original descendants before any mutation.
        for r in &regions {
            if !old.contains(&r.id) && self.host.get(r.id).is_some() {
                return Err(CompositionError::DuplicateId(r.id));
            }
        }
        if self.window != window || self.viewport != viewport {
            self.host.cancel();
        } else {
            // Moving/hiding one container must not blur a fixed window editor
            // or recreate a persistent control in a different region.
            for id in changed {
                self.host.cancel_subtree(id);
            }
        }
        self.invocations.clear();
        self.invalidate();
        for id in old {
            if !ids.contains(&id) {
                self.host.remove(id)?;
                self.registrations.retain(|_, r| r.region != id);
            }
        }
        for region in &regions {
            if self.host.get(region.id).is_none() {
                self.host
                    .insert(self.root, Content::new(region.id, ToolbarContent::hosted()))?;
            }
        }
        self.host
            .reorder(self.root, &regions.iter().map(|r| r.id).collect::<Vec<_>>())?;
        self.host
            .get_mut(self.root)
            .unwrap()
            .component_mut::<Regions>()
            .unwrap()
            .allocations = regions;
        self.window = window;
        self.viewport = viewport;
        Ok(())
    }
    /// New or replacement declaration. Replacing one group unmounts only that
    /// original subtree; unrelated groups retain their controls and mount IDs.
    pub fn register(
        &mut self,
        declaration: ToolbarRegistration<B>,
    ) -> Result<(), CompositionError> {
        let ToolbarRegistration {
            region,
            order,
            group,
            bindings,
        } = declaration;
        if self
            .host
            .get(region)
            .and_then(|n| n.component::<ToolbarContent>())
            .is_none()
        {
            return Err(CompositionError::MissingId(region));
        }
        if group.component::<ToolbarGroup>().is_none() {
            return Err(CompositionError::NotContainer(group.id));
        }
        let id = group.id;
        let old = self.registrations.get(&id).and_then(|_| self.host.get(id));
        let mut ids = BTreeSet::new();
        group.ids(&mut ids)?;
        for child in ids {
            if self.host.get(child).is_some() && old.is_none_or(|old| old.find(child).is_none()) {
                return Err(CompositionError::DuplicateId(child));
            }
        }
        let mut keys = BTreeSet::new();
        for b in &bindings {
            if group.find(b.target).is_none() {
                return Err(CompositionError::MissingId(b.target));
            }
            if !keys.insert((b.target, b.action)) {
                return Err(CompositionError::DuplicateId(b.target));
            }
        }
        let replacing = old.is_some();
        let old_region = self.registrations.get(&id).map(|r| r.region);
        self.host.cancel_subtree(region);
        if let Some(old_region) = old_region.filter(|r| *r != region) {
            self.host.cancel_subtree(old_region);
        }
        if replacing {
            self.host.remove(id)?;
        }
        self.host.insert(region, group)?;
        self.registrations.insert(
            id,
            Registration {
                region,
                order,
                bindings,
            },
        );
        self.order_region(region)?;
        self.invocations.retain(|i| i.group != id);
        self.invalidate();
        Ok(())
    }
    fn order_region(&mut self, region: ComponentId) -> Result<(), CompositionError> {
        let mut order: Vec<_> = self
            .registrations
            .iter()
            .filter(|(_, r)| r.region == region)
            .map(|(id, r)| (r.order, *id))
            .collect();
        order.sort();
        self.host.reorder(
            region,
            &order.into_iter().map(|(_, id)| id).collect::<Vec<_>>(),
        )
    }
    /// Update values on an original descendant, retaining its mounted identity.
    /// Structural child changes use register/remove, so lifecycle is not bypassed.
    pub fn update_component<C: Component>(
        &mut self,
        group: ComponentId,
        target: ComponentId,
        update: impl FnOnce(&mut C),
    ) -> Result<(), CompositionError> {
        if !self.registrations.contains_key(&group) {
            return Err(CompositionError::MissingId(group));
        }
        if self
            .host
            .get(group)
            .and_then(|n| n.find(target))
            .and_then(|n| n.component::<C>())
            .is_none()
        {
            return Err(CompositionError::MissingId(target));
        }
        // Retire stale hits if this value changes width; unrelated original
        // editors retain focus and native text sessions during button refreshes.
        self.host.cancel_pointer_capture();
        self.host.cancel_subtree(target);
        update(
            self.host
                .get_mut(target)
                .unwrap()
                .component_mut::<C>()
                .unwrap(),
        );
        self.invocations.retain(|i| i.group != group);
        self.invalidate();
        Ok(())
    }
    pub fn set_visible(
        &mut self,
        group: ComponentId,
        visible: bool,
    ) -> Result<(), CompositionError> {
        let Some(record) = self.registrations.get(&group) else {
            return Err(CompositionError::MissingId(group));
        };
        if self
            .host
            .get(group)
            .unwrap()
            .component::<ToolbarGroup>()
            .unwrap()
            .visible
            == visible
        {
            return Ok(());
        }
        self.host.cancel_subtree(record.region);
        self.host
            .get_mut(group)
            .unwrap()
            .component_mut::<ToolbarGroup>()
            .unwrap()
            .visible = visible;
        self.invocations.retain(|i| i.group != group);
        self.invalidate();
        Ok(())
    }
    pub fn move_group(
        &mut self,
        group: ComponentId,
        region: ComponentId,
        placement: ToolbarPlacement,
        order: i32,
    ) -> Result<(), CompositionError> {
        if !self.registrations.contains_key(&group) {
            return Err(CompositionError::MissingId(group));
        }
        if self
            .host
            .get(region)
            .and_then(|c| c.component::<ToolbarContent>())
            .is_none()
        {
            return Err(CompositionError::MissingId(region));
        }
        self.host.cancel_subtree(self.registrations[&group].region);
        self.host.cancel_subtree(region);
        self.host.reparent(group, region)?;
        self.host
            .get_mut(group)
            .unwrap()
            .component_mut::<ToolbarGroup>()
            .unwrap()
            .placement = placement;
        let record = self.registrations.get_mut(&group).unwrap();
        let old = record.region;
        record.region = region;
        record.order = order;
        self.order_region(old)?;
        if old != region {
            self.order_region(region)?;
        }
        self.invocations.retain(|i| i.group != group);
        self.invalidate();
        Ok(())
    }
    pub fn remove(&mut self, group: ComponentId) -> Result<Content, CompositionError> {
        if !self.registrations.contains_key(&group) {
            return Err(CompositionError::MissingId(group));
        }
        self.host.cancel_subtree(self.registrations[&group].region);
        let node = self.host.remove(group)?;
        self.registrations.remove(&group);
        self.invocations.retain(|i| i.group != group);
        self.invalidate();
        Ok(node)
    }
    pub fn set_window_active(&mut self, active: bool) {
        if self.active == active {
            return;
        }
        self.active = active;
        self.host.set_window_active(active);
        self.invalidate();
        if !active {
            self.invocations.clear();
        }
    }
    pub fn needs_render(&self) -> bool {
        !self.ready || self.host.needs_render()
    }
    /// One call paints every original group AND its menus and help overlays.
    /// The viewport is the window clip, so hints are not cut to one group row.
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        let typography = (
            text.resolved_font_size(proposal::TextRole::UiBody),
            text.backing_scale(),
        );
        if self.typography.is_some_and(|old| old != typography) {
            self.host.cancel();
            self.invalidate();
        }
        self.typography = Some(typography);
        fn viewports(node: &mut Content, viewport: Rect) {
            if let Some(c) = node.component_mut::<ToolbarControl>() {
                c.set_viewport(viewport);
            }
            if let Some(children) = node.component.children_mut() {
                for child in children {
                    viewports(child, viewport);
                }
            }
        }
        viewports(&mut self.host.root, self.viewport);
        self.ready = false;
        self.host
            .render(scene, text, self.viewport, self.viewport, palette)?;
        self.host
            .paint_help_overlay(scene, text, self.viewport, palette);
        self.ready = true;
        Ok(())
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if !self.ready || !self.active {
            return ContentResponse::consumed(false);
        }
        let result = self.host.event(event);
        self.collect();
        result
    }
    /// Text/IME-capable descendants keep the existing ContentHost key ordering.
    pub fn event_with_text(
        &mut self,
        event: &ContentEvent,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<ContentResponse, CompositionError> {
        if !self.ready || !self.active {
            return Ok(ContentResponse::consumed(false));
        }
        let response = self.host.event_with_text(event, text)?;
        self.collect();
        Ok(response)
    }
    /// Read-only window adaptation queries (focused text/native IME geometry,
    /// semantic part hierarchy). Input and overlays still go through this host.
    pub fn content_host(&self) -> &ContentHost {
        &self.host
    }
    pub fn event_bubbled(&self) -> bool {
        self.ready && self.active && self.host.event_bubbled()
    }
    fn collect(&mut self) {
        for record in self.host.drain_action_records() {
            if record.action.kind != ContentActionKind::Activate {
                continue;
            }
            let target = record.action.target;
            let action = match record.payload {
                Some(ContentActionPayload::ToolbarCommand(command)) => Some(command),
                _ => None,
            };
            // Consume the legacy delivery latch too; routing uses the owned
            // journal payload, not mutable state after scope/focus cleanup.
            if let Some(control) = self
                .host
                .get_mut(target)
                .and_then(|n| n.component_mut::<ToolbarControl>())
            {
                control.take_action();
            }
            for (group, r) in &self.registrations {
                if let Some(binding) = r
                    .bindings
                    .iter()
                    .find(|b| b.target == target && b.action == action)
                {
                    self.invocations.push(ToolbarInvocation {
                        window: self.window,
                        group: *group,
                        command: binding.command,
                        source: binding.source.clone(),
                        input: record.action.source,
                    });
                    break;
                }
            }
        }
    }
    pub fn drain_commands(&mut self) -> Vec<ToolbarInvocation<B>> {
        std::mem::take(&mut self.invocations)
    }
    pub fn semantics(&self) -> Vec<ToolbarSemanticNode> {
        if !self.ready || !self.active {
            return Vec::new();
        }
        self.host
            .semantics()
            .into_iter()
            .map(|node| ToolbarSemanticNode {
                target: ToolbarAccessibilityTarget {
                    id: node.id,
                    mount: self.host.semantic_mount(node.id).unwrap(),
                    part: None,
                    revision: self.revision,
                },
                parts: node
                    .parts
                    .iter()
                    .cloned()
                    .map(|part| {
                        (
                            ToolbarAccessibilityTarget {
                                id: node.id,
                                mount: self.host.semantic_mount(node.id).unwrap(),
                                part: Some(part.index),
                                revision: self.revision,
                            },
                            part,
                        )
                    })
                    .collect(),
                node,
            })
            .collect()
    }
    pub fn accessibility_action(
        &mut self,
        target: ToolbarAccessibilityTarget,
        action: AccessibilityAction,
    ) -> ContentResponse {
        if !self.ready
            || !self.active
            || target.revision != self.revision
            || self.host.semantic_mount(target.id) != Some(target.mount)
        {
            return ContentResponse::default();
        }
        let result = self
            .host
            .accessibility_action(target.id, target.part, action);
        self.collect();
        result
    }
    /// Restore a window adapter's already authenticated original owner without
    /// synthesizing input. Callers re-resolve the current opaque owner target;
    /// stale window/revision/mounts and semantic parts are rejected unchanged.
    /// The original Host still decides focus/reveal/IME admission. `true` means
    /// accepted, including an unchanged focused owner, never a refused transfer.
    pub fn restore_focus(&mut self, target: ToolbarAccessibilityTarget, visible: bool) -> bool {
        if !self.ready
            || !self.active
            || target.part.is_some()
            || target.revision != self.revision
            || self.host.semantic_mount(target.id) != Some(target.mount)
            || !self.registrations.keys().any(|group| {
                self.host
                    .get(*group)
                    .is_some_and(|node| node.find(target.id).is_some())
            })
        {
            return false;
        }
        matches!(
            self.host.admit_ordinary_focus(
                Some(target.id),
                visible,
                None,
                action_preparation::FocusOrigin::Direct
            ),
            action_preparation::FocusAdmission::Accepted { .. }
        )
    }
    pub fn help_tooltip(&self) -> Option<(ComponentId, &str, crate::feedback_view::OverlayLayout)> {
        if self.ready && self.active {
            self.host.help_tooltip()
        } else {
            None
        }
    }
}
