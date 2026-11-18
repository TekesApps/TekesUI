//! Gallery stories for the additional production control models.
//!
//! The story owns deterministic fixtures and scene placement. Interaction semantics
//! are delegated to `tekes_ui_controls::extended` models.
mod button_specimen;
mod collections_adapter;
use button_specimen::ButtonSpecimen;
mod feedback_adapter;
pub(super) mod navigation_states;
use tekes_ui_controls::{
    BarChart, ButtonModel, ButtonSize, ButtonTextRenderer, ChartDatum, ControlKey, ControlState,
    extended as controls,
    layout_view::{
        BreadcrumbView, CommandPaletteView, DisclosureView, LinkView, RadioGroupView,
        SegmentedControlView, SliderView, SplitViewView, SwitchView,
    },
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

pub(super) fn matrix_control_height(
    renderer: &str,
    text: &mut impl ButtonTextRenderer,
    width: f32,
) -> f32 {
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

pub(super) fn paint_selection_fixture(
    renderer: &str,
    fixture: (&str, ControlState),
    scene: &mut Scene,
    text: &mut impl ButtonTextRenderer,
    control: Rect,
    c: Rect,
    palette: Palette,
) {
    let (name, state) = fixture;
    match renderer {
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
                text,
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
            view.render_labeled(scene, text, model, control, Some(c), palette, "Enabled");
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
                text,
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
            view.render(scene, text, &model, control, Some(c), palette, &["Option"]);
        }
        "slider" => {
            let mut slider = controls::Slider::new(50., 0., 100., 5.).expect("valid fixture");
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
                text,
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
                text,
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
                text,
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
                text,
                &mut model,
                &labels,
                control,
                Some(c),
                palette,
                proposal::TextRole::UiBody,
                false,
            );
        }
        _ => unreachable!("selection matrix renderer"),
    }
}

pub(super) fn matrix_states(renderer: &str) -> Vec<(&'static str, ControlState)> {
    match renderer {
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
    }
}

pub(super) fn behavior_copy(renderer: &str) -> &'static str {
    match renderer {
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
        "list" => "Single selection. Up and Down move; Home and End select the first and last row.",
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
    }
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

pub(super) fn supports_story(renderer: &str, story: &str) -> bool {
    match story {
        "states" => matches!(
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
                | "list"
                | "tree"
                | "data-table"
                | "chart"
                | "tooltip"
                | "popover"
                | "toast"
                | "banner"
                | "empty-state"
        ),
        "variants" => matches!(
            renderer,
            "toast"
                | "banner"
                | "status-badge"
                | "progress"
                | "empty-state"
                | "stack"
                | "grid"
                | "separator"
                | "split-view"
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
        _ => false,
    }
}

#[derive(Debug)]
pub(super) struct ExtendedStory {
    pub split_view: SplitViewView,
    pub disclosure_view: DisclosureView,
    pub icon_button_view: ButtonSpecimen,
    pub link_view: LinkView,
    pub segmented_view: SegmentedControlView,
    pub breadcrumb_view: BreadcrumbView,
    pub command_view: CommandPaletteView,
    pub disclosure: controls::Disclosure,
    pub split: controls::SplitView,
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
    pub list: controls::List,
    pub tree: controls::Tree,
    pub table: controls::DataTable,
    pub chart: BarChart,
    pub tooltip: controls::Tooltip,
    pub banner: controls::Banner,
    pub toast: controls::Toast,
    pub progress: controls::Progress,
    pub markdown: controls::Markdown,
    pub code: controls::CodeBlock,
    pub diff: controls::DiffView,
    pub disabled: bool,
    pub focused: Option<usize>,
    pub visible_rows: Vec<usize>,
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
            split_view: SplitViewView::new(controls::Axis::Horizontal),
            disclosure_view: DisclosureView::default(),
            icon_button_view: ButtonSpecimen::default(),
            link_view: LinkView::default(),
            segmented_view: SegmentedControlView::new(3),
            breadcrumb_view: BreadcrumbView::default(),
            command_view: CommandPaletteView::default(),
            disclosure: controls::Disclosure::default(),
            split: controls::SplitView::new(0.5),
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
            focused: None,
            visible_rows: Vec::new(),
            selected: 0,
            empty_state_created: false,
        }
    }
}

impl ExtendedStory {
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
            self.cancel_pointer();
            self.focused = None;
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
                    log(format!("icon-button: {}", self.icon_button_view.name()));
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
            ("progress", 20) => {
                let next = (self.progress.value() + 0.1).min(1.0);
                let changed = self.progress.set(next);
                if changed {
                    log(format!("progress: {:.0}%", next * 100.0));
                }
                changed
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
