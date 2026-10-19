//! EXPERIMENTAL: minimum-scenario protocol; not yet approved for broad migration.
//!
//! Owned component composition over the existing scene, text, layout and scroll APIs.
//!
//! Keys identify owned instances, not positions. Reordering moves instances; it
//! never reconstructs controls. Removed content is returned to its owner, which
//! may retain it for virtualization. No implicit serialization or rehydration.
mod actions;
mod adapters;
mod key;
mod pointer;
use crate::extended::{Axis, Grid, Stack};
use crate::layout::{AlignmentOverride, MeasuredChild, layout_measured_children};
use crate::paint::ButtonVariant;
use crate::{
    Button, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext,
    ScrollArea,
};
pub use actions::{ContentAction, ContentActionKind, ContentActionSource};
pub use adapters::{
    DialogContent, ListContent, MenuContent, SliderContent, SwitchContent, TextFieldContent,
    TextViewport,
};
pub use key::{
    ContentKeyEvent, TextKeyDisposition, TextKeyOutcome, TextSubmitPolicy, TextTabPolicy,
};
pub use pointer::{InputModifiers, PointerButton, PointerEvent, PointerModifiers, PointerPhase};
use std::{any::Any, collections::BTreeSet, fmt::Debug, ops::Range};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub u64);

/// Platform-neutral events. Native adapters retain IME clause/candidate handling.
#[derive(Clone, Debug, PartialEq)]
pub enum ContentEvent {
    /// Lossless native pointer input. Position-only variants remain primary/single-click shorthand.
    Pointer(PointerEvent),
    KeyInput(ContentKeyEvent),
    /// Enclose the existing once-only text callbacks from one native key-down.
    TextKeyStart {
        id: u64,
        key: ContentKeyEvent,
        composing: bool,
    },
    /// `input_context_consumed` means the input system consumed the key without
    /// dispatching any callbacks. Do not set it for an ordinary command callback.
    TextKeyEnd {
        id: u64,
        input_context_consumed: bool,
    },
    PointerMove(Point),
    PointerDown(Point),
    PointerUp(Point),
    Key {
        key: ControlKey,
        down: bool,
    },
    /// Delta is a document-space offset (positive moves toward later content).
    Scroll {
        position: Point,
        delta: Point,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
        time: f64,
    },
    Commit {
        text: String,
        replacement: Option<Range<usize>>,
    },
    Preedit {
        text: String,
        selection: Range<usize>,
        clauses: Vec<Range<usize>>,
        replacement: Option<Range<usize>>,
    },
    Unmark,
    TextCommand(String),
    Cancel,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContentResponse {
    pub consumed: bool,
    pub changed: bool,
    pub capture_pointer: bool,
    pub activated: bool,
    pub text_key: Option<TextKeyOutcome>,
}
impl ContentResponse {
    pub const fn consumed(changed: bool) -> Self {
        Self {
            consumed: true,
            changed,
            capture_pointer: false,
            activated: false,
            text_key: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Mounted,
    Unmounted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentRole {
    Group,
    Text,
    Button,
    TextInput,
    Slider,
    Switch,
    List,
    ListItem,
    Dialog,
    MenuButton,
    MenuItem,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentSemantics {
    pub role: ContentRole,
    pub name: String,
    pub value: Option<String>,
    pub disabled: bool,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub invalid: bool,
    pub description: Option<String>,
}
impl ContentSemantics {
    pub fn new(role: ContentRole, name: impl Into<String>) -> Self {
        Self {
            role,
            name: name.into(),
            value: None,
            disabled: false,
            selected: false,
            expanded: None,
            invalid: false,
            description: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccessibilityAction {
    Increment,
    Decrement,
    Focus,
    Press,
    SetValue(String),
    SetSelection(std::ops::Range<usize>),
    ReplaceSelectedText(String),
}
/// Semantic descendants owned by a component model (for example menu items),
/// identified by a stable component-local index rather than another UI instance.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticPart {
    pub index: usize,
    pub frame: Rect,
    pub focused: bool,
    pub semantics: ContentSemantics,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: ComponentId,
    pub parent: Option<ComponentId>,
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub focused: bool,
    pub semantics: ContentSemantics,
    pub parts: Vec<SemanticPart>,
}

/// Palette plus an optional inherited foreground for decorative action content.
/// No colors are authored here: both values resolve from the JSON palette.
#[derive(Clone, Copy, Debug)]
pub struct ContentTheme {
    pub palette: Palette,
    pub foreground: Option<Rgba>,
}
impl From<Palette> for ContentTheme {
    fn from(palette: Palette) -> Self {
        Self {
            palette,
            foreground: None,
        }
    }
}

/// Consumer-extensible component protocol. The object owns its existing model
/// and view. Containers expose the same Content children as library containers.
/// Geometry is in host coordinates; `clip` is the actual ancestor viewport,
/// never an implicit clip to a control's own frame (including its focus ring).
/// `measure` is read-layout work, `arrange` places descendants, `paint` may use
/// existing render APIs. Hosts call render before input after geometry changes.
pub trait Component: Debug + Any {
    /// Refresh only the existing arranged text geometry, not a second layout or
    /// painting pipeline. Text-owning wrappers forward this to their public field.
    fn prepare_text_input(
        &mut self,
        _text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
    ) {
    }
    /// Pure per-delivery declarations, in order; independent of consumed/changed.
    /// Defaults preserve legacy activation; producers declare more specific kinds.
    /// Override for an owner emitting multiple kinds from one event. No host
    /// callback is invoked here; consumers handle the owned drain afterwards.
    fn action_kinds(&self, result: &ContentResponse) -> Vec<ContentActionKind> {
        if result.activated {
            vec![ContentActionKind::Activate]
        } else {
            Vec::new()
        }
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        key.key
            .map(|k| {
                self.event(&ContentEvent::Key {
                    key: k,
                    down: key.down,
                })
            })
            .unwrap_or_default()
    }
    fn overlay_key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        key.key
            .map(|k| {
                self.overlay_event(&ContentEvent::Key {
                    key: k,
                    down: key.down,
                })
            })
            .unwrap_or_default()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild;
    /// Check semantic slot constraints before a host mounts or uses this node.
    fn validate(&self) -> Result<(), CompositionError> {
        Ok(())
    }
    /// Closed semantic subtrees retain owned state but do not paint or receive input.
    fn active(&self) -> bool {
        true
    }
    fn fixed_slots(&self) -> bool {
        false
    }
    fn requires_text_services(&self) -> bool {
        false
    }
    fn text_surface(&self) -> Option<&crate::TextSurface> {
        None
    }
    fn overlay_open(&self) -> bool {
        false
    }
    fn overlay_presented(&self) -> bool {
        self.overlay_open()
    }
    fn paint_overlay(
        &mut self,
        _scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        _theme: ContentTheme,
    ) {
    }
    /// Semantic bounds may differ from the composite layout frame (input slot/modal).
    fn semantic_frame(&self, frame: Rect) -> Rect {
        frame
    }
    /// Optional numerical value domain, independent of the semantic snapshot's
    /// string value. Existing component implementations/literals stay unchanged.
    fn value_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        None
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        Vec::new()
    }
    fn accessibility_action(
        &mut self,
        _part: Option<usize>,
        _action: &AccessibilityAction,
    ) -> ContentResponse {
        ContentResponse::default()
    }
    fn overlay_event(&mut self, _event: &ContentEvent) -> ContentResponse {
        ContentResponse::default()
    }
    fn arrange(&mut self, _text: &mut dyn ButtonTextRenderer, _frame: Rect, _clip: Rect) {}
    fn paint(
        &mut self,
        _scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        _theme: ContentTheme,
    ) {
    }
    /// Declare independent interaction, separately from keyboard focus.
    /// Pointer-only consumer controls must override this; passive decorative
    /// content never receives routed pointer events, even outside an action.
    fn interactive(&self) -> bool {
        self.focusable()
            || matches!(
                self.semantics().role,
                ContentRole::Button
                    | ContentRole::TextInput
                    | ContentRole::Slider
                    | ContentRole::Switch
            )
    }
    /// Full metadata entry point. The default preserves existing primary-pointer
    /// components; context actions opt in here instead of activating ordinary controls.
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if pointer.is_primary() {
            self.event(&pointer.routing_event())
        } else {
            ContentResponse::default()
        }
    }
    fn overlay_pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if pointer.is_primary() {
            self.overlay_event(&pointer.routing_event())
        } else {
            ContentResponse::default()
        }
    }
    fn event(&mut self, _event: &ContentEvent) -> ContentResponse {
        ContentResponse::default()
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        theme
    }
    fn children(&self) -> &[Content] {
        &[]
    }
    /// Some enables child mutations through ContentHost; None identifies a leaf.
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        None
    }
    fn focusable(&self) -> bool {
        false
    }
    fn set_focus(&mut self, _focused: bool, _visible: bool) {}
    fn composing(&self) -> bool {
        false
    }
    fn lifecycle(&mut self, _event: Lifecycle) {}
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "")
    }
    fn reveal(&mut self, descendant: Rect) -> Rect {
        descendant
    }
}
#[derive(Debug)]
pub struct Content {
    id: ComponentId,
    component: Box<dyn Component>,
    frame: Rect,
    clip: Rect,
    mount: u64,
}
impl Content {
    pub fn new(id: ComponentId, component: impl Component) -> Self {
        Self {
            id,
            component: Box::new(component),
            frame: Rect::default(),
            clip: Rect::default(),
            mount: actions::next_mount(),
        }
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
    pub fn component<T: Component>(&self) -> Option<&T> {
        (self.component.as_ref() as &dyn Any).downcast_ref()
    }
    pub fn component_mut<T: Component>(&mut self) -> Option<&mut T> {
        (self.component.as_mut() as &mut dyn Any).downcast_mut()
    }
    pub fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let mut measured = self.component.measure(text, clean_size(available));
        measured.size = clean_size(measured.size);
        measured.first_baseline = measured.first_baseline.filter(|v| v.is_finite());
        measured
    }
    pub fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.frame = clean_rect(frame);
        self.clip = clean_rect(clip);
        self.component.arrange(text, self.frame, self.clip);
    }
    fn paint(&mut self, scene: &mut Scene, text: &mut dyn ButtonTextRenderer, theme: ContentTheme) {
        self.paint_except(scene, text, theme, None);
    }
    fn paint_except(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
        skip: Option<ComponentId>,
    ) {
        if skip == Some(self.id) {
            return;
        }
        if !self.component.active() {
            return;
        }
        // Offscreen ancestors may contain visible overflowing children. Walk
        // children even when their parent frame itself is outside the viewport.
        if self.visible().is_some() {
            scene.push_clip(self.clip);
            self.component
                .paint(scene, text, self.frame, self.clip, theme);
            scene.pop_clip();
        }
        let child_theme = self.component.child_theme(theme);
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.paint_except(scene, text, child_theme, skip);
            }
        }
    }
    fn paint_overlays(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        if !self.component.active() {
            return;
        }
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.paint_overlays(scene, text, theme);
            }
        }
        if self.component.overlay_presented() {
            self.component
                .paint_overlay(scene, text, self.frame, self.clip, theme);
        }
    }
    fn overlay_id(&self) -> Option<ComponentId> {
        if !self.component.active() {
            return None;
        }
        self.component
            .children()
            .iter()
            .rev()
            .find_map(Content::overlay_id)
            .or_else(|| self.component.overlay_open().then_some(self.id))
    }
    fn needs_text(&self) -> Option<ComponentId> {
        if !self.component.active() {
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
    fn find_active(&self, id: ComponentId) -> Option<&Content> {
        self.find_in_active_ancestors(id)
            .filter(|node| node.component.active())
    }
    // Exit presentation may retain an inactive Dialog owner, but never bypass
    // an inactive ancestor such as a collapsed or replaced content panel.
    fn find_in_active_ancestors(&self, id: ComponentId) -> Option<&Content> {
        if self.id == id {
            return Some(self);
        }
        if !self.component.active() {
            return None;
        }
        self.component
            .children()
            .iter()
            .find_map(|c| c.find_in_active_ancestors(id))
    }
    fn find(&self, id: ComponentId) -> Option<&Content> {
        if self.id == id {
            Some(self)
        } else {
            self.component.children().iter().find_map(|c| c.find(id))
        }
    }
    fn find_mut(&mut self, id: ComponentId) -> Option<&mut Content> {
        if self.id == id {
            return Some(self);
        }
        self.component
            .children_mut()?
            .iter_mut()
            .find_map(|c| c.find_mut(id))
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.mount = actions::next_mount();
        if event == Lifecycle::Unmounted {
            self.component.event(&ContentEvent::Cancel);
            self.component.set_focus(false, false);
        }
        self.component.lifecycle(event);
        if let Some(children) = self.component.children_mut() {
            for c in children {
                c.lifecycle(event)
            }
        }
    }
    fn interactive_id(&self) -> Option<ComponentId> {
        if self.component.interactive() {
            return Some(self.id);
        }
        self.component
            .children()
            .iter()
            .find_map(Content::interactive_id)
    }
    fn action_ancestor(&self, target: ComponentId) -> Option<ComponentId> {
        self.find(target)?;
        if self.component.semantics().role == ContentRole::Button {
            return Some(self.id);
        }
        self.component
            .children()
            .iter()
            .find_map(|c| c.action_ancestor(target))
    }
    fn ids(&self, ids: &mut BTreeSet<ComponentId>) -> Result<(), CompositionError> {
        self.component.validate()?;
        if !ids.insert(self.id) {
            return Err(CompositionError::DuplicateId(self.id));
        }
        if self.component.semantics().role == ContentRole::Button
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
    fn focus_ids(&self, ids: &mut Vec<ComponentId>) {
        if !self.component.active() {
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
    fn route(
        &mut self,
        event: &ContentEvent,
        position: Point,
        actions: &mut Option<Vec<actions::PendingAction>>,
    ) -> Option<(ComponentId, ContentResponse)> {
        if !self.component.active() {
            return None;
        }
        let action = self.component.semantics().role == ContentRole::Button;
        if !action && let Some(children) = self.component.children_mut() {
            for c in children.iter_mut().rev() {
                if let Some(r) = c.route(event, position, actions) {
                    return Some(r);
                }
            }
        }
        if self.component.interactive() && self.visible().is_some_and(|r| r.contains(position)) {
            let r = self.deliver_actions(event, false, actions);
            if r.consumed {
                return Some((self.id, r));
            }
        }
        None
    }
    fn move_pointer(
        &mut self,
        event: &ContentEvent,
        actions: &mut Option<Vec<actions::PendingAction>>,
    ) {
        if !self.component.active() {
            return;
        }
        if self.component.interactive() {
            self.deliver_actions(event, false, actions);
        }
        if let Some(children) = self.component.children_mut() {
            for c in children {
                c.move_pointer(event, actions)
            }
        }
    }
    fn reveal(&mut self, id: ComponentId, frame: Rect) -> Option<Rect> {
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
    fn semantic_nodes(
        &self,
        parent: Option<ComponentId>,
        focused: Option<ComponentId>,
        out: &mut Vec<SemanticNode>,
    ) {
        if !self.component.active() {
            return;
        }
        out.push(SemanticNode {
            id: self.id,
            parent,
            frame: self.component.semantic_frame(self.frame),
            visible: self
                .component
                .semantic_frame(self.frame)
                .intersection(&self.clip),
            focused: focused == Some(self.id),
            semantics: self.component.semantics(),
            parts: self.component.semantic_parts(),
        });
        // Ordinary action content is decorative to assistive technology;
        // the action supplies one explicit accessible name and one focus.
        if self.component.semantics().role != ContentRole::Button {
            for c in self.component.children() {
                c.semantic_nodes(Some(self.id), focused, out)
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionError {
    DuplicateId(ComponentId),
    MissingId(ComponentId),
    NotContainer(ComponentId),
    InvalidOrder,
    EmptyAccessibleName,
    InvalidSlotCount,
    MissingTextServices(ComponentId),
    NotDialog(ComponentId),
    ModalAlreadyOpen,
    NestedInteractive {
        action: ComponentId,
        child: ComponentId,
    },
}
#[derive(Debug)]
pub struct ContentHost {
    root: Content,
    focused: Option<ComponentId>,
    captured: Option<ComponentId>,
    dirty: bool,
    modal: Option<(ComponentId, Option<ComponentId>)>,
    modal_focus_pending: bool,
    // Paint-only owner after close. It is never used as the event/focus scope.
    presented_dialog: Option<ComponentId>,
    text_keys: Vec<key::KeyFrame>,
    last_text_key: u64,
    focus_epoch: u64,
    actions: Option<Vec<actions::PendingAction>>,
}
impl ContentHost {
    pub fn new(mut root: Content) -> Result<Self, CompositionError> {
        root.ids(&mut BTreeSet::new())?;
        root.lifecycle(Lifecycle::Mounted);
        Ok(Self {
            root,
            focused: None,
            captured: None,
            dirty: true,
            modal: None,
            modal_focus_pending: false,
            presented_dialog: None,
            text_keys: Vec::new(),
            last_text_key: 0,
            focus_epoch: 0,
            actions: None,
        })
    }
    pub fn get(&self, id: ComponentId) -> Option<&Content> {
        self.root.find(id)
    }
    pub fn get_mut(&mut self, id: ComponentId) -> Option<&mut Content> {
        self.dirty = true;
        self.root.find_mut(id)
    }
    /// Mutation and event invalidation; animation callbacks remain on the
    /// existing text adapter. A successful render acknowledges this flag.
    pub fn needs_render(&self) -> bool {
        self.dirty
            || self
                .presented_dialog
                .and_then(|id| self.root.find(id))
                .and_then(|node| node.component::<DialogContent>())
                .is_some_and(DialogContent::needs_animation)
    }
    pub fn focus_targets(&self) -> Vec<(ComponentId, Rect)> {
        let mut ids = Vec::new();
        self.scope().focus_ids(&mut ids);
        ids.into_iter()
            .filter_map(|id| self.root.find(id).map(|c| (id, c.frame)))
            .collect()
    }
    pub fn focused(&self) -> Option<ComponentId> {
        self.focused
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        if let Err(e) = self.root.ids(&mut BTreeSet::new()) {
            self.cancel();
            return Err(e);
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
        let measured = self.root.measure(text, frame.size);
        let frame = clean_rect(Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            frame.size.height.max(measured.size.height),
        ));
        self.root.arrange(text, frame, clip);
        self.reconcile_active_inputs();
        if self.modal_focus_pending && self.focused.is_none() {
            self.modal_focus_pending = false;
            let mut ids = Vec::new();
            self.scope().focus_ids(&mut ids);
            self.focus(ids.first().copied(), true);
        }
        let paint_id = self.presented_dialog;
        self.root
            .paint_except(scene, text, palette.into(), paint_id);
        if self.modal.is_none() {
            self.root.paint_overlays(scene, text, palette.into());
        }
        if let Some(id) = paint_id {
            if let Some(node) = self.root.find_mut(id) {
                let presented = node
                    .component::<DialogContent>()
                    .is_some_and(DialogContent::is_presented);
                if presented && node.visible().is_some() {
                    // Rebuild all glyph/image geometry in this host frame, including
                    // exit frames. OverlayPresence must never replay stale atlas slots.
                    let mut current = Scene::new();
                    scene_dialog_contents(node, &mut current, text, palette.into());
                    let (frame, clip) = (node.frame, node.clip);
                    node.component_mut::<DialogContent>()
                        .expect("dialog paint owner")
                        .present(scene, text, frame, clip, palette, Some(current));
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
        self.dirty = false;
        Ok(())
    }
    fn reconcile_active_inputs(&mut self) {
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
        if self.captured.is_some_and(|id| {
            self.scope()
                .find_active(id)
                .is_none_or(|c| c.visible().is_none() || c.component.semantics().disabled)
        }) {
            if let Some(c) = self.captured.take().and_then(|id| self.root.find_mut(id)) {
                c.component.event(&ContentEvent::Cancel);
            }
            self.dirty = true;
        }
        if self.focused.is_some_and(|id| {
            self.scope()
                .find_active(id)
                .is_none_or(|c| !c.component.focusable() || c.component.semantics().disabled)
        }) {
            if let Some(c) = self.focused.and_then(|id| self.root.find_mut(id)) {
                c.component.event(&ContentEvent::Cancel);
            }
            self.focus(None, false);
        }
    }
    pub fn focus(&mut self, id: Option<ComponentId>, visible: bool) -> bool {
        self.dirty = true;
        if let Some(target) = id
            && (self.root.find_active(target).is_none()
                || self
                    .scope()
                    .find_active(target)
                    .is_none_or(|c| !c.component.focusable() || c.component.semantics().disabled))
        {
            return false;
        }
        if self.focused != id {
            self.focus_epoch = self
                .focus_epoch
                .checked_add(1)
                .expect("focus epoch exhausted");
        }
        if self.focused != id
            && let Some(c) = self.focused.and_then(|old| self.root.find_mut(old))
        {
            c.component.set_focus(false, false);
        }
        let changed = self.focused != id;
        self.focused = id;
        if let Some(id) = id {
            let frame = {
                let c = self.root.find_mut(id).unwrap();
                c.component.set_focus(true, visible);
                c.component
                    .text_surface()
                    .filter(|surface| {
                        surface.presentation == crate::TextSurfacePresentation::Content
                    })
                    .map_or(c.frame, |surface| surface.caret())
            };
            self.root.reveal(id, frame);
        }
        changed
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let previous_caret = self.focused.and_then(|id| {
            self.focused_text()
                .filter(|surface| {
                    surface.presentation == crate::TextSurfacePresentation::Content
                        && surface.input.read_only
                })
                .map(|surface| (id, surface.input.caret))
        });
        let result = self.key_routed_event(event);
        self.reconcile_active_inputs();
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
            self.root.reveal(id, frame);
        }
        self.dirty |= result.changed;
        result
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
        if let Some(id) = self.root.needs_text()
            && text.text_surface_host().is_none()
        {
            return Err(CompositionError::MissingTextServices(id));
        }
        self.prepare_focused_text(text);
        let result = self.event(event);
        self.prepare_focused_text(text);
        Ok(result)
    }
    fn prepare_focused_text(&mut self, text: &mut dyn ButtonTextRenderer) {
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
    fn dispatch(&mut self, event: &ContentEvent) -> ContentResponse {
        if let ContentEvent::TextCommand(command) = event {
            let key = match command.as_str() {
                "insertTab:" => Some(ControlKey::Tab { reverse: false }),
                "insertBacktab:" => Some(ControlKey::Tab { reverse: true }),
                "cancelOperation:"
                    if self.focused_text().is_none_or(|s| s.input.marked.is_none()) =>
                {
                    Some(ControlKey::Escape)
                }
                _ => None,
            };
            if let Some(key) = key {
                return self.dispatch(&ContentEvent::Key { key, down: true });
            }
        }
        if self.root.ids(&mut BTreeSet::new()).is_err() {
            self.cancel();
            return ContentResponse::default();
        }
        self.reconcile_active_inputs();
        if matches!(event, ContentEvent::Cancel) {
            self.cancel();
            return ContentResponse::consumed(true);
        }
        let routing = event.routing_event();
        let routed = routing.as_ref();
        // The composing editor owns control keys before any overlay. Preserve
        // its existing Escape/caret behavior; unhandled keys remain available to
        // the native input method. Text commits/commands still route normally.
        if matches!(
            routed,
            ContentEvent::Key { .. }
                | ContentEvent::TextCommand(_)
                | ContentEvent::Commit { .. }
                | ContentEvent::Preedit { .. }
                | ContentEvent::Unmark
        ) && (self
            .focused_text()
            .is_some_and(|surface| surface.input.marked.is_some())
            || self
                .focused
                .and_then(|id| self.root.find(id))
                .is_some_and(|content| content.component.composing()))
        {
            return self
                .focused
                .and_then(|id| self.root.find_mut(id))
                .map(|content| content.deliver_actions(event, false, &mut self.actions))
                .unwrap_or_default();
        }
        // Text callbacks always belong to the actual focused text owner, not
        // to an unrelated menu overlay (including the first preedit callback).
        if matches!(
            event,
            ContentEvent::Commit { .. } | ContentEvent::Preedit { .. } | ContentEvent::Unmark
        ) && self.focused_text().is_some()
        {
            return self
                .focused
                .and_then(|id| self.root.find_mut(id))
                .map(|c| c.deliver_actions(event, false, &mut self.actions))
                .unwrap_or_default();
        }
        // A content drag keeps ownership until release, even if that gesture
        // opens an overlay. Candidate surfaces must not steal selection capture.
        if matches!(
            routed,
            ContentEvent::PointerMove(_) | ContentEvent::PointerUp(_)
        ) && event.is_primary_pointer()
            && let Some(id) = self.captured
        {
            if matches!(routed, ContentEvent::PointerUp(_)) {
                self.captured = None;
            }
            return self
                .root
                .find_mut(id)
                .map(|content| content.deliver_actions(event, false, &mut self.actions))
                .unwrap_or_default();
        }
        if let Some(id) = self.scope().overlay_id() {
            let response =
                self.root
                    .find_mut(id)
                    .unwrap()
                    .deliver_actions(event, true, &mut self.actions);
            if response.consumed {
                return response;
            }
            if let Some(key) = self.overlay_command_key(event) {
                let response = self.root.find_mut(id).unwrap().deliver_actions(
                    &ContentEvent::KeyInput(key),
                    true,
                    &mut self.actions,
                );
                if response.consumed {
                    return response;
                }
            }
        }
        if matches!(
            routed,
            ContentEvent::Key {
                key: ControlKey::Escape,
                down: true
            }
        ) && self.modal.is_some()
            && self.focused_text().is_none_or(|s| s.input.marked.is_none())
        {
            self.close_dialog();
            return ContentResponse::consumed(true);
        }
        if let ContentEvent::Key {
            key: ControlKey::Tab { reverse },
            down: true,
        } = routed
        {
            if self
                .focused
                .and_then(|id| self.root.find(id))
                .is_some_and(|c| c.component.composing())
            {
                return ContentResponse::default();
            }
            // An editor may explicitly consume Tab (for indentation). Only
            // an unhandled key reaches the existing focus traversal fallback.
            if let Some(response) = self
                .focused
                .and_then(|id| self.root.find_mut(id))
                .map(|c| c.deliver_actions(event, false, &mut self.actions))
                && response.consumed
            {
                return response;
            }
            let mut ids = Vec::new();
            self.scope().focus_ids(&mut ids);
            if ids.is_empty() {
                return ContentResponse::default();
            }
            let next = match self
                .focused
                .and_then(|id| ids.iter().position(|v| *v == id))
            {
                Some(i) if *reverse => (i + ids.len() - 1) % ids.len(),
                Some(i) => (i + 1) % ids.len(),
                None if *reverse => ids.len() - 1,
                None => 0,
            };
            let changed = self.focus(Some(ids[next]), true);
            return ContentResponse::consumed(changed);
        }
        if matches!(routed, ContentEvent::PointerMove(_)) {
            let (scope, actions) = self.scope_with_actions();
            scope.move_pointer(event, actions);
            return ContentResponse::consumed(true);
        }
        let result = match routed {
            ContentEvent::PointerUp(p)
            | ContentEvent::PointerDown(p)
            | ContentEvent::Scroll { position: p, .. } => {
                let (scope, actions) = self.scope_with_actions();
                scope.route(event, *p, actions)
            }
            _ => self.focused.and_then(|id| {
                self.root
                    .find_mut(id)
                    .map(|c| (id, c.deliver_actions(event, false, &mut self.actions)))
            }),
        };
        if let Some((id, r)) = result {
            if matches!(routed, ContentEvent::PointerDown(_)) && event.is_primary_pointer() {
                if r.capture_pointer {
                    self.captured = Some(id)
                }
                if self.root.find(id).is_some_and(|c| c.component.focusable()) {
                    self.focus(Some(id), false);
                }
            }
            r
        } else {
            if self.modal.is_some() {
                ContentResponse::consumed(false)
            } else {
                ContentResponse::default()
            }
        }
    }
    fn scope(&self) -> &Content {
        self.modal
            .and_then(|(id, _)| self.root.find(id))
            .unwrap_or(&self.root)
    }
    fn scope_with_actions(&mut self) -> (&mut Content, &mut Option<Vec<actions::PendingAction>>) {
        let scope = if let Some((id, _)) = self.modal {
            self.root.find_mut(id).expect("mounted modal")
        } else {
            &mut self.root
        };
        (scope, &mut self.actions)
    }
    /// Current UTF-16 editing/geometry source for a platform's text query bridge.
    pub fn focused_text(&self) -> Option<&crate::TextSurface> {
        self.focused.and_then(|id| self.text_surface(id))
    }
    /// Published text queries for active semantic owners, including unfocused
    /// fields. Closed or background modal subtrees cannot expose editable text.
    pub fn text_surface(&self, id: ComponentId) -> Option<&crate::TextSurface> {
        self.scope()
            .find_active(id)
            .and_then(|c| c.component.text_surface())
    }
    /// Actions use the same semantic owner and production state machine as input.
    /// Closed/dismissed/inactive subtrees and stale menu parts reject requests.
    pub fn accessibility_action(
        &mut self,
        id: ComponentId,
        part: Option<usize>,
        action: AccessibilityAction,
    ) -> ContentResponse {
        self.reconcile_active_inputs();
        if self
            .scope()
            .find_active(id)
            .is_none_or(|n| n.component.semantics().disabled)
        {
            return ContentResponse::default();
        }
        if part.is_none()
            && matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::Press
            )
            && self
                .scope()
                .find_active(id)
                .is_some_and(|n| n.component.focusable())
        {
            let changed = self.focus(Some(id), true);
            if matches!(action, AccessibilityAction::Focus) {
                return ContentResponse::consumed(changed);
            }
        }
        let owner = self
            .root
            .find_mut(id)
            .expect("validated active semantic owner");
        let result = owner.component.accessibility_action(part, &action);
        owner.record_actions(
            &result,
            ContentActionSource::Accessibility,
            true,
            &mut self.actions,
        );
        if result.consumed && matches!(action, AccessibilityAction::SetSelection(_)) {
            self.focus(Some(id), true);
        }
        self.reconcile_active_inputs();
        self.dirty |= result.changed;
        result
    }
    pub fn show_dialog(&mut self, id: ComponentId) -> Result<(), CompositionError> {
        if self.modal.is_some() {
            return Err(CompositionError::ModalAlreadyOpen);
        }
        let dialog = self
            .root
            .find_mut(id)
            .and_then(|c| c.component_mut::<DialogContent>())
            .ok_or(CompositionError::NotDialog(id))?;
        dialog.model.show(0);
        if let Some(previous) = self.presented_dialog.filter(|old| *old != id)
            && let Some(dialog) = self
                .root
                .find_mut(previous)
                .and_then(|n| n.component_mut::<DialogContent>())
        {
            dialog.clear_presentation();
        }
        self.presented_dialog = Some(id);
        let restore = self.focused;
        self.cancel();
        self.modal = Some((id, restore));
        self.modal_focus_pending = true;
        Ok(())
    }
    /// Discard paint retained across dismissal before replacing atlas/backend resources.
    /// Semantic open/focus ownership is unchanged.
    pub fn clear_dialog_presentation(&mut self) {
        if let Some(id) = self.presented_dialog
            && let Some(dialog) = self
                .root
                .find_mut(id)
                .and_then(|n| n.component_mut::<DialogContent>())
        {
            dialog.clear_presentation();
        }
        self.dirty = true;
    }
    pub fn close_dialog(&mut self) -> bool {
        let Some((id, restore)) = self.modal else {
            return false;
        };
        self.cancel();
        if let Some(dialog) = self
            .root
            .find_mut(id)
            .and_then(|c| c.component_mut::<DialogContent>())
        {
            dialog.model.close();
        }
        self.modal = None;
        self.focus(restore, true);
        true
    }
    pub fn cancel(&mut self) {
        self.dirty = true;
        self.modal_focus_pending = false;
        fn cancel(c: &mut Content) {
            c.component.event(&ContentEvent::Cancel);
            if let Some(children) = c.component.children_mut() {
                for c in children {
                    cancel(c)
                }
            }
        }
        cancel(&mut self.root);
        self.captured = None;
        self.focus(None, false);
    }
    pub fn insert(&mut self, parent: ComponentId, child: Content) -> Result<(), CompositionError> {
        if let Some(action) = self.root.action_ancestor(parent)
            && let Some(child) = child.interactive_id()
        {
            return Err(CompositionError::NestedInteractive { action, child });
        }
        let mut ids = BTreeSet::new();
        self.root.ids(&mut ids)?;
        child.ids(&mut ids)?;
        let node = self
            .root
            .find_mut(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if node.component.fixed_slots() || node.component.semantics().role == ContentRole::Button {
            return Err(CompositionError::NotContainer(parent));
        }
        let children = node
            .component
            .children_mut()
            .ok_or(CompositionError::NotContainer(parent))?;
        let mut child = child;
        child.lifecycle(Lifecycle::Mounted);
        children.push(child);
        self.dirty = true;
        Ok(())
    }
    pub fn remove(&mut self, id: ComponentId) -> Result<Content, CompositionError> {
        // Check the actual mutable child boundary before dismissing a modal.
        // Root/fixed-slot/read-only containers cannot be removed; an error must
        // preserve the current scope, focus and retained presentation.
        fn can_remove(node: &mut Content, id: ComponentId) -> bool {
            let fixed = node.component.fixed_slots()
                || node.component.semantics().role == ContentRole::Button;
            let Some(children) = node.component.children_mut() else {
                return false;
            };
            if children.iter().any(|child| child.id == id) {
                return !fixed;
            }
            children.iter_mut().any(|child| can_remove(child, id))
        }
        if !can_remove(&mut self.root, id) {
            return Err(CompositionError::MissingId(id));
        }
        if self
            .modal
            .is_some_and(|(modal, _)| self.root.find(id).is_some_and(|n| n.find(modal).is_some()))
        {
            self.close_dialog();
        }
        fn remove(node: &mut Content, id: ComponentId) -> Option<Content> {
            let action = node.component.fixed_slots()
                || node.component.semantics().role == ContentRole::Button;
            let children = node.component.children_mut()?;
            if let Some(i) = children.iter().position(|c| c.id == id).filter(|_| !action) {
                return Some(children.remove(i));
            }
            children.iter_mut().find_map(|c| remove(c, id))
        }
        let mut child = remove(&mut self.root, id).ok_or(CompositionError::MissingId(id))?;
        if self.focused.is_some_and(|id| child.find(id).is_some()) {
            self.focused = None;
        }
        if self.captured.is_some_and(|id| child.find(id).is_some()) {
            self.captured = None;
        }
        child.lifecycle(Lifecycle::Unmounted);
        self.dirty = true;
        Ok(child)
    }
    /// Validate the whole order before moving any child; keys retain owned state.
    pub fn reorder(
        &mut self,
        parent: ComponentId,
        order: &[ComponentId],
    ) -> Result<(), CompositionError> {
        let node = self
            .root
            .find_mut(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if node.component.fixed_slots() || node.component.semantics().role == ContentRole::Button {
            return Err(CompositionError::NotContainer(parent));
        }
        let children = node
            .component
            .children_mut()
            .ok_or(CompositionError::NotContainer(parent))?;
        let actual: BTreeSet<_> = children.iter().map(|c| c.id).collect();
        if order.len() != children.len() || order.iter().copied().collect::<BTreeSet<_>>() != actual
        {
            return Err(CompositionError::InvalidOrder);
        }
        let mut old = std::mem::take(children);
        for id in order {
            let i = old.iter().position(|c| c.id == *id).unwrap();
            children.push(old.remove(i));
        }
        self.dirty = true;
        Ok(())
    }
    /// Current range metadata for an active semantic owner. Hidden/removed
    /// content cannot publish a stale adjustable value domain.
    pub fn value_range(&self, id: ComponentId) -> Option<std::ops::RangeInclusive<f64>> {
        self.scope()
            .find_active(id)
            .and_then(|n| n.component.value_range())
            .filter(|range| {
                range.start().is_finite() && range.end().is_finite() && range.start() <= range.end()
            })
    }
    /// Actual mounted tree projection; this is not a platform accessibility bridge.
    pub fn semantics(&self) -> Vec<SemanticNode> {
        let mut nodes = Vec::new();
        self.scope().semantic_nodes(None, self.focused, &mut nodes);
        nodes
    }
}
impl Drop for ContentHost {
    fn drop(&mut self) {
        self.root.lifecycle(Lifecycle::Unmounted)
    }
}

#[derive(Debug)]
/// Intrinsic stack: children keep measured main-axis sizes. Like existing
/// layout primitives it does not introduce a viewport; use ScrollContent or
/// the host clip to bound overflow. No implicit flex/shrink or per-child frames.
pub struct StackContent {
    pub model: Stack,
    pub children: Vec<Content>,
    /// One child is remeasured in the main-axis space left by fixed siblings.
    /// This expresses e.g. wrapping text beside an intrinsic icon and badge.
    pub flexible_child: Option<ComponentId>,
}
impl StackContent {
    fn measurements(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> Vec<MeasuredChild> {
        let mut measured: Vec<_> = self
            .children
            .iter_mut()
            .map(|c| c.measure(text, available))
            .collect();
        if let Some(i) = self
            .flexible_child
            .and_then(|id| self.children.iter().position(|c| c.id == id))
        {
            let horizontal = self.model.axis == Axis::Horizontal;
            let fixed = measured
                .iter()
                .enumerate()
                .filter(|(n, _)| *n != i)
                .map(|(_, m)| {
                    if horizontal {
                        m.size.width
                    } else {
                        m.size.height
                    }
                })
                .sum::<f32>();
            let left = ((if horizontal {
                available.width
            } else {
                available.height
            }) - fixed
                - finite(self.model.gap) * self.children.len().saturating_sub(1) as f32)
                .max(0.);
            let constrained = if horizontal {
                Size::new(left, available.height)
            } else {
                Size::new(available.width, left)
            };
            measured[i] = self.children[i].measure(text, constrained);
        }
        measured
    }
}
impl Component for StackContent {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let horizontal = self.model.axis == Axis::Horizontal;
        let mut main = 0_f32;
        let mut cross = 0_f32;
        for m in self.measurements(text, available) {
            let m = m.size;
            main += (if horizontal { m.width } else { m.height }).min(1e8);
            cross = cross.max(if horizontal { m.height } else { m.width });
        }
        main += finite(self.model.gap) * self.children.len().saturating_sub(1) as f32;
        MeasuredChild::new(if horizontal {
            Size::new(main, cross)
        } else {
            Size::new(cross, main)
        })
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let measured = self.measurements(text, frame.size);
        let placements = layout_measured_children(
            frame,
            self.model.axis,
            &measured,
            self.model.gap,
            if self.model.axis == Axis::Horizontal {
                proposal::layout::ROW
            } else {
                proposal::layout::COLUMN
            },
            AlignmentOverride::default(),
        );
        for (c, p) in self.children.iter_mut().zip(placements) {
            c.arrange(text, p.frame, clip)
        }
    }
}
#[derive(Debug)]
pub struct GridContent {
    pub model: Grid,
    pub children: Vec<Content>,
}
impl Component for GridContent {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let columns = self.model.columns.max(1);
        let cells = self.model.layout(
            Rect::new(0., 0., available.width, available.height),
            self.children.len(),
            0.,
        );
        let mut rows = vec![0_f32; self.children.len().div_ceil(columns)];
        for (i, (c, cell)) in self.children.iter_mut().zip(cells).enumerate() {
            rows[i / columns] = rows[i / columns].max(
                c.measure(text, Size::new(cell.size.width, available.height))
                    .size
                    .height,
            )
        }
        MeasuredChild::new(Size::new(
            available.width,
            rows.iter().sum::<f32>()
                + finite(self.model.gap_y) * rows.len().saturating_sub(1) as f32,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let columns = self.model.columns.max(1);
        let cells = self.model.layout(frame, self.children.len(), 0.);
        let mut heights = vec![0_f32; self.children.len().div_ceil(columns)];
        for (i, (c, cell)) in self.children.iter_mut().zip(&cells).enumerate() {
            heights[i / columns] = heights[i / columns].max(
                c.measure(text, Size::new(cell.size.width, frame.size.height))
                    .size
                    .height,
            )
        }
        let mut y = frame.min_y();
        for (i, (c, cell)) in self.children.iter_mut().zip(cells).enumerate() {
            if i > 0 && i % columns == 0 {
                y += heights[i / columns - 1] + finite(self.model.gap_y)
            }
            c.arrange(
                text,
                Rect::new(cell.min_x(), y, cell.size.width, heights[i / columns]),
                clip,
            )
        }
    }
}
#[derive(Debug)]
pub struct ScrollContent {
    pub scroll: ScrollArea,
    pub viewport_height: Option<f32>,
    backing_scale: f32,
    children: Vec<Content>,
    viewport: Rect,
    last_frame: Option<std::time::Duration>,
}
impl ScrollContent {
    pub fn new(scroll: ScrollArea, child: Content) -> Self {
        Self {
            scroll,
            viewport_height: None,
            backing_scale: 1.,
            children: vec![child],
            viewport: Rect::default(),
            last_frame: None,
        }
    }
}
impl Component for ScrollContent {
    fn interactive(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(
            available.width,
            self.viewport_height
                .map_or(available.height, |h| finite(h).min(available.height)),
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.backing_scale = text.backing_scale();
        if let Some(m) = text.motion_context() {
            self.scroll.set_reduced_motion(m.reduced_motion);
            if let Some(previous) = self.last_frame {
                self.scroll
                    .tick(m.now.saturating_sub(previous).as_secs_f32());
            }
            self.last_frame = Some(m.now);
            if self.scroll.needs_animation() {
                text.request_animation_frame();
            }
        }
        self.viewport = frame;
        // Scroll is a viewport: reserve the existing ControlFrame focus bleed
        // in document geometry, rather than relaxing the ancestor clip.
        let focus_bounds =
            crate::extended::ControlFrame::new(frame, None).focus_bounds(self.backing_scale);
        let gutter = (frame.min_x() - focus_bounds.min_x()).max(0.0);
        let width = (frame.size.width - 2.0 * gutter).max(0.0);
        let available = Size::new(width, 1e8);
        let mut height = 2.0 * gutter;
        for c in &mut self.children {
            height += c.measure(text, available).size.height;
        }
        self.scroll
            .layout(frame.size, Size::new(frame.size.width, height));
        let geometry = self.scroll.geometry(frame, Some(clip));
        let origin = geometry.content_origin();
        let child_clip = geometry.visible_viewport_rect().unwrap_or_default();
        let mut y = origin.y + gutter;
        for c in &mut self.children {
            let h = c.measure(text, available).size.height;
            c.arrange(text, Rect::new(origin.x + gutter, y, width, h), child_clip);
            y += h;
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::Scroll {
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
                ..
            } => {
                let changed = self
                    .scroll
                    .wheel_at(*delta, *precise, *begin, *end, *momentum, *time);
                ContentResponse {
                    consumed: changed,
                    changed,
                    ..Default::default()
                }
            }
            ContentEvent::Cancel => {
                self.scroll.cancel();
                ContentResponse::default()
            }
            _ => ContentResponse::default(),
        }
    }
    fn reveal(&mut self, descendant: Rect) -> Rect {
        let g = self.scroll.geometry(self.viewport, None);
        let origin = g.content_origin();
        let old = g.visual_offset();
        self.scroll
            .reveal_focus(descendant.offset(-origin.x, -origin.y), self.backing_scale);
        let new = self.scroll.visual_offset();
        descendant.offset(old.x - new.x, old.y - new.y)
    }
}

/// The forwarding value keeps existing sized renderer APIs source-compatible.
struct TextAdapter<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for TextAdapter<'_> {
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.0.text_surface_host()
    }
    fn measure(&mut self, t: &str, r: proposal::TextRole, w: f32) -> Size {
        self.0.measure(t, r, w)
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
        self.0.draw(s, t, r, f, a, i)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.0.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.0.request_animation_frame()
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn first_line_height(&mut self, t: &str, r: proposal::TextRole, w: f32) -> f32 {
        self.0.first_line_height(t, r, w)
    }
}
/// Real Button model/painter; composition does not duplicate press or key logic.
#[derive(Debug)]
pub struct ButtonContent {
    pub button: Button,
    pub label: String,
    pub variant: ButtonVariant,
    pub activations: usize,
}
impl ButtonContent {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            button: Button::default(),
            label: label.into(),
            variant: ButtonVariant::Quiet,
            activations: 0,
        }
    }
}
impl Component for ButtonContent {
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || !self.focusable() {
            return ContentResponse::default();
        }
        self.button.pointer_down();
        let activated = self.button.pointer_up(true);
        if activated {
            self.activations += 1;
        }
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Button::measure(
            &mut TextAdapter(text),
            &self.label,
            available.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.button.render(
            scene,
            &mut TextAdapter(text),
            &self.label,
            frame,
            Some(clip),
            theme.palette,
            self.variant,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
        );
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
        let mut r = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(p) => {
                self.button.pointer_move_at(*p);
                r.changed = true
            }
            ContentEvent::PointerDown(p) => {
                r.consumed = self.button.pointer_down_at(*p);
                r.capture_pointer = r.consumed;
                r.changed = r.consumed
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
        let mut s = ContentSemantics::new(ContentRole::Button, &self.label);
        s.disabled = self.button.state.disabled || self.button.state.loading;
        s.selected = self.button.state.selected;
        s
    }
}
#[derive(Debug)]
pub struct TextContent {
    pub text: String,
    pub role: proposal::TextRole,
}
impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            role: proposal::TextRole::UiBody,
        }
    }
}
impl Component for TextContent {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(text.measure(&self.text, self.role, available.width))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        text.draw(
            scene,
            &self.text,
            self.role,
            frame,
            ButtonTextAlign::Leading,
            theme
                .foreground
                .unwrap_or_else(|| theme.palette.color("primary-text").expect("text role")),
        )
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Text, &self.text)
    }
}
fn finite(v: f32) -> f32 {
    if v.is_finite() { v.clamp(0., 1e8) } else { 0. }
}
fn clean_size(s: Size) -> Size {
    Size::new(finite(s.width), finite(s.height))
}
fn clean_rect(r: Rect) -> Rect {
    if !r.min_x().is_finite() || !r.min_y().is_finite() {
        return Rect::default();
    }
    Rect::from_origin_size(r.origin, clean_size(r.size))
}

/// One production Button action with an arbitrary decorative component subtree.
/// The accessible name is explicit; child names are not concatenated or exported
/// as independent accessibility nodes. Interactive descendants are diagnosed by
/// ContentHost before mounting, painting or dispatching input.
#[derive(Debug)]
pub struct CompoundButton {
    pub action: ButtonContent,
    content: Vec<Content>,
    measured: Size,
}
impl CompoundButton {
    pub fn new(
        accessible_name: impl Into<String>,
        content: Content,
    ) -> Result<Self, CompositionError> {
        let name = accessible_name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(Self {
            action: ButtonContent::new(name),
            content: vec![content],
            measured: Size::default(),
        })
    }
    fn resolve(&self, frame: Rect, clip: Rect) -> crate::ButtonLayout {
        crate::ButtonLayout::resolve(
            frame,
            Some(clip),
            self.measured,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
        )
    }
}
impl Component for CompoundButton {
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.action.accessibility_action(part, action)
    }

    fn fixed_slots(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.content.len() != 1 {
            return Err(CompositionError::InvalidSlotCount);
        }
        if self.action.label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn children(&self) -> &[Content] {
        &self.content
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.content)
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        let role = if self.action.button.state.disabled {
            "tertiary-text"
        } else if self.action.variant == ButtonVariant::Primary {
            "action-primary-foreground"
        } else {
            "primary-text"
        };
        ContentTheme {
            palette: theme.palette,
            foreground: theme.palette.color(role),
        }
    }
    // A Button has one fixed content slot, so arbitrary sibling insertion is
    // intentionally unavailable. Its subtree can use normal container slots.
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let frame = Rect::from_origin_size(Point::default(), available);
        let empty = crate::ButtonLayout::resolve(
            frame,
            None,
            Size::default(),
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
        );
        self.measured = self.content[0]
            .measure(text, Size::new(empty.content.size.width, available.height))
            .size;
        let layout = self.resolve(Rect::new(0., 0., available.width, 0.), frame);
        MeasuredChild::new(Size::new(
            (self.measured.width + available.width - empty.content.size.width).min(available.width),
            layout.frame.size.height,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.measure(text, frame.size);
        let layout = self.resolve(frame, clip);
        let child_clip = layout.content.intersection(&clip).unwrap_or_default();
        self.content[0].arrange(text, layout.content, child_clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.action.button.render_content_frame(
            scene,
            &mut TextAdapter(text),
            self.measured,
            frame,
            Some(clip),
            theme.palette,
            self.action.variant,
        );
    }
    fn focusable(&self) -> bool {
        self.action.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.action.set_focus(f, v)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.action.event(e)
    }
    fn semantics(&self) -> ContentSemantics {
        self.action.semantics()
    }
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composition contract violation: {self:?}")
    }
}
impl std::error::Error for CompositionError {}

fn scene_dialog_contents(
    node: &mut Content,
    scene: &mut Scene,
    text: &mut dyn ButtonTextRenderer,
    theme: ContentTheme,
) {
    scene.push_clip(node.clip);
    node.component
        .paint(scene, text, node.frame, node.clip, theme);
    scene.pop_clip();
    if let Some(children) = node.component.children_mut() {
        for child in children {
            child.paint(scene, text, theme);
            child.paint_overlays(scene, text, theme);
        }
    }
}
