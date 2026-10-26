//! Independent public-API caller: original fields and Stack, no custom arrange.
use tekes_ui_controls::{
    ReadingController, TextSurface,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(9200);
pub const TRANSCRIPT: ComponentId = ComponentId(9201);
pub const COMPOSER: ComponentId = ComponentId(9202);
pub const SEND: ComponentId = ComponentId(9203);
#[derive(Debug)]
pub struct ViewportFixture {
    pub host: ContentHost,
}
impl Default for ViewportFixture {
    fn default() -> Self {
        Self::new()
    }
}
impl ViewportFixture {
    pub fn new() -> Self {
        let mut transcript = TextFieldContent::new("会话记录", "");
        transcript.surface = TextSurface::new("", true, true);
        transcript.viewport = TextViewport::Fill;
        let mut reading = ReadingController::default();
        let text = (0..80)
            .map(|n| format!("消息 {n}：中文与 English 😀 e\u{301}\n"))
            .collect::<String>();
        reading
            .set_document(&mut transcript.surface, "reading", 1, &text)
            .unwrap();
        transcript.reading = Some(reading);
        let mut composer = TextFieldContent::new("消息", "");
        composer.surface = TextSurface::new("", true, false);
        composer.viewport = TextViewport::Grow {
            min: 56.,
            max: 160.,
        };
        composer.tab_policy = TextTabPolicy::Insert;
        // A fixture reports the existing activated intent, not business delivery.
        composer.submit_policy = TextSubmitPolicy::CommandEnter;
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: Some(TRANSCRIPT),
                children: vec![
                    Content::new(TRANSCRIPT, transcript),
                    Content::new(COMPOSER, composer),
                    Content::new(SEND, Button::new("发送")),
                ],
            },
        ))
        .unwrap();
        host.focus(Some(COMPOSER), true);
        Self { host }
    }
    pub fn field(&self, id: ComponentId) -> &TextFieldContent {
        self.host
            .get(id)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
    }
    pub fn field_mut(&mut self, id: ComponentId) -> &mut TextFieldContent {
        self.host
            .get_mut(id)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.host.event(event)
    }
}
