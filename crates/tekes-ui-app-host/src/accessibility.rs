//! Data-only projection of original Content semantics onto the existing macOS
//! bridge. No mirrored component model or native control state is introduced.
//! Text layout/query geometry still uses Window::set_accessibility_text_layouts.
use tekes_ui_controls::composition::{
    AccessibilityAction as ContentAction, ComponentId, ContentHost, ContentResponse, ContentRole,
    ContentSemantics,
};
use tekes_ui_platform_macos::{
    AccessibilityAction, AccessibilityKey, AccessibilityNode, AccessibilityRange,
    AccessibilityRole, Point, Rect, Size,
};

/// The original mount disambiguates hosts sharing logical IDs, and replacements
/// using the same ID. Parts retain their original component-local identities.
pub fn content_key(
    host: &ContentHost,
    id: ComponentId,
    part: Option<usize>,
) -> Option<AccessibilityKey> {
    host.semantic_mount(id)
        .map(|component| AccessibilityKey { component, part })
}

/// Metadata and hierarchy only; this does not publish shaped text geometry or
/// prove VoiceOver behavior. No focus/layout/state mutation occurs here.
pub fn content_nodes(host: &ContentHost, active: bool) -> Vec<AccessibilityNode> {
    let mut nodes = Vec::new();
    for node in host.semantics() {
        let Some(key) = content_key(host, node.id, None) else {
            continue;
        };
        let mut owner = project(
            key,
            node.parent.and_then(|id| content_key(host, id, None)),
            node.frame,
            active && node.focused && !node.parts.iter().any(|p| p.focused),
            &node.semantics,
        );
        if matches!(node.semantics.role, ContentRole::Slider)
            && let Some(domain) = host.value_range(node.id)
            && let Some(range) = AccessibilityRange::new(*domain.start(), *domain.end())
        {
            owner.role = AccessibilityRole::Slider(range);
        }
        if let Some(surface) = host.text_surface(node.id) {
            if surface.input.multiline {
                owner.role = AccessibilityRole::TextArea;
            }
            owner.text_selection = Some(surface.input.selection());
            owner.read_only = surface.input.read_only;
        }
        nodes.push(owner);
        for part in node.parts {
            nodes.push(project(
                AccessibilityKey {
                    part: Some(part.index),
                    ..key
                },
                Some(AccessibilityKey {
                    part: host.semantic_part_parent(node.id, part.index),
                    ..key
                }),
                part.frame,
                active && part.focused,
                &part.semantics,
            ));
        }
    }
    nodes
}

/// Resolve the current original mount at dispatch time. Delayed events cannot
/// reach a new owner reusing a logical ID, another host, or an inactive subtree.
/// Callers still forward real window deactivation/cancellation to ContentHost.
pub fn content_action(
    host: &mut ContentHost,
    active: bool,
    target: AccessibilityKey,
    action: AccessibilityAction,
) -> ContentResponse {
    if !active {
        return ContentResponse::default();
    }
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
        AccessibilityAction::ReplaceSelectedText(value) => {
            ContentAction::ReplaceSelectedText(value)
        }
        // Disclosure is not a generic activation. View-specific Tree adapters
        // retain their original native contract until a Content role exists.
        AccessibilityAction::SetExpanded(_) => return ContentResponse::default(),
    };
    host.accessibility_action(id, target.part, action)
}
fn project(
    key: AccessibilityKey,
    parent: Option<AccessibilityKey>,
    frame: tekes_ui_scene::Rect,
    focused: bool,
    semantics: &ContentSemantics,
) -> AccessibilityNode {
    use AccessibilityRole as A;
    use ContentRole as R;
    AccessibilityNode {
        key,
        parent,
        role: match semantics.role {
            R::Group => A::Group,
            R::Text => A::StaticText,
            R::Button => A::Button,
            R::Link => A::Link,
            R::TextInput => A::TextField,
            R::Switch => A::Switch,
            R::List => A::List,
            R::ListItem => A::ListItem,
            R::Dialog => A::Dialog,
            R::MenuButton => A::MenuButton,
            R::Menu => A::Menu,
            R::MenuItem => A::MenuItem,
            _ => A::Group,
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

#[cfg(test)]
mod numeric_projection_tests {
    use super::*;
    use tekes_ui_controls::composition::{Content, DecimalNumberFormat, NumberFieldContent};
    #[test]
    fn formatted_numeric_editor_keeps_text_field_role_instead_of_becoming_slider() {
        let mut n = NumberFieldContent::new(
            "Percent",
            100.,
            50.,
            300.,
            10.,
            DecimalNumberFormat { suffix: "%".into() },
        )
        .unwrap();
        n.set_input_digits(3);
        let mut host = ContentHost::new(Content::new(ComponentId(1), n)).unwrap();
        let rect = tekes_ui_scene::Rect::new(0., 0., 180., 40.);
        host.render(
            &mut tekes_ui_scene::Scene::new(),
            &mut crate::text::Text::new(),
            rect,
            rect,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let nodes = content_nodes(&host, true);
        let editor = nodes.iter().find(|n| n.key.part.is_none()).unwrap();
        assert_eq!(editor.role, AccessibilityRole::TextField);
        assert_eq!(editor.value.as_deref(), Some("100%"));
        assert_eq!(
            nodes
                .iter()
                .filter(|n| n.role == AccessibilityRole::Button)
                .count(),
            2
        );
    }
}
