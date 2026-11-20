//! JSON-owned sections and items. Page IDs stay stable; visual order is independent.
use super::Page;
use std::sync::OnceLock;
use tekes_ui_controls::{
    composition::{
        ComponentId, Content, KeyedSelectableListContent, ListContent, ListDisplayUpdate,
        ListRowDisplay,
    },
    extended::{KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey},
};

pub(super) const SCROLL: ComponentId = ComponentId(0x0047_414c_0010);
pub(super) const LIST: ComponentId = ComponentId(0x0047_414c_0011);
const GROUP_BASE: u64 = 0x0047_414c_0100;
const HEADER_BASE: u64 = 0x0047_414c_0200;
const ITEMS_BASE: u64 = 0x0047_414c_0300;

#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
    pub items: Vec<Page>,
}

pub fn sections() -> &'static [Section] {
    static SECTIONS: OnceLock<Vec<Section>> = OnceLock::new();
    SECTIONS.get_or_init(|| {
        let mut all: Vec<Section> = Vec::new();
        for (i, entry) in super::workbench::entries().iter().enumerate() {
            let title = entry.section();
            if all.last().is_none_or(|s| s.title != title) {
                all.push(Section {
                    title: title.into(),
                    items: Vec::new(),
                });
            }
            all.last_mut().unwrap().items.push(Page::Workbench(i));
        }
        all
    })
}

pub(super) struct Navigation;
impl Navigation {
    pub(super) fn list_id(section: &Section) -> ComponentId {
        let index = sections()
            .iter()
            .position(|candidate| candidate.title == section.title)
            .expect("filtered section retains original identity");
        ComponentId(ITEMS_BASE + index as u64)
    }

    pub(super) fn group_id(section: &Section) -> ComponentId {
        ComponentId(GROUP_BASE + section_slot(&section.title) as u64)
    }

    pub(super) fn filter_sections(query: &str) -> Vec<Section> {
        let q = query.trim().to_lowercase();
        let mut result = Vec::new();
        for section in sections() {
            let matched: Vec<Page> = section
                .items
                .iter()
                .copied()
                .filter(|p| {
                    let Page::Workbench(i) = p;
                    super::workbench::entry(*i).matches(&q)
                })
                .collect();
            if matched.is_empty() {
                continue;
            }
            result.push(Section {
                title: section.title.clone(),
                items: matched,
            });
        }
        result
    }
}

fn section_slot(title: &str) -> usize {
    sections()
        .iter()
        .position(|section| section.title == title)
        .expect("Gallery section has stable identity")
}

fn page_key(page: Page) -> ListItemKey {
    let Page::Workbench(index) = page;
    ListItemKey(index as u64 + 1)
}

pub(super) fn section_content(section: &Section, selected: Page) -> Content {
    let slot = section_slot(&section.title) as u64;
    let snapshot = KeyedListSnapshot {
        dataset: ListDatasetKey(ITEMS_BASE + slot),
        items: section
            .items
            .iter()
            .map(|&page| KeyedListItem {
                key: page_key(page),
                name: page.name().to_owned(),
            })
            .collect(),
        display_items: section.items.iter().map(|&page| page_key(page)).collect(),
    };
    let mut rows = snapshot
        .items
        .iter()
        .map(|item| ListRowDisplay {
            key: item.key,
            row_id: ComponentId(0x0047_414d_0000 + item.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect();
    let mut list = KeyedSelectableListContent::new(
        &section.title,
        &snapshot,
        &mut rows,
        ListBudget {
            max_items: Page::ALL.len(),
            max_display_items: Page::ALL.len(),
            max_utf8_bytes: 1_000_000,
            max_staging_units: 100_000,
        },
    )
    .expect("bounded Gallery catalog navigation");
    list.set_navigation_selection_fill_only(true);
    list.set_navigation_layout(true);
    list.select_key(page_key(selected)).unwrap();
    ListContent::section(
        ComponentId(GROUP_BASE + slot),
        ComponentId(HEADER_BASE + slot),
        section.title.clone(),
        Content::new(ComponentId(ITEMS_BASE + slot), list),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{layout, text::TextEngine};
    use tekes_ui_controls::{
        ControlKey, ScrollArea, ScrollGeometry,
        composition::{
            AccessibilityAction, Component, ContentActionPayload, ContentEvent, ContentHost,
            ContentResponse, ListIntentKind, ScrollContent, SearchFieldContent, TextContent,
        },
    };
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Theme, proposal};

    const ROOT: ComponentId = ComponentId(10);
    const HEADER: ComponentId = ComponentId(11);
    const FILTER: ComponentId = ComponentId(1);

    struct NavHarness {
        host: ContentHost,
        height: f32,
        query: String,
        filtered: Vec<Section>,
        selected: Page,
    }

    impl NavHarness {
        fn new() -> Self {
            let selected = Page::Workbench(0);
            let section_contents = sections()
                .iter()
                .map(|section| section_content(section, selected))
                .collect();
            let root = ListContent::scrollable_with_header(
                ROOT,
                HEADER,
                SCROLL,
                LIST,
                "Components",
                Content::new(FILTER, SearchFieldContent::new("Filter Gallery pages", "")),
                section_contents,
            );
            let mut host = ContentHost::new(root).expect("Gallery navigation ContentHost");
            host.set_action_events(true);
            Self {
                host,
                height: layout::FRAME_H,
                query: String::new(),
                filtered: Vec::new(),
                selected,
            }
        }

        fn sections(&self) -> &[Section] {
            if self.query.trim().is_empty() {
                sections()
            } else {
                &self.filtered
            }
        }

        fn pages(&self) -> Vec<Page> {
            self.sections()
                .iter()
                .flat_map(|s| s.items.iter().copied())
                .collect()
        }

        fn render(&mut self, text: &mut TextEngine) -> Scene {
            let mut scene = Scene::new();
            let frame = Rect::new(0.0, 0.0, layout::CATALOG_W, self.height);
            self.host
                .render(&mut scene, text, frame, frame, Theme::LIGHT.into())
                .expect("Gallery navigation host renders");
            scene
        }

        fn scroll(&self) -> &ScrollArea {
            &self
                .host
                .get(SCROLL)
                .expect("SCROLL")
                .component::<ScrollContent>()
                .expect("ScrollContent")
                .scroll
        }

        fn scroll_geometry(&self) -> ScrollGeometry {
            let viewport = self.host.get(SCROLL).expect("SCROLL").frame();
            self.scroll().geometry(viewport, Some(viewport))
        }

        fn scroll_by(&mut self, delta: Point) {
            self.host
                .get_mut(SCROLL)
                .unwrap()
                .component_mut::<ScrollContent>()
                .unwrap()
                .scroll
                .scroll_by(delta.x, delta.y);
        }

        fn set_query(&mut self, query: &str, text: &mut TextEngine) {
            if self.query == query {
                return;
            }
            self.host.cancel();
            for section in self.sections().to_vec() {
                self.host
                    .remove(Navigation::group_id(&section))
                    .expect("mounted group");
            }
            self.query = query.to_owned();
            self.filtered = if query.trim().is_empty() {
                Vec::new()
            } else {
                Navigation::filter_sections(query)
            };
            for section in self.sections().to_vec() {
                self.host
                    .insert(LIST, section_content(&section, self.selected))
                    .expect("insert group");
            }
            self.render(text);
        }

        fn section_for_page(&self, page: Page) -> Option<(&Section, usize)> {
            self.sections().iter().find_map(|section| {
                section
                    .items
                    .iter()
                    .position(|candidate| *candidate == page)
                    .map(|index| (section, index))
            })
        }

        fn row_frames(&self) -> Vec<(Page, Rect)> {
            let nodes = self.host.semantics();
            let Some(clip) = self.scroll_geometry().visible_viewport_rect() else {
                return Vec::new();
            };
            self.sections()
                .iter()
                .flat_map(|section| {
                    let id = Navigation::list_id(section);
                    let parts = nodes
                        .iter()
                        .find(|node| node.id == id)
                        .map(|node| node.parts.as_slice())
                        .unwrap_or_default();
                    parts.iter().filter_map(move |part| {
                        self.host
                            .get(id)
                            .and_then(|c| c.component::<KeyedSelectableListContent>())
                            .and_then(|list| list.key_for_semantic_part(part.index))
                            .and_then(|key| section.items.iter().find(|&&p| page_key(p) == key))
                            .and_then(|&page| {
                                part.frame
                                    .intersection(&clip)
                                    .map(|visible| (page, visible))
                            })
                    })
                })
                .collect()
        }

        fn row_frame(&self, page: Page) -> Option<Rect> {
            self.row_frames()
                .into_iter()
                .find_map(|(candidate, frame)| (candidate == page).then_some(frame))
        }

        fn set_selected(&mut self, page: Page) {
            self.selected = page;
            for section in self.sections().to_vec() {
                let selected = section.items.contains(&page).then(|| page_key(page));
                let id = Navigation::list_id(&section);
                if self
                    .host
                    .get(id)
                    .unwrap()
                    .component::<KeyedSelectableListContent>()
                    .unwrap()
                    .selected_key()
                    != selected
                {
                    self.host.set_keyed_list_selection(id, selected).unwrap();
                }
            }
        }

        fn selected_page(&self) -> Option<Page> {
            self.sections().iter().find_map(|section| {
                let key = self
                    .host
                    .get(Navigation::list_id(section))?
                    .component::<KeyedSelectableListContent>()?
                    .selected_key()?;
                section
                    .items
                    .iter()
                    .copied()
                    .find(|&page| page_key(page) == key)
            })
        }

        fn consume_selection(&mut self) -> bool {
            let mut selected = None;
            for record in self.host.drain_action_records() {
                let Some(ContentActionPayload::ListIntent(intent)) = record.payload else {
                    continue;
                };
                let key = match intent.kind {
                    ListIntentKind::SelectionChanged => intent.selected_key,
                    ListIntentKind::Activated => Some(intent.key),
                };
                selected = self
                    .sections()
                    .iter()
                    .find(|section| Navigation::list_id(section) == record.action.target)
                    .and_then(|section| {
                        section
                            .items
                            .iter()
                            .copied()
                            .find(|&page| Some(page_key(page)) == key)
                    })
                    .or(selected);
            }
            if let Some(page) = selected {
                self.set_selected(page);
                true
            } else {
                false
            }
        }

        fn event(&mut self, event: &ContentEvent) -> ContentResponse {
            let response = self.host.event(event);
            self.consume_selection();
            response
        }

        fn pointer_down(&mut self, p: Point) -> ContentResponse {
            let hit = self
                .row_frames()
                .into_iter()
                .find_map(|(page, frame)| frame.contains(p).then_some(page));
            let response = self.host.event(&ContentEvent::PointerDown(p));
            self.consume_selection();
            if response.consumed
                && response.capture_pointer
                && let Some(page) = hit
                && let Some((owner, _)) = self.host.pointer_capture()
                && self
                    .section_for_page(page)
                    .is_some_and(|(section, _)| Navigation::list_id(section) == owner)
            {
                self.set_selected(page);
                return ContentResponse {
                    activated: true,
                    ..response
                };
            }
            response
        }

        fn accessibility_action(
            &mut self,
            page: Page,
            action: AccessibilityAction,
        ) -> ContentResponse {
            let Some((section, _)) = self.section_for_page(page) else {
                return ContentResponse::default();
            };
            let list = Navigation::list_id(section);
            let Some(part) = self
                .host
                .get(list)
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap()
                .semantic_part_for_key(page_key(page))
            else {
                return ContentResponse::default();
            };
            let response = self.host.accessibility_action(list, Some(part), action);
            self.consume_selection();
            response
        }
    }

    fn drawn_scrollbar(scene: &Scene, viewport: Rect) -> Option<Rect> {
        scene.primitives().iter().find_map(|primitive| {
            let tekes_ui_scene::Primitive::Quad(quad) = primitive else {
                return None;
            };
            let rect = quad.rect;
            (rect.size.width > 2.0
                && rect.size.width <= proposal::SCROLLBAR_WIDTH + 0.1
                && rect.size.height > 20.0
                && rect.min_x() >= viewport.max_x() - 10.0
                && rect.intersection(&viewport).is_some())
            .then_some(rect)
        })
    }

    #[test]
    fn filtered_navigation_keeps_its_extent_after_resizing() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.set_query("component-button", &mut text);
        assert_eq!(n.sections().iter().map(|s| s.items.len()).sum::<usize>(), 1);
        for height in [300., 600., 1000.] {
            n.height = height;
            let _ = n.render(&mut text);
            assert_eq!(n.scroll().limit().y, 0.);
        }
        n.set_query("no-component-matches-this", &mut text);
        n.height = 300.;
        let _ = n.render(&mut text);
        assert!(n.sections().is_empty());
        assert_eq!(n.scroll().limit().y, 0.);
        n.set_query("", &mut text);
        n.render(&mut text);
        assert!(n.scroll().limit().y > 0.);
    }

    #[test]
    fn shared_scrollbar_is_hover_driven_and_scrolling_remains_available() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 300.;
        let scene = n.render(&mut text);
        assert!(n.scroll().limit().y > 0.);
        let viewport = n.scroll_geometry().visible_viewport_rect().unwrap();
        assert!(drawn_scrollbar(&scene, viewport).is_none());
        let first = n.row_frames()[0].1.center();
        n.event(&ContentEvent::PointerMove(first));
        let scene = n.render(&mut text);
        assert!(drawn_scrollbar(&scene, viewport).is_some());
        n.event(&ContentEvent::PointerMove(Point::new(-1., -1.)));
        let scene = n.render(&mut text);
        assert!(drawn_scrollbar(&scene, viewport).is_none());
        n.scroll_by(Point::new(0., 100.));
        assert_eq!(n.scroll().offset.y, 100.);
    }

    #[test]
    fn every_page_occurs_once_and_headers_are_not_items() {
        let n = NavHarness::new();
        let p = n.pages();
        assert_eq!(p.len(), super::super::workbench::entries().len());
        let set: std::collections::HashSet<_> = p.iter().collect();
        assert_eq!(set.len(), p.len());
    }

    #[test]
    fn a_short_sidebar_reveals_the_last_item_through_shared_focus() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 480.0;
        let _ = n.render(&mut text);
        let last_section = sections().last().unwrap();
        let last = *last_section.items.last().unwrap();
        let last_list = Navigation::list_id(last_section);
        let list_frame = n.host.get(last_list).unwrap().frame();
        let viewport = n.host.get(SCROLL).unwrap().frame();
        // Reveal via ScrollContent + real Key::End keyboard navigation inside the list.
        let delta = list_frame.max_y() - viewport.max_y();
        if delta > 0.0 {
            n.scroll_by(Point::new(0.0, delta));
        }
        let _ = n.render(&mut text);
        assert!(n.host.focus(Some(last_list), true));
        n.event(&ContentEvent::Key {
            key: ControlKey::End,
            down: true,
        });
        let _ = n.render(&mut text);
        assert!(n.scroll().offset.y > 0.0);
        let frame = n.row_frame(last).expect("revealed page is exposed");
        assert!(
            n.scroll_geometry()
                .visible_viewport_rect()
                .unwrap()
                .contains(frame.center())
        );
    }

    #[test]
    fn filtering_rebuilds_real_rows_and_maps_selection_to_the_original_page() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.set_query("component-button", &mut text);
        assert_eq!(n.row_frames().len(), 1);
        let (page, frame) = n.row_frames()[0];
        assert!(matches!(page, Page::Workbench(_)));
        let response = n.accessibility_action(page, AccessibilityAction::Press);
        assert!(response.activated);
        assert_eq!(n.selected_page(), Some(page));
        assert!(
            n.scroll_geometry()
                .visible_viewport_rect()
                .unwrap()
                .contains(frame.center())
        );
    }

    #[test]
    fn focus_reveal_keeps_the_complete_ring_inside_a_short_sidebar_viewport() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 512.0;
        let settings = n
            .pages()
            .into_iter()
            .find(|page| {
                matches!(page, Page::Workbench(i) if super::super::workbench::entry(*i).renderer() == "settings")
            })
            .expect("Settings navigation item");
        let first = n.pages()[0];
        let last = *n.pages().last().unwrap();
        let ring_outset = proposal::metric("stroke", "c-focus-width").expect("focus width token")
            + proposal::metric("spacing", "c-focus-offset").expect("focus offset token")
            + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS;

        for page in [first, settings, last, first] {
            let _ = n.render(&mut text);
            let (section, _) = n.section_for_page(page).unwrap();
            let list_frame = n.host.get(Navigation::list_id(section)).unwrap().frame();
            let viewport = n.host.get(SCROLL).unwrap().frame();
            let target_y = (list_frame.min_y() - viewport.min_y() + n.scroll().offset.y
                - viewport.size.height * 0.25)
                .clamp(0.0, n.scroll().limit().y);
            let delta = target_y - n.scroll().offset.y;
            n.scroll_by(Point::new(0.0, delta));
            let _ = n.render(&mut text);
            assert!(
                n.accessibility_action(page, AccessibilityAction::Focus)
                    .consumed
            );
            let _ = n.render(&mut text);
            let visible = n
                .scroll_geometry()
                .visible_viewport_rect()
                .expect("sidebar has a visible scrolling viewport");
            let row = n.row_frame(page).expect("focused item is exposed");
            let ring = row.inset(-ring_outset);
            assert!(
                ring.min_y() >= visible.min_y() - 0.01 && ring.max_y() <= visible.max_y() + 0.01,
                "complete keyboard focus ring for {page:?} stays visible: {ring:?} vs {visible:?}"
            );
        }
    }

    #[test]
    fn navigation_pointer_uses_first_down_without_prior_move_after_scroll() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 360.0;
        let page = n.pages()[5];
        let _ = n.render(&mut text);
        n.scroll_by(Point::new(0.0, 85.0));
        let _ = n.render(&mut text);
        let frame = n.row_frame(page).expect("scrolled item remains visible");
        let point = frame.center();
        let response = n.pointer_down(point);
        let visible = n.scroll_geometry().visible_viewport_rect();
        assert!(
            response.consumed,
            "first Down on a rendered visible row is handled: point={point:?}, row={frame:?}, viewport={visible:?}, offset={:?}, response={response:?}, capture={:?}",
            n.scroll().offset,
            n.host.pointer_capture()
        );
        assert!(
            response.capture_pointer,
            "row owns pointer sequence: {response:?}"
        );
        assert_eq!(n.selected_page(), Some(page));
        let released = n.event(&ContentEvent::PointerUp(frame.center()));
        assert!(released.consumed, "release is handled: {released:?}");
    }

    #[test]
    fn navigation_pointer_move_then_down_selects_the_visible_shared_row() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 360.0;
        let page = n.pages()[5];
        let _ = n.render(&mut text);
        n.scroll_by(Point::new(0.0, 85.0));
        let _ = n.render(&mut text);
        let point = n
            .row_frame(page)
            .expect("scrolled item remains visible")
            .center();
        n.event(&ContentEvent::PointerMove(point));
        let down = n.pointer_down(point);
        assert!(down.consumed && down.capture_pointer, "down: {down:?}");
        assert_eq!(n.selected_page(), Some(page));
        assert!(n.event(&ContentEvent::PointerUp(point)).consumed);
    }

    #[test]
    fn navigation_ax_uses_shared_list_geometry_after_scroll_and_filter() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 360.0;
        let page = n.pages()[5];
        let _ = n.render(&mut text);
        n.scroll_by(Point::new(0.0, 85.0));
        let _ = n.render(&mut text);
        let response = n.accessibility_action(page, AccessibilityAction::Focus);
        assert!(response.consumed);
        let list = Navigation::list_id(n.section_for_page(page).unwrap().0);
        assert_eq!(
            n.host
                .get(list)
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap()
                .focused_key(),
            Some(page_key(page))
        );
        let _ = n.render(&mut text);
        let press = n.accessibility_action(page, AccessibilityAction::Press);
        assert!(press.activated);
        assert_eq!(n.selected_page(), Some(page));
        n.set_query("component-button", &mut text);
        let (filtered_page, filtered_row) = n.row_frames()[0];
        assert_eq!(n.selected_page(), None);
        assert_ne!(filtered_page, page);
        n.height = 280.0;
        let _ = n.render(&mut text);
        let point = n.row_frame(filtered_page).unwrap().center();
        assert!(
            n.scroll_geometry()
                .visible_viewport_rect()
                .is_some_and(|visible| visible.contains(point))
        );
        assert_eq!(filtered_row, n.row_frame(filtered_page).unwrap());
    }

    #[test]
    fn navigation_events_keep_page_identity_with_duplicate_labels_reorder_and_filter() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 2400.;
        let section = n
            .sections()
            .iter()
            .find(|section| section.items.len() >= 2)
            .unwrap()
            .clone();
        let first = section.items[0];
        let second = section.items[1];
        let id = Navigation::list_id(&section);
        let mut snapshot = n
            .host
            .get(id)
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
            .model()
            .snapshot()
            .clone();
        for item in &mut snapshot.items {
            item.name = "Duplicate label".into();
        }
        snapshot.items.reverse();
        snapshot.display_items = vec![page_key(second), page_key(first)];
        let mut rows = snapshot
            .items
            .iter()
            .map(|item| ListRowDisplay {
                key: item.key,
                row_id: ComponentId(0x0047_414d_0000 + item.key.0),
                display: ListDisplayUpdate::Keep,
            })
            .collect();
        n.host
            .replace_keyed_list(id, &snapshot, &mut rows, None)
            .unwrap();
        n.set_selected(first);
        let _ = n.render(&mut text);
        let first_frame = n.row_frame(first).unwrap();
        let second_frame = n.row_frame(second).unwrap();
        assert!(second_frame.min_y() < first_frame.min_y());
        assert!(n.pointer_down(second_frame.center()).activated);
        n.event(&ContentEvent::PointerUp(second_frame.center()));
        assert_eq!(n.selected_page(), Some(second));
        n.set_selected(first);
        assert!(
            n.host.drain_action_records().is_empty(),
            "programmatic synchronization emits no user action"
        );
        let _ = n.render(&mut text);
        n.host.focus(Some(id), true);
        assert_eq!(n.host.focused(), Some(id));
        assert!(
            n.event(&ContentEvent::Key {
                key: ControlKey::Home,
                down: true
            })
            .changed
        );
        assert_eq!(
            n.selected_page(),
            Some(second),
            "keyboard follows display order, then resolves page identity"
        );
        let _ = n.render(&mut text);
        assert!(
            n.accessibility_action(first, AccessibilityAction::Press)
                .activated
        );
        assert_eq!(
            n.selected_page(),
            Some(first),
            "AX uses the current key exposure, not source ordinal"
        );
    }

    #[test]
    fn pointer_selection_replaces_selection_in_another_group() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 2800.0;
        let groups: Vec<_> = n
            .sections()
            .iter()
            .filter(|section| !section.items.is_empty())
            .collect();
        assert!(groups.len() > 1);
        let first = groups[0].items[0];
        let second = groups[1].items[0];
        let _ = n.render(&mut text);
        for page in [first, second] {
            let frame = n.row_frame(page).expect("group row is visible");
            let down = n.pointer_down(frame.center());
            assert!(down.consumed && down.capture_pointer);
            let up = n.event(&ContentEvent::PointerUp(frame.center()));
            assert!(up.consumed);
            assert_eq!(n.selected_page(), Some(page));
        }
    }

    #[test]
    fn accessibility_selection_replaces_selection_in_another_group() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 2800.0;
        let groups: Vec<_> = n
            .sections()
            .iter()
            .filter(|section| !section.items.is_empty())
            .collect();
        assert!(groups.len() > 1);
        let page = groups[1].items[0];
        let _ = n.render(&mut text);
        let response = n.accessibility_action(page, AccessibilityAction::Press);
        assert!(response.consumed && response.activated);
        assert_eq!(n.selected_page(), Some(page));
    }

    #[test]
    fn type_scale_scales_real_navigation_frames_without_fixed_row_constants() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 4200.0;
        let _ = n.render(&mut text);
        let doc_1x = n.host.get(LIST).unwrap().frame().size.height;
        let row_1x = n.row_frame(n.pages()[0]).unwrap().size.height;
        text.set_type_scale(1.5);
        let _ = n.render(&mut text);
        let doc_1_5x = n.host.get(LIST).unwrap().frame().size.height;
        let row_1_5x = n.row_frame(n.pages()[0]).unwrap().size.height;
        assert!(row_1_5x > row_1x, "1.5x={row_1_5x} 1.0x={row_1x}");
        assert!(doc_1_5x > doc_1x, "1.5x={doc_1_5x} 1.0x={doc_1x}");
    }

    #[test]
    fn section_headers_keep_equal_top_and_bottom_spacing_across_directory_and_filter() {
        let mut n = NavHarness::new();
        let mut text = TextEngine::new();
        n.height = 4000.0;
        for query in ["", "component"] {
            n.set_query(query, &mut text);
            let _ = n.render(&mut text);
            let pairs = n.sections().to_vec();
            assert!(pairs.len() >= 2, "query={query:?}");
            let first_list = n
                .host
                .get(Navigation::list_id(&pairs[0]))
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap();
            let first_parts = first_list.semantic_parts();
            if first_parts.len() >= 2 {
                assert_eq!(
                    first_parts[1].frame.min_y() - first_parts[0].frame.max_y(),
                    2.0,
                    "PR #367 2pt row gap in navigation list"
                );
            }
            for window in pairs.windows(2) {
                let prev_list_node = n.host.get(Navigation::list_id(&window[0])).unwrap();
                let prev_list = prev_list_node
                    .component::<KeyedSelectableListContent>()
                    .unwrap();
                let next_header = n
                    .host
                    .get(ComponentId(
                        HEADER_BASE + section_slot(&window[1].title) as u64,
                    ))
                    .unwrap();
                let next_list_node = n.host.get(Navigation::list_id(&window[1])).unwrap();
                let next_list = next_list_node
                    .component::<KeyedSelectableListContent>()
                    .unwrap();
                assert_eq!(
                    next_header.component::<TextContent>().unwrap().role,
                    proposal::TextRole::UiCaption
                );
                let prev_last = prev_list.semantic_parts().last().unwrap().frame;
                let next_first = next_list.semantic_parts().first().unwrap().frame;
                assert_eq!(prev_last.max_y(), prev_list_node.frame().max_y());
                assert_eq!(next_first.min_y(), next_list_node.frame().min_y());
                assert_eq!(
                    next_header.frame().min_y() - prev_last.max_y(),
                    ListContent::SECTION_GAP,
                    "above {} for query={query:?}",
                    window[1].title
                );
                assert_eq!(
                    next_first.min_y() - next_header.frame().max_y(),
                    ListContent::SECTION_GAP,
                    "below {} for query={query:?}",
                    window[1].title
                );
            }
        }
    }
}
