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
