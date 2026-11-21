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
#[cfg(feature = "bitmap-images")]
mod media_viewer;
#[cfg(feature = "bitmap-images")]
pub use media_viewer::{
    MediaViewerCommand, MediaViewerContent, MediaViewerError, MediaViewerMode, MediaViewerTransform,
};
mod icon;
pub use icon::{IconContent, IconImage, IconRenderError, WithIconRenderer};
mod icon_text;
#[cfg(feature = "svg-icons")]
pub use icon::{SvgIconCache, SvgIconRenderer};
pub use icon_text::IconTextContent;
mod disclosure;
pub use disclosure::{DisclosureContent, DisclosureIntent};
mod actions;
#[cfg(feature = "inspection")]
mod inspection;
#[cfg(feature = "inspection")]
pub use inspection::{InspectionGeometry, InspectionPaint, InspectionProperty, InspectionSnapshot};
mod collection_transaction;
mod legacy_selectable_list;
pub use legacy_selectable_list::SelectableListContent;
mod selectable_list;
pub use selectable_list::{
    KeyedSelectableListContent, ListContentError, ListDisplayUpdate, ListIntent, ListIntentKind,
    ListReadingUpdate, ListRowDisplay,
};
mod tree;
pub use tree::{
    TreeContent, TreeContentError, TreeContentNode, TreeDisplayUpdate, TreeIntent, TreeIntentKind,
};
mod key_value_table;
pub use key_value_table::KeyValueTableContent;
mod grouped_form_row;
pub use grouped_form_row::GroupedFormRowContent;
mod datatable;
pub use datatable::{
    DataTableContent, TableCellDisplay, TableContentError, TableDisplayUpdate, TableDisplays,
    TableHeaderDisplay, TableIntent, TableIntentKind, TableIntentTarget, TableReadingUpdate,
    TableRowDisplay,
};
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
mod precheck_dispatch;
mod prepared;
use action_preparation::{
    ActionPreparation, ActionTransactionCompatibility, ComponentActionRequest,
    PreparedActionEffect, PreparedActionFocus, PreparedComponentAction,
};
use prepared::{
    PreparedEffects, PreparedEffectsError, PreparedHook, PreparedHookKind, PreparedLifecycleContext,
};
mod checkbox;
pub use checkbox::{CheckboxContent, CheckboxIntent};
mod radio;
pub use radio::{RadioContentError, RadioGroupContent, RadioSelectionIntent};
mod segmented;
pub use segmented::{SegmentIntent, SegmentedContentError, SegmentedControlContent};
mod action_proxy;
mod notice;
pub use notice::{BannerContent, ToastContent};
mod empty_state;
pub use empty_state::EmptyStateContent;
mod action_preparation;
mod content_reveal_plan;
pub use content_reveal_plan::{
    HostRevealBudget, HostRevealError, HostRevealOutcome, HostRevealPlan, HostRevealVerticalClip,
};
mod markdown_editor;
mod markdown_projection;
mod rich_content;
mod rich_document;
pub use markdown_editor::{MarkdownEditorContent, MarkdownEditorError, MarkdownEditorVersion};
pub use markdown_projection::{
    MarkdownInlineKind, MarkdownInlineSpan, MarkdownProjectedBlock, MarkdownProjection,
};
pub use rich_content::{
    RichBlockGeometry, RichContent, RichContentError, RichContentVersion, RichLinkAction,
};
pub use rich_document::{RichBlock, RichBlockKind, RichDocument, RichLink};
mod adapters;
mod stepper;
pub use stepper::StepperContent;
mod surface;
pub use surface::SurfaceContent;
mod combobox;
pub use combobox::{ComboBoxContent, ComboBoxSelection, ComboBoxUpdateError};
mod chart_content;
pub use chart_content::{ChartContent, ChartContentError};
mod button;
pub use button::{Button, ButtonActivationPhase, ButtonContents, ButtonImagePosition};
use button::{action_child_theme, measure_action_slot, resolve_action_slot};
mod content_reading;
mod content_reveal;
mod status_badge;
pub use status_badge::{BadgeContent, StatusBadgeContent};
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
mod link;
pub use link::LinkContent;
mod tab_strip;
pub use tab_strip::{TAB_TITLE_MAX_WIDTH, TabStripContent, TabStripIntent, TabStripItem};
mod navigation;
pub use navigation::{BreadcrumbContent, CommandPaletteContent, TabItem, TabsContent};
mod menu;
mod search;
pub use search::{
    SearchAction, SearchCommand, SearchCommandError, SearchCommandKind, SearchFieldContent,
};
mod text_field;
pub use text_field::{TextFieldContent, TextSubmitPolicy, TextTabPolicy, TextViewport};
mod select;
pub use select::{SelectContent, SelectUpdateError};
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
pub use crate::TextEditCommand;
use crate::extended::{Axis, Stack};
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
    DialogContent, ListContent, ModalViewportContent, SliderContent, SwitchContent,
};
pub use content_reading::{
    ContentReadingAnchorUpdate, ContentReadingCheckpoint, ContentReadingCheckpointError,
    ContentReadingPosition, ContentReadingRestoreRequest, ContentReadingResult,
};
pub use content_reveal::{ContentRevealError, ContentRevealResult, ContentRevealStatus};
pub use key::{ContentKeyEvent, TextKeyDisposition, TextKeyOutcome};
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
    /// Positive delta moves toward later content. Precise input uses logical
    /// points; non-precise input uses lines, converted by the receiving viewport.
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
    TextCommand(crate::TextEditCommand),
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
    /// Single-line Search over the original editor.
    SearchInput,
    /// Single-line editable original ComboBox owner.
    ComboBox,
    Slider,
    Switch,
    Checkbox,
    RadioGroup,
    RadioButton,
    SegmentedGroup,
    Segment,
    List,
    ListItem,
    Tree,
    TreeItem {
        level: usize,
    },
    Table,
    TableColumn {
        index: usize,
    },
    TableColumnHeader {
        index: usize,
        sort: Option<crate::extended::SortDirection>,
    },
    TableRow {
        index: usize,
    },
    TableCell {
        row: usize,
        column: usize,
    },
    Dialog,
    MenuButton,
    /// Noneditable original Select trigger.
    Select,
    Menu,
    MenuItem,
    TabList,
    Tab,
    /// The original visible split divider, with its finite ratio domain.
    Splitter,
}
impl ContentRole {
    fn is_selection(self) -> bool {
        matches!(
            self,
            Self::Checkbox
                | Self::RadioGroup
                | Self::RadioButton
                | Self::SegmentedGroup
                | Self::Segment
        )
    }
    fn is_action_owner(self) -> bool {
        matches!(
            self,
            Self::Button | Self::Link | Self::MenuButton | Self::Select
        )
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
    fn valid_selection(&self, parent: Option<ContentRole>) -> bool {
        use ContentRole as R;
        if !self.role.is_selection() {
            return true;
        }
        if self.expanded.is_some() {
            return false;
        }
        match self.role {
            R::Checkbox => matches!(
                (self.value.as_deref(), self.selected),
                (Some("off" | "mixed"), false) | (Some("on"), true)
            ),
            R::RadioGroup | R::SegmentedGroup => self.value.is_none() && !self.selected,
            R::RadioButton => self.value.is_none() && parent == Some(R::RadioGroup),
            R::Segment => self.value.is_none() && parent == Some(R::SegmentedGroup),
            _ => true,
        }
    }
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
    /// Set the original owner disclosure state; never toggle via Press.
    SetExpanded(bool),
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentTheme {
    pub(crate) palette: Palette,
    pub(crate) foreground: Option<Rgba>,
    pub(crate) icon_foreground: Option<Rgba>,
    surface: &'static str,
    surface_declared: bool,
}
impl ContentTheme {
    pub fn palette(self) -> Palette {
        self.palette
    }
    pub fn foreground(self) -> Option<Rgba> {
        self.foreground
    }
    pub fn icon_foreground(self) -> Option<Rgba> {
        self.icon_foreground
    }
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.palette = palette;
        self
    }
    pub fn with_foreground(mut self, foreground: Option<Rgba>) -> Self {
        self.foreground = foreground;
        self
    }
    pub fn with_icon_foreground(mut self, icon_foreground: Option<Rgba>) -> Self {
        self.icon_foreground = icon_foreground;
        self
    }
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
pub(crate) enum TableTransactionCompatibility {
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

    fn has_stable_hooks(&self) -> bool {
        self.delegated_owner()
            .is_some_and(Component::has_stable_hooks)
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        self.delegated_owner()
            .is_some_and(Component::supports_prepared_lifecycle)
    }
    fn can_accept_window_active_transitions(&self, count: usize) -> bool {
        self.delegated_owner()
            .is_none_or(|owner| owner.can_accept_window_active_transitions(count))
    }
    fn ignores_pointer_actions(&self) -> bool {
        self.delegated_owner()
            .is_some_and(Component::ignores_pointer_actions)
    }
    fn delegated_owner(&self) -> Option<&dyn Component> {
        None
    }
    fn delegated_owner_mut(&mut self) -> Option<&mut dyn Component> {
        None
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
    /// An additional vertical boundary for this direct child edge, in the last
    /// arranged Host coordinates before ancestor intersection. The planner
    /// translates it with this owner, preserving the ancestor's horizontal clip.
    /// This declaration does not grant translation/passthrough capability.
    fn host_reveal_child_vertical_clip(
        &self,
        _child: ComponentId,
    ) -> Option<HostRevealVerticalClip> {
        None
    }

    /// Optional component-specific action hook declaration kept clean on public trait.
    /// Prechecks and prepared actions dispatch internally through `precheck_dispatch`.
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
    /// Explicit auxiliary action within a selection owner. The host currently
    /// admits only a visible Button parented by an original Tab in a TabList;
    /// ordinary selection parts retain their existing validation contract.
    fn semantic_part_is_auxiliary_action(&self, _part: usize) -> bool {
        false
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
                    | ContentRole::MenuButton
                    | ContentRole::Link
                    | ContentRole::TextInput
                    | ContentRole::Checkbox
                    | ContentRole::RadioGroup
                    | ContentRole::RadioButton
                    | ContentRole::SegmentedGroup
                    | ContentRole::Segment
                    | ContentRole::Slider
                    | ContentRole::Switch
                    | ContentRole::Splitter
            )
    }
    /// Whether an already acquired pointer capture still belongs to this owner.
    /// Pure query: no mutation, event delivery, capture acquisition or other side effects.
    /// A gesture-aware owner returns false after cancellation and remains false until
    /// it admits a fresh primary PointerDown. Repeated queries must be observational.
    /// Controls that cancel during layout or source updates retire the original
    /// Host capture through this query. The default preserves existing control policy.
    fn pointer_capture_current(&self) -> bool {
        true
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
    /// Optional observational properties. Must not mutate state or perform work
    /// that changes layout, focus, input, animation, or action queues.
    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        None
    }
    fn reveal(&mut self, descendant: Rect) -> Rect {
        descendant
    }
}

pub(super) fn finite(v: f32) -> f32 {
    if v.is_finite() { v.clamp(0., 1e8) } else { 0. }
}
pub(super) fn clean_size(s: Size) -> Size {
    Size::new(finite(s.width), finite(s.height))
}
pub(super) fn clean_rect(r: Rect) -> Rect {
    if !r.min_x().is_finite() || !r.min_y().is_finite() {
        return Rect::default();
    }
    Rect::from_origin_size(r.origin, clean_size(r.size))
}

pub(super) fn scene_dialog_contents(
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

mod containers;
mod content_node;
mod host;
mod host_ops;

pub(crate) use containers::{FrameTextAdapter, TextAdapter};
pub use containers::{
    GridContent, MaxWidthContent, ResponsiveGridContent, ScrollContent, StackContent, TextContent,
    ThemeScopeContent,
};
pub(crate) use content_node::RequiredFocusEvidence;
pub use content_node::{
    CompositionError, Content, ContentWheelEvent, ContentWheelPhase, RequiredFocusResult,
    RequiredFocusSpace,
};
pub use host::ContentHost;

#[cfg(test)]
mod core_tests;
