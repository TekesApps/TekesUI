// The exact controls-number-field-20261022 pin exports this control as ButtonContent.
use crate::{Draft, FieldType, Schema};
#[path = "form_row.rs"]
mod form_row;
use anyhow::Result;
use tekes_ui_controls::{
    ButtonTextRenderer, ScrollArea, TextSurface, TextSurfaceAlignment, TextSurfacePresentation,
    composition::{
        ButtonContent as Button, ComponentId, Content, ContentActionPayload, ContentEvent,
        ContentHost, DecimalNumberFormat, MenuContent, MenuEntry, NumberCommitReason,
        NumberFieldContent, NumberValidation, ScrollContent, StackContent, SwitchContent,
        TextContent, TextFieldContent,
    },
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Palette;
const ROOT: ComponentId = ComponentId(1);
const STATUS: ComponentId = ComponentId(2);
const RESET: ComponentId = ComponentId(3);
const CANCEL: ComponentId = ComponentId(4);
const APPLY: ComponentId = ComponentId(5);
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormIntent {
    Apply,
    Cancel,
    Reset,
}
/// One persistent component tree. Drafts contain values; TextField owns editing/IME.
#[derive(Debug)]
pub struct SettingsForm {
    pub host: ContentHost,
    field_ids: Vec<ComponentId>,
    autosave: bool,
    save_error: Option<(String, bool)>,
}
fn stack(id: u64, axis: Axis, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent {
            model: Stack { axis, gap: 12. },
            children,
            flexible_child: None,
        },
    )
}
impl SettingsForm {
    pub fn new(schema: &Schema, draft: &Draft) -> Result<Self> {
        Self::build(schema, draft, false)
    }
    /// Autosave consumers have no persistent footer or draft-action buttons.
    pub fn new_autosave(schema: &Schema, draft: &Draft) -> Result<Self> {
        Self::build(schema, draft, true)
    }
    fn build(schema: &Schema, draft: &Draft, autosave: bool) -> Result<Self> {
        let ids: Vec<_> = (0..schema.settings.len())
            .map(|i| ComponentId(100 + i as u64))
            .collect();
        fn groups(
            schema: &Schema,
            draft: &Draft,
            ids: &[ComponentId],
            parent: Option<&str>,
        ) -> Vec<Content> {
            let mut children = vec![];
            for group in schema
                .groups
                .iter()
                .filter(|g| g.parent.as_deref() == parent)
            {
                for (index, f) in schema
                    .settings
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.group == group.id)
                {
                    let raw = draft.display(f);
                    let control = match f.kind {
                        FieldType::Boolean => {
                            Content::new(ids[index], SwitchContent::new("", raw == "true"))
                        }
                        FieldType::Enum => {
                            let selected = f
                                .options
                                .iter()
                                .find(|o| o.value == raw)
                                .map_or(raw.as_str(), |o| o.label.as_str());
                            let mut menu = MenuContent::new(selected, vec![]);
                            menu.set_entries(
                                f.options
                                    .iter()
                                    .map(|o| MenuEntry::item(&o.value, &o.label))
                                    .collect(),
                            )
                            .expect("validated enum");
                            Content::new(ids[index], menu)
                        }
                        FieldType::Integer | FieldType::Decimal
                            if f.step.is_some() || !f.suffix.is_empty() =>
                        {
                            let mut number = NumberFieldContent::new(
                                &f.label,
                                raw.parse().expect("validated numeric value"),
                                f.min.unwrap(),
                                f.max.unwrap(),
                                f.step.unwrap_or(1.),
                                DecimalNumberFormat {
                                    suffix: f.suffix.clone(),
                                },
                            )
                            .expect("validated numeric configuration");
                            number.set_input_digits(3);
                            number.set_alignment(TextSurfaceAlignment::Trailing);
                            number.set_borderless(true);
                            Content::new(ids[index], number)
                        }
                        _ => {
                            let mut field = TextFieldContent::new("", &raw);
                            field.surface.presentation = TextSurfacePresentation::Content;
                            field.surface.alignment = TextSurfaceAlignment::Trailing;
                            Content::new(ids[index], field)
                        }
                    };
                    let base = (1_u64 << 32) + index as u64 * 4;
                    // Shared help owns hover timing and accessibility descriptions;
                    // descriptions never create permanent form rows.
                    let labels = vec![
                        Content::new(ComponentId(base + 1), TextContent::new(&f.label))
                            .with_help(&f.description)
                            .with_label_layout(Default::default())
                            .expect("Text label supports wrapping"),
                    ];
                    children.push(Content::new(
                        ComponentId(base),
                        form_row::SettingsRow::new(
                            &f.label,
                            stack(base + 3, Axis::Vertical, labels),
                            control.with_help(&f.description),
                            f.kind == FieldType::Boolean,
                        ),
                    ));
                }
                children.extend(groups(schema, draft, ids, Some(&group.id)));
            }
            children
        }
        let children = groups(schema, draft, &ids, None);
        let mut scroll =
            ScrollContent::new(ScrollArea::default(), stack(20, Axis::Vertical, children));
        scroll.viewport_height = Some(390.);
        let mut root_children = vec![Content::new(ComponentId(11), scroll)];
        if !autosave {
            root_children.extend([
                Content::new(STATUS, TextContent::new("No changes")),
                stack(
                    12,
                    Axis::Horizontal,
                    vec![
                        Content::new(RESET, Button::new("Reset")),
                        Content::new(CANCEL, Button::new("Cancel")),
                        Content::new(APPLY, Button::new("Apply")),
                    ],
                ),
            ]);
        }
        let mut root = stack(ROOT.0, Axis::Vertical, root_children);
        // The body alone shrinks on small windows; footer remains reachable.
        root.component_mut::<StackContent>().unwrap().flexible_child = Some(ComponentId(11));
        let mut host =
            ContentHost::new(root).map_err(|e| anyhow::anyhow!("Settings form: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            field_ids: ids,
            autosave,
            save_error: None,
        })
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        palette: Palette,
    ) -> Result<()> {
        self.host
            .render(scene, text, frame, frame, palette)
            .map_err(|e| anyhow::anyhow!("Settings render: {e:?}"))
    }
    pub fn event(
        &mut self,
        event: &ContentEvent,
        text: &mut dyn ButtonTextRenderer,
        schema: &Schema,
        draft: &mut Draft,
    ) -> Result<Vec<FormIntent>> {
        self.host
            .event_with_text(event, text)
            .map_err(|e| anyhow::anyhow!("Settings input: {e:?}"))?;
        Ok(self.collect(schema, draft))
    }
    /// Also used after native accessibility actions; never replays pointer events.
    pub fn collect(&mut self, schema: &Schema, draft: &mut Draft) -> Vec<FormIntent> {
        let records = self.host.drain_action_records();
        for record in &records {
            if let Some(ContentActionPayload::MenuSelection(selection)) = &record.payload
                && let Some(index) = self
                    .field_ids
                    .iter()
                    .position(|id| *id == record.action.target)
                && let Some(value) = selection.value_path().and_then(|p| p.first())
            {
                draft.edit(&schema.settings[index], value.clone());
                let f = &schema.settings[index];
                let label = f
                    .options
                    .iter()
                    .find(|o| &o.value == value)
                    .map_or(value.as_str(), |o| o.label.as_str());
                self.host
                    .get_mut(self.field_ids[index])
                    .unwrap()
                    .component_mut::<MenuContent>()
                    .unwrap()
                    .trigger
                    .label = label.into();
            }
        }
        self.collect_number_changes(schema, draft);
        for (index, f) in schema.settings.iter().enumerate() {
            let node = self.host.get(self.field_ids[index]).unwrap();
            let raw = if let Some(field) = node.component::<TextFieldContent>() {
                Some(field.surface.input.text.clone())
            } else {
                node.component::<SwitchContent>()
                    .map(|v| v.model.checked.to_string())
            };
            if let Some(raw) = raw {
                draft.edit(f, raw);
            }
        }
        records
            .into_iter()
            .filter_map(|r| match r.action.target {
                APPLY => Some(FormIntent::Apply),
                CANCEL => Some(FormIntent::Cancel),
                RESET => Some(FormIntent::Reset),
                _ => None,
            })
            .collect()
    }
    pub fn collect_number_changes(&mut self, schema: &Schema, draft: &mut Draft) {
        for (index, id) in self.field_ids.iter().enumerate() {
            if let Some(number) = self
                .host
                .get_mut(*id)
                .unwrap()
                .component_mut::<NumberFieldContent>()
            {
                for change in number.drain_value_changes() {
                    draft.edit(&schema.settings[index], change.value.to_string());
                }
            }
        }
    }
    pub fn has_number_draft(&self) -> bool {
        self.field_ids.iter().any(|id| {
            self.host
                .get(*id)
                .unwrap()
                .component::<NumberFieldContent>()
                .is_some_and(|n| n.edit_state().dirty || n.edit_state().composing)
        })
    }
    pub fn number_editing(&self) -> bool {
        self.host
            .focused()
            .and_then(|id| self.host.get(id))
            .and_then(|n| n.component::<NumberFieldContent>())
            .is_some_and(|n| n.edit_state().dirty)
    }
    /// Ask original numeric controls to commit valid drafts; never parse editor text here.
    pub fn flush_numbers(&mut self, schema: &Schema, draft: &mut Draft) -> bool {
        let mut valid = true;
        for (index, id) in self.field_ids.iter().enumerate() {
            if let Some(number) = self
                .host
                .get_mut(*id)
                .unwrap()
                .component_mut::<NumberFieldContent>()
            {
                number.commit(NumberCommitReason::Blur);
                valid &= !number.edit_state().composing
                    && matches!(number.validation(), NumberValidation::Valid(_));
                for change in number.drain_value_changes() {
                    draft.edit(&schema.settings[index], change.value.to_string());
                }
            }
        }
        valid
    }
    pub fn has_open_menu(&self) -> bool {
        self.field_ids.iter().any(|id| {
            self.host
                .get(*id)
                .and_then(|n| n.component::<MenuContent>())
                .is_some_and(|m| m.menu.is_open())
        })
    }
    pub fn replace_values(&mut self, schema: &Schema, draft: &Draft) {
        self.host.cancel();
        self.host.focus(None, false);
        for (index, f) in schema.settings.iter().enumerate() {
            let raw = draft.display(f);
            let node = self.host.get_mut(self.field_ids[index]).unwrap();
            if let Some(field) = node.component_mut::<TextFieldContent>() {
                field.surface = TextSurface::new(&raw, false, false);
                field.surface.presentation = TextSurfacePresentation::Content;
                field.surface.alignment = TextSurfaceAlignment::Trailing;
            } else if let Some(number) = node.component_mut::<NumberFieldContent>() {
                // Preserve a genuinely newer in-progress edit during history reconciliation.
                let _ = number.set_value(raw.parse().expect("validated numeric snapshot"));
            } else if let Some(switch) = node.component_mut::<SwitchContent>() {
                switch.model.checked = raw == "true";
            } else if let Some(menu) = node.component_mut::<MenuContent>() {
                let label = f
                    .options
                    .iter()
                    .find(|o| o.value == raw)
                    .map_or(raw.as_str(), |o| o.label.as_str());
                *menu = MenuContent::new(label, vec![]);
                menu.set_entries(
                    f.options
                        .iter()
                        .map(|o| MenuEntry::item(&o.value, &o.label))
                        .collect(),
                )
                .expect("validated enum");
            }
        }
    }
    /// A transient error/retry row appears only when saving needs attention.
    pub fn save_error(&mut self, message: Option<&str>, busy: bool) {
        if !self.autosave {
            return;
        }
        let state = message.map(|m| (m.to_owned(), busy));
        if self.save_error == state {
            return;
        }
        self.save_error = state;
        let root = self
            .host
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<StackContent>()
            .unwrap();
        root.children.retain(|c| c.id() == ComponentId(11));
        if let Some(message) = message {
            root.children
                .push(Content::new(STATUS, TextContent::new(message)));
            let mut retry = Button::new("Retry");
            retry.button.state.disabled = busy;
            root.children.push(Content::new(APPLY, retry));
        }
    }
    pub fn status(&mut self, message: &str, dirty: bool, busy: bool) {
        if self.autosave {
            return;
        }
        self.host
            .get_mut(STATUS)
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = message.into();
        for (id, disabled) in [(APPLY, !dirty || busy), (RESET, busy), (CANCEL, busy)] {
            self.host
                .get_mut(id)
                .unwrap()
                .component_mut::<Button>()
                .unwrap()
                .button
                .state
                .disabled = disabled;
        }
    }
}
