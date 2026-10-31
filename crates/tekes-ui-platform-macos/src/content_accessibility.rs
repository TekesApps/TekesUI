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
    if semantics.role == ContentRole::ProgressIndicator {
        let range = host
            .semantic_value_range(id, part)
            .and_then(|range| AccessibilityRange::new(*range.start(), *range.end()));
        node.role = AccessibilityRole::ProgressIndicator(range);
        node.read_only = true;
        // Preserve the Content's display string. Native numeric value is the
        // existing percent representation without its suffix; the publication
        // boundary validates it against the original supplied numeric domain.
        node.value = range.and_then(|_| {
            semantics
                .value
                .as_deref()
                .map(|value| value.strip_suffix('%').unwrap_or(value).to_owned())
        });
    }
    if part.is_none() {
        node.can_show_menu = host.can_show_context_menu(id);
        add_slider_range(host, id, semantics, node);
        // Preserve the existing value/selection contract before shaping. Complete
        // geometry is still published only from the original shaped layout below.
        if matches!(
            semantics.role,
            ContentRole::TextInput | ContentRole::ComboBox | ContentRole::SearchInput | ContentRole::Image
        ) && let Some(input) = host.semantic_text_input_snapshot(id)
        {
            if !matches!(semantics.role, ContentRole::Image | ContentRole::ComboBox | ContentRole::SearchInput)
                && host
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
        if !matches!(
            node.role,
            AccessibilityRole::Image | AccessibilityRole::Splitter(_) | AccessibilityRole::ComboBox | AccessibilityRole::SearchField
        ) && layout.multiline
        {
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
    project_content_nodes(host).unwrap_or_default()
}
fn project_content_nodes(host: &ContentHost) -> Option<Vec<AccessibilityNode>> {
    if !host.is_window_active() {
        return Some(Vec::new());
    }
    let mut nodes = Vec::new();
    for n in host.semantics() {
        if n.semantics.role == ContentRole::Image && n.semantics.name.is_empty() {
            continue;
        }
        let Some(key) = content_key(host, n.id, None) else {
            continue;
        };
        if n.semantics.role == ContentRole::Select
            && (host.text_surface(n.id).is_some()
                || host.semantic_text_input_snapshot(n.id).is_some()
                || host.semantic_text_layout_snapshot(n.id, None).is_some())
        {
            return None;
        }
        if matches!(n.semantics.role, ContentRole::ComboBox | ContentRole::SearchInput)
            && (host
                .text_surface(n.id)
                .is_some_and(|surface| surface.input.multiline)
                || host
                    .semantic_text_layout_snapshot(n.id, None)
                    .is_some_and(|layout| layout.multiline))
        {
            return None;
        }
        let mut owner = project(
            key,
            n.parent.and_then(|p| content_key(host, p, None)),
            n.frame,
            n.focused && !n.parts.iter().any(|p| p.focused),
            &n.semantics,
        );
        if n.semantics.role == ContentRole::Splitter {
            let range = host.semantic_value_range(n.id, None)?;
            owner.role =
                AccessibilityRole::Splitter(AccessibilityRange::new(*range.start(), *range.end())?);
        }
        enrich(host, n.id, None, &n.semantics, &mut owner);
        nodes.push(owner);
        for part in n.parts {
            if part.semantics.role == ContentRole::Image && part.semantics.name.is_empty() {
                continue;
            }
            if part.semantics.role == ContentRole::Select
                && host.semantic_text_layout_snapshot(n.id, Some(part.index)).is_some()
            {
                return None;
            }
            if matches!(part.semantics.role, ContentRole::ComboBox | ContentRole::SearchInput)
                && host
                    .semantic_text_layout_snapshot(n.id, Some(part.index))
                    .is_some_and(|layout| layout.multiline)
            {
                return None;
            }
            let key = AccessibilityKey {
                part: Some(part.index),
                ..key
            };
            let parent = content_key(host, n.id, host.semantic_part_parent(n.id, part.index));
            let mut node = project(key, parent, part.frame, part.focused, &part.semantics);
            if matches!(
                part.semantics.role,
                ContentRole::Slider | ContentRole::Splitter
            ) {
                // A virtual adjustable part must retain its original real domain.
                // Unlike the legacy local-owner branch, no Group fallback is valid.
                let range = host.semantic_value_range(n.id, Some(part.index))?;
                let range = AccessibilityRange::new(*range.start(), *range.end())?;
                node.role = if part.semantics.role == ContentRole::Splitter {
                    AccessibilityRole::Splitter(range)
                } else {
                    AccessibilityRole::Slider(range)
                };
            }
            enrich(host, n.id, Some(part.index), &part.semantics, &mut node);
            nodes.push(node);
        }
    }
    Some(nodes)
}
/// Complete original shaped lines for every active text owner, including
/// unfocused/readonly child editors. Invalid or absent geometry is not guessed.
pub fn content_text_layouts(
    host: &ContentHost,
) -> HashMap<AccessibilityKey, AccessibilityTextLayout> {
    let mut result = HashMap::new();
    for node in project_content_nodes(host).unwrap_or_default() {
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
            Role::ProgressIndicator => NativeRole::ProgressIndicator(None),
            Role::Text => NativeRole::StaticText,
            Role::Image => NativeRole::Image,
            Role::Button => NativeRole::Button,
            Role::Link => NativeRole::Link,
            Role::TextInput => NativeRole::TextField,
            Role::SearchInput => NativeRole::SearchField,
            Role::ComboBox => NativeRole::ComboBox,
            Role::Slider => NativeRole::Group,
            Role::Splitter => {
                NativeRole::Splitter(AccessibilityRange::new(0., 1.).expect("split ratio domain"))
            }
            Role::Switch => NativeRole::Switch,
            Role::Checkbox => NativeRole::Checkbox,
            Role::RadioGroup | Role::SegmentedGroup => NativeRole::RadioGroup,
            Role::RadioButton | Role::Segment => NativeRole::RadioButton,
            Role::List => NativeRole::List,
            Role::ListItem => NativeRole::ListItem,
            Role::Dialog => NativeRole::Dialog,
            Role::MenuButton => NativeRole::MenuButton,
            Role::Select => NativeRole::PopUpButton,
            Role::Menu => NativeRole::Menu,
            Role::MenuItem => NativeRole::MenuItem,
            Role::TabList => NativeRole::TabGroup,
            Role::Tab => NativeRole::Tab,
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
        can_show_menu: false,
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
        AccessibilityAction::ShowMenu => ContentAction::ShowMenu,
        AccessibilityAction::SetValue(value) => ContentAction::SetValue(value),
        AccessibilityAction::SetSelection(range) => ContentAction::SetSelection(range),
        AccessibilityAction::ReplaceSelectedText(text) => ContentAction::ReplaceSelectedText(text),
        AccessibilityAction::SetExpanded(value) => ContentAction::SetExpanded(value),
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
        let mut nodes = project_content_nodes(host)?;
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

#[cfg(test)]
mod image_role_local_port_tests {
    use super::*;
    use tekes_ui_controls::{
        ButtonTextRenderer, TextSurface,
        composition::{Component, Content},
        layout::MeasuredChild,
    };
    #[derive(Debug)]
    struct ImageWithTextSurface(TextSurface);
    impl Component for ImageWithTextSurface {
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            _: tekes_ui_scene::Size,
        ) -> MeasuredChild {
            MeasuredChild::new(tekes_ui_scene::Size::new(20., 10.))
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::image_alt("Informative image").unwrap()
        }
        fn text_surface(&self) -> Option<&TextSurface> {
            Some(&self.0)
        }
    }
    #[test]
    fn local_image_alt_role_and_invalid_text_owner_preserve_atomic_native_acceptance() {
        assert!(ContentSemantics::image_alt(" \n").is_none());
        let decorative = ContentSemantics::decorative_image();
        assert_eq!(decorative.role, ContentRole::Image);
        assert!(decorative.name.is_empty());
        let semantics = ContentSemantics::image_alt("Informative image").unwrap();
        let key = AccessibilityKey {
            component: 120,
            part: None,
        };
        let mut image = project(
            key,
            None,
            tekes_ui_scene::Rect::new(0., 0., 20., 10.),
            false,
            &semantics,
        );
        assert_eq!(image.role, AccessibilityRole::Image);
        assert_eq!(image.label, "Informative image");
        assert_eq!(
            crate::accessibility::accept_accessibility_nodes(vec![image.clone()]),
            vec![image.clone()]
        );
        let host = ContentHost::new(Content::new(
            ComponentId(120),
            ImageWithTextSurface(TextSurface::new("not image alt", true, false)),
        ))
        .unwrap();
        enrich(&host, ComponentId(120), None, &semantics, &mut image);
        assert_eq!(
            image.role,
            AccessibilityRole::Image,
            "multiline metadata cannot downgrade the real Image role"
        );
        assert!(image.text_selection.is_some());
        let prefix = project(
            AccessibilityKey {
                component: 1,
                part: None,
            },
            None,
            tekes_ui_scene::Rect::new(0., 0., 20., 10.),
            false,
            &ContentSemantics::new(ContentRole::Text, "valid prefix"),
        );
        assert_eq!(
            crate::accessibility::accept_accessibility_nodes(vec![prefix.clone()]),
            vec![prefix.clone()]
        );
        assert!(crate::accessibility::accept_accessibility_nodes(vec![prefix, image]).is_empty());
    }
    #[derive(Debug)]
    struct AdjustablePart(std::ops::RangeInclusive<f64>);
    impl Component for AdjustablePart {
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            _: tekes_ui_scene::Size,
        ) -> MeasuredChild {
            MeasuredChild::new(tekes_ui_scene::Size::new(40., 20.))
        }
        fn semantic_parts(&self) -> Vec<tekes_ui_controls::composition::SemanticPart> {
            vec![tekes_ui_controls::composition::SemanticPart {
                index: 7,
                frame: tekes_ui_scene::Rect::new(0., 0., 40., 20.),
                focused: false,
                semantics: ContentSemantics::new(ContentRole::Slider, "remote slider"),
            }]
        }
        fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
            (part == 7).then(|| self.0.clone())
        }
    }
    #[test]
    fn virtual_slider_retains_domain_and_invalid_part_withdraws_the_complete_projection() {
        let id = ComponentId(91);
        let mut host = ContentHost::new(Content::new(id, AdjustablePart(-40.0..=160.0))).unwrap();
        assert_eq!(host.semantic_value_range(id, Some(7)), Some(-40.0..=160.0));
        assert!(host.semantic_value_range(id, Some(8)).is_none());
        let nodes = content_nodes(&host);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].key.part, Some(7));
        assert_eq!(
            nodes[1].role,
            AccessibilityRole::Slider(AccessibilityRange::new(-40., 160.).unwrap())
        );
        for range in [
            std::ops::RangeInclusive::new(f64::NAN, 1.0),
            std::ops::RangeInclusive::new(0.0, f64::INFINITY),
            std::ops::RangeInclusive::new(2.0, 1.0),
        ] {
            host.get_mut(id)
                .unwrap()
                .component_mut::<AdjustablePart>()
                .unwrap()
                .0 = range;
            assert!(host.semantic_value_range(id, Some(7)).is_none());
            assert!(
                content_nodes(&host).is_empty(),
                "no valid-prefix or Group downgrade"
            );
        }
        host.get_mut(id)
            .unwrap()
            .component_mut::<AdjustablePart>()
            .unwrap()
            .0 = -40.0..=160.0;
        host.set_window_active(false);
        assert!(host.semantic_value_range(id, Some(7)).is_none());
        assert!(content_nodes(&host).is_empty());
    }
    #[test]
    fn ordinary_local_slider_domain_query_keeps_its_existing_inactive_contract() {
        use tekes_ui_controls::{composition::SliderContent, extended::Slider};
        let id = ComponentId(92);
        let mut host = ContentHost::new(Content::new(
            id,
            SliderContent::new("local slider", Slider::new(20., -40., 160., 1.).unwrap()),
        ))
        .unwrap();
        assert_eq!(host.value_range(id), Some(-40.0..=160.0));
        assert_eq!(
            content_nodes(&host)[0].role,
            AccessibilityRole::Slider(AccessibilityRange::new(-40., 160.).unwrap())
        );
        host.set_window_active(false);
        assert_eq!(host.value_range(id), Some(-40.0..=160.0));
        assert_eq!(host.semantic_value_range(id, None), Some(-40.0..=160.0));
        assert!(content_nodes(&host).is_empty());
    }
}

#[cfg(test)]
mod splitter_projection_tests {
    use super::*;

    #[test]
    fn splitter_role_projects_to_native_splitter_with_finite_range() {
        let mut semantics = ContentSemantics::new(ContentRole::Splitter, "Split view divider");
        semantics.value = Some("0.5".into());
        let node = project(
            AccessibilityKey {
                component: 103,
                part: None,
            },
            None,
            tekes_ui_scene::Rect::new(20.0, 0.0, 1.0, 80.0),
            true,
            &semantics,
        );
        assert_eq!(
            node.role,
            AccessibilityRole::Splitter(AccessibilityRange::new(0.0, 1.0).unwrap())
        );
        assert_eq!(node.value.as_deref(), Some("0.5"));
        assert!(node.focused);
    }
}

#[cfg(test)]
mod splitter_content_bridge_tests {
    use super::*;
    use crate::AccessibilityAction;
    use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
    use tekes_ui_scene::{Rect as SceneRect, Scene, Size as SceneSize};
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, text: &str, _: proposal::TextRole, width: f32) -> SceneSize {
            SceneSize::new((text.len() as f32 * 7.0).min(width), 20.0)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: SceneRect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }

    fn split_host(min_pane: f32) -> ContentHost {
        ContentHost::try_new_prepared(
            Content::new(
                ComponentId(1),
                SplitViewContent::try_new(SplitViewConstruction {
                    config: SplitViewConfig {
                        min_pane,
                        ..SplitViewConfig::default()
                    },
                    slots: SplitViewSlotIds {
                        leading: ComponentId(2),
                        trailing: ComponentId(3),
                        divider: ComponentId(4),
                    },
                    leading: Content::new(ComponentId(5), Button::new("leading")),
                    trailing: Content::new(ComponentId(6), Button::new("trailing")),
                    budget: 64 * 1024 * 1024,
                })
                .unwrap(),
            ),
            64 * 1024 * 1024,
        )
        .unwrap()
    }

    fn paint(host: &mut ContentHost, width: f32) {
        let frame = SceneRect::new(0.0, 0.0, width, 80.0);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    }

    #[derive(Debug)]
    struct SplitterPart {
        domain: Option<std::ops::RangeInclusive<f64>>,
    }
    impl Component for SplitterPart {
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            _: SceneSize,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(SceneSize::new(40., 80.))
        }
        fn semantic_parts(&self) -> Vec<SemanticPart> {
            let mut semantics =
                ContentSemantics::new(ContentRole::Splitter, "Remote split divider");
            semantics.value = Some("0.5".into());
            vec![SemanticPart {
                index: 7,
                frame: SceneRect::new(20., 0., 1., 80.),
                focused: false,
                semantics,
            }]
        }
        fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
            (part == 7).then(|| self.domain.clone()).flatten()
        }
    }
    #[test]
    fn splitter_part_uses_original_range_and_missing_domain_withdraws_complete_snapshot() {
        let id = ComponentId(91);
        let mut host = ContentHost::new(Content::new(
            id,
            SplitterPart {
                domain: Some(0.1..=0.9),
            },
        ))
        .unwrap();
        paint(&mut host, 101.);
        let nodes = content_nodes(&host);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].key.part, Some(7));
        assert_eq!(
            nodes[1].role,
            AccessibilityRole::Splitter(AccessibilityRange::new(0.1, 0.9).unwrap())
        );
        assert_eq!(nodes[1].value.as_deref(), Some("0.5"));
        assert!(crate::accessibility::validate_accessibility_nodes(&nodes));
        assert!(host.semantic_value_range(id, Some(8)).is_none());
        host.get_mut(id)
            .unwrap()
            .component_mut::<SplitterPart>()
            .unwrap()
            .domain = None;
        assert!(content_nodes(&host).is_empty());
        for domain in [
            std::ops::RangeInclusive::new(f64::NAN, 1.),
            std::ops::RangeInclusive::new(0., f64::INFINITY),
            std::ops::RangeInclusive::new(0.9, 0.1),
        ] {
            host.get_mut(id)
                .unwrap()
                .component_mut::<SplitterPart>()
                .unwrap()
                .domain = Some(domain);
            assert!(content_nodes(&host).is_empty());
        }
    }

    #[test]
    fn non_binary_minimum_domain_survives_native_snapshot_validation_atomically() {
        let mut host = split_host(10.0);
        paint(&mut host, 101.0);
        let nodes = content_nodes(&host);
        let splitter = nodes
            .iter()
            .find(|node| matches!(node.role, AccessibilityRole::Splitter(_)))
            .expect("visible splitter")
            .clone();
        let AccessibilityRole::Splitter(range) = splitter.role else {
            unreachable!()
        };
        assert_eq!(range.minimum(), 0.1);
        assert_eq!(range.maximum(), 0.9);
        let published_value = splitter.value.as_deref().unwrap().parse::<f64>().unwrap();
        assert!(range.minimum() <= published_value && published_value <= range.maximum());
        assert!(crate::accessibility::validate_accessibility_nodes(&nodes));
        assert_eq!(
            crate::accessibility::accept_accessibility_nodes(nodes.clone()),
            nodes
        );

        let root_key = content_key(&host, ComponentId(1), None).unwrap();
        let prefix = content_nodes(&host)
            .into_iter()
            .find(|node| node.key == root_key)
            .expect("valid prefix owner");
        assert!(crate::accessibility::validate_accessibility_nodes(
            std::slice::from_ref(&prefix)
        ));
        let mut invalid_splitter = splitter;
        invalid_splitter.value = Some("0.05".into());
        let invalid_packet = vec![prefix, invalid_splitter];
        assert!(!crate::accessibility::validate_accessibility_nodes(
            &invalid_packet
        ));
        assert!(crate::accessibility::accept_accessibility_nodes(invalid_packet).is_empty());
    }

    #[test]
    fn constrained_divider_is_absent_and_its_old_action_is_rejected() {
        let mut host = split_host(10.0);
        paint(&mut host, 101.0);
        let old_key = content_key(&host, ComponentId(4), None).unwrap();
        assert!(
            content_nodes(&host)
                .iter()
                .any(|node| node.key == old_key
                    && matches!(node.role, AccessibilityRole::Splitter(_)))
        );

        paint(&mut host, 10.0);
        assert!(
            content_nodes(&host)
                .iter()
                .all(|node| !matches!(node.role, AccessibilityRole::Splitter(_)))
        );
        assert!(
            !content_action(
                &mut host,
                old_key,
                AccessibilityAction::SetValue("0.5".into())
            )
            .consumed
        );
    }
}

#[cfg(test)]
mod context_menu_bridge_tests {
    use super::*;
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer, ControlKey,
        composition::{
            Button, Content, ContentActionPayload, ContentActionSource, ContentEvent, MenuContent,
            MenuEntry, StackContent,
        },
        extended::{Axis, Stack},
    };
    use tekes_ui_scene::{Rect as SceneRect, Scene, Size as SceneSize};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, max: f32) -> SceneSize {
            SceneSize::new((s.len() as f32 * 8.).min(max), 24.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: SceneRect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    const ROOT: ComponentId = ComponentId(4400);
    const A: ComponentId = ComponentId(4401);
    const B: ComponentId = ComponentId(4402);
    const MENU: ComponentId = ComponentId(4403);
    fn fixture() -> ContentHost {
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(A, Button::new("A")),
                    Content::new(B, Button::new("B")),
                    Content::new(
                        MENU,
                        MenuContent::context_menu("Actions", vec![MenuEntry::item("pin", "Pin")])
                            .unwrap(),
                    ),
                ],
            },
        ))
        .unwrap();
        host.set_action_events(true);
        host.bind_context_menu(B, MENU).unwrap();
        host
    }
    fn render(host: &mut ContentHost) {
        host.render(
            &mut Scene::new(),
            &mut Text,
            SceneRect::new(0., 0., 300., 200.),
            SceneRect::new(0., 0., 300., 200.),
            Theme::LIGHT.into(),
        )
        .unwrap();
    }
    #[test]
    fn native_context_projection_advertises_only_original_bound_owner_and_routes_same_mount() {
        let mut host = fixture();
        let b = content_key(&host, B, None).unwrap();
        assert!(
            !content_nodes(&host)
                .iter()
                .find(|node| node.key == b)
                .unwrap()
                .can_show_menu
        );
        render(&mut host);
        host.focus(Some(A), true);
        let nodes = content_nodes(&host);
        assert!(
            nodes
                .iter()
                .find(|node| node.key == b)
                .unwrap()
                .can_show_menu
        );
        assert!(
            nodes
                .iter()
                .filter(|node| node.key != b)
                .all(|node| !node.can_show_menu)
        );
        let response = content_action(&mut host, b, AccessibilityAction::ShowMenu);
        assert!(response.consumed && !response.activated);
        assert_eq!(host.focused(), Some(A));
        assert!(host.drain_actions().is_empty());
        render(&mut host);
        host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
        let record = host.drain_action_records().remove(0);
        let Some(ContentActionPayload::MenuSelection(selection)) = record.payload else {
            panic!("missing menu selection")
        };
        assert_eq!(selection.context_request().unwrap().owner(), B);
        assert_eq!(
            selection.context_request().unwrap().source(),
            ContentActionSource::Accessibility
        );
        assert!(content_action(&mut host, b, AccessibilityAction::Press).activated);
    }
    #[test]
    fn native_context_projection_and_action_reject_disabled_stale_and_inactive_owner() {
        let mut host = fixture();
        render(&mut host);
        let old = content_key(&host, B, None).unwrap();
        host.get_mut(B)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = true;
        assert!(
            !content_nodes(&host)
                .iter()
                .find(|node| node.key == old)
                .unwrap()
                .can_show_menu
        );
        assert!(!content_action(&mut host, old, AccessibilityAction::ShowMenu).consumed);
        host.get_mut(B)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = false;
        let row = host.remove(B).unwrap();
        host.insert(ROOT, row).unwrap();
        render(&mut host);
        assert!(!content_action(&mut host, old, AccessibilityAction::ShowMenu).consumed);
        let current = content_key(&host, B, None).unwrap();
        assert!(
            !content_nodes(&host)
                .iter()
                .find(|node| node.key == current)
                .unwrap()
                .can_show_menu
        );
        host.bind_context_menu(B, MENU).unwrap();
        render(&mut host);
        host.set_window_active(false);
        assert!(content_nodes(&host).is_empty());
        assert!(!content_action(&mut host, current, AccessibilityAction::ShowMenu).consumed);
    }
}

#[cfg(test)]
#[cfg(test)]
mod navigation_role_tests {
    use super::*;

    #[test]
    fn tab_roles_keep_their_native_accessibility_identity() {
        for (role, native) in [
            (ContentRole::TabList, AccessibilityRole::TabGroup),
            (ContentRole::Tab, AccessibilityRole::Tab),
        ] {
            let mut semantics = ContentSemantics::new(role, "Workspace");
            semantics.selected = true;
            let node = project(
                AccessibilityKey {
                    component: 63,
                    part: Some(1),
                },
                None,
                tekes_ui_scene::Rect::new(10., 20., 80., 40.),
                true,
                &semantics,
            );
            assert_eq!(node.role, native);
            assert!(node.selected && node.focused);
            assert_eq!(node.label, "Workspace");
        }
    }
}

#[cfg(test)]
mod selection_projection_tests {
    use super::*;
    #[test]
    fn all_five_selection_roles_keep_values_and_finite_native_identity() {
        let key = AccessibilityKey {
            component: 107,
            part: None,
        };
        for (role, expected) in [
            (ContentRole::Checkbox, AccessibilityRole::Checkbox),
            (ContentRole::RadioGroup, AccessibilityRole::RadioGroup),
            (ContentRole::RadioButton, AccessibilityRole::RadioButton),
            (ContentRole::SegmentedGroup, AccessibilityRole::RadioGroup),
            (ContentRole::Segment, AccessibilityRole::RadioButton),
        ] {
            let mut s = ContentSemantics::new(role, "Choice");
            s.disabled = true;
            if role == ContentRole::Checkbox {
                s.value = Some("mixed".into());
            } else if matches!(role, ContentRole::RadioButton | ContentRole::Segment) {
                s.selected = true;
            }
            let n = project(
                key,
                None,
                tekes_ui_scene::Rect::new(0., 0., 40., 30.),
                false,
                &s,
            );
            assert_eq!(n.role, expected);
            assert_eq!(n.value, s.value);
            assert_eq!(n.selected, s.selected);
            assert!(!n.enabled);
            assert!(n.text_selection.is_none());
        }
    }
}
