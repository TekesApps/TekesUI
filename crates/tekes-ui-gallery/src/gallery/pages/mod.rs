//! Shared Gallery page registry types and helper modules.
#[cfg(test)]
use tekes_ui_tokens::Theme;

#[cfg(test)]
use super::static_text::StaticText;
#[cfg(test)]
use super::text::TextEngine;

pub mod icons;
pub mod motion;
pub mod primitives;
pub mod showcase;
pub mod static_text;
pub mod tokens;

/// Context passed into shared page calibration and preview helpers.
#[cfg(test)]
pub struct PageCtx<'a> {
    pub theme: Theme,
    pub text: &'a mut TextEngine,
    pub static_text: &'a mut StaticText,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Workbench(usize),
}

impl Page {
    pub const ALL: [Page; include!(concat!(env!("OUT_DIR"), "/workbench_count.rs"))] =
        include!(concat!(env!("OUT_DIR"), "/workbench_pages.rs"));

    pub fn name(self) -> &'static str {
        match self {
            Page::Workbench(i) => super::workbench::entry(i).title(),
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Page::Workbench(i) => super::workbench::entry(i).slug(),
        }
    }

    #[cfg(test)]
    pub fn index(self) -> usize {
        let Page::Workbench(i) = self;
        i
    }
}

#[cfg(test)]
mod tests {
    use super::Page;

    #[test]
    fn index_matches_all_order() {
        for (i, p) in Page::ALL.iter().enumerate() {
            assert_eq!(p.index(), i, "{p:?}");
        }
    }

    #[test]
    fn names_and_slugs_are_unique() {
        let mut names = std::collections::HashSet::new();
        let mut slugs = std::collections::HashSet::new();
        for p in Page::ALL {
            assert!(names.insert(p.name()), "重名：{p:?}");
            assert!(slugs.insert(p.slug()), "重 slug：{p:?}");
        }
    }
}
