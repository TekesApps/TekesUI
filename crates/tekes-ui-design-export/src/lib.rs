//! Compile the generated compatibility projection, design/tokens.json, into Rust.
//! Author inputs live in design/source and design/brand; stale projections are rejected.
//! The project format preserves logical pt, encoded sRGB and explicit theme sets;
//! it borrows DTCG syntax without claiming complete DTCG compatibility.

use serde_json::{Map, Value, json};
use std::{collections::BTreeSet, fmt::Write};
mod effects;
mod freshness;
mod gallery_brands;
mod proposal;
pub mod theme_system;
mod validator;
pub use freshness::{verify_consumer_inputs, verify_inputs};
pub use gallery_brands::validate_gallery_brands;
pub use validator::canonical_json;

pub const FORMAT: &str = "tekesui.tokens/1";
const FAMILIES: &[(&str, &str)] = &[
    ("spacing", "Spacing"),
    ("radius", "CornerRadius"),
    ("stroke", "Stroke"),
    ("size", "ControlSize"),
    ("motion", "Motion"),
    ("text", "TextRole"),
    ("opacity", "Alpha"),
    ("color", "ColorRole"),
];
const THEMES: &[(&str, &str)] = &[
    ("light", "LIGHT"),
    ("dark", "DARK"),
    ("light-ic", "LIGHT_INCREASED"),
    ("dark-ic", "DARK_INCREASED"),
];
type Errors = Vec<String>;

fn object(properties: Value) -> Value {
    let required: Vec<_> = properties
        .as_object()
        .expect("schema properties")
        .keys()
        .cloned()
        .collect();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn reference(name: &str) -> Value {
    json!({"$ref":format!("#/$defs/{name}")})
}
fn scalar(kind: &str, value: Value) -> Value {
    let symbolic = json!({"oneOf":[value,object(json!({"$ref":{"type":"string"}})),object(json!({"$base":{"type":"string"},"$alpha":{"type":"string"}}))]});
    object(json!({"$type":{"const":kind},"$value":symbolic,"$description":{"type":"string"}}))
}
fn entries<'a>(d: &'a Value, family: &str) -> &'a [Value] {
    d["api"][family]["variants"]
        .as_array()
        .expect("validated registry")
}
fn family_schema(d: &Value, family: &str, definition: &str) -> Value {
    object(Value::Object(
        entries(d, family)
            .iter()
            .map(|entry| (entry["id"].as_str().unwrap().into(), reference(definition)))
            .collect(),
    ))
}
fn validate_api(d: &Value) -> Result<(), Errors> {
    let Some(api) = d["api"].as_object() else {
        return Err(vec!["/api: required API registry object".into()]);
    };
    let expected: BTreeSet<_> = FAMILIES.iter().map(|(family, _)| *family).collect();
    if api.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err(vec![
            "/api: missing or unknown family (new families require compiler support)".into(),
        ]);
    }
    for (family, name) in FAMILIES {
        let group = &api[*family];
        if group.as_object().map(|o| o.len()) != Some(2) || group["enum"] != *name {
            return Err(vec![format!(
                "/api/{family}: expected enum {name} and variants only"
            )]);
        }
        let Some(variants) = group["variants"].as_array().filter(|v| !v.is_empty()) else {
            return Err(vec![format!(
                "/api/{family}/variants: nonempty ordered array required"
            )]);
        };
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for entry in variants {
            let id = entry["id"].as_str().unwrap_or("");
            let name = entry["variant"].as_str().unwrap_or("");
            let valid_id = !id.is_empty()
                && id
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
                && !id.starts_with('-')
                && !id.ends_with('-');
            let valid_name = name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                && name.bytes().all(|c| c.is_ascii_alphanumeric())
                && !["Self", "ALL", "value", "resolve", "style", "seconds"].contains(&name);
            if entry.as_object().map(|o| o.len()) != Some(2)
                || !valid_id
                || !valid_name
                || !ids.insert(id)
                || !names.insert(name)
            {
                return Err(vec![format!(
                    "/api/{family}: invalid/duplicate ID or Rust variant"
                )]);
            }
        }
    }
    Ok(())
}

/// Derived schema contains IDs from the authoritative JSON registry, never values.
pub fn export_schema(d: &Value) -> Result<Value, Errors> {
    validate_api(d)?;
    let normalized = json!({"type":"number","minimum":0,"maximum":1});
    let dimension = scalar(
        "dimension",
        object(json!({"value":{"type":"number","minimum":0},"unit":{"const":"pt"}})),
    );
    let duration = scalar(
        "duration",
        object(json!({"value":{"type":"number","minimum":0},"unit":{"const":"s"}})),
    );
    let color = scalar(
        "color",
        object(
            json!({"colorSpace":{"const":"srgb"},"components":{"type":"array","items":normalized,"minItems":3,"maxItems":3},"alpha":normalized}),
        ),
    );
    let set = object(
        json!({"color":family_schema(d,"color","color"),"opacity":family_schema(d,"opacity","opacity")}),
    );
    let sets: Map<String, Value> = THEMES
        .iter()
        .map(|(key, _)| (key.to_string(), set.clone()))
        .collect();
    let text=object(Value::Object(entries(d,"text").iter().map(|entry|(entry["id"].as_str().unwrap().into(),object(json!({"font-size":reference("dimension"),"line-height":reference("dimension"),"weight":reference("fontWeight")})))).collect()));
    let mut schema = object(json!({
        "$schema":{"const":"./tokens.schema.json"},"format":{"const":FORMAT},"status":{"const":"generated-projection"},
        "api":{"const":d["api"]},
        "source":object(json!({"authority":{"const":"design/source plus design/brand JSON inputs"},"expectedBaselineCommit":{"type":"string","minLength":40,"maxLength":40},"baselineManifest":{"const":"design/baseline/manifest.json"},"generator":{"type":"string"},"policy":{"type":"string"},"compatibility":{"type":"string"},"historicalState":{"type":"string"}})),
        "sets":object(Value::Object(sets)),
        "proposal":{"type":"object"},"merge":{"type":"object"},
        "tokens":object(json!({"spacing":family_schema(d,"spacing","dimension"),"radius":family_schema(d,"radius","dimension"),"stroke":family_schema(d,"stroke","dimension"),"size":family_schema(d,"size","dimension"),"motion":object(json!({"standard":family_schema(d,"motion","duration"),"reduced-motion":family_schema(d,"motion","duration")})),"text":text,"opacity":object(json!({"text-selection-increased":reference("opacity")}))}))
    }));
    let map = schema.as_object_mut().unwrap();
    map.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    map.insert(
        "title".into(),
        json!("TekesUI authoritative token format v1 (project schema)"),
    );
    map.insert("$comment".into(),json!("Generated shape; JSON IDs/values are authoritative. $ref token expressions reference complete leaves; $base plus $alpha replaces alpha. Logical dimensions remain pt. Not full DTCG compatibility."));
    map.insert("$defs".into(),json!({"dimension":dimension,"duration":duration,"color":color,"opacity":scalar("number",normalized),"fontWeight":scalar("fontWeight",json!({"type":"integer","minimum":1,"maximum":1000}))}));
    Ok(schema)
}

fn check_reference_scope(source: &str, target: &str) -> Result<(), String> {
    let src: Vec<_> = source.split('/').collect();
    let dst: Vec<_> = target.split('/').collect();
    if src.get(1) != Some(&"sets") && dst.get(1) == Some(&"sets") {
        return Err(format!(
            "{source}: global token cannot depend on theme-specific {target}"
        ));
    }
    if src.get(1) == Some(&"sets") && dst.get(1) == Some(&"sets") && src.get(2) != dst.get(2) {
        let same_appearance_standard = src.get(2).and_then(|x| x.strip_suffix("-ic"))
            == dst.get(2).copied()
            && src.get(3..) == dst.get(3..);
        if !same_appearance_standard {
            return Err(format!(
                "{source}: cross-theme reference to {target} would change theme semantics"
            ));
        }
    }
    Ok(())
}

fn resolve_leaf(d: &Value, path: &str, stack: &mut BTreeSet<String>) -> Result<Value, String> {
    if !stack.insert(path.into()) {
        return Err(format!("{path}: cyclic token reference"));
    }
    let result = (|| {
        let leaf = d
            .pointer(path)
            .ok_or_else(|| format!("{path}: missing referenced token"))?;
        let value = leaf
            .get("$value")
            .ok_or_else(|| format!("{path}: reference must target a token leaf"))?;
        if let Some(target) = value["$ref"].as_str() {
            if value.as_object().unwrap().len() != 1 {
                return Err(format!("{path}: reference has extra keys"));
            }
            check_reference_scope(path, target)?;
            let source = d
                .pointer(target)
                .ok_or_else(|| format!("{path}: missing {target}"))?;
            if source["$type"] != leaf["$type"] {
                return Err(format!("{path}: reference type mismatch"));
            }
            return resolve_leaf(d, target, stack);
        }
        if let Some(base) = value["$base"].as_str() {
            if value.as_object().unwrap().len() != 2 || leaf["$type"] != "color" {
                return Err(format!("{path}: invalid color derivation"));
            }
            check_reference_scope(path, base)?;
            let alpha = value["$alpha"]
                .as_str()
                .ok_or_else(|| format!("{path}: alpha token reference required"))?;
            check_reference_scope(path, alpha)?;
            if d.pointer(base).map(|x| &x["$type"]) != Some(&json!("color"))
                || d.pointer(alpha).map(|x| &x["$type"]) != Some(&json!("number"))
            {
                return Err(format!("{path}: base/alpha type mismatch"));
            }
            let mut result = resolve_leaf(d, base, stack)?;
            result["alpha"] = resolve_leaf(d, alpha, stack)?;
            return Ok(result);
        }
        Ok(value.clone())
    })();
    stack.remove(path);
    result
}
fn resolved(d: &Value) -> Result<Value, Errors> {
    fn walk(d: &Value, v: &mut Value, path: &str) -> Result<(), Errors> {
        if v.get("$value").is_some() {
            v["$value"] = resolve_leaf(d, path, &mut BTreeSet::new()).map_err(|e| vec![e])?;
        } else if let Some(map) = v.as_object_mut() {
            for (key, item) in map {
                walk(d, item, &format!("{path}/{key}"))?;
            }
        }
        Ok(())
    }
    let mut result = d.clone();
    walk(d, &mut result, "")?;
    Ok(result)
}

/// Strict structural + resolved semantic validation. Unknown values are never ignored.
pub fn validate_document(d: &Value) -> Result<(), Errors> {
    let schema = export_schema(d)?;
    let mut errors = vec![];
    proposal::validate(&d["proposal"])?;
    validator::validate(&schema, &schema, d, "", &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    let normalized = resolved(d)?;
    validator::validate(&schema, &schema, &normalized, "", &mut errors);
    for channels in numeric_channels_unchecked(&normalized) {
        if !f32::from_bits(channels.1).is_finite() {
            errors.push(format!("{}: outside finite f32 range", channels.0));
        }
    }
    for entry in entries(d, "opacity") {
        let id = entry["id"].as_str().unwrap();
        for (light, dark) in [("light", "dark"), ("light-ic", "dark-ic")] {
            if normalized["sets"][light]["opacity"][id]["$value"]
                != normalized["sets"][dark]["opacity"][id]["$value"]
            {
                errors.push(format!(
                    "/sets/{dark}/opacity/{id}: Alpha API is contrast-only; light/dark must agree"
                ));
            }
        }
    }
    for entry in entries(d, "text") {
        let id = entry["id"].as_str().unwrap();
        let t = &normalized["tokens"]["text"][id];
        if t["font-size"]["$value"]["value"]
            .as_f64()
            .unwrap_or_default()
            > t["line-height"]["$value"]["value"]
                .as_f64()
                .unwrap_or_default()
        {
            errors.push(format!(
                "/tokens/text/{id}: line height smaller than font size"
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
fn numeric_channels_unchecked(d: &Value) -> Vec<(String, u32)> {
    fn walk(v: &Value, path: &str, out: &mut Vec<(String, u32)>) {
        match v {
            Value::Object(map) => {
                for (key, item) in map {
                    walk(item, &format!("{path}/{key}"), out)
                }
            }
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    walk(item, &format!("{path}/{i}"), out)
                }
            }
            Value::Number(n) if path.contains("/$value") => {
                out.push((path.into(), (n.as_f64().unwrap() as f32).to_bits()))
            }
            _ => {}
        }
    }
    let mut values = vec![];
    walk(d, "", &mut values);
    values.sort_by(|a, b| a.0.cmp(&b.0));
    values
}
/// Resolved f32 channels, including font weights represented exactly as f32, for migration evidence.
pub fn numeric_channels(d: &Value) -> Result<Vec<(String, u32)>, Errors> {
    validate_document(d)?;
    Ok(numeric_channels_unchecked(&resolved(d)?))
}
fn bits(v: &Value) -> String {
    format!(
        "f32::from_bits(0x{:08x})",
        (v.as_f64().expect("validated f32") as f32).to_bits()
    )
}

/// Deterministic Rust API compiled directly from JSON IDs, variants and values.
pub fn generate_rust(d: &Value) -> Result<String, Errors> {
    validate_document(d)?;
    let n = resolved(d)?;
    let mut out = String::from("// @generated from design/tokens.json. Do not hand-edit.\n");
    let mut runtime = vec![];
    for (family, name) in FAMILIES {
        let list = entries(d, family);
        writeln!(
            out,
            "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum {name} {{"
        )
        .unwrap();
        for e in list {
            writeln!(out, "    {},", e["variant"].as_str().unwrap()).unwrap();
        }
        writeln!(
            out,
            "}}\nimpl {name} {{\n    pub const ALL: [{name}; {}] = [",
            list.len()
        )
        .unwrap();
        for e in list {
            writeln!(out, "        Self::{},", e["variant"].as_str().unwrap()).unwrap();
        }
        out.push_str("    ];\n");
        match *family {
            "color" => {
                out.push_str("    pub fn resolve(self, theme: crate::Theme) -> crate::Rgba {\n        match (self, theme.appearance, theme.contrast) {\n");
                for (theme, _) in THEMES {
                    let appearance = if theme.starts_with("dark") {
                        "Dark"
                    } else {
                        "Light"
                    };
                    let contrast = if theme.ends_with("-ic") {
                        "Increased"
                    } else {
                        "Standard"
                    };
                    for e in list {
                        let id = e["id"].as_str().unwrap();
                        let variant = e["variant"].as_str().unwrap();
                        let p = format!("/sets/{theme}/color/{id}/$value");
                        let v = n.pointer(&p).unwrap();
                        writeln!(out,"            (Self::{variant}, crate::Appearance::{appearance}, crate::Contrast::{contrast}) => crate::Rgba::new({}, {}, {}, {}),",bits(&v["components"][0]),bits(&v["components"][1]),bits(&v["components"][2]),bits(&v["alpha"])).unwrap();
                        let theme_const = THEMES.iter().find(|(key, _)| key == theme).unwrap().1;
                        for (index, field) in [(0, "r"), (1, "g"), (2, "b")] {
                            runtime.push((format!("{p}/components/{index}"),format!("ColorRole::{variant}.resolve(crate::Theme::{theme_const}).{field}")));
                        }
                        runtime.push((
                            format!("{p}/alpha"),
                            format!("ColorRole::{variant}.resolve(crate::Theme::{theme_const}).a"),
                        ));
                    }
                }
                out.push_str("        }\n    }\n");
            }
            "text" => {
                out.push_str(
                    "    pub const fn style(self) -> crate::TextStyle {\n        match self {\n",
                );
                for e in list {
                    let id = e["id"].as_str().unwrap();
                    let variant = e["variant"].as_str().unwrap();
                    let t = &n["tokens"]["text"][id];
                    writeln!(out,"            Self::{variant} => crate::TextStyle {{ size: {}, line_height: {}, weight: {} }},",bits(&t["font-size"]["$value"]["value"]),bits(&t["line-height"]["$value"]["value"]),t["weight"]["$value"].as_f64().unwrap() as u16).unwrap();
                    for (key, field) in [("font-size", "size"), ("line-height", "line_height")] {
                        runtime.push((
                            format!("/tokens/text/{id}/{key}/$value/value"),
                            format!("TextRole::{variant}.style().{field}"),
                        ));
                    }
                    runtime.push((
                        format!("/tokens/text/{id}/weight/$value"),
                        format!("TextRole::{variant}.style().weight as f32"),
                    ));
                }
                out.push_str("        }\n    }\n");
            }
            "opacity" => {
                out.push_str("    pub const fn value(self) -> f32 { self.resolve(crate::Contrast::Standard) }\n    pub const fn resolve(self, contrast: crate::Contrast) -> f32 {\n        match (self, contrast) {\n");
                for (theme, _) in THEMES {
                    for e in list {
                        let id = e["id"].as_str().unwrap();
                        let variant = e["variant"].as_str().unwrap();
                        let p = format!("/sets/{theme}/opacity/{id}/$value");
                        let contrast = if theme.ends_with("-ic") {
                            "Increased"
                        } else {
                            "Standard"
                        };
                        if theme.starts_with("light") {
                            writeln!(
                                out,
                                "            (Self::{variant}, crate::Contrast::{contrast}) => {},",
                                bits(n.pointer(&p).unwrap())
                            )
                            .unwrap();
                        } else {
                            let light = theme.replacen("dark", "light", 1);
                            if n.pointer(&p)
                                != n.pointer(&format!("/sets/{light}/opacity/{id}/$value"))
                            {
                                return Err(vec![format!(
                                    "{p}: Alpha API is contrast-only; light/dark opacity must agree"
                                )]);
                            }
                        }
                        runtime.push((
                            p,
                            format!("Alpha::{variant}.resolve(crate::Contrast::{contrast})"),
                        ));
                    }
                }
                out.push_str("        }\n    }\n");
            }
            _ => {
                let method = if *family == "motion" {
                    "seconds"
                } else {
                    "value"
                };
                writeln!(
                    out,
                    "    pub const fn {method}(self) -> f32 {{\n        match self {{"
                )
                .unwrap();
                for e in list {
                    let id = e["id"].as_str().unwrap();
                    let variant = e["variant"].as_str().unwrap();
                    let p = if *family == "motion" {
                        format!("/tokens/motion/standard/{id}/$value/value")
                    } else {
                        format!("/tokens/{family}/{id}/$value/value")
                    };
                    writeln!(
                        out,
                        "            Self::{variant} => {},",
                        bits(n.pointer(&p).unwrap())
                    )
                    .unwrap();
                    runtime.push((p, format!("{name}::{variant}.{method}()")));
                }
                out.push_str("        }\n    }\n");
                if *family == "motion" {
                    out.push_str("    pub const fn resolve(self, reduce_motion: bool) -> f32 {\n        if !reduce_motion { return self.seconds(); }\n        match self {\n");
                    for e in list {
                        let id = e["id"].as_str().unwrap();
                        let variant = e["variant"].as_str().unwrap();
                        let p = format!("/tokens/motion/reduced-motion/{id}/$value/value");
                        writeln!(
                            out,
                            "            Self::{variant} => {},",
                            bits(n.pointer(&p).unwrap())
                        )
                        .unwrap();
                        runtime.push((p, format!("Motion::{variant}.resolve(true)")));
                    }
                    out.push_str("        }\n    }\n");
                }
            }
        }
        out.push_str("}\n");
    }
    out.push_str(&format!(
        "pub const TEXT_SELECTION_INCREASED: f32 = {};\n",
        bits(&n["tokens"]["opacity"]["text-selection-increased"]["$value"])
    ));
    proposal::generate(&d["proposal"], &mut out)?;
    runtime.push((
        "/tokens/opacity/text-selection-increased/$value".into(),
        "TEXT_SELECTION_INCREASED".into(),
    ));
    out.push_str(
        "#[cfg(test)]\npub(crate) fn __runtime_channels() -> Vec<(String,u32)> {\n    vec![\n",
    );
    for (path, expression) in runtime {
        writeln!(out, "        ({path:?}.into(), ({expression}).to_bits()),").unwrap();
    }
    out.push_str("    ]\n}\n");
    Ok(out)
}
/// A review artifact is fresh only when every byte equals a current compilation.
/// Normal token builds do not consume this artifact; they generate their own OUT_DIR.
pub fn check_generated(d: &Value, actual: &str) -> Result<(), Errors> {
    if generate_rust(d)? == actual {
        Ok(())
    } else {
        Err(vec![
            "stale generated Rust; regenerate explicitly (no automatic update performed)".into(),
        ])
    }
}

/// Parse JSON for build-script freshness checks without a token runtime dependency.
pub fn parse_json(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// Parse and compile, shared by build.rs and the command-line tool.
pub fn compile_json(text: &str) -> Result<String, Errors> {
    let d = serde_json::from_str(text).map_err(|e| vec![format!("invalid JSON: {e}")])?;
    generate_rust(&d)
}

/// Fully resolve aliases into the original leaf shape for auditing; never writes the authority.
pub fn resolved_document(d: &Value) -> Result<Value, Errors> {
    validate_document(d)?;
    resolved(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Value {
        serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap()
    }
    #[test]
    fn historical_a8_channels_remain_preserved_as_evidence_only() {
        let text = include_str!("../../tekes-ui-tokens/tests/approved-a8-channels.txt");
        assert_eq!(text.lines().count(), 508);
        assert_eq!(numeric_channels(&document()).unwrap().len(), 508);
    }
    #[test]
    fn deterministic_and_freshness_checks_do_not_silently_update() {
        let d = document();
        let first = generate_rust(&d).unwrap();
        assert_eq!(first, generate_rust(&d).unwrap());
        check_generated(&d, &first).unwrap();
        assert!(check_generated(&d, &format!("{first}\n")).is_err());
        let mut changed = d;
        changed["tokens"]["spacing"]["regular"]["$value"]["value"] = json!(10);
        assert!(check_generated(&changed, &first).is_err());
        assert_ne!(first, generate_rust(&changed).unwrap());
    }
    #[test]
    fn rejects_unknown_missing_invalid_and_cyclic_input() {
        let d = document();
        let mut cases = vec![];
        let mut x = d.clone();
        x["sets"].as_object_mut().unwrap().remove("dark");
        cases.push(x);
        let mut x = d.clone();
        x["sets"]["unexpected"] = d["sets"]["light"].clone();
        cases.push(x);
        let mut x = d.clone();
        x["tokens"]["spacing"]
            .as_object_mut()
            .unwrap()
            .remove("regular");
        cases.push(x);
        let mut x = d.clone();
        x["tokens"]["spacing"]["unexpected"] = d["tokens"]["spacing"]["regular"].clone();
        cases.push(x);
        let mut x = d.clone();
        x["tokens"]["spacing"]["regular"]["$value"]["unit"] = json!("px");
        cases.push(x);
        let mut x = d.clone();
        x["tokens"]["spacing"]["regular"]["$value"]["value"] = json!(1e100);
        cases.push(x);
        let mut x = d.clone();
        x["sets"]["light"]["color"]["accent"]["$value"]["alpha"] = json!(1.1);
        cases.push(x);
        let mut x = d.clone();
        x["api"]["spacing"]["variants"][0]["variant"] = json!("Bad; injected");
        cases.push(x);
        let mut x = d.clone();
        x["api"]["spacing"]["variants"][0]["id"] = json!("regular");
        cases.push(x);
        let mut x = d.clone();
        x["sets"]["light"]["color"]["accent"]["$value"] =
            json!({"$ref":"/sets/light/color/accent"});
        cases.push(x);
        let mut x = d.clone();
        x["sets"]["light"]["color"]["accent"]["$value"] = json!({"$ref":"/tokens/spacing/regular"});
        cases.push(x);
        let mut x = d.clone();
        x["sets"]["light"]["color"]["accent"]["$value"] = json!({"$ref":"/missing"});
        cases.push(x);
        let mut x = d.clone();
        x["sets"]["light"]["color"]["accent"]["$value"] = json!({"$ref":"/sets/dark/color/accent"});
        cases.push(x);
        for (index, case) in cases.into_iter().enumerate() {
            assert!(generate_rust(&case).is_err(), "invalid case {index}");
        }
        assert!(compile_json("{").is_err());
    }
    #[test]
    fn new_semantic_id_requires_registry_and_generates_a_distinct_variant() {
        let mut d = document();
        d["api"]["spacing"]["variants"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"new-step","variant":"NewStep"}));
        assert!(generate_rust(&d).is_err());
        d["tokens"]["spacing"]["new-step"] = d["tokens"]["spacing"]["regular"].clone();
        let generated = generate_rust(&d).unwrap();
        assert!(generated.contains("Self::NewStep =>"));
    }
    #[test]
    fn validation_and_compilation_share_api_and_contrast_constraints() {
        let d = document();
        let mut unequal = d.clone();
        unequal["sets"]["dark"]["opacity"]["faint"]["$value"] = json!(0.5);
        assert!(validate_document(&unequal).is_err());
        assert!(generate_rust(&unequal).is_err());
        for reserved in ["ALL", "Self", "value", "resolve", "style", "seconds"] {
            let mut bad = d.clone();
            bad["api"]["spacing"]["variants"][0]["variant"] = json!(reserved);
            assert!(
                validate_document(&bad).is_err(),
                "reserved associated name {reserved}"
            );
            assert!(generate_rust(&bad).is_err());
        }
        let mut wrong_base = d.clone();
        wrong_base["sets"]["light"]["color"]["hover-wash"]["$value"]["$base"] =
            json!("/sets/dark/color/primary-text");
        assert!(validate_document(&wrong_base).is_err());
        let mut wrong_alpha = d;
        wrong_alpha["sets"]["light"]["color"]["hover-wash"]["$value"]["$alpha"] =
            json!("/sets/dark/opacity/faint");
        assert!(validate_document(&wrong_alpha).is_err());
    }

    #[test]
    fn metadata_is_not_part_of_numeric_values_or_generated_api() {
        let d = document();
        let mut edited = d.clone();
        edited["source"]["historicalState"] = json!("Historical approximations remain documented.");
        assert_eq!(numeric_channels(&d), numeric_channels(&edited));
        assert_eq!(generate_rust(&d), generate_rust(&edited));
    }
    #[test]
    fn changed_json_compiles_and_executes_changed_values_and_derivations() {
        // Execute the generated API independently of this crate's implementation.
        // This cannot pass just by changing the serializer or comparing source strings.
        let mut d = document();
        d["tokens"]["spacing"]["regular"]["$value"]["value"] = json!(10);
        d["sets"]["light"]["color"]["status-succeeded"]["$value"]["components"][0] = json!(0.25);
        d["sets"]["light"]["opacity"]["faint"]["$value"] = json!(0.125);
        d["sets"]["dark"]["opacity"]["faint"]["$value"] = json!(0.125);
        d["proposal"]["tokens"]["color"]["tokens"].as_array_mut().unwrap().push(json!({"name":"c-test-scrim","value":"{scrim}","usage":"test alias inherits render alpha"}));
        // The compatibility projection is generated; test its code emitter separately
        // from the canonical-input merge and freshness tests.
        for id in ["diff-added-marker", "diff-added-surface"] {
            d["sets"]["light"]["color"][id]["$value"]["components"][0] = json!(0.25);
        }
        d["sets"]["light"]["color"]["diff-added-surface"]["$value"]["alpha"] = json!(0.125);
        let generated = generate_rust(&d).unwrap();
        let root = std::env::temp_dir().join(format!("tekes-json-compiler-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let prefix = r#"
#[derive(Clone,Copy)] pub enum Appearance { Light,Dark }
#[derive(Clone,Copy)] pub enum Contrast { Standard,Increased }
#[derive(Clone,Copy)] pub struct Theme { pub appearance:Appearance,pub contrast:Contrast }
impl Theme { pub const LIGHT:Self=Self{appearance:Appearance::Light,contrast:Contrast::Standard}; }
pub struct Rgba{pub r:f32,pub g:f32,pub b:f32,pub a:f32}
impl Rgba {pub fn new(r:f32,g:f32,b:f32,a:f32)->Self{Self{r,g,b,a}}}
pub struct TextStyle{pub size:f32,pub line_height:f32,pub weight:u16}
"#;
        let assertions = r#"
fn main(){
assert_eq!(Spacing::Regular.value().to_bits(),10f32.to_bits());
assert_eq!(proposal::color("c-test-scrim",Theme::LIGHT).unwrap().a.to_bits(),1f32.to_bits());
assert_eq!(proposal::render_color("c-test-scrim",Theme::LIGHT).unwrap().a.to_bits(),0.45f32.to_bits());
assert_eq!(ColorRole::DiffAddedMarker.resolve(Theme::LIGHT).r.to_bits(),0.25f32.to_bits());
assert_eq!(ColorRole::DiffAddedSurface.resolve(Theme::LIGHT).r.to_bits(),0.25f32.to_bits());
assert_eq!(ColorRole::DiffAddedSurface.resolve(Theme::LIGHT).a.to_bits(),0.125f32.to_bits());
assert_eq!(Motion::Expansion.resolve(true).to_bits(),Motion::Instant.seconds().to_bits());
}
"#;
        let source = root.join("main.rs");
        let binary = root.join("compiled");
        std::fs::write(&source, format!("{prefix}\n{generated}\n{assertions}")).unwrap();
        let output =
            std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .arg("--edition=2024")
                .arg(&source)
                .arg("-o")
                .arg(&binary)
                .output()
                .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            std::process::Command::new(&binary)
                .status()
                .unwrap()
                .success()
        );
    }
    #[test]
    fn full_proposal_aliases_and_text_metadata_are_consumable() {
        let d = document();
        let proposal = &d["proposal"];
        let mut seen = BTreeSet::new();
        assert_eq!(
            super::proposal::resolve(
                &proposal["tokens"],
                "color",
                "c-list-selection-marker",
                "light",
                &mut seen
            )
            .unwrap(),
            super::proposal::resolve(
                &proposal["tokens"],
                "color",
                "accent-outline",
                "light",
                &mut BTreeSet::new()
            )
            .unwrap()
        );
        let source = generate_rust(&d).unwrap();
        assert!(source.contains("pub fn text_family"));
        assert!(source.contains("pub fn text_color"));
        let mut invalid = d.clone();
        invalid["proposal"]["tokens"]["color"]["tokens"][0]["value"] =
            json!("{missing-proposal-token}");
        assert!(validate_document(&invalid).is_err());
        let mut invalid = d.clone();
        invalid["proposal"]["tokens"]["type"]["groups"][0]["styles"][0]["family"] =
            json!("missing-family");
        assert!(validate_document(&invalid).is_err());
        let mut invalid = d.clone();
        invalid["proposal"]["motion"]["duration"]["geometry"] = json!("invalid");
        assert!(validate_document(&invalid).is_err());
    }

    #[test]
    fn alignment_policy_is_typed_and_generated_from_layout_json() {
        let mut d = document();
        let baseline = generate_rust(&d).unwrap();
        assert!(baseline.contains("pub const CHART_LABEL:AlignmentPolicy"));
        assert!(baseline.contains("pub const TEXT_ROW:AlignmentPolicy"));
        d["proposal"]["layout"]["alignment"]["presets"]["row"]["main_axis"] = json!("center");
        let changed = generate_rust(&d).unwrap();
        assert!(changed.contains(
            "AlignmentPreset::Row=>AlignmentPolicy{main_axis:MainAxisDistribution::Center"
        ));
        d["proposal"]["layout"]["alignment"]["presets"]["chart_label"]["cross_axis"] =
            json!("baseline-ish");
        assert!(validate_document(&d).is_err());
    }
    #[test]
    fn effect_bindings_are_validated_and_generated_from_authoritative_json() {
        let mut d = document();
        let baseline = generate_rust(&d).unwrap();
        assert!(baseline.contains("pub fn elevation("));
        assert!(baseline.contains("pub fn control_duration("));
        d["proposal"]["motion"]["control_duration"]["press_feedback"] = json!(0.08);
        d["proposal"]["layout"]["elevation"]["levels"]["modal"]["dark"][0]["alpha"] = json!(0.35);
        assert_ne!(generate_rust(&d).unwrap(), baseline);
        for path in [
            "/proposal/motion/control_transitions/button_press/easing",
            "/proposal/motion/control_transitions/switch_thumb/duration",
            "/proposal/layout/elevation/roles/menu",
        ] {
            let mut invalid = d.clone();
            *invalid.pointer_mut(path).unwrap() = json!("unknown");
            assert!(validate_document(&invalid).is_err(), "{path}");
        }
        let mut invalid = d;
        invalid["proposal"]["layout"]["elevation"]["levels"]["modal"]["dark"][0]["blur_radius"] =
            json!(-1);
        assert!(validate_document(&invalid).is_err());
    }
    #[test]
    fn proposal_text_ids_and_metrics_cannot_generate_invalid_rust() {
        let d = document();
        for names in [["self", "another"], ["ui-body2", "ui-body-2"]] {
            let mut bad = d.clone();
            bad["proposal"]["tokens"]["type"]["groups"][0]["styles"][0]["name"] = json!(names[0]);
            bad["proposal"]["tokens"]["type"]["groups"][0]["styles"][1]["name"] = json!(names[1]);
            assert!(validate_document(&bad).is_err());
        }
        let mut bad = d;
        bad["proposal"]["tokens"]["type"]["groups"][0]["styles"][0]["fontSize"] = json!(1e100);
        bad["proposal"]["tokens"]["type"]["groups"][0]["styles"][0]["lineHeight"] = json!(1e101);
        assert!(validate_document(&bad).is_err());
    }

    #[test]
    fn unknown_final_alpha_contract_is_rejected() {
        let mut d = document();
        d["proposal"]["render_opacity"]["scrim"]["default"] = json!("missing-alpha");
        assert!(validate_document(&d).is_err());
    }

    #[test]
    fn normal_build_receipt_rejects_omitted_inputs_and_edited_projection() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let d = document();
        verify_inputs(&root, &d).unwrap();
        for required in [
            "design/source/motion.json",
            "design/source/layout.json",
            "design/source/interaction-contracts.json",
            "design/brand/gallery.json",
        ] {
            let mut missing = d.clone();
            missing["merge"]["inputs_sha256"]
                .as_object_mut()
                .unwrap()
                .remove(required);
            assert!(verify_inputs(&root, &missing).is_err(), "{required}");
        }
        let mut edited = d;
        edited["tokens"]["spacing"]["regular"]["$value"]["value"] = json!(999);
        assert!(verify_inputs(&root, &edited).is_err());
    }

    #[test]
    fn checked_in_schema_is_fresh() {
        let schema = export_schema(&document()).unwrap();
        assert_eq!(
            canonical_json(&schema),
            include_str!("../../../design/tokens.schema.json")
        );
    }
}
