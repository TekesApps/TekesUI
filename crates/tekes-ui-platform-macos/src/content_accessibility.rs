//! Projection of the original composition tree into the platform's existing
//! macOS accessibility node/action contract.
//!
//! This adapter owns no editor or control state. Stable keys resolve through
//! the current ContentHost mount, and accepted actions are routed back through
//! the original component state machine.

use crate::{
    AccessibilityAction, AccessibilityKey, AccessibilityNode, AccessibilityRange,
    AccessibilityRole, Point, Rect, Size,
};
use tekes_ui_controls::composition::{
    AccessibilityAction as ContentAction, ComponentId, ContentHost, ContentResponse, ContentRole,
    ContentSemantics,
};

pub fn content_key(
    host: &ContentHost,
    id: ComponentId,
    part: Option<usize>,
) -> Option<AccessibilityKey> {
    host.semantic_mount(id)
        .map(|component| AccessibilityKey { component, part })
}

pub fn content_nodes(host: &ContentHost) -> Vec<AccessibilityNode> {
    let mut out = Vec::new();
    for node in host.semantics() {
        let Some(key) = content_key(host, node.id, None) else {
            continue;
        };
        let parent = node.parent.and_then(|id| content_key(host, id, None));
        let mut projected = project(key, parent, node.frame, node.focused, &node.semantics);
        add_slider_range(host, node.id, &node.semantics, &mut projected);
        if matches!(node.semantics.role, ContentRole::TextInput)
            && let Some(input) = host.semantic_text_input_snapshot(node.id)
        {
            if host
                .text_surface(node.id)
                .is_some_and(|surface| surface.input.multiline)
            {
                projected.role = AccessibilityRole::TextArea;
            }
            projected.value = Some(input.text);
            projected.text_selection = Some(input.selection);
            projected.read_only = input.read_only;
        }
        out.push(projected);

        for part in node.parts {
            let part_key = AccessibilityKey {
                part: Some(part.index),
                ..key
            };
            let parent = host
                .semantic_part_parent(node.id, part.index)
                .map(|index| AccessibilityKey {
                    part: Some(index),
                    ..key
                })
                .or(Some(key));
            let projected = project(part_key, parent, part.frame, part.focused, &part.semantics);
            out.push(projected);
        }
    }
    out
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
        composition::{ButtonContent, Content, StackContent, TextFieldContent},
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
                children: vec![Content::new(CHILD, ButtonContent::new("Run"))],
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
                .component::<ButtonContent>()
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
        host.insert(ROOT, Content::new(CHILD, ButtonContent::new("Replacement")))
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
                .component::<ButtonContent>()
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
