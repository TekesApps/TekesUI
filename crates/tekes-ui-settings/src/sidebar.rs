#[path = "sidebar_list.rs"]
mod sidebar_list;
use crate::Schema;
use anyhow::Result;
pub use sidebar_list::SelectableListContent;
use std::collections::BTreeMap;
use tekes_ui_controls::{
    ButtonTextRenderer, ScrollArea,
    composition::{
        ComponentId, Content, ContentEvent, ContentHost, ScrollContent, StackContent, TextContent,
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
}
impl SettingsSidebar {
    pub fn new(schema: &Schema) -> Result<Self> {
        let mut children = vec![];
        let mut items = BTreeMap::new();
        let mut next = 100;
        for group in &schema.groups {
            children.push(Content::new(
                ComponentId(next),
                TextContent::new(&group.label),
            ));
            next += 1;
            let id = ComponentId(next);
            next += 1;
            children.push(Content::new(
                id,
                SelectableListContent::new(
                    &group.label,
                    group.items.iter().map(|item| item.label.clone()).collect(),
                ),
            ));
            items.insert(
                id,
                group
                    .items
                    .iter()
                    .map(|item| format!("{}/{}", group.id, item.id))
                    .collect(),
            );
        }
        let rows = Content::new(
            ComponentId(2),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 12.,
                },
                children,
                flexible_child: None,
            },
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            ScrollContent::new(ScrollArea::default(), rows),
        ))
        .map_err(|e| anyhow::anyhow!("Settings sidebar: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            items,
            selected_item: None,
        })
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
        self.host
            .render(scene, text, frame, frame, palette)
            .map_err(|e| anyhow::anyhow!("Settings sidebar: {e:?}"))
    }
    pub fn event(&mut self, event: &ContentEvent) -> Option<String> {
        self.host.event(event);
        self.take_selection()
    }
    pub fn take_selection(&mut self) -> Option<String> {
        self.host.drain_actions().into_iter().find_map(|a| {
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
