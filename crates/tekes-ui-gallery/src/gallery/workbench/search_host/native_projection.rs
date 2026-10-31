//! Native identifiers only; editor, actions and semantics remain in ContentHost.
use super::SearchHost;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_controls::composition::ContentResponse;
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
pub(super) struct NativeProjection {
    keys: HashMap<K, K>,
}
impl SearchHost {
    fn projected_nodes(&self) -> Vec<N> {
        let mut nodes = content_nodes(&self.host);
        let current: HashSet<_> = nodes.iter().map(|n| n.key).collect();
        let mut projection = self.projection.borrow_mut();
        projection.keys.retain(|key, _| current.contains(key));
        for node in &nodes {
            if projection.keys.contains_key(&node.key) {
                continue;
            }
            let Ok(component) =
                NEXT_NATIVE
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            else {
                projection.keys.clear();
                return Vec::new();
            };
            projection.keys.insert(
                node.key,
                K {
                    component,
                    part: node.key.part,
                },
            );
        }
        for node in &mut nodes {
            let key = projection.keys[&node.key];
            node.parent = node
                .parent
                .and_then(|parent| projection.keys.get(&parent).copied());
            node.key = key;
        }
        nodes
    }
    pub(in crate::gallery) fn native_nodes(&self) -> Vec<N> {
        self.projected_nodes()
    }
    pub(in crate::gallery) fn revoke_projection(&self) {
        self.projection.borrow_mut().keys.clear();
    }
    pub(in crate::gallery) fn owns_native_key(&self, key: K) -> bool {
        self.projected_nodes().iter().any(|node| node.key == key)
    }
    pub(in crate::gallery) fn native_layouts(&self) -> Vec<(K, AccessibilityTextLayout)> {
        self.projected_nodes();
        let projection = self.projection.borrow();
        content_text_layouts(&self.host)
            .into_iter()
            .filter_map(|(key, layout)| projection.keys.get(&key).copied().map(|key| (key, layout)))
            .collect()
    }
    pub(in crate::gallery) fn native_action(
        &mut self,
        key: K,
        action: AccessibilityAction,
    ) -> ContentResponse {
        self.projected_nodes();
        let original = self
            .projection
            .borrow()
            .keys
            .iter()
            .find_map(|(original, projected)| (*projected == key).then_some(*original));
        original
            .map(|key| content_action(&mut self.host, key, action))
            .unwrap_or_default()
    }
}
