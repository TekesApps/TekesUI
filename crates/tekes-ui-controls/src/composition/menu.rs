//! Original Menu composition; input and painting stay in MenuRuntime.
use super::*;
use crate::{MenuEvent, MenuView, MenuWheelInput};

/// Controlled declarations for the original Menu runtime. Values identify an
/// entry among its siblings; the delivered path includes all submenu values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item {
        value: String,
        label: String,
        checked: Option<bool>,
        disabled: bool,
    },
    Submenu {
        value: String,
        label: String,
        disabled: bool,
        entries: Vec<MenuEntry>,
    },
    Separator,
}
impl MenuEntry {
    pub fn item(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Item {
            value: value.into(),
            label: label.into(),
            checked: None,
            disabled: false,
        }
    }
    pub fn submenu(value: impl Into<String>, label: impl Into<String>, entries: Vec<Self>) -> Self {
        Self::Submenu {
            value: value.into(),
            label: label.into(),
            disabled: false,
            entries,
        }
    }
    fn label(&self) -> &str {
        match self {
            Self::Item { label, .. } | Self::Submenu { label, .. } => label,
            Self::Separator => "",
        }
    }
    fn value(&self) -> Option<&str> {
        match self {
            Self::Item { value, .. } | Self::Submenu { value, .. } => Some(value),
            Self::Separator => None,
        }
    }
    fn style(&self) -> crate::composite::MenuRowStyle {
        match self {
            Self::Item {
                checked, disabled, ..
            } => crate::composite::MenuRowStyle {
                checked: *checked,
                disabled: *disabled,
                ..Default::default()
            },
            Self::Submenu { disabled, .. } => crate::composite::MenuRowStyle {
                submenu: true,
                disabled: *disabled,
                ..Default::default()
            },
            Self::Separator => crate::composite::MenuRowStyle {
                separator: true,
                ..Default::default()
            },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEntriesError {
    EmptyValue,
    DuplicateValue,
    EmptyLabel,
}
fn validate_entries(entries: &[MenuEntry]) -> Result<(), MenuEntriesError> {
    let mut values = BTreeSet::new();
    for entry in entries {
        if let Some(value) = entry.value() {
            if value.trim().is_empty() {
                return Err(MenuEntriesError::EmptyValue);
            }
            if !values.insert(value) {
                return Err(MenuEntriesError::DuplicateValue);
            }
            if entry.label().trim().is_empty() {
                return Err(MenuEntriesError::EmptyLabel);
            }
        }
        if let MenuEntry::Submenu { entries, .. } = entry {
            validate_entries(entries)?;
        }
    }
    Ok(())
}
fn part_identity() -> usize {
    // Vec-backed legacy item positions cannot occupy the high-bit namespace.
    // A delayed declared token must never alias a positional item after a
    // consumer switches back to the retained flat API. IDs remain opaque usize.
    usize::try_from(actions::next_mount())
        .ok()
        .and_then(|identity| identity.checked_add(1usize << (usize::BITS - 1)))
        .expect("menu semantic identity exhausted")
}
#[derive(Debug)]
struct MenuPanel {
    path: Vec<usize>,
    view: MenuView,
    layout: Option<crate::MenuLayout>,
    parts: Vec<usize>,
    group: usize,
    parent: usize,
}
/// One semantic menu owner with its existing MenuView item contracts and an
/// optional decorative trigger slot, sharing the original CompoundButton rules.
/// Its popup is painted/routed in the host overlay pass, above later siblings.
#[derive(Debug)]
pub struct MenuContent {
    pub trigger: Button,
    pub menu: MenuView,
    pub labels: Vec<String>,
    pub selected: Option<usize>,
    pub popup: Option<crate::MenuLayout>,
}
#[derive(Debug)]
pub(crate) struct MenuContentState {
    model_labels: Vec<String>,
    model_revision: u64,
    values: Option<Vec<String>>,
    selection_event: Option<MenuSelection>,
    entries: Option<Vec<MenuEntry>>,
    panels: Vec<MenuPanel>,
    retiring: Vec<MenuPanel>,
    root_parts: Vec<usize>,
    root_group: Option<usize>,
    root_generation: u64,
    trigger_content: Vec<Content>,
    trigger_measured: Size,
    last_frame: Option<std::time::Duration>,
}
impl MenuContentState {
    fn new(labels: &[String]) -> Self {
        Self {
            model_labels: labels.to_vec(),
            model_revision: actions::next_mount(),
            values: None,
            selection_event: None,
            entries: None,
            panels: Vec::new(),
            retiring: Vec::new(),
            root_parts: Vec::new(),
            root_group: None,
            root_generation: 0,
            trigger_content: Vec::new(),
            trigger_measured: Size::default(),
            last_frame: None,
        }
    }
    fn reset(&mut self, labels: &[String]) {
        let content = std::mem::take(&mut self.trigger_content);
        let measured = self.trigger_measured;
        *self = Self::new(labels);
        self.trigger_content = content;
        self.trigger_measured = measured;
    }
    fn entries_at(&self, path: &[usize]) -> Option<&[MenuEntry]> {
        let mut entries = self.entries.as_deref()?;
        for &index in path {
            let MenuEntry::Submenu {
                entries: children, ..
            } = entries.get(index)?
            else {
                return None;
            };
            entries = children;
        }
        Some(entries)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuValuesError {
    Count,
    Empty,
    Duplicate,
}
impl MenuContent {
    pub fn new(label: impl Into<String>, labels: Vec<String>) -> Self {
        let mut menu = MenuView::new(labels.len());
        menu.content_state = Some(MenuContentState::new(&labels));
        Self {
            trigger: Button::new(label.into()),
            menu,
            labels,
            selected: None,
            popup: None,
        }
    }
    /// One original trigger action with arbitrary decorative Content, using
    /// CompoundButton's same slot geometry/chrome. Call before mounting; host
    /// validation rejects interactive descendants and duplicate identities.
    pub fn with_trigger_content(
        mut self,
        accessible_name: impl Into<String>,
        content: Content,
    ) -> Result<Self, CompositionError> {
        let name = accessible_name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.reconcile_model();
        self.trigger.label = name;
        self.menu.content_state.as_mut().unwrap().trigger_content = vec![content];
        Ok(self)
    }
    /// Atomically replace a controlled tree. Equal declarations are a no-op;
    /// changed declarations revoke old popup generations and held input.
    pub fn set_entries(&mut self, entries: Vec<MenuEntry>) -> Result<(), MenuEntriesError> {
        validate_entries(&entries)?;
        self.reconcile_model();
        if self
            .menu
            .content_state
            .as_ref()
            .and_then(|state| state.entries.as_ref())
            == Some(&entries)
        {
            return Ok(());
        }
        let labels = entries
            .iter()
            .map(|entry| entry.label().to_owned())
            .collect::<Vec<_>>();
        self.menu.replace_items(labels.len());
        self.menu
            .set_row_styles(entries.iter().map(MenuEntry::style).collect());
        self.labels = labels;
        let state = self
            .menu
            .content_state
            .as_mut()
            .expect("reconciled declaration");
        state.reset(&self.labels);
        state.entries = Some(entries);
        self.selected = None;
        self.popup = None;
        Ok(())
    }
    fn declared(&self) -> bool {
        self.menu
            .content_state
            .as_ref()
            .is_some_and(|state| state.entries.is_some())
    }
    fn retire_after(&mut self, level: usize) {
        let state = self.menu.content_state.as_mut().expect("declaration");
        for mut panel in state.panels.split_off(level.min(state.panels.len())) {
            panel.view.deactivate();
            state.retiring.push(panel);
        }
    }
    fn close_panels(&mut self) {
        if let Some(state) = &mut self.menu.content_state {
            for panel in &mut state.panels {
                panel.view.deactivate();
            }
        }
    }
    fn reconcile_popup_generation(&mut self) {
        if !self.declared() {
            return;
        }
        let generation = self.menu.open_generation();
        if self.menu.content_state.as_ref().unwrap().root_generation != generation {
            self.retire_after(0);
            let state = self.menu.content_state.as_mut().unwrap();
            state.root_generation = generation;
            state.root_parts = (0..self.labels.len()).map(|_| part_identity()).collect();
            state.root_group = Some(part_identity());
            self.popup = None;
        }
    }
    fn show_menu(&mut self) {
        if self.declared() && self.labels.is_empty() {
            return;
        }
        self.menu.show(0);
        self.reconcile_popup_generation();
    }
    /// Bind stable flat values to current labels. Validation is atomic. Model
    /// changes cancel the old popup; unchanged bindings are an idempotent no-op.
    pub fn set_values(&mut self, values: Vec<String>) -> Result<(), MenuValuesError> {
        if values.len() != self.labels.len() {
            return Err(MenuValuesError::Count);
        }
        if values.iter().any(|v| v.trim().is_empty()) {
            return Err(MenuValuesError::Empty);
        }
        if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(MenuValuesError::Duplicate);
        }
        self.reconcile_model();
        if self
            .menu
            .content_state
            .as_ref()
            .and_then(|state| state.values.as_ref())
            != Some(&values)
        {
            self.menu.replace_items(self.labels.len());
            self.popup = None;
            let state = self
                .menu
                .content_state
                .as_mut()
                .expect("reconciled declaration");
            state.model_revision = actions::next_mount();
            state.values = Some(values);
            state.selection_event = None;
            state.entries = None;
            state.panels.clear();
            state.retiring.clear();
            state.root_parts.clear();
            state.root_group = None;
        }
        Ok(())
    }
    /// Controlled check state is independent of the active keyboard item.
    /// A changed declaration revokes old popup/held input; equal is a no-op.
    pub fn set_item_checked(&mut self, index: usize, checked: bool) -> bool {
        self.update_item_style(index, |style| style.checked = Some(checked))
    }
    /// Disabled items remain readable but cannot receive item focus or activate.
    pub fn set_item_disabled(&mut self, index: usize, disabled: bool) -> bool {
        self.update_item_style(index, |style| style.disabled = disabled)
    }
    /// Reuse the existing decorative separator painter; the model index stays
    /// stable, but this row is omitted from actionable semantic descendants.
    pub fn set_item_separator(&mut self, index: usize, separator: bool) -> bool {
        self.update_item_style(index, |style| style.separator = separator)
    }
    fn update_item_style(
        &mut self,
        index: usize,
        update: impl FnOnce(&mut crate::composite::MenuRowStyle),
    ) -> bool {
        self.reconcile_model();
        // Legacy flat setters do not create a second source for a controlled
        // tree. Its owner updates the original declaration with set_entries.
        if self.declared() {
            return false;
        }
        if index >= self.labels.len() {
            return false;
        }
        let mut styles = (0..self.labels.len())
            .map(|n| self.menu.item_style(n).unwrap_or_default())
            .collect::<Vec<_>>();
        let before = styles[index];
        update(&mut styles[index]);
        if before == styles[index] {
            return false;
        }
        self.menu.deactivate();
        self.menu.clear_presentation();
        self.menu.set_row_styles(styles);
        self.popup = None;
        let state = self
            .menu
            .content_state
            .as_mut()
            .expect("reconciled declaration");
        state.model_revision = actions::next_mount();
        state.selection_event = None;
        true
    }
    /// None until direct legacy label mutation is reconciled. A positional caller
    /// captures this revision with its value map, or explicitly binds values.
    pub fn model_revision(&self) -> Option<u64> {
        let state = self.menu.content_state.as_ref()?;
        (self.labels == state.model_labels && self.menu.menu().items.count == self.labels.len())
            .then_some(state.model_revision)
    }
    fn reconcile_model(&mut self) {
        if self.menu.content_state.is_none() {
            self.menu.content_state = Some(MenuContentState::new(&self.labels));
        }
        if self.model_revision().is_none() {
            self.menu.replace_items(self.labels.len());
            self.menu
                .content_state
                .as_mut()
                .expect("declaration")
                .reset(&self.labels);
            self.selected = None;
            self.popup = None;
        }
    }
    fn select(&mut self, index: usize) {
        self.selected = Some(index);
        let state = self
            .menu
            .content_state
            .as_mut()
            .expect("reconciled declaration");
        state.selection_event = self.labels.get(index).map(|label| MenuSelection {
            index,
            label: label.clone(),
            model_revision: state.model_revision,
            value_path: state
                .values
                .as_ref()
                .and_then(|values| values.get(index))
                .map(|value| vec![value.clone()]),
        });
    }
    // The public trigger model remains authoritative between paints. Closing
    // through MenuView also cancels held rows and keeps ordinary exit paint.
    fn reconcile_trigger(&mut self) -> bool {
        self.reconcile_model();
        self.reconcile_popup_generation();
        let available = self.trigger.focusable();
        if !available {
            self.menu.deactivate();
            self.close_panels();
            self.trigger.button.cancel();
        }
        available
    }
    fn handle_menu(&mut self, e: &ContentEvent) -> ContentResponse {
        let was_open = self.menu.is_open();
        if !self.reconcile_trigger() {
            return ContentResponse {
                changed: was_open,
                ..Default::default()
            };
        }
        if self.declared() {
            return self.handle_declared(e);
        }
        let (result, wheel) = deliver_menu_input(&mut self.menu, e);
        if let MenuEvent::Activated { index, .. } = result {
            self.select(index);
        }
        ContentResponse {
            text_key: None,
            consumed: if matches!(e, ContentEvent::Scroll { .. }) {
                wheel
            } else {
                !matches!(
                    e,
                    ContentEvent::Key {
                        key: ControlKey::Tab { .. },
                        ..
                    }
                )
            },
            changed: !matches!(e, ContentEvent::Scroll { .. }) || wheel,
            activated: matches!(result, MenuEvent::Activated { .. }),
            capture_pointer: false,
        }
    }
}
impl MenuContent {
    fn panel_mut(&mut self, level: usize) -> Option<&mut MenuView> {
        if level == 0 {
            Some(&mut self.menu)
        } else {
            self.menu
                .content_state
                .as_mut()?
                .panels
                .get_mut(level - 1)
                .map(|panel| &mut panel.view)
        }
    }
    fn panel_path(&self, level: usize) -> Vec<usize> {
        if level == 0 {
            Vec::new()
        } else {
            self.menu.content_state.as_ref().unwrap().panels[level - 1]
                .path
                .clone()
        }
    }
    fn active_level(&self) -> usize {
        self.menu
            .content_state
            .as_ref()
            .unwrap()
            .panels
            .iter()
            .take_while(|panel| panel.view.is_open())
            .count()
    }
    fn open_submenu(&mut self, level: usize, index: usize, keyboard: bool) -> bool {
        let mut path = self.panel_path(level);
        path.push(index);
        let state = self.menu.content_state.as_ref().unwrap();
        if state
            .panels
            .get(level)
            .is_some_and(|panel| panel.path == path && panel.view.is_open())
        {
            return false;
        }
        let Some(MenuEntry::Submenu {
            entries,
            disabled: false,
            ..
        }) = state
            .entries_at(&path[..path.len() - 1])
            .and_then(|entries| entries.get(index))
        else {
            return false;
        };
        let styles = entries.iter().map(MenuEntry::style).collect::<Vec<_>>();
        let count = entries.len();
        let parent = if level == 0 {
            state.root_parts[index]
        } else {
            state.panels[level - 1].parts[index]
        };
        self.retire_after(level);
        let mut view = MenuView::new(count);
        view.set_row_styles(styles);
        view.set_submenu_placement();
        view.show(0);
        view.set_keyboard_focus(keyboard);
        self.menu
            .content_state
            .as_mut()
            .unwrap()
            .panels
            .push(MenuPanel {
                path,
                view,
                layout: None,
                parts: (0..count).map(|_| part_identity()).collect(),
                group: part_identity(),
                parent,
            });
        true
    }
    fn select_declared(&mut self, level: usize, index: usize) {
        let mut indices = self.panel_path(level);
        indices.push(index);
        let state = self.menu.content_state.as_ref().unwrap();
        let mut values = Vec::new();
        let mut entries = state.entries.as_deref().unwrap();
        let mut label = String::new();
        for (depth, &i) in indices.iter().enumerate() {
            let entry = &entries[i];
            values.push(entry.value().expect("actionable entry").to_owned());
            match entry {
                MenuEntry::Submenu {
                    entries: children, ..
                } => entries = children,
                MenuEntry::Item { label: name, .. } if depth == indices.len() - 1 => {
                    label = name.clone()
                }
                _ => return,
            }
        }
        let revision = state.model_revision;
        self.selected = indices.first().copied();
        self.menu.content_state.as_mut().unwrap().selection_event = Some(MenuSelection {
            index,
            label,
            model_revision: revision,
            value_path: Some(values),
        });
        self.menu.deactivate();
        self.close_panels();
    }
    fn declared_location(&self, part: usize) -> Option<(usize, usize)> {
        if !self.menu.is_open() || self.model_revision().is_none() {
            return None;
        }
        let state = self.menu.content_state.as_ref()?;
        if state.root_generation != self.menu.open_generation() {
            return None;
        }
        if self.popup.is_some()
            && let Some(index) = state.root_parts.iter().position(|&id| id == part)
        {
            return Some((0, index));
        }
        state.panels.iter().enumerate().find_map(|(n, panel)| {
            (panel.view.is_open() && panel.layout.is_some())
                .then(|| {
                    panel
                        .parts
                        .iter()
                        .position(|&id| id == part)
                        .map(|index| (n + 1, index))
                })
                .flatten()
        })
    }
    fn finish_declared_event(
        &mut self,
        level: usize,
        result: MenuEvent,
        keyboard: bool,
    ) -> ContentResponse {
        let request = self
            .panel_mut(level)
            .and_then(MenuView::take_submenu_request);
        if let Some(index) = request {
            self.open_submenu(level, index, keyboard);
            return ContentResponse::consumed(true);
        }
        match result {
            MenuEvent::Activated { index, .. } => {
                self.select_declared(level, index);
                ContentResponse {
                    consumed: true,
                    changed: true,
                    activated: true,
                    ..Default::default()
                }
            }
            MenuEvent::Dismissed { .. } => {
                self.menu.deactivate();
                self.close_panels();
                ContentResponse::consumed(true)
            }
            MenuEvent::None => ContentResponse::consumed(true),
        }
    }
    fn declared_ax(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let Some(part) = part else {
            if matches!(action, AccessibilityAction::Press) {
                self.show_menu();
                return ContentResponse::consumed(true);
            }
            return ContentResponse::default();
        };
        let Some((level, index)) = self.declared_location(part) else {
            return ContentResponse::default();
        };
        if matches!(action, AccessibilityAction::Focus) {
            let changed = self.panel_mut(level).unwrap().focus_item(index);
            if changed {
                self.retire_after(level);
            }
            return if changed {
                ContentResponse::consumed(true)
            } else {
                ContentResponse::default()
            };
        }
        if !matches!(action, AccessibilityAction::Press) {
            return ContentResponse::default();
        }
        let view = self.panel_mut(level).unwrap();
        let result = view.activate_item(index);
        if result == MenuEvent::None && view.take_submenu_request().is_none() {
            return ContentResponse::default();
        }
        // Activation was already guarded by the original runtime. A submenu
        // request is navigation only, never a selected business command.
        if matches!(result, MenuEvent::None) {
            self.open_submenu(level, index, true);
            ContentResponse::consumed(true)
        } else {
            self.finish_declared_event(level, result, true)
        }
    }
    fn level_at(&self, point: Point) -> Option<usize> {
        let state = self.menu.content_state.as_ref().unwrap();
        state
            .panels
            .iter()
            .enumerate()
            .rev()
            .find_map(|(n, p)| {
                p.layout
                    .as_ref()
                    .filter(|layout| {
                        p.view.is_open()
                            && layout.visible.is_some_and(|clip| clip.contains(point))
                            && layout.popup.contains(point)
                    })
                    .map(|_| n + 1)
            })
            .or_else(|| {
                self.popup
                    .as_ref()
                    .filter(|layout| {
                        layout.visible.is_some_and(|clip| clip.contains(point))
                            && layout.popup.contains(point)
                    })
                    .map(|_| 0)
            })
    }
    fn handle_declared(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.menu.deactivate();
            self.close_panels();
            return ContentResponse::consumed(true);
        }
        if !self.menu.is_open() {
            return ContentResponse::default();
        }
        let point = match event {
            ContentEvent::PointerDown(p)
            | ContentEvent::PointerUp(p)
            | ContentEvent::PointerMove(p) => Some(*p),
            ContentEvent::Scroll { position, .. } => Some(*position),
            _ => None,
        };
        let hit = point.and_then(|p| self.level_at(p));
        let captured = if matches!(event, ContentEvent::Scroll { begin: false, .. }) {
            let state = self.menu.content_state.as_ref().unwrap();
            state
                .panels
                .iter()
                .enumerate()
                .rev()
                .find_map(|(n, panel)| panel.view.scroll_gesture_captured().then_some(n + 1))
                .or_else(|| self.menu.scroll_gesture_captured().then_some(0))
        } else {
            None
        };
        let level = captured.unwrap_or_else(|| {
            if point.is_some() {
                hit.unwrap_or(0)
            } else {
                self.active_level()
            }
        });
        if matches!(event, ContentEvent::Scroll { begin: true, .. }) {
            // A new gesture transfers ownership through the original runtimes;
            // continuation packets stay with the runtime which captured them.
            if level != 0 {
                self.menu.cancel_scroll_gesture();
            }
            for (n, panel) in self
                .menu
                .content_state
                .as_mut()
                .unwrap()
                .panels
                .iter_mut()
                .enumerate()
            {
                if n + 1 != level {
                    panel.view.cancel_scroll_gesture();
                }
            }
        }
        if matches!(
            event,
            ContentEvent::Key {
                key: ControlKey::Left,
                down: true
            }
        ) && level > 0
        {
            self.retire_after(level - 1);
            self.panel_mut(level - 1).unwrap().set_keyboard_focus(true);
            return ContentResponse::consumed(true);
        }
        let (result, wheel) = deliver_menu_input(self.panel_mut(level).unwrap(), event);
        let keyboard = matches!(event, ContentEvent::Key { .. });
        let mut response = self.finish_declared_event(level, result, keyboard);
        if matches!(event, ContentEvent::Scroll { .. }) {
            if wheel {
                self.retire_after(level);
            }
            response.consumed = wheel;
            response.changed = wheel;
        }
        if hit.is_some()
            && matches!(
                event,
                ContentEvent::PointerMove(_) | ContentEvent::PointerDown(_)
            )
        {
            let index = self.panel_mut(level).unwrap().menu().items.selected;
            let path = self.panel_path(level);
            let submenu = self
                .menu
                .content_state
                .as_ref()
                .unwrap()
                .entries_at(&path)
                .and_then(|entries| entries.get(index))
                .is_some_and(|entry| {
                    matches!(
                        entry,
                        MenuEntry::Submenu {
                            disabled: false,
                            ..
                        }
                    )
                });
            if !submenu {
                self.retire_after(level);
            }
        }
        if matches!(
            event,
            ContentEvent::Key {
                key: ControlKey::Tab { .. },
                ..
            }
        ) {
            response.consumed = false;
        }
        response
    }
    fn paint_declared_panels(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let root_open = self.menu.is_open();
        let state = self.menu.content_state.as_mut().unwrap();
        for index in 0..state.retiring.len() {
            let panel = &state.retiring[index];
            let labels = state
                .entries_at(&panel.path)
                .unwrap_or_default()
                .iter()
                .map(|entry| entry.label().to_owned())
                .collect::<Vec<_>>();
            let anchor = panel
                .layout
                .as_ref()
                .map(|layout| layout.anchor)
                .unwrap_or_default();
            render_menu_panel(
                &mut state.retiring[index],
                &labels,
                anchor,
                scene,
                text,
                clip,
                theme,
            );
        }
        for level in 0..state.panels.len() {
            let parent_layout = if level == 0 {
                self.popup.clone()
            } else {
                state.panels[level - 1].layout.clone()
            };
            let parent_open = if level == 0 {
                root_open
            } else {
                state.panels[level - 1].view.is_open()
            };
            let parent_index = *state.panels[level].path.last().unwrap();
            let current = parent_layout
                .as_ref()
                .and_then(|p| p.items.get(parent_index))
                .filter(|row| row.visible.is_some())
                .map(|row| row.frame);
            if (!parent_open || current.is_none()) && state.panels[level].view.is_open() {
                state.panels[level].view.deactivate();
            }
            let anchor = current
                .or_else(|| {
                    state.panels[level]
                        .layout
                        .as_ref()
                        .map(|layout| layout.anchor)
                })
                .unwrap_or_default();
            let labels = state
                .entries_at(&state.panels[level].path)
                .unwrap_or_default()
                .iter()
                .map(|entry| entry.label().to_owned())
                .collect::<Vec<_>>();
            render_menu_panel(
                &mut state.panels[level],
                &labels,
                anchor,
                scene,
                text,
                clip,
                theme,
            );
        }
        state.retiring.retain(|panel| panel.view.is_presented());
    }
    fn declared_parts(&self) -> Vec<SemanticPart> {
        let Some(state) = self.menu.content_state.as_ref() else {
            return Vec::new();
        };
        if !self.overlay_open()
            || state.root_generation != self.menu.open_generation()
            || self.model_revision().is_none()
        {
            return Vec::new();
        }
        let level = self.active_level();
        let mut parts = Vec::new();
        let mut append = |entries: &[MenuEntry],
                          layout: &crate::MenuLayout,
                          ids: &[usize],
                          group: usize,
                          active: bool,
                          expanded: Option<usize>| {
            parts.push(SemanticPart {
                index: group,
                frame: layout.popup,
                focused: false,
                semantics: ContentSemantics::new(ContentRole::Menu, ""),
            });
            for (index, (entry, row)) in entries.iter().zip(&layout.items).enumerate() {
                if matches!(entry, MenuEntry::Separator) {
                    continue;
                }
                let style = entry.style();
                let mut semantics = ContentSemantics::new(ContentRole::MenuItem, entry.label());
                semantics.disabled = style.disabled;
                semantics.selected = style.checked.unwrap_or(false);
                semantics.value = style
                    .checked
                    .map(|checked| if checked { "on" } else { "off" }.into());
                semantics.expanded = style.submenu.then_some(expanded == Some(index));
                parts.push(SemanticPart {
                    index: ids[index],
                    frame: row.frame,
                    focused: active && layout.active_item == index && !style.disabled,
                    semantics,
                });
            }
        };
        if let (Some(layout), Some(group)) = (&self.popup, state.root_group) {
            append(
                state.entries.as_deref().unwrap(),
                layout,
                &state.root_parts,
                group,
                level == 0,
                state
                    .panels
                    .first()
                    .filter(|p| p.view.is_open())
                    .map(|p| p.path[0]),
            );
        }
        for (index, panel) in state
            .panels
            .iter()
            .enumerate()
            .filter(|(_, p)| p.view.is_open())
        {
            if let Some(layout) = &panel.layout {
                append(
                    state.entries_at(&panel.path).unwrap(),
                    layout,
                    &panel.parts,
                    panel.group,
                    level == index + 1,
                    state
                        .panels
                        .get(index + 1)
                        .filter(|p| p.view.is_open())
                        .map(|p| *p.path.last().unwrap()),
                );
            }
        }
        parts
    }
    fn declared_parent(&self, part: usize) -> Option<usize> {
        let state = self.menu.content_state.as_ref()?;
        if !self.menu.is_open() || state.root_generation != self.menu.open_generation() {
            return None;
        }
        if state.root_parts.contains(&part) {
            return state.root_group;
        }
        for panel in state.panels.iter().filter(|p| p.view.is_open()) {
            if panel.group == part {
                return Some(panel.parent);
            }
            if panel.parts.contains(&part) {
                return Some(panel.group);
            }
        }
        None
    }
}
fn render_menu_panel(
    panel: &mut MenuPanel,
    labels: &[String],
    anchor: Rect,
    scene: &mut Scene,
    text: &mut dyn ButtonTextRenderer,
    clip: Rect,
    theme: ContentTheme,
) {
    if let Some(context) = text.motion_context() {
        panel.view.set_reduced_motion(context.reduced_motion);
    }
    let labels = labels.iter().map(String::as_str).collect::<Vec<_>>();
    let layout = panel.view.render(
        scene,
        &mut TextAdapter(text),
        &labels,
        anchor,
        clip,
        Some(clip),
        theme.palette,
        proposal::TextRole::UiBody,
        false,
    );
    if layout.is_some() {
        panel.layout = layout;
    } else if !panel.view.is_presented() {
        panel.layout = None;
    }
    if panel.view.needs_animation() {
        text.request_animation_frame();
    }
}
fn deliver_menu_input(view: &mut MenuView, event: &ContentEvent) -> (MenuEvent, bool) {
    let mut wheel = false;
    let result = match event {
        ContentEvent::PointerMove(p) => {
            view.pointer_move_at(*p);
            MenuEvent::None
        }
        ContentEvent::PointerDown(p) => view.pointer_down_at(*p),
        ContentEvent::PointerUp(p) => view.pointer_up_at(*p),
        ContentEvent::Key { key, down } => view.key(*key, *down),
        ContentEvent::Scroll {
            position,
            delta,
            precise,
            begin,
            end,
            momentum,
            time,
        } => {
            wheel = matches!(
                view.wheel_event(MenuWheelInput {
                    point: *position,
                    delta: *delta,
                    precise: *precise,
                    begin: *begin,
                    end: *end,
                    momentum: *momentum,
                    time: *time
                }),
                crate::MenuWheelEvent::Consumed { .. }
            );
            MenuEvent::None
        }
        ContentEvent::Cancel => view.deactivate(),
        _ => MenuEvent::None,
    };
    (result, wheel)
}

impl Component for MenuContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.children().len() > 1 {
            return Err(CompositionError::InvalidSlotCount);
        }
        if !self.children().is_empty() && self.trigger.label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn children(&self) -> &[Content] {
        self.menu
            .content_state
            .as_ref()
            .map_or(&[], |state| state.trigger_content.as_slice())
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.menu
            .content_state
            .as_mut()
            .map(|state| &mut state.trigger_content)
    }
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        action_child_theme(&self.trigger, theme)
    }
    fn focus_decoration_above_siblings(&self) -> bool {
        self.trigger.focus_decoration_above_siblings()
    }
    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then(|| {
                self.menu
                    .content_state
                    .as_ref()
                    .and_then(|state| state.selection_event.clone())
                    .map(ContentActionPayload::MenuSelection)
            })
            .flatten()
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.menu.deactivate();
            self.menu.clear_presentation();
            self.popup = None;
            if let Some(state) = &mut self.menu.content_state {
                state.selection_event = None;
                state.last_frame = None;
                for panel in state.panels.iter_mut().chain(&mut state.retiring) {
                    panel.view.deactivate();
                    panel.view.clear_presentation();
                }
                state.panels.clear();
                state.retiring.clear();
                state.root_parts.clear();
                state.root_group = None;
            }
        }
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        self.declared_parent(part)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if self.declared() {
            return self.declared_parts();
        }
        if !self.overlay_open() || self.model_revision().is_none() {
            return Vec::new();
        }
        self.popup.as_ref().map_or_else(Vec::new, |p| {
            p.items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let label = self.labels.get(index)?;
                    let style = self.menu.item_style(index);
                    if style.is_some_and(|style| style.separator) {
                        return None;
                    }
                    Some({
                        let frame = item.frame;
                        let mut semantics = ContentSemantics::new(ContentRole::MenuItem, label);
                        semantics.disabled = style.is_some_and(|style| style.disabled);
                        semantics.selected = style.map_or(self.selected == Some(index), |style| {
                            style.checked.unwrap_or(false)
                        });
                        semantics.value = style
                            .and_then(|style| style.checked)
                            .map(|checked| if checked { "on" } else { "off" }.into());
                        SemanticPart {
                            index,
                            frame,
                            focused: p.active_item == index && !semantics.disabled,
                            semantics,
                        }
                    })
                })
                .collect()
        })
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if !self.reconcile_trigger() {
            return ContentResponse::default();
        }
        if self.declared() {
            return self.declared_ax(part, action);
        }
        if let Some(index) = part {
            if matches!(action, AccessibilityAction::Focus) {
                return if self.menu.focus_item(index) {
                    ContentResponse::consumed(true)
                } else {
                    ContentResponse::default()
                };
            }
            if !matches!(action, AccessibilityAction::Press) {
                return ContentResponse::default();
            }
            if let MenuEvent::Activated { index, .. } = self.menu.activate_item(index) {
                self.select(index);
                return ContentResponse {
                    consumed: true,
                    changed: true,
                    activated: true,
                    ..Default::default()
                };
            }
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::Press) {
            self.show_menu();
            return ContentResponse::consumed(true);
        }
        ContentResponse::default()
    }

    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        if let Some(state) = &mut self.menu.content_state
            && let Some(content) = state.trigger_content.first_mut()
        {
            return measure_action_slot(&self.trigger, content, &mut state.trigger_measured, t, a);
        }
        self.trigger.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.measure(t, frame.size);
        if let Some(state) = &mut self.menu.content_state
            && let Some(content) = state.trigger_content.first_mut()
        {
            let layout = resolve_action_slot(&self.trigger, state.trigger_measured, frame, clip, t);
            content.arrange(
                t,
                layout.content,
                layout.content.intersection(&clip).unwrap_or_default(),
            );
        }
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.reconcile_trigger();
        if let Some(state) = &self.menu.content_state
            && !state.trigger_content.is_empty()
        {
            self.trigger.button.render_content_frame(
                s,
                &mut TextAdapter(t),
                state.trigger_measured,
                f,
                Some(c),
                theme.palette,
                self.trigger.variant,
            );
        } else {
            self.trigger.paint(s, t, f, c, theme);
        }
    }
    fn paint_overlay(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.reconcile_trigger();
        if let Some(context) = t.motion_context() {
            // Same host-clock bridge as ScrollContent. The nested MenuViews
            // retain their own ScrollArea; no second scroll state is introduced.
            let state = self
                .menu
                .content_state
                .as_mut()
                .expect("reconciled declaration");
            let dt = state
                .last_frame
                .replace(context.now)
                .map(|previous| context.now.saturating_sub(previous).as_secs_f32());
            self.menu.set_reduced_motion(context.reduced_motion);
            if let Some(dt) = dt {
                self.menu.tick(dt);
            }
            let state = self.menu.content_state.as_mut().unwrap();
            for panel in state.panels.iter_mut().chain(&mut state.retiring) {
                panel.view.set_reduced_motion(context.reduced_motion);
                if let Some(dt) = dt {
                    panel.view.tick(dt);
                }
            }
        }
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.popup = self.menu.render(
            s,
            &mut TextAdapter(t),
            &labels,
            f,
            c,
            Some(c),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        if self.declared() {
            self.paint_declared_panels(s, t, c, theme);
        }
        if self.menu.needs_animation() {
            t.request_animation_frame();
        }
    }
    fn overlay_open(&self) -> bool {
        self.trigger.focusable() && self.menu.is_open()
    }
    fn overlay_presented(&self) -> bool {
        self.menu.is_open()
            || self.menu.is_presented()
            || self.menu.content_state.as_ref().is_some_and(|state| {
                state
                    .panels
                    .iter()
                    .chain(&state.retiring)
                    .any(|panel| panel.view.is_open() || panel.view.is_presented())
            })
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.handle_menu(e)
    }
    fn focusable(&self) -> bool {
        self.trigger.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.trigger.set_focus(f, v);
        if !f {
            self.menu.deactivate();
            self.close_panels();
        }
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let was_open = self.menu.is_open();
        if !self.reconcile_trigger() {
            return ContentResponse {
                changed: was_open,
                ..Default::default()
            };
        }
        if matches!(e, ContentEvent::Cancel) {
            self.menu.deactivate();
            self.close_panels();
        }
        let mut r = self.trigger.event(e);
        if r.activated {
            self.show_menu();
            // Opening the original trigger is not selecting a menu command.
            r.activated = false;
        }
        r
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = self.trigger.semantics();
        s.role = ContentRole::MenuButton;
        s.expanded = Some(self.overlay_open());
        s.value = if self.declared() {
            self.menu
                .content_state
                .as_ref()
                .and_then(|state| state.selection_event.as_ref())
                .map(|selection| selection.label().to_owned())
        } else {
            self.selected.and_then(|i| self.labels.get(i).cloned())
        };
        s
    }
}

impl std::fmt::Display for MenuEntriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyValue => "menu entry value must not be empty",
            Self::DuplicateValue => "menu sibling values must be unique",
            Self::EmptyLabel => "menu entry label must not be empty",
        })
    }
}
impl std::error::Error for MenuEntriesError {}
impl std::fmt::Display for MenuValuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Count => "menu values must match label count",
            Self::Empty => "menu value must not be empty",
            Self::Duplicate => "menu values must be unique",
        })
    }
}
impl std::error::Error for MenuValuesError {}
