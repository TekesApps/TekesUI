//! Registry-driven native workbench. Fixtures orchestrate production control models;
//! no duplicate component implementation lives in a story.
mod accessibility;
mod basic_states;
mod brand_preview;
#[cfg(any(test, feature = "composition-review"))]
pub(super) mod content_status;
#[cfg(test)]
mod draft_capture;
mod durable_draft;
mod extended;
mod focus;
#[cfg(test)]
mod state_capture;
use super::{Page, PageCtx, text_demo::TextDemo};
use brand_preview::BrandPreview;
use focus::FocusTarget;
use std::{
    collections::{HashMap, VecDeque},
    sync::OnceLock,
    time::Duration,
};
use tekes_ui_controls::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, CheckState, Checkbox, ControlKey,
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
                (!self.planned()
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
                    .contains(&self.renderer()))
                    || (!self.planned() && basic_states::supports(self.renderer()))
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
            "edge-cases" => {
                !self.planned()
                    && [
                        "button", "checkbox", "field", "area", "search", "form", "settings",
                        "async",
                    ]
                    .contains(&self.renderer())
            }
            "behaviors" => {
                if self.renderer() == "scene3d" {
                    false
                } else if extended::supports(self.renderer()) {
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
    scene3d_reset: bool,
    draft_fixture: Option<durable_draft::DurableDraft>,
    draft_error: Option<String>,
    draft_attempt: Option<(u64, u64)>,
    focus: Option<usize>,
    pub section: usize,
    brand: usize,
    density: usize,
    disabled: bool,
    loading: bool,
    error: bool,
    select_empty: bool,
    hits: Vec<(usize, Rect)>,
    logical_focus: Vec<FocusTarget>,
    button_hits: Vec<usize>,
    button_clip: Option<Rect>,
    tabs_layout: Option<TabsLayout>,
    menu_layout: Option<MenuLayout>,
    select_menu_layout: Option<MenuLayout>,
    dialog_layout: Option<tekes_ui_controls::DialogLayout>,
    buttons: HashMap<usize, ButtonModel>,
    button_labels: HashMap<usize, String>,
    log: VecDeque<String>,
    phase_frame: Option<Rect>,
    event_log_frame: Option<Rect>,
    document_bottom: f32,
    document_origin_y: f32,
    selected: usize,
    list_scroll: tekes_ui_controls::ScrollArea,
    list_bounds: Rect,
    list_frame: Rect,
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
            scene3d_reset: false,
            draft_fixture: None,
            draft_error: None,
            draft_attempt: None,
            focus: None,
            section: 1,
            brand: 0,
            density: 0,
            disabled: false,
            loading: false,
            error: false,
            select_empty: false,
            hits: Vec::new(),
            logical_focus: Vec::new(),
            button_hits: Vec::new(),
            button_clip: None,
            tabs_layout: None,
            menu_layout: None,
            select_menu_layout: None,
            dialog_layout: None,
            buttons: HashMap::new(),
            button_labels: HashMap::new(),
            log: VecDeque::new(),
            phase_frame: None,
            event_log_frame: None,
            document_bottom: 0.,
            document_origin_y: 0.,
            selected: 0,
            list_scroll: tekes_ui_controls::ScrollArea::default(),
            list_bounds: Rect::default(),
            list_frame: Rect::default(),
            focus_reveal: None,
            preview: Rect::default(),
            window_active: true,
            pointer_began_composing: false,
            extended: extended::ExtendedStory::default(),
        }
    }
}
impl StoryState {
    fn set_window_active(&mut self, active: bool, now: Duration) {
        self.window_active = active;
        self.input.set_window_active(active);
        self.detail.set_window_active(active);
        self.tabs_view.set_window_active(active);
        let menu_event = self.menu_view.set_window_active(active);
        self.menu_event(menu_event, "menu", false);
        self.select_view.set_window_active(active, &mut self.select);
        self.dialog_view.set_window_active(active);
        self.extended.set_window_active(active);
        for button in self.buttons.values_mut() {
            button.set_window_active(active, now);
        }
        if !active {
            // A dismissed transient must not require an inactive redraw to
            // discard retained exit paint. The persistent Dialog below keeps
            // its logical scope and is intentionally not treated as a Menu.
            self.menu_view.clear_presentation();
            self.toolbar_view.focus(None, false);
            self.number_view.blur();
            self.dialog_view.deactivate();
            self.extended.cancel_pointer();
            self.list_scroll.cancel();
            self.tabs_view.cancel();
            for b in self.buttons.values_mut() {
                b.cancel();
            }
        }
    }
    fn focus_frame(&self, renderer: &str, id: usize) -> Option<Rect> {
        if renderer == "master" && (10000..10100).contains(&id) {
            // Reveal the nested list viewport through the outer page first;
            // its own ScrollArea then reveals the focused row.
            return (self.list_frame.size.width > 0.0 && self.list_frame.size.height > 0.0)
                .then_some(self.list_frame);
        }
        if let Some(target) = self
            .logical_focus
            .iter()
            .find(|target| target.id == id && target.mounted && target.eligible)
        {
            return Some(target.frame);
        }
        let collection_item = match renderer {
            "list" | "tree" => id >= 30,
            "data-table" => (20..23).contains(&id) || id >= 30,
            "chart" => id >= 30,
            _ => false,
        };
        if collection_item {
            return self.extended.collection_focus_frame(renderer, id);
        }
        let frame = match renderer {
            "tabs" if (30..33).contains(&id) => {
                self.tabs_layout.as_ref()?.items.get(id - 30)?.frame
            }
            "menu" if (50..53).contains(&id) => {
                self.menu_layout.as_ref()?.items.get(id - 50)?.frame
            }
            "select" if id == 20 => self.select_view.layout().trigger.frame,
            "select" if (60..63).contains(&id) => {
                self.select_menu_layout.as_ref()?.items.get(id - 60)?.frame
            }
            "dialog" | "danger" if (70..72).contains(&id) => {
                self.dialog_layout.as_ref()?.buttons.get(id - 70)?.frame
            }
            "checkbox" if id == 20 => self.checkbox.button.layout()?.frame,
            "field" | "area" | "search" | "form" | "settings" | "async" | "filter"
                if id == 1000 =>
            {
                self.input.frame_at(0)?
            }
            "slider" if id == 20 => self.extended.slider_view.layout().frame,
            "switch" if id == 20 => self.extended.switch_view.layout().frame(),
            "radio-group" if (30..33).contains(&id) => {
                self.extended.radio_view.layout().rows.get(id - 30)?.frame()
            }
            "icon-button" if id == 20 => self.extended.icon_button_view.layout().frame,
            "link" if id == 20 => self.extended.link_view.layout().frame,
            "segmented-control" if (30..33).contains(&id) => {
                self.extended
                    .segmented_view
                    .layout()
                    .items
                    .get(id - 30)?
                    .frame
            }
            "breadcrumb" if (20..23).contains(&id) => self
                .extended
                .breadcrumb_view
                .layout()
                .items
                .get(id - 20)?
                .frame(),
            "number-field" if id == 23 => self.number_view.layout().decrement.frame,
            "number-field" if id == 24 => self.number_view.layout().increment.frame,
            "combo-box" | "command-palette" if id == 1000 => self.input.frame_at(0)?,
            "combo-box" if (40..43).contains(&id) => {
                self.extended
                    .combo_view
                    .options
                    .layout()?
                    .items
                    .get(id - 40)?
                    .frame
            }
            "command-palette" if (40..43).contains(&id) => {
                self.extended
                    .command_view
                    .options
                    .layout()?
                    .items
                    .get(id - 40)?
                    .frame
            }
            _ => self.buttons.get(&id).and_then(ButtonModel::layout)?.frame,
        };
        Some(frame)
    }

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
                    .and_then(ButtonModel::layout)
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
        if e.renderer() == "form" {
            s.draft_fixture = Some(durable_draft::DurableDraft::memory());
        }
        // Unit tests inject temporary storage explicitly; never inherit a
        // developer's runtime fixture file through the process environment.
        if !cfg!(test)
            && e.renderer() == "form"
            && let Some(path) = std::env::var_os("TEKESUI_GALLERY_DRAFT_FIXTURE")
        {
            s.open_draft_fixture(path.into());
        }
        s
    }
    fn open_draft_fixture(&mut self, path: std::path::PathBuf) {
        match durable_draft::DurableDraft::open(path) {
            Ok(store) => {
                self.input.set_value(&store.draft().name);
                self.checkbox.checked = if store.draft().notifications {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                };
                self.phase = if store.has_pending() {
                    Phase::Failed
                } else if store.is_acknowledged() {
                    Phase::Saved
                } else {
                    Phase::Ready
                };
                self.draft_fixture = Some(store);
                self.draft_error = None;
                self.draft_attempt = None;
                self.record("draft fixture: restored from explicit storage; no network");
            }
            Err(error) => self.draft_failed(error),
        }
    }
    fn draft_failed(&mut self, error: std::io::Error) {
        self.draft_error = Some(format!(
            "Draft storage failed ({:?}); input retained. Retry save.",
            error.kind()
        ));
        self.phase = Phase::Failed;
        self.record("draft: storage failed; not acknowledged");
    }
    fn persist_draft(&mut self) -> bool {
        if self.input.state().is_some_and(|s| s.marked.is_some()) {
            return true;
        }
        // An explicitly requested file adapter must not silently fall back to
        // the default in-memory journal after configuration/storage failure.
        if self.draft_error.is_some()
            && self.draft_fixture.as_ref().is_some_and(|s| !s.is_durable())
        {
            return false;
        }
        let name = self.input.query().to_owned();
        let notifications = self.checkbox.checked == CheckState::Checked;
        let Some(store) = self.draft_fixture.as_mut() else {
            return self.draft_error.is_none();
        };
        match store.update(&name, notifications) {
            Ok(changed) => {
                if changed {
                    self.draft_error = None;
                }
                if changed && self.phase != Phase::Pending {
                    self.phase = Phase::Ready;
                }
                true
            }
            Err(error) => {
                self.draft_failed(error);
                false
            }
        }
    }
    fn interrupt_draft(&mut self) {
        if !self.persist_draft() {
            return;
        }
        let Some(store) = self.draft_fixture.as_ref() else {
            return;
        };
        match store.reopen() {
            Ok(restored) => {
                self.input.set_value(&restored.draft().name);
                self.checkbox.checked = if restored.draft().notifications {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                };
                self.phase = if restored.has_pending() {
                    Phase::Failed
                } else if restored.is_acknowledged() {
                    Phase::Saved
                } else {
                    Phase::Ready
                };
                self.draft_fixture = Some(restored);
                self.draft_attempt = None;
                self.record("interruption: discarded session; recovered durable draft/outbox");
            }
            Err(error) => self.draft_failed(error),
        }
    }
    fn input_changed(&mut self, renderer: &str) {
        if renderer == "combo-box" {
            self.extended.combo.set_query(self.input.query());
        } else if renderer == "command-palette" {
            self.extended.command.combo.set_query(self.input.query());
        }
        self.record("input: value changed");
    }
    fn record(&mut self, event: impl Into<String>) {
        if self.log.len() == 6 {
            self.log.pop_front();
        }
        self.log.push_back(event.into());
    }
    fn focusable_mounted(&self, id: usize) -> bool {
        self.logical_focus
            .iter()
            .any(|target| target.id == id && target.mounted && target.eligible)
    }
    fn restore_focus(&self, candidate: Option<usize>) -> Option<usize> {
        candidate.filter(|id| self.focusable_mounted(*id))
    }
    pub(super) fn register_logical_focus(
        &mut self,
        id: usize,
        frame: Rect,
        eligible: bool,
        mounted: bool,
        roving_group: Option<u8>,
        roving_selected: bool,
    ) {
        if !mounted || frame.size.width <= 0.0 || frame.size.height <= 0.0 {
            return;
        }
        if let Some(target) = self.logical_focus.iter_mut().find(|target| target.id == id) {
            target.frame = frame;
            target.eligible = eligible;
            target.mounted = mounted;
            target.roving_group = roving_group;
            target.roving_selected = roving_selected;
        } else {
            self.logical_focus.push(FocusTarget {
                id,
                frame,
                eligible,
                mounted,
                roving_group,
                roving_selected,
            });
        }
    }
    fn logical_tab_ids(&self) -> Vec<usize> {
        let mut targets = self.logical_focus.clone();
        let mut selected_groups = HashMap::<u8, usize>::new();
        for target in targets
            .iter()
            .filter(|target| target.mounted && target.eligible)
        {
            let Some(group) = target.roving_group else {
                continue;
            };
            selected_groups.entry(group).or_insert_with(|| {
                self.focus
                    .filter(|focused| {
                        targets.iter().any(|candidate| {
                            candidate.id == *focused
                                && candidate.roving_group == Some(group)
                                && candidate.mounted
                                && candidate.eligible
                        })
                    })
                    .or_else(|| {
                        targets
                            .iter()
                            .find(|candidate| {
                                candidate.roving_group == Some(group)
                                    && candidate.mounted
                                    && candidate.eligible
                                    && candidate.roving_selected
                            })
                            .map(|candidate| candidate.id)
                    })
                    .unwrap_or(target.id)
            });
        }
        targets.retain(|target| {
            target.mounted
                && target.eligible
                && target
                    .roving_group
                    .is_none_or(|group| selected_groups.get(&group).copied() == Some(target.id))
        });
        // Registration order is render/reading order. Preserve it explicitly;
        // geometry sorting breaks when rows wrap or type scale changes.
        targets.into_iter().map(|target| target.id).collect()
    }
    #[cfg(test)]
    fn logical_tab_frames(&self) -> Vec<FocusTarget> {
        self.logical_tab_ids()
            .into_iter()
            .filter_map(|id| {
                self.logical_focus
                    .iter()
                    .find(|target| target.id == id)
                    .copied()
            })
            .collect()
    }
    fn sync_owned_focus(&mut self, renderer: &str, target: Option<usize>, visible: bool) {
        if renderer == "command-palette" && target == Some(20) {
            self.extended.command_view.focus_trigger(visible);
        } else {
            self.extended.command_view.blur_trigger();
        }
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
    fn open_command_palette(&mut self, text: &mut super::text::TextEngine) {
        if !self
            .extended
            .command_view
            .open_from(&mut self.extended.command, 20)
        {
            return;
        }
        self.input.set_value("");
        self.input.focus_field();
        self.detail.blur();
        self.input.refresh(text);
        self.focus = Some(1000);
        self.extended.focused = Some(1000);
        self.extended.command_view.blur_trigger();
    }

    fn close_command_palette(&mut self) {
        self.extended
            .command_view
            .cancel(&mut self.extended.command);
        let caller = self
            .extended
            .command_view
            .take_restore_focus(&self.extended.command);
        self.input.blur();

        // Focus targets use mounted document-space geometry. A clipped trigger
        // remains a valid caller and the Gallery can reveal it after restore.
        let mounted = self
            .logical_focus
            .iter()
            .any(|target| target.id == 20 && target.mounted);
        let trigger = self
            .extended
            .command_view
            .trigger_layout()
            .map(|layout| layout.frame)
            .filter(|frame| {
                frame.min_x().is_finite()
                    && frame.min_y().is_finite()
                    && frame.size.width.is_finite()
                    && frame.size.height.is_finite()
                    && frame.size.width > 0.0
                    && frame.size.height > 0.0
            });
        if let Some(target) = self.logical_focus.iter_mut().find(|target| target.id == 20) {
            if mounted && let Some(frame) = trigger {
                target.frame = frame;
                target.eligible = !self.extended.command.combo.disabled;
                target.mounted = true;
            } else {
                target.mounted = false;
                target.eligible = false;
            }
        }
        self.focus = self.restore_focus(caller);
        self.extended.focused = self.focus;
        self.sync_owned_focus("command-palette", self.focus, true);
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
    fn dialog_title(renderer: &str) -> &'static str {
        if renderer == "danger" {
            "Delete this workspace? This fixture does not delete files."
        } else {
            "Confirm changes?"
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
        if self.phase == Phase::Pending || self.input.state().is_some_and(|s| s.marked.is_some()) {
            return;
        }
        if self.input.query().trim().is_empty() {
            self.phase = Phase::Invalid;
            self.record("validation: name is required");
        } else {
            if !self.persist_draft() {
                return;
            }
            if let Some(store) = self.draft_fixture.as_mut() {
                if store.is_acknowledged() {
                    self.phase = Phase::Saved;
                    return;
                }
                match store.begin_attempt() {
                    Ok(attempt) => {
                        self.draft_attempt = Some(attempt);
                        self.draft_error = None;
                    }
                    Err(error) => {
                        self.draft_failed(error);
                        return;
                    }
                }
            }
            self.phase = Phase::Pending;
            self.attempts += 1;
            self.record("submit: pending (Resolve fixture to complete)");
        }
    }
    fn resolve(&mut self) {
        if self.phase != Phase::Pending {
            return;
        }
        if let Some((revision, attempt)) = self.draft_attempt.take() {
            if attempt == 1 {
                self.phase = Phase::Failed;
                self.record("delivery: simulated failure; durable revision remains retryable");
            } else if let Some(store) = self.draft_fixture.as_mut() {
                match store.acknowledge(revision) {
                    Ok(true) => {
                        self.draft_error = None;
                        self.phase = if store.is_acknowledged() {
                            Phase::Saved
                        } else {
                            Phase::Ready
                        };
                        self.record(format!(
                            "delivery: acknowledged revision {revision}; newer edits retained"
                        ));
                    }
                    Ok(false) => {
                        self.phase = Phase::Failed;
                        self.record("delivery: stale acknowledgement ignored");
                    }
                    Err(error) => self.draft_failed(error),
                }
            }
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
        // Story tabs belong to the workbench host for every renderer.
        // Dispatch them before the component-specific adapter.
        if (1..=6).contains(&id) {
            self.section = id - 1;
            self.document_bottom = 0.;
            self.event_log_frame = None;
            self.input.blur();
            return;
        }
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
                let active = self.window_active;
                *self = Self::for_entry_renderer(renderer);
                self.set_window_active(active, Duration::ZERO);
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
                let active = self.window_active;
                *self = Self::for_entry_renderer(renderer);
                self.set_window_active(active, Duration::ZERO);
                self.section = section;
                self.scene3d_reset = renderer == "scene3d";
                self.record("reset: initial fixture");
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
                if renderer == "select" {
                    self.select_empty = !self.select_empty;
                } else {
                    self.checkbox.checked = CheckState::Mixed;
                }
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
                    // Use the retained trigger path: opening from the model
                    // alone resets active focus to zero instead of the current value.
                    self.select_view.focus(false);
                    let _ = self
                        .select_view
                        .key(ControlKey::Enter, true, &mut self.select);
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
            26 if renderer == "form" => self.interrupt_draft(),
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
#[derive(Debug)]
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
    window_active: bool,
}
impl Workbench {
    pub fn resize_target(&self, i: usize) -> Option<(Rect, Axis)> {
        if entries()[i].renderer() != "split-view" || !self.show_live(i) {
            return None;
        }
        let state = self.states.get(&i)?;
        state
            .extended
            .split_view
            .resize_target(&state.extended.split)
    }
}
impl Default for Workbench {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            brand_preview: BrandPreview::default(),
            preview_width: 0,
            reduced: false,
            brand: 0,
            density: 0,
            text_scale: 0,
            snapshot: false,
            motion_now: Duration::ZERO,
            window_active: true,
        }
    }
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

fn matrix_caption(ctx: &mut PageCtx<'_>, scene: &mut Scene, r: Rect, text: &str) {
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        r,
        text,
        proposal::TextRole::UiCaption,
        ColorRole::SecondaryText.resolve(ctx.theme),
    );
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MatrixCellGeometry {
    control: Rect,
    caption: Rect,
}

fn matrix_cell_geometries(
    bounds: Rect,
    columns: usize,
    explanation_height: f32,
    measurements: &[(f32, f32)],
) -> (Vec<MatrixCellGeometry>, f32) {
    let columns = columns.max(1);
    let cell_width = (bounds.size.width / columns as f32 - 16.0).max(0.0);
    let mut row_heights = vec![92.0_f32; measurements.len().div_ceil(columns)];
    for (index, (caption_height, control_height)) in measurements.iter().enumerate() {
        row_heights[index / columns] =
            row_heights[index / columns].max(control_height + 8.0 + caption_height + 12.0);
    }
    let matrix_top = bounds.min_y()
        + explanation_height.max(proposal::TextRole::UiBody.style().line_height)
        + 16.0;
    let mut row_tops = Vec::with_capacity(row_heights.len());
    let mut bottom = matrix_top;
    for height in &row_heights {
        row_tops.push(bottom);
        bottom += *height;
    }
    let cell_stride = bounds.size.width / columns as f32;
    let cells = measurements
        .iter()
        .enumerate()
        .map(|(index, (caption_height, control_height))| {
            let x = bounds.min_x() + (index % columns) as f32 * cell_stride;
            let y = row_tops[index / columns];
            let control = Rect::new(x, y, cell_width, *control_height);
            MatrixCellGeometry {
                control,
                caption: Rect::new(x, control.max_y() + 8.0, cell_width, *caption_height),
            }
        })
        .collect();
    (cells, bottom + 8.0)
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

fn api_reference(entry: &Entry) -> &str {
    if !entry.get("api").is_empty() {
        return entry.get("api");
    }
    if entry.get("kind") == "component" {
        return "Missing public API registration; registry validation failed";
    }
    // Component API references come from the authoritative JSON registry and
    // are compiled by the public integration test, never guessed per renderer.
    match entry.renderer() {
        "form" | "settings" | "async" => "Gallery pattern using TextInput, FieldView and Button",
        "filter" => "Gallery pattern using TextInput and local file filtering",
        "master" => "Gallery pattern using ScrollArea and TextSurface",
        _ => "Gallery fixture; see the route renderer",
    }
}

fn contract_summary(entry: &Entry) -> Vec<String> {
    if entry.get("kind") != "component" {
        return Vec::new();
    }
    static CAPABILITIES: OnceLock<serde_json::Value> = OnceLock::new();
    let inventory = CAPABILITIES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../design/gallery/capabilities.json"))
            .expect("current component capabilities")
    });
    let Some(component) = inventory["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|component| component["id"] == entry.component())
    else {
        return vec![format!("Missing contract coverage: {}", entry.component())];
    };
    let states = component["state_coverage"]["required"].as_array().unwrap();
    let pending = component["state_coverage"]["fixture_unverified"]
        .as_array()
        .unwrap()
        .len();
    let names = states
        .iter()
        .map(|state| state.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(", ");
    let references = component["token_dependencies"]["contract_references"]
        .as_array()
        .unwrap();
    let mut token_line = if references.is_empty() {
        "No per-component token references in the existing JSON contract; runtime token dependencies remain unverified.".to_owned()
    } else {
        let names = references
            .iter()
            .map(|reference| {
                format!(
                    "{}:{}",
                    reference["family"].as_str().unwrap(),
                    reference["token"].as_str().unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let owner = component["token_dependencies"]["contract_owner"]
            .as_str()
            .unwrap();
        let inherited = if owner == entry.component() {
            String::new()
        } else {
            format!(" (shared with {owner})")
        };
        format!(
            "JSON contract token references{inherited}: {names}. Runtime token dependencies remain unverified."
        )
    };
    if let Some(observed) = component["token_dependencies"].get("runtime_observation") {
        let count = observed["case_count"].as_u64().unwrap();
        let tokens = observed["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|token| token.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(", ");
        token_line.push_str(&format!(
            " Observed public-API lookup scope ({count} cases, incomplete): {tokens}."
        ));
    }
    vec![
        format!("Design-required states: {names}."),
        format!(
            "State coverage: {}/{} fixtures source-reviewed; runtime and native acceptance remain unverified.",
            states.len() - pending,
            states.len()
        ),
        token_line,
    ]
}

fn behavior_copy(renderer: &str) -> &'static str {
    match renderer {
        "button" => {
            "Tab focuses the action. Space activates on release; Enter activates on key down. Releasing the pointer outside cancels. Disabled and loading actions do not activate."
        }
        "checkbox" => {
            "Tab focuses the checkbox. Space toggles on release; Enter does not toggle it. Pointer activation needs matching press and release. Disabled state blocks changes; Mixed is a fixture state."
        }
        "tabs" => {
            "Tab enters the tab strip. Left/Right wrap selection. Home/End select endpoints. The panel follows the active tab."
        }
        "menu" | "select" => {
            "Enter or Space opens. Up/Down move through options. Enter selects. Escape dismisses and restores focus."
        }
        "dialog" | "danger" => {
            "Enter opens. Tab/Shift+Tab stay inside the modal scope. Escape cancels. Closing restores focus to the invoker. The destructive fixture changes local state only."
        }
        "field" | "search" => {
            "Click or Tab to focus the single-line input. Native text services handle IME composition, selection, clipboard, undo and redo. Tab leaves the field. Disabled input rejects edits."
        }
        "area" => {
            "Click or Tab to focus the multiline editor. Native text services handle IME composition, selection, clipboard and undo. The editor fills the remaining preview; its document scrolls inside that frame."
        }
        "text" => {
            "This surface is read-only. Pointer and keyboard selection, Copy and scrolling remain available; editing commands do not change the text. Appending fixture text preserves a reading anchor; Return to latest follows the document end."
        }
        "scroll" => {
            "Wheel and trackpad input scroll the read-only text document inside its viewport. Resize preserves the reading anchor and an existing bottom anchor. Append paragraph and Return to latest are separate fixture actions."
        }
        "icon" => {
            "Search filters the local icon catalog; category and grid controls update the visible fixture. The preview renders canonical icon assets. Individual decorative glyphs do not activate button actions."
        }
        "surface" => {
            "Surface/Panel provide a semantic container and border, with no container activation. The nested action is a real Button with its own focus and input handling."
        }
        "toolbar" => {
            "Tab enters the toolbar. Left/Right and Home/End move its focused action. Enter or Space activates the focused Create, Rename or Remove fixture action; releasing outside cancels pointer activation."
        }
        "number-field" => {
            "Tab focuses a stepper action. Arrow keys, Home/End and the decrement/increment actions change the bounded 1–32 value. Bound-disabled actions do not change it. Editable numeric text is not implemented."
        }
        "field-meta" => {
            "FieldView presents a label, help and error message around the real text input. Error changes the message and input presentation; Disabled blocks edits and the associated fixture actions. This metadata specimen does not save settings."
        }
        "skeleton" => {
            "Skeleton is a static loading placeholder with no selection, focus or activation behavior. Reset restores the initial Gallery fixture; no timer or network request resolves this specimen."
        }
        "form" | "settings" | "async" => {
            "Edit the workspace name and notification preference, then Save. Empty names fail validation. Resolve fixture explicitly completes a pending local attempt: the first fails and Retry can succeed. Disabled blocks edits and save actions. Reset restores fixture data; no real network request runs."
        }
        "filter" => {
            "Type in the real text input to filter three local file names. Clear filter empties the query; an empty query shows all files. No-match results are local and deterministic."
        }
        "master" => {
            "The document list and detail text have separate scroll areas. Pointer selection and Up/Down/Home/End choose a document; keyboard focus reveals the selected row. The detail surface is read-only and supports selection and Copy."
        }
        _ => {
            "This route has no component-specific input contract beyond its documented fixture controls. See Playground and the registered API for the implemented behavior."
        }
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
    state.button_labels.insert(id, title.to_owned());
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
        tekes_ui_tokens::Palette::new(ctx.theme, state.brand).expect("registered Gallery brand"),
        if id == 20 {
            paint::ButtonVariant::Primary
        } else {
            paint::ButtonVariant::Quiet
        },
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::default(),
    );
    state.register_logical_focus(
        id,
        layout.frame,
        !(disabled || state.loading && id == 20),
        true,
        None,
        true,
    );
    if let Some(visible) = layout.visible {
        state.hits.push((id, visible));
        state.button_hits.push(id);
    }
}
/// Draw a wrapping action group and return the first unobstructed body y.
fn action_group(
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    items: &[(usize, &str, bool)],
) -> f32 {
    if items.is_empty() {
        return bounds.min_y();
    }
    let mut x = bounds.min_x();
    let mut y = bounds.min_y();
    let mut row_height: f32 = 0.;
    for &(id, title, selected) in items {
        let natural = ctx.text.measure_button(
            title,
            bounds.size.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let width = natural.width.max(112.).min(bounds.size.width.max(0.));
        let measured =
            ctx.text
                .measure_button(title, width, ButtonSize::Body, proposal::TextRole::UiBody);
        let height = measured.height.max(36.);
        if x > bounds.min_x() && x + width > bounds.max_x() {
            y += row_height + 8.;
            x = bounds.min_x();
            row_height = 0.;
        }
        let frame_height = height + if state.density == 1 { 4. } else { 0. };
        action(
            state,
            ctx,
            scene,
            id,
            Rect::new(x, y, width, frame_height),
            title,
            (selected, false),
        );
        row_height = row_height.max(height);
        x += width + 8.;
    }
    y + row_height + 18.
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
    state.button_labels.insert(id, title.to_owned());
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
        tekes_ui_tokens::Palette::new(ctx.theme, state.brand).expect("registered Gallery brand"),
        paint::ButtonVariant::Quiet,
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::default(),
    );
    state.register_logical_focus(id, layout.frame, !disabled, true, None, true);
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
        if entry(i).renderer() == "matrix" {
            return self.brand_preview.focused_frame();
        }
        let state = self.states.get(&i)?;
        let id = state.focus?;
        let renderer = entry(i).renderer();
        state.focus_frame(renderer, id)
    }

    /// Whether a modal dialog currently owns input inside this story.
    pub fn modal_is_open(&self, i: usize) -> bool {
        self.states
            .get(&i)
            .is_some_and(|state| state.dialog_view.dialog().open)
    }

    /// Move focus into the first or last enabled, mounted target in the current
    /// story. The full layout frame lets the Gallery reveal it through the
    /// actual viewport without making it a pointer hit target.
    pub fn focus_edge(&mut self, i: usize, reverse: bool) -> Option<Rect> {
        let renderer = entry(i).renderer();
        if renderer == "matrix" {
            return self.brand_preview.focus_edge(reverse);
        }
        let state = self.state(i);
        let ids = state.logical_tab_ids();
        let target = if reverse {
            ids.last().copied()
        } else {
            ids.first().copied()
        }?;
        self.focus_control(i, target)
    }

    fn focus_control(&mut self, i: usize, target: usize) -> Option<Rect> {
        let renderer = entry(i).renderer();
        let state = self.state(i);
        if !state.window_active || !state.focusable_mounted(target) {
            return None;
        }
        state.input.blur();
        state.detail.blur();
        state.focus = Some(target);
        state.sync_owned_focus(renderer, Some(target), true);
        if renderer == "tabs" && (30..33).contains(&target) {
            state.tabs_view.focus(target - 30, true);
        } else if renderer == "tabs" {
            state.tabs_view.clear_focus();
        }
        if extended::supports(renderer) {
            state.extended.host_focus_changed(renderer, target, true);
            if renderer == "link" {
                state.extended.link.button.state.focused = true;
            }
        }
        if target == 1000 {
            if renderer == "master" {
                state.detail.focus_field();
            } else {
                state.input.focus_field();
            }
        }
        if renderer == "master" && (10000..10100).contains(&target) {
            state.focus_reveal = Some(target);
        }
        for button in state.buttons.values_mut() {
            button.state.focused = false;
            button.state.focus_visible = false;
        }
        let button = state.buttons.entry(target).or_default();
        button.state.focused = true;
        button.state.focus_visible = true;
        state.focus_frame(renderer, target)
    }

    pub fn clear_focus(&mut self, i: usize) {
        let renderer = entry(i).renderer();
        if renderer == "matrix" {
            self.brand_preview.clear_focus();
        }
        let state = self.state(i);
        // Leaving this scope dismisses its transient action menu without
        // stealing focus back from the new owner. Dialog scope is unchanged.
        let _ = state.menu_view.deactivate();
        state.menu_layout = None;
        state.extended.combo.open = false;
        state.extended.combo_view.options.cancel();
        state
            .extended
            .command_view
            .blur(&mut state.extended.command);
        state.input.blur();
        state.detail.blur();
        state.focus = None;
        state.sync_owned_focus(renderer, None, false);
        state.extended.blur_selection_controls();
        for button in state.buttons.values_mut() {
            button.state.focused = false;
            button.state.focus_visible = false;
        }
    }

    #[cfg(test)]
    pub(super) fn focused_control_id(&self, i: usize) -> Option<usize> {
        self.states.get(&i).and_then(|state| state.focus)
    }

    #[cfg(test)]
    pub(super) fn combo_popup_open(&self, i: usize) -> bool {
        self.states
            .get(&i)
            .is_some_and(|state| state.extended.combo.open)
    }

    #[cfg(test)]
    pub(super) fn select_value_for_test(&self, i: usize) -> (bool, usize) {
        let state = self.states.get(&i).expect("mounted Select story");
        (state.select.menu.open, state.select.selected)
    }

    #[cfg(test)]
    pub(super) fn tab_stop_ids(&self, i: usize) -> Vec<usize> {
        self.states
            .get(&i)
            .map_or_else(Vec::new, StoryState::logical_tab_ids)
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
        text.set_motion_enabled(true);
        text.set_motion_context(Duration::ZERO, false);
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
    pub(super) fn is_window_active(&self) -> bool {
        self.window_active
    }

    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        let now = self.motion_now;
        self.brand_preview.set_window_active(active);
        for state in self.states.values_mut() {
            state.set_window_active(active, now);
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
                || s.dialog_view.needs_animation()
                || s.extended.overlay_needs_animation()
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
    pub(super) fn take_scene3d_reset(&mut self, i: usize) -> bool {
        std::mem::take(&mut self.state(i).scene3d_reset)
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
    /// Extent reported by the last real render, expressed in document space.
    /// Reading this never changes component state, text caches, or presence.
    pub fn measured_document_extent(&self, i: usize) -> f32 {
        self.states.get(&i).map_or(820., |state| {
            let bottom = match (state.document_bottom != 0., state.event_log_frame) {
                (true, Some(log)) => state.document_bottom.max(log.max_y()),
                (true, None) => state.document_bottom,
                (false, Some(log)) => log.max_y(),
                (false, None) => return 820.,
            };
            (bottom - state.document_origin_y + 8.).max(820.)
        })
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
            state.document_bottom = 0.;
            state.event_log_frame = None;
            state.extended.blur_selection_controls();
            state.focus = None;
            state.logical_focus.clear();
        }
        self.state(i).section = next_section;
    }
    fn state(&mut self, i: usize) -> &mut StoryState {
        let reduced = self.reduced;
        let active = self.window_active;
        let now = self.motion_now;
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
            state.set_window_active(active, now);
            state
        })
    }
    pub fn text_state(&self, i: usize) -> Option<tekes_ui_platform_macos::TextInputState> {
        if entry(i).renderer() == "matrix" {
            return self.brand_preview.text_state();
        }
        self.states.get(&i).and_then(|s| {
            if entry(i).renderer() == "command-palette"
                && (!s.extended.command.combo.open || s.focus != Some(1000))
            {
                return None;
            }
            s.input.state().or_else(|| s.detail.state())
        })
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
        if entry(i).renderer() == "command-palette"
            && (!s.extended.command.combo.open || s.focus != Some(1000))
        {
            return (false, None);
        }
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
        if entry(i).renderer() == "form" {
            s.persist_draft();
        }
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
            s.logical_focus.clear();
            s.toolbar_view.focus(None, false);
            s.number_view.blur();
            let _ = s.menu_view.deactivate();
            s.menu_view.clear_presentation();
            s.dialog_view.clear_presentation();
            s.extended.clear_overlay_presentation();
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
        // Snapshot stories have no window frame loop. Borrow an explicit,
        // deterministic frame sample and restore the caller's exact context.
        if self.snapshot {
            let host_clock = ctx.text.motion_context();
            ctx.text.replace_fixture_motion_context(Some(
                tekes_ui_controls::ControlMotionContext {
                    now: host_clock.map_or(Duration::ZERO, |clock| clock.now),
                    reduced_motion: true,
                },
            ));
            self.build_clipped_story(i, ctx, content, viewport_clip, scene);
            ctx.text.replace_fixture_motion_context(host_clock);
        } else {
            self.build_clipped_story(i, ctx, content, viewport_clip, scene);
        }
    }

    fn build_clipped_story(
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
        let active = self.window_active;
        let now = self.motion_now;
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
            state.set_window_active(active, now);
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
        s.logical_focus.clear();
        s.button_hits.clear();
        s.button_labels.clear();
        s.phase_frame = None;
        s.event_log_frame = None;
        s.document_bottom = 0.;
        s.document_origin_y = content.min_y();
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
        let visible_bottom = c.max_y().min(viewport_clip.max_y());
        s.preview = Rect::new(
            x,
            header_bottom.min(visible_bottom),
            w,
            (visible_bottom - header_bottom).max(0.),
        );
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
            s.preview = Rect::new(
                x,
                body_y.min(visible_bottom),
                w,
                (visible_bottom - body_y).max(0.),
            );
            if extended::supports(e.renderer()) {
                let preview = body;
                if s.section == 0 {
                    Self::docs(e, s, ctx, scene, preview);
                    return;
                }
                if s.section == 2 {
                    let mut ext = std::mem::take(&mut s.extended);
                    let host_clock = ctx.text.motion_context();
                    ctx.text
                        .replace_fixture_motion_context(host_clock.map(|mut clock| {
                            clock.reduced_motion = true;
                            clock
                        }));
                    ext.matrix(e, s, ctx, scene, preview);
                    ctx.text.replace_fixture_motion_context(host_clock);
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
                let component_bounds = if [
                    "list",
                    "tree",
                    "data-table",
                    "chart",
                    "markdown",
                    "code-block",
                    "diff-view",
                    "tooltip",
                    "popover",
                    "toast",
                    "banner",
                    "status-badge",
                    "progress",
                    "empty-state",
                ]
                .contains(&e.renderer())
                {
                    body
                } else {
                    s.preview
                };
                ext.draw(e, s, ctx, scene, component_bounds);
                for target in ext.logical_focus_targets(e.renderer()) {
                    s.register_logical_focus(
                        target.id,
                        target.frame,
                        target.eligible,
                        target.mounted,
                        target.roving_group,
                        target.roving_selected,
                    );
                }
                s.extended = ext;
                return;
            }
            if s.section == 0 {
                Self::docs(e, s, ctx, scene, body);
                return;
            }
            if s.section == 2 {
                let host_clock = ctx.text.motion_context();
                ctx.text
                    .replace_fixture_motion_context(host_clock.map(|mut clock| {
                        clock.reduced_motion = true;
                        clock
                    }));
                Self::matrix(e, s, ctx, scene, body);
                ctx.text.replace_fixture_motion_context(host_clock);
                return;
            }
            if s.section == 5 {
                Self::variants(e, s, ctx, scene, body);
                return;
            }
            if s.section == 3 {
                let copy = behavior_copy(e.renderer());
                let height = ctx
                    .text
                    .proposal_label(copy, proposal::TextRole::UiBody, Some(w))
                    .height as f32;
                label(ctx, scene, Rect::new(x, body_y, w, height), copy, false);
                let note_y = body_y + height + 24.;
                let note = "State matrices are deterministic visual comparisons. Use Playground for real input. Native VoiceOver and physical IME checks remain manual.";
                let note_h = ctx
                    .text
                    .proposal_label(note, proposal::TextRole::UiBody, Some(w))
                    .height as f32;
                label(ctx, scene, Rect::new(x, note_y, w, note_h), note, false);
                s.document_bottom = note_y + note_h;
                return;
            }
            if ["scroll", "text", "icon"].contains(&e.renderer()) {
                return;
            }
            Self::playground(e, s, ctx, scene, body, viewport_clip);
        } else {
            if e.renderer() == "matrix" {
                self.brand_preview.build(
                    ctx,
                    scene,
                    Rect::new(x, header_bottom, w, (c.max_y() - header_bottom).max(0.)),
                    s.button_clip,
                    self.brand,
                    density,
                );
                return;
            }
            Self::reference(e, s, ctx, scene, c);
        }
    }
    fn docs(e: &Entry, s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, c: Rect) {
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
            lines.extend(contract_summary(e));
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
            s.document_bottom = y;
            scene.pop_clip();
            return;
        }
        let mut lines=vec![format!("Component: {}  |  maturity: experimental",e.component()),format!("Story routes: {}", ["docs", "playground", "states", "behaviors", "edge-cases", "variants"].into_iter().filter(|story| e.supports_story(story)).map(|story| format!("{}/{story}", e.slug())).collect::<Vec<_>>().join(" · ")),"State is local to this route. Reset restores deterministic fixture data. The host supplies the animation clock; no random source or real network request is used.".into(),format!("Usage: {}", api_reference(e)),"State comparison is a visual aid, not a keyboard or accessibility test. Native input services are exercised through the real window.".into()];
        lines.extend(contract_summary(e));
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
        s.document_bottom = y;
    }
    fn matrix(e: &Entry, s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, c: Rect) {
        if basic_states::supports(e.renderer()) {
            basic_states::matrix(e, s, ctx, scene, c);
            return;
        }
        if ["scroll", "text", "icon"].contains(&e.renderer()) {
            let copy = "Use Playground to inspect the real viewport and selection. ScrollArea supports independent Enabled / Disabled / Auto axes. Text exposes read-only selection; it has no pressed or loading state.";
            let measured = ctx.text.proposal_label(
                copy,
                proposal::TextRole::UiBody,
                Some(c.size.width.max(0.)),
            );
            label(
                ctx,
                scene,
                Rect::new(c.min_x(), c.min_y(), c.size.width, measured.height as f32),
                copy,
                false,
            );
            s.document_bottom = c.min_y() + measured.height as f32;
            return;
        }
        if ["tabs", "menu", "dialog", "select"].contains(&e.renderer()) {
            Self::composite_matrix(e, s, ctx, scene, c);
            return;
        }
        let focus_gutter = -ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
            .focus_bounds(ctx.text.backing_scale())
            .min_x();
        // Keep the real ancestor clip and reserve the focus ring plus its
        // antialiasing guard inside it, just as the extended state matrix does.
        let layout_bounds = c.inset(focus_gutter);
        let explanation = "Visual fixtures only — simulated focus is not a focus test. Real interaction: Playground.";
        let explanation_height = ctx
            .text
            .proposal_label(
                explanation,
                proposal::TextRole::UiBody,
                Some(layout_bounds.size.width.max(0.)),
            )
            .height as f32;
        label(
            ctx,
            scene,
            Rect::new(
                layout_bounds.min_x(),
                layout_bounds.min_y(),
                layout_bounds.size.width,
                explanation_height,
            ),
            explanation,
            false,
        );
        let checkbox_states = [
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
        ];
        let standard_states = [
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
        ];
        let checkbox = e.renderer() == "checkbox";
        let mut states: Vec<(&str, ControlState, Option<CheckState>)> = if checkbox {
            checkbox_states
                .iter()
                .map(|(name, state, checked)| (*name, *state, Some(*checked)))
                .collect()
        } else {
            standard_states
                .iter()
                .filter(|(name, _)| match *name {
                    "error + focus" => {
                        ["field", "area", "search", "field-meta"].contains(&e.renderer())
                    }
                    "loading" => e.renderer() == "button",
                    "hover" | "pressed" => e.renderer() != "field-meta",
                    _ => true,
                })
                .map(|(name, state)| (*name, *state, None))
                .collect()
        };
        if ["field", "area", "search"].contains(&e.renderer()) {
            for name in ["read-only", "composing", "selection"] {
                states.push((
                    name,
                    ControlState {
                        focused: true,
                        ..Default::default()
                    },
                    None,
                ));
            }
        }
        let columns = if layout_bounds.size.width < 600. {
            2
        } else {
            3
        };
        let cell_width = (layout_bounds.size.width / columns as f32 - 16.).max(0.);
        let multiline = e.renderer() == "area";
        let field_meta = e.renderer() == "field-meta";
        let mut measurements = Vec::with_capacity(states.len());
        for (name, _, _) in &states {
            let title_height = ctx
                .text
                .proposal_label(name, proposal::TextRole::UiCaption, Some(cell_width))
                .height as f32;
            let control_height = if checkbox {
                Checkbox::measure(
                    ctx.text,
                    "Notifications",
                    cell_width,
                    proposal::TextRole::UiBody,
                )
                .height
            } else if multiline {
                let value = "A first complete sentence for the editor.\nA second line keeps the text layout visible.\nA third line gives the multiline control room to breathe.";
                let mut input = TextDemo::field(value, true, false);
                input.use_fixture_cache();
                let minimum = input.measure_search_height(ctx.text);
                let document_height = ctx
                    .text
                    .proposal_label(
                        value,
                        proposal::TextRole::UiBody,
                        Some((cell_width - 24.0).max(0.0)),
                    )
                    .height as f32
                    + 2.0 * proposal::SCROLL_SINGLE_LINE_PADDING_Y;
                minimum.max(document_height)
            } else if field_meta {
                let is_error = *name == "error + focus";
                let mut field = s.field.clone();
                field.label = "Workspace name".into();
                field.help = "Use a clear name for your workspace.".into();
                field.error = is_error.then(|| "A name is required.".into());
                let mut view = FieldView::default();
                let mut probe = Scene::new();
                let layout = view.render_with_presentation(
                    &mut probe,
                    ctx.text,
                    &field,
                    Rect::new(0.0, 0.0, cell_width, 4096.0),
                    None,
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
                    proposal::TextRole::UiBody,
                    FieldPresentation::emphasized_form(),
                );
                layout.message.max_y().max(layout.slot.max_y())
            } else if ["field", "search"].contains(&e.renderer()) {
                let value = if *name == "default" {
                    "Studio"
                } else {
                    "Sample text"
                };
                let mut input = TextDemo::field(value, false, false);
                input.use_fixture_cache();
                input.measure_search_height(ctx.text)
            } else {
                let title = if *name == "loading" {
                    "Loading…"
                } else {
                    "Sample"
                };
                ctx.text
                    .measure_button(
                        title,
                        cell_width,
                        ButtonSize::Body,
                        proposal::TextRole::UiBody,
                    )
                    .height
            };
            measurements.push((title_height, control_height));
        }
        let (cells, document_bottom) =
            matrix_cell_geometries(layout_bounds, columns, explanation_height, &measurements);
        let palette =
            tekes_ui_tokens::Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
        for (n, (name, state, check_state)) in states.into_iter().enumerate() {
            let cell = cells[n];
            let r = cell.control;
            if field_meta {
                let mut field = s.field.clone();
                let is_error = name == "error + focus";
                field.label = "Workspace name".into();
                field.help = "Use a clear name for your workspace.".into();
                field.error = is_error.then(|| "A name is required.".into());
                let layout = s.field_view.render_with_presentation(
                    scene,
                    ctx.text,
                    &field,
                    r,
                    Some(c),
                    palette,
                    proposal::TextRole::UiBody,
                    FieldPresentation::emphasized_form(),
                );
                let mut input = TextDemo::field("Studio", false, false);
                input.use_fixture_cache();
                input.set_disabled(state.disabled);
                input.set_error(is_error);
                if state.focused {
                    input.focus_field();
                }
                input.build_search_clipped(ctx, layout.slot, Some(c), scene);
            } else if ["field", "area", "search"].contains(&e.renderer()) {
                let multiline_value = "A first complete sentence for the editor.\nA second line keeps the text layout visible.\nA third line gives the multiline control room to breathe.";
                let mut input = TextDemo::field(
                    if multiline {
                        multiline_value
                    } else if name == "default" {
                        "Studio"
                    } else {
                        "Sample text"
                    },
                    multiline,
                    name == "read-only",
                );
                input.use_fixture_cache();
                input.set_disabled(state.disabled);
                input.set_error(state.error);
                input.prepare_input_state_fixture(name);
                input.prepare_input_pointer_fixture(ctx.text, r, Some(c), name);
                if state.focused {
                    input.focus_field();
                }
                input.build_search_clipped(ctx, r, Some(c), scene);
            } else if checkbox {
                let mut button = ButtonModel::default();
                button.state = state;
                let mut control = Checkbox {
                    button,
                    checked: check_state.expect("checkbox state"),
                };
                control.render(
                    scene,
                    ctx.text,
                    "Notifications",
                    r,
                    Some(c),
                    palette,
                    proposal::TextRole::UiBody,
                );
            } else {
                let mut fixture = ButtonModel::default();
                fixture.state = state;
                let title = if state.loading {
                    "Loading…"
                } else {
                    "Sample"
                };
                ctx.text.render_button(
                    scene,
                    &mut fixture,
                    title,
                    r,
                    Some(c),
                    palette,
                    paint::ButtonVariant::Quiet,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::default(),
                );
            }
            matrix_caption(ctx, scene, cell.caption, name);
        }
        s.hits.retain(|(id, _)| *id <= 6);
        s.button_hits.retain(|id| *id <= 6);
        s.document_bottom = document_bottom;
    }
    fn composite_matrix(
        e: &Entry,
        s: &mut StoryState,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        c: Rect,
    ) {
        if e.renderer() == "select" {
            extended::navigation_states::matrix(e, s, ctx, scene, c);
            return;
        }
        let palette =
            tekes_ui_tokens::Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
        let role = proposal::TextRole::UiBody;
        let mut y = c.min_y();
        let titles: &[&str] = match e.renderer() {
            "tabs" => &[
                "Selected first",
                "Selected second · focus-visible",
                "Pressed selected tab",
                "Disabled tabs",
                "Selected tab · hover",
            ],
            "menu" => &[
                "Open menu",
                "Keyboard next item",
                "Pressed first item",
                "Disabled menu items",
            ],
            _ => &[
                "Open dialog",
                "Keyboard focus",
                "Pressed confirm",
                "Disabled dialog actions",
            ],
        };
        for (n, title) in titles.iter().enumerate() {
            let h = ctx
                .text
                .proposal_label(title, role, Some(c.size.width))
                .height as f32;
            label(
                ctx,
                scene,
                Rect::new(c.min_x(), y, c.size.width, h),
                title,
                false,
            );
            let body = Rect::new(
                c.min_x(),
                y + h + 8.,
                c.size.width,
                if e.renderer() == "tabs" { 100. } else { 230. },
            );
            let mut probe = Scene::new();
            match e.renderer() {
                "tabs" => {
                    let mut view = TabsView::new(3);
                    if n == 1 {
                        view.focus(1, true);
                    }
                    view.set_disabled(n == 3);
                    let layout = view.render(
                        &mut probe,
                        ctx.text,
                        &["General", "Security", "Advanced"],
                        Rect::new(body.min_x(), body.min_y(), body.size.width, 36.),
                        Some(body),
                        palette,
                        role,
                        false,
                    );
                    if n == 4 {
                        view.pointer_move_at(layout.items[0].frame.center());
                    }
                    if n == 2 {
                        view.pointer_down_at(layout.items[0].frame.center());
                    }
                    let layout = view.render(
                        scene,
                        ctx.text,
                        &["General", "Security", "Advanced"],
                        Rect::new(body.min_x(), body.min_y(), body.size.width, 36.),
                        Some(body),
                        palette,
                        role,
                        false,
                    );
                    label(
                        ctx,
                        scene,
                        Rect::new(
                            body.min_x(),
                            layout.frame.max_y() + 8.,
                            body.size.width,
                            48.,
                        ),
                        ["General panel", "Security panel", "Advanced panel"][view.selected()],
                        false,
                    );
                }
                "menu" => {
                    let mut view = MenuView::new(3);
                    view.set_reduced_motion(true);
                    view.show(0);
                    if n == 1 {
                        view.key(ControlKey::Down, true);
                    }
                    view.set_disabled(n == 3);
                    let anchor =
                        Rect::new(body.min_x(), body.min_y(), body.size.width.min(230.), 1.);
                    if let Some(layout) = view.render(
                        &mut probe,
                        ctx.text,
                        &["Open", "Duplicate", "Archive"],
                        anchor,
                        body,
                        Some(body),
                        palette,
                        role,
                        false,
                    ) && n == 2
                    {
                        view.pointer_down_at(layout.items[0].frame.center());
                    }
                    view.render(
                        scene,
                        ctx.text,
                        &["Open", "Duplicate", "Archive"],
                        anchor,
                        body,
                        Some(body),
                        palette,
                        role,
                        false,
                    );
                }
                _ => {
                    let mut view = DialogView::default();
                    view.show(0);
                    if n == 1 {
                        view.key(ControlKey::Tab { reverse: false }, true);
                    }
                    view.set_disabled(n == 3);
                    if let Some(layout) = view.render(
                        &mut probe,
                        ctx.text,
                        "Confirm changes?",
                        body,
                        Some(body),
                        palette,
                        role,
                        false,
                    ) && n == 2
                    {
                        view.pointer_down_at(layout.buttons[1].frame.center());
                    }
                    view.render(
                        scene,
                        ctx.text,
                        "Confirm changes?",
                        body,
                        Some(body),
                        palette,
                        role,
                        false,
                    );
                }
            }
            y = body.max_y() + 24.;
        }
        #[cfg(any(test, feature = "composition-review"))]
        if e.renderer() == "tabs" {
            y = content_status::matrix(
                tekes_ui_composition_consumer::content_status::Kind::Tabs,
                s,
                ctx,
                scene,
                c,
                y + 24.,
            );
        }
        s.document_bottom = y;
        s.hits.retain(|(id, _)| *id <= 6);
        s.button_hits.retain(|id| *id <= 6);
    }

    fn variants(e: &Entry, s: &mut StoryState, ctx: &mut PageCtx<'_>, scene: &mut Scene, c: Rect) {
        if e.renderer() == "button" {
            let mut row_y = c.min_y();
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
                let r = Rect::new(c.min_x(), row_y, c.size.width.min(200.), height);
                let mut button = ButtonModel::default();
                ctx.text.render_button(
                    scene,
                    &mut button,
                    title,
                    r,
                    Some(c),
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
                    paint::ButtonVariant::Primary,
                    [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg][n],
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::default(),
                );
                let primary_bottom = button
                    .layout()
                    .expect("variant button laid out")
                    .frame
                    .max_y();
                let quiet = if c.size.width >= 440. {
                    r.offset(240., 0.)
                } else {
                    Rect::new(r.min_x(), primary_bottom + 12., r.size.width, r.size.height)
                };
                let mut quiet_button = ButtonModel::default();
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
                row_y = primary_bottom.max(
                    quiet_button
                        .layout()
                        .expect("quiet variant laid out")
                        .frame
                        .max_y(),
                ) + 24.;
            }
            s.document_bottom = row_y;
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
        viewport_clip: Rect,
    ) {
        let x = c.min_x();
        let y = c.min_y();
        let w = c.size.width;
        // All controls in a group share measured rows. Text scale and narrow
        // previews may wrap both a button's label and the group itself.
        let mut properties = Vec::new();
        for (n, title) in ["Disabled", "Loading", "Error", "Mixed"].iter().enumerate() {
            let supported = match n {
                0 => [
                    "button",
                    "checkbox",
                    "field",
                    "area",
                    "search",
                    "menu",
                    "select",
                    "dialog",
                    "danger",
                    "form",
                    "settings",
                    "async",
                    "field-meta",
                ]
                .contains(&e.renderer()),
                1 => e.renderer() == "button",
                2 => [
                    "field",
                    "area",
                    "search",
                    "field-meta",
                    "form",
                    "settings",
                    "select",
                ]
                .contains(&e.renderer()),
                _ => ["checkbox", "select"].contains(&e.renderer()),
            };
            if supported {
                let selected = match n {
                    0 => s.disabled,
                    1 => s.loading,
                    2 => s.error,
                    _ if e.renderer() == "select" => s.select_empty,
                    _ => s.checkbox.checked == CheckState::Mixed,
                };
                let title = if n == 3 && e.renderer() == "select" {
                    "Empty options"
                } else {
                    title
                };
                properties.push((10 + n, title, selected));
            }
        }
        let mut body_y = action_group(s, ctx, scene, Rect::new(x, y, w, 0.), &properties);
        let edge = s.section == 4;
        if edge && ["field", "area", "search", "form", "settings", "async"].contains(&e.renderer())
        {
            body_y = action_group(
                s,
                ctx,
                scene,
                Rect::new(x, body_y, w, 0.),
                &[(14, "Long mixed text", false), (15, "Empty text", false)],
            );
        }
        let r = Rect::new(x, body_y, w.min(600.), 36.);
        let mut content_bottom = None;
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
                    let remaining_preview = (s.preview.max_y() - input_y).max(0.);
                    let remaining_document = (c.max_y() - input_y).max(0.);
                    if remaining_preview > 0. {
                        remaining_preview.clamp(100., 270.)
                    } else {
                        // The field is still mounted in the document when the
                        // preview is fully clipped. Keep its real logical
                        // frame so Tab can reveal it on the next viewport.
                        remaining_document.clamp(100., 270.)
                    }
                } else {
                    (proposal::TextRole::UiBody.style().line_height * ctx.text.type_scale() + 16.)
                        .max(36.)
                        .max(field_min_height)
                };
                let input = Rect::new(x, input_y, w.min(650.), input_height);
                s.input.set_disabled(s.disabled);
                s.input.set_error(s.error || s.phase == Phase::Invalid);
                let input_clip = if e.renderer() == "area" {
                    // Clip visible content to the remaining preview, while
                    // retaining a valid disjoint viewport clip for a fully
                    // offscreen but still mounted editor.
                    Some(if s.preview.size.height > 0.0 {
                        s.preview
                    } else {
                        viewport_clip
                    })
                } else {
                    s.button_clip
                };
                s.input.build_search_clipped(ctx, input, input_clip, scene);
                if let Some(visible) = s.input.visible_frame() {
                    s.hits.push((1000, visible));
                }
                if let Some(frame) = s.input.frame_at(0) {
                    s.register_logical_focus(1000, frame, !s.disabled, true, None, true);
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
                let title = if edge {
                    "Receive project notifications 项目通知設定 日本語 — a long setting label wraps inside the available row"
                } else {
                    "Receive project notifications"
                };
                s.button_labels.insert(20, title.to_owned());
                let layout = s.checkbox.render(
                    scene,
                    ctx.text,
                    title,
                    row,
                    s.button_clip,
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
                    proposal::TextRole::UiBody,
                );
                s.buttons.insert(20, s.checkbox.button.clone());
                if let Some(visible) = layout.visible {
                    s.hits.push((20, visible));
                    s.button_hits.push(20);
                }
                s.register_logical_focus(
                    20,
                    layout.frame,
                    !s.checkbox.button.state.disabled,
                    true,
                    None,
                    true,
                );
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
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
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
                for (index, item) in layout.items.iter().enumerate() {
                    s.register_logical_focus(
                        30 + index,
                        item.frame,
                        !s.disabled,
                        true,
                        Some(1),
                        index == s.tabs_view.selected(),
                    );
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
                s.select_view.set_error(s.error);
                let labels: &[&str] = if s.select_empty {
                    &[]
                } else {
                    &["Automatic", "Compact", "Comfortable"]
                };
                s.button_labels.insert(
                    20,
                    labels
                        .get(s.select.selected.min(labels.len().saturating_sub(1)))
                        .copied()
                        .unwrap_or("")
                        .to_owned(),
                );
                for (index, label) in labels.iter().enumerate() {
                    s.button_labels.insert(60 + index, (*label).to_owned());
                }
                let trigger_frame = Rect::new(x, body_y, w.min(230.), 36.);
                // Caller feedback is base content. Paint it before the owned
                // popup so it cannot cover options in the overlay.
                let message = if s.select_empty {
                    Some("No options available.")
                } else if s.error {
                    Some("This choice is unavailable. Choose another option.")
                } else {
                    None
                };
                if let Some(message) = message {
                    let height = ctx
                        .text
                        .proposal_label(message, proposal::TextRole::UiBody, Some(w))
                        .height as f32;
                    label(
                        ctx,
                        scene,
                        Rect::new(x, trigger_frame.max_y() + 8., w, height),
                        message,
                        false,
                    );
                }
                let layout = s.select_view.render(
                    scene,
                    ctx.text,
                    &mut s.select,
                    labels,
                    trigger_frame,
                    s.preview,
                    s.button_clip,
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
                    proposal::TextRole::UiBody,
                    s.density == 1,
                );
                if let Some(visible) = layout.trigger.visible {
                    s.hits.push((20, visible));
                }
                let available = !s.disabled && !s.select_empty;
                s.register_logical_focus(20, layout.trigger.frame, available, true, None, true);
                s.buttons.entry(20).or_default().state.disabled = !available;
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
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
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
                if let Some(frame) = s.input.frame_at(0) {
                    s.register_logical_focus(1000, frame, !s.disabled, true, None, true);
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
                    (s.checkbox.checked == CheckState::Checked, s.disabled),
                );
                let save_y = s.buttons[&25]
                    .layout()
                    .expect("notification action laid out")
                    .frame
                    .max_y()
                    + 12.;
                action(
                    s,
                    ctx,
                    scene,
                    20,
                    Rect::new(x, save_y, w.min(150.), 36.),
                    if s.phase == Phase::Failed {
                        "Retry save"
                    } else {
                        "Save settings"
                    },
                    (false, s.disabled || s.phase == Phase::Pending),
                );
                let phase_y = s.buttons[&20]
                    .layout()
                    .expect("save action laid out")
                    .frame
                    .max_y()
                    + 12.;
                let phase_frame = Self::phase(s, ctx, scene, Rect::new(x, phase_y, w, 80.));
                s.phase_frame = Some(phase_frame);
                content_bottom = Some(phase_frame.max_y());
                if e.renderer() == "form"
                    && (s
                        .draft_fixture
                        .as_ref()
                        .is_some_and(|store| store.is_durable())
                        || s.draft_error.is_some())
                {
                    let y = phase_frame.max_y() + 16.;
                    action(
                        s,
                        ctx,
                        scene,
                        26,
                        Rect::new(x, y, 210., 36.),
                        "Interrupt & recover",
                        (
                            false,
                            !s.draft_fixture
                                .as_ref()
                                .is_some_and(|store| store.is_durable()),
                        ),
                    );
                    let status = s.draft_error.clone().unwrap_or_else(|| {
                        let store = s.draft_fixture.as_ref().expect("enabled fixture");
                        format!(
                            "Durable fixture · revision {} · {} · no network",
                            store.draft().revision,
                            if store.has_pending() {
                                "outbox awaiting acknowledgement"
                            } else if store.is_acknowledged() {
                                "revision acknowledged"
                            } else {
                                "draft stored locally"
                            }
                        )
                    });
                    let status_y = y + 44.;
                    let h = ctx
                        .text
                        .proposal_label(&status, proposal::TextRole::UiBody, Some(w))
                        .height as f32;
                    label(ctx, scene, Rect::new(x, status_y, w, h), &status, false);
                    content_bottom = Some(status_y + h);
                }
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
                if let Some(frame) = s.input.frame_at(0) {
                    s.register_logical_focus(1000, frame, !s.disabled, true, None, true);
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
                let bottom = c.max_y().min(s.preview.max_y());
                let bottom = s
                    .button_clip
                    .map_or(bottom, |clip| bottom.min(clip.max_y()));
                let list_y = body_y.min(bottom);
                let h = (c.size.height - 190.)
                    .clamp(100., 600.)
                    .min((bottom - list_y).max(0.));
                let list = Rect::new(x, list_y, lw - 12., h);
                s.list_frame = list;
                s.list_scroll.set_scroll_policy(
                    tekes_ui_controls::ScrollPolicy::Disabled,
                    tekes_ui_controls::ScrollPolicy::Auto,
                );
                let focus_padding = -ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
                    .focus_bounds(1.0)
                    .min_x();
                let row_width = (lw - 20.0 - focus_padding * 2.0).max(0.0);
                let row_height = (0..80)
                    .map(|n| {
                        ctx.text
                            .measure_button(
                                &format!("Document {:02}", n + 1),
                                row_width,
                                ButtonSize::Body,
                                proposal::TextRole::UiBody,
                            )
                            .height
                    })
                    .fold(36.0, f32::max);
                let row_stride = row_height + 6.;
                s.list_scroll.layout(
                    list.size,
                    tekes_ui_scene::Size::new(
                        list.size.width,
                        80. * row_stride + focus_padding * 2.0,
                    ),
                );
                if let Some(index) = s
                    .focus_reveal
                    .take()
                    .filter(|id| (10000..10100).contains(id))
                    .map(|id| id - 10000)
                {
                    let row = Rect::new(
                        focus_padding,
                        focus_padding + index as f32 * row_stride,
                        row_width,
                        row_height,
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
                        focus_padding + n as f32 * row_stride,
                        row_width,
                        row_height,
                    );
                    let row = document_row.offset(origin.x, origin.y);
                    // All rows are mounted by this deterministic fixture. Keep
                    // their real layout frames in the keyboard sequence even
                    // while the nested ScrollArea clips their paint and hits.
                    s.register_logical_focus(10000 + n, row, true, true, Some(1), s.selected == n);
                    if geometry.document_to_viewport(document_row).is_some() {
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
                if let Some(frame) = s.detail.frame_at(0) {
                    s.register_logical_focus(1000, frame, true, true, None, true);
                }
            }
            "surface" => {
                let backing_scale = ctx.text.scale();
                let layout = s.surface_view.render_at_scale(
                    scene,
                    Rect::new(x, body_y, w.min(660.), 150.),
                    s.button_clip,
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
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
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
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
                    s.button_clip,
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
                    proposal::TextRole::UiBody,
                );
                for (index, layout) in layouts.into_iter().enumerate() {
                    s.buttons.entry(20 + index).or_default().state.disabled = s.disabled;
                    if let Some(visible) = layout.visible {
                        s.hits.push((20 + index, visible));
                    }
                    s.register_logical_focus(
                        20 + index,
                        layout.frame,
                        !s.disabled,
                        true,
                        None,
                        true,
                    );
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
                    tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                        .expect("registered Gallery brand"),
                    proposal::TextRole::UiBody,
                );
                s.buttons.entry(23).or_default().state.disabled =
                    s.disabled || !s.number.can_increment(false);
                s.buttons.entry(24).or_default().state.disabled =
                    s.disabled || !s.number.can_increment(true);
                if let Some(visible) = layout.decrement.visible {
                    s.hits.push((23, visible));
                }
                s.register_logical_focus(
                    23,
                    layout.decrement.frame,
                    !s.disabled && s.number.can_increment(false),
                    true,
                    None,
                    true,
                );
                if let Some(visible) = layout.increment.visible {
                    s.hits.push((24, visible));
                }
                s.register_logical_focus(
                    24,
                    layout.increment.frame,
                    !s.disabled && s.number.can_increment(true),
                    true,
                    None,
                    true,
                );
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
                        tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                            .expect("registered Gallery brand"),
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
        let palette =
            tekes_ui_tokens::Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
        let trigger = s
            .buttons
            .get(&20)
            .and_then(ButtonModel::layout)
            .map(|layout| layout.frame);
        if (s.menu_view.is_open() || s.menu_view.is_presented())
            && let Some(trigger) = trigger
        {
            s.menu_view.set_disabled(s.disabled);
            let labels = ["Open document", "Duplicate document", "Archive document"];
            for (index, label) in labels.iter().enumerate() {
                s.button_labels.insert(50 + index, (*label).to_owned());
            }
            s.menu_layout = s.menu_view.render(
                scene,
                ctx.text,
                &labels,
                trigger,
                s.preview,
                s.button_clip,
                palette,
                proposal::TextRole::UiBody,
                s.density == 1,
            );
        }
        if s.dialog_view.dialog().open || s.dialog_view.is_presented() {
            s.dialog_view.set_disabled(s.disabled);
            s.dialog_layout = s.dialog_view.render(
                scene,
                ctx.text,
                StoryState::dialog_title(e.renderer()),
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
        let text = match (
            s.draft_fixture
                .as_ref()
                .is_some_and(|store| store.is_durable()),
            s.phase,
        ) {
            (true, Phase::Ready) => {
                "Draft stored locally. Save queues this revision for the controlled fixture."
            }
            (true, Phase::Pending) => {
                "Revision queued durably. Resolve or interrupt to test recovery."
            }
            (true, Phase::Failed) => {
                "Delivery interrupted or failed. Draft retained; retry the queued revision."
            }
            (true, Phase::Saved) => {
                "Current revision acknowledged by the local fixture. No network was used."
            }
            (_, Phase::Ready) => "Ready. First submission fails; retry succeeds.",
            (_, Phase::Invalid) => "Validation error: enter a workspace name.",
            (_, Phase::Pending) => "Submitting… use Resolve to complete the controlled fixture.",
            (_, Phase::Failed) => "Save failed. Your input is preserved. Retry is available.",
            (_, Phase::Saved) => "Saved successfully. Preferences remain in this story only.",
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
            if let Some(button) = s.buttons.get(&21).and_then(ButtonModel::layout) {
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
                let palette = tekes_ui_tokens::Palette::new(ctx.theme, s.brand)
                    .expect("registered Gallery brand");
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
            "internal-text" | "motion" | "scene3d" => {}
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
    pub fn has_pointer_capture(&self, i: usize) -> bool {
        entry(i).renderer() == "split-view"
            && self
                .states
                .get(&i)
                .is_some_and(|s| s.extended.pointer_dragging == Some(90))
    }
    pub fn handle(&mut self, i: usize, event: &Event, text: &mut super::text::TextEngine) -> bool {
        let handled = self.handle_inner(i, event, text);
        if entry(i).renderer() == "form" {
            self.state(i).persist_draft();
        }
        handled
    }
    fn handle_inner(
        &mut self,
        i: usize,
        event: &Event,
        text: &mut super::text::TextEngine,
    ) -> bool {
        let renderer = entry(i).renderer();
        if renderer == "matrix"
            && let Event::Key(key) = event
            && key.kind == KeyEventKind::Down
            && key.key_code == 48
            && self.brand_preview.focused_frame().is_none()
        {
            return self.brand_preview.focus_edge(key.modifiers.shift).is_some();
        }
        if renderer == "matrix"
            && let Some(applied) = self.brand_preview.handle(event, text)
        {
            if let Some(brand) = applied {
                self.brand = brand;
            }
            return true;
        }
        let s = self.state(i);
        if renderer == "command-palette"
            && !s.extended.command.combo.open
            && matches!(
                event,
                Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextUnmark
                    | Event::TextCommand(_)
            )
        {
            return false;
        }
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
                    if renderer == "command-palette" && key == ControlKey::Escape {
                        s.close_command_palette();
                        return true;
                    }
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
                        if renderer == "command-palette" {
                            s.close_command_palette();
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
                    let was_open = renderer == "command-palette" && s.extended.command.combo.open;
                    let mut events = Vec::new();
                    let handled = s
                        .extended
                        .key(renderer, key, down, &mut |event| events.push(event));
                    for event in events {
                        s.record(event);
                    }
                    if handled {
                        if renderer == "command-palette"
                            && !was_open
                            && s.extended.command.combo.open
                        {
                            s.open_command_palette(text);
                        }
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
                        if renderer == "popover" && !s.extended.popover.open {
                            s.focus = s.restore_focus(s.extended.focused);
                            s.sync_owned_focus(renderer, s.focus, true);
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
                            s.close_command_palette();
                        }
                        if renderer == "command-palette" && down && key == ControlKey::Escape {
                            s.close_command_palette();
                        }
                        return true;
                    }
                }
                // Production navigation links leave Space for page handling;
                // do not reinterpret an unconsumed key as generic Button input.
                if matches!(renderer, "link" | "breadcrumb")
                    && key == ControlKey::Space
                    && extended::handles_key(renderer, key, s.focus, &s.extended)
                {
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
                if down && matches!(key, ControlKey::Tab { .. }) {
                    if renderer == "combo-box" && s.extended.combo.open {
                        s.extended.combo_view.key_event(
                            ControlKey::Escape,
                            true,
                            &mut s.extended.combo,
                        );
                    } else if renderer == "command-palette" && s.extended.command.combo.open {
                        s.extended.command_view.key_event(
                            ControlKey::Escape,
                            true,
                            &mut s.extended.command,
                        );
                    }
                }
                if down && matches!(key, ControlKey::Tab { .. }) {
                    if s.menu_view.is_open() {
                        let event = s.menu_view.key(ControlKey::Escape, true);
                        s.menu_event(event, renderer, false);
                    }
                    if s.select.menu.open {
                        let event = s.select_view.key(ControlKey::Escape, true, &mut s.select);
                        s.menu_event(event, renderer, true);
                    }
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
                    let keyboard_closed = down
                        && matches!(
                            event,
                            MenuEvent::Activated { .. } | MenuEvent::Dismissed { .. }
                        );
                    s.menu_event(event, renderer, select);
                    // Menu exposes the caller target; its caller owns the
                    // trigger Button and therefore its input-modality paint.
                    if keyboard_closed && let Some(id) = s.focus {
                        let trigger = s.buttons.entry(id).or_default();
                        trigger.state.focused = true;
                        trigger.state.focus_visible = true;
                    }
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
                        let ids = s.logical_tab_ids();
                        if ids.is_empty() {
                            return false;
                        }
                        let current = s.focus.and_then(|id| ids.iter().position(|n| *n == id));
                        let next = match current {
                            None => Some(if reverse { ids.len() - 1 } else { 0 }),
                            Some(0) if reverse => None,
                            Some(n) if reverse => Some(n - 1),
                            Some(n) if n + 1 == ids.len() => None,
                            Some(n) => Some(n + 1),
                        };
                        let Some(n) = next else {
                            // A scope edge bubbles to the Gallery, but the
                            // production owner must release its focus/press
                            // state before the next key-up is routed elsewhere.
                            s.input.blur();
                            s.detail.blur();
                            s.focus = None;
                            s.sync_owned_focus(renderer, None, false);
                            s.extended.blur_selection_controls();
                            s.tabs_view.clear_focus();
                            for button in s.buttons.values_mut() {
                                button.state.focused = false;
                                button.state.focus_visible = false;
                            }
                            return false;
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
                            if renderer == "link" {
                                s.extended.link.button.state.focused = true;
                            }
                        }
                        if ids[n] == 1000 {
                            if renderer == "master" {
                                s.detail.focus_field();
                            } else {
                                s.input.focus_field();
                            }
                        }
                        for b in s.buttons.values_mut() {
                            b.state.focused = false;
                            b.state.focus_visible = true;
                        }
                        let button = s.buttons.entry(ids[n]).or_default();
                        button.state.focused = true;
                        button.state.focus_visible = true;
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
            // AppKit delivers Dragged while the button is held, including
            // outside the window. A subsequent buttonless Moved means the
            // release was missed: roll back instead of keeping stale capture.
            if renderer == "split-view" && s.extended.pointer_dragging == Some(90) {
                match m.kind {
                    MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => {
                        return s
                            .extended
                            .pointer_move(renderer, p, s.hit_at(p), &mut |_| {});
                    }
                    MouseEventKind::Moved => return s.extended.cancel_pointer(),
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        s.extended.cancel_pointer();
                    }
                    _ => {}
                }
            }
            if renderer == "split-view"
                && matches!(
                    m.kind,
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        click_count: 2
                    }
                )
                && s.extended.split_view.double_click(p, &mut s.extended.split)
            {
                s.extended.pointer_dragging = None;
                s.focus = Some(90);
                s.extended.host_focus_changed(renderer, 90, false);
                return true;
            }
            if renderer == "command-palette"
                && s.extended.command.combo.open
                && matches!(
                    m.kind,
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    }
                )
                && !matches!(s.hit_at(p), Some(20 | 1000 | 40..=42))
            {
                s.close_command_palette();
            }
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
                    MouseEventKind::Moved
                    | MouseEventKind::Entered
                    | MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => {
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
                    MouseEventKind::Moved
                    | MouseEventKind::Entered
                    | MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => {
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
                    MouseEventKind::Moved
                    | MouseEventKind::Entered
                    | MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => {
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
                    MouseEventKind::Moved
                    | MouseEventKind::Entered
                    | MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => {
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
                        if was_open
                            && !s.select.menu.open
                            && s.hit_at(p).is_some()
                            && s.hit_at(p) != Some(20)
                            && !inside_popup
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
                MouseEventKind::Moved
                | MouseEventKind::Entered
                | MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } => {
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
                                s.extended.icon_button_view.focus(false);
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
                    // Modal/pop-up scopes were handled above. Release the
                    // host's own story buttons before body adapters that may
                    // consume an otherwise unrelated pointer-up.
                    let mut header = None;
                    for id in s.button_hits.iter().copied().filter(|id| *id <= 6) {
                        if s.buttons.get_mut(&id).is_some_and(|b| b.pointer_up_at(p)) {
                            header = Some(id);
                        }
                    }
                    if let Some(id) = header {
                        s.activate_from_pointer(id, renderer, composition_guarded);
                        return true;
                    }
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
                        let was_open =
                            renderer == "command-palette" && s.extended.command.combo.open;
                        let handled = s
                            .extended
                            .pointer_up(renderer, p, hit, &mut |event| events.push(event));
                        if renderer == "command-palette"
                            && !was_open
                            && s.extended.command.combo.open
                        {
                            s.open_command_palette(text);
                        }
                        if renderer == "command-palette"
                            && was_open
                            && !s.extended.command.combo.open
                        {
                            s.close_command_palette();
                        }
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
            s.input_changed(renderer);
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
            .filter_map(|id| state.buttons.get(&id).and_then(ButtonModel::layout))
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
    fn component_usage_uses_the_compiled_registry_api_without_renderer_fallbacks() {
        for entry in entries().iter().filter(|e| e.get("kind") == "component") {
            assert!(
                entry.get("api").starts_with("tekes_ui_controls::"),
                "{}",
                entry.slug()
            );
            assert_eq!(api_reference(entry), entry.get("api"));
            assert_eq!(entry.0["api_bindings"][0], entry.get("api"));
            let summary = contract_summary(entry);
            assert_eq!(summary.len(), 3, "{}", entry.slug());
            assert!(summary[0].starts_with("Design-required states: rest"));
            assert!(summary[1].contains("runtime and native acceptance remain unverified"));
            assert!(
                summary[2].contains("runtime token dependencies remain unverified")
                    || summary[2].contains("Runtime token dependencies remain unverified")
            );
        }
        use super::super::Gallery;
        use tekes_ui_render::Renderer;
        use tekes_ui_tokens::Theme;
        let mut renderer = Renderer::new_headless().expect("actual GPU for registry usage pages");
        let output = std::env::var_os("TEKESUI_REGISTRY_DOC_OUT").map(std::path::PathBuf::from);
        if let Some(path) = &output {
            std::fs::create_dir_all(path).unwrap();
        }
        let mut manifest = Vec::new();
        for entry in entries().iter().filter(|e| e.get("kind") == "component") {
            for theme in Theme::ALL {
                for (width, height, scale, type_scale) in
                    [(1440_u32, 960_u32, 1_f32, 0), (720, 480, 2., 2)]
                {
                    let mut gallery = Gallery::new();
                    gallery.set_system_theme(theme);
                    gallery.set_reduced_motion(true);
                    gallery.text.set_scale(scale);
                    gallery.workbench.text_scale = type_scale;
                    let route = format!("{}/docs", entry.slug());
                    assert!(gallery.open_route(&route));
                    gallery.set_view_size(width as f32, height as f32);
                    state_capture::stabilize_document(&mut gallery);
                    let target = gallery.page_scroll_range().max(0.);
                    let (offset, range) = state_capture::settle_scroll(&mut gallery, 1.0, target);
                    assert!(
                        (offset - range).abs() < 0.5,
                        "docs bottom must be reached: {route}"
                    );
                    let mut scene = Scene::new();
                    gallery.build_scene(&mut scene);
                    let pixels = (
                        (width as f32 * scale) as u32,
                        (height as f32 * scale) as u32,
                    );
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            gallery.atlas_mut(),
                            pixels,
                            scale,
                            Gallery::background(theme),
                        )
                        .unwrap();
                    assert!(
                        rgba.chunks_exact(4).any(|p| p != &rgba[0..4]),
                        "blank {route}"
                    );
                    let file = format!(
                        "{}-{}-{}-{width}x{height}@{scale}x.png",
                        entry.slug(),
                        if theme.is_dark() { "dark" } else { "light" },
                        if theme.is_increased() {
                            "increased"
                        } else {
                            "standard"
                        }
                    );
                    if let Some(output) = &output {
                        let mut encoder = png::Encoder::new(
                            std::fs::File::create(output.join(&file)).unwrap(),
                            pixels.0,
                            pixels.1,
                        );
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&rgba)
                            .unwrap();
                    }
                    manifest.push(serde_json::json!({"route":route, "api":entry.get("api"), "theme":format!("{theme:?}"), "points":[width,height], "pixels":[pixels.0,pixels.1], "scale":scale, "text_scale_index":type_scale, "file":file, "checks":["registry API binding", "actual Gallery docs route", "stable document extent", "actual bottom scroll", "real GPU render"], "experience":"requires visual review; nonblank is not acceptance"}));
                }
            }
        }
        if let Some(output) = &output {
            std::fs::write(
                output.join("manifest.json"),
                serde_json::to_vec_pretty(&manifest).unwrap(),
            )
            .unwrap();
        }
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

    fn reveal_registered_collection_row(
        gallery: &mut super::super::Gallery,
        i: usize,
        row_id: usize,
    ) {
        let row = gallery
            .workbench
            .state(i)
            .logical_focus
            .iter()
            .find(|target| target.id == row_id && target.mounted && target.eligible)
            .copied()
            .unwrap_or_else(|| panic!("{row_id} has a mounted production layout frame"));
        let visible = gallery
            .viewport
            .geometry()
            .visible_viewport_rect()
            .expect("Gallery viewport is visible");
        let desired_offset = gallery.viewport.offset.y + row.frame.min_y() - visible.min_y() - 8.0;
        gallery.scroll_page_to(desired_offset.max(0.0));
        gallery.build_scene(&mut Scene::new());
        assert!(
            gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .any(|(id, _)| *id == row_id),
            "scrolling the measured row frame brings its production hit into view"
        );
    }

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
        // Interactive fixtures start at zero; tests explicitly advance time.
        if t.motion_context().is_none() {
            t.set_motion_enabled(true);
            t.set_motion_context(Duration::ZERO, false);
        }
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
    fn issue79_snapshot_clock_is_explicit_and_restores_the_caller() {
        let i = index("toast");
        let mut text = super::super::text::TextEngine::new();
        for clock in [
            None,
            Some(tekes_ui_controls::ControlMotionContext {
                now: Duration::from_secs(42),
                reduced_motion: false,
            }),
        ] {
            text.replace_fixture_motion_context(clock);
            let mut w = Workbench::fixture();
            let mut st = super::super::static_text::StaticText::new(None);
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut st,
            };
            ctx.text.begin_frame();
            let mut first = Scene::new();
            w.build(i, &mut ctx, Rect::new(312., 70., 1000., 820.), &mut first);
            assert_eq!(ctx.text.motion_context(), clock);
            ctx.text.begin_frame();
            let mut second = Scene::new();
            w.build(i, &mut ctx, Rect::new(312., 70., 1000., 820.), &mut second);
            assert_eq!(ctx.text.motion_context(), clock);
            assert_eq!(first, second);
            assert_eq!(w.state(i).extended.toast.remaining_ms(), 5000);
        }
    }

    #[test]
    fn issue79_real_gallery_retains_toast_and_advances_the_frame_clock() {
        let i = index("toast");
        let mut gallery = super::super::Gallery::new();
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.set_reduced_motion(true);
        gallery.build_scene(&mut Scene::new());
        assert_eq!(
            gallery.workbench.state(i).extended.toast.remaining_ms(),
            5000
        );
        assert!(gallery.text.motion_frame_requested());
        assert!(gallery.animate(0.125));
        gallery.build_scene(&mut Scene::new());
        assert_eq!(
            gallery.workbench.state(i).extended.toast.remaining_ms(),
            4875
        );
        gallery.build_scene(&mut Scene::new());
        assert_eq!(
            gallery.workbench.state(i).extended.toast.remaining_ms(),
            4875
        );
    }

    #[test]
    fn narrow_large_text_state_matrix_allocates_wrapped_caption_and_real_control_extent() {
        use tekes_ui_scene::Primitive;

        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let bounds = Rect::new(12.0, 24.0, 360.0, 1800.0);
        let columns = 2;
        let focus_gutter = -super::ControlFrame::new(Rect::new(0.0, 0.0, 1.0, 1.0), None)
            .focus_bounds(text.backing_scale())
            .min_x();
        let layout_bounds = bounds.inset(focus_gutter);
        let cell_width = layout_bounds.size.width / columns as f32 - 16.0;
        let caption = text.proposal_label(
            "Checked · focus-visible",
            proposal::TextRole::UiCaption,
            Some(cell_width),
        );
        assert!(
            caption.lines.len() > 1,
            "fixture must exercise wrapped state labels"
        );
        let checkbox_height = Checkbox::measure(
            &mut text,
            "Notifications",
            cell_width,
            proposal::TextRole::UiBody,
        )
        .height;
        let fields = [
            (caption.height as f32, checkbox_height),
            (caption.height as f32, checkbox_height),
            (caption.height as f32, checkbox_height),
        ];
        let (cells, bottom) = matrix_cell_geometries(layout_bounds, columns, 82.0, &fields);
        for cell in &cells {
            assert!(cell.control.max_y() + 8.0 <= cell.caption.min_y());
            assert!(cell.caption.max_y() <= bottom);
            assert!(cell.control.min_x() >= layout_bounds.min_x());
            assert!(cell.control.max_x() <= layout_bounds.max_x());
            assert!(cell.caption.max_x() <= layout_bounds.max_x());
            let focus_bounds = super::ControlFrame::new(cell.control, Some(bounds))
                .focus_bounds(text.backing_scale());
            assert!(focus_bounds.min_x() >= bounds.min_x());
            assert!(focus_bounds.max_x() <= bounds.max_x());
            assert!(focus_bounds.min_y() >= bounds.min_y());
            assert!(focus_bounds.max_y() <= bounds.max_y());
        }
        assert!(cells[0].caption.max_y() <= cells[2].control.min_y());
        assert!(cells[0].caption.size.height >= caption.height as f32);

        let mut scene = Scene::new();
        let mut checkbox = Checkbox::default();
        let palette = tekes_ui_tokens::Palette::new(tekes_ui_tokens::Theme::LIGHT, 0)
            .expect("registered Gallery brand");
        let layout = checkbox.render(
            &mut scene,
            &mut text,
            "Notifications",
            cells[0].control,
            Some(bounds),
            palette,
            proposal::TextRole::UiBody,
        );
        let first_line = text.first_line_height(
            "Notifications",
            proposal::TextRole::UiBody,
            layout.label.size.width,
        );
        assert!(layout.label.size.width > 0.0);
        assert!(layout.label.size.height >= first_line);
        assert!(scene.primitives().iter().any(|primitive| matches!(
            primitive,
            Primitive::Glyphs(run) if !run.glyphs.is_empty()
        )));
        let text_center_y = layout.label.min_y() + first_line * 0.5;
        assert!(
            (layout.control.center().y - text_center_y).abs() < 0.1,
            "checkbox mark and first label line share a vertical center"
        );
    }

    #[test]
    fn canonical_supported_stories_keep_distinct_actions_disjoint_at_large_text() {
        let mut failures = Vec::new();
        let mut cases = Vec::new();
        for theme in tekes_ui_tokens::Theme::ALL {
            for (width, type_scale) in [(360., 1.5), (1000., 1.)] {
                for (i, entry) in entries().iter().enumerate().filter(|(_, e)| !e.planned()) {
                    let supported = [
                        "docs",
                        "playground",
                        "states",
                        "behaviors",
                        "edge-cases",
                        "variants",
                    ]
                    .into_iter()
                    .filter(|story| entry.supports_story(story))
                    .collect::<Vec<_>>();
                    for story in &supported {
                        let mut w = Workbench::default();
                        w.select_route(i, &format!("{}/{story}", entry.slug()));
                        let mut text = super::super::text::TextEngine::new();
                        text.set_scale(1.);
                        text.set_type_scale(type_scale);
                        text.set_motion_enabled(true);
                        text.set_motion_context(Duration::ZERO, false);
                        text.begin_frame();
                        let mut static_text = super::super::static_text::StaticText::new(None);
                        let mut ctx = PageCtx {
                            theme,
                            text: &mut text,
                            static_text: &mut static_text,
                        };
                        let mut scene = Scene::new();
                        w.build(i, &mut ctx, Rect::new(0., 0., width, 1800.), &mut scene);
                        assert!(!scene.is_empty(), "{}/{story} is blank", entry.slug());
                        let state = w.state(i);
                        let hits = state
                            .hits
                            .iter()
                            .filter(|(id, _)| *id >= 10)
                            .collect::<Vec<_>>();
                        for (n, (id, rect)) in hits.iter().enumerate() {
                            for (other_id, other) in hits.iter().skip(n + 1) {
                                if id != other_id
                                    && rect
                                        .intersection(other)
                                        .is_some_and(|r| r.size.width > 0.1 && r.size.height > 0.1)
                                {
                                    failures.push(format!("{}/{story}/{width}/{type_scale}: {id} {rect:?} overlaps {other_id} {other:?}", entry.slug()));
                                }
                            }
                        }
                        cases.push(serde_json::json!({"slug":entry.slug(), "renderer":entry.renderer(), "story":story, "theme":format!("{theme:?}"), "preview_width":width, "text_scale":type_scale, "supported_stories":supported, "invariants":["scene_nonempty", "distinct_action_hit_regions_disjoint"], "evidence":"deterministic scene geometry", "matrix_is_simulated":*story == "states", "event_assertions":false}));
                    }
                }
            }
        }
        if let Some(output) =
            std::env::var_os("TEKESUI_GOLD7_PAGE_OUT").map(std::path::PathBuf::from)
        {
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(
                output.join("canonical-story-geometry.json"),
                serde_json::to_vec_pretty(&cases).unwrap(),
            )
            .unwrap();
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn settings_disabled_error_and_reset_controls_drive_real_models() {
        let i = index("settings");
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let content = Rect::new(0., 0., 360., 1400.);
        build_in(&mut workbench, i, &mut text, content);
        let activate = |w: &mut Workbench, text: &mut super::super::text::TextEngine, id| {
            let center = w
                .state(i)
                .hits
                .iter()
                .find(|(candidate, _)| *candidate == id)
                .unwrap()
                .1
                .center();
            w.handle(i, &mouse(center, true), text);
            w.handle(i, &mouse(center, false), text);
            build_in(w, i, text, content);
        };
        activate(&mut workbench, &mut text, 12);
        assert!(workbench.state(i).error);
        activate(&mut workbench, &mut text, 10);
        assert!(workbench.state(i).disabled);
        let notifications = workbench.state(i).checkbox.checked;
        activate(&mut workbench, &mut text, 25);
        activate(&mut workbench, &mut text, 20);
        assert_eq!(workbench.state(i).checkbox.checked, notifications);
        assert_eq!(workbench.state(i).phase, Phase::Ready);
        assert!(workbench.state(i).buttons[&20].state.disabled);
        assert!(workbench.state(i).buttons[&25].state.disabled);
        activate(&mut workbench, &mut text, 0);
        assert!(!workbench.state(i).disabled);
        assert!(!workbench.state(i).error);
        assert_eq!(workbench.state(i).phase, Phase::Ready);
        let state = workbench.state(i);
        let notifications = state.buttons[&25].layout().unwrap().frame;
        let save = state.buttons[&20].layout().unwrap().frame;
        assert!(notifications.max_y() + 11.9 <= save.min_y());
        assert!(save.max_y() + 11.9 <= state.phase_frame.unwrap().min_y());
    }

    #[test]
    fn canonical_pattern_pages_render_real_gpu_frames_in_four_themes() {
        use super::super::Gallery;
        use tekes_ui_render::Renderer;
        use tekes_ui_tokens::Theme;
        let mut renderer =
            Renderer::new_headless().expect("actual GPU required for canonical page checks");
        let output = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT").map(std::path::PathBuf::from);
        if let Some(output) = &output {
            std::fs::create_dir_all(output).unwrap();
        }
        for theme in Theme::ALL {
            for (width, height, type_scale) in [(1440_u32, 960_u32, 0), (720, 480, 2)] {
                for kind in [
                    "settings",
                    "form",
                    "master",
                    "filter",
                    "async",
                    "button",
                    "checkbox",
                    "field",
                    "area",
                    "search",
                    "field-meta",
                    "tabs",
                    "menu",
                    "dialog",
                    "select",
                    "disclosure",
                    "icon-button",
                    "link",
                    "slider",
                    "radio-group",
                    "switch",
                    "segmented-control",
                    "tooltip",
                    "popover",
                    "toast",
                    "banner",
                    "status-badge",
                    "progress",
                    "empty-state",
                    "markdown",
                    "code-block",
                    "diff-view",
                ] {
                    let i = index(kind);
                    let mut gallery = Gallery::new();
                    gallery.set_system_theme(theme);
                    gallery.text.set_scale(1.);
                    gallery.set_reduced_motion(true);
                    gallery.workbench.text_scale = type_scale;
                    let has_matrix = [
                        "button",
                        "checkbox",
                        "field",
                        "area",
                        "search",
                        "field-meta",
                        "tabs",
                        "menu",
                        "dialog",
                        "select",
                        "disclosure",
                        "icon-button",
                        "link",
                        "slider",
                        "radio-group",
                        "switch",
                        "segmented-control",
                    ]
                    .contains(&kind);
                    let story = if has_matrix { "states" } else { "playground" };
                    assert!(gallery.open_route(&format!("{}/{story}", entry(i).slug())));
                    gallery.set_view_size(width as f32, height as f32);
                    gallery.build_scene(&mut Scene::new());
                    let positions: &[&str] = if kind == "banner" {
                        &["top", "component", "bottom"]
                    } else {
                        &["top", "bottom"]
                    };
                    for position in positions {
                        if *position == "component" {
                            let range = gallery.page_scroll_range();
                            let step = (gallery.viewport.bounds().size.height * 0.5).max(40.0);
                            let mut revealed = None;
                            for n in 0..=(range / step).ceil() as usize {
                                let offset = (n as f32 * step).min(range);
                                gallery.scroll_page_to(offset);
                                gallery.build_scene(&mut Scene::new());
                                let viewport = gallery.viewport.bounds();
                                let dismiss = gallery
                                    .workbench
                                    .states
                                    .get(&i)
                                    .and_then(|state| state.hits.iter().find(|(id, _)| *id == 20))
                                    .map(|(_, frame)| *frame)
                                    .filter(|frame| viewport.intersection(frame).is_some());
                                if dismiss.is_some() {
                                    revealed = Some(offset);
                                    break;
                                }
                            }
                            gallery.scroll_page_to(revealed.expect(
                                "Banner dismiss target must scroll into the actual viewport",
                            ));
                        } else if *position == "top" {
                            gallery.scroll_page_to(0.0);
                        }
                        if *position == "bottom" {
                            gallery.scroll_page_to(f32::MAX);
                        }
                        let mut scene = Scene::new();
                        gallery.build_scene(&mut scene);
                        let rgba = renderer
                            .render_to_rgba8(
                                &scene,
                                gallery.atlas_mut(),
                                (width, height),
                                1.,
                                Gallery::background(theme),
                            )
                            .unwrap();
                        assert!(
                            rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]),
                            "blank {kind}/{theme:?}/{width}/{position}"
                        );
                        if let Some(output) = &output {
                            let path = output.join(format!(
                                "pattern-{kind}-{}-{}-{width}x{height}-{position}.png",
                                if theme.is_dark() { "dark" } else { "light" },
                                if theme.is_increased() {
                                    "increased"
                                } else {
                                    "standard"
                                }
                            ));
                            let file = std::fs::File::create(path).unwrap();
                            let mut encoder = png::Encoder::new(file, width, height);
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            encoder
                                .write_header()
                                .unwrap()
                                .write_image_data(&rgba)
                                .unwrap();
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn measured_story_feedback_extends_real_scroll_document_without_probe_render() {
        use super::super::Gallery;
        for renderer in [
            "settings", "tabs", "menu", "dialog", "select", "toast", "banner",
        ] {
            let i = index(renderer);
            let mut gallery = Gallery::new();
            gallery.text.set_scale(1.);
            gallery.workbench.text_scale = 2;
            let story = if ["settings", "toast", "banner"].contains(&renderer) {
                "playground"
            } else {
                "states"
            };
            assert!(gallery.open_route(&format!("{}/{story}", entry(i).slug())));
            gallery.set_view_size(720., 480.);
            gallery.build_scene(&mut Scene::new());
            let extent = gallery.page_document_height();
            assert!(extent >= gallery.workbench.measured_document_extent(i));
            gallery.scroll_page_to(f32::MAX);
            gallery.build_scene(&mut Scene::new());
            assert!(gallery.page_scroll_range() > 0.);
            assert!(
                (extent - gallery.page_document_height()).abs() < 0.1,
                "{renderer} extent must stabilize"
            );
            assert!(gallery.viewport.offset.y > 0.);
            if ["toast", "banner"].contains(&renderer) {
                let range = gallery.page_scroll_range();
                let step = (gallery.viewport.bounds().size.height * 0.5).max(40.0);
                let mut target = None;
                for n in 0..=(range / step).ceil() as usize {
                    let offset = (n as f32 * step).min(range);
                    gallery.scroll_page_to(offset);
                    gallery.build_scene(&mut Scene::new());
                    let viewport = gallery.viewport.bounds();
                    let dismiss = gallery
                        .workbench
                        .states
                        .get(&i)
                        .and_then(|state| state.hits.iter().find(|(id, _)| *id == 20))
                        .map(|(_, frame)| *frame)
                        .filter(|frame| viewport.intersection(frame).is_some());
                    if let Some(frame) = dismiss {
                        target = Some((offset, frame));
                        break;
                    }
                }
                let (offset, dismiss) = target.unwrap_or_else(|| {
                    panic!("{renderer} dismiss must be reachable by scrolling within the actual document")
                });
                gallery.scroll_page_to(offset);
                gallery.build_scene(&mut Scene::new());
                let viewport = gallery.viewport.bounds();
                assert!(viewport.intersection(&dismiss).is_some());
                let point = dismiss.center();
                assert!(gallery.handle_input(&mouse(point, true)).redraw);
                assert!(gallery.handle_input(&mouse(point, false)).redraw);
                gallery.build_scene(&mut Scene::new());
                let state = gallery.workbench.states.get(&i).unwrap();
                match renderer {
                    "toast" => assert!(!state.extended.toast.visible),
                    "banner" => assert!(!state.extended.banner.visible),
                    _ => unreachable!(),
                }
                assert!(
                    state
                        .log
                        .back()
                        .is_some_and(|line| line.contains("dismissed"))
                );
                let after_dismiss = gallery.page_document_height();
                gallery.build_scene(&mut Scene::new());
                assert!((after_dismiss - gallery.page_document_height()).abs() < 0.1);
            }
            gallery
                .workbench
                .select_route(i, &format!("{}/playground", entry(i).slug()));
            if story != "playground" {
                assert_eq!(gallery.workbench.measured_document_extent(i), 820.);
            }
        }
    }

    #[test]
    fn static_state_matrices_restore_the_live_clock_without_overlay_frame_loops() {
        use tekes_ui_controls::ControlMotionContext;
        use tekes_ui_scene::Primitive;
        for renderer in [
            "tabs",
            "menu",
            "dialog",
            "select",
            "button",
            "switch",
            "radio-group",
        ] {
            let i = index(renderer);
            let mut w = Workbench::default();
            w.select_route(i, &format!("{}/states", entry(i).slug()));
            let mut text = super::super::text::TextEngine::new();
            text.set_scale(1.);
            text.set_motion_enabled(true);
            text.set_motion_context(Duration::from_secs(3), false);
            build(&mut w, i, &mut text);
            for second in 5..8 {
                let clock = ControlMotionContext {
                    now: Duration::from_secs(second),
                    reduced_motion: false,
                };
                text.set_motion_context(clock.now, clock.reduced_motion);
                let scene = build(&mut w, i, &mut text);
                assert_eq!(
                    text.motion_context(),
                    Some(clock),
                    "{renderer} must restore exact host clock"
                );
                assert!(
                    !text.motion_frame_requested(),
                    "{renderer} static matrix must not schedule animation forever"
                );
                if ["menu", "dialog", "select"].contains(&renderer) {
                    assert!(
                        scene
                            .primitives()
                            .iter()
                            .any(|p| matches!(p, Primitive::Shadow(_))),
                        "{renderer} native-motion context must paint actual overlay surface"
                    );
                }
            }
        }
    }

    #[test]
    fn extended_natural_extent_stabilizes_for_twenty_builds_and_can_shrink_after_reflow() {
        use super::super::Gallery;
        let mut observed_shrink = false;
        for renderer in [
            "list",
            "tree",
            "data-table",
            "chart",
            "markdown",
            "code-block",
            "diff-view",
        ] {
            let i = index(renderer);
            let mut gallery = Gallery::new();
            gallery.workbench.text_scale = 2;
            assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
            if renderer == "markdown" {
                gallery.workbench.state(i).extended.markdown = tekes_ui_controls::extended::Markdown::parse(&"A long mixed-script paragraph verifies natural reflow geometry 项目通知設定 日本語 and keeps meaningful text visible across constrained widths. Repeated content should grow the document, then shrink when the available width expands.\n\n".repeat(8));
            }
            gallery.set_view_size(720., 480.);
            gallery.build_scene(&mut Scene::new());
            gallery.build_scene(&mut Scene::new());
            let narrow = gallery.page_document_height();
            assert!(
                gallery.workbench.state(i).event_log_frame.is_some(),
                "{renderer} retains actual log geometry"
            );
            for _ in 0..20 {
                gallery.build_scene(&mut Scene::new());
                assert!(
                    (narrow - gallery.page_document_height()).abs() < 0.01,
                    "{renderer} extent feeds back into itself"
                );
            }
            gallery.set_view_size(1440., 480.);
            for _ in 0..3 {
                gallery.build_scene(&mut Scene::new());
            }
            let wide = gallery.page_document_height();
            assert!(
                wide <= narrow + 0.01,
                "{renderer} wider reflow may shrink but cannot grow: {narrow} to {wide}"
            );
            observed_shrink |= wide + 1. < narrow;
        }
        assert!(
            observed_shrink,
            "at least one long document genuinely shrinks after wide reflow"
        );
    }

    #[test]
    fn collection_story_headers_and_reset_keep_generic_focus_reveal_geometry() {
        use super::super::Gallery;
        for renderer in ["list", "tree", "data-table", "chart"] {
            let i = index(renderer);
            let mut gallery = Gallery::new();
            assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
            gallery.set_view_size(1440., 960.);
            gallery.build_scene(&mut Scene::new());
            let mut visited_reset = false;
            for _ in 0..20 {
                gallery.handle_input(&key(48, true));
                gallery.build_scene(&mut Scene::new());
                let Some(id) = gallery.workbench.state(i).focus else {
                    continue;
                };
                if id > 10 {
                    continue;
                }
                let expected = gallery.workbench.state(i).buttons[&id]
                    .layout()
                    .unwrap()
                    .frame;
                assert_eq!(
                    gallery.workbench.focused_control_frame(i),
                    Some(expected),
                    "{renderer} actual Tab header/reset id{id} uses Button reveal geometry"
                );
                if id == 0 {
                    gallery.handle_input(&key(36, true));
                    assert!(
                        gallery
                            .workbench
                            .state(i)
                            .log
                            .iter()
                            .any(|line| line.contains("reset: initial fixture")),
                        "{renderer} Enter activates Reset"
                    );
                    visited_reset = true;
                    break;
                }
            }
            assert!(visited_reset, "{renderer} Reset remains keyboard reachable");
        }
    }

    #[test]
    fn logical_tab_targets_keep_full_frames_when_the_preview_clip_hides_them() {
        let mut gallery = super::super::Gallery::new();
        let i = index("settings");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.workbench.text_scale = 2;
        gallery.set_view_size(720., 240.);
        gallery.build_scene(&mut Scene::new());

        let state = gallery.workbench.state(i);
        let targets = state.logical_tab_frames();
        assert!(targets.iter().any(|target| target.id == 1000));
        let input = targets.iter().find(|target| target.id == 1000).unwrap();
        assert!(input.frame.max_y() > state.button_clip.unwrap().max_y());
        assert!(!state.hits.iter().any(|(id, _)| *id == 1000));
        assert!(
            targets
                .windows(2)
                .all(|pair| pair[0].id != 1000 || pair[1].id == 20 || pair[1].id == 25)
        );
    }

    #[test]
    fn roving_group_keeps_one_focus_target_and_falls_back_when_selection_is_empty() {
        let mut state = StoryState::for_entry(entry(index("tabs")));
        let frame = Rect::new(0., 0., 40., 30.);
        state.register_logical_focus(30, frame, true, true, Some(1), false);
        state.register_logical_focus(31, frame.offset(48., 0.), true, true, Some(1), false);
        state.register_logical_focus(32, frame.offset(96., 0.), true, true, Some(1), false);
        assert_eq!(state.logical_tab_ids(), vec![30]);

        state.register_logical_focus(31, frame.offset(48., 0.), true, true, Some(1), true);
        state.register_logical_focus(32, frame.offset(96., 0.), true, true, Some(1), true);
        assert_eq!(state.logical_tab_ids(), vec![31]);

        state.focus = Some(32);
        assert_eq!(state.logical_tab_ids(), vec![32]);
    }

    #[test]
    fn tab_reveals_full_offscreen_text_input_and_shift_tab_reveals_previous_control() {
        use super::super::Gallery;
        let mut gallery = Gallery::new();
        let i = index("settings");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.workbench.text_scale = 2;
        gallery.set_view_size(720., 240.);
        gallery.build_scene(&mut Scene::new());

        let ids = gallery.workbench.state(i).logical_tab_ids();
        let input_index = ids.iter().position(|id| *id == 1000).unwrap();
        let state = gallery.workbench.state(i);
        let clip_bottom = state.button_clip.unwrap().max_y();
        assert!(
            state
                .logical_focus
                .iter()
                .any(|target| target.id == 1000 && target.frame.max_y() > clip_bottom)
        );
        for _ in 0..=input_index {
            gallery.handle_input(&key(48, true));
            gallery.build_scene(&mut Scene::new());
        }
        assert_eq!(gallery.workbench.state(i).focus, Some(1000));
        assert!(
            gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .any(|(id, _)| *id == 1000),
            "the offscreen input is brought into the real pointer-visible viewport"
        );

        gallery.handle_input(&key_with_modifiers(
            48,
            true,
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
        ));
        gallery.build_scene(&mut Scene::new());
        let previous = gallery.workbench.state(i).focus.unwrap();
        assert_ne!(previous, 1000);
        assert!(
            gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .any(|(id, _)| *id == previous),
            "Shift+Tab reveals the preceding production control"
        );
    }

    #[test]
    fn narrow_large_type_text_area_keeps_a_real_focus_frame_and_tabs_into_view() {
        use super::super::Gallery;
        let mut gallery = Gallery::new();
        let i = index("area");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.workbench.text_scale = 2;
        gallery.set_view_size(720., 240.);
        gallery.build_scene(&mut Scene::new());

        let area = gallery
            .workbench
            .state(i)
            .logical_tab_frames()
            .into_iter()
            .find(|target| target.id == 1000)
            .expect("TextArea is mounted with a real unclipped frame");
        assert!(
            area.frame.size.height >= 100.0,
            "minimum usable TextArea frame: {:?}",
            area.frame
        );
        assert!(
            !gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .any(|(id, _)| *id == 1000),
            "a clipped TextArea is keyboard mounted without a pointer hit"
        );

        gallery.navigation_search.focus_field();
        gallery.handle_input(&Event::TextCommand("insertTab:".into()));
        gallery.handle_input(&key(48, true));
        for _ in 0..gallery.workbench.tab_stop_ids(i).len() + 1 {
            if gallery.workbench.focused_control_id(i) == Some(1000) {
                break;
            }
            gallery.handle_input(&Event::TextCommand("insertTab:".into()));
            gallery.build_scene(&mut Scene::new());
        }
        assert_eq!(gallery.workbench.focused_control_id(i), Some(1000));
        gallery.build_scene(&mut Scene::new());
        assert!(
            gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .any(|(id, _)| *id == 1000),
            "Tab scrolls the real TextArea fully into the preview"
        );
    }

    #[test]
    fn disabled_logical_target_is_skipped() {
        let mut state = StoryState::for_entry(entry(index("button")));
        state.register_logical_focus(20, Rect::new(0., 0., 80., 36.), false, true, None, true);
        state.register_logical_focus(21, Rect::new(0., 48., 80., 36.), true, true, None, true);
        assert_eq!(state.logical_tab_ids(), vec![21]);
    }

    #[test]
    fn rendered_disabled_button_is_removed_from_the_tab_sequence() {
        use super::super::Gallery;
        let mut gallery = Gallery::new();
        let i = index("button");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.workbench.state(i).disabled = true;
        gallery.build_scene(&mut Scene::new());
        assert!(
            gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .any(|(id, _)| *id == 20)
        );
        assert!(!gallery.workbench.state(i).logical_tab_ids().contains(&20));
        for _ in 0..gallery.workbench.state(i).logical_tab_ids().len() {
            gallery.handle_input(&key(48, true));
            assert_ne!(gallery.workbench.state(i).focus, Some(20));
            gallery.build_scene(&mut Scene::new());
        }
    }

    #[test]
    fn list_theme_retains_actual_gallery_selection_hover_focus_and_semantics() {
        use tekes_ui_platform_macos::AccessibilityRole;
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_motion_enabled(true);
        text.set_motion_context(Duration::ZERO, true);
        let i = index("list");
        click(&mut w, i, 31, &mut text);
        w.handle(i, &key(125, true), &mut text);
        build(&mut w, i, &mut text);
        assert_eq!(w.state(i).extended.list.selected, Some(2));
        let selected = w
            .state(i)
            .visible_for_id(32)
            .expect("real visible List row");
        w.handle(i, &move_pointer(selected.center()), &mut text);
        let content = Rect::new(312., 70., 1000., 820.);
        let mut original_frame = None;
        for theme in Theme::ALL {
            text.begin_frame();
            let mut static_text = super::super::static_text::StaticText::new(None);
            let mut ctx = PageCtx {
                theme,
                text: &mut text,
                static_text: &mut static_text,
            };
            let mut scene = Scene::new();
            w.build_clipped(i, &mut ctx, content, content, &mut scene);
            assert!(scene.composition_error().is_none());
            assert_eq!(w.state(i).extended.list.selected, Some(2));
            assert_eq!(w.state(i).focus, Some(32));
            let row = w.state(i).visible_for_id(32).unwrap();
            if let Some(original) = original_frame {
                assert_eq!(row, original);
            } else {
                original_frame = Some(row);
            }
            let palette = Palette::new(theme, w.brand).unwrap();
            let (surface_index, surface) = scene
                .primitives()
                .iter()
                .enumerate()
                .find_map(|(index, p)| match p {
                    Primitive::Quad(q) if q.rect == row => Some((index, q)),
                    _ => None,
                })
                .expect("own actual Gallery selected row ground");
            assert_eq!(surface.fill, palette.color("row-selection-hover").unwrap());
            let copies: Vec<_> = scene.primitives()[surface_index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if !run.glyphs.is_empty()
                            && run.glyphs.iter().all(|g| row.contains(g.origin)) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !copies.is_empty(),
                "actual Gallery row glyphs, not a header or caption"
            );
            assert!(
                copies
                    .iter()
                    .all(|run| run.color == palette.color("primary-text").unwrap())
            );
            assert_eq!((copies[0].color.a, surface.fill.a), (1., 1.));
            let luminance = |color: tekes_ui_tokens::Rgba| {
                let c = color.to_linear();
                0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
            };
            let (a, b) = (luminance(copies[0].color), luminance(surface.fill));
            assert!(
                (a.max(b) + 0.05) / (a.min(b) + 0.05)
                    >= if theme.is_increased() { 7. } else { 4.5 }
            );
            assert!(scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.fill.a == 0. && q.border_color == palette.color("border-focused").unwrap()
                    && q.rect.intersection(&row).is_some()
            )));
            let nodes = w.accessibility_nodes(i);
            let list = nodes
                .iter()
                .find(|node| node.role == AccessibilityRole::List)
                .expect("real List semantic group");
            let rows: Vec<_> = nodes
                .iter()
                .filter(|node| {
                    node.role == AccessibilityRole::ListItem && node.parent == Some(list.key)
                })
                .collect();
            assert_eq!(rows.len(), 5);
            assert_eq!(rows.iter().filter(|row| row.selected).count(), 1);
            let selected = rows.iter().find(|row| row.selected).unwrap();
            assert_eq!(selected.label, "Images");
            assert!(selected.focused && selected.enabled);
        }
        for (code, selected) in [(115, 0), (119, 4)] {
            w.handle(i, &key(code, true), &mut text);
            assert_eq!(w.state(i).extended.list.selected, Some(selected));
            assert_eq!(w.state(i).focus, Some(30 + selected));
        }
    }

    #[test]
    fn tree_theme_retains_actual_gallery_disclosure_hierarchy_and_row_ink() {
        use tekes_ui_platform_macos::AccessibilityRole;
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_motion_enabled(true);
        text.set_motion_context(Duration::ZERO, true);
        let i = index("tree");
        click(&mut w, i, 31, &mut text);
        w.handle(i, &key(123, true), &mut text);
        assert_eq!(w.state(i).extended.tree.selected, Some(0));
        let content = Rect::new(312., 70., 1000., 820.);
        let mut original_marks = [None, None];
        for theme in Theme::ALL {
            for expanded in [false, true] {
                w.handle(i, &key(if expanded { 124 } else { 123 }, true), &mut text);
                let root = w
                    .state(i)
                    .visible_for_id(30)
                    .expect("real visible Tree root");
                w.handle(i, &move_pointer(root.center()), &mut text);
                text.begin_frame();
                let mut static_text = super::super::static_text::StaticText::new(None);
                let mut ctx = PageCtx {
                    theme,
                    text: &mut text,
                    static_text: &mut static_text,
                };
                let mut scene = Scene::new();
                w.build_clipped(i, &mut ctx, content, content, &mut scene);
                assert!(scene.composition_error().is_none());
                assert_eq!(w.state(i).extended.tree.selected, Some(0));
                assert_eq!(w.state(i).focus, Some(30));
                assert_eq!(w.state(i).extended.tree.nodes()[0].expanded, expanded);
                let row = w.state(i).visible_for_id(30).unwrap();
                let palette = Palette::new(theme, w.brand).unwrap();
                let (index, ground) = scene
                    .primitives()
                    .iter()
                    .enumerate()
                    .find_map(|(index, p)| match p {
                        Primitive::Quad(q) if q.rect == row => Some((index, q)),
                        _ => None,
                    })
                    .expect("actual own Tree root ground");
                assert_eq!(ground.fill, palette.color("row-selection-hover").unwrap());
                let labels: Vec<_> = scene.primitives()[index + 1..]
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if !run.glyphs.is_empty()
                                && run.glyphs.iter().all(|g| row.contains(g.origin)) =>
                        {
                            Some(run)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!labels.is_empty());
                let ink = palette.color("primary-text").unwrap();
                assert!(labels.iter().all(|run| run.color == ink));
                let slot = proposal::metric("size", "size-inline-action").unwrap()
                    + 2. * proposal::metric("spacing", "space-relaxed").unwrap();
                let marks: Vec<_> = scene.primitives()[index + 1..]
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Quad(q)
                            if q.rect.min_x() >= row.min_x()
                                && q.rect.max_x() <= row.min_x() + slot
                                && q.rect.min_y() >= row.min_y()
                                && q.rect.max_y() <= row.max_y() =>
                        {
                            Some(q)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    !marks.is_empty(),
                    "actual production Gallery chevron, not row ground/focus or label glyphs"
                );
                assert!(marks.iter().all(|mark| mark.fill == ink));
                let geometry: Vec<_> = marks.iter().map(|mark| mark.rect).collect();
                let original = &mut original_marks[usize::from(expanded)];
                if let Some(previous) = original.as_ref() {
                    assert_eq!(&geometry, previous);
                } else {
                    *original = Some(geometry);
                }
                assert_eq!(
                    (labels[0].color.a, marks[0].fill.a, ground.fill.a),
                    (1., 1., 1.)
                );
                let luminance = |color: tekes_ui_tokens::Rgba| {
                    let c = color.to_linear();
                    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                };
                let contrast = |ink: tekes_ui_tokens::Rgba| {
                    let (a, b) = (luminance(ink), luminance(ground.fill));
                    (a.max(b) + 0.05) / (a.min(b) + 0.05)
                };
                assert!(contrast(labels[0].color) >= if theme.is_increased() { 7. } else { 4.5 });
                assert!(
                    contrast(marks[0].fill) >= 3.,
                    "necessary enabled branch disclosure mark"
                );
                assert!(scene.primitives().iter().any(|p|matches!(p,
                    Primitive::Quad(q) if q.fill.a==0. && q.border_color==palette.color("border-focused").unwrap() && q.rect.intersection(&row).is_some()
                )));
                let nodes = w.accessibility_nodes(i);
                let group = nodes
                    .iter()
                    .find(|node| node.role == AccessibilityRole::Tree)
                    .unwrap();
                let rows: Vec<_> = nodes
                    .iter()
                    .filter(|node| matches!(node.role, AccessibilityRole::TreeItem { .. }))
                    .collect();
                assert_eq!(rows.len(), if expanded { 3 } else { 1 });
                let root = rows.iter().find(|node| node.label == "Workspace").unwrap();
                assert_eq!(root.role, AccessibilityRole::TreeItem { level: 0 });
                assert_eq!(root.parent, Some(group.key));
                assert_eq!(root.expanded, Some(expanded));
                assert!(root.selected && root.focused && root.enabled);
                for child in rows.iter().filter(|node| node.label != "Workspace") {
                    assert_eq!(child.role, AccessibilityRole::TreeItem { level: 1 });
                    assert_eq!(child.parent, Some(root.key));
                    assert!(!child.selected);
                    assert_eq!(child.expanded, None);
                }
            }
        }
        assert_ne!(
            original_marks[0], original_marks[1],
            "original collapsed and expanded chevrons differ"
        );
    }

    #[test]
    fn gallery_wholly_offscreen_collections_reveal_home_and_end_without_pointer_hits() {
        use super::super::Gallery;
        for renderer in ["list", "tree", "data-table", "chart"] {
            let i = index(renderer);
            let mut gallery = Gallery::new();
            gallery.text.set_scale(1.);
            assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
            gallery.set_view_size(720., 240.);
            gallery.build_scene(&mut Scene::new());
            let first_row_id = if renderer == "tree" { 31 } else { 30 };
            reveal_registered_collection_row(&mut gallery, i, first_row_id);
            let (_, frame) = gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .find(|(id, _)| {
                    if renderer == "tree" {
                        *id == 31
                    } else {
                        (30..40).contains(id)
                    }
                })
                .copied()
                .expect("initial pointer-visible logical item");
            gallery.handle_input(&mouse(frame.center(), true));
            gallery.handle_input(&mouse(frame.center(), false));
            for key_code in [119, 115] {
                gallery.scroll_page_to(f32::MAX);
                gallery.build_scene(&mut Scene::new());
                assert!(
                    !gallery
                        .workbench
                        .state(i)
                        .hits
                        .iter()
                        .any(|(id, _)| (30..40).contains(id)),
                    "{renderer} entire collection is outside pointer viewport"
                );
                let before = gallery.viewport.offset.y;
                assert!(
                    gallery.workbench.focused_control_frame(i).is_some(),
                    "{renderer} keeps valid logical focus geometry when clipped"
                );
                gallery.handle_input(&key(key_code, true));
                assert!(
                    gallery.viewport.offset.y < before,
                    "{renderer} logical Home/End must reveal whole offscreen control"
                );
                let focused = gallery
                    .workbench
                    .state(i)
                    .focus
                    .expect("logical focus retained");
                gallery.build_scene(&mut Scene::new());
                assert!(
                    gallery
                        .workbench
                        .state(i)
                        .hits
                        .iter()
                        .any(|(id, _)| *id == focused),
                    "{renderer} revealed logical focus is pointer-visible"
                );
            }
        }
    }

    #[test]
    fn gallery_collection_end_reveals_logical_rows_beyond_pointer_clip() {
        use super::super::Gallery;
        for renderer in ["list", "tree", "data-table"] {
            let i = index(renderer);
            let mut gallery = Gallery::new();
            gallery.text.set_scale(1.);
            assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
            gallery.set_view_size(720., 240.);
            gallery.build_scene(&mut Scene::new());
            let first_row_id = if renderer == "tree" { 31 } else { 30 };
            reveal_registered_collection_row(&mut gallery, i, first_row_id);
            let (first_id, first) = gallery
                .workbench
                .state(i)
                .hits
                .iter()
                .find(|(id, _)| {
                    if renderer == "tree" {
                        *id == 31
                    } else {
                        (30..40).contains(id)
                    }
                })
                .copied()
                .expect("one row is pointer-visible");
            let point = first.center();
            gallery.handle_input(&mouse(point, true));
            gallery.handle_input(&mouse(point, false));
            assert_eq!(gallery.workbench.state(i).focus, Some(first_id));
            let end_target = gallery
                .workbench
                .state(i)
                .logical_focus
                .iter()
                .filter(|target| target.mounted && target.eligible)
                .max_by_key(|target| target.id)
                .copied()
                .expect("the production layout registers the collection's final row");
            let visible = gallery
                .viewport
                .geometry()
                .visible_viewport_rect()
                .expect("Gallery viewport is visible");
            let end_was_offscreen = end_target.frame.max_y() <= visible.min_y()
                || end_target.frame.min_y() >= visible.max_y();
            let before = gallery.viewport.offset.y;
            gallery.handle_input(&key(119, true));
            let focus = gallery
                .workbench
                .state(i)
                .focus
                .expect("logical focused row");
            assert!(focus >= first_id, "{renderer} End reaches logical end");
            let frame = gallery
                .workbench
                .focused_control_frame(i)
                .expect("offscreen logical focus has reveal geometry");
            if end_was_offscreen {
                assert!(
                    gallery.viewport.offset.y > before,
                    "{renderer} host must scroll to offscreen logical row: before={before}, after={}, first={first_id}, focus={focus}, frame={frame:?}",
                    gallery.viewport.offset.y
                );
            } else {
                assert!(
                    frame.min_y() >= visible.min_y() && frame.max_y() <= visible.max_y(),
                    "{renderer} does not scroll when End's row is already visible: {frame:?} vs {visible:?}"
                );
            }
            gallery.build_scene(&mut Scene::new());
            assert!(
                gallery
                    .workbench
                    .state(i)
                    .hits
                    .iter()
                    .any(|(id, _)| *id == focus),
                "{renderer} focused row becomes pointer-visible after reveal: {frame:?}"
            );
        }
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
    fn extended_story_headers_switch_through_real_pointer_dispatch() {
        let mut text = super::super::text::TextEngine::new();
        for e in entries()
            .iter()
            .filter(|e| extended::supports(e.renderer()))
        {
            let i = index(e.renderer());
            let mut w = Workbench::default();
            for (story, id, section) in [("states", 3, 2), ("docs", 1, 0), ("playground", 2, 1)] {
                if !e.supports_story(story) {
                    continue;
                }
                click(&mut w, i, id, &mut text);
                assert_eq!(w.state(i).section, section, "{} {story}", e.renderer());
            }
        }
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
        // Start the real 150ms presence transition, then inspect its stable frame.
        build(&mut w, i, &mut t);
        t.set_motion_context(Duration::from_secs(1), false);
        let scene = build(&mut w, i, &mut t);
        let layout = w.brand_preview.dialog_layout().unwrap();
        assert!(layout.visible.is_some());
        let candidate = tekes_ui_tokens::Palette::new(
            tekes_ui_tokens::Theme::LIGHT,
            tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .find(|preset| preset.id == "rose")
                .unwrap()
                .palette_index,
        )
        .expect("registered Gallery brand");
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
        assert!(
            !w.handle(i, &key(48, true), &mut t),
            "the last matrix control yields to the Gallery shell instead of wrapping"
        );
        assert_eq!(w.brand_preview.focus_for_test(), Some(723));
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
        assert_eq!(w.brand_preview.focus_for_test(), Some(722));

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
    fn brand_preview_joins_the_workbench_focus_scope_and_yields_at_its_edges() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("matrix");
        workbench.state(i).section = 1;
        build(&mut workbench, i, &mut text);

        let first = workbench
            .focus_edge(i, false)
            .expect("entering the matrix scope focuses its first real control");
        assert!(first.size.width > 0. && first.size.height > 0.);
        assert_eq!(workbench.brand_preview.focus_for_test(), Some(700));
        assert_eq!(workbench.focused_control_frame(i), Some(first));
        assert!(
            !workbench.brand_preview.tab_would_leave(false),
            "the first live control is not the final target"
        );

        let mut visited = vec![700];
        while workbench.handle(i, &key(48, true), &mut text) {
            let focused = workbench.brand_preview.focus_for_test().unwrap();
            assert!(
                !visited.contains(&focused),
                "matrix Tab order must not wrap at {focused}"
            );
            visited.push(focused);
        }
        assert!(
            visited.len() > 2,
            "the real matrix scope has multiple targets"
        );
        let last_id = *visited.last().unwrap();
        assert!(
            !workbench.handle(i, &key(48, true), &mut text),
            "the final matrix target keeps yielding forward Tab to the Gallery shell"
        );
        assert_eq!(workbench.brand_preview.focus_for_test(), Some(last_id));

        let last = workbench
            .focus_edge(i, true)
            .expect("reverse entry uses the real last matrix control");
        assert_eq!(workbench.brand_preview.focus_for_test(), Some(last_id));
        assert_eq!(workbench.focused_control_frame(i), Some(last));
        assert!(workbench.focus_edge(i, false).is_some());
        assert_eq!(workbench.brand_preview.focus_for_test(), Some(700));
        assert!(
            !workbench.handle(
                i,
                &key_with_modifiers(
                    48,
                    true,
                    Modifiers {
                        shift: true,
                        ..Modifiers::NONE
                    }
                ),
                &mut text,
            ),
            "the first reverse matrix target yields Shift+Tab to the shell"
        );
        workbench.clear_focus(i);
        assert_eq!(workbench.brand_preview.focus_for_test(), None);
        assert_eq!(workbench.focused_control_frame(i), None);
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
    fn checkbox_gallery_theme_switch_keeps_check_state_and_repaints_actual_mark_and_label() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Rgba, Theme};
        fn assert_checkbox(
            scene: &Scene,
            geometry: (Rect, Rect),
            palette: Palette,
            checked: CheckState,
            ground: &str,
            disabled: bool,
            backing: f32,
        ) -> Rect {
            let (row, label) = geometry;
            let side = proposal::metric("size", "c-checkbox-size").unwrap();
            let (index, control) = scene
                .primitives()
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, p)| match p {
                    Primitive::Quad(q)
                        if q.rect.min_x() == row.min_x()
                            && q.rect.size == tekes_ui_scene::Size::new(side, side)
                            && row.intersection(&q.rect) == Some(q.rect) =>
                    {
                        Some((i, *q))
                    }
                    _ => None,
                })
                .expect("actual Checkbox indicator background");
            assert_eq!(control.fill, palette.color(ground).unwrap());
            assert_eq!(
                control.corner_radii,
                tekes_ui_scene::CornerRadii::uniform(
                    proposal::metric("radius", "c-checkbox-radius").unwrap()
                )
            );
            assert_eq!(control.border_color, palette.color("border").unwrap());
            let parent = scene.primitives()[..index]
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a == 1. && q.rect.intersection(&label) == Some(label) =>
                    {
                        Some(q.fill)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    // This Gallery preview is transparent to the real renderer's clear color.
                    // No synthetic background quad is inserted into the complete scene.
                    Gallery::background(palette.theme)
                });
            let marks: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a > 0. && control.rect.intersection(&q.rect) == Some(q.rect) =>
                    {
                        Some(q.fill)
                    }
                    _ => None,
                })
                .collect();
            if checked == CheckState::Unchecked {
                assert!(marks.is_empty());
            } else {
                assert!(!marks.is_empty(), "checked/mixed emits real mark quads");
                if checked == CheckState::Mixed {
                    assert_eq!(marks.len(), 1);
                }
                let role = if disabled {
                    "tertiary-text"
                } else {
                    "control-checked-mark"
                };
                for ink in marks {
                    assert_eq!(ink, palette.color(role).unwrap());
                    if !disabled {
                        assert!(contrast(ink, control.fill) >= 3., "actual mark/own ground");
                    }
                }
            }
            let inks: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| {
                            let a = g.glyph;
                            label
                                .intersection(&Rect::new(
                                    g.origin.x + a.left as f32 / backing,
                                    g.origin.y + a.top as f32 / backing,
                                    a.width as f32 / backing,
                                    a.height as f32 / backing,
                                ))
                                .is_some()
                        }) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !inks.is_empty(),
                "real CoreText label glyphs after this Checkbox paint"
            );
            for ink in inks {
                assert_eq!(
                    ink,
                    palette
                        .color(if disabled {
                            "tertiary-text"
                        } else {
                            "primary-text"
                        })
                        .unwrap()
                );
                if !disabled {
                    assert!(
                        contrast(ink, parent)
                            >= if palette.theme.is_increased() {
                                7.
                            } else {
                                4.5
                            },
                        "actual label/parent ground"
                    );
                }
            }
            control.rect
        }
        fn contrast(a: Rgba, b: Rgba) -> f32 {
            let l = |c: Rgba| {
                let c = c.to_linear();
                0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
            };
            let (a, b) = (l(a), l(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }

        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::System));
        gallery.apply(Command::Contrast(ContrastChoice::System));
        gallery.set_reduced_motion(true);
        gallery.set_view_size(1440., 900.);
        assert!(gallery.open_route("component-checkbox/playground"));
        let i = index("checkbox");
        gallery.build_scene(&mut Scene::new());
        let initial = gallery.workbench.state(i).checkbox.layout().unwrap();
        let background = Point::new(initial.frame.max_x() + 40., initial.frame.min_y());
        for checked in [
            CheckState::Unchecked,
            CheckState::Checked,
            CheckState::Mixed,
        ] {
            gallery.workbench.state(i).checkbox.checked = checked;
            for theme in Theme::ALL {
                gallery.set_system_theme(theme);
                gallery.handle_input(&move_pointer(background));
                let mut scene = Scene::new();
                gallery.build_scene(&mut scene);
                assert_eq!(gallery.theme(), theme);
                let checkbox = &gallery.workbench.state(i).checkbox;
                assert_eq!(checkbox.checked, checked);
                assert!(!checkbox.button.state.hovered);
                assert!(!checkbox.button.state.pressed);
                let layout = checkbox.layout().unwrap();
                assert_eq!(layout, initial);
                assert_checkbox(
                    &scene,
                    (layout.frame, layout.content),
                    theme.into(),
                    checked,
                    if checked == CheckState::Unchecked {
                        "surface-2"
                    } else {
                        "control-checked-ground"
                    },
                    false,
                    gallery.text.scale(),
                );
            }
        }
        let palette: Palette = Theme::DARK_INCREASED.into();
        let point = initial.frame.center();
        gallery.handle_input(&move_pointer(point));
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        assert!(gallery.workbench.state(i).checkbox.button.state.hovered);
        assert_checkbox(
            &scene,
            (initial.frame, initial.content),
            palette,
            CheckState::Mixed,
            "control-checked-hover",
            false,
            gallery.text.scale(),
        );
        gallery.handle_input(&mouse(point, true));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(gallery.workbench.state(i).checkbox.button.state.pressed);
        assert_checkbox(
            &scene,
            (initial.frame, initial.content),
            palette,
            CheckState::Mixed,
            "control-checked-pressed",
            false,
            gallery.text.scale(),
        );
        gallery.handle_input(&mouse(background, false));
        gallery.handle_input(&move_pointer(background));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_eq!(
            gallery.workbench.state(i).checkbox.checked,
            CheckState::Mixed
        );
        gallery.handle_input(&key(36, true));
        gallery.handle_input(&key(36, false));
        assert_eq!(
            gallery.workbench.state(i).checkbox.checked,
            CheckState::Mixed,
            "Enter never toggles"
        );
        gallery.handle_input(&key(49, true));
        assert_eq!(
            gallery.workbench.state(i).checkbox.checked,
            CheckState::Mixed,
            "Space activates on release"
        );
        gallery.handle_input(&key(49, false));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_eq!(
            gallery.workbench.state(i).checkbox.checked,
            CheckState::Checked
        );
        let control = assert_checkbox(
            &scene,
            (initial.frame, initial.content),
            palette,
            CheckState::Checked,
            "control-checked-ground",
            false,
            gallery.text.scale(),
        );
        assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
            && q.rect.center()==control.center() && q.border_color==palette.color("border-focused").unwrap())));
        gallery.workbench.state(i).disabled = true;
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_checkbox(
            &scene,
            (initial.frame, initial.content),
            palette,
            CheckState::Checked,
            "surface-2",
            true,
            gallery.text.scale(),
        );
        assert!(!scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0. && q.rect.center()==control.center())));
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        gallery.handle_input(&key(49, true));
        gallery.handle_input(&key(49, false));
        assert_eq!(
            gallery.workbench.state(i).checkbox.checked,
            CheckState::Checked
        );
        gallery.workbench.state(i).disabled = false;
        gallery.build_scene(&mut Scene::new());
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        assert_eq!(
            gallery.workbench.state(i).checkbox.checked,
            CheckState::Unchecked,
            "matching pointer release toggles Checked to Unchecked"
        );
    }

    #[test]
    fn radio_group_gallery_theme_switch_repaints_own_indicator_and_preserves_roving_selection() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Rgba, Theme};
        fn assert_radio(
            scene: &Scene,
            geometry: (Rect, Rect),
            palette: Palette,
            state: ControlState,
            ground: &str,
            backing: f32,
            clear: Rgba,
        ) {
            let (indicator, label) = geometry;
            let (index, outer) = scene
                .primitives()
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, p)| match p {
                    Primitive::Quad(q) if q.rect == indicator => Some((i, *q)),
                    _ => None,
                })
                .expect("actual radio indicator own background");
            assert_eq!(outer.fill, palette.color(ground).unwrap());
            assert_eq!(
                outer.corner_radii,
                tekes_ui_scene::CornerRadii::uniform(indicator.size.width * 0.5)
            );
            assert_eq!(
                outer.border_color,
                palette
                    .color(if state.disabled {
                        "disabled-text"
                    } else {
                        "border"
                    })
                    .unwrap()
            );
            let marks: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a > 0.
                            && q.rect.min_x() >= indicator.min_x()
                            && q.rect.min_y() >= indicator.min_y()
                            && q.rect.max_x() <= indicator.max_x()
                            && q.rect.max_y() <= indicator.max_y() =>
                    {
                        Some(q.fill)
                    }
                    _ => None,
                })
                .collect();
            if state.selected {
                assert_eq!(marks.len(), 1, "selected radio emits exactly its own dot");
                assert_eq!(
                    marks[0],
                    palette
                        .color(if state.disabled {
                            "disabled-text"
                        } else {
                            "c-radio-checked-mark"
                        })
                        .unwrap()
                );
                if !state.disabled {
                    assert!(
                        contrast(marks[0], outer.fill) >= 3.,
                        "dot/own indicator ground contrast"
                    );
                }
            } else {
                assert!(marks.is_empty(), "unselected radio has no selection dot");
            }
            // Labels inherit actual painted parent ground, or the real renderer clear.
            let parent = scene.primitives()[..index]
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a == 1. && q.rect.intersection(&label) == Some(label) =>
                    {
                        Some(q.fill)
                    }
                    _ => None,
                })
                .unwrap_or(clear);
            let inks: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| {
                            let a = g.glyph;
                            label
                                .intersection(&Rect::new(
                                    g.origin.x + a.left as f32 / backing,
                                    g.origin.y + a.top as f32 / backing,
                                    a.width as f32 / backing,
                                    a.height as f32 / backing,
                                ))
                                .is_some()
                        }) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !inks.is_empty(),
                "actual CoreText radio label glyphs after this indicator"
            );
            for ink in inks {
                assert_eq!(
                    ink,
                    palette
                        .color(if state.disabled {
                            "disabled-text"
                        } else {
                            "primary-text"
                        })
                        .unwrap()
                );
                if !state.disabled {
                    assert!(
                        contrast(ink, parent)
                            >= if palette.theme.is_increased() {
                                7.
                            } else {
                                4.5
                            },
                        "actual label/parent ground contrast"
                    );
                }
            }
        }
        fn contrast(a: Rgba, b: Rgba) -> f32 {
            let l = |c: Rgba| {
                let c = c.to_linear();
                0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
            };
            let (a, b) = (l(a), l(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }

        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::System));
        gallery.apply(Command::Contrast(ContrastChoice::System));
        gallery.set_reduced_motion(true);
        gallery.set_view_size(1440., 900.);
        assert!(gallery.open_route("component-radio-group/playground"));
        let i = index("radio-group");
        gallery.build_scene(&mut Scene::new());
        gallery
            .workbench
            .state(i)
            .extended
            .radio
            .set_enabled(1, false);
        gallery.build_scene(&mut Scene::new());
        let initial = gallery
            .workbench
            .state(i)
            .extended
            .radio_view
            .layout()
            .clone();
        let background = Point::new(initial.frame.max_x() + 40., initial.frame.min_y());
        for theme in Theme::ALL {
            gallery.set_system_theme(theme);
            gallery.handle_input(&move_pointer(background));
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            assert_eq!(gallery.theme(), theme);
            let state = &gallery.workbench.state(i).extended;
            assert_eq!(state.radio.selected, Some(0));
            assert!(!state.radio.is_enabled(1));
            let layout = state.radio_view.layout();
            assert_eq!(layout, &initial);
            for row in 0..3 {
                assert_radio(
                    &scene,
                    (layout.indicators[row], layout.labels[row]),
                    theme.into(),
                    ControlState {
                        selected: row == 0,
                        disabled: row == 1,
                        ..Default::default()
                    },
                    if row == 0 {
                        "c-radio-checked-ground"
                    } else if row == 1 {
                        "surface-2"
                    } else {
                        "surface-0"
                    },
                    gallery.text.scale(),
                    Gallery::background(theme),
                );
            }
        }
        let palette: Palette = Theme::DARK_INCREASED.into();
        let clear = Gallery::background(Theme::DARK_INCREASED);
        let point = initial.rows[0].frame().center();
        gallery.handle_input(&move_pointer(point));
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        assert_radio(
            &scene,
            (initial.indicators[0], initial.labels[0]),
            palette,
            ControlState {
                selected: true,
                ..Default::default()
            },
            "control-checked-hover",
            gallery.text.scale(),
            clear,
        );
        gallery.handle_input(&mouse(point, true));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_radio(
            &scene,
            (initial.indicators[0], initial.labels[0]),
            palette,
            ControlState {
                selected: true,
                ..Default::default()
            },
            "control-checked-pressed",
            gallery.text.scale(),
            clear,
        );
        gallery.handle_input(&mouse(background, false));
        gallery.handle_input(&move_pointer(background));
        assert_eq!(gallery.workbench.state(i).extended.radio.selected, Some(0));
        gallery.handle_input(&key(124, true));
        scene.clear();
        gallery.build_scene(&mut scene);
        let state = &gallery.workbench.state(i).extended;
        assert_eq!(state.radio.selected, Some(2));
        assert_eq!(state.radio_view.focused(), Some(2));
        assert_radio(
            &scene,
            (initial.indicators[2], initial.labels[2]),
            palette,
            ControlState {
                selected: true,
                ..Default::default()
            },
            "c-radio-checked-ground",
            gallery.text.scale(),
            clear,
        );
        assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
            && q.rect.center()==initial.indicators[2].center() && q.border_color==palette.color("border-focused").unwrap())));
        let disabled = initial.rows[1].frame().center();
        gallery.handle_input(&mouse(disabled, true));
        gallery.handle_input(&mouse(disabled, false));
        assert_eq!(gallery.workbench.state(i).extended.radio.selected, Some(2));
        gallery.workbench.state(i).extended.disabled = true;
        scene.clear();
        gallery.build_scene(&mut scene);
        for row in 0..3 {
            assert_radio(
                &scene,
                (initial.indicators[row], initial.labels[row]),
                palette,
                ControlState {
                    selected: row == 2,
                    disabled: true,
                    ..Default::default()
                },
                "surface-2",
                gallery.text.scale(),
                clear,
            );
        }
        assert_eq!(
            gallery.workbench.state(i).extended.radio_view.focused(),
            None
        );
        assert!(!scene.primitives().iter().any(
            |p| matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
            && initial.indicators.iter().any(|indicator|indicator.center()==q.rect.center()))
        ));
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        gallery.handle_input(&key(123, true));
        assert_eq!(gallery.workbench.state(i).extended.radio.selected, Some(2));
        gallery.workbench.state(i).extended.disabled = false;
        gallery.build_scene(&mut Scene::new());
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        assert_eq!(gallery.workbench.state(i).extended.radio.selected, Some(0));
    }

    #[test]
    fn switch_gallery_theme_switch_preserves_unfocused_hover_and_actual_thumb_label_contrast() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Rgba, Theme};
        fn assert_switch(
            scene: &Scene,
            geometry: (Rect, Rect),
            palette: Palette,
            state: ControlState,
            ground: &str,
            backing: f32,
            clear: Rgba,
        ) -> Rect {
            let (track, label) = geometry;
            let (index, outer) = scene
                .primitives()
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, p)| match p {
                    Primitive::Quad(q) if q.rect == track => Some((i, *q)),
                    _ => None,
                })
                .expect("actual Switch track background");
            assert_eq!(outer.fill, palette.color(ground).unwrap());
            let boundary = if state.disabled {
                "disabled-text"
            } else {
                "border"
            };
            assert_eq!(outer.border_color, palette.color(boundary).unwrap());
            let thumbs: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a > 0.
                            && q.rect.min_x() >= track.min_x()
                            && q.rect.max_x() <= track.max_x()
                            && q.rect.min_y() >= track.min_y()
                            && q.rect.max_y() <= track.max_y() =>
                    {
                        Some(*q)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                thumbs.len(),
                1,
                "exactly the own thumb after the actual track paint"
            );
            let thumb = thumbs[0];
            assert_eq!(
                thumb.fill,
                palette
                    .color(if state.disabled {
                        "disabled-text"
                    } else if state.selected {
                        "c-switch-on-thumb"
                    } else {
                        "surface-0"
                    })
                    .unwrap()
            );
            assert_eq!(thumb.border_color, palette.color(boundary).unwrap());
            assert!(
                if state.selected {
                    thumb.rect.center().x > track.center().x
                } else {
                    thumb.rect.center().x < track.center().x
                },
                "reduced-motion on/off endpoint"
            );
            if !state.disabled {
                if state.selected {
                    assert!(
                        contrast(thumb.fill, outer.fill) >= 3.,
                        "actual on thumb/track contrast"
                    );
                } else {
                    assert!(
                        contrast(thumb.fill, outer.fill)
                            .max(contrast(thumb.border_color, outer.fill))
                            >= 3.,
                        "actual off thumb fill-or-boundary/track contrast"
                    );
                }
            }
            let parent = scene.primitives()[..index]
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a == 1.
                            && q.rect.min_x() <= label.min_x()
                            && q.rect.max_x() >= label.max_x()
                            && q.rect.min_y() <= label.min_y()
                            && q.rect.max_y() >= label.max_y() =>
                    {
                        Some(q.fill)
                    }
                    _ => None,
                })
                .unwrap_or(clear);
            let inks: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| {
                            let a = g.glyph;
                            label
                                .intersection(&Rect::new(
                                    g.origin.x + a.left as f32 / backing,
                                    g.origin.y + a.top as f32 / backing,
                                    a.width as f32 / backing,
                                    a.height as f32 / backing,
                                ))
                                .is_some()
                        }) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !inks.is_empty(),
                "real CoreText Switch label glyphs after own track paint"
            );
            for ink in inks {
                assert_eq!(
                    ink,
                    palette
                        .color(if state.disabled {
                            "disabled-text"
                        } else {
                            "primary-text"
                        })
                        .unwrap()
                );
                if !state.disabled {
                    assert!(
                        contrast(ink, parent)
                            >= if palette.theme.is_increased() {
                                7.
                            } else {
                                4.5
                            },
                        "actual label/parent ground contrast"
                    );
                }
            }
            thumb.rect
        }
        fn contrast(a: Rgba, b: Rgba) -> f32 {
            let l = |c: Rgba| {
                let c = c.to_linear();
                0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
            };
            let (a, b) = (l(a), l(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }

        fn geometry(scene: &Scene, track: Rect, backing: f32) -> (Rect, Rect) {
            let index = scene
                .primitives()
                .iter()
                .rposition(|p| matches!(p,Primitive::Quad(q)if q.rect==track))
                .unwrap();
            // SwitchView paints its own label immediately after its track/thumb/focus.
            let run = scene.primitives()[index + 1..]
                .iter()
                .find_map(|p| match p {
                    Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
                    _ => None,
                })
                .expect("actual first Switch label run after its own chrome");
            let mut bounds: Option<Rect> = None;
            for g in &run.glyphs {
                let a = g.glyph;
                let r = Rect::new(
                    g.origin.x + a.left as f32 / backing,
                    g.origin.y + a.top as f32 / backing,
                    a.width as f32 / backing,
                    a.height as f32 / backing,
                );
                bounds = Some(match bounds {
                    None => r,
                    Some(b) => Rect::new(
                        b.min_x().min(r.min_x()),
                        b.min_y().min(r.min_y()),
                        b.max_x().max(r.max_x()) - b.min_x().min(r.min_x()),
                        b.max_y().max(r.max_y()) - b.min_y().min(r.min_y()),
                    ),
                });
            }
            let label = bounds.unwrap();
            assert!(
                label.min_x() > track.max_x(),
                "real Switch label lies beside its own track"
            );
            (track, label)
        }
        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::System));
        gallery.apply(Command::Contrast(ContrastChoice::System));
        gallery.set_reduced_motion(true);
        gallery.set_view_size(1440., 900.);
        assert!(gallery.open_route("component-switch/playground"));
        let i = index("switch");
        gallery.build_scene(&mut Scene::new());
        let track = gallery
            .workbench
            .state(i)
            .extended
            .switch_view
            .layout()
            .frame();
        let background = Point::new(track.max_x() + 420., track.min_y());
        for checked in [false, true] {
            gallery.workbench.state(i).extended.switch.checked = checked;
            let mut original = None;
            for theme in Theme::ALL {
                gallery.set_system_theme(theme);
                gallery.handle_input(&move_pointer(background));
                let mut scene = Scene::new();
                gallery.build_scene(&mut scene);
                assert_eq!(gallery.theme(), theme);
                let state = &gallery.workbench.state(i).extended;
                assert_eq!(state.switch.checked, checked);
                assert!(!state.switch_view.is_focused());
                assert_eq!(state.switch_view.layout().frame(), track);
                let thumb = assert_switch(
                    &scene,
                    geometry(&scene, track, gallery.text.scale()),
                    theme.into(),
                    ControlState {
                        selected: checked,
                        ..Default::default()
                    },
                    if checked {
                        "c-switch-on-ground"
                    } else {
                        "surface-3"
                    },
                    gallery.text.scale(),
                    Gallery::background(theme),
                );
                if let Some(before) = original {
                    assert_eq!(thumb, before);
                } else {
                    original = Some(thumb);
                }
            }
        }
        gallery.workbench.state(i).extended.switch.checked = false;
        gallery.build_scene(&mut Scene::new());
        let palette: Palette = Theme::DARK_INCREASED.into();
        let clear = Gallery::background(Theme::DARK_INCREASED);
        let point = track.center();
        gallery.handle_input(&move_pointer(point));
        assert!(
            !gallery.workbench.state(i).extended.switch_view.is_focused(),
            "first hover must not require focus"
        );
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        assert!(!gallery.workbench.state(i).extended.switch_view.is_focused());
        assert_switch(
            &scene,
            geometry(&scene, track, gallery.text.scale()),
            palette,
            ControlState::default(),
            "button-hover",
            gallery.text.scale(),
            clear,
        );
        gallery.handle_input(&mouse(point, true));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_switch(
            &scene,
            geometry(&scene, track, gallery.text.scale()),
            palette,
            ControlState::default(),
            "button-pressed",
            gallery.text.scale(),
            clear,
        );
        assert!(!gallery.workbench.state(i).extended.switch.checked);
        gallery.handle_input(&mouse(background, false));
        gallery.handle_input(&move_pointer(background));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(!gallery.workbench.state(i).extended.switch.checked);
        gallery.handle_input(&key(36, true));
        assert!(!gallery.workbench.state(i).extended.switch.checked);
        gallery.handle_input(&key(49, true));
        assert!(!gallery.workbench.state(i).extended.switch.checked);
        gallery.handle_input(&key(49, false));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(gallery.workbench.state(i).extended.switch.checked);
        assert_switch(
            &scene,
            geometry(&scene, track, gallery.text.scale()),
            palette,
            ControlState {
                selected: true,
                ..Default::default()
            },
            "c-switch-on-ground",
            gallery.text.scale(),
            clear,
        );
        assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
            && q.rect.center()==track.center() && q.border_color==palette.color("border-focused").unwrap())));
        gallery.workbench.state(i).extended.disabled = true;
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_switch(
            &scene,
            geometry(&scene, track, gallery.text.scale()),
            palette,
            ControlState {
                selected: true,
                disabled: true,
                ..Default::default()
            },
            "surface-2",
            gallery.text.scale(),
            clear,
        );
        assert!(!gallery.workbench.state(i).extended.switch_view.is_focused());
        assert!(!scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0. && q.rect.center()==track.center())));
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        gallery.handle_input(&key(49, true));
        gallery.handle_input(&key(49, false));
        assert!(gallery.workbench.state(i).extended.switch.checked);
        gallery.workbench.state(i).extended.disabled = false;
        gallery.build_scene(&mut Scene::new());
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        assert!(!gallery.workbench.state(i).extended.switch.checked);
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
        click(&mut w, i, 20, &mut t);
        assert!(w.state(i).menu_view.is_open());
        w.handle(i, &key(48, true), &mut t);
        assert!(
            !w.state(i).menu_view.is_open(),
            "Tab dismisses a non-modal Menu before leaving its scope"
        );
        let i = index("dialog");
        click(&mut w, i, 20, &mut t);
        w.handle(i, &key(48, true), &mut t);
        assert_eq!(w.state(i).dialog_view.dialog().focus, 1);
        w.handle(i, &key(48, true), &mut t);
        assert_eq!(
            w.state(i).dialog_view.dialog().focus,
            0,
            "modal Tab continues to wrap within the Dialog"
        );
        w.handle(i, &key(53, true), &mut t);
        assert!(!w.state(i).dialog_view.dialog().open);
        assert_eq!(w.state(i).focus, Some(20));
    }

    #[test]
    fn tabs_gallery_theme_switch_preserves_selected_panel_and_repaints_real_tab_text() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::System));
        gallery.apply(Command::Contrast(ContrastChoice::System));
        assert!(gallery.open_route("component-tabs/playground"));
        gallery.set_view_size(1440., 900.);
        gallery.set_reduced_motion(true);
        let i = index("tabs");
        gallery.build_scene(&mut Scene::new());
        let target = gallery
            .workbench
            .state(i)
            .tabs_layout
            .as_ref()
            .unwrap()
            .items[1]
            .frame
            .center();
        gallery.handle_input(&mouse(target, true));
        gallery.handle_input(&mouse(target, false));
        assert_eq!(gallery.workbench.state(i).tabs_view.selected(), 1);
        gallery.handle_input(&move_pointer(Point::new(0., 0.)));
        // The sidebar owns (0, 0); request tab rest through the real Workbench
        // background so the existing Tabs pointer reducer receives the move.
        let tab_frame = gallery
            .workbench
            .state(i)
            .tabs_layout
            .as_ref()
            .unwrap()
            .frame;
        gallery.handle_input(&move_pointer(Point::new(
            tab_frame.center().x,
            tab_frame.max_y() + 4.,
        )));
        gallery.build_scene(&mut Scene::new());
        let initial = gallery.workbench.state(i).tabs_layout.clone().unwrap();
        let mut fills = Vec::new();
        for theme in Theme::ALL {
            gallery.set_system_theme(theme);
            assert_eq!(gallery.theme(), theme);
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            let state = gallery.workbench.state(i);
            assert_eq!(state.tabs_view.selected(), 1);
            let layout = state.tabs_layout.as_ref().unwrap();
            assert_eq!(
                layout, &initial,
                "Theme change preserves selected tab layout"
            );
            let selected = &layout.items[1];
            let palette: Palette = theme.into();
            let fill = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == selected.frame => Some(q.fill),
                    _ => None,
                })
                .expect("actual Gallery selected tab background");
            assert_eq!(fill, palette.color("row-selection").unwrap());
            fills.push(fill);
            let inks: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|positioned| {
                            let glyph = positioned.glyph;
                            let scale = gallery.text.scale();
                            let bounds = Rect::new(
                                positioned.origin.x + glyph.left as f32 / scale,
                                positioned.origin.y + glyph.top as f32 / scale,
                                glyph.width as f32 / scale,
                                glyph.height as f32 / scale,
                            );
                            selected.content.intersection(&bounds).is_some()
                        }) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !inks.is_empty(),
                "Gallery emits selected tab CoreText glyphs"
            );
            for ink in inks {
                assert_eq!(ink, palette.color("primary-text").unwrap());
                let luminance = |c: tekes_ui_tokens::Rgba| {
                    let c = c.to_linear();
                    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                };
                let (a, b) = (luminance(ink), luminance(fill));
                let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                assert!(ratio >= if theme.is_increased() { 7. } else { 4.5 });
            }
            assert!(
                scene.primitives().iter().any(|p| matches!(p,
                        Primitive::Glyphs(run) if run.glyphs.iter().any(|g|
                            g.origin.y > layout.frame.max_y() + 20.
                            && g.origin.y < layout.frame.max_y() + 100.
                            && g.origin.x >= layout.frame.min_x()
                            && g.origin.x < layout.frame.max_x()))),
                "selected settings panel remains rendered below the tabs"
            );
        }
        assert_ne!(fills[0], fills[1]);
        assert_ne!(fills[2], fills[3]);
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
    fn gallery_retained_dialog_theme_switch_repaints_modal_and_copy() {
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        text.set_motion_enabled(true);
        text.set_motion_context(Duration::ZERO, true);
        let i = index("dialog");
        // Open through the actual Gallery trigger and keep this same DialogView.
        click(&mut w, i, 20, &mut text);
        assert!(w.state(i).dialog_view.focus_button(1));
        let content = Rect::new(312., 70., 1000., 820.);
        let mut previous_ground = None;
        for theme in Theme::ALL {
            text.begin_frame();
            let mut static_text = super::super::static_text::StaticText::new(None);
            let mut ctx = PageCtx {
                theme,
                text: &mut text,
                static_text: &mut static_text,
            };
            let mut scene = Scene::new();
            w.build_clipped(i, &mut ctx, content, content, &mut scene);
            assert!(scene.composition_error().is_none());
            let palette = Palette::new(theme, w.brand).unwrap();
            let state = w.state(i);
            assert!(state.dialog_view.is_open());
            assert_eq!(state.dialog_view.dialog().focus, 1);
            assert_eq!(state.focus, Some(71));
            let layout = state.dialog_layout.as_ref().expect("actual Gallery modal");
            assert!(layout.visible.is_some());
            let (surface_index, surface) = scene
                .primitives()
                .iter()
                .enumerate()
                .find_map(|(index, p)| match p {
                    Primitive::Quad(q) if q.rect == layout.modal => Some((index, q)),
                    _ => None,
                })
                .expect("actual Gallery Dialog surface");
            assert_eq!(surface.fill, palette.color("surface-popover").unwrap());
            assert_eq!(
                surface.corner_radii.top_left,
                proposal::metric("radius", "c-dialog-radius").unwrap()
            );
            assert!(scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.rect == layout.preview && q.fill == palette.color("scrim").unwrap()
            )));
            let copy = scene.primitives()[surface_index + 1..]
                .iter()
                .find_map(|p| match p {
                    Primitive::Glyphs(run)
                        if !run.glyphs.is_empty()
                            && run
                                .glyphs
                                .iter()
                                .all(|glyph| layout.title.contains(glyph.origin)) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .expect("actual Gallery Dialog copy, independently of action labels");
            assert_eq!(copy.color, palette.color("primary-text").unwrap());
            assert_eq!((copy.color.a, surface.fill.a), (1., 1.));
            let luminance = |color: tekes_ui_tokens::Rgba| {
                let linear = color.to_linear();
                0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
            };
            let (a, b) = (luminance(copy.color), luminance(surface.fill));
            assert!(
                (a.max(b) + 0.05) / (a.min(b) + 0.05)
                    >= if theme.is_increased() { 7. } else { 4.5 }
            );
            assert!(scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(q) if q.fill.a == 0. && q.border_color == palette.color("border-focused").unwrap()
                    && q.rect.intersection(&layout.buttons[1].frame).is_some()
            )));
            if let Some(previous) = previous_ground
                && theme.is_dark()
            {
                assert_ne!(previous, surface.fill);
            }
            previous_ground = Some(surface.fill);
        }
        // Existing Gallery availability feeds the same retained Dialog actions.
        w.state(i).disabled = true;
        build(&mut w, i, &mut text);
        assert_eq!(
            w.state(i).dialog_view.activate_button(1),
            DialogEvent::Blocked
        );
        assert!(w.state(i).dialog_view.is_open());
        w.state(i).disabled = false;
        build(&mut w, i, &mut text);
        w.handle(i, &key(53, true), &mut text);
        assert!(!w.state(i).dialog_view.is_open());
        assert_eq!(w.state(i).focus, Some(20));
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
    fn requested_file_failure_cannot_fall_back_to_memory_submission() {
        let mut w = Workbench::default();
        let i = index("form");
        w.state(i).input.set_value("unsaved but retained");
        w.state(i)
            .open_draft_fixture(std::path::PathBuf::from("relative-invalid-fixture.json"));
        w.state(i).submit();
        assert_eq!(w.state(i).phase, Phase::Failed);
        assert!(w.state(i).draft_attempt.is_none());
        assert_eq!(w.state(i).input.query(), "unsaved but retained");
        assert!(!w.state(i).draft_fixture.as_ref().unwrap().has_pending());
    }

    #[test]
    fn memory_form_uses_public_journal_and_keeps_newer_editor_on_late_ack() {
        let mut w = Workbench::default();
        let i = index("form");
        assert!(!w.state(i).draft_fixture.as_ref().unwrap().is_durable());
        w.state(i).input.set_value("first");
        w.state(i).persist_draft();
        w.state(i).submit();
        let first = w.state(i).draft_attempt.unwrap().0;
        w.state(i).input.set_value("newer");
        w.state(i).persist_draft();
        w.state(i).resolve();
        assert_eq!(w.state(i).phase, Phase::Failed);
        w.state(i).submit();
        assert_eq!(w.state(i).draft_attempt, Some((first, 2)));
        w.state(i).resolve();
        assert_eq!(w.state(i).phase, Phase::Ready);
        assert_eq!(w.state(i).input.query(), "newer");
        w.state(i).input.focus_field();
        w.state(i).input.handle(
            &Event::TextPreedit {
                text: "输入".into(),
                selection: 2..2,
                clauses: vec![],
                replacement: None,
            },
            None,
        );
        assert!(w.state(i).input.state().unwrap().marked.is_some());
        w.state(i).submit();
        assert_eq!(w.state(i).phase, Phase::Ready);
        assert!(w.state(i).draft_attempt.is_none());
        assert_eq!(
            w.state(i).draft_fixture.as_ref().unwrap().draft().name,
            "newer"
        );
    }

    #[test]
    fn durable_form_fixture_recovers_committed_input_and_retries_immutable_revision() {
        let directory =
            std::env::temp_dir().join(format!("tekesui-form-workflow-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("form.json");
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("form");
        w.state(i).open_draft_fixture(path.clone());
        click(&mut w, i, 1000, &mut text);
        w.text_command(i, "selectAll:", None, &mut text);
        w.handle(
            i,
            &Event::TextCommit {
                text: "项目 Draft".into(),
                replacement: None,
            },
            &mut text,
        );
        click(&mut w, i, 25, &mut text);
        let persisted = durable_draft::DurableDraft::open(path.clone()).unwrap();
        assert_eq!(persisted.draft().name, "项目 Draft");
        assert!(persisted.draft().notifications);
        click(&mut w, i, 20, &mut text);
        let revision = w.state(i).draft_attempt.unwrap().0;
        click(&mut w, i, 21, &mut text);
        assert_eq!(w.state(i).phase, Phase::Failed);
        drop(w);
        let mut w = Workbench::default();
        w.state(i).open_draft_fixture(path.clone());
        assert_eq!(w.state(i).input.query(), "项目 Draft");
        assert_eq!(w.state(i).phase, Phase::Failed);
        click(&mut w, i, 20, &mut text);
        assert_eq!(w.state(i).draft_attempt, Some((revision, 2)));
        // Edit while the older revision is in flight: its ack must not claim
        // the newer input was saved or clear that durable draft.
        click(&mut w, i, 1000, &mut text);
        w.text_command(i, "selectAll:", None, &mut text);
        w.text_command(i, "paste:", Some("Newer draft"), &mut text);
        click(&mut w, i, 21, &mut text);
        assert_eq!(w.state(i).phase, Phase::Ready);
        assert_eq!(w.state(i).input.query(), "Newer draft");
        assert!(!w.state(i).draft_fixture.as_ref().unwrap().is_acknowledged());
        click(&mut w, i, 20, &mut text);
        assert_eq!(w.state(i).draft_attempt.unwrap().1, 1);
        click(&mut w, i, 26, &mut text);
        assert_eq!(w.state(i).phase, Phase::Failed);
        assert!(w.state(i).draft_attempt.is_none());
        click(&mut w, i, 20, &mut text);
        click(&mut w, i, 21, &mut text);
        assert_eq!(w.state(i).phase, Phase::Saved);
        assert!(
            durable_draft::DurableDraft::open(path)
                .unwrap()
                .is_acknowledged()
        );
        std::fs::remove_dir_all(directory).unwrap();
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
    fn icon_button_gallery_theme_switch_repaints_original_action_artwork_and_state_endpoints() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Rgba, Theme};

        fn sample(
            scene: &Scene,
            frame: Rect,
            artwork: Rect,
            palette: Palette,
            ground: &str,
            foreground: &str,
            active: bool,
        ) -> (Rgba, Rgba) {
            let (index, background) = scene
                .primitives()
                .iter()
                .enumerate()
                .rev()
                .find_map(|(i, p)| match p {
                    Primitive::Quad(q) if q.rect == frame => Some((i, q)),
                    _ => None,
                })
                .expect("actual original Gallery icon Button background");
            assert_eq!(background.fill, palette.color(ground).unwrap());
            let strokes: Vec<_> = scene.primitives()[index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q)
                        if q.fill.a > 0. && artwork.intersection(&q.rect) == Some(q.rect) =>
                    {
                        Some(q.fill)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                strokes.len(),
                2,
                "native Plus emits two real strokes after this Button's own ground"
            );
            for ink in &strokes {
                assert_eq!(*ink, palette.color(foreground).unwrap());
                if active {
                    let luminance = |c: Rgba| {
                        let c = c.to_linear();
                        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                    };
                    let (a, b) = (luminance(*ink), luminance(background.fill));
                    assert!(
                        (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 3.,
                        "actual artwork/ground contrast"
                    );
                }
            }
            (background.fill, strokes[0])
        }
        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::System));
        gallery.apply(Command::Contrast(ContrastChoice::System));
        gallery.set_reduced_motion(true);
        gallery.set_view_size(1440., 900.);
        assert!(gallery.open_route("component-icon-button/playground"));
        let i = index("icon-button");
        gallery.build_scene(&mut Scene::new());
        let (frame, artwork) = {
            let view = &gallery.workbench.state(i).extended.icon_button_view;
            assert_eq!(view.button().variant, paint::ButtonVariant::Quiet);
            (view.layout().frame, view.icon_frame())
        };
        let point = frame.center();
        let background = Point::new(frame.max_x() + 60., frame.min_y());
        let mut rest_colors = Vec::new();
        for theme in Theme::ALL {
            gallery.set_system_theme(theme);
            gallery.handle_input(&move_pointer(background));
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            assert_eq!(gallery.theme(), theme);
            let view = &gallery.workbench.state(i).extended.icon_button_view;
            assert_eq!(view.layout().frame, frame);
            assert_eq!(view.icon_frame(), artwork);
            assert_eq!(view.button().activations, 0);
            assert!(!view.button().button.state.hovered);
            assert!(!view.button().button.state.pressed);
            rest_colors.push(sample(
                &scene,
                frame,
                artwork,
                theme.into(),
                "surface-2",
                "primary-text",
                true,
            ));
        }
        assert_ne!(rest_colors[0], rest_colors[1]);
        assert_ne!(rest_colors[2], rest_colors[3]);
        let palette: Palette = Theme::DARK_INCREASED.into();
        gallery.handle_input(&move_pointer(point));
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        assert!(
            gallery
                .workbench
                .state(i)
                .extended
                .icon_button_view
                .button()
                .button
                .state
                .hovered
        );
        sample(
            &scene,
            frame,
            artwork,
            palette,
            "button-hover",
            "primary-text",
            true,
        );
        gallery.handle_input(&mouse(point, true));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(
            gallery
                .workbench
                .state(i)
                .extended
                .icon_button_view
                .button()
                .button
                .state
                .pressed
        );
        sample(
            &scene,
            frame,
            artwork,
            palette,
            "button-pressed",
            "primary-text",
            true,
        );
        gallery.handle_input(&move_pointer(background));
        gallery.handle_input(&mouse(background, false));
        scene.clear();
        gallery.build_scene(&mut scene);
        assert_eq!(
            gallery
                .workbench
                .state(i)
                .extended
                .icon_button_view
                .button()
                .activations,
            0
        );
        assert!(
            !gallery
                .workbench
                .state(i)
                .extended
                .icon_button_view
                .button()
                .button
                .state
                .pressed
        );
        sample(
            &scene,
            frame,
            artwork,
            palette,
            "surface-2",
            "primary-text",
            true,
        );
        // Retained original host focus contract, with a real reduced-motion endpoint.
        gallery
            .workbench
            .state(i)
            .extended
            .icon_button_view
            .focus(true);
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p,Primitive::Quad(q)
            if q.fill.a==0. && q.border_width>0. && q.rect.center()==frame.center()
            && q.border_color==palette.color("border-focused").unwrap()))
        );
        gallery.workbench.state(i).extended.disabled = true;
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(
            gallery
                .workbench
                .state(i)
                .extended
                .icon_button_view
                .button()
                .button
                .state
                .disabled
        );
        sample(
            &scene,
            frame,
            artwork,
            palette,
            "surface-2",
            "tertiary-text",
            false,
        );
        assert!(
            !scene
                .primitives()
                .iter()
                .any(|p| matches!(p,Primitive::Quad(q)
            if q.fill.a==0. && q.border_width>0. && q.rect.center()==frame.center()))
        );
        gallery.handle_input(&mouse(point, true));
        gallery.handle_input(&mouse(point, false));
        assert_eq!(
            gallery
                .workbench
                .state(i)
                .extended
                .icon_button_view
                .button()
                .activations,
            0
        );
    }

    #[test]
    fn gallery_primary_button_theme_and_brand_changes_repaint_real_button_states() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        fn sample_button(
            scene: &Scene,
            frame: Rect,
            content: Rect,
            scale: f32,
        ) -> (tekes_ui_tokens::Rgba, Vec<tekes_ui_tokens::Rgba>) {
            let fill = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    Primitive::Quad(quad) if quad.rect == frame => Some(quad.fill),
                    _ => None,
                })
                .expect("real Gallery Button background quad");
            let inks = scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|positioned| {
                            let glyph = positioned.glyph;
                            let bounds = Rect::new(
                                positioned.origin.x + glyph.left as f32 / scale,
                                positioned.origin.y + glyph.top as f32 / scale,
                                glyph.width as f32 / scale,
                                glyph.height as f32 / scale,
                            );
                            content.intersection(&bounds).is_some()
                        }) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            (fill, inks)
        }

        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::Light));
        gallery.apply(Command::Contrast(ContrastChoice::Off));
        assert_eq!(gallery.theme(), Theme::LIGHT);
        assert!(gallery.open_route("component-button/playground"));
        gallery.set_view_size(1440., 900.);
        let i = index("button");
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        let (button_frame, button_content) = {
            let layout = gallery.workbench.state(i).buttons[&20]
                .layout()
                .expect("public Gallery primary Button is laid out");
            (layout.frame, layout.content)
        };
        let point = button_frame.center();

        gallery.handle_input(&move_pointer(point));
        assert!(gallery.workbench.state(i).buttons[&20].state.hovered);
        gallery.animate(0.001);
        gallery.build_scene(&mut Scene::new());
        gallery.animate(0.1);
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(gallery.workbench.state(i).buttons[&20].needs_animation());
        let (mid_hover_fill, _) =
            sample_button(&scene, button_frame, button_content, gallery.text.scale());
        let light_palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        assert_ne!(
            mid_hover_fill,
            light_palette.color("action-primary-ground").unwrap()
        );
        assert_ne!(
            mid_hover_fill,
            light_palette.color("action-primary-hover").unwrap()
        );

        gallery.apply(Command::Appearance(AppearanceChoice::Dark));
        gallery.apply(Command::Contrast(ContrastChoice::On));
        assert_eq!(gallery.theme(), Theme::DARK_INCREASED);
        scene.clear();
        gallery.build_scene(&mut scene);
        let (pending_hover_animation, current_frame) = {
            let button = &gallery.workbench.state(i).buttons[&20];
            (
                button.needs_animation(),
                button.layout().expect("real Gallery Button layout").frame,
            )
        };
        assert!(!pending_hover_animation);
        assert_eq!(current_frame, button_frame, "theme change preserves layout");
        let palette = Palette::new(Theme::DARK_INCREASED, 0).expect("registered Gallery brand");
        let (fill, inks) =
            sample_button(&scene, button_frame, button_content, gallery.text.scale());
        assert_eq!(
            fill,
            palette
                .color("action-primary-hover")
                .expect("primary hover role")
        );
        assert!(
            !inks.is_empty(),
            "real Button label glyphs are in the Gallery scene"
        );
        assert!(inks.iter().all(|ink| {
            *ink == palette
                .color("action-primary-foreground")
                .expect("primary ink role")
        }));

        gallery.handle_input(&mouse(point, true));
        assert!(gallery.workbench.state(i).buttons[&20].state.pressed);
        gallery.animate(0.001);
        gallery.build_scene(&mut Scene::new());
        gallery.animate(0.028);
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(gallery.workbench.state(i).buttons[&20].needs_animation());
        let dark_palette =
            Palette::new(Theme::DARK_INCREASED, 0).expect("registered Gallery brand");
        let (mid_press_fill, _) =
            sample_button(&scene, button_frame, button_content, gallery.text.scale());
        assert_ne!(
            mid_press_fill,
            dark_palette.color("action-primary-hover").unwrap()
        );
        assert_ne!(
            mid_press_fill,
            dark_palette.color("action-primary-pressed").unwrap()
        );

        gallery.apply(Command::Brand(1));
        scene.clear();
        gallery.build_scene(&mut scene);
        let pending_press_animation = gallery.workbench.state(i).buttons[&20].needs_animation();
        assert!(!pending_press_animation);
        let palette = Palette::new(Theme::DARK_INCREASED, 1).expect("registered Gallery brand");
        let (fill, inks) =
            sample_button(&scene, button_frame, button_content, gallery.text.scale());
        assert_eq!(
            fill,
            palette
                .color("action-primary-pressed")
                .expect("primary pressed role")
        );
        assert!(!inks.is_empty(), "real Button label glyphs remain rendered");
        assert!(inks.iter().all(|ink| {
            *ink == palette
                .color("action-primary-foreground")
                .expect("primary ink role")
        }));
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
        let mut exited_scope = false;
        for _ in 0..tab_count {
            w.handle(radio, &key(48, true), &mut text);
            let Some(focus) = w.state(radio).focus else {
                exited_scope = true;
                break;
            };
            assert!(
                radio_ids.contains(&focus),
                "Tab focused non-entry id {focus}"
            );
        }
        assert!(exited_scope, "Tab at the scope edge yields to the Gallery");
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
    fn menu_gallery_theme_changes_keep_real_popup_title_items_and_navigation() {
        use super::super::{
            Gallery,
            commands::Command,
            theme::{AppearanceChoice, ContrastChoice},
        };
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};
        let contrast = |a: tekes_ui_tokens::Rgba, b: tekes_ui_tokens::Rgba| {
            let luminance = |c: tekes_ui_tokens::Rgba| {
                let v = c.to_linear();
                0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
            };
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        };
        let mut gallery = Gallery::new();
        let i = index("menu");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.set_view_size(1440., 960.);
        gallery.set_reduced_motion(true);
        gallery.animate(1. / 60.);
        gallery.build_scene(&mut Scene::new());
        let trigger = gallery.workbench.state(i).buttons[&20].layout().unwrap();
        gallery.handle_input(&mouse(trigger.frame.center(), true));
        gallery.handle_input(&mouse(trigger.frame.center(), false));
        gallery.build_scene(&mut Scene::new());
        assert!(gallery.workbench.state(i).menu_view.is_open());
        let mut surfaces = Vec::new();
        for theme in Theme::ALL {
            gallery.apply(Command::Appearance(if theme.is_dark() {
                AppearanceChoice::Dark
            } else {
                AppearanceChoice::Light
            }));
            gallery.apply(Command::Contrast(if theme.is_increased() {
                ContrastChoice::On
            } else {
                ContrastChoice::Off
            }));
            assert_eq!(gallery.theme(), theme);
            let popup = gallery.workbench.state(i).menu_layout.clone().unwrap();
            // This gutter is inside the real Menu popup input domain; a window-exterior move does not reach its reducer.
            gallery.handle_input(&move_pointer(popup.popup.origin));
            gallery.handle_input(&key(115, true));
            gallery.animate(1. / 60.);
            for hovered in [false, true] {
                if hovered {
                    gallery.handle_input(&move_pointer(popup.items[1].frame.center()));
                }
                let mut scene = Scene::new();
                gallery.build_scene(&mut scene);
                let state = gallery.workbench.state(i);
                let popup = state.menu_layout.as_ref().unwrap();
                let selected = if hovered { 1 } else { 0 };
                assert_eq!(popup.active_item, selected);
                assert!(state.menu_view.is_open());
                let palette = Palette::new(theme, 0).unwrap();
                let surface = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect == popup.popup => Some(q.fill),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(surface, palette.color("surface-popover").unwrap());
                if !hovered {
                    surfaces.push(surface);
                }
                let trigger = state.buttons[&20].layout().unwrap();
                // The menu owns pointer moves while open. Its original
                // Primary invoker retains the hover from the real opening click.
                assert!(state.buttons[&20].state.hovered);
                assert!(!state.buttons[&20].state.pressed);
                // Gallery's main action uses Primary; the separate public
                // MenuContent fixture intentionally uses its Quiet trigger.
                for (layout, expected_fill, expected_ink) in [
                    (trigger, "action-primary-hover", "action-primary-foreground"),
                    (
                        popup.items[selected],
                        if hovered {
                            "row-selection-hover"
                        } else {
                            "row-selection"
                        },
                        "primary-text",
                    ),
                ] {
                    let (background_index, fill) = scene
                        .primitives()
                        .iter()
                        .enumerate()
                        .find_map(|(index, p)| match p {
                            Primitive::Quad(q) if q.rect == layout.frame => Some((index, q.fill)),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        fill,
                        palette.color(expected_fill).unwrap(),
                        "{theme:?} actual Gallery {expected_fill}"
                    );
                    // Underlay text may occupy the same coordinates, but the
                    // actual title/item glyphs paint after their own background.
                    let inks: Vec<_> = scene.primitives()[background_index + 1..]
                        .iter()
                        .filter_map(|p| match p {
                            Primitive::Glyphs(run)
                                if run.glyphs.iter().any(|g| layout.content.contains(g.origin)) =>
                            {
                                Some(run.color)
                            }
                            _ => None,
                        })
                        .collect();
                    assert!(
                        !inks.is_empty(),
                        "actual Gallery title/item CoreText glyphs"
                    );
                    for ink in inks {
                        assert_eq!(
                            ink,
                            palette.color(expected_ink).unwrap(),
                            "{theme:?} {expected_fill} actual title/item after background {background_index}"
                        );
                        assert!(contrast(ink, fill) >= if theme.is_increased() { 7. } else { 4.5 });
                    }
                }
                let focus = scene.primitives().iter().any(|p| matches!(p, Primitive::Quad(q) if q.rect.center() == popup.items[selected].frame.center() && q.border_color == palette.color("border-focused").unwrap()));
                assert_eq!(focus, !hovered);
            }
        }
        assert!(surfaces.windows(2).any(|w| w[0] != w[1]));
        let before = gallery
            .workbench
            .state(i)
            .log
            .iter()
            .filter(|s| *s == "menu action: 1")
            .count();
        gallery.handle_input(&key(36, true));
        assert!(!gallery.workbench.state(i).menu_view.is_open());
        assert_eq!(gallery.workbench.state(i).focus, Some(20));
        assert_eq!(
            gallery
                .workbench
                .state(i)
                .log
                .iter()
                .filter(|s| *s == "menu action: 1")
                .count(),
            before + 1
        );
    }

    #[test]
    fn menu_keyboard_close_restores_caller_button_focus_modality() {
        for close in [36, 53] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            let i = index("menu");
            click(&mut w, i, 20, &mut text);
            assert!(w.state(i).menu_view.is_open());
            w.handle(i, &key(125, true), &mut text);
            w.handle(i, &key(close, true), &mut text);
            build(&mut w, i, &mut text);
            assert!(!w.state(i).menu_view.is_open());
            assert_eq!(w.state(i).focus, Some(20));
            assert!(
                w.state(i).buttons[&20].state.focus_visible,
                "caller-owned Menu trigger restores keyboard modality after {close}"
            );
        }
    }

    #[test]
    fn separator_gallery_theme_switch_keeps_both_decorative_directions_and_scale() {
        use super::super::{
            Gallery,
            commands::Command,
            theme::{AppearanceChoice, ContrastChoice},
        };
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};
        let mut gallery = Gallery::new();
        let i = index("separator");
        gallery.set_view_size(1440., 960.);
        gallery.text_mut().set_scale(2.);
        // Playground uses SeparatorView; variants use the original at-scale painter.
        for story in ["playground", "variants"] {
            assert!(gallery.open_route(&format!("{}/{story}", entry(i).slug())));
            let mut original_frames = None;
            let mut colors = Vec::new();
            for theme in Theme::ALL {
                gallery.apply(Command::Appearance(if theme.is_dark() {
                    AppearanceChoice::Dark
                } else {
                    AppearanceChoice::Light
                }));
                gallery.apply(Command::Contrast(if theme.is_increased() {
                    ContrastChoice::On
                } else {
                    ContrastChoice::Off
                }));
                assert_eq!(gallery.theme(), theme);
                let mut scene = Scene::new();
                gallery.build_scene(&mut scene);
                let state = gallery.workbench.state(i);
                let palette = Palette::new(theme, 0).unwrap();
                let color = palette.color("separator").unwrap();
                let width = if theme.is_increased() {
                    proposal::metric("stroke", "stroke-rule").unwrap()
                } else {
                    1. / gallery.text.scale()
                };
                let lines: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Quad(q)
                            if q.fill == color
                                && q.border_width == 0.
                                && state.preview.contains(q.rect.center())
                                && ((q.rect.size.width > 100. && q.rect.size.height <= 1.)
                                    || (q.rect.size.height > 100. && q.rect.size.width <= 1.)) =>
                        {
                            Some(q.rect)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    lines.len(),
                    2,
                    "{story}: both real standalone separator directions"
                );
                assert_eq!(
                    lines[0].size.height, width,
                    "{story}: horizontal scale policy"
                );
                assert_eq!(lines[1].size.width, width, "{story}: vertical scale policy");
                // Theme changes may change decorative thickness, but not the allocated span or center.
                let frames = [
                    (lines[0].center(), lines[0].size.width),
                    (lines[1].center(), lines[1].size.height),
                ];
                if let Some(original) = original_frames {
                    assert_eq!(frames, original);
                } else {
                    original_frames = Some(frames);
                }
                if story == "playground" {
                    let layout = state.extended.separator_view.layout();
                    assert_eq!(layout.frame.center(), lines[1].center());
                    assert_eq!(layout.frame.size.height, lines[1].size.height);
                    assert!(layout.visible.is_some());
                }
                colors.push(color);
            }
            assert!(
                colors.windows(2).any(|w| w[0] != w[1]),
                "{story}: retained Gallery consumes Theme ink"
            );
        }
    }

    #[test]
    fn surface_gallery_theme_switch_keeps_panel_text_border_and_nested_action() {
        use super::super::{
            Gallery,
            commands::Command,
            theme::{AppearanceChoice, ContrastChoice},
        };
        use tekes_ui_scene::{CornerRadii, Primitive};
        use tekes_ui_tokens::{Palette, Theme};
        let contrast = |a: tekes_ui_tokens::Rgba, b: tekes_ui_tokens::Rgba| {
            let luminance = |c: tekes_ui_tokens::Rgba| {
                let v = c.to_linear();
                0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
            };
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        };
        let mut gallery = Gallery::new();
        let i = index("surface");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.set_view_size(1440., 960.);
        gallery.text_mut().set_scale(2.);
        gallery.set_reduced_motion(true);
        gallery.animate(1. / 60.);
        let mut original_layout = None;
        let mut fills = Vec::new();
        for theme in Theme::ALL {
            gallery.apply(Command::Appearance(if theme.is_dark() {
                AppearanceChoice::Dark
            } else {
                AppearanceChoice::Light
            }));
            gallery.apply(Command::Contrast(if theme.is_increased() {
                ContrastChoice::On
            } else {
                ContrastChoice::Off
            }));
            assert_eq!(gallery.theme(), theme);
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            let state = gallery.workbench.state(i);
            let layout = state.surface_view.layout();
            let geometry = (layout.frame, layout.content, layout.visible);
            if let Some(original) = original_layout {
                assert_eq!(geometry, original);
            } else {
                original_layout = Some(geometry);
            }
            let (panel_index, panel) = scene
                .primitives()
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, p)| match p {
                    Primitive::Quad(q) if q.rect == layout.frame => Some((index, q)),
                    _ => None,
                })
                .expect("real Gallery SurfaceView panel");
            let palette = Palette::new(theme, 0).unwrap();
            assert_eq!(panel.fill, palette.color("surface-panel").unwrap());
            assert_eq!(panel.border_color, palette.color("separator").unwrap());
            assert_eq!(
                panel.border_width,
                if theme.is_increased() {
                    proposal::metric("stroke", "stroke-rule").unwrap()
                } else {
                    1. / gallery.text.scale()
                }
            );
            assert_eq!(
                panel.corner_radii,
                CornerRadii::uniform(proposal::metric("radius", "radius-panel").unwrap())
            );
            let heading = Rect::new(
                layout.content.min_x(),
                layout.content.min_y(),
                layout.content.size.width,
                36.,
            );
            let inks: Vec<_> = scene.primitives()[panel_index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| heading.contains(g.origin)) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(
                !inks.is_empty(),
                "actual Gallery panel-heading CoreText glyphs"
            );
            for ink in inks {
                assert_eq!(ink, palette.color("primary-text").unwrap());
                assert!(contrast(ink, panel.fill) >= if theme.is_increased() { 7. } else { 4.5 });
            }
            fills.push(panel.fill);
        }
        assert!(
            fills.windows(2).any(|w| w[0] != w[1]),
            "same Gallery SurfaceView consumes Theme changes"
        );
        let state = gallery.workbench.state(i);
        let panel = state.surface_view.layout().frame;
        let action = state.buttons[&20].layout().unwrap().frame;
        let before = state
            .log
            .iter()
            .filter(|s| *s == "activate: primary action")
            .count();
        let decoration = Point::new(panel.max_x() - 5., panel.min_y() + 5.);
        gallery.handle_input(&mouse(decoration, true));
        gallery.handle_input(&mouse(decoration, false));
        assert_eq!(
            gallery
                .workbench
                .state(i)
                .log
                .iter()
                .filter(|s| *s == "activate: primary action")
                .count(),
            before,
            "passive container has no activation"
        );
        gallery.handle_input(&mouse(action.center(), true));
        gallery.handle_input(&mouse(action.center(), false));
        assert_eq!(
            gallery
                .workbench
                .state(i)
                .log
                .iter()
                .filter(|s| *s == "activate: primary action")
                .count(),
            before + 1,
            "nested real Button retains its own action"
        );
    }

    #[test]
    fn status_badge_gallery_retained_theme_switch_keeps_four_tones_readable_and_passive() {
        use super::super::{
            Gallery,
            commands::Command,
            theme::{AppearanceChoice, ContrastChoice},
        };
        use tekes_ui_scene::{CornerRadii, Primitive};
        use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
        let contrast = |a: tekes_ui_tokens::Rgba, b: tekes_ui_tokens::Rgba| {
            let luminance = |c: tekes_ui_tokens::Rgba| {
                let v = c.to_linear();
                0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]
            };
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        };
        let mut gallery = Gallery::new();
        let i = index("status-badge");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.set_view_size(1440., 960.);
        let mut original_frames = None;
        let mut observed = Vec::new();
        for theme in Theme::ALL {
            gallery.apply(Command::Appearance(if theme.is_dark() {
                AppearanceChoice::Dark
            } else {
                AppearanceChoice::Light
            }));
            gallery.apply(Command::Contrast(if theme.is_increased() {
                ContrastChoice::On
            } else {
                ContrastChoice::Off
            }));
            assert_eq!(gallery.theme(), theme);
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            let state = gallery.workbench.state(i);
            let palette = Palette::new(theme, 0).unwrap();
            let ground = Gallery::background(theme);
            assert_eq!(
                ground,
                resolve_surface(palette, "canvas", "content", "rest")
                    .unwrap()
                    .background
            );
            let mut frames = Vec::new();
            for usage in [
                "status-badge-info",
                "status-badge-running",
                "status-badge-succeeded",
                "status-badge-failed",
            ] {
                let packet = resolve_surface(palette, "canvas", usage, "rest").unwrap();
                let badges: Vec<_> = scene
                    .primitives()
                    .iter()
                    .enumerate()
                    .filter_map(|(index, p)| match p {
                        Primitive::Quad(q)
                            if q.fill == packet.background
                                && q.corner_radii
                                    == CornerRadii::uniform(q.rect.size.height * 0.5)
                                && state.preview.contains(q.rect.center()) =>
                        {
                            Some((index, q))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(badges.len(), 1, "one real Gallery {usage} badge");
                let (badge_index, badge) = badges[0];
                assert_eq!(badge.border_width, 0.);
                let label_frame = badge.rect.inset(5.);
                let inks: Vec<_> = scene.primitives()[badge_index + 1..]
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if run.glyphs.iter().any(|g| label_frame.contains(g.origin)) =>
                        {
                            Some(run.color)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!inks.is_empty(), "actual Gallery {usage} label glyphs");
                for ink in inks {
                    assert_eq!(ink, packet.text);
                    assert_eq!(ink, palette.color("c-status-badge-foreground").unwrap());
                    assert!(
                        contrast(ink, badge.fill) >= if theme.is_increased() { 7. } else { 4.5 },
                        "{theme:?} {usage} label contrast={}",
                        contrast(ink, badge.fill)
                    );
                }
                assert!(
                    contrast(badge.fill, ground) >= 3.,
                    "{theme:?} {usage} status solid/parent contrast={}",
                    contrast(badge.fill, ground)
                );
                frames.push(badge.rect);
                observed.push(badge.fill);
            }
            if let Some(original) = &original_frames {
                assert_eq!(&frames, original);
            } else {
                original_frames = Some(frames);
            }
            assert!(
                state.logical_tab_ids().into_iter().all(|id| id < 20),
                "badges add no action/focus target"
            );
        }
        assert!(
            observed
                .chunks_exact(4)
                .any(|fills| fills != &observed[..4]),
            "retained Gallery consumes changed Theme solids"
        );
    }

    #[test]
    fn select_real_feedback_paints_before_its_popup_surface() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("select");
        click(&mut w, i, 12, &mut text);
        click(&mut w, i, 20, &mut text);
        build(&mut w, i, &mut text);
        text.set_motion_context(Duration::from_secs(1), false);
        let scene = build(&mut w, i, &mut text);
        let trigger = w.state(i).select_view.layout().trigger.frame;
        let popup = w.state(i).select_menu_layout.as_ref().unwrap().popup;
        let popup_index = scene
            .primitives()
            .iter()
            .position(|p| matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==popup))
            .unwrap();
        let feedback: Vec<_> = scene
            .primitives()
            .iter()
            .enumerate()
            .filter_map(|(index, p)| match p {
                tekes_ui_scene::Primitive::Glyphs(run)
                    if run.glyphs.first().is_some_and(|g| {
                        g.origin.x >= trigger.min_x()
                            && g.origin.x < trigger.min_x() + 8.
                            && g.origin.y > trigger.max_y()
                            && g.origin.y < popup.max_y()
                    }) =>
                {
                    Some(index)
                }
                _ => None,
            })
            .collect();
        assert!(
            !feedback.is_empty(),
            "actual caller feedback must be rendered"
        );
        assert!(
            feedback.into_iter().all(|i| i < popup_index),
            "popup must occlude caller feedback, never vice versa"
        );
    }

    #[test]
    fn select_real_empty_options_toggle_cancels_menu_and_retains_value_until_choices_return() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("select");
        build(&mut w, i, &mut text);
        w.state(i).select.choose(2);
        click(&mut w, i, 13, &mut text);
        assert!(w.state(i).select_empty);
        assert_eq!(w.state(i).select.menu.items.count, 0);
        assert_eq!(w.state(i).select.selected, 2);
        click(&mut w, i, 20, &mut text);
        assert!(!w.state(i).select.menu.open);
        assert!(!w.state(i).logical_tab_ids().contains(&20));
        click(&mut w, i, 13, &mut text);
        assert_eq!(w.state(i).select.menu.items.count, 3);
        assert_eq!(w.state(i).select.selected, 2);
        click(&mut w, i, 20, &mut text);
        assert!(w.state(i).select.menu.open);
    }

    #[test]
    fn select_real_validation_toggle_keeps_value_and_repairs_through_existing_view() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("select");
        build(&mut w, i, &mut text);
        w.state(i).select.choose(1);
        click(&mut w, i, 12, &mut text);
        assert!(w.state(i).error);
        let scene = build(&mut w, i, &mut text);
        let frame = w.state(i).select_view.layout().trigger.frame;
        let role = proposal::control_boundary("error", tekes_ui_tokens::Theme::LIGHT).unwrap();
        assert!(scene.primitives().iter().any(|p| matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==frame && q.border_color==proposal::render_color(role,tekes_ui_tokens::Theme::LIGHT).unwrap())));
        assert_eq!(w.state(i).select.selected, 1);
        click(&mut w, i, 20, &mut text);
        w.handle(i, &key(125, true), &mut text);
        w.handle(i, &key(36, true), &mut text);
        build(&mut w, i, &mut text);
        assert_eq!(w.state(i).select.selected, 2);
        assert!(w.state(i).error, "caller still owns validation outcome");
        click(&mut w, i, 12, &mut text);
        assert!(!w.state(i).error);
        assert_eq!(w.state(i).select.selected, 2);
    }

    #[test]
    fn select_real_trigger_open_close_and_fresh_keyboard_selection_use_public_view() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("select");
        for open in [true, false, true] {
            click(&mut w, i, 20, &mut text);
            assert_eq!(w.state(i).select.menu.open, open);
            assert_eq!(w.state(i).select.selected, 0);
        }
        w.handle(i, &key(125, true), &mut text);
        w.handle(i, &key(36, true), &mut text);
        build(&mut w, i, &mut text);
        assert_eq!(w.state(i).select.selected, 1);
        assert!(!w.state(i).select.menu.open);
    }

    #[test]
    fn combo_real_pointer_states_keep_the_text_surface_skin_and_draft() {
        for scale in [1., 1.5] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            text.set_type_scale(scale);
            let i = index("combo-box");
            let rest = build(&mut w, i, &mut text);
            let frame = w.state(i).input.frame_at(0).unwrap();
            let fill = |scene: &Scene| {
                scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        tekes_ui_scene::Primitive::Quad(q) if q.rect == frame => Some(q.fill),
                        _ => None,
                    })
                    .unwrap()
            };
            let draft = w.state(i).input.query().to_owned();
            w.handle(i, &move_pointer(frame.center()), &mut text);
            assert_eq!(fill(&rest), fill(&build(&mut w, i, &mut text)));
            w.handle(i, &mouse(frame.center(), true), &mut text);
            assert_eq!(fill(&rest), fill(&build(&mut w, i, &mut text)));
            assert_eq!(w.state(i).focus, Some(1000));
            assert_eq!(w.state(i).input.query(), draft);
            w.handle(i, &mouse(frame.center(), false), &mut text);
            assert_eq!(w.state(i).input.query(), draft);
        }
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
    fn breadcrumb_gallery_theme_switch_preserves_path_and_real_navigation() {
        use super::super::{
            Gallery,
            commands::Command,
            theme::{AppearanceChoice, ContrastChoice},
        };
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};
        let mut gallery = Gallery::new();
        let i = index("breadcrumb");
        assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
        gallery.set_view_size(1440., 960.);
        gallery.set_reduced_motion(true);
        gallery.build_scene(&mut Scene::new());
        let trail_frame = gallery
            .workbench
            .state(i)
            .extended
            .breadcrumb_view
            .layout()
            .frame;
        let rest_point = Point::new(trail_frame.center().x, trail_frame.max_y() + 4.);
        let mut observed = Vec::new();
        for theme in Theme::ALL {
            gallery.apply(Command::Appearance(if theme.is_dark() {
                AppearanceChoice::Dark
            } else {
                AppearanceChoice::Light
            }));
            gallery.apply(Command::Contrast(if theme.is_increased() {
                ContrastChoice::On
            } else {
                ContrastChoice::Off
            }));
            assert_eq!(gallery.theme(), theme);
            gallery.handle_input(&move_pointer(Point::new(-100., -100.)));
            // Theme role endpoints use the real Workbench input scope, not an
            // out-of-window move which never reaches the Breadcrumb reducer.
            gallery.handle_input(&move_pointer(rest_point));
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            let state = gallery.workbench.state(i);
            let layout = state.extended.breadcrumb_view.layout();
            assert_eq!(layout.items.len(), 3);
            let ancestor = layout.items[1].frame();
            let current = layout.items[2].frame();
            let palette = Palette::new(theme, 0).unwrap();
            let fills: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q) if q.rect == ancestor || q.rect == current => Some(q.fill),
                    _ => None,
                })
                .collect();
            assert_eq!(fills, vec![palette.color("surface-2").unwrap(); 2]);
            for (frame, role) in [(ancestor, "primary-text"), (current, "tertiary-text")] {
                let inks: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if run.glyphs.iter().any(|g| frame.contains(g.origin)) =>
                        {
                            Some(run.color)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!inks.is_empty(), "real Gallery Breadcrumb {role} glyphs");
                assert!(inks.iter().all(|ink| *ink == palette.color(role).unwrap()));
            }
            observed.push(fills[0]);
            let before = gallery.workbench.state(i).log.len();
            gallery.handle_input(&mouse(current.center(), true));
            gallery.handle_input(&mouse(current.center(), false));
            assert_eq!(
                gallery.workbench.state(i).log.len(),
                before,
                "current page does not navigate"
            );
            gallery.handle_input(&mouse(ancestor.center(), true));
            gallery.handle_input(&mouse(ancestor.center(), false));
            assert_eq!(gallery.workbench.state(i).extended.selected, 1);
            assert_eq!(gallery.workbench.state(i).log.len(), before + 1);
            assert_eq!(
                gallery.workbench.state(i).log.back().map(String::as_str),
                Some("breadcrumb: 1")
            );
            assert_eq!(
                gallery.workbench.state(i).extended.breadcrumb.items.len(),
                3
            );
        }
        assert!(observed.windows(2).any(|w| w[0] != w[1]));
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
    fn transient_overlays_do_not_resume_old_paint_after_unpainted_window_deactivation() {
        let mut resumed = Vec::new();
        for renderer in ["menu", "popover", "select"] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            text.set_motion_enabled(true);
            text.set_motion_context(Duration::ZERO, false);
            let i = index(renderer);
            click(&mut w, i, 20, &mut text);
            text.set_motion_context(Duration::from_millis(200), false);
            build(&mut w, i, &mut text);
            // A hidden/minimized window need not paint between losing and
            // regaining activation. Do not depend on that missing frame to
            // clear retained presentation belonging to a dismissed transient.
            w.set_window_active(false);
            w.set_window_active(true);
            build(&mut w, i, &mut text);
            let state = w.state(i);
            let open = match renderer {
                "menu" => state.menu_view.is_open(),
                "popover" => state.extended.popover.open,
                "select" => state.select.menu.open,
                _ => unreachable!(),
            };
            assert!(!open, "{renderer}: transient model reopened");
            // Button focus feedback may legitimately request a new frame on
            // activation; inspect only the dismissed overlay's own runtime.
            let retained_overlay = match renderer {
                "menu" => state.menu_view.is_presented(),
                "popover" => state.extended.overlay_needs_animation(),
                "select" => state.select_view.needs_animation(),
                _ => unreachable!(),
            };
            if retained_overlay {
                resumed.push(renderer);
            }
        }
        assert!(
            resumed.is_empty(),
            "dismissed transient paint resumed: {resumed:?}"
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
    fn combobox_preedit_filter_navigation_and_selector_submit_preserve_text_state() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("combo-box");
        click(&mut workbench, i, 1000, &mut text);
        assert_eq!(workbench.state(i).focus, Some(1000));
        assert!(workbench.text_state(i).is_some());

        workbench.handle(
            i,
            &Event::TextPreedit {
                text: "C".into(),
                clauses: vec![],
                selection: 0..0,
                replacement: None,
            },
            &mut text,
        );
        build(&mut workbench, i, &mut text);
        assert!(workbench.text_state(i).unwrap().marked.is_some());
        workbench.handle(
            i,
            &Event::TextCommit {
                text: "Com".into(),
                replacement: None,
            },
            &mut text,
        );
        build(&mut workbench, i, &mut text);
        assert_eq!(workbench.state(i).extended.combo.query, "Com");
        assert_eq!(workbench.text_state(i).unwrap().text, "Com");
        assert!(workbench.text_state(i).unwrap().marked.is_none());

        assert!(workbench.handle(i, &Event::TextCommand("moveDown:".into()), &mut text));
        build(&mut workbench, i, &mut text);
        assert_eq!(workbench.state(i).focus, Some(1000));
        assert_eq!(workbench.text_state(i).unwrap().text, "Com");
        assert_eq!(workbench.state(i).extended.combo.active, Some(2));

        assert!(workbench.handle(i, &Event::TextCommand("insertNewline:".into()), &mut text));
        assert!(!workbench.state(i).extended.combo.open);
        assert_eq!(workbench.state(i).focus, Some(1000));
        assert_eq!(workbench.text_state(i).unwrap().text, "Comfortable");
        build(&mut workbench, i, &mut text);
        assert_eq!(workbench.state(i).focus, Some(1000));
        assert_eq!(workbench.text_state(i).unwrap().text, "Comfortable");
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
    fn command_palette_gallery_theme_switch_keeps_query_active_row_and_real_text_contrast() {
        use super::super::Gallery;
        use super::super::commands::Command;
        use super::super::theme::{AppearanceChoice, ContrastChoice};
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        fn assert_text(scene: &Scene, frame: Rect, scale: f32, palette: Palette, ground: &str) {
            let (background_index, fill) = scene
                .primitives()
                .iter()
                .enumerate()
                .find_map(|(index, p)| match p {
                    Primitive::Quad(q) if q.rect == frame => Some((index, q.fill)),
                    _ => None,
                })
                .expect("actual Gallery command component background");
            assert_eq!(fill, palette.color(ground).unwrap());
            assert_eq!(fill.a, 1.);
            let inks: Vec<_> = scene.primitives()[background_index + 1..]
                .iter()
                .filter_map(|p| match p {
                    Primitive::Glyphs(run)
                        if run.glyphs.iter().any(|g| {
                            let glyph = g.glyph;
                            frame
                                .intersection(&Rect::new(
                                    g.origin.x + glyph.left as f32 / scale,
                                    g.origin.y + glyph.top as f32 / scale,
                                    glyph.width as f32 / scale,
                                    glyph.height as f32 / scale,
                                ))
                                .is_some()
                        }) =>
                    {
                        Some(run.color)
                    }
                    _ => None,
                })
                .collect();
            assert!(!inks.is_empty(), "actual Gallery query/candidate glyphs");
            for ink in inks {
                assert_eq!(ink, palette.color("primary-text").unwrap());
                let luminance = |c: tekes_ui_tokens::Rgba| {
                    let c = c.to_linear();
                    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                };
                let (a, b) = (luminance(ink), luminance(fill));
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05)
                        >= if palette.theme.is_increased() {
                            7.
                        } else {
                            4.5
                        }
                );
            }
        }

        let mut gallery = Gallery::new();
        gallery.apply(Command::Appearance(AppearanceChoice::System));
        gallery.apply(Command::Contrast(ContrastChoice::System));
        gallery.set_reduced_motion(true);
        gallery.set_view_size(1440., 1000.);
        assert!(gallery.open_route("component-command-palette/playground"));
        let i = index("command-palette");
        gallery.build_scene(&mut Scene::new());
        let trigger = gallery
            .workbench
            .state(i)
            .extended
            .command_view
            .trigger_layout()
            .unwrap()
            .frame;
        gallery.handle_input(&mouse(trigger.center(), true));
        gallery.handle_input(&mouse(trigger.center(), false));
        gallery.build_scene(&mut Scene::new());
        assert_eq!(gallery.workbench.state(i).focus, Some(1000));
        gallery.handle_input(&Event::TextCommit {
            text: "e".into(),
            replacement: None,
        });
        gallery.build_scene(&mut Scene::new());
        gallery.handle_input(&key(125, true));
        gallery.build_scene(&mut Scene::new());
        let initial_query = gallery.workbench.state(i).input.frame_at(0).unwrap();
        let initial_popup = gallery
            .workbench
            .state(i)
            .extended
            .command_view
            .options
            .layout()
            .unwrap()
            .clone();
        // Actual Workbench input scope, outside both trigger and popup. Sidebar moves do not reach it.
        let background = Point::new(initial_query.max_x() + 40., initial_query.min_y());
        gallery.handle_input(&move_pointer(background));
        let mut selected_fills = Vec::new();
        for theme in Theme::ALL {
            gallery.set_system_theme(theme);
            assert_eq!(gallery.theme(), theme);
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            let state = gallery.workbench.state(i);
            assert!(state.extended.command.combo.open);
            assert_eq!(state.focus, Some(1000));
            assert_eq!(state.input.query(), "e");
            assert_eq!(state.extended.command.combo.query, "e");
            assert_eq!(state.extended.command.combo.active, Some(1));
            assert_eq!(state.extended.command.combo.selected, None);
            assert_eq!(state.input.frame_at(0), Some(initial_query));
            let popup = state.extended.command_view.options.layout().unwrap();
            assert_eq!(popup.popup, initial_popup.popup);
            assert_eq!(popup.items.len(), 3);
            assert_eq!(popup.active_item, 1);
            let palette: Palette = theme.into();
            assert_text(
                &scene,
                initial_query,
                gallery.text.scale(),
                palette,
                "transcript-input",
            );
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, Primitive::Quad(q)
                if q.fill.a == 0. && q.border_width > 0. && q.rect.center() == initial_query.center()
                && q.border_color == palette.color("border-focused").unwrap()))
            );
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, Primitive::Quad(q)
                if q.rect == popup.popup && q.fill == palette.color("surface-popover").unwrap()
                && q.corner_radii == tekes_ui_scene::CornerRadii::uniform(
                    proposal::metric("radius", "radius-floating").unwrap())))
            );
            for (row, item) in popup.items.iter().enumerate() {
                assert_text(
                    &scene,
                    item.frame,
                    gallery.text.scale(),
                    palette,
                    if row == 1 {
                        "row-selection"
                    } else {
                        "surface-2"
                    },
                );
            }
            selected_fills.push(palette.color("row-selection").unwrap());
        }
        assert_ne!(selected_fills[0], selected_fills[1]);
        assert_ne!(selected_fills[2], selected_fills[3]);
        gallery.handle_input(&key(53, true));
        gallery.build_scene(&mut Scene::new());
        assert!(!gallery.workbench.state(i).extended.command.combo.open);
        assert_eq!(gallery.workbench.state(i).focus, Some(20));
    }

    #[test]
    fn command_palette_measured_trigger_leaves_search_and_popup_regions_separate() {
        for width in [240., 360., 800.] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            text.set_type_scale(1.5);
            let i = index("command-palette");
            let viewport = Rect::new(0., 0., width, 1600.);
            build_in(&mut w, i, &mut text, viewport);
            let trigger = w.state(i).extended.command_view.trigger_layout().unwrap();
            let expected = ButtonModel::measure(
                &mut text,
                "Open command palette",
                trigger.frame.size.width,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
            );
            assert!(trigger.frame.size.height >= expected.height);
            w.state(i).extended.command.open();
            build_in(&mut w, i, &mut text, viewport);
            let search = w.state(i).input.frame_at(0).unwrap();
            let popup = w.state(i).extended.command_view.options.layout().unwrap();
            assert!(search.min_y() >= trigger.frame.max_y());
            assert!(popup.popup.min_y() >= search.max_y());
            w.state(i).input.set_value("no matching command");
            build_in(&mut w, i, &mut text, viewport);
            assert!(w.state(i).extended.command.combo.open);
            assert!(w.state(i).extended.command_view.options.layout().is_none());
            assert!(w.state(i).hits.iter().all(|(id, _)| !(40..43).contains(id)));
        }
    }

    #[test]
    fn command_palette_cold_keyboard_focus_is_visible_and_opens_without_prior_click() {
        for code in [36, 49] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            text.set_motion_enabled(false);
            let i = index("command-palette");
            build(&mut w, i, &mut text);
            for _ in 0..16 {
                if w.state(i).focus == Some(20) {
                    break;
                }
                w.handle(i, &key(48, true), &mut text);
                build(&mut w, i, &mut text);
            }
            assert_eq!(w.state(i).focus, Some(20));
            let scene = build(&mut w, i, &mut text);
            let trigger = w
                .state(i)
                .extended
                .command_view
                .trigger_layout()
                .unwrap()
                .frame;
            let has_ring = |scene: &Scene| {
                scene.primitives().iter().any(|p| {
                    matches!(p,
                tekes_ui_scene::Primitive::Quad(q) if q.fill.a == 0. && q.border_width > 0.
                && q.rect.center() == trigger.center() && q.rect.size.width > trigger.size.width)
                })
            };
            assert!(
                has_ring(&scene),
                "cold keyboard focus must reach the production trigger"
            );
            w.handle(i, &key(code, true), &mut text);
            w.handle(i, &key(code, false), &mut text);
            let scene = build(&mut w, i, &mut text);
            assert!(w.state(i).extended.command.combo.open);
            assert_eq!(w.state(i).focus, Some(1000));
            assert!(
                !has_ring(&scene),
                "focus belongs to query while palette is open"
            );
            w.handle(i, &key(53, true), &mut text);
            let scene = build(&mut w, i, &mut text);
            assert!(!w.state(i).extended.command.combo.open);
            assert_eq!(w.state(i).focus, Some(20));
            assert!(has_ring(&scene), "Escape restores the trigger focus ring");
        }
    }

    #[test]
    fn command_palette_real_pointer_hover_and_drag_outside_release_do_not_leave_a_press() {
        for scale in [1.0, 1.5] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            text.set_type_scale(scale);
            text.set_motion_enabled(false);
            // Explicit static feedback sample; the fixture must not enable motion.
            text.replace_fixture_motion_context(Some(tekes_ui_controls::ControlMotionContext {
                now: Duration::ZERO,
                reduced_motion: true,
            }));
            let i = index("command-palette");
            let rest = build(&mut w, i, &mut text);
            let frame = w
                .state(i)
                .extended
                .command_view
                .trigger_layout()
                .unwrap()
                .frame;
            let fill = |scene: &Scene| {
                scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        tekes_ui_scene::Primitive::Quad(q) if q.rect == frame => Some(q.fill),
                        _ => None,
                    })
                    .unwrap()
            };
            let outside = Point::new(frame.max_x() + 120., frame.max_y() + 80.);
            w.handle(i, &move_pointer(frame.center()), &mut text);
            let hover = build(&mut w, i, &mut text);
            assert_ne!(fill(&rest), fill(&hover));
            w.handle(i, &mouse(frame.center(), true), &mut text);
            let pressed = build(&mut w, i, &mut text);
            assert_ne!(fill(&hover), fill(&pressed));
            w.handle(i, &move_pointer(outside), &mut text);
            assert_eq!(fill(&rest), fill(&build(&mut w, i, &mut text)));
            w.handle(i, &mouse(outside, false), &mut text);
            assert!(!w.state(i).extended.command.combo.open);
            w.handle(i, &move_pointer(frame.center()), &mut text);
            assert_eq!(
                fill(&hover),
                fill(&build(&mut w, i, &mut text)),
                "outside release must clear the production trigger arm"
            );
            w.handle(i, &mouse(frame.center(), true), &mut text);
            w.handle(i, &mouse(frame.center(), false), &mut text);
            build(&mut w, i, &mut text);
            assert!(w.state(i).extended.command.combo.open);
            assert_eq!(w.state(i).focus, Some(1000));
        }
    }

    #[test]
    fn choice_escape_closes_without_recording_a_candidate_activation() {
        for renderer in ["combo-box", "command-palette"] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            let i = index(renderer);
            click(
                &mut w,
                i,
                if renderer == "combo-box" { 1000 } else { 20 },
                &mut text,
            );
            w.state(i).log.clear();
            assert!(w.handle(i, &key(53, true), &mut text));
            assert!(
                !w.state(i)
                    .log
                    .iter()
                    .any(|event| event.starts_with("command:")
                        || event.starts_with("combo selection:")),
                "{} cancellation log: {:?}",
                renderer,
                w.state(i).log
            );
        }
    }

    #[test]
    fn choice_popup_old_release_cannot_replace_a_new_native_text_query() {
        for renderer in ["combo-box", "command-palette"] {
            let mut w = Workbench::default();
            let mut text = super::super::text::TextEngine::new();
            let i = index(renderer);
            click(
                &mut w,
                i,
                if renderer == "combo-box" { 1000 } else { 20 },
                &mut text,
            );
            let point = w.state(i).visible_for_id(40).unwrap().center();
            w.handle(i, &mouse(point, true), &mut text);
            assert!(w.handle(
                i,
                &Event::TextCommit {
                    text: "no-matching-choice".into(),
                    replacement: None
                },
                &mut text
            ));
            let query = w.state(i).input.query().to_owned();
            assert!(query.contains("no-matching-choice"));
            // A native text command can precede the next paint. Geometry and
            // pointer capture still refer to the old list at this boundary.
            w.handle(i, &mouse(point, false), &mut text);
            assert_eq!(w.state(i).input.query(), query);
            let model = if renderer == "combo-box" {
                &w.state(i).extended.combo
            } else {
                &w.state(i).extended.command.combo
            };
            assert_eq!(model.query, query);
            assert_eq!(model.selected, None);
            build(&mut w, i, &mut text);
            assert!(w.state(i).hits.iter().all(|(id, _)| !(40..43).contains(id)));
        }
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
    fn command_palette_repeats_the_same_command_and_keeps_empty_query_live() {
        let mut w = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("command-palette");
        for count in 1..=2 {
            click(&mut w, i, 20, &mut text);
            assert!(w.state(i).extended.command.combo.open);
            assert_eq!(w.state(i).focus, Some(1000));
            w.handle(i, &key(36, true), &mut text);
            build(&mut w, i, &mut text);
            assert!(!w.state(i).extended.command.combo.open);
            assert_eq!(w.state(i).focus, Some(20));
            assert_eq!(w.state(i).extended.command.combo.selected, Some(0));
            assert_eq!(
                w.state(i)
                    .log
                    .iter()
                    .filter(|e| e.as_str() == "command: 0")
                    .count(),
                count
            );
        }
        click(&mut w, i, 20, &mut text);
        w.handle(
            i,
            &Event::TextCommit {
                text: "no command matches".into(),
                replacement: None,
            },
            &mut text,
        );
        build(&mut w, i, &mut text);
        let log_before_enter = w.state(i).log.clone();
        w.handle(i, &key(36, true), &mut text);
        build(&mut w, i, &mut text);
        assert!(w.state(i).extended.command.combo.open);
        assert_eq!(w.state(i).focus, Some(1000));
        assert_eq!(w.state(i).input.query(), "no command matches");
        assert_eq!(
            w.state(i).extended.command.combo.query,
            "no command matches"
        );
        assert!(w.state(i).extended.command.combo.filtered().is_empty());
        assert!(!w.state(i).extended.command_view.options.is_visible());
        assert_eq!(
            w.state(i).log,
            log_before_enter,
            "empty confirmation never executes an old command"
        );
    }

    #[test]
    fn command_palette_escape_releases_search_focus_and_ignores_late_input() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("command-palette");
        click(&mut workbench, i, 20, &mut text);
        assert!(workbench.state(i).extended.command.combo.open);
        assert_eq!(workbench.state(i).focus, Some(1000));
        assert!(workbench.text_state(i).is_some());
        workbench.handle(
            i,
            &Event::TextCommit {
                text: "Open".into(),
                replacement: None,
            },
            &mut text,
        );
        assert_eq!(workbench.state(i).extended.command.combo.query, "Open");
        assert!(workbench.handle(i, &Event::TextCommand("cancelOperation:".into()), &mut text));
        assert!(!workbench.state(i).extended.command.combo.open);
        assert_eq!(workbench.state(i).focus, Some(20));
        assert!(workbench.text_state(i).is_none());
        assert!(!workbench.handle(
            i,
            &Event::TextCommit {
                text: "!".into(),
                replacement: None
            },
            &mut text
        ));
        assert!(!workbench.handle(i, &Event::TextCommand("paste:".into()), &mut text));
        let (changed, _) = workbench.text_command(i, "paste:", Some("stale"), &mut text);
        assert!(!changed);
        assert!(!workbench.state(i).extended.command.combo.open);
        assert_eq!(workbench.state(i).extended.command.combo.query, "Open");
    }

    #[test]
    fn command_palette_does_not_restore_disabled_or_unmounted_invoker() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("command-palette");
        click(&mut workbench, i, 20, &mut text);
        workbench.state(i).extended.command.combo.disabled = true;
        workbench.state(i).close_command_palette();
        assert_eq!(
            workbench.state(i).focus,
            None,
            "disabled trigger is not a valid restore target"
        );

        let state = workbench.state(i);
        state.extended.command.combo.disabled = false;
        state.logical_focus.retain(|target| target.id != 20);
        state.extended.command.combo.open = true;
        state.close_command_palette();
        assert_eq!(
            workbench.state(i).focus,
            None,
            "unmounted trigger is not a valid restore target"
        );
    }

    #[test]
    fn command_palette_restores_mounted_invoker_when_its_frame_is_clipped() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("command-palette");
        click(&mut workbench, i, 20, &mut text);
        let viewport = Rect::new(312., 70., 1000., 820.);
        let narrow_clip = Rect::new(312., 70., 1., 1.);
        build_in_clipped(&mut workbench, i, &mut text, viewport, narrow_clip);
        assert!(!workbench.state(i).hits.iter().any(|(id, _)| *id == 20));
        assert!(
            workbench
                .state(i)
                .logical_focus
                .iter()
                .any(|target| target.id == 20 && target.mounted && !target.eligible)
        );

        workbench.state(i).close_command_palette();
        assert_eq!(workbench.state(i).focus, Some(20));
        assert!(
            workbench
                .state(i)
                .logical_focus
                .iter()
                .any(|target| target.id == 20 && target.mounted && target.eligible)
        );
    }

    #[test]
    fn command_palette_ime_cancel_composes_first_and_enter_and_outside_dismiss_restore_focus() {
        let mut workbench = Workbench::default();
        let mut text = super::super::text::TextEngine::new();
        let i = index("command-palette");
        click(&mut workbench, i, 20, &mut text);
        workbench.handle(
            i,
            &Event::TextPreedit {
                text: "開".into(),
                clauses: vec![],
                selection: 0..0,
                replacement: None,
            },
            &mut text,
        );
        workbench.handle(i, &Event::TextCommand("cancelOperation:".into()), &mut text);
        assert!(
            workbench.state(i).extended.command.combo.open,
            "IME cancel must not dismiss the palette"
        );
        assert!(workbench.text_state(i).is_some());
        workbench.handle(i, &Event::TextUnmark, &mut text);
        workbench.handle(
            i,
            &Event::TextCommit {
                text: "Open".into(),
                replacement: None,
            },
            &mut text,
        );
        assert!(workbench.handle(i, &Event::TextCommand("insertNewline:".into()), &mut text));
        assert!(!workbench.state(i).extended.command.combo.open);
        assert_eq!(workbench.state(i).focus, Some(20));
        assert!(workbench.text_state(i).is_none());
        click(&mut workbench, i, 20, &mut text);
        assert!(workbench.state(i).extended.command.combo.open);
        workbench.handle(i, &mouse(Point::new(1500., 900.), true), &mut text);
        assert!(!workbench.state(i).extended.command.combo.open);
        assert_eq!(workbench.state(i).focus, Some(20));
        assert!(workbench.text_state(i).is_none());
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
        text.set_motion_enabled(true);
        let mut workbench = Workbench::default();
        for renderer in ["menu", "select"] {
            let i = index(renderer);
            workbench.apply_reduced_motion(true);
            text.set_motion_context(Duration::ZERO, true);
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
        text.set_motion_context(Duration::ZERO, true);
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
        text.set_motion_context(Duration::ZERO, false);
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
        text.set_motion_context(Duration::ZERO, true);
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
        text.set_motion_context(Duration::ZERO, false);
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

#[cfg(test)]
mod scene3d_contract {
    use super::*;
    #[test]
    fn scene3d_reset_is_consumed_once_and_only_implemented_stories_are_exposed() {
        let i = entries()
            .iter()
            .position(|e| e.renderer() == "scene3d")
            .unwrap();
        assert!(!entry(i).supports_story("behaviors"));
        let mut workbench = Workbench::default();
        assert!(!workbench.take_scene3d_reset(i));
        workbench.state(i).activate(0, "scene3d");
        assert!(workbench.take_scene3d_reset(i));
        assert!(!workbench.take_scene3d_reset(i));
    }
}

#[cfg(test)]
mod skeleton_gpu_acceptance {
    use super::*;
    #[test]
    #[ignore = "explicit Gallery GPU acceptance requires a fresh SSD output directory"]
    fn skeleton_original_gallery_route_gpu() {
        let out = std::path::PathBuf::from(
            std::env::var_os("TEKESUI_SKELETON_GALLERY_OUT").expect("fresh GPU output"),
        );
        std::fs::create_dir(&out).unwrap();
        let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
        let mut cases = vec![];
        let i = entries()
            .iter()
            .position(|e| e.renderer() == "skeleton")
            .unwrap();
        for (mode, theme) in tekes_ui_tokens::Theme::ALL.into_iter().enumerate() {
            for brand in [0, 1, 2] {
                for scale in [1_u32, 2] {
                    for (width, height) in [(1440_u32, 960_u32), (720, 480)] {
                        let mut g = super::super::Gallery::new();
                        g.set_system_theme(theme);
                        g.text.set_scale(scale as f32);
                        g.set_reduced_motion(true);
                        g.workbench.brand = brand;
                        assert!(g.open_route("component-skeleton/playground"));
                        g.set_view_size(width as f32, height as f32);
                        let mut scene = Scene::new();
                        g.build_scene(&mut scene);
                        let f = g.workbench.state(i).skeleton_view.layout().frame;
                        assert_eq!(f.size.height, 18.);
                        let p = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
                        let packet = tekes_ui_tokens::surface_palette::resolve_surface(
                            p, "canvas", "skeleton", "rest",
                        )
                        .unwrap();
                        assert!(scene.primitives().iter().any(|p|matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==f&&q.fill==packet.background)));
                        assert!(scene.primitives().iter().any(|p|matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==f&&q.border_color==packet.boundary.unwrap()&&q.border_width>0.)));
                        let size = (width * scale, height * scale);
                        let rgba = gpu
                            .render_to_rgba8(
                                &scene,
                                g.atlas_mut(),
                                size,
                                scale as f32,
                                super::super::Gallery::background(theme),
                            )
                            .unwrap();
                        let key = format!("mode{mode}-brand{brand}-{width}x{height}@{scale}x");
                        let file = format!("{key}.png");
                        let mut png = png::Encoder::new(
                            std::fs::File::create(out.join(&file)).unwrap(),
                            size.0,
                            size.1,
                        );
                        png.set_color(png::ColorType::Rgba);
                        png.set_depth(png::BitDepth::Eight);
                        png.write_header().unwrap().write_image_data(&rgba).unwrap();
                        // Count ink variation on the final original 18pt fixture row. It
                        // must contain actual boundary/fill pixels, not just page chrome.
                        let mut colors = std::collections::BTreeSet::new();
                        for y in ((f.min_y() * scale as f32).ceil() as u32)
                            ..((f.max_y() * scale as f32).floor() as u32).min(size.1)
                        {
                            for x in ((f.min_x() * scale as f32).ceil() as u32)
                                ..((f.max_x() * scale as f32).floor() as u32).min(size.0)
                            {
                                let off = ((y * size.0 + x) * 4) as usize;
                                colors.insert(rgba[off..off + 4].to_vec());
                            }
                        }
                        assert!(colors.len() > 1, "original row missing visible boundary");
                        cases.push(serde_json::json!({"case":key,"file":file,"row_distinct_colors":colors.len(),"original_gallery_route":true,"actual_GPU":true,"fixture_height":18,"SDK_default_height":false}));
                    }
                }
            }
        }
        std::fs::write(
            out.join("receipt.json"),
            serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"native_window":false}))
                .unwrap(),
        )
        .unwrap();
    }
}
