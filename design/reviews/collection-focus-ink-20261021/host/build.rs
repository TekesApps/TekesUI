use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../../..");
    let source =
        root.join("crates/tekes-ui-gallery/src/gallery/workbench/extended/collections_adapter.rs");
    println!("cargo:rerun-if-changed={}", source.display());
    let text = fs::read_to_string(source).unwrap();
    let start = text.find("pub(super) fn focus_changed(").unwrap();
    let opening = start + text[start..].find('{').unwrap();
    let mut depth = 0;
    let mut end = None;
    for (i, c) in text[opening..].char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                end = Some(opening + i + 1);
                break;
            }
        }
    }
    // Byte-for-byte production function; only fixture context is supplied by tests.
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("gallery_focus.rs"),
        &text[start..end.unwrap()],
    )
    .unwrap();
}
