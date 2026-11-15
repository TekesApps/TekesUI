//! AppHost composition of the original hosted Playground stories. The original
//! StoryState constructs and owns the component host; only its outer layout and
//! Gallery control transport change. No legacy paint/input/native tree runs.
use super::{Entry, StoryState};
use tekes_ui_controls::{
    CheckState,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::Spacing;

const ROOT: ComponentId = ComponentId(91000);
const ROWS: ComponentId = ComponentId(91001);
const STATUS: ComponentId = ComponentId(91002);
const LOG: ComponentId = ComponentId(91003);
const CONTROL: u64 = 91100;

pub(crate) fn supports(page: &str) -> bool {
    matches!(
        page,
        "component-text-field" | "component-text-area" | "component-combo-box" | "component-select"
    )
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn root(mut specimen: Content) -> Content {
    let id = specimen.id();
    let select = id == ComponentId(8901);
    let combo = id == ComponentId(27601);
    // The old Gallery allocated an explicit viewport (Area 100..270pt). In
    // the new scrolling composition the same editor needs a bounded viewport,
    // rather than Fill consuming the entire outer scroll document.
    if let Some(field) = specimen.component_mut::<TextFieldContent>() {
        field.viewport = TextViewport::Fixed(if id == ComponentId(27502) { 270. } else { 40. });
    }
    let mut controls = vec![
        Content::new(ComponentId(CONTROL), Button::new("Reset")),
        Content::new(
            ComponentId(CONTROL + 10),
            CheckboxContent::new("Disabled", CheckState::Unchecked),
        ),
    ];
    if !combo {
        controls.push(Content::new(
            ComponentId(CONTROL + 12),
            CheckboxContent::new("Error", CheckState::Unchecked),
        ));
    }
    if select {
        controls.push(Content::new(
            ComponentId(CONTROL + 13),
            CheckboxContent::new("Empty options", CheckState::Unchecked),
        ));
    }
    let mut children = vec![
        text(91004, "Original Playground"),
        Content::new(
            ComponentId(91005),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                children: controls,
                flexible_child: None,
            },
        ),
        specimen,
        text(STATUS.0, ""),
        text(LOG.0, ""),
    ];
    if !select && !combo {
        children.extend([
            text(91006, "Original edge-case controls"),
            Content::new(ComponentId(CONTROL + 14), Button::new("Long mixed text")),
            Content::new(ComponentId(CONTROL + 15), Button::new("Empty text")),
        ]);
    }
    if id == ComponentId(27501) {
        // Retain #291's accepted multi-instance inspection fixture and keys.
        // It is explicitly separate from the original TextField story above.
        children.push(super::super::apphost_workbench::composition_fixture());
    }
    Content::new(
        ROOT,
        ListContent::scrollable("Gallery Playground", ROWS, children)
            .content_padding(Spacing::Loose.value()),
    )
}

pub(crate) struct AppHostStory {
    state: StoryState,
    renderer: &'static str,
    last_query: String,
}
impl AppHostStory {
    pub(crate) fn new(entry: &'static Entry) -> Self {
        let mut story = Self {
            state: StoryState::for_entry_with_root(entry, root),
            renderer: entry.renderer(),
            last_query: String::new(),
        };
        story.last_query = story.state.input.query().into();
        story.apply_properties();
        story.refresh_copy();
        story
    }
    pub(crate) fn host(&self) -> &ContentHost {
        if self.renderer == "select" {
            &self.state.select_story.host
        } else {
            &self.state.input.hosted().expect("hosted Playground").host
        }
    }
    pub(crate) fn host_mut(&mut self) -> &mut ContentHost {
        if self.renderer == "select" {
            &mut self.state.select_story.host
        } else {
            &mut self
                .state
                .input
                .hosted_mut()
                .expect("hosted Playground")
                .host
        }
    }
    fn apply_properties(&mut self) {
        match self.renderer {
            "select" => {
                self.state
                    .select_story
                    .set_labels(if self.state.select_empty {
                        vec![]
                    } else {
                        vec!["Automatic".into(), "Compact".into(), "Comfortable".into()]
                    })
                    .expect("original bounded Select fixture");
                self.state.select_story.set_disabled(self.state.disabled);
                self.state.select_story.set_error(self.state.error);
            }
            "combo-box" => self.state.input.set_disabled(self.state.extended.disabled),
            _ => {
                self.state.input.set_disabled(self.state.disabled);
                self.state
                    .input
                    .set_error(self.state.error || self.state.phase == super::Phase::Invalid);
            }
        }
        let disabled = if self.renderer == "combo-box" {
            self.state.extended.disabled
        } else {
            self.state.disabled
        };
        for (command, checked) in [
            (10, disabled),
            (12, self.state.error),
            (13, self.state.select_empty),
        ] {
            if let Some(content) = self.host_mut().get_mut(ComponentId(CONTROL + command)) {
                content
                    .component_mut::<CheckboxContent>()
                    .unwrap()
                    .set_checked(if checked {
                        CheckState::Checked
                    } else {
                        CheckState::Unchecked
                    });
            }
        }
    }
    fn refresh_copy(&mut self) {
        let status = match self.renderer {
            "select" if self.state.select_empty => "No options available.".into(),
            "select" if self.state.error => {
                "This choice is unavailable. Choose another option.".into()
            }
            "select" => format!(
                "Selected original index: {}",
                self.state.select_story.model().selected
            ),
            "combo-box" => {
                let model = self.state.input.combo().model();
                if model.open && model.filtered().is_empty() {
                    "No matching options".into()
                } else {
                    format!(
                        "Query: {} · selected original index: {:?}",
                        self.state.input.query(),
                        model.selected
                    )
                }
            }
            _ if self.state.error => "Validation: name is required (fixture)".into(),
            _ => "Project name — native text input".into(),
        };
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
    }
    /// Called after the one formal Host operation, never while dispatching an
    /// input event. Gallery consumes control intents without replaying them.
    pub(crate) fn after_host_visit(&mut self) {
        // Reset must inherit the activity currently delivered by AppHost, not
        // the unused legacy window adapter's initial flag.
        self.state.window_active = self.host().is_window_active();
        let records = self.host_mut().drain_action_records();
        for record in records {
            let command = record.action.target.0.checked_sub(CONTROL);
            if record.action.kind == ContentActionKind::Activate
                && matches!(command, Some(0 | 10 | 12 | 13 | 14 | 15))
            {
                self.state
                    .activate_with_root(command.unwrap() as usize, self.renderer, root);
                self.apply_properties();
            } else {
                match record.payload {
                    Some(ContentActionPayload::MenuSelection(selection)) => {
                        self.state.record(format!("select: {}", selection.index()))
                    }
                    Some(ContentActionPayload::ComboBoxSelection(selection)) => self
                        .state
                        .record(format!("combo selection: {}", selection.index())),
                    _ => {}
                }
            }
        }
        if let Some(host) = self.state.input.hosted_mut() {
            // Preserve original controlled-update error reporting. Action
            // records were already consumed above, so none are replayed.
            let logs = host.drain_logs();
            for log in logs {
                self.state.record(log);
            }
        }
        let query = self.state.input.query().to_owned();
        if query != self.last_query {
            self.state.input_changed(self.renderer);
            self.last_query = query;
        }
        self.refresh_copy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_scene::{Rect, Scene};

    fn story(renderer: &str) -> AppHostStory {
        AppHostStory::new(
            super::super::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer)
                .unwrap(),
        )
    }
    fn render(story: &mut AppHostStory, text: &mut TextEngine) {
        let frame = Rect::new(0., 0., 740., 1200.);
        story
            .host_mut()
            .render(
                &mut Scene::new(),
                text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
        story.after_host_visit();
    }
    fn press(story: &mut AppHostStory, text: &mut TextEngine, command: u64) {
        render(story, text);
        assert!(
            story
                .host_mut()
                .accessibility_action(
                    ComponentId(CONTROL + command),
                    None,
                    AccessibilityAction::Press
                )
                .consumed
        );
        story.after_host_visit();
        render(story, text);
    }
    #[test]
    fn apphost_original_playground_initial_values_controls_and_reset() {
        let mut text = TextEngine::new();
        for renderer in ["field", "area", "combo-box", "select"] {
            let original = StoryState::for_entry_renderer(renderer);
            let mut story = story(renderer);
            if renderer == "select" {
                assert_eq!(
                    story.state.select_story.model().selected,
                    original.select_story.model().selected
                );
            } else {
                assert_eq!(story.state.input.query(), original.input.query());
            }
            press(&mut story, &mut text, 10);
            let specimen = match renderer {
                "field" => 27501,
                "area" => 27502,
                "combo-box" => 27601,
                _ => 8901,
            };
            assert!(
                story
                    .host()
                    .inspection_snapshot(ComponentId(specimen))
                    .unwrap()
                    .semantics
                    .disabled
            );
            assert_eq!(
                story
                    .state
                    .log
                    .iter()
                    .filter(|event| event.as_str() == "disabled: true")
                    .count(),
                1
            );
            press(&mut story, &mut text, 10);
            assert!(
                !story
                    .host()
                    .inspection_snapshot(ComponentId(specimen))
                    .unwrap()
                    .semantics
                    .disabled
            );
            if renderer != "combo-box" {
                press(&mut story, &mut text, 12);
                assert!(
                    story
                        .host()
                        .inspection_snapshot(ComponentId(specimen))
                        .unwrap()
                        .semantics
                        .invalid
                );
            }
            if matches!(renderer, "field" | "area") {
                press(&mut story, &mut text, 14);
                assert!(story.state.input.query().contains("项目計画"));
                press(&mut story, &mut text, 15);
                assert!(story.state.input.query().is_empty());
            }
            press(&mut story, &mut text, 0);
            assert!(story.host().get(ROOT).is_some());
            assert!(story.host().get(ComponentId(CONTROL + 10)).is_some());
            assert!(
                !story
                    .host()
                    .inspection_snapshot(ComponentId(specimen))
                    .unwrap()
                    .semantics
                    .disabled
            );
            assert!(
                !story
                    .host()
                    .inspection_snapshot(ComponentId(specimen))
                    .unwrap()
                    .semantics
                    .invalid
            );
            if renderer != "select" {
                assert_eq!(story.state.input.query(), original.input.query());
            }
        }
    }
    #[test]
    fn apphost_original_combo_ime_filter_commits_original_index_once() {
        let mut story = story("combo-box");
        let mut text = TextEngine::new();
        render(&mut story, &mut text);
        assert!(story.host_mut().focus(Some(ComponentId(27601)), false));
        render(&mut story, &mut text);
        for event in [
            ContentEvent::Preedit {
                text: "C".into(),
                selection: 0..1,
                clauses: vec![],
                replacement: None,
            },
            ContentEvent::Commit {
                text: "Com".into(),
                replacement: None,
            },
        ] {
            story.host_mut().event_with_text(&event, &mut text).unwrap();
            story.after_host_visit();
            render(&mut story, &mut text);
        }
        assert!(
            story
                .host()
                .text_surface(ComponentId(27601))
                .unwrap()
                .input
                .marked
                .is_none()
        );
        assert_eq!(story.state.input.combo().model().filtered(), vec![1, 2]);
        for command in ["moveDown:", "insertNewline:"] {
            story
                .host_mut()
                .event_with_text(&ContentEvent::TextCommand(command.into()), &mut text)
                .unwrap();
            story.after_host_visit();
            render(&mut story, &mut text);
        }
        assert_eq!(story.state.input.combo().model().selected, Some(2));
        assert_eq!(story.state.input.query(), "Comfortable");
        assert!(!story.state.input.combo().model().open);
        assert_eq!(
            story
                .state
                .log
                .iter()
                .filter(|event| event.as_str() == "combo selection: 2")
                .count(),
            1
        );
    }
    #[test]
    fn apphost_original_select_empty_options_revoke_old_menu_parts() {
        let mut story = story("select");
        let mut text = TextEngine::new();
        render(&mut story, &mut text);
        assert!(
            story
                .host_mut()
                .accessibility_action(ComponentId(8901), None, AccessibilityAction::Press)
                .consumed
        );
        story.after_host_visit();
        render(&mut story, &mut text);
        let token = story
            .host()
            .get(ComponentId(8901))
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .semantic_parts()
            .into_iter()
            .find(|part| part.semantics.name == "Comfortable")
            .unwrap()
            .index;
        assert!(
            story
                .host_mut()
                .accessibility_action(ComponentId(8901), Some(token), AccessibilityAction::Press)
                .consumed
        );
        story.after_host_visit();
        render(&mut story, &mut text);
        assert_eq!(story.state.select_story.model().selected, 2);
        assert_eq!(
            story
                .state
                .log
                .iter()
                .filter(|event| event.as_str() == "select: 2")
                .count(),
            1
        );
        press(&mut story, &mut text, 13);
        assert!(story.state.select_empty);
        assert!(
            !story
                .host_mut()
                .accessibility_action(ComponentId(8901), Some(token), AccessibilityAction::Press)
                .consumed
        );
        press(&mut story, &mut text, 13);
        assert!(!story.state.select_empty);
        assert_eq!(
            story
                .host()
                .get(ComponentId(8901))
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .model()
                .menu
                .items
                .count,
            3
        );
    }
}
