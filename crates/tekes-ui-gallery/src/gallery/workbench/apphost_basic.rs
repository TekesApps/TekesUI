//! Basic story composition using original models and production Content owners.
use super::{Entry, StoryState};
use tekes_ui_controls::composition::*;
use tekes_ui_controls::extended::{Axis, Stack};
use tekes_ui_controls::layout::MeasuredChild;
use tekes_ui_controls::{ButtonModel, ButtonTextRenderer, CheckState, Checkbox, ControlState};
use tekes_ui_controls::{ButtonSize, ButtonTextAlign, ButtonVariant};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::proposal::TextRole;

/// Original state matrices are frozen visual fixtures. Forward the real window
/// text/resources, while retaining their existing reduced-motion/no-frame policy.
pub(super) struct FixtureText<'a>(pub &'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for FixtureText<'_> {
    fn measure(&mut self, value: &str, role: TextRole, width: f32) -> Size {
        self.0.measure(value, role, width)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        value: &str,
        role: TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: tekes_ui_tokens::Rgba,
    ) {
        self.0.draw(scene, value, role, rect, align, ink)
    }
    fn icon_renderer(&mut self) -> Option<&mut dyn tekes_ui_controls::layout_view::IconRenderer> {
        self.0.icon_renderer()
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        self.0.text_surface_host()
    }
    fn resolved_font_size(&mut self, role: TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
    fn first_line_height(&mut self, value: &str, role: TextRole, width: f32) -> f32 {
        self.0.first_line_height(value, role, width)
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.0.motion_context().map(|mut context| {
            context.reduced_motion = true;
            context
        })
    }
    fn animation_frames_allowed(&self) -> bool {
        false
    }
}

const ROOT: ComponentId = ComponentId(92000);
const ROWS: ComponentId = ComponentId(92001);
const BODY: ComponentId = ComponentId(92002);
const LOG: ComponentId = ComponentId(92003);
pub(crate) const SPECIMEN: ComponentId = ComponentId(92020);
const COMMAND: u64 = 92100;

// Shared with the legacy reference: there is one definition of the original
// visual fixtures, not two slowly diverging matrices.
pub(super) fn matrix_states() -> [(&'static str, ControlState, CheckState); 16] {
    [
        ("Unchecked", ControlState::default(), CheckState::Unchecked),
        (
            "Unchecked · hover",
            ControlState {
                hovered: true,
                ..Default::default()
            },
            CheckState::Unchecked,
        ),
        (
            "Unchecked · pressed",
            ControlState {
                pressed: true,
                ..Default::default()
            },
            CheckState::Unchecked,
        ),
        (
            "Unchecked · focus-visible",
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            CheckState::Unchecked,
        ),
        (
            "Unchecked · disabled",
            ControlState {
                disabled: true,
                ..Default::default()
            },
            CheckState::Unchecked,
        ),
        (
            "Unchecked · error + focus-visible",
            ControlState {
                error: true,
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            CheckState::Unchecked,
        ),
        (
            "Checked · error",
            ControlState {
                error: true,
                ..Default::default()
            },
            CheckState::Checked,
        ),
        (
            "Indeterminate · error + disabled",
            ControlState {
                error: true,
                disabled: true,
                ..Default::default()
            },
            CheckState::Mixed,
        ),
        ("Checked", ControlState::default(), CheckState::Checked),
        (
            "Checked · pressed",
            ControlState {
                selected: true,
                pressed: true,
                ..Default::default()
            },
            CheckState::Checked,
        ),
        (
            "Checked · hover",
            ControlState {
                selected: true,
                hovered: true,
                ..Default::default()
            },
            CheckState::Checked,
        ),
        (
            "Checked · focus-visible",
            ControlState {
                selected: true,
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            CheckState::Checked,
        ),
        ("Indeterminate", ControlState::default(), CheckState::Mixed),
        (
            "Indeterminate · focus-visible",
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            CheckState::Mixed,
        ),
        (
            "Checked · disabled",
            ControlState {
                selected: true,
                disabled: true,
                ..Default::default()
            },
            CheckState::Checked,
        ),
        (
            "Indeterminate · disabled",
            ControlState {
                disabled: true,
                ..Default::default()
            },
            CheckState::Mixed,
        ),
    ]
}
pub(super) fn standard_states() -> [(&'static str, ControlState); 7] {
    [
        ("default", ControlState::default()),
        (
            "hover",
            ControlState {
                hovered: true,
                ..Default::default()
            },
        ),
        (
            "pressed",
            ControlState {
                pressed: true,
                ..Default::default()
            },
        ),
        (
            "focus-visible",
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
        ),
        (
            "disabled",
            ControlState {
                disabled: true,
                ..Default::default()
            },
        ),
        (
            "loading",
            ControlState {
                loading: true,
                ..Default::default()
            },
        ),
        (
            "error + focus",
            ControlState {
                error: true,
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
        ),
    ]
}

/// Passive sized reference, using the same production ButtonModel renderer as
/// the original matrix/variants. Actual actions use Button in the live Host.
#[derive(Debug)]
struct ButtonFixture {
    state: ControlState,
    label: &'static str,
    size: ButtonSize,
    variant: ButtonVariant,
}
impl Component for ButtonFixture {
    fn measure(&mut self, mut text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(ButtonModel::measure(
            &mut text,
            self.label,
            available.width.min(200.),
            self.size,
            TextRole::UiBody,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let mut model = ButtonModel::default();
        model.state = self.state;
        model.render(
            scene,
            &mut text,
            self.label,
            frame,
            Some(clip),
            theme.palette,
            self.variant,
            self.size,
            TextRole::UiBody,
            ButtonTextAlign::default(),
        );
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("component", "Button visual fixture"),
            InspectionProperty::new("state", format!("{:?}", self.state)),
            InspectionProperty::new("label", self.label),
        ])
    }
}

fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn stack(id: ComponentId, children: Vec<Content>) -> Content {
    Content::new(
        id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            children,
            flexible_child: None,
        },
    )
}

/// Passive reference fixture delegates actual measurement, arrangement and
/// painting to the production adapter. It never publishes a second interactive
/// checkbox or native focus owner. Simulated focus remains explicitly visual.
#[derive(Debug)]
struct MatrixCheckbox {
    state: ControlState,
    checked: CheckState,
    control: CheckboxContent,
}
impl MatrixCheckbox {
    fn new(state: ControlState, checked: CheckState) -> Self {
        Self {
            state,
            checked,
            control: CheckboxContent::new("Notifications", checked),
        }
    }
}
impl Component for MatrixCheckbox {
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        self.control.inspection_properties()
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.control.measure(text, available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let mut button = ButtonModel::default();
        button.state = self.state;
        self.control = CheckboxContent::from_original(
            "Notifications",
            Checkbox {
                button,
                checked: self.checked,
            },
        );
        self.control.arrange(text, frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.control
            .paint(scene, &mut FixtureText(text), frame, clip, theme);
    }
}

/// One shared section-row layout. Its original horizontal ScrollContent owns
/// overflow/reveal; this parent only derives a finite height from real buttons.
#[derive(Debug)]
struct SectionBar {
    children: Vec<Content>,
    arranged: bool,
}
impl Component for SectionBar {
    fn host_reveal_passthrough(&self) -> bool {
        // This parent adds no clip or coordinate transform. Its one arranged
        // Scroll child translates unchanged with the enclosing page viewport.
        self.arranged
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        let scroll = self.children[0].component_mut::<ScrollContent>().unwrap();
        let natural = scroll.children_mut().unwrap()[0]
            .measure_intrinsic(text, Size::new(1e8, available.height))
            .size;
        let bleed =
            -tekes_ui_controls::extended::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
                .focus_bounds(text.backing_scale())
                .min_x();
        let height = natural.height
            + 2. * bleed
            + tekes_ui_tokens::proposal::SCROLLBAR_WIDTH
            + tekes_ui_tokens::proposal::SCROLLBAR_INSET;
        scroll.viewport_height = Some(height);
        self.children[0].measure(text, Size::new(available.width, height))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.children[0].arrange(text, frame, clip);
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }
}

pub(crate) struct BasicStory {
    state: StoryState,
    entry: &'static Entry,
    retained_live: Option<Content>,
    pub(crate) host: ContentHost,
}
impl BasicStory {
    pub(crate) fn new(entry: &'static Entry) -> Self {
        let state = StoryState::for_entry(entry);
        let mut children = vec![
            super::story_heading(92004, entry.title()),
            text(92005, super::story_docs(entry)),
        ];
        children.push(section_bar(entry, COMMAND));
        children.push(Self::body(entry, &state));
        let mut host = ContentHost::new(Content::new(
            ROOT,
            ListContent::scrollable(format!("{} stories", entry.title()), ROWS, children)
                .content_padding(16.),
        ))
        .expect("unique basic story IDs");
        host.set_action_events(true);
        Self {
            state,
            entry,
            host,
            retained_live: None,
        }
    }
    fn body(entry: &'static Entry, state: &StoryState) -> Content {
        let mut children = vec![text(
            92007,
            [
                "Docs",
                "Playground",
                "State matrix",
                "Behaviors",
                "Edge cases",
                "Variants / sizes",
            ][state.section],
        )];
        match state.section {
            0 => {
                children.push(text(
                    92008,
                    format!(
                        "Component: {} | maturity: experimental\nUsage: {}",
                        entry.component(),
                        super::api_reference(entry)
                    ),
                ));
                children.push(text(
                    92010,
                    format!(
                        "Story routes: {}",
                        [
                            "docs",
                            "playground",
                            "states",
                            "behaviors",
                            "edge-cases",
                            "variants"
                        ]
                        .into_iter()
                        .filter(|section| entry.supports_story(section))
                        .map(|section| format!("{}/{section}", entry.slug()))
                        .collect::<Vec<_>>()
                        .join(" · ")
                    ),
                ));
                children.push(text(92011, "State comparison is a visual aid, not a keyboard or accessibility test. Native input services are exercised through the real window."));
                children.push(text(92009, "State is local to this route. Reset restores deterministic fixture data. The host supplies the animation clock; no random source or real network request is used."));
                for (i, line) in super::contract_summary(entry).into_iter().enumerate() {
                    children.push(text(92500 + i as u64, line));
                }
            }
            2 if super::apphost_collections::supports(entry.renderer()) => {
                children.extend(super::apphost_collections::matrix(entry.renderer()));
            }
            1 | 3 if super::apphost_collections::supports(entry.renderer()) => {
                children.push(text(
                    92008,
                    super::extended::behavior_copy(entry.renderer()),
                ));
                children.extend(super::apphost_collections::playground(
                    entry.renderer(),
                    state,
                ));
            }
            2 if super::apphost_selection::supports(entry.renderer()) => {
                children.push(text(
                    92008,
                    "Visual comparison only. Use Playground for real focus and keyboard behavior.",
                ));
                children.extend(super::apphost_selection::matrix(entry.renderer()));
            }
            2 => {
                children.push(text(92008, "Visual fixtures only — simulated focus is not a focus test. Real interaction: Playground."));
                if entry.renderer() == "checkbox" {
                    for (i, (name, state, checked)) in matrix_states().into_iter().enumerate() {
                        children.push(text(92300 + i as u64 * 2, name));
                        children.push(Content::new(
                            ComponentId(92301 + i as u64 * 2),
                            MatrixCheckbox::new(state, checked),
                        ));
                    }
                } else {
                    for (i, (name, state)) in standard_states()
                        .into_iter()
                        .filter(|(name, _)| *name != "error + focus")
                        .enumerate()
                    {
                        children.push(text(92300 + i as u64 * 2, name));
                        children.push(Content::new(
                            ComponentId(92301 + i as u64 * 2),
                            ButtonFixture {
                                state,
                                label: if state.loading {
                                    "Loading…"
                                } else {
                                    "Sample"
                                },
                                size: ButtonSize::Body,
                                variant: ButtonVariant::Quiet,
                            },
                        ));
                    }
                }
            }
            3 => {
                children.push(text(
                    92008,
                    if super::extended::supports(entry.renderer()) {
                        super::extended::behavior_copy(entry.renderer())
                    } else {
                        super::behavior_copy(entry.renderer())
                    },
                ));
                children.push(text(92009, "State matrices are deterministic visual comparisons. Use Playground for real input. Native VoiceOver and physical IME checks remain manual."));
            }
            5 if entry.renderer() == "split-view" => {
                children.extend(super::apphost_split::variants());
            }
            5 if entry.renderer() == "number-field" => {
                children.push(text(92008, "Bounded numeric stepper: 1…32, step 1. Editable numeric input is not implemented."));
            }
            5 if super::apphost_display::supports(entry.renderer()) => {
                children.extend(super::apphost_display::children(
                    entry.renderer(),
                    state,
                    true,
                ));
            }
            5 if super::apphost_layout::supports(entry.renderer()) => {
                children.extend(super::apphost_layout::children(entry.renderer(), true));
            }
            5 => {
                for (i, (label, size)) in [
                    ("Small", ButtonSize::Sm),
                    ("Medium", ButtonSize::Md),
                    ("Large", ButtonSize::Lg),
                ]
                .into_iter()
                .enumerate()
                {
                    for (j, (label, variant)) in [
                        (label, ButtonVariant::Primary),
                        ("Quiet", ButtonVariant::Quiet),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        children.push(Content::new(
                            ComponentId(92400 + i as u64 * 2 + j as u64),
                            ButtonFixture {
                                state: ControlState::default(),
                                label,
                                size,
                                variant,
                            },
                        ));
                    }
                }
            }
            _ if entry.renderer() == "split-view" => {
                children.extend(super::apphost_split::playground(state));
            }
            _ if super::apphost_display::supports(entry.renderer()) => {
                children.extend(super::apphost_display::children(
                    entry.renderer(),
                    state,
                    false,
                ));
            }
            _ if super::apphost_selection::supports(entry.renderer()) => {
                children.extend(super::apphost_selection::playground(
                    entry.renderer(),
                    state,
                ));
            }
            _ if super::apphost_layout::supports(entry.renderer()) => {
                children.extend(super::apphost_layout::children(entry.renderer(), false));
            }
            _ => {
                children.push(Content::new(
                    ComponentId(COMMAND + 10),
                    Button::new(if state.disabled {
                        "Disabled: on"
                    } else {
                        "Disabled: off"
                    }),
                ));
                if entry.renderer() == "checkbox" {
                    children.push(Content::new(
                        ComponentId(COMMAND + 13),
                        Button::new("Mixed"),
                    ));
                    let label = if state.section == 4 {
                        "Receive project notifications 项目通知設定 日本語 — a long setting label wraps inside the available row"
                    } else {
                        "Receive project notifications"
                    };
                    let mut model = state.checkbox.clone();
                    model.button.cancel();
                    model.button.blur();
                    model.button.state.disabled = state.disabled;
                    children.push(Content::new(
                        SPECIMEN,
                        CheckboxContent::from_original(label, model),
                    ));
                } else {
                    children.push(Content::new(
                        ComponentId(COMMAND + 11),
                        Button::new(if state.loading {
                            "Loading: on"
                        } else {
                            "Loading: off"
                        }),
                    ));
                    let mut button = Button::new(if state.loading {
                        "Loading…"
                    } else if state.section == 4 {
                        "A long action label 中日文"
                    } else {
                        "Run action"
                    });
                    button.button.state.disabled = state.disabled;
                    button.button.state.loading = state.loading;
                    children.push(Content::new(SPECIMEN, button));
                }
                children.push(text(
                    LOG.0,
                    state.log.iter().cloned().collect::<Vec<_>>().join("\n"),
                ));
            }
        }
        stack(BODY, children)
    }
    pub(crate) fn after_host_visit(&mut self) {
        self.state.window_active = self.host.is_window_active();
        if let Some(control) = self
            .host
            .get(SPECIMEN)
            .and_then(|c| c.component::<CheckboxContent>())
        {
            self.state.checkbox = control.model().clone();
        }
        if super::apphost_selection::supports(self.entry.renderer()) {
            super::apphost_selection::sync_model(&mut self.state, &mut self.host);
        }
        if self.entry.renderer() == "split-view" {
            super::apphost_split::sync(&mut self.state, &mut self.host);
        }
        let records = self.host.drain_action_records();
        if super::apphost_collections::supports(self.entry.renderer()) {
            // Settle typed intents before sorting can replace their snapshots.
            for record in &records {
                if record.action.target == SPECIMEN
                    && let Some(payload) = &record.payload
                {
                    super::apphost_collections::action(&mut self.state, &mut self.host, payload);
                }
            }
            super::apphost_collections::sync_model(
                self.entry.renderer(),
                &mut self.state,
                &mut self.host,
            );
        }
        for record in records {
            if self.entry.renderer() == "split-view"
                && record.action.kind == ContentActionKind::Activate
                && super::apphost_split::choose(&mut self.host, record.action.target)
            {
                super::apphost_split::sync(&mut self.state, &mut self.host);
                self.state
                    .record(if record.action.target == ComponentId(123091) {
                        "split pane: leading"
                    } else {
                        "split pane: trailing"
                    });
                self.host
                    .get_mut(LOG)
                    .unwrap()
                    .component_mut::<TextContent>()
                    .unwrap()
                    .text = self
                    .state
                    .log
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");
                continue;
            }
            if self.entry.renderer() == "disclosure"
                && record.action.target == super::apphost_navigation::TRIGGER
            {
                if let Some(ContentActionPayload::DisclosureExpansion(intent)) = record.payload {
                    let control = self
                        .host
                        .get_mut(SPECIMEN)
                        .unwrap()
                        .component_mut::<DisclosureContent>()
                        .unwrap();
                    if control.confirm(&intent, true) {
                        self.state.extended.disclosure.open = control.expanded();
                        self.state
                            .record(format!("disclosure: {}", intent.expanded()));
                        self.host
                            .get_mut(LOG)
                            .unwrap()
                            .component_mut::<TextContent>()
                            .unwrap()
                            .text = self
                            .state
                            .log
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }
                continue;
            }
            if record.action.target == SPECIMEN {
                if let Some(ContentActionPayload::CheckboxState(intent)) = record.payload {
                    self.state.record(format!("change: {:?}", intent.checked()));
                    if let Some(log) = self.host.get_mut(LOG) {
                        log.component_mut::<TextContent>().unwrap().text = self
                            .state
                            .log
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                } else if let Some(ContentActionPayload::BreadcrumbNavigation { index, .. }) =
                    record.payload
                {
                    self.state.extended.selected = index;
                    self.state.record(format!("breadcrumb: {index}"));
                    self.host
                        .get_mut(LOG)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = self
                        .state
                        .log
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                } else if let Some(ContentActionPayload::CommandSelection { index, label }) =
                    record.payload
                {
                    self.state.extended.selected = index;
                    self.state.record(format!("command: {label}"));
                    self.host
                        .get_mut(LOG)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = self
                        .state
                        .log
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                } else if let Some(ContentActionPayload::ToolbarCommand(id)) = record.payload {
                    self.state.activate(id as usize, "toolbar");
                    self.host
                        .get_mut(LOG)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = self
                        .state
                        .log
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                } else if self.entry.renderer() == "icon-button"
                    && record.action.kind == ContentActionKind::Activate
                {
                    self.state.record("icon-button: Create item");
                    self.host
                        .get_mut(LOG)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = self
                        .state
                        .log
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                } else if self.entry.renderer() == "link"
                    && record.action.kind == ContentActionKind::Activate
                {
                    self.state.record("link: activated");
                    self.host
                        .get_mut(LOG)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = self
                        .state
                        .log
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                } else if self.entry.renderer() == "button"
                    && record.action.kind == ContentActionKind::Activate
                {
                    self.state.activate(20, "button");
                    self.host
                        .get_mut(LOG)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = self
                        .state
                        .log
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            } else if record.action.kind == ContentActionKind::Activate {
                let Some(command) = record.action.target.0.checked_sub(COMMAND) else {
                    continue;
                };
                if command == 20 && super::apphost_display::supports(self.entry.renderer()) {
                    self.state.activate(20, self.entry.renderer());
                    super::apphost_display::after_action(
                        self.entry.renderer(),
                        &self.state,
                        &mut self.host,
                    );
                } else if matches!(command, 0..=6 | 10 | 11 | 13) {
                    let previous_section = self.state.section;
                    self.state.activate(command as usize, self.entry.renderer());
                    if matches!(command, 10 | 11 | 13) {
                        if super::apphost_selection::supports(self.entry.renderer()) {
                            super::apphost_selection::apply_disabled(
                                &mut self.host,
                                super::apphost_selection::disabled(
                                    self.entry.renderer(),
                                    &self.state,
                                ),
                            );
                        } else if self.entry.renderer() == "checkbox" {
                            let control = self
                                .host
                                .get_mut(SPECIMEN)
                                .expect("Playground property target")
                                .component_mut::<CheckboxContent>()
                                .unwrap();
                            control.set_checked(self.state.checkbox.checked);
                            control.set_availability(self.state.disabled, false);
                        } else {
                            let button = self
                                .host
                                .get_mut(SPECIMEN)
                                .unwrap()
                                .component_mut::<Button>()
                                .unwrap();
                            button.button.cancel();
                            button.button.state.disabled = self.state.disabled;
                            button.button.state.loading = self.state.loading;
                            button.label = if self.state.loading {
                                "Loading…"
                            } else if self.state.section == 4 {
                                "A long action label 中日文"
                            } else {
                                "Run action"
                            }
                            .into();
                            self.host
                                .get_mut(ComponentId(COMMAND + 11))
                                .unwrap()
                                .component_mut::<Button>()
                                .unwrap()
                                .label = if self.state.loading {
                                "Loading: on"
                            } else {
                                "Loading: off"
                            }
                            .into();
                        }
                        self.host
                            .get_mut(ComponentId(COMMAND + 10))
                            .unwrap()
                            .component_mut::<Button>()
                            .unwrap()
                            .label =
                            if if super::apphost_selection::supports(self.entry.renderer()) {
                                super::apphost_selection::disabled(
                                    self.entry.renderer(),
                                    &self.state,
                                )
                            } else {
                                self.state.disabled
                            } {
                                "Disabled: on"
                            } else {
                                "Disabled: off"
                            }
                            .into();
                        self.host
                            .get_mut(LOG)
                            .unwrap()
                            .component_mut::<TextContent>()
                            .unwrap()
                            .text = self
                            .state
                            .log
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\n");
                    } else if super::apphost_collections::supports(self.entry.renderer())
                        || self.entry.renderer() == "command-palette"
                    {
                        let live = |section| {
                            section == 1
                                || (section == 3
                                    && super::apphost_collections::supports(self.entry.renderer()))
                        };
                        self.host.focus(None, false);
                        let body = self.host.remove(BODY).expect("mounted collection section");
                        if command == 0 {
                            self.retained_live = None;
                        } else if live(previous_section) {
                            self.retained_live = Some(body);
                        }
                        let next = if live(self.state.section) {
                            self.retained_live
                                .take()
                                .unwrap_or_else(|| Self::body(self.entry, &self.state))
                        } else {
                            Self::body(self.entry, &self.state)
                        };
                        self.host
                            .insert(ROWS, next)
                            .expect("collection section slot");
                        self.host
                            .get_mut(ComponentId(92007))
                            .unwrap()
                            .component_mut::<TextContent>()
                            .unwrap()
                            .text = [
                            "Docs",
                            "Playground",
                            "State matrix",
                            "Behaviors",
                            "Edge cases",
                            "Variants / sizes",
                        ][self.state.section]
                            .into();
                    } else {
                        self.host.remove(BODY).expect("mounted Checkbox section");
                        self.host
                            .insert(ROWS, Self::body(self.entry, &self.state))
                            .expect("Checkbox section slot");
                    }
                }
            }
        }
    }
}

/// Shared section navigation; all pages use the same formal horizontal scroll.
pub(super) fn section_bar(entry: &Entry, command: u64) -> Content {
    let commands = Content::new(
        ComponentId(92013),
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: 8.,
            },
            flexible_child: None,
            children: [
                (1, "Docs"),
                (2, "Playground"),
                (3, "State matrix"),
                (4, "Behaviors"),
                (5, "Edge cases"),
                (6, "Variants / sizes"),
                (0, "Reset"),
            ]
            .into_iter()
            .filter(|(id, _)| {
                *id == 0
                    || entry.supports_story(
                        [
                            "docs",
                            "playground",
                            "states",
                            "behaviors",
                            "edge-cases",
                            "variants",
                        ][(*id - 1) as usize],
                    )
            })
            .map(|(id, label)| Content::new(ComponentId(command + id), Button::new(label)))
            .collect(),
        },
    );
    Content::new(
        ComponentId(92006),
        SectionBar {
            arranged: false,
            children: vec![Content::new(
                ComponentId(92012),
                ScrollContent::new(tekes_ui_controls::ScrollArea::default(), commands)
                    .horizontal(true)
                    .scrollbar(true),
            )],
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tekes_ui_controls::layout_view::IconRenderer;
    struct Images;
    impl IconRenderer for Images {
        fn resource_identity(&self) -> Option<u64> {
            Some(1)
        }
        fn register_icon_bitmap(
            &mut self,
            _: &[u8],
            _: u32,
            _: u32,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(tekes_ui_scene::ImageId(1), Rc::new(())))
        }
        fn prepare_icon(
            &mut self,
            _: &str,
            _: f32,
            _: f32,
            _: tekes_ui_tokens::Rgba,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(tekes_ui_scene::ImageId(2), Rc::new(())))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: tekes_ui_tokens::Rgba) {}
    }
    fn render_navigation_story(
        story: &mut BasicStory,
        text: &mut crate::gallery::text::TextEngine,
    ) {
        let frame = Rect::new(0., 0., 740., 5000.);
        story
            .host
            .render(
                &mut Scene::new(),
                &mut WithIconRenderer::new(text, &mut Images),
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
    }
    fn press_navigation_story(
        story: &mut BasicStory,
        text: &mut crate::gallery::text::TextEngine,
        target: ComponentId,
        part: Option<usize>,
    ) {
        render_navigation_story(story, text);
        assert!(
            story
                .host
                .accessibility_action(target, part, AccessibilityAction::Press)
                .consumed,
            "{} {target:?}",
            story.entry.renderer()
        );
        story.after_host_visit();
        render_navigation_story(story, text);
    }
    #[test]
    fn apphost_tabs_disclosure_and_slider_keep_original_live_consumers() {
        let mut text = crate::gallery::text::TextEngine::new();
        for renderer in ["tabs", "disclosure", "slider"] {
            let entry = super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap();
            let mut story = BasicStory::new(entry);
            story.host.set_window_active(true);
            render_navigation_story(&mut story, &mut text);
            match renderer {
                "tabs" => {
                    press_navigation_story(&mut story, &mut text, SPECIMEN, Some(1));
                    assert_eq!(story.state.tabs_view.selected(), 1);
                }
                "disclosure" => {
                    press_navigation_story(
                        &mut story,
                        &mut text,
                        super::super::apphost_navigation::TRIGGER,
                        None,
                    );
                    assert!(story.state.extended.disclosure.open);
                }
                _ => {}
            }
            press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 3), None);
            if renderer == "slider" {
                use tekes_ui_composition_consumer::slider_content::{EDITOR, RANGE};
                assert_eq!(
                    story
                        .host
                        .get(RANGE)
                        .unwrap()
                        .component::<SliderContent>()
                        .unwrap()
                        .model
                        .value(),
                    60.
                );
                assert!(story.host.focus(Some(RANGE), true));
                render_navigation_story(&mut story, &mut text);
                assert!(
                    story
                        .host
                        .accessibility_action(RANGE, None, AccessibilityAction::Increment)
                        .changed
                );
                render_navigation_story(&mut story, &mut text);
                assert_eq!(
                    story
                        .host
                        .get(RANGE)
                        .unwrap()
                        .component::<SliderContent>()
                        .unwrap()
                        .model
                        .value(),
                    65.
                );
                assert_eq!(
                    story.host.text_surface(EDITOR).unwrap().text(),
                    "Keep this draft"
                );
            } else {
                // Every original independent status fixture is a child of this
                // Host, and its header still changes visibility/selection.
                for i in 0..5 {
                    let base = 102000 + i * 10;
                    assert!(story.host.get(ComponentId(base + 4)).is_some());
                    assert!(story.host.get(ComponentId(base + 6)).is_some());
                    press_navigation_story(
                        &mut story,
                        &mut text,
                        ComponentId(base + 2),
                        (renderer == "tabs").then_some(1),
                    );
                    let header = story
                        .host
                        .semantics()
                        .into_iter()
                        .find(|n| n.id == ComponentId(base + 2))
                        .unwrap();
                    if renderer == "tabs" {
                        assert_eq!(header.semantics.value.as_deref(), Some("1"));
                    } else {
                        assert_eq!(header.semantics.expanded, Some(false));
                    }
                }
            }
            press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 1), None);
            press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 2), None);
            if renderer == "tabs" {
                assert_eq!(story.state.tabs_view.selected(), 1);
            }
            if renderer == "disclosure" {
                assert!(story.state.extended.disclosure.open);
            }
            press_navigation_story(&mut story, &mut text, ComponentId(COMMAND), None);
            if renderer == "tabs" {
                assert_eq!(story.state.tabs_view.selected(), 0);
            }
            if renderer == "disclosure" {
                assert!(!story.state.extended.disclosure.open);
            }
        }
    }
    #[test]
    fn apphost_command_palette_uses_scroll_host_trigger_and_original_query() {
        let entry = super::super::entries()
            .iter()
            .find(|e| e.renderer() == "command-palette")
            .unwrap();
        let mut story = BasicStory::new(entry);
        story.host.set_window_active(true);
        let mut text = crate::gallery::text::TextEngine::new();
        press_navigation_story(&mut story, &mut text, SPECIMEN, None);
        assert!(
            story
                .host
                .get(SPECIMEN)
                .unwrap()
                .component::<CommandPaletteContent>()
                .unwrap()
                .is_open()
        );
        let query = super::super::apphost_command::QUERY;
        story.host.focus(Some(query), true);
        assert_eq!(
            story.host.focused(),
            Some(query),
            "query reveal: {:?}",
            story
                .host
                .plan_host_reveal(query, Default::default(), &[], Default::default())
        );
        render_navigation_story(&mut story, &mut text);
        story
            .host
            .event_with_text(
                &ContentEvent::Commit {
                    text: "Open".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        story.after_host_visit();
        render_navigation_story(&mut story, &mut text);
        assert_eq!(
            story
                .host
                .text_surface(super::super::apphost_command::QUERY)
                .unwrap()
                .text(),
            "Open"
        );
        story.host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Escape,
            down: true,
        });
        story.after_host_visit();
        let baseline = story
            .host
            .get(query)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input()
            .external_baseline();
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 1), None);
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 2), None);
        let editor = story
            .host
            .get(query)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert_eq!(editor.surface.text(), "Open");
        assert_eq!(editor.surface.input().external_baseline(), baseline);
        // Original CommandPalette::open starts a new lookup with an empty query.
        press_navigation_story(&mut story, &mut text, SPECIMEN, None);
        assert_eq!(
            story
                .host
                .text_surface(super::super::apphost_command::QUERY)
                .unwrap()
                .text(),
            ""
        );
    }
    #[test]
    fn apphost_icon_button_and_split_use_original_action_and_divider_owners() {
        let mut text = crate::gallery::text::TextEngine::new();
        let entry = super::super::entries()
            .iter()
            .find(|e| e.renderer() == "icon-button")
            .unwrap();
        let mut story = BasicStory::new(entry);
        story.host.set_window_active(true);
        press_navigation_story(&mut story, &mut text, SPECIMEN, None);
        assert_eq!(
            story
                .state
                .log
                .iter()
                .filter(|line| line.as_str() == "icon-button: Create item")
                .count(),
            1
        );
        assert_eq!(
            story
                .host
                .get(SPECIMEN)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .accessible_name(),
            "Create item"
        );
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 10), None);
        assert!(
            !story
                .host
                .accessibility_action(SPECIMEN, None, AccessibilityAction::Press)
                .activated
        );
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 3), None);
        assert!(story.host.get(ComponentId(122101)).is_some());
        let entry = super::super::entries()
            .iter()
            .find(|e| e.renderer() == "split-view")
            .unwrap();
        let mut story = BasicStory::new(entry);
        story.host.set_window_active(true);
        render_navigation_story(&mut story, &mut text);
        let divider = super::super::apphost_split::DIVIDER;
        story.host.focus(Some(divider), true);
        assert_eq!(story.host.focused(), Some(divider));
        render_navigation_story(&mut story, &mut text);
        let before = story.state.extended.split.ratio();
        assert!(
            story
                .host
                .event(&ContentEvent::Key {
                    key: tekes_ui_controls::ControlKey::Right,
                    down: true
                })
                .consumed
        );
        story.after_host_visit();
        assert!(story.state.extended.split.ratio() > before);
        let ratio = story.state.extended.split.ratio();
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 1), None);
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 2), None);
        assert_eq!(
            story
                .host
                .get(SPECIMEN)
                .unwrap()
                .component::<SplitViewContent>()
                .unwrap()
                .config()
                .unwrap()
                .ratio,
            ratio
        );
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 6), None);
        assert!(story.host.get(ComponentId(123105)).is_some());
        assert_eq!(story.state.extended.split.ratio(), ratio);
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + 2), None);
        press_navigation_story(&mut story, &mut text, ComponentId(COMMAND), None);
        assert_eq!(story.state.extended.split.ratio(), 0.5);
    }
    #[test]
    fn apphost_split_scroll_host_pointer_captures_drags_and_releases() {
        let entry = super::super::entries()
            .iter()
            .find(|e| e.renderer() == "split-view")
            .unwrap();
        let mut story = BasicStory::new(entry);
        let mut text = crate::gallery::text::TextEngine::new();
        let frame = Rect::new(0., 0., 740., 600.);
        let present = |story: &mut BasicStory, text: &mut crate::gallery::text::TextEngine| {
            story
                .host
                .render(
                    &mut Scene::new(),
                    text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
        };
        story.host.set_window_active(true);
        present(&mut story, &mut text);
        let divider = super::super::apphost_split::DIVIDER;
        let start = story.host.get(divider).unwrap().visible().unwrap().center();
        let before = story.state.extended.split.ratio();
        assert!(
            story.host.event(&ContentEvent::PointerDown(start)).consumed,
            "real divider pointer down must be admitted through the Scroll Host"
        );
        assert!(story.host.pointer_capture().is_some());
        let end = tekes_ui_scene::Point::new(start.x + 90., start.y);
        assert!(story.host.event(&ContentEvent::PointerMove(end)).consumed);
        story.after_host_visit();
        present(&mut story, &mut text);
        assert!(story.state.extended.split.ratio() > before);
        assert!(
            story.host.pointer_capture().is_some(),
            "repaint preserves the original divider gesture"
        );
        story.host.event(&ContentEvent::PointerUp(end));
        assert!(story.host.pointer_capture().is_none());
        let settled = story.state.extended.split.ratio();
        story.host.event(&ContentEvent::PointerMove(start));
        story.after_host_visit();
        assert_eq!(story.state.extended.split.ratio(), settled);
        story.host.set_window_active(false);
        let point = story.host.get(divider).unwrap().visible().unwrap().center();
        assert!(!story.host.event(&ContentEvent::PointerDown(point)).consumed);
        assert!(story.host.pointer_capture().is_none());
    }

    #[test]
    fn apphost_collection_chapters_preserve_tree_expansion_and_table_sort() {
        let mut text = crate::gallery::text::TextEngine::new();
        for renderer in ["tree", "data-table"] {
            let entry = super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap();
            let mut story = BasicStory::new(entry);
            story.host.set_window_active(true);
            render_navigation_story(&mut story, &mut text);
            let parts = story
                .host
                .semantics()
                .into_iter()
                .find(|n| n.id == SPECIMEN)
                .unwrap()
                .parts;
            let (part, action) = if renderer == "tree" {
                (
                    parts
                        .iter()
                        .find(|p| p.semantics.expanded == Some(true))
                        .unwrap()
                        .index,
                    AccessibilityAction::SetExpanded(false),
                )
            } else {
                (
                    parts
                        .iter()
                        .find(|p| {
                            matches!(
                                p.semantics.role,
                                ContentRole::TableColumnHeader { index: 0, .. }
                            )
                        })
                        .unwrap()
                        .index,
                    AccessibilityAction::Press,
                )
            };
            assert!(
                story
                    .host
                    .accessibility_action(SPECIMEN, Some(part), action)
                    .consumed
            );
            story.after_host_visit();
            render_navigation_story(&mut story, &mut text);
            let sorted = if renderer == "data-table" {
                Some(
                    story
                        .host
                        .get(SPECIMEN)
                        .unwrap()
                        .component::<DataTableContent>()
                        .unwrap()
                        .model()
                        .display_indices()
                        .to_vec(),
                )
            } else {
                None
            };
            for section in [1, 2, 4, 3, 2] {
                press_navigation_story(&mut story, &mut text, ComponentId(COMMAND + section), None);
                if matches!(section, 2 | 4) {
                    if renderer == "tree" {
                        assert_eq!(
                            story
                                .host
                                .semantics()
                                .into_iter()
                                .find(|n| n.id == SPECIMEN)
                                .unwrap()
                                .parts
                                .len(),
                            1
                        );
                    } else {
                        assert_eq!(
                            story
                                .host
                                .get(SPECIMEN)
                                .unwrap()
                                .component::<DataTableContent>()
                                .unwrap()
                                .model()
                                .display_indices(),
                            *sorted.as_ref().unwrap()
                        );
                    }
                }
            }
            press_navigation_story(&mut story, &mut text, ComponentId(COMMAND), None);
            if renderer == "tree" {
                assert_eq!(
                    story
                        .host
                        .semantics()
                        .into_iter()
                        .find(|n| n.id == SPECIMEN)
                        .unwrap()
                        .parts
                        .len(),
                    3
                );
            } else {
                assert!(
                    story
                        .host
                        .get(SPECIMEN)
                        .unwrap()
                        .component::<DataTableContent>()
                        .unwrap()
                        .model()
                        .original()
                        .sort
                        .is_none()
                );
            }
        }
    }
    #[test]
    fn apphost_breadcrumb_toolbar_stepper_preserve_original_actions_and_bounds() {
        fn render(story: &mut BasicStory, text: &mut crate::gallery::text::TextEngine) {
            let frame = Rect::new(0., 0., 740., 1200.);
            story
                .host
                .render(
                    &mut Scene::new(),
                    text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
            story.after_host_visit();
        }
        fn press(
            story: &mut BasicStory,
            text: &mut crate::gallery::text::TextEngine,
            command: u64,
        ) {
            render(story, text);
            assert!(
                story
                    .host
                    .accessibility_action(
                        ComponentId(COMMAND + command),
                        None,
                        AccessibilityAction::Press
                    )
                    .consumed,
                "{} command {command}",
                story.entry.renderer()
            );
            story.after_host_visit();
            render(story, text);
        }
        let mut text = crate::gallery::text::TextEngine::new();
        for renderer in ["breadcrumb", "toolbar", "number-field"] {
            let entry = super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap();
            let mut story = BasicStory::new(entry);
            story.host.set_window_active(true);
            render(&mut story, &mut text);
            match renderer {
                "breadcrumb" => {
                    assert!(
                        !story
                            .host
                            .accessibility_action(SPECIMEN, Some(2), AccessibilityAction::Press)
                            .activated
                    );
                    assert!(
                        story
                            .host
                            .accessibility_action(SPECIMEN, Some(1), AccessibilityAction::Press)
                            .activated
                    );
                    story.after_host_visit();
                    assert_eq!(story.state.extended.selected, 1);
                    assert_eq!(
                        story
                            .state
                            .log
                            .iter()
                            .filter(|s| s.as_str() == "breadcrumb: 1")
                            .count(),
                        1
                    );
                }
                "toolbar" => {
                    assert!(story.host.focus(Some(SPECIMEN), true));
                    story.host.event(&ContentEvent::Key {
                        key: tekes_ui_controls::ControlKey::Right,
                        down: true,
                    });
                    story.host.event(&ContentEvent::Key {
                        key: tekes_ui_controls::ControlKey::Enter,
                        down: true,
                    });
                    story.after_host_visit();
                    assert_eq!(
                        story
                            .state
                            .log
                            .iter()
                            .filter(|s| s.as_str() == "toolbar: rename")
                            .count(),
                        1
                    );
                    assert!(!story.state.log.iter().any(|s| s == "toolbar: create"));
                }
                _ => {
                    assert!(
                        story.host.text_surface(SPECIMEN).is_none(),
                        "original NumberField must remain a non-editable stepper"
                    );
                    assert!(story.host.focus(Some(SPECIMEN), true));
                    story.host.event(&ContentEvent::Key {
                        key: tekes_ui_controls::ControlKey::End,
                        down: true,
                    });
                    story.after_host_visit();
                    assert_eq!(story.state.number.value(), 32.);
                    render(&mut story, &mut text);
                    assert!(
                        !story
                            .host
                            .accessibility_action(SPECIMEN, Some(1), AccessibilityAction::Press)
                            .changed
                    );
                    assert!(
                        story
                            .host
                            .accessibility_action(SPECIMEN, Some(0), AccessibilityAction::Press)
                            .changed
                    );
                    story.after_host_visit();
                    assert_eq!(story.state.number.value(), 31.);
                }
            }
            press(&mut story, &mut text, 3);
            assert!(story.host.get(SPECIMEN).is_none());
            press(&mut story, &mut text, 2);
            if renderer == "number-field" {
                assert_eq!(story.state.number.value(), 31.);
            }
            press(&mut story, &mut text, 10);
            if renderer == "toolbar" {
                let control = story
                    .host
                    .get(SPECIMEN)
                    .unwrap()
                    .component::<tekes_ui_controls::ToolbarControl>()
                    .unwrap();
                assert!(
                    control
                        .semantic_parts()
                        .iter()
                        .all(|part| part.semantics.disabled)
                );
            } else {
                assert!(
                    story
                        .host
                        .inspection_snapshot(SPECIMEN)
                        .unwrap()
                        .semantics
                        .disabled
                );
            }
            press(&mut story, &mut text, 0);
            if renderer == "number-field" {
                assert_eq!(story.state.number.value(), 8.);
            }
        }
    }
    #[test]
    fn apphost_link_and_segments_use_original_activation_and_retained_selection() {
        fn render(story: &mut BasicStory, text: &mut crate::gallery::text::TextEngine) {
            let frame = Rect::new(0., 0., 740., 1200.);
            story
                .host
                .render(
                    &mut Scene::new(),
                    text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
            story.after_host_visit();
        }
        fn press(
            story: &mut BasicStory,
            text: &mut crate::gallery::text::TextEngine,
            command: u64,
        ) {
            // Host actions consume the geometry acknowledged by the preceding
            // frame, just as the AppHost event/render cycle does.
            render(story, text);
            assert!(
                story
                    .host
                    .accessibility_action(
                        ComponentId(COMMAND + command),
                        None,
                        AccessibilityAction::Press
                    )
                    .consumed,
                "{} section command {command}",
                story.entry.renderer()
            );
            story.after_host_visit();
            render(story, text);
        }
        let mut text = crate::gallery::text::TextEngine::new();
        for renderer in ["link", "segmented-control"] {
            let entry = super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap();
            let mut story = BasicStory::new(entry);
            story.host.set_window_active(true);
            render(&mut story, &mut text);
            assert!(story.host.focus(Some(SPECIMEN), true));
            let key = if renderer == "link" {
                story.host.event(&ContentEvent::Key {
                    key: tekes_ui_controls::ControlKey::Space,
                    down: true,
                });
                story.host.event(&ContentEvent::Key {
                    key: tekes_ui_controls::ControlKey::Space,
                    down: false,
                });
                story.after_host_visit();
                assert!(!story.state.log.iter().any(|s| s == "link: activated"));
                tekes_ui_controls::ControlKey::Enter
            } else {
                tekes_ui_controls::ControlKey::Right
            };
            story.host.event(&ContentEvent::Key { key, down: true });
            story.after_host_visit();
            if renderer == "link" {
                assert_eq!(
                    story
                        .state
                        .log
                        .iter()
                        .filter(|s| s.as_str() == "link: activated")
                        .count(),
                    1
                );
            } else {
                assert_eq!(story.state.extended.segmented.selected, 1);
            }
            press(&mut story, &mut text, 3);
            assert!(story.host.get(SPECIMEN).is_none());
            press(&mut story, &mut text, 2);
            if renderer == "segmented-control" {
                assert_eq!(
                    story
                        .host
                        .get(SPECIMEN)
                        .unwrap()
                        .component::<SegmentedControlContent>()
                        .unwrap()
                        .model()
                        .selected,
                    1
                );
            }
            press(&mut story, &mut text, 10);
            assert!(!story.host.focus(Some(SPECIMEN), true));
            let before = story.state.log.len();
            assert!(
                !story
                    .host
                    .accessibility_action(SPECIMEN, None, AccessibilityAction::Press)
                    .activated
            );
            story.after_host_visit();
            assert_eq!(story.state.log.len(), before);
            press(&mut story, &mut text, 0);
            assert!(!story.state.extended.disabled);
            if renderer == "segmented-control" {
                assert_eq!(story.state.extended.segmented.selected, 0);
            }
        }
    }
    #[test]
    fn apphost_passive_matrices_borrow_live_text_without_requesting_animation_frames() {
        for renderer in [
            "checkbox",
            "switch",
            "radio-group",
            "slider",
            "link",
            "segmented-control",
            "breadcrumb",
            "toolbar",
            "number-field",
        ] {
            let entry = super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap();
            let mut state = StoryState::for_entry(entry);
            state.section = 2;
            let mut host = ContentHost::new(BasicStory::body(entry, &state)).unwrap();
            let mut text = crate::gallery::text::TextEngine::new();
            text.set_motion_enabled(true);
            let frame = Rect::new(0., 0., 740., 2000.);
            for millis in [42, 84] {
                text.begin_frame();
                text.set_motion_context(std::time::Duration::from_millis(millis), false);
                let clock = text.motion_context();
                host.render(
                    &mut Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
                assert!(
                    !text.motion_frame_requested(),
                    "passive {renderer} fixture restarted animation"
                );
                assert_eq!(
                    text.motion_context(),
                    clock,
                    "fixture changed the live host clock"
                );
            }
        }
    }
}
