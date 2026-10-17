//! Gallery stories for the additional production control models.
//!
//! The story owns deterministic fixtures and scene placement. Interaction semantics
//! are delegated to `tekes_ui_controls::extended` models.
mod collections_adapter;
mod feedback_adapter;
use super::super::pages::icons::GalleryIconRenderer;
use super::super::pages::showcase::{Icon, draw_icon};
use super::{Entry, PageCtx, StoryState, event_log, label, quiet_action};
use collections_adapter::CollectionsViews;
use feedback_adapter::FeedbackViews;
use tekes_ui_controls::{
    BarChart, ChartDatum, ControlKey, ControlState, RadioGroupView, SelectionEvent, SliderView,
    SwitchView, extended as controls,
    layout_view::{
        BreadcrumbView, ComboBoxView, CommandPaletteView, DisclosureView, GridView, IconButtonView,
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
    )
}
pub(super) fn supports_story(renderer: &str, story: &str) -> bool {
    match story {
        "states" => matches!(
            renderer,
            "disclosure" | "icon-button" | "link" | "slider" | "radio-group" | "switch"
        ),
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
        "variants" => matches!(renderer, "stack" | "grid" | "separator" | "split-view"),
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
    pub icon_button_view: IconButtonView,
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
    pub icon_button: controls::IconButton,
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
            icon_button_view: IconButtonView::default(),
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
            icon_button: controls::IconButton::new("Create item").expect("accessible name"),
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
    pub(super) fn set_window_active(&mut self, active: bool) {
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
        s: &StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        scene.push_clip(c);
        let x = c.min_x();
        let y = c.min_y();
        let width = c.size.width.max(0.);
        let columns = if width < 520. { 2 } else { 3 };
        let cell_w = (width / columns as f32).max(0.);
        let states: Vec<(&str, ControlState)> = match e.renderer() {
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
                (
                    "Selected · hover",
                    ControlState {
                        selected: true,
                        hovered: true,
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
        for (n, (name, state)) in states.into_iter().enumerate() {
            let col = n % columns;
            let row = n / columns;
            let rect = Rect::new(
                x + col as f32 * cell_w,
                state_rows_y + row as f32 * 82.,
                (cell_w - 12.).max(0.),
                62.,
            );
            label(
                ctx,
                scene,
                Rect::new(rect.min_x(), rect.min_y(), rect.size.width, 20.),
                name,
                false,
            );
            let control = Rect::new(
                rect.min_x(),
                rect.min_y() + 24.,
                rect.size.width.min(220.),
                36.,
            );
            let palette = Palette {
                theme: ctx.theme,
                brand: s.brand,
            };
            match e.renderer() {
                "switch" => controls::switch_with_state(
                    scene,
                    Rect::new(control.min_x(), control.min_y() + 5., 48., 26.),
                    palette,
                    state,
                ),
                "radio-group" => controls::radio_with_state(
                    scene,
                    Rect::new(control.min_x(), control.min_y() + 9., 18., 18.),
                    palette,
                    state,
                ),
                "slider" => {
                    let mut slider =
                        controls::Slider::new(50., 0., 100., 5.).expect("valid fixture");
                    slider.disabled = state.disabled;
                    controls::slider(scene, control, palette, slider, state);
                }
                "disclosure" => {
                    paint::button(scene, control, palette, state);
                    label(ctx, scene, control.inset(8.), "Details", false);
                }
                "link" => {
                    let ink = controls::link_underline(scene, control, palette, state);
                    ctx.text.proposal_label_in(
                        scene,
                        ctx.theme,
                        control.inset(8.),
                        "Open documentation",
                        proposal::TextRole::UiBody,
                        ink,
                    );
                }
                "icon-button" => {
                    paint::button(scene, control, palette, state);
                    draw_icon(ctx, scene, control.inset(10.), Icon::Plus);
                }
                _ => {}
            }
        }
        scene.pop_clip();
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
        s: &StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        let palette = Palette {
            theme: ctx.theme,
            brand: s.brand,
        };
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
                        paint::surface(scene, cell, palette, true);
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
                        paint::surface(scene, cell, palette, true);
                    }
                }
            }
            "separator" => {
                label(
                    ctx,
                    scene,
                    Rect::new(x, y, w.min(320.), 28.),
                    "Horizontal",
                    false,
                );
                controls::separator(
                    scene,
                    Rect::new(x, y + 36., w.min(620.), 12.),
                    palette,
                    controls::Axis::Horizontal,
                );
                label(
                    ctx,
                    scene,
                    Rect::new(x, y + 60., w.min(320.), 28.),
                    "Vertical",
                    false,
                );
                controls::separator(
                    scene,
                    Rect::new(x + 340., y + 48., 12., 160.),
                    palette,
                    controls::Axis::Vertical,
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
                    paint::surface(scene, leading, palette, true);
                    controls::separator(scene, divider, palette, controls::Axis::Vertical);
                    paint::surface(scene, trailing, palette, true);
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
            return collections_adapter::activate(self, renderer, id, log)
                .is_some_and(|result| result.changed);
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
                self.icon_button.button.state.focused = true;
                self.icon_button.pointer_down();
                let changed = self.icon_button.pointer_up(true);
                if changed {
                    log(format!(
                        "icon-button: {}",
                        self.icon_button.accessible_name()
                    ));
                }
                changed
            }
            ("link", 20) => {
                self.link.button.state.focused = true;
                self.link.pointer_down();
                let changed = self.link.pointer_up(true);
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
                self.command.combo.selected = Some(id - 40);
                self.command.combo.open = false;
                log(format!("command: {}", id - 40));
                true
            }
            ("combo-box", 40..=42) => {
                let i = id - 40;
                self.combo.selected = Some(i);
                self.combo.query = self.combo.items[i].clone();
                self.combo.open = false;
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
            "icon-button" => Some(self.icon_button_view.key_event(
                key,
                down,
                &mut self.icon_button,
            )),
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
                    "icon-button" => format!("icon-button: {}", self.icon_button.accessible_name()),
                    _ => format!("{renderer}: keyboard state changed"),
                };
                log(message);
            }
            return event.consumed;
        }
        if !down {
            return match renderer {
                "icon-button" => self.icon_button_view.key(key, false, &mut self.icon_button),
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
                if event.changed && !self.combo.open {
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
                    if event.changed && !self.command.combo.open {
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
            "icon-button" => self.icon_button_view.key(key, true, &mut self.icon_button),
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
        let palette = Palette {
            theme: ctx.theme,
            brand: s.brand,
        };
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
        self.icon_button.button.state.disabled = self.disabled;
        self.link.button.state.disabled = self.disabled;
        for n in 0..3 {
            self.segmented.set_enabled(n, !self.disabled);
        }
        self.chart
            .set_focused(self.focused.is_some(), self.focused.is_some());
        let rendered_by_component_adapter =
            collections_adapter::build(self, e, s, ctx, scene, body, control_clip, palette)
                || feedback_adapter::build(self, e, s, ctx, scene, body, control_clip, palette);
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
                            paint::surface(scene, r.inset(0.5), palette, true);
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
                            paint::surface(scene, r.inset(0.5), palette, true);
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
                    self.separator_view.render(
                        scene,
                        Rect::new(bx, by + 42., bw.min(600.), 12.),
                        controls::Axis::Horizontal,
                        Some(control_clip),
                        palette,
                    );
                    label(
                        ctx,
                        scene,
                        Rect::new(bx, by + 62., bw.min(500.), 28.),
                        "Profile and preferences",
                        false,
                    );
                    self.separator_view.render(
                        scene,
                        Rect::new(bx + 300., by + 110., 12., 160.),
                        controls::Axis::Vertical,
                        Some(control_clip),
                        palette,
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
                        paint::surface(scene, body_layout.body_rect, palette, true);
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
                        "Divider position: {:.0}% · drag, arrow keys, Home / End; Escape restores the drag start.",
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
                    let split_layout = self.split_view.render(
                        scene,
                        &mut self.split,
                        bounds,
                        Some(control_clip),
                        palette,
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
                    paint::surface(scene, leading, palette, true);
                    paint::surface(scene, trailing, palette, true);
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
                    self.icon_button.button.state.disabled = self.disabled;
                    self.link.button.state.disabled = self.disabled;
                    let r = Rect::new(bx, by + 18., if is_link { 260. } else { 44. }, 40.);
                    let component = if is_link {
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
                        self.icon_button_view.render(
                            scene,
                            ctx.text,
                            &mut self.icon_button,
                            r,
                            Some(control_clip),
                            palette,
                        )
                    };
                    let r = component.frame;
                    if let Some(hit) = component.visible {
                        s.hits.push((20, hit));
                    }
                    if !is_link {
                        let mut renderer = GalleryIconRenderer::native(ctx);
                        self.icon_button_view.render_icon(
                            scene,
                            &mut renderer,
                            "plus",
                            Some(control_clip),
                            palette,
                            20.,
                        );
                    }
                    let help = if is_link {
                        "Activates with Enter. Space is not a link activation key.".to_owned()
                    } else {
                        format!("Accessible name: {}", self.icon_button.accessible_name())
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
                            by + 60.,
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
                    let trigger = self.command_view.render_trigger(
                        scene,
                        ctx.text,
                        &mut self.command,
                        "Open command palette",
                        Rect::new(bx, by + 8., 200., 36.),
                        Some(control_clip),
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    if let Some(hit) = trigger.visible {
                        s.hits.push((20, hit));
                    }
                    if self.command.combo.open {
                        let input_height = 38.0_f32.max(s.input.measure_search_height(ctx.text));
                        let input = Rect::new(bx, by + 52., bw.min(420.), input_height);
                        s.input.set_disabled(self.disabled);
                        s.input
                            .build_search_clipped(ctx, input, s.button_clip, scene);
                        if self.command.combo.query != s.input.query() {
                            self.command.combo.set_query(s.input.query());
                        }
                        if let Some(visible) = s.input.visible_frame() {
                            self.hits.push((1000, visible));
                            s.hits.push((1000, visible));
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
                    }
                }
                _ => {}
            }
        }
        scene.pop_clip();
        let log_y = (c.max_y() - 112.).max(c.min_y());
        event_log(ctx, scene, x, log_y, w, &s.log, true);
        scene.pop_clip();
        let mut story_hits = s.hits.split_off(story_hit_start);
        story_hits.retain_mut(|(id, r)| {
            let clip = if *id == 10 { c } else { body };
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
                    .pointer_up(point, &mut self.icon_button)
                    && hit == Some(20);
                if activated {
                    log(format!(
                        "icon-button: {}",
                        self.icon_button.accessible_name()
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
        workbench.handle(index, &mouse(drag_point, MouseEventKind::Moved), &mut text);
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
        workbench.handle(story, &mouse(moved, MouseEventKind::Moved), text);
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
            &mouse(Point::new(moved.x + 30., moved.y), MouseEventKind::Moved),
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
        workbench.handle(story, &mouse(moved, MouseEventKind::Moved), &mut text);
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);

        let moved = begin_split_drag(&mut workbench, story, &mut text);
        workbench
            .state(story)
            .extended
            .host_focus_changed("split-view", 91, false);
        let state = workbench.state(story);
        assert_eq!(state.extended.split.ratio(), 0.5);
        assert_eq!(state.extended.pointer_dragging, None);
        workbench.handle(story, &mouse(moved, MouseEventKind::Moved), &mut text);
        assert_eq!(workbench.state(story).extended.split.ratio(), 0.5);
        assert_eq!(workbench.state(story).extended.pointer_dragging, None);
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
}
