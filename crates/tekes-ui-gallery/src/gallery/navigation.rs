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
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../../../design/gallery/navigation.json"))
                .expect("navigation JSON");
        let mut y = layout::CATALOG_TOP;
        let mut all = Vec::new();
        for s in source["sections"].as_array().expect("sections") {
            let h = proposal::TextRole::UiCaption.style().line_height + Spacing::Regular.value();
            let rect = Rect::new(
                Spacing::Loose.value(),
                y,
                layout::CATALOG_W - 2.0 * Spacing::Loose.value(),
                h,
            );
            y += h;
            let mut items = Vec::new();
            for slug in s["items"].as_array().expect("items") {
                let slug = slug.as_str().expect("page slug");
                let page = Page::ALL
                    .into_iter()
                    .find(|p| p.slug() == slug)
                    .expect("registered navigation page");
                items.push((
                    page,
                    Rect::new(
                        layout::ROW_INSET_X,
                        y,
                        layout::CATALOG_W - 2.0 * layout::ROW_INSET_X,
                        layout::ROW_H,
                    ),
                ));
                y += layout::ROW_H + layout::ROW_GAP;
            }
            all.push(Section {
                title: s["title"].as_str().expect("section title").into(),
                rect,
                items,
            });
            y += Spacing::Loose.value();
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
pub fn row(page: Page) -> Rect {
    sections()
        .iter()
        .flat_map(|s| s.items.iter())
        .find(|v| v.0 == page)
        .expect("page in JSON navigation")
        .1
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
}
impl Default for Navigation {
    fn default() -> Self {
        let mut n = Self {
            scroll: ScrollArea::default(),
            height: layout::FRAME_H,
        };
        n.resize(layout::FRAME_H);
        n
    }
}
impl Navigation {
    pub fn resize(&mut self, height: f32) {
        if height.is_finite() && height > 0.0 {
            self.height = height;
            self.scroll.layout(
                Size::new(layout::CATALOG_W, height),
                Size::new(layout::CATALOG_W, document_height()),
            );
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, layout::CATALOG_W, self.height)
    }
    pub fn hit(&self, p: Point) -> Option<usize> {
        if !self.bounds().contains(p) {
            return None;
        }
        let y = p.y + self.scroll.visual_offset().y;
        sections()
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|(_, r)| r.inset(-layout::ROW_GAP * 0.5).contains(Point::new(p.x, y)))
            .map(|v| v.0.index())
    }
    pub fn reveal(&mut self, page: Page) {
        self.scroll.reveal(row(page));
    }
    pub fn track(&self) -> Rect {
        Rect::new(
            layout::CATALOG_W - Spacing::Regular.value(),
            Spacing::Regular.value(),
            Spacing::Tight.value(),
            (self.height - 2.0 * Spacing::Regular.value()).max(0.0),
        )
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        if self.scroll.limit().y <= 0.0 {
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_page_occurs_once_and_headers_are_not_items() {
        let p = pages();
        assert_eq!(p.len(), Page::ALL.len());
        let set: std::collections::HashSet<_> = p.iter().collect();
        assert_eq!(set.len(), Page::ALL.len());
        let n = Navigation::default();
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
}
