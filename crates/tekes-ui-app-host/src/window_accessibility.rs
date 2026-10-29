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
        let mut keys = HashMap::new();
        for node in self.toolbar.host.semantics() {
            for (target, part) in std::iter::once((node.target, None)).chain(
                node.parts
                    .iter()
                    .map(|(target, part)| (*target, Some(part.index))),
            ) {
                let key = old
                    .iter()
                    .find(|(_, previous)| *previous == target)
                    .map(|(key, _)| *key)
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
                let Some(original) =
                    content_key(self.toolbar.host.content_host(), node.node.id, part)
                else {
                    return Vec::new();
                };
                keys.insert(original, key);
                mapped.push((key, target));
            }
        }
        let Some(snapshot) =
            ContentAccessibilitySnapshot::from_host(self.toolbar.host.content_host())
                .and_then(|snapshot| snapshot.with_native_keys(&keys))
        else {
            return Vec::new();
        };
        self.toolbar_ax = mapped;
        snapshot.nodes().to_vec()
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
        let Some(target) = self
            .toolbar_ax
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, target)| *target)
        else {
            return Ok(false);
        };
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
        let before = self.toolbar.host.content_host().focused();
        let response = self.toolbar.host.accessibility_action(target, action);
        self.drain_toolbar_commands()?;
        self.remember_toolbar_focus(
            (self.toolbar.host.content_host().focused() != before).then_some(true),
        );
        Ok(response.consumed)
    }
}
