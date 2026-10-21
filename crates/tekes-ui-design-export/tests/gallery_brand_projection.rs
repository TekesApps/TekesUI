use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::PathBuf};
use tekes_ui_design_export::{validate_gallery_brands, verify_inputs};
const THEMES: [&str; 4] = ["light", "dark", "light-ic", "dark-ic"];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn brands() -> Value {
    serde_json::from_slice(&fs::read(root().join("design/gallery/brands.json")).unwrap()).unwrap()
}
fn color(d: &Value, role: &str, theme: &str) -> String {
    let tokens = d["presets"][0]["color"]["tokens"].as_array().unwrap();
    let mut name = role.to_owned();
    for _ in 0..64 {
        let token = tokens.iter().find(|t| t["name"] == name).unwrap();
        let v = &token["value"];
        let text = if v.is_object() { &v[theme] } else { v }.as_str().unwrap();
        if let Some(alias) = text.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            name = alias.to_owned();
        } else {
            return text.to_owned();
        }
    }
    panic!("cyclic baseline color")
}
fn primary_index(d: &Value, role: &str) -> usize {
    d["presets"][0]["color"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .position(|t| t["name"] == role)
        .unwrap()
}
#[test]
fn each_missing_primary_theme_is_rejected_without_changing_contrast_or_receipts() {
    let baseline = brands();
    validate_gallery_brands(&baseline).unwrap();
    let index = primary_index(&baseline, "action-primary-ground");
    for missing in THEMES {
        let mut d = baseline.clone();
        let values: BTreeMap<_, _> = THEMES
            .into_iter()
            .filter(|t| *t != missing)
            .map(|t| (t, color(&baseline, "action-primary-ground", t)))
            .collect();
        d["presets"][0]["color"]["tokens"][index]["value"] = json!(values);
        assert_eq!(d["inputs_sha256"], baseline["inputs_sha256"]);
        let errors = validate_gallery_brands(&d).unwrap_err();
        assert_eq!(errors.len(), 1, "{missing}: {errors:?}");
        assert!(errors[0].contains(&format!("invalid primary action color in {missing}")));
    }
}
#[test]
fn each_primary_same_color_is_rejected_without_removing_any_theme_or_receipt() {
    let baseline = brands();
    let index = primary_index(&baseline, "action-primary-foreground");
    for theme in THEMES {
        let mut d = baseline.clone();
        let values: BTreeMap<_, _> = THEMES
            .into_iter()
            .map(|t| {
                let role = if t == theme {
                    "action-primary-ground"
                } else {
                    "action-primary-foreground"
                };
                (t, color(&baseline, role, t))
            })
            .collect();
        d["presets"][0]["color"]["tokens"][index]["value"] = json!(values);
        assert_eq!(d["inputs_sha256"], baseline["inputs_sha256"]);
        let errors = validate_gallery_brands(&d).unwrap_err();
        assert_eq!(errors.len(), 1, "{theme}: {errors:?}");
        assert!(errors[0].contains(&format!("primary action contrast below 4.5:1 in {theme}")));
    }
}
#[test]
fn every_declared_required_input_is_independently_required() {
    let baseline = brands();
    for path in baseline["inputs_sha256"].as_object().unwrap().keys() {
        let mut d = baseline.clone();
        d["inputs_sha256"].as_object_mut().unwrap().remove(path);
        let errors = validate_gallery_brands(&d).unwrap_err();
        assert_eq!(errors.len(), 1, "{path}: {errors:?}");
        assert_eq!(
            errors[0],
            format!("missing required Gallery brand input receipt: {path}")
        );
    }
}
struct TempRoot(PathBuf);
impl Drop for TempRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn changing_only_generated_brand_bytes_is_rejected_by_public_consumption_validation() {
    let source = root();
    let tokens: Value =
        serde_json::from_slice(&fs::read(source.join("design/tokens.json")).unwrap()).unwrap();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_path = std::env::temp_dir().join(format!(
        "tekesui-brand-public-proof-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&temp_path).unwrap();
    let temp = TempRoot(temp_path);
    for path in tokens["merge"]["inputs_sha256"].as_object().unwrap().keys() {
        let target = temp.0.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(path), target).unwrap();
    }
    verify_inputs(&temp.0, &tokens).unwrap();
    let mut changed = brands();
    let index = primary_index(&changed, "action-primary-ground");
    changed["presets"][0]["color"]["tokens"][index]["value"] = json!("#FFFFFF");
    fs::write(
        temp.0.join("design/gallery/brands.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let errors = verify_inputs(&temp.0, &tokens).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("stale/missing design input design/gallery/brands.json"));
}
