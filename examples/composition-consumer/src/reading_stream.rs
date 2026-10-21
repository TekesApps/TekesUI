//! Caller-owned streamed text source. Reading, input, focus and actions stay in
//! the existing public controls; this example supplies only arrivals and UI.
use std::time::Duration;
use tekes_ui_controls::{
    ReadingController, TextSurface,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::Spacing;

pub const ROOT: ComponentId = ComponentId(8800);
pub const TRANSCRIPT: ComponentId = ComponentId(8801);
pub const COMPOSER: ComponentId = ComponentId(8802);
pub const APPEND: ComponentId = ComponentId(8803);
pub const STREAM: ComponentId = ComponentId(8804);
pub const LATEST: ComponentId = ComponentId(8805);
pub const ZOOM: ComponentId = ComponentId(8806);
pub const WIDTH: ComponentId = ComponentId(8807);
const STATUS: ComponentId = ComponentId(8808);

#[derive(Debug, Default)]
pub struct StreamState {
    pub appended: usize,
    pub pending: u8,
    pub large_text: bool,
    pub narrow: bool,
    next: Option<Duration>,
    was_active: bool,
}
fn row(id: u64, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children,
        },
    )
}
pub fn build() -> Result<(ContentHost, StreamState), CompositionError> {
    let document: String = (0..90)
        .map(|i| {
            format!(
                "History {i:03}｜中文 e\u{301} 😀 streamed arrivals preserve this reading line.\n"
            )
        })
        .collect();
    let mut transcript = TextFieldContent::new("会话记录", "");
    transcript.surface = TextSurface::new(&document, true, true);
    transcript.reading = Some(ReadingController::default());
    transcript.viewport = TextViewport::Fill;
    let mut composer = TextFieldContent::new("独立草稿", "Keep this draft 中文😀");
    composer.tab_policy = TextTabPolicy::Navigate;
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: Some(TRANSCRIPT),
            children: vec![
                Content::new(
                    ComponentId(8809),
                    TextContent::new("阅读验收 · 追加不抢历史位置或草稿焦点"),
                ),
                row(
                    8810,
                    vec![
                        Content::new(APPEND, ButtonContent::new("Append one")),
                        Content::new(STREAM, ButtonContent::new("Stream four")),
                        Content::new(LATEST, ButtonContent::new("Return to latest"))
                            .preserving_focus(),
                    ],
                ),
                row(
                    8811,
                    vec![
                        Content::new(ZOOM, ButtonContent::new("Toggle text 100/150%")),
                        Content::new(WIDTH, ButtonContent::new("Toggle width")),
                    ],
                ),
                Content::new(TRANSCRIPT, transcript),
                Content::new(COMPOSER, composer),
                Content::new(STATUS, TextContent::new("History · appended 0 · queued 0")),
            ],
        },
    ))?;
    for id in [APPEND, STREAM, ZOOM, WIDTH] {
        host.get_mut(id)
            .unwrap()
            .component_mut::<ButtonContent>()
            .unwrap()
            .variant = tekes_ui_controls::paint::ButtonVariant::Quiet;
    }
    host.set_action_events(true);
    Ok((host, StreamState::default()))
}
fn field(host: &mut ContentHost) -> &mut TextFieldContent {
    host.get_mut(TRANSCRIPT)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
}
impl StreamState {
    fn append(&mut self, host: &mut ContentHost) {
        self.appended += 1;
        field(host).surface.input.text.push_str(&format!(
            "Arrival {:03}｜新内容，保持当前阅读意图与独立草稿。\n",
            self.appended
        ));
    }
    pub fn process_actions(&mut self, host: &mut ContentHost, now: Duration) {
        for action in host.drain_actions() {
            if action.kind != ContentActionKind::Activate {
                continue;
            }
            match action.target {
                APPEND => self.append(host),
                STREAM => {
                    self.pending = 4;
                    self.next = Some(now);
                }
                LATEST => {
                    let f = field(host);
                    f.reading.as_mut().unwrap().follow_latest(&mut f.surface);
                }
                ZOOM => self.large_text = !self.large_text,
                WIDTH => self.narrow = !self.narrow,
                _ => {}
            }
        }
        self.update_status(host);
    }
    /// The caller supplies time and window activity. Pausing does not accumulate
    /// arrivals or change follow intent; the next active delivery starts a slot.
    pub fn tick(&mut self, host: &mut ContentHost, now: Duration, active: bool) -> bool {
        if !active {
            self.was_active = false;
            return false;
        }
        if !self.was_active && self.pending > 0 {
            self.next = Some(now + Duration::from_millis(400));
        }
        self.was_active = true;
        if self.pending > 0 && self.next.is_some_and(|deadline| now >= deadline) {
            self.append(host);
            self.pending -= 1;
            self.next = (self.pending > 0).then_some(now + Duration::from_millis(400));
        }
        self.update_status(host);
        self.pending > 0
    }
    fn update_status(&self, host: &mut ContentHost) {
        let following = field(host).reading.as_ref().unwrap().is_following_latest();
        host.get_mut(STATUS)
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = format!(
            "{} · appended {} · queued {}",
            if following {
                "Following latest"
            } else {
                "History"
            },
            self.appended,
            self.pending,
        );
    }
}
