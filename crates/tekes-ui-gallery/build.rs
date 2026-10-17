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
}
