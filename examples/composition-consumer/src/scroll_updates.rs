//! Mixed public Content under the existing ScrollContent. No scroll state machine
//! or pixel compensation lives in this consumer. Reading is explicitly opt-in;
//! the default scene remains a functional stress fixture, not visual acceptance.
use tekes_ui_controls::{
    ReadingDocumentVersion, ReadingUpdateError, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::{Spacing, proposal};

pub const ROOT: ComponentId = ComponentId(9700);
pub const SCROLLER: ComponentId = ComponentId(9701);
pub const ITEMS: ComponentId = ComponentId(9702);
pub const COMPOSER: ComponentId = ComponentId(9703);
pub const TOP: ComponentId = ComponentId(9704);
pub const TOGGLE: ComponentId = ComponentId(9705);
pub const ACTION: ComponentId = ComponentId(9706);
pub const LATEST: ComponentId = ComponentId(9707);
pub const PANEL: ComponentId = ComponentId(9708);
pub const PANEL_TEXT: ComponentId = ComponentId(9709);
pub const INLINE_EDITOR: ComponentId = ComponentId(9712);

pub struct Scenario {
    pub host: ContentHost,
    next: u64,
    pub expanded: bool,
}
impl Scenario {
    pub fn reading_document() -> Result<Self, CompositionError> {
        let mut s = Self::new()?;
        s.host.insert(
            ITEMS,
            Content::new(
                PANEL,
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: Spacing::Regular.value(),
                    },
                    flexible_child: None,
                    children: vec![
                        Content::new(
                            PANEL_TEXT,
                            TextContent::new("Nested content panel · 工具详情"),
                        ),
                        Content::new(ComponentId(9710), ButtonContent::new("Inspect details")),
                        Content::new(ComponentId(9711), SwitchContent::new("Show details", true)),
                        Content::new(
                            INLINE_EDITOR,
                            TextFieldContent::new("Inline notes", "Local note"),
                        ),
                    ],
                },
            ),
        )?;
        let mut keys = s.item_keys();
        keys.retain(|id| *id != PANEL);
        keys.insert(8, PANEL);
        s.host.reorder(ITEMS, &keys)?;
        Ok(s)
    }
    pub fn item_keys(&self) -> Vec<ComponentId> {
        self.host
            .get(ITEMS)
            .unwrap()
            .component::<ListContent>()
            .unwrap()
            .rows
            .children
            .iter()
            .map(Content::id)
            .collect()
    }
    /// A real structured document opts in; ordinary input continuity fixtures
    /// continue to use new() without any reading policy.
    pub fn attach_reading(
        &mut self,
        key: &str,
        revision: u64,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        let items = self.item_keys();
        self.host
            .set_scroll_reading_document(SCROLLER, key, revision, items, |h| {
                if h.get(LATEST).is_none() {
                    h.insert(
                        ROOT,
                        Content::new(LATEST, ButtonContent::new("Return to latest"))
                            .preserving_focus(),
                    )
                    .unwrap();
                    h.reorder(ROOT, &[SCROLLER, LATEST, COMPOSER]).unwrap();
                }
                h.set_action_events(true);
            })
            .map(|(version, ())| version)
    }
    pub fn process_actions(&mut self) {
        for action in self.host.drain_actions() {
            if action.target == LATEST && action.kind == ContentActionKind::Activate {
                self.host
                    .get_mut(SCROLLER)
                    .unwrap()
                    .component_mut::<ScrollContent>()
                    .unwrap()
                    .follow_latest();
            }
        }
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let response = self.host.event(event);
        self.process_actions();
        response
    }
    pub fn accessibility_action(
        &mut self,
        id: ComponentId,
        action: AccessibilityAction,
    ) -> ContentResponse {
        let response = self.host.accessibility_action(id, None, action);
        self.process_actions();
        response
    }
    /// Retain expected at request time. No source mutation runs for an old
    /// response. The caller supplies keys/order, never a pixel correction.
    pub fn insert_versioned(
        &mut self,
        expected: &ReadingDocumentVersion,
        prepend: bool,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        let id = ComponentId(self.next);
        let mut order = self.item_keys();
        if prepend {
            order.insert(0, id);
        } else {
            order.push(id);
        }
        let revision = expected
            .revision()
            .checked_add(1)
            .ok_or(ReadingUpdateError::InvalidDocument)?;
        let content = Content::new(
            id,
            TextContent {
                text: format!("Page {} · mixed Content document update", self.next),
                role: proposal::TextRole::UiBody,
            },
        );
        let (version, ()) = self.host.update_scroll_reading_document(
            SCROLLER,
            expected,
            revision,
            order.clone(),
            |h| {
                h.insert(ITEMS, content).unwrap();
                h.reorder(ITEMS, &order).unwrap();
            },
        )?;
        self.next += 1;
        Ok(version)
    }
    pub fn toggle_top_versioned(
        &mut self,
        expected: &ReadingDocumentVersion,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        let expanded = !self.expanded;
        let value = top_text(expanded);
        let revision = expected
            .revision()
            .checked_add(1)
            .ok_or(ReadingUpdateError::InvalidDocument)?;
        let items = self.item_keys();
        let (version, ()) =
            self.host
                .update_scroll_reading_document(SCROLLER, expected, revision, items, |h| {
                    h.get_mut(TOP)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .text = value;
                })?;
        self.expanded = expanded;
        Ok(version)
    }
    pub fn new() -> Result<Self, CompositionError> {
        let mut children = vec![
            Content::new(
                TOP,
                TextContent {
                    text: "Live document · 混合内容".into(),
                    role: proposal::TextRole::UiHeading,
                },
            ),
            Content::new(TOGGLE, SwitchContent::new("Keep local draft", true)),
            Content::new(ACTION, ButtonContent::new("Inspect item")),
        ];
        for index in 0..20 {
            children.push(Content::new(ComponentId(9720 + index), TextContent {
                text: format!("Item {index:02} · 内容、面板与动作共用真实滚动容器。 Reflow must preserve a valid native scroll sequence."),
                role: proposal::TextRole::UiBody,
            }));
        }
        let scroll = ScrollContent::new(
            ScrollArea::default(),
            Content::new(ITEMS, ListContent::new("Mixed document", children)),
        );
        Ok(Self {
            host: ContentHost::new(Content::new(
                ROOT,
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: Spacing::Regular.value(),
                    },
                    flexible_child: Some(SCROLLER),
                    children: vec![
                        Content::new(SCROLLER, scroll),
                        Content::new(
                            COMPOSER,
                            TextFieldContent::new("Independent draft", "Draft A😀B"),
                        ),
                    ],
                },
            ))?,
            next: 9800,
            expanded: false,
        })
    }
    pub fn toggle_top(&mut self) {
        self.expanded = !self.expanded;
        self.host
            .get_mut(TOP)
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = top_text(self.expanded);
    }
    pub fn append(&mut self) -> Result<(), CompositionError> {
        self.host.insert(
            ITEMS,
            Content::new(
                ComponentId(self.next),
                TextContent {
                    text: format!("Appended {} · streaming content update", self.next),
                    role: proposal::TextRole::UiBody,
                },
            ),
        )?;
        self.next += 1;
        Ok(())
    }
}

fn top_text(expanded: bool) -> String {
    if expanded {
        "Expanded document header · 上方内容更新不会结束滚动。\n".repeat(8)
    } else {
        "Live document · 混合内容".into()
    }
}
