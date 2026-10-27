//! Minimal caller-owned authority and delayed response, over the public editor.
//! Journals own versions; TextSurface owns editing. No Session or transport.
use std::collections::BTreeMap;
use tekes_ui_controls::{
    DraftJournal, DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult,
    TextEditBaseline, TextSurface,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(9100);
pub const EDITOR: ComponentId = ComponentId(9101);
pub const CLEAR: ComponentId = ComponentId(9102);
pub const CAPTURE: ComponentId = ComponentId(9103);
pub const APPLY: ComponentId = ComponentId(9104);
pub const SWITCH: ComponentId = ComponentId(9105);
pub const STATUS: ComponentId = ComponentId(9106);
pub const PREFIX: ComponentId = ComponentId(9107);
#[derive(Debug)]
pub struct ControlledEditorFixture {
    pub host: ContentHost,
    journals: BTreeMap<&'static str, DraftJournal>,
    key: &'static str,
    captured: Option<(DraftRevision, TextEditBaseline)>,
    counts: [usize; 5],
    pub last_result: Option<ExternalValueResult>,
}
impl Default for ControlledEditorFixture {
    fn default() -> Self {
        Self::new()
    }
}
impl ControlledEditorFixture {
    pub fn new() -> Self {
        let journals = BTreeMap::from([
            (
                "A",
                DraftJournal::new("A", "A 中文😀 e\u{301} draft").unwrap(),
            ),
            ("B", DraftJournal::new("B", "B 草稿").unwrap()),
        ]);
        let mut field = TextFieldContent::new("受控输入", "");
        field.surface = TextSurface::new("", true, false);
        let baseline = field.surface.input.external_baseline();
        field.surface.apply_external_value(
            &baseline,
            journals["A"].current(),
            ExternalSelection::Reset,
            ExternalUndo::Reset,
        );
        let children = vec![
            Content::new(EDITOR, field),
            Content::new(CLEAR, Button::new("清空事务")),
            Content::new(CAPTURE, Button::new("捕获清空")),
            Content::new(APPLY, Button::new("应用捕获")),
            Content::new(SWITCH, Button::new("切换文档")),
            Content::new(PREFIX, Button::new("前插并映射选区")),
            Content::new(STATUS, TextContent::new("")),
        ];
        let host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: None,
                children,
            },
        ))
        .unwrap();
        let mut result = Self {
            host,
            journals,
            key: "A",
            captured: None,
            counts: [0; 5],
            last_result: None,
        };
        result.host.focus(Some(EDITOR), true);
        result.status();
        result
    }
    pub fn editor(&self) -> &TextSurface {
        &self
            .host
            .get(EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
    }
    fn editor_mut(&mut self) -> &mut TextSurface {
        &mut self
            .host
            .get_mut(EDITOR)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
    }
    pub fn authority(&self) -> &DraftRevision {
        self.journals[self.key].current()
    }
    pub fn captured_value(&self) -> Option<&DraftRevision> {
        self.captured.as_ref().map(|(v, _)| v)
    }
    fn capture_clear(&mut self) {
        let baseline = self.editor().input.external_baseline();
        self.journals
            .get_mut(self.key)
            .unwrap()
            .edit("", |_| Ok(()))
            .unwrap();
        self.captured = Some((self.authority().clone(), baseline));
    }
    fn apply_captured(&mut self) {
        if let Some((value, baseline)) = self.captured.clone() {
            self.last_result = Some(self.editor_mut().apply_external_value(
                &baseline,
                &value,
                ExternalSelection::Reset,
                ExternalUndo::Transaction,
            ));
        }
    }
    fn status(&mut self) {
        let text = format!(
            "文档 {} · revision {} · {:?}",
            self.key,
            self.authority().version,
            self.last_result
        );
        self.host
            .get_mut(STATUS)
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = text;
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let before = self.editor().input.text.clone();
        let marked = self.editor().input.marked.is_some();
        let mut response = self.host.event(event);
        if self.editor().input.marked.is_none() && (self.editor().input.text != before || marked) {
            let text = self.editor().input.text.clone();
            self.journals
                .get_mut(self.key)
                .unwrap()
                .edit(text, |_| Ok(()))
                .unwrap();
            let value = self.authority().clone();
            let baseline = self.editor().input.external_baseline();
            self.last_result = Some(self.editor_mut().apply_external_value(
                &baseline,
                &value,
                ExternalSelection::Preserve,
                ExternalUndo::Preserve,
            ));
        }
        let counts = [CLEAR, CAPTURE, APPLY, SWITCH, PREFIX].map(|id| {
            self.host
                .get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations
        });
        let old = self.counts;
        self.counts = counts;
        if counts[0] > old[0] {
            self.capture_clear();
            self.apply_captured();
        }
        if counts[1] > old[1] {
            self.capture_clear();
        }
        if counts[2] > old[2] {
            self.apply_captured();
        }
        if counts[3] > old[3] {
            self.key = if self.key == "A" { "B" } else { "A" };
            let value = self.authority().clone();
            let baseline = self.editor().input.external_baseline();
            self.last_result = Some(self.editor_mut().apply_external_value(
                &baseline,
                &value,
                ExternalSelection::Reset,
                ExternalUndo::Reset,
            ));
        }
        if counts[4] > old[4] {
            let baseline = self.editor().input.external_baseline();
            let text = format!("前{}", self.editor().input.text);
            self.journals
                .get_mut(self.key)
                .unwrap()
                .edit(text, |_| Ok(()))
                .unwrap();
            let value = self.authority().clone();
            self.last_result = Some(self.editor_mut().apply_external_value(
                &baseline,
                &value,
                ExternalSelection::Preserve,
                ExternalUndo::Preserve,
            ));
        }
        if counts != old {
            self.host.focus(Some(EDITOR), true);
            response.changed = true;
        }
        self.status();
        response
    }
}
