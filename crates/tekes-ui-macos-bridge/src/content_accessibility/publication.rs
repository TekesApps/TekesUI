//! One window publication keeps each original Host lease and paint order intact.
use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Window adapter owner identity, used for direct dispatch rather than trial routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContentAccessibilityOwner(pub u64);

#[derive(Clone, Debug)]
pub struct WindowAccessibilityPublication {
    fragments: Vec<(ContentAccessibilityOwner, ContentAccessibilitySnapshot)>,
    owners: HashMap<AccessibilityKey, usize>,
    pub(crate) nodes: Vec<AccessibilityNode>,
    pub(crate) text_layouts: HashMap<AccessibilityKey, AccessibilityTextLayout>,
    pub(crate) structural_only: HashSet<AccessibilityKey>,
    authority: Arc<AtomicBool>,
}
impl WindowAccessibilityPublication {
    /// Fragments are in original paint order, back to front. No independent
    /// generations are merged, changed, or re-signed. All validation precedes commit.
    pub fn prepare(
        fragments: Vec<(ContentAccessibilityOwner, ContentAccessibilitySnapshot)>,
    ) -> Option<Self> {
        if fragments.len() > 4096 {
            return None;
        }
        let mut result = Self {
            fragments,
            owners: HashMap::new(),
            nodes: Vec::new(),
            text_layouts: HashMap::new(),
            structural_only: HashSet::new(),
            authority: Arc::new(AtomicBool::new(true)),
        };
        let mut owners = HashSet::new();
        let mut bytes = 0usize;
        for (index, (owner, snapshot)) in result.fragments.iter().enumerate() {
            if !owners.insert(*owner) || !snapshot.is_valid() {
                return None;
            }
            let mut fragment_nodes = HashMap::with_capacity(snapshot.nodes.len());
            for (node_idx, node) in snapshot.nodes.iter().enumerate() {
                if result.nodes.len() >= 100_000 || result.owners.insert(node.key, index).is_some()
                {
                    return None;
                }
                fragment_nodes.insert(node.key, node_idx);
                bytes = bytes
                    .checked_add(node.label.len())?
                    .checked_add(node.value.as_ref().map_or(0, String::len))?
                    .checked_add(node.description.as_ref().map_or(0, String::len))?;
                if bytes > 16 * 1024 * 1024 {
                    return None;
                }
                result.nodes.push(node.clone());
            }
            for (key, layout) in &snapshot.text_layouts {
                let &node_idx = fragment_nodes.get(key)?;
                let node = &snapshot.nodes[node_idx];
                if node.value.as_deref() != Some(layout.state.text.as_str())
                    || !layout.is_valid()
                    || result.text_layouts.insert(*key, layout.clone()).is_some()
                {
                    return None;
                }
                bytes = bytes
                    .checked_add(layout.state.text.len())?
                    .checked_add(layout.state.carets.len().checked_mul(std::mem::size_of::<(
                        usize,
                        Point,
                        Size,
                    )>(
                    ))?)?
                    .checked_add(
                        layout
                            .lines
                            .len()
                            .checked_mul(std::mem::size_of::<std::ops::Range<usize>>())?,
                    )?;
                if bytes > 16 * 1024 * 1024 {
                    return None;
                }
            }
            result
                .structural_only
                .extend(snapshot.structural_only.iter().copied());
        }
        result.is_valid().then_some(result)
    }
    /// The Window setter uses this same commit point. Preparation failure or one
    /// stale source revokes the old batch and leaves an explicit denied authority.
    pub fn commit(slot: &mut Option<Self>, next: Option<Self>) -> bool {
        if let Some(old) = slot.as_ref() {
            old.revoke();
        }
        let accepted = next.as_ref().is_some_and(Self::is_valid);
        *slot = Some(if accepted {
            next.unwrap()
        } else {
            if let Some(p) = next {
                p.revoke();
            }
            Self::revoked()
        });
        accepted
    }
    pub fn nodes(&self) -> &[AccessibilityNode] {
        if self.is_current() { &self.nodes } else { &[] }
    }
    pub fn text_layouts(&self) -> Option<&HashMap<AccessibilityKey, AccessibilityTextLayout>> {
        self.is_current().then_some(&self.text_layouts)
    }
    pub fn is_current(&self) -> bool {
        self.authority.load(Ordering::Acquire) && self.fragments.iter().all(|(_, s)| s.is_current())
    }
    pub fn is_valid(&self) -> bool {
        self.is_current()
            && self.fragments.iter().all(|(_, s)| s.is_valid())
            && tekes_ui_platform_macos::validate_accessibility_nodes_with_geometry(
                &self.nodes,
                &self.owners.keys().copied().collect(),
            )
    }
    pub fn owner(&self, key: AccessibilityKey) -> Option<ContentAccessibilityOwner> {
        if !self.is_current() {
            return None;
        }
        Some(self.fragments.get(*self.owners.get(&key)?)?.0)
    }
    pub fn hit_test(&self, point: Point) -> Option<AccessibilityKey> {
        if !self.is_current() {
            return None;
        }
        self.fragments
            .iter()
            .rev()
            .find_map(|(_, s)| s.hit_test(point))
    }
    pub fn contains(&self, key: AccessibilityKey, point: Point) -> bool {
        self.hit_test(point) == Some(key)
    }
    pub fn frame_for_range(
        &self,
        key: AccessibilityKey,
        range: std::ops::Range<usize>,
    ) -> Option<Rect> {
        self.owner(key)?;
        self.fragments[*self.owners.get(&key)?]
            .1
            .frame_for_range(key, range)
    }
    /// Dispatch exactly once to the designated original Host. Native aliases never
    /// become original mount IDs; the source geometry provides the original identity.
    pub fn dispatch(
        &self,
        owner: ContentAccessibilityOwner,
        host: &mut ContentHost,
        key: AccessibilityKey,
        action: AccessibilityAction,
    ) -> ContentResponse {
        if self.owner(key) != Some(owner) {
            return ContentResponse::default();
        }
        let snapshot = &self.fragments[self.owners[&key]].1;
        let Some(&(id, part)) = snapshot.identities.get(&key) else {
            return ContentResponse::default();
        };
        let Some(mount) = snapshot.geometry.identity(id, part) else {
            return ContentResponse::default();
        };
        if host.semantic_mount(id) != Some(mount) {
            return ContentResponse::default();
        }
        content_action_raw(
            host,
            AccessibilityKey {
                component: mount,
                part,
            },
            action,
        )
    }
    /// Revoke every clone before replacing or rejecting a batch. Retained native
    /// objects and saved publication handles can never recover their old authority.
    pub fn revoke(&self) {
        self.authority.store(false, Ordering::Release);
    }
    pub(crate) fn revoked() -> Self {
        let result = Self::prepare(Vec::new()).expect("empty publication");
        result.revoke();
        result
    }
}

impl tekes_ui_platform_macos::AccessibilityGeometryLease for WindowAccessibilityPublication {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn is_current(&self) -> bool {
        WindowAccessibilityPublication::is_current(self)
    }
    fn is_valid(&self) -> bool {
        WindowAccessibilityPublication::is_valid(self)
    }
    fn revoke(&self) {
        WindowAccessibilityPublication::revoke(self);
    }
    fn nodes(&self) -> &[AccessibilityNode] {
        WindowAccessibilityPublication::nodes(self)
    }
    fn text_layouts(&self) -> Option<&HashMap<AccessibilityKey, AccessibilityTextLayout>> {
        WindowAccessibilityPublication::text_layouts(self)
    }
    fn is_structural_only(&self, key: AccessibilityKey) -> bool {
        self.is_current() && self.structural_only.contains(&key)
    }
    fn contains(&self, key: AccessibilityKey, point: Point) -> bool {
        WindowAccessibilityPublication::contains(self, key, point)
    }
    fn hit_test(&self, point: Point) -> Option<AccessibilityKey> {
        WindowAccessibilityPublication::hit_test(self, point)
    }
    fn frame_for_range(
        &self,
        key: AccessibilityKey,
        range: std::ops::Range<usize>,
    ) -> Option<Rect> {
        WindowAccessibilityPublication::frame_for_range(self, key, range)
    }
    fn authorizes_target(&self, target: AccessibilityKey) -> bool {
        self.owner(target).is_some()
    }
}

/// Extension methods binding [`WindowAccessibilityPublication`] and
/// [`ContentAccessibilitySnapshot`] to [`tekes_ui_platform_macos::Window`].
#[cfg(target_os = "macos")]
pub trait WindowContentAccessibilityExt {
    fn set_content_accessibility_snapshot(&self, snapshot: ContentAccessibilitySnapshot) -> bool;
    fn set_content_accessibility_publication(
        &self,
        publication: Option<WindowAccessibilityPublication>,
    ) -> bool;
    fn content_accessibility_publication(&self) -> Option<WindowAccessibilityPublication>;
}

#[cfg(target_os = "macos")]
impl WindowContentAccessibilityExt for tekes_ui_platform_macos::Window {
    fn set_content_accessibility_snapshot(&self, snapshot: ContentAccessibilitySnapshot) -> bool {
        self.set_content_accessibility_publication(WindowAccessibilityPublication::prepare(vec![(
            ContentAccessibilityOwner(0),
            snapshot,
        )]))
    }

    fn set_content_accessibility_publication(
        &self,
        publication: Option<WindowAccessibilityPublication>,
    ) -> bool {
        let lease = publication.map(|p| {
            std::rc::Rc::new(p)
                as std::rc::Rc<dyn tekes_ui_platform_macos::AccessibilityGeometryLease>
        });
        self.set_accessibility_geometry_lease(lease)
    }

    fn content_accessibility_publication(&self) -> Option<WindowAccessibilityPublication> {
        self.accessibility_geometry_lease().and_then(|lease| {
            lease
                .as_any()
                .downcast_ref::<WindowAccessibilityPublication>()
                .cloned()
        })
    }
}
