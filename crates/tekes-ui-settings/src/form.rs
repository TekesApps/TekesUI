use crate::{Draft, FieldPresentation, FieldType, Result, Schema, SettingsError};
#[path = "form_row.rs"]
mod form_row;
use tekes_ui_controls::{
    Button, ButtonTextRenderer, ScrollArea, TextSurface, TextSurfaceAlignment,
    TextSurfacePresentation,
    composition::{
        Component, ComponentId, Content, ContentActionPayload, ContentEvent, ContentHost,
        DecimalNumberFormat, MenuContent, MenuEntry, NumberCommitReason, NumberFieldContent,
        NumberFormat, NumberParseError, NumberValidation, RadioGroupContent, ScrollContent,
        StackContent, SwitchContent, TextContent, TextFieldContent,
    },
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Spacing, proposal};
const ROOT: ComponentId = ComponentId(1);
const STATUS: ComponentId = ComponentId(2);
const RESET: ComponentId = ComponentId(3);
const CANCEL: ComponentId = ComponentId(4);
const APPLY: ComponentId = ComponentId(5);
const DISCARD: ComponentId = ComponentId(6);
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormIntent {
    Apply,
    Cancel,
    Reset,
    Discard,
}
/// Number formatter for integer fields that rejects fractional decimal input.
#[derive(Clone, Debug, Default)]
struct IntegerNumberFormat {
    suffix: String,
}
impl IntegerNumberFormat {
    fn strip_suffix<'a>(&self, text: &'a str) -> &'a str {
        if self.suffix.is_empty() {
            text.trim()
        } else if let Some(stem) = text.strip_suffix(&self.suffix) {
            stem.trim()
        } else if let Some(stem) = text.trim().strip_suffix(&self.suffix) {
            stem.trim()
        } else if !self.suffix.trim().is_empty()
            && let Some(stem) = text.trim().strip_suffix(self.suffix.trim())
        {
            stem.trim()
        } else {
            text.trim()
        }
    }
}
impl NumberFormat for IntegerNumberFormat {
    fn width_sample(&self, digits: &str) -> String {
        format!("{digits}{}", self.suffix)
    }
    fn format(&self, value: f64) -> String {
        if value.is_finite()
            && value.fract() == 0.0
            && value >= i64::MIN as f64
            && value <= i64::MAX as f64
        {
            format!("{}{}", value as i64, self.suffix)
        } else {
            format!("{value}{}", self.suffix)
        }
    }
    fn parse(&self, text: &str) -> std::result::Result<f64, NumberParseError> {
        let text = self.strip_suffix(text);
        if text.is_empty() || matches!(text, "+" | "-") {
            return Err(NumberParseError::Incomplete);
        }
        if !text
            .bytes()
            .enumerate()
            .all(|(i, byte)| byte.is_ascii_digit() || (i == 0 && matches!(byte, b'+' | b'-')))
        {
            return Err(NumberParseError::Invalid);
        }
        let value = text.parse::<i64>().map_err(|_| NumberParseError::Invalid)?;
        let as_f64 = value as f64;
        if as_f64.is_finite() && as_f64 as i64 == value {
            Ok(as_f64)
        } else {
            Err(NumberParseError::Invalid)
        }
    }
}
fn validated_numeric_default(field: &crate::Field) -> Result<f64> {
    field
        .default
        .as_float()
        .or_else(|| field.default.as_integer().map(|v| v as f64))
        .or(field.min)
        .filter(|v| v.is_finite())
        .ok_or_else(|| {
            SettingsError::InvalidSchema(format!("{}: invalid numeric default", field.id))
        })
}
fn parse_valid_numeric(field: &crate::Field, raw: &str) -> Option<f64> {
    field
        .parse_input(raw)
        .ok()
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .filter(|v| v.is_finite())
}
fn build_number_control(field: &crate::Field, initial: f64) -> Result<NumberFieldContent> {
    let min = field.min.ok_or_else(|| {
        SettingsError::InvalidSchema(format!("{}: missing numeric min", field.id))
    })?;
    let max = field.max.ok_or_else(|| {
        SettingsError::InvalidSchema(format!("{}: missing numeric max", field.id))
    })?;
    let step = field.step.unwrap_or(1.);
    match field.kind {
        FieldType::Integer => NumberFieldContent::new(
            &field.label,
            initial,
            min,
            max,
            step,
            IntegerNumberFormat {
                suffix: field.suffix.clone(),
            },
        ),
        FieldType::Decimal => NumberFieldContent::new(
            &field.label,
            initial,
            min,
            max,
            step,
            DecimalNumberFormat {
                suffix: field.suffix.clone(),
            },
        ),
        _ => None,
    }
    .ok_or_else(|| {
        SettingsError::InvalidSchema(format!(
            "{}: invalid numeric control configuration",
            field.id
        ))
    })
}
/// One persistent component tree. Drafts contain values; TextField owns editing/IME.
#[derive(Debug)]
pub struct SettingsForm {
    host: ContentHost,
    field_ids: Vec<ComponentId>,
    autosave: bool,
    save_error: Option<(String, bool)>,
}
fn stack(id: u64, axis: Axis, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent {
            model: Stack {
                axis,
                gap: Spacing::Relaxed.value(),
            },
            children,
            flexible_child: None,
        },
    )
}
impl SettingsForm {
    pub fn new(schema: &Schema, draft: &Draft) -> Result<Self> {
        Self::build(schema, draft, false)
    }
    pub fn host(&self) -> &ContentHost {
        &self.host
    }
    pub fn focus(&mut self, id: Option<ComponentId>, visible: bool) -> bool {
        self.host.focus(id, visible)
    }
    pub fn enter_focus(&mut self, reverse: bool) -> bool {
        self.host.enter_focus(reverse)
    }
    pub fn set_focus_wrap(&mut self, wrap: bool) {
        self.host.set_focus_wrap(wrap);
    }
    pub fn cancel_host(&mut self) {
        self.host.cancel();
    }
    pub fn with_host_mut<R>(&mut self, visit: impl FnOnce(&mut ContentHost) -> R) -> R {
        visit(&mut self.host)
    }
    /// Autosave consumers have no persistent footer or draft-action buttons.
    pub fn new_autosave(schema: &Schema, draft: &Draft) -> Result<Self> {
        Self::build(schema, draft, true)
    }
    fn build(schema: &Schema, draft: &Draft, autosave: bool) -> Result<Self> {
        schema.validate_snapshot(&draft.base)?;
        let ids: Vec<_> = (0..schema.settings.len())
            .map(|i| ComponentId(100 + i as u64))
            .collect();
        fn groups(
            schema: &Schema,
            draft: &Draft,
            ids: &[ComponentId],
            parent: Option<&str>,
        ) -> Result<Vec<Content>> {
            let mut children = vec![];
            for (group_index, group) in schema
                .groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.parent.as_deref() == parent)
            {
                let fields: Vec<_> = schema
                    .settings
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.group == group.id)
                    .collect();
                let mut rows = Vec::with_capacity(fields.len());
                for (position, &(index, f)) in fields.iter().enumerate() {
                    let raw = draft.display(f);
                    let control = match f.kind {
                        FieldType::Boolean => {
                            Content::new(ids[index], SwitchContent::new("", raw == "true"))
                        }
                        FieldType::Enum if f.presentation == FieldPresentation::Radio => {
                            let mut radio = RadioGroupContent::new(
                                &f.label,
                                f.options.iter().map(|o| o.label.clone()).collect(),
                            );
                            radio.set_axis(Axis::Horizontal);
                            let _ =
                                radio.set_selected(f.options.iter().position(|o| o.value == raw));
                            Content::new(ids[index], radio)
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
                            .map_err(|e| SettingsError::Ui(format!("Settings menu: {e:?}")))?;
                            Content::new(ids[index], menu)
                        }
                        FieldType::Integer | FieldType::Decimal
                            if f.step.is_some() || !f.suffix.is_empty() =>
                        {
                            let parsed = parse_valid_numeric(f, &raw);
                            let initial = match parsed {
                                Some(value) => value,
                                None => validated_numeric_default(f)?,
                            };
                            let mut number = build_number_control(f, initial)?;
                            number.set_input_digits(3);
                            number.set_alignment(TextSurfaceAlignment::Trailing);
                            number.set_borderless(true);
                            if parsed.is_none() {
                                number.set_focus(true, false);
                                number.event(&ContentEvent::TextCommand(
                                    tekes_ui_controls::TextEditCommand::SelectAll,
                                ));
                                number.event(&ContentEvent::Commit {
                                    text: raw.clone(),
                                    replacement: None,
                                });
                                number.set_focus(false, false);
                                let _ = number.drain_value_changes();
                            }
                            Content::new(ids[index], number)
                        }
                        _ => {
                            let mut field = TextFieldContent::new("", &raw);
                            field
                                .surface
                                .set_presentation(TextSurfacePresentation::Content);
                            field.surface.set_alignment(TextSurfaceAlignment::Trailing);
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
                            .map_err(|e| SettingsError::Ui(format!("Text label layout: {e:?}")))?,
                    ];
                    rows.push(Content::new(
                        ComponentId(base),
                        form_row::SettingsRow::new(
                            &f.label,
                            stack(base + 3, Axis::Vertical, labels),
                            control.with_help(&f.description),
                            f.kind == FieldType::Boolean,
                        )
                        .in_group(position == 0, position + 1 == fields.len()),
                    ));
                }
                if !rows.is_empty() {
                    children.push(Content::new(
                        ComponentId((1_u64 << 48) + group_index as u64),
                        StackContent {
                            model: Stack {
                                axis: Axis::Vertical,
                                gap: 0.,
                            },
                            children: rows,
                            flexible_child: None,
                        },
                    ));
                }
                children.extend(groups(schema, draft, ids, Some(&group.id))?);
            }
            Ok(children)
        }
        let children = groups(schema, draft, &ids, None)?;
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
        let mut host = ContentHost::new(root)
            .map_err(|e| SettingsError::Ui(format!("Settings form: {e:?}")))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            field_ids: ids,
            autosave,
            save_error: None,
        })
    }
    /// Center the readable body within the caller's content allocation.
    /// Hosts own pane/titlebar insets; Settings owns the typography-scaled width.
    pub fn body_rect(text: &mut dyn ButtonTextRenderer, frame: Rect) -> Rect {
        let role = proposal::TextRole::UiBody;
        let max_width = proposal::metric("size", "c-settings-content-max-width")
            .expect("Settings content width")
            * text.resolved_font_size(role)
            / role.style().size;
        let width = frame.size.width.min(max_width);
        Rect::new(
            frame.min_x() + (frame.size.width - width) * 0.5,
            frame.min_y(),
            width,
            frame.size.height,
        )
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        palette: Palette,
    ) -> Result<()> {
        let body = Self::body_rect(text, frame);
        self.host
            .render(scene, text, body, frame, palette)
            .map_err(|e| SettingsError::Ui(format!("Settings render: {e:?}")))
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
            .map_err(|e| SettingsError::Ui(format!("Settings input: {e:?}")))?;
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
        for record in &records {
            if let Some(ContentActionPayload::RadioSelection(selection)) = &record.payload
                && let Some(index) = self
                    .field_ids
                    .iter()
                    .position(|id| *id == record.action.target)
            {
                let field = &schema.settings[index];
                draft.edit(field, field.options[selection.index()].value.clone());
            }
        }
        self.collect_number_changes(schema, draft);
        for (index, f) in schema.settings.iter().enumerate() {
            let node = self.host.get(self.field_ids[index]).unwrap();
            let raw = if let Some(field) = node.component::<TextFieldContent>() {
                Some(field.surface.text().to_owned())
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
                DISCARD => Some(FormIntent::Discard),
                _ => None,
            })
            .collect()
    }
    pub fn collect_number_changes(&mut self, schema: &Schema, draft: &mut Draft) {
        for (index, id) in self.field_ids.iter().enumerate() {
            // Empty intent polling must not invalidate the arrangement needed
            // by the following callbacks in the same native text-key span.
            if !self
                .host
                .get(*id)
                .unwrap()
                .component::<NumberFieldContent>()
                .is_some_and(NumberFieldContent::has_value_changes)
            {
                continue;
            }
            let number = self
                .host
                .get_mut(*id)
                .unwrap()
                .component_mut::<NumberFieldContent>()
                .unwrap();
            for change in number.drain_value_changes() {
                draft.edit(&schema.settings[index], change.value.to_string());
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
    /// Read-only admission before a native action uses its current publication.
    /// Actual collection/commit remains in flush_numbers at the request boundary.
    pub fn can_flush_numbers(&self) -> bool {
        self.field_ids.iter().all(|id| {
            self.host
                .get(*id)
                .unwrap()
                .component::<NumberFieldContent>()
                .is_none_or(|number| {
                    let state = number.edit_state();
                    !state.composing && matches!(state.validation, NumberValidation::Valid(_))
                })
        })
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
    pub fn cancel_edits(&mut self) {
        self.host.cancel();
        self.host.focus(None, false);
        for id in &self.field_ids {
            if let Some(number) = self
                .host
                .get_mut(*id)
                .and_then(|node| node.component_mut::<NumberFieldContent>())
            {
                number.cancel_edit();
                let _ = number.drain_value_changes();
            }
        }
    }
    pub fn replace_values(&mut self, schema: &Schema, draft: &Draft) {
        self.host.cancel();
        self.host.focus(None, false);
        for (index, f) in schema.settings.iter().enumerate() {
            let raw = draft.display(f);
            let node = self.host.get_mut(self.field_ids[index]).unwrap();
            if let Some(field) = node.component_mut::<TextFieldContent>() {
                field.surface = TextSurface::new(&raw, false, false);
                field
                    .surface
                    .set_presentation(TextSurfacePresentation::Content);
                field.surface.set_alignment(TextSurfaceAlignment::Trailing);
            } else if let Some(number) = node.component_mut::<NumberFieldContent>() {
                // Drain any commit generated by the blur above before syncing the target value.
                let _ = number.drain_value_changes();
                // Preserve a genuinely newer in-progress edit during history reconciliation.
                let target =
                    parse_valid_numeric(f, &raw).or_else(|| validated_numeric_default(f).ok());
                if let Some(target) = target {
                    let _ = number.set_value(target);
                }
                let _ = number.drain_value_changes();
            } else if let Some(switch) = node.component_mut::<SwitchContent>() {
                switch.model.checked = raw == "true";
            } else if let Some(radio) = node.component_mut::<RadioGroupContent>() {
                let _ = radio.set_selected(f.options.iter().position(|o| o.value == raw));
            } else if let Some(menu) = node.component_mut::<MenuContent>() {
                let label = f
                    .options
                    .iter()
                    .find(|o| o.value == raw)
                    .map_or(raw.as_str(), |o| o.label.as_str());
                *menu = MenuContent::new(label, vec![]);
                let _ = menu.set_entries(
                    f.options
                        .iter()
                        .map(|o| MenuEntry::item(&o.value, &o.label))
                        .collect(),
                );
            }
        }
    }
    /// A transient error/retry/discard row appears only when saving needs attention.
    pub fn save_error(&mut self, message: Option<&str>, busy: bool) {
        if !self.autosave {
            return;
        }
        let state = message.map(|m| (m.to_owned(), busy));
        if self.save_error == state {
            return;
        }
        self.save_error = state;
        let _ = self.host.remove(STATUS);
        let _ = self.host.remove(APPLY);
        let _ = self.host.remove(DISCARD);
        if let Some(message) = message {
            self.host
                .insert(ROOT, Content::new(STATUS, TextContent::new(message)))
                .expect("Settings root accepts error status");
            let mut retry = Button::new("Retry");
            retry.button.state.disabled = busy;
            self.host
                .insert(ROOT, Content::new(APPLY, retry))
                .expect("Settings root accepts retry button");
            let mut discard = Button::new("Discard");
            discard.button.state.disabled = busy;
            self.host
                .insert(ROOT, Content::new(DISCARD, discard))
                .expect("Settings root accepts discard button");
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
