//! Showcase grouping and selected illustration states. Design names/states stay in JSON.
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub component_ids: Vec<String>,
}

#[derive(Debug)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub states: Vec<String>,
    pub default_state: String,
}

#[derive(Debug)]
struct Catalog {
    groups: Vec<Group>,
    components: HashMap<String, Component>,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let grouping: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../design/gallery/catalog.json"
        )).expect("valid Gallery catalog JSON");
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json"
        )).expect("valid component registry JSON");
        let groups = grouping["groups"].as_array().expect("groups").iter().map(|g| Group {
            id: g["id"].as_str().expect("group ID").into(),
            name: g["name"].as_str().expect("group name").into(),
            component_ids: g["component_ids"].as_array().expect("group components").iter().map(|v| v.as_str().expect("component ID").into()).collect(),
        }).collect();
        let components = registry["components"].as_array().expect("components").iter().map(|c| {
            let id = c["id"].as_str().expect("component ID");
            let default_state = grouping["default_states"][id].as_str().unwrap_or("rest");
            let states: Vec<String> = c["states"].as_array().expect("component states").iter().map(|v| v.as_str().expect("state ID").into()).collect();
            assert!(states.iter().any(|s| s == default_state), "invalid default state for {id}");
            (id.into(), Component { id: id.into(), name: c["name"].as_str().expect("component name").into(), states, default_state: default_state.into() })
        }).collect();
        Catalog { groups, components }
    })
}

pub fn group(id: &str) -> &'static Group {
    catalog()
        .groups
        .iter()
        .find(|g| g.id == id)
        .expect("registered showcase group")
}

pub fn component(id: &str) -> &'static Component {
    catalog()
        .components
        .get(id)
        .expect("registered showcase component")
}

#[derive(Debug, Default)]
pub struct ShowcaseState {
    selected: HashMap<String, String>,
}

impl ShowcaseState {
    pub fn selected<'a>(&'a self, component: &'a Component) -> &'a str {
        self.selected
            .get(&component.id)
            .map(String::as_str)
            .unwrap_or(&component.default_state)
    }

    pub fn select(&mut self, id: &str, next: &str) -> bool {
        let Some(component) = catalog().components.get(id) else {
            return false;
        };
        if !component.states.iter().any(|s| s == next) || self.selected(component) == next {
            return false;
        }
        self.selected.insert(id.into(), next.into());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_forty_components_are_grouped_exactly_once() {
        let c = catalog();
        let ids: Vec<_> = c.groups.iter().flat_map(|g| &g.component_ids).collect();
        assert_eq!(ids.len(), 40);
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 40);
        assert_eq!(
            ids.into_iter().map(String::as_str).collect::<HashSet<_>>(),
            c.components.keys().map(String::as_str).collect()
        );
        assert_eq!(c.groups.len(), 8);
    }

    #[test]
    fn state_selection_rejects_unknown_and_avoids_duplicate_redraws() {
        let mut state = ShowcaseState::default();
        let button = component("tui.button");
        assert_eq!(state.selected(button), "rest");
        assert!(!state.select("tui.button", "rest"));
        assert!(state.select("tui.button", "pressed"));
        assert_eq!(state.selected(button), "pressed");
        assert!(!state.select("tui.button", "pressed"));
        assert!(!state.select("tui.button", "composing"));
        assert!(!state.select("unknown", "rest"));
    }
}
