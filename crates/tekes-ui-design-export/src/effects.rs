//! Compile local component-feedback and elevation bindings from authoritative JSON.
use serde_json::Value;
use std::fmt::Write;

const ROLES: [&str; 4] = [
    "button_hover",
    "button_press",
    "switch_thumb",
    "disclosure_reveal",
];

pub(crate) fn validate(p: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    for role in ROLES {
        let binding = &p["motion"]["control_transitions"][role];
        let duration = binding["duration"]
            .as_str()
            .and_then(|id| p["motion"]["control_duration"][id].as_f64());
        let curve = binding["easing"]
            .as_str()
            .and_then(|id| p["motion"]["easing"][id].as_array());
        if !duration.is_some_and(|n| n.is_finite() && (0.0..=1.0).contains(&n)) {
            errors.push(format!(
                "control transition {role}: valid bound duration required"
            ));
        }
        if !curve.is_some_and(|v| {
            v.len() == 4
                && v.iter().all(|n| {
                    n.as_f64()
                        .is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))
                })
        }) {
            errors.push(format!(
                "control transition {role}: valid bound cubic easing required"
            ));
        }
    }
    if !p["layout"]["boundary_styles"]["focus"]["antialias_guard_physical_pixels"]
        .as_f64()
        .is_some_and(|v| v.is_finite() && (0.0..=4.0).contains(&v))
    {
        errors.push("focus: finite physical-pixel antialias guard required".into());
    }
    let elevation = &p["layout"]["elevation"];
    for (key, expected) in [
        ("unit", "logical-pt"),
        (
            "blur_model",
            "gaussian-sigma-equals-css-box-shadow-blur-radius-divided-by-two",
        ),
        ("layer_order", "front-to-back"),
    ] {
        if elevation[key].as_str() != Some(expected) {
            errors.push(format!("elevation {key}: unsupported rendering contract"));
        }
    }
    for role in ["menu", "popover", "dialog", "panel"] {
        if !matches!(
            elevation["roles"][role].as_str(),
            Some("floating" | "modal" | "none")
        ) {
            errors.push(format!("elevation role {role}: known level required"));
        }
    }
    for level in ["floating", "modal"] {
        for mode in ["light", "dark"] {
            let Some(layers) = elevation["levels"][level][mode].as_array() else {
                errors.push(format!("elevation {level}/{mode}: layers required"));
                continue;
            };
            if layers.len() != 2 {
                errors.push(format!("elevation {level}/{mode}: two layers required"));
            }
            for layer in layers {
                let finite = |key: &str| {
                    layer[key]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && n.abs() <= 512.)
                };
                if !finite("blur_radius")
                    || !finite("spread")
                    || layer["blur_radius"].as_f64().unwrap_or(-1.) < 0.
                    || !layer["alpha"]
                        .as_f64()
                        .is_some_and(|a| (0.0..=1.0).contains(&a))
                    || !layer["offset"].as_array().is_some_and(|a| {
                        a.len() == 2
                            && a.iter().all(|v| {
                                v.as_f64().is_some_and(|n| n.is_finite() && n.abs() <= 512.)
                            })
                    })
                    || layer["color"].as_str() != Some("shadow-ink")
                {
                    errors.push(format!("elevation {level}/{mode}: invalid Gaussian layer"));
                }
            }
        }
    }
    if elevation["increased_contrast"]["shadow"] != "none" {
        errors.push(
            "increased contrast must retain explicit boundaries without relying on shadows".into(),
        );
    }
    errors
}

pub(crate) fn generate(p: &Value, out: &mut String) {
    writeln!(
        out,
        "pub const FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS:f32={};",
        crate::bits(&p["layout"]["boundary_styles"]["focus"]["antialias_guard_physical_pixels"])
    )
    .unwrap();
    out.push_str("pub fn control_duration(role:&str)->Option<f32>{match role{\n");
    for role in ROLES {
        let binding = &p["motion"]["control_transitions"][role];
        let key = binding["duration"].as_str().unwrap();
        writeln!(
            out,
            "{role:?}=>Some({}),",
            crate::bits(&p["motion"]["control_duration"][key])
        )
        .unwrap();
    }
    out.push_str("_=>None,}}\npub fn control_easing(role:&str)->Option<[f32;4]>{match role{\n");
    for role in ROLES {
        let key = p["motion"]["control_transitions"][role]["easing"]
            .as_str()
            .unwrap();
        let values = p["motion"]["easing"][key]
            .as_array()
            .unwrap()
            .iter()
            .map(crate::bits)
            .collect::<Vec<_>>()
            .join(",");
        writeln!(out, "{role:?}=>Some([{values}]),").unwrap();
    }
    out.push_str("_=>None,}}\n#[derive(Clone,Copy,Debug,PartialEq)] pub struct ShadowLayer { pub offset_x:f32,pub offset_y:f32,pub blur_radius:f32,pub spread:f32,pub alpha:f32,pub color:&'static str }\n");
    for level in ["floating", "modal"] {
        for mode in ["light", "dark"] {
            writeln!(
                out,
                "const SHADOW_{}_{}:&[ShadowLayer]=&[",
                level.to_uppercase(),
                mode.to_uppercase()
            )
            .unwrap();
            for layer in p["layout"]["elevation"]["levels"][level][mode]
                .as_array()
                .unwrap()
            {
                writeln!(out,"ShadowLayer{{offset_x:{},offset_y:{},blur_radius:{},spread:{},alpha:{},color:\"shadow-ink\"}},",
                    crate::bits(&layer["offset"][0]),crate::bits(&layer["offset"][1]),crate::bits(&layer["blur_radius"]),crate::bits(&layer["spread"]),crate::bits(&layer["alpha"])).unwrap();
            }
            out.push_str("];\n");
        }
    }
    out.push_str("pub fn elevation(role:&str,theme:crate::Theme)->&'static[ShadowLayer]{if matches!(theme.contrast,crate::Contrast::Increased){return &[];}match(role,theme.appearance){\n");
    for role in ["menu", "popover", "dialog", "panel"] {
        let level = p["layout"]["elevation"]["roles"][role]
            .as_str()
            .unwrap_or("none");
        if level == "none" {
            continue;
        }
        for (mode, appearance) in [("light", "Light"), ("dark", "Dark")] {
            writeln!(
                out,
                "({role:?},crate::Appearance::{appearance})=>SHADOW_{}_{},",
                level.to_uppercase(),
                mode.to_uppercase()
            )
            .unwrap();
        }
    }
    out.push_str("_=>&[],}}\n");
}
