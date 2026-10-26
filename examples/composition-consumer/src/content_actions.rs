//! Independent caller consuming actions from the original controls, never counters.
use tekes_ui_controls::extended::{Axis, Stack};
use tekes_ui_controls::{TextSurface, composition::*};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(9300);
pub const FIRST: ComponentId = ComponentId(9301);
pub const SECOND: ComponentId = ComponentId(9302);
pub const EDITOR: ComponentId = ComponentId(9303);
pub const STATUS: ComponentId = ComponentId(9304);
#[derive(Debug)]
pub struct ActionFixture {
    pub host: ContentHost,
    pub delivered: Vec<ContentAction>,
}
impl Default for ActionFixture {
    fn default() -> Self {
        Self::new()
    }
}
impl ActionFixture {
    pub fn new() -> Self {
        let mut editor = TextFieldContent::new("仅提交意图", "");
        editor.surface = TextSurface::new("", true, false);
        editor.submit_policy = TextSubmitPolicy::CommandEnter;
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: None,
                children: vec![
                    Content::new(FIRST, Button::new("动作 A")),
                    Content::new(SECOND, Button::new("动作 B")),
                    Content::new(EDITOR, editor),
                    Content::new(STATUS, TextContent::new("尚无动作")),
                ],
            },
        ))
        .unwrap();
        host.set_action_events(true);
        Self {
            host,
            delivered: Vec::new(),
        }
    }
    fn collect(&mut self) {
        let actions = self.host.drain_actions();
        if !actions.is_empty() {
            // Drain owns data, so this mutation cannot borrow/re-enter its queue.
            let label = format!("{:?}", actions);
            self.host
                .get_mut(STATUS)
                .unwrap()
                .component_mut::<TextContent>()
                .unwrap()
                .text = label;
            self.delivered.extend(actions);
        }
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let response = self.host.event(event);
        self.collect();
        response
    }
    pub fn accessibility_action(
        &mut self,
        id: ComponentId,
        part: Option<usize>,
        action: AccessibilityAction,
    ) -> ContentResponse {
        let response = self.host.accessibility_action(id, part, action);
        self.collect();
        response
    }
}
