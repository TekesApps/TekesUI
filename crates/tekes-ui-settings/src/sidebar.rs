#[path = "sidebar_list.rs"]
mod sidebar_list;
use crate::Schema;
use anyhow::Result;
pub use sidebar_list::SelectableListContent;
use std::collections::BTreeMap;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        Button, ComponentId, Content, ContentEvent, ContentHost, IconContent, ListContent,
        StackContent, TextContent, TextFieldContent,
    },
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Palette;
/// Definition-driven Group -> Item navigation. History remains owned by the Host service.
#[derive(Debug)]
pub struct SettingsSidebar {
    pub host: ContentHost,
    items: BTreeMap<ComponentId, Vec<String>>,
    selected_item: Option<String>,
    back_requested: bool,
    back_template: Option<&'static str>,
    search_button: Option<Content>,
    search_entries: BTreeMap<ComponentId, Vec<(String, String, String)>>,
    query: String,
}
impl SettingsSidebar {
    pub fn new(schema: &Schema) -> Result<Self> {
        Self::build(schema, false)
    }
    /// A fixed ordinary Button above the independently scrolling navigation list.
    pub fn new_with_back(schema: &Schema) -> Result<Self> {
        Self::build(schema, true)
    }
    fn build(schema: &Schema, back: bool) -> Result<Self> {
        let mut group_children = vec![];
        let mut items = BTreeMap::new();
        let mut search_entries = BTreeMap::new();
        let mut next = 100;
        for group in &schema.groups {
            let mut heading = TextContent::new(&group.label);
            heading.role = tekes_ui_tokens::proposal::TextRole::UiCaption;
            let heading = Content::new(ComponentId(next), heading);
            next += 1;
            let id = ComponentId(next);
            next += 1;
            let mut list = SelectableListContent::new(
                &group.label,
                group.items.iter().map(|item| item.label.clone()).collect(),
            );
            list.view.set_navigation_selection_fill_only(true);
            list.view.set_navigation_layout(true);
            group_children.push((heading, Content::new(id, list)));
            search_entries.insert(
                id,
                group
                    .items
                    .iter()
                    .map(|item| {
                        let mut terms = format!("{} {}", group.label, item.label);
                        for field in &item.fields {
                            terms.push_str(&format!(" {} {}", field.label, field.description));
                        }
                        (
                            format!("{}/{}", group.id, item.id),
                            item.label.clone(),
                            terms.to_lowercase(),
                        )
                    })
                    .collect(),
            );
            items.insert(
                id,
                group
                    .items
                    .iter()
                    .map(|item| format!("{}/{}", group.id, item.id))
                    .collect(),
            );
        }
        let mut groups = Vec::with_capacity(group_children.len());
        for (heading, list) in group_children {
            let id = ComponentId(next);
            next += 1;
            groups.push(Content::new(
                id,
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: tekes_ui_tokens::Spacing::Regular.value(),
                    },
                    children: vec![heading, list],
                    flexible_child: None,
                },
            ));
        }
        let groups = Content::new(
            ComponentId(next),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: tekes_ui_tokens::Spacing::Micro.value(),
                },
                children: groups,
                flexible_child: None,
            },
        );
        let scroll = Content::new(
            ComponentId(1),
            ListContent::scrollable("Settings", ComponentId(2), vec![groups]),
        );
        let root = if back {
            let mut back_button = Button::new("〈 返回")
                .with_accessible_name("返回主窗口")
                .with_help("返回主窗口");
            back_button.leading_content_inset = Some(0.);
            back_button.content_alignment =
                tekes_ui_tokens::proposal::layout::MainAxisDistribution::Start;
            Content::new(
                ComponentId(3),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: tekes_ui_tokens::Spacing::Relaxed.value(),
                    },
                    children: vec![
                        Content::new(
                            ComponentId(5),
                            StackContent {
                                model: Stack {
                                    axis: Axis::Horizontal,
                                    gap: tekes_ui_tokens::Spacing::Regular.value(),
                                },
                                children: vec![
                                    Content::new(ComponentId(4), back_button),
                                    Content::new(
                                        ComponentId(10),
                                        Button::new(Content::new(
                                            ComponentId(11),
                                            IconContent::new("tekes-search").map_err(|error| {
                                                anyhow::anyhow!("Settings search icon: {error:?}")
                                            })?,
                                        ))
                                        .with_accessible_name("搜索设置")
                                        .with_help("搜索设置"),
                                    ),
                                ],
                                flexible_child: Some(ComponentId(4)),
                            },
                        ),
                        scroll,
                    ],
                    flexible_child: Some(ComponentId(1)),
                },
            )
        } else {
            scroll
        };
        let mut host =
            ContentHost::new(root).map_err(|e| anyhow::anyhow!("Settings sidebar: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            items,
            selected_item: None,
            back_requested: false,
            back_template: None,
            search_button: None,
            search_entries,
            query: String::new(),
        })
    }
    pub fn set_back_icon(&mut self, image: tekes_ui_scene::ImageId) {
        self.back_template = None;
        if let Some(button) = self
            .host
            .get_mut(ComponentId(4))
            .and_then(|n| n.component_mut::<Button>())
        {
            button.set_contents(image);
        }
    }
    /// A canonical monochrome Back icon inherits its original Button's state ink.
    /// Repeated declarations retain the same passive content and image lease.
    pub fn set_back_template_icon(&mut self, name: &str) -> Result<()> {
        let icon = IconContent::new(name)
            .map_err(|error| anyhow::anyhow!("Settings Back icon: {error:?}"))?;
        let name = icon.glyph().name;
        if self.back_template == Some(name) {
            return Ok(());
        }
        if let Some(button) = self
            .host
            .get_mut(ComponentId(4))
            .and_then(|node| node.component_mut::<Button>())
        {
            button.set_contents(Content::new(
                ComponentId(7),
                StackContent {
                    model: Stack {
                        axis: Axis::Horizontal,
                        gap: tekes_ui_tokens::Spacing::Tight.value(),
                    },
                    children: vec![
                        Content::new(ComponentId(8), icon),
                        Content::new(ComponentId(9), TextContent::new("返回")),
                    ],
                    flexible_child: None,
                },
            ));
            self.back_template = Some(name);
        }
        Ok(())
    }
    fn open_search(&mut self) -> Result<()> {
        self.search_button = Some(
            self.host
                .remove(ComponentId(10))
                .map_err(|error| anyhow::anyhow!("Settings search: {error:?}"))?,
        );
        self.host
            .insert(
                ComponentId(5),
                Content::new(
                    ComponentId(6),
                    TextFieldContent::inline("搜索设置", "", 20.),
                )
                .with_help("搜索设置"),
            )
            .map_err(|error| anyhow::anyhow!("Settings search: {error:?}"))?;
        self.host
            .get_mut(ComponentId(5))
            .unwrap()
            .component_mut::<StackContent>()
            .unwrap()
            .flexible_child = Some(ComponentId(6));
        self.host.focus(Some(ComponentId(6)), true);
        Ok(())
    }
    /// Escape belongs to the original search editor, before window Back/Cancel.
    /// Marked text is left to the original IME cancellation path.
    pub fn cancel_search(&mut self) -> Result<bool> {
        if self.search_button.is_none() || self.host.is_composing() {
            return Ok(false);
        }
        self.host
            .remove(ComponentId(6))
            .map_err(|error| anyhow::anyhow!("Settings search: {error:?}"))?;
        self.host
            .insert(ComponentId(5), self.search_button.take().unwrap())
            .map_err(|error| anyhow::anyhow!("Settings search: {error:?}"))?;
        self.host
            .get_mut(ComponentId(5))
            .unwrap()
            .component_mut::<StackContent>()
            .unwrap()
            .flexible_child = Some(ComponentId(4));
        self.host.focus(Some(ComponentId(10)), true);
        self.sync_search()?;
        Ok(true)
    }
    /// Update the original navigation rows from committed search text. Each
    /// visible row retains its original route; searching never edits the draft.
    pub fn sync_search(&mut self) -> Result<()> {
        if self.host.is_composing() {
            return Ok(());
        }
        let query = self
            .host
            .get(ComponentId(6))
            .and_then(|node| node.component::<TextFieldContent>())
            .map(|field| field.surface.input.text.trim().to_lowercase())
            .unwrap_or_default();
        if self.search_button.is_some()
            && query.is_empty()
            && self.host.focused() != Some(ComponentId(6))
            && !self.host.is_composing()
        {
            self.cancel_search()?;
            return Ok(());
        }
        if query == self.query {
            return Ok(());
        }
        self.query = query;
        for (id, entries) in &self.search_entries {
            let matching: Vec<_> = entries
                .iter()
                .filter(|(_, _, terms)| terms.contains(&self.query))
                .collect();
            let list = self
                .host
                .get_mut(*id)
                .unwrap()
                .component_mut::<SelectableListContent>()
                .unwrap();
            list.view.cancel();
            list.labels = matching.iter().map(|(_, label, _)| label.clone()).collect();
            list.model.count = matching.len();
            list.model.selected = matching
                .iter()
                .position(|(route, _, _)| self.selected_item.as_ref() == Some(route));
            self.items.insert(
                *id,
                matching.iter().map(|(route, _, _)| route.clone()).collect(),
            );
        }
        Ok(())
    }
    pub fn take_back(&mut self) -> bool {
        std::mem::take(&mut self.back_requested)
    }
    pub fn select(&mut self, item: &str) {
        self.selected_item = Some(item.into());
        for (id, values) in &self.items {
            let list = self
                .host
                .get_mut(*id)
                .unwrap()
                .component_mut::<SelectableListContent>()
                .unwrap();
            list.model.selected = values.iter().position(|value| value == item);
        }
    }

    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        palette: Palette,
    ) -> Result<()> {
        self.sync_search()?;
        self.host
            .render(scene, text, frame, frame, palette)
            .map_err(|e| anyhow::anyhow!("Settings sidebar: {e:?}"))
    }
    pub fn event(&mut self, event: &ContentEvent) -> Option<String> {
        self.host.event(event);
        self.take_selection()
    }
    pub fn take_selection(&mut self) -> Option<String> {
        let actions = self.host.drain_actions();
        self.back_requested |= actions.iter().any(|a| a.target == ComponentId(4));
        if actions.iter().any(|a| a.target == ComponentId(10)) {
            // Both nodes and their parent are owned by this fixed declaration.
            self.open_search().expect("declared Settings search slots");
        }
        actions.into_iter().find_map(|a| {
            let list = self
                .host
                .get(a.target)?
                .component::<SelectableListContent>()?;
            self.items
                .get(&a.target)?
                .get(list.model.selected?)
                .filter(|item| self.selected_item.as_ref() != Some(item))
                .cloned()
        })
    }
}
