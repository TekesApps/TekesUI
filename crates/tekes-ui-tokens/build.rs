use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let json = root.join("design/tokens.json");
    println!("cargo:rerun-if-changed={}", json.display());
    let text = fs::read_to_string(&json)
        .expect("design/tokens.json must be available when building tokens");
    let document = tekes_ui_design_export::parse_json(&text).expect("parse merged token JSON");
    let watched = tekes_ui_design_export::verify_inputs(&root, &document)
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
}
