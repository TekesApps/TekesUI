use super::content_node::{
    PaddingFocusBinding, PaddingFocusHit, TextWheelDispatch, TextWheelLease,
};
use super::*;
use crate::{ButtonTextRenderer, ControlKey};
use std::collections::BTreeSet;
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Palette;

#[derive(Debug)]
pub struct ContentHost {
    pub(super) root: Content,
    pub(super) curve_shapes: Option<tekes_ui_scene::ShapeTable>,
    /// Parent-authorized external geometry constrains this original Host's
    /// input/AX/reveal tree. Its AA clip is deferred to the parent composite.
    pub(super) external_curve_chain: Option<(u64, std::sync::Arc<[tekes_ui_scene::ClipShape]>)>,
    pub(super) reveal_clock: content_reveal_plan::RevealClock,
    pub(super) focused: Option<ComponentId>,
    pub(super) focus_wrap: bool,
    pub(super) overlay_scopes: Vec<overlay_scope::ScopedOverlay>,
    pub(super) popover_order: Vec<PopoverSession>,
    pub(super) captured: Option<ComponentId>,
    pub(super) captured_button: PointerButton,
    pub(super) text_wheel: Option<TextWheelLease>,
    pub(super) text_wheel_retired: bool,
    pub(super) text_wheel_dispatch: Option<TextWheelDispatch>,
    pub(super) captured_overlay: bool,
    // Capture source, distinct from merely being inside an overlay scope.
    // Meaningful only while captured is Some; set at every acquisition.
    pub(super) captured_overlay_handler: bool,
    pub(super) retired_combo_release: bool,
    pub(super) captured_mount: Option<u64>,
    pub(super) capture_epoch: u64,
    pub(super) dirty: bool,
    pub(super) inherited_surface: &'static str,
    pub(super) inherited_surface_declared: bool,
    pub(super) modal: Option<(ComponentId, Option<(ComponentId, u64)>)>,
    pub(super) modal_focus_pending: bool,
    // Paint-only owner after close. It is never used as the event/focus scope.
    pub(super) presented_dialog: Option<ComponentId>,
    pub(super) text_keys: Vec<key::KeyFrame>,
    pub(super) deferred_text_key: Option<key::KeyFrame>,
    pub(super) completed_text_key: Option<(ContentResponse, bool)>,
    pub(super) last_text_key: u64,
    pub(super) focus_epoch: u64,
    pub(super) window_active: bool,
    pub(super) animation_frame_requested: bool,
    pub(super) actions: Option<Vec<actions::PendingAction>>,
    pub(super) reading_transaction: bool,
    pub(super) last_layout: Option<(Rect, Rect)>,
    pub(super) last_render_stamp: Option<(u64, u64)>,
    pub(super) last_palette: Option<Palette>,
    pub(super) last_surface: Option<(&'static str, bool)>,
    pub(super) item_reveal: Option<content_reveal::ItemReveal>,
    pub(super) list_navigation_reveal: Option<selectable_list::ListNavigationReveal>,
    pub(super) reading_scroll_source: Option<(ComponentId, u64)>,
    pub(super) help: help::HelpState,
    pub(super) help_overlay_deferred: bool,
    pub(super) event_bubbled: Option<bool>,
    pub(super) pointer_focus_requested: bool,
}
impl ContentHost {
    pub fn new(mut root: Content) -> Result<Self, CompositionError> {
        root.validate_curve_tree(false)?;
        root.ids(&mut BTreeSet::new())?;
        root.lifecycle(Lifecycle::Mounted);
        Ok(Self::from_mounted(root))
    }
    pub(super) fn from_mounted(root: Content) -> Self {
        Self {
            root,
            curve_shapes: None,
            external_curve_chain: None,
            reveal_clock: Default::default(),
            focused: None,
            focus_wrap: true,
            overlay_scopes: Vec::new(),
            popover_order: Vec::new(),
            captured: None,
            captured_button: PointerButton::Primary,
            text_wheel: None,
            text_wheel_retired: false,
            text_wheel_dispatch: None,
            captured_overlay: false,
            captured_overlay_handler: false,
            retired_combo_release: false,
            captured_mount: None,
            capture_epoch: 0,
            dirty: true,
            inherited_surface: "canvas",
            inherited_surface_declared: false,
            modal: None,
            modal_focus_pending: false,
            presented_dialog: None,
            text_keys: Vec::new(),
            deferred_text_key: None,
            completed_text_key: None,
            last_text_key: 0,
            focus_epoch: 0,
            window_active: true,
            animation_frame_requested: false,
            actions: None,
            reading_transaction: false,
            last_layout: None,
            last_render_stamp: None,
            last_palette: None,
            last_surface: None,
            item_reveal: None,
            list_navigation_reveal: None,
            reading_scroll_source: None,
            help: Default::default(),
            help_overlay_deferred: false,
            event_bubbled: None,
            pointer_focus_requested: false,
        }
    }
    pub fn set_inherited_surface(
        &mut self,
        name: &str,
    ) -> Result<bool, tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        let changed = !self.inherited_surface_declared || self.inherited_surface != surface;
        self.inherited_surface = surface;
        self.inherited_surface_declared = true;
        self.dirty |= changed;
        Ok(changed)
    }
    pub fn inherited_surface(&self) -> &'static str {
        self.inherited_surface
    }
    /// Install the validated external ancestor chain received for the current
    /// remote View grant. These shapes constrain original Host geometry while
    /// their antialias coverage remains deferred to the parent's final image
    /// composite, preventing the child from baking the same edge twice.
    pub fn set_external_curve_chain(
        &mut self,
        parent_generation: Option<u64>,
        shapes: std::sync::Arc<[tekes_ui_scene::ClipShape]>,
    ) -> Result<(), CompositionError> {
        let next = if let Some(generation) = parent_generation {
            if generation == 0 || shapes.is_empty() || shapes.len() > 256 {
                return Err(CompositionError::CurveGeometry(
                    tekes_ui_scene::ClipError::InvalidInput,
                ));
            }
            Some((generation, shapes))
        } else {
            if !shapes.is_empty() {
                return Err(CompositionError::CurveGeometry(
                    tekes_ui_scene::ClipError::InvalidInput,
                ));
            }
            None
        };
        if self
            .external_curve_chain
            .as_ref()
            .map(|(generation, current)| (*generation, current.as_ref()))
            == next
                .as_ref()
                .map(|(generation, current)| (*generation, current.as_ref()))
        {
            return Ok(());
        }
        self.reveal_clock.invalidate_geometry();
        self.root.clear_curve_geometry_context();
        self.external_curve_chain = next;
        self.curve_shapes = None;
        self.dirty = true;
        Ok(())
    }
    /// Current child geometry generation after a successful original Host
    /// render under the installed external chain.
    pub fn external_curve_ack(&self) -> Option<(u64, u64)> {
        let (parent_generation, _) = self.external_curve_chain.as_ref()?;
        let current = self.reveal_clock.stamp().ok()?;
        if self.reveal_clock.pending
            || self.last_render_stamp != Some(current)
            || self.last_layout.is_none()
        {
            return None;
        }
        let snapshot = self.geometry_snapshot()?;
        snapshot
            .is_current()
            .then_some((*parent_generation, snapshot.generation))
    }
    pub fn get(&self, id: ComponentId) -> Option<&Content> {
        self.root.find(id)
    }
    pub fn get_mut(&mut self, id: ComponentId) -> Option<&mut Content> {
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        self.root.find_mut(id)
    }
    /// Whether visual state, layout reveal, help tooltip, or dialog/popover
    /// animations need a render (excluding input-lease retirement on no-op events).
    pub fn has_visual_changes(&self) -> bool {
        self.dirty
            || self.reveal_clock.pending
            || self.help.needs_frame()
            || self.popover_animation_pending()
            || self
                .presented_dialog
                .and_then(|id| self.root.find(id))
                .and_then(|node| node.component::<DialogContent>())
                .is_some_and(DialogContent::needs_animation)
    }
    /// Mutation and event invalidation; animation callbacks remain on the
    /// existing text adapter. A successful render acknowledges this flag.
    pub fn needs_render(&self) -> bool {
        self.has_visual_changes()
            // Even refused input retires the frozen geometry's input lease.
            // The next frame must acknowledge that generation before repaint.
            || self
                .last_render_stamp
                .is_some_and(|last| self.reveal_clock.stamp().ok() != Some(last))
    }
    pub fn focus_targets(&self) -> Vec<(ComponentId, Rect)> {
        let mut ids = Vec::new();
        self.scope().focus_ids(&mut ids);
        ids.into_iter()
            .filter_map(|id| self.root.find(id).map(|c| (id, c.frame)))
            .collect()
    }
    /// Visible original pointer/hover regions, including disabled controls.
    /// No semantic-frame approximation or second control layout is computed.
    pub fn pointer_regions(&self) -> Vec<Rect> {
        self.try_pointer_regions().unwrap_or_default()
    }
    /// Curved domains cannot be exported as exact rectangle pointer regions.
    pub fn try_pointer_regions(&self) -> Result<Vec<Rect>, CompositionError> {
        if self.requires_curve_geometry() {
            return Err(CompositionError::CurvePointerRegionsUnsupported);
        }
        fn collect(node: &Content, regions: &mut Vec<Rect>) {
            if !node.active() {
                return;
            }
            regions.extend(node.component.pointer_regions(node.frame, node.clip));
            for child in node.component.children() {
                collect(child, regions);
            }
        }
        let mut regions = Vec::new();
        collect(self.scope(), &mut regions);
        Ok(regions)
    }
    /// Visible text hit regions for the platform's native text cursor. Uses
    /// each original TextSurface viewport, including ancestor and scroll clips;
    /// it does not focus a field or modify its editing state.
    pub fn text_cursor_regions(&self) -> Vec<Rect> {
        if !self.window_active || self.requires_curve_geometry() {
            return Vec::new();
        }
        fn collect(node: &Content, regions: &mut Vec<Rect>) {
            if !node.active() {
                return;
            }
            // Compound inputs retain their own semantic role while exposing
            // the same original text viewport (for example editable ComboBox).
            if let Some(surface) = node.component.text_surface()
                && !surface.input.disabled
                && let Some(region) = surface.visible_content()
                && !region.is_empty()
            {
                regions.push(region);
            }
            for child in node.component.children() {
                collect(child, regions);
            }
        }
        let mut regions = Vec::new();
        collect(self.scope(), &mut regions);
        regions
    }
    /// Current actual-layout focus result of the retained original owner.
    pub fn required_focus_result(&self, id: ComponentId) -> Option<RequiredFocusResult> {
        self.scope()
            .find_active(id)?
            .component
            .required_focus_result()
    }
    pub fn focused(&self) -> Option<ComponentId> {
        self.focused
    }
    /// Make a passive container's decorative padding focus an existing editable
    /// descendant. Call after mounting (layout may follow); `None` removes the
    /// declaration. The target's current mount is retained, so replacement or
    /// removal/reinsertion requires an explicit new declaration.
    ///
    /// Real interactive descendants always win, including disabled controls.
    /// A primary down in remaining visible padding changes only focus: it does
    /// not synthesize a pointer/caret event, acquire capture, or create an AX node.
    /// An already-focused editor is left untouched, including preedit/selection.
    /// Hidden, disabled, read-only, foreign or stale targets cannot be redirected.
    pub fn set_padding_focus(
        &mut self,
        owner: ComponentId,
        target: Option<ComponentId>,
    ) -> Result<(), CompositionError> {
        self.bind_padding_focus(owner, target, false)
    }
    /// Opt in to decorative padding focus for an original selectable text
    /// descendant, including read-only text. This uses the same mount, subtree,
    /// active/disabled, clipping, real-child, capture and IME/focus barriers as
    /// `set_padding_focus`; that older API remains editable-only.
    ///
    /// Padding changes only focus. It does not forward pointer coordinates,
    /// move the caret, select a whole row, acquire capture or make text editable.
    /// Direct text input/selection stays with the original text component.
    /// Repeated same-owner padding preserves selection, undo and marked text.
    /// `padding_focus_target` exposes this declaration's same-turn host intent.
    pub fn set_padding_text_focus(
        &mut self,
        owner: ComponentId,
        target: Option<ComponentId>,
    ) -> Result<(), CompositionError> {
        self.bind_padding_focus(owner, target, true)
    }
    pub(super) fn bind_padding_focus(
        &mut self,
        owner: ComponentId,
        target: Option<ComponentId>,
        allow_read_only: bool,
    ) -> Result<(), CompositionError> {
        let node = self
            .scope()
            .find_active(owner)
            .ok_or(CompositionError::MissingId(owner))?;
        let binding = if let Some(id) = target {
            let invalid = CompositionError::InvalidPaddingFocus { owner, target: id };
            if id == owner || node.component.interactive() {
                return Err(invalid);
            }
            let editor = node.padding_text(id, allow_read_only).ok_or(invalid)?;
            Some(PaddingFocusBinding {
                target: id,
                mount: editor.mount,
                allow_read_only,
            })
        } else {
            None
        };
        self.root.find_mut(owner).unwrap().padding_focus = binding;
        Ok(())
    }
    /// Read the original text owner selected by registered decorative
    /// padding for this primary pointer-down. This is not a general hit test:
    /// real controls (including unavailable ones), open overlays, capture and
    /// content outside the current modal scope do not report a padding target.
    /// Use the current rendered layout, as for `event`.
    ///
    /// This query does not reconcile, focus, reveal, deliver an event, acquire
    /// capture or advance an editing/key epoch. Use it only as a same-turn host
    /// policy input, for example to preserve the currently composing editor.
    /// The returned ID is not a reservation: `event` checks the current route
    /// again. Pending stale input ownership may conservatively return `None`
    /// until ordinary event/layout reconciliation has run.
    pub fn padding_focus_target(&self, event: &ContentEvent) -> Option<ComponentId> {
        if !event.is_primary_pointer()
            || self.captured.is_some()
            || self.scope().overlay_id().is_some()
            || self.root.ids(&mut BTreeSet::new()).is_err()
        {
            return None;
        }
        let routing = event.routing_event();
        let ContentEvent::PointerDown(position) = routing.as_ref() else {
            return None;
        };
        match self.scope().padding_focus_at(*position) {
            PaddingFocusHit::Focus(id) => Some(id),
            _ => None,
        }
    }
    /// Capture an eligible focused object's current mount identity. This is a
    /// process-local return intent, not a persistent or user-assigned key.
    pub fn capture_focus(&self) -> Option<(ComponentId, u64)> {
        self.focused.and_then(|id| {
            self.scope()
                .find_active(id)
                .filter(|caller| {
                    caller.component.focusable() && !caller.component.semantics().disabled
                })
                .map(|caller| (id, caller.mount))
        })
    }
    /// Explicitly finish a temporary presentation's return-focus intent.
    /// Reorder preserves identity; replacement/remount/unavailability clears
    /// focus instead. Returns whether the focus owner changed, like `focus`.
    pub fn restore_focus(&mut self, captured: Option<(ComponentId, u64)>) -> bool {
        let target = captured.and_then(|(id, mount)| {
            self.scope()
                .find_active(id)
                .filter(|caller| {
                    caller.mount == mount
                        && caller.component.focusable()
                        && !caller.component.semantics().disabled
                })
                .map(|_| id)
        });
        self.focus(target, true)
    }
    fn check_text_services(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<(), CompositionError> {
        let paint_icons = self
            .presented_dialog
            .and_then(|id| self.root.find(id))
            .and_then(|node| {
                node.component
                    .children()
                    .iter()
                    .find_map(Content::needs_icons)
            });
        if let Some(id) = self.root.needs_icons().or(paint_icons)
            && text.icon_renderer().is_none()
        {
            self.cancel_pointer_capture();
            return Err(CompositionError::MissingIconServices(id));
        }
        let paint_text = self
            .presented_dialog
            .and_then(|id| self.root.find(id))
            .and_then(|node| {
                node.component
                    .children()
                    .iter()
                    .find_map(Content::needs_text)
            });
        if let Some(id) = self.root.needs_text().or(paint_text)
            && text.text_surface_host().is_none()
        {
            self.cancel();
            if let Some(dialog) = self
                .presented_dialog
                .take()
                .and_then(|id| self.root.find_mut(id))
                .and_then(|node| node.component_mut::<DialogContent>())
            {
                dialog.clear_presentation();
            }
            return Err(CompositionError::MissingTextServices(id));
        }
        Ok(())
    }
    /// Original frame and clip passed to the last successful render. This is
    /// remembered application allocation, not current geometry authorization;
    /// reading it does not refresh layout or validate pending mutations.
    pub fn last_layout_allocation(&self) -> Option<(Rect, Rect)> {
        self.last_layout
    }
    /// Supports activation changes during measure/arrange. Required text
    /// services are checked at entry and after each layout stage, before paint
    /// and modal focus selection. On failure, do not publish native text/AX
    /// layout from that frame. Application layout mutations are retained;
    /// transient input ownership is cancelled as for entry preflight failure.
    /// A valid backend can complete the next frame without replacing editors.
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        self.render_into(scene, None, text, frame, clip, palette)
    }
    /// Arrange once and paint the original body and overlays into separate layers.
    pub fn render_layers(
        &mut self,
        scene: &mut Scene,
        overlays: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        self.render_into(scene, Some(overlays), text, frame, clip, palette)
    }
    pub(super) fn render_into(
        &mut self,
        scene: &mut Scene,
        overlays: Option<&mut Scene>,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        let mut requested = false;
        let active = self.window_active;
        let result = self.render_frame(
            scene,
            overlays,
            &mut FrameTextAdapter(text, Some(&mut requested), active),
            frame,
            clip,
            palette,
        );
        self.animation_frame_requested = result.is_ok() && requested;
        self.last_layout = result.is_ok().then_some((frame, clip));
        self.last_render_stamp = result
            .is_ok()
            .then(|| self.reveal_clock.stamp().ok())
            .flatten();
        self.last_palette = result.is_ok().then_some(palette);
        self.last_surface = result
            .is_ok()
            .then_some((self.inherited_surface, self.inherited_surface_declared));
        if let Err(error) = &result {
            notice::clear_samples(&mut self.root);
            self.fail_item_reveal(ContentRevealError::Layout(*error));
        }
        result
    }
    /// Repaint the exact last-arranged tree for a display-only refresh. This
    /// reuses its frozen curve geometry and painter without layout or a new
    /// geometry generation; stale geometry/input/theme is rejected.
    pub fn repaint_current(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        if let Some(error) = scene.composition_error() {
            return Err(CompositionError::SceneFragment(error));
        }
        if self.reading_transaction {
            return Err(CompositionError::ReadingTransactionInProgress);
        }
        let current_stamp = self.reveal_clock.stamp().map_err(|_| {
            CompositionError::CurveGeometry(tekes_ui_scene::ClipError::StaleShapeKey)
        })?;
        let snapshot_current = self
            .geometry_snapshot()
            .is_some_and(|snapshot| snapshot.is_current());
        if self.reveal_clock.pending
            || self.last_layout != Some((frame, clip))
            || self.last_render_stamp != Some(current_stamp)
            || self.last_palette != Some(palette)
            || self.last_surface != Some((self.inherited_surface, self.inherited_surface_declared))
            || !snapshot_current
            || !self.active_curve_geometry_is_current()
        {
            return Err(CompositionError::CurveGeometry(
                tekes_ui_scene::ClipError::StaleShapeKey,
            ));
        }
        if let Some(table) = &self.curve_shapes {
            scene
                .try_import_clip_shapes(table, Default::default())
                .map_err(CompositionError::SceneFragment)?;
        }
        self.paint_current_frame(scene, None, text, frame, clip, palette)
    }
    pub(super) fn render_frame(
        &mut self,
        scene: &mut Scene,
        mut overlays: Option<&mut Scene>,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        if let Some(error) = scene.composition_error() {
            return Err(CompositionError::SceneFragment(error));
        }
        if let Some(error) = overlays.as_deref().and_then(Scene::composition_error) {
            return Err(CompositionError::SceneFragment(error));
        }
        if self.reading_transaction {
            return Err(CompositionError::ReadingTransactionInProgress);
        }
        self.preflight_empty_state_structures()?;
        self.preflight_notice_structures()?;
        self.root.refresh_activity(self.window_active);
        if let Err(e) = self.root.ids(&mut BTreeSet::new()) {
            self.cancel();
            return Err(e);
        }
        self.root
            .reconcile_presentation_runnable(self.window_active);
        self.root.validate_curve_tree(false)?;
        // Reject a changed attachment/mount before ticking any old navigation.
        self.observe_item_reveal();
        self.reveal_clock.invalidate_geometry();
        // The Host clock withdraws geometry during layout. Publish the final
        // replacement context after arrange instead of reporting an intermediate
        // removal, which would cancel a remote gesture even on an unchanged
        // parent mask. Lifecycle and actual removal still publish None.
        self.layout_preflighted(text, frame, clip)?;
        self.freeze_curve_geometry()?;
        self.reveal_clock.pending = false;
        if let Some(result) = self.apply_list_navigation_reveal() {
            if result.is_ok_and(|outcome| outcome.changed) {
                self.reveal_clock.invalidate_geometry();
                self.layout_preflighted(text, frame, clip)?;
            }
            if result.is_ok() {
                // Even a stationary accepted commit retires the old geometry
                // lease; publish the final geometry before importing/painting.
                self.freeze_curve_geometry()?;
                self.reveal_clock.pending = false;
            }
            // Planner refusals are recorded on the original List. They do not
            // turn a valid clipped/unsupported presentation into a render error.
        }
        if let Some(table) = &self.curve_shapes {
            scene
                .try_import_clip_shapes(table, Default::default())
                .map_err(CompositionError::SceneFragment)?;
            if let Some(overlays) = overlays.as_deref_mut() {
                overlays
                    .try_import_clip_shapes(table, Default::default())
                    .map_err(CompositionError::SceneFragment)?;
            }
        }
        self.reveal_clock.pending = false;
        self.observe_item_reveal();
        self.advance_notices(text)?;
        self.reconcile_active_inputs();
        if self.modal_focus_pending && self.focused.is_none() {
            self.modal_focus_pending = false;
            let mut ids = Vec::new();
            self.scope().focus_ids(&mut ids);
            self.focus(ids.first().copied(), true);
        }
        self.paint_current_frame(scene, overlays, text, frame, clip, palette)
    }
    pub(super) fn paint_current_frame(
        &mut self,
        scene: &mut Scene,
        overlays: Option<&mut Scene>,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        let paint_id = self.presented_dialog;
        let mut theme = ContentTheme::from(palette);
        if self.inherited_surface_declared {
            theme = theme
                .with_surface(self.inherited_surface)
                .expect("validated inherited Surface");
        }
        self.root.paint_except(scene, text, theme, paint_id);
        if overlays.is_some()
            && let Some(error) = scene.composition_error()
        {
            return Err(CompositionError::SceneFragment(error));
        }
        let scene = overlays.unwrap_or(scene);
        if self.modal.is_none() {
            self.root.paint_overlays(scene, text, theme);
        }
        self.paint_registered_popovers(scene, text, theme);
        if let Some(id) = paint_id {
            let dialog_theme = self.root.theme_for(id, theme);
            if let (Some(node), Some(dialog_theme)) = (self.root.find_mut(id), dialog_theme) {
                let presented = node
                    .component::<DialogContent>()
                    .is_some_and(DialogContent::is_presented);
                if presented && node.visible().is_some() {
                    // Rebuild all glyph/image geometry in this host frame, including
                    // exit frames. OverlayPresence must never replay stale atlas slots.
                    let mut current = scene.fragment();
                    scene_dialog_contents(node, &mut current, text, dialog_theme);
                    let (frame, clip) = (node.frame, node.clip);
                    node.component_mut::<DialogContent>()
                        .expect("dialog paint owner")
                        .try_present(scene, text, frame, clip, palette, Some(current))
                        .map_err(|error| {
                            scene.record_composition_error(error);
                            CompositionError::SceneFragment(error)
                        })?;
                } else if let Some(dialog) = node.component_mut::<DialogContent>() {
                    dialog.clear_presentation();
                }
                if node
                    .component::<DialogContent>()
                    .is_none_or(|d| !d.is_presented())
                {
                    self.presented_dialog = None;
                }
            } else {
                self.presented_dialog = None;
            }
        }
        if let Some((owner, reason)) = self.root.icon_failure().or_else(|| {
            self.presented_dialog
                .and_then(|id| self.root.find(id))
                .and_then(|node| {
                    node.component
                        .children()
                        .iter()
                        .find_map(Content::icon_failure)
                })
        }) {
            self.cancel_pointer_capture();
            return Err(CompositionError::IconResource { owner, reason });
        }
        if let Some(id) = self.root.label_failure() {
            self.cancel();
            return Err(CompositionError::MissingLabelServices(id));
        }
        if !self.help_overlay_deferred {
            self.render_help(scene, text, frame, clip, palette);
        }
        if let Some(error) = scene.composition_error() {
            return Err(CompositionError::SceneFragment(error));
        }
        self.dirty = false;
        Ok(())
    }
    // render_frame already checked the adapter boundary before legacy ID-error
    // cancellation.
    pub(super) fn layout_preflighted(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
    ) -> Result<Rect, CompositionError> {
        let requested = (frame, clip);
        if self
            .last_layout
            .is_some_and(|previous| previous != requested)
        {
            self.interrupt_item_reveal_layout();
        }
        self.last_layout = None;
        self.check_text_services(text)?;
        self.root.reset_label_layouts();
        self.prepare_split_layout(frame, clip);
        let measured = self.root.measure(text, frame.size);
        self.check_text_services(text)?;
        if let Some(id) = self.root.label_failure() {
            self.cancel();
            return Err(CompositionError::MissingLabelServices(id));
        }
        let frame = clean_rect(Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            frame.size.height.max(measured.size.height),
        ));
        self.root.arrange(text, frame, clip);
        self.finish_split_layout();
        self.arrange_popovers(text, requested.0, requested.1);
        self.arrange_context_menus(requested.0, requested.1);
        self.arrange_hover_previews(text, requested.0, requested.1);
        self.check_text_services(text)?;
        if let Some(id) = self.root.label_failure() {
            self.cancel();
            return Err(CompositionError::MissingLabelServices(id));
        }
        self.root.refresh_activity(self.window_active);
        self.root.notify_presentation_runnable(self.window_active);
        self.publish_checkbox_exposures()?;
        self.publish_empty_state_exposures()?;
        self.publish_notice_exposures()?;
        self.last_layout = Some(requested);
        Ok(frame)
    }
    pub(super) fn reconcile_active_inputs(&mut self) {
        self.reconcile_popovers();
        self.reconcile_context_menus();
        self.reconcile_hover_previews();
        self.root.refresh_activity(self.window_active);
        self.root
            .reconcile_presentation_runnable(self.window_active);
        if self
            .modal
            .is_some_and(|(id, _)| self.root.find_active(id).is_none())
        {
            self.close_dialog();
            self.clear_dialog_presentation();
        }
        if self
            .presented_dialog
            .is_some_and(|id| self.root.find_in_active_ancestors(id).is_none())
        {
            self.clear_dialog_presentation();
        }
        self.reconcile_overlay_scopes();
        if self.captured.is_some_and(|id| {
            self.root.find_active(id).is_none_or(|c| {
                self.captured_mount != Some(c.mount)
                    || c.visible().is_none()
                    || c.component.semantics().disabled
                    || !c.component.pointer_capture_current()
                    || c.component::<ComboBoxContent>()
                        .is_some_and(|combo| !combo.capture_current(self.captured_overlay))
            })
        }) {
            if let Some(c) = self.captured.take().and_then(|id| self.root.find_mut(id)) {
                if c.component::<ComboBoxContent>().is_some()
                    && self.captured_mount == Some(c.mount)
                {
                    self.retired_combo_release = true;
                    c.component.cancel_pointer();
                    c.component
                        .event(&ContentEvent::PointerMove(Point::new(f32::MIN, f32::MIN)));
                } else {
                    c.component.event(&ContentEvent::Cancel);
                }
            }
            self.captured_mount = None;
            self.captured_overlay = false;
            self.captured_overlay_handler = false;
            self.dirty = true;
        }
        if let Some(id) = self.focused.filter(|id| {
            self.captured != Some(*id)
                && self
                    .scope()
                    .find_active(*id)
                    .is_none_or(|c| !c.component.focusable() || c.component.semantics().disabled)
        }) && matches!(
            self.admit_ordinary_focus(None, false, None, action_preparation::FocusOrigin::Direct),
            action_preparation::FocusAdmission::Accepted { .. }
        ) {
            // Cancel belongs to a committed departure. A refused unmark must
            // preserve the original editor's pointer/scroll interaction too.
            if let Some(c) = self.root.find_mut(id) {
                c.component.event(&ContentEvent::Cancel);
            }
        }
        self.reconcile_deferred_text_key();
        self.reconcile_text_wheel();
        self.reconcile_list_navigation_reveal();
    }
    pub fn focus(&mut self, id: Option<ComponentId>, visible: bool) -> bool {
        let changed = matches!(
            self.admit_ordinary_focus(id, visible, None, action_preparation::FocusOrigin::Direct),
            action_preparation::FocusAdmission::Accepted { changed: true }
        );
        self.reconcile_text_wheel();
        self.reconcile_list_navigation_reveal();
        changed
    }
    /// Disable ordinary boundary wrapping for an embedded host. Modal and
    /// interactive overlay scopes still keep traversal inside their owner.
    pub fn set_focus_wrap(&mut self, wrap: bool) {
        self.focus_wrap = wrap;
    }
    /// Enter the first/last eligible original owner in the current input scope.
    /// Composing editors retain their existing Tab/IME boundary; accepted entry
    /// uses the same focus/reveal admission and traversal direction as Tab.
    pub fn enter_focus(&mut self, reverse: bool) -> bool {
        if self.is_composing() {
            return false;
        }
        let mut ids = Vec::new();
        self.scope().focus_ids(&mut ids);
        let target = if reverse { ids.last() } else { ids.first() }.copied();
        matches!(
            self.admit_ordinary_focus(
                target,
                true,
                Some(reverse),
                action_preparation::FocusOrigin::Traversal
            ),
            action_preparation::FocusAdmission::Accepted { changed: true }
        )
    }
    /// Whether the last event may continue to the enclosing host. A handled
    /// event can bubble, so consumed alone is not a propagation decision.
    /// Modal, IME and system command barriers remain authoritative.
    pub fn event_bubbled(&self) -> bool {
        self.event_bubbled.unwrap_or(false)
    }
    /// Whether the last pointer delivery requested activation focus, including
    /// a click on an already focused owner. Read-only original routing outcome.
    pub fn pointer_focus_requested(&self) -> bool {
        self.pointer_focus_requested
    }
    /// Result of the most recent accepted reading wheel, not a queued callback.
    /// A replaced, inactive, disabled or out-of-scope source is no longer current.
    pub fn reading_scroll_source(&self) -> Option<(ComponentId, u64)> {
        if !self.window_active {
            return None;
        }
        let (id, mount) = self.reading_scroll_source?;
        self.scope()
            .find_active(id)
            .filter(|node| node.mount == mount && !node.component.semantics().disabled)
            .map(|_| (id, mount))
    }
    pub(super) fn record_reading_scroll(
        &mut self,
        event: &ContentEvent,
        id: ComponentId,
        response: &ContentResponse,
    ) {
        let ContentEvent::Scroll { position, .. } = event else {
            return;
        };
        if response.consumed
            && let Some(node) = self.scope().find_active(id)
            && node
                .visible()
                .is_some_and(|frame| frame.contains(*position))
            && node.curve_contains(*position)
            && !node.component.semantics().disabled
            && node.component.reading_scroll_source()
        {
            self.reading_scroll_source = Some((id, node.mount));
        }
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Scroll { .. }) && self.text_wheel_dispatch.is_none() {
            // Compatibility packets carry no gesture provenance and cannot continue a lease.
            self.retire_text_wheel();
        }
        self.reading_scroll_source = None;
        self.pointer_focus_requested = false;
        if !self.window_active {
            return ContentResponse::default();
        }
        if self.search_pointer_blocked(event) {
            return ContentResponse::default();
        }
        self.reveal_clock.invalidate_input();
        self.event_bubbled = None;
        let overlay_barrier = self.overlay_scope_id().is_some();
        let reader_lease = self.focused.and_then(|id| {
            let owner = self.scope().find_active(id)?;
            let surface = owner.component.text_surface()?;
            (surface.input.read_only && !surface.input.disabled).then_some((
                id,
                owner.mount,
                self.focus_epoch,
                surface.input.controlled_binding(),
            ))
        });
        let completing_navigation = self.completing_reading_navigation(event);
        let previous_caret = self.focused.and_then(|id| {
            self.focused_text()
                .filter(|surface| {
                    surface.presentation == crate::TextSurfacePresentation::Content
                        && surface.input.read_only
                })
                .map(|surface| (id, surface.input.caret))
        });
        let mut result = self.key_routed_event(event);
        self.reconcile_active_inputs();
        let routed = event.routing_event();
        let navigation = matches!(
            &*routed,
            ContentEvent::Key {
                key: ControlKey::Left
                    | ControlKey::Right
                    | ControlKey::Up
                    | ControlKey::Down
                    | ControlKey::Home
                    | ControlKey::End,
                down: true,
            }
        ) || matches!(&*routed, ContentEvent::TextCommand(command)
            if command.is_movement() || *command == TextEditCommand::SelectAll);
        // A consumed key is a reading intent only if the original reader
        // accepted navigation, not merely because an IME/host barrier consumed it.
        let delivered_navigation = navigation && self.event_bubbled.is_some()
            || completing_navigation
                && result.text_key.is_some_and(|key| {
                    key.disposition == TextKeyDisposition::EditHandled
                        && key.target == reader_lease.map(|lease| lease.0)
                });
        let reading_source = match &*routed {
            ContentEvent::PointerDown(_)
                if event.is_primary_pointer() && result.capture_pointer =>
            {
                self.captured.and_then(|id| {
                    let mount = self.captured_mount?;
                    self.root
                        .find_active(id)
                        .filter(|owner| owner.mount == mount)
                        .map(|_| (id, mount))
                })
            }
            ContentEvent::PointerMove(_) if event.is_primary_pointer() && result.changed => {
                self.captured.and_then(|id| {
                    let mount = self.captured_mount?;
                    self.root
                        .find_active(id)
                        .filter(|owner| owner.mount == mount)
                        .map(|_| (id, mount))
                })
            }
            _ if delivered_navigation && result.consumed => {
                reader_lease.and_then(|(id, mount, epoch, binding)| {
                    (self.focused == Some(id)
                        && self.focus_epoch == epoch
                        && self
                            .focused_text()
                            .is_some_and(|s| s.input.controlled_binding() == binding))
                    .then_some((id, mount))
                })
            }
            _ => None,
        };
        if let Some((source, mount)) = reading_source {
            result.changed |= self
                .scope_with_actions()
                .0
                .observe_manual_history(source, mount)
                .unwrap_or(false);
        }
        // Document text has no nested scroll owner. Explicit navigation/drag
        // reveals through the same ancestor path as ordinary focus. Pointer
        // down already called focus(), so must not reveal stale geometry twice.
        let pointer_down = matches!(event, ContentEvent::PointerDown(_))
            || matches!(event, ContentEvent::Pointer(p) if p.phase == PointerPhase::Down);
        if !pointer_down
            && let Some((id, caret)) = previous_caret
            && self.focused == Some(id)
            && let Some(surface) = self.focused_text()
            && surface.input.caret != caret
        {
            let frame = surface.caret();
            match self.plan_host_reveal_rect(
                id,
                frame,
                crate::ScrollRevealOptions {
                    animated: false,
                    ..Default::default()
                },
                &[],
                content_reveal_plan::HostRevealBudget::default(),
            ) {
                Ok(plan) => {
                    let _ = self.commit_host_reveal(plan);
                }
                Err(HostRevealError::UnsupportedLayout) if self.original_reveal_admissible(id) => {
                    self.root.reveal(id, frame);
                }
                Err(_) => {}
            }
        }
        // Preserve the existing modal barrier for hosts which only inspect
        // consumed; explicit bubbling cannot expose background input.
        if self.modal.is_some() || overlay_barrier {
            result.consumed = true;
            self.event_bubbled = Some(false);
        }
        self.dirty |= result.changed;
        self.event_bubbled =
            Some(self.modal.is_none() && self.event_bubbled.unwrap_or(!result.consumed));
        result
    }
    pub(super) fn retire_text_wheel(&mut self) {
        if let Some(lease) = self.text_wheel.take() {
            // End only the original mount's wheel physics, never pointer/IME/focus.
            if let Some(node) = self.root.find_mut(lease.target)
                && lease.path.last() == Some(&(node.id, node.mount))
                && let Some(field) = node.component_mut::<TextFieldContent>()
            {
                field.surface.scroll.end_gesture();
            }
            self.text_wheel_retired = true;
        }
    }
    pub(super) fn reconcile_text_wheel(&mut self) {
        if self.text_wheel.as_ref().is_some_and(|lease| {
            !self.window_active
                || lease.focus_epoch != self.focus_epoch
                || self.scope().text_wheel_path(lease.target).as_ref() != Some(&lease.path)
                || self.scope().find_active(lease.target).is_none_or(|node| {
                    node.visible().is_none()
                        || node.component.semantics().disabled
                        || node.component::<TextFieldContent>().is_none()
                })
        }) {
            self.retire_text_wheel();
        }
    }
    /// Native phased wheel transport for the original TextField owner. Ordinary
    /// ContentEvent::Scroll and non-TextField consumers retain hit routing.
    pub fn wheel_event(&mut self, wheel: &ContentWheelEvent) -> ContentResponse {
        use ContentWheelPhase as P;
        self.reading_scroll_source = None;
        self.event_bubbled = Some(false);
        self.pointer_focus_requested = false;
        self.reconcile_active_inputs();
        let phased = wheel.precise && (wheel.phase != P::None || wheel.momentum_phase != P::None);
        let starts = phased && matches!(wheel.phase, P::MayBegin | P::Began);
        if starts || !phased {
            self.retire_text_wheel();
            self.text_wheel_retired = false;
        } else if self.text_wheel_retired {
            // Tail packets from a retired mount/blur must not acquire a new owner.
            return ContentResponse::consumed(false);
        }
        if self
            .text_wheel
            .as_ref()
            .is_some_and(|lease| lease.awaiting_momentum)
            && wheel.momentum_phase == P::None
            && wheel.phase == P::Changed
        {
            return ContentResponse::consumed(false);
        }
        self.text_wheel_dispatch = phased.then_some(if starts {
            TextWheelDispatch::Start
        } else {
            TextWheelDispatch::Continue
        });
        let result = self.event(&ContentEvent::Scroll {
            position: wheel.position,
            delta: wheel.delta,
            precise: wheel.precise,
            begin: starts,
            end: matches!(wheel.phase, P::Ended | P::Cancelled)
                || matches!(wheel.momentum_phase, P::Ended | P::Cancelled),
            momentum: wheel.momentum_phase != P::None,
            time: wheel.time,
        });
        self.text_wheel_dispatch = None;
        if wheel.phase == P::Cancelled || matches!(wheel.momentum_phase, P::Ended | P::Cancelled) {
            self.retire_text_wheel();
        } else if wheel.phase == P::Ended
            && wheel.momentum_phase == P::None
            && let Some(lease) = &mut self.text_wheel
        {
            // A finger Ended can be followed by native momentum outside the frame.
            lease.awaiting_momentum = true;
        } else if wheel.momentum_phase != P::None
            && let Some(lease) = &mut self.text_wheel
        {
            lease.awaiting_momentum = false;
        }
        result
    }
    pub fn wheel_event_with_text(
        &mut self,
        wheel: &ContentWheelEvent,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<ContentResponse, CompositionError> {
        self.with_text_input(text, |host| host.wheel_event(wheel))
    }
    /// Use the caller's existing font backend before/after an input callback.
    /// This keeps back-to-back commands and native text queries on the current
    /// paragraph without rendering a GPU frame or inventing a text backend.
    /// Render first and after parent geometry changes, as for ordinary event().
    pub fn event_with_text(
        &mut self,
        event: &ContentEvent,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<ContentResponse, CompositionError> {
        self.reading_scroll_source = None;
        if matches!(event, ContentEvent::Cancel) {
            // Preserve original cancellation before any delegated ack reconciliation.
            return Ok(self.event(event));
        }
        self.with_text_input(text, |host| host.event(event))
    }
    /// Run a native text-service operation against the current original owner.
    /// Clipboard claims need the same pre/post layout as ordinary text events.
    pub fn with_text_input<R>(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        dispatch: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, CompositionError> {
        self.reading_scroll_source = None;
        if let Some(id) = self.root.needs_text()
            && text.text_surface_host().is_none()
        {
            return Err(CompositionError::MissingTextServices(id));
        }
        self.prepare_focused_text(text);
        let result = dispatch(self);
        self.prepare_focused_text(text);
        Ok(result)
    }
    pub(super) fn prepare_focused_text(&mut self, text: &mut dyn ButtonTextRenderer) {
        self.reconcile_active_inputs();
        if let Some(id) = self.focused
            && self.scope().find_active(id).is_some()
            && let Some(owner) = self.root.find_mut(id)
        {
            owner
                .component
                .prepare_text_input(text, owner.frame, owner.clip);
        }
    }
}
