use std::{env, fs, path::PathBuf};
fn main() {
    let path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../design/behavior/scroll.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let data: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
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
