use super::Entry;
use std::sync::OnceLock;
use tekes_ui_tokens::proposal;

/// Shared heading for migrated stories. The existing proposal heading role
/// restores hierarchy; it is not a pixel-equivalence claim for legacy chrome.
pub(crate) fn story_heading(
    id: u64,
    value: impl Into<String>,
) -> tekes_ui_controls::composition::Content {
    use tekes_ui_controls::composition::{ComponentId, Content, TextContent};
    let mut content = TextContent::new(value);
    content.role = proposal::TextRole::UiHeading;
    Content::new(ComponentId(id), content)
}

pub(crate) fn api_reference(entry: &Entry) -> &str {
    if !entry.get("api").is_empty() {
        return entry.get("api");
    }
    if entry.get("kind") == "component" {
        return "Missing public API registration; registry validation failed";
    }
    // Component API references come from the authoritative JSON registry and
    // are compiled by the public integration test, never guessed per renderer.
    match entry.renderer() {
        "form" | "settings" | "async" => "Gallery pattern using TextInput, FieldView and Button",
        "filter" => "Gallery pattern using TextInput and local file filtering",
        "master" => "Gallery pattern using keyed List and TextSurface",
        _ => "Gallery fixture; see the route renderer",
    }
}

pub(crate) fn contract_summary(entry: &Entry) -> Vec<String> {
    if entry.get("kind") != "component" {
        return Vec::new();
    }
    static CAPABILITIES: OnceLock<serde_json::Value> = OnceLock::new();
    let inventory = CAPABILITIES.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../design/gallery/capabilities.json"
        ))
        .expect("current component capabilities")
    });
    let Some(component) = inventory["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|component| component["id"] == entry.component())
    else {
        return vec![format!("Missing contract coverage: {}", entry.component())];
    };
    let states = component["state_coverage"]["required"].as_array().unwrap();
    let pending = component["state_coverage"]["fixture_unverified"]
        .as_array()
        .unwrap()
        .len();
    let names = states
        .iter()
        .map(|state| state.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(", ");
    let references = component["token_dependencies"]["contract_references"]
        .as_array()
        .unwrap();
    let mut token_line = if references.is_empty() {
        "No per-component token references in the existing JSON contract; runtime token dependencies remain unverified.".to_owned()
    } else {
        let names = references
            .iter()
            .map(|reference| {
                format!(
                    "{}:{}",
                    reference["family"].as_str().unwrap(),
                    reference["token"].as_str().unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let owner = component["token_dependencies"]["contract_owner"]
            .as_str()
            .unwrap();
        let inherited = if owner == entry.component() {
            String::new()
        } else {
            format!(" (shared with {owner})")
        };
        format!(
            "JSON contract token references{inherited}: {names}. Runtime token dependencies remain unverified."
        )
    };
    if let Some(observed) = component["token_dependencies"].get("runtime_observation") {
        let count = observed["case_count"].as_u64().unwrap();
        let tokens = observed["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|token| token.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(", ");
        token_line.push_str(&format!(
            " Observed public-API lookup scope ({count} cases, incomplete): {tokens}."
        ));
    }
    vec![
        format!("Design-required states: {names}."),
        format!(
            "State coverage: {}/{} fixtures source-reviewed; runtime and native acceptance remain unverified.",
            states.len() - pending,
            states.len()
        ),
        token_line,
    ]
}

pub(crate) fn overview_copy(e: &Entry) -> &'static str {
    if e.slug() == "getting-started" {
        "Open a component from the sidebar or search by name, ID or story keyword.\n\nDirect route: TEKESUI_GALLERY_PAGE=component-button/playground\n\nUse Preview for semantic brand presets, density, text scale and constrained width. Appearance keeps the existing Light / Dark and Increase Contrast choices.\n\nReset restores this story. The event log records real input and fixture transitions."
    } else {
        "Foundations explain the design rules. Components exercise reusable Rust controls. Patterns combine them, Examples present complete workflows, and Quality lab keeps calibration and acceptance evidence.\n\nStart with Component index, or search for Button, TextField, ScrollArea or Settings. A planned label reserves a missing capability; experimental means interactive but not fully accepted for production.\n\nStories use deterministic local data. Async fixtures resolve explicitly, so you can repeat failure and recovery without a network or user files."
    }
}

pub(crate) fn behavior_copy(renderer: &str) -> &'static str {
    match renderer {
        "button" => {
            "Tab focuses the action. Space activates on release; Enter activates on key down. Releasing the pointer outside cancels. Disabled and loading actions do not activate."
        }
        "checkbox" => {
            "Tab focuses the checkbox. Space toggles on release; Enter does not toggle it. Pointer activation needs matching press and release. Disabled state blocks changes; Mixed is a fixture state."
        }
        "tabs" => {
            "Tab enters the tab strip. Left/Right wrap selection. Home/End select endpoints. The panel follows the active tab."
        }
        "menu" | "select" => {
            "Enter or Space opens. Up/Down move through options. Enter selects. Escape dismisses and restores focus."
        }
        "dialog" | "danger" => {
            "Enter opens. Tab/Shift+Tab stay inside the modal scope. Escape cancels. Closing restores focus to the invoker. The destructive fixture changes local state only."
        }
        "field" | "search" => {
            "Click or Tab to focus the single-line input. Native text services handle IME composition, selection, clipboard, undo and redo. Tab leaves the field. Disabled input rejects edits."
        }
        "area" => {
            "Click or Tab to focus the multiline editor. Native text services handle IME composition, selection, clipboard and undo. The editor fills the remaining preview; its document scrolls inside that frame."
        }
        "text" => {
            "This surface is read-only. Pointer and keyboard selection, Copy and scrolling remain available; editing commands do not change the text. Appending fixture text preserves a reading anchor; Return to latest follows the document end."
        }
        "scroll" => {
            "Wheel and trackpad input scroll the read-only text document inside its viewport. Resize preserves the reading anchor and an existing bottom anchor. Append paragraph and Return to latest are separate fixture actions."
        }
        "icon" => {
            "Search filters the local icon catalog; category and grid controls update the visible fixture. The preview renders canonical icon assets. Individual decorative glyphs do not activate button actions."
        }
        "surface" => {
            "Surface/Panel provide a semantic container and border, with no container activation. The nested action is a real Button with its own focus and input handling."
        }
        "toolbar" => {
            "Tab enters the toolbar. Left/Right and Home/End move its focused action. Enter or Space activates the focused Create, Rename or Remove fixture action; releasing outside cancels pointer activation."
        }
        "number-field" => {
            "Tab focuses a stepper action. Arrow keys, Home/End and the decrement/increment actions change the bounded 1–32 value. Bound-disabled actions do not change it. Editable numeric text is not implemented."
        }
        "field-meta" => {
            "FieldView presents a label, help and error message around the real text input. Error changes the message and input presentation; Disabled blocks edits and the associated fixture actions. This metadata specimen does not save settings."
        }
        "skeleton" => {
            "Skeleton is a static loading placeholder with no selection, focus or activation behavior. Reset restores the initial Gallery fixture; no timer or network request resolves this specimen."
        }
        "form" | "settings" | "async" => {
            "Edit the workspace name and notification preference, then Save. Empty names fail validation. Resolve fixture explicitly completes a pending local attempt: the first fails and Retry can succeed. Disabled blocks edits and save actions. Reset restores fixture data; no real network request runs."
        }
        "filter" => {
            "Type in the real text input to filter three local file names. Clear filter empties the query; an empty query shows all files. No-match results are local and deterministic."
        }
        "master" => {
            "The document list and detail text have separate scroll areas. Pointer selection and Up/Down/Home/End choose a document; keyboard focus reveals the selected row. The detail surface is read-only and supports selection and Copy."
        }
        _ => {
            "This route has no component-specific input contract beyond its documented fixture controls. See Playground and the registered API for the implemented behavior."
        }
    }
}

pub(crate) fn story_docs(entry: &Entry) -> &str {
    if entry.renderer() == "master" {
        behavior_copy("master")
    } else if entry.renderer() == "filter" {
        "Type to filter the local file fixture. Clear filter resets the query; an empty query shows all three items. Filtering is deterministic and does not resolve a network request."
    } else {
        entry.get("docs")
    }
}
