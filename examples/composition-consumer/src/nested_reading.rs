//! Issue53 independent declaration: the SDK owns nested reading input/routing.
use tekes_ui_controls::{
    ReadingController, ReadingDocumentVersion, ScrollArea, ScrollPolicy,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::proposal;
pub const ROOT: ComponentId = ComponentId(53000);
pub const PARENT: ComponentId = ComponentId(53001);
pub const BODY: ComponentId = ComponentId(53002);
pub const INNER: ComponentId = ComponentId(53003);
pub const COMPOSER: ComponentId = ComponentId(53004);
pub const TAIL: ComponentId = ComponentId(53005);
pub const APPEND: ComponentId = ComponentId(53008);
pub const LATEST: ComponentId = ComponentId(53009);
pub struct State {
    pub version: ReadingDocumentVersion,
    pub appended: usize,
    items: Vec<ComponentId>,
}
pub fn build() -> Result<(ContentHost, State), CompositionError> {
    let value = (0..60)
        .map(|n| format!("history line {n} 中文\n"))
        .collect::<String>();
    let mut inner =
        TextFieldContent::read_only_content("Original detail", &value, proposal::TextRole::UiBody);
    inner.viewport = TextViewport::Grow {
        min: 120.,
        max: 120.,
    };
    inner
        .surface
        .scroll
        .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
    let mut reading = ReadingController::default();
    reading
        .set_document(&mut inner.surface, "inner-document", 1, &value)
        .unwrap();
    inner.reading = Some(reading);
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: Some(PARENT),
            children: vec![
                Content::new(
                    ComponentId(53010),
                    StackContent {
                        model: Stack {
                            axis: Axis::Horizontal,
                            gap: 8.,
                        },
                        flexible_child: None,
                        children: vec![
                            Content::new(APPEND, Button::new("Append content")).preserving_focus(),
                            Content::new(LATEST, Button::new("Return to latest"))
                                .preserving_focus(),
                        ],
                    },
                ),
                Content::new(
                    PARENT,
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(
                            BODY,
                            StackContent {
                                model: Stack {
                                    axis: Axis::Vertical,
                                    gap: 8.,
                                },
                                flexible_child: None,
                                children: vec![
                                    Content::new(INNER, inner),
                                    Content::new(TAIL, TextContent::new("After detail")),
                                ],
                            },
                        ),
                    ),
                ),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft")),
            ],
        },
    ))?;
    let items = vec![INNER, TAIL];
    let (version, ()) = host
        .set_scroll_reading_document(PARENT, "parent-A", 1, items.clone(), |_| ())
        .unwrap();
    host.get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    host.set_action_events(true);
    Ok((
        host,
        State {
            version,
            appended: 0,
            items,
        },
    ))
}
impl State {
    pub fn process_actions(&mut self, host: &mut ContentHost) {
        for action in host.drain_actions() {
            if action.kind != ContentActionKind::Activate {
                continue;
            }
            match action.target {
                APPEND => {
                    let id = ComponentId(53400 + self.appended as u64);
                    let mut items = self.items.clone();
                    items.push(id);
                    let (version, ()) = host
                        .update_scroll_reading_document(
                            PARENT,
                            &self.version,
                            self.version.revision() + 1,
                            items.clone(),
                            |h| {
                                h.insert(
                                    BODY,
                                    Content::new(id, TextContent::new("new line\n".repeat(50))),
                                )
                                .unwrap();
                            },
                        )
                        .unwrap();
                    self.version = version;
                    self.items = items;
                    self.appended += 1;
                }
                LATEST => {
                    host.get_mut(PARENT)
                        .unwrap()
                        .component_mut::<ScrollContent>()
                        .unwrap()
                        .follow_latest();
                }
                _ => {}
            }
        }
    }
}
