//! Independent consumer of the public Theme/Surface API; no copied engine.
pub use tekes_ui_tokens::{Palette, Rgba};
use tekes_ui_tokens::surface_palette;
use tekes_ui_design_export::theme_system;
use surface_palette::{
    SurfaceError, SurfaceOptions, canonical_surface, resolve_surface, resolve_surface_with_options,
    surface_names, resolve_toggle_surface, resolve_range_surface,
};
use tekes_ui_tokens::Theme;
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tokens =
        serde_json::from_slice(&std::fs::read(root.join("design/tokens.json")).unwrap()).unwrap();
    theme_system::verify_theme_system(&root, &tokens).unwrap();
    // The dependency slice must be consumed through real public Theme identities.
    for mode in Theme::ALL {
        assert!(Palette::for_theme_definition(mode, "missing-theme").is_none());
        let default = Palette::for_theme_definition(mode, "default").unwrap();
        let blue = Palette::for_theme_definition(mode, "blue").unwrap();
        let clay = Palette::for_theme_definition(mode, "clay").unwrap();
        assert!(matches!(blue.source(), tekes_ui_tokens::PaletteSource::ThemeDefinition { .. }));
        assert_ne!(blue.source(), clay.source());
        assert_eq!(blue.brand_index(), None);
        assert_ne!(blue.color("action-primary-ground"), clay.color("action-primary-ground"));
        for role in ["primary-text", "secondary-text", "border-focused"] {
            assert_eq!(blue.color(role), clay.color(role));
            assert_eq!(default.color(role), Palette::from(mode).color(role));
        }
        for selected in [blue, clay] {
            let packet = resolve_surface(selected, "canvas", "action-primary", "rest").unwrap();
            assert_eq!(Some(packet.background), selected.color("action-primary-ground"));
            assert_eq!(packet.background.a, Palette::from(mode).color("action-primary-ground").unwrap().a);
        }
    }
    assert_eq!(surface_names().len(), 7);
    for name in surface_names() {
        assert_eq!(canonical_surface(name), Ok(*name));
    }
    for unknown in ["", "unknown", "Panel", "#FFFFFF"] {
        assert!(canonical_surface(unknown).is_err());
    }
    let mut pairs = 0;
    let rules: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("design/theme/rules.json")).unwrap())
            .unwrap();
    let instances: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("design/theme/generated/instances.json")).unwrap()).unwrap();
    for theme in Theme::ALL {
        for preset in tekes_ui_tokens::palette::gallery_brand_presets() {
            let palette = Palette::new(theme, preset.palette_index).unwrap();
            for (surface, definition) in rules["surfaces"].as_object().unwrap() {
                for usage in definition["usages"].as_object().unwrap().keys() {
                    // Generated instances were verified against the same rules above.
                    // Enumerate their states instead of assuming every new purpose
                    // supports the action/selection state vocabulary.
                    let states = instances["instances"][0]["modes"]["light"][surface][usage]
                        .as_object().unwrap().keys();
                    for state in states {
                        let colors = resolve_surface(palette, surface, usage, state).unwrap();
                        assert!(
                            colors.background.in_range()
                                && colors.text.in_range()
                                && colors.icon.in_range()
                        );
                        if state.contains("disabled") {
                            assert!(colors.focus.is_none());
                        }
                        pairs += 1;
                    }
                }
                let rest = resolve_surface(palette, surface, "action-quiet", "rest").unwrap();
                let hover = resolve_surface(palette, surface, "action-quiet", "hover").unwrap();
                assert_ne!(rest.text, hover.text);
                assert_ne!(rest.icon, hover.icon);
                let options = SurfaceOptions {
                    error: true,
                    read_only: true,
                };
                let input =
                    resolve_surface_with_options(palette, surface, "input", "rest", options)
                        .unwrap();
                assert_eq!(input.background, palette.color("surface-1").unwrap());
                assert_eq!(input.text, palette.color("primary-text").unwrap());
                assert_eq!(input.boundary, palette.color("status-failed"));
                assert_eq!(input.helper, palette.color("status-failed"));
                assert_eq!(input.focus, palette.color("border-focused"));
                assert_eq!(
                    resolve_surface_with_options(palette, surface, "content", "rest", options),
                    Err(SurfaceError::UnknownCombination)
                );
                let toggle = resolve_toggle_surface(palette, surface, "selected-hover", SurfaceOptions { error: true, read_only: false }).unwrap();
                assert_eq!(toggle.track.boundary, palette.color("status-failed"));
                assert_eq!(toggle.thumb.boundary, palette.color("border"));
                assert_eq!(toggle.label.text, palette.color("primary-text").unwrap());
                for state in ["rest","hover","pressed","disabled"] {
                    let range = resolve_range_surface(palette,surface,state).unwrap();
                    assert_eq!(range.track,resolve_surface(palette,surface,"range-track",state).unwrap());
                    assert_eq!(range.thumb,resolve_surface(palette,surface,"range-thumb",state).unwrap());
                    assert_eq!(range.label,resolve_surface(palette,surface,"control-label",state).unwrap());
                    if state == "disabled" {assert!(range.thumb.focus.is_none());}
                }
                assert_eq!(resolve_range_surface(palette,surface,"selected"),Err(SurfaceError::UnknownCombination));
                let mark = resolve_surface(palette, surface, "check-checked", "hover").unwrap();
                assert_eq!(mark.marker, palette.color("control-checked-mark"));
                let label = resolve_surface(palette, surface, "content", "rest").unwrap();
                assert_eq!(label.text, palette.color("primary-text").unwrap());
                let selection =
                    resolve_surface(palette, surface, "text-selection", "rest").unwrap();
                assert_eq!(
                    selection.background,
                    palette.color("text-selection").unwrap()
                );
                assert_eq!(selection.text, palette.color("selected-text").unwrap());
            }
            for (surface, usage, state) in [
                ("unknown", "input", "rest"),
                ("panel", "unknown", "rest"),
                ("panel", "input", "unknown"),
                ("panel", "content", "pressed"),
            ] {
                assert_eq!(
                    resolve_surface(palette, surface, usage, state),
                    Err(SurfaceError::UnknownCombination)
                );
            }
        }
    }
    println!(
        "THEME_SURFACE_CONTRACT_OK pairs={pairs} public_integration=true component_consumption=false native=false gpu=false"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
    fn load(path: &str) -> Value {
        serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
    }
    #[test]
    fn rust_recomputes_python_bindings_and_rejects_incomplete_definitions() {
        let rules = load("design/theme/rules.json");
        assert_eq!(
            theme_system::compile_surface_rules(&rules)
                .unwrap()
                .as_bytes(),
            std::fs::read(root().join("design/theme/generated/surface_rules.rs")).unwrap()
        );
        for path in [
            "/profiles/quiet/hover",
            "/profiles/quiet/hover/icon",
            "/surfaces/panel/usages/input",
        ] {
            let mut bad = rules.clone();
            let (parent, key) = path.rsplit_once('/').unwrap();
            bad.pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(theme_system::compile_surface_rules(&bad).is_err(), "{path}");
        }
        let mut added = rules.clone();
        added["surfaces"]["inspector"] = rules["surfaces"]["panel"].clone();
        assert!(
            theme_system::compile_surface_rules(&added)
                .unwrap()
                .contains("(\"inspector\", \"action-quiet\", \"hover\")")
        );
        for mutation in ["cycle", "unknown", "override"] {
            let mut bad = rules.clone();
            if mutation == "cycle" {
                bad["profiles"]["readonly"]["extends"] = json!("readonly");
            }
            if mutation == "unknown" {
                bad["profiles"]["readonly"]["extends"] = json!("unknown");
            }
            if mutation == "override" {
                bad["profiles"]["readonly"]["overrides"]["unknown"] = json!("primary-text");
            }
            assert!(theme_system::compile_surface_rules(&bad).is_err());
        }
        let mut bad = rules;
        bad["profiles"]["quiet"]["disabled"]["focus"] = json!("border-focused");
        assert!(theme_system::compile_surface_rules(&bad).is_err());
    }
    #[test]
    fn rust_consumption_rejects_modified_generated_outputs_and_unknown_role() {
        let source = root();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!(
            "tekesui-theme-negative-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&temp).unwrap();
        let output = load("design/theme/generated/instances.json");
        for path in output["inputs_sha256"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .chain([
                "design/theme/generated/instances.json",
                "design/theme/generated/surface_rules.rs",
                "design/theme/artifact.lock.json",
                "design/gallery/brands.json",
                "design/brand/gallery.json",
                "design/tokens.json",
            ])
        {
            let to = temp.join(path);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(source.join(path), to).unwrap();
        }
        let tokens = load("design/tokens.json");
        theme_system::verify_theme_system(&temp, &tokens).unwrap();
        let path = temp.join("design/theme/generated/instances.json");
        let mut bad = output.clone();
        bad["instances"][0]["modes"]["light"]["panel"]["action-quiet"]["hover"]["text"] =
            json!("#FFFFFF");
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            theme_system::verify_theme_system(&temp, &tokens)
                .unwrap_err()
                .contains("does not match its reviewed source lock")
        );
        let mut bad = output.clone();
        bad["instances"][0]["modes"]
            .as_object_mut()
            .unwrap()
            .remove("dark-ic");
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(theme_system::verify_theme_system(&temp, &tokens).is_err());
        let mut bad = output.clone();
        bad["inputs_sha256"]
            .as_object_mut()
            .unwrap()
            .remove("design/theme/rules.json");
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(theme_system::verify_theme_system(&temp, &tokens).is_err());
        std::fs::copy(source.join("design/theme/generated/instances.json"), &path).unwrap();
        std::fs::write(
            temp.join("design/theme/generated/surface_rules.rs"),
            b"tampered while hashes retained",
        )
        .unwrap();
        assert!(theme_system::verify_theme_system(&temp, &tokens).is_err());
        std::fs::copy(
            source.join("design/theme/generated/surface_rules.rs"),
            temp.join("design/theme/generated/surface_rules.rs"),
        )
        .unwrap();
        let mut bad = tokens.clone();
        for color in bad["proposal"]["tokens"]["color"]["tokens"]
            .as_array_mut()
            .unwrap()
        {
            if color["name"] == "primary-text" {
                color["name"] = json!("renamed");
            }
        }
        assert!(
            theme_system::verify_theme_system(&temp, &bad)
                .unwrap_err()
                .contains("unknown color role")
        );
        std::fs::remove_dir_all(temp).unwrap();
    }
    #[test]
    fn complete_contract_smoke() {
        main();
    }
}
