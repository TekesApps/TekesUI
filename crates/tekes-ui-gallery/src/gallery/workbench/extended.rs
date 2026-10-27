//! Gallery stories for the additional production control models.
//!
//! The story owns deterministic fixtures and scene placement. Interaction semantics
//! are delegated to `tekes_ui_controls::extended` models.
mod collections_adapter;
mod button_specimen;
use button_specimen::ButtonSpecimen;
mod feedback_adapter;
pub(super) mod navigation_states;
use super::super::pages::icons::GalleryIconRenderer;
use super::{Entry, PageCtx, StoryState, event_log, label, quiet_action};
use collections_adapter::CollectionsViews;
use feedback_adapter::FeedbackViews;
use tekes_ui_controls::{
    BarChart, ButtonModel, ButtonSize, ButtonTextRenderer, ChartDatum, ControlKey, ControlState,
    RadioGroupView, SelectionEvent, SliderView, SwitchView, extended as controls,
    layout_view::{
        BreadcrumbView, ComboBoxView, CommandPaletteView, DisclosureView, GridView,
        LinkView, SegmentedControlView, SeparatorView, SplitViewView, StackView,
    },
    paint,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{ColorRole, Palette, proposal};

fn caption_size(ctx: &mut PageCtx<'_>, text: &str, width: f32) -> tekes_ui_scene::Size {
    let paragraph =
        ctx.text
            .proposal_label(text, proposal::TextRole::UiCaption, Some(width.max(0.)));
    tekes_ui_scene::Size::new(paragraph.width as f32, paragraph.height as f32)
}

fn matrix_control_height(renderer: &str, text: &mut impl ButtonTextRenderer, width: f32) -> f32 {
    match renderer {
        "switch" => {
            let text_width = (width - 62.).max(0.);
            let measured = text
                .measure("Enabled", proposal::TextRole::UiBody, text_width)
                .height;
            let first_line =
                text.first_line_height("Enabled", proposal::TextRole::UiBody, text_width);
            32.0_f32.max(measured.max(first_line) + 8.0)
        }
        "radio-group" => {
            let text_width = (width - 40.).max(0.);
            let measured = text
                .measure("Option", proposal::TextRole::UiBody, text_width)
                .height;
            let first_line =
                text.first_line_height("Option", proposal::TextRole::UiBody, text_width);
            let comfortable = proposal::metric("size", "size-comfortable-row").unwrap_or(40.);
            comfortable.max(measured.max(first_line) + 8.0)
        }
        "link" => text
            .measure(
                "Open documentation",
                proposal::TextRole::UiBody,
                (width - 16.).max(0.),
            )
            .height
            .max(36.),
        "disclosure" => {
            let inset = proposal::metric("spacing", "space-relaxed").unwrap_or(12.);
            let icon_slot = 24.0_f32.min((width - inset * 2.).max(0.));
            let text_width = (width - inset * 2. - icon_slot).max(0.);
            let label_height = text
                .measure("Details", proposal::TextRole::UiBody, text_width)
                .height;
            let vertical = ((proposal::metric("size", "size-action-button").unwrap_or(40.)
                - proposal::TextRole::UiBody.style().line_height)
                * 0.5)
                .max(0.);
            (label_height + 2. * vertical)
                .max(proposal::metric("size", "size-action-button").unwrap_or(40.))
        }
        "icon-button" => ButtonSpecimen::measure(text).height,
        "slider" => 36.0_f32.max(proposal::metric("size", "size-action-button").unwrap_or(40.)),
        "segmented-control" => ["General", "Advanced", "Security"]
            .into_iter()
            .map(|label| {
                ButtonModel::measure(
                    text,
                    label,
                    width / 3.0,
                    ButtonSize::Md,
                    proposal::TextRole::UiBody,
                )
                .height
            })
            .fold(0.0_f32, f32::max),
        _ => 36.0,
    }
}

/// Draw explanatory copy in a caption slot measured with the same shaping,
/// locale, and type scale as its rendering path.
pub(super) fn wrapped_caption(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    x: f32,
    y: f32,
    width: f32,
    text: &str,
) -> Rect {
    let size = caption_size(ctx, text, width);
    let slot = Rect::new(x, y, width.max(0.), size.height);
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        slot,
        text,
        proposal::TextRole::UiCaption,
        ColorRole::SecondaryText.resolve(ctx.theme),
    );
    slot
}

pub(super) fn supports(renderer: &str) -> bool {
    matches!(
        renderer,
        "stack"
            | "grid"
            | "separator"
            | "disclosure"
            | "split-view"
            | "icon-button"
            | "link"
            | "combo-box"
            | "slider"
            | "radio-group"
            | "switch"
            | "segmented-control"
            | "breadcrumb"
            | "command-palette"
            | "list"
            | "tree"
            | "data-table"
            | "chart"
            | "tooltip"
            | "popover"
            | "toast"
            | "banner"
            | "status-badge"
            | "progress"
            | "empty-state"
            | "markdown"
            | "code-block"
            | "diff-view"
    )
}
pub(super) fn supports_disabled(renderer: &str) -> bool {
    matches!(
        renderer,
        "disclosure"
            | "icon-button"
            | "link"
            | "combo-box"
            | "slider"
            | "radio-group"
            | "switch"
            | "segmented-control"
            | "breadcrumb"
    )
}
pub(super) fn supports_story(renderer: &str, story: &str) -> bool {
    match story {
        "states" => {
            matches!(
                renderer,
                "disclosure"
                    | "icon-button"
                    | "link"
                    | "combo-box"
                    | "slider"
                    | "radio-group"
                    | "switch"
                    | "segmented-control"
                    | "breadcrumb"
                    | "command-palette"
            ) || collections_adapter::supports_states(renderer)
                || feedback_adapter::supports_states(renderer)
        }
        "variants" => {
            feedback_adapter::supports_variants(renderer)
                || matches!(renderer, "stack" | "grid" | "separator" | "split-view")
        }
        "behaviors" => matches!(
            renderer,
            "disclosure"
                | "split-view"
                | "icon-button"
                | "link"
                | "combo-box"
                | "slider"
                | "radio-group"
                | "switch"
                | "segmented-control"
                | "breadcrumb"
                | "command-palette"
                | "list"
                | "tree"
                | "data-table"
                | "chart"
                | "tooltip"
                | "popover"
                | "toast"
                | "banner"
                | "progress"
        ),
        _ => false,
    }
}

/// Only route component-owned keys into the production model. In particular,
/// focus on the Docs/Reset tabs or the native text input must never mutate a
/// nearby story control.
pub(super) fn handles_key(
    renderer: &str,
    key: ControlKey,
    focus: Option<usize>,
    s: &ExtendedStory,
) -> bool {
    if collections_adapter::is_collection(renderer) {
        return collections_adapter::accepts_key(renderer, focus);
    }
    if feedback_adapter::supports(renderer) {
        return feedback_adapter::accepts_key(renderer, focus, key);
    }
    if renderer == "command-palette" && focus == Some(20) {
        return matches!(key, ControlKey::Enter | ControlKey::Space);
    }
    if matches!(renderer, "combo-box" | "command-palette") {
        let (open, has_candidates) = if renderer == "combo-box" {
            (s.combo.open, !s.combo.filtered().is_empty())
        } else {
            (s.command.combo.open, !s.command.combo.filtered().is_empty())
        };
        let list_key = matches!(key, ControlKey::Up | ControlKey::Down | ControlKey::Enter);
        return focus == Some(1000)
            && (key == ControlKey::Escape && open || list_key && has_candidates);
    }
    match renderer {
        "popover" => s.popover.open && (focus == Some(20) || key == ControlKey::Escape),
        "tooltip" => (focus == Some(20) || s.hovered == Some(20)) && key == ControlKey::Escape,
        "slider" | "disclosure" | "switch" | "icon-button" | "link" => focus == Some(20),
        "split-view" => focus == Some(90),
        "radio-group" => focus.is_some_and(|id| (30..33).contains(&id)),
        "segmented-control" => focus.is_some_and(|id| (30..33).contains(&id)),
        "breadcrumb" => focus.is_some_and(|id| (30..33).contains(&id)),
        "list" => focus.is_some_and(|id| (30..35).contains(&id)),
        "tree" => focus.is_some_and(|id| (30..33).contains(&id)),
        "data-table" => focus.is_some_and(|id| (30..35).contains(&id) || (20..23).contains(&id)),
        "chart" => focus.is_some_and(|id| (30..34).contains(&id)),
        _ => false,
    }
}

#[derive(Debug)]
pub(super) struct ExtendedStory {
    pub stack_view: StackView,
    pub grid_view: GridView,
    pub separator_view: SeparatorView,
    pub split_view: SplitViewView,
    pub disclosure_view: DisclosureView,
    pub icon_button_view: ButtonSpecimen,
    pub link_view: LinkView,
    pub segmented_view: SegmentedControlView,
    pub breadcrumb_view: BreadcrumbView,
    pub combo_view: ComboBoxView,
    pub command_view: CommandPaletteView,
    collections_views: CollectionsViews,
    feedback_views: FeedbackViews,
    pub disclosure: controls::Disclosure,
    pub split: controls::SplitView,
    pub split_bounds: Rect,
    pub pointer_dragging: Option<usize>,
    pub link: controls::Link,
    pub slider: controls::Slider,
    pub slider_view: SliderView,
    pub radio: controls::RadioGroup,
    pub radio_view: RadioGroupView,
    pub switch: controls::Switch,
    pub switch_view: SwitchView,
    pub segmented: controls::SegmentedControl,
    pub breadcrumb: controls::Breadcrumb,
    pub command: controls::CommandPalette,
    pub combo: controls::ComboBox,
    pub list: controls::List,
    pub tree: controls::Tree,
    pub table: controls::DataTable,
    pub chart: BarChart,
    pub tooltip: controls::Tooltip,
    pub popover: controls::Popover,
    pub banner: controls::Banner,
    pub toast: controls::Toast,
    pub progress: controls::Progress,
    pub markdown: controls::Markdown,
    pub code: controls::CodeBlock,
    pub diff: controls::DiffView,
    pub disabled: bool,
    pub hovered: Option<usize>,
    pub focused: Option<usize>,
    pub visible_rows: Vec<usize>,
    pub hits: Vec<(usize, Rect)>,
    pub rows: Vec<Rect>,
    pub query: String,
    pub selected: usize,
    pub empty_state_created: bool,
}

impl Default for ExtendedStory {
    fn default() -> Self {
        let mut tree = controls::Tree::default();
        let root = tree.add("Workspace", None).unwrap();
        tree.add("Documents", Some(root)).unwrap();
        tree.add("Design system", Some(root)).unwrap();
        tree.toggle(root);
        let mut command = controls::CommandPalette::default();
        command.combo.items = ["Open Settings", "Create workspace", "Search files"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        Self {
            stack_view: StackView::new(controls::Stack::new(controls::Axis::Horizontal)),
            grid_view: GridView::new(controls::Grid::new(3)),
            separator_view: SeparatorView::default(),
            split_view: SplitViewView::new(controls::Axis::Horizontal),
            disclosure_view: DisclosureView::default(),
            icon_button_view: ButtonSpecimen::default(),
            link_view: LinkView::default(),
            segmented_view: SegmentedControlView::new(3),
            breadcrumb_view: BreadcrumbView::default(),
            combo_view: ComboBoxView::default(),
            command_view: CommandPaletteView::default(),
            collections_views: CollectionsViews::default(),
            feedback_views: FeedbackViews::default(),
            disclosure: controls::Disclosure::default(),
            split: controls::SplitView::new(0.5),
            split_bounds: Rect::default(),
            pointer_dragging: None,
            link: controls::Link::default(),
            slider: controls::Slider::new(50., 0., 100., 5.).unwrap(),
            slider_view: SliderView::new(),
            radio: controls::RadioGroup::new(3),
            radio_view: RadioGroupView::new(),
            switch: controls::Switch::default(),
            switch_view: SwitchView::new(),
            segmented: controls::SegmentedControl::new(3),
            breadcrumb: controls::Breadcrumb {
                items: ["Workspace", "Documents", "Drafts"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            },
            command,
            combo: controls::ComboBox {
                items: ["Automatic", "Compact", "Comfortable"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ..Default::default()
            },
            list: controls::List::new(5),
            tree,
            table: controls::DataTable::new(5, 3),
            chart: BarChart::new(
                [
                    ChartDatum::new("Plan", 4.0),
                    ChartDatum::new("Build", 7.0),
                    ChartDatum::new("Review", 5.0),
                    ChartDatum::new("Release", 9.0),
                ]
                .into_iter()
                .collect(),
            ),
            tooltip: controls::Tooltip {
                visible: false,
                text: "A deterministic tooltip fixture".into(),
            },
            popover: controls::Popover::default(),
            banner: controls::Banner {
                message: "Your workspace has unsaved changes.".into(),
                tone: controls::NoticeTone::Info,
                dismissible: true,
                visible: true,
            },
            toast: controls::Toast::new("Changes saved", controls::NoticeTone::Success, 5000),
            progress: controls::Progress::new(0.6),
            markdown: controls::Markdown::parse(
                "# Release notes\n\nA readable paragraph describing this deterministic workspace fixture.\n\n- Stable fixtures\n- Token-driven rendering",
            ),
            code: controls::CodeBlock {
                language: "rust".into(),
                source: "let ready = true;\nprintln!(\"{ready}\");".into(),
            },
            diff: controls::DiffView::between(
                "title = \"Gallery\"\nstatus = \"planned\"",
                "title = \"Gallery\"\nstatus = \"experimental\"",
            ),
            disabled: false,
            hovered: None,
            focused: None,
            visible_rows: Vec::new(),
            hits: Vec::new(),
            rows: Vec::new(),
            query: String::new(),
            selected: 0,
            empty_state_created: false,
        }
    }
}

impl ExtendedStory {
    pub(super) fn collection_focus_frame(&self, renderer: &str, id: usize) -> Option<Rect> {
        collections_adapter::focus_frame(self, renderer, id)
    }

    pub(super) fn tree_view(&self) -> &tekes_ui_controls::collections_view::TreeView {
        &self.collections_views.tree
    }
    pub(super) fn table_view(&self) -> &tekes_ui_controls::collections_view::DataTableView {
        &self.collections_views.table
    }
    pub(super) fn set_tree_expanded(
        &mut self,
        index: usize,
        expanded: bool,
    ) -> tekes_ui_controls::collections_view::CollectionEvent {
        self.collections_views
            .tree
            .set_expanded(index, expanded, &mut self.tree)
    }

    /// Visible feedback body geometry and copy from the actual component layout.
    pub(super) fn feedback_surface(&self, renderer: &str) -> Option<(Rect, String)> {
        feedback_adapter::semantic_surface(self, renderer)
    }

    /// Full document-space component frames from the most recent production
    /// layout. These must not be taken from clipped hit regions: the workbench
    /// uses them to reveal keyboard focus targets that sit outside the viewport.
    pub(super) fn logical_focus_targets(&self, renderer: &str) -> Vec<super::FocusTarget> {
        let mut targets = Vec::new();
        {
            let mut add = |id, frame: Rect, eligible, roving_group, roving_selected| {
                if frame.size.width > 0. && frame.size.height > 0. {
                    targets.push(super::FocusTarget {
                        id,
                        frame,
                        eligible,
                        mounted: true,
                        roving_group,
                        roving_selected,
                    });
                }
            };
            match renderer {
                "split-view" => add(
                    90,
                    self.split_view.layout().divider.frame(),
                    !self.disabled,
                    None,
                    true,
                ),
                "disclosure" => add(
                    20,
                    self.disclosure_view.layout().frame,
                    !self.disabled && !self.disclosure.disabled,
                    None,
                    true,
                ),
                "icon-button" => add(
                    20,
                    self.icon_button_view.layout().frame,
                    !self.disabled
                        && !self.icon_button_view.button().button.state.disabled
                        && !self.icon_button_view.button().button.state.loading,
                    None,
                    true,
                ),
                "link" => add(
                    20,
                    self.link_view.layout().frame,
                    !self.disabled && !self.link.button.state.disabled,
                    None,
                    true,
                ),
                "slider" => add(
                    20,
                    self.slider_view.layout().frame,
                    !self.disabled && !self.slider.disabled,
                    None,
                    true,
                ),
                "radio-group" => {
                    let roving = self
                        .focused
                        .filter(|id| (30..30 + self.radio.len()).contains(id))
                        .map(|id| id - 30)
                        .filter(|index| self.radio.is_enabled(*index))
                        .or(self
                            .radio
                            .selected
                            .filter(|index| self.radio.is_enabled(*index)))
                        .or_else(|| {
                            (0..self.radio.len()).find(|index| self.radio.is_enabled(*index))
                        });
                    for (index, row) in self.radio_view.layout().rows.iter().enumerate() {
                        add(
                            30 + index,
                            row.frame(),
                            !self.disabled && self.radio.is_enabled(index),
                            Some(1),
                            roving == Some(index),
                        );
                    }
                }
                "switch" => add(
                    20,
                    self.switch_view.layout().frame(),
                    !self.disabled && !self.switch.disabled,
                    None,
                    true,
                ),
                "segmented-control" => {
                    let roving = self
                        .focused
                        .filter(|id| (30..33).contains(id))
                        .map(|id| id - 30)
                        .filter(|index| self.segmented.is_enabled(*index))
                        .or_else(|| {
                            self.segmented
                                .is_enabled(self.segmented.selected)
                                .then_some(self.segmented.selected)
                        })
                        .or_else(|| {
                            (0..self.segmented.len())
                                .find(|index| self.segmented.is_enabled(*index))
                        });
                    for (index, item) in self.segmented_view.layout().items.iter().enumerate() {
                        add(
                            30 + index,
                            item.frame,
                            !self.disabled && self.segmented.is_enabled(index),
                            Some(1),
                            roving == Some(index),
                        );
                    }
                }
                "breadcrumb" => {
                    for (index, item) in self
                        .breadcrumb_view
                        .layout()
                        .items
                        .iter()
                        .take(self.breadcrumb.items.len().saturating_sub(1))
                        .enumerate()
                    {
                        add(30 + index, item.frame(), !self.disabled, None, true);
                    }
                }
                "command-palette" => {
                    if let Some(trigger) = self.command_view.trigger_layout() {
                        add(
                            20,
                            trigger.frame,
                            !self.disabled
                                && !self.command.combo.disabled
                                && !self.command.combo.open,
                            None,
                            true,
                        );
                    }
                }
                _ => {}
            }
        }
        for (id, frame) in feedback_adapter::logical_focus_targets(self, renderer) {
            if !targets
                .iter()
                .any(|target: &super::FocusTarget| target.id == id)
                && frame.size.width > 0.
                && frame.size.height > 0.
            {
                targets.push(super::FocusTarget {
                    id,
                    frame,
                    eligible: !self.disabled,
                    mounted: true,
                    roving_group: None,
                    roving_selected: true,
                });
            }
        }
        targets
    }

    pub(super) fn overlay_needs_animation(&self) -> bool {
        self.feedback_views.popover.needs_animation()
    }
    pub(super) fn clear_overlay_presentation(&mut self) {
        let _ = self.feedback_views.popover_close(&mut self.popover);
        self.feedback_views.popover.clear_presentation();
    }

    pub(super) fn set_window_active(&mut self, active: bool) {
        self.split_view.set_window_active(active, &mut self.split);
        self.slider_view.set_window_active(active);
        self.radio_view.set_window_active(active);
        self.switch_view.set_window_active(active);
        self.disclosure_view.set_window_active(active);
        self.icon_button_view.set_window_active(active);
        self.link_view.set_window_active(active);
        self.segmented_view.set_window_active(active);
        self.command_view
            .set_window_active(active, &mut self.command);
        if !active {
            self.combo_view.options.cancel();
        }
        for renderer in ["list", "tree", "data-table", "chart"] {
            collections_adapter::window_active(self, renderer, active);
        }
        let mut ignore = |_| {};
        for renderer in [
            "tooltip",
            "popover",
            "toast",
            "banner",
            "status-badge",
            "progress",
            "empty-state",
        ] {
            let _ = feedback_adapter::window_active(self, renderer, active, &mut ignore);
        }
        if !active {
            // Popover is a transient: window deactivation has already closed
            // its model. Hidden windows may skip painting until reactivation.
            self.clear_overlay_presentation();
            self.cancel_pointer();
            self.focused = None;
        }
    }
    pub(super) fn blur_selection_controls(&mut self) {
        self.cancel_pointer();
        self.slider_view.blur();
        self.radio_view.blur();
        self.switch_view.blur();
        self.disclosure_view.blur();
        self.icon_button_view.blur();
        self.link_view.blur();
        self.segmented_view.blur();
        self.breadcrumb_view.blur();
        for renderer in ["list", "tree", "data-table", "chart"] {
            collections_adapter::blur(self, renderer);
        }
        for renderer in [
            "tooltip",
            "popover",
            "toast",
            "banner",
            "status-badge",
            "progress",
            "empty-state",
        ] {
            feedback_adapter::blur(self, renderer);
        }
        self.focused = None;
    }

    /// Keep the retained selection runtimes in sync with the host's immediate
    /// focus move, even when a frame has not been rebuilt yet.
    pub(super) fn host_focus_changed(
        &mut self,
        renderer: &str,
        target: usize,
        focus_visible: bool,
    ) {
        match renderer {
            "disclosure" => {
                if target == 20 && !self.disabled {
                    self.disclosure_view.focus(focus_visible);
                } else {
                    self.disclosure_view.blur();
                }
            }
            "icon-button" => {
                if target == 20 && !self.disabled {
                    self.icon_button_view.focus(focus_visible);
                } else {
                    self.icon_button_view.blur();
                }
            }
            "link" => {
                if target == 20 && !self.disabled {
                    self.link_view.focus(focus_visible);
                } else {
                    self.link_view.blur();
                }
            }
            "segmented-control" => {
                if let Some(index) = target.checked_sub(30).filter(|i| *i < self.segmented.len()) {
                    self.segmented_view
                        .focus_with_visibility(index, focus_visible);
                } else {
                    self.segmented_view.blur();
                }
            }
            "breadcrumb" => {
                if let Some(index) = target
                    .checked_sub(30)
                    .filter(|i| *i + 1 < self.breadcrumb.items.len())
                {
                    self.breadcrumb_view
                        .focus_with_visibility(index, focus_visible);
                } else {
                    self.breadcrumb_view.blur();
                }
            }
            "list" | "tree" | "data-table" | "chart" => {
                collections_adapter::focus_changed(self, renderer, Some(target), focus_visible);
            }
            "tooltip" | "popover" | "toast" | "banner" | "empty-state" => {
                feedback_adapter::focus_changed(self, renderer, Some(target), focus_visible);
            }
            "slider" => {
                if target == 20 && !self.disabled {
                    self.slider_view.focus(focus_visible);
                } else {
                    self.slider_view.blur();
                }
            }
            "switch" => {
                if target == 20 && !self.disabled {
                    self.switch_view.focus(focus_visible);
                } else {
                    self.switch_view.blur();
                }
            }
            "split-view" if target != 90 => {
                self.cancel_pointer();
            }
            "radio-group" => {
                if (30..30 + self.radio.len()).contains(&target) && !self.disabled {
                    self.radio_view
                        .focus(&self.radio, Some(target - 30), focus_visible);
                } else {
                    self.radio_view.blur();
                }
            }
            _ => {}
        }
        self.focused = Some(target);
    }

    pub(super) fn matrix(
        &mut self,
        e: &Entry,
        s: &mut StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        if navigation_states::supports_states(e.renderer()) {
            navigation_states::matrix(e, s, ctx, scene, c);
            return;
        }
        if collections_adapter::supports_states(e.renderer()) {
            collections_adapter::matrix(e, s, ctx, scene, c);
            return;
        }
        if feedback_adapter::supports_states(e.renderer()) {
            feedback_adapter::matrix(e, s, ctx, scene, c);
            return;
        }
        scene.push_clip(c);
        let focus_gutter = -controls::ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
            .focus_bounds(ctx.text.backing_scale())
            .min_x();
        // Keep the true ancestor clip and reserve the focus ring inside it.
        let content = c.inset(focus_gutter);
        let x = content.min_x();
        let y = content.min_y();
        let width = content.size.width.max(0.);
        let columns = ((width / 300.).floor() as usize).clamp(1, 3);
        let cell_w = (width / columns as f32).max(0.);
        let states: Vec<(&str, ControlState)> = match e.renderer() {
            "disclosure" => vec![
                ("Collapsed · default", ControlState::default()),
                (
                    "Collapsed · hover",
                    ControlState {
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Collapsed · pressed",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Collapsed · focus-visible",
                    ControlState {
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Expanded · default",
                    ControlState {
                        selected: true,
                        ..Default::default()
                    },
                ),
                (
                    "Expanded · hover",
                    ControlState {
                        selected: true,
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Expanded · disabled",
                    ControlState {
                        selected: true,
                        disabled: true,
                        ..Default::default()
                    },
                ),
                (
                    "Disabled",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
            "icon-button" => vec![
                ("Default", ControlState::default()),
                (
                    "Hover",
                    ControlState {
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Pressed",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Focus-visible",
                    ControlState {
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Selected",
                    ControlState {
                        selected: true,
                        ..Default::default()
                    },
                ),
                (
                    "Loading",
                    ControlState {
                        loading: true,
                        ..Default::default()
                    },
                ),
                (
                    "Disabled",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
            "link" => vec![
                ("Default", ControlState::default()),
                (
                    "Hover",
                    ControlState {
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Pressed",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Focus-visible",
                    ControlState {
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Disabled",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
            "segmented-control" => vec![
                ("Selected · default", ControlState::default()),
                (
                    "Selected · alternate",
                    ControlState {
                        selected: true,
                        ..Default::default()
                    },
                ),
                (
                    "Disabled · all segments",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
                (
                    "Selected · hover",
                    ControlState {
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Selected · pressed",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Selected · focus-visible",
                    ControlState {
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Selected · disabled peer",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
            "switch" | "radio-group" => vec![
                (
                    "Checked · default",
                    ControlState {
                        selected: true,
                        ..Default::default()
                    },
                ),
                (
                    "Checked · hover",
                    ControlState {
                        selected: true,
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Checked · pressed",
                    ControlState {
                        selected: true,
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Checked · focus-visible",
                    ControlState {
                        selected: true,
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Checked · disabled",
                    ControlState {
                        selected: true,
                        disabled: true,
                        ..Default::default()
                    },
                ),
                (
                    "Unchecked · error + focus-visible",
                    ControlState {
                        error: true,
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Checked · error",
                    ControlState {
                        error: true,
                        selected: true,
                        ..Default::default()
                    },
                ),
                (
                    "Checked · error + disabled",
                    ControlState {
                        error: true,
                        selected: true,
                        disabled: true,
                        ..Default::default()
                    },
                ),
                ("Unchecked", ControlState::default()),
                (
                    "Unchecked · disabled",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
            "slider" => vec![
                ("Default", ControlState::default()),
                (
                    "Dragging",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Hover",
                    ControlState {
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Pressed",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Focus-visible",
                    ControlState {
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Disabled",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
            _ => vec![
                ("Default", ControlState::default()),
                (
                    "Hover",
                    ControlState {
                        hovered: true,
                        ..Default::default()
                    },
                ),
                (
                    "Pressed",
                    ControlState {
                        pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    "Focus-visible",
                    ControlState {
                        focused: true,
                        focus_visible: true,
                        ..Default::default()
                    },
                ),
                (
                    "Disabled",
                    ControlState {
                        disabled: true,
                        ..Default::default()
                    },
                ),
            ],
        };
        const STATE_HELP: &str =
            "Visual comparison only. Use Playground for real focus and keyboard behavior.";
        let help_height = caption_size(ctx, STATE_HELP, width).height;
        wrapped_caption(ctx, scene, x, y, width, STATE_HELP);
        let state_rows_y = y + 38.0_f32.max(help_height + 10.);
        let title_height = states
            .iter()
            .map(|(title, _)| {
                ctx.text
                    .proposal_label(
                        title,
                        proposal::TextRole::UiBody,
                        Some((cell_w - 12.).max(0.)),
                    )
                    .height as f32
            })
            .fold(20., f32::max);
        // Size each specimen from the same label width and text shaping that
        // its production view uses. In particular, long Link labels wrap at
        // the component's 220pt preview width, while Radio/Switch rows follow
        // their own comfortable-row and first-line metrics.
        let specimen_width = match e.renderer() {
            "icon-button" => ButtonSpecimen::measure(ctx.text)
                .width
                .min((cell_w - 12.).max(0.)),
            // A segmented control is one joined group. Keep its production
            // segment widths useful instead of squeezing three labels into the
            // generic 220pt single-control specimen cap.
            "segmented-control" => (cell_w - 12.).max(0.),
            _ => (cell_w - 12.).clamp(0., 220.),
        };
        let control_height = matrix_control_height(e.renderer(), ctx.text, specimen_width);
        let row_stride = title_height + 8. + control_height + 20.;
        s.document_bottom =
            state_rows_y + states.len().div_ceil(columns) as f32 * row_stride + focus_gutter;
        for (n, (name, state)) in states.into_iter().enumerate() {
            let col = n % columns;
            let row = n / columns;
            let rect = Rect::new(
                x + col as f32 * cell_w,
                state_rows_y + row as f32 * row_stride,
                (cell_w - 12.).max(0.),
                row_stride - 20.,
            );
            label(
                ctx,
                scene,
                Rect::new(rect.min_x(), rect.min_y(), rect.size.width, title_height),
                name,
                false,
            );
            let control = Rect::new(
                rect.min_x(),
                rect.min_y() + title_height + 8.,
                specimen_width,
                control_height,
            );
            let palette = Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
            match e.renderer() {
                "switch" => {
                    let model = controls::Switch {
                        checked: state.selected,
                        disabled: state.disabled,
                    };
                    let mut view = SwitchView::new();
                    view.set_error(state.error);
                    if state.focused {
                        view.focus(state.focus_visible);
                    }
                    let mut probe = Scene::new();
                    let layout = view.render_labeled(
                        &mut probe,
                        ctx.text,
                        model,
                        control,
                        Some(c),
                        palette,
                        "Enabled",
                    );
                    let point = layout.toggle.frame().center();
                    if state.hovered {
                        view.pointer_move(point);
                    }
                    if state.pressed {
                        view.pointer_down(point, &model);
                    }
                    view.render_labeled(
                        scene,
                        ctx.text,
                        model,
                        control,
                        Some(c),
                        palette,
                        "Enabled",
                    );
                }
                "radio-group" => {
                    let mut model = controls::RadioGroup::new(1);
                    model.selected = state.selected.then_some(0);
                    let mut view = RadioGroupView::new();
                    view.set_error(state.error);
                    view.set_disabled(state.disabled);
                    if state.focused {
                        view.focus(&model, Some(0), state.focus_visible);
                    }
                    let mut probe = Scene::new();
                    let layout = view.render(
                        &mut probe,
                        ctx.text,
                        &model,
                        control,
                        Some(c),
                        palette,
                        &["Option"],
                    );
                    let point = layout.rows[0].frame().center();
                    if state.hovered {
                        view.pointer_move(point);
                    }
                    if state.pressed {
                        view.pointer_down(point, &model);
                    }
                    view.render(
                        scene,
                        ctx.text,
                        &model,
                        control,
                        Some(c),
                        palette,
                        &["Option"],
                    );
                }
                "slider" => {
                    let mut slider =
                        controls::Slider::new(50., 0., 100., 5.).expect("valid fixture");
                    slider.disabled = state.disabled;
                    let mut view = SliderView::new();
                    if state.focused {
                        view.focus(state.focus_visible);
                    }
                    let mut probe = Scene::new();
                    let layout = view.render(&mut probe, slider, control, Some(c), palette);
                    if state.hovered {
                        view.pointer_move(layout.thumb.center(), &mut slider);
                    }
                    if state.pressed {
                        view.pointer_down(layout.thumb.center(), &mut slider);
                    }
                    if name == "Dragging" {
                        view.pointer_move(
                            Point::new(
                                layout.track.min_x() + layout.track.size.width * 0.75,
                                layout.track.center().y,
                            ),
                            &mut slider,
                        );
                    }
                    view.render(scene, slider, control, Some(c), palette);
                }
                "disclosure" => {
                    let model = controls::Disclosure {
                        open: state.selected,
                        disabled: state.disabled,
                    };
                    let mut view = DisclosureView::default();
                    if state.focused {
                        view.focus(state.focus_visible);
                    }
                    let mut probe = Scene::new();
                    let layout = view.render(
                        &mut probe,
                        ctx.text,
                        &model,
                        "Details",
                        control,
                        Some(c),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    if state.hovered {
                        view.pointer_move(layout.frame.center());
                    }
                    if state.pressed {
                        view.pointer_down(layout.frame.center());
                    }
                    view.render(
                        scene,
                        ctx.text,
                        &model,
                        "Details",
                        control,
                        Some(c),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    let mut icons = GalleryIconRenderer::native(ctx);
                    view.render_icon(scene, &mut icons, &model, Some(c), palette, 16.);
                }
                "link" => {
                    let mut model = controls::Link::default();
                    model.button.state.disabled = state.disabled;
                    let mut view = LinkView::default();
                    if state.focused {
                        view.focus(state.focus_visible);
                    }
                    let mut probe = Scene::new();
                    let layout = view.render(
                        &mut probe,
                        ctx.text,
                        &mut model,
                        "Open documentation",
                        control,
                        Some(c),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    if state.hovered {
                        view.pointer_move(layout.frame.center());
                    }
                    if state.pressed {
                        view.pointer_down(layout.frame.center());
                    }
                    view.render(
                        scene,
                        ctx.text,
                        &mut model,
                        "Open documentation",
                        control,
                        Some(c),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                }
                "segmented-control" => {
                    let mut model = controls::SegmentedControl::new(3);
                    if state.selected {
                        model.select(1);
                    }
                    if state.disabled && name != "Disabled · all segments" {
                        model.set_enabled(2, false);
                    }
                    let mut view = SegmentedControlView::new(3);
                    view.set_disabled(state.disabled && name == "Disabled · all segments");
                    if state.focused {
                        view.focus_with_visibility(0, state.focus_visible);
                    }
                    let labels = ["General", "Advanced", "Security"];
                    let mut probe = Scene::new();
                    let layout = view.render(
                        &mut probe,
                        ctx.text,
                        &mut model,
                        &labels,
                        control,
                        Some(c),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    let point = layout.items[0].frame.center();
                    if state.hovered {
                        view.pointer_move(point);
                    }
                    if state.pressed {
                        view.pointer_down(point);
                    }
                    view.render(
                        scene,
                        ctx.text,
                        &mut model,
                        &labels,
                        control,
                        Some(c),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                }
                "icon-button" => {
                    let mut view=ButtonSpecimen::new("Create item");
                    view.button_mut().button.state=state;
                    let mut probe=Scene::new();
                    let layout=view.render(&mut probe,ctx.text,control,Some(c),palette);
                    if state.focused {view.focus(state.focus_visible);}
                    if state.hovered {view.pointer_move(layout.frame.center());}
                    if state.pressed {view.pointer_down(layout.frame.center());}
                    view.render(scene,ctx.text,control,Some(c),palette);
                    view.draw_icon(scene,&mut GalleryIconRenderer::native(ctx))
                        .expect("Native Gallery IconButton must emit artwork");
                }
                _ => {}
            }
        }
        scene.pop_clip();
        #[cfg(any(test, feature = "composition-review"))]
        if e.renderer() == "slider" {
            use tekes_ui_composition_consumer::slider_content::{RANGE, ROOT, build};
            use tekes_ui_controls::composition::{AccessibilityAction, ContentEvent};
            let mut sample = build().expect("independent public range composition");
            sample.focus(Some(RANGE), true);
            sample.accessibility_action(RANGE, None, AccessibilityAction::Increment);
            sample.event(&ContentEvent::Key {
                key: ControlKey::Right,
                down: true,
            });
            let available = c.inset(tekes_ui_tokens::Spacing::Regular.value());
            let w = available.size.width.min(420.);
            let size = sample
                .get_mut(ROOT)
                .unwrap()
                .measure(ctx.text, tekes_ui_scene::Size::new(w, 4096.))
                .size;
            let frame = Rect::new(c.min_x() + 12., s.document_bottom + 24., w, size.height);
            sample
                .render(
                    scene,
                    ctx.text,
                    frame,
                    c,
                    Palette::new(ctx.theme, s.brand).expect("registered Gallery brand"),
                )
                .unwrap();
            s.document_bottom = frame.max_y() + 12.;
        }
        #[cfg(any(test, feature = "composition-review"))]
        if e.renderer() == "disclosure" {
            s.document_bottom = super::content_status::matrix(
                tekes_ui_composition_consumer::content_status::Kind::Disclosure,
                s,
                ctx,
                scene,
                c,
                s.document_bottom + 24.,
            );
        }
    }

    pub(super) fn behaviors(
        &mut self,
        e: &Entry,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        let text = match e.renderer() {
            "disclosure" => {
                "Enter or Space toggles the disclosure. Collapsed content is not mounted in the fixture."
            }
            "split-view" => {
                "Drag the divider after its 4 pt dead zone. Arrow keys adjust; Home and End set limits. Escape restores the starting ratio. Constrained widths expose master/detail pane selection."
            }
            "icon-button" => {
                "Accessible name: Create item. Enter and Space activate; pointer activation requires press and release on the same control."
            }
            "link" => "Enter follows the link action. Space does not activate a link.",
            "combo-box" => {
                "Type to filter. Arrow keys move the active option, Enter commits, Escape closes. Marked IME composition is passed to the native text input first."
            }
            "slider" => {
                "Left and Right adjust by the declared step; Home and End move to the range bounds. Pointer drag updates the value."
            }
            "radio-group" | "segmented-control" => {
                "Arrow keys move selection; Space selects the focused item. Disabled items are skipped."
            }
            "switch" => "Space toggles the focused value. Disabled state prevents changes.",
            "breadcrumb" => {
                "Activate a crumb to navigate to its ancestor. It does not consume Space as a link activation."
            }
            "command-palette" => {
                "Open the palette, type to filter, use arrow keys to move, Enter to run, and Escape to close."
            }
            "list" => {
                "Single selection. Up and Down move; Home and End select the first and last row."
            }
            "tree" => {
                "Up and Down move among visible rows. Right expands or enters the first child; Left collapses or moves to the parent."
            }
            "data-table" => {
                "Activate a header to sort deterministically. Up and Down move through rows; Enter selects the focused row."
            }
            "chart" => {
                "Arrow keys move the selected data mark. Non-finite values are omitted and are not selectable."
            }
            "tooltip" => {
                "Hover or focus-visible reveals the tooltip. Escape dismisses it until pointer leave or focus changes."
            }
            "popover" => {
                "Activate the trigger, then Escape or click outside. Dismissal restores focus to the invoker."
            }
            "toast" => {
                "Dismiss or advance the deterministic fixture timer by one second. No wall-clock timer is used."
            }
            "banner" => "Dismiss the notice; Reset restores its initial visible state.",
            "progress" => "Advance the value in 10% steps. The value is clamped to 100%.",
            _ => "This component is a static rendering specimen in this release.",
        };
        label(ctx, scene, c, text, false);
    }

    pub(super) fn variants(
        &mut self,
        e: &Entry,
        s: &mut StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        if feedback_adapter::supports_variants(e.renderer()) {
            feedback_adapter::variants(e, s, ctx, scene, c);
            return;
        }
        let palette = Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
        let x = c.min_x();
        let y = c.min_y();
        let w = c.size.width.max(0.);
        match e.renderer() {
            "stack" => {
                for (index, gap) in [0., 8., 16.].into_iter().enumerate() {
                    let row = Rect::new(x, y + index as f32 * 72., w.min(620.), 58.);
                    label(
                        ctx,
                        scene,
                        Rect::new(row.min_x(), row.min_y(), row.size.width, 18.),
                        &format!("Gap: {gap:.0} pt"),
                        false,
                    );
                    self.stack_view.model_mut().gap = gap;
                    for cell in self.stack_view.model().layout(
                        Rect::new(row.min_x(), row.min_y() + 20., row.size.width, 36.),
                        3,
                    ) {
                        paint::surface_at_scale(
                            scene,
                            cell,
                            palette,
                            true,
                            ctx.text.backing_scale(),
                        );
                    }
                }
            }
            "grid" => {
                for (index, cols) in [2, 3, 4].into_iter().enumerate() {
                    let top = y + index as f32 * 82.;
                    label(
                        ctx,
                        scene,
                        Rect::new(x, top, w, 18.),
                        &format!("{cols} columns"),
                        false,
                    );
                    self.grid_view.model_mut().columns = cols;
                    for cell in self.grid_view.model().layout(
                        Rect::new(x, top + 22., w.min(620.), 58.),
                        cols * 2,
                        26.,
                    ) {
                        paint::surface_at_scale(
                            scene,
                            cell,
                            palette,
                            true,
                            ctx.text.backing_scale(),
                        );
                    }
                }
            }
            "separator" => {
                let horizontal_h = ctx
                    .text
                    .proposal_label("Horizontal", proposal::TextRole::UiBody, Some(w))
                    .height as f32;
                label(
                    ctx,
                    scene,
                    Rect::new(x, y, w, horizontal_h),
                    "Horizontal",
                    false,
                );
                let horizontal_y = y + horizontal_h + 8.;
                controls::separator_at_scale(
                    scene,
                    Rect::new(x, horizontal_y, w.min(620.), 12.),
                    palette,
                    controls::Axis::Horizontal,
                    ctx.text.backing_scale(),
                );
                let vertical_y = horizontal_y + 28.;
                let vertical_h = ctx
                    .text
                    .proposal_label("Vertical", proposal::TextRole::UiBody, Some(w))
                    .height as f32;
                label(
                    ctx,
                    scene,
                    Rect::new(x, vertical_y, w, vertical_h),
                    "Vertical",
                    false,
                );
                controls::separator_at_scale(
                    scene,
                    Rect::new(x, vertical_y + vertical_h + 8., 12_f32.min(w), 160.),
                    palette,
                    controls::Axis::Vertical,
                    ctx.text.backing_scale(),
                );
            }
            "split-view" => {
                for (index, ratio) in [0.3, 0.5, 0.7].into_iter().enumerate() {
                    let bounds = Rect::new(x, y + index as f32 * 110. + 20., w.min(620.), 78.);
                    label(
                        ctx,
                        scene,
                        Rect::new(x, bounds.min_y() - 20., w, 18.),
                        &format!("Leading {:.0}%", ratio * 100.),
                        false,
                    );
                    self.split.set_ratio(ratio);
                    let (leading, divider, trailing) =
                        self.split.layout(bounds, controls::Axis::Horizontal);
                    paint::surface_at_scale(
                        scene,
                        leading,
                        palette,
                        true,
                        ctx.text.backing_scale(),
                    );
                    controls::separator(scene, divider, palette, controls::Axis::Vertical);
                    paint::surface_at_scale(
                        scene,
                        trailing,
                        palette,
                        true,
                        ctx.text.backing_scale(),
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn activate(
        &mut self,
        renderer: &str,
        id: usize,
        log: &mut impl FnMut(String),
    ) -> bool {
        if self.disabled && !matches!(id, 0 | 10) {
            return false;
        }
        if collections_adapter::is_collection(renderer) {
            // Activation success is consumption, not a selection delta: Enter
            // may activate the already selected row and emit a real action.
            return collections_adapter::activate(self, renderer, id, log)
                .is_some_and(|result| result.consumed);
        }
        if feedback_adapter::supports(renderer) {
            return feedback_adapter::activate(self, renderer, id, log)
                .is_some_and(|result| result.changed);
        }
        match (renderer, id) {
            (_, 10) => {
                self.disabled = !self.disabled;
                log(format!("disabled: {}", self.disabled));
                true
            }
            ("disclosure", 20) => {
                let changed = self.disclosure_view.activate(&mut self.disclosure);
                log(format!("disclosure: {}", self.disclosure.open));
                changed
            }
            ("split-view", 91) => {
                self.split.choose_visible(controls::Pane::Leading);
                log("split pane: leading".into());
                true
            }
            ("split-view", 92) => {
                self.split.choose_visible(controls::Pane::Trailing);
                log("split pane: trailing".into());
                true
            }
            ("icon-button", 20) => {
                let changed = self.icon_button_view.activate();
                if changed {
                    log(format!(
                        "icon-button: {}",
                        self.icon_button_view.name()
                    ));
                }
                changed
            }
            ("link", 20) => {
                let changed = self.link_view.activate(&mut self.link);
                if changed {
                    log("link: activated".into());
                }
                changed
            }
            ("slider", 20) => {
                let changed = self.slider.key(ControlKey::Right);
                if changed {
                    log(format!("value: {}", self.slider.value()));
                }
                changed
            }
            ("radio-group", 30..=32) => {
                let changed = self.radio.select(id - 30);
                if changed {
                    log(format!("radio: {}", id - 30));
                }
                changed
            }
            ("switch", 20) => {
                let changed = self.switch.toggle();
                if changed {
                    log(format!("switch: {}", self.switch.checked));
                }
                changed
            }
            ("segmented-control", 30..=32) => {
                let changed = self.segmented_view.select(id - 30, &mut self.segmented);
                if changed {
                    log(format!("segment: {}", id - 30));
                }
                changed
            }
            ("breadcrumb", 30..=32) => {
                let target = self.breadcrumb_view.activate(id - 30, &self.breadcrumb);
                if let Some(n) = target {
                    self.selected = n;
                    log(format!("breadcrumb: {n}"));
                    true
                } else {
                    false
                }
            }
            ("command-palette", 20) => {
                self.command.open();
                log("command palette: open".into());
                true
            }
            ("command-palette", 40..=42) => {
                let Some(index) = self
                    .command_view
                    .activate_option(id - 40, &mut self.command)
                else {
                    return false;
                };
                log(format!("command: {index}"));
                true
            }
            ("combo-box", 40..=42) => {
                let Some(i) = self.combo_view.activate_option(id - 40, &mut self.combo) else {
                    return false;
                };
                log(format!("combo selection: {i}"));
                true
            }
            _ => false,
        }
    }

    pub(super) fn key(
        &mut self,
        renderer: &str,
        key: ControlKey,
        down: bool,
        log: &mut impl FnMut(String),
    ) -> bool {
        if collections_adapter::is_collection(renderer) {
            if self.disabled {
                self.cancel_pointer();
                return true;
            }
            return collections_adapter::key(self, renderer, self.focused, key, down, log)
                .is_some_and(|result| result.consumed);
        }
        if feedback_adapter::supports(renderer) {
            if self.disabled {
                self.cancel_pointer();
                return true;
            }
            return feedback_adapter::key(self, renderer, self.focused, key, down, log)
                .is_some_and(|result| result.consumed);
        }
        let component_event = match renderer {
            "icon-button" => Some(self.icon_button_view.key_event(key, down)),
            "link" => Some(self.link_view.key(key, down, &mut self.link)),
            "disclosure" => Some(
                self.disclosure_view
                    .key_event(key, down, &mut self.disclosure),
            ),
            "segmented-control" => Some(self.segmented_view.key_event(
                key,
                down,
                &mut self.segmented,
            )),
            "breadcrumb" => {
                let (consumed, activated) =
                    self.breadcrumb_view.key_event(key, down, &self.breadcrumb);
                if let Some(index) = activated {
                    self.selected = index;
                    log(format!("breadcrumb: {index}"));
                }
                return consumed;
            }
            _ => None,
        };
        if let Some(event) = component_event {
            if event.changed {
                let message = match renderer {
                    "disclosure" => format!("disclosure: {}", self.disclosure.open),
                    "segmented-control" => format!("segment: {}", self.segmented.selected),
                    "link" => "link: activated".to_owned(),
                    "icon-button" => format!("icon-button: {}", self.icon_button_view.name()),
                    _ => format!("{renderer}: keyboard state changed"),
                };
                log(message);
            }
            return event.consumed;
        }
        if !down {
            return match renderer {
                "icon-button" => self.icon_button_view.key(key, false),
                "link" => self.link_view.key(key, false, &mut self.link).consumed,
                "disclosure" => self.disclosure_view.key(key, false, &mut self.disclosure),
                "segmented-control" => self.segmented_view.key(key, false, &mut self.segmented),
                "breadcrumb" => self
                    .breadcrumb_view
                    .key(key, false, &self.breadcrumb)
                    .is_some(),
                "slider" => self.slider_view.key(key, false, &mut self.slider),
                "radio-group" => !matches!(
                    self.radio_view.key(key, false, &mut self.radio),
                    SelectionEvent::Ignored
                ),
                "switch" => !matches!(
                    self.switch_view.key(key, false, &mut self.switch),
                    SelectionEvent::Ignored
                ),
                "combo-box" => {
                    self.combo_view
                        .key_event(key, false, &mut self.combo)
                        .consumed
                }
                "command-palette" if self.focused == Some(20) => {
                    self.command_view
                        .trigger_key(key, false, &mut self.command)
                        .consumed
                }
                "command-palette" => {
                    self.command_view
                        .key_event(key, false, &mut self.command)
                        .consumed
                }
                _ => supports(renderer),
            };
        }
        if renderer == "split-view" && matches!(key, ControlKey::Enter | ControlKey::Space) {
            self.pointer_dragging = None;
        }
        if renderer == "split-view" && key == ControlKey::Escape {
            let captured = self.pointer_dragging.take() == Some(90);
            return self.split_view.cancel(&mut self.split) || captured;
        }
        if self.disabled {
            self.cancel_pointer();
            return true;
        }
        let changed = match renderer {
            "slider" => self.slider_view.key(key, true, &mut self.slider),
            "radio-group" => {
                self.radio_view.key(key, true, &mut self.radio) != SelectionEvent::Ignored
            }
            "segmented-control" => self.segmented_view.key(key, true, &mut self.segmented),
            "split-view" => self.split_view.key(key, &mut self.split),
            "combo-box" => {
                let event = self.combo_view.key_event(key, true, &mut self.combo);
                if key == ControlKey::Enter && event.changed && !self.combo.open {
                    log(format!(
                        "combo selection: {}",
                        self.combo.selected.unwrap_or_default()
                    ));
                }
                event.consumed
            }
            "command-palette" => {
                if self.focused == Some(20) {
                    self.command_view
                        .trigger_key(key, true, &mut self.command)
                        .consumed
                } else {
                    let event = self.command_view.key_event(key, true, &mut self.command);
                    if key == ControlKey::Enter && event.changed && !self.command.combo.open {
                        log(format!(
                            "command: {}",
                            self.command.combo.selected.unwrap_or_default()
                        ));
                    }
                    event.consumed
                }
            }
            "link" if key == ControlKey::Enter => {
                self.link_view.key(key, true, &mut self.link).consumed
            }
            "icon-button" => self.icon_button_view.key(key, true),
            "disclosure" if matches!(key, ControlKey::Enter | ControlKey::Space) => {
                self.disclosure_view.key(key, true, &mut self.disclosure)
            }
            "switch" => {
                self.switch_view.key(key, true, &mut self.switch) != SelectionEvent::Ignored
            }
            "popover" if key == ControlKey::Escape && self.popover.open => {
                self.focused = self.popover.close();
                true
            }
            _ => return false,
        };
        if changed {
            match renderer {
                "radio-group" => self.focused = self.radio_view.focused().map(|i| 30 + i),
                "segmented-control" => self.focused = Some(30 + self.segmented.selected),
                _ => {}
            }
            log(format!("{}: keyboard state changed", renderer));
        }
        true
    }

    pub(super) fn draw(
        &mut self,
        e: &Entry,
        s: &mut StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        let story_hit_start = s.hits.len();
        self.hits.clear();
        self.rows.clear();
        let original_clip = s.button_clip;
        let paint_clip = controls::ControlFrame::new(c, None)
            .focus_bounds(ctx.text.scale())
            .intersection(&original_clip.unwrap_or(c))
            .unwrap_or(Rect::new(c.min_x(), c.min_y(), 0., 0.));
        scene.push_clip(paint_clip);
        let x = c.min_x();
        let y = c.min_y();
        let w = c.size.width;
        let has_disabled_toggle = supports_disabled(e.renderer());
        let body_top = if has_disabled_toggle { 46. } else { 0. };
        if has_disabled_toggle {
            quiet_action(
                s,
                ctx,
                scene,
                10,
                Rect::new(x, y, 112., 32.),
                if self.disabled { "Enabled" } else { "Disabled" },
                (self.disabled, false),
            );
        }
        let body = Rect::new(
            x,
            y + body_top,
            w,
            (c.max_y() - y - body_top - 116.).max(0.),
        );
        let control_clip = controls::ControlFrame::new(body, None)
            .focus_bounds(ctx.text.scale())
            .intersection(&paint_clip)
            .unwrap_or(Rect::new(body.min_x(), body.min_y(), 0., 0.));
        // Reserve paint space for focus; content and event hit bounds remain
        // the original body. No paint escapes the actual preview viewport.
        s.button_clip = Some(control_clip);
        scene.push_clip(control_clip);
        let bx = body.min_x();
        let by = body.min_y();
        let bw = body.size.width;
        let palette = Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
        if self.disabled && self.pointer_dragging.is_some() {
            self.cancel_pointer();
        }
        self.disclosure.disabled = self.disabled;
        self.slider.disabled = self.disabled;
        self.switch.disabled = self.disabled;
        if self.disabled {
            self.slider_view.cancel();
            self.radio_view.cancel();
            self.switch_view.cancel();
        }
        self.radio_view.set_disabled(self.disabled);
        if self.focused == Some(20) {
            if !self.slider_view.is_focused() {
                self.slider_view.focus(
                    s.buttons
                        .get(&20)
                        .is_some_and(|button| button.state.focus_visible),
                );
            }
            if !self.switch_view.is_focused() {
                self.switch_view.focus(
                    s.buttons
                        .get(&20)
                        .is_some_and(|button| button.state.focus_visible),
                );
            }
        } else {
            self.slider_view.blur();
            self.switch_view.blur();
        }
        if self.focused.is_some_and(|id| (30..33).contains(&id)) {
            if self.radio_view.focused() != self.focused.map(|id| id - 30) {
                let focus_visible = self.focused.is_some_and(|id| {
                    s.buttons
                        .get(&id)
                        .is_some_and(|button| button.state.focus_visible)
                });
                self.radio_view
                    .focus(&self.radio, self.focused.map(|id| id - 30), focus_visible);
            }
        } else {
            self.radio_view.blur();
        }
        self.icon_button_view.button_mut().button.state.disabled = self.disabled;
        self.link.button.state.disabled = self.disabled;
        self.segmented_view.set_disabled(self.disabled);
        self.breadcrumb_view.set_disabled(self.disabled);
        self.chart
            .set_focused(self.focused.is_some(), self.focused.is_some());
        let rendered_by_component_adapter =
            collections_adapter::build(
                self,
                e,
                s,
                ctx,
                scene,
                body,
                original_clip.unwrap_or(control_clip),
                palette,
            ) || feedback_adapter::build(self, e, s, ctx, scene, body, control_clip, palette);
        if !rendered_by_component_adapter {
            match e.renderer() {
                "stack" => {
                    self.stack_view.model_mut().axis = controls::Axis::Horizontal;
                    self.stack_view.model_mut().gap = 8.;
                    let items = self
                        .stack_view
                        .render(
                            Rect::new(bx, by + 20., bw.min(650.), 64.),
                            3,
                            Some(control_clip),
                        )
                        .items
                        .clone();
                    for (n, item) in items.into_iter().enumerate() {
                        let r = item.frame();
                        if let Some(visible) = item.visible_frame() {
                            scene.push_clip(visible);
                            paint::surface_at_scale(
                                scene,
                                r.inset(0.5),
                                palette,
                                true,
                                ctx.text.backing_scale(),
                            );
                            label(
                                ctx,
                                scene,
                                r.inset(10.),
                                ["Leading", "Center", "Trailing"][n],
                                false,
                            );
                            scene.pop_clip();
                        }
                    }
                    wrapped_caption(
                        ctx,
                        scene,
                        bx,
                        by + 96.,
                        bw,
                        "Stack divides the available main axis evenly; gap is 8 pt.",
                    );
                }
                "grid" => {
                    self.grid_view.model_mut().gap_x = 8.;
                    self.grid_view.model_mut().gap_y = 8.;
                    let items = self
                        .grid_view
                        .render(
                            Rect::new(bx, by + 16., bw.min(720.), (body.size.height - 28.).max(0.)),
                            9,
                            58.,
                            Some(control_clip),
                        )
                        .items
                        .clone();
                    for (n, item) in items.into_iter().enumerate() {
                        let r = item.frame();
                        if let Some(visible) = item.visible_frame() {
                            scene.push_clip(visible);
                            paint::surface_at_scale(
                                scene,
                                r.inset(0.5),
                                palette,
                                true,
                                ctx.text.backing_scale(),
                            );
                            label(
                                ctx,
                                scene,
                                r.inset(8.),
                                &format!("Item {:02}", n + 1),
                                false,
                            );
                            scene.pop_clip();
                        }
                    }
                }
                "separator" => {
                    label(
                        ctx,
                        scene,
                        Rect::new(bx, by + 4., bw.min(300.), 28.),
                        "Account settings",
                        true,
                    );
                    self.separator_view.render_at_scale(
                        scene,
                        Rect::new(bx, by + 42., bw.min(600.), 12.),
                        controls::Axis::Horizontal,
                        Some(control_clip),
                        palette,
                        ctx.text.backing_scale(),
                    );
                    label(
                        ctx,
                        scene,
                        Rect::new(bx, by + 62., bw.min(500.), 28.),
                        "Profile and preferences",
                        false,
                    );
                    self.separator_view.render_at_scale(
                        scene,
                        Rect::new(
                            bx + (bw - 12.).clamp(0., 300.),
                            by + 110.,
                            12_f32.min(bw),
                            160.,
                        ),
                        controls::Axis::Vertical,
                        Some(control_clip),
                        palette,
                        ctx.text.backing_scale(),
                    );
                }
                "disclosure" => {
                    self.disclosure_view.set_window_active(s.window_active);
                    self.disclosure.disabled = self.disabled;
                    let l = self.disclosure_view.render(
                        scene,
                        ctx.text,
                        &self.disclosure,
                        "Details",
                        Rect::new(bx, by + 12., bw.min(500.), 40.),
                        Some(control_clip),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    let mut renderer = GalleryIconRenderer::native(ctx);
                    self.disclosure_view.render_icon(
                        scene,
                        &mut renderer,
                        &self.disclosure,
                        Some(control_clip),
                        palette,
                        16.,
                    );
                    if let Some(hit) = l.visible {
                        s.hits.push((20, hit));
                    }
                    let body_copy = "Disclosure content is mounted only while open. State remains local to this story.";
                    let body_width = bw.min(500.);
                    let copy_width = (body_width - 30.).max(0.);
                    let copy_height = ctx
                        .text
                        .proposal_label(body_copy, proposal::TextRole::UiBody, Some(copy_width))
                        .height as f32;
                    let body_height = 92_f32.max(copy_height + 24.);
                    let body_layout = self.disclosure_view.layout_body(
                        ctx.text,
                        &self.disclosure,
                        Rect::new(bx, l.frame.max_y() + 8., body_width, body_height),
                        Some(control_clip),
                    );
                    if let Some(body_clip) = body_layout.visible_clip {
                        scene.push_clip(body_clip);
                        paint::surface_at_scale(
                            scene,
                            body_layout.body_rect,
                            palette,
                            true,
                            ctx.text.backing_scale(),
                        );
                        label(
                            ctx,
                            scene,
                            Rect::new(
                                body_layout.body_rect.origin.x + 14.,
                                body_layout.body_rect.origin.y + 12.,
                                (body_layout.body_rect.size.width - 30.).max(0.),
                                (body_layout.body_rect.size.height - 24.).max(0.),
                            ),
                            body_copy,
                            false,
                        );
                        scene.pop_clip();
                    }
                }
                "split-view" => {
                    let split_help = format!(
                        "Divider position: {:.0}% · drag, arrow keys, Home / End; double-click or Enter / Space resets; Escape restores the drag start.",
                        self.split.ratio() * 100.
                    );
                    let help_height = caption_size(ctx, &split_help, bw).height;
                    let bounds = Rect::new(
                        bx,
                        by + 8.,
                        bw.min(760.),
                        (body.size.height - help_height - 16.).max(0.),
                    );
                    self.split_bounds = bounds;
                    let split_layout = self.split_view.render_at_scale(
                        scene,
                        &mut self.split,
                        bounds,
                        Some(control_clip),
                        palette,
                        ctx.text.backing_scale(),
                    );
                    let visible_frame = split_layout.frame.visible_frame();
                    if self.pointer_dragging == Some(90)
                        && (visible_frame.is_none_or(|visible| {
                            visible.size.width <= 0.0 || visible.size.height <= 0.0
                        }) || self
                            .split
                            .is_constrained(bounds, controls::Axis::Horizontal))
                    {
                        self.cancel_pointer();
                    }
                    let leading = split_layout.leading.frame();
                    let trailing = split_layout.trailing.frame();
                    paint::surface_at_scale(
                        scene,
                        leading,
                        palette,
                        true,
                        ctx.text.backing_scale(),
                    );
                    paint::surface_at_scale(
                        scene,
                        trailing,
                        palette,
                        true,
                        ctx.text.backing_scale(),
                    );
                    if !self
                        .split
                        .is_constrained(bounds, controls::Axis::Horizontal)
                    {
                        let divider_hit = split_layout.divider_hit;
                        if let Some(divider_hit) = divider_hit {
                            s.hits.push((90, divider_hit));
                        }
                    } else {
                        quiet_action(
                            s,
                            ctx,
                            scene,
                            91,
                            Rect::new(bx, by + 8., 112., 32.),
                            "Show master",
                            (self.split.visible_pane() == controls::Pane::Leading, false),
                        );
                        quiet_action(
                            s,
                            ctx,
                            scene,
                            92,
                            Rect::new(bx + 120., by + 8., 112., 32.),
                            "Show detail",
                            (self.split.visible_pane() == controls::Pane::Trailing, false),
                        );
                    }
                    label(ctx, scene, leading.inset(12.), "Master pane", false);
                    label(ctx, scene, trailing.inset(12.), "Detail pane", false);
                    wrapped_caption(ctx, scene, bx, bounds.max_y() + 8., bw, &split_help);
                }
                "icon-button" | "link" => {
                    let is_link = e.renderer() == "link";
                    self.icon_button_view.button_mut().button.state.disabled = self.disabled;
                    self.link.button.state.disabled = self.disabled;
                    let natural = ButtonSpecimen::measure(ctx.text);
                    let r = Rect::new(
                        bx,
                        by + 18.,
                        if is_link { 260. } else { natural.width },
                        if is_link { 40. } else { natural.height },
                    );
                    let component = if is_link {
                        s.button_labels.insert(20, "Open documentation ↗".into());
                        self.link_view.render(
                            scene,
                            ctx.text,
                            &mut self.link,
                            "Open documentation ↗",
                            r,
                            Some(control_clip),
                            palette,
                            proposal::TextRole::UiBody,
                        )
                    } else {
                        let component = self.icon_button_view.render(
                            scene,
                            ctx.text,
                            r,
                            Some(control_clip),
                            palette,
                        );
                        self.icon_button_view.draw_icon(
                            scene,
                            &mut GalleryIconRenderer::native(ctx),
                        ).expect("Native Gallery IconButton must emit artwork");
                        component
                    };
                    let r = component.frame;
                    if let Some(hit) = component.visible {
                        s.hits.push((20, hit));
                    }
                    let help = if is_link {
                        "Activates with Enter. Space is not a link activation key.".to_owned()
                    } else {
                        format!("Accessible name: {}", self.icon_button_view.name())
                    };
                    wrapped_caption(ctx, scene, bx, r.max_y() + 12., bw, &help);
                }
                "slider" => {
                    let control = Rect::new(bx, by + 28., bw.min(440.), 36.);
                    let layout = self.slider_view.render(
                        scene,
                        self.slider,
                        control,
                        Some(control_clip),
                        palette,
                    );
                    if let Some(hit) = layout.visible {
                        s.hits.push((20, hit));
                    } else if self.pointer_dragging == Some(20) {
                        self.pointer_dragging = None;
                    }
                    label(
                        ctx,
                        scene,
                        Rect::new(bx, by + 10., 160., 26.),
                        &format!("Value: {:.0}", self.slider.value()),
                        false,
                    );
                    wrapped_caption(
                        ctx,
                        scene,
                        bx,
                        control.max_y() + 8.,
                        bw.min(540.),
                        "Arrow keys change by 5; Home / End move to bounds. Drag the track to set a value.",
                    );
                }
                "combo-box" => {
                    let input_height = 38.0_f32.max(s.input.measure_search_height(ctx.text));
                    let input = Rect::new(bx, by + 8., bw.min(440.), input_height);
                    s.input.set_disabled(self.disabled);
                    s.input
                        .build_search_clipped(ctx, input, s.button_clip, scene);
                    if let Some(frame) = s.input.frame_at(0) {
                        s.register_logical_focus(1000, frame, !self.disabled, true, None, true);
                    }
                    self.combo.disabled = self.disabled;
                    if self.combo.query != s.input.query() {
                        self.combo.set_query(s.input.query());
                    }
                    self.query = self.combo.query.clone();
                    if let Some(visible) = s.input.visible_frame() {
                        self.hits.push((1000, visible));
                        s.hits.push((1000, visible));
                    }
                    let options = self.combo_view.render(
                        scene,
                        ctx.text,
                        &mut self.combo,
                        input,
                        body,
                        Some(control_clip),
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    );
                    if let Some(options) = options {
                        for (filtered_index, item) in options.items.iter().enumerate() {
                            if let Some(index) =
                                self.combo_view.options.original_index(filtered_index)
                                && let Some(hit) = item.visible
                            {
                                s.hits.push((40 + index, hit));
                            }
                        }
                    }
                    if self.combo.open && self.combo.filtered().is_empty() {
                        wrapped_caption(
                            ctx,
                            scene,
                            bx,
                            input.max_y() + 8.,
                            bw.min(440.),
                            "No matching options",
                        );
                    }
                }
                "radio-group" => {
                    let labels: Vec<_> = (0..self.radio.len())
                        .map(|index| format!("Option {}", index + 1))
                        .collect();
                    let label_refs: Vec<_> = labels.iter().map(String::as_str).collect();
                    let layout = self.radio_view.render(
                        scene,
                        ctx.text,
                        &self.radio,
                        Rect::new(bx, by, bw.min(300.), 0.),
                        Some(control_clip),
                        palette,
                        &label_refs,
                    );
                    for (n, row) in layout.rows.iter().enumerate() {
                        let id = 30 + n;
                        s.button_labels.insert(id, labels[n].clone());
                        if let Some(hit) = row.visible_frame() {
                            s.hits.push((id, hit));
                        }
                    }
                }
                "switch" => {
                    let label_text = if self.switch.checked {
                        "Notifications on"
                    } else {
                        "Notifications off"
                    };
                    s.button_labels.insert(20, label_text.to_owned());
                    let component = self.switch_view.render_labeled(
                        scene,
                        ctx.text,
                        self.switch,
                        Rect::new(bx, by + 12., bw.min(382.), 32.),
                        Some(control_clip),
                        palette,
                        label_text,
                    );
                    if let Some(hit) = component.toggle.visible_frame() {
                        s.hits.push((20, hit));
                    }
                }
                "segmented-control" => {
                    let opts = ["Day", "Week", "Month"];
                    self.segmented_view.set_window_active(s.window_active);
                    if let Some(id) = self
                        .focused
                        .and_then(|id| id.checked_sub(30))
                        .filter(|i| *i < 3)
                    {
                        self.segmented_view.focus(id);
                    }
                    let layout = self.segmented_view.render(
                        scene,
                        ctx.text,
                        &mut self.segmented,
                        &opts,
                        Rect::new(bx, by + 18., bw.min(480.), 38.),
                        Some(control_clip),
                        palette,
                        proposal::TextRole::UiBody,
                        s.density == 1,
                    );
                    for (n, item) in layout.items.iter().enumerate() {
                        s.button_labels.insert(30 + n, opts[n].to_owned());
                        if let Some(visible) = item.visible {
                            s.hits.push((30 + n, visible));
                        }
                    }
                }
                "breadcrumb" => {
                    let titles = self.breadcrumb.items.clone();
                    let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
                    let layout = self.breadcrumb_view.render(
                        scene,
                        ctx.text,
                        &self.breadcrumb,
                        &refs,
                        Rect::new(bx, by + 18., bw.min(620.), 34.),
                        Some(control_clip),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    for (n, item) in layout.items.iter().enumerate() {
                        if let Some(hit) = item.visible_frame() {
                            s.hits.push((30 + n, hit));
                        }
                    }
                }
                "command-palette" => {
                    self.command.combo.disabled = self.disabled;
                    let trigger_width = bw.min(200.);
                    let trigger_height = ButtonModel::measure(
                        ctx.text,
                        "Open command palette",
                        trigger_width,
                        ButtonSize::Body,
                        proposal::TextRole::UiBody,
                    )
                    .height;
                    let trigger = self.command_view.render_trigger(
                        scene,
                        ctx.text,
                        &mut self.command,
                        "Open command palette",
                        Rect::new(bx, by + 8., trigger_width, trigger_height),
                        Some(control_clip),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    if let Some(hit) = trigger.visible {
                        s.hits.push((20, hit));
                    }
                    if self.command.combo.open || self.command_view.options.is_presented() {
                        let input_height = 38.0_f32.max(s.input.measure_search_height(ctx.text));
                        let input =
                            Rect::new(bx, trigger.frame.max_y() + 8., bw.min(420.), input_height);
                        if self.command.combo.open {
                            s.input.set_disabled(self.disabled);
                            s.input
                                .build_search_clipped(ctx, input, s.button_clip, scene);
                            if let Some(frame) = s.input.frame_at(0) {
                                s.register_logical_focus(
                                    1000,
                                    frame,
                                    !self.disabled,
                                    true,
                                    None,
                                    true,
                                );
                            }
                            if self.command.combo.query != s.input.query() {
                                self.command.combo.set_query(s.input.query());
                            }
                            if let Some(visible) = s.input.visible_frame() {
                                self.hits.push((1000, visible));
                                s.hits.push((1000, visible));
                            }
                        }
                        let options = self.command_view.render(
                            scene,
                            ctx.text,
                            &mut self.command,
                            input,
                            body,
                            Some(control_clip),
                            palette,
                            proposal::TextRole::UiBody,
                            false,
                        );
                        if let Some(options) = options {
                            for (filtered_index, item) in options.items.iter().enumerate() {
                                if let Some(index) =
                                    self.command_view.options.original_index(filtered_index)
                                    && let Some(hit) = item.visible
                                {
                                    s.hits.push((40 + index, hit));
                                }
                            }
                        }
                        if self.command.combo.open && self.command.combo.filtered().is_empty() {
                            wrapped_caption(
                                ctx,
                                scene,
                                input.min_x(),
                                input.max_y() + 8.,
                                input.size.width,
                                "No matching commands",
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        scene.pop_clip();
        // The log follows intrinsic story content, not the current document
        // height: feeding the previous extent back into this position prevents
        // shrinking after reflow and can grow the document on each build.
        let natural_bottom = if s.document_bottom != 0. {
            s.document_bottom
        } else {
            body.min_y() + body.size.height.min(320.)
        };
        s.document_bottom = natural_bottom;
        let log_y = natural_bottom + 24.;
        s.event_log_frame = Some(event_log(ctx, scene, x, log_y, w, &s.log, true));
        scene.pop_clip();
        let mut story_hits = s.hits.split_off(story_hit_start);
        story_hits.retain_mut(|(id, r)| {
            let clip = if *id == 10 { c } else { body };
            let clip = clip.intersection(&s.preview).unwrap_or(Rect::new(
                s.preview.min_x(),
                s.preview.min_y(),
                0.,
                0.,
            ));
            if let Some(clipped) = r.intersection(&clip) {
                *r = clipped;
                true
            } else {
                false
            }
        });
        s.hits.extend(story_hits);
        s.button_clip = original_clip;
        self.hits = s.hits.clone();
    }

    pub(super) fn hover(&mut self, id: Option<usize>) -> bool {
        let changed = self.hovered != id;
        self.hovered = id;
        self.chart.set_hovered(id.and_then(|id| id.checked_sub(30)));
        changed
    }
    pub(super) fn pointer_down(
        &mut self,
        renderer: &str,
        id: usize,
        point: Point,
        log: &mut impl FnMut(String),
    ) -> bool {
        if self.disabled && !matches!(id, 0 | 10) {
            return false;
        }
        if collections_adapter::is_collection(renderer) {
            return collections_adapter::pointer_down(self, renderer, point, Some(id), log)
                .is_some_and(|result| result.consumed);
        }
        if feedback_adapter::supports(renderer) {
            return feedback_adapter::pointer_down(self, renderer, point)
                .is_some_and(|result| result.consumed);
        }
        match renderer {
            "combo-box" if id >= 40 => {
                let _ = self.combo_view.pointer_down(point);
                return self.combo.open;
            }
            "command-palette" if id == 20 => {
                return self.command_view.pointer_down_trigger(point);
            }
            "command-palette" if id >= 40 => {
                let _ = self.command_view.pointer_down(point);
                return self.command.combo.open;
            }
            _ => {}
        }
        self.focused = Some(id);
        self.host_focus_changed(renderer, id, false);
        match renderer {
            "slider" if id == 20 => {
                let before = self.slider.value();
                self.slider_view.pointer_down(point, &mut self.slider);
                if before != self.slider.value() {
                    log(format!("value: {:.0}", self.slider.value()));
                }
                true
            }
            "radio-group" if (30..33).contains(&id) => {
                let event = self.radio_view.pointer_down(point, &self.radio);
                self.focused = self.radio_view.focused().map(|index| 30 + index);
                !matches!(event, SelectionEvent::Ignored)
            }
            "switch" if id == 20 => self.switch_view.pointer_down(point, &self.switch),
            "split-view" if id == 90 => {
                let began = self.split_view.pointer_down(point, &mut self.split);
                self.pointer_dragging = began.then_some(id);
                began
            }
            "disclosure" if id == 20 => {
                let armed = self.disclosure_view.pointer_down(point);
                if armed {
                    self.pointer_dragging = Some(id);
                }
                armed
            }
            "segmented-control" if (30..33).contains(&id) => {
                let i = self.segmented_view.pointer_down(point);
                if let Some(i) = i {
                    self.focused = Some(30 + i);
                    self.pointer_dragging = Some(30 + i);
                }
                i.is_some()
            }
            "breadcrumb" if (30..33).contains(&id) => {
                let index = self.breadcrumb_view.pointer_down(point);
                if let Some(index) = index {
                    self.pointer_dragging = Some(30 + index);
                }
                index.is_some()
            }
            "icon-button" if id == 20 => {
                self.pointer_dragging = Some(id);
                self.icon_button_view.pointer_down(point)
            }
            "link" if id == 20 => {
                self.pointer_dragging = Some(id);
                self.link_view.pointer_down(point)
            }
            _ => false,
        }
    }
    pub(super) fn pointer_move(
        &mut self,
        renderer: &str,
        point: Point,
        hit: Option<usize>,
        log: &mut impl FnMut(String),
    ) -> bool {
        let mut changed = self.hover(hit);
        if feedback_adapter::supports(renderer) {
            let result = feedback_adapter::pointer_move(self, renderer, point);
            changed |= result.is_some_and(|result| result.changed);
        }
        if collections_adapter::is_collection(renderer) {
            let result = collections_adapter::pointer_move(self, renderer, point, hit, log);
            changed |= result.is_some_and(|result| result.changed);
        }
        if renderer == "slider" {
            let before = self.slider.value();
            let moved = self.slider_view.pointer_move(point, &mut self.slider);
            if before != self.slider.value() {
                log(format!("value: {:.0}", self.slider.value()));
            }
            changed |= moved;
        }
        if renderer == "radio-group" {
            changed |= self.radio_view.pointer_move(point);
        }
        if renderer == "switch" {
            changed |= self.switch_view.pointer_move(point);
        }
        match renderer {
            "disclosure" => {
                self.disclosure_view.pointer_move(point);
            }
            "icon-button" => self.icon_button_view.pointer_move(point),
            "link" => self.link_view.pointer_move(point),
            "segmented-control" => self.segmented_view.pointer_move(point),
            "breadcrumb" => self.breadcrumb_view.pointer_move(point),
            _ => {}
        }
        if renderer == "combo-box" {
            self.combo_view.pointer_move(point, &mut self.combo);
        }
        if renderer == "command-palette" {
            self.command_view.pointer_move(point, &mut self.command);
        }
        if let ("split-view", Some(90)) = (renderer, self.pointer_dragging) {
            let next = self.split_view.pointer_move(point, &mut self.split);
            changed |= next;
            if next {
                log(format!("split: {:.0}%", self.split.ratio() * 100.));
            }
        }
        changed
    }
    pub(super) fn pointer_up(
        &mut self,
        renderer: &str,
        point: Point,
        hit: Option<usize>,
        log: &mut impl FnMut(String),
    ) -> bool {
        if collections_adapter::is_collection(renderer) {
            return collections_adapter::pointer_up(self, renderer, point, hit, log)
                .is_some_and(|result| result.consumed);
        }
        if feedback_adapter::supports(renderer) {
            return feedback_adapter::pointer_up(self, renderer, point, log)
                .is_some_and(|result| result.consumed);
        }
        match renderer {
            "combo-box" => {
                if let Some(index) = self.combo_view.pointer_up(point, &mut self.combo) {
                    log(format!("combo selection: {index}"));
                }
                return self.combo.open || hit.is_some_and(|id| id >= 40);
            }
            "command-palette" if hit == Some(20) => {
                let opened = self
                    .command_view
                    .pointer_up_trigger(point, &mut self.command);
                if opened {
                    log("command palette: open".into());
                }
                return true;
            }
            "command-palette" => {
                // Release the trigger even when capture ends outside its hit.
                // Candidate releases must not leave a prior trigger arm alive.
                if self
                    .command_view
                    .pointer_up_trigger(point, &mut self.command)
                {
                    log("command palette: open".into());
                    return true;
                }
                if let Some(index) = self.command_view.pointer_up(point, &mut self.command) {
                    log(format!("command: {index}"));
                }
                return self.command.combo.open || hit.is_some_and(|id| id >= 40);
            }
            _ => {}
        }
        match renderer {
            "disclosure" => {
                let captured = self.pointer_dragging.take() == Some(20);
                let changed = self.disclosure_view.pointer_up(point, &mut self.disclosure);
                if changed {
                    log(format!("disclosure: {}", self.disclosure.open));
                }
                return captured || changed;
            }
            "segmented-control" => {
                let captured = self
                    .pointer_dragging
                    .take()
                    .is_some_and(|id| (30..33).contains(&id));
                let selected = self.segmented_view.pointer_up(point, &mut self.segmented);
                if let Some(index) = selected {
                    self.focused = Some(30 + index);
                    log(format!("segment: {index}"));
                }
                return captured || selected.is_some();
            }
            "breadcrumb" => {
                let captured = self
                    .pointer_dragging
                    .take()
                    .is_some_and(|id| (30..33).contains(&id));
                let activated = self.breadcrumb_view.pointer_up(point, &self.breadcrumb);
                if let Some(index) = activated {
                    self.selected = index;
                    log(format!("breadcrumb: {index}"));
                }
                return captured || activated.is_some();
            }
            _ => {}
        }
        match renderer {
            "slider" => return self.slider_view.pointer_up(point, &self.slider),
            "radio-group" => {
                let result = self.radio_view.pointer_up(point, &mut self.radio);
                self.focused = self.radio_view.focused().map(|index| 30 + index);
                if result == SelectionEvent::Changed {
                    log(format!(
                        "radio: {}",
                        self.radio.selected.unwrap_or_default()
                    ));
                }
                return result != SelectionEvent::Ignored;
            }
            "switch" => {
                let result = self.switch_view.pointer_up(point, &mut self.switch);
                if result == SelectionEvent::Changed {
                    log(format!("switch: {}", self.switch.checked));
                }
                return result != SelectionEvent::Ignored;
            }
            _ => {}
        }
        let dragging = self.pointer_dragging.take();
        match (renderer, dragging) {
            ("split-view", Some(90)) => {
                self.split_view.pointer_up(&mut self.split);
                true
            }
            ("icon-button", Some(20)) => {
                let activated = self
                    .icon_button_view
                    .pointer_up(point)
                    && hit == Some(20);
                if activated {
                    log(format!(
                        "icon-button: {}",
                        self.icon_button_view.name()
                    ));
                }
                activated
            }
            ("link", Some(20)) => {
                let activated = self.link_view.pointer_up(point, &mut self.link) && hit == Some(20);
                if activated {
                    log("link: activated".into());
                }
                activated
            }
            _ => false,
        }
    }
    pub(super) fn cancel_pointer(&mut self) -> bool {
        let was_dragging = self.pointer_dragging.take().is_some();
        self.slider_view.cancel();
        self.radio_view.cancel();
        self.switch_view.cancel();
        self.split_view.cancel(&mut self.split);
        self.disclosure_view.cancel();
        self.icon_button_view.cancel();
        self.link_view.cancel();
        self.segmented_view.cancel();
        self.breadcrumb_view.cancel();
        was_dragging
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Workbench, entries};
    use super::*;
    use crate::gallery::{static_text::StaticText, text::TextEngine};
    use tekes_ui_platform_macos::{
        Event, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
        Point as NativePoint,
    };

    fn index(renderer: &str) -> usize {
        entries()
            .iter()
            .position(|entry| entry.renderer() == renderer)
            .expect("registered story")
    }

    fn build(
        workbench: &mut Workbench,
        index: usize,
        text: &mut TextEngine,
        content: Rect,
        clip: Rect,
    ) {
        text.begin_frame();
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text,
            static_text: &mut static_text,
        };
        workbench.build_clipped(index, &mut ctx, content, clip, &mut Scene::new());
    }

    #[test]
    fn real_link_state_matrix_repaints_semantic_inks_when_theme_changes() {
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::Theme;

        let index = index("link");
        let content = Rect::new(312., 70., 1000., 820.);
        let mut workbench = Workbench::default();
        workbench.select_route(index, &format!("{}/states", entries()[index].slug()));
        let mut text = TextEngine::new();
        let mut static_text = StaticText::new(None);
        let mut seen_by_theme = Vec::new();

        for theme in Theme::ALL {
            text.begin_frame();
            let mut ctx = PageCtx {
                theme,
                text: &mut text,
                static_text: &mut static_text,
            };
            let mut scene = Scene::new();
            workbench.build_clipped(index, &mut ctx, content, content, &mut scene);
            let inks: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::Glyphs(run) => Some(run.color),
                    _ => None,
                })
                .collect();
            let palette = Palette::from(theme);
            for role in [
                "c-link-foreground",
                "c-link-hover",
                "c-link-pressed",
                "disabled-text",
            ] {
                let ink = palette.color(role).expect("generated Link semantic color");
                assert!(
                    inks.contains(&ink),
                    "real Link state matrix must draw {role} under {theme:?}"
                );
            }
            seen_by_theme.push(inks);
        }
        assert_eq!(seen_by_theme.len(), Theme::ALL.len());
    }
    #[test]
    fn navigation_link_space_never_falls_back_to_gallery_button_activation() {
        for renderer in ["link", "breadcrumb"] {
            let mut workbench = Workbench::default();
            let mut text = TextEngine::new();
            let index = index(renderer);
            let content = Rect::new(312., 70., 1000., 820.);
            build(&mut workbench, index, &mut text, content, content);
            let id = if renderer == "link" { 20 } else { 31 };
            let point = workbench
                .state(index)
                .hits
                .iter()
                .find(|(hit, _)| *hit == id)
                .unwrap()
                .1
                .center();
            click(&mut workbench, index, point, &mut text);
            let count = workbench.state(index).log.len();
            assert!(!workbench.handle(index, &key_down(49), &mut text));
            let mut release = key_down(49);
            if let Event::Key(key) = &mut release {
                key.kind = KeyEventKind::Up;
            }
            assert!(!workbench.handle(index, &release, &mut text));
            assert_eq!(
                workbench.state(index).log.len(),
                count,
                "{renderer} must not activate through the generic button fallback"
            );
        }
    }

    #[test]
    fn breadcrumb_overall_disable_reaches_the_real_view_and_preserves_path() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let index = index("breadcrumb");
        let content = Rect::new(312., 70., 1000., 820.);
        build(&mut workbench, index, &mut text, content, content);
        let point = workbench
            .state(index)
            .hits
            .iter()
            .find(|(id, _)| *id == 10)
            .expect("existing disabled toggle must be exposed for Breadcrumb")
            .1
            .center();
        click(&mut workbench, index, point, &mut text);
        assert!(workbench.state(index).extended.disabled);
        build(&mut workbench, index, &mut text, content, content);
        let story = &mut workbench.state(index).extended;
        assert_eq!(story.breadcrumb_view.activate(0, &story.breadcrumb), None);
        assert_eq!(story.breadcrumb.items.len(), 3);
    }

    #[test]
    fn icon_playground_uses_shared_natural_geometry() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let index = index("icon-button");
        let content = Rect::new(312., 70., 1000., 820.);
        build(&mut workbench, index, &mut text, content, content);
        let story = &workbench.state(index).extended;
        assert_eq!(
            story.icon_button_view.layout().frame.size,
            ButtonSpecimen::measure(&mut text)
        );
        let side = proposal::metric("size", "size-icon-standard").unwrap();
        assert_eq!(
            story.icon_button_view.icon_frame().size,
            tekes_ui_scene::Size::new(side, side)
        );
    }

    #[test]
    fn overall_segment_disable_preserves_selection_and_item_availability() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let index = index("segmented-control");
        let content = Rect::new(312., 70., 1000., 820.);
        workbench.state(index).extended.segmented.select(0);
        workbench
            .state(index)
            .extended
            .segmented
            .set_enabled(1, false);
        workbench.state(index).extended.disabled = true;
        build(&mut workbench, index, &mut text, content, content);
        let model = &workbench.state(index).extended.segmented;
        assert_eq!(
            model.selected, 0,
            "overall disable must preserve controlled selection"
        );
        assert!(
            !model.is_enabled(1),
            "overall availability must preserve per-item choices"
        );
        workbench.state(index).extended.disabled = false;
        build(&mut workbench, index, &mut text, content, content);
        let model = &workbench.state(index).extended.segmented;
        assert_eq!(model.selected, 0);
        assert!(!model.is_enabled(1));
    }

    #[test]
    fn state_matrix_reserves_wrapped_text_rows_and_focus_gutter_at_large_type() {
        for renderer in [
            "link",
            "radio-group",
            "switch",
            "disclosure",
            "segmented-control",
        ] {
            let index = entries()
                .iter()
                .position(|entry| entry.renderer() == renderer)
                .unwrap();
            for scale in [1.0, 1.5] {
                let mut text = TextEngine::new();
                text.set_type_scale(scale);
                let mut static_text = StaticText::new(None);
                let mut ctx = PageCtx {
                    theme: tekes_ui_tokens::Theme::LIGHT,
                    text: &mut text,
                    static_text: &mut static_text,
                };
                let viewport = Rect::new(0., 0., 360., 12000.);
                let gutter = -controls::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
                    .focus_bounds(ctx.text.backing_scale())
                    .min_x();
                let content = viewport.inset(gutter);
                let columns = ((content.size.width / 300.).floor() as usize).clamp(1, 3);
                let cell_width = content.size.width / columns as f32;
                let specimen_width = match renderer {
                    "segmented-control" => (cell_width - 12.).max(0.),
                    _ => (cell_width - 12.).clamp(0., 220.),
                };
                let control_height = matrix_control_height(renderer, ctx.text, specimen_width);

                let (states, longest_label) = match renderer {
                    "link" => (5usize, "Focus-visible"),
                    "radio-group" | "switch" => (7usize, "Checked · focus-visible"),
                    "disclosure" => (8usize, "Collapsed · focus-visible"),
                    "segmented-control" => (5usize, "Selected · disabled peer"),
                    _ => unreachable!(),
                };
                let title_height = ctx
                    .text
                    .proposal_label(
                        longest_label,
                        proposal::TextRole::UiBody,
                        Some((cell_width - 12.).max(0.)),
                    )
                    .height as f32;
                let help =
                    "Visual comparison only. Use Playground for real focus and keyboard behavior.";
                let state_rows_y = content.min_y()
                    + 38.0_f32.max(caption_size(&mut ctx, help, content.size.width).height + 10.);
                let expected_bottom = state_rows_y
                    + states.div_ceil(columns) as f32 * (title_height + 8. + control_height + 20.)
                    + gutter;

                let entry = &entries()[index];
                let mut state = super::super::StoryState::default();
                let mut story = ExtendedStory::default();
                let mut scene = Scene::new();
                story.matrix(entry, &mut state, &mut ctx, &mut scene, viewport);
                assert!(
                    !scene.is_empty(),
                    "{renderer} matrix should paint real specimens"
                );
                assert!(
                    state.document_bottom + 0.01 >= expected_bottom,
                    "{renderer} at {scale}x needs {expected_bottom}pt, got {}pt; control row {control_height}pt",
                    state.document_bottom
                );
                assert!(
                    control_height
                        >= match renderer {
                            "link" => ctx
                                .text
                                .measure(
                                    "Open documentation",
                                    proposal::TextRole::UiBody,
                                    (specimen_width - 16.).max(0.),
                                )
                                .height
                                .max(36.),
                            "radio-group" => {
                                ctx.text
                                    .measure(
                                        "Option",
                                        proposal::TextRole::UiBody,
                                        (specimen_width - 40.).max(0.),
                                    )
                                    .height
                            }
                            "switch" => {
                                ctx.text
                                    .measure(
                                        "Enabled",
                                        proposal::TextRole::UiBody,
                                        (specimen_width - 62.).max(0.),
                                    )
                                    .height
                            }
                            "disclosure" => {
                                ctx.text
                                    .measure(
                                        "Details",
                                        proposal::TextRole::UiBody,
                                        (specimen_width - 48.).max(0.),
                                    )
                                    .height
                            }
                            "segmented-control" => ["General", "Advanced", "Security"]
                                .into_iter()
                                .map(|label| {
                                    ButtonModel::measure(
                                        ctx.text,
                                        label,
                                        specimen_width / 3.,
                                        ButtonSize::Md,
                                        proposal::TextRole::UiBody,
                                    )
                                    .height
                                })
                                .fold(0.0_f32, f32::max),
                            _ => unreachable!(),
                        },
                    "{renderer} text must fit its measured row at {scale}x"
                );
                eprintln!(
                    "state-matrix-layout renderer={renderer} width=360 type_scale={scale} specimen_width={specimen_width:.1} row_height={control_height:.1} document_bottom={:.1}",
                    state.document_bottom
                );
            }
        }
    }

    fn mouse(point: Point, kind: MouseEventKind) -> Event {
        Event::Mouse(MouseEvent {
            position: NativePoint {
                x: point.x as f64,
                y: point.y as f64,
            },
            modifiers: Modifiers::NONE,
            kind,
        })
    }

    fn key_down(key_code: u16) -> Event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Down,
            key_code,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
            modifiers: Modifiers::NONE,
            is_repeat: false,
        })
    }

    fn click(workbench: &mut Workbench, index: usize, point: Point, text: &mut TextEngine) {
        workbench.handle(
            index,
            &mouse(
                point,
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ),
            text,
        );
        workbench.handle(
            index,
            &mouse(
                point,
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ),
            text,
        );
    }

    #[test]
    fn slider_workbench_uses_full_thumb_hit_and_retained_visual_track_until_hidden() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let index = index("slider");
        let content = Rect::new(312., 70., 1000., 820.);
        build(&mut workbench, index, &mut text, content, content);

        let state = workbench.state(index);
        let hit = state
            .hits
            .iter()
            .find(|(id, _)| *id == 20)
            .map(|(_, rect)| *rect)
            .expect("visible Slider hit frame");
        let track = state.extended.slider_view.layout().track;
        let thumb_point = Point::new(track.center().x + 12., track.center().y);
        assert!(
            hit.contains(thumb_point),
            "the full thumb frame is clickable"
        );

        workbench.handle(
            index,
            &mouse(
                thumb_point,
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ),
            &mut text,
        );
        assert_eq!(workbench.state(index).extended.slider.value(), 55.);
        let visual_track = workbench.state(index).extended.slider_view.layout().track;
        let drag_point = Point::new(
            visual_track.min_x() + visual_track.size.width * 0.75,
            visual_track.center().y,
        );
        workbench.handle(
            index,
            &mouse(
                drag_point,
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
            ),
            &mut text,
        );
        assert_eq!(workbench.state(index).extended.slider.value(), 75.);
        assert!(workbench.state(index).extended.slider_view.is_captured());

        let outside = Rect::new(0., 0., 100., 100.);
        build(&mut workbench, index, &mut text, content, outside);
        assert!(!workbench.state(index).hits.iter().any(|(id, _)| *id == 20));
        assert_eq!(
            workbench.state(index).extended.slider_view.layout().visible,
            None
        );
        assert!(!workbench.state(index).extended.slider_view.is_captured());
        workbench.handle(index, &mouse(drag_point, MouseEventKind::Moved), &mut text);
        assert_eq!(workbench.state(index).extended.slider.value(), 75.);
    }

    fn begin_split_drag(workbench: &mut Workbench, story: usize, text: &mut TextEngine) -> Point {
        let content = Rect::new(312., 70., 1000., 820.);
        build(workbench, story, text, content, content);
        let divider = workbench
            .state(story)
            .extended
            .split_view
            .layout()
            .divider_hit
            .expect("split divider hit region")
            .center();
        workbench.handle(
            story,
            &mouse(
                divider,
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ),
            text,
        );
        assert_eq!(workbench.state(story).extended.pointer_dragging, Some(90));
        let moved = Point::new(divider.x + 48., divider.y);
        workbench.handle(
            story,
            &mouse(
                moved,
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
            ),
            text,
        );
        assert_ne!(workbench.state(story).extended.split.ratio(), 0.5);
        moved
    }

    #[test]
    fn split_view_escape_rolls_back_and_releases_gallery_pointer_capture() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let story = index("split-view");
        let moved = begin_split_drag(&mut workbench, story, &mut text);

        assert!(workbench.handle(story, &key_down(53), &mut text));
        let state = workbench.state(story);
        assert_eq!(state.extended.split.ratio(), 0.5);
        assert_eq!(state.extended.pointer_dragging, None);
        workbench.handle(
            story,
            &mouse(
                Point::new(moved.x + 30., moved.y),
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
            ),
            &mut text,
        );
        let state = workbench.state(story);
        assert_eq!(
            state.extended.split.ratio(),
            0.5,
            "post-Escape movement is ignored"
        );
        assert_eq!(state.extended.pointer_dragging, None);
    }

    #[test]
    fn split_view_invalid_layout_and_focus_loss_restore_ratio_and_release_capture() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let story = index("split-view");
        let content = Rect::new(312., 70., 1000., 820.);

        let moved = begin_split_drag(&mut workbench, story, &mut text);
        build(
            &mut workbench,
            story,
            &mut text,
            content,
            Rect::new(0., 0., 10., 10.),
        );
        let state = workbench.state(story);
        assert_eq!(state.extended.split.ratio(), 0.5);
        assert_eq!(state.extended.pointer_dragging, None);
        workbench.handle(
            story,
            &mouse(
                moved,
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
            ),
            &mut text,
        );
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);

        let moved = begin_split_drag(&mut workbench, story, &mut text);
        workbench
            .state(story)
            .extended
            .host_focus_changed("split-view", 91, false);
        let state = workbench.state(story);
        assert_eq!(state.extended.split.ratio(), 0.5);
        assert_eq!(state.extended.pointer_dragging, None);
        workbench.handle(
            story,
            &mouse(
                moved,
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
            ),
            &mut text,
        );
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
        assert_eq!(workbench.state(story).extended.pointer_dragging, None);
    }

    #[test]
    fn split_native_drag_lost_release_and_double_click_reset() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let story = index("split-view");
        let moved = begin_split_drag(&mut workbench, story, &mut text);
        assert!(workbench.has_pointer_capture(story));
        assert!(!workbench.handle(story, &mouse(moved, MouseEventKind::Exited), &mut text));
        assert!(
            workbench.has_pointer_capture(story),
            "leaving does not lose AppKit tracking"
        );
        assert!(workbench.handle(story, &mouse(moved, MouseEventKind::Moved), &mut text));
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
        assert!(!workbench.has_pointer_capture(story));
        workbench.handle(
            story,
            &mouse(
                moved,
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ),
            &mut text,
        );
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);

        let moved = begin_split_drag(&mut workbench, story, &mut text);
        workbench.handle(
            story,
            &mouse(
                moved,
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ),
            &mut text,
        );
        let content = Rect::new(312., 70., 1000., 820.);
        build(&mut workbench, story, &mut text, content, content);
        let divider = workbench
            .state(story)
            .extended
            .split_view
            .layout()
            .divider_hit
            .unwrap()
            .center();
        assert!(workbench.handle(
            story,
            &mouse(
                divider,
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 2
                }
            ),
            &mut text
        ));
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
        assert!(!workbench.has_pointer_capture(story));
        workbench.handle(
            story,
            &mouse(
                moved,
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
            ),
            &mut text,
        );
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
    }

    #[test]
    fn gallery_split_cursor_regions_follow_page_and_window_lifecycle() {
        let mut gallery = crate::gallery::Gallery::new();
        let story = index("split-view");
        gallery.apply(crate::gallery::Command::Page(
            crate::gallery::Page::Workbench(story),
        ));
        gallery.set_view_size(1440., 1000.);
        gallery.build_scene(&mut Scene::new());
        let regions = gallery.cursor_regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0].icon,
            tekes_ui_platform_macos::CursorIcon::ResizeHorizontal
        );
        assert!(regions[0].size.width > 0. && regions[0].size.height > 0.);
        gallery.handle_input(&Event::Focus(false));
        assert!(gallery.cursor_regions().is_empty());
        gallery.handle_input(&Event::Focus(true));
        gallery.build_scene(&mut Scene::new());
        assert_eq!(gallery.cursor_regions().len(), 1);
        gallery.apply(crate::gallery::Command::Page(
            crate::gallery::Page::Workbench(index("button")),
        ));
        assert!(gallery.cursor_regions().is_empty());
    }

    #[test]
    fn gallery_split_capture_routes_across_catalog_boundary() {
        let mut gallery = crate::gallery::Gallery::new();
        let story = index("split-view");
        gallery.apply(crate::gallery::Command::Page(
            crate::gallery::Page::Workbench(story),
        ));
        gallery.set_view_size(1440., 1000.);
        gallery.build_scene(&mut Scene::new());
        let divider = gallery
            .workbench
            .state(story)
            .extended
            .split_view
            .layout()
            .divider_hit
            .unwrap()
            .center();
        gallery.handle_input(&mouse(
            divider,
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            },
        ));
        assert!(gallery.workbench.has_pointer_capture(story));
        let outside = Point::new(100., divider.y);
        assert_eq!(
            gallery.handle_input(&mouse(
                outside,
                MouseEventKind::Dragged {
                    button: MouseButton::Left
                }
            )),
            crate::gallery::Change::REDRAW
        );
        assert!(gallery.workbench.state(story).extended.split.ratio() < 0.5);
        gallery.handle_input(&mouse(
            outside,
            MouseEventKind::Up {
                button: MouseButton::Left,
                click_count: 1,
            },
        ));
        assert!(!gallery.workbench.has_pointer_capture(story));
        let committed = gallery.workbench.state(story).extended.split.ratio();
        gallery.handle_input(&mouse(divider, MouseEventKind::Moved));
        assert_eq!(
            gallery.workbench.state(story).extended.split.ratio(),
            committed
        );
    }

    #[test]
    fn split_reset_has_keyboard_alternative_and_releases_capture() {
        let mut workbench = Workbench::default();
        let mut text = TextEngine::new();
        let story = index("split-view");
        for code in [36, 49] {
            let moved = begin_split_drag(&mut workbench, story, &mut text);
            assert!(workbench.handle(story, &key_down(code), &mut text));
            assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
            assert!(!workbench.has_pointer_capture(story));
            workbench.handle(
                story,
                &mouse(
                    moved,
                    MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    },
                ),
                &mut text,
            );
            assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
        }
    }

    #[test]
    fn caption_help_slots_grow_from_real_measurement_at_large_text_scales() {
        let help = "Accessible name: Create item. Enter and Space activate; pointer activation requires press and release on the same control.";
        let slider_help =
            "Arrow keys change by 5; Home / End move to bounds. Drag the track to set a value.";
        let number_help = "Bounded stepper: 1…32. Editable numeric text is not implemented.";
        let mut text = TextEngine::new();
        let mut static_text = StaticText::new(None);
        let mut scene = Scene::new();
        for scale in [1.0, 1.25, 1.5] {
            text.set_type_scale(scale);
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            let narrow = caption_size(&mut ctx, help, 44.);
            let wide = caption_size(&mut ctx, help, 1000.);
            assert!(
                narrow.height > wide.height,
                "a 44 pt icon-button slot must wrap at {scale}x"
            );
            let slot = wrapped_caption(&mut ctx, &mut scene, 8., 12., 44., help);
            assert_eq!(slot.size.height, narrow.height);

            let slider = caption_size(&mut ctx, slider_help, 540.);
            assert!(slider.height >= proposal::TextRole::UiCaption.style().line_height * scale);
            if scale == 1.5 {
                assert!(
                    slider.height > proposal::TextRole::UiCaption.style().line_height * scale,
                    "Slider instructions must reserve every wrapped line"
                );
                let number = caption_size(&mut ctx, number_help, 420.);
                assert!(
                    number.height > proposal::TextRole::UiCaption.style().line_height * scale,
                    "NumberField note must reserve wrapped copy at 150%"
                );
                let number_slot =
                    wrapped_caption(&mut ctx, &mut scene, 8., 240., 420., number_help);
                assert_eq!(number_slot.size.height, number.height);
            }
        }
    }

    #[test]
    fn icon_link_radio_and_switch_workbench_hits_are_unique_and_drive_models() {
        for renderer in ["icon-button", "link"] {
            let mut workbench = Workbench::default();
            let mut text = TextEngine::new();
            let index = index(renderer);
            let content = Rect::new(312., 70., 1000., 820.);
            build(&mut workbench, index, &mut text, content, content);
            let state = workbench.state(index);
            let hits: Vec<_> = state.hits.iter().filter(|(id, _)| *id == 20).collect();
            assert_eq!(hits.len(), 1, "{renderer} must register one effective hit");
            let point = hits[0].1.center();
            click(&mut workbench, index, point, &mut text);
            assert!(
                workbench.state(index).log.back().is_some_and(
                    |event| event.contains("activated") || event.contains("Create item")
                )
            );
        }

        for (renderer, id) in [("radio-group", 32), ("switch", 20)] {
            let mut workbench = Workbench::default();
            let mut text = TextEngine::new();
            let index = index(renderer);
            let content = Rect::new(312., 70., 1000., 820.);
            build(&mut workbench, index, &mut text, content, content);
            let point = workbench
                .state(index)
                .hits
                .iter()
                .find(|(hit_id, _)| *hit_id == id)
                .map(|(_, rect)| rect.center())
                .expect("visible component row hit");
            click(&mut workbench, index, point, &mut text);
            let story = &workbench.state(index).extended;
            match renderer {
                "radio-group" => assert_eq!(story.radio.selected, Some(2)),
                "switch" => assert!(story.switch.checked),
                _ => unreachable!(),
            }
        }
    }
    #[test]
    fn separator_story_and_variants_forward_actual_renderer_backing_scale() {
        let entry = &entries()[index("separator")];
        for theme in tekes_ui_tokens::Theme::ALL {
            for scale in [1., 2.] {
                for variants in [false, true] {
                    let mut story = ExtendedStory::default();
                    let mut state = StoryState::default();
                    let mut text = TextEngine::new();
                    text.set_scale(scale);
                    text.begin_frame();
                    let mut static_text = StaticText::new(None);
                    let mut ctx = PageCtx {
                        theme,
                        text: &mut text,
                        static_text: &mut static_text,
                    };
                    let mut scene = Scene::new();
                    let bounds = Rect::new(8., 8., 800., 800.);
                    if variants {
                        story.variants(entry, &mut state, &mut ctx, &mut scene, bounds);
                    } else {
                        story.draw(entry, &mut state, &mut ctx, &mut scene, bounds);
                    }
                    let color = Palette::from(theme).color("separator").unwrap();
                    let widths = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| match p {
                            tekes_ui_scene::Primitive::Quad(q)
                                if q.fill == color && q.border_width == 0. =>
                            {
                                if q.rect.size.width > 100. && q.rect.size.height <= 1. {
                                    Some(q.rect.size.height)
                                } else if q.rect.size.height > 100. && q.rect.size.width <= 1. {
                                    Some(q.rect.size.width)
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        widths.len(),
                        2,
                        "two real standalone separators: variants={variants}"
                    );
                    let expected = if theme.contrast == tekes_ui_tokens::Contrast::Increased {
                        1.
                    } else {
                        1. / scale
                    };
                    assert!(
                        widths.into_iter().all(|width| width == expected),
                        "Gallery must forward scale {scale}, variants={variants}"
                    );
                }
            }
        }
    }
    #[test]
    fn grid_playground_zero_area_drops_child_geometry_and_restores_it() {
        let entry = &entries()[index("grid")];
        let mut story = ExtendedStory::default();
        let mut state = StoryState::default();
        let mut text = TextEngine::new();
        let mut static_text = StaticText::new(None);
        for (height, count) in [(120., 0), (800., 9), (120., 0)] {
            text.begin_frame();
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            story.draw(
                entry,
                &mut state,
                &mut ctx,
                &mut Scene::new(),
                Rect::new(8., 8., 800., height),
            );
            assert_eq!(story.grid_view.layout().items.len(), count);
        }
    }
    #[test]
    fn layout_panel_callers_forward_host_backing_scale() {
        let mut gpu = std::env::var_os("TEKESUI_PANEL_SCALE_OUT")
            .map(|_| tekes_ui_render::Renderer::new_headless().unwrap());
        let mut wrong = 0;
        for (ti, theme) in tekes_ui_tokens::Theme::ALL.into_iter().enumerate() {
            for brand in 0..3 {
                for scale in [1., 2.] {
                    for (renderer, variants) in [
                        ("stack", false),
                        ("grid", false),
                        ("split-view", false),
                        ("disclosure", false),
                        ("stack", true),
                        ("grid", true),
                        ("split-view", true),
                    ] {
                        let entry = &entries()[index(renderer)];
                        let mut story = ExtendedStory::default();
                        story.disclosure.open = true;
                        let mut state = StoryState {
                            brand,
                            ..Default::default()
                        };
                        let mut text = TextEngine::new();
                        text.set_scale(scale);
                        text.begin_frame();
                        let mut static_text = StaticText::new(None);
                        let mut scene = Scene::new();
                        let mut ctx = PageCtx {
                            theme,
                            text: &mut text,
                            static_text: &mut static_text,
                        };
                        let bounds = Rect::new(8., 8., 700., 520.);
                        if variants {
                            story.variants(entry, &mut state, &mut ctx, &mut scene, bounds);
                        } else {
                            story.draw(entry, &mut state, &mut ctx, &mut scene, bounds);
                        }
                        let name = format!("layout-{renderer}-v{variants}-t{ti}-b{brand}-s{scale}");
                        wrong += crate::gallery::quality_effects::record_panel_host_scale(
                            &scene,
                            &mut text,
                            Palette::new(theme, brand).expect("registered Gallery brand"),
                            &name,
                            gpu.as_mut(),
                        );
                    }
                }
            }
        }
        assert_eq!(
            wrong, 0,
            "all layout panel callers must forward existing backing scale"
        );
    }
}
