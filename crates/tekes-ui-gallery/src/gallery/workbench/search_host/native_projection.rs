//! Native identifiers only; editor, actions and semantics remain in ContentHost.
use super::StoryTextHost;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_controls::composition::{ContentHost, ContentResponse};
use tekes_ui_platform_macos::content_accessibility::{
    content_action, content_nodes, content_text_layouts,
};
use tekes_ui_platform_macos::{
    AccessibilityAction, AccessibilityKey as K, AccessibilityNode as N, AccessibilityTextLayout,
};

// Gallery's fixed NAV/SEARCH/STORY/INPUT/DETAIL keys occupy the low domain.
// All Content keys, including arbitrarily large original mounts, are translated.
static NEXT_NATIVE: AtomicU64 = AtomicU64::new(1_u64 << 63);
#[derive(Debug, Default)]
pub(in crate::gallery::workbench) struct NativeProjection {
    keys: HashMap<K, K>,
}
impl NativeProjection {
    pub(in crate::gallery::workbench) fn nodes(&mut self, host: &ContentHost) -> Vec<N> {
        let mut nodes = content_nodes(host);
        let current: HashSet<_> = nodes.iter().map(|n| n.key).collect();
        self.keys.retain(|key, _| current.contains(key));
        for node in &nodes {
            if self.keys.contains_key(&node.key) {
                continue;
            }
            let Ok(component) =
                NEXT_NATIVE
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            else {
                self.keys.clear();
                return Vec::new();
            };
            self.keys.insert(
                node.key,
                K {
                    component,
                    part: node.key.part,
                },
            );
        }
        for node in &mut nodes {
            let key = self.keys[&node.key];
            node.parent = node
                .parent
                .and_then(|parent| self.keys.get(&parent).copied());
            node.key = key;
        }
        nodes
    }
    pub(in crate::gallery::workbench) fn revoke(&mut self) {
        self.keys.clear();
    }
    pub(in crate::gallery::workbench) fn owns_key(&mut self, host: &ContentHost, key: K) -> bool {
        self.nodes(host).iter().any(|node| node.key == key)
    }
    pub(in crate::gallery::workbench) fn layouts(
        &mut self,
        host: &ContentHost,
    ) -> Vec<(K, AccessibilityTextLayout)> {
        self.nodes(host);
        content_text_layouts(host)
            .into_iter()
            .filter_map(|(key, layout)| self.keys.get(&key).copied().map(|key| (key, layout)))
            .collect()
    }
    pub(in crate::gallery::workbench) fn action(
        &mut self,
        host: &mut ContentHost,
        key: K,
        action: AccessibilityAction,
    ) -> ContentResponse {
        self.nodes(host);
        let original = self
            .keys
            .iter()
            .find_map(|(original, projected)| (*projected == key).then_some(*original));
        original
            .map(|key| content_action(host, key, action))
            .unwrap_or_default()
    }
}
impl StoryTextHost {
    pub(in crate::gallery) fn native_nodes(&self) -> Vec<N> {
        self.projection.borrow_mut().nodes(&self.host)
    }
    pub(in crate::gallery) fn revoke_projection(&self) {
        self.projection.borrow_mut().revoke();
    }
    pub(in crate::gallery) fn owns_native_key(&self, key: K) -> bool {
        self.projection.borrow_mut().owns_key(&self.host, key)
    }
    pub(in crate::gallery) fn native_layouts(&self) -> Vec<(K, AccessibilityTextLayout)> {
        self.projection.borrow_mut().layouts(&self.host)
    }
    pub(in crate::gallery) fn native_action(
        &mut self,
        key: K,
        action: AccessibilityAction,
    ) -> ContentResponse {
        self.projection
            .borrow_mut()
            .action(&mut self.host, key, action)
    }
}
