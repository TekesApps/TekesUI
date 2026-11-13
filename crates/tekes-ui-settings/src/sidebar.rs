#[path = "sidebar_list.rs"]
mod sidebar_list;
use crate::Schema;
use anyhow::Result;
pub use sidebar_list::SelectableListContent;
use std::collections::BTreeMap;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        Button, ComponentId, Content, ContentActionPayload, ContentEvent, ContentHost, IconContent,
        KeyedSelectableListContent, ListContent, ListDisplayUpdate, ListIntentKind, ListRowDisplay,
        StackContent, TextContent, TextFieldContent,
    },
    extended::{
        Axis, KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey, Stack,
    },
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Palette;
#[derive(Debug)]
struct SidebarItem {
    key: ListItemKey,
    row_id: ComponentId,
    route: String,
    label: String,
    terms: String,
}
#[derive(Debug)]
struct SidebarGroup {
    dataset: ListDatasetKey,
    items: Vec<SidebarItem>,
}
impl SidebarGroup {
    fn snapshot(&self, query: &str) -> KeyedListSnapshot {
        KeyedListSnapshot {
            dataset: self.dataset,
            items: self
                .items
                .iter()
                .map(|item| KeyedListItem {
                    key: item.key,
                    name: item.label.clone(),
                })
                .collect(),
            display_items: self
                .items
                .iter()
                .filter(|item| item.terms.contains(query))
                .map(|item| item.key)
                .collect(),
        }
    }
    fn rows(&self) -> Vec<ListRowDisplay> {
        self.items
            .iter()
            .map(|item| ListRowDisplay {
                key: item.key,
                row_id: item.row_id,
                display: ListDisplayUpdate::Keep,
            })
            .collect()
    }
}
/// Definition-driven Group -> Item navigation. History remains owned by the Host service.
#[derive(Debug)]
pub struct SettingsSidebar {
    pub host: ContentHost,
    items: BTreeMap<ComponentId, SidebarGroup>,
    selected_item: Option<String>,
    back_requested: bool,
    back_template: Option<&'static str>,
    search_button: Option<Content>,
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
        let mut items = BTreeMap::new();
        // IDs are registered once for this immutable schema, never reconstructed
        // from a filtered row ordinal or its display label.
        let mut next = 100;
        let mut next_key = 1;
        let mut declarations = Vec::new();
        for (dataset, group) in schema.groups.iter().enumerate() {
            let heading_id = ComponentId(next);
            let list_id = ComponentId(next + 1);
            next += 2;
            declarations.push((
                heading_id,
                list_id,
                ListDatasetKey(dataset as u64 + 1),
                group,
            ));
        }
        let mut groups = Vec::new();
        for (heading_id, id, dataset, group) in declarations {
            let mut heading = TextContent::new(&group.label);
            heading.role = tekes_ui_tokens::proposal::TextRole::UiCaption;
            let mut entries = Vec::new();
            for item in &group.items {
                let mut terms = format!("{} {}", group.label, item.label);
                for field in &item.fields {
                    terms.push_str(&format!(" {} {}", field.label, field.description));
                }
                entries.push(SidebarItem {
                    key: ListItemKey(next_key),
                    row_id: ComponentId(next),
                    route: format!("{}/{}", group.id, item.id),
                    label: item.label.clone(),
                    terms: terms.to_lowercase(),
                });
                next_key += 1;
                next += 1;
            }
            let group_model = SidebarGroup {
                dataset,
                items: entries,
            };
            let snapshot = group_model.snapshot("");
            let mut list = KeyedSelectableListContent::new(
                &group.label,
                &snapshot,
                &mut group_model.rows(),
                ListBudget {
                    max_items: snapshot.items.len().max(1),
                    max_display_items: snapshot.items.len().max(1),
                    // Collection transactions include the surrounding sidebar
                    // and retained/staged text, not just visible row labels.
                    max_utf8_bytes: 1_000_000,
                    max_staging_units: 100_000,
                },
            )
            .map_err(|error| anyhow::anyhow!("Settings list: {error:?}"))?;
            list.set_navigation_selection_fill_only(true);
            list.set_navigation_layout(true);
            items.insert(id, group_model);
            groups.push(Content::new(
                ComponentId(next),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: tekes_ui_tokens::Spacing::Regular.value(),
                    },
                    children: vec![Content::new(heading_id, heading), Content::new(id, list)],
                    flexible_child: None,
                },
            ));
            next += 1;
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
            let mut search_button = Button::new(Content::new(
                ComponentId(11),
                IconContent::new("tekes-search")
                    .map_err(|error| anyhow::anyhow!("Settings search icon: {error:?}"))?,
            ))
            .with_accessible_name("搜索设置")
            .with_help("搜索设置");
            search_button.set_toolbar_icon_slot(true);
            Content::new(
                ComponentId(3),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: tekes_ui_tokens::Spacing::Relaxed.value(),
                    },
                    children: vec![
                        StackContent {
                            model: Stack {
                                axis: Axis::Horizontal,
                                gap: tekes_ui_tokens::Spacing::Regular.value(),
                            },
                            children: vec![
                                Content::new(ComponentId(4), back_button),
                                Content::new(ComponentId(10), search_button),
                            ],
                            flexible_child: Some(ComponentId(4)),
                        }
                        .into_content_with_distribution(
                            ComponentId(5),
                            tekes_ui_tokens::proposal::layout::MainAxisDistribution::SpaceBetween,
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
        let header = self.host.get_mut(ComponentId(5)).unwrap();
        *header.stack_distribution_mut().unwrap() =
            tekes_ui_tokens::proposal::layout::MainAxisDistribution::Start;
        header
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
        let header = self.host.get_mut(ComponentId(5)).unwrap();
        *header.stack_distribution_mut().unwrap() =
            tekes_ui_tokens::proposal::layout::MainAxisDistribution::SpaceBetween;
        header
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
        for (id, group) in &self.items {
            self.host
                .replace_keyed_list(*id, &group.snapshot(&self.query), &mut group.rows(), None)
                .map_err(|error| anyhow::anyhow!("Settings search rows: {error:?}"))?;
        }
        if let Some(selected) = self.selected_item.clone() {
            self.select(&selected);
        }
        Ok(())
    }
    pub fn take_back(&mut self) -> bool {
        std::mem::take(&mut self.back_requested)
    }
    pub fn select(&mut self, item: &str) {
        self.selected_item = Some(item.into());
        for (id, group) in &self.items {
            let key = group
                .items
                .iter()
                .find(|entry| entry.route == item)
                .map(|entry| entry.key);
            self.host
                .set_keyed_list_selection(*id, key)
                .expect("declared Settings list");
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
        let actions = self.host.drain_action_records();
        self.back_requested |= actions.iter().any(|a| a.action.target == ComponentId(4));
        if actions.iter().any(|a| a.action.target == ComponentId(10)) {
            // Both nodes and their parent are owned by this fixed declaration.
            self.open_search().expect("declared Settings search slots");
        }
        actions.into_iter().find_map(|record| {
            let Some(ContentActionPayload::ListIntent(intent)) = record.payload else {
                return None;
            };
            let key = match intent.kind {
                ListIntentKind::SelectionChanged => intent.selected_key?,
                ListIntentKind::Activated => intent.key,
            };
            self.items
                .get(&record.action.target)?
                .items
                .iter()
                .find(|item| item.key == key)
                .map(|item| &item.route)
                .filter(|route| self.selected_item.as_ref() != Some(route))
                .cloned()
        })
    }
}
