//! Dynamic and menu-capable local container window toolbar declarations.
use anyhow::Result;
use std::collections::BTreeSet;
use tekes_ui_app_protocol::ToolbarItem;
use tekes_ui_controls::composition::{
    ButtonTone, ContentActionPayload, ContentActionSource, MenuEntry, MenuSelection,
};

/// Semantic foreground tone for a local toolbar button or menu trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalToolbarTone {
    #[default]
    Default,
    Secondary,
    Warning,
    Error,
}

impl From<LocalToolbarTone> for ButtonTone {
    fn from(tone: LocalToolbarTone) -> Self {
        match tone {
            LocalToolbarTone::Default => Self::Default,
            LocalToolbarTone::Secondary => Self::Secondary,
            LocalToolbarTone::Warning => Self::Warning,
            LocalToolbarTone::Error => Self::Error,
        }
    }
}

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
    pub tone: LocalToolbarTone,
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
            tone: LocalToolbarTone::Default,
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

    /// Set an optional semantic foreground tone for this button.
    pub fn tone(mut self, tone: LocalToolbarTone) -> Self {
        self.tone = tone;
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
            tone: LocalToolbarTone::Default,
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
    pub tone: LocalToolbarTone,
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
            tone: LocalToolbarTone::Default,
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

    /// Set an optional semantic foreground tone for the trigger button.
    pub fn tone(mut self, tone: LocalToolbarTone) -> Self {
        self.tone = tone;
        self
    }
}

/// Narrow-width overflow policy for a [`LocalToolbarGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalToolbarGroupOverflow {
    /// Fold the entire group atomically into the column overflow menu when it does not fit.
    #[default]
    Together,
    /// Keep a fitting prefix in the toolbar and move the remaining segments to overflow.
    Split,
}

/// Integrated capsule segmented group of related [`LocalToolbarItem`] buttons and menus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalToolbarGroup {
    pub id: u64,
    pub items: Vec<LocalToolbarItem>,
    pub divided: bool,
    pub label: Option<String>,
    pub tooltip: Option<String>,
    pub overflow: LocalToolbarGroupOverflow,
}

impl LocalToolbarGroup {
    /// Construct a divided capsule toolbar group whose members overflow together by default.
    pub fn new(id: u64, items: Vec<LocalToolbarItem>) -> Self {
        Self {
            id,
            items,
            divided: true,
            label: None,
            tooltip: None,
            overflow: LocalToolbarGroupOverflow::Together,
        }
    }

    /// Control whether 1px dividers are painted between adjacent segments (`true` by default).
    pub fn divided(mut self, divided: bool) -> Self {
        self.divided = divided;
        self
    }

    /// Set the accessible group label and overflow submenu title.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        self.label = (!label.trim().is_empty()).then_some(label);
        self
    }

    /// Set an optional descriptive tooltip for the group.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        let tooltip = tooltip.into();
        self.tooltip = (!tooltip.trim().is_empty()).then_some(tooltip);
        self
    }

    /// Configure how the group participates in narrow-toolbar overflow.
    pub fn overflow(mut self, overflow: LocalToolbarGroupOverflow) -> Self {
        self.overflow = overflow;
        self
    }
}

/// A button, dropdown menu, or grouped capsule contributed by a [`crate::LocalContainer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalToolbarItem {
    Button(LocalToolbarButton),
    Menu(LocalToolbarMenu),
    Group(LocalToolbarGroup),
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

    /// Shorthand for constructing a [`LocalToolbarItem::Group`].
    pub fn group(id: u64, items: Vec<LocalToolbarItem>) -> Self {
        Self::Group(LocalToolbarGroup::new(id, items))
    }

    /// Return the stable item identifier within the owning container.
    pub fn id(&self) -> u64 {
        match self {
            Self::Button(button) => button.id,
            Self::Menu(menu) => menu.id,
            Self::Group(group) => group.id,
        }
    }

    /// Return the trigger or group title.
    pub fn title(&self) -> &str {
        match self {
            Self::Button(button) => &button.title,
            Self::Menu(menu) => &menu.title,
            Self::Group(group) => group
                .label
                .as_deref()
                .or(group.tooltip.as_deref())
                .unwrap_or(""),
        }
    }

    /// Return whether this item is enabled.
    pub fn enabled(&self) -> bool {
        match self {
            Self::Button(button) => button.enabled,
            Self::Menu(menu) => menu.enabled,
            Self::Group(group) => group.items.iter().any(Self::enabled),
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

impl From<LocalToolbarGroup> for LocalToolbarItem {
    fn from(group: LocalToolbarGroup) -> Self {
        Self::Group(group)
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
            id != 0,
            "invalid toolbar item id 0 for container {container_id}"
        );
        anyhow::ensure!(
            ids.insert(id),
            "duplicate toolbar item id {id} for container {container_id}"
        );
        if let LocalToolbarItem::Group(group) = item {
            for member in &group.items {
                anyhow::ensure!(
                    !matches!(member, LocalToolbarItem::Group(_)),
                    "nested toolbar group {} inside group {id} is not allowed for container {container_id}",
                    member.id()
                );
                let member_id = member.id();
                anyhow::ensure!(
                    member_id != 0,
                    "invalid toolbar item id 0 in group {id} for container {container_id}"
                );
                anyhow::ensure!(
                    ids.insert(member_id),
                    "duplicate toolbar item id {member_id} for container {container_id}"
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn find_member(items: &[LocalToolbarItem], id: u64) -> Option<&LocalToolbarItem> {
    for item in items {
        match item {
            LocalToolbarItem::Button(b) if b.id == id => return Some(item),
            LocalToolbarItem::Menu(m) if m.id == id => return Some(item),
            LocalToolbarItem::Group(group) => {
                for member in &group.items {
                    match member {
                        LocalToolbarItem::Button(b) if b.id == id => return Some(member),
                        LocalToolbarItem::Menu(m) if m.id == id => return Some(member),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn collect_protocol_buttons(items: &[LocalToolbarItem]) -> Vec<ToolbarItem> {
    let mut out = Vec::new();
    for item in items {
        match item {
            LocalToolbarItem::Button(button) => out.push(button.to_protocol_item()),
            LocalToolbarItem::Menu(_) => {}
            LocalToolbarItem::Group(group) => {
                for member in &group.items {
                    if let LocalToolbarItem::Button(button) = member {
                        out.push(button.to_protocol_item());
                    }
                }
            }
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_defaults_builders_and_tones() {
        let group = LocalToolbarGroup::new(
            10,
            vec![
                LocalToolbarItem::menu(101, "", vec![MenuEntry::item("a", "A")]),
                LocalToolbarItem::button(102, "Go"),
            ],
        );
        assert!(group.divided);
        assert_eq!(group.overflow, LocalToolbarGroupOverflow::Together);
        assert_eq!(group.label, None);
        assert_eq!(group.tooltip, None);

        let configured = group
            .divided(false)
            .label("  ")
            .label("Controls")
            .tooltip("Session controls")
            .overflow(LocalToolbarGroupOverflow::Split);
        assert!(!configured.divided);
        assert_eq!(configured.label.as_deref(), Some("Controls"));
        assert_eq!(configured.tooltip.as_deref(), Some("Session controls"));
        assert_eq!(configured.overflow, LocalToolbarGroupOverflow::Split);

        let item = LocalToolbarItem::from(configured);
        assert_eq!(item.id(), 10);
        assert_eq!(item.title(), "Controls");
        assert!(item.enabled());

        assert_eq!(
            ButtonTone::from(LocalToolbarTone::Default),
            ButtonTone::Default
        );
        assert_eq!(
            ButtonTone::from(LocalToolbarTone::Secondary),
            ButtonTone::Secondary
        );
        assert_eq!(
            ButtonTone::from(LocalToolbarTone::Warning),
            ButtonTone::Warning
        );
        assert_eq!(ButtonTone::from(LocalToolbarTone::Error), ButtonTone::Error);
    }

    #[test]
    fn validate_rejects_zero_duplicate_and_nested_group_ids() {
        assert!(validate_unique_ids("c", &[LocalToolbarItem::button(0, "Zero")]).is_err());
        assert!(
            validate_unique_ids(
                "c",
                &[LocalToolbarItem::group(
                    0,
                    vec![LocalToolbarItem::button(1, "Ok")]
                )]
            )
            .is_err()
        );
        assert!(
            validate_unique_ids(
                "c",
                &[LocalToolbarItem::group(
                    10,
                    vec![LocalToolbarItem::button(0, "Zero")]
                )]
            )
            .is_err()
        );
        // Top-level collides with group ID.
        assert!(
            validate_unique_ids(
                "c",
                &[
                    LocalToolbarItem::button(10, "Top"),
                    LocalToolbarItem::group(10, vec![LocalToolbarItem::button(11, "Member")])
                ]
            )
            .is_err()
        );
        // Top-level collides with member ID.
        assert!(
            validate_unique_ids(
                "c",
                &[
                    LocalToolbarItem::button(11, "Top"),
                    LocalToolbarItem::group(10, vec![LocalToolbarItem::button(11, "Member")])
                ]
            )
            .is_err()
        );
        // Group ID collides with its own member.
        assert!(
            validate_unique_ids(
                "c",
                &[LocalToolbarItem::group(
                    10,
                    vec![LocalToolbarItem::button(10, "Self")]
                )]
            )
            .is_err()
        );
        // Member ID collides across two groups.
        assert!(
            validate_unique_ids(
                "c",
                &[
                    LocalToolbarItem::group(10, vec![LocalToolbarItem::button(99, "A")]),
                    LocalToolbarItem::group(20, vec![LocalToolbarItem::button(99, "B")]),
                ]
            )
            .is_err()
        );
        // Nested group rejected.
        assert!(
            validate_unique_ids(
                "c",
                &[LocalToolbarItem::group(
                    10,
                    vec![LocalToolbarItem::group(
                        20,
                        vec![LocalToolbarItem::button(21, "Nested")]
                    )]
                )]
            )
            .is_err()
        );
    }

    #[test]
    fn find_member_and_collect_protocol_buttons_cover_group_members() {
        let items = vec![
            LocalToolbarItem::button(1, "Top"),
            LocalToolbarItem::group(
                10,
                vec![
                    LocalToolbarItem::Menu(LocalToolbarMenu::new(
                        101,
                        "Model",
                        vec![MenuEntry::item("m1", "M1")],
                    )),
                    LocalToolbarItem::button(102, "Apply"),
                ],
            ),
        ];
        assert!(validate_unique_ids("c", &items).is_ok());
        assert_eq!(find_member(&items, 1).unwrap().id(), 1);
        assert_eq!(find_member(&items, 101).unwrap().id(), 101);
        assert_eq!(find_member(&items, 102).unwrap().id(), 102);
        assert!(find_member(&items, 10).is_none());

        let proto = collect_protocol_buttons(&items);
        let ids: Vec<u64> = proto.into_iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![1, 102]);
    }
}
