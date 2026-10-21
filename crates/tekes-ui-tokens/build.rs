use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    println!("cargo:rerun-if-env-changed=TEKESUI_DESIGN_ARTIFACT");
    let artifact = env::var_os("TEKESUI_DESIGN_ARTIFACT").map(PathBuf::from);
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
    let watched = match &artifact {
        Some(artifact) => {
            println!(
                "cargo:rerun-if-changed={}",
                root.join("design/tokens.json").display()
            );
            tekes_ui_design_export::verify_consumer_inputs(&root, artifact, &document)
        }
        None => tekes_ui_design_export::verify_inputs(&root, &document),
    }
    .unwrap_or_else(|e| panic!("{}", e.join("\n")));
    for path in watched {
        println!("cargo:rerun-if-changed={}", path.display());
    }
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
