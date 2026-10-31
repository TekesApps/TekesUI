//! Public-only SearchField consumer: host owns effects, query filtering and copy.
use tekes_ui_controls::{
    ReadingController,
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(8800);
pub const SEARCH: ComponentId = ComponentId(8801);
pub const OTHER: ComponentId = ComponentId(8802);
pub const HISTORY: ComponentId = ComponentId(8803);

#[derive(Debug)]
pub struct SearchFixture {
    pub host: ContentHost,
    pub delivered: Vec<ContentActionRecord>,
}
impl Default for SearchFixture {
    fn default() -> Self {
        Self::new("A😀e\u{301}中 query")
    }
}
impl SearchFixture {
    pub fn new(query: &str) -> Self {
        let mut search = SearchFieldContent::new("Search documents", query);
        search.set_submit_policy(TextSubmitPolicy::Enter);
        search.set_placeholder(Some("按名称搜索…".into()));
        search.set_clear_label("清除查询").unwrap();
        let mut history = TextFieldContent::read_only_content(
            "History",
            &(0..40)
                .map(|i| format!("line {i}: retained history 中文\n"))
                .collect::<String>(),
            tekes_ui_tokens::proposal::TextRole::UiBody,
        );
        history.viewport = TextViewport::Fixed(96.);
        history.surface.scroll.set_scroll_policy(
            tekes_ui_controls::ScrollPolicy::Disabled,
            tekes_ui_controls::ScrollPolicy::Auto,
        );
        history.reading = Some(ReadingController::default());
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(SEARCH, search),
                    Content::new(OTHER, TextFieldContent::new("Other editor", "other draft")),
                    Content::new(HISTORY, history),
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
    pub fn search(&self) -> &SearchFieldContent {
        self.host
            .get(SEARCH)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
    }
    pub fn search_mut(&mut self) -> &mut SearchFieldContent {
        self.host
            .get_mut(SEARCH)
            .unwrap()
            .component_mut::<SearchFieldContent>()
            .unwrap()
    }
    pub fn collect(&mut self) {
        self.delivered.extend(self.host.drain_action_records());
    }
}
