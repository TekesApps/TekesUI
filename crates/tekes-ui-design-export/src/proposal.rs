//! Native access to the complete imported proposal vocabulary, not only old API mappings.
use crate::{Errors, THEMES, bits};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt::Write};

const GROUPS: &[&str] = &["color", "spacing", "radius", "stroke", "opacity", "size"];

pub(crate) fn resolve(
    tokens: &Value,
    group: &str,
    id: &str,
    theme: &str,
    seen: &mut BTreeSet<String>,
) -> Result<Value, String> {
    let key = format!("{group}/{id}");
    if !seen.insert(key.clone()) {
        return Err(format!("cyclic proposal alias {key}"));
    }
    let list = tokens[group]["tokens"]
        .as_array()
        .ok_or_else(|| format!("missing proposal group {group}"))?;
    let values: Vec<_> = list.iter().filter(|v| v["name"] == id).collect();
    if values.len() != 1 {
        return Err(format!("unknown/duplicate proposal ID {key}"));
    }
    let v = &values[0]["value"];
    let result = if let Some(alias) = v
        .as_str()
        .and_then(|s| s.strip_prefix('{'))
        .and_then(|s| s.strip_suffix('}'))
    {
        resolve(tokens, group, alias, theme, seen)?
    } else if v.is_object() {
        v.get(theme)
            .ok_or_else(|| format!("{key}: missing theme {theme}"))?
            .clone()
    } else {
        v.clone()
    };
    seen.remove(&key);
    Ok(result)
}
fn color(value: &Value) -> Result<[f32; 3], String> {
    let hex = value
        .as_str()
        .filter(|s| s.len() == 7 && s.starts_with('#'))
        .ok_or("proposal color must be opaque #RRGGBB")?;
    let mut rgb = [0.; 3];
    for (i, v) in rgb.iter_mut().enumerate() {
        let b = hex.as_bytes();
        let part =
            std::str::from_utf8(&b[1 + i * 2..3 + i * 2]).map_err(|_| "invalid hex bytes")?;
        *v = u8::from_str_radix(part, 16).map_err(|_| "invalid hex digit")? as f32 / 255.;
    }
    Ok(rgb)
}
pub(crate) fn validate(p: &Value) -> Result<(), Errors> {
    if !p.is_object() {
        return Err(vec!["proposal must be object".into()]);
    }
    let t = &p["tokens"];
    let expected: BTreeSet<_> = GROUPS.iter().copied().chain(["type"]).collect();
    if t.as_object()
        .map(|m| m.keys().map(String::as_str).collect::<BTreeSet<_>>())
        != Some(expected)
    {
        return Err(vec!["proposal token families missing/unknown".into()]);
    }
    if t["color"]["themes"] != json!(THEMES.iter().map(|(name, _)| name).collect::<Vec<_>>()) {
        return Err(vec!["proposal requires exact four themes".into()]);
    }
    let mut errors = vec![];
    for group in GROUPS {
        let Some(list) = t[group]["tokens"].as_array() else {
            errors.push(format!("proposal/{group}: tokens array missing"));
            continue;
        };
        let mut seen = BTreeSet::new();
        for v in list {
            let id = v["name"].as_str().unwrap_or("");
            let value = &v["value"];
            if *group != "color"
                && !(value.is_number()
                    || value
                        .as_str()
                        .is_some_and(|s| s.starts_with('{') && s.ends_with('}')))
            {
                errors.push(format!("proposal/{group}/{id}: expected scalar or alias"));
            }
            if *group == "color"
                && value.is_object()
                && value
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>()
                    != THEMES
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<BTreeSet<_>>()
            {
                errors.push(format!("proposal/color/{id}: exact theme keys required"));
            }
            if id.is_empty() || !seen.insert(id) {
                errors.push(format!("proposal/{group}: invalid or duplicate ID {id}"));
                continue;
            }
            for (theme, _) in THEMES {
                match resolve(t, group, id, theme, &mut BTreeSet::new()) {
                    Err(e) => errors.push(e),
                    Ok(value) => {
                        if *group == "color" {
                            if let Err(e) = color(&value) {
                                errors.push(e);
                            }
                        } else if !value.as_f64().is_some_and(|v| {
                            v >= 0. && (v as f32).is_finite() && (*group != "opacity" || v <= 1.)
                        }) {
                            errors.push(format!("proposal/{group}/{id}: invalid numeric value"));
                        }
                    }
                }
            }
        }
    }
    let Some(fonts) = t["type"]["fonts"].as_array() else {
        return Err(vec!["proposal fonts missing".into()]);
    };
    let font_names: BTreeSet<_> = fonts.iter().filter_map(|f| f["name"].as_str()).collect();
    let Some(families) = t["type"]["families"].as_array() else {
        return Err(vec!["proposal font families missing".into()]);
    };
    let family_names: BTreeSet<_> = families.iter().filter_map(|f| f["name"].as_str()).collect();
    for family in families {
        if !family["fonts"].as_array().is_some_and(|names| {
            !names.is_empty()
                && names
                    .iter()
                    .all(|name| name.as_str().is_some_and(|s| font_names.contains(s)))
        }) {
            errors.push("proposal font family refers to missing font".into());
        }
    }
    let Some(durations) = p["motion"]["duration"].as_object() else {
        return Err(vec!["proposal durations missing".into()]);
    };
    if !durations.values().all(|v| {
        v.as_f64()
            .is_some_and(|n| n >= 0. && (n as f32).is_finite())
    }) {
        errors.push("invalid proposal duration".into());
    }
    let Some(rules) = p["render_opacity"].as_object() else {
        return Err(vec!["proposal render opacity rules missing".into()]);
    };
    for (id, rule) in rules {
        if resolve(t, "color", id, "light", &mut BTreeSet::new()).is_err() {
            errors.push(format!("render opacity unknown color {id}"));
        }
        if rule
            .as_object()
            .map(|m| m.keys().map(String::as_str).collect::<BTreeSet<_>>())
            != Some(["default", "increased"].into_iter().collect())
        {
            errors.push(format!("invalid render opacity rule {id}"));
            continue;
        }
        for mode in ["default", "increased"] {
            if !rule[mode].as_str().is_some_and(|alpha| {
                resolve(t, "opacity", alpha, "light", &mut BTreeSet::new()).is_ok()
            }) {
                errors.push(format!("render opacity {id}: unknown alpha token"));
            }
        }
    }
    let mut style_names = BTreeSet::new();
    let mut variants = BTreeSet::new();
    let Some(groups) = t["type"]["groups"].as_array() else {
        return Err(vec!["proposal type groups missing".into()]);
    };
    for group in groups {
        let Some(styles) = group["styles"].as_array() else {
            return Err(vec!["proposal type styles missing".into()]);
        };
        for s in styles {
            let id = s["name"].as_str().unwrap_or("");
            let size = s["fontSize"].as_f64().unwrap_or(-1.);
            let height = s["lineHeight"].as_f64().unwrap_or(-1.);
            let weight = s["fontWeight"].as_u64().unwrap_or_default();
            let safe_id = !id.is_empty()
                && id.as_bytes()[0].is_ascii_lowercase()
                && id.split('-').all(|part| {
                    !part.is_empty()
                        && part
                            .bytes()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                });
            let variant = id
                .split('-')
                .map(|part| {
                    let mut chars = part.chars();
                    format!(
                        "{}{}",
                        chars.next().unwrap_or(' ').to_ascii_uppercase(),
                        chars.as_str()
                    )
                })
                .collect::<String>();
            if variant == "Self" || !variants.insert(variant) {
                errors.push(format!(
                    "proposal text {id}: reserved/colliding Rust variant"
                ));
            }
            if !safe_id
                || !s["family"]
                    .as_str()
                    .is_some_and(|f| family_names.contains(f))
            {
                errors.push(format!("proposal text {id}: invalid ID/family"));
            }
            let role = s["color"]
                .as_str()
                .and_then(|s| s.strip_prefix('{'))
                .and_then(|s| s.strip_suffix('}'));
            if let Some(role) = role {
                for (theme, _) in THEMES {
                    if resolve(t, "color", role, theme, &mut BTreeSet::new()).is_err() {
                        errors.push(format!("proposal text {id}: unknown color"));
                    }
                }
            } else {
                errors.push(format!("proposal text {id}: color alias required"));
            }
            if id.is_empty()
                || !style_names.insert(id)
                || size <= 0.
                || !(size as f32).is_finite()
                || !(height as f32).is_finite()
                || height < size
                || !(1..=1000).contains(&weight)
            {
                errors.push(format!("proposal style {id}: invalid metrics/ID"));
            }
        }
    }
    for path in [
        "/layout/gallery/capture_size/0",
        "/layout/gallery/capture_size/1",
        "/layout/sidebar/default",
    ] {
        if !p
            .pointer(path)
            .and_then(Value::as_f64)
            .is_some_and(|n| n > 0. && (n as f32).is_finite())
        {
            errors.push(format!("proposal{path}: invalid layout dimension"));
        }
    }
    if !p["brand"]["config"]["brand_id"].is_string() {
        errors.push("proposal requires explicit consumer brand".into());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn generate(p: &Value, out: &mut String) -> Result<(), Errors> {
    validate(p)?;
    let t = &p["tokens"];
    out.push_str("/// Full proposal vocabulary; old typed APIs are compatibility mappings.\npub mod proposal {\n");
    writeln!(
        out,
        "pub const BRAND_ID: &str = {:?};",
        p["brand"]["config"]["brand_id"].as_str().unwrap()
    )
    .unwrap();
    for (name, path) in [
        ("CAPTURE_WIDTH", "/layout/gallery/capture_size/0"),
        ("CAPTURE_HEIGHT", "/layout/gallery/capture_size/1"),
        ("SIDEBAR_WIDTH", "/layout/sidebar/default"),
    ] {
        writeln!(
            out,
            "pub const {name}: f32 = {};",
            bits(p.pointer(path).unwrap())
        )
        .unwrap();
    }
    out.push_str("/// Raw opaque sRGB token payload; use render_color to apply final-alpha contracts.\npub fn color(id:&str,theme:crate::Theme)->Option<crate::Rgba>{\nmatch(id,theme.appearance,theme.contrast){\n");
    for v in t["color"]["tokens"].as_array().unwrap() {
        let id = v["name"].as_str().unwrap();
        for (theme, _) in THEMES {
            let rgb =
                color(&resolve(t, "color", id, theme, &mut BTreeSet::new()).map_err(|e| vec![e])?)
                    .map_err(|e| vec![e])?;
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
            writeln!(out,"({id:?},crate::Appearance::{appearance},crate::Contrast::{contrast})=>Some(crate::Rgba::new({},{},{},f32::from_bits(0x3f800000))),",bits(&json!(rgb[0])),bits(&json!(rgb[1])),bits(&json!(rgb[2]))).unwrap();
        }
    }
    out.push_str("_=>None,}}\n");
    out.push_str("/// Final render color, replacing alpha according to JSON usage rules; never multiplying.\npub fn render_color(id:&str,theme:crate::Theme)->Option<crate::Rgba>{let raw=color(id,theme)?;let alpha=match(id,theme.contrast){\n");
    let rules = p["render_opacity"].as_object().unwrap();
    for item in t["color"]["tokens"].as_array().unwrap() {
        let id = item["name"].as_str().unwrap();
        let mut current = id;
        let mut seen = BTreeSet::new();
        let mut selected = None;
        while seen.insert(current) {
            if let Some(rule) = rules.get(current) {
                selected = Some(rule);
                break;
            }
            let record = t["color"]["tokens"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["name"] == current)
                .unwrap();
            if let Some(next) = record["value"]
                .as_str()
                .and_then(|s| s.strip_prefix('{'))
                .and_then(|s| s.strip_suffix('}'))
            {
                current = next;
            } else {
                break;
            }
        }
        if let Some(rule) = selected {
            for (mode, contrast) in [("default", "Standard"), ("increased", "Increased")] {
                let value = resolve(
                    t,
                    "opacity",
                    rule[mode].as_str().unwrap(),
                    "light",
                    &mut BTreeSet::new(),
                )
                .map_err(|e| vec![e])?;
                writeln!(
                    out,
                    "({id:?},crate::Contrast::{contrast})=>{},",
                    bits(&value)
                )
                .unwrap();
            }
        }
    }
    out.push_str("_=>raw.a,};Some(crate::Rgba::new(raw.r,raw.g,raw.b,alpha))}\n");
    out.push_str("pub fn metric(group:&str,id:&str)->Option<f32>{match(group,id){\n");
    for group in GROUPS.iter().filter(|g| **g != "color") {
        for v in t[group]["tokens"].as_array().unwrap() {
            let id = v["name"].as_str().unwrap();
            let value =
                resolve(t, group, id, "light", &mut BTreeSet::new()).map_err(|e| vec![e])?;
            writeln!(out, "({group:?},{id:?})=>Some({}),", bits(&value)).unwrap();
        }
    }
    out.push_str("_=>None,}}\n");
    out.push_str("pub fn duration(id:&str)->Option<f32>{match id{\n");
    for (id, value) in p["motion"]["duration"]
        .as_object()
        .ok_or_else(|| vec!["proposal duration missing".into()])?
    {
        writeln!(out, "{id:?}=>Some({}),", bits(value)).unwrap();
    }
    out.push_str("_=>None,}}\n");
    let styles: Vec<_> = t["type"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["styles"].as_array().unwrap())
        .collect();
    writeln!(
        out,
        "pub const TEXT_STYLES:[&str;{}]=[{}];",
        styles.len(),
        styles
            .iter()
            .map(|s| format!("{:?}", s["name"].as_str().unwrap()))
            .collect::<Vec<_>>()
            .join(",")
    )
    .unwrap();
    out.push_str("#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]\npub enum TextRole {\n");
    for style in &styles {
        let variant = style["name"]
            .as_str()
            .unwrap()
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                format!(
                    "{}{}",
                    chars.next().unwrap().to_ascii_uppercase(),
                    chars.as_str()
                )
            })
            .collect::<String>();
        writeln!(out, "{variant},").unwrap();
    }
    out.push_str("}\nimpl TextRole { pub const fn style(self)->crate::TextStyle {match self {\n");
    for style in &styles {
        let variant = style["name"]
            .as_str()
            .unwrap()
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                format!(
                    "{}{}",
                    chars.next().unwrap().to_ascii_uppercase(),
                    chars.as_str()
                )
            })
            .collect::<String>();
        writeln!(
            out,
            "Self::{variant}=>crate::TextStyle{{size:{},line_height:{},weight:{}}},",
            bits(&style["fontSize"]),
            bits(&style["lineHeight"]),
            style["fontWeight"].as_u64().unwrap()
        )
        .unwrap();
    }
    out.push_str("}} pub const fn id(self)->&'static str {match self {\n");
    for style in &styles {
        let id = style["name"].as_str().unwrap();
        let variant = id
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                format!(
                    "{}{}",
                    chars.next().unwrap().to_ascii_uppercase(),
                    chars.as_str()
                )
            })
            .collect::<String>();
        writeln!(out, "Self::{variant}=>{id:?},").unwrap();
    }
    out.push_str("}}}\n");
    out.push_str("pub fn text_family(id:&str)->Option<&'static str>{match id{\n");
    for style in &styles {
        writeln!(
            out,
            "{:?}=>Some({:?}),",
            style["name"].as_str().unwrap(),
            style["family"].as_str().unwrap()
        )
        .unwrap();
    }
    out.push_str("_=>None,}}\n");
    out.push_str("pub fn text_color(id:&str,theme:crate::Theme)->Option<crate::Rgba>{match id{\n");
    for style in &styles {
        let name = style["color"]
            .as_str()
            .unwrap()
            .trim_start_matches('{')
            .trim_end_matches('}');
        writeln!(
            out,
            "{:?}=>render_color({name:?},theme),",
            style["name"].as_str().unwrap()
        )
        .unwrap();
    }
    out.push_str("_=>None,}}\n");
    out.push_str("pub fn text(id:&str)->Option<crate::TextStyle>{match id{\n");
    for s in styles {
        writeln!(
            out,
            "{:?}=>Some(crate::TextStyle{{size:{},line_height:{},weight:{}}}),",
            s["name"].as_str().unwrap(),
            bits(&s["fontSize"]),
            bits(&s["lineHeight"]),
            s["fontWeight"].as_u64().unwrap()
        )
        .unwrap();
    }
    out.push_str("_=>None,}}\n}\n");
    Ok(())
}
