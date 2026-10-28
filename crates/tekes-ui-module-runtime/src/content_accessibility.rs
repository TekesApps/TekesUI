//! Read/project the original active Content tree; execute actions on its owner.
use anyhow::{Result, ensure};
use tekes_ui_app_protocol::component::content as wire;
use tekes_ui_controls::composition::{
    AccessibilityAction, ComponentId, ContentHost, ContentResponse, ContentRole, ContentSemantics,
};
use tekes_ui_scene::Rect;
fn rect(r: Rect) -> [f32; 4] {
    [r.min_x(), r.min_y(), r.size.width, r.size.height]
}
fn semantics(s: ContentSemantics) -> Result<wire::Semantics> {
    use wire::SemanticRole as W;
    let role = match s.role {
        ContentRole::Group => W::Group,
        ContentRole::Text => W::Text,
        ContentRole::Image => {
            ensure!(
                s.name.is_empty() || !s.name.trim().is_empty(),
                "Image alternate text must not be blank"
            );
            W::Image
        }
        ContentRole::Button => W::Button,
        ContentRole::Link => W::Link,
        ContentRole::TextInput => W::TextInput,
        ContentRole::Slider => W::Slider,
        ContentRole::Splitter => W::Splitter,
        ContentRole::Switch => W::Switch,
        ContentRole::List => W::List,
        ContentRole::ListItem => W::ListItem,
        ContentRole::Dialog => W::Dialog,
        ContentRole::MenuButton => W::MenuButton,
        ContentRole::Menu => W::Menu,
        ContentRole::MenuItem => W::MenuItem,
        unsupported => anyhow::bail!("unsupported Content semantic role: {unsupported:?}"),
    };
    Ok(wire::Semantics {
        role,
        name: s.name,
        value: s.value,
        disabled: s.disabled,
        selected: s.selected,
        expanded: s.expanded,
        invalid: s.invalid,
        required: s.required,
        description: s.description,
    })
}
fn text_layout(
    owner: wire::Target,
    layout: tekes_ui_controls::composition::ContentTextLayout,
) -> wire::TextLayout {
    let s = layout.input;
    wire::TextLayout {
        binding: s.binding,
        input: wire::TextState {
            owner,
            text: s.text,
            caret_index: s.caret_index,
            selection: s.selection,
            marked: s.marked,
            read_only: s.read_only,
            caret: rect(s.caret.unwrap_or_default()),
            clip: rect(s.clip.unwrap_or_default()),
            carets: s.carets.into_iter().map(|(i, r)| (i, rect(r))).collect(),
        },
        multiline: layout.multiline,
        lines: layout.lines,
        carets: layout
            .carets
            .into_iter()
            .map(|(i, r)| (i, rect(r)))
            .collect(),
    }
}
pub(crate) fn snapshot(host: &ContentHost) -> Result<Vec<wire::SemanticNode>> {
    if !host.is_window_active() {
        return Ok(Vec::new());
    }
    let target = |id: ComponentId| {
        host.semantic_mount(id)
            .map(|mount| wire::Target { id: id.0, mount })
    };
    let mut nodes = Vec::new();
    for node in host.semantics() {
        let Some(owner) = target(node.id) else {
            continue;
        };
        let input = host.semantic_text_input_snapshot(node.id);
        let text_binding = input.as_ref().map(|s| s.binding.clone());
        let text = input.map(|s| wire::TextState {
            owner,
            text: s.text,
            caret_index: s.caret_index,
            selection: s.selection,
            marked: s.marked,
            read_only: s.read_only,
            caret: rect(s.caret.unwrap_or_default()),
            clip: rect(s.clip.unwrap_or_default()),
            carets: s.carets.into_iter().map(|(i, r)| (i, rect(r))).collect(),
        });
        nodes.push(wire::SemanticNode {
            text_layout: host
                .semantic_text_layout_snapshot(node.id, None)
                .map(|s| text_layout(owner, s)),
            target: owner,
            parent: node.parent.and_then(target),
            frame: rect(node.frame),
            visible: node.visible.map(rect),
            focused: node.focused,
            semantics: semantics(node.semantics)?,
            preserves_activation_focus: host.semantic_preserves_activation_focus(node.id, None),
            parts: node
                .parts
                .into_iter()
                .map(|p| {
                    Ok(wire::SemanticPart {
                        text_layout: host
                            .semantic_text_layout_snapshot(node.id, Some(p.index))
                            .map(|s| text_layout(owner, s)),
                        value_range: host
                            .semantic_value_range(node.id, Some(p.index))
                            .map(|r| [*r.start(), *r.end()]),
                        index: p.index,
                        parent: host.semantic_part_parent(node.id, p.index),
                        frame: rect(p.frame),
                        focused: p.focused,
                        semantics: semantics(p.semantics)?,
                        preserves_activation_focus: host
                            .semantic_preserves_activation_focus(node.id, Some(p.index)),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            value_range: host.value_range(node.id).map(|r| [*r.start(), *r.end()]),
            text_binding,
            text,
        });
    }
    crate::container_semantics::validate(&nodes)?;
    // Leave room for the outer identity/error envelope. Reject an oversized
    // query as a result, never terminate the original actor writing a packet.
    ensure!(
        serde_json::to_vec(&nodes)?.len() <= crate::pipes::MAX_PACKET / 2,
        "Content semantic snapshot exceeds transport budget"
    );
    Ok(nodes)
}
pub(crate) fn action(
    host: &mut ContentHost,
    target: wire::Target,
    part: Option<usize>,
    binding: Option<&[u8]>,
    action: &wire::AccessibilityAction,
) -> Result<ContentResponse> {
    ensure!(action.is_valid(), "invalid Content accessibility action");
    let id = ComponentId(target.id);
    ensure!(
        host.is_window_active() && host.semantic_mount(id) == Some(target.mount),
        "stale Content semantic mount or inactive window"
    );
    let node = host
        .semantics()
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| anyhow::anyhow!("unavailable Content semantic owner"))?;
    ensure!(
        part.is_none_or(|i| node.parts.iter().any(|p| p.index == i)),
        "stale Content semantic part"
    );
    let current_binding = if part.is_some() {
        host.semantic_text_layout_snapshot(id, part)
            .map(|s| s.input.binding)
    } else {
        host.semantic_text_input_snapshot(id).map(|s| s.binding)
    };
    ensure!(
        current_binding.as_deref() == binding,
        "stale Content semantic text binding"
    );
    let action = match action {
        wire::AccessibilityAction::Increment => AccessibilityAction::Increment,
        wire::AccessibilityAction::Decrement => AccessibilityAction::Decrement,
        wire::AccessibilityAction::Focus => AccessibilityAction::Focus,
        wire::AccessibilityAction::Press => AccessibilityAction::Press,
        wire::AccessibilityAction::SetValue(v) => AccessibilityAction::SetValue(v.clone()),
        wire::AccessibilityAction::SetSelection(v) => AccessibilityAction::SetSelection(v.clone()),
        wire::AccessibilityAction::ReplaceSelectedText(v) => {
            AccessibilityAction::ReplaceSelectedText(v.clone())
        }
    };
    Ok(host.accessibility_action(id, part, action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::{Button, ButtonTextAlign, ButtonTextRenderer, composition::*};
    use tekes_ui_scene::{Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new((s.len() as f32 * 8.).min(w), 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    fn setup() -> ContentHost {
        let mut h = ContentHost::new(Content::new(
            ComponentId(1),
            ListContent::new(
                "Actions",
                vec![Content::new(ComponentId(2), Button::new("Action"))],
            ),
        ))
        .unwrap();
        render(&mut h);
        h
    }
    fn render(h: &mut ContentHost) {
        let f = Rect::new(0., 0., 300., 200.);
        h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
            .unwrap();
    }
    #[test]
    fn semantic_query_keeps_focus_capture_and_pending_paint() {
        let mut h = setup();
        h.focus(Some(ComponentId(2)), true);
        let before = (
            h.focused(),
            h.focus_epoch(),
            h.pointer_capture(),
            h.needs_render(),
        );
        let req = wire::Request {
            view: 1,
            viewport: 1,
            owner: None,
            focus_epoch: 0,
            text_binding: None,
            capture: None,
            input: wire::Input::Semantics,
        };
        let r = crate::content_input::dispatch(&mut h, &mut Text, req).unwrap();
        assert!(!r.changed && !r.consumed);
        assert_eq!(r.semantics.unwrap().len(), 2);
        assert_eq!(
            before,
            (
                h.focused(),
                h.focus_epoch(),
                h.pointer_capture(),
                h.needs_render()
            )
        );
    }
    #[test]
    fn removed_mount_and_late_part_cannot_activate_replacement() {
        let mut h = setup();
        let old = snapshot(&h)
            .unwrap()
            .into_iter()
            .find(|n| n.target.id == 2)
            .unwrap()
            .target;
        h.remove(ComponentId(2)).unwrap();
        h.insert(
            ComponentId(1),
            Content::new(ComponentId(2), Button::new("Replacement")),
        )
        .unwrap();
        render(&mut h);
        assert!(action(&mut h, old, None, None, &wire::AccessibilityAction::Press).is_err());
        let new = snapshot(&h)
            .unwrap()
            .into_iter()
            .find(|n| n.target.id == 2)
            .unwrap()
            .target;
        assert_ne!(old.mount, new.mount);
        assert!(
            action(
                &mut h,
                new,
                Some(17),
                None,
                &wire::AccessibilityAction::Press
            )
            .is_err()
        );
        assert!(
            action(&mut h, new, None, None, &wire::AccessibilityAction::Press)
                .unwrap()
                .activated
        );
    }
    #[derive(Debug)]
    struct Gate {
        active: std::rc::Rc<std::cell::Cell<bool>>,
        children: Vec<Content>,
    }
    impl Component for Gate {
        fn active(&self) -> bool {
            self.active.get()
        }
        fn measure(
            &mut self,
            t: &mut dyn ButtonTextRenderer,
            a: Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            self.children[0].measure(t, a)
        }
        fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
            self.children[0].arrange(t, f, c)
        }
        fn children(&self) -> &[Content] {
            &self.children
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.children)
        }
    }
    #[test]
    fn inactive_and_hidden_scope_revoke_semantics_and_actions() {
        let visible = std::rc::Rc::new(std::cell::Cell::new(true));
        let mut h = ContentHost::new(Content::new(
            ComponentId(1),
            Gate {
                active: visible.clone(),
                children: vec![Content::new(ComponentId(2), Button::new("Action"))],
            },
        ))
        .unwrap();
        render(&mut h);
        let old = snapshot(&h).unwrap()[1].target;
        h.set_window_active(false);
        assert!(snapshot(&h).unwrap().is_empty());
        assert!(action(&mut h, old, None, None, &wire::AccessibilityAction::Press).is_err());
        h.set_window_active(true);
        visible.set(false);
        render(&mut h);
        assert!(snapshot(&h).unwrap().iter().all(|n| n.target.id != 2));
        assert!(action(&mut h, old, None, None, &wire::AccessibilityAction::Press).is_err());
    }
    #[derive(Debug)]
    struct ImageSemantics(String);
    impl Component for ImageSemantics {
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            _: Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(Size::new(20., 10.))
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::new(ContentRole::Image, self.0.clone())
        }
    }
    #[test]
    fn image_snapshot_retains_alt_or_decorative_semantics_without_action_or_resource_transport() {
        for name in ["Informative image", ""] {
            let mut host =
                ContentHost::new(Content::new(ComponentId(71), ImageSemantics(name.into())))
                    .unwrap();
            render(&mut host);
            let nodes = snapshot(&host).unwrap();
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].semantics.role, wire::SemanticRole::Image);
            assert_eq!(nodes[0].semantics.name, name);
            let encoded = serde_json::to_value(&nodes).unwrap();
            assert!(encoded[0].get("image_id").is_none());
            assert!(
                !action(
                    &mut host,
                    nodes[0].target,
                    None,
                    None,
                    &wire::AccessibilityAction::Press
                )
                .unwrap()
                .consumed
            );
        }
        assert!(semantics(ContentSemantics::new(ContentRole::Image, " \n")).is_err());
    }
    #[test]
    fn original_splitter_snapshot_and_action_keep_the_real_finite_domain_on_wire() {
        let mut host = ContentHost::try_new_prepared(
            Content::new(
                ComponentId(81),
                SplitViewContent::try_new(SplitViewConstruction {
                    config: SplitViewConfig {
                        min_pane: 10.,
                        ..Default::default()
                    },
                    slots: SplitViewSlotIds {
                        leading: ComponentId(82),
                        trailing: ComponentId(83),
                        divider: ComponentId(84),
                    },
                    leading: Content::new(ComponentId(85), Button::new("leading")),
                    trailing: Content::new(ComponentId(86), Button::new("trailing")),
                    budget: 64 * 1024 * 1024,
                })
                .unwrap(),
            ),
            64 * 1024 * 1024,
        )
        .unwrap();
        render(&mut host);
        let nodes = snapshot(&host).unwrap();
        let divider = nodes.iter().find(|node| node.target.id == 84).unwrap();
        assert_eq!(divider.semantics.role, wire::SemanticRole::Splitter);
        let domain = divider.value_range.unwrap();
        assert!(domain.iter().all(|value| value.is_finite()));
        assert!(0. < domain[0] && domain[0] <= 0.5 && 0.5 <= domain[1] && domain[1] < 1.);
        let bytes = serde_json::to_vec(&nodes).unwrap();
        let received: Vec<wire::SemanticNode> = serde_json::from_slice(&bytes).unwrap();
        crate::container_semantics::validate(&received).unwrap();
        assert_eq!(
            received
                .iter()
                .find(|node| node.target.id == 84)
                .unwrap()
                .value_range,
            Some(domain)
        );
        assert!(
            action(
                &mut host,
                divider.target,
                None,
                None,
                &wire::AccessibilityAction::Increment
            )
            .unwrap()
            .consumed
        );
        assert!(
            host.get(ComponentId(81))
                .unwrap()
                .component::<SplitViewContent>()
                .unwrap()
                .config()
                .unwrap()
                .ratio
                > 0.5
        );
        assert!(
            action(
                &mut host,
                divider.target,
                None,
                None,
                &wire::AccessibilityAction::Decrement
            )
            .unwrap()
            .consumed
        );
        assert!(
            (host
                .get(ComponentId(81))
                .unwrap()
                .component::<SplitViewContent>()
                .unwrap()
                .config()
                .unwrap()
                .ratio
                - 0.5)
                .abs()
                <= f32::EPSILON
        );
        let minimum = domain[0].to_string();
        assert!(
            action(
                &mut host,
                divider.target,
                None,
                None,
                &wire::AccessibilityAction::SetValue(minimum.clone())
            )
            .unwrap()
            .consumed
        );
        assert_eq!(
            snapshot(&host)
                .unwrap()
                .iter()
                .find(|node| node.target.id == 84)
                .unwrap()
                .semantics
                .value
                .as_deref(),
            Some(minimum.as_str())
        );
    }
}
