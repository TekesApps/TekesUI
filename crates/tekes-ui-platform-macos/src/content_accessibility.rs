//! Projection of the original composition tree into the platform's existing
//! macOS accessibility node/action contract.
//!
//! This adapter owns no editor or control state. Stable keys resolve through
//! the current ContentHost mount, and accepted actions are routed back through
//! the original component state machine.

use crate::{
    AccessibilityAction, AccessibilityKey, AccessibilityNode, AccessibilityRange,
    AccessibilityRole, AccessibilityTextLayout, Point, Rect, Size, TextInputState,
};
use tekes_ui_controls::composition::{
    AccessibilityAction as ContentAction, ComponentId, ContentHost, ContentResponse, ContentRole,
    ContentSemantics, ContentTextLayout,
};

use std::collections::HashMap;

pub fn content_key(
    host: &ContentHost,
    id: ComponentId,
    part: Option<usize>,
) -> Option<AccessibilityKey> {
    host.semantic_mount(id)
        .map(|component| AccessibilityKey { component, part })
}

fn text_layout(layout: ContentTextLayout) -> AccessibilityTextLayout {
    let caret = layout.input.caret.unwrap_or_default();
    let convert = |r: tekes_ui_scene::Rect| {
        (
            Point {
                x: r.min_x() as f64,
                y: r.min_y() as f64,
            },
            Size {
                width: r.size.width as f64,
                height: r.size.height as f64,
            },
        )
    };
    let (caret_origin, caret_size) = convert(caret);
    AccessibilityTextLayout {
        state: TextInputState {
            text: layout.input.text,
            caret_index: layout.input.caret_index,
            selection: layout.input.selection,
            marked: layout.input.marked,
            read_only: layout.input.read_only,
            caret_origin,
            caret_size,
            carets: layout
                .carets
                .into_iter()
                .map(|(i, r)| {
                    let (p, s) = convert(r);
                    (i, p, s)
                })
                .collect(),
        },
        lines: layout.lines,
    }
}
fn enrich(
    host: &ContentHost,
    id: ComponentId,
    part: Option<usize>,
    semantics: &ContentSemantics,
    node: &mut AccessibilityNode,
) {
    if part.is_none() {
        add_slider_range(host, id, semantics, node);
        // Preserve the existing value/selection contract before shaping. Complete
        // geometry is still published only from the original shaped layout below.
        if semantics.role == ContentRole::TextInput
            && let Some(input) = host.semantic_text_input_snapshot(id)
        {
            if host
                .text_surface(id)
                .is_some_and(|surface| surface.input.multiline)
            {
                node.role = AccessibilityRole::TextArea;
            }
            node.value = Some(input.text);
            node.text_selection = Some(input.selection);
            node.read_only = input.read_only;
        }
    }
    if let Some(layout) = host.semantic_text_layout_snapshot(id, part) {
        if layout.multiline {
            node.role = AccessibilityRole::TextArea;
        }
        node.value = Some(layout.input.text.clone());
        node.text_selection = Some(layout.input.selection);
        node.read_only = layout.input.read_only;
    }
}
pub fn content_nodes(host: &ContentHost) -> Vec<AccessibilityNode> {
    if host.requires_curve_geometry() {
        return Vec::new();
    }
    project_content_nodes(host)
}
fn project_content_nodes(host: &ContentHost) -> Vec<AccessibilityNode> {
    if !host.is_window_active() {
        return Vec::new();
    }
    let mut nodes = Vec::new();
    for n in host.semantics() {
        let Some(key) = content_key(host, n.id, None) else {
            continue;
        };
        let mut owner = project(
            key,
            n.parent.and_then(|p| content_key(host, p, None)),
            n.frame,
            n.focused && !n.parts.iter().any(|p| p.focused),
            &n.semantics,
        );
        enrich(host, n.id, None, &n.semantics, &mut owner);
        nodes.push(owner);
        for part in n.parts {
            let key = AccessibilityKey {
                part: Some(part.index),
                ..key
            };
            let parent = content_key(host, n.id, host.semantic_part_parent(n.id, part.index));
            let mut node = project(key, parent, part.frame, part.focused, &part.semantics);
            enrich(host, n.id, Some(part.index), &part.semantics, &mut node);
            nodes.push(node);
        }
    }
    nodes
}
/// Complete original shaped lines for every active text owner, including
/// unfocused/readonly child editors. Invalid or absent geometry is not guessed.
pub fn content_text_layouts(
    host: &ContentHost,
) -> HashMap<AccessibilityKey, AccessibilityTextLayout> {
    let mut result = HashMap::new();
    for node in project_content_nodes(host) {
        let Some(id) = host
            .semantics()
            .iter()
            .find(|n| host.semantic_mount(n.id) == Some(node.key.component))
            .map(|n| n.id)
        else {
            continue;
        };
        if let Some(layout) = host.semantic_text_layout_snapshot(id, node.key.part) {
            let layout = text_layout(layout);
            if layout.is_valid() {
                result.insert(node.key, layout);
            }
        }
    }
    result
}
fn add_slider_range(
    host: &ContentHost,
    id: ComponentId,
    semantics: &ContentSemantics,
    projected: &mut AccessibilityNode,
) {
    if !matches!(semantics.role, ContentRole::Slider) {
        return;
    }
    if let Some(range) = host.value_range(id) {
        if let Some(range) = AccessibilityRange::new(*range.start(), *range.end()) {
            projected.role = AccessibilityRole::Slider(range);
        }
    } else {
        // Do not publish an adjustable native role without its real domain.
        projected.role = AccessibilityRole::Group;
    }
}

fn project(
    key: AccessibilityKey,
    parent: Option<AccessibilityKey>,
    frame: tekes_ui_scene::Rect,
    focused: bool,
    semantics: &ContentSemantics,
) -> AccessibilityNode {
    use AccessibilityRole as NativeRole;
    use ContentRole as Role;

    AccessibilityNode {
        key,
        parent,
        role: match semantics.role {
            Role::Group => NativeRole::Group,
            Role::Text => NativeRole::StaticText,
            Role::Button => NativeRole::Button,
            Role::Link => NativeRole::Link,
            Role::TextInput => NativeRole::TextField,
            Role::Slider => NativeRole::Group,
            Role::Switch => NativeRole::Switch,
            Role::List => NativeRole::List,
            Role::ListItem => NativeRole::ListItem,
            Role::Dialog => NativeRole::Dialog,
            Role::MenuButton => NativeRole::MenuButton,
            Role::Menu => NativeRole::Menu,
            Role::MenuItem => NativeRole::MenuItem,
            _ => NativeRole::Group,
        },
        frame: Rect {
            origin: Point {
                x: frame.min_x() as f64,
                y: frame.min_y() as f64,
            },
            size: Size {
                width: frame.size.width as f64,
                height: frame.size.height as f64,
            },
        },
        label: semantics.name.clone(),
        value: semantics.value.clone(),
        description: semantics.description.clone(),
        enabled: !semantics.disabled,
        focused,
        selected: semantics.selected,
        expanded: semantics.expanded,
        invalid: semantics.invalid,
        required: semantics.required,
        text_selection: None,
        read_only: false,
    }
}

pub fn content_action(
    host: &mut ContentHost,
    target: AccessibilityKey,
    action: AccessibilityAction,
) -> ContentResponse {
    let Some(id) = host.semantics().into_iter().find_map(|node| {
        (host.semantic_mount(node.id) == Some(target.component)).then_some(node.id)
    }) else {
        return ContentResponse::default();
    };
    let action = match action {
        AccessibilityAction::Increment => ContentAction::Increment,
        AccessibilityAction::Decrement => ContentAction::Decrement,
        AccessibilityAction::Focus => ContentAction::Focus,
        AccessibilityAction::Press => ContentAction::Press,
        AccessibilityAction::SetValue(value) => ContentAction::SetValue(value),
        AccessibilityAction::SetSelection(range) => ContentAction::SetSelection(range),
        AccessibilityAction::ReplaceSelectedText(text) => ContentAction::ReplaceSelectedText(text),
        AccessibilityAction::SetExpanded(_) => return ContentResponse::default(),
    };
    host.accessibility_action(id, target.part, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::{
        composition::{Button, Content, StackContent, TextFieldContent},
        extended::{Axis, Stack},
    };

    const ROOT: ComponentId = ComponentId(1);
    const CHILD: ComponentId = ComponentId(2);

    fn button_host() -> ContentHost {
        ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                children: vec![Content::new(CHILD, Button::new("Run"))],
                flexible_child: None,
            },
        ))
        .unwrap()
    }

    #[test]
    fn current_button_key_routes_press_to_the_original_button() {
        let mut host = button_host();
        let key = content_key(&host, CHILD, None).unwrap();
        let node = content_nodes(&host)
            .into_iter()
            .find(|node| node.key == key)
            .unwrap();
        assert_eq!(node.role, AccessibilityRole::Button);
        let response = content_action(&mut host, key, AccessibilityAction::Press);
        assert!(response.consumed && response.activated);
        assert_eq!(
            host.get(CHILD)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            1
        );
    }

    #[test]
    fn stale_mount_key_cannot_press_a_replacement_with_the_same_component_id() {
        let mut host = button_host();
        let old_key = content_key(&host, CHILD, None).unwrap();
        host.remove(CHILD).unwrap();
        host.insert(ROOT, Content::new(CHILD, Button::new("Replacement")))
            .unwrap();
        let new_key = content_key(&host, CHILD, None).unwrap();
        assert_ne!(old_key, new_key);
        assert_eq!(
            content_action(&mut host, old_key, AccessibilityAction::Press),
            ContentResponse::default()
        );
        assert_eq!(
            host.get(CHILD)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            0
        );
    }

    #[test]
    fn text_field_projection_uses_original_value_selection_and_read_only_state() {
        let mut field = TextFieldContent::new("Notes", "alpha beta");
        field.surface = tekes_ui_controls::TextSurface::new("alpha beta", true, true);
        assert!(field.surface.input.select_range(6..10));
        let host = ContentHost::new(Content::new(ComponentId(3), field)).unwrap();
        let projected = content_nodes(&host).into_iter().next().unwrap();
        assert_eq!(projected.role, AccessibilityRole::TextArea);
        assert_eq!(projected.value.as_deref(), Some("alpha beta"));
        assert_eq!(projected.text_selection, Some(6..10));
        assert!(projected.read_only);
    }
}

/// One immutable original Host publication: nodes, shaped text and exact point authority.
#[derive(Clone, Debug)]
pub struct ContentAccessibilitySnapshot {
    pub(crate) nodes: Vec<AccessibilityNode>,
    pub(crate) text_layouts: HashMap<AccessibilityKey, AccessibilityTextLayout>,
    geometry: tekes_ui_controls::composition::ContentGeometrySnapshot,
    identities: HashMap<AccessibilityKey, (ComponentId, Option<usize>)>,
    pub(crate) structural_only: std::collections::HashSet<AccessibilityKey>,
}
impl ContentAccessibilitySnapshot {
    pub fn nodes(&self) -> &[AccessibilityNode] {
        &self.nodes
    }
    pub fn text_layouts(&self) -> &HashMap<AccessibilityKey, AccessibilityTextLayout> {
        &self.text_layouts
    }
    pub fn from_host(host: &ContentHost) -> Option<Self> {
        if !host.is_window_active() {
            return None;
        }
        let geometry = host.geometry_snapshot()?;
        let mut nodes = project_content_nodes(host);
        let text_layouts = content_text_layouts(host);
        let semantics = host.semantics();
        let mut identities = HashMap::new();
        let mut structural_only = std::collections::HashSet::new();
        for node in &mut nodes {
            let id = semantics
                .iter()
                .find(|n| host.semantic_mount(n.id) == Some(node.key.component))?
                .id;
            if geometry.identity(id, node.key.part) != Some(node.key.component) {
                return None;
            }
            identities.insert(node.key, (id, node.key.part));
            let frame = tekes_ui_scene::Rect::new(
                node.frame.origin.x as f32,
                node.frame.origin.y as f32,
                node.frame.size.width as f32,
                node.frame.size.height as f32,
            );
            node.frame = match geometry.bounds(id, node.key.part, frame).ok()? {
                Some(b) => crate::placement::Rect {
                    origin: Point {
                        x: b.min_x() as f64,
                        y: b.min_y() as f64,
                    },
                    size: Size {
                        width: b.size.width as f64,
                        height: b.size.height as f64,
                    },
                },
                None => {
                    if matches!(node.role, AccessibilityRole::Group)
                        && !text_layouts.contains_key(&node.key)
                    {
                        structural_only.insert(node.key);
                    }
                    crate::placement::Rect {
                        origin: Point { x: 0., y: 0. },
                        size: Size {
                            width: 0.,
                            height: 0.,
                        },
                    }
                }
            };
        }
        let snapshot = Self {
            nodes,
            text_layouts,
            geometry,
            identities,
            structural_only,
        };
        if snapshot.is_valid() {
            Some(snapshot)
        } else {
            None
        }
    }
    /// Remap only explicit original keys into a window adapter's native namespace.
    /// Geometry and its original lease are retained unchanged; missing parents fail validation.
    pub fn with_native_keys(
        mut self,
        keys: &HashMap<AccessibilityKey, AccessibilityKey>,
    ) -> Option<Self> {
        let original = self
            .nodes
            .iter()
            .map(|n| n.key)
            .collect::<std::collections::HashSet<_>>();
        if keys.keys().any(|k| !original.contains(k)) {
            return None;
        }
        self.nodes.retain(|n| keys.contains_key(&n.key));
        for n in &mut self.nodes {
            n.key = *keys.get(&n.key)?;
            n.parent = match n.parent {
                Some(p) => Some(*keys.get(&p)?),
                None => None,
            };
        }
        self.text_layouts = self
            .text_layouts
            .into_iter()
            .filter_map(|(k, v)| keys.get(&k).map(|k| (*k, v)))
            .collect();
        self.identities = self
            .identities
            .into_iter()
            .filter_map(|(k, v)| keys.get(&k).map(|k| (*k, v)))
            .collect();
        self.structural_only = self
            .structural_only
            .into_iter()
            .filter_map(|k| keys.get(&k).copied())
            .collect();
        self.is_valid().then_some(self)
    }
    pub fn generation(&self) -> u64 {
        self.geometry.generation
    }
    pub fn is_current(&self) -> bool {
        self.geometry.is_current()
    }
    pub fn is_valid(&self) -> bool {
        self.is_current()
            && crate::accessibility::validate_accessibility_nodes_with_geometry(
                &self.nodes,
                &self.identities.keys().copied().collect(),
            )
            && self
                .text_layouts
                .iter()
                .all(|(key, layout)| self.identities.contains_key(key) && layout.is_valid())
    }
    pub fn hit_test(&self, point: Point) -> Option<AccessibilityKey> {
        let (id, mount, part) = self
            .geometry
            .hit_test(tekes_ui_scene::Point::new(point.x as f32, point.y as f32))?;
        let key = *self
            .identities
            .iter()
            .find(|(_, v)| **v == (id, part))
            .map(|(k, _)| k)?;
        (self.geometry.identity(id, part) == Some(mount) && !self.structural_only.contains(&key))
            .then_some(key)
    }
    pub fn contains(&self, key: AccessibilityKey, point: Point) -> bool {
        self.hit_test(point) == Some(key)
    }
    pub fn frame_for_range(
        &self,
        key: AccessibilityKey,
        range: std::ops::Range<usize>,
    ) -> Option<crate::placement::Rect> {
        let (id, part) = *self.identities.get(&key)?;
        let rect = self.text_layouts.get(&key)?.frame_for_range(range)?;
        let r = tekes_ui_scene::Rect::new(
            rect.origin.x as f32,
            rect.origin.y as f32,
            rect.size.width as f32,
            rect.size.height as f32,
        );
        let b = self.geometry.bounds(id, part, r).ok()??;
        Some(crate::placement::Rect {
            origin: Point {
                x: b.min_x() as f64,
                y: b.min_y() as f64,
            },
            size: Size {
                width: b.size.width as f64,
                height: b.size.height as f64,
            },
        })
    }
}

mod publication;
pub use publication::{ContentAccessibilityOwner, WindowAccessibilityPublication};
