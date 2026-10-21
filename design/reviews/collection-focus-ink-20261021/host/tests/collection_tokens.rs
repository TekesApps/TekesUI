#![cfg(all(feature = "token-audit", target_os = "macos"))]
include!("../../../collection-focus-availability-20261020/host/src/text_backend.rs");
include!("../../../collection-focus-availability-20261020/host/src/fixture.rs");
use tekes_ui_tokens::{Palette, Theme};
#[test]
fn actual_collection_render_states_and_recovery_use_observed_tokens() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for family in [
                Family::List,
                Family::Tree,
                Family::TableRow,
                Family::TableHeader,
                Family::Chart,
            ] {
                for scenario in [
                    "rest",
                    "focus",
                    "disabled",
                    "selected",
                    "inactive",
                    "late-inactive-restored",
                    "late-disabled-restored",
                    "fresh-inactive-restored",
                    "fresh-disabled-restored",
                ] {
                    if family == Family::Chart && scenario.contains("disabled") {
                        continue;
                    }
                    let scenario = if family == Family::TableHeader && scenario == "selected" {
                        "sort"
                    } else {
                        scenario
                    };
                    let mut story = ExtendedStory::new();
                    let mut text = TextBackend::new(1., 1.);
                    let palette = Palette { theme, brand };
                    story.paint(family, &mut Scene::new(), &mut text, 300., palette);
                    match scenario {
                        "focus" => story.focus(family),
                        "disabled" => {
                            story.disabled(family, true);
                            story.focus(family);
                        }
                        "selected" | "sort" => story.select(family),
                        "inactive" => {
                            story.active(family, false);
                            story.focus(family);
                        }
                        "late-inactive-restored" | "fresh-inactive-restored" => {
                            story.active(family, false);
                            story.focus(family);
                            story.active(family, true);
                            if scenario.starts_with("fresh") {
                                story.focus(family);
                            }
                        }
                        "late-disabled-restored" | "fresh-disabled-restored" => {
                            story.disabled(family, true);
                            story.focus(family);
                            story.disabled(family, false);
                            if scenario.starts_with("fresh") {
                                story.focus(family);
                            }
                        }
                        _ => {}
                    }
                    if scenario == "sort" {
                        assert_eq!(story.table.selected_row, None);
                        assert!(story.table.sort.is_some());
                    }
                    let mut scene = Scene::new();
                    let ((), reads) = tekes_ui_tokens::audit::capture(|| {
                        story.paint(family, &mut scene, &mut text, 300., palette)
                    });
                    assert!(!scene.is_empty());
                    assert!(!reads.is_empty());
                    let state = if scenario == "disabled" {
                        "disabled"
                    } else if scenario == "selected" {
                        "selected"
                    } else if scenario == "focus" || scenario.starts_with("fresh") {
                        "focus"
                    } else {
                        "rest"
                    };
                    if state == "focus" {
                        assert_ne!(
                            story.key(family),
                            E::Ignored,
                            "fresh public focus and exact production adapter must own keyboard input"
                        );
                    } else {
                        assert_eq!(
                            story.key(family),
                            E::Ignored,
                            "nonfocused/restored/disabled input stays unowned"
                        );
                    }
                    cases.push(serde_json::json!({"component":family.component(),"target":format!("{family:?}"),"content_state":state,"scenario":scenario,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 504);
    if let Ok(path) = std::env::var("TEKESUI_COLLECTION_FOCUS_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","scope":"Actual public List/Tree/DataTable row+header/Chart Views, exact Gallery focus_changed, CoreText. Asserted rest/focus/selected and applicable disabled, inactive/restored/fresh focus;4themes/3brands. Not exhaustive state coverage.","complete":false,"excluded":["hover/pressed and application loading/error/empty statuses","Chart has no disabled model contract","constant and nonlookup dependencies","full Gallery native/global focus or ClientKit/Gold7 visual acceptance"],"cases":cases})).unwrap()).unwrap();
    }
}
