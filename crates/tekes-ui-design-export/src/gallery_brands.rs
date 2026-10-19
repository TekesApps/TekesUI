//! Structural and semantic checks for the generated Gallery brand palette.
use crate::Errors;
use serde_json::Value;
use std::collections::BTreeSet;

const THEMES: [&str; 4] = ["light", "dark", "light-ic", "dark-ic"];
const FIXED_INPUTS: [&str; 7] = [
    "design/source/base.tokens.json",
    "design/source/token-metadata.json",
    "design/brand/policy.json",
    "design/brand/gallery.json",
    "scripts/generate-gallery-brands.py",
    "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand.py",
    "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand_color.py",
];

/// Reject malformed generated brand data before the token crate compiles it.
/// The generator receipt is separately bound by the design projection receipt.
pub fn validate_gallery_brands(document: &Value) -> Result<(), Errors> {
    let mut errors = Vec::new();
    if document["format"] != "tekesui-gallery-brands/1" {
        errors.push("Gallery brand projection has an unsupported format".to_owned());
    }
    let inputs = document["inputs_sha256"].as_object();
    match inputs {
        Some(inputs) if !inputs.is_empty() => {
            for (path, digest) in inputs {
                if path.is_empty()
                    || !digest
                        .as_str()
                        .is_some_and(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
                {
                    errors.push(format!("invalid Gallery brand input digest: {path}"));
                }
            }
        }
        _ => errors.push("Gallery brand input digests are required".to_owned()),
    }

    let presets = match document["presets"].as_array() {
        Some(presets) if !presets.is_empty() => presets,
        _ => {
            errors.push("Gallery brand presets are required".to_owned());
            return Err(errors);
        }
    };
    let mut required_inputs = FIXED_INPUTS
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    let mut indexes = BTreeSet::new();
    for preset in presets {
        let id = preset["id"].as_str().filter(|id| !id.trim().is_empty());
        let Some(id) = id else {
            errors.push("Gallery brand preset id must be a nonempty string".to_owned());
            continue;
        };
        if !ids.insert(id) {
            errors.push(format!("duplicate Gallery brand id: {id}"));
        }
        match preset["runtime_index"].as_u64() {
            Some(index) if indexes.insert(index) => {}
            _ => errors.push(format!("invalid or duplicate runtime index for {id}")),
        }
        match preset["config"]
            .as_str()
            .filter(|path| !path.trim().is_empty())
        {
            Some(config) if required_inputs.insert(config.to_owned()) => {}
            Some(config) => errors.push(format!(
                "duplicate or reserved Gallery brand config path: {config}"
            )),
            None => errors.push(format!("{id}: brand config path is required")),
        }
        let tokens = match preset["color"]["tokens"].as_array() {
            Some(tokens) => tokens,
            None => {
                errors.push(format!("{id}: compiled color tokens are required"));
                continue;
            }
        };
        let mut by_name = std::collections::BTreeMap::new();
        for token in tokens {
            let name = token["name"].as_str().unwrap_or("");
            if name.is_empty() || by_name.insert(name, token).is_some() {
                errors.push(format!(
                    "{id}: missing or duplicate color token name {name:?}"
                ));
            }
        }
        for name in ["action-primary-ground", "action-primary-foreground"] {
            if !by_name.contains_key(name) {
                errors.push(format!("{id}: required semantic color is missing: {name}"));
            }
        }
        for theme in THEMES {
            let ground = resolve_color(
                &by_name,
                "action-primary-ground",
                theme,
                &mut BTreeSet::new(),
            );
            let foreground = resolve_color(
                &by_name,
                "action-primary-foreground",
                theme,
                &mut BTreeSet::new(),
            );
            match (ground, foreground) {
                (Some(ground), Some(foreground)) => {
                    if contrast(ground, foreground) < 4.5 {
                        errors.push(format!(
                            "{id}: primary action contrast below 4.5:1 in {theme}"
                        ));
                    }
                }
                _ => errors.push(format!("{id}: invalid primary action color in {theme}")),
            }
        }
    }
    if let Some(inputs) = inputs {
        let actual = inputs.keys().cloned().collect::<BTreeSet<_>>();
        if actual != required_inputs {
            for missing in required_inputs.difference(&actual) {
                errors.push(format!(
                    "missing required Gallery brand input receipt: {missing}"
                ));
            }
            for extra in actual.difference(&required_inputs) {
                errors.push(format!("unexpected Gallery brand input receipt: {extra}"));
            }
        }
    }
    if indexes.iter().copied().ne(0..presets.len() as u64) {
        errors.push("Gallery brand runtime indexes must be contiguous from zero".to_owned());
    }
    if !ids.contains(document["reference_brand"].as_str().unwrap_or("")) {
        errors.push("Gallery reference brand must identify a registered preset".to_owned());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn resolve_color<'a>(
    tokens: &std::collections::BTreeMap<&'a str, &'a Value>,
    name: &'a str,
    theme: &str,
    seen: &mut BTreeSet<&'a str>,
) -> Option<[u8; 3]> {
    if !seen.insert(name) {
        return None;
    }
    let value = &tokens.get(name)?["value"];
    let value = if let Some(themes) = value.as_object() {
        themes.get(theme)?
    } else {
        value
    };
    let value = value.as_str()?;
    if let Some(reference) = value.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return resolve_color(tokens, reference, theme, seen);
    }
    if value.len() != 7 || !value.starts_with('#') {
        return None;
    }
    Some([
        u8::from_str_radix(&value[1..3], 16).ok()?,
        u8::from_str_radix(&value[3..5], 16).ok()?,
        u8::from_str_radix(&value[5..7], 16).ok()?,
    ])
}

fn luminance(rgb: [u8; 3]) -> f64 {
    let channel = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
}

fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (light, dark) = if luminance(a) >= luminance(b) {
        (luminance(a), luminance(b))
    } else {
        (luminance(b), luminance(a))
    };
    (light + 0.05) / (dark + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        let mut inputs = serde_json::Map::new();
        for path in FIXED_INPUTS
            .into_iter()
            .chain(["design/brand/neutral.json"])
        {
            inputs.insert(path.to_owned(), json!("a".repeat(64)));
        }
        json!({"format":"tekesui-gallery-brands/1","reference_brand":"neutral",
        "inputs_sha256":inputs,
        "presets":[{"id":"neutral","runtime_index":0,"config":"design/brand/neutral.json","color":{"tokens":[
            {"name":"action-primary-ground","value":"{ground}"},
            {"name":"action-primary-foreground","value":"{foreground}"},
            {"name":"ground","value":{"light":"#696969","dark":"#ABABAB","light-ic":"#505050","dark-ic":"#C7C7C7"}},
            {"name":"foreground","value":{"light":"#FFFFFF","dark":"#000000","light-ic":"#FFFFFF","dark-ic":"#000000"}}
        ]}}]})
    }

    #[test]
    fn accepts_valid_gallery_brand_projection() {
        assert_eq!(validate_gallery_brands(&fixture()), Ok(()));
    }

    #[test]
    fn rejects_edited_or_incomplete_gallery_brand_projection() {
        let mut bad = fixture();
        bad["presets"][0]["color"]["tokens"][2]["value"]["light"] = json!("#FFFFFF");
        bad["presets"][0]["color"]["tokens"][2]["value"]
            .as_object_mut()
            .unwrap()
            .remove("dark-ic");
        assert!(validate_gallery_brands(&bad).is_err());
        let mut bad = fixture();
        bad["inputs_sha256"].as_object_mut().unwrap().clear();
        assert!(validate_gallery_brands(&bad).is_err());
        let mut bad = fixture();
        bad["inputs_sha256"]
            .as_object_mut()
            .unwrap()
            .remove("design/source/base.tokens.json");
        assert!(validate_gallery_brands(&bad).is_err());
        let mut bad = fixture();
        bad["presets"][0]["id"] = json!("");
        assert!(validate_gallery_brands(&bad).is_err());
        let mut bad = fixture();
        bad["presets"][0]["color"]["tokens"][3]["value"]["light"] = json!("#696969");
        assert!(validate_gallery_brands(&bad).is_err());
    }

    #[test]
    fn input_receipt_rejects_edited_brand_bytes_while_hashes_are_unchanged() {
        use std::{fs, path::PathBuf, time::SystemTime};

        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tokens: Value =
            serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap();
        let input_hashes = tokens["merge"]["inputs_sha256"].as_object().unwrap();
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!(
            "tekesui-brand-receipt-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_root).unwrap();
        for relative in input_hashes.keys() {
            let source = source_root.join(relative);
            let target = temp_root.join(relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(source, target).unwrap();
        }
        let brands_path = temp_root.join("design/gallery/brands.json");
        let mut brands: Value = serde_json::from_slice(&fs::read(&brands_path).unwrap()).unwrap();
        brands["presets"][0]["color"]["tokens"][0]["value"] = serde_json::json!("#FFFFFF");
        fs::write(&brands_path, serde_json::to_vec(&brands).unwrap()).unwrap();

        let result = crate::verify_inputs(&temp_root, &tokens);
        fs::remove_dir_all(&temp_root).unwrap();
        assert!(
            result.is_err(),
            "changed generated brand bytes must fail the immutable projection receipt"
        );
    }
}
