//! Registry-driven native workbench. Fixtures orchestrate production control models;
//! no duplicate component implementation lives in a story.
mod brand_preview;
mod extended;
use super::{Page, PageCtx, text_demo::TextDemo};
use brand_preview::BrandPreview;
use std::{
    collections::{HashMap, VecDeque},
    sync::OnceLock,
    time::Duration,
};
use tekes_ui_controls::{
    Button, ButtonSize, ButtonTextAlign, ButtonTextRenderer, CheckState, Checkbox, ControlKey,
    ControlState, DialogEvent, DialogView, Field, MenuEvent, MenuLayout, MenuView, MenuWheelEvent,
    MenuWheelInput, NumberField, Select, TabsLayout, TabsView,
    extended::{Axis, ControlFrame},
    feedback_view::{
        FieldPresentation, FieldView, NumberFieldView, SkeletonView, SurfaceView, ToolbarView,
    },
    layout::{AlignmentOverride, MeasuredChild, layout_measured_children},
    layout_view::SelectView,
    paint,
};
use tekes_ui_platform_macos::{Event, KeyEventKind, MouseButton, MouseEventKind};
use tekes_ui_scene::{Point, Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, proposal};

#[derive(Debug)]
pub struct Entry(serde_json::Value);
impl Entry {
    fn get(&self, key: &str) -> &str {
        self.0[key].as_str().unwrap_or("")
    }
    pub fn title(&self) -> &str {
        self.get("title")
    }
    pub fn slug(&self) -> &str {
        self.get("slug")
    }
    pub fn section(&self) -> &str {
        self.get("section")
    }
    pub fn renderer(&self) -> &str {
        self.get("renderer")
    }
    pub fn planned(&self) -> bool {
        self.get("kind") == "planned"
    }
    pub fn component(&self) -> &str {
        self.get("component")
    }
    /// One capability contract for visible tabs, direct routes and Docs links.
    pub fn supports_story(&self, story: &str) -> bool {
        match story {
            "docs" | "playground" => true,
            "states" => {
                !self.planned()
                    && [
                        "button",
                        "checkbox",
                        "tabs",
                        "menu",
                        "dialog",
                        "field",
                        "area",
                        "search",
                        "select",
                        "field-meta",
                    ]
                    .contains(&self.renderer())
                    || (!self.planned() && extended::supports_story(self.renderer(), story))
            }
            "variants" => {
                !self.planned()
                    && [
                        "button",
                        "field",
                        "area",
                        "search",
                        "surface",
                        "number-field",
                    ]
                    .contains(&self.renderer())
                    || (!self.planned() && extended::supports_story(self.renderer(), story))
            }
            "behaviors" | "edge-cases" => {
                if extended::supports(self.renderer()) {
                    extended::supports_story(self.renderer(), story)
                } else {
                    !self.planned()
                        && ["component", "pattern", "example"].contains(&self.get("kind"))
                }
            }
            _ => false,
        }
    }
    pub fn matches(&self, query: &str) -> bool {
        let terms=format!("{} {} {} {} {} {}",self.title(),self.slug(),self.section(),self.component(),self.get("keywords"),if self.planned(){"planned"}else{"docs playground states state matrix variants sizes behaviors keyboard edge-cases hover pressed focus disabled loading"}).to_lowercase();
        if query.starts_with(&format!("{}/", self.slug())) {
            return route(query).is_some();
        }
        query.split_whitespace().all(|term| terms.contains(term))
    }
}
pub fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../../../design/gallery/workbench.json"))
                .expect("workbench registry");
        v["entries"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(Entry)
            .collect()
    })
}
pub fn entry(i: usize) -> &'static Entry {
    &entries()[i]
}
pub fn route(route: &str) -> Option<Page> {
    let (slug, story) = route.split_once('/').unwrap_or((route, "playground"));
    if ![
        "docs",
        "playground",
        "states",
        "behaviors",
        "edge-cases",
        "variants",
    ]
    .contains(&story)
    {
        return None;
    }
    entries()
        .iter()
        .position(|e| (e.slug() == slug || e.component() == slug) && e.supports_story(story))
        .map(Page::Workbench)
        .or_else(|| Page::ALL.into_iter().find(|p| p.slug() == route))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Invalid,
    Pending,
    Failed,
    Saved,
}
#[derive(Debug)]
pub struct StoryState {
    input: TextDemo,
    detail: TextDemo,
    pub checkbox: Checkbox,
    tabs_view: TabsView,
    menu_view: MenuView,
    select_view: SelectView,
    dialog_view: DialogView,
    pub select: Select,
    pub number: NumberField,
    field: Field,
    field_view: FieldView,
    surface_view: SurfaceView,
    toolbar_view: ToolbarView,
    number_view: NumberFieldView,
    skeleton_view: SkeletonView,
    pub phase: Phase,
    attempts: usize,
    focus: Option<usize>,
    pub section: usize,
    brand: usize,
    density: usize,
    disabled: bool,
    loading: bool,
    error: bool,
    hits: Vec<(usize, Rect)>,
    button_hits: Vec<usize>,
    button_clip: Option<Rect>,
    tabs_layout: Option<TabsLayout>,
    menu_layout: Option<MenuLayout>,
    select_menu_layout: Option<MenuLayout>,
    dialog_layout: Option<tekes_ui_controls::DialogLayout>,
    buttons: HashMap<usize, Button>,
    log: VecDeque<String>,
    phase_frame: Option<Rect>,
    event_log_frame: Option<Rect>,
    selected: usize,
    list_scroll: tekes_ui_controls::ScrollArea,
    list_bounds: Rect,
    focus_reveal: Option<usize>,
    preview: Rect,
    window_active: bool,
    pointer_began_composing: bool,
    extended: extended::ExtendedStory,
}
impl Default for StoryState {
    fn default() -> Self {
        Self {
            input: TextDemo::field("Studio", false, false),
            detail: TextDemo::field(
                include_str!("../../../../fixtures/text/gallery-reading-en.txt"),
                true,
                true,
            ),
            checkbox: Checkbox::default(),
            tabs_view: TabsView::new(3),
            menu_view: MenuView::new(3),
            select_view: SelectView::new(3),
            dialog_view: DialogView::default(),
            select: Select::new(3),
            number: NumberField::new(8., 1., 32., 1.).unwrap(),
            field: Field {
                label: "Workspace name".into(),
                help: "Use a clear name for your workspace.".into(),
                error: None,
                required: true,
            },
            field_view: FieldView::default(),
            surface_view: SurfaceView::default(),
            toolbar_view: ToolbarView::new(3, 4.),
            number_view: NumberFieldView::default(),
            skeleton_view: SkeletonView::default(),
            phase: Phase::Ready,
            attempts: 0,
            focus: None,
            section: 1,
            brand: 0,
            density: 0,
            disabled: false,
            loading: false,
            error: false,
            hits: Vec::new(),
            button_hits: Vec::new(),
            button_clip: None,
            tabs_layout: None,
            menu_layout: None,
            select_menu_layout: None,
            dialog_layout: None,
            buttons: HashMap::new(),
            log: VecDeque::new(),
            phase_frame: None,
            event_log_frame: None,
            selected: 0,
            list_scroll: tekes_ui_controls::ScrollArea::default(),
            list_bounds: Rect::default(),
            focus_reveal: None,
            preview: Rect::default(),
            window_active: true,
            pointer_began_composing: false,
            extended: extended::ExtendedStory::default(),
        }
    }
}
impl StoryState {
    fn visible_for_id(&self, id: usize) -> Option<Rect> {
        if (30..33).contains(&id)
            && let Some(layout) = &self.tabs_layout
        {
            return layout.items.get(id - 30)?.visible;
        }
        if (50..53).contains(&id)
            && let Some(layout) = &self.menu_layout
        {
            return layout.items.get(id - 50)?.visible;
        }
        if (60..63).contains(&id)
            && let Some(layout) = &self.select_menu_layout
        {
            return layout.items.get(id - 60)?.visible;
        }
        if (70..72).contains(&id)
            && let Some(layout) = &self.dialog_layout
        {
            return layout.buttons.get(id - 70)?.visible;
        }
        self.hits
            .iter()
            .find(|(hit_id, _)| *hit_id == id)
            .map(|(_, visible)| *visible)
    }

    /// Resolve controls in paint order. Buttons re-check their own retained
    /// rendered placement so drawing and activation cannot drift apart.
    fn hit_at(&self, point: Point) -> Option<usize> {
        self.hits.iter().rev().find_map(|(id, rect)| {
            if self.button_hits.contains(id) {
                self.buttons
                    .get(id)
                    .and_then(Button::layout)
                    .is_some_and(|layout| layout.hit_test(point) && rect.contains(point))
                    .then_some(*id)
            } else {
                rect.contains(point).then_some(*id)
            }
        })
    }

    fn for_entry(e: &Entry) -> Self {
        let mut s = Self::default();
        s.extended = extended::ExtendedStory::default();
        s.input = match e.renderer() {
            "area" => TextDemo::field(
                "A quiet room, an open window.\nWrite a complete paragraph here. 中日文と English.",
                true,
                false,
            ),
            "search" | "filter" | "combo-box" | "command-palette" => {
                TextDemo::field("", false, false)
            }
            _ => s.input,
        };
        s
    }
    fn record(&mut self, event: impl Into<String>) {
        if self.log.len() == 6 {
            self.log.pop_front();
        }
        self.log.push_back(event.into());
    }
    fn focusable_visible(&self, id: usize) -> bool {
        self.hits.iter().any(|(hit_id, _)| *hit_id == id)
            && !self
                .buttons
                .get(&id)
                .is_some_and(|button| button.state.disabled)
    }
    fn restore_focus(&self, candidate: Option<usize>) -> Option<usize> {
        candidate.filter(|id| self.focusable_visible(*id))
    }
    fn sync_owned_focus(&mut self, renderer: &str, target: Option<usize>, visible: bool) {
        if renderer == "toolbar" {
            self.toolbar_view.focus(
                target.filter(|id| (20..23).contains(id)).map(|id| id - 20),
                visible,
            );
        } else {
            self.toolbar_view.focus(None, false);
        }
        if renderer == "number-field" {
            if let Some(id @ (23 | 24)) = target {
                self.number_view.focus(id == 24, visible);
            } else {
                self.number_view.blur();
            }
        } else {
            self.number_view.blur();
        }
        if renderer == "select" && target == Some(20) {
            self.select_view.focus(visible);
        } else {
            self.select_view.blur(&mut self.select);
        }
    }
    fn menu_event(&mut self, event: MenuEvent, renderer: &str, select: bool) {
        match event {
            MenuEvent::Activated {
                index,
                restore_focus,
            } => {
                if select {
                    self.select_menu_layout = None;
                } else {
                    self.menu_layout = None;
                }
                self.focus = if select {
                    self.restore_focus(Some(20))
                } else {
                    self.restore_focus(restore_focus)
                };
                if select {
                    let _ = self.select.choose(index);
                    self.record(format!("select: {index}"));
                } else {
                    self.record(format!("menu action: {index}"));
                }
                let _ = renderer;
            }
            MenuEvent::Dismissed { restore_focus } => {
                if select {
                    self.select_menu_layout = None;
                } else {
                    self.menu_layout = None;
                }
                self.focus = if select {
                    self.restore_focus(Some(20))
                } else {
                    self.restore_focus(restore_focus)
                };
            }
            MenuEvent::None => {}
        }
    }
    fn dialog_event(&mut self, event: DialogEvent, renderer: &str, composition_guarded: bool) {
        match event {
            DialogEvent::Confirmed { restore_focus } => {
                self.focus = self.restore_focus(restore_focus);
                if renderer == "danger" {
                    if composition_guarded {
                        self.record("submit: deferred while text composition is active");
                    } else {
                        self.submit();
                    }
                }
                self.record("dialog: confirm; focus restored");
            }
            DialogEvent::Cancelled { restore_focus } => {
                self.focus = self.restore_focus(restore_focus);
                self.record("dialog: cancel; focus restored");
            }
            DialogEvent::Blocked | DialogEvent::None => {}
        }
    }
    fn submit(&mut self) {
        if self.phase == Phase::Pending {
            return;
        }
        if self.input.query().trim().is_empty() {
            self.phase = Phase::Invalid;
            self.record("validation: name is required");
        } else {
            self.phase = Phase::Pending;
            self.attempts += 1;
            self.record("submit: pending (Resolve fixture to complete)");
        }
    }
    fn resolve(&mut self) {
        if self.phase != Phase::Pending {
            return;
        }
        self.phase = if self.attempts == 1 {
            Phase::Failed
        } else {
            Phase::Saved
        };
        self.record(if self.phase == Phase::Failed {
            "result: simulated failure; retry is available"
        } else {
            "result: saved"
        });
    }
    fn activate(&mut self, id: usize, renderer: &str) {
        if renderer == "tokens" && (400..=401).contains(&id) {
            self.selected = id - 400;
            return;
        }
        if renderer == "toolbar" && (20..=22).contains(&id) {
            self.record(format!(
                "toolbar: {}",
                ["create", "rename", "remove"][id - 20]
            ));
            return;
        }
        if (10000..10080).contains(&id) {
            self.selected = id - 10000;
            self.detail = TextDemo::field(
                &format!(
                    "Document {} — reading notes\n\n{}",
                    self.selected + 1,
                    include_str!("../../../../fixtures/text/gallery-reading-en.txt")
                ),
                true,
                true,
            );
            self.record(format!("selection: {}", self.selected));
            return;
        }
        if extended::supports(renderer) {
            if id == 0 {
                let section = self.section;
                *self = Self::for_entry_renderer(renderer);
                self.section = section;
                self.record("reset: initial fixture");
                return;
            }
            let mut events = Vec::new();
            let changed = self
                .extended
                .activate(renderer, id, &mut |event| events.push(event));
            for event in events {
                self.record(event);
            }
            if changed && renderer == "command-palette" && id == 20 {
                self.input.set_value("");
            }
            if changed && ["combo-box", "command-palette"].contains(&renderer) {
                let (selected, items) = if renderer == "combo-box" {
                    (self.extended.combo.selected, &self.extended.combo.items)
                } else {
                    (
                        self.extended.command.combo.selected,
                        &self.extended.command.combo.items,
                    )
                };
                if let Some(index) = selected
                    && let Some(value) = items.get(index)
                {
                    self.input.set_value(value);
                }
            }
            return;
        }
        match id {
            0 => {
                let section = self.section;
                *self = Self::for_entry_renderer(renderer);
                self.section = section;
                self.record("reset: initial fixture");
            }
            1..=6 => {
                self.section = id - 1;
                self.input.blur();
            }
            10 => {
                self.disabled = !self.disabled;
                self.record(format!("disabled: {}", self.disabled));
            }
            11 => {
                self.loading = !self.loading;
                self.record(format!("loading: {}", self.loading));
            }
            12 => {
                self.error = !self.error;
            }
            14 => {
                self.input.set_value("A long workspace name with mixed scripts: 项目計画 and Japanese 日本語. Narrow containers should clip or wrap naturally.");
                self.record("fixture: long mixed text");
            }
            15 => {
                self.input.set_value("");
                self.record("fixture: empty text");
            }
            13 => {
                self.checkbox.checked = CheckState::Mixed;
            }
            20 => match renderer {
                "checkbox" => {
                    self.checkbox.toggle();
                    self.record(format!("change: {:?}", self.checkbox.checked));
                }
                "menu" => {
                    self.menu_view.show(20);
                    self.menu_layout = None;
                    self.record("menu: open");
                }
                "dialog" | "danger" => {
                    self.dialog_view.show(20);
                    self.dialog_layout = None;
                    self.record("dialog: open");
                }
                "select" => {
                    self.select.menu.show();
                    self.select_view.focus(false);
                    self.select_menu_layout = None;
                }
                "form" | "async" | "settings" => self.submit(),
                _ => self.record("activate: primary action"),
            },
            21 => self.resolve(),
            22 => {
                self.input.set_value("");
                self.record("filter: cleared");
            }
            23 => {
                self.number.increment(false);
                self.record(format!("value: {}", self.number.value()));
            }
            24 => {
                self.number.increment(true);
                self.record(format!("value: {}", self.number.value()));
            }
            25 => {
                self.checkbox.toggle();
                self.record(format!("preference: {:?}", self.checkbox.checked));
            }
            30..=32 if self.tabs_view.select(id - 30) => {
                self.record(format!("tab: {}", id - 30));
            }
            30..=32 => {}
            40..=42 => {
                self.selected = id - 40;
                self.detail = TextDemo::field(
                    [
                        "First document. A calm morning in the studio.\n\n",
                        "Second document. The afternoon light changes.\n\n",
                        "Third document. Evening brings a different rhythm.\n\n",
                    ][self.selected]
                        .to_owned()
                        .add(include_str!(
                            "../../../../fixtures/text/gallery-reading-en.txt"
                        ))
                        .as_str(),
                    true,
                    true,
                );
                self.record(format!("selection: {}", self.selected));
            }
            50..=52 => {
                self.menu_event(
                    MenuEvent::Activated {
                        index: id - 50,
                        restore_focus: Some(20),
                    },
                    renderer,
                    false,
                );
            }
            60..=62 => {
                self.menu_event(
                    MenuEvent::Activated {
                        index: id - 60,
                        restore_focus: Some(20),
                    },
                    renderer,
                    true,
                );
            }
            70 => self.dialog_event(
                DialogEvent::Cancelled {
                    restore_focus: Some(20),
                },
                renderer,
                false,
            ),
            71 => self.dialog_event(
                DialogEvent::Confirmed {
                    restore_focus: Some(20),
                },
                renderer,
                false,
            ),
            _ => {}
        }
    }
    fn activate_from_pointer(&mut self, id: usize, renderer: &str, composition_guarded: bool) {
        if composition_guarded && matches!(renderer, "form" | "settings" | "async") && id == 20 {
            self.record("submit: deferred while text composition is active");
            return;
        }
        self.activate(id, renderer);
    }
    fn for_entry_renderer(renderer: &str) -> Self {
        if let Some(e) = entries().iter().find(|e| e.renderer() == renderer) {
            Self::for_entry(e)
        } else {
            Self::default()
        }
    }
}
use std::ops::Add;
#[derive(Debug, Default)]
pub struct Workbench {
    states: HashMap<usize, StoryState>,
    brand_preview: BrandPreview,
    pub preview_width: usize,
    pub reduced: bool,
    pub brand: usize,
    pub density: usize,
    pub text_scale: usize,
    snapshot: bool,
    motion_now: Duration,
}
fn label(ctx: &mut PageCtx<'_>, scene: &mut Scene, r: Rect, s: &str, heading: bool) {
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        r,
        s,
        if heading {
            proposal::TextRole::UiHeading
        } else {
            proposal::TextRole::UiBody
        },
        ColorRole::PrimaryText.resolve(ctx.theme),
    );
}
fn line(ctx: &mut PageCtx<'_>, scene: &mut Scene, r: Rect, s: &str) {
    ctx.text.proposal_label_in(
        scene,
        ctx.theme,
        r,
        s,
        proposal::TextRole::UiCaption,
        ColorRole::SecondaryText.resolve(ctx.theme),
    );
}

#[derive(Clone, Debug)]
struct PreviewHeaderMetrics {
    info: String,
    info_height: f32,
    docs_height: f32,
}

/// Content geometry needed by a host to size a scroll viewport before drawing a workbench story.
/// `header_height` is a vertical offset from `content.min_y()`; the header draws at that same origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewHeaderLayout {
    pub preview_width: f32,
    pub header_height: f32,
}

impl PreviewHeaderMetrics {
    fn height(self) -> f32 {
        self.info_height + 12.0 + self.docs_height + 24.0
    }
}

fn preview_header_metrics(
    entry: &Entry,
    text: &mut super::text::TextEngine,
    width: f32,
) -> PreviewHeaderMetrics {
    let info = format!(
        "{} / {} · {}",
        entry.section(),
        entry.slug(),
        if entry.planned() {
            "planned"
        } else {
            "experimental"
        }
    );
    let info_height = text
        .proposal_label(&info, proposal::TextRole::UiCaption, Some(width))
        .height as f32;
    let docs_height = text
        .proposal_label(entry.get("docs"), proposal::TextRole::UiBody, Some(width))
        .height as f32;
    PreviewHeaderMetrics {
        info,
        info_height,
        docs_height,
    }
}

fn story_docs(entry: &Entry) -> &str {
    if entry.renderer() == "filter" {
        "Type to filter the local file fixture. Clear filter resets the query; an empty query shows all three items. Filtering is deterministic and does not resolve a network request."
    } else {
        entry.get("docs")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FilterRowLayout {
    input: Rect,
    clear: Rect,
    results_y: f32,
}

/// Place the search input and Clear filter button from their measured sizes.
/// The input keeps a useful minimum width beside the action; narrow previews
/// stack the action below it and move results after the complete row group.
fn filter_row_layout(ctx: &mut PageCtx<'_>, bounds: Rect, input_height: f32) -> FilterRowLayout {
    let role = proposal::TextRole::UiBody;
    let button_size =
        ctx.text
            .measure_button("Clear filter", bounds.size.width, ButtonSize::Body, role);
    let button_width = button_size.width.max(140.0).min(bounds.size.width.max(0.0));
    let button_height = button_size.height.max(36.0);
    let input_height = input_height.max(bounds.size.height).max(0.0);
    let row_gap = proposal::metric("spacing", "space-regular")
        .expect("Design System defines regular spacing");
    let results_gap = proposal::metric("spacing", "space-section")
        .expect("Design System defines section spacing");
    let min_input_width = 180.0;

    let (axis, layout_bounds, children, policy) =
        if bounds.size.width >= min_input_width + row_gap + button_width {
            let input_width = bounds.size.width - row_gap - button_width;
            let row_height = input_height.max(button_height);
            (
                Axis::Horizontal,
                Rect::new(
                    bounds.min_x(),
                    bounds.min_y(),
                    bounds.size.width,
                    row_height,
                ),
                vec![
                    MeasuredChild::new(tekes_ui_scene::Size::new(input_width, input_height)),
                    MeasuredChild::new(tekes_ui_scene::Size::new(button_width, button_height)),
                ],
                proposal::layout::ROW,
            )
        } else {
            (
                Axis::Vertical,
                Rect::new(
                    bounds.min_x(),
                    bounds.min_y(),
                    bounds.size.width,
                    input_height + row_gap + button_height,
                ),
                vec![
                    MeasuredChild::new(tekes_ui_scene::Size::new(bounds.size.width, input_height)),
                    MeasuredChild::new(tekes_ui_scene::Size::new(button_width, button_height)),
                ],
                proposal::layout::COLUMN,
            )
        };
    let children = layout_measured_children(
        layout_bounds,
        axis,
        &children,
        row_gap,
        policy,
        AlignmentOverride::default(),
    );
    let input = children[0].frame;
    let clear = children[1].frame;
    FilterRowLayout {
        input,
        clear,
        results_y: input.max_y().max(clear.max_y()) + results_gap,
    }
}

fn action(
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    id: usize,
    r: Rect,
    title: &str,
    flags: (bool, bool),
) {
    let (selected, disabled) = flags;
    let mut r = r;
    if state.density == 1 {
        r.size.height = (r.size.height - 4.).max(24.);
    }
    let b = state.buttons.entry(id).or_default();
    b.state.focused = state.window_active && state.focus == Some(id);
    b.state.selected = selected;
    b.state.disabled = disabled;
    b.state.loading = state.loading && id == 20;
    let layout = ctx.text.render_button(
        scene,
        b,
        title,
        r,
        state.button_clip,
        tekes_ui_tokens::Palette {
            theme: ctx.theme,
            brand: state.brand,
        },
        if id == 20 {
            paint::ButtonVariant::Primary
        } else {
            paint::ButtonVariant::Quiet
        },
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::default(),
    );
    if let Some(visible) = layout.visible {
        state.hits.push((id, visible));
        state.button_hits.push(id);
    }
}
fn quiet_action(
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    id: usize,
    r: Rect,
    title: &str,
    flags: (bool, bool),
) {
    let (selected, disabled) = flags;
    let mut rect = r;
    if state.density == 1 {
        rect.size.height = (rect.size.height - 4.).max(24.);
    }
    let button = state.buttons.entry(id).or_default();
    button.state.focused = state.window_active && state.focus == Some(id);
    button.state.selected = selected;
    button.state.disabled = disabled;
    let layout = ctx.text.render_button(
        scene,
        button,
        title,
        rect,
        state.button_clip,
        tekes_ui_tokens::Palette {
            theme: ctx.theme,
            brand: state.brand,
        },
        paint::ButtonVariant::Quiet,
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::default(),
    );
    if let Some(visible) = layout.visible {
        state.hits.push((id, visible));
        state.button_hits.push(id);
    }
}
fn event_log(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    x: f32,
    y: f32,
    width: f32,
    log: &VecDeque<String>,
    show_empty: bool,
) -> Rect {
    line(
        ctx,
        scene,
        Rect::new(x, y, width, 22.),
        "Event log — real interactions (Reset clears)",
    );
    if log.is_empty() {
        if show_empty {
            line(
                ctx,
                scene,
                Rect::new(x, y + 24., width, 22.),
                "No events yet.",
            );
            return Rect::new(x, y, width, 46.);
        }
        return Rect::new(x, y, width, 22.);
    }
    let visible_count = log.len().min(3);
    for (n, item) in log.iter().rev().take(3).rev().enumerate() {
        line(
            ctx,
            scene,
            Rect::new(x, y + 24. + n as f32 * 22., width, 22.),
            item,
        );
    }
    Rect::new(x, y, width, 24. + visible_count as f32 * 22.)
}
impl Workbench {
    /// Current focused control frame in window coordinates. Hosts can reveal
    /// it through the outer page viewport without expanding it a second time;
    /// `Viewport::reveal_focus` adds the Design System focus-ring outset.
    pub fn focused_control_frame(&self, i: usize) -> Option<Rect> {
        let state = self.states.get(&i)?;
        let id = state.focus?;
        if !state.hits.iter().any(|(hit_id, _)| *hit_id == id) {
            return None;
        }
        let renderer = entry(i).renderer();
        let frame = match renderer {
            "tabs" if (30..33).contains(&id) => {
                state.tabs_layout.as_ref()?.items.get(id - 30)?.frame
            }
            "menu" if (50..53).contains(&id) => {
                state.menu_layout.as_ref()?.items.get(id - 50)?.frame
            }
            "select" if id == 20 => state.select_view.layout().trigger.frame,
            "select" if (60..63).contains(&id) => {
                state.select_menu_layout.as_ref()?.items.get(id - 60)?.frame
            }
            "dialog" | "danger" if (70..72).contains(&id) => {
                state.dialog_layout.as_ref()?.buttons.get(id - 70)?.frame
            }
            "checkbox" if id == 20 => state.checkbox.button.layout()?.frame,
            "field" | "area" | "search" | "form" | "settings" | "async" | "filter"
                if id == 1000 =>
            {
                state.input.frame_at(0)?
            }
            "slider" if id == 20 => state.extended.slider_view.layout().frame,
            "switch" if id == 20 => state.extended.switch_view.layout().frame(),
            "radio-group" if (30..33).contains(&id) => state
                .extended
                .radio_view
                .layout()
                .rows
                .get(id - 30)?
                .frame(),
            "icon-button" if id == 20 => state.extended.icon_button_view.layout().frame,
            "link" if id == 20 => state.extended.link_view.layout().frame,
            "segmented-control" if (30..33).contains(&id) => {
                state
                    .extended
                    .segmented_view
                    .layout()
                    .items
                    .get(id - 30)?
                    .frame
            }
            "breadcrumb" if (20..23).contains(&id) => state
                .extended
                .breadcrumb_view
                .layout()
                .items
                .get(id - 20)?
                .frame(),
            "number-field" if id == 23 => state.number_view.layout().decrement.frame,
            "number-field" if id == 24 => state.number_view.layout().increment.frame,
            _ => state.buttons.get(&id).and_then(Button::layout)?.frame,
        };
        Some(frame)
    }

    pub fn fixture_point(route: &str, id: usize) -> Point {
        let Some(Page::Workbench(i)) = super::workbench::route(route) else {
            panic!("story route")
        };
        let mut w = Self::default();
        if id == 21 {
            w.state(i).phase = Phase::Pending;
        }
        if entry(i).renderer() == "combo-box" && (40..43).contains(&id) {
            // Option fixtures are exposed only when the real popup is open.
            w.state(i).extended.combo.open = true;
        }
        let mut text = super::text::TextEngine::new();
        text.begin_frame();
        let mut st = super::static_text::StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut st,
        };
        w.build(i, &mut ctx, super::layout::PAGE_CONTENT, &mut Scene::new());
        w.state(i)
            .visible_for_id(id)
            .expect("fixture control")
            .center()
    }
    pub fn fixture() -> Self {
        Self {
            snapshot: true,
            ..Self::default()
        }
    }
    pub fn set_window_active(&mut self, active: bool) {
        let now = self.motion_now;
        self.brand_preview.set_window_active(active);
        for s in self.states.values_mut() {
            s.window_active = active;
            s.input.set_window_active(active);
            s.detail.set_window_active(active);
            s.tabs_view.set_window_active(active);
            let menu_event = s.menu_view.set_window_active(active);
            s.menu_event(menu_event, "menu", false);
            s.select_view.set_window_active(active, &mut s.select);
            s.dialog_view.set_window_active(active);
            s.extended.set_window_active(active);
            for button in s.buttons.values_mut() {
                button.set_window_active(active, now);
            }
            if !active {
                s.toolbar_view.focus(None, false);
                s.number_view.blur();
                s.dialog_view.deactivate();
                s.extended.cancel_pointer();
                s.list_scroll.cancel();
                s.tabs_view.cancel();
                for b in s.buttons.values_mut() {
                    b.cancel();
                }
            }
        }
    }
    pub fn set_window_active_with_time(&mut self, active: bool, now: Duration) {
        self.motion_now = now;
        self.set_window_active(active);
    }
    pub fn apply_reduced_motion(&mut self, reduced: bool) {
        self.reduced = reduced;
        self.brand_preview.set_reduced_motion(reduced);
        for s in self.states.values_mut() {
            s.input.set_reduced_motion(reduced);
            s.detail.set_reduced_motion(reduced);
            s.list_scroll.set_reduced_motion(reduced);
            s.menu_view.set_reduced_motion(reduced);
            s.select_view.set_reduced_motion(reduced);
            s.extended.combo_view.options.set_reduced_motion(reduced);
            s.extended.command_view.options.set_reduced_motion(reduced);
        }
    }
    pub fn needs_animation(&self, i: usize) -> bool {
        self.states.get(&i).is_some_and(|s| {
            s.input.needs_animation()
                || s.detail.needs_animation()
                || s.list_scroll.needs_animation()
                || s.menu_view.needs_animation()
                || s.select_view.needs_animation()
                || s.extended.combo_view.options.needs_animation()
                || s.extended.command_view.options.needs_animation()
        }) || (entry(i).renderer() == "matrix" && self.brand_preview.needs_animation())
    }
    pub fn animate(&mut self, i: usize, dt: f32) -> bool {
        let brand_preview_animated =
            entry(i).renderer() == "matrix" && self.brand_preview.animate(dt);
        let Some(s) = self.states.get_mut(&i) else {
            return brand_preview_animated;
        };
        s.input.animate(dt)
            | s.detail.animate(dt)
            | s.list_scroll.tick(dt)
            | s.menu_view.tick(dt)
            | s.select_view.tick(dt)
            | s.extended.combo_view.options.tick(dt)
            | s.extended.command_view.options.tick(dt)
            | brand_preview_animated
    }
    pub fn preview_rect(&self, i: usize) -> Rect {
        self.states.get(&i).map_or(Rect::default(), |s| s.preview)
    }
    #[cfg(test)]
    pub(super) fn brand_preview_target_center_for_test(&self, id: usize) -> Option<Point> {
        self.brand_preview.target_center_for_test(id)
    }
    #[cfg(test)]
    pub(super) fn brand_preview_dialog_is_visible_for_test(&self) -> bool {
        self.brand_preview
            .dialog_layout()
            .is_some_and(|layout| layout.visible.is_some())
    }
    #[cfg(test)]
    pub(super) fn brand_preview_candidate_for_test(&self) -> Option<usize> {
        self.brand_preview.candidate_for_test()
    }
    /// Measure the real shared story header before a host clamps its scroll viewport.
    /// Uses the same TextEngine cache, locale, type scale, width policy and geometry as build.
    pub fn preview_header_layout(
        &self,
        i: usize,
        text: &mut super::text::TextEngine,
        available_width: f32,
    ) -> PreviewHeaderLayout {
        let requested = match self.preview_width {
            1 => 420.0,
            2 => 720.0,
            _ => available_width,
        };
        let focus_gutter = -ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
            .focus_bounds(1.0)
            .min_x();
        let width = (requested.min(available_width) - focus_gutter * 2.0).max(0.0);
        PreviewHeaderLayout {
            preview_width: width,
            header_height: preview_header_metrics(entry(i), text, width).height() + focus_gutter,
        }
    }
    /// Full scroll-document extent for the interactive brand matrix. The tab
    /// strip and body use the same measured geometry as `build_clipped`.
    pub fn matrix_document_extent(
        &self,
        i: usize,
        text: &mut super::text::TextEngine,
        available_width: f32,
    ) -> f32 {
        let e = entry(i);
        let requested = match self.preview_width {
            1 => 420.0,
            2 => 720.0,
            _ => available_width,
        };
        let focus_gutter = -ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
            .focus_bounds(1.0)
            .min_x();
        let width = (requested.min(available_width) - focus_gutter * 2.0).max(0.0);
        let header = preview_header_metrics(e, text, width);
        let base_tab_h =
            (proposal::TextRole::UiBody.style().line_height * text.type_scale() + 16.0).max(32.0);
        let density_inset = if self.density == 1 { 4.0 } else { 0.0 };
        let mut row_width = 0.0;
        let mut row_height = base_tab_h;
        let mut tab_bottom = header.height();
        for (n, title) in [
            "Docs",
            "Playground",
            "State matrix",
            "Behaviors",
            "Edge cases",
            "Variants / sizes",
        ]
        .iter()
        .enumerate()
        {
            if n < 6
                && !e.supports_story(
                    [
                        "docs",
                        "playground",
                        "states",
                        "behaviors",
                        "edge-cases",
                        "variants",
                    ][n],
                )
            {
                continue;
            }
            let button_width = (text
                .proposal_label(title, proposal::TextRole::UiBody, None)
                .width as f32
                + 28.0)
                .min(width);
            if row_width > 0.0 && row_width + button_width > width {
                tab_bottom += (row_height - density_inset).max(24.0) + 8.0;
                row_width = 0.0;
                row_height = base_tab_h;
            }
            let measured = text
                .measure_button(
                    title,
                    button_width,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .height;
            row_height = row_height.max((measured + density_inset).max(24.0));
            row_width += button_width + 8.0;
        }
        tab_bottom += (row_height - density_inset).max(24.0) + 24.0;
        tab_bottom
            + focus_gutter * 2.0
            + self
                .brand_preview
                .document_extent(text, width, self.density, self.brand)
    }
    pub fn show_live(&self, i: usize) -> bool {
        self.states
            .get(&i)
            .is_none_or(|s| matches!(s.section, 1 | 4))
    }
    pub fn select_route(&mut self, i: usize, route: &str) {
        let next_section = match route.rsplit('/').next().unwrap_or("") {
            "docs" => 0,
            "states" => 2,
            "behaviors" => 3,
            "edge-cases" => 4,
            "variants" => 5,
            _ => 1,
        };
        let state = self.state(i);
        if state.section != next_section {
            state.extended.blur_selection_controls();
            state.focus = None;
        }
        self.state(i).section = next_section;
    }
    fn state(&mut self, i: usize) -> &mut StoryState {
        let reduced = self.reduced;
        self.states.entry(i).or_insert_with(|| {
            let mut state = StoryState::for_entry(entry(i));
            state.input.set_reduced_motion(reduced);
            state.detail.set_reduced_motion(reduced);
            state.list_scroll.set_reduced_motion(reduced);
            state.menu_view.set_reduced_motion(reduced);
            state.select_view.set_reduced_motion(reduced);
            state
                .extended
                .combo_view
                .options
                .set_reduced_motion(reduced);
            state
                .extended
                .command_view
                .options
                .set_reduced_motion(reduced);
            state.dialog_view.set_window_active(true);
            state
        })
    }
    pub fn text_state(&self, i: usize) -> Option<tekes_ui_platform_macos::TextInputState> {
        if entry(i).renderer() == "matrix" {
            return self.brand_preview.text_state();
        }
        self.states
            .get(&i)
            .and_then(|s| s.input.state().or_else(|| s.detail.state()))
    }
    pub fn text_command(
        &mut self,
        i: usize,
        command: &str,
        clipboard: Option<&str>,
        text: &mut super::text::TextEngine,
    ) -> (bool, Option<String>) {
        if entry(i).renderer() == "matrix" {
            return self.brand_preview.text_command(command, clipboard, text);
        }
        let s = self.state(i);
        let input = if s.detail.state().is_some() {
            &mut s.detail
        } else {
            &mut s.input
        };
        let mut copied = None;
        let changed = match command {
            "copy:" => {
                copied = input.clipboard_selection();
                false
            }
            "cut:" => {
                copied = input.clipboard_selection();
                input.cut()
            }
            "paste:" => input.paste(clipboard.unwrap_or("")),
            _ => input.command(command).unwrap_or(false),
        };
        input.refresh(text);
        (changed, copied)
    }
    pub fn blur(&mut self) {
        self.brand_preview.blur();
        for s in self.states.values_mut() {
            s.pointer_began_composing = false;
            s.input.blur();
            s.detail.blur();
            s.tabs_view.cancel();
            for b in s.buttons.values_mut() {
                b.cancel();
            }
            s.focus = None;
            s.toolbar_view.focus(None, false);
            s.number_view.blur();
            let _ = s.menu_view.deactivate();
            s.select_view.cancel(&mut s.select);
            s.menu_layout = None;
            s.select_menu_layout = None;
            s.select.menu.open = false;
            if s.dialog_view.dialog().open {
                let _ = s.dialog_view.close();
            }
            s.dialog_layout = None;
        }
    }
    pub fn build(&mut self, i: usize, ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
        self.build_clipped(i, ctx, content, content, scene);
    }

    /// Build a story whose document geometry may be translated while the actual host viewport
    /// remains fixed in window coordinates. Component hit geometry and drawing share this clip.
    pub fn build_clipped(
        &mut self,
        i: usize,
        ctx: &mut PageCtx<'_>,
        content: Rect,
        viewport_clip: Rect,
        scene: &mut Scene,
    ) {
        let width = match self.preview_width {
            1 => 420.,
            2 => 720.,
            _ => content.size.width,
        };
        let preview_frame = Rect::new(
            content.min_x(),
            content.min_y(),
            width.min(content.size.width).max(0.),
            content.size.height,
        );
        let focus_gutter = -ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
            .focus_bounds(1.0)
            .min_x();
        let c = preview_frame.inset(focus_gutter);
        let brand = self.brand;
        let density = self.density;
        let snapshot = self.snapshot;
        let reduced = self.reduced;
        let e = entry(i);
        let s = self.states.entry(i).or_insert_with(|| {
            let mut state = StoryState::for_entry(e);
            state.input.set_reduced_motion(reduced);
            state.detail.set_reduced_motion(reduced);
            state.list_scroll.set_reduced_motion(reduced);
            state.menu_view.set_reduced_motion(reduced);
            state.select_view.set_reduced_motion(reduced);
            state
                .extended
                .combo_view
                .options
                .set_reduced_motion(reduced);
            state
                .extended
                .command_view
                .options
                .set_reduced_motion(reduced);
            state.dialog_view.set_window_active(true);
            state
        });
        s.brand = brand;
        s.density = density;
        let preview_clip = preview_frame
            .intersection(&viewport_clip)
            .unwrap_or(Rect::new(
                viewport_clip.min_x(),
                viewport_clip.min_y(),
                0.0,
                0.0,
            ));
        s.button_clip = Some(preview_clip);
        if snapshot {
            s.input.use_fixture_cache();
            s.detail.use_fixture_cache();
            self.brand_preview.use_fixture_cache();
        }
        s.hits.clear();
        s.button_hits.clear();
        s.phase_frame = None;
        s.event_log_frame = None;
        s.tabs_layout = None;
        s.menu_layout = None;
        s.select_menu_layout = None;
        s.dialog_layout = None;
        let x = c.min_x();
        let w = c.size.width;
        let y = c.min_y();
        let header = preview_header_metrics(e, ctx.text, w);
        ctx.text.proposal_wrapped_in(
            scene,
            ctx.theme,
            Rect::new(x, y, w, header.info_height),
            &header.info,
            proposal::TextRole::UiCaption,
            ColorRole::SecondaryText.resolve(ctx.theme),
        );
        let docs_y = y + header.info_height + 12.;
        label(
            ctx,
            scene,
            Rect::new(x, docs_y, w, header.docs_height),
            story_docs(e),
            false,
        );
        let header_bottom = y + header.height();
        s.preview = Rect::new(x, header_bottom, w, (c.max_y() - header_bottom).max(0.));
        if e.planned() {
            label(
                ctx,
                scene,
                Rect::new(x, header_bottom, w, 80.),
                "Planned\nCanonical entry reserved. No interactive implementation is claimed.",
                true,
            );
            line(
                ctx,
                scene,
                Rect::new(x, header_bottom + 100., w, 100.),
                "The imported specification and existing state illustrations are available in Quality lab → Cross-component consistency. No release or accessibility acceptance is implied.",
            );
            return;
        }
        if e.get("kind") == "component" || ["pattern", "example"].contains(&e.get("kind")) {
            let base_tab_h =
                (proposal::TextRole::UiBody.style().line_height * ctx.text.type_scale() + 16.)
                    .max(32.);
            let density_inset = if density == 1 { 4.0 } else { 0.0 };
            type TabPlacement<'a> = (usize, usize, &'a str, f32, f32, f32);
            let mut tab_rows: Vec<Vec<TabPlacement<'_>>> = vec![Vec::new()];
            let mut tab_x = x;
            for (n, title) in [
                "Docs",
                "Playground",
                "State matrix",
                "Behaviors",
                "Edge cases",
                "Variants / sizes",
                "Reset",
            ]
            .iter()
            .enumerate()
            {
                if n < 6
                    && !e.supports_story(
                        [
                            "docs",
                            "playground",
                            "states",
                            "behaviors",
                            "edge-cases",
                            "variants",
                        ][n],
                    )
                {
                    continue;
                }
                let id = if n == 6 { 0 } else { n + 1 };
                let tw = (ctx
                    .text
                    .proposal_label(title, proposal::TextRole::UiBody, None)
                    .width as f32
                    + 28.)
                    .min(w);
                if !tab_rows.last().is_none_or(Vec::is_empty) && tab_x + tw > c.max_x() {
                    tab_rows.push(Vec::new());
                    tab_x = x;
                }
                let measured_h = ctx
                    .text
                    .measure_button(title, tw, ButtonSize::Body, proposal::TextRole::UiBody)
                    .height;
                let required_frame_h = (measured_h + density_inset).max(24.0);
                tab_rows.last_mut().expect("tab row").push((
                    id,
                    n,
                    title,
                    tab_x,
                    tw,
                    required_frame_h,
                ));
                tab_x += tw + 8.;
            }
            let mut tab_y = header_bottom;
            let mut tab_bottom = header_bottom;
            for row in &tab_rows {
                let frame_h = row
                    .iter()
                    .map(|(_, _, _, _, _, height)| *height)
                    .fold(base_tab_h, f32::max);
                for (id, index, title, x, width, _) in row {
                    action(
                        s,
                        ctx,
                        scene,
                        *id,
                        Rect::new(*x, tab_y, *width, frame_h),
                        title,
                        (s.section == *index, false),
                    );
                }
                tab_bottom = tab_y + (frame_h - density_inset).max(24.0);
                tab_y = tab_bottom + 8.0;
            }
            let body_y = tab_bottom + 24.;
            let body = Rect::new(x, body_y, w, (c.max_y() - body_y).max(400.));
            s.preview = Rect::new(x, body_y, w, (c.max_y() - body_y).max(0.));
            if extended::supports(e.renderer()) {
                let preview = s.preview;
                if s.section == 0 {
                    Self::docs(e, s, ctx, scene, preview);
                    return;
                }
                if s.section == 2 {
                    let mut ext = std::mem::take(&mut s.extended);
                    ext.matrix(e, s, ctx, scene, preview);
                    s.extended = ext;
                    return;
                }
                if s.section == 3 {
                    let mut ext = std::mem::take(&mut s.extended);
                    ext.behaviors(e, ctx, scene, preview);
                    s.extended = ext;
                    return;
                }
                if s.section == 5 {
                    let mut ext = std::mem::take(&mut s.extended);
                    ext.variants(e, s, ctx, scene, preview);
                    s.extended = ext;
                    return;
                }
                let mut ext = std::mem::take(&mut s.extended);
                ext.draw(e, s, ctx, scene, s.preview);
                s.extended = ext;
                return;
            }
            if s.section == 0 {
                Self::docs(e, s, ctx, scene, body);
                return;
            }
            if s.section == 2 {
                Self::matrix(e, s, ctx, scene, body);
                return;
            }
            if s.section == 5 {
                Self::variants(e, s, ctx, scene, body);
                return;
            }
            if s.section == 3 {
                label(
                    ctx,
                    scene,
                    Rect::new(x, body_y, w, 160.),
                    match e.renderer() {
                        "tabs" => {
                            "Tab enters the tab strip. Left/Right wrap selection. Home/End select endpoints. The panel follows the active tab."
                        }
                        "menu" | "select" => {
                            "Enter or Space opens. Up/Down move through options. Enter selects. Escape dismisses and restores focus."
                        }
                        "dialog" | "danger" => {
                            "Enter opens. Tab/Shift+Tab stay inside the modal scope. Escape cancels. Closing restores focus to the invoker."
                        }
                        "field" | "area" | "search" => {
                            "Click or Tab to focus. Native text services handle IME composition, selection, clipboard, undo and redo. Tab leaves single-line input."
                        }
                        _ => {
                            "Tab / Shift+Tab move focus. Space activates on release. Enter activates the focused action. Pointer release outside cancels. Disabled controls do not activate."
                        }
                    },
                    false,
                );
                line(
                    ctx,
                    scene,
                    Rect::new(x, body_y + 180., w, 100.),
                    "The matrix simulates states for comparison only. Use Playground to exercise real focus and input. VoiceOver and physical IME checks are still manual.",
                );
                return;
            }
            if ["scroll", "text", "icon"].contains(&e.renderer()) {
                return;
            }
            Self::playground(e, s, ctx, scene, body);
        } else {
            if e.renderer() == "matrix" {
                self.brand_preview
                    .build(ctx, scene, s.preview, s.button_clip, self.brand, density);
                return;
            }
            Self::reference(e, s, ctx, scene, c);
        }
    }
    fn docs(e: &Entry, _s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, c: Rect) {
        if extended::supports(e.renderer()) {
            scene.push_clip(c);
            let routes = [
                "docs",
                "playground",
                "states",
                "behaviors",
                "edge-cases",
                "variants",
            ]
            .into_iter()
            .filter(|story| e.supports_story(story))
            .map(|story| format!("{}/{story}", e.slug()))
            .collect::<Vec<_>>()
            .join(" · ");
            let mut lines = vec![
                format!("{} · {}", e.component(), e.get("status")),
                story_docs(e).to_owned(),
                format!("API: {}", e.get("api")),
                format!("Stories: {routes}"),
                "Reset returns this deterministic fixture to its initial state. The event log reports local model transitions; no real network or user data is used.".to_owned(),
            ];
            if e.renderer() == "combo-box" || e.renderer() == "command-palette" {
                lines.push("IME composition is routed through the native text input before list navigation. Synthetic event fixtures do not replace physical input-method testing.".into());
            }
            let mut y = c.min_y();
            for text in &lines {
                let height = ctx
                    .text
                    .proposal_label(text, proposal::TextRole::UiBody, Some(c.size.width))
                    .height as f32;
                label(
                    ctx,
                    scene,
                    Rect::new(c.min_x(), y, c.size.width, height),
                    text,
                    false,
                );
                y += height + 16.;
            }
            scene.pop_clip();
            return;
        }
        let lines=[format!("Component: {}  |  maturity: experimental",e.component()),format!("Story routes: {}", ["docs", "playground", "states", "behaviors", "edge-cases", "variants"].into_iter().filter(|story| e.supports_story(story)).map(|story| format!("{}/{story}", e.slug())).collect::<Vec<_>>().join(" · ")),"State is local to this route. Reset restores deterministic initial data. No clock, random source or real network is used.".into(),"Tokens: surface-1/2, primary-text, secondary-text, border, border-focused, row-selection, button-hover/pressed.".into(),format!("Usage: tekes_ui_controls::{} — see widgets.rs / text_input.rs / scroll_view.rs",match e.renderer(){"field"|"area"|"search"=>"TextInput","scroll"=>"ScrollArea",_=>e.title()}),"State comparison is a visual aid, not a keyboard or accessibility test. Native input services are exercised through the real window.".into()];
        let mut y = c.min_y();
        for text in &lines {
            let height = ctx
                .text
                .proposal_label(text, proposal::TextRole::UiBody, Some(c.size.width))
                .height as f32;
            label(
                ctx,
                scene,
                Rect::new(c.min_x(), y, c.size.width, height),
                text,
                false,
            );
            y += height + 16.;
        }
    }
    fn matrix(e: &Entry, s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, c: Rect) {
        if ["scroll", "text", "icon"].contains(&e.renderer()) {
            label(
                ctx,
                scene,
                c,
                "Use Playground to inspect the real viewport and selection. ScrollArea supports independent Enabled / Disabled / Auto axes. Text exposes read-only selection; it has no pressed or loading state.",
                false,
            );
            return;
        }
        line(
            ctx,
            scene,
            Rect::new(c.min_x(), c.min_y(), c.size.width, 34.),
            "Visual fixtures only — simulated focus is not a focus test. Real interaction: Playground.",
        );
        let states = [
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
                "selected + hover",
                ControlState {
                    selected: true,
                    hovered: true,
                    ..Default::default()
                },
            ),
            (
                "checked + disabled",
                ControlState {
                    selected: true,
                    disabled: true,
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
        ];
        let columns = if c.size.width < 600. { 2 } else { 3 };
        let cw = c.size.width / columns as f32;
        let states: Vec<_> = states
            .into_iter()
            .filter(|(name, _)| match *name {
                "selected + hover" => ["tabs", "menu", "select"].contains(&e.renderer()),
                "checked + disabled" => e.renderer() == "checkbox",
                "error + focus" => {
                    ["field", "area", "search", "field-meta"].contains(&e.renderer())
                }
                "loading" => e.renderer() == "button",
                _ => true,
            })
            .collect();
        for (n, (name, state)) in states.into_iter().enumerate() {
            let r = Rect::new(
                c.min_x() + (n % columns) as f32 * cw,
                c.min_y() + 42. + (n / columns) as f32 * 92.,
                cw - 16.,
                36.,
            );
            if ["field", "area", "search", "field-meta"].contains(&e.renderer()) {
                let mut input = TextDemo::field(
                    if name == "default" {
                        "Studio"
                    } else {
                        "Sample text"
                    },
                    e.renderer() == "area",
                    state.disabled,
                );
                input.use_fixture_cache();
                if state.focused {
                    input.focus_field();
                }
                input.set_error(state.error);
                input.build_search_clipped(ctx, r, Some(c), scene);
            } else if e.renderer() == "checkbox" {
                let mut button = Button::default();
                button.state = state;
                let mut checkbox = Checkbox {
                    button,
                    checked: if name == "checked + disabled" {
                        CheckState::Checked
                    } else if name == "selected + hover" {
                        CheckState::Mixed
                    } else {
                        CheckState::Unchecked
                    },
                };
                checkbox.render(
                    scene,
                    ctx.text,
                    "",
                    Rect::new(r.min_x(), r.min_y(), 24., 24.),
                    Some(c),
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                );
            } else {
                let mut fixture = Button::default();
                fixture.state = state;
                ctx.text.render_button(
                    scene,
                    &mut fixture,
                    if state.loading {
                        "Loading…"
                    } else {
                        "Sample"
                    },
                    r,
                    Some(c),
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    paint::ButtonVariant::Quiet,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::default(),
                );
            }
            line(ctx, scene, r.offset(0., 40.), name);
        }
        s.hits.retain(|(id, _)| *id <= 5);
        s.button_hits.retain(|id| *id <= 5);
    }
    fn variants(e: &Entry, s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, c: Rect) {
        if e.renderer() == "button" {
            for (n, (title, height)) in [
                ("Small", proposal::metric("size", "size-button-sm").unwrap()),
                (
                    "Medium",
                    proposal::metric("size", "size-button-md").unwrap(),
                ),
                ("Large", proposal::metric("size", "size-button-lg").unwrap()),
            ]
            .into_iter()
            .enumerate()
            {
                let r = Rect::new(
                    c.min_x(),
                    c.min_y() + n as f32 * 78.,
                    c.size.width.min(200.),
                    height,
                );
                let mut button = Button::default();
                ctx.text.render_button(
                    scene,
                    &mut button,
                    title,
                    r,
                    Some(c),
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    paint::ButtonVariant::Primary,
                    [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg][n],
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::default(),
                );
                let quiet = r.offset(c.size.width.min(240.), 0.);
                let mut quiet_button = Button::default();
                ctx.text.render_button(
                    scene,
                    &mut quiet_button,
                    "Quiet",
                    quiet,
                    Some(c),
                    ctx.theme,
                    paint::ButtonVariant::Quiet,
                    [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg][n],
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::default(),
                );
            }
        } else {
            label(
                ctx,
                scene,
                c,
                match e.renderer() {
                    "area" => {
                        "Multiline plain-text variant; fills the remaining preview. TextEditor remains an internal adapter."
                    }
                    "field" => {
                        "Single-line TextInput. Native IME, selection, clipboard and undo use the shared production model."
                    }
                    "surface" => {
                        "Surface / Panel share the same semantic container; Panel adds the border."
                    }
                    "number-field" => {
                        "Bounded numeric stepper: 1…32, step 1. Editable numeric input is not implemented."
                    }
                    _ => {
                        "One supported variant. Use the responsive preview widths to inspect constrained containers."
                    }
                },
                false,
            );
        }
    }
    fn playground(
        e: &Entry,
        s: &mut StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        let x = c.min_x();
        let y = c.min_y();
        let w = c.size.width;
        let mut property_x = x;
        for (n, title) in ["Disabled", "Loading", "Error", "Mixed"].iter().enumerate() {
            if n == 3 && e.renderer() != "checkbox" {
                continue;
            }
            if n == 1 && e.renderer() != "button" {
                continue;
            }
            if n == 2
                && !["field", "area", "search", "field-meta", "form", "settings"]
                    .contains(&e.renderer())
            {
                continue;
            }
            if n == 0
                && ![
                    "button", "checkbox", "field", "area", "search", "menu", "select", "dialog",
                    "danger",
                ]
                .contains(&e.renderer())
            {
                continue;
            }
            let selected = match n {
                0 => s.disabled,
                1 => s.loading,
                2 => s.error,
                _ => s.checkbox.checked == CheckState::Mixed,
            };
            action(
                s,
                ctx,
                scene,
                10 + n,
                Rect::new(property_x, y, 112., 36.),
                title,
                (selected, false),
            );
            property_x += 120.;
        }
        let r = Rect::new(x, y + 54., w.min(600.), 36.);
        let body_y = y + 54.;
        let edge = s.section == 4;
        let mut content_bottom = None;
        if edge && ["field", "area", "search", "form", "settings"].contains(&e.renderer()) {
            action(
                s,
                ctx,
                scene,
                14,
                Rect::new(x, y, 156., 30.),
                "Long mixed text",
                (false, false),
            );
            action(
                s,
                ctx,
                scene,
                15,
                Rect::new(x + 164., y, 120., 30.),
                "Empty text",
                (false, false),
            );
        }
        match e.renderer() {
            "field" | "area" | "search" => {
                let field_min_height = if e.renderer() == "area" {
                    0.0
                } else {
                    s.input.measure_search_height(ctx.text)
                };
                line(ctx, scene, r, "Project name — native text input");
                let input_y = body_y + 32.;
                let input_height = if e.renderer() == "area" {
                    (c.size.height - 200.)
                        .clamp(100., 270.)
                        .min((s.preview.max_y() - input_y).max(0.))
                } else {
                    (proposal::TextRole::UiBody.style().line_height * ctx.text.type_scale() + 16.)
                        .max(36.)
                        .max(field_min_height)
                };
                let input = Rect::new(x, input_y, w.min(650.), input_height);
                s.input.set_disabled(s.disabled);
                s.input.set_error(s.error || s.phase == Phase::Invalid);
                s.input
                    .build_search_clipped(ctx, input, s.button_clip, scene);
                if let Some(visible) = s.input.visible_frame() {
                    s.hits.push((1000, visible));
                }
                if e.renderer() == "search" {
                    action(
                        s,
                        ctx,
                        scene,
                        22,
                        Rect::new(x, input.max_y() + 12., 140., 34.),
                        "Clear query",
                        (false, false),
                    );
                }
                if s.error {
                    line(
                        ctx,
                        scene,
                        input.offset(0., input.size.height + 8.),
                        "Validation: name is required (fixture)",
                    );
                }
            }
            "button" => {
                action(
                    s,
                    ctx,
                    scene,
                    20,
                    Rect::new(x, body_y, w.min(if edge { 280. } else { 180. }), 36.),
                    if s.loading {
                        "Loading…"
                    } else if edge {
                        "A long action label 中日文"
                    } else {
                        "Run action"
                    },
                    (false, s.disabled || s.loading),
                );
            }
            "checkbox" => {
                let mut button = s.buttons.get(&20).cloned().unwrap_or_default();
                button.state.disabled = s.disabled;
                button.state.focused = s.window_active && s.focus == Some(20);
                s.checkbox.button = button;
                let row = Rect::new(x, body_y, w.min(400.), 36.);
                let layout = s.checkbox.render(
                    scene,
                    ctx.text,
                    "Receive project notifications",
                    row,
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                );
                s.buttons.insert(20, s.checkbox.button.clone());
                if let Some(visible) = layout.visible {
                    s.hits.push((20, visible));
                    s.button_hits.push(20);
                }
            }
            "tabs" => {
                s.tabs_view.set_window_active(s.window_active);
                s.tabs_view.set_disabled(s.disabled);
                let layout = s.tabs_view.render(
                    scene,
                    ctx.text,
                    &["General", "Security", "Advanced"],
                    Rect::new(x, body_y, w, 34.),
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                    s.density == 1,
                );
                if let Some(visible) = layout
                    .items
                    .get(s.tabs_view.selected())
                    .and_then(|item| item.visible)
                {
                    s.hits.push((30 + s.tabs_view.selected(), visible));
                }
                let panel_y = layout.frame.max_y() + 20.0;
                s.tabs_layout = Some(layout);
                label(
                    ctx,
                    scene,
                    Rect::new(x, panel_y, w, 80.),
                    [
                        "General settings panel",
                        "Security settings panel",
                        "Advanced settings panel",
                    ][s.tabs_view.selected()],
                    false,
                );
            }
            "select" => {
                s.select_view.set_disabled(s.disabled, &mut s.select);
                let layout = s.select_view.render(
                    scene,
                    ctx.text,
                    &mut s.select,
                    &["Automatic", "Compact", "Comfortable"],
                    Rect::new(x, body_y, w.min(230.), 36.),
                    s.preview,
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                    s.density == 1,
                );
                if let Some(visible) = layout.trigger.visible {
                    s.hits.push((20, visible));
                }
                s.buttons.entry(20).or_default().state.disabled = s.disabled;
                s.select_menu_layout = layout.popup;
            }
            "menu" | "dialog" | "danger" => {
                action(
                    s,
                    ctx,
                    scene,
                    20,
                    Rect::new(x, body_y, w.min(230.), 36.),
                    match e.renderer() {
                        "menu" => "Open action menu",
                        "danger" => "Delete workspace…",
                        _ => "Open dialog",
                    },
                    (false, s.disabled),
                );
                if e.renderer() == "danger" {
                    Self::phase(s, ctx, scene, Rect::new(x, body_y + 58., w, 100.));
                }
            }
            "form" | "settings" | "async" | "field-meta" => {
                s.field.label = "Workspace name".into();
                s.field.help = "Use a clear name for your workspace.".into();
                s.field.required = false;
                s.field.error =
                    (s.phase == Phase::Invalid || s.error).then(|| "A name is required.".into());
                let field_layout = s.field_view.render_with_presentation(
                    scene,
                    ctx.text,
                    &s.field,
                    Rect::new(x, body_y, w.min(600.), 118.),
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                    FieldPresentation::emphasized_form(),
                );
                let input = field_layout.slot;
                s.input.set_disabled(s.disabled);
                s.input.set_error(s.error || s.phase == Phase::Invalid);
                s.input
                    .build_search_clipped(ctx, input, s.button_clip, scene);
                if let Some(visible) = s.input.visible_frame() {
                    s.hits.push((1000, visible));
                }
                let action_y = field_layout.message.max_y() + 2.;
                action(
                    s,
                    ctx,
                    scene,
                    25,
                    Rect::new(x, action_y, w.min(340.), 34.),
                    if s.checkbox.checked == CheckState::Checked {
                        "✓ Notifications enabled"
                    } else {
                        "Enable notifications"
                    },
                    (s.checkbox.checked == CheckState::Checked, false),
                );
                action(
                    s,
                    ctx,
                    scene,
                    20,
                    Rect::new(x, action_y + 48., 150., 36.),
                    if s.phase == Phase::Failed {
                        "Retry save"
                    } else {
                        "Save settings"
                    },
                    (false, s.phase == Phase::Pending),
                );
                let phase_frame = Self::phase(s, ctx, scene, Rect::new(x, action_y + 96., w, 80.));
                s.phase_frame = Some(phase_frame);
                content_bottom = Some(phase_frame.max_y());
            }
            "filter" => {
                let input_height = s.input.measure_search_height(ctx.text);
                let row = filter_row_layout(ctx, r, input_height);
                s.input.set_disabled(s.disabled);
                s.input.set_error(s.error || s.phase == Phase::Invalid);
                s.input
                    .build_search_clipped(ctx, row.input, s.button_clip, scene);
                if let Some(visible) = s.input.visible_frame() {
                    s.hits.push((1000, visible));
                }
                action(s, ctx, scene, 22, row.clear, "Clear filter", (false, false));
                let q = s.input.query().to_lowercase();
                let results: Vec<_> = ["Studio notes.md", "Research notes.txt", "项目计划.md"]
                    .into_iter()
                    .filter(|t| t.to_lowercase().contains(&q))
                    .collect();
                if results.is_empty() {
                    label(
                        ctx,
                        scene,
                        Rect::new(x, row.results_y, w, 40.0),
                        "No results. Clear the filter to see all files.",
                        false,
                    );
                } else {
                    for (n, t) in results.iter().enumerate() {
                        label(
                            ctx,
                            scene,
                            Rect::new(x, row.results_y + n as f32 * 40.0, w, 40.0),
                            t,
                            false,
                        );
                    }
                }
            }
            "master" => {
                let lw = (w * 0.32).min(240.);
                let h = (c.size.height - 190.)
                    .max(100.)
                    .min((s.preview.max_y() - body_y).max(0.));
                let list = Rect::new(x, body_y, lw - 12., h);
                s.list_scroll.set_scroll_policy(
                    tekes_ui_controls::ScrollPolicy::Disabled,
                    tekes_ui_controls::ScrollPolicy::Auto,
                );
                let focus_padding = -ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
                    .focus_bounds(1.0)
                    .min_x();
                s.list_scroll.layout(
                    list.size,
                    tekes_ui_scene::Size::new(list.size.width, 80. * 42. + focus_padding * 2.0),
                );
                if let Some(index) = s
                    .focus_reveal
                    .take()
                    .filter(|id| (10000..10100).contains(id))
                    .map(|id| id - 10000)
                {
                    let row = Rect::new(
                        focus_padding,
                        focus_padding + index as f32 * 42.0,
                        (lw - 20.0 - focus_padding * 2.0).max(0.0),
                        36.0,
                    );
                    s.list_scroll.reveal_focus(row, ctx.text.backing_scale());
                }
                let geometry = s.list_scroll.geometry(list, s.button_clip);
                let visible_list = geometry.visible_viewport_rect();
                s.list_bounds =
                    visible_list.unwrap_or(Rect::new(list.min_x(), list.min_y(), 0.0, 0.0));
                let previous_button_clip = s.button_clip;
                s.button_clip = visible_list;
                if let Some(visible_list) = visible_list {
                    scene.push_clip(visible_list);
                }
                let origin = geometry.content_origin();
                for n in 0..80 {
                    let document_row = Rect::new(
                        focus_padding,
                        focus_padding + n as f32 * 42.0,
                        (lw - 20.0 - focus_padding * 2.0).max(0.0),
                        36.0,
                    );
                    if geometry.document_to_viewport(document_row).is_some() {
                        let row = document_row.offset(origin.x, origin.y);
                        action(
                            s,
                            ctx,
                            scene,
                            10000 + n,
                            row,
                            &format!("Document {:02}", n + 1),
                            (s.selected == n, false),
                        );
                    }
                }
                s.button_clip = previous_button_clip;
                if visible_list.is_some() {
                    scene.pop_clip();
                }
                if s.list_scroll.scrollbar_visible(true)
                    && let Some(visible_list) = visible_list
                {
                    let thumb = s.list_scroll.thumb(
                        true,
                        Rect::new(list.max_x() - 4., list.min_y(), 4., list.size.height),
                        24.,
                    );
                    scene.push_clip(visible_list);
                    scene.push_quad(
                        Quad::new(thumb, paint::color(ctx.theme, "secondary-text"))
                            .with_corner_radius(2.),
                    );
                    scene.pop_clip();
                }
                let detail = Rect::new(x + lw, body_y, (w - lw).max(1.), h);
                s.detail
                    .build_search_clipped(ctx, detail, s.button_clip, scene);
            }
            "surface" => {
                let backing_scale = ctx.text.scale();
                let layout = s.surface_view.render_at_scale(
                    scene,
                    Rect::new(x, body_y, w.min(660.), 150.),
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    true,
                    backing_scale,
                );
                label(
                    ctx,
                    scene,
                    Rect::new(
                        layout.content.min_x(),
                        layout.content.min_y(),
                        layout.content.size.width,
                        36.,
                    ),
                    "Panel — semantic surface and border",
                    true,
                );
                action(
                    s,
                    ctx,
                    scene,
                    20,
                    Rect::new(
                        layout.content.min_x(),
                        layout.content.min_y() + 60.,
                        180.,
                        36.,
                    ),
                    "Nested real action",
                    (false, false),
                );
            }
            "toolbar" => {
                let frame = Rect::new(x, body_y, w.min(600.), 36.);
                let backing_scale = ctx.text.scale();
                let surface = s.surface_view.render_at_scale(
                    scene,
                    frame,
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    true,
                    backing_scale,
                );
                s.toolbar_view.set_disabled(s.disabled);
                s.toolbar_view.focus(
                    s.focus.filter(|id| (20..23).contains(id)).map(|id| id - 20),
                    s.focus.is_some_and(|id| (20..23).contains(&id)),
                );
                let layouts = s.toolbar_view.render(
                    scene,
                    ctx.text,
                    &["Create", "Rename", "Remove"],
                    surface.frame,
                    surface.visible,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                );
                for (index, layout) in layouts.into_iter().enumerate() {
                    s.buttons.entry(20 + index).or_default().state.disabled = s.disabled;
                    if let Some(visible) = layout.visible {
                        s.hits.push((20 + index, visible));
                    }
                }
            }
            "number-field" => {
                s.number_view.set_disabled(s.disabled);
                let layout = s.number_view.render(
                    scene,
                    ctx.text,
                    &s.number,
                    Rect::new(x, body_y, w.min(198.), 36.),
                    s.button_clip,
                    tekes_ui_tokens::Palette {
                        theme: ctx.theme,
                        brand: s.brand,
                    },
                    proposal::TextRole::UiBody,
                );
                s.buttons.entry(23).or_default().state.disabled =
                    s.disabled || !s.number.can_increment(false);
                s.buttons.entry(24).or_default().state.disabled =
                    s.disabled || !s.number.can_increment(true);
                if let Some(visible) = layout.decrement.visible {
                    s.hits.push((23, visible));
                }
                if let Some(visible) = layout.increment.visible {
                    s.hits.push((24, visible));
                }
                extended::wrapped_caption(
                    ctx,
                    scene,
                    x,
                    r.min_y() + 56.,
                    w,
                    "Bounded stepper: 1…32. Editable numeric text is not implemented.",
                );
            }
            "skeleton" => {
                for n in 0..4 {
                    let layout = s.skeleton_view.render(
                        scene,
                        Rect::new(
                            x,
                            body_y + n as f32 * 30.,
                            w.min(500.) - (n % 2) as f32 * 80.,
                            18.,
                        ),
                        s.button_clip,
                        tekes_ui_tokens::Palette {
                            theme: ctx.theme,
                            brand: s.brand,
                        },
                    );
                    let _ = layout;
                }
            }
            _ => {}
        }
        let logy = (c.max_y() - 116.)
            .max(body_y + 100.)
            .max(content_bottom.map_or(f32::NEG_INFINITY, |bottom| bottom + 24.));
        s.event_log_frame = Some(event_log(ctx, scene, x, logy, w, &s.log, false));
        let palette = tekes_ui_tokens::Palette {
            theme: ctx.theme,
            brand: s.brand,
        };
        let trigger = s
            .buttons
            .get(&20)
            .and_then(Button::layout)
            .map(|layout| layout.frame);
        if s.menu_view.is_open()
            && let Some(trigger) = trigger
        {
            s.menu_view.set_disabled(s.disabled);
            s.menu_layout = s.menu_view.render(
                scene,
                ctx.text,
                &["Open document", "Duplicate document", "Archive document"],
                trigger,
                s.preview,
                s.button_clip,
                palette,
                proposal::TextRole::UiBody,
                s.density == 1,
            );
        }
        if s.dialog_view.dialog().open {
            s.dialog_view.set_disabled(s.disabled);
            s.dialog_layout = s.dialog_view.render(
                scene,
                ctx.text,
                if e.renderer() == "danger" {
                    "Delete this workspace? This fixture does not delete files."
                } else {
                    "Confirm changes?"
                },
                s.preview,
                s.button_clip,
                palette,
                proposal::TextRole::UiBody,
                s.density == 1,
            );
            if let Some(layout) = &s.dialog_layout {
                s.focus = Some(70 + layout.focus);
            }
        }
    }
    fn phase(s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, r: Rect) -> Rect {
        let text = match s.phase {
            Phase::Ready => "Ready. First submission fails; retry succeeds.",
            Phase::Invalid => "Validation error: enter a workspace name.",
            Phase::Pending => "Submitting… use Resolve to complete the controlled fixture.",
            Phase::Failed => "Save failed. Your input is preserved. Retry is available.",
            Phase::Saved => "Saved successfully. Preferences remain in this story only.",
        };
        let measured_height = ctx
            .text
            .proposal_label(text, proposal::TextRole::UiBody, Some(r.size.width.max(0.)))
            .height as f32;
        let mut occupied = Rect::new(r.min_x(), r.min_y(), r.size.width, measured_height);
        label(ctx, scene, r, text, false);
        if s.phase == Phase::Pending {
            action(
                s,
                ctx,
                scene,
                21,
                Rect::new(r.min_x(), r.min_y() + 56., 180., 34.),
                "Resolve fixture",
                (false, false),
            );
            if let Some(button) = s.buttons.get(&21).and_then(Button::layout) {
                let left = occupied.min_x().min(button.frame.min_x());
                let top = occupied.min_y().min(button.frame.min_y());
                let right = occupied.max_x().max(button.frame.max_x());
                let bottom = occupied.max_y().max(button.frame.max_y());
                occupied = Rect::new(left, top, right - left, bottom - top);
            }
        }
        occupied
    }
    fn reference(
        e: &Entry,
        s: &mut StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        _c: Rect,
    ) {
        let r = s.preview;
        match e.renderer() {
            "docs" => {
                let copy = if e.slug() == "getting-started" {
                    "Open a component from the sidebar or search by name, ID or story keyword.\n\nDirect route: TEKESUI_GALLERY_PAGE=component-button/playground\n\nUse Preview for semantic brand presets, density, text scale and constrained width. Appearance keeps the existing Light / Dark and Increase Contrast choices.\n\nReset restores this story. The event log records real input and fixture transitions."
                } else {
                    "Foundations explain the design rules. Components exercise reusable Rust controls. Patterns combine them, Examples present complete workflows, and Quality lab keeps calibration and acceptance evidence.\n\nStart with Component index, or search for Button, TextField, ScrollArea or Settings. A planned label reserves a missing capability; experimental means interactive but not fully accepted for production.\n\nStories use deterministic local data. Async fixtures resolve explicitly, so you can repeat failure and recovery without a network or user files."
                };
                label(ctx, scene, r, copy, false);
            }
            "index" => {
                let components: Vec<_> = entries()
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| !e.component().is_empty())
                    .collect();
                let columns = if r.size.width < 600. { 1 } else { 3 };
                let cw = r.size.width / columns as f32;
                for (n, (index, e)) in components.iter().enumerate() {
                    action(
                        s,
                        ctx,
                        scene,
                        30000 + index,
                        Rect::new(
                            r.min_x() + (n % columns) as f32 * cw,
                            r.min_y() + (n / columns) as f32 * 40.,
                            cw - 8.,
                            34.,
                        ),
                        &format!(
                            "{}{}",
                            e.title(),
                            if e.planned() { " · planned" } else { "" }
                        ),
                        (false, false),
                    );
                }
            }
            "tokens" => {
                for (n, title) in ["Semantic colors", "Complete token atlas"]
                    .iter()
                    .enumerate()
                {
                    action(
                        s,
                        ctx,
                        scene,
                        400 + n,
                        Rect::new(r.min_x() + n as f32 * 240., r.min_y(), 228., 34.),
                        title,
                        (s.selected == n, false),
                    );
                }
                let r = Rect::new(r.min_x(), r.min_y() + 50., r.size.width, r.size.height);
                if s.selected == 1 {
                    super::pages::tokens::build(ctx, r, scene);
                    return;
                }
                let palette = tekes_ui_tokens::Palette {
                    theme: ctx.theme,
                    brand: s.brand,
                };
                let roles = [
                    "surface-0",
                    "surface-1",
                    "surface-2",
                    "surface-popover",
                    "primary-text",
                    "secondary-text",
                    "border",
                    "border-focused",
                    "row-selection",
                    "row-selection-hover",
                    "action-primary-ground",
                    "action-primary-hover",
                    "action-primary-pressed",
                    "action-primary-foreground",
                    "control-checked-ground",
                    "control-checked-mark",
                    "text-selection",
                    "status-failed",
                    "status-succeeded",
                    "warning",
                ];
                let columns = if r.size.width < 600. { 2 } else { 4 };
                let cw = r.size.width / columns as f32;
                for (n, role) in roles.iter().enumerate() {
                    let cell = Rect::new(
                        r.min_x() + (n % columns) as f32 * cw,
                        r.min_y() + (n / columns) as f32 * 92.,
                        cw - 12.,
                        48.,
                    );
                    scene.push_quad(
                        Quad::new(cell, palette.color(role).unwrap())
                            .with_corner_radius(6.)
                            .with_border(1., paint::color(ctx.theme, "border")),
                    );
                    line(ctx, scene, cell.offset(0., 52.), role);
                }
                let top = r.min_y() + roles.len().div_ceil(columns) as f32 * 92. + 24.;
                label(
                    ctx,
                    scene,
                    Rect::new(r.min_x(), top, r.size.width, 32.),
                    "Opacity & compositing",
                    true,
                );
                super::pages::primitives::alpha_samples(
                    scene,
                    ctx.theme,
                    Point::new(r.min_x(), top + 48.),
                );
                for (n, alpha) in [
                    tekes_ui_tokens::Alpha::Faint,
                    tekes_ui_tokens::Alpha::Soft,
                    tekes_ui_tokens::Alpha::Wash,
                ]
                .into_iter()
                .enumerate()
                {
                    line(
                        ctx,
                        scene,
                        Rect::new(r.min_x() + n as f32 * 132., top + 112., 128., 32.),
                        &format!("{alpha:?}: {}", alpha.resolve(ctx.theme.contrast)),
                    );
                }
                label(
                    ctx,
                    scene,
                    Rect::new(r.min_x(), top + 160., r.size.width, 80.),
                    "Alpha is applied once during compositing. Overlap shows the resulting blend. These washes are decorative; preserve semantic text, control boundaries and focus contrast.",
                    false,
                );
            }
            "shape" => {
                label(
                    ctx,
                    scene,
                    Rect::new(r.min_x(), r.min_y() + 320., r.size.width, 32.),
                    "Stroke widths & boundary roles",
                    true,
                );
                super::pages::primitives::stroke_samples(
                    scene,
                    ctx.theme,
                    Point::new(r.min_x(), r.min_y() + 364.),
                );
                for (n, stroke) in [
                    tekes_ui_tokens::Stroke::Hairline,
                    tekes_ui_tokens::Stroke::Rule,
                    tekes_ui_tokens::Stroke::Emphasis,
                ]
                .into_iter()
                .enumerate()
                {
                    line(
                        ctx,
                        scene,
                        Rect::new(r.min_x() + n as f32 * 108., r.min_y() + 428., 104., 32.),
                        &format!("{stroke:?}: {} pt", stroke.value()),
                    );
                }
                label(
                    ctx,
                    scene,
                    Rect::new(r.min_x(), r.min_y() + 476., r.size.width, 72.),
                    "Required control boundary: border / stroke-rule (at least 3:1). Decorative frame: separator / 1 physical pixel. Focus: border-focused. Hairline 0.5 pt is the 2x specimen; Increase Contrast uses stroke-rule for decorative frames.",
                    false,
                );
                for (n, radius) in tekes_ui_tokens::CornerRadius::ALL.into_iter().enumerate() {
                    let q = Rect::new(
                        r.min_x() + (n % 4) as f32 * 130.,
                        r.min_y() + (n / 4) as f32 * 100.,
                        100.,
                        60.,
                    );
                    scene.push_quad(
                        Quad::new(q, paint::color(ctx.theme, "surface-2"))
                            .with_corner_radius(radius.value())
                            .with_border(1., paint::color(ctx.theme, "border")),
                    );
                    line(
                        ctx,
                        scene,
                        q.offset(0., 64.),
                        &format!("{radius:?}: {}", radius.value()),
                    );
                }
                label(
                    ctx,
                    scene,
                    Rect::new(r.min_x(), r.min_y() + 560., r.size.width, 80.),
                    "Elevation uses semantic surfaces and layer order. Shadows are not implemented in the current scene primitive; no shadow acceptance is claimed.",
                    false,
                );
            }
            "calibration" => {
                super::pages::static_text::build_clipped(ctx, r, s.button_clip.unwrap_or(r), scene)
            }
            "geometry" => super::pages::primitives::build_calibration(ctx, r, scene),
            "consistency" => {
                for (n, t) in [
                    "Layout specimens",
                    "Action specimens",
                    "Input specimens",
                    "Navigation specimens",
                    "Overlay specimens",
                    "Feedback specimens",
                    "Document specimens",
                    "Data specimens",
                ]
                .iter()
                .enumerate()
                {
                    action(
                        s,
                        ctx,
                        scene,
                        200 + n,
                        Rect::new(
                            r.min_x(),
                            r.min_y() + n as f32 * 42.,
                            r.size.width.min(340.),
                            36.,
                        ),
                        t,
                        (false, false),
                    );
                }
            }
            "accessibility" => {
                let copy = "Interaction states: rest, hover, pressed, disabled and focus-visible. Selection and checked states belong to each component contract. Disabled controls do not activate; show focus-visible for keyboard navigation. A visual matrix does not prove real keyboard focus.\n\nKeyboard rules are component-specific. Dialogs contain focus and restore the invoker. Manual checks remain: VoiceOver role/name/value, physical IME, system contrast and assistive input.";
                let height = ctx
                    .text
                    .proposal_label(copy, proposal::TextRole::UiBody, Some(r.size.width))
                    .height as f32;
                label(
                    ctx,
                    scene,
                    Rect::new(r.min_x(), r.min_y(), r.size.width, height),
                    copy,
                    false,
                );
                super::pages::primitives::state_samples(
                    scene,
                    ctx.theme,
                    ctx.text,
                    Point::new(r.min_x(), r.min_y() + height + 32.),
                );
            }
            "internal-text" | "motion" => {}
            "typography" => {
                for (n, (title, role)) in [
                    ("Heading — Aa", proposal::TextRole::UiHeading),
                    ("Body — The quick brown fox", proposal::TextRole::UiBody),
                    ("Caption — 0123456789", proposal::TextRole::UiCaption),
                    (
                        "Composer — Natural paragraphs",
                        proposal::TextRole::ComposerBody,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    ctx.text.proposal_label_in(
                        scene,
                        ctx.theme,
                        Rect::new(r.min_x(), r.min_y() + n as f32 * 68., r.size.width, 58.),
                        title,
                        role,
                        ColorRole::PrimaryText.resolve(ctx.theme),
                    );
                }
            }
            "spacing" => {
                for (n, (name, v)) in tekes_ui_tokens::Spacing::ALL
                    .into_iter()
                    .map(|m| (format!("{m:?}"), m.value()))
                    .enumerate()
                {
                    label(
                        ctx,
                        scene,
                        Rect::new(r.min_x(), r.min_y() + n as f32 * 38., 220., 30.),
                        &format!("{name}: {v} pt"),
                        false,
                    );
                    scene.push_quad(Quad::new(
                        Rect::new(r.min_x() + 240., r.min_y() + n as f32 * 38., v, 24.),
                        paint::color(ctx.theme, "row-selection"),
                    ));
                }
            }
            _ => {
                let copy = match e.renderer() {
                    "iconography" => {
                        "Icon rules: use the documented 12 / 16 / 24 pt sizes, align optical centers, and pair semantic icons with readable labels. Open Components / Content / Icon for actual vector specimens."
                    }
                    "accessibility" | "a11y" => {
                        "Keyboard contracts are component-specific. Focus-visible follows keyboard input. Disabled controls do not activate. Dialogs contain focus and restore the invoker.\n\nManual checks still required: VoiceOver role/name/value, physical IME, system contrast, physical keyboard traversal and assistive input."
                    }
                    "matrix" => unreachable!("brand matrix has its own real preview"),
                    "stress" => {
                        "Use TEKES_PERF=scroll:60 and the existing measurement scripts. Actual presentation receipts determine 120 Hz acceptance. Build/test success does not prove 8.33 ms. Long-run isolated drops remain unresolved.\n\nText and ScrollArea pages provide real large-document scrolling and resize fixtures."
                    }
                    _ => story_docs(e),
                };
                label(ctx, scene, r, copy, false);
            }
        }
    }
    pub fn handle(&mut self, i: usize, event: &Event, text: &mut super::text::TextEngine) -> bool {
        let renderer = entry(i).renderer();
        if renderer == "matrix"
            && let Some(applied) = self.brand_preview.handle(event, text)
        {
            if let Some(brand) = applied {
                self.brand = brand;
            }
            return true;
        }
        let s = self.state(i);
        if let Event::Scroll(e) = event {
            let p = Point::new(e.position.x as f32, e.position.y as f32);
            if s.dialog_view.dialog().open {
                return s.dialog_view.blocks_background()
                    || matches!(s.dialog_view.wheel_at(p), DialogEvent::Blocked);
            }
            let input = MenuWheelInput {
                point: p,
                delta: Point::new(
                    e.delta.x as f32,
                    -(e.delta.y as f32) * if e.precise { 1.0 } else { 32.0 },
                ),
                precise: e.precise,
                begin: e.phase == tekes_ui_platform_macos::ScrollPhase::Began,
                end: matches!(
                    e.phase,
                    tekes_ui_platform_macos::ScrollPhase::Ended
                        | tekes_ui_platform_macos::ScrollPhase::Cancelled
                ),
                momentum: e.momentum_phase != tekes_ui_platform_macos::ScrollPhase::None,
                time: e.timestamp,
            };
            if s.menu_view.is_open()
                && matches!(
                    s.menu_view.wheel_event(input),
                    MenuWheelEvent::Consumed { .. }
                )
            {
                return true;
            }
            if s.select.menu.open
                && matches!(
                    s.select_view.wheel_event(input, &s.select),
                    MenuWheelEvent::Consumed { .. }
                )
            {
                return true;
            }
            let filtered_popup_open = match renderer {
                "combo-box" => s.extended.combo.open,
                "command-palette" => s.extended.command.combo.open,
                _ => false,
            };
            if filtered_popup_open {
                let result = match renderer {
                    "combo-box" => s.extended.combo_view.options.wheel_event(input),
                    "command-palette" => s.extended.command_view.options.wheel_event(input),
                    _ => MenuWheelEvent::Ignored,
                };
                if matches!(result, MenuWheelEvent::Consumed { .. }) {
                    return true;
                }
            }
        }
        if let Event::Scroll(e) = event {
            let p = Point::new(e.position.x as f32, e.position.y as f32);
            if renderer == "master" && s.list_bounds.contains(p) {
                let changed = s.list_scroll.wheel_at(
                    Point::new(0., -(e.delta.y as f32) * if e.precise { 1. } else { 32. }),
                    e.precise,
                    e.phase == tekes_ui_platform_macos::ScrollPhase::Began,
                    matches!(
                        e.phase,
                        tekes_ui_platform_macos::ScrollPhase::Ended
                            | tekes_ui_platform_macos::ScrollPhase::Cancelled
                    ),
                    e.momentum_phase != tekes_ui_platform_macos::ScrollPhase::None,
                    e.timestamp,
                );
                return changed || e.precise;
            }
        }

        // AppKit routes arrow and Return key-downs through NSTextInputClient's
        // `doCommandBySelector:` while the editable ComboBox input owns focus.
        // Route only list-navigation commands; caret/editing commands remain
        // owned by the real TextInput.
        if let Event::TextCommand(command) = event
            && matches!(renderer, "combo-box" | "command-palette")
            && s.focus == Some(1000)
            && s.input.state().is_none_or(|state| state.marked.is_none())
        {
            let key = match command.as_str() {
                "moveDown:" => Some(ControlKey::Down),
                "moveUp:" => Some(ControlKey::Up),
                "insertNewline:" => Some(ControlKey::Enter),
                "cancelOperation:" => Some(ControlKey::Escape),
                _ => None,
            };
            if let Some(key) = key
                && extended::handles_key(renderer, key, s.focus, &s.extended)
            {
                let mut events = Vec::new();
                let handled = s
                    .extended
                    .key(renderer, key, true, &mut |event| events.push(event));
                for event in events {
                    s.record(event);
                }
                if handled {
                    if key == ControlKey::Enter {
                        let (selected, items) = if renderer == "combo-box" {
                            (s.extended.combo.selected, &s.extended.combo.items)
                        } else {
                            (
                                s.extended.command.combo.selected,
                                &s.extended.command.combo.items,
                            )
                        };
                        if let Some(value) = selected.and_then(|index| items.get(index)) {
                            s.input.set_value(value);
                        }
                    }
                    return true;
                }
            }
        }
        if let Event::Key(k) = event {
            let key = match k.key_code {
                36 => Some(ControlKey::Enter),
                49 => Some(ControlKey::Space),
                123 => Some(ControlKey::Left),
                124 => Some(ControlKey::Right),
                125 => Some(ControlKey::Down),
                126 => Some(ControlKey::Up),
                115 => Some(ControlKey::Home),
                119 => Some(ControlKey::End),
                53 => Some(ControlKey::Escape),
                48 => Some(ControlKey::Tab {
                    reverse: k.modifiers.shift,
                }),
                _ => None,
            };
            if let Some(key) = key {
                let down = k.kind == KeyEventKind::Down;
                let composing = s.input.state().is_some_and(|state| state.marked.is_some());
                let extension_key = matches!(
                    key,
                    ControlKey::Enter
                        | ControlKey::Space
                        | ControlKey::Left
                        | ControlKey::Right
                        | ControlKey::Up
                        | ControlKey::Down
                        | ControlKey::Home
                        | ControlKey::End
                        | ControlKey::Escape
                );
                if extended::supports(renderer)
                    && extension_key
                    && extended::handles_key(renderer, key, s.focus, &s.extended)
                    && !composing
                    && (!matches!(renderer, "combo-box" | "command-palette")
                        || k.modifiers == tekes_ui_platform_macos::Modifiers::NONE)
                {
                    let mut events = Vec::new();
                    let handled = s
                        .extended
                        .key(renderer, key, down, &mut |event| events.push(event));
                    for event in events {
                        s.record(event);
                    }
                    if handled {
                        if matches!(
                            renderer,
                            "radio-group"
                                | "segmented-control"
                                | "list"
                                | "tree"
                                | "data-table"
                                | "chart"
                        ) && let Some(focused) = s.extended.focused
                        {
                            s.focus = Some(focused);
                        }
                        if renderer == "popover" && key == ControlKey::Escape && down {
                            s.focus = s.extended.focused;
                        }
                        if renderer == "combo-box"
                            && down
                            && key == ControlKey::Enter
                            && let Some(index) = s.extended.combo.selected
                            && let Some(value) = s.extended.combo.items.get(index)
                        {
                            s.input.set_value(value);
                        }
                        if renderer == "command-palette"
                            && down
                            && key == ControlKey::Enter
                            && let Some(index) = s.extended.command.combo.selected
                            && let Some(value) = s.extended.command.combo.items.get(index)
                        {
                            s.input.set_value(value);
                        }
                        return true;
                    }
                }
                if renderer == "link" && key == ControlKey::Space && s.focus == Some(20) {
                    return false;
                }
                if s.dialog_view.dialog().open {
                    let event = s.dialog_view.key(key, down);
                    s.dialog_event(event, renderer, false);
                    if s.dialog_view.dialog().open {
                        s.focus = Some(70 + s.dialog_view.dialog().focus);
                    }
                    return true;
                }
                if renderer == "select" && (s.select.menu.open || s.focus == Some(20)) {
                    if s.disabled {
                        return false;
                    }
                    let was_open = s.select.menu.open;
                    let event = s.select_view.key(key, down, &mut s.select);
                    s.menu_event(event, renderer, true);
                    if s.select.menu.open != was_open && s.select.menu.open {
                        s.focus = Some(20);
                        s.record("select: open");
                    }
                    if matches!(key, ControlKey::Enter | ControlKey::Space)
                        || (was_open
                            && matches!(
                                key,
                                ControlKey::Escape | ControlKey::Up | ControlKey::Down
                            ))
                    {
                        return true;
                    }
                }
                if s.menu_view.is_open() || s.select.menu.open {
                    let select = s.select.menu.open;
                    let event = if select {
                        // The select runtime above owns open-list keyboard
                        // navigation and selection.
                        MenuEvent::None
                    } else {
                        s.menu_view.key(key, down)
                    };
                    s.menu_event(event, renderer, select);
                    if !select {
                        return true;
                    }
                }
                if renderer == "number-field"
                    && s.number_view.focused_index().is_some()
                    && matches!(
                        key,
                        ControlKey::Up | ControlKey::Down | ControlKey::Left | ControlKey::Right
                    )
                {
                    if down
                        && s.number_view.key(&mut s.number, key)
                            == tekes_ui_controls::feedback_view::NumberFieldEvent::Changed
                    {
                        s.record(format!("value: {}", s.number.value()));
                    }
                    return true;
                }
                if renderer == "toolbar" && (20..23).contains(&s.focus.unwrap_or(usize::MAX)) {
                    if let Some(index) = s.toolbar_view.key(key, down) {
                        s.activate(20 + index, renderer);
                    }
                    if matches!(key, ControlKey::Enter | ControlKey::Space) {
                        return true;
                    }
                }
                if renderer == "number-field"
                    && s.focus.is_some_and(|id| (23..=24).contains(&id))
                    && matches!(key, ControlKey::Enter | ControlKey::Space)
                {
                    if s.number_view.key_event(&mut s.number, key, down)
                        == tekes_ui_controls::feedback_view::NumberFieldEvent::Changed
                    {
                        s.record(format!("value: {}", s.number.value()));
                    }
                    return true;
                }
                if renderer == "master"
                    && let Some(id) = s.focus.filter(|id| (10000..10100).contains(id))
                    && matches!(key, ControlKey::Up | ControlKey::Down)
                {
                    if down {
                        let current = id - 10000;
                        let next = match key {
                            ControlKey::Up => current.saturating_sub(1),
                            ControlKey::Down => (current + 1).min(79),
                            _ => current,
                        };
                        let target = 10000 + next;
                        if let Some(previous) = s.buttons.get_mut(&id) {
                            previous.state.focus_visible = false;
                        }
                        s.focus = Some(target);
                        s.selected = next;
                        s.focus_reveal = Some(target);
                        let focused = s.buttons.entry(target).or_default();
                        focused.state.focused = true;
                        focused.state.focus_visible = true;
                        s.record(format!("focus: document {next}"));
                    }
                    return true;
                }
                if let ControlKey::Tab { reverse } = key {
                    if down {
                        let mut ids: Vec<_> = s
                            .hits
                            .iter()
                            .map(|h| h.0)
                            .filter(|id| !s.buttons.get(id).is_some_and(|b| b.state.disabled))
                            .collect();
                        if s.extended.disabled {
                            match renderer {
                                "slider" | "switch" => ids.retain(|id| *id != 20),
                                "radio-group" => {
                                    ids.retain(|id| !(30..30 + s.extended.radio.len()).contains(id))
                                }
                                _ => {}
                            }
                        } else if renderer == "radio-group" {
                            let radio_stop = s
                                .extended
                                .radio_view
                                .focused()
                                .or(s.extended.radio.selected)
                                .map(|index| 30 + index);
                            ids.retain(|id| {
                                !(30..30 + s.extended.radio.len()).contains(id)
                                    || Some(*id) == radio_stop
                            });
                        }
                        if !ids.is_empty() {
                            let current = s.focus.and_then(|id| ids.iter().position(|n| *n == id));
                            let n = match current {
                                None => {
                                    if reverse {
                                        ids.len() - 1
                                    } else {
                                        0
                                    }
                                }
                                Some(n) => {
                                    if reverse {
                                        (n + ids.len() - 1) % ids.len()
                                    } else {
                                        (n + 1) % ids.len()
                                    }
                                }
                            };
                            s.input.blur();
                            s.detail.blur();
                            s.focus = Some(ids[n]);
                            if renderer == "master" && (10000..10100).contains(&ids[n]) {
                                s.focus_reveal = Some(ids[n]);
                            }
                            s.sync_owned_focus(renderer, s.focus, true);
                            if renderer == "tabs" && (30..33).contains(&ids[n]) {
                                s.tabs_view.focus(ids[n] - 30, true);
                            } else if renderer == "tabs" {
                                s.tabs_view.clear_focus();
                            }
                            if extended::supports(renderer) {
                                s.extended.host_focus_changed(renderer, ids[n], true);
                                if renderer == "icon-button" {
                                    s.extended.icon_button.button.state.focused = true;
                                }
                                if renderer == "link" {
                                    s.extended.link.button.state.focused = true;
                                }
                            }
                            if ids[n] == 1000 {
                                s.input.focus_field();
                            }
                            for b in s.buttons.values_mut() {
                                b.state.focused = false;
                                b.state.focus_visible = true;
                            }
                            let button = s.buttons.entry(ids[n]).or_default();
                            button.state.focused = true;
                            button.state.focus_visible = true;
                        }
                    }
                    return true;
                }
                if renderer == "tabs"
                    && s.focus.is_some_and(|f| (30..33).contains(&f))
                    && !matches!(key, ControlKey::Tab { .. })
                {
                    let previous = s.tabs_view.selected();
                    if let Some(selected) = s.tabs_view.key(key, down) {
                        s.focus = Some(30 + selected);
                    }
                    if previous != s.tabs_view.selected() {
                        s.record(format!("tab: {}", s.tabs_view.selected()));
                    }
                    return matches!(
                        key,
                        ControlKey::Left
                            | ControlKey::Right
                            | ControlKey::Home
                            | ControlKey::End
                            | ControlKey::Enter
                            | ControlKey::Space
                    );
                }
                if renderer == "checkbox" && key == ControlKey::Enter {
                    return false;
                }
                if let Some(id) = s.focus.filter(|id| *id != 1000) {
                    let b = s.buttons.entry(id).or_default();
                    b.state.focused = true;
                    b.state.focus_visible = true;
                    if b.key(key, down) {
                        s.activate(id, renderer);
                    }
                    return true;
                }
            }
        }
        if let Event::Mouse(m) = event {
            let p = Point::new(m.position.x as f32, m.position.y as f32);
            let composition_guarded = match m.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } => {
                    s.pointer_began_composing =
                        s.input.state().is_some_and(|state| state.marked.is_some());
                    false
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } => std::mem::take(&mut s.pointer_began_composing),
                _ => false,
            };
            if s.dialog_view.dialog().open {
                match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        s.dialog_view.pointer_move_at(p);
                    }
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let event = s.dialog_view.pointer_down_at(p);
                        s.dialog_event(event, renderer, false);
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let event = s.dialog_view.pointer_up_at(p);
                        s.dialog_event(event, renderer, composition_guarded);
                    }
                    _ => {}
                }
                if s.dialog_view.dialog().open {
                    s.focus = Some(70 + s.dialog_view.dialog().focus);
                }
                // A modal owns all pointer input in this Story's preview scope,
                // including header and underlay buttons. The gallery routes the
                // sidebar through a separate scope before reaching this handler.
                return true;
            }

            if renderer == "tabs" {
                match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        s.tabs_view.pointer_move_at(p);
                    }
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        if let Some(index) = s.tabs_view.pointer_down_at(p) {
                            s.focus = Some(30 + index);
                            return true;
                        }
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        if let Some(index) = s.tabs_view.pointer_up_at(p) {
                            s.focus = Some(30 + index);
                            s.record(format!("tab: {index}"));
                            return true;
                        }
                        if s.tabs_layout.as_ref().is_some_and(|layout| {
                            layout.visible.is_some_and(|visible| visible.contains(p))
                        }) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }

            if s.menu_view.is_open() {
                let layout = s.menu_layout.clone();
                match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        s.menu_view.pointer_move_at(p);
                        if layout.as_ref().is_some_and(|layout| {
                            layout.visible.is_some_and(|visible| visible.contains(p))
                        }) {
                            return true;
                        }
                    }
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let event = s.menu_view.pointer_down_at(p);
                        let dismissed = matches!(event, MenuEvent::Dismissed { .. });
                        s.menu_event(event, renderer, false);
                        let inside_popup = layout.as_ref().is_some_and(|layout| {
                            layout.popup.contains(p)
                                && layout.visible.is_some_and(|visible| visible.contains(p))
                        });
                        if !dismissed && !inside_popup && s.hit_at(p).is_some() {
                            let event = s.menu_view.deactivate();
                            s.menu_event(event, renderer, false);
                        }
                        if !dismissed && inside_popup {
                            return true;
                        }
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let event = s.menu_view.pointer_up_at(p);
                        let activated = matches!(event, MenuEvent::Activated { .. });
                        s.menu_event(event, renderer, false);
                        if activated
                            || layout.as_ref().is_some_and(|layout| {
                                layout.visible.is_some_and(|visible| visible.contains(p))
                            })
                        {
                            return true;
                        }
                    }
                    _ => {}
                }
            } else if s.select.menu.open {
                let layout = s.select_menu_layout.clone();
                match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        s.select_view.pointer_move(p, &mut s.select);
                        if layout.as_ref().is_some_and(|layout| {
                            layout.visible.is_some_and(|visible| visible.contains(p))
                        }) {
                            return true;
                        }
                    }
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let was_open = s.select.menu.open;
                        let consumed = s.select_view.pointer_down(p, &mut s.select);
                        let inside_popup = layout.as_ref().is_some_and(|layout| {
                            layout.popup.contains(p)
                                && layout.visible.is_some_and(|visible| visible.contains(p))
                        });
                        if was_open && !s.select.menu.open && s.hit_at(p).is_some() && !inside_popup
                        {
                            s.select_view.blur(&mut s.select);
                        }
                        if inside_popup || (consumed && s.hit_at(p) == Some(20)) {
                            s.focus = Some(20);
                            s.sync_owned_focus(renderer, s.focus, false);
                            return true;
                        }
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let event = s.select_view.pointer_up(p, &mut s.select);
                        let activated = matches!(event, MenuEvent::Activated { .. });
                        s.menu_event(event, renderer, true);
                        if activated
                            || layout.as_ref().is_some_and(|layout| {
                                layout.visible.is_some_and(|visible| visible.contains(p))
                            })
                        {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            let hit = s.hit_at(p);
            match m.kind {
                MouseEventKind::Moved | MouseEventKind::Entered => {
                    if renderer == "toolbar" {
                        s.toolbar_view.pointer_move(p);
                        if hit.is_some_and(|id| (20..23).contains(&id)) {
                            return true;
                        }
                    }
                    if renderer == "number-field" {
                        s.number_view.pointer_move(p);
                        if hit.is_some_and(|id| (23..=24).contains(&id)) {
                            return true;
                        }
                    }
                    if renderer == "select" {
                        s.select_view.pointer_move(p, &mut s.select);
                        if hit == Some(20) {
                            return true;
                        }
                    }
                    let mut changed = false;
                    for id in &s.button_hits {
                        if let Some(button) = s.buttons.get_mut(id) {
                            let hovered = button.state.hovered;
                            button.pointer_move_at(p);
                            changed |= hovered != button.state.hovered;
                        }
                    }
                    if extended::supports(renderer) {
                        let mut events = Vec::new();
                        let extended_changed =
                            s.extended
                                .pointer_move(renderer, p, hit, &mut |event| events.push(event));
                        for event in events {
                            s.record(event);
                        }
                        if changed || extended_changed {
                            return true;
                        }
                    }
                    for (id, b) in &mut s.buttons {
                        if s.button_hits.contains(id) {
                            continue;
                        }
                        let h = hit == Some(*id);
                        changed |= h != b.state.hovered;
                        b.state.hovered = h;
                    }
                    return changed;
                }
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } => {
                    if renderer == "toolbar" {
                        if let Some(index) = s.toolbar_view.pointer_down(p) {
                            s.input.blur();
                            s.detail.blur();
                            s.focus = Some(20 + index);
                            s.sync_owned_focus(renderer, s.focus, false);
                            return true;
                        }
                        if hit.is_some_and(|id| (20..23).contains(&id)) {
                            return true;
                        }
                    }
                    if renderer == "number-field" {
                        if let Some(increment) = s.number_view.pointer_down(p) {
                            s.input.blur();
                            s.detail.blur();
                            let id = if increment { 24 } else { 23 };
                            s.focus = Some(id);
                            s.sync_owned_focus(renderer, s.focus, false);
                            return true;
                        }
                        if hit.is_some_and(|id| (23..=24).contains(&id)) {
                            return true;
                        }
                    }
                    if renderer == "select" && s.disabled && hit == Some(20) {
                        return true;
                    }
                    if renderer == "select"
                        && !s.select.menu.open
                        && hit == Some(20)
                        && s.select_view.pointer_down(p, &mut s.select)
                    {
                        s.input.blur();
                        s.detail.blur();
                        s.focus = Some(20);
                        s.sync_owned_focus(renderer, s.focus, false);
                        return true;
                    }
                    if extended::supports(renderer)
                        && renderer == "popover"
                        && s.extended.popover.open
                        && !matches!(hit, Some(20 | 21))
                    {
                        let mut events = Vec::new();
                        let consumed =
                            s.extended
                                .pointer_down(renderer, hit.unwrap_or(0), p, &mut |event| {
                                    events.push(event)
                                });
                        for event in events {
                            s.record(event);
                        }
                        if consumed {
                            return true;
                        }
                    }
                    if let Some(id) = hit {
                        if extended::supports(renderer)
                            && match renderer {
                                "slider" | "switch" => id == 20,
                                "radio-group" => (30..30 + s.extended.radio.len()).contains(&id),
                                _ => false,
                            }
                            && s.extended.disabled
                        {
                            // Disabled retained controls consume their own hit
                            // region without manufacturing a generic Button
                            // focus/press state or changing host focus.
                            return true;
                        }
                        if s.button_hits.contains(&id)
                            && !(renderer == "popover" && s.extended.popover.open)
                        {
                            s.buttons.entry(id).or_default().pointer_down_at(p);
                        }
                        if extended::supports(renderer) {
                            s.extended.focused = Some(id);
                            if renderer == "icon-button" {
                                s.extended.icon_button.button.state.focused = true;
                            }
                            if renderer == "link" {
                                s.extended.link.button.state.focused = true;
                            }
                            let mut events = Vec::new();
                            let handled = s
                                .extended
                                .pointer_down(renderer, id, p, &mut |event| events.push(event));
                            for event in events {
                                s.record(event);
                            }
                            if handled {
                                s.focus = Some(id);
                                return true;
                            }
                        }
                        if renderer == "combo-box" && (40..43).contains(&id) {
                            // Selecting an option must leave the editable
                            // ComboBox input as the active native text client.
                            s.focus = Some(1000);
                            s.input.focus_field();
                            s.detail.blur();
                            s.buttons.entry(id).or_default().pointer_down();
                            return true;
                        }
                        if id != 1000 {
                            s.input.blur();
                            s.detail.blur();
                            s.sync_owned_focus(renderer, Some(id), false);
                            if renderer == "tabs" && !(30..33).contains(&id) {
                                s.tabs_view.clear_focus();
                            }
                            s.focus = Some(id);
                            let b = s.buttons.entry(id).or_default();
                            if !s.button_hits.contains(&id) {
                                b.pointer_down();
                            }
                            return true;
                        }
                        s.sync_owned_focus(renderer, Some(1000), false);
                        s.focus = Some(1000);
                        if renderer == "combo-box" && !s.extended.disabled && !s.extended.combo.open
                        {
                            s.extended.combo.open = true;
                            s.extended.combo.active = s.extended.combo.filtered().first().copied();
                        }
                    } else if s.dialog_view.dialog().open {
                        return true;
                    } else if s.menu_view.is_open() || s.select.menu.open {
                        let select = s.select.menu.open;
                        let event = if select {
                            s.select_view.blur(&mut s.select);
                            MenuEvent::None
                        } else {
                            s.menu_view.deactivate()
                        };
                        s.menu_event(event, renderer, select);
                        return true;
                    }
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } => {
                    if renderer == "toolbar" {
                        if let Some(index) = s.toolbar_view.pointer_up(p) {
                            s.activate(20 + index, renderer);
                        }
                        return true;
                    }
                    if renderer == "number-field" {
                        if s.number_view.pointer_up(p, &mut s.number)
                            == tekes_ui_controls::feedback_view::NumberFieldEvent::Changed
                        {
                            s.record(format!("value: {}", s.number.value()));
                        }
                        return true;
                    }
                    if renderer == "select" && !s.select.menu.open {
                        let event = s.select_view.pointer_up(p, &mut s.select);
                        let opened = s.select.menu.open;
                        if opened {
                            s.focus = Some(20);
                            s.record("select: open");
                        }
                        s.menu_event(event, renderer, true);
                        if hit == Some(20) || opened {
                            return true;
                        }
                    }
                    if extended::supports(renderer) {
                        let mut events = Vec::new();
                        let handled = s
                            .extended
                            .pointer_up(renderer, p, hit, &mut |event| events.push(event));
                        if renderer == "popover" && !s.extended.popover.open {
                            s.focus = s.extended.focused;
                        }
                        if renderer == "combo-box" && s.input.query() != s.extended.combo.query {
                            s.input.set_value(&s.extended.combo.query);
                            s.input.focus_field();
                            s.input.refresh(text);
                            s.focus = Some(1000);
                        }
                        for event in events {
                            s.record(event);
                        }
                        if handled {
                            return true;
                        }
                    }
                    let mut activated = None;
                    for id in s.button_hits.iter().rev().copied() {
                        if s.buttons
                            .get_mut(&id)
                            .is_some_and(|button| button.pointer_up_at(p))
                            && activated.is_none()
                        {
                            activated = Some(id);
                        }
                    }
                    for (id, button) in &mut s.buttons {
                        if !s.button_hits.contains(id)
                            && button.pointer_up(hit == Some(*id))
                            && activated.is_none()
                        {
                            activated = Some(*id);
                        }
                    }
                    if let Some(id) = activated {
                        s.activate_from_pointer(id, renderer, composition_guarded);
                        return true;
                    }
                }
                _ => {}
            }
        }
        let point = match event {
            Event::Mouse(m) => Some(Point::new(m.position.x as f32, m.position.y as f32)),
            Event::Scroll(e) => Some(Point::new(e.position.x as f32, e.position.y as f32)),
            _ => None,
        };
        let before = s.input.query().to_string();
        let input = s.input.handle(event, point).unwrap_or(false);
        let detail = if !input {
            s.detail.handle(event, point).unwrap_or(false)
        } else {
            false
        };
        s.input.refresh(text);
        s.detail.refresh(text);
        if before != s.input.query() {
            if renderer == "combo-box" {
                s.extended.combo.set_query(s.input.query());
            } else if renderer == "command-palette" {
                s.extended.command.combo.set_query(s.input.query());
            }
            s.record("input: value changed");
        }
        input || detail
    }
    pub fn specimen_destination(&self, i: usize, point: Point) -> Option<Page> {
        let s = self.states.get(&i)?;
        let id = s.hits.iter().find(|(_, r)| r.contains(point))?.0;
        if id >= 30000 {
            return entries()
                .get(id - 30000)
                .map(|_| Page::Workbench(id - 30000));
        }
        [
            Page::Foundations,
            Page::Actions,
            Page::Forms,
            Page::Navigation,
            Page::Overlays,
            Page::Feedback,
            Page::Documents,
            Page::Data,
        ]
        .get(id.checked_sub(200)?)
        .copied()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn story_routes_match_the_sections_exposed_by_each_entry() {
        for e in entries() {
            for story in [
                "docs",
                "playground",
                "states",
                "behaviors",
                "edge-cases",
                "variants",
            ] {
                assert_eq!(
                    route(&format!("{}/{story}", e.slug())).is_some(),
                    e.supports_story(story)
                );
            }
        }
        assert!(route("component-scroll-area/states").is_none());
        assert!(route("component-button/states").is_some());
    }

    #[test]
    fn calibration_and_full_token_atlas_reuse_shared_renderers() {
        for theme in tekes_ui_tokens::Theme::ALL {
            for renderer in ["geometry", "tokens"] {
                let i = entries()
                    .iter()
                    .position(|e| e.renderer() == renderer)
                    .unwrap();
                assert!(entry(i).matches(if renderer == "geometry" {
                    "overlap"
                } else {
                    "atlas"
                }));
                assert!(super::super::navigation::pages().contains(&Page::Workbench(i)));
                let mut w = Workbench::default();
                if renderer == "tokens" {
                    w.state(i).activate(401, renderer);
                }
                let mut text = super::super::text::TextEngine::new();
                let mut st = super::super::static_text::StaticText::new(None);
                let mut ctx = PageCtx {
                    theme,
                    text: &mut text,
                    static_text: &mut st,
                };
                let mut actual = Scene::new();
                w.build(i, &mut ctx, super::super::layout::PAGE_CONTENT, &mut actual);
                let r = w.preview_rect(i);
                let mut original = Scene::new();
                if renderer == "geometry" {
                    super::super::pages::primitives::build_calibration(&mut ctx, r, &mut original);
                } else {
                    super::super::pages::tokens::build(&mut ctx, r.offset(0., 50.), &mut original);
                }
                assert!(!original.is_empty());
                assert!(
                    actual.primitives().ends_with(original.primitives()),
                    "{renderer}/{theme:?}: migration dropped or altered content"
                );
            }
        }
    }

    #[test]
    fn motion_header_prepass_matches_the_header_drawn_by_workbench() {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == "motion")
            .unwrap();
        let mut workbench = Workbench {
            text_scale: 2,
            preview_width: 2,
            ..Workbench::default()
        };
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let content = Rect::new(312.0, 70.0, 1000.0, 820.0);
        let header = workbench.preview_header_layout(i, &mut text, content.size.width);
        let mut st = super::super::static_text::StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut st,
        };
        workbench.build(i, &mut ctx, content, &mut Scene::new());
        let preview = workbench.preview_rect(i);
        assert_eq!(preview.size.width, header.preview_width);
        assert!((preview.min_y() - (content.min_y() + header.header_height)).abs() <= 0.01);
    }

    #[test]
    fn wrapped_workbench_tabs_push_panel_below_real_button_frames() {
        use tekes_ui_scene::Primitive;

        let i = entries()
            .iter()
            .position(|entry| entry.renderer() == "button")
            .unwrap();
        let mut workbench = Workbench {
            text_scale: 2,
            preview_width: 0,
            ..Workbench::default()
        };
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let mut static_text = super::super::static_text::StaticText::new(None);
        let content = Rect::new(0.0, 0.0, 384.0, 1200.0);
        let mut scene = Scene::new();
        {
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            workbench.build(i, &mut ctx, content, &mut scene);
        }

        let state = workbench.state(i);
        let tab_frames: Vec<_> = (0..=6)
            .filter_map(|id| state.buttons.get(&id).and_then(Button::layout))
            .map(|layout| layout.frame)
            .collect();
        assert!(!tab_frames.is_empty());
        let tab_bottom = tab_frames
            .iter()
            .map(|frame| frame.max_y())
            .fold(f32::NEG_INFINITY, f32::max);
        let panel = workbench.preview_rect(i);
        assert!(
            panel.min_y() >= tab_bottom + 24.0 - 0.01,
            "tab row bottom={tab_bottom}, panel starts at {}",
            panel.min_y()
        );

        let scale = text.scale();
        let panel_glyph_top = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Glyphs(run) => Some(run.glyphs.iter()),
                _ => None,
            })
            .flatten()
            .filter_map(|positioned| {
                let glyph_top = positioned.origin.y + positioned.glyph.top as f32 / scale;
                let glyph_bottom = glyph_top + positioned.glyph.height as f32 / scale;
                if glyph_bottom < panel.min_y() {
                    return None;
                }
                if tab_frames
                    .iter()
                    .any(|frame| glyph_bottom > frame.min_y() && glyph_top < frame.max_y())
                {
                    return None;
                }
                Some(glyph_top)
            })
            .min_by(f32::total_cmp)
            .expect("panel should render text after the tabs");
        assert!(
            panel_glyph_top >= panel.min_y() - 1.0,
            "panel glyph starts at {panel_glyph_top}, panel at {}",
            panel.min_y()
        );
    }

    #[test]
    fn registry_has_unique_routes_and_component_entries() {
        let mut routes = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        for e in entries() {
            assert!(routes.insert(e.slug()));
            if !e.component().is_empty() {
                assert!(ids.insert(e.component()));
            }
            assert!(route(&format!("{}/playground", e.slug())).is_some());
        }
        assert!(route("component-button/invalid").is_none());
    }
    #[test]
    fn form_validation_failure_retry_and_reset() {
        let mut s = StoryState::default();
        s.input.set_value("");
        s.submit();
        assert_eq!(s.phase, Phase::Invalid);
        s.input.set_value("Workspace");
        s.submit();
        assert_eq!(s.phase, Phase::Pending);
        s.resolve();
        assert_eq!(s.phase, Phase::Failed);
        assert_eq!(s.input.query(), "Workspace");
        s.submit();
        s.resolve();
        assert_eq!(s.phase, Phase::Saved);
        s.activate(0, "settings");
        assert_eq!(s.phase, Phase::Ready);
        assert_eq!(s.attempts, 0);
    }
    #[test]
    fn deterministic_search_covers_ids_and_keywords() {
        let button = entries().iter().find(|e| e.title() == "Button").unwrap();
        assert!(button.matches("tui.button"));
        assert!(button.matches("keyboard"));
        assert!(!button.matches("xyzq"));
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use tekes_ui_platform_macos::{KeyEvent, Modifiers, MouseEvent, Point as NativePoint};
    fn index(renderer: &str) -> usize {
        entries()
            .iter()
            .position(|e| e.renderer() == renderer)
            .unwrap()
    }
    fn build(w: &mut Workbench, i: usize, t: &mut super::super::text::TextEngine) -> Scene {
        build_in(w, i, t, Rect::new(312., 70., 1000., 820.))
    }
    fn build_in(
        w: &mut Workbench,
        i: usize,
        t: &mut super::super::text::TextEngine,
        viewport: Rect,
    ) -> Scene {
        build_in_clipped(w, i, t, viewport, viewport)
    }
    fn build_in_clipped(
        w: &mut Workbench,
        i: usize,
        t: &mut super::super::text::TextEngine,
        content: Rect,
        viewport_clip: Rect,
    ) -> Scene {
        t.begin_frame();
        let mut st = super::super::static_text::StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: t,
            static_text: &mut st,
        };
        let mut scene = Scene::new();
        w.build_clipped(i, &mut ctx, content, viewport_clip, &mut scene);
        scene
    }

    #[test]
    fn master_detail_keyboard_focus_reveals_scrolled_rows_with_focus_bounds() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("master");
        workbench.state(i).focus = Some(10000);
        build(&mut workbench, i, &mut text);

        for _ in 0..50 {
            assert!(workbench.handle(i, &key(125, true), &mut text));
        }
        let scene = build(&mut workbench, i, &mut text);
        let state = workbench.state(i);
        assert_eq!(state.focus, Some(10050));
        assert_eq!(state.selected, 50);
        assert!(state.list_scroll.offset.y > 0.0);
        let button = &state.buttons[&10050];
        assert!(button.state.focus_visible);
        let layout = button.layout().expect("focused row is rendered");
        assert!(layout.hit_test(layout.frame.center()));
        assert!(
            layout
                .focus_bounds()
                .intersection(&state.list_bounds)
                .is_some()
        );
        assert!(
            scene.primitives().iter().any(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::Quad(q)
                if q.rect.min_y() < layout.frame.min_y()
                    && q.rect.max_y() > layout.frame.max_y())
            }),
            "focus ring is painted for the keyboard-focused row"
        );
    }

    #[test]
    fn standalone_action_uses_button_content_center_for_wrapped_150_percent_glyphs() {
        use proposal::layout::{AlignmentPreset, TextAlignment, alignment};

        assert_eq!(
            alignment(AlignmentPreset::ButtonContent).text_alignment,
            TextAlignment::Center
        );
        assert_eq!(ButtonTextAlign::default(), ButtonTextAlign::Center);

        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        text.begin_frame();
        let mut fixture = super::super::static_text::StaticText::new(None);
        let mut ctx = PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut fixture,
        };
        let mut state = StoryState::default();
        let mut scene = Scene::new();
        let title = "A centered action label that wraps naturally across several lines";
        action(
            &mut state,
            &mut ctx,
            &mut scene,
            22,
            Rect::new(40.0, 50.0, 190.0, 36.0),
            title,
            (false, false),
        );
        let layout = state.buttons[&22].layout().expect("action button layout");
        let runs = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                tekes_ui_scene::Primitive::Glyphs(run) => Some(run),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(runs.len() >= 2, "expected wrapped real GlyphRuns: {runs:?}");
        let scale = ctx.text.scale();
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        for glyph in runs.iter().flat_map(|run| &run.glyphs) {
            let left = (glyph.origin.x * scale).round() + glyph.glyph.left as f32;
            min_x = min_x.min(left);
            max_x = max_x.max(left + glyph.glyph.width as f32);
        }
        let glyph_center = (min_x + max_x) * 0.5 / scale;
        assert!(
            (glyph_center - layout.content.center().x).abs() <= 3.0,
            "real wrapped glyph block center {glyph_center} should match BUTTON_CONTENT center {}: {layout:?}",
            layout.content.center().x
        );
        assert!(
            layout.frame.size.height > 36.0,
            "wrapped action grows naturally"
        );
        assert_eq!(state.hit_at(layout.frame.center()), Some(22));
    }

    #[test]
    fn search_filter_row_wraps_without_overlapping_hits_or_results() {
        let gap = proposal::metric("spacing", "space-regular").unwrap();
        let results_gap = proposal::metric("spacing", "space-section").unwrap();
        for scale in [1.0, 1.5] {
            for width in [480.0, 280.0] {
                let mut workbench = Workbench::default();
                let mut text = super::super::text::TextEngine::new();
                text.set_type_scale(scale);
                let i = index("filter");
                assert!(!story_docs(entry(i)).contains("Resolve"));
                workbench.state(i).input.set_value("not-found");
                let content = Rect::new(24.0, 32.0, width, 1000.0);
                let scene = build_in(&mut workbench, i, &mut text, content);
                let state = workbench.state(i);
                let input = state.input.visible_frame().expect("filter input visible");
                let button = state.buttons[&22].layout().unwrap().frame;
                assert!(
                    input.intersection(&button).is_none(),
                    "input and clear button overlap at {scale}x/{width}pt: {input:?} {button:?}"
                );
                assert_eq!(state.hit_at(input.center()), Some(1000));
                assert_eq!(state.hit_at(button.center()), Some(22));
                if width > 400.0 {
                    assert!(input.max_x() + gap <= button.min_x() + 0.01);
                    assert!(
                        (input.center().y - button.center().y).abs() <= 1.0,
                        "wide preview should align input and button centers: {input:?} {button:?}"
                    );
                } else {
                    assert!(
                        button.min_y() >= input.max_y() + gap - 0.01,
                        "narrow preview should stack the button after the input"
                    );
                }

                let results_top = input.max_y().max(button.max_y()) + results_gap;
                let result_glyph_top = scene
                    .primitives()
                    .iter()
                    .filter_map(|primitive| match primitive {
                        tekes_ui_scene::Primitive::Glyphs(run) => Some(run.glyphs.iter()),
                        _ => None,
                    })
                    .flatten()
                    .map(|glyph| glyph.origin.y + glyph.glyph.top as f32 / text.scale())
                    .filter(|glyph_top| *glyph_top >= button.max_y())
                    .min_by(f32::total_cmp)
                    .expect("empty-result text should be drawn after the controls");
                assert!(
                    result_glyph_top >= results_top - 1.0,
                    "results begin at {result_glyph_top}, expected below row at {results_top}"
                );

                let point = button.center();
                assert!(workbench.handle(i, &mouse(point, true), &mut text));
                assert!(workbench.handle(i, &mouse(point, false), &mut text));
                assert_eq!(workbench.state(i).input.query(), "");
                assert!(
                    workbench
                        .state(i)
                        .log
                        .back()
                        .unwrap()
                        .contains("filter: cleared")
                );
            }
        }
    }

    fn mouse(p: Point, down: bool) -> Event {
        Event::Mouse(MouseEvent {
            position: NativePoint {
                x: p.x as f64,
                y: p.y as f64,
            },
            modifiers: Modifiers::NONE,
            kind: if down {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                }
            } else {
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                }
            },
        })
    }
    fn move_pointer(p: Point) -> Event {
        Event::Mouse(MouseEvent {
            position: NativePoint {
                x: p.x as f64,
                y: p.y as f64,
            },
            modifiers: Modifiers::NONE,
            kind: MouseEventKind::Moved,
        })
    }
    fn key(code: u16, down: bool) -> Event {
        key_with_modifiers(code, down, Modifiers::NONE)
    }
    fn key_with_modifiers(code: u16, down: bool, modifiers: Modifiers) -> Event {
        Event::Key(KeyEvent {
            kind: if down {
                KeyEventKind::Down
            } else {
                KeyEventKind::Up
            },
            key_code: code,
            modifiers,
            is_repeat: false,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        })
    }
    fn click(w: &mut Workbench, i: usize, id: usize, t: &mut super::super::text::TextEngine) {
        build(w, i, t);
        let state = w.state(i);
        let p = match state.visible_for_id(id) {
            Some(visible) => visible.center(),
            None => panic!(
                "missing visible component id {id} for {} (section {}, hits {:?})",
                entry(i).renderer(),
                state.section,
                state.hits.iter().map(|h| h.0).collect::<Vec<_>>()
            ),
        };
        w.handle(i, &mouse(p, true), t);
        w.handle(i, &mouse(p, false), t);
        build(w, i, t);
    }
    #[test]
    fn marked_composition_defers_pointer_submit_but_not_other_actions_or_later_commit() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("settings");
        {
            let state = w.state(i);
            state.phase = Phase::Failed;
            state.attempts = 1;
        }

        click(&mut w, i, 1000, &mut text);
        w.handle(
            i,
            &Event::TextPreedit {
                text: "计划".into(),
                clauses: vec![],
                selection: 2..2,
                replacement: None,
            },
            &mut text,
        );
        assert!(w.text_state(i).unwrap().marked.is_some());
        let draft_before_submit = w.state(i).input.query().to_owned();
        let save = w.state(i).visible_for_id(20).unwrap().center();
        assert!(w.handle(i, &mouse(save, true), &mut text));
        assert!(
            w.state(i)
                .input
                .state()
                .is_none_or(|state| state.marked.is_none())
        );
        assert!(w.handle(i, &mouse(save, false), &mut text));
        assert_eq!(w.state(i).phase, Phase::Failed);
        assert_eq!(w.state(i).attempts, 1);
        assert_eq!(w.state(i).input.query(), draft_before_submit);

        // A non-submit button still activates normally while it ends marked
        // input; its value remains in the draft rather than being discarded.
        click(&mut w, i, 1000, &mut text);
        w.handle(
            i,
            &Event::TextPreedit {
                text: "草稿".into(),
                clauses: vec![],
                selection: 2..2,
                replacement: None,
            },
            &mut text,
        );
        let draft_before_toggle = w.state(i).input.query().to_owned();
        click(&mut w, i, 25, &mut text);
        assert_eq!(w.state(i).checkbox.checked, CheckState::Checked);
        assert_eq!(w.state(i).input.query(), draft_before_toggle);
        assert_eq!(w.state(i).phase, Phase::Failed);

        // Once the platform reports a commit, pointer and keyboard activation
        // work normally; the composition guard is only for that pointer press.
        click(&mut w, i, 1000, &mut text);
        w.handle(
            i,
            &Event::TextPreedit {
                text: "项目".into(),
                clauses: vec![],
                selection: 2..2,
                replacement: None,
            },
            &mut text,
        );
        w.handle(
            i,
            &Event::TextCommit {
                text: "项目".into(),
                replacement: None,
            },
            &mut text,
        );
        assert!(w.text_state(i).unwrap().marked.is_none());
        click(&mut w, i, 20, &mut text);
        assert_eq!(w.state(i).phase, Phase::Pending);
        assert_eq!(w.state(i).attempts, 2);

        let mut keyboard = Workbench::default();
        let j = index("settings");
        keyboard.state(j).phase = Phase::Failed;
        keyboard.state(j).attempts = 1;
        click(&mut keyboard, j, 1000, &mut text);
        keyboard.handle(
            j,
            &Event::TextPreedit {
                text: "重试".into(),
                clauses: vec![],
                selection: 2..2,
                replacement: None,
            },
            &mut text,
        );
        keyboard.handle(
            j,
            &Event::TextCommit {
                text: "重试".into(),
                replacement: None,
            },
            &mut text,
        );
        keyboard.handle(j, &key(48, true), &mut text);
        keyboard.handle(j, &key(48, false), &mut text);
        keyboard.handle(j, &key(48, true), &mut text);
        keyboard.handle(j, &key(48, false), &mut text);
        assert_eq!(keyboard.state(j).focus, Some(20));
        keyboard.handle(j, &key(36, true), &mut text);
        keyboard.handle(j, &key(36, false), &mut text);
        assert_eq!(keyboard.state(j).attempts, 2);
        assert_eq!(keyboard.state(j).phase, Phase::Pending);
    }
    fn click_brand_preview(
        w: &mut Workbench,
        i: usize,
        id: usize,
        t: &mut super::super::text::TextEngine,
    ) -> Scene {
        build(w, i, t);
        let point = w
            .brand_preview
            .visible_for_id(id)
            .unwrap_or_else(|| panic!("missing brand preview hit target {id}"))
            .center();
        assert!(w.handle(i, &mouse(point, true), t));
        assert!(w.handle(i, &mouse(point, false), t));
        build(w, i, t)
    }

    #[test]
    fn brand_direction_is_preview_only_until_explicit_confirmation_and_cancel_keeps_input() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("matrix");
        let original = w.brand;
        build(&mut w, i, &mut t);
        let input = w.brand_preview.visible_for_id(1000).unwrap().center();
        assert!(w.handle(i, &mouse(input, true), &mut t));
        assert_eq!(
            w.brand_preview.input_value(),
            "Type to preview candidate text"
        );
        click_brand_preview(&mut w, i, 700, &mut t);
        assert_eq!(w.brand, original, "choosing a direction only previews");
        assert_eq!(
            w.brand_preview.input_value(),
            "Type to preview candidate text",
            "changing direction must preserve the input fixture"
        );
        click_brand_preview(&mut w, i, 723, &mut t);
        assert_eq!(w.brand, original, "cancel leaves the applied palette alone");
        build(&mut w, i, &mut t);
        assert!(w.brand_preview.visible_for_id(722).is_none());

        click_brand_preview(&mut w, i, 700, &mut t);
        assert_eq!(w.brand, original);
        click_brand_preview(&mut w, i, 722, &mut t);
        let rose = tekes_ui_tokens::palette::gallery_brand_presets()
            .iter()
            .find(|preset| preset.id == "rose")
            .unwrap()
            .palette_index;
        assert_eq!(
            w.brand, rose,
            "Confirm use alone updates the global Palette"
        );
    }

    #[test]
    fn candidate_dialog_keeps_neutral_surface_roles_without_applying_the_brand() {
        use tekes_ui_scene::Primitive;

        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("matrix");
        click_brand_preview(&mut w, i, 700, &mut t);
        click_brand_preview(&mut w, i, 721, &mut t);
        let scene = build(&mut w, i, &mut t);
        let layout = w.brand_preview.dialog_layout().unwrap();
        assert!(layout.visible.is_some());
        let candidate = tekes_ui_tokens::Palette {
            theme: tekes_ui_tokens::Theme::LIGHT,
            brand: tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .find(|preset| preset.id == "rose")
                .unwrap()
                .palette_index,
        };
        // This Dialog uses quiet actions, so its surface stays neutral. A
        // branded button behind the scrim must not count as modal evidence.
        let modal_ground = candidate.color("surface-popover").unwrap();
        assert!(
            scene.primitives().iter().any(|primitive| {
                matches!(primitive, Primitive::Quad(quad)
                if quad.fill == modal_ground && quad.rect == layout.modal)
            }),
            "the actual modal surface must use its neutral semantic role"
        );
        assert_eq!(
            w.brand, 0,
            "opening a preview dialog must not apply the candidate"
        );
    }

    #[test]
    fn brand_preview_tab_visits_each_target_once_and_enter_activates_on_key_down() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("matrix");
        click_brand_preview(&mut w, i, 700, &mut t);
        w.blur();
        build(&mut w, i, &mut t);
        let order = [700, 701, 702, 703, 704, 705, 706, 720, 1000, 721, 722, 723];
        for expected in order {
            assert!(w.handle(i, &key(48, true), &mut t));
            assert_eq!(w.brand_preview.focus_for_test(), Some(expected));
            build(&mut w, i, &mut t);
        }
        assert!(w.handle(i, &key(48, true), &mut t));
        assert_eq!(w.brand_preview.focus_for_test(), Some(700));
        assert!(w.handle(
            i,
            &key_with_modifiers(
                48,
                true,
                Modifiers {
                    shift: true,
                    ..Modifiers::NONE
                }
            ),
            &mut t
        ));
        assert_eq!(w.brand_preview.focus_for_test(), Some(723));

        let original = w.brand;
        let mut keyboard = Workbench::default();
        build(&mut keyboard, i, &mut t);
        assert!(keyboard.handle(i, &key(48, true), &mut t));
        assert_eq!(keyboard.brand_preview.focus_for_test(), Some(700));
        build(&mut keyboard, i, &mut t);
        assert!(keyboard.handle(i, &key(36, true), &mut t));
        assert_eq!(keyboard.brand, original);
        assert_eq!(
            keyboard.brand_preview.candidate_for_test(),
            Some(
                tekes_ui_tokens::palette::gallery_brand_presets()
                    .iter()
                    .find(|preset| preset.id == "rose")
                    .unwrap()
                    .palette_index
            )
        );
    }

    #[test]
    fn cancelling_pointer_capture_on_blur_prevents_delayed_confirm() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("matrix");
        let original = w.brand;
        click_brand_preview(&mut w, i, 700, &mut t);
        build(&mut w, i, &mut t);
        let confirm = w.brand_preview.visible_for_id(722).unwrap().center();
        assert!(w.handle(i, &mouse(confirm, true), &mut t));
        w.blur();
        build(&mut w, i, &mut t);
        assert!(w.handle(i, &mouse(confirm, false), &mut t));
        assert_eq!(w.brand, original);
        assert!(w.brand_preview.candidate_for_test().is_some());
    }

    #[test]
    fn candidate_dialog_consumes_scroll_and_unrecognized_keys() {
        use tekes_ui_platform_macos::{ScrollEvent, ScrollPhase};

        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("matrix");
        build(&mut w, i, &mut t);
        let input = w.brand_preview.visible_for_id(1000).unwrap().center();
        assert!(w.handle(i, &mouse(input, true), &mut t));
        assert!(w.text_state(i).is_some());
        click_brand_preview(&mut w, i, 700, &mut t);
        click_brand_preview(&mut w, i, 721, &mut t);
        assert!(w.text_state(i).is_none());
        let before = w.brand_preview.input_value().to_owned();
        for command in ["copy:", "cut:", "paste:", "undo:"] {
            let (changed, copied) = w.text_command(i, command, Some("background paste"), &mut t);
            assert!(
                !changed,
                "{command} must be blocked while the modal is open"
            );
            assert!(
                copied.is_none(),
                "{command} must not read the background selection"
            );
        }
        let scroll = Event::Scroll(ScrollEvent {
            timestamp: 0.0,
            delta: tekes_ui_platform_macos::Vec2 { x: 0.0, y: -40.0 },
            precise: true,
            phase: ScrollPhase::Began,
            momentum_phase: ScrollPhase::None,
            inverted: false,
            position: NativePoint { x: 20.0, y: 20.0 },
            modifiers: Modifiers::NONE,
        });
        assert!(w.handle(i, &scroll, &mut t));
        let unknown = key(0, true);
        assert!(w.handle(i, &unknown, &mut t));
        assert!(w.handle(
            i,
            &Event::TextCommit {
                text: "background text".into(),
                replacement: None,
            },
            &mut t,
        ));
        assert_eq!(w.brand_preview.input_value(), before);
        assert_eq!(w.brand_preview.focus_for_test(), Some(721));
        assert!(w.handle(i, &key(53, true), &mut t));
        assert_eq!(w.brand_preview.focus_for_test(), Some(721));
        assert!(w.text_state(i).is_none());
    }
    #[test]
    fn pointer_release_outside_cancels_real_button_and_inside_logs() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("button");
        build(&mut w, i, &mut t);
        let p = w
            .state(i)
            .hits
            .iter()
            .find(|h| h.0 == 20)
            .unwrap()
            .1
            .center();
        w.handle(i, &mouse(p, true), &mut t);
        assert!(w.state(i).buttons[&20].state.pressed);
        w.handle(i, &mouse(Point::new(1400., 900.), false), &mut t);
        assert!(w.state(i).log.is_empty());
        click(&mut w, i, 20, &mut t);
        assert_eq!(w.state(i).log.len(), 1);
    }

    #[test]
    fn checkbox_story_uses_component_row_geometry_for_click_disabled_and_resize() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("checkbox");
        build(&mut w, i, &mut t);
        let state = w.state(i);
        let interaction_layout = state.buttons[&20].layout().unwrap();
        assert_eq!(interaction_layout.frame.size.width, 400.0);
        assert_eq!(interaction_layout.frame.size.height, 36.0);

        let row_end = Point::new(
            interaction_layout.frame.max_x() - 4.0,
            interaction_layout.frame.center().y,
        );
        assert_eq!(state.hit_at(row_end), Some(20));
        w.handle(i, &mouse(row_end, true), &mut t);
        assert!(w.state(i).buttons[&20].state.pressed);
        w.handle(i, &mouse(row_end, false), &mut t);
        assert_eq!(w.state(i).checkbox.checked, CheckState::Checked);

        w.state(i).disabled = true;
        build(&mut w, i, &mut t);
        let disabled_row = w.state(i).buttons[&20].layout().unwrap().frame;
        let disabled_point = Point::new(disabled_row.max_x() - 4.0, disabled_row.center().y);
        w.handle(i, &mouse(disabled_point, true), &mut t);
        w.handle(i, &mouse(disabled_point, false), &mut t);
        assert_eq!(w.state(i).checkbox.checked, CheckState::Checked);
        assert!(!w.state(i).buttons[&20].state.pressed);

        w.state(i).disabled = false;
        let old_point = disabled_point;
        let content = Rect::new(312.0, 70.0, 180.0, 700.0);
        build_in(&mut w, i, &mut t, content);
        let resized = w.state(i).buttons[&20].layout().unwrap();
        assert!(resized.frame.size.width < 400.0);
        assert!(!resized.hit_test(old_point));
        assert_ne!(w.state(i).hit_at(old_point), Some(20));
    }

    #[test]
    fn checkbox_preserves_input_modality_for_focus_visible_and_window_blur() {
        let mut mouse_workbench = Workbench::default();
        let mut mouse_text = super::super::text::TextEngine::new();
        let i = index("checkbox");
        build(&mut mouse_workbench, i, &mut mouse_text);
        let row = mouse_workbench.state(i).buttons[&20]
            .layout()
            .unwrap()
            .frame;
        let point = row.center();
        mouse_workbench.handle(i, &mouse(point, true), &mut mouse_text);
        mouse_workbench.handle(i, &mouse(point, false), &mut mouse_text);
        build(&mut mouse_workbench, i, &mut mouse_text);
        assert!(mouse_workbench.state(i).buttons[&20].state.focused);
        assert!(!mouse_workbench.state(i).buttons[&20].state.focus_visible);

        let mut keyboard_workbench = Workbench::default();
        let mut keyboard_text = super::super::text::TextEngine::new();
        build(&mut keyboard_workbench, i, &mut keyboard_text);
        let tab_steps = keyboard_workbench.state(i).hits.len() + 1;
        for _ in 0..tab_steps {
            if keyboard_workbench.state(i).focus == Some(20) {
                break;
            }
            keyboard_workbench.handle(i, &key(48, true), &mut keyboard_text);
            build(&mut keyboard_workbench, i, &mut keyboard_text);
        }
        assert_eq!(keyboard_workbench.state(i).focus, Some(20));
        keyboard_workbench.handle(i, &key(49, true), &mut keyboard_text);
        build(&mut keyboard_workbench, i, &mut keyboard_text);
        assert!(keyboard_workbench.state(i).buttons[&20].state.focused);
        assert!(keyboard_workbench.state(i).buttons[&20].state.focus_visible);
        assert!(keyboard_workbench.state(i).buttons[&20].state.pressed);

        keyboard_workbench.set_window_active(false);
        build(&mut keyboard_workbench, i, &mut keyboard_text);
        assert!(!keyboard_workbench.state(i).buttons[&20].state.focused);
        assert!(!keyboard_workbench.state(i).buttons[&20].state.focus_visible);
        assert!(!keyboard_workbench.state(i).buttons[&20].state.pressed);
        assert!(!keyboard_workbench.state(i).buttons[&20].state.hovered);
        keyboard_workbench.handle(i, &key(49, false), &mut keyboard_text);
        assert_eq!(
            keyboard_workbench.state(i).checkbox.checked,
            CheckState::Unchecked
        );
    }

    #[test]
    fn checkbox_text_first_line_stays_centered_on_the_mark_at_large_scale_and_wrap() {
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let role = proposal::TextRole::UiBody;
        for (label, width, wraps) in [
            ("Notifications", 400.0, false),
            ("通知项目设置 日本語 🐈", 400.0, false),
            (
                "A long mixed-script setting 通知项目設定 日本語 🐈 that wraps in a narrow column",
                160.0,
                true,
            ),
        ] {
            let mut checkbox = Checkbox::default();
            let mut scene = Scene::new();
            let layout = checkbox.render(
                &mut scene,
                &mut text,
                label,
                Rect::new(10.0, 20.0, width, 36.0),
                None,
                tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT),
                role,
            );
            let expected = text.proposal_label(label, role, Some(layout.label.size.width));
            assert_eq!(expected.lines.len() > 1, wraps);
            let first_line_height = expected.lines[0].height as f32;
            assert!((layout.label.size.height - expected.height as f32).abs() < 0.1);
            assert!(
                (layout.control.center().y - (layout.label.min_y() + first_line_height * 0.5))
                    .abs()
                    < 0.1,
                "first line must align with the checkbox mark for {label:?}"
            );
            assert!(layout.control.min_y() >= layout.frame.min_y());
            assert!(layout.control.max_y() <= layout.frame.max_y());
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, tekes_ui_scene::Primitive::PushClip(_)))
            );
        }
    }

    #[test]
    fn master_detail_rows_share_scroll_geometry_for_visible_paint_and_selection() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("master");
        let content = Rect::new(312.0, 70.0, 1000.0, 820.0);
        build_in(&mut w, i, &mut t, content);
        let list = w.state(i).list_bounds;
        assert!(!list.is_empty());
        w.state(i).list_scroll.scroll_by(0.0, 84.0);
        build_in(&mut w, i, &mut t, content);
        let row_two = w
            .state(i)
            .hits
            .iter()
            .find(|hit| hit.0 == 10002)
            .map(|hit| hit.1)
            .expect("third document row remains visible after scrolling");
        let point = row_two.center();
        w.handle(i, &mouse(point, true), &mut t);
        w.handle(i, &mouse(point, false), &mut t);
        assert_eq!(w.state(i).selected, 2);
        assert!(w.state(i).log.back().unwrap().contains("selection: 2"));

        let narrow_content = Rect::new(312.0, 70.0, 420.0, 600.0);
        let tiny_clip = Rect::new(312.0, 70.0, 420.0, 90.0);
        build_in_clipped(&mut w, i, &mut t, narrow_content, tiny_clip);
        let visible_list = w.state(i).list_bounds;
        assert!(
            visible_list.is_empty() || visible_list.max_y() <= tiny_clip.max_y(),
            "list bounds must be clipped or empty: {visible_list:?}"
        );
        let stale = row_two.center();
        assert_ne!(w.state(i).hit_at(stale), Some(10002));
    }

    #[test]
    fn clipped_workbench_text_field_measures_scaled_line_and_only_focuses_visible_area() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let field = index("field");
        workbench.select_route(field, "playground");
        let content = Rect::new(312.0, 70.0, 420.0, 700.0);
        build_in_clipped(&mut workbench, field, &mut text, content, content);
        let full_height = workbench
            .state(field)
            .input
            .visible_frame()
            .unwrap()
            .size
            .height;
        assert!(
            full_height > 36.0,
            "scaled text field height: {full_height}"
        );
        let preview = workbench.preview_rect(field);
        let input_clip = Rect::new(preview.min_x(), preview.min_y(), preview.size.width, 100.0);
        build_in_clipped(&mut workbench, field, &mut text, content, input_clip);

        let visible = workbench.state(field).input.visible_frame().unwrap();
        assert!(visible.max_y() <= input_clip.max_y());
        assert!(visible.size.height > 0.0);
        let hidden_point = Point::new(visible.center().x, input_clip.max_y() + 8.0);
        workbench.handle(field, &mouse(hidden_point, true), &mut text);
        assert!(workbench.text_state(field).is_none());

        let visible_point = visible.center();
        workbench.handle(field, &mouse(visible_point, true), &mut text);
        assert!(workbench.text_state(field).is_some());
    }

    #[test]
    fn multiline_text_area_reuses_its_wrapped_layout_across_builds() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let area = index("area");
        workbench.select_route(area, "playground");
        let content = Rect::new(312.0, 70.0, 420.0, 700.0);
        build_in_clipped(&mut workbench, area, &mut text, content, content);
        let shaped = text.shape_count();
        build_in_clipped(&mut workbench, area, &mut text, content, content);
        assert_eq!(text.shape_count(), shaped);
    }

    #[test]
    fn multiline_controls_stop_at_the_remaining_preview_bottom() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let content = Rect::new(312.0, 70.0, 420.0, 500.0);

        let area = index("area");
        workbench.select_route(area, "playground");
        build_in_clipped(&mut workbench, area, &mut text, content, content);
        let preview = workbench.preview_rect(area);
        let input = workbench
            .state(area)
            .input
            .visible_frame()
            .expect("TextArea has visible remaining space");
        assert!(input.max_y() <= preview.max_y() + 0.1);

        let master = index("master");
        workbench.select_route(master, "playground");
        build_in_clipped(&mut workbench, master, &mut text, content, content);
        let preview = workbench.preview_rect(master);
        let list = workbench.state(master).list_bounds;
        assert!(list.max_y() <= preview.max_y() + 0.1);
    }

    #[test]
    fn scrolled_workbench_button_uses_fixed_viewport_for_hit_and_paint() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("button");
        let content = Rect::new(312.0, 70.0, 1000.0, 820.0);
        build_in_clipped(&mut w, i, &mut t, content, content);
        let original = w.state(i).buttons[&20].layout().unwrap().frame;

        let viewport = Rect::new(312.0, 70.0, 1000.0, 100.0);
        let scroll_offset = original.min_y() - (viewport.max_y() - 8.0);
        assert!(scroll_offset > 0.0);
        let translated = content.offset(0.0, -scroll_offset);
        build_in_clipped(&mut w, i, &mut t, translated, viewport);

        let button = &w.state(i).buttons[&20];
        let layout = button.layout().unwrap();
        let visible = layout.visible.unwrap();
        assert!((visible.min_y() - (viewport.max_y() - 8.0)).abs() < 0.01);
        assert!((visible.max_y() - viewport.max_y()).abs() < 0.01);
        let outside = Point::new(layout.frame.center().x, layout.frame.max_y() - 1.0);
        assert!(!layout.hit_test(outside));
        assert!(visible.size.height < layout.frame.size.height);

        w.handle(i, &mouse(outside, true), &mut t);
        w.handle(i, &mouse(outside, false), &mut t);
        assert!(w.state(i).log.is_empty());

        let inside = visible.center();
        w.handle(i, &mouse(inside, true), &mut t);
        w.handle(i, &mouse(inside, false), &mut t);
        assert_eq!(w.state(i).log.len(), 1);
    }

    #[test]
    fn unrelated_mouse_release_does_not_cancel_keyboard_or_cross_release_button_arms() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("button");
        build(&mut w, i, &mut t);
        w.state(i).focus = Some(20);
        w.state(i).buttons.entry(20).or_default().state.focused = true;

        w.handle(i, &key(49, true), &mut t);
        assert!(w.state(i).buttons[&20].state.pressed);
        w.handle(i, &mouse(Point::new(1400.0, 900.0), false), &mut t);
        assert!(w.state(i).buttons[&20].state.pressed);
        w.handle(i, &key(49, false), &mut t);
        assert!(!w.state(i).buttons[&20].state.pressed);
        assert_eq!(w.state(i).log.len(), 1);

        build(&mut w, i, &mut t);
        let p = w
            .state(i)
            .hits
            .iter()
            .find(|h| h.0 == 20)
            .unwrap()
            .1
            .center();
        w.handle(i, &mouse(p, true), &mut t);
        w.handle(i, &key(49, false), &mut t);
        assert!(w.state(i).buttons[&20].state.pressed);
        assert_eq!(w.state(i).log.len(), 1);
        w.handle(i, &mouse(Point::new(1400.0, 900.0), false), &mut t);
        assert!(!w.state(i).buttons[&20].state.pressed);
        assert_eq!(w.state(i).log.len(), 1);
    }
    #[test]
    fn tabs_menu_and_modal_use_distinct_keyboard_contracts() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("tabs");
        click(&mut w, i, 30, &mut t);
        w.handle(i, &key(124, true), &mut t);
        assert_eq!(w.state(i).tabs_view.selected(), 1);
        let i = index("menu");
        click(&mut w, i, 20, &mut t);
        w.handle(i, &key(125, true), &mut t);
        w.handle(i, &key(36, true), &mut t);
        w.handle(i, &key(36, false), &mut t);
        assert_eq!(w.state(i).menu_view.menu().last_action, Some(1));
        assert_eq!(w.state(i).focus, Some(20));
        let i = index("dialog");
        click(&mut w, i, 20, &mut t);
        w.handle(i, &key(48, true), &mut t);
        assert_eq!(w.state(i).dialog_view.dialog().focus, 1);
        w.handle(i, &key(53, true), &mut t);
        assert!(!w.state(i).dialog_view.dialog().open);
        assert_eq!(w.state(i).focus, Some(20));
    }

    #[test]
    fn tabs_use_one_roving_tabstop_and_pointer_release_cancels_selection() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("tabs");
        build(&mut w, i, &mut t);
        let tab_ids: Vec<_> = w
            .state(i)
            .hits
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| (30..33).contains(id))
            .collect();
        assert_eq!(tab_ids, [30], "only the selected tab is a tab stop");

        let target = w.state(i).visible_for_id(31).unwrap().center();
        w.handle(i, &mouse(target, true), &mut t);
        assert_eq!(w.state(i).tabs_view.selected(), 0);
        w.handle(
            i,
            &mouse(Point::new(target.x, target.y + 200.0), false),
            &mut t,
        );
        assert_eq!(w.state(i).tabs_view.selected(), 0);

        w.handle(i, &mouse(target, true), &mut t);
        w.handle(i, &mouse(target, false), &mut t);
        assert_eq!(w.state(i).tabs_view.selected(), 1);
        build(&mut w, i, &mut t);
        let tab_ids: Vec<_> = w
            .state(i)
            .hits
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| (30..33).contains(id))
            .collect();
        assert_eq!(tab_ids, [31]);
    }

    #[test]
    fn tabs_panel_uses_expanded_component_frame_at_large_text_scale() {
        use tekes_ui_scene::Primitive;

        let mut workbench = Workbench {
            text_scale: 2,
            density: 1,
            ..Workbench::default()
        };
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let i = index("tabs");
        let content = Rect::new(312.0, 70.0, 420.0, 700.0);
        let scene = build_in(&mut workbench, i, &mut text, content);
        let tabs = workbench.state(i).tabs_layout.as_ref().unwrap().clone();
        assert!(
            tabs.frame.size.height > 30.0,
            "fixture should expand the compact tab row"
        );

        let panel_glyph_top = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Glyphs(run) => Some(run.glyphs.iter()),
                _ => None,
            })
            .flatten()
            .filter_map(|positioned| {
                let top = positioned.origin.y + positioned.glyph.top as f32 / text.scale();
                (top >= tabs.frame.max_y()).then_some(top)
            })
            .min_by(f32::total_cmp)
            .expect("tab panel text is rendered below the component");
        assert!(
            panel_glyph_top >= tabs.frame.max_y() + 19.0,
            "panel text starts at {panel_glyph_top}, tabs end at {}",
            tabs.frame.max_y()
        );
    }

    #[test]
    fn menu_escape_outside_click_and_relayout_use_component_geometry() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("menu");
        click(&mut w, i, 20, &mut t);
        assert!(w.state(i).menu_view.is_open());
        w.handle(i, &key(53, true), &mut t);
        assert!(!w.state(i).menu_view.is_open());
        assert_eq!(w.state(i).focus, Some(20));

        click(&mut w, i, 20, &mut t);
        let old = w.state(i).menu_layout.as_ref().unwrap().items[0]
            .visible
            .unwrap()
            .center();
        build_in_clipped(
            &mut w,
            i,
            &mut t,
            Rect::new(700.0, 90.0, 360.0, 600.0),
            Rect::new(700.0, 90.0, 360.0, 600.0),
        );
        w.handle(i, &mouse(old, true), &mut t);
        w.handle(i, &mouse(old, false), &mut t);
        assert_ne!(w.state(i).menu_view.menu().last_action, Some(0));

        // Opening again and pressing a different live control dismisses first,
        // then dispatches the same click to that control.
        w.state(i).buttons.entry(20).or_default().state.disabled = false;
        click(&mut w, i, 20, &mut t);
        build(&mut w, i, &mut t);
        let outside = w
            .state(i)
            .hits
            .iter()
            .find(|(id, _)| *id == 10)
            .map(|(_, frame)| frame.center())
            .expect("the enabled property control is a live underlying target");
        w.handle(i, &mouse(outside, true), &mut t);
        w.handle(i, &mouse(outside, false), &mut t);
        assert!(!w.state(i).menu_view.is_open());
        assert!(w.state(i).disabled);
    }

    #[test]
    fn dialog_contains_focus_and_pointer_input_and_survives_window_deactivation() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("dialog");
        click(&mut w, i, 20, &mut t);
        let layout = w.state(i).dialog_layout.as_ref().unwrap().clone();
        let backdrop = Point::new(
            layout.visible.unwrap().min_x() + 2.0,
            layout.visible.unwrap().min_y() + 2.0,
        );
        w.handle(i, &mouse(backdrop, true), &mut t);
        w.handle(i, &mouse(backdrop, false), &mut t);
        assert!(w.state(i).dialog_view.dialog().open);

        w.set_window_active(false);
        assert!(w.state(i).dialog_view.dialog().open);
        w.handle(i, &key(53, true), &mut t);
        assert!(w.state(i).dialog_view.dialog().open);
        w.set_window_active(true);
        w.handle(i, &key(53, true), &mut t);
        assert!(!w.state(i).dialog_view.dialog().open);
        assert_eq!(w.state(i).focus, Some(20));
    }

    #[test]
    fn dialog_blocks_story_header_and_underlay_controls_and_space_arms() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("dialog");
        click(&mut w, i, 20, &mut t);
        assert!(w.state(i).dialog_view.dialog().open);
        let section = w.state(i).section;

        for id in [0, 2, 10] {
            build(&mut w, i, &mut t);
            let target = w
                .state(i)
                .visible_for_id(id)
                .expect("header / underlay button is laid out")
                .center();
            w.handle(i, &mouse(target, true), &mut t);
            w.handle(i, &mouse(target, false), &mut t);
            assert!(w.state(i).dialog_view.dialog().open);
            assert_eq!(
                w.state(i).section,
                section,
                "Story navigation is modal-blocked"
            );
            assert!(!w.state(i).disabled, "Disabled property remains unchanged");
        }

        w.state(i).focus = Some(10);
        w.handle(i, &key(49, true), &mut t);
        assert!(
            !w.state(i)
                .buttons
                .get(&10)
                .is_some_and(|button| button.state.pressed)
        );
        w.handle(i, &key(49, false), &mut t);
        assert!(
            !w.state(i).disabled,
            "Space never activates the underlay toggle"
        );
        assert_eq!(
            w.state(i).section,
            section,
            "Space never activates Story navigation"
        );
        assert!(
            !w.state(i)
                .buttons
                .get(&10)
                .is_some_and(|button| button.state.pressed)
        );
        if w.state(i).dialog_view.dialog().open {
            w.handle(i, &key(53, true), &mut t);
        }
        assert!(!w.state(i).dialog_view.dialog().open);
    }

    #[test]
    fn input_composition_clipboard_undo_and_disabled_are_real() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("field");
        click(&mut w, i, 1000, &mut t);
        w.handle(
            i,
            &Event::TextPreedit {
                text: "中文".into(),
                clauses: vec![],
                selection: 2..2,
                replacement: None,
            },
            &mut t,
        );
        assert!(w.text_state(i).unwrap().marked.is_some());
        w.handle(
            i,
            &Event::TextCommit {
                text: "项目".into(),
                replacement: None,
            },
            &mut t,
        );
        assert!(w.state(i).input.query().contains("项目"));
        w.text_command(i, "undo:", None, &mut t);
        assert_eq!(w.state(i).input.query(), "Studio");
        w.text_command(i, "selectAll:", None, &mut t);
        assert_eq!(
            w.text_command(i, "copy:", None, &mut t).1.as_deref(),
            Some("Studio")
        );
        click(&mut w, i, 10, &mut t);
        click(&mut w, i, 1000, &mut t);
        w.handle(
            i,
            &Event::TextCommit {
                text: "No edit".into(),
                replacement: None,
            },
            &mut t,
        );
        assert_eq!(w.state(i).input.query(), "Studio");
    }
    #[test]
    fn settings_full_failure_retry_and_danger_cancel_recovery() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("settings");
        click(&mut w, i, 20, &mut t);
        assert_eq!(w.state(i).phase, Phase::Pending);
        click(&mut w, i, 21, &mut t);
        assert_eq!(w.state(i).phase, Phase::Failed);
        click(&mut w, i, 20, &mut t);
        click(&mut w, i, 21, &mut t);
        assert_eq!(w.state(i).phase, Phase::Saved);
        let i = index("danger");
        click(&mut w, i, 20, &mut t);
        click(&mut w, i, 70, &mut t);
        assert_eq!(w.state(i).phase, Phase::Ready);
        click(&mut w, i, 20, &mut t);
        click(&mut w, i, 71, &mut t);
        click(&mut w, i, 21, &mut t);
        assert_eq!(w.state(i).phase, Phase::Failed);
    }

    #[test]
    fn large_type_form_phase_and_event_log_glyphs_do_not_overlap_in_visible_preview() {
        use tekes_ui_scene::Primitive;

        let mut workbench = Workbench {
            preview_width: 1,
            text_scale: 2,
            ..Workbench::default()
        };
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let i = index("settings");
        {
            let state = workbench.state(i);
            state.log.push_back("submit: failed".into());
            state.log.push_back("submit: retry ready".into());
        }
        let content = Rect::new(312.0, 70.0, 900.0, 720.0);
        let scene = build_in(&mut workbench, i, &mut text, content);
        let state = workbench.state(i);
        let phase = state.phase_frame.expect("form phase frame is recorded");
        let log = state.event_log_frame.expect("event log frame is recorded");
        assert!(phase.max_y() + 23.0 <= log.min_y());
        assert!(
            log.max_y() <= content.max_y(),
            "event log must remain within the visible preview: {log:?} vs {content:?}"
        );

        let scale = text.scale();
        let glyph_bounds = |color| {
            scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::Glyphs(run) if run.color == color => Some(run.glyphs.iter()),
                    _ => None,
                })
                .flatten()
                .map(|glyph| {
                    Rect::new(
                        glyph.origin.x + glyph.glyph.left as f32 / scale,
                        glyph.origin.y + glyph.glyph.top as f32 / scale,
                        glyph.glyph.width as f32 / scale,
                        glyph.glyph.height as f32 / scale,
                    )
                })
                .collect::<Vec<_>>()
        };
        let in_frame = |bounds: Vec<Rect>, frame: Rect| {
            bounds
                .into_iter()
                .filter(|bounds| frame.intersection(bounds).is_some())
                .collect::<Vec<_>>()
        };
        let phase_glyphs = in_frame(
            glyph_bounds(ColorRole::PrimaryText.resolve(tekes_ui_tokens::Theme::LIGHT)),
            phase,
        );
        let log_glyphs = in_frame(
            glyph_bounds(ColorRole::SecondaryText.resolve(tekes_ui_tokens::Theme::LIGHT)),
            log,
        );
        assert!(
            !phase_glyphs.is_empty(),
            "phase copy is present in the scene"
        );
        assert!(
            !log_glyphs.is_empty(),
            "event log copy is present in the scene"
        );
        let log_title = Rect::new(log.min_x(), log.min_y(), log.size.width, 22.);
        assert!(
            !in_frame(
                glyph_bounds(ColorRole::SecondaryText.resolve(tekes_ui_tokens::Theme::LIGHT)),
                log_title,
            )
            .is_empty(),
            "event log title is rendered"
        );
        assert!(
            log_glyphs.iter().any(|glyph| {
                content
                    .intersection(glyph)
                    .is_some_and(|visible| !visible.is_empty())
            }),
            "at least one event log glyph must remain visible in the preview"
        );
        for phase_glyph in &phase_glyphs {
            for log_glyph in &log_glyphs {
                assert!(
                    phase_glyph
                        .intersection(log_glyph)
                        .is_none_or(|overlap| overlap.is_empty()),
                    "phase glyph {phase_glyph:?} overlaps event log glyph {log_glyph:?}"
                );
            }
        }
    }
    #[test]
    fn preview_density_brand_and_font_scale_change_actual_scene() {
        let mut w = Workbench::default();
        let mut t = super::super::text::TextEngine::new();
        let i = index("button");
        let original = build(&mut w, i, &mut t);
        w.brand = 1;
        assert_ne!(original, build(&mut w, i, &mut t));
        w.brand = 0;
        w.density = 1;
        assert_ne!(original, build(&mut w, i, &mut t));
        w.density = 0;
        t.set_type_scale(1.5);
        assert_ne!(original, build(&mut w, i, &mut t));
    }

    #[test]
    fn extended_stories_render_within_narrow_preview_and_only_expose_supported_sections() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        for (i, entry) in entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| extended::supports(e.renderer()))
        {
            let mut sections = vec![1];
            if entry.supports_story("states") {
                sections.push(2);
            }
            if entry.supports_story("behaviors") {
                sections.push(3);
            }
            if entry.supports_story("variants") {
                sections.push(5);
            }
            for section in sections {
                w.state(i).section = section;
                let scene = build_in(&mut w, i, &mut text, Rect::new(0., 0., 360., 250.));
                assert!(!scene.is_empty(), "{} section {section}", entry.renderer());
                let preview = w.preview_rect(i);
                for (id, rect) in &w.state(i).hits {
                    if *id >= 10 {
                        assert!(
                            rect.min_x() >= preview.min_x() - 0.1
                                && rect.max_x() <= preview.max_x() + 0.1
                                && rect.min_y() >= preview.min_y() - 0.1
                                && rect.max_y() <= preview.max_y() + 0.1,
                            "{} hit {id} escaped preview: {rect:?} vs {preview:?}",
                            entry.renderer()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn extended_keyboard_events_are_focus_scoped_and_update_models() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let slider = index("slider");
        build(&mut w, slider, &mut text);
        let initial = w.state(slider).extended.slider.value();
        w.state(slider).focus = None;
        w.handle(slider, &key(124, true), &mut text);
        assert_eq!(w.state(slider).extended.slider.value(), initial);
        let tab_count = w.state(slider).hits.len() + 1;
        for _ in 0..tab_count {
            if w.state(slider).focus == Some(20) {
                break;
            }
            w.handle(slider, &key(48, true), &mut text);
        }
        assert_eq!(w.state(slider).focus, Some(20));
        w.handle(slider, &key(124, true), &mut text);
        assert!(w.state(slider).extended.slider.value() > initial);

        let radio = index("radio-group");
        click(&mut w, radio, 31, &mut text);
        assert_eq!(w.state(radio).extended.radio.selected, Some(1));
        w.handle(radio, &key(125, true), &mut text);
        assert_eq!(w.state(radio).extended.radio.selected, Some(2));

        let switch = index("switch");
        build(&mut w, switch, &mut text);
        let tab_count = w.state(switch).hits.len() + 1;
        for _ in 0..tab_count {
            if w.state(switch).focus == Some(20) {
                break;
            }
            w.handle(switch, &key(48, true), &mut text);
        }
        assert_eq!(w.state(switch).focus, Some(20));
        let checked = w.state(switch).extended.switch.checked;
        w.handle(switch, &key(49, true), &mut text);
        w.handle(switch, &key(49, false), &mut text);
        assert_ne!(w.state(switch).extended.switch.checked, checked);

        let table = index("data-table");
        build(&mut w, table, &mut text);
        click(&mut w, table, 20, &mut text);
        assert_eq!(w.state(table).extended.visible_rows.first(), Some(&3));
    }

    #[test]
    fn switch_tab_focus_uses_production_focus_visible_painting() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("switch");
        let before = build(&mut w, i, &mut text);
        for _ in 0..10 {
            if w.state(i).focus == Some(20) {
                break;
            }
            w.handle(i, &key(48, true), &mut text);
        }
        assert_eq!(w.state(i).focus, Some(20));
        assert!(w.state(i).buttons[&20].state.focus_visible);
        let focused = build(&mut w, i, &mut text);
        assert!(
            focused.len() > before.len(),
            "focus ring should add real scene geometry"
        );
    }

    #[test]
    fn selection_tab_exit_and_pointer_focus_move_cancel_arms_without_a_render() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let switch = index("switch");
        build(&mut w, switch, &mut text);
        let tab_count = w.state(switch).hits.len() + 1;
        for _ in 0..tab_count {
            w.handle(switch, &key(48, true), &mut text);
        }
        assert!(w.state(switch).hits.iter().any(|(id, _)| *id < 20));

        // Radio Group exposes exactly one roving tab stop while retaining the
        // page's other controls and header actions in the traversal.
        let radio = index("radio-group");
        build(&mut w, radio, &mut text);
        let radio_ids: Vec<_> = w.state(radio).hits.iter().map(|(id, _)| *id).collect();
        let tab_count = radio_ids.len() + 1;
        for _ in 0..tab_count {
            w.handle(radio, &key(48, true), &mut text);
            let focus = w.state(radio).focus.unwrap();
            assert!(
                radio_ids.contains(&focus),
                "Tab focused non-entry id {focus}"
            );
        }
        assert!(
            radio_ids.iter().any(|id| *id < 30),
            "header actions remain tabbable"
        );

        // Focus the switch through the real host Tab path, arm Space, then
        // leave it before another frame is built. The old key-up must not act.
        for _ in 0..tab_count {
            if w.state(switch).focus == Some(20) {
                break;
            }
            w.handle(switch, &key(48, true), &mut text);
        }
        assert_eq!(w.state(switch).focus, Some(20));
        let checked = w.state(switch).extended.switch.checked;
        w.handle(switch, &key(49, true), &mut text);
        assert!(w.state(switch).extended.switch_view.is_focused());
        w.handle(switch, &key(48, true), &mut text);
        assert!(!w.state(switch).extended.switch_view.is_focused());
        w.handle(switch, &key(49, false), &mut text);
        assert_eq!(w.state(switch).extended.switch.checked, checked);

        // A pointer focus move to another workbench item cancels the same arm.
        for _ in 0..tab_count {
            if w.state(switch).focus == Some(20) {
                break;
            }
            w.handle(switch, &key(48, true), &mut text);
        }
        assert_eq!(w.state(switch).focus, Some(20));
        w.handle(switch, &key(49, true), &mut text);
        let other = w
            .state(switch)
            .hits
            .iter()
            .find(|(id, _)| *id != 20)
            .unwrap()
            .1
            .center();
        w.handle(switch, &mouse(other, true), &mut text);
        assert!(!w.state(switch).extended.switch_view.is_focused());
        w.handle(switch, &key(49, false), &mut text);
        assert_eq!(w.state(switch).extended.switch.checked, checked);
    }

    #[test]
    fn disabled_selection_control_click_does_not_create_generic_focus_or_press() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let switch = index("switch");
        w.state(switch).extended.disabled = true;
        w.state(switch).focus = Some(1);
        let checked = w.state(switch).extended.switch.checked;
        build(&mut w, switch, &mut text);
        let point = w.state(switch).visible_for_id(20).unwrap().center();
        w.handle(switch, &mouse(point, true), &mut text);
        assert_eq!(w.state(switch).focus, Some(1));
        assert_eq!(w.state(switch).extended.switch.checked, checked);
        assert!(
            !w.state(switch)
                .buttons
                .get(&20)
                .is_some_and(|button| button.state.pressed)
        );
    }

    #[test]
    fn select_story_uses_its_owned_popup_keyboard_runtime() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("select");
        click(&mut w, i, 20, &mut text);
        assert!(w.state(i).select.menu.open);
        assert!(w.state(i).select_view.layout().popup.is_some());
        assert_eq!(w.state(i).focus, Some(20));

        w.handle(i, &key(125, true), &mut text);
        w.handle(i, &key(36, true), &mut text);
        assert!(!w.state(i).select.menu.open);
        assert_eq!(w.state(i).select.selected, 1);
        assert!(w.state(i).log.iter().any(|event| event == "select: 1"));

        click(&mut w, i, 20, &mut text);
        w.handle(i, &key(53, true), &mut text);
        assert!(!w.state(i).select.menu.open);
        assert_eq!(w.state(i).focus, Some(20));
    }

    #[test]
    fn p0_gallery_controls_route_real_pointer_actions_through_owned_views() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();

        let field = index("form");
        build(&mut w, field, &mut text);
        let field_layout = w.state(field).field_view.layout();
        assert_eq!(
            w.state(field).input.visible_frame(),
            Some(field_layout.slot)
        );
        assert_eq!(field_layout.frame.size.width, 600.0);
        assert_eq!(field_layout.slot.min_y(), field_layout.label.max_y() + 4.0);
        assert_eq!(field_layout.slot.size.height, 36.0);
        assert_eq!(
            field_layout.message.min_y(),
            field_layout.slot.max_y() + 6.0
        );
        assert_eq!(field_layout.message.size.height, 36.0);
        assert_eq!(
            w.state(field).visible_for_id(25).unwrap().min_y(),
            field_layout.message.max_y() + 2.0,
            "notification action follows the field message with a two point gap"
        );

        let toolbar = index("toolbar");
        click(&mut w, toolbar, 21, &mut text);
        assert!(
            w.state(toolbar)
                .log
                .iter()
                .any(|event| event == "toolbar: rename")
        );
        assert_eq!(
            w.state(toolbar).toolbar_view.layouts().len(),
            3,
            "the runtime owns all three equal-width action frames"
        );

        let number = index("number-field");
        let before = w.state(number).number.value();
        click(&mut w, number, 23, &mut text);
        assert_eq!(w.state(number).number.value(), before - 1.0);
        assert!(w.state(number).log.iter().any(|event| event == "value: 7"));
        w.handle(number, &key(124, true), &mut text);
        assert_eq!(w.state(number).number.value(), before);
        w.handle(number, &key(48, true), &mut text);
        assert_eq!(w.state(number).focus, Some(24));
        w.handle(number, &key(36, true), &mut text);
        assert_eq!(w.state(number).number.value(), before + 1.0);
        w.handle(number, &key(49, true), &mut text);
        w.handle(number, &key(49, false), &mut text);
        assert_eq!(w.state(number).number.value(), before + 2.0);
    }

    #[test]
    fn disclosure_segmented_and_breadcrumb_pointer_activation_needs_a_matching_press() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let outside = Point::new(1400.0, 900.0);

        let disclosure = index("disclosure");
        build(&mut w, disclosure, &mut text);
        let row = w.state(disclosure).visible_for_id(20).unwrap();
        let point = row.center();
        assert!(!w.state(disclosure).extended.disclosure.open);
        w.handle(disclosure, &move_pointer(point), &mut text);
        w.handle(disclosure, &mouse(point, false), &mut text);
        assert!(!w.state(disclosure).extended.disclosure.open);
        assert!(
            w.state(disclosure).log.is_empty(),
            "move→up cannot activate"
        );
        w.handle(disclosure, &mouse(outside, true), &mut text);
        w.handle(disclosure, &mouse(point, false), &mut text);
        assert!(!w.state(disclosure).extended.disclosure.open);
        assert!(
            w.state(disclosure).log.is_empty(),
            "outside down cannot arm the row"
        );
        w.handle(disclosure, &mouse(point, true), &mut text);
        assert!(
            !w.state(disclosure).extended.disclosure.open,
            "activation waits for up"
        );
        w.handle(disclosure, &move_pointer(outside), &mut text);
        w.handle(disclosure, &mouse(outside, false), &mut text);
        assert!(!w.state(disclosure).extended.disclosure.open);
        w.handle(disclosure, &mouse(point, true), &mut text);
        w.handle(disclosure, &mouse(point, false), &mut text);
        assert!(w.state(disclosure).extended.disclosure.open);
        assert_eq!(
            w.state(disclosure)
                .log
                .iter()
                .filter(|event| *event == "disclosure: true")
                .count(),
            1
        );

        let segmented = index("segmented-control");
        build(&mut w, segmented, &mut text);
        let item = w.state(segmented).visible_for_id(32).unwrap();
        let point = item.center();
        assert_eq!(w.state(segmented).extended.segmented.selected, 0);
        w.handle(segmented, &move_pointer(point), &mut text);
        w.handle(segmented, &mouse(point, false), &mut text);
        assert_eq!(w.state(segmented).extended.segmented.selected, 0);
        assert!(
            w.state(segmented).log.is_empty(),
            "move→up cannot select a segment"
        );
        w.handle(segmented, &mouse(outside, true), &mut text);
        w.handle(segmented, &mouse(point, false), &mut text);
        assert_eq!(w.state(segmented).extended.segmented.selected, 0);
        w.handle(segmented, &mouse(point, true), &mut text);
        assert_eq!(w.state(segmented).extended.segmented.selected, 0);
        w.handle(segmented, &move_pointer(outside), &mut text);
        w.handle(segmented, &mouse(outside, false), &mut text);
        assert_eq!(w.state(segmented).extended.segmented.selected, 0);
        w.handle(segmented, &mouse(point, true), &mut text);
        w.handle(segmented, &mouse(point, false), &mut text);
        assert_eq!(w.state(segmented).extended.segmented.selected, 2);
        assert_eq!(
            w.state(segmented)
                .log
                .iter()
                .filter(|event| *event == "segment: 2")
                .count(),
            1,
            "one matched press/release changes the model and logs exactly once"
        );

        let breadcrumb = index("breadcrumb");
        build(&mut w, breadcrumb, &mut text);
        let item = w.state(breadcrumb).visible_for_id(31).unwrap();
        let point = item.center();
        assert_eq!(w.state(breadcrumb).extended.selected, 0);
        w.handle(breadcrumb, &move_pointer(point), &mut text);
        w.handle(breadcrumb, &mouse(point, false), &mut text);
        assert_eq!(w.state(breadcrumb).extended.selected, 0);
        assert!(
            w.state(breadcrumb).log.is_empty(),
            "move→up cannot navigate"
        );
        w.handle(breadcrumb, &mouse(outside, true), &mut text);
        w.handle(breadcrumb, &mouse(point, false), &mut text);
        assert_eq!(w.state(breadcrumb).extended.selected, 0);
        w.handle(breadcrumb, &mouse(point, true), &mut text);
        assert_eq!(w.state(breadcrumb).extended.selected, 0);
        w.handle(breadcrumb, &move_pointer(outside), &mut text);
        w.handle(breadcrumb, &mouse(outside, false), &mut text);
        assert_eq!(w.state(breadcrumb).extended.selected, 0);
        w.handle(breadcrumb, &mouse(point, true), &mut text);
        w.handle(breadcrumb, &mouse(point, false), &mut text);
        assert_eq!(w.state(breadcrumb).extended.selected, 1);
        assert_eq!(
            w.state(breadcrumb)
                .log
                .iter()
                .filter(|event| *event == "breadcrumb: 1")
                .count(),
            1,
            "one matched press/release navigates and logs exactly once"
        );
    }

    #[test]
    fn panel_and_skeleton_stories_use_component_owned_visible_layouts() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let panel = index("surface");
        build(&mut w, panel, &mut text);
        let panel_layout = w.state(panel).surface_view.layout();
        assert_eq!(panel_layout.visible, Some(panel_layout.frame));
        assert!(panel_layout.content.min_x() > panel_layout.frame.min_x());
        assert!(panel_layout.content.min_y() > panel_layout.frame.min_y());
        let nested_action = w.state(panel).visible_for_id(20).unwrap();
        assert_eq!(nested_action.min_x(), panel_layout.frame.min_x() + 16.0);
        assert_eq!(nested_action.min_y(), panel_layout.frame.min_y() + 76.0);

        let skeleton = index("skeleton");
        build(&mut w, skeleton, &mut text);
        let layout = w.state(skeleton).skeleton_view.layout();
        assert!(layout.visible.is_some());
        assert!(layout.frame.size.width > 0.0 && layout.frame.size.height == 18.0);
    }

    #[test]
    fn gallery_panel_uses_actual_backing_scale_without_changing_panel_geometry() {
        let panel = index("surface");
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let mut layouts = Vec::new();
        for scale in [1.0, 2.0] {
            text.set_scale(scale);
            let scene = build(&mut workbench, panel, &mut text);
            let layout = workbench.state(panel).surface_view.layout();
            let panel_quad = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    tekes_ui_scene::Primitive::Quad(q) if q.rect == layout.frame => Some(q),
                    _ => None,
                })
                .expect("panel surface quad");
            assert_eq!(
                panel_quad.border_width,
                paint::decorative_border(tekes_ui_tokens::Theme::LIGHT, scale).0
            );
            assert_eq!(
                panel_quad.fill,
                paint::color(tekes_ui_tokens::Theme::LIGHT, "surface-panel")
            );
            assert_eq!(
                workbench.state(panel).visible_for_id(20).unwrap().min_x(),
                layout.frame.min_x() + 16.0
            );
            layouts.push(layout);
        }
        assert_eq!(layouts[0], layouts[1]);
    }

    #[test]
    fn disabled_selection_controls_are_skipped_by_real_tab_traversal() {
        for renderer in ["slider", "radio-group", "switch"] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            let story = index(renderer);
            click(&mut w, story, 10, &mut text);
            assert!(w.state(story).extended.disabled);
            let tab_count = w.state(story).hits.len() + 1;
            for _ in 0..tab_count {
                w.handle(story, &key(48, true), &mut text);
                let focus = w.state(story).focus;
                match renderer {
                    "slider" | "switch" => {
                        assert_ne!(focus, Some(20), "{renderer} entered while disabled")
                    }
                    "radio-group" => assert!(
                        !focus.is_some_and(|id| (30..33).contains(&id)),
                        "disabled radio row entered"
                    ),
                    _ => unreachable!(),
                }
            }
            let state = w.state(story);
            match renderer {
                "slider" => assert!(!state.extended.slider_view.is_focused()),
                "switch" => assert!(!state.extended.switch_view.is_focused()),
                "radio-group" => assert_eq!(state.extended.radio_view.focused(), None),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn slider_pointer_down_at_current_value_is_consumed_without_a_change_log() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let story = index("slider");
        build(&mut w, story, &mut text);
        let frame = w.state(story).visible_for_id(20).unwrap();
        let point = frame.center();
        let value = w.state(story).extended.slider.value();

        w.handle(story, &mouse(point, true), &mut text);

        let state = w.state(story);
        assert_eq!(state.extended.slider.value(), value);
        assert!(state.extended.slider_view.is_captured());
        assert!(
            state.log.is_empty(),
            "a no-op press must not log a value change"
        );
    }

    #[test]
    fn extended_overlay_timer_and_reset_follow_deterministic_fixtures() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let popover = index("popover");
        click(&mut w, popover, 20, &mut text);
        assert!(w.state(popover).extended.popover.open);
        w.handle(popover, &key(53, true), &mut text);
        assert!(!w.state(popover).extended.popover.open);
        assert_eq!(w.state(popover).focus, Some(20));

        let switch = index("switch");
        click(&mut w, switch, 10, &mut text);
        assert!(w.state(switch).extended.disabled);
        click(&mut w, switch, 0, &mut text);
        assert!(!w.state(switch).extended.disabled);
        let before = w.state(switch).extended.switch.checked;
        click(&mut w, switch, 20, &mut text);
        assert_ne!(w.state(switch).extended.switch.checked, before);

        let toast = index("toast");
        click(&mut w, toast, 21, &mut text);
        assert!(w.state(toast).extended.toast.remaining_ms() <= 4000);
        click(&mut w, toast, 20, &mut text);
        assert!(!w.state(toast).extended.toast.visible);

        let empty = index("empty-state");
        click(&mut w, empty, 20, &mut text);
        assert!(w.state(empty).extended.empty_state_created);
        assert!(
            w.state(empty)
                .log
                .back()
                .unwrap()
                .contains("created Documents")
        );
    }

    #[test]
    fn feedback_component_buttons_require_down_and_cancel_outside() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let outside = |p: Point| Point::new(p.x - 500.0, p.y - 500.0);

        let toast = index("toast");
        click(&mut w, toast, 21, &mut text);
        build(&mut w, toast, &mut text);
        let dismiss = w.state(toast).visible_for_id(20).unwrap().center();
        w.handle(toast, &move_pointer(dismiss), &mut text);
        w.handle(toast, &mouse(dismiss, false), &mut text);
        assert!(
            w.state(toast).extended.toast.visible,
            "Move→Up must not dismiss"
        );
        w.handle(toast, &mouse(dismiss, true), &mut text);
        w.handle(toast, &mouse(dismiss, false), &mut text);
        assert!(!w.state(toast).extended.toast.visible, "Down→Up dismisses");
        click(&mut w, toast, 21, &mut text);
        build(&mut w, toast, &mut text);
        let dismiss = w.state(toast).visible_for_id(20).unwrap().center();
        w.handle(toast, &mouse(dismiss, true), &mut text);
        w.handle(toast, &mouse(outside(dismiss), false), &mut text);
        assert!(
            w.state(toast).extended.toast.visible,
            "Down→Outside→Up cancels"
        );

        let banner = index("banner");
        build(&mut w, banner, &mut text);
        let dismiss = w.state(banner).visible_for_id(20).unwrap().center();
        w.handle(banner, &move_pointer(dismiss), &mut text);
        w.handle(banner, &mouse(dismiss, false), &mut text);
        assert!(
            w.state(banner).extended.banner.visible,
            "Move→Up must not dismiss"
        );
        w.handle(banner, &mouse(dismiss, true), &mut text);
        w.handle(banner, &mouse(dismiss, false), &mut text);
        assert!(
            !w.state(banner).extended.banner.visible,
            "Down→Up dismisses"
        );
        click(&mut w, banner, 0, &mut text);
        build(&mut w, banner, &mut text);
        let dismiss = w.state(banner).visible_for_id(20).unwrap().center();
        w.handle(banner, &mouse(dismiss, true), &mut text);
        w.handle(banner, &mouse(outside(dismiss), false), &mut text);
        assert!(
            w.state(banner).extended.banner.visible,
            "Down→Outside→Up cancels"
        );

        let empty = index("empty-state");
        build(&mut w, empty, &mut text);
        let action = w.state(empty).visible_for_id(20).unwrap().center();
        w.handle(empty, &move_pointer(action), &mut text);
        w.handle(empty, &mouse(action, false), &mut text);
        assert!(
            !w.state(empty).extended.empty_state_created,
            "Move→Up must not activate"
        );
        w.handle(empty, &mouse(action, true), &mut text);
        w.handle(empty, &mouse(action, false), &mut text);
        assert!(
            w.state(empty).extended.empty_state_created,
            "Down→Up activates"
        );
        click(&mut w, empty, 0, &mut text);
        build(&mut w, empty, &mut text);
        let action = w.state(empty).visible_for_id(20).unwrap().center();
        w.handle(empty, &mouse(action, true), &mut text);
        w.handle(empty, &mouse(outside(action), false), &mut text);
        assert!(
            !w.state(empty).extended.empty_state_created,
            "Down→Outside→Up cancels"
        );
    }

    #[test]
    fn popover_pointer_arms_are_component_owned_and_release_cleans_up() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let popover = index("popover");
        click(&mut w, popover, 20, &mut text);
        assert!(w.state(popover).extended.popover.open);
        assert!(!w.state(popover).buttons[&20].state.pressed);

        build(&mut w, popover, &mut text);
        let dismiss = w.state(popover).visible_for_id(21).unwrap().center();
        let outside = Point::new(dismiss.x + 600.0, dismiss.y + 400.0);
        w.handle(popover, &mouse(outside, true), &mut text);
        w.handle(popover, &mouse(dismiss, false), &mut text);
        assert!(
            w.state(popover).extended.popover.open,
            "outside down→inside up cancels"
        );

        click(&mut w, popover, 20, &mut text);
        assert!(
            !w.state(popover).extended.popover.open,
            "the invoker dismisses an open popover"
        );
        assert!(
            !w.state(popover).buttons[&20].state.pressed,
            "component dismissal clears host arm"
        );

        click(&mut w, popover, 20, &mut text);
        build(&mut w, popover, &mut text);
        w.handle(popover, &mouse(outside, true), &mut text);
        w.handle(popover, &mouse(outside, false), &mut text);
        assert!(
            !w.state(popover).extended.popover.open,
            "outside down→up dismisses"
        );
        assert!(!w.state(popover).buttons[&20].state.pressed);
    }

    #[test]
    fn combobox_native_text_command_moves_the_real_input_caret() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("combo-box");
        click(&mut w, i, 1000, &mut text);
        w.handle(
            i,
            &Event::TextCommit {
                text: "Auto".into(),
                replacement: None,
            },
            &mut text,
        );
        build(&mut w, i, &mut text);
        assert_eq!(w.state(i).extended.combo.query, "Auto");
        let before = w.text_state(i).unwrap().caret_index;
        assert!(
            !w.handle(i, &key(123, true), &mut text),
            "Left must remain available to the platform text-input path"
        );
        w.handle(i, &Event::TextCommand("moveLeft:".into()), &mut text);
        let after = w.text_state(i).unwrap().caret_index;
        assert!(
            after < before,
            "native text editing commands should move the real input caret"
        );
        assert_eq!(w.state(i).extended.combo.query, "Auto");
    }

    #[test]
    fn combobox_closed_state_hides_option_hits_and_input_reopens_candidates() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("combo-box");
        build(&mut w, i, &mut text);
        assert!(!w.state(i).extended.combo.open);
        assert!(w.state(i).hits.iter().all(|(id, _)| !(40..43).contains(id)));
        assert!(w.state(i).hits.iter().any(|(id, _)| *id == 1000));

        click(&mut w, i, 1000, &mut text);
        assert!(w.state(i).extended.combo.open);
        assert!(w.state(i).hits.iter().any(|(id, _)| *id == 40));

        w.handle(
            i,
            &Event::TextCommit {
                text: "Com".into(),
                replacement: None,
            },
            &mut text,
        );
        build(&mut w, i, &mut text);
        assert!(w.state(i).extended.combo.open);
        assert!(w.state(i).hits.iter().any(|(id, _)| *id == 41));
        assert!(w.state(i).hits.iter().any(|(id, _)| *id == 42));

        w.handle(i, &Event::TextCommand("insertNewline:".into()), &mut text);
        build(&mut w, i, &mut text);
        assert!(!w.state(i).extended.combo.open);
        assert!(w.state(i).hits.iter().all(|(id, _)| !(40..43).contains(id)));

        click(&mut w, i, 1000, &mut text);
        assert!(w.state(i).extended.combo.open);
        assert!(w.state(i).hits.iter().any(|(id, _)| *id == 41));
    }

    #[test]
    fn combobox_pointer_selection_closes_list_and_keeps_input_focused() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("combo-box");
        click(&mut w, i, 1000, &mut text);
        w.handle(
            i,
            &Event::TextCommit {
                text: "Com".into(),
                replacement: None,
            },
            &mut text,
        );
        build(&mut w, i, &mut text);
        assert!(w.state(i).hits.iter().any(|(id, _)| *id == 42));

        click(&mut w, i, 42, &mut text);
        assert_eq!(w.state(i).input.query(), "Comfortable");
        assert!(!w.state(i).extended.combo.open);
        assert_eq!(w.state(i).focus, Some(1000));
        assert_eq!(w.text_state(i).unwrap().text, "Comfortable");
        assert!(w.state(i).hits.iter().all(|(id, _)| !(40..43).contains(id)));
    }

    #[test]
    fn reduced_motion_is_applied_before_first_popup_creation_and_wheel_endpoint() {
        fn wheel(point: Point, timestamp: f64, ended: bool) -> Event {
            Event::Scroll(tekes_ui_platform_macos::ScrollEvent {
                timestamp,
                delta: tekes_ui_platform_macos::Vec2 {
                    x: 0.0,
                    y: if ended { 0.0 } else { 500.0 },
                },
                precise: true,
                phase: if ended {
                    tekes_ui_platform_macos::ScrollPhase::Ended
                } else {
                    tekes_ui_platform_macos::ScrollPhase::Began
                },
                momentum_phase: tekes_ui_platform_macos::ScrollPhase::None,
                inverted: false,
                position: NativePoint {
                    x: point.x as f64,
                    y: point.y as f64,
                },
                modifiers: Modifiers::NONE,
            })
        }

        let mut text = super::super::text::TextEngine::new();
        let mut workbench = Workbench::default();
        for renderer in ["menu", "select"] {
            let i = index(renderer);
            workbench.apply_reduced_motion(true);
            click(&mut workbench, i, 20, &mut text);
            let layout = if renderer == "menu" {
                assert!(workbench.state(i).menu_view.is_open());
                workbench.state(i).menu_layout.as_ref().unwrap().clone()
            } else {
                assert!(workbench.state(i).select.menu.open);
                workbench
                    .state(i)
                    .select_menu_layout
                    .as_ref()
                    .unwrap()
                    .clone()
            };
            let position = layout.popup.center();
            assert!(workbench.handle(i, &wheel(position, 1.0, false), &mut text));
            assert!(workbench.handle(i, &wheel(position, 1.01, true), &mut text));
            let needs_animation = if renderer == "menu" {
                workbench.state(i).menu_view.needs_animation()
            } else {
                workbench.state(i).select_view.needs_animation()
            };
            assert!(
                !needs_animation,
                "{renderer} scheduled popup animation with reduced motion"
            );
        }

        let combo = index("combo-box");
        workbench.apply_reduced_motion(true);
        build(&mut workbench, combo, &mut text);
        workbench.state(combo).extended.combo.items =
            (0..24).map(|n| format!("Com candidate {n:02}")).collect();
        click(&mut workbench, combo, 1000, &mut text);
        workbench.handle(
            combo,
            &Event::TextCommit {
                text: "Com".into(),
                replacement: None,
            },
            &mut text,
        );
        build(&mut workbench, combo, &mut text);
        assert!(workbench.state(combo).extended.combo.open);
        let popup = workbench
            .state(combo)
            .extended
            .combo_view
            .options
            .layout()
            .expect("open ComboBox options are laid out")
            .clone();
        assert_eq!(popup.items.len(), 24);
        assert!(popup.popup.size.height < 24.0 * 36.0);
        let position = popup.popup.center();
        assert!(workbench.handle(combo, &wheel(position, 2.0, false), &mut text));
        assert!(workbench.handle(combo, &wheel(position, 2.01, true), &mut text));
        build(&mut workbench, combo, &mut text);
        assert!(
            !workbench
                .state(combo)
                .extended
                .combo_view
                .options
                .needs_animation()
        );
        assert_eq!(
            workbench
                .state(combo)
                .extended
                .combo_view
                .options
                .layout()
                .unwrap()
                .scroll_offset,
            0.0
        );
        workbench.apply_reduced_motion(false);
        assert!(workbench.handle(combo, &wheel(position, 2.02, false), &mut text));
        assert!(workbench.handle(combo, &wheel(position, 2.03, true), &mut text));
        build(&mut workbench, combo, &mut text);
        assert!(
            workbench
                .state(combo)
                .extended
                .combo_view
                .options
                .needs_animation()
        );
        assert_ne!(
            workbench
                .state(combo)
                .extended
                .combo_view
                .options
                .layout()
                .unwrap()
                .scroll_offset,
            0.0
        );

        let command = index("command-palette");
        workbench.apply_reduced_motion(true);
        build(&mut workbench, command, &mut text);
        workbench.state(command).extended.command.combo.items = (0..24)
            .map(|n| format!("Command candidate {n:02}"))
            .collect();
        click(&mut workbench, command, 20, &mut text);
        assert!(workbench.state(command).extended.command.combo.open);
        let popup = workbench
            .state(command)
            .extended
            .command_view
            .options
            .layout()
            .expect("open CommandPalette options are laid out")
            .clone();
        assert_eq!(popup.items.len(), 24);
        assert!(popup.popup.size.height < 24.0 * 36.0);
        let position = popup.popup.center();
        assert!(workbench.handle(command, &wheel(position, 3.0, false), &mut text));
        assert!(workbench.handle(command, &wheel(position, 3.01, true), &mut text));
        build(&mut workbench, command, &mut text);
        assert!(
            !workbench
                .state(command)
                .extended
                .command_view
                .options
                .needs_animation()
        );
        assert_eq!(
            workbench
                .state(command)
                .extended
                .command_view
                .options
                .layout()
                .unwrap()
                .scroll_offset,
            0.0
        );
        workbench.apply_reduced_motion(false);
        assert!(workbench.handle(command, &wheel(position, 3.02, false), &mut text));
        assert!(workbench.handle(command, &wheel(position, 3.03, true), &mut text));
        build(&mut workbench, command, &mut text);
        assert!(
            workbench
                .state(command)
                .extended
                .command_view
                .options
                .needs_animation()
        );
        assert_ne!(
            workbench
                .state(command)
                .extended
                .command_view
                .options
                .layout()
                .unwrap()
                .scroll_offset,
            0.0
        );
    }
}
