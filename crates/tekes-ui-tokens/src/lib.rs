//! Design tokens compiled from the generated design/tokens.json compatibility projection.
//! Canonical author inputs live in design/source and design/brand JSON files.
//! Lengths use logical points; durations use seconds. No runtime dependencies.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

pub use generated::proposal;

pub mod alpha;
pub mod color;
pub mod metrics;
pub mod text;
pub mod theme;

pub use alpha::Alpha;
pub use color::{ColorRole, Rgba};
pub use metrics::{ControlSize, CornerRadius, Motion, Spacing, Stroke};
pub use text::{TextRole, TextStyle};
pub use theme::{Appearance, Contrast, Theme};

#[cfg(test)]
mod migration_tests {
    #[test]
    fn all_runtime_channels_match_current_merged_json() {
        let mut actual = crate::generated::__runtime_channels();
        actual.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(actual.len(), 508);
        let text = actual
            .iter()
            .map(|(path, bits)| format!("{path} {bits:08x}\n"))
            .collect::<String>();
        let document = serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap();
        let expected = tekes_ui_design_export::numeric_channels(&document)
            .unwrap()
            .into_iter()
            .map(|(path, bits)| format!("{path} {bits:08x}\n"))
            .collect::<String>();
        assert_eq!(text, expected);
    }
    #[test]
    fn complete_proposal_vocabulary_and_text_metadata_reach_runtime() {
        use crate::{Rgba, Theme, proposal};
        use serde_json::Value;
        let document: Value =
            serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap();
        let tokens = &document["proposal"]["tokens"];
        fn resolve(tokens: &Value, family: &str, id: &str, theme: &str) -> Value {
            let item = tokens[family]["tokens"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["name"] == id)
                .unwrap();
            match &item["value"] {
                Value::String(v) if v.starts_with('{') => {
                    resolve(tokens, family, &v[1..v.len() - 1], theme)
                }
                Value::Object(map) => map[theme].clone(),
                value => value.clone(),
            }
        }
        for item in tokens["color"]["tokens"].as_array().unwrap() {
            let id = item["name"].as_str().unwrap();
            for (theme, name) in Theme::ALL
                .into_iter()
                .zip(["light", "dark", "light-ic", "dark-ic"])
            {
                let value = resolve(tokens, "color", id, name);
                let h = value.as_str().unwrap();
                let rgb =
                    [1, 3, 5].map(|offset| u8::from_str_radix(&h[offset..offset + 2], 16).unwrap());
                assert_eq!(
                    proposal::color(id, theme).unwrap(),
                    Rgba::rgb8(rgb[0], rgb[1], rgb[2])
                );
            }
        }
        for family in ["spacing", "radius", "stroke", "opacity", "size"] {
            for item in tokens[family]["tokens"].as_array().unwrap() {
                let id = item["name"].as_str().unwrap();
                let expected = resolve(tokens, family, id, "light").as_f64().unwrap() as f32;
                assert_eq!(
                    proposal::metric(family, id).unwrap().to_bits(),
                    expected.to_bits()
                );
            }
        }
        let styles: Vec<_> = tokens["type"]["groups"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["styles"].as_array().unwrap())
            .collect();
        assert_eq!(proposal::TEXT_STYLES.len(), styles.len());
        for style in styles {
            let id = style["name"].as_str().unwrap();
            let actual = proposal::text(id).unwrap();
            assert_eq!(
                actual.size.to_bits(),
                (style["fontSize"].as_f64().unwrap() as f32).to_bits()
            );
            assert_eq!(
                actual.line_height.to_bits(),
                (style["lineHeight"].as_f64().unwrap() as f32).to_bits()
            );
            assert_eq!(
                actual.weight,
                u16::try_from(style["fontWeight"].as_u64().unwrap()).unwrap()
            );
            assert_eq!(proposal::text_family(id), style["family"].as_str());
            let color = style["color"].as_str().unwrap();
            assert_eq!(
                proposal::text_color(id, Theme::LIGHT),
                proposal::color(&color[1..color.len() - 1], Theme::LIGHT)
            );
        }
        assert!(proposal::text("unknown-style").is_none());
        assert!(proposal::color("unknown-role", Theme::LIGHT).is_none());
    }
    #[test]
    fn final_render_alpha_contracts_match_legacy_and_keep_raw_opaque() {
        use crate::{Alpha, ColorRole, Theme, proposal};
        for theme in Theme::ALL {
            for (id, role, alpha) in [
                ("scrim", ColorRole::Scrim, Alpha::Wash.value()),
                (
                    "hover-wash",
                    ColorRole::HoverWash,
                    Alpha::Faint.resolve(theme.contrast),
                ),
                (
                    "text-selection",
                    ColorRole::TextSelection,
                    if theme.is_increased() {
                        crate::alpha::TEXT_SELECTION_INCREASED
                    } else {
                        1.0
                    },
                ),
            ] {
                assert_eq!(proposal::color(id, theme).unwrap().a, 1.0);
                let actual = proposal::render_color(id, theme).unwrap();
                assert_eq!(actual.a.to_bits(), alpha.to_bits());
                assert_eq!(role.resolve(theme), actual);
            }
        }
    }
}
pub mod palette;
pub use palette::Palette;

#[cfg(test)]
mod effect_binding_tests {
    use super::{Theme, proposal};
    #[test]
    fn component_motion_and_overlay_elevation_round_trip_json() {
        let document: serde_json::Value =
            serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap();
        let motion = &document["proposal"]["motion"];
        for role in [
            "button_hover",
            "button_press",
            "switch_thumb",
            "disclosure_reveal",
        ] {
            let binding = &motion["control_transitions"][role];
            let duration = motion["control_duration"][binding["duration"].as_str().unwrap()]
                .as_f64()
                .unwrap() as f32;
            assert_eq!(proposal::control_duration(role), Some(duration));
            let curve = motion["easing"][binding["easing"].as_str().unwrap()]
                .as_array()
                .unwrap();
            assert_eq!(
                proposal::control_easing(role).unwrap(),
                std::array::from_fn(|i| curve[i].as_f64().unwrap() as f32)
            );
        }
        assert_eq!(proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS,
            document["proposal"]["layout"]["boundary_styles"]["focus"]["antialias_guard_physical_pixels"].as_f64().unwrap() as f32);
        let elevation = &document["proposal"]["layout"]["elevation"];
        for theme in Theme::ALL {
            for role in ["menu", "popover", "dialog", "panel"] {
                let layers = proposal::elevation(role, theme);
                if theme.is_increased() || role == "panel" {
                    assert!(layers.is_empty());
                    continue;
                }
                let level = elevation["roles"][role].as_str().unwrap();
                let source = elevation["levels"][level]
                    [if theme.is_dark() { "dark" } else { "light" }]
                .as_array()
                .unwrap();
                assert_eq!(layers.len(), source.len());
                for (actual, expected) in layers.iter().zip(source) {
                    assert_eq!(
                        actual.offset_x,
                        expected["offset"][0].as_f64().unwrap() as f32
                    );
                    assert_eq!(
                        actual.offset_y,
                        expected["offset"][1].as_f64().unwrap() as f32
                    );
                    assert_eq!(
                        actual.blur_radius,
                        expected["blur_radius"].as_f64().unwrap() as f32
                    );
                    assert_eq!(actual.spread, expected["spread"].as_f64().unwrap() as f32);
                    assert_eq!(actual.alpha, expected["alpha"].as_f64().unwrap() as f32);
                    assert_eq!(actual.color, expected["color"].as_str().unwrap());
                }
            }
        }
    }
}
