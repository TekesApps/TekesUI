#[path = "sidebar_list.rs"]
mod sidebar_list;
use crate::Schema;
use anyhow::Result;
pub use sidebar_list::SelectableListContent;
use std::collections::BTreeMap;
use tekes_ui_controls::{
    ButtonTextRenderer, ScrollArea,
    composition::{
        Button, ComponentId, Content, ContentEvent, ContentHost, ScrollContent, StackContent, TextContent, TextFieldContent,
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
        let scroll = Content::new(ComponentId(1), ScrollContent::new(ScrollArea::default(), rows));
        let root = if back {
            Content::new(ComponentId(3), StackContent {
                model: Stack { axis: Axis::Vertical, gap: 12. },
                children: vec![
                    Content::new(ComponentId(5), StackContent {
                        model: Stack { axis: Axis::Horizontal, gap: 12. },
                        children: vec![
                            Content::new(ComponentId(4), Button::new("Back").with_accessible_name("Back").with_help("返回主窗口")),
                            Content::new(ComponentId(6), TextFieldContent::inline("Search", "", 20.)).with_help("Search"),
                        ],
                        flexible_child: Some(ComponentId(6)),
                    }),
                    scroll,
                ],
                flexible_child: Some(ComponentId(1)),
            })
        } else { scroll };
        let mut host = ContentHost::new(root)
        .map_err(|e| anyhow::anyhow!("Settings sidebar: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            items,
            selected_item: None,
            back_requested: false,
        })
    }
    pub fn set_back_icon(&mut self, image: tekes_ui_scene::ImageId) {
        if let Some(button) = self.host.get_mut(ComponentId(4)).and_then(|n| n.component_mut::<Button>()) {
            button.set_contents(image);
        }
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
