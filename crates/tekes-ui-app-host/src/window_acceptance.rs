use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

impl super::Host {
    /// Finite opt-in continuation of the original Main23 driver. Three-column
    /// profiles start with leading collapsed; exercise its real shared control.
    pub(super) fn accept_columns(&mut self, w: &super::Window) -> anyhow::Result<()> {
        use super::{ContentKeyEvent, ControlKey, Event, ToolbarTarget};
        use tekes_ui_platform_macos::{AccessibilityAction, AccessibilityRole};
        if !w.is_active()
            || !w.is_key_window()
            || w.is_occluded()
            || self
                .navigation
                .as_ref()
                .is_some_and(|n| n.busy || !n.host.input_open || n.error.is_some())
        {
            w.request_redraw();
            return Ok(());
        }
        if self.step == 24 {
            if !self.leading_visible {
                self.set_toolbar_focus(Some(ToolbarTarget::Visibility(0)), true);
                for down in [true, false] {
                    self.toolbar_key_input(ContentKeyEvent {
                        key: Some(ControlKey::Space),
                        down,
                        repeat: false,
                        modifiers: Default::default(),
                        native_code: 49,
                        characters: " ".into(),
                        characters_ignoring_modifiers: " ".into(),
                    })?;
                }
                anyhow::ensure!(
                    self.leading_visible,
                    "original leading toggle did not activate"
                );
            }
            self.step = 25;
            w.request_redraw();
        } else if (25..=26).contains(&self.step)
            && !self.sidebar_motion.needs_frame()
            && self.panels.iter().all(|p| {
                p.bounds.size.width > 0.
                    && p.frame_viewport == p.viewport
                    && (p.local.is_some() || (p.pid.is_some() && p.image.is_some()))
            })
        {
            if self.step == 25 {
                if self.panels[2].local.is_some() {
                    let publication = w
                        .content_accessibility_publication()
                        .ok_or_else(|| anyhow::anyhow!("mixed publication missing"))?;
                    let target = publication
                        .nodes()
                        .iter()
                        .find(|n| {
                            n.enabled
                                && n.role == AccessibilityRole::TextField
                                && n.label == "本地备注"
                        })
                        .ok_or_else(|| anyhow::anyhow!("mixed original text field missing"))?
                        .key;
                    for event in [
                        Event::Accessibility {
                            target,
                            action: AccessibilityAction::Focus,
                        },
                        Event::TextCommand("selectAll:".into()),
                        Event::TextCommit {
                            text: "mixed 本地输入验收".into(),
                            replacement: None,
                        },
                    ] {
                        anyhow::ensure!(
                            self.local_content_event(&event, w)?,
                            "mixed original input rejected"
                        );
                    }
                }
                self.step = 26;
                w.request_redraw();
            } else {
                let mut local_input = None;
                if let Some(local) = &mut self.panels[2].local {
                    local.with_host(&mut |h| {
                        local_input = h.focused_text().map(|s| s.input.text.clone())
                    });
                    anyhow::ensure!(
                        local_input.as_deref() == Some("mixed 本地输入验收"),
                        "mixed text/focus not retained"
                    );
                }
                self.step = 27;
                eprintln!(
                    "COLUMNS_WINDOW_READY panels={} pids={:?} leading_expanded=true local_input={} physical_input=false native_window={}",
                    self.panels.len(),
                    self.panels.iter().map(|p| p.pid).collect::<Vec<_>>(),
                    local_input.is_some(),
                    w.native_window_number()
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    Main,
    Opening,
    AwaitingShown,
    Shown,
    Editing,
    Saving,
    Undoing,
    Restoring,
    Closing,
    Done,
    Failed,
}
#[derive(Debug)]
pub(super) struct ResultState {
    pub stage: Stage,
    pub watchdog_started: bool,
    pub started: Instant,
    pub phase_started: Instant,
    pub parent_window: isize,
    pub child_window: Option<isize>,
    pub parent_wake: Option<tekes_ui_platform_macos::WindowWake>,
    pub child_wake: Option<tekes_ui_platform_macos::WindowWake>,
    pub mode: Option<crate::SettingsDisplayMode>,
    pub field_path: Vec<String>,
    pub original: Option<String>,
    pub edited: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub(super) struct Shared(pub Arc<Mutex<ResultState>>);
impl Shared {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(ResultState {
            stage: Stage::Main,
            watchdog_started: false,
            started: Instant::now(),
            phase_started: Instant::now(),
            parent_window: 0,
            child_window: None,
            parent_wake: None,
            child_wake: None,
            mode: None,
            field_path: vec![],
            original: None,
            edited: None,
            error: None,
        })))
    }
    pub fn fail(&self, message: impl Into<String>) {
        let mut s = self.0.lock().unwrap();
        if !matches!(s.stage, Stage::Done | Stage::Failed) {
            s.error = Some(message.into());
            s.stage = Stage::Failed;
        }
    }
    pub fn stage(&self) -> Stage {
        self.0.lock().unwrap().stage
    }
    pub fn transition(&self, stage: Stage) {
        if let Ok(mut result) = self.0.lock() {
            result.stage = stage;
            result.phase_started = Instant::now();
        }
    }
}

/// Read the fixed demo integer override from FileProvider's pretty TOML output.
/// This intentionally accepts only the simple integer contract of text-size.
pub(super) fn read_integer_override(
    source: &str,
    path: &[String],
) -> Result<Option<String>, String> {
    if path.len() != 2 {
        return Err(format!("expected two-part demo field path, got {path:?}"));
    }
    let wanted_table = format!("[{}]", path[0]);
    let mut in_table = false;
    for line in source.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_table = line == wanted_table;
            continue;
        }
        if in_table {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() != path[1] {
                continue;
            }
            let value = value.trim();
            if value.parse::<i64>().is_ok() {
                return Ok(Some(value.into()));
            }
            return Err(format!("saved demo field is not an integer: {value}"));
        }
    }
    Ok(None)
}

/// Fixed opt-in scenario for the existing mixed container demo. It retains only
/// observations; all mutations below enter the production native event handler.
pub(super) struct PresentationAcceptance {
    pub shared: Shared,
    identities: Vec<(u64, Option<u32>, Option<usize>)>,
    original_size: Option<super::WindowSize>,
    others: Option<crate::OthersOptions>,
    list_order: Vec<String>,
    toolbar_point: Option<tekes_ui_scene::Point>,
    navigation_target: Option<String>,
}
impl PresentationAcceptance {
    pub fn new() -> Self {
        Self {
            shared: Shared::new(),
            identities: Vec::new(),
            original_size: None,
            others: None,
            list_order: Vec::new(),
            toolbar_point: None,
            navigation_target: None,
        }
    }
}
impl super::Host {
    fn acceptance_index(&self, id: &str) -> anyhow::Result<usize> {
        self.panels
            .iter()
            .position(|panel| panel.module_id == id)
            .ok_or_else(|| anyhow::anyhow!("presentation fixture container missing: {id}"))
    }
    fn acceptance_identities(&mut self) -> Vec<(u64, Option<u32>, Option<usize>)> {
        self.panels
            .iter_mut()
            .map(|panel| {
                let mut original = None;
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |host| {
                        original = Some(host as *mut super::ContentHost as usize)
                    });
                }
                (panel.instance, panel.pid, original)
            })
            .collect()
    }
    fn acceptance_input(
        &mut self,
        event: super::Event,
        w: &mut super::Window,
    ) -> anyhow::Result<()> {
        eprintln!(
            "PRESENTATION_ACCEPTANCE_INPUT step={} automated_native=true event={event:?}",
            self.step
        );
        self.event(event, w)
    }
    fn acceptance_menu(&mut self, title: &str, w: &mut super::Window) -> anyhow::Result<()> {
        let command = self
            .presentation_menu()
            .items
            .into_iter()
            .find_map(|item| match item {
                tekes_ui_platform_macos::MenuItem::Command(command) if command.title == title => {
                    Some(command.command)
                }
                _ => None,
            })
            .ok_or_else(|| {
                anyhow::anyhow!("production presentation menu command missing: {title}")
            })?;
        self.acceptance_input(super::Event::Command(command), w)
    }
    fn acceptance_text(
        &mut self,
        id: &str,
    ) -> anyhow::Result<(String, Option<std::ops::Range<usize>>)> {
        let index = self.acceptance_index(id)?;
        let mut value = None;
        self.panels[index]
            .local
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("{id} is not local"))?
            .with_host(&mut |host| {
                value = host
                    .get(super::ComponentId(1))
                    .and_then(|node| {
                        node.component::<tekes_ui_controls::composition::TextFieldContent>()
                    })
                    .map(|field| {
                        (
                            field.surface.input.text.clone(),
                            field.surface.input.marked.clone(),
                        )
                    });
            });
        value.ok_or_else(|| anyhow::anyhow!("original text field missing: {id}"))
    }
    fn acceptance_focus_text(&mut self, id: &str, w: &mut super::Window) -> anyhow::Result<()> {
        let label = format!("Input {id}");
        let target = w
            .content_accessibility_publication()
            .and_then(|publication| {
                publication
                    .nodes()
                    .iter()
                    .find(|node| node.label == label && node.enabled)
                    .map(|node| node.key)
            })
            .ok_or_else(|| anyhow::anyhow!("visible original input missing: {label}"))?;
        self.acceptance_input(
            super::Event::Accessibility {
                target,
                action: tekes_ui_platform_macos::AccessibilityAction::Focus,
            },
            w,
        )
    }
    fn acceptance_button_point(
        &self,
        owner: usize,
        presenter: usize,
        command: u64,
    ) -> anyhow::Result<tekes_ui_scene::Point> {
        let title = &self.panels[owner]
            .toolbar
            .iter()
            .find(|item| item.id == command)
            .ok_or_else(|| anyhow::anyhow!("original command missing: {owner}/{command}"))?
            .title;
        let control = Self::toolbar_control(super::ToolbarTarget::Panel(owner));
        self.panels[presenter].mounted_toolbar.host.semantics().into_iter()
            .find(|node| node.node.id == control).into_iter().flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == *title && !part.semantics.disabled)
            .map(|(_, part)| part.frame.center())
            .ok_or_else(|| anyhow::anyhow!("visible original toolbar button missing: owner={owner} presenter={presenter} command={command}"))
    }
    fn acceptance_click(
        &mut self,
        point: tekes_ui_scene::Point,
        w: &mut super::Window,
    ) -> anyhow::Result<()> {
        for down in [true, false] {
            self.acceptance_input(
                super::Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                    kind: if down {
                        tekes_ui_platform_macos::MouseEventKind::Down {
                            button: tekes_ui_platform_macos::MouseButton::Left,
                            click_count: 1,
                        }
                    } else {
                        tekes_ui_platform_macos::MouseEventKind::Up {
                            button: tekes_ui_platform_macos::MouseButton::Left,
                            click_count: 1,
                        }
                    },
                    position: tekes_ui_platform_macos::Point {
                        x: point.x as f64,
                        y: point.y as f64,
                    },
                    modifiers: tekes_ui_platform_macos::Modifiers::NONE,
                }),
                w,
            )?;
        }
        Ok(())
    }
    fn acceptance_command_count(&self, owner: usize, command: u64) -> usize {
        self.panels[owner]
            .acceptance_commands
            .as_ref()
            .map_or(0, |trace| {
                trace.iter().filter(|(id, _)| *id == command).count()
            })
    }
    pub(super) fn accept_presentation(
        &mut self,
        w: &mut super::Window,
        probe: &mut PresentationAcceptance,
    ) -> anyhow::Result<()> {
        use super::{Event, Rect};
        use crate::OthersTab;
        use anyhow::ensure;
        if probe.shared.stage() == Stage::Failed {
            anyhow::bail!(
                "{}",
                probe
                    .shared
                    .0
                    .lock()
                    .unwrap()
                    .error
                    .as_deref()
                    .unwrap_or("presentation acceptance failed")
            );
        }
        if probe.shared.stage() == Stage::Done {
            return Ok(());
        }
        if !w.is_active() || !w.is_key_window() || w.is_occluded() {
            w.request_redraw();
            return Ok(());
        }
        if let Some(nav) = &self.navigation
            && let Some(error) = &nav.error
        {
            anyhow::bail!("navigation failed: {error}");
        }
        if let Some(panel) = self.panels.iter().find(|panel| panel.failed.is_some()) {
            anyhow::bail!("container {} failed: {:?}", panel.module_id, panel.failed);
        }
        let ready = self
            .navigation
            .as_ref()
            .is_some_and(|nav| !nav.busy && nav.host.input_open)
            && self.panels.iter().all(|panel| {
                panel.bounds.size.width <= 0.
                    || panel.frame_viewport == panel.viewport
                        && (panel.local.is_some() || panel.pid.is_some() && panel.image.is_some())
            });
        if !ready {
            w.request_redraw();
            return Ok(());
        }
        let notes = self.acceptance_index("notes")?;
        let tools = self.acceptance_index("tools")?;
        let remote = self.acceptance_index("demo-first")?;
        let plain = self.acceptance_index("plain")?;
        if self.step > 0 {
            ensure!(
                self.acceptance_identities() == probe.identities,
                "presentation recreated a container, local host or original worker PID"
            );
        }
        match self.step {
            0 => {
                let state = self
                    .presentation
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("presentation fixture missing"))?;
                ensure!(
                    state.focus_id() == "notes"
                        && state.nav_bar_id() == Some("navbar")
                        && state.others_visible(),
                    "requires the mixed nav-others demo"
                );
                ensure!(
                    self.panels[remote].pid.is_some()
                        && self.panels[remote].bounds.size.width == 0.,
                    "initial hidden remote reply not observed"
                );
                probe.identities = self.acceptance_identities();
                probe.original_size = Some(w.size());
                for panel in &mut self.panels {
                    panel.acceptance_commands = Some(Vec::new());
                }
                eprintln!(
                    "PRESENTATION_ACCEPTANCE_SCOPE automated_native=true os_ime=false physical_input=false system_menu_operation=false timeout_seconds=30 identities={:?}",
                    probe.identities
                );
                self.acceptance_focus_text("notes", w)?;
                self.acceptance_input(Event::TextCommand("selectAll:".into()), w)?;
                self.acceptance_input(
                    Event::TextPreedit {
                        text: "かな".into(),
                        selection: 0..2,
                        clauses: std::iter::once(0..2).collect(),
                        replacement: None,
                    },
                    w,
                )?;
            }
            1 => {
                ensure!(
                    self.acceptance_text("notes")? == ("かな".into(), Some(0..2)),
                    "original preedit state missing"
                );
                self.acceptance_input(
                    Event::TextCommit {
                        text: "かな".into(),
                        replacement: None,
                    },
                    w,
                )?;
            }
            2 => {
                ensure!(
                    self.acceptance_text("notes")? == ("かな".into(), None),
                    "preedit commit duplicated or retained marked state"
                );
                self.acceptance_menu("Promote tools", w)?;
            }
            3 => {
                let state = self.presentation.as_ref().unwrap();
                ensure!(
                    state.focus_id() == "tools"
                        && state.others_ids().iter().any(|id| id == "notes"),
                    "promotion did not swap original containers"
                );
                ensure!(
                    self.acceptance_text("notes")?.0 == "かな",
                    "promotion lost original text"
                );
                self.acceptance_menu("Select plain", w)?;
                probe.others = self.presentation.as_ref().unwrap().others().cloned();
                self.acceptance_menu("Show / Hide Others", w)?;
            }
            4 => {
                ensure!(
                    !self.presentation.as_ref().unwrap().others_visible()
                        && self.presentation_layout.tabs.is_none(),
                    "others off retained a painted tab region"
                );
                self.acceptance_menu("Show / Hide Others", w)?;
            }
            5 => {
                ensure!(
                    self.presentation.as_ref().unwrap().others() == probe.others.as_ref(),
                    "others off/on lost selection or order"
                );
                self.acceptance_menu("Promote notes", w)?;
                self.acceptance_menu("Select demo-first", w)?;
            }
            6 => {
                ensure!(
                    self.presentation.as_ref().unwrap().focus_id() == "notes"
                        && self.panels[tools].bounds.size.width == 0.,
                    "retained owner must be hidden"
                );
                let first = self.acceptance_button_point(tools, notes, 101)?;
                let second = self.acceptance_button_point(tools, remote, 102)?;
                self.acceptance_click(first, w)?;
                self.acceptance_click(second, w)?;
                ensure!(
                    self.acceptance_command_count(tools, 101) == 1
                        && self.acceptance_command_count(tools, 102) == 1,
                    "retained buttons did not each invoke their original owner once"
                );
                ensure!(
                    self.acceptance_command_count(notes, 101) == 0
                        && self.acceptance_command_count(remote, 102) == 0,
                    "retained display changed business owner"
                );
                self.acceptance_menu("Select tools", w)?;
            }
            7 => {
                ensure!(
                    self.panels[notes]
                        .mounted_toolbar
                        .source(Self::toolbar_group(super::ToolbarTarget::Panel(tools)))
                        .is_none()
                        && self.panels[remote]
                            .mounted_toolbar
                            .source(Self::toolbar_group(super::ToolbarTarget::Panel(tools)))
                            .is_none(),
                    "old retained presentation survived restore"
                );
                let point = self.acceptance_button_point(tools, tools, 101)?;
                self.acceptance_click(point, w)?;
                ensure!(
                    self.acceptance_command_count(tools, 101) == 2,
                    "restored toolbar lost its original owner or duplicated activation"
                );
                self.acceptance_menu("Select List", w)?;
            }
            8 => {
                ensure!(
                    self.presentation.as_ref().unwrap().selected_tab() == Some(&OthersTab::List),
                    "List did not become the selected special Tab"
                );
                probe.list_order = self
                    .presentation
                    .as_ref()
                    .unwrap()
                    .list_ids()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                probe.toolbar_point = Some(self.acceptance_button_point(tools, tools, 101)?);
                self.acceptance_focus_text("tools", w)?;
                self.acceptance_input(Event::TextCommand("selectAll:".into()), w)?;
                self.acceptance_input(
                    Event::TextPreedit {
                        text: "リスト".into(),
                        selection: 0..3,
                        clauses: std::iter::once(0..3).collect(),
                        replacement: None,
                    },
                    w,
                )?;
                let viewport = self
                    .presentation_layout
                    .list_viewport
                    .ok_or_else(|| anyhow::anyhow!("List viewport missing"))?;
                self.acceptance_input(
                    Event::Scroll(tekes_ui_platform_macos::ScrollEvent {
                        timestamp: self.motion_epoch.elapsed().as_secs_f64(),
                        delta: tekes_ui_platform_macos::Vec2 { x: 0., y: -80. },
                        precise: true,
                        phase: tekes_ui_platform_macos::ScrollPhase::Began,
                        momentum_phase: tekes_ui_platform_macos::ScrollPhase::None,
                        inverted: false,
                        position: tekes_ui_platform_macos::Point {
                            x: viewport.center().x as f64,
                            y: viewport.center().y as f64,
                        },
                        modifiers: tekes_ui_platform_macos::Modifiers::NONE,
                    }),
                    w,
                )?;
            }
            9 => {
                ensure!(
                    self.presentation.as_ref().unwrap().list_scroll() > 0.,
                    "List scroll did not reach production layout"
                );
                ensure!(
                    self.acceptance_text("tools")? == ("リスト".into(), Some(0..3)),
                    "List scroll changed original editor/preedit"
                );
                let viewport = self.presentation_layout.list_viewport.unwrap();
                let panel = &self.panels[tools];
                ensure!(
                    panel.bounds.min_y() < viewport.min_y()
                        && panel.clip.min_y() >= viewport.min_y()
                        && panel.clip.max_y() <= viewport.max_y(),
                    "List item did not keep full layout with clipped geometry"
                );
                ensure!(
                    !panel
                        .mounted_toolbar
                        .host
                        .semantics()
                        .iter()
                        .any(|node| node.node.id
                            == Self::toolbar_control(super::ToolbarTarget::Panel(tools))),
                    "clipped-out Toolbar remained semantically hittable"
                );
                self.acceptance_input(
                    Event::TextCommit {
                        text: "リスト".into(),
                        replacement: None,
                    },
                    w,
                )?;
                self.acceptance_click(probe.toolbar_point.unwrap(), w)?;
                ensure!(
                    self.acceptance_command_count(tools, 101) == 2,
                    "old clipped Toolbar point activated a command through content"
                );
                self.acceptance_menu("Move down tools", w)?;
            }
            10 => {
                let order: Vec<_> = self
                    .presentation
                    .as_ref()
                    .unwrap()
                    .list_ids()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                ensure!(
                    order != probe.list_order
                        && order.iter().position(|id| id == "tools")
                            > probe.list_order.iter().position(|id| id == "tools"),
                    "List reorder did not affect actual identities"
                );
                ensure!(
                    self.acceptance_text("tools")? == ("リスト".into(), None),
                    "List reorder lost editor contents"
                );
                let viewport = self.presentation_layout.list_viewport.unwrap();
                for panel in &self.panels {
                    if panel.bounds.size.width > 0.
                        && panel.module_id != "notes"
                        && panel.module_id != "navbar"
                    {
                        ensure!(
                            panel.clip.min_y() >= viewport.min_y()
                                && panel.clip.max_y() <= viewport.max_y(),
                            "List paint/input clip escaped viewport"
                        );
                    }
                }
                self.acceptance_menu("Move up tools", w)?;
            }
            11 => {
                ensure!(
                    self.presentation
                        .as_ref()
                        .unwrap()
                        .list_ids()
                        .into_iter()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                        == probe.list_order,
                    "List reorder restore failed"
                );
                w.set_content_size(super::WindowSize {
                    width: 1180.,
                    height: 760.,
                });
            }
            12 => {
                ensure!(
                    w.size().width == 1180. && w.size().height == 760.,
                    "native resize did not settle"
                );
                let (safe, leading) = w.titlebar_safe_area();
                let focus = self.presentation.as_ref().unwrap().focus_id();
                let focus_frame = self
                    .presentation_layout
                    .frames
                    .iter()
                    .find(|(id, _)| id == focus)
                    .unwrap()
                    .1;
                ensure!(
                    focus_frame.min_y() == 0.,
                    "top focus retained an extra titlebar gap"
                );
                for panel in &self.panels {
                    if panel.bounds.min_y() == 0.
                        && panel.clip.min_y() == 0.
                        && panel.bounds.size.width > 0.
                    {
                        if panel.bounds.min_x() < leading as f32 {
                            ensure!(
                                panel.geometry.insets.top >= safe as f32,
                                "content overlaps native controls"
                            );
                        }
                        for (_, part) in panel
                            .mounted_toolbar
                            .host
                            .semantics()
                            .into_iter()
                            .flat_map(|node| node.parts)
                        {
                            ensure!(
                                part.frame.min_x() >= leading as f32
                                    || part.frame.min_y() >= safe as f32,
                                "Toolbar hit target overlaps native controls"
                            );
                        }
                    }
                }
                if let Some(tabs) = self.presentation_layout.tabs {
                    ensure!(
                        tabs.min_x() > focus_frame.max_x(),
                        "Others is not to the right of focus"
                    );
                }
                ensure!(
                    self.acceptance_text("tools")? == ("リスト".into(), None),
                    "resize changed editor state"
                );
                eprintln!(
                    "PRESENTATION_ACCEPTANCE_GEOMETRY scale={} drawable={:?} safe_height={} frames={:?} clips={:?}",
                    w.scale(),
                    w.drawable_size(),
                    safe,
                    self.presentation_layout.frames,
                    self.presentation_layout.clips
                );
                self.acceptance_menu("Select plain", w)?;
            }
            13 => {
                self.acceptance_focus_text("plain", w)?;
                self.acceptance_input(Event::TextCommand("selectAll:".into()), w)?;
                self.acceptance_input(
                    Event::TextCommit {
                        text: "handoff".into(),
                        replacement: None,
                    },
                    w,
                )?;
            }
            14 => {
                ensure!(
                    self.focused == plain
                        && self.acceptance_text("plain")? == ("handoff".into(), None)
                        && self.acceptance_text("notes")? == ("かな".into(), None),
                    "production focus handoff routed text to a previous owner"
                );
                self.acceptance_menu("Promote tools", w)?;
            }
            15 => {
                ensure!(
                    self.panels[notes].bounds.size.width == 0.
                        && self.panels[remote].bounds.size.width == 0.,
                    "both original navigation recipients must remain hidden"
                );
                probe.navigation_target = self
                    .navigation
                    .as_ref()
                    .and_then(|nav| nav.host.target.as_ref().map(|entry| entry.id.clone()));
                let title = self
                    .navigation
                    .as_ref()
                    .unwrap()
                    .actions
                    .iter()
                    .find(|action| {
                        action.command.id == crate::navigation::BACK && action.command.enabled
                    })
                    .map(|action| {
                        action
                            .accessible_name
                            .as_deref()
                            .unwrap_or(&action.command.title)
                            .to_owned()
                    })
                    .ok_or_else(|| anyhow::anyhow!("original Back action unavailable"))?;
                let presenter = self
                    .toolbar_container(super::ToolbarTarget::Navigation)
                    .ok_or_else(|| anyhow::anyhow!("navigation toolbar not mounted"))?;
                ensure!(
                    self.panels[presenter].module_id == "navbar",
                    "requires the explicit acceptance-only Navbar navigation fixture"
                );
                let point = self.panels[presenter]
                    .mounted_toolbar
                    .host
                    .semantics()
                    .into_iter()
                    .find(|node| {
                        node.node.id == Self::toolbar_control(super::ToolbarTarget::Navigation)
                    })
                    .into_iter()
                    .flat_map(|node| node.parts)
                    .find(|(_, part)| part.semantics.name == title && !part.semantics.disabled)
                    .map(|(_, part)| part.frame.center())
                    .ok_or_else(|| anyhow::anyhow!("visible original Back button missing"))?;
                self.acceptance_click(point, w)?;
            }
            16 => {
                let target = self
                    .navigation
                    .as_ref()
                    .and_then(|nav| nav.host.target.as_ref().map(|entry| entry.id.clone()));
                ensure!(
                    target != probe.navigation_target
                        && self.navigation_order.is_none()
                        && self.navigation_ack.is_none(),
                    "hidden targets navigation did not commit"
                );
                let remote_trace = self.panels[remote].acceptance_commands.as_ref().unwrap();
                ensure!(
                    remote_trace.len() == 1,
                    "hidden remote navigation command was lost or duplicated"
                );
                let (command, sequence) = remote_trace[0];
                ensure!(
                    self.panels[remote].frame_seq >= sequence
                        && self.acceptance_command_count(notes, command) == 1
                        && self.acceptance_command_count(tools, command) == 0
                        && self.acceptance_command_count(plain, command) == 0,
                    "navigation failed its explicit targets/real remote reply contract"
                );
                ensure!(
                    self.panels[remote].bounds == Rect::default()
                        && self.panels[notes].bounds == Rect::default(),
                    "navigation implicitly revealed its hidden target"
                );
                eprintln!(
                    "PRESENTATION_ACCEPTANCE_HIDDEN_NAV command={} sequence={} reply={} pid={:?} target={target:?} local_once=true remote_once=true automated_native=true",
                    command, sequence, self.panels[remote].frame_seq, self.panels[remote].pid
                );
                self.acceptance_menu("Promote notes", w)?;
            }
            17 => {
                ensure!(
                    self.presentation.as_ref().unwrap().focus_id() == "notes"
                        && self.panels[notes].bounds.size.width > 0.
                        && self.acceptance_text("notes")? == ("かな".into(), None)
                        && self.acceptance_text("tools")? == ("リスト".into(), None),
                    "revealing original targets lost presentation or editor state"
                );
                if let Some(size) = probe.original_size {
                    w.set_content_size(size);
                }
            }
            18 => {
                ensure!(
                    self.acceptance_draws >= 18,
                    "scenario did not render the real native Window"
                );
                probe.shared.transition(Stage::Done);
                eprintln!(
                    "PRESENTATION_ACCEPTANCE_PASS steps=19 draws={} automated_native=true os_ime=false physical_input=false system_menu_operation=false native_window={} identities={:?} commands={:?}",
                    self.acceptance_draws,
                    w.native_window_number(),
                    probe.identities,
                    self.panels
                        .iter()
                        .map(|panel| (&panel.module_id, &panel.acceptance_commands))
                        .collect::<Vec<_>>()
                );
                w.close();
                return Ok(());
            }
            _ => anyhow::bail!("unknown presentation acceptance step {}", self.step),
        }
        eprintln!(
            "PRESENTATION_ACCEPTANCE_STEP step={} draws={} focused={} focus={} elapsed_ms={}",
            self.step,
            self.acceptance_draws,
            self.focused,
            self.presentation.as_ref().unwrap().focus_id(),
            probe.shared.0.lock().unwrap().started.elapsed().as_millis()
        );
        self.step += 1;
        w.request_redraw();
        Ok(())
    }
}

#[cfg(test)]
mod presentation_trace_tests {
    use super::super::*;
    use std::{cell::Cell, rc::Rc};
    #[test]
    fn optional_command_witness_counts_only_original_accepted_local_commands() {
        let calls = Rc::new(Cell::new(0));
        let original = calls.clone();
        let local = crate::LocalContainer::new(
            "trace",
            tekes_ui_controls::composition::Content::new(
                ComponentId(1),
                tekes_ui_controls::composition::TextContent::new("Trace"),
            ),
        )
        .unwrap()
        .toolbar(vec![
            ToolbarItem {
                id: 42,
                title: "Accepted".into(),
                enabled: true,
                geometry_dependent: false,
            },
            ToolbarItem {
                id: 43,
                title: "Disabled".into(),
                enabled: false,
                geometry_dependent: false,
            },
        ])
        .on_command(move |_, command| {
            assert_eq!(command, 42);
            original.set(original.get() + 1);
            Ok(())
        });
        let mut host = Host::from_columns(vec![local.into()], false, None);
        host.mount_initial_containers();
        let panel = &mut host.panels[0];
        panel.command(42).unwrap();
        assert!(
            panel.acceptance_commands.is_none(),
            "normal production does not allocate witness history"
        );
        panel.acceptance_commands = Some(Vec::new());
        panel.command(43).unwrap();
        panel.command(999).unwrap();
        panel.request(InputKind::Layout, 0., 0.).unwrap();
        assert!(panel.acceptance_commands.as_ref().unwrap().is_empty());
        let before = panel.seq;
        panel.command(42).unwrap();
        assert_eq!(calls.get(), 2);
        assert_eq!(
            panel.acceptance_commands.as_ref().unwrap(),
            &[(42, before + 1)]
        );
    }
}
