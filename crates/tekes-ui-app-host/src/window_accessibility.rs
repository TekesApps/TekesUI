//! Window-native aliases retain the original shared Host target and geometry.
use super::Host;
use anyhow::Result;
use std::collections::HashMap;
use tekes_ui_controls::composition::AccessibilityAction as Action;
use tekes_ui_platform_macos::{
    AccessibilityAction, AccessibilityKey, AccessibilityNode,
    content_accessibility::{ContentAccessibilitySnapshot, content_key},
};
impl Host {
    pub(super) fn toolbar_accessibility_nodes(&mut self, active: bool) -> Vec<AccessibilityNode> {
        if !active {
            return Vec::new();
        }
        let old = std::mem::take(&mut self.toolbar_ax);
        let mut mapped = Vec::new();
        let mut nodes = Vec::new();
        let modal = self
            .panels
            .iter()
            .position(|panel| panel.mounted_toolbar.modal());
        for (index, panel) in self.panels.iter().enumerate() {
            if modal.is_some_and(|owner| owner != index)
                || panel.mounted_toolbar.host.semantics().is_empty()
            {
                continue;
            }
            let mut keys = HashMap::new();
            for node in panel.mounted_toolbar.host.semantics() {
                for (target, part) in std::iter::once((node.target, None)).chain(
                    node.parts
                        .iter()
                        .map(|(target, part)| (*target, Some(part.index))),
                ) {
                    let key = old
                        .iter()
                        .find(|(_, previous_index, previous)| {
                            *previous_index == index && *previous == target
                        })
                        .map(|(key, _, _)| *key)
                        .unwrap_or_else(|| {
                            let key = AccessibilityKey {
                                component: self.toolbar_ax_next,
                                part: None,
                            };
                            self.toolbar_ax_next = self
                                .toolbar_ax_next
                                .checked_add(1)
                                .expect("native toolbar identity exhausted");
                            key
                        });
                    let Some(original) = content_key(
                        panel.mounted_toolbar.host.content_host(),
                        node.node.id,
                        part,
                    ) else {
                        return Vec::new();
                    };
                    keys.insert(original, key);
                    mapped.push((key, index, target));
                }
            }
            let Some(snapshot) =
                ContentAccessibilitySnapshot::from_host(panel.mounted_toolbar.host.content_host())
                    .and_then(|snapshot| snapshot.with_native_keys(&keys))
            else {
                return Vec::new();
            };
            nodes.extend_from_slice(snapshot.nodes());
        }
        self.toolbar_ax = mapped;
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
        let Some((index, target)) = self
            .toolbar_ax
            .iter()
            .find(|(candidate, _, _)| *candidate == key)
            .map(|(_, index, target)| (*index, *target))
        else {
            return Ok(false);
        };
        self.select_toolbar_container(index);
        let action = match action {
            AccessibilityAction::Focus => Action::Focus,
            AccessibilityAction::Press => Action::Press,
            AccessibilityAction::ShowMenu => Action::ShowMenu,
            AccessibilityAction::Increment => Action::Increment,
            AccessibilityAction::Decrement => Action::Decrement,
            AccessibilityAction::SetValue(value) => Action::SetValue(value),
            AccessibilityAction::SetSelection(range) => Action::SetSelection(range),
            AccessibilityAction::ReplaceSelectedText(value) => Action::ReplaceSelectedText(value),
            AccessibilityAction::SetExpanded(_) => return Ok(false),
        };
        let before = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused();
        let response = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .accessibility_action(target, action);
        self.drain_toolbar_commands()?;
        if self.panels[index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_some()
        {
            self.blur_other_toolbars(index);
        }
        self.remember_toolbar_focus(
            (self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused()
                != before)
                .then_some(true),
        );
        Ok(response.consumed)
    }
}
