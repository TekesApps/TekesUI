use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
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
    let json = artifact
        .as_ref()
        .unwrap_or(&root)
        .join("design/tokens.json");
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
        (None, None) => tekes_ui_design_export::verify_inputs(&root, &document),
        (None, Some(_)) => unreachable!("Theme lock without consumer artifact was rejected above"),
    }
    .unwrap_or_else(|e| panic!("{}", e.join("\n")));
    for path in watched {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let theme_artifact_root = match (&artifact, &theme_lock) {
        (Some(artifact), Some(_)) => artifact
            .canonicalize()
            .expect("canonical Theme artifact root"),
        _ => root.clone(),
    };
    if theme_lock.is_none() {
        for path in tekes_ui_design_export::theme_system::verify_theme_system(&root, &document)
            .unwrap_or_else(|error| panic!("invalid Theme system: {error}"))
        {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    let theme_manifest = theme_artifact_root.join("design/theme/generated/instances.json");
    let package_theme_lock = theme_artifact_root.join("design/theme/artifact.lock.json");
    println!("cargo:rerun-if-changed={}", theme_manifest.display());
    if theme_lock.is_none() {
        println!("cargo:rerun-if-changed={}", package_theme_lock.display());
    }
    let theme_instances: serde_json::Value =
        serde_json::from_slice(&fs::read(&theme_manifest).expect("read verified Theme instances"))
            .expect("parse verified Theme instances");
    let theme_source = compile_theme_definition_colors(&theme_instances);
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("theme_definition_colors.rs"),
        theme_source,
    )
    .expect("write compiled Theme-definition color table");
    // Parsing belongs to the independent compiler, so tokens have no runtime JSON dependency.
    let output = tekes_ui_design_export::compile_json(&text).unwrap_or_else(|errors| {
        panic!("invalid authoritative tokens.json:\n{}", errors.join("\n"))
    });
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("tokens.rs"),
        output,
    )
    .expect("write generated tokens");
    let mut brands = String::from(
        "fn brand_color(brand: usize, role: &str, theme: usize) -> Option<crate::Rgba> { Some(match (brand, role, theme) {\n",
    );
    let mut metadata =
        String::from("pub const GALLERY_BRAND_PRESETS: &[GalleryBrandPreset] = &[\n");
    fn resolve(
        tokens: &[serde_json::Value],
        name: &str,
        theme: &str,
        depth: usize,
    ) -> Option<String> {
        if depth > 24 {
            return None;
        }
        let item = tokens.iter().find(|v| v["name"].as_str() == Some(name))?;
        let value = &item["value"];
        let value = if value.is_object() {
            &value[theme]
        } else {
            value
        };
        let text = value.as_str()?;
        if text.starts_with('{') {
            resolve(tokens, text.trim_matches(['{', '}']), theme, depth + 1)
        } else {
            Some(text.into())
        }
    }
    let path = root.join("design/gallery/brands.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let gallery: serde_json::Value =
        serde_json::from_slice(&fs::read(path).unwrap()).expect("Gallery brand projection");
    tekes_ui_design_export::validate_gallery_brands(&gallery).unwrap_or_else(|errors| {
        panic!("invalid Gallery brand projection:\n{}", errors.join("\n"))
    });
    use sha2::{Digest, Sha256};
    for (path, expected) in gallery["inputs_sha256"].as_object().unwrap() {
        let path = root.join(path);
        println!("cargo:rerun-if-changed={}", path.display());
        let actual = Sha256::digest(fs::read(path).unwrap())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            Some(actual.as_str()),
            expected.as_str(),
            "Stale Gallery brands: run scripts/generate-gallery-brands.py"
        );
    }
    for v in gallery["presets"].as_array().unwrap() {
        let brand = v["runtime_index"].as_u64().expect("runtime index") as usize;
        let str_value = |key: &str| v[key].as_str().expect("brand metadata string");
        let recommended = v["recommended_for_direction"]
            .as_array()
            .expect("brand recommendations")
            .iter()
            .map(|value| format!("{:?}", value.as_str().expect("direction id")))
            .collect::<Vec<_>>()
            .join(", ");
        let seed = v["seed_srgb"].as_str().expect("compiled seed");
        metadata.push_str(&format!(
            "GalleryBrandPreset {{ id: {:?}, name: {:?}, palette_index: {brand}, seed_srgb: {:?}, category: {:?}, color_family: {:?}, recommended_for_direction: &[{recommended}], description: {:?} }},\n",
            str_value("id"), str_value("name"), seed,
            str_value("category"), str_value("color_family"), str_value("description")
        ));
        let colors = v["color"]["tokens"]
            .as_array()
            .expect("compiled semantic colors");
        for token in colors {
            let role = token["name"].as_str().unwrap();
            for (i, theme) in ["light", "dark", "light-ic", "dark-ic"].iter().enumerate() {
                if let Some(hex) = resolve(colors, role, theme, 0)
                    && hex.len() == 7
                    && hex.starts_with('#')
                {
                    let rgb = [1, 3, 5].map(|n| u8::from_str_radix(&hex[n..n + 2], 16).unwrap());
                    brands.push_str(&format!(
                        "({brand},{role:?},{i})=>crate::Rgba::rgb8({},{},{}),\n",
                        rgb[0], rgb[1], rgb[2]
                    ));
                }
            }
        }
    }
    brands.push_str("_=>return None, }) }\n");
    metadata.push_str("];\n");
    metadata.push_str("pub const GALLERY_BRAND_DIRECTIONS: &[GalleryBrandDirection] = &[\n");
    for d in gallery["directions"].as_array().expect("brand directions") {
        metadata.push_str(&format!(
            "GalleryBrandDirection {{ id: {:?}, name: {:?}, description: {:?} }},\n",
            d["id"].as_str().expect("direction id"),
            d["name"].as_str().expect("direction name"),
            d["description"].as_str().expect("direction description")
        ));
    }
    metadata.push_str("];\n");
    brands.push_str(&metadata);
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("brand_colors.rs"),
        brands,
    )
    .unwrap();
}

fn compile_theme_definition_colors(instances: &serde_json::Value) -> String {
    const MODES: [&str; 4] = ["light", "dark", "light-ic", "dark-ic"];
    let themes = instances["instances"]
        .as_array()
        .expect("verified Theme instance list");
    assert!(!themes.is_empty(), "verified Theme list must not be empty");
    let mut definitions =
        String::from("fn theme_definition_index(id: &str) -> Option<usize> { Some(match id {\n");
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
