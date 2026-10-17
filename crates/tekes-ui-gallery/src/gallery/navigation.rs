//! JSON-owned sections and items. Page IDs stay stable; visual order is independent.
use super::{Page, layout};
use std::sync::OnceLock;
use tekes_ui_controls::ScrollArea;
use tekes_ui_scene::{Point, Rect, Size};
use tekes_ui_tokens::{Spacing, proposal};
#[derive(Debug)]
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
    pub scroll: ScrollArea,
    height: f32,
    query: String,
    filtered: Vec<Section>,
}
impl Default for Navigation {
    fn default() -> Self {
        let mut n = Self {
            scroll: ScrollArea::default(),
            height: layout::FRAME_H,
            query: String::new(),
            filtered: Vec::new(),
        };
        n.scroll.set_scrollbar_visibility(false, false);
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

    pub fn resize(&mut self, height: f32) {
        if height.is_finite() && height > 0.0 {
            self.height = height;
            self.scroll.layout(
                Size::new(layout::CATALOG_W, height),
                Size::new(layout::CATALOG_W, self.filtered_document_height()),
            );
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, layout::CATALOG_W, self.height)
    }

    /// A per-pass snapshot shared by sidebar drawing, clipping, culling, and
    /// hit testing. The full-height document preserves the existing scroll
    /// extent; the fixed search field is excluded only by the ancestor clip.
    pub fn geometry(&self) -> tekes_ui_controls::ScrollGeometry {
        let bounds = self.bounds();
        let top = Self::content_top();
        self.scroll.geometry(
            bounds,
            Some(Rect::new(
                bounds.min_x(),
                top,
                bounds.size.width,
                (bounds.max_y() - top).max(0.0),
            )),
        )
    }

    pub fn hit(&self, p: Point) -> Option<usize> {
        let geometry = self.geometry();
        let point = geometry.viewport_to_document(p)?;
        self.sections()
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|(_, r)| r.inset(-layout::ROW_GAP * 0.5).contains(point))
            .map(|v| v.0.index())
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
            self.query = query.into();
            self.filtered = if query.trim().is_empty() {
                Vec::new()
            } else {
                Self::filter_sections(query)
            };
            self.scroll.cancel();
            self.scroll.offset = Point::default();
            let h = self.filtered_document_height();
            self.scroll.layout(
                Size::new(layout::CATALOG_W, self.height),
                Size::new(layout::CATALOG_W, h),
            );
        }
    }
    pub fn reveal(&mut self, page: Page) {
        if let Some((_, rect)) = self
            .sections()
            .iter()
            .flat_map(|s| &s.items)
            .find(|(p, _)| *p == page)
        {
            self.scroll.reveal(*rect);
        }
    }

    pub fn track(&self) -> Rect {
        Rect::new(
            layout::CATALOG_W - Spacing::Regular.value(),
            Spacing::Regular.value(),
            proposal::SCROLLBAR_WIDTH,
            (self.height - 2.0 * Spacing::Regular.value()).max(0.0),
        )
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        if !self.scroll.scrollbar_visible(true) {
            return false;
        }
        let t = self.track().inset(-Spacing::Micro.value());
        self.scroll.pointer_down(p, true, t, layout::ROW_H)
    }
    pub fn pointer_drag(&mut self, p: Point) -> bool {
        let t = self.track().inset(-Spacing::Micro.value());
        self.scroll.pointer_drag(p, t, layout::ROW_H)
    }
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
    #[test]
    fn filtered_navigation_keeps_its_extent_after_resizing() {
        let mut n = Navigation::default();
        n.set_query("component-button");
        assert_eq!(n.sections().iter().map(|s| s.items.len()).sum::<usize>(), 1);
        for height in [300., 600., 1000.] {
            n.resize(height);
            assert_eq!(n.scroll.limit().y, 0.);
        }
        n.set_query("no-component-matches-this");
        n.resize(300.);
        assert!(n.sections().is_empty());
        assert_eq!(n.scroll.limit().y, 0.);
        n.set_query("");
        assert!(n.scroll.limit().y > 0.);
    }
    #[test]
    fn sidebar_padding_clears_optional_scrollbar_and_hidden_bar_still_scrolls() {
        let mut n = Navigation::default();
        n.resize(300.);
        assert!(!n.scroll.scrollbar_visible(true));
        assert!(n.scroll.limit().y > 0.);
        n.scroll.scroll_by(0., 100.);
        assert_eq!(n.scroll.offset.y, 100.);
        n.scroll.set_scrollbar_visibility(false, true);
        let track = n.track().inset(-Spacing::Micro.value());
        assert!(search_rect().max_x() < track.min_x());
        for (_, rect) in sections().iter().flat_map(|s| &s.items) {
            assert!(rect.max_x() < track.min_x());
        }
    }
    #[test]
    fn every_page_occurs_once_and_headers_are_not_items() {
        let p = pages();
        assert_eq!(p.len(), super::super::workbench::entries().len());
        let set: std::collections::HashSet<_> = p.iter().collect();
        assert_eq!(set.len(), p.len());
        let mut n = Navigation::default();
        n.resize(document_height() + 100.);
        for section in sections() {
            assert_eq!(n.hit(section.rect.center()), None);
            for (page, r) in &section.items {
                assert_eq!(n.hit(r.center()), Some(page.index()));
            }
        }
    }
    #[test]
    fn a_short_sidebar_reveals_the_last_item_and_inverse_hits_it() {
        let mut n = Navigation::default();
        n.resize(480.0);
        let page = *pages().last().unwrap();
        n.reveal(page);
        assert!(n.scroll.offset.y > 0.0);
        let p = row(page).center();
        let p = Point::new(p.x, p.y - n.scroll.offset.y);
        assert!(n.bounds().contains(p));
        assert_eq!(n.hit(p), Some(page.index()));
    }

    #[test]
    fn navigation_draw_and_hit_share_geometry_across_scroll_bounce_and_filter() {
        let mut n = Navigation::default();
        n.resize(360.0);
        let page = pages()[5];
        let row = n
            .sections()
            .iter()
            .flat_map(|section| &section.items)
            .find(|(candidate, _)| *candidate == page)
            .unwrap()
            .1;

        n.scroll.scroll_by(0.0, 85.0);
        let geometry = n.geometry();
        let origin = geometry.content_origin();
        let drawn = row.offset(origin.x, origin.y);
        assert_eq!(drawn, row.offset(0.0, -n.scroll.offset.y));
        let point = drawn.center();
        assert_eq!(n.hit(point), Some(page.index()));
        assert!(
            geometry
                .visible_document_rect()
                .is_some_and(|visible| row.intersection(&visible).is_some())
        );

        // The fixed search field and the elastic blank above the first item
        // never become navigation hit targets.
        assert_eq!(n.hit(search_rect().center()), None);
        n.scroll.offset = Point::default();
        n.scroll.overscroll.y = -24.0;
        assert_eq!(
            n.hit(Point::new(row.center().x, Navigation::content_top() + 1.0)),
            None
        );

        n.scroll.overscroll.y = 0.0;
        n.scroll.scroll_by(0.0, f32::MAX);
        n.scroll.overscroll.y = 24.0;
        assert_eq!(
            n.hit(Point::new(row.center().x, n.bounds().max_y() - 1.0)),
            None
        );

        n.scroll.overscroll.y = 0.0;
        n.set_query("component-button");
        let (filtered_page, filtered_row) = n.sections()[0].items[0];
        let geometry = n.geometry();
        let drawn = filtered_row.offset(geometry.content_origin().x, geometry.content_origin().y);
        assert_eq!(n.hit(drawn.center()), Some(filtered_page.index()));
        n.resize(280.0);
        let geometry = n.geometry();
        let point = filtered_row
            .offset(geometry.content_origin().x, geometry.content_origin().y)
            .center();
        assert!(
            geometry
                .visible_viewport_rect()
                .is_some_and(|visible| visible.contains(point))
        );
        assert_eq!(n.hit(point), Some(filtered_page.index()));
    }
}
