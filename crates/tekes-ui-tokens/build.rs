use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest_dir.join("../..");
    let assets_dir = manifest_dir.join("assets");
    println!("cargo:rerun-if-env-changed=TEKESUI_DESIGN_ARTIFACT");
    println!("cargo:rerun-if-env-changed=TEKESUI_THEME_LOCK");
    let artifact = env::var_os("TEKESUI_DESIGN_ARTIFACT").map(PathBuf::from);
    let theme_lock = env::var_os("TEKESUI_THEME_LOCK").map(PathBuf::from);
    if theme_lock.is_some() && artifact.is_none() {
        panic!(
            "TEKESUI_THEME_LOCK requires TEKESUI_DESIGN_ARTIFACT to select the consumer Theme artifact root"
        );
    }
    if let (Some(artifact), Some(lock)) = (&artifact, &theme_lock) {
        assert!(
            artifact.is_absolute() && lock.is_absolute(),
            "locked Theme builds require absolute TEKESUI_DESIGN_ARTIFACT and TEKESUI_THEME_LOCK paths"
        );
    }
    let workspace_mode = root.join("design/tokens.json").is_file();
    if artifact.is_some() && !workspace_mode {
        panic!(
            "TEKESUI_DESIGN_ARTIFACT is only supported from a full TekesUI workspace checkout with design/"
        );
    }
    let json = if let Some(artifact) = &artifact {
        artifact.join("design/tokens.json")
    } else if workspace_mode {
        root.join("design/tokens.json")
    } else {
        assets_dir.join("tokens.json")
    };
    println!("cargo:rerun-if-changed={}", json.display());
    // Unit migration checks compare against the selected author projection,
    // including an explicit consumer artifact, rather than the default brand.
    println!(
        "cargo:rustc-env=TEKESUI_SELECTED_TOKEN_JSON={}",
        json.display()
    );
    let text = fs::read_to_string(&json)
        .expect("design/tokens.json must be available when building tokens");
    let document = tekes_ui_design_export::parse_json(&text).expect("parse merged token JSON");
    if workspace_mode {
        let watched = match (&artifact, &theme_lock) {
            (Some(artifact), Some(lock)) => {
                let package_root = root
                    .canonicalize()
                    .expect("canonicalize package design root");
                let artifact_root = artifact
                    .canonicalize()
                    .expect("canonicalize consumer Theme artifact root");
                tekes_ui_design_export::theme_system::verify_consumer_theme_system(
                    &package_root,
                    &artifact_root,
                    lock,
                    &document,
                )
                .map_err(|error| vec![error])
            }
            (Some(artifact), None) => {
                println!(
                    "cargo:rerun-if-changed={}",
                    root.join("design/tokens.json").display()
                );
                tekes_ui_design_export::verify_consumer_inputs(&root, artifact, &document)
            }
            (None, None) => {
                let asset_tokens = assets_dir.join("tokens.json");
                println!("cargo:rerun-if-changed={}", asset_tokens.display());
                assert_eq!(
                    fs::read(&asset_tokens).expect("packaged assets/tokens.json missing"),
                    text.as_bytes(),
                    "crates/tekes-ui-tokens/assets/tokens.json is out of sync with design/tokens.json; run scripts/merge-design-system.py"
                );
                tekes_ui_design_export::verify_inputs(&root, &document)
            }
            (None, Some(_)) => {
                unreachable!("Theme lock without consumer artifact was rejected above")
            }
        }
        .unwrap_or_else(|e| panic!("{}", e.join("\n")));
        for path in watched {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    } else {
        tekes_ui_design_export::validate_document(&document)
            .unwrap_or_else(|e| panic!("invalid packaged tokens.json:\n{}", e.join("\n")));
    }
    let theme_artifact_root = match (&artifact, &theme_lock) {
        (Some(artifact), Some(_)) => artifact
            .canonicalize()
            .expect("canonical Theme artifact root"),
        _ => root.clone(),
    };
    if workspace_mode && theme_lock.is_none() {
        for path in tekes_ui_design_export::theme_system::verify_theme_system(&root, &document)
            .unwrap_or_else(|error| panic!("invalid Theme system: {error}"))
        {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    let theme_manifest = if workspace_mode {
        theme_artifact_root.join("design/theme/generated/instances.json")
    } else {
        assets_dir.join("instances.json")
    };
    let surface_rules_path = if workspace_mode {
        theme_artifact_root.join("design/theme/generated/surface_rules.rs")
    } else {
        assets_dir.join("surface_rules.rs")
    };
    println!("cargo:rerun-if-changed={}", theme_manifest.display());
    println!("cargo:rerun-if-changed={}", surface_rules_path.display());
    if workspace_mode && theme_lock.is_none() {
        let package_theme_lock = theme_artifact_root.join("design/theme/artifact.lock.json");
        println!("cargo:rerun-if-changed={}", package_theme_lock.display());
        if artifact.is_none() {
            let asset_instances = assets_dir.join("instances.json");
            let asset_rules = assets_dir.join("surface_rules.rs");
            println!("cargo:rerun-if-changed={}", asset_instances.display());
            println!("cargo:rerun-if-changed={}", asset_rules.display());
            assert_eq!(
                fs::read(&asset_instances).expect("packaged assets/instances.json missing"),
                fs::read(&theme_manifest)
                    .expect("workspace design/theme/generated/instances.json missing"),
                "crates/tekes-ui-tokens/assets/instances.json is out of sync; run scripts/generate-theme-system.py --write-artifact-lock"
            );
            assert_eq!(
                fs::read(&asset_rules).expect("packaged assets/surface_rules.rs missing"),
                fs::read(&surface_rules_path)
                    .expect("workspace design/theme/generated/surface_rules.rs missing"),
                "crates/tekes-ui-tokens/assets/surface_rules.rs is out of sync; run scripts/generate-theme-system.py --write-artifact-lock"
            );
        }
    }
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let theme_instances: serde_json::Value =
        serde_json::from_slice(&fs::read(&theme_manifest).expect("read verified Theme instances"))
            .expect("parse verified Theme instances");
    let theme_source = compile_theme_definition_colors(&theme_instances);
    fs::write(out_dir.join("theme_definition_colors.rs"), theme_source)
        .expect("write compiled Theme-definition color table");
    let surface_rules =
        fs::read_to_string(&surface_rules_path).expect("read verified surface_rules.rs");
    fs::write(out_dir.join("surface_rules.rs"), surface_rules)
        .expect("write generated surface rules");
    // Parsing belongs to the independent compiler, so tokens have no runtime JSON dependency.
    let output = tekes_ui_design_export::compile_json(&text).unwrap_or_else(|errors| {
        panic!("invalid authoritative tokens.json:\n{}", errors.join("\n"))
    });
    fs::write(out_dir.join("tokens.rs"), output).expect("write generated tokens");
}

fn compile_theme_definition_colors(instances: &serde_json::Value) -> String {
    const MODES: [&str; 4] = ["light", "dark", "light-ic", "dark-ic"];
    let themes = instances["instances"]
        .as_array()
        .expect("verified Theme instance list");
    assert!(!themes.is_empty(), "verified Theme list must not be empty");
    let mut definitions = format!(
        "fn theme_definition_count() -> usize {{ {} }}\nfn theme_definition_index(id: &str) -> Option<usize> {{ Some(match id {{\n",
        themes.len()
    );
    let mut colors = String::from(
        "fn theme_definition_color(index: usize, role: &str, mode: usize) -> Option<crate::Rgba> { Some(match (index, role, mode) {\n",
    );
    for (index, theme) in themes.iter().enumerate() {
        let id = theme["id"].as_str().expect("verified Theme id");
        definitions.push_str(&format!("{id:?} => {index},\n"));
        let role_maps = theme["compiled_color_roles"]
            .as_object()
            .expect("verified complete compiled Theme color roles");
        assert_eq!(
            role_maps
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            MODES.into_iter().collect(),
            "Theme {id} must provide all four canonical modes"
        );
        let mut canonical_roles = None;
        for (mode_index, mode) in MODES.iter().enumerate() {
            let role_map = role_maps[*mode]
                .as_object()
                .expect("verified Theme mode color roles");
            let roles = role_map
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            if let Some(expected) = &canonical_roles {
                assert_eq!(&roles, expected, "Theme {id} role map differs by mode");
            } else {
                canonical_roles = Some(roles);
            }
            for (role, value) in role_map {
                let hex = value.as_str().expect("verified compiled role hex");
                assert!(
                    hex.len() == 7
                        && hex.starts_with('#')
                        && hex[1..].bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "Theme {id} role {role} must be opaque sRGB hex"
                );
                let rgb = [1, 3, 5].map(|offset| {
                    u8::from_str_radix(&hex[offset..offset + 2], 16)
                        .expect("validated hexadecimal channel")
                });
                colors.push_str(&format!(
                    "({index}, {role:?}, {mode_index}) => crate::Rgba::rgb8({}, {}, {}),\n",
                    rgb[0], rgb[1], rgb[2]
                ));
            }
        }
    }
    definitions.push_str("_ => return None, }) }\n");
    colors.push_str("_ => return None, }) }\n");
    definitions.push_str(&colors);
    definitions
}
