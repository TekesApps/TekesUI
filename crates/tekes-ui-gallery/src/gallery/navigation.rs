//! JSON-owned sections and items. Page IDs stay stable; visual order is independent.
use super::{Page, layout};
use std::sync::OnceLock;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ScrollArea,
    composition::{
        AccessibilityAction, Component, ComponentId, Content, ContentActionPayload, ContentEvent,
        ContentHost, ContentResponse, ContentSemantics, ContentTheme, KeyedSelectableListContent,
        ListContent, ListDisplayUpdate, ListIntentKind, ListRowDisplay, ScrollContent,
        StackContent,
    },
    extended::{
        Axis, KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey, Stack,
    },
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};

const SCROLL: ComponentId = ComponentId(0x0047_414c_0010);
const LIST: ComponentId = ComponentId(0x0047_414c_0011);
const GROUP_BASE: u64 = 0x0047_414c_0100;
const HEADER_BASE: u64 = 0x0047_414c_0200;
const ITEMS_BASE: u64 = 0x0047_414c_0300;

#[derive(Debug)]
struct SectionLabel(String);
impl Component for SectionLabel {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(text.measure(&self.0, proposal::TextRole::UiCaption, available.width))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        text.draw(
            scene,
            &self.0,
            proposal::TextRole::UiCaption,
            frame,
            ButtonTextAlign::Leading,
            theme
                .palette
                .color("secondary-text")
                .expect("Gallery section label token"),
        );
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(
            tekes_ui_controls::composition::ContentRole::Text,
            self.0.clone(),
        )
    }
}
#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
    pub rect: Rect,
    pub items: Vec<(Page, Rect)>,
}
pub fn sections() -> &'static [Section] {
    static SECTIONS: OnceLock<Vec<Section>> = OnceLock::new();
    SECTIONS.get_or_init(|| {
        let mut y = layout::CATALOG_TOP + 48.0;
        let mut all = Vec::new();
        for (i, entry) in super::workbench::entries().iter().enumerate() {
            let title = entry.section();
            if all.last().is_none_or(|s: &Section| s.title != title) {
                let h =
                    proposal::TextRole::UiCaption.style().line_height + Spacing::Regular.value();
                all.push(Section {
                    title: title.into(),
                    rect: Rect::new(
                        Spacing::Loose.value(),
                        y,
                        layout::CATALOG_W - 2.0 * Spacing::Loose.value(),
                        h,
                    ),
                    items: Vec::new(),
                });
                y += h;
            }
            all.last_mut().unwrap().items.push((
                Page::Workbench(i),
                Rect::new(
                    layout::ROW_INSET_X,
                    y,
                    layout::CATALOG_W - 2.0 * layout::ROW_INSET_X,
                    layout::ROW_H,
                ),
            ));
            y += layout::ROW_H + layout::ROW_GAP;
        }
        all
    })
}
pub fn pages() -> Vec<Page> {
    sections()
        .iter()
        .flat_map(|s| s.items.iter().map(|v| v.0))
        .collect()
}
#[cfg(test)]
pub fn row(page: Page) -> Rect {
    sections()
        .iter()
        .flat_map(|s| s.items.iter())
        .find(|v| v.0 == page)
        .map(|v| v.1)
        .unwrap_or(Rect::new(
            layout::ROW_INSET_X,
            layout::CATALOG_TOP + 48.0,
            layout::CATALOG_W - 2.0 * layout::ROW_INSET_X,
            layout::ROW_H,
        ))
}
pub fn document_height() -> f32 {
    sections()
        .last()
        .and_then(|s| s.items.last())
        .map_or(0.0, |v| v.1.max_y() + Spacing::Loose.value())
}
#[derive(Debug)]
pub struct Navigation {
    host: ContentHost,
    height: f32,
    query: String,
    filtered: Vec<Section>,
    selected: Page,
    pending_focus: Option<Page>,
}
impl Default for Navigation {
    fn default() -> Self {
        let selected = Page::Workbench(0);
        let sections = sections();
        let mut n = Self {
            host: content_host(sections, selected),
            height: layout::FRAME_H,
            query: String::new(),
            filtered: Vec::new(),
            selected,
            pending_focus: None,
        };
        n.resize(layout::FRAME_H);
        n
    }
}
impl Navigation {
    /// The navigation document begins below the fixed search field. Section
    /// rectangles remain in their existing local coordinates; the shared
    /// scroll geometry uses document coordinates beginning at zero.
    fn content_top() -> f32 {
        search_rect().max_y() + 8.0
    }

    fn content_frame(&self) -> Rect {
        let top = sections().first().map_or(Self::content_top(), |section| {
            section.rect.min_y() - Spacing::Loose.value()
        });
        Rect::new(0., top, layout::CATALOG_W, (self.height - top).max(0.))
    }
    pub fn content_clip(&self) -> Rect {
        let top = Self::content_top();
        Rect::new(0., top, layout::CATALOG_W, (self.height - top).max(0.))
    }
    fn scroll(&self) -> &ScrollArea {
        &self
            .host
            .get(SCROLL)
            .expect("mounted Gallery scroll content")
            .component::<ScrollContent>()
            .expect("Gallery uses shared ScrollContent")
            .scroll
    }
    fn scroll_mut(&mut self) -> &mut ScrollArea {
        &mut self
            .host
            .get_mut(SCROLL)
            .expect("mounted Gallery scroll content")
            .component_mut::<ScrollContent>()
            .expect("Gallery uses shared ScrollContent")
            .scroll
    }
    fn list_id(section: &Section) -> ComponentId {
        let index = sections()
            .iter()
            .position(|candidate| candidate.title == section.title)
            .expect("filtered section retains original identity");
        ComponentId(ITEMS_BASE + index as u64)
    }
    fn group_id(section: &Section) -> ComponentId {
        ComponentId(GROUP_BASE + section_slot(&section.title) as u64)
    }
    fn section_for_page(&self, page: Page) -> Option<(&Section, usize)> {
        self.sections().iter().find_map(|section| {
            section
                .items
                .iter()
                .position(|(candidate, _)| *candidate == page)
                .map(|index| (section, index))
        })
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        theme: tekes_ui_tokens::Theme,
    ) {
        if let Some(page) = self.pending_focus.take() {
            // Selection and reveal mutate the Host allocation. Admit keyboard
            // focus only after the shared layout has acknowledged those changes.
            self.host
                .render(
                    &mut Scene::new(),
                    text,
                    self.content_frame(),
                    self.content_clip(),
                    theme.into(),
                )
                .expect("Gallery navigation lays out before admitting focus");
            if let Some((section, _)) = self.section_for_page(page) {
                self.host.focus(Some(Self::list_id(section)), true);
                // Host focus addresses the grouped List; Gallery's roving
                // navigation exposes the selected row within that group.
                self.reveal(page);
            }
        }
        self.host
            .render(
                scene,
                text,
                self.content_frame(),
                self.content_clip(),
                theme.into(),
            )
            .expect("Gallery navigation host renders its current layout");
    }

    pub fn resize(&mut self, height: f32) {
        if height.is_finite() && height > 0.0 {
            self.height = height;
            let frame = self.content_frame();
            let document_height = (self.filtered_document_height() - frame.min_y()).max(0.);
            self.scroll_mut()
                .layout(frame.size, Size::new(layout::CATALOG_W, document_height));
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, layout::CATALOG_W, self.height)
    }

    /// A per-pass snapshot shared by sidebar drawing, clipping, culling, and
    /// hit testing. The full-height document preserves the existing scroll
    /// extent; the fixed search field is excluded only by the ancestor clip.
    pub fn geometry(&self) -> tekes_ui_controls::ScrollGeometry {
        self.scroll()
            .geometry(self.content_frame(), Some(self.content_clip()))
    }

    pub fn row_frames(&self) -> Vec<(Page, Rect)> {
        let nodes = self.host.semantics();
        let Some(clip) = self.geometry().visible_viewport_rect() else {
            return Vec::new();
        };
        self.sections()
            .iter()
            .flat_map(|section| {
                let id = Self::list_id(section);
                let parts = nodes
                    .iter()
                    .find(|node| node.id == id)
                    .map(|node| node.parts.as_slice())
                    .unwrap_or_default();
                parts.iter().filter_map(move |part| {
                    self.host
                        .get(id)
                        .and_then(|content| content.component::<KeyedSelectableListContent>())
                        .and_then(|list| list.key_for_semantic_part(part.index))
                        .and_then(|key| {
                            section
                                .items
                                .iter()
                                .find(|(page, _)| page_key(*page) == key)
                        })
                        .and_then(|(page, _)| {
                            part.frame
                                .intersection(&clip)
                                .map(|visible| (*page, visible))
                        })
                })
            })
            .collect()
    }
    #[cfg(test)]
    pub fn row_frame(&self, page: Page) -> Option<Rect> {
        self.row_frames()
            .into_iter()
            .find_map(|(candidate, frame)| (candidate == page).then_some(frame))
    }
    pub fn focus_page(&mut self, page: Page) -> bool {
        if self.section_for_page(page).is_none() {
            return false;
        }
        self.set_selected(page);
        self.reveal(page);
        self.pending_focus = Some(page);
        true
    }
    pub fn clear_focus(&mut self) {
        self.pending_focus = None;
        self.host.focus(None, false);
    }
    pub fn selected_page(&self) -> Option<Page> {
        self.sections().iter().find_map(|section| {
            let key = self
                .host
                .get(Self::list_id(section))?
                .component::<KeyedSelectableListContent>()?
                .selected_key()?;
            section
                .items
                .iter()
                .find_map(|(page, _)| (page_key(*page) == key).then_some(*page))
        })
    }
    pub fn set_selected(&mut self, page: Page) {
        self.selected = page;
        let sections = self.sections().to_vec();
        for section in &sections {
            let selected = section
                .items
                .iter()
                .any(|(candidate, _)| *candidate == page)
                .then(|| page_key(page));
            let id = Self::list_id(section);
            if self
                .host
                .get(id)
                .unwrap()
                .component::<KeyedSelectableListContent>()
                .unwrap()
                .selected_key()
                == selected
            {
                continue;
            }
            self.host.set_keyed_list_selection(id, selected).unwrap();
        }
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
                .find(|section| Self::list_id(section) == record.action.target)
                .and_then(|section| {
                    section
                        .items
                        .iter()
                        .find_map(|(page, _)| (Some(page_key(*page)) == key).then_some(*page))
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
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let response = self.host.event(event);
        self.consume_selection();
        response
    }
    pub fn pointer_capture(&self) -> Option<(ComponentId, u64)> {
        self.host.pointer_capture()
    }
    pub fn accessibility_action(
        &mut self,
        page: Page,
        action: AccessibilityAction,
    ) -> ContentResponse {
        let Some((section, _)) = self.section_for_page(page) else {
            return ContentResponse::default();
        };
        let list = Self::list_id(section);
        if matches!(action, AccessibilityAction::Focus) {
            return ContentResponse::consumed(self.focus_page(page));
        }
        let part = self
            .host
            .get(list)
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
            .semantic_part_for_key(page_key(page));
        let Some(part) = part else {
            return ContentResponse::default();
        };
        let response = self.host.accessibility_action(list, Some(part), action);
        self.consume_selection();
        response
    }
    pub fn needs_animation(&self) -> bool {
        self.scroll().needs_animation()
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        self.scroll_mut().tick(dt)
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.scroll_mut().set_reduced_motion(reduced)
    }
    pub fn end_gesture(&mut self) {
        self.scroll_mut().end_gesture();
    }
    pub fn cancel(&mut self) {
        self.host.cancel();
    }
    #[cfg(test)]
    pub fn offset(&self) -> Point {
        self.scroll().offset
    }
    #[cfg(test)]
    pub fn limit(&self) -> Point {
        self.scroll().limit()
    }
    #[cfg(test)]
    pub fn scroll_by(&mut self, delta: Point) {
        self.scroll_mut().scroll_by(delta.x, delta.y);
    }
    pub fn sections(&self) -> &[Section] {
        if self.query.trim().is_empty() {
            sections()
        } else {
            &self.filtered
        }
    }
    fn filter_sections(query: &str) -> Vec<Section> {
        let q = query.to_lowercase();
        let mut y = layout::CATALOG_TOP + 48.0;
        let mut result = Vec::new();
        for section in sections() {
            let matched: Vec<_> = section
                .items
                .iter()
                .filter(|(p, _)| match p {
                    Page::Workbench(i) => super::workbench::entry(*i).matches(&q),
                    _ => false,
                })
                .collect();
            if matched.is_empty() {
                continue;
            }
            let mut r = Section {
                title: section.title.clone(),
                rect: section.rect,
                items: Vec::new(),
            };
            r.rect.origin.y = y;
            y += r.rect.size.height;
            for (p, rect) in matched {
                let mut rect = *rect;
                rect.origin.y = y;
                r.items.push((*p, rect));
                y += layout::ROW_H + layout::ROW_GAP;
            }
            result.push(r);
        }
        result
    }
    fn filtered_document_height(&self) -> f32 {
        if self.query.trim().is_empty() {
            return document_height();
        }
        self.sections()
            .last()
            .and_then(|s| s.items.last())
            .map_or(0., |v| v.1.max_y() + Spacing::Loose.value())
    }
    pub fn set_query(&mut self, query: &str) {
        if self.query != query {
            let old_sections = self.sections().to_vec();
            self.host.cancel();
            for section in &old_sections {
                self.host
                    .remove(Self::group_id(section))
                    .expect("Gallery section is a mounted List child");
            }
            self.query = query.into();
            self.filtered = if query.trim().is_empty() {
                Vec::new()
            } else {
                Self::filter_sections(query)
            };
            let new_sections = self.sections().to_vec();
            for section in &new_sections {
                self.host
                    .insert(LIST, section_content(section, self.selected))
                    .expect("Gallery List accepts a grouped section");
            }
            self.resize(self.height);
            self.reveal(self.selected);
        }
    }
    pub fn reveal(&mut self, page: Page) {
        let rect = self
            .sections()
            .iter()
            .flat_map(|s| &s.items)
            .find(|(p, _)| *p == page)
            .map(|(_, rect)| *rect);
        if let Some(rect) = rect {
            // Sidebar focus uses the same visible focus ring as controls.
            // Reserve the worst-case 1x backing-pixel antialias guard so the
            // ring remains fully visible on both 1x and 2x displays.
            let ring_outset = proposal::metric("stroke", "c-focus-width")
                .expect("focus width token")
                + proposal::metric("spacing", "c-focus-offset").expect("focus offset token")
                + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS;
            let geometry = self.geometry();
            let origin = geometry.content_origin();
            let frame = self.content_frame();
            let row = rect.offset(origin.x - frame.min_x(), origin.y - frame.min_y());
            let focus_bounds = row.inset(-ring_outset);
            let Some(visible) = geometry.visible_viewport_rect() else {
                return;
            };
            let dx = if focus_bounds.min_x() < visible.min_x() {
                focus_bounds.min_x() - visible.min_x()
            } else {
                (focus_bounds.max_x() - visible.max_x()).max(0.0)
            };
            let dy = if focus_bounds.min_y() < visible.min_y() {
                focus_bounds.min_y() - visible.min_y()
            } else {
                (focus_bounds.max_y() - visible.max_y()).max(0.0)
            };
            if dx != 0. || dy != 0. {
                self.scroll_mut().scroll_by(dx, dy);
            }
        }
    }

    pub fn pointer_down(&mut self, p: Point) -> ContentResponse {
        // Gallery activates on Down, including the already selected page.
        // Capture the current exposed hit identity before delivering the event;
        // selection callbacks can mutate the host's layout and selection.
        let hit = self
            .row_frames()
            .into_iter()
            .find_map(|(page, frame)| frame.contains(p).then_some(page));
        let response = self.event(&ContentEvent::PointerDown(p));
        if response.consumed
            && response.capture_pointer
            && let Some(page) = hit
            && let Some((owner, _)) = self.pointer_capture()
            && self
                .section_for_page(page)
                .is_some_and(|(section, _)| Self::list_id(section) == owner)
        {
            self.set_selected(page);
            return ContentResponse {
                activated: true,
                ..response
            };
        }
        response
    }
    pub fn pointer_move(&mut self, p: Point) -> ContentResponse {
        self.event(&ContentEvent::PointerMove(p))
    }
    pub fn pointer_up(&mut self, p: Point) -> ContentResponse {
        self.event(&ContentEvent::PointerUp(p))
    }
    pub fn scroll_event(&mut self, event: ContentEvent) -> ContentResponse {
        self.event(&event)
    }
}

fn section_slot(title: &str) -> usize {
    sections()
        .iter()
        .position(|section| section.title == title)
        .expect("Gallery section has stable identity")
}
fn page_key(page: Page) -> ListItemKey {
    let Page::Workbench(index) = page else {
        unreachable!("catalog contains Workbench pages")
    };
    ListItemKey(index as u64 + 1)
}
fn section_content(section: &Section, selected: Page) -> Content {
    let slot = section_slot(&section.title) as u64;
    let snapshot = KeyedListSnapshot {
        dataset: ListDatasetKey(ITEMS_BASE + slot),
        items: section
            .items
            .iter()
            .map(|(page, _)| KeyedListItem {
                key: page_key(*page),
                name: page.name().to_owned(),
            })
            .collect(),
        display_items: section
            .items
            .iter()
            .map(|(page, _)| page_key(*page))
            .collect(),
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
    Content::new(
        ComponentId(GROUP_BASE + slot),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(
                    ComponentId(HEADER_BASE + slot),
                    SectionLabel(section.title.clone()),
                ),
                Content::new(ComponentId(ITEMS_BASE + slot), list),
            ],
        },
    )
}
fn content_host(sections: &[Section], selected: Page) -> ContentHost {
    let mut scroll = ListContent::scrollable(
        "Components",
        LIST,
        sections
            .iter()
            .map(|section| section_content(section, selected))
            .collect(),
    )
    .content_padding(Spacing::Loose.value());
    if let Some(list) = scroll
        .children_mut()
        .and_then(|children| children.first_mut())
        .and_then(|content| content.component_mut::<ListContent>())
    {
        list.rows.model.gap = Spacing::Micro.value();
    }
    let mut host = ContentHost::new(Content::new(SCROLL, scroll))
        .expect("Gallery navigation Content tree is valid");
    host.set_action_events(true);
    host
}

pub fn search_rect() -> Rect {
    Rect::new(
        layout::ROW_INSET_X,
        layout::CATALOG_TOP,
        layout::CATALOG_W - 2.0 * layout::ROW_INSET_X,
        34.,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_tokens::Theme;

    fn render(n: &mut Navigation) -> Scene {
        let mut scene = Scene::new();
        let mut text = TextEngine::new();
        text.begin_frame();
        n.render(&mut scene, &mut text, Theme::LIGHT);
        scene
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
        let mut n = Navigation::default();
        n.set_query("component-button");
        assert_eq!(n.sections().iter().map(|s| s.items.len()).sum::<usize>(), 1);
        for height in [300., 600., 1000.] {
            n.resize(height);
            assert_eq!(n.limit().y, 0.);
        }
        n.set_query("no-component-matches-this");
        n.resize(300.);
        assert!(n.sections().is_empty());
        assert_eq!(n.limit().y, 0.);
        n.set_query("");
        assert!(n.limit().y > 0.);
    }

    #[test]
    fn shared_scrollbar_is_hover_driven_and_scrolling_remains_available() {
        let mut n = Navigation::default();
        n.resize(300.);
        let scene = render(&mut n);
        assert!(n.limit().y > 0.);
        let viewport = n.geometry().visible_viewport_rect().unwrap();
        assert!(drawn_scrollbar(&scene, viewport).is_none());
        let first = n.row_frames()[0].1.center();
        n.pointer_move(first);
        let scene = render(&mut n);
        assert!(drawn_scrollbar(&scene, viewport).is_some());
        n.pointer_move(Point::new(-1., -1.));
        let scene = render(&mut n);
        assert!(drawn_scrollbar(&scene, viewport).is_none());
        n.scroll_by(Point::new(0., 100.));
        assert_eq!(n.offset().y, 100.);
    }

    #[test]
    fn every_page_occurs_once_and_headers_are_not_items() {
        let p = pages();
        assert_eq!(p.len(), super::super::workbench::entries().len());
        let set: std::collections::HashSet<_> = p.iter().collect();
        assert_eq!(set.len(), p.len());
    }

    #[test]
    fn a_short_sidebar_reveals_the_last_item_through_shared_focus() {
        let mut n = Navigation::default();
        n.resize(480.0);
        let page = *pages().last().unwrap();
        let _ = render(&mut n);
        n.focus_page(page);
        assert!(n.offset().y > 0.0);
        let _ = render(&mut n);
        let frame = n.row_frame(page).expect("revealed page is exposed");
        assert!(
            n.geometry()
                .visible_viewport_rect()
                .unwrap()
                .contains(frame.center())
        );
    }

    #[test]
    fn filtering_rebuilds_real_rows_and_maps_selection_to_the_original_page() {
        let mut n = Navigation::default();
        n.set_query("component-button");
        let _ = render(&mut n);
        assert_eq!(n.row_frames().len(), 1);
        let (page, frame) = n.row_frames()[0];
        assert!(matches!(page, Page::Workbench(_)));
        let response = n.accessibility_action(page, AccessibilityAction::Press);
        assert!(response.activated);
        assert_eq!(n.selected_page(), Some(page));
        assert!(
            n.geometry()
                .visible_viewport_rect()
                .unwrap()
                .contains(frame.center())
        );
    }

    #[test]
    fn focus_reveal_keeps_the_complete_ring_inside_a_short_sidebar_viewport() {
        let mut n = Navigation::default();
        n.resize(512.0);
        let settings = pages()
            .into_iter()
            .find(|page| {
                matches!(page, Page::Workbench(i) if super::super::workbench::entry(*i).renderer() == "settings")
            })
            .expect("Settings navigation item");
        let first = pages()[0];
        let ring_outset = proposal::metric("stroke", "c-focus-width").expect("focus width token")
            + proposal::metric("spacing", "c-focus-offset").expect("focus offset token")
            + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS;

        for page in [first, settings, *pages().last().unwrap(), first] {
            let _ = render(&mut n);
            n.focus_page(page);
            let _ = render(&mut n);
            let geometry = n.geometry();
            let visible = geometry
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
        let mut n = Navigation::default();
        n.resize(360.0);
        let page = pages()[5];
        let _ = render(&mut n);
        n.scroll_by(Point::new(0.0, 85.0));
        let _ = render(&mut n);
        let frame = n.row_frame(page).expect("scrolled item remains visible");
        let point = frame.center();
        let response = n.pointer_down(point);
        let visible = n.geometry().visible_viewport_rect();
        assert!(
            response.consumed,
            "first Down on a rendered visible row is handled: point={point:?}, row={frame:?}, clip={:?}, viewport={visible:?}, offset={:?}, response={response:?}, capture={:?}",
            n.content_clip(),
            n.offset(),
            n.pointer_capture()
        );
        assert!(
            response.capture_pointer,
            "row owns pointer sequence: {response:?}"
        );
        assert_eq!(n.selected_page(), Some(page));
        let released = n.pointer_up(frame.center());
        assert!(released.consumed, "release is handled: {released:?}");
    }

    #[test]
    fn navigation_pointer_move_then_down_selects_the_visible_shared_row() {
        let mut n = Navigation::default();
        n.resize(360.0);
        let page = pages()[5];
        let _ = render(&mut n);
        n.scroll_by(Point::new(0.0, 85.0));
        let _ = render(&mut n);
        let point = n
            .row_frame(page)
            .expect("scrolled item remains visible")
            .center();
        n.pointer_move(point);
        let down = n.pointer_down(point);
        assert!(down.consumed && down.capture_pointer, "down: {down:?}");
        assert_eq!(n.selected_page(), Some(page));
        assert!(n.pointer_up(point).consumed);
    }

    #[test]
    fn navigation_ax_uses_shared_list_geometry_after_scroll_and_filter() {
        let mut n = Navigation::default();
        n.resize(360.0);
        let page = pages()[5];
        let _ = render(&mut n);
        n.scroll_by(Point::new(0.0, 85.0));
        let _ = render(&mut n);
        let response = n.accessibility_action(page, AccessibilityAction::Focus);
        assert!(response.consumed);
        assert_eq!(n.selected_page(), Some(page));
        n.set_query("component-button");
        let _ = render(&mut n);
        let (filtered_page, filtered_row) = n.row_frames()[0];
        assert_eq!(n.selected_page(), None);
        assert_ne!(filtered_page, page);
        n.resize(280.0);
        let _ = render(&mut n);
        let point = n.row_frame(filtered_page).unwrap().center();
        assert!(
            n.geometry()
                .visible_viewport_rect()
                .is_some_and(|visible| visible.contains(point))
        );
        assert_eq!(filtered_row, n.row_frame(filtered_page).unwrap());
    }

    #[test]
    fn navigation_events_keep_page_identity_with_duplicate_labels_reorder_and_filter() {
        use tekes_ui_controls::ControlKey;
        let mut n = Navigation::default();
        n.resize(2400.);
        let section = n
            .sections()
            .iter()
            .find(|section| section.items.len() >= 2)
            .unwrap()
            .clone();
        let first = section.items[0].0;
        let second = section.items[1].0;
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
        let _ = render(&mut n);
        let first_frame = n.row_frame(first).unwrap();
        let second_frame = n.row_frame(second).unwrap();
        assert!(second_frame.min_y() < first_frame.min_y());
        assert!(n.pointer_down(second_frame.center()).activated);
        n.pointer_up(second_frame.center());
        assert_eq!(n.selected_page(), Some(second));
        n.set_selected(first);
        assert!(
            n.host.drain_action_records().is_empty(),
            "programmatic synchronization emits no user action"
        );
        let _ = render(&mut n);
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
        let _ = render(&mut n);
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
        let mut n = Navigation::default();
        n.resize(2400.0);
        let groups: Vec<_> = n
            .sections()
            .iter()
            .filter(|section| !section.items.is_empty())
            .collect();
        assert!(groups.len() > 1);
        let first = groups[0].items[0].0;
        let second = groups[1].items[0].0;
        let _ = render(&mut n);
        for page in [first, second] {
            let frame = n.row_frame(page).expect("group row is visible");
            let down = n.pointer_down(frame.center());
            assert!(down.consumed && down.capture_pointer);
            let up = n.pointer_up(frame.center());
            assert!(up.consumed);
            assert_eq!(n.selected_page(), Some(page));
        }
    }

    #[test]
    fn accessibility_selection_replaces_selection_in_another_group() {
        let mut n = Navigation::default();
        n.resize(2400.0);
        let groups: Vec<_> = n
            .sections()
            .iter()
            .filter(|section| !section.items.is_empty())
            .collect();
        assert!(groups.len() > 1);
        let page = groups[1].items[0].0;
        let _ = render(&mut n);
        let response = n.accessibility_action(page, AccessibilityAction::Press);
        assert!(response.consumed && response.activated);
        assert_eq!(n.selected_page(), Some(page));
    }
}
