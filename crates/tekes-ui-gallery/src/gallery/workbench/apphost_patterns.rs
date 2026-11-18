//! Pattern transport: original StoryState owns validation, persistence and delivery.
//! The receiving ContentHost owns every mounted input, list and modal.
use super::{Entry, Phase, StoryInput, StoryState};
use tekes_ui_controls::{
    CheckState,
    composition::*,
    extended::{Axis, Stack},
};
const ROOT: ComponentId = ComponentId(110000);
const ROWS: ComponentId = ComponentId(110001);
const BODY: ComponentId = ComponentId(110002);
const STATUS: ComponentId = ComponentId(110003);
const LOG: ComponentId = ComponentId(110004);
const DETAIL: ComponentId = ComponentId(110005);
const OVERLAY: ComponentId = ComponentId(110006);
const DANGER_VIEWPORT: ComponentId = ComponentId(110007);
const COMMAND: u64 = 110100;
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn stack(id: ComponentId, children: Vec<Content>) -> Content {
    Content::new(
        id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            children,
            flexible_child: None,
        },
    )
}
fn button(command: u64, label: &str) -> Content {
    Content::new(ComponentId(COMMAND + command), Button::new(label))
}
fn root(input: Content) -> Content {
    Content::new(
        ROOT,
        ListContent::scrollable("Pattern", ROWS, vec![stack(BODY, vec![input])])
            .content_padding(16.),
    )
}
pub(crate) struct AppHostPatterns {
    state: StoryState,
    entry: &'static Entry,
    standalone: Option<ContentHost>,
    retained: Option<Content>,
    last_query: String,
    dialog_open: bool,
}
impl AppHostPatterns {
    pub(crate) fn new(entry: &'static Entry) -> anyhow::Result<Self> {
        let mut state = StoryState::for_entry(entry);
        let renderer = entry.renderer();
        let standalone = if matches!(renderer, "master" | "danger") {
            Some(
                ContentHost::new(if renderer == "danger" {
                    super::apphost_layout::dialog_viewport(
                        DANGER_VIEWPORT,
                        vec![root(text(110010, ""))],
                    )
                } else {
                    root(text(110010, ""))
                })
                .map_err(|e| anyhow::anyhow!("pattern: {e:?}"))?,
            )
        } else {
            let value = state.input.query().to_owned();
            state.input = StoryInput::field(&value, false, root);
            None
        };
        let mut page = Self {
            state,
            entry,
            standalone,
            retained: None,
            last_query: String::new(),
            dialog_open: false,
        };
        page.last_query = page.state.input.query().into();
        page.host_mut().set_action_events(true);
        // Header precedes the retained body in the document.
        let body = page.host_mut().remove(BODY).unwrap();
        page.host_mut()
            .insert(ROWS, super::story_heading(110011, entry.title()))
            .unwrap();
        page.host_mut()
            .insert(ROWS, super::apphost_basic::section_bar(entry, COMMAND))
            .unwrap();
        page.host_mut().insert(ROWS, body).unwrap();
        page.build_live();
        page.refresh();
        Ok(page)
    }
    pub(crate) fn host(&self) -> &ContentHost {
        self.standalone
            .as_ref()
            .unwrap_or_else(|| &self.state.input.hosted().unwrap().host)
    }
    pub(crate) fn host_mut(&mut self) -> &mut ContentHost {
        match &mut self.standalone {
            Some(host) => host,
            None => &mut self.state.input.hosted_mut().unwrap().host,
        }
    }
    fn build_live(&mut self) {
        let renderer = self.entry.renderer();
        let mut children = vec![button(10, "Toggle disabled"), button(12, "Toggle error")];
        if !matches!(renderer, "master" | "danger") {
            let mut field = self.host_mut().remove(ComponentId(27501)).unwrap();
            let editor = field.component_mut::<TextFieldContent>().unwrap();
            editor.show_metadata = true;
            editor.field.label = if renderer == "filter" {
                "Filter files"
            } else {
                "Workspace name"
            }
            .into();
            editor.field.help = if renderer == "filter" {
                ""
            } else {
                "Use a clear name for your workspace."
            }
            .into();
            editor.viewport = TextViewport::Form;
            if renderer == "filter" {
                children.push(field);
            } else {
                editor.show_metadata = false;
                editor.viewport = TextViewport::Fill;
                children.push(Content::new(
                    ComponentId(110023),
                    FormMetadata {
                        children: vec![field],
                        view: Default::default(),
                        arranged: false,
                    },
                ));
            }
        }
        match renderer {
            "master" => {
                let mut master = self.state.master.take().unwrap();
                let list = master.host.remove(super::master_host::LIST).unwrap();
                // Reuse the exact keyed fixture; discard its old transport after moving it.
                let list = Content::new(
                    ComponentId(110020),
                    ListContent::scrollable("Documents", ComponentId(110021), vec![list]),
                );
                let detail = self.take_detail();
                children.push(Content::new(
                    ComponentId(110022),
                    MasterPane {
                        children: vec![list, detail],
                        arranged: false,
                    },
                ));
            }
            "danger" => {
                children.push(button(20, "Delete workspace…"));
                // Dialog fills the receiving viewport; it is not a row in the
                // scrolling document (whose child measurement is unbounded).
                self.host_mut()
                    .insert(
                        DANGER_VIEWPORT,
                        Content::new(
                            OVERLAY,
                            DialogContent::new(
                                "Delete workspace",
                                text(110030, StoryState::dialog_title("danger")),
                                text(110031, "This fixture does not delete files."),
                                stack(
                                    ComponentId(110032),
                                    vec![button(70, "Cancel"), button(71, "Delete workspace")],
                                ),
                            ),
                        ),
                    )
                    .unwrap();
                children.push(button(21, "Resolve fixture"));
            }
            "filter" => children.push(button(22, "Clear filter")),
            _ => {
                children.push(button(25, "Enable notifications"));
                children.push(button(20, "Save settings"));
                children.push(button(21, "Resolve fixture"));
                if renderer == "form"
                    && self
                        .state
                        .draft_fixture
                        .as_ref()
                        .is_some_and(|store| store.is_durable())
                {
                    children.push(button(26, "Interrupt & recover"));
                }
            }
        }
        children.extend([
            button(14, "Long mixed text"),
            button(15, "Empty text"),
            text(STATUS.0, ""),
            text(LOG.0, ""),
        ]);
        self.host_mut().remove(BODY).unwrap();
        self.host_mut().insert(ROWS, stack(BODY, children)).unwrap();
    }
    fn take_detail(&mut self) -> Content {
        let original = std::mem::replace(
            &mut self.state.detail,
            super::super::text_demo::TextDemo::field("", true, true),
        );
        let mut field = original.into_content_fields().remove(0);
        field.viewport = TextViewport::Fill;
        Content::new(DETAIL, field)
    }
    fn select_section(&mut self, section: usize) {
        self.host_mut().focus(None, false);
        let old = self.host_mut().remove(BODY).unwrap();
        if self.retained.is_none() {
            self.retained = Some(old);
        }
        self.state.section = section;
        let content = if matches!(section, 1 | 4) {
            self.retained.take().unwrap()
        } else if section == 2 && self.entry.renderer() == "field-meta" {
            stack(BODY, vec![super::apphost_field_meta::matrix(&self.state)])
        } else {
            let mut lines = vec![
                super::story_docs(self.entry).to_owned(),
                super::behavior_copy(self.entry.renderer()).to_owned(),
            ];
            lines.extend(super::contract_summary(self.entry));
            lines.push(format!(
                "Component: {}\nUsage: {}",
                self.entry.component(),
                super::api_reference(self.entry)
            ));
            stack(
                BODY,
                lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| text(110050 + i as u64, s))
                    .collect(),
            )
        };
        self.host_mut().insert(ROWS, content).unwrap();
    }
    fn refresh(&mut self) {
        if self.retained.is_some() {
            return;
        }
        let renderer = self.entry.renderer();
        if self.standalone.is_none() {
            let field = self
                .host()
                .get(ComponentId(27501))
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap();
            let disabled_changed = field.surface.input().is_disabled() != self.state.disabled;
            let error = self.state.error || self.state.phase == Phase::Invalid;
            let message = error.then(|| {
                if renderer == "filter" {
                    "Validation: name is required (fixture)".to_owned()
                } else {
                    "A name is required.".to_owned()
                }
            });
            let error_changed = field.field.error != message;
            if disabled_changed {
                self.state.input.set_disabled(self.state.disabled);
            }
            if error_changed {
                let field = self
                    .host_mut()
                    .get_mut(ComponentId(27501))
                    .unwrap()
                    .component_mut::<TextFieldContent>()
                    .unwrap();
                field.field.error = message;
                field.surface.set_error(error);
            }
        }
        let mut status = if renderer == "filter" {
            let query = self.state.input.query().to_lowercase();
            let results = ["Studio notes.md", "Research notes.txt", "项目计划.md"]
                .into_iter()
                .filter(|s| s.to_lowercase().contains(&query))
                .collect::<Vec<_>>();
            if results.is_empty() {
                "No results. Clear the filter to see all files.".into()
            } else {
                results.join("\n")
            }
        } else {
            self.state.draft_error.clone().unwrap_or_else(|| {
                match self.state.phase {
                    Phase::Ready => "Ready. First submission fails; retry succeeds.",
                    Phase::Invalid => "Validation error: enter a workspace name.",
                    Phase::Pending => "Submitting… use Resolve to complete the controlled fixture.",
                    Phase::Failed => "Save failed. Your input is preserved. Retry is available.",
                    Phase::Saved => "Saved successfully. Preferences remain in this story only.",
                }
                .into()
            })
        };
        if renderer == "form"
            && let Some(store) = &self.state.draft_fixture
        {
            status.push_str(&format!(
                "\nDraft revision {} · {} · no network",
                store.draft().revision,
                if store.has_pending() {
                    "outbox awaiting acknowledgement"
                } else if store.is_acknowledged() {
                    "revision acknowledged"
                } else {
                    "draft stored locally"
                }
            ));
        }
        let log = self
            .state
            .log
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        for (id, value) in [(STATUS, status), (LOG, log)] {
            if self
                .host()
                .get(id)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text
                != value
            {
                self.host_mut()
                    .get_mut(id)
                    .unwrap()
                    .component_mut::<TextContent>()
                    .unwrap()
                    .text = value;
            }
        }
        let disabled = self.state.disabled;
        let pending = self.state.phase == Phase::Pending;
        let failed = self.state.phase == Phase::Failed;
        let checked = self.state.checkbox.checked == CheckState::Checked;
        for command in [20, 21, 25] {
            let id = ComponentId(COMMAND + command);
            let Some(button) = self.host().get(id).and_then(|n| n.component::<Button>()) else {
                continue;
            };
            let disabled = if command == 21 {
                !pending
            } else {
                disabled || command == 20 && pending
            };
            let label = if command == 25 {
                if checked {
                    "✓ Notifications enabled"
                } else {
                    "Enable notifications"
                }
            } else if command == 20 && !matches!(renderer, "danger" | "master") {
                if failed {
                    "Retry save"
                } else {
                    "Save settings"
                }
            } else {
                &button.label
            };
            if button.button.state.disabled != disabled || button.label != label {
                let label = label.to_owned();
                let button = self
                    .host_mut()
                    .get_mut(id)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap();
                button.button.state.disabled = disabled;
                button.label = label;
            }
        }
    }

    pub(crate) fn after_host_visit(&mut self) {
        self.state.window_active = self.host().is_window_active();
        if self.retained.is_none() && self.standalone.is_none() {
            let query = self.state.input.query().to_owned();
            if query != self.last_query {
                self.state.input_changed(self.entry.renderer());
                self.state.persist_draft();
                self.last_query = query;
            }
        }
        if self.retained.is_none() && self.entry.renderer() == "danger" {
            let open = self
                .host()
                .get(OVERLAY)
                .unwrap()
                .component::<DialogContent>()
                .unwrap()
                .is_open();
            // Escape is handled inside ContentHost and produces no footer action.
            if self.dialog_open && !open {
                self.state.activate(70, "danger");
            }
            self.dialog_open = open;
        }
        if self.entry.renderer() == "master" && self.retained.is_none() {
            let selected = self
                .host()
                .get(super::master_host::LIST)
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap()
                .selected_key()
                .unwrap()
                .0 as usize
                - 1;
            if selected != self.state.selected {
                self.state.activate(10000 + selected, "master");
                let detail = self.take_detail();
                self.host_mut().remove(DETAIL).unwrap();
                self.host_mut().insert(ComponentId(110022), detail).unwrap();
            }
        }
        for record in self.host_mut().drain_action_records() {
            if record.action.kind != ContentActionKind::Activate {
                continue;
            }
            let Some(command) = record.action.target.0.checked_sub(COMMAND) else {
                continue;
            };
            if (1..=6).contains(&command) {
                self.select_section(command as usize - 1);
            } else if command == 0 {
                let section = self.state.section;
                let active = self.state.window_active;
                *self = Self::new(self.entry).expect("original pattern fixture");
                self.host_mut().set_window_active(active);
                self.state.record("reset: initial fixture");
                self.select_section(section);
            } else if self.entry.renderer() == "danger" && command == 20 {
                self.host_mut().show_dialog(OVERLAY).unwrap();
                self.dialog_open = true;
                self.state.record("dialog: open");
            } else {
                if matches!(command, 70 | 71) {
                    self.host_mut().close_dialog();
                    self.dialog_open = false;
                }
                self.state.activate(command as usize, self.entry.renderer());
                if self.entry.renderer() == "form" {
                    self.state.persist_draft();
                }
            }
        }
        if self.retained.is_none() && self.standalone.is_none() {
            self.last_query = self.state.input.query().into();
        }
        if self.retained.is_none()
            && let Some(input) = self.state.input.hosted_mut()
        {
            let logs = input.drain_logs();
            for log in logs {
                self.state.record(log);
            }
        }
        self.refresh();
    }
}

/// Preserve the original emphasized metadata while the same native editor owns
/// the input slot. Metadata comes from that editor, not a second field model.
#[derive(Debug)]
struct FormMetadata {
    children: Vec<Content>,
    view: tekes_ui_controls::feedback_view::FieldView,
    arranged: bool,
}
impl FormMetadata {
    fn layout(
        &mut self,
        scene: &mut tekes_ui_scene::Scene,
        text: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        frame: tekes_ui_scene::Rect,
        clip: Option<tekes_ui_scene::Rect>,
        palette: tekes_ui_tokens::Palette,
    ) -> tekes_ui_controls::feedback_view::FieldLayout {
        let field = &self.children[0]
            .component::<TextFieldContent>()
            .unwrap()
            .field;
        self.view.render_with_presentation(
            scene,
            &mut super::apphost_basic::FixtureText(text),
            field,
            frame,
            clip,
            palette,
            tekes_ui_tokens::proposal::TextRole::UiBody,
            tekes_ui_controls::feedback_view::FieldPresentation::emphasized_form(),
        )
    }
}
impl Component for FormMetadata {
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        text: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        available: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.arranged = false;
        let width = available.width.min(600.);
        let layout = self.layout(
            &mut tekes_ui_scene::Scene::new(),
            text,
            tekes_ui_scene::Rect::new(0., 0., width, 118.),
            None,
            tekes_ui_tokens::Theme::LIGHT.into(),
        );
        tekes_ui_controls::layout::MeasuredChild::new(tekes_ui_scene::Size::new(
            width,
            layout.message.max_y().max(layout.slot.max_y()),
        ))
    }
    fn arrange(
        &mut self,
        text: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        mut frame: tekes_ui_scene::Rect,
        clip: tekes_ui_scene::Rect,
    ) {
        frame.size.width = frame.size.width.min(600.);
        let layout = self.layout(
            &mut tekes_ui_scene::Scene::new(),
            text,
            frame,
            Some(clip),
            tekes_ui_tokens::Theme::LIGHT.into(),
        );
        self.children[0].arrange(text, layout.slot, clip);
        self.arranged = true;
    }
    fn paint(
        &mut self,
        scene: &mut tekes_ui_scene::Scene,
        text: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        mut frame: tekes_ui_scene::Rect,
        clip: tekes_ui_scene::Rect,
        theme: ContentTheme,
    ) {
        frame.size.width = frame.size.width.min(600.);
        self.layout(scene, text, frame, Some(clip), theme.palette);
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Workspace name field")
    }
}

/// The original master list and reader share one bounded receiving allocation.
#[derive(Debug)]
struct MasterPane {
    children: Vec<Content>,
    arranged: bool,
}
impl Component for MasterPane {
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        _: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        available: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.arranged = false;
        tekes_ui_controls::layout::MeasuredChild::new(tekes_ui_scene::Size::new(
            available.width,
            480.,
        ))
    }
    fn arrange(
        &mut self,
        text: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        frame: tekes_ui_scene::Rect,
        clip: tekes_ui_scene::Rect,
    ) {
        self.arranged = true;
        let word = text
            .measure(
                "Document",
                tekes_ui_tokens::proposal::TextRole::UiBody,
                frame.size.width,
            )
            .width;
        let left = (frame.size.width * 0.32)
            .max(word + 44.)
            .min((frame.size.width - 12.).max(0.));
        self.children[0].arrange(
            text,
            tekes_ui_scene::Rect::new(frame.min_x(), frame.min_y(), left, frame.size.height),
            clip,
        );
        self.children[1].arrange(
            text,
            tekes_ui_scene::Rect::new(
                frame.min_x() + left + 12.,
                frame.min_y(),
                (frame.size.width - left - 12.).max(0.),
                frame.size.height,
            ),
            clip,
        );
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Master and detail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_scene::{Rect, Scene};
    fn page(renderer: &str) -> AppHostPatterns {
        AppHostPatterns::new(
            super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap(),
        )
        .unwrap()
    }
    fn render(page: &mut AppHostPatterns, text: &mut TextEngine) {
        let frame = Rect::new(0., 0., 740., 1800.);
        page.host_mut()
            .render(
                &mut Scene::new(),
                text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
        page.after_host_visit();
        if page.entry.renderer() != "danger" {
            assert!(
                !page.host().needs_render(),
                "unchanged post-render sync must preserve the Host input lease"
            );
        }
    }
    fn press(page: &mut AppHostPatterns, text: &mut TextEngine, command: u64) {
        render(page, text);
        let target = ComponentId(COMMAND + command);
        // Capture before dispatch: even a refused action retires its input lease.
        let before = page.host().inspection_snapshot(target);
        let focused = page.host().focused();
        let needs_render = page.host().needs_render();
        let reveal = page
            .host()
            .plan_host_reveal(
                target,
                tekes_ui_controls::ScrollRevealOptions::default(),
                &[],
                HostRevealBudget::default(),
            )
            .map(|plan| plan.planned_offsets().collect::<Vec<_>>());
        let dialog = page
            .host()
            .get(OVERLAY)
            .and_then(|n| n.component::<DialogContent>())
            .map(|d| (d.is_open(), d.modal_frame()));
        let response =
            page.host_mut()
                .accessibility_action(target, None, AccessibilityAction::Press);
        assert!(
            response.consumed,
            "renderer={} command={command} phase={:?} focused={focused:?} needs_render={needs_render} target={before:?} reveal={reveal:?} dialog={dialog:?} response={response:?}",
            page.entry.renderer(),
            page.state.phase
        );
        page.after_host_visit();
        render(page, text);
    }
    fn edit(page: &mut AppHostPatterns, text: &mut TextEngine, value: &str) {
        render(page, text);
        page.host_mut().focus(Some(ComponentId(27501)), false);
        assert_eq!(page.host().focused(), Some(ComponentId(27501)));
        render(page, text);
        page.host_mut()
            .event_with_text(
                &ContentEvent::TextCommand(tekes_ui_controls::TextEditCommand::SelectAll),
                text,
            )
            .unwrap();
        render(page, text);
        page.host_mut()
            .event_with_text(
                &ContentEvent::Commit {
                    text: value.into(),
                    replacement: None,
                },
                text,
            )
            .unwrap();
        page.after_host_visit();
        render(page, text);
    }
    #[test]
    fn apphost_patterns_form_retry_retains_pending_revision_and_newer_draft() {
        let mut page = page("form");
        let mut text = TextEngine::new();
        edit(&mut page, &mut text, "Original revision");
        press(&mut page, &mut text, 20);
        let pending = page.state.draft_attempt.unwrap().0;
        edit(&mut page, &mut text, "Newer draft");
        press(&mut page, &mut text, 21);
        assert_eq!(page.state.phase, Phase::Failed);
        press(&mut page, &mut text, 20);
        assert_eq!(page.state.draft_attempt.unwrap().0, pending);
        press(&mut page, &mut text, 21);
        assert_eq!(page.state.phase, Phase::Ready);
        assert_eq!(page.state.input.query(), "Newer draft");
        assert!(!page.state.draft_fixture.as_ref().unwrap().is_acknowledged());
    }
    #[test]
    fn apphost_patterns_input_chapters_validation_retry_and_reset() {
        let mut text = TextEngine::new();
        for renderer in ["form", "async", "settings"] {
            let mut page = page(renderer);
            press(&mut page, &mut text, 15);
            press(&mut page, &mut text, 20);
            assert_eq!(page.state.phase, Phase::Invalid);
            edit(&mut page, &mut text, "Retained name");
            press(&mut page, &mut text, 25);
            for command in [1, 4, 5, 2] {
                press(&mut page, &mut text, command);
            }
            assert_eq!(page.state.input.query(), "Retained name");
            assert_eq!(page.state.checkbox.checked, CheckState::Checked);
            press(&mut page, &mut text, 20);
            press(&mut page, &mut text, 21);
            assert_eq!(page.state.phase, Phase::Failed);
            press(&mut page, &mut text, 20);
            press(&mut page, &mut text, 21);
            assert_eq!(page.state.phase, Phase::Saved);
            press(&mut page, &mut text, 1);
            press(&mut page, &mut text, 0);
            assert!(page.host().get(ComponentId(27501)).is_none());
            press(&mut page, &mut text, 2);
            assert_eq!(page.state.input.query(), "Studio");
            assert_eq!(page.state.checkbox.checked, CheckState::Unchecked);
            if renderer != "form" {
                assert!(page.state.draft_fixture.is_none());
            }
        }
    }
    #[test]
    fn apphost_patterns_filter_original_matching_clear_and_empty_result() {
        let mut page = page("filter");
        let mut text = TextEngine::new();
        edit(&mut page, &mut text, "NOTES");
        assert_eq!(
            page.host()
                .get(STATUS)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text,
            "Studio notes.md\nResearch notes.txt"
        );
        edit(&mut page, &mut text, "not-found");
        assert!(
            page.host()
                .get(STATUS)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text
                .starts_with("No results.")
        );
        press(&mut page, &mut text, 22);
        assert_eq!(page.state.input.query(), "");
        assert!(
            page.host()
                .get(STATUS)
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text
                .contains("项目计划.md")
        );
        assert_eq!(page.state.log.back().unwrap(), "filter: cleared");
    }
    #[test]
    fn apphost_patterns_master_selection_updates_original_readonly_reader() {
        let mut page = page("master");
        let mut text = TextEngine::new();
        render(&mut page, &mut text);
        let token = page
            .host()
            .get(super::super::master_host::LIST)
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
            .semantic_part_for_key(tekes_ui_controls::extended::ListItemKey(2))
            .unwrap();
        assert!(
            page.host_mut()
                .accessibility_action(
                    super::super::master_host::LIST,
                    Some(token),
                    AccessibilityAction::Press
                )
                .consumed
        );
        page.after_host_visit();
        render(&mut page, &mut text);
        assert!(
            page.host()
                .text_surface(DETAIL)
                .unwrap()
                .text()
                .starts_with("Document 2 — reading notes")
        );
        assert!(
            page.host()
                .text_surface(DETAIL)
                .unwrap()
                .input()
                .is_read_only()
        );
        press(&mut page, &mut text, 1);
        press(&mut page, &mut text, 2);
        assert_eq!(page.state.selected, 1);
        assert!(
            page.host()
                .text_surface(DETAIL)
                .unwrap()
                .text()
                .starts_with("Document 2 — reading notes")
        );
    }
    #[test]
    fn apphost_patterns_danger_cancel_does_not_submit_and_confirm_retries() {
        let mut page = page("danger");
        let mut text = TextEngine::new();
        press(&mut page, &mut text, 20);
        press(&mut page, &mut text, 70);
        assert_eq!(page.state.phase, Phase::Ready);
        press(&mut page, &mut text, 20);
        press(&mut page, &mut text, 71);
        assert_eq!(page.state.phase, Phase::Pending);
        let resolve = page
            .host()
            .inspection_snapshot(ComponentId(COMMAND + 21))
            .unwrap()
            .geometry
            .unwrap();
        assert!(
            resolve.frame.max_y().is_finite() && resolve.frame.max_y() <= 1800.,
            "closed dialog must not enlarge the document: {resolve:?}"
        );
        assert!(
            resolve.visible.is_some(),
            "Resolve must be visible after confirmation: {resolve:?}"
        );
        let dialog = page
            .host()
            .get(OVERLAY)
            .unwrap()
            .component::<DialogContent>()
            .unwrap();
        assert!(!dialog.is_open());
        assert!(
            dialog.modal_frame().max_y() <= 1800.,
            "Dialog must use the finite receiving viewport"
        );
        press(&mut page, &mut text, 21);
        assert_eq!(page.state.phase, Phase::Failed);
        press(&mut page, &mut text, 20);
        press(&mut page, &mut text, 71);
        press(&mut page, &mut text, 21);
        assert_eq!(page.state.phase, Phase::Saved);
    }
}
