use std::{env, fs, path::PathBuf};
fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let path = manifest_dir.join("assets/scroll.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let text = fs::read_to_string(&path).expect("packaged assets/scroll.json missing");
    let workspace_scroll = manifest_dir.join("../../design/behavior/scroll.json");
    if workspace_scroll.is_file() {
        println!("cargo:rerun-if-changed={}", workspace_scroll.display());
        assert_eq!(
            fs::read(&workspace_scroll).unwrap(),
            text.as_bytes(),
            "crates/tekes-ui-controls/assets/scroll.json is out of sync with design/behavior/scroll.json"
        );
    }
    let data: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut generated = String::new();
    for key in [
        "stiffness",
        "release_amplitude",
        "release_period",
        "velocity_timeout_s",
        "settle_position_pt",
        "maximum_stretch_fraction",
        "nominal_hz",
    ] {
        let value = data[key].as_f64().expect("numeric scrolling behavior");
        assert!(value.is_finite() && value > 0.0);
        generated.push_str(&format!(
            "const {}: f64 = {:?};\n",
            key.to_uppercase(),
            value
        ));
    }
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("scroll_tokens.rs"),
        generated,
    )
    .unwrap();
}
