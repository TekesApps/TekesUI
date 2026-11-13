//! Master document transport; keyed List and ScrollContent retain all interaction state.
use super::search_host::native_projection::NativeProjection;
use super::*;
use tekes_ui_controls::{
    composition::*,
    extended::{KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey},
};
use tekes_ui_platform_macos::{
    AccessibilityAction as A, AccessibilityKey as K, AccessibilityNode as N,
};

const SCROLL: ComponentId = ComponentId(27401);
const ITEMS: ComponentId = ComponentId(27402);
pub(super) const LIST: ComponentId = ComponentId(27403);
fn row_id(index: usize) -> ComponentId {
    ComponentId(27500 + index as u64)
}

#[derive(Debug)]
pub(super) struct MasterHost {
    pub(super) host: ContentHost,
    projection: std::cell::RefCell<NativeProjection>,
}
impl Default for MasterHost {
    fn default() -> Self {
        let snapshot = KeyedListSnapshot {
            dataset: ListDatasetKey(274),
            items: (0..80)
                .map(|n| KeyedListItem {
                    key: ListItemKey(n + 1),
                    name: format!("Document {:02}", n + 1),
                })
                .collect(),
            display_items: (1..=80).map(ListItemKey).collect(),
        };
        let mut rows = snapshot
            .items
            .iter()
            .enumerate()
            .map(|(n, item)| ListRowDisplay {
                key: item.key,
                row_id: row_id(n),
                display: ListDisplayUpdate::DefaultLabel,
            })
            .collect();
        let mut list = SelectableListContent::keyed(
            "Documents",
            &snapshot,
            &mut rows,
            ListBudget {
                max_items: 80,
                max_display_items: 80,
                // Fixed fixture allowance for retained/staged text across 80 labels.
                // 4 KiB failed the original List preflight; retain its budget checks.
                max_utf8_bytes: 8 * 1024,
                max_staging_units: 8192,
            },
        )
        .expect("fixed document dataset");
        list.select_key(ListItemKey(1))
            .expect("initial document key");
        let mut host = ContentHost::new(Content::new(
            SCROLL,
            ListContent::scrollable("Documents", ITEMS, vec![Content::new(LIST, list)]),
        ))
        .expect("fixed master List structure");
        host.set_action_events(true);
        host.set_focus_wrap(false);
        Self {
            host,
            projection: Default::default(),
        }
    }
}
impl MasterHost {
    pub(super) fn list(&self) -> &KeyedSelectableListContent {
        self.host
            .get(LIST)
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
    }
    pub(super) fn scroll(&self) -> &tekes_ui_controls::ScrollArea {
        &self
            .host
            .get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
    }
    pub(super) fn scroll_mut(&mut self) -> &mut tekes_ui_controls::ScrollArea {
        &mut self
            .host
            .get_mut(SCROLL)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
    }
    pub(super) fn row(&self, index: usize) -> &Content {
        self.host.get(row_id(index)).unwrap()
    }
    pub(super) fn focus(&mut self, focused: bool, visible: bool) {
        self.host.focus(focused.then_some(LIST), visible);
    }
    pub(super) fn paint(
        &mut self,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        frame: Rect,
        clip: Rect,
        brand: usize,
    ) {
        self.host
            .render(
                scene,
                ctx.text,
                frame,
                clip,
                tekes_ui_tokens::Palette::new(ctx.theme, brand).expect("Gallery brand"),
            )
            .expect("master List layout");
    }
    pub(super) fn handle(&mut self, event: &Event, bounds: Rect) -> Option<ContentResponse> {
        let input = match event {
            Event::Mouse(m) => {
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                if !bounds.contains(p)
                    && self.host.pointer_capture().is_none()
                    && !matches!(m.kind, MouseEventKind::Moved | MouseEventKind::Exited)
                {
                    return None;
                }
                crate::reading_pointer::translate(*m)
            }
            Event::Key(k) if self.host.focused().is_some() && k.key_code != 48 => {
                ContentEvent::KeyInput(crate::reading_key::key(k.clone()))
            }
            Event::Scroll(e)
                if bounds.contains(Point::new(e.position.x as f32, e.position.y as f32)) =>
            {
                let wheel = tekes_ui_module_runtime::content_wheel_event(e)?;
                return Some(self.host.wheel_event(&wheel));
            }
            _ => return None,
        };
        Some(self.host.event(&input))
    }
    pub(super) fn drain_documents(&mut self) -> Vec<usize> {
        self.host
            .drain_action_records()
            .into_iter()
            .filter_map(|record| {
                let Some(ContentActionPayload::ListIntent(intent)) = record.payload else {
                    return None;
                };
                let key = match intent.kind {
                    ListIntentKind::SelectionChanged => intent.selected_key?,
                    ListIntentKind::Activated => intent.key,
                };
                Some((key.0 - 1) as usize)
            })
            .collect()
    }
    pub(super) fn native_nodes(&self) -> Vec<N> {
        self.projection.borrow_mut().nodes(&self.host)
    }
    pub(super) fn owns_native_key(&self, key: K) -> bool {
        self.projection.borrow_mut().owns_key(&self.host, key)
    }
    pub(super) fn native_action(&mut self, key: K, action: A) -> ContentResponse {
        self.projection
            .borrow_mut()
            .action(&mut self.host, key, action)
    }
}
