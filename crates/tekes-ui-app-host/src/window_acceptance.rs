use std::{sync::{Arc, Mutex}, time::Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage { Main, Opening, AwaitingShown, Shown, Editing, Saving, Undoing, Restoring, Closing, Done, Failed }
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
    pub fn new() -> Self { Self(Arc::new(Mutex::new(ResultState { stage: Stage::Main, watchdog_started: false, started: Instant::now(), phase_started: Instant::now(), parent_window: 0, child_window: None, parent_wake: None, child_wake: None, mode: None, field_path: vec![], original: None, edited: None, error: None }))) }
    pub fn fail(&self, message: impl Into<String>) {
        let mut s = self.0.lock().unwrap();
        if !matches!(s.stage, Stage::Done | Stage::Failed) { s.error = Some(message.into()); s.stage = Stage::Failed; }
    }
    pub fn stage(&self) -> Stage { self.0.lock().unwrap().stage }
    pub fn transition(&self, stage: Stage) {
        if let Ok(mut result) = self.0.lock() {
            result.stage = stage;
            result.phase_started = Instant::now();
        }
    }
}

/// Read the fixed demo integer override from FileProvider's pretty TOML output.
/// This intentionally accepts only the simple integer contract of text-size.
pub(super) fn read_integer_override(source: &str, path: &[String]) -> Result<Option<String>, String> {
    if path.len() != 2 { return Err(format!("expected two-part demo field path, got {path:?}")); }
    let wanted_table = format!("[{}]", path[0]);
    let mut in_table = false;
    for line in source.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            in_table = line == wanted_table;
            continue;
        }
        if in_table {
            let Some((key, value)) = line.split_once('=') else { continue; };
            if key.trim() != path[1] { continue; }
            let value = value.trim();
            if value.parse::<i64>().is_ok() { return Ok(Some(value.into())); }
            return Err(format!("saved demo field is not an integer: {value}"));
        }
    }
    Ok(None)
}
