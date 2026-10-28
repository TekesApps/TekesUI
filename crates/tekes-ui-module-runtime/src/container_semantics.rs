//! Routing identities and a read-only projection of the original child tree.
//! No control, focus, text, selection or interaction state lives here.
use crate::{
    ProcessContentSnapshot,
    component::{Session, content as wire},
};
use anyhow::{Result, ensure};
use tekes_ui_controls::composition::{ContentRole, ContentSemantics, SemanticPart};
use tekes_ui_scene::Rect;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    target: wire::Target,
    part: Option<usize>,
}
#[derive(Debug)]
struct Entry {
    key: Key,
    index: usize,
    parent: Option<usize>,
    frame: [f32; 4],
    visible: Option<[f32; 4]>,
    focused: bool,
    semantics: wire::Semantics,
    preserves_focus: bool,
    text_binding: Option<Vec<u8>>,
    text_layout: Option<wire::TextLayout>,
    value_range: Option<[f64; 2]>,
}
#[derive(Debug, Default)]
pub(crate) struct RemoteSemantics {
    attachment: Option<(Session, u64)>,
    snapshot: Option<ProcessContentSnapshot>,
    entries: Vec<Entry>,
    next: usize,
    dirty: bool,
}
fn rect(r: [f32; 4]) -> Rect {
    Rect::new(r[0], r[1], r[2], r[3])
}
fn valid_rect(r: [f32; 4]) -> bool {
    r.iter().all(|v| v.is_finite())
        && r[2] >= 0.
        && r[3] >= 0.
        && (r[0] + r[2]).is_finite()
        && (r[1] + r[3]).is_finite()
}
pub(crate) fn validate(nodes: &[wire::SemanticNode]) -> Result<()> {
    let mut ids = std::collections::HashSet::new();
    for n in nodes {
        ensure!(
            n.target.id != 0 && n.target.mount != 0 && ids.insert(n.target.id),
            "invalid/duplicate child semantic identity"
        );
        ensure!(
            valid_rect(n.frame) && n.visible.is_none_or(valid_rect),
            "invalid child semantic geometry"
        );
        ensure!(
            n.semantics.role != wire::SemanticRole::Image
                || n.semantics.name.is_empty()
                || !n.semantics.name.trim().is_empty(),
            "blank child Image alternate text"
        );
        ensure!(
            n.semantics.role != wire::SemanticRole::Slider || n.value_range.is_some(),
            "missing child Slider range"
        );
        ensure!(
            n.value_range
                .is_none_or(|r| r.iter().all(|v| v.is_finite()) && r[0] <= r[1]),
            "invalid child semantic range"
        );
        ensure!(
            n.text_layout.as_ref().is_none_or(|t| t.is_valid()
                && t.input.owner == n.target
                && Some(&t.binding) == n.text_binding.as_ref()),
            "invalid child shaped text layout"
        );
        let mut parts = std::collections::HashSet::new();
        for p in &n.parts {
            ensure!(
                p.semantics.role != wire::SemanticRole::Image
                    || p.semantics.name.is_empty()
                    || !p.semantics.name.trim().is_empty(),
                "blank child Image part alternate text"
            );
            ensure!(
                p.semantics.role != wire::SemanticRole::Slider || p.value_range.is_some(),
                "missing child Slider part range"
            );
            ensure!(
                p.text_layout
                    .as_ref()
                    .is_none_or(|t| t.is_valid() && t.input.owner == n.target),
                "invalid child part shaped text layout"
            );
            ensure!(
                p.value_range
                    .is_none_or(|r| r.iter().all(|v| v.is_finite()) && r[0] <= r[1]),
                "invalid child part value range"
            );
            ensure!(
                parts.insert(p.index) && valid_rect(p.frame),
                "invalid child semantic part"
            );
        }
        for p in &n.parts {
            let mut parent = p.parent;
            let mut seen = std::collections::HashSet::from([p.index]);
            while let Some(i) = parent {
                ensure!(seen.insert(i), "cyclic child semantic part");
                parent = n
                    .parts
                    .iter()
                    .find(|p| p.index == i)
                    .ok_or_else(|| anyhow::anyhow!("missing child semantic parent part"))?
                    .parent;
            }
        }
    }
    for n in nodes {
        let mut parent = n.parent;
        let mut seen = std::collections::HashSet::from([n.target.id]);
        while let Some(t) = parent {
            ensure!(seen.insert(t.id), "cyclic child semantic hierarchy");
            parent = nodes
                .iter()
                .find(|n| n.target == t)
                .ok_or_else(|| anyhow::anyhow!("missing child semantic parent"))?
                .parent;
        }
    }
    Ok(())
}
fn semantics(s: &wire::Semantics) -> ContentSemantics {
    use ContentRole as R;
    use wire::SemanticRole as W;
    ContentSemantics {
        role: match s.role {
            W::Group => R::Group,
            W::Text => R::Text,
            W::Image => R::Image,
            W::Button => R::Button,
            W::Link => R::Link,
            W::TextInput => R::TextInput,
            W::Slider => R::Slider,
            W::Switch => R::Switch,
            W::List => R::List,
            W::ListItem => R::ListItem,
            W::Dialog => R::Dialog,
            W::MenuButton => R::MenuButton,
            W::Menu => R::Menu,
            W::MenuItem => R::MenuItem,
        },
        name: s.name.clone(),
        value: s.value.clone(),
        disabled: s.disabled,
        selected: s.selected,
        expanded: s.expanded,
        invalid: s.invalid,
        required: s.required,
        description: s.description.clone(),
    }
}
impl RemoteSemantics {
    pub fn dirty(&mut self) {
        self.dirty = true;
    }
    pub fn needs_refresh(&self) -> bool {
        self.dirty || self.snapshot.is_none()
    }
    /// Revoke publication without resetting the monotonic ID allocator.
    pub fn revoke(&mut self) {
        self.entries.clear();
        self.snapshot = None;
        self.attachment = None;
        self.dirty = true;
    }
    pub fn update(&mut self, snapshot: ProcessContentSnapshot) -> Result<()> {
        self.dirty = true;
        let nodes = snapshot
            .reply()
            .semantics
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("semantic query required"))?;
        if let Err(e) = validate(nodes) {
            self.revoke();
            return Err(e);
        }
        let attachment = snapshot.semantic_attachment();
        let same = self.attachment.as_ref() == Some(&attachment);
        let mut entries = Vec::new();
        let mut parents = Vec::new();
        for node in nodes {
            let key = Key {
                target: node.target,
                part: None,
            };
            for (
                key,
                parent,
                frame,
                visible,
                focused,
                s,
                preserve,
                binding,
                text_layout,
                value_range,
            ) in std::iter::once((
                key,
                node.parent.map(|target| Key { target, part: None }),
                node.frame,
                node.visible,
                node.focused,
                &node.semantics,
                node.preserves_activation_focus,
                node.text_binding.clone(),
                node.text_layout.clone(),
                node.value_range,
            ))
            .chain(node.parts.iter().map(|p| {
                (
                    Key {
                        target: node.target,
                        part: Some(p.index),
                    },
                    Some(Key {
                        target: node.target,
                        part: p.parent,
                    }),
                    p.frame,
                    Some(p.frame),
                    p.focused,
                    &p.semantics,
                    p.preserves_activation_focus,
                    p.text_layout.as_ref().map(|s| s.binding.clone()),
                    p.text_layout.clone(),
                    p.value_range,
                )
            })) {
                let index = if let Some(old) = same
                    .then(|| {
                        self.entries
                            .iter()
                            .find(|e| e.key == key && e.text_binding == binding)
                    })
                    .flatten()
                {
                    old.index
                } else {
                    self.next = self
                        .next
                        .checked_add(1)
                        .ok_or_else(|| anyhow::anyhow!("child semantic ID exhausted"))?;
                    self.next
                };
                entries.push(Entry {
                    key,
                    index,
                    parent: None,
                    frame,
                    visible,
                    focused,
                    semantics: s.clone(),
                    preserves_focus: preserve,
                    text_binding: binding,
                    text_layout,
                    value_range,
                });
                parents.push(parent);
            }
        }
        let parent_indices = parents
            .into_iter()
            .map(|key| key.and_then(|key| entries.iter().find(|e| e.key == key).map(|e| e.index)))
            .collect::<Vec<_>>();
        for (e, p) in entries.iter_mut().zip(parent_indices) {
            e.parent = p;
        }
        self.entries = entries;
        self.attachment = Some(attachment);
        self.snapshot = Some(snapshot);
        self.dirty = false;
        Ok(())
    }
    pub fn parts(&self, frame: Rect, clip: Rect, focused: bool) -> Vec<SemanticPart> {
        if self.needs_refresh() {
            return Vec::new();
        }
        self.entries
            .iter()
            .map(|e| {
                let raw = rect(e.frame).offset(frame.min_x(), frame.min_y());
                let bounds = e
                    .visible
                    .and_then(|r| {
                        rect(r)
                            .offset(frame.min_x(), frame.min_y())
                            .intersection(&raw)
                    })
                    .and_then(|r| r.intersection(&frame))
                    .and_then(|r| r.intersection(&clip));
                SemanticPart {
                    index: e.index,
                    frame: bounds
                        .unwrap_or_else(|| Rect::new(frame.min_x(), frame.min_y(), 0., 0.)),
                    focused: focused && e.focused,
                    semantics: semantics(&e.semantics),
                }
            })
            .collect()
    }
    pub fn parent(&self, index: usize) -> Option<usize> {
        (!self.needs_refresh())
            .then(|| self.entries.iter().find(|e| e.index == index)?.parent)
            .flatten()
    }
    pub fn preserves_focus(&self, index: usize) -> bool {
        !self.needs_refresh()
            && self
                .entries
                .iter()
                .any(|e| e.index == index && e.preserves_focus)
    }
    pub fn value_range(&self, index: usize) -> Option<std::ops::RangeInclusive<f64>> {
        if self.needs_refresh() {
            return None;
        }
        self.entries
            .iter()
            .find(|e| e.index == index)?
            .value_range
            .map(|r| r[0]..=r[1])
    }
    pub fn text_layout(
        &self,
        index: usize,
        frame: Rect,
        clip: Rect,
    ) -> Option<tekes_ui_controls::composition::ContentTextLayout> {
        if self.needs_refresh() {
            return None;
        }
        let t = self
            .entries
            .iter()
            .find(|e| e.index == index)?
            .text_layout
            .as_ref()?;
        let offset = |r| rect(r).offset(frame.min_x(), frame.min_y());
        let visible = |r| {
            offset(r)
                .intersection(&frame)
                .and_then(|r| r.intersection(&clip))
                .filter(|r| r.size.height > 0. && r.size.width > 0.)
        };
        Some(tekes_ui_controls::composition::ContentTextLayout {
            input: tekes_ui_controls::composition::ContentTextInput {
                binding: t.binding.clone(),
                text: t.input.text.clone(),
                caret_index: t.input.caret_index,
                selection: t.input.selection.clone(),
                marked: t.input.marked.clone(),
                read_only: t.input.read_only,
                caret: visible(t.input.caret),
                clip: visible(t.input.clip),
                carets: t
                    .input
                    .carets
                    .iter()
                    .filter_map(|(i, r)| visible(*r).map(|r| (*i, r)))
                    .collect(),
            },
            multiline: t.multiline,
            lines: t.lines.clone(),
            carets: t.carets.iter().map(|(i, r)| (*i, offset(*r))).collect(),
        })
    }
    pub fn callback(
        &self,
        index: usize,
    ) -> Option<(ProcessContentSnapshot, wire::Target, Option<usize>)> {
        if self.needs_refresh() {
            return None;
        }
        let e = self.entries.iter().find(|e| e.index == index)?;
        Some((self.snapshot.clone()?, e.key.target, e.key.part))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: u64, parent: Option<u64>) -> wire::SemanticNode {
        wire::SemanticNode {
            text_layout: None,
            target: wire::Target {
                id,
                mount: id + 100,
            },
            parent: parent.map(|id| wire::Target {
                id,
                mount: id + 100,
            }),
            frame: [0., 0., 100., 50.],
            visible: Some([0., 0., 100., 50.]),
            focused: false,
            semantics: wire::Semantics {
                role: wire::SemanticRole::Group,
                name: "node".into(),
                value: None,
                disabled: false,
                selected: false,
                expanded: None,
                invalid: false,
                required: false,
                description: None,
            },
            preserves_activation_focus: false,
            parts: vec![],
            value_range: None,
            text_binding: None,
            text: None,
        }
    }
    #[test]
    fn semantic_hierarchy_rejects_missing_cyclic_or_duplicate_identity() {
        assert!(validate(&[node(1, None), node(2, Some(1))]).is_ok());
        assert!(validate(&[node(1, Some(2)), node(2, Some(1))]).is_err());
        assert!(validate(&[node(1, Some(2))]).is_err());
        assert!(validate(&[node(1, None), node(1, None)]).is_err());
        assert!(validate(&[node(0, None)]).is_err());
    }
    #[test]
    fn semantic_parts_reject_unknown_parent_and_nonfinite_geometry() {
        let mut n = node(1, None);
        n.parts.push(wire::SemanticPart {
            text_layout: None,
            value_range: None,
            index: 7,
            parent: Some(8),
            frame: [0., 0., 10., 10.],
            focused: false,
            semantics: n.semantics.clone(),
            preserves_activation_focus: false,
        });
        assert!(validate(&[n.clone()]).is_err());
        n.parts[0].parent = Some(7);
        assert!(validate(&[n.clone()]).is_err());
        n.parts[0].parent = None;
        n.parts[0].frame[0] = f32::NAN;
        assert!(validate(&[n.clone()]).is_err());
        n.parts.clear();
        n.value_range = Some([3., 1.]);
        assert!(validate(&[n]).is_err());
    }
    #[test]
    fn image_alt_and_slider_domains_fail_closed_before_retaining_any_prefix() {
        let prefix = node(1, None);
        let mut image = node(2, None);
        image.semantics.role = wire::SemanticRole::Image;
        for name in ["Informative image", ""] {
            image.semantics.name = name.into();
            assert!(validate(&[prefix.clone(), image.clone()]).is_ok());
            let projected = semantics(&image.semantics);
            assert_eq!(projected.role, ContentRole::Image);
            assert_eq!(projected.name, name);
        }
        image.semantics.name = " \n".into();
        assert!(validate(&[prefix.clone(), image]).is_err());
        let mut slider = node(2, None);
        slider.semantics.role = wire::SemanticRole::Slider;
        assert!(validate(&[prefix.clone(), slider.clone()]).is_err());
        for range in [[f64::NAN, 1.], [0., f64::INFINITY], [2., 1.]] {
            slider.value_range = Some(range);
            assert!(validate(&[prefix.clone(), slider.clone()]).is_err());
        }
        slider.value_range = Some([-40., 160.]);
        assert!(validate(&[prefix, slider]).is_ok());
    }
}
