//! EXPERIMENTAL: minimum-scenario protocol; not yet approved for broad migration.
//!
//! Owned component composition over the existing scene, text, layout and scroll APIs.
//!
//! Keys identify owned instances, not positions. Reordering moves instances; it
//! never reconstructs controls. Removed content is returned to its owner, which
//! may retain it for virtualization. No implicit serialization or rehydration.
#[cfg(feature = "bitmap-images")]
mod bitmap;
#[cfg(feature = "bitmap-images")]
pub use bitmap::{
    ImageAlignment, ImageContent, ImageContentError, ImageFit, ImageLoadOutcome, ImageLoadRequest,
    ImageLoadStatus, ImageSemantics,
};
mod icon;
pub use icon::{IconContent, IconImage, IconRenderError, WithIconRenderer};
#[cfg(feature = "svg-icons")]
pub use icon::{SvgIconCache, SvgIconRenderer};
mod disclosure;
pub use disclosure::{DisclosureContent, DisclosureIntent};
mod actions;
mod panel;
pub use panel::{
    ContentGeometrySnapshot, CurveGeometryContext, CurveSubtreeCapability, HostRevealGeometry,
    PanelContent, PanelGeometry,
};
mod split_view;
pub use split_view::{
    Rejected, SplitViewConfig, SplitViewConstruction, SplitViewContent, SplitViewError,
    SplitViewExpected, SplitViewReplacement, SplitViewSlotIds,
};
mod prepared;
pub use prepared::{
    PreparedEffects, PreparedEffectsError, PreparedHook, PreparedHookKind,
    PreparedLifecycleContext, PreparedPhase,
};
mod action_proxy;
mod notice;
pub use notice::{BannerContent, ToastContent};
mod empty_state;
pub use empty_state::EmptyStateContent;
mod action_preparation;
pub use action_preparation::{
    ActionPreparation, ActionTransactionCompatibility, ComponentActionRequest,
    PreparedActionEffect, PreparedActionFocus, PreparedComponentAction,
};
mod content_reveal_plan;
pub use content_reveal_plan::{
    HostRevealBudget, HostRevealError, HostRevealOutcome, HostRevealPlan,
};
mod rich_content;
mod rich_document;
pub use rich_content::{
    RichBlockGeometry, RichContent, RichContentError, RichContentVersion, RichLinkAction,
};
pub use rich_document::{RichBlock, RichBlockKind, RichDocument, RichLink};
mod adapters;
mod chart_content;
pub use chart_content::{ChartContent, ChartContentError};
mod button;
pub use button::{Button, ButtonActivationPhase, ButtonContents, ButtonImagePosition};
use button::{action_child_theme, measure_action_slot, resolve_action_slot};
mod content_reading;
mod content_reveal;
mod status_badge;
pub use status_badge::StatusBadgeContent;
mod separator;
pub use separator::SeparatorContent;
mod skeleton;
pub use skeleton::SkeletonContent;
mod progress;
pub use progress::ProgressContent;
mod help;
mod key;
mod label;
pub use tekes_ui_text::{
    LabelForeground, LabelForegroundRun, LabelSourceVersion, SemanticLabel, SemanticLabelError,
};
mod menu;
mod number;
pub use number::{
    DecimalNumberFormat, NumberCommitReason, NumberEditState, NumberFieldContent, NumberFormat,
    NumberParseError, NumberValidation, NumberValueChange, NumberValueError,
};
mod overlay_scope;
mod popover;
pub use popover::{
    HoverPreviewContent, HoverPreviewError, HoverPreviewVersion, PopoverCloseReason, PopoverClosed,
    PopoverContent, PopoverSession,
};
mod pointer;
mod text_query;
pub use overlay_scope::OverlayScope;
mod toolbar_content;
mod toolbar_host;
use crate::extended::{Axis, Grid, Stack};
use crate::layout::{AlignmentOverride, MeasuredChild, layout_measured_children};
use crate::paint::ButtonVariant;
use crate::{
    ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext, ScrollArea,
};
pub use actions::{
    ContentAction, ContentActionKind, ContentActionPayload, ContentActionRecord,
    ContentActionSource, MenuSelection,
};
pub use adapters::{
    DialogContent, ListContent, SliderContent, SwitchContent, TextFieldContent, TextViewport,
};
pub use content_reading::{
    ContentReadingAnchorUpdate, ContentReadingCheckpoint, ContentReadingCheckpointError,
    ContentReadingPosition, ContentReadingRestoreRequest, ContentReadingResult,
};
pub use content_reveal::{ContentRevealError, ContentRevealResult, ContentRevealStatus};
pub use key::{
    ContentKeyEvent, TextKeyDisposition, TextKeyOutcome, TextSubmitPolicy, TextTabPolicy,
};
pub(crate) use menu::MenuContentState;
pub use menu::{
    ContextMenuBinding, ContextMenuError, ContextMenuRequest, MenuContent, MenuEntriesError,
    MenuEntry, MenuValuesError,
};
pub use pointer::{InputModifiers, PointerButton, PointerEvent, PointerModifiers, PointerPhase};
use std::{
    any::Any,
    collections::BTreeSet,
    fmt::Debug,
    ops::{Range, RangeInclusive},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, proposal};
pub use text_query::{ContentTextInput, ContentTextLayout};
pub use toolbar_content::{ToolbarContent, ToolbarGroup, ToolbarPlacement};
pub use toolbar_host::{
    ToolbarAccessibilityTarget, ToolbarButtonDeclaration, ToolbarCommandBinding,
    ToolbarGroupOverflow, ToolbarHost, ToolbarInvocation, ToolbarRegion, ToolbarRegionScope,
    ToolbarRegistration, ToolbarSemanticNode, ToolbarWindowEdge, toolbar_commands,
};

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
    /// At least one recipient handled the event; propagation is independent.
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
    /// Passive numeric progress; never an adjustable action owner.
    ProgressIndicator,
    Text,
    /// Noninteractive image; nonempty name is alternative text, empty is explicitly decorative.
    Image,
    Button,
    /// A navigation action with one semantic owner; its children are decorative.
    Link,
    TextInput,
    Slider,
    Switch,
    List,
    ListItem,
    Dialog,
    MenuButton,
    Menu,
    MenuItem,
    /// The original visible split divider, with its finite ratio domain.
    Splitter,
}
impl ContentRole {
    fn is_action_owner(self) -> bool {
        matches!(self, Self::Button | Self::Link | Self::MenuButton)
    }
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
    /// Declarative required-content metadata, independent of availability or validation.
    pub required: bool,
    pub description: Option<String>,
}
impl ContentSemantics {
    /// Construct an informative image semantic; blank alternative text is invalid.
    pub fn image_alt(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        (!name.trim().is_empty()).then(|| Self::new(ContentRole::Image, name))
    }
    pub fn decorative_image() -> Self {
        Self::new(ContentRole::Image, "")
    }
    pub fn new(role: ContentRole, name: impl Into<String>) -> Self {
        Self {
            role,
            name: name.into(),
            value: None,
            disabled: false,
            selected: false,
            expanded: None,
            invalid: false,
            required: false,
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
    /// Show the original owner's bound context menu without a primary action.
    ShowMenu,
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
    pub icon_foreground: Option<Rgba>,
    surface: &'static str,
    surface_declared: bool,
}
impl ContentTheme {
    pub fn surface(self) -> &'static str {
        self.surface
    }
    pub fn with_surface(
        mut self,
        name: &str,
    ) -> Result<Self, tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        self.surface_declared = true;
        self.foreground = None;
        self.icon_foreground = None;
        Ok(self)
    }
}
impl From<Palette> for ContentTheme {
    fn from(palette: Palette) -> Self {
        Self {
            palette,
            foreground: None,
            icon_foreground: None,
            surface: "canvas",
            surface_declared: false,
        }
    }
}

/// Consumer-extensible component protocol. The object owns its existing model
/// and view. Containers expose the same Content children as library containers.
/// Geometry is in host coordinates; `clip` is the actual ancestor viewport,
/// never an implicit clip to a control's own frame (including its focus ring).
/// `measure` is read-layout work, `arrange` places descendants, `paint` may use
/// existing render APIs. Hosts call render before input after geometry changes.
/// Opt-in contract required recursively by typed collection transactions.
/// StableHooks promises Cancel, focus, lifecycle, activity, mutable child access
/// (including window_active_changed and presentation_runnable_changed, their
/// forwarding and actually notified descendants),
/// and Drop neither allocate Content/mount identities, change the declared tree,
/// nor reenter component/Host updates. The mutable children getter itself is
/// accessor-only: it returns the existing vector without other observable state
/// changes. This matters on ancestor paths accessed before reservation.
/// Pure queries remain side-effect free. This is a
/// semantic contract, not protection against arbitrary panics or violations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableTransactionCompatibility {
    Unknown,
    StableHooks,
}
pub trait Component: Debug + Any {
    /// Opt in to the original Host's exact curve-clip geometry. Unknown owners
    /// remain rejected inside a curved Panel subtree.
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Unsupported
    }
    /// Opt in to ordinary input using the current Host rectangular allocation
    /// and clip. This grants no curved Panel, prepared-effect or IPC transaction
    /// capability; unknown components remain unavailable on strict Scroll paths.
    fn rectangular_input_geometry(&self) -> bool {
        false
    }
    /// Existing controls receive the same event; this adds exact clip-region
    /// membership to their original reducer without creating another owner.
    fn set_clip_region(&mut self, _region: Option<tekes_ui_scene::ClipRegion>) {}
    /// Current immutable geometry projected by the actual owning Host. This
    /// conveys no permission by itself and expires with its generation.
    fn set_curve_geometry_context(&mut self, _context: Option<CurveGeometryContext>) {}
    /// The original delegated Content owner admits its own focus under this
    /// Host's exact external curve chain. This is logical keyboard ownership,
    /// not permission to translate an ancestor Scroll or bypass current input.
    /// Ordinary dispatch additionally requires current geometry and the
    /// component's input-ready opt-in; unknown owners retain the default.
    fn delegates_curve_focus(&self) -> bool {
        false
    }
    fn pointer_event_in_region(&mut self, event: &ContentEvent, _inside: bool) -> ContentResponse {
        match event {
            ContentEvent::Pointer(pointer) => self.pointer_event(pointer),
            _ => self.event(event),
        }
    }

    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::Unknown
    }
    /// Pure finite preparation for this owner's complete ordered hook sequence.
    /// Default unavailable: StableHooks alone does not promise eligibility purity.
    /// A compliant wrapper delegates preparation and lifecycle context to the same
    /// retained original owner; refusal occurs before notification side effects.
    fn prepared_effects_work(&self, _max_calls: usize) -> Result<usize, PreparedEffectsError> {
        Err(PreparedEffectsError::Unavailable)
    }
    fn prepare_effects(
        &self,
        _hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Err(PreparedEffectsError::Unavailable)
    }
    fn lifecycle_prepared(&mut self, event: Lifecycle, context: &mut PreparedLifecycleContext) {
        context.original_lifecycle(self, event);
    }
    fn participates_in_focus_traversal(&self) -> bool {
        true
    }
    fn semantic_subtree_hidden(&self) -> bool {
        false
    }
    /// Optional component-owned help text and revision; Content declarations override it.
    fn help(&self) -> Option<(&str, u64)> {
        None
    }

    /// The read-only focus/reveal planner consumes this original-owner geometry
    /// constraint and revalidates its generation before commit.
    fn requires_focus_reveal(&self) -> bool {
        false
    }
    fn host_reveal_geometry_constraint(&self) -> Option<HostRevealGeometry> {
        None
    }
    fn required_focus_space(&self) -> RequiredFocusSpace {
        RequiredFocusSpace::Legacy
    }
    fn required_focus_result(&self) -> Option<RequiredFocusResult> {
        None
    }
    fn focus_boundary_visible(&self) -> bool {
        self.focus_decoration_above_siblings()
    }

    /// Pure positive translation capability for the most recent valid arrange.
    /// Child identity/order, relative frames and local clip remain invariant under
    /// ancestor Scroll translation. Return false when unarranged or requiring reflow.
    /// Indexed mutable child access is accessor-only; this is not a runtime sandbox.
    fn host_reveal_passthrough(&self) -> bool {
        false
    }
    /// Additional local rectangular clip in Host coordinates. None is no extra
    /// clip; Some zero area is an empty domain. Finite safe-edge numeric input is
    /// checked during planning and final validation before any effects.
    fn host_reveal_local_clip(&self) -> Option<Rect> {
        None
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::Unsupported
    }
    fn prepare_action(&self, _request: ComponentActionRequest<'_>) -> ActionPreparation {
        ActionPreparation::Unsupported
    }
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
    /// Snapshot optional data from this originating delivery, never by polling
    /// later. Wrappers delegating actions forward this to their original owner.
    fn action_payload(&self, _kind: ContentActionKind) -> Option<ContentActionPayload> {
        None
    }
    /// Pure filter for an undrained delivery. Defaults preserve existing owners.
    /// Implementations must not mutate models, consume queues or produce actions.
    fn action_payload_current(&self, _payload: &ContentActionPayload) -> bool {
        true
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
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.measure(text, available)
    }
    /// Check semantic slot constraints before a host mounts or uses this node.
    fn validate(&self) -> Result<(), CompositionError> {
        Ok(())
    }
    /// Closed semantic subtrees retain owned state but do not paint or receive input.
    /// Layout may update activation. ContentHost revalidates text capabilities
    /// after measure/arrange; native layout publication requires a successful render.
    fn active(&self) -> bool {
        true
    }
    /// Activity notification, including inactive retained descendants. Notice
    /// owners receive window + ancestor activity; other owners retain the #78
    /// effective self/ancestor activity contract. Own notice visibility is runnable state.
    fn window_active_changed(&mut self, _active: bool) {}
    /// Effective active ancestry and this node's arranged nonempty visible clip.
    fn presentation_runnable_changed(&mut self, _runnable: bool) {}
    fn fixed_slots(&self) -> bool {
        false
    }
    /// Opt in when a component paints its owned child Content using the public
    /// subtree paint methods, for example inside an existing overlay callback.
    /// Layout, lifecycle, input, focus and semantics still use this same tree.
    /// The host then skips automatic child paint in both ordinary/overlay passes.
    fn paints_children(&self) -> bool {
        false
    }
    /// Whether this original component currently needs its keyboard focus
    /// decoration above adjacent sibling chrome in ordinary Content painting.
    /// The host paints this same branch last; layout, hit/AX order and overlays
    /// are unchanged. A wrapper delegates this method to its retained control,
    /// rather than copying focus, frame-policy or availability predicates.
    /// Components owning child paint order retain that responsibility.
    fn focus_decoration_above_siblings(&self) -> bool {
        false
    }
    /// This owner paints canonical icon resources through the host service.
    fn requires_icon_services(&self) -> bool {
        false
    }
    /// Latest explicit icon resource/backend failure for this original owner.
    fn icon_error(&self) -> Option<IconRenderError> {
        None
    }
    fn requires_text_services(&self) -> bool {
        false
    }
    fn text_surface(&self) -> Option<&crate::TextSurface> {
        None
    }
    /// Whether this original owner accepted a wheel that can update reading intent.
    fn reading_scroll_source(&self) -> bool {
        self.text_surface().is_some_and(|surface| {
            surface.input.read_only && !surface.input.disabled && surface.scroll.limit().y > 0.
        })
    }
    /// Complete current shaper output from this retained text owner, without
    /// shaping, focus or editor mutation. Decorative parts have no text owner.
    fn semantic_text_layout(&self) -> Option<ContentTextLayout> {
        self.text_surface()
            .and_then(ContentTextLayout::from_surface)
    }
    fn semantic_part_text_layout(&self, _part: usize) -> Option<ContentTextLayout> {
        None
    }
    /// Original remote owner query/ack delegation; local defaults remain unchanged.
    fn text_input_snapshot(&self) -> Option<ContentTextInput> {
        self.text_surface().map(ContentTextInput::from_surface)
    }
    fn take_text_key_result(
        &mut self,
        _id: u64,
        _binding: Option<&[u8]>,
    ) -> Option<ContentResponse> {
        None
    }
    fn focus_boundary(&mut self, _reverse: bool) -> ContentResponse {
        ContentResponse::default()
    }
    fn delegates_text_keys(&self) -> bool {
        false
    }
    fn accessibility_focus_requested(&self) -> bool {
        false
    }
    fn blocks_event_bubbling(&self, _event: &ContentEvent) -> bool {
        false
    }
    fn semantic_part_value_range(&self, _part: usize) -> Option<std::ops::RangeInclusive<f64>> {
        None
    }

    /// Native key provenance from the retained original input owner, independent
    /// of arranged caret/clip geometry. Host key frames separately validate the
    /// original owner mount and focus epoch against blur/refocus and remount ABA.
    fn native_key_binding(&self) -> Option<Vec<u8>> {
        self.text_surface().map(ContentTextInput::local_binding)
    }
    fn overlay_open(&self) -> bool {
        false
    }
    /// Interactive original Content overlays opt into input/focus/AX scope.
    /// Paint-only exit presentation never acquires this authority.
    fn overlay_scope(&self) -> Option<OverlayScope> {
        None
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
    /// Optional parent among the same owner's semantic parts (nested menus).
    /// Default flat parts and their public struct representation stay unchanged.
    fn semantic_part_parent(&self, _part: usize) -> Option<usize> {
        None
    }
    fn semantic_part_preserves_activation_focus(&self, _part: usize) -> bool {
        false
    }
    fn preserves_activation_focus(&self) -> bool {
        false
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
    /// Hide this semantic node, promoting visible descendants to its parent.
    /// Painting, layout and input ownership remain unchanged.
    fn semantic_hidden(&self) -> bool {
        false
    }
    /// Declare independent interaction, separately from keyboard focus.
    /// Pointer-only consumer controls must override this; passive decorative
    /// content never receives routed pointer events, even outside an action.
    fn interactive(&self) -> bool {
        self.focusable()
            || matches!(
                self.semantics().role,
                ContentRole::Button
                    | ContentRole::Link
                    | ContentRole::TextInput
                    | ContentRole::Slider
                    | ContentRole::Switch
                    | ContentRole::Splitter
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
    /// Pointer/hover ownership for composition with sibling chrome. Default
    /// uses the same clipped allocation as ordinary Content routing. Compound
    /// controls with internal gaps override using their original hit geometry.
    fn pointer_regions(&self, frame: Rect, clip: Rect) -> Vec<Rect> {
        if self.interactive() {
            frame
                .intersection(&clip)
                .filter(|r| !r.is_empty())
                .into_iter()
                .collect()
        } else {
            Vec::new()
        }
    }
    /// Continue propagation after this component handled an ordinary routed event.
    /// Unhandled events always bubble. Defaults to false; this does not surrender
    /// pointer capture or override IME, focus-traversal, overlay or modal barriers.
    fn bubble_handled_event(&self, _event: &ContentEvent) -> bool {
        false
    }
    /// Optional help for a currently published semantic part. The revision
    /// changes when its explanation changes; part identity and frame continue
    /// to come from semantic_parts(). No new action/focus owner is introduced.
    fn semantic_part_help(&self, _part: usize) -> Option<(&str, u64)> {
        None
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
    /// Default semantic Surface for this component's own paint and children.
    fn default_surface(&self) -> Option<&'static str> {
        None
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
    fn set_focus_from_traversal(&mut self, _reverse: bool) {
        self.set_focus(true, true);
    }
    fn cancel_pointer(&mut self) -> ContentResponse {
        self.event(&ContentEvent::Cancel)
    }
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
    curve_geometry: Option<panel::NodeGeometry>,
    mount: u64,
    preserve_activation_focus: bool,
    bubble_handled_events: Option<bool>,
    padding_focus: Option<PaddingFocusBinding>,
    label: Option<label::LabelState>,
    help: Option<String>,
    help_revision: u64,
    surface: Option<&'static str>,
    // Cached value last notified to this owner; notice and ordinary owner
    // notification semantics differ. This is not a generic activity fact.
    activity_effective: bool,
    activity_parent: bool,
    composition_active: bool,
    retained_body_paint: bool,
}
impl Content {
    fn active(&self) -> bool {
        self.composition_active
            && self.component.active()
            && (self.component.overlay_scope().is_none()
                || self.component.overlay_open() && !self.component.semantics().disabled)
    }
    pub fn new(id: ComponentId, component: impl Component) -> Self {
        Self::new_with_mount(id, component, actions::next_mount())
    }
    fn new_with_mount(id: ComponentId, component: impl Component, mount: u64) -> Self {
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
            activity_effective: true,
            activity_parent: true,
            composition_active: true,
            retained_body_paint: false,
        }
    }
    // The retained body stays mounted. Only ordinary paint may outlive activity.
    fn set_composition_active(&mut self, active: bool) {
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
    fn retain_body_paint(&mut self, retained: bool) {
        self.retained_body_paint = retained;
    }
    fn paint_available(&self) -> bool {
        self.active() || self.retained_body_paint && self.component.active()
    }
    fn paint_services_available(&self) -> bool {
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
    fn resolved_theme(&self, theme: ContentTheme) -> ContentTheme {
        let surface = self
            .surface
            .or_else(|| self.component.default_surface())
            .map(|name| {
                tekes_ui_tokens::surface_palette::canonical_surface(name)
                    .expect("component Surface validated before host paint")
            });
        surface.map_or(theme, |surface| ContentTheme {
            surface,
            surface_declared: true,
            foreground: None,
            icon_foreground: None,
            ..theme
        })
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        let mut child = self.component.child_theme(theme);
        if let Some(surface) = self.surface {
            child.surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)
                .expect("Content Surface validated before host paint");
            child.surface_declared = true;
        }
        child
    }
    fn theme_for(&self, id: ComponentId, theme: ContentTheme) -> Option<ContentTheme> {
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
    fn toolbar_label_layouts(&mut self) {
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
    fn reset_label_layouts(&mut self) {
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
    fn label_failure(&self) -> Option<ComponentId> {
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
    fn routes_pointer_children(&self) -> bool {
        !self.component.semantics().role.is_action_owner()
    }
    fn pointer_target_at(&self, point: Point) -> bool {
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
    fn bubbles(&self, event: &ContentEvent, response: &ContentResponse) -> bool {
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
    fn measure_content(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
        intrinsic: bool,
    ) -> MeasuredChild {
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
        measured.first_baseline = measured.first_baseline.filter(|v| v.is_finite());
        measured
    }
    pub fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.frame = clean_rect(frame);
        self.clip = clean_rect(clip);
        if let Some(label) = &mut self.label {
            let mut renderer = label::LabelRenderer::new(text, label, self.clip);
            self.component.measure(&mut renderer, self.frame.size);
            self.component.arrange(&mut renderer, self.frame, self.clip);
        } else {
            self.component.arrange(text, self.frame, self.clip);
        }
    }
    /// Paint this existing mounted subtree with its arranged frames and clips.
    /// This does not create another input/focus owner or perform layout.
    pub fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
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
        if !self.paint_available() {
            return;
        }
        let curve_scope = self.push_curve_scope(scene);
        let theme = self.resolved_theme(theme);
        // Offscreen ancestors may contain visible overflowing children. Walk
        // children even when their parent frame itself is outside the viewport.
        if self.visible().is_some() {
            scene.push_clip(self.clip);
            if let Some(label) = &mut self.label {
                self.component.paint(
                    scene,
                    &mut label::LabelRenderer::new(text, label, self.clip)
                        .with_palette(theme.palette),
                    self.frame,
                    self.clip,
                    theme,
                );
            } else {
                self.component
                    .paint(scene, text, self.frame, self.clip, theme);
            }
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
    fn has_focus_boundary(&self) -> bool {
        self.active()
            && (self.component.focus_boundary_visible()
                || self
                    .component
                    .children()
                    .iter()
                    .any(Content::has_focus_boundary))
    }
    fn has_raised_focus_decoration(&self) -> bool {
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
        if self.component::<PopoverContent>().is_some() {
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
    fn overlay_id(&self) -> Option<ComponentId> {
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
    fn needs_icons(&self) -> Option<ComponentId> {
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
    fn icon_failure(&self) -> Option<(ComponentId, IconRenderError)> {
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
    fn needs_text(&self) -> Option<ComponentId> {
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
    fn find_active(&self, id: ComponentId) -> Option<&Content> {
        self.find_in_active_ancestors(id)
            .filter(|node| node.active())
    }
    // Exit presentation may retain an inactive Dialog owner, but never bypass
    // an inactive ancestor such as a collapsed or replaced content panel.
    fn find_in_active_ancestors(&self, id: ComponentId) -> Option<&Content> {
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
    fn notify_presentation_runnable(&mut self, ancestor_active: bool) {
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
    fn reconcile_presentation_runnable(&mut self, ancestor_active: bool) {
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
    fn refresh_activity(&mut self, parent_active: bool) {
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
    fn notification_activity(&self, parent: bool) -> bool {
        if self.component::<ToastContent>().is_some() || self.component::<BannerContent>().is_some()
        {
            parent
        } else {
            parent && self.active()
        }
    }
    fn window_active_state(&mut self, parent: bool) -> (bool, bool) {
        self.activity_parent = parent;
        let effective = self.notification_activity(parent);
        let changed = self.activity_effective != effective;
        self.activity_effective = effective;
        (effective, changed)
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.lifecycle_with(event, &mut actions::next_mount);
    }
    fn lifecycle_start(&mut self, event: Lifecycle, mount: u64) {
        self.curve_geometry = None;
        self.component.set_clip_region(None);
        self.component.set_curve_geometry_context(None);
        self.mount = mount;
        if event == Lifecycle::Unmounted {
            self.set_help(None);
        }
    }
    fn lifecycle_with(&mut self, event: Lifecycle, next: &mut impl FnMut() -> u64) {
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
        if self.component.semantics().role.is_action_owner() {
            return Some(self.id);
        }
        self.component
            .children()
            .iter()
            .find_map(|c| c.action_ancestor(target))
    }
    fn ids(&self, ids: &mut BTreeSet<ComponentId>) -> Result<(), CompositionError> {
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
    fn focus_ids(&self, ids: &mut Vec<ComponentId>) {
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
    fn route(
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
    fn finish_bubble(
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
    fn route_from(
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
    // Hit classification is independent of event consumption: even a disabled
    // or currently unhandled real control shields its bounds from a parent's
    // decorative focus area. No coordinates are forwarded to the editor.
    fn padding_focus_at(&self, position: Point) -> PaddingFocusHit {
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
    fn padding_text(&self, id: ComponentId, allow_read_only: bool) -> Option<&Content> {
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
    fn move_pointer(
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
        if !self.active() || self.component.semantic_subtree_hidden() {
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
pub struct RequiredFocusEvidence(std::sync::Arc<()>);
impl RequiredFocusEvidence {
    pub fn new() -> Self {
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
    Sufficient {
        witness: Rect,
        evidence: RequiredFocusEvidence,
    },
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
struct PaddingFocusBinding {
    target: ComponentId,
    mount: u64,
    allow_read_only: bool,
}

enum PaddingFocusHit {
    None,
    Blocked,
    Focus(ComponentId),
}
#[derive(Debug)]
pub struct ContentHost {
    root: Content,
    curve_shapes: Option<tekes_ui_scene::ShapeTable>,
    /// Parent-authorized external geometry constrains this original Host's
    /// input/AX/reveal tree. Its AA clip is deferred to the parent composite.
    external_curve_chain: Option<(u64, std::sync::Arc<[tekes_ui_scene::ClipShape]>)>,
    reveal_clock: content_reveal_plan::RevealClock,
    focused: Option<ComponentId>,
    focus_wrap: bool,
    overlay_scopes: Vec<overlay_scope::ScopedOverlay>,
    popover_order: Vec<PopoverSession>,
    captured: Option<ComponentId>,
    captured_overlay: bool,
    captured_mount: Option<u64>,
    capture_epoch: u64,
    dirty: bool,
    inherited_surface: &'static str,
    inherited_surface_declared: bool,
    modal: Option<(ComponentId, Option<(ComponentId, u64)>)>,
    modal_focus_pending: bool,
    // Paint-only owner after close. It is never used as the event/focus scope.
    presented_dialog: Option<ComponentId>,
    text_keys: Vec<key::KeyFrame>,
    deferred_text_key: Option<key::KeyFrame>,
    completed_text_key: Option<(ContentResponse, bool)>,
    last_text_key: u64,
    focus_epoch: u64,
    window_active: bool,
    animation_frame_requested: bool,
    actions: Option<Vec<actions::PendingAction>>,
    reading_transaction: bool,
    last_layout: Option<(Rect, Rect)>,
    last_render_stamp: Option<(u64, u64)>,
    last_palette: Option<Palette>,
    last_surface: Option<(&'static str, bool)>,
    item_reveal: Option<content_reveal::ItemReveal>,
    reading_scroll_source: Option<(ComponentId, u64)>,
    help: help::HelpState,
    help_overlay_deferred: bool,
    event_bubbled: Option<bool>,
    pointer_focus_requested: bool,
}
impl ContentHost {
    pub fn new(mut root: Content) -> Result<Self, CompositionError> {
        root.validate_curve_tree(false)?;
        root.ids(&mut BTreeSet::new())?;
        root.lifecycle(Lifecycle::Mounted);
        Ok(Self::from_mounted(root))
    }
    fn from_mounted(root: Content) -> Self {
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
            captured_overlay: false,
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
    /// Mutation and event invalidation; animation callbacks remain on the
    /// existing text adapter. A successful render acknowledges this flag.
    pub fn needs_render(&self) -> bool {
        self.dirty
            // Even refused input retires the frozen geometry's input lease.
            // The next frame must acknowledge that generation before repaint.
            || self.reveal_clock.pending
            || self
                .last_render_stamp
                .is_some_and(|last| self.reveal_clock.stamp().ok() != Some(last))
            || self.help.needs_frame()
            || self.popover_animation_pending()
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
    fn bind_padding_focus(
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
        let mut requested = false;
        let active = self.window_active;
        let result = self.render_frame(
            scene,
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
        self.paint_current_frame(scene, text, frame, clip, palette)
    }
    fn render_frame(
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
        if let Some(table) = &self.curve_shapes {
            scene
                .try_import_clip_shapes(table, Default::default())
                .map_err(CompositionError::SceneFragment)?;
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
        self.paint_current_frame(scene, text, frame, clip, palette)
    }
    fn paint_current_frame(
        &mut self,
        scene: &mut Scene,
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
    fn layout_preflighted(
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
        self.publish_empty_state_exposures()?;
        self.publish_notice_exposures()?;
        self.last_layout = Some(requested);
        Ok(frame)
    }
    fn reconcile_active_inputs(&mut self) {
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
            })
        }) {
            if let Some(c) = self.captured.take().and_then(|id| self.root.find_mut(id)) {
                c.component.event(&ContentEvent::Cancel);
            }
            self.captured_mount = None;
            self.captured_overlay = false;
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
    }
    pub fn focus(&mut self, id: Option<ComponentId>, visible: bool) -> bool {
        matches!(
            self.admit_ordinary_focus(id, visible, None, action_preparation::FocusOrigin::Direct),
            action_preparation::FocusAdmission::Accepted { changed: true }
        )
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
    fn record_reading_scroll(
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
        self.reading_scroll_source = None;
        self.pointer_focus_requested = false;
        if !self.window_active {
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
            if command.starts_with("move") || command == "selectAll:");
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
        if !self.split_structures_intact() {
            return ContentResponse::default();
        }
        if self.split_input_guard(event) {
            return ContentResponse::consumed(false);
        }
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
        if !matches!(event, ContentEvent::Cancel) && self.preflight_notice_structures().is_err() {
            return ContentResponse::default();
        }
        if self.root.ids(&mut BTreeSet::new()).is_err() {
            self.cancel();
            return ContentResponse::default();
        }
        self.reconcile_active_inputs();
        if event.is_primary_pointer()
            && self.requires_curve_geometry()
            && self
                .geometry_snapshot()
                .is_none_or(|snapshot| !snapshot.is_current())
        {
            self.event_bubbled = Some(false);
            return ContentResponse::default();
        }
        if matches!(event, ContentEvent::Cancel) {
            self.cancel();
            return ContentResponse::consumed(true);
        }
        let routing = event.routing_event();
        let routed = routing.as_ref();
        if matches!(routed, ContentEvent::PointerDown(_))
            && event.is_primary_pointer()
            && self.capture_epoch == u64::MAX
        {
            self.event_bubbled = Some(false);
            return ContentResponse::default();
        }
        if let Some(response) = self.context_menu_event(event) {
            return response;
        }
        if self.help_event(event) {
            return ContentResponse::consumed(true);
        }
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
            let captured_overlay = self.captured_overlay;
            if matches!(routed, ContentEvent::PointerUp(_)) {
                self.captured = None;
                self.captured_mount = None;
                self.captured_overlay = false;
            }
            let result = if captured_overlay {
                {
                    let (scope, actions) = self.scope_with_actions();
                    scope.route_from(id, event, actions)
                }
            } else {
                self.root.route_from(id, event, &mut self.actions)
            };
            return result
                .map(|(_, response, bubbles)| {
                    self.event_bubbled = Some(bubbles);
                    response
                })
                .unwrap_or_default();
        }
        if let Some(id) = self.scope().overlay_id() {
            let response =
                self.root
                    .find_mut(id)
                    .unwrap()
                    .deliver_actions(event, true, &mut self.actions);
            if response.consumed {
                self.record_reading_scroll(event, id, &response);
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
        let pointer_down = match event {
            ContentEvent::PointerDown(point) => Some(*point),
            ContentEvent::Pointer(pointer)
                if pointer.phase == PointerPhase::Down && pointer.is_primary() =>
            {
                Some(pointer.position)
            }
            _ => None,
        };
        if let Some(point) = pointer_down
            && let Some(response) = self.try_prepared_pointer(event, point)
        {
            return response;
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
            if let Some((response, bubbles)) =
                self.focused.and_then(|id| self.root.find_mut(id)).map(|c| {
                    let response = c.deliver_actions(event, false, &mut self.actions);
                    let bubbles = c.bubbles(event, &response);
                    (response, bubbles)
                })
                && (response.consumed || !bubbles)
            {
                self.event_bubbled = Some(bubbles);
                return response;
            }
            return self.advance_focus(*reverse);
        }
        if let ContentEvent::PointerMove(position) = routed {
            let (scope, actions) = self.scope_with_actions();
            let (hit, changed) = scope.move_pointer(event, *position, actions);
            let (_, mut response, bubbles) =
                hit.unwrap_or((ComponentId(0), ContentResponse::default(), true));
            response.changed |= changed;
            self.event_bubbled = Some(bubbles);
            return response;
        }
        if let Some(id) = self.padding_focus_target(event) {
            // Resolve before ordinary pointer dispatch. In particular there is
            // no blur/callback/refocus gap, and repeated padding clicks do not
            // re-run set_focus/reveal against a composing editor.
            let changed = if self.focused == Some(id) {
                self.pointer_focus_requested = true;
                false
            } else if let action_preparation::FocusAdmission::Accepted { changed } = self
                .admit_ordinary_focus(
                    Some(id),
                    false,
                    None,
                    action_preparation::FocusOrigin::Direct,
                )
            {
                self.pointer_focus_requested = true;
                changed
            } else {
                false
            };
            return ContentResponse::consumed(changed);
        }
        let result = match routed {
            ContentEvent::PointerUp(p)
            | ContentEvent::PointerDown(p)
            | ContentEvent::Scroll { position: p, .. } => {
                let (scope, actions) = self.scope_with_actions();
                scope.route(event, *p, actions)
            }
            _ => self.focused.and_then(|id| {
                let (scope, actions) = self.scope_with_actions();
                scope.route_from(id, event, actions)
            }),
        };
        if let Some((id, r, bubbles)) = result {
            self.record_reading_scroll(event, id, &r);
            self.event_bubbled = Some(bubbles);
            if matches!(routed, ContentEvent::PointerDown(_)) && event.is_primary_pointer() {
                if r.capture_pointer {
                    self.capture_epoch = self
                        .capture_epoch
                        .checked_add(1)
                        .expect("pointer capture identity exhausted");
                    self.captured = Some(id);
                    self.captured_mount = self.root.find_active(id).map(|node| node.mount);
                    self.captured_overlay = self.overlay_scope_id().is_some_and(|scope| {
                        self.root
                            .find_active(scope)
                            .is_some_and(|owner| owner.find(id).is_some())
                    });
                }
                if r.consumed
                    && self.root.find(id).is_some_and(|c| {
                        c.component.focusable()
                            && !c.preserve_activation_focus
                            && !c.component.preserves_activation_focus()
                    })
                {
                    self.pointer_focus_requested = matches!(
                        self.admit_ordinary_focus(
                            Some(id),
                            false,
                            None,
                            action_preparation::FocusOrigin::Direct
                        ),
                        action_preparation::FocusAdmission::Accepted { .. }
                    );
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
    fn base_scope(&self) -> &Content {
        self.modal
            .and_then(|(id, _)| self.root.find(id))
            .unwrap_or(&self.root)
    }
    fn scope(&self) -> &Content {
        self.overlay_scope_id()
            .and_then(|id| self.root.find_active(id))
            .unwrap_or_else(|| self.base_scope())
    }
    fn scope_with_actions(&mut self) -> (&mut Content, &mut Option<Vec<actions::PendingAction>>) {
        let scope = if let Some(id) = self.overlay_scope_id().or(self.modal.map(|(id, _)| id)) {
            self.root.find_mut(id).expect("mounted input scope")
        } else {
            &mut self.root
        };
        (scope, &mut self.actions)
    }
    /// Current original interactive overlay/modal authority. A retained exit
    /// painter has no input authority, and an inactive window publishes none.
    pub fn input_overlay_owner(&self) -> Option<(ComponentId, u64)> {
        if !self.window_active {
            return None;
        }
        let id = self
            .overlay_scope_id()
            .or(self.modal.map(|(id, _)| id))
            .or_else(|| self.scope().overlay_id())?;
        let node = self.root.find_active(id)?;
        Some((id, node.mount))
    }
    /// Original native key owner binding, retained across geometry-only changes
    /// and zero pane allocation. Focus epoch and mount authenticate key frames;
    /// editor/document identity remains owned by the original component.
    pub fn native_key_binding(&self) -> Option<Vec<u8>> {
        if !self.window_active {
            return None;
        }
        let owner = self.scope().find_active(self.focused?)?;
        if owner.component.semantics().disabled {
            return None;
        }
        owner.component.native_key_binding()
    }
    /// Current UTF-16 editing/geometry source for a platform's text query bridge.
    pub fn focused_text(&self) -> Option<&crate::TextSurface> {
        self.focused.and_then(|id| self.text_surface(id))
    }
    /// Existing editor/document binding, unchanged by ordinary edits or selection.
    pub fn focused_text_binding(&self) -> Option<(u64, u64)> {
        self.focused_text()
            .map(|surface| surface.input.controlled_binding())
    }
    /// Read the original focused text owner without editing or reshaping it.
    pub fn text_input_snapshot(&self) -> Option<ContentTextInput> {
        self.semantic_text_input_snapshot(self.focused?)
    }
    /// Read the original semantic owner's text and visible caret geometry.
    pub fn semantic_text_input_snapshot(&self, id: ComponentId) -> Option<ContentTextInput> {
        if !self.window_active {
            return None;
        }
        let owner = self.scope().find_active(id)?;
        if owner.component.semantics().disabled {
            return None;
        }
        let mut input = owner.component.text_input_snapshot()?;
        // Zero panes retain the original logical editor, selection and marked
        // state. They must not publish retained geometry from an earlier frame.
        if self.split_text_geometry_hidden(id) {
            input.caret = None;
            input.clip = None;
            input.carets.clear();
        }
        Some(input)
    }
    /// Complete original text geometry, including off-focus and offscreen text.
    /// Zero Split panes retain logical editing state but cannot publish old layout.
    pub fn semantic_text_layout_snapshot(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> Option<ContentTextLayout> {
        if !self.window_active || self.split_text_geometry_hidden(id) {
            return None;
        }
        let owner = self.scope().find_active(id)?;
        // Disabled controls remain readable to accessibility. This query grants
        // no input authority; accessibility_action retains the disabled guard.
        if let Some(part) = part {
            if !owner
                .component
                .semantic_parts()
                .iter()
                .any(|p| p.index == part)
            {
                return None;
            }
            owner.component.semantic_part_text_layout(part)
        } else {
            owner.component.semantic_text_layout()
        }
    }
    /// Published text queries for active semantic owners, including unfocused
    /// fields. Closed or background modal subtrees cannot expose editable text.
    pub fn text_surface(&self, id: ComponentId) -> Option<&crate::TextSurface> {
        self.scope()
            .find_active(id)
            .and_then(|c| c.component.text_surface())
    }
    /// Current active owner's process-local mount identity for native bridges.
    /// Reorder retains it; replacement/remount changes it even for the same ID.
    /// This is not a persistent identifier and does not acquire focus or input.
    pub fn semantic_mount(&self, id: ComponentId) -> Option<u64> {
        if !self.window_active {
            return None;
        }
        self.scope().find_active(id).map(|node| node.mount)
    }
    /// Read the existing focus lease, including blur/refocus ABA.
    pub fn animation_frame_requested(&self) -> bool {
        self.window_active && self.animation_frame_requested
    }
    pub fn focus_epoch(&self) -> u64 {
        self.focus_epoch
    }
    /// Current original capture owner and mount; no synthetic gesture identity.
    pub fn pointer_capture(&self) -> Option<(ComponentId, u64)> {
        if self.requires_curve_geometry()
            && self
                .geometry_snapshot()
                .is_none_or(|snapshot| !snapshot.is_current())
        {
            return None;
        }
        self.captured.and_then(|id| {
            self.root
                .find_active(id)
                .filter(|node| {
                    self.captured_mount == Some(node.mount)
                        && node.visible().is_some()
                        && !node.component.semantics().disabled
                })
                .map(|node| (id, node.mount))
        })
    }
    /// Same validated original capture, with its existing acquisition epoch for
    /// asynchronous Content callbacks. The legacy pair query stays unchanged.
    pub fn pointer_capture_lease(&self) -> Option<(ComponentId, u64, u64)> {
        self.pointer_capture()
            .map(|(id, mount)| (id, mount, self.capture_epoch))
    }
    /// The existing focused editor/custom component composition barrier.
    pub fn is_composing(&self) -> bool {
        self.focused
            .and_then(|id| self.root.find_active(id))
            .is_some_and(|node| node.component.composing())
            || self
                .focused_text()
                .is_some_and(|s| s.input.marked.is_some())
    }
    /// Original window activity for platform publication admission.
    pub fn is_window_active(&self) -> bool {
        self.window_active
    }
    /// Revoke adapter authorization recursively, including hidden descendants.
    pub fn set_window_active(&mut self, active: bool) {
        if self.window_active == active {
            return;
        }
        let deactivated = self.window_active && !active;
        // Cancel while old input authority is still live, then update retained descendants.
        self.reveal_clock.invalidate_input();
        // Retire the transient session with its precise reason before generic cancellation.
        let mut ids = Vec::new();
        self.root.popover_ids(&mut ids);
        for id in ids {
            if let Some(popover) = self
                .root
                .find_mut(id)
                .and_then(|node| node.component_mut::<PopoverContent>())
            {
                popover.host_window_active(active);
            }
        }
        if deactivated {
            self.cancel();
        }
        self.window_active = active;
        self.root.refresh_activity(active);
        self.dirty = true;
    }
    /// Cancel only this original subtree before its typed declaration changes.
    pub fn cancel_subtree(&mut self, id: ComponentId) -> bool {
        let Some(node) = self.root.find(id) else {
            return false;
        };
        let focus = self.focused.is_some_and(|id| node.find(id).is_some());
        let capture = self.captured.is_some_and(|id| node.find(id).is_some());
        for scope in &mut self.overlay_scopes {
            if node.find(scope.id).is_some() {
                scope.restore = None;
                scope.focus_pending = false;
            }
        }
        fn cancel(node: &mut Content) {
            node.component.event(&ContentEvent::Cancel);
            node.component
                .event(&ContentEvent::PointerMove(Point::new(f32::MIN, f32::MIN)));
            if let Some(children) = node.component.children_mut() {
                for child in children {
                    cancel(child);
                }
            }
        }
        cancel(self.root.find_mut(id).unwrap());
        if capture {
            self.captured = None;
            self.captured_mount = None;
            self.captured_overlay = false;
        }
        if focus {
            self.focus(None, false);
        }
        self.help.clear();
        self.dirty = true;
        self.reconcile_active_inputs();
        true
    }
    /// Read the existing original owner's part hierarchy; no focus or input.
    pub fn semantic_part_parent(&self, id: ComponentId, part: usize) -> Option<usize> {
        self.scope()
            .find_active(id)?
            .component
            .semantic_part_parent(part)
    }
    pub fn semantic_preserves_activation_focus(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> bool {
        self.scope().find_active(id).is_some_and(|node| {
            node.preserve_activation_focus
                || node.component.preserves_activation_focus()
                || part.is_some_and(|part| {
                    node.component
                        .semantic_part_preserves_activation_focus(part)
                })
        })
    }
    /// Actions use the same semantic owner and production state machine as input.
    /// Closed/dismissed/inactive subtrees and stale menu parts reject requests.
    pub fn accessibility_action(
        &mut self,
        id: ComponentId,
        part: Option<usize>,
        action: AccessibilityAction,
    ) -> ContentResponse {
        if self.split_ax_guard(id) {
            return ContentResponse::default();
        }
        if !self.window_active {
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::ShowMenu) {
            return self.context_menu_accessibility_action(id, part);
        }
        if self.scope().find_active(id).is_some_and(|node| {
            node.component.semantics().role == ContentRole::Image
                || part.is_some_and(|part| {
                    node.component
                        .semantic_parts()
                        .iter()
                        .any(|p| p.index == part && p.semantics.role == ContentRole::Image)
                })
        }) {
            return ContentResponse::default();
        }
        if let Some(response) = self.try_prepared_ax(id, part, &action) {
            return response;
        }
        self.reconcile_active_inputs();
        if self
            .scope()
            .find_active(id)
            .is_none_or(|n| n.component.semantics().disabled)
        {
            return ContentResponse::default();
        }
        // Preserve the original composing owner unless an auxiliary action
        // explicitly retains focus. Use the same marked/custom barrier as Tab.
        if self.is_composing()
            && self.focused != Some(id)
            && (matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::SetSelection(_)
            ) || (part.is_none()
                && matches!(action, AccessibilityAction::Press)
                && self
                    .scope()
                    .find_active(id)
                    .is_some_and(|n| n.component.focusable() && !n.preserve_activation_focus)))
        {
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::Press) {
            self.help.suppress();
        }
        if part.is_none()
            && matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::Press
            )
            && (matches!(action, AccessibilityAction::Focus)
                || self
                    .scope()
                    .find_active(id)
                    .is_some_and(|n| !n.preserve_activation_focus))
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
        let mut result = owner.component.accessibility_action(part, &action);
        owner.record_actions(
            &result,
            ContentActionSource::Accessibility,
            true,
            &mut self.actions,
        );
        if result.consumed && matches!(action, AccessibilityAction::Press) {
            // Press may focus through the ordinary AX path above. It is still
            // an activation, not a fresh request for delayed keyboard help.
            self.help.suppress();
        }
        let delegated_focus = owner.component.accessibility_focus_requested();
        if result.consumed
            && (matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::SetSelection(_)
            ) || delegated_focus)
        {
            // Original semantic part validation must precede keyboard ownership.
            result.changed |= self.focus(Some(id), true);
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
        // Reordering preserves the mount; replacing or remounting the same ID
        // cannot inherit a previous object's modal return-focus intent.
        let restore = self.capture_focus();
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
        if let Some(node) = self.root.find_mut(id) {
            empty_state::hide_subtree(node);
            notice::hide_subtree(node);
            node.reconcile_presentation_runnable(false);
        }
        self.modal = None;
        self.restore_focus(restore);
        true
    }
    pub fn cancel_pointer_capture(&mut self) -> bool {
        let Some(id) = self.captured.take() else {
            return false;
        };
        self.captured_mount = None;
        self.captured_overlay = false;
        if let Some(owner) = self.root.find_mut(id) {
            owner.component.cancel_pointer();
        }
        self.help.clear();
        self.dirty = true;
        true
    }
    pub fn cancel(&mut self) {
        self.text_keys.clear();
        self.deferred_text_key = None;
        self.completed_text_key = None;
        for scope in &mut self.overlay_scopes {
            scope.restore = None;
            scope.focus_pending = false;
        }
        self.reading_scroll_source = None;
        self.help.clear();
        self.dirty = true;
        self.modal_focus_pending = false;
        fn cancel(c: &mut Content) {
            if let Some(popover) = c.component_mut::<PopoverContent>() {
                popover.close(PopoverCloseReason::Cancelled);
            }
            c.component.event(&ContentEvent::Cancel);
            if let Some(children) = c.component.children_mut() {
                for c in children {
                    cancel(c)
                }
            }
        }
        cancel(&mut self.root);
        self.captured = None;
        self.captured_mount = None;
        self.captured_overlay = false;
        self.focus(None, false);
    }
    /// Move the original mounted subtree, preserving its Content identities and
    /// help declarations. Unlike remove/insert this is not an unmount/remount.
    pub fn reparent(
        &mut self,
        id: ComponentId,
        parent: ComponentId,
    ) -> Result<(), CompositionError> {
        let node = self.root.find(id).ok_or(CompositionError::MissingId(id))?;
        if id == self.root.id || node.find(parent).is_some() {
            return Err(CompositionError::InvalidOrder);
        }
        if let Some(action) = self.root.action_ancestor(parent)
            && let Some(child) = node.interactive_id()
        {
            return Err(CompositionError::NestedInteractive { action, child });
        }
        if self.root.curve_ancestor(parent, false).unwrap_or(false) {
            node.validate_curve_tree(true)?;
        }
        let target = self
            .root
            .find(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if target.component.fixed_slots() || target.component.semantics().role.is_action_owner() {
            return Err(CompositionError::NotContainer(parent));
        }
        // Validate both mutable boundaries before touching the source.
        if self
            .root
            .find_mut(parent)
            .unwrap()
            .component
            .children_mut()
            .is_none()
        {
            return Err(CompositionError::NotContainer(parent));
        }
        fn mutable_parent(node: &mut Content, id: ComponentId) -> bool {
            let fixed =
                node.component.fixed_slots() || node.component.semantics().role.is_action_owner();
            let Some(children) = node.component.children_mut() else {
                return false;
            };
            if children.iter().any(|c| c.id == id) {
                return !fixed;
            }
            children.iter_mut().any(|c| mutable_parent(c, id))
        }
        if !mutable_parent(&mut self.root, id) {
            return Err(CompositionError::MissingId(id));
        }
        self.cancel_subtree(id);
        fn take(node: &mut Content, id: ComponentId) -> Option<Content> {
            let children = node.component.children_mut()?;
            if let Some(i) = children.iter().position(|c| c.id == id) {
                return Some(children.remove(i));
            }
            children.iter_mut().find_map(|c| take(c, id))
        }
        let node = take(&mut self.root, id).expect("validated source parent");
        self.root
            .find_mut(parent)
            .unwrap()
            .component
            .children_mut()
            .unwrap()
            .push(node);
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(())
    }
    pub fn insert(
        &mut self,
        parent: ComponentId,
        mut child: Content,
    ) -> Result<(), CompositionError> {
        if let Some(action) = self.root.action_ancestor(parent)
            && let Some(child) = child.interactive_id()
        {
            return Err(CompositionError::NestedInteractive { action, child });
        }
        let mut ids = BTreeSet::new();
        self.root.ids(&mut ids)?;
        child.ids(&mut ids)?;
        if self.root.curve_ancestor(parent, false).unwrap_or(false) {
            child.validate_curve_tree(true)?;
        }
        child.refresh_activity(self.window_active);
        let node = self
            .root
            .find_mut(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if node.component.fixed_slots() || node.component.semantics().role.is_action_owner() {
            return Err(CompositionError::NotContainer(parent));
        }
        let children = node
            .component
            .children_mut()
            .ok_or(CompositionError::NotContainer(parent))?;
        let mut child = child;
        child.lifecycle(Lifecycle::Mounted);
        children.push(child);
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(())
    }
    pub fn remove(&mut self, id: ComponentId) -> Result<Content, CompositionError> {
        // Check the actual mutable child boundary before dismissing a modal.
        // Root/fixed-slot/read-only containers cannot be removed; an error must
        // preserve the current scope, focus and retained presentation.
        fn can_remove(node: &mut Content, id: ComponentId) -> bool {
            let fixed =
                node.component.fixed_slots() || node.component.semantics().role.is_action_owner();
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
            let action =
                node.component.fixed_slots() || node.component.semantics().role.is_action_owner();
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
            self.captured_mount = None;
            self.captured_overlay = false;
        }
        child.lifecycle(Lifecycle::Unmounted);
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
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
        if node.component.fixed_slots() || node.component.semantics().role.is_action_owner() {
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
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
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
    /// Query a published virtual part's original finite domain. The existing
    /// local-owner branch retains value_range's window-activity behavior.
    pub fn semantic_value_range(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> Option<RangeInclusive<f64>> {
        let Some(part) = part else {
            return self.value_range(id);
        };
        if !self.window_active {
            return None;
        }
        let owner = self.scope().find_active(id)?;
        if !owner
            .component
            .semantic_parts()
            .iter()
            .any(|p| p.index == part)
        {
            return None;
        }
        let range = owner.component.semantic_part_value_range(part)?;
        (range.start().is_finite() && range.end().is_finite() && range.start() <= range.end())
            .then_some(range)
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
        self.reveal_clock.invalidate_geometry();
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
        intrinsic: bool,
    ) -> Vec<MeasuredChild> {
        let mut measured: Vec<_> = self
            .children
            .iter_mut()
            .map(|c| {
                if intrinsic {
                    c.measure_intrinsic(text, available)
                } else {
                    c.measure(text, available)
                }
            })
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
            measured[i] = if intrinsic {
                self.children[i].measure_intrinsic(text, constrained)
            } else {
                self.children[i].measure(text, constrained)
            };
        }
        measured
    }
    fn measure_content(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
        intrinsic: bool,
    ) -> MeasuredChild {
        let horizontal = self.model.axis == Axis::Horizontal;
        let mut main = 0_f32;
        let mut cross = 0_f32;
        for m in self.measurements(text, available, intrinsic) {
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
}
impl Component for StackContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.measure_content(text, available, false)
    }
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.measure_content(text, available, true)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let measured = self.measurements(text, frame.size, false);
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
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
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
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        let columns = self.model.columns.max(1);
        let cells = self.model.layout(
            Rect::new(0., 0., available.width, available.height),
            self.children.len(),
            0.,
        );
        let width = self
            .children
            .iter_mut()
            .zip(cells)
            .map(|(child, cell)| {
                child
                    .measure_intrinsic(text, Size::new(cell.size.width, available.height))
                    .size
                    .width
            })
            .fold(0_f32, f32::max);
        let gap =
            finite(self.model.gap_x).min(available.width / columns.saturating_sub(1).max(1) as f32);
        let width =
            (width * columns as f32 + gap * columns.saturating_sub(1) as f32).min(available.width);
        // Preserve the original equal-column allocation and remeasure its rows
        // at the receiving width. No alternative Grid arrangement is introduced.
        self.measure(text, Size::new(width, available.height))
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
    reading: Option<content_reading::ContentReadingState>,
    reading_mounted: bool,
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
            reading: None,
            reading_mounted: false,
        }
    }
    fn arrange_content(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        gutter: f32,
        width: f32,
    ) -> Rect {
        let geometry = self.scroll.geometry(frame, Some(clip));
        let origin = geometry.content_origin();
        let child_clip = geometry.visible_viewport_rect().unwrap_or_default();
        let mut y = origin.y + gutter;
        for c in &mut self.children {
            let h = c.measure(text, Size::new(width, 1e8)).size.height;
            c.arrange(text, Rect::new(origin.x + gutter, y, width, h), child_clip);
            y += h;
        }
        child_clip
    }
}
impl Component for ScrollContent {
    fn reading_scroll_source(&self) -> bool {
        true
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        let base = PreparedEffects::local_work(n)?;
        base.checked_add(match &self.reading {
            Some(r) => content_reading::reader_preparation_work(r, n)
                .ok_or(PreparedEffectsError::CounterCapacity)?,
            None => 0,
        })
        .ok_or(PreparedEffectsError::CounterCapacity)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        self.prepare_scroll_effects(hooks)
    }
    fn lifecycle_prepared(&mut self, event: Lifecycle, context: &mut PreparedLifecycleContext) {
        context.scroll_lifecycle(self, event);
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
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
        let reading_anchor = self.reading.as_ref().and_then(|s| s.capture(&self.scroll));
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
        if let Some(reading) = &mut self.reading {
            reading.prepare(&mut self.scroll);
        }
        let visible = self.arrange_content(text, frame, clip, gutter, width);
        let restored = self.reading.as_mut().is_some_and(|reading| {
            reading.restore(&mut self.scroll, &self.children, visible, reading_anchor)
        });
        if restored {
            self.arrange_content(text, frame, clip, gutter, width);
        }
        if let Some(reading) = &mut self.reading {
            reading.record(&self.scroll, &self.children, visible);
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
                if self
                    .reading
                    .as_mut()
                    .is_some_and(|reading| !reading.observe_scroll(*delta, *begin, *momentum))
                {
                    return ContentResponse::default();
                }
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
                self.last_frame = None;
                if let Some(reading) = &mut self.reading {
                    reading.fence_momentum();
                }
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
        if new != old {
            self.stop_following();
        }
        descendant.offset(old.x - new.x, old.y - new.y)
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.reading_mounted = event == Lifecycle::Mounted;
        if event == Lifecycle::Unmounted {
            self.last_frame = None;
            self.reading_unmounted();
        }
    }
}

/// The forwarding value keeps existing sized renderer APIs source-compatible.
struct FrameTextAdapter<'a>(&'a mut dyn ButtonTextRenderer, Option<&'a mut bool>, bool);
impl ButtonTextRenderer for FrameTextAdapter<'_> {
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        self.0.icon_renderer()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
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
    fn animation_frames_allowed(&self) -> bool {
        self.2 && self.0.animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        if !self.animation_frames_allowed() {
            return;
        }
        if let Some(requested) = &mut self.1 {
            **requested = true;
        }
        self.0.request_animation_frame()
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn first_line_height(&mut self, t: &str, r: proposal::TextRole, w: f32) -> f32 {
        self.0.first_line_height(t, r, w)
    }
}

struct TextAdapter<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for TextAdapter<'_> {
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        self.0.icon_renderer()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
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
    fn animation_frames_allowed(&self) -> bool {
        self.0.animation_frames_allowed()
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
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)?
            .checked_add(
                self.text
                    .len()
                    .checked_mul(n.max(1))
                    .ok_or(PreparedEffectsError::CounterCapacity)?,
            )
            .ok_or(PreparedEffectsError::CounterCapacity)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
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
    let theme = node.resolved_theme(theme);
    scene.push_clip(node.clip);
    node.component
        .paint(scene, text, node.frame, node.clip, theme);
    scene.pop_clip();
    let child_theme = node.child_theme(theme);
    if let Some(children) = node.component.children_mut() {
        for child in children {
            child.paint(scene, text, child_theme);
            child.paint_overlays(scene, text, child_theme);
        }
    }
}
