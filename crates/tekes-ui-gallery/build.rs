use std::{env, fs, path::PathBuf};
fn main() {
    let path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../design/gallery/workbench.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let registry: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let entries = registry["entries"].as_array().unwrap();
    let pages: Vec<String> = (0..entries.len())
        .map(|i| format!("Page::Workbench({i})"))
        .collect();
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("workbench_pages.rs"),
        format!("[{}]", pages.join(",")),
    )
    .unwrap();
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("workbench_count.rs"),
        entries.len().to_string(),
    )
    .unwrap();
    // Bundle the reviewed offline catalogue with the exact application source.
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let catalogue_path = root.join("design/gallery/component-documents.json");
    println!("cargo:rerun-if-changed={}", catalogue_path.display());
    let catalogue: serde_json::Value =
        serde_json::from_slice(&fs::read(&catalogue_path).expect("component documents catalogue"))
            .expect("component documents JSON");
    assert_eq!(catalogue["format"], "tekesui-component-documents/1");
    let mut ids = std::collections::BTreeSet::new();
    let mut documents = Vec::new();
    let mut paths = std::collections::BTreeSet::new();
    for (section, bundled) in [("documents", true), ("pending", false)] {
        for document in catalogue[section].as_array().expect("catalogue section") {
            let id = document["id"].as_str().expect("document id");
            assert!(ids.insert(id), "duplicate component document {id}");
            let routes: std::collections::BTreeSet<_> = entries
                .iter()
                .filter(|entry| entry["component"].as_str() == Some(id))
                .map(|entry| entry["slug"].as_str().unwrap())
                .collect();
            let declared: std::collections::BTreeSet<_> = document["related_routes"]
                .as_array()
                .expect("related routes")
                .iter()
                .map(|route| route.as_str().unwrap())
                .collect();
            assert!(
                !routes.is_empty() && routes == declared,
                "document route mismatch {id}"
            );
            if bundled {
                let path = document["path"].as_str().expect("document path");
                assert!(paths.insert(path), "duplicate document path {path}");
                let file = root.join(path);
                assert!(file.is_file(), "missing document {}", file.display());
                println!("cargo:rerun-if-changed={}", file.display());
                documents.push(format!("({id:?}, {path:?}, include_str!({file:?}))"));
            }
        }
    }
    let registered: std::collections::BTreeSet<_> = entries
        .iter()
        .filter_map(|entry| entry["component"].as_str())
        .collect();
    assert_eq!(
        ids, registered,
        "catalogue must account for every component, including pending manuals"
    );
    // Supplemental manuals are bundled links, never component registrations/routes.
    let mut supplemental = Vec::new();
    for document in catalogue["supplemental"]
        .as_array()
        .expect("supplemental manuals")
    {
        let id = document["id"].as_str().expect("supplemental id");
        assert!(
            id.starts_with("supplemental."),
            "supplemental identity {id}"
        );
        assert!(ids.insert(id), "duplicate document identity {id}");
        let path = document["path"].as_str().expect("supplemental path");
        assert!(paths.insert(path), "duplicate document path {path}");
        let file = root.join(path);
        assert!(
            file.is_file(),
            "missing supplemental manual {}",
            file.display()
        );
        println!("cargo:rerun-if-changed={}", file.display());
        supplemental.push(format!("({id:?}, {path:?}, include_str!({file:?}))"));
    }
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("supplemental_documents.rs"),
        format!("&[{}]", supplemental.join(",")),
    )
    .unwrap();
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("component_documents.rs"),
        format!("&[{}]", documents.join(",")),
    )
    .unwrap();
    compile_gallery_brands(&root);
}

fn compile_gallery_brands(root: &std::path::Path) {
    let mut brands = String::from(
        "fn brand_color(brand: usize, role: &str, theme: usize) -> Option<tekes_ui_tokens::Rgba> { Some(match (brand, role, theme) {\n",
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
                        "({brand},{role:?},{i})=>tekes_ui_tokens::Rgba::rgb8({},{},{}),\n",
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
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("gallery_brands.rs"),
        brands,
    )
    .unwrap();
}
