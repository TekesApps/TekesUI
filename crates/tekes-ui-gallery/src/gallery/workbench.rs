//! Registry-driven AppHost workbench. Fixtures orchestrate production control models;
//! no duplicate component implementation lives in a story.
mod accessibility;
mod apphost_actions;
pub(super) mod apphost_basic;
pub(super) mod apphost_brand;
mod apphost_collections;
mod apphost_command;
mod apphost_display;
pub(super) mod apphost_documents;
pub(super) mod apphost_feedback;
mod apphost_field_meta;
pub(crate) mod apphost_foundations;
pub(crate) mod apphost_icons;
mod apphost_input_sections;
mod apphost_layout;
pub(crate) mod apphost_motion;
mod apphost_navigation;
pub(super) mod apphost_patterns;
pub(crate) mod apphost_quality;
pub(crate) mod apphost_scene3d;
mod apphost_selection;
mod apphost_split;
pub(super) mod apphost_story;
pub(crate) mod apphost_text_pages;
mod basic_states;
mod brand_preview;
mod durable_draft;
mod extended;
mod master_host;
mod search_host;
mod search_input;
mod select_story;
mod story_copy;
mod story_state;

use super::Page;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct Entry(serde_json::Value);
impl Entry {
    fn get(&self, key: &str) -> &str {
        self.0[key].as_str().unwrap_or("")
    }
    pub fn title(&self) -> &str {
        self.get("title")
    }
    pub fn slug(&self) -> &str {
        self.get("slug")
    }
    pub fn section(&self) -> &str {
        self.get("section")
    }
    pub fn renderer(&self) -> &str {
        self.get("renderer")
    }
    pub fn planned(&self) -> bool {
        self.get("kind") == "planned"
    }
    pub fn component(&self) -> &str {
        self.get("component")
    }
    /// One capability contract for visible tabs, direct routes and Docs links.
    pub fn supports_story(&self, story: &str) -> bool {
        match story {
            "docs" | "playground" => true,
            "states" => {
                (!self.planned()
                    && [
                        "button",
                        "checkbox",
                        "tabs",
                        "menu",
                        "dialog",
                        "field",
                        "area",
                        "search",
                        "select",
                        "field-meta",
                    ]
                    .contains(&self.renderer()))
                    || (!self.planned() && basic_states::supports(self.renderer()))
                    || (!self.planned() && extended::supports_story(self.renderer(), story))
            }
            "variants" => {
                !self.planned()
                    && [
                        "button",
                        "field",
                        "area",
                        "search",
                        "surface",
                        "number-field",
                    ]
                    .contains(&self.renderer())
                    || (!self.planned() && extended::supports_story(self.renderer(), story))
            }
            "edge-cases" => {
                !self.planned()
                    && [
                        "button", "checkbox", "field", "area", "search", "form", "settings",
                        "async",
                    ]
                    .contains(&self.renderer())
            }
            "behaviors" => {
                if self.renderer() == "scene3d" {
                    false
                } else if extended::supports(self.renderer()) {
                    extended::supports_story(self.renderer(), story)
                } else {
                    !self.planned()
                        && ["component", "pattern", "example"].contains(&self.get("kind"))
                }
            }
            _ => false,
        }
    }
    pub fn matches(&self, query: &str) -> bool {
        let terms=format!("{} {} {} {} {} {}",self.title(),self.slug(),self.section(),self.component(),self.get("keywords"),if self.planned(){"planned"}else{"docs playground states state matrix variants sizes behaviors keyboard edge-cases hover pressed focus disabled loading"}).to_lowercase();
        if query.starts_with(&format!("{}/", self.slug())) {
            return route(query).is_some();
        }
        query.split_whitespace().all(|term| terms.contains(term))
    }
}
pub fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../../../design/gallery/workbench.json"))
                .expect("workbench registry");
        v["entries"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(Entry)
            .collect()
    })
}
pub fn entry(i: usize) -> &'static Entry {
    &entries()[i]
}
pub fn route(route: &str) -> Option<Page> {
    let (slug, story) = route.split_once('/').unwrap_or((route, "playground"));
    if ![
        "docs",
        "playground",
        "states",
        "behaviors",
        "edge-cases",
        "variants",
    ]
    .contains(&story)
    {
        return None;
    }
    entries()
        .iter()
        .position(|e| (e.slug() == slug || e.component() == slug) && e.supports_story(story))
        .map(Page::Workbench)
        .or_else(|| Page::ALL.into_iter().find(|p| p.slug() == route))
}

#[cfg(test)]
pub(super) use super::PageCtx;
use search_input::StoryInput;
pub(crate) use story_copy::{
    api_reference, behavior_copy, contract_summary, overview_copy, story_docs, story_heading,
};
pub use story_state::{Phase, StoryState};
