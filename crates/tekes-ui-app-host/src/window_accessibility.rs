//! Projection and dispatch for the original Host-owned toolbar mounts only.
use super::{Host, ToolbarTarget};
use crate::accessibility::{content_action, content_nodes};
use anyhow::Result;
use tekes_ui_platform_macos::{AccessibilityAction, AccessibilityKey, AccessibilityNode};

impl Host {
    pub(super) fn toolbar_accessibility_nodes(&self, active: bool) -> Vec<AccessibilityNode> {
        let overlay = self.open_toolbar();
        let mut nodes: Vec<_> = self
            .toolbar_groups()
            .into_iter()
            .filter(|&group| {
                self.toolbar_available(group) && overlay.is_none_or(|owner| owner == group)
            })
            .filter_map(|group| self.toolbar_host(group))
            .flat_map(|host| content_nodes(host, active))
            .map(|mut node| {
                node.enabled &= active;
                node
            })
            .collect();
        if overlay.is_none()
            && let Some(nav) = &self.navigation
            && nav
                .title_toolbar
                .get(tekes_ui_controls::composition::ComponentId(2))
                .is_some_and(|title| {
                    title.frame().size.width > 0. && title.frame().size.height > 0.
                })
        {
            nodes.extend(content_nodes(&nav.title_toolbar, active));
        }
        nodes
    }

    pub(super) fn toolbar_accessibility_action(
        &mut self,
        active: bool,
        key: AccessibilityKey,
        action: AccessibilityAction,
    ) -> Result<bool> {
        if !active {
            return Ok(false);
        }
        let overlay = self.open_toolbar();
        let owner = self.toolbar_groups().into_iter().find(|&group| {
            self.toolbar_available(group)
                && overlay.is_none_or(|owner| owner == group)
                && self.toolbar_host(group).is_some_and(|host| {
                    content_nodes(host, true)
                        .iter()
                        .any(|node| node.key == key && node.enabled)
                })
        });
        let Some(owner) = owner else {
            return Ok(false);
        };
        let focus = matches!(action, AccessibilityAction::Focus);
        let response = content_action(self.toolbar_host_mut(owner).unwrap(), true, key, action);
        if !response.consumed {
            return Ok(false);
        }
        if focus {
            self.set_toolbar_focus(Some(owner), true);
        }
        // Drain the action created by the original component exactly once.
        // No synthetic pointer/key event and no second accessibility activation.
        match owner {
            ToolbarTarget::Panel(i) => self.panels[i].drain_toolbar_commands()?,
            ToolbarTarget::Navigation => self.navigation.as_mut().unwrap().drain_action()?,
            ToolbarTarget::Editing => self.navigation.as_mut().unwrap().drain_external_action(),
            ToolbarTarget::SettingsBack => self.drain_settings_back(),
            ToolbarTarget::Visibility(i) => {
                if self.visibility_bars[i].take_action().is_some() {
                    self.toggle_visibility(i)?;
                }
            }
        }
        Ok(true)
    }
}
