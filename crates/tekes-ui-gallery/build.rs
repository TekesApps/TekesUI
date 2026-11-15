use std::{env, fs, path::PathBuf};
fn main() {
    let path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../design/gallery/workbench.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let registry: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let entries = registry["entries"].as_array().unwrap();
    let legacy = [
        "Primitives",
        "Tokens",
        "StaticText",
        "Foundations",
        "Actions",
        "Forms",
        "Navigation",
        "Overlays",
        "Feedback",
        "Documents",
        "Data",
        "TextEditing",
        "Icons",
        "Text",
        "ScrollArea",
        "Motion",
    ];
    let mut pages: Vec<String> = legacy.iter().map(|p| format!("Page::{p}")).collect();
    pages.extend((0..entries.len()).map(|i| format!("Page::Workbench({i})")));
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("workbench_pages.rs"),
        format!("[{}]", pages.join(",")),
    )
    .unwrap();
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("workbench_count.rs"),
        (16 + entries.len()).to_string(),
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
}
