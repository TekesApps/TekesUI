//! Input/output receipt validation for generated compatibility JSON.
use crate::Errors;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path},
};
fn hash(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn semantic(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Null => out.push(b'n'),
        Value::Bool(b) => out.push(if *b { b't' } else { b'f' }),
        Value::Number(n) => {
            out.push(b'd');
            out.extend_from_slice(&n.as_f64().unwrap().to_bits().to_be_bytes());
        }
        Value::String(s) => {
            out.push(b's');
            out.extend_from_slice(&(s.len() as u64).to_be_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        Value::Array(a) => {
            out.push(b'a');
            out.extend_from_slice(&(a.len() as u64).to_be_bytes());
            for v in a {
                semantic(v, out);
            }
        }
        Value::Object(m) => {
            out.push(b'o');
            out.extend_from_slice(&(m.len() as u64).to_be_bytes());
            let mut pairs: Vec<_> = m.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            for (k, v) in pairs {
                semantic(&Value::String(k.clone()), out);
                semantic(v, out);
            }
        }
    }
}
/// Verify canonical author inputs and the projection itself. Returns watched paths.
/// A stale input never silently falls back to a previously generated bundle.
pub fn verify_inputs(root: &Path, d: &Value) -> Result<Vec<std::path::PathBuf>, Errors> {
    let merge = &d["merge"];
    if merge["format"] != "tekesui.projection/1" {
        return Err(vec!["missing generated projection receipt".into()]);
    }
    let inputs = merge["inputs_sha256"]
        .as_object()
        .filter(|m| !m.is_empty())
        .ok_or_else(|| vec!["merge input hashes required".into()])?;
    let mut errors = vec![];
    let mut paths = vec![];
    for (relative, expected) in inputs {
        let p = Path::new(relative);
        if p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
            errors.push(format!("unsafe design input path {relative}"));
            continue;
        }
        let p = root.join(p);
        paths.push(p.clone());
        match fs::read(&p) {
            Ok(bytes) if expected.as_str() == Some(&hash(&bytes)) => {}
            _ => errors.push(format!(
                "stale/missing design input {relative}; run scripts/merge-design-system.py"
            )),
        }
    }
    for required in [
        "design/source/base.tokens.json",
        "design/source/legacy.tokens.json",
        "design/source/rust-mapping.json",
        "design/source/motion.json",
        "design/source/token-metadata.json",
        "design/source/layout.json",
        "design/source/interaction-contracts.json",
        "design/brand/config.json",
        "design/brand/policy.json",
        "design/brand/gallery.json",
        "scripts/merge-design-system.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand_color.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/validate.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/token-metadata.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/tokens.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/brand.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/policy.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/quality/contrast-pairs.json",
    ] {
        if !inputs.contains_key(required) {
            errors.push(format!("missing required design input receipt {required}"));
        }
    }
    let mut body = d.clone();
    body.as_object_mut()
        .ok_or_else(|| vec!["projection must be object".into()])?
        .remove("merge");
    let mut bytes = vec![];
    semantic(&body, &mut bytes);
    if merge["projection_sha256"].as_str() != Some(&hash(&bytes)) {
        errors.push("modified generated projection; edit source JSON then run scripts/merge-design-system.py".into());
    }
    if errors.is_empty() {
        Ok(paths)
    } else {
        Err(errors)
    }
}
