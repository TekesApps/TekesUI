//! Dynamic and menu-capable local container window toolbar declarations.
use anyhow::Result;
use std::collections::BTreeSet;
use tekes_ui_app_protocol::ToolbarItem;
use tekes_ui_controls::composition::{
    ContentActionPayload, ContentActionSource, MenuEntry, MenuSelection,
};

/// A button item declared by a [`crate::LocalContainer`] for the window or container toolbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalToolbarButton {
    pub id: u64,
    pub title: String,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub enabled: bool,
    pub selected: bool,
    pub geometry_dependent: bool,
}

impl LocalToolbarButton {
    /// Construct an enabled toolbar button with no icon, tooltip, or selection state.
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            icon: None,
            tooltip: None,
            enabled: true,
            selected: false,
            geometry_dependent: false,
        }
    }

    /// Attach a canonical `tekes-ui-icons` template glyph name to this button.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Attach a hover/accessibility help tooltip for this button.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set whether the button can be activated.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set whether the button is rendered in its selected/checked state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Mark whether activation depends on the container having laid out the latest viewport.
    pub fn geometry_dependent(mut self, geometry_dependent: bool) -> Self {
        self.geometry_dependent = geometry_dependent;
        self
    }

    pub(crate) fn to_protocol_item(&self) -> ToolbarItem {
        ToolbarItem {
            id: self.id,
            title: self.title.clone(),
            enabled: self.enabled,
            geometry_dependent: self.geometry_dependent,
        }
    }
}

impl From<ToolbarItem> for LocalToolbarButton {
    fn from(item: ToolbarItem) -> Self {
        Self {
            id: item.id,
            title: item.title,
            icon: None,
            tooltip: None,
            enabled: item.enabled,
            selected: false,
            geometry_dependent: item.geometry_dependent,
        }
    }
}

/// A native dropdown menu item declared by a [`crate::LocalContainer`] backed by `MenuContent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalToolbarMenu {
    pub id: u64,
    pub title: String,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub enabled: bool,
    pub selected: bool,
    pub dropdown_indicator: bool,
    pub entries: Vec<MenuEntry>,
}

impl LocalToolbarMenu {
    /// Construct an enabled dropdown menu trigger with a trailing disclosure indicator.
    pub fn new(id: u64, title: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        Self {
            id,
            title: title.into(),
            icon: None,
            tooltip: None,
            enabled: true,
            selected: false,
            dropdown_indicator: true,
            entries,
        }
    }

    /// Attach a canonical `tekes-ui-icons` template glyph name to the trigger button.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Attach a hover/accessibility help tooltip for the trigger button.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set whether the menu trigger button is interactive.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set whether the menu trigger button shows its selected chrome.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Show or hide the trailing chevron indicator when no explicit icon is set.
    /// Useful for locked-identity menus where the trigger remains readable without a chevron.
    pub fn dropdown_indicator(mut self, dropdown_indicator: bool) -> Self {
        self.dropdown_indicator = dropdown_indicator;
        self
    }

    /// Replace the menu's declared `MenuEntry` tree.
    pub fn entries(mut self, entries: Vec<MenuEntry>) -> Self {
        self.entries = entries;
        self
    }
}

/// A button or dropdown menu item contributed by a [`crate::LocalContainer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalToolbarItem {
    Button(LocalToolbarButton),
    Menu(LocalToolbarMenu),
}

impl LocalToolbarItem {
    /// Shorthand for constructing a [`LocalToolbarItem::Button`].
    pub fn button(id: u64, title: impl Into<String>) -> Self {
        Self::Button(LocalToolbarButton::new(id, title))
    }

    /// Shorthand for constructing a [`LocalToolbarItem::Menu`].
    pub fn menu(id: u64, title: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        Self::Menu(LocalToolbarMenu::new(id, title, entries))
    }

    /// Return the stable item identifier within the owning container.
    pub fn id(&self) -> u64 {
        match self {
            Self::Button(button) => button.id,
            Self::Menu(menu) => menu.id,
        }
    }

    /// Return the trigger title.
    pub fn title(&self) -> &str {
        match self {
            Self::Button(button) => &button.title,
            Self::Menu(menu) => &menu.title,
        }
    }

    /// Return whether this item is enabled.
    pub fn enabled(&self) -> bool {
        match self {
            Self::Button(button) => button.enabled,
            Self::Menu(menu) => menu.enabled,
        }
    }
}

impl From<ToolbarItem> for LocalToolbarItem {
    fn from(item: ToolbarItem) -> Self {
        Self::Button(item.into())
    }
}

impl From<LocalToolbarButton> for LocalToolbarItem {
    fn from(button: LocalToolbarButton) -> Self {
        Self::Button(button)
    }
}

impl From<LocalToolbarMenu> for LocalToolbarItem {
    fn from(menu: LocalToolbarMenu) -> Self {
        Self::Menu(menu)
    }
}

/// Owned toolbar activation record delivered back to the originating [`crate::LocalContainer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalToolbarAction {
    pub id: u64,
    pub source: ContentActionSource,
    pub payload: Option<ContentActionPayload>,
}

impl LocalToolbarAction {
    /// Return the [`MenuSelection`] payload if this action came from a toolbar menu.
    pub fn menu_selection(&self) -> Option<&MenuSelection> {
        match self.payload.as_ref()? {
            ContentActionPayload::MenuSelection(selection) => Some(selection),
            _ => None,
        }
    }

    /// Return the full selected value path (including any parent submenu values).
    pub fn menu_value_path(&self) -> Option<&[String]> {
        self.menu_selection()?.value_path()
    }

    /// Return the leaf selected value for a menu selection.
    pub fn menu_value(&self) -> Option<&str> {
        self.menu_value_path()?.last().map(String::as_str)
    }
}

pub(crate) fn validate_unique_ids(container_id: &str, items: &[LocalToolbarItem]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for item in items {
        let id = item.id();
        anyhow::ensure!(
            ids.insert(id),
            "duplicate toolbar item id {id} for container {container_id}"
        );
    }
    Ok(())
}

pub(crate) fn entry_enabled(entries: &[MenuEntry], value_path: &[String]) -> bool {
    let Some((head, tail)) = value_path.split_first() else {
        return false;
    };
    for entry in entries {
        match entry {
            MenuEntry::Item {
                value, disabled, ..
            } if tail.is_empty() && value == head => {
                return !*disabled;
            }
            MenuEntry::Submenu {
                value,
                disabled,
                entries: children,
                ..
            } if !tail.is_empty() && value == head => {
                return !*disabled && entry_enabled(children, tail);
            }
            _ => {}
        }
    }
    false
}
