use serde_json::{Map, Value};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};
use tekes_ui_tokens::{
    Palette, Theme,
    palette::PaletteSource,
    proposal,
    surface_palette::{SurfaceColors, resolve_surface},
};

const MODES: [(&str, Theme); 4] = [
    ("light", Theme::LIGHT),
    ("dark", Theme::DARK),
    ("light-ic", Theme::LIGHT_INCREASED),
    ("dark-ic", Theme::DARK_INCREASED),
];

fn object<'a>(value: &'a Value, context: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{context}: expected JSON object"))
}

fn expected_rgb(value: &Value, context: &str) -> Result<[u8; 3], String> {
    let hex = value
        .as_str()
        .ok_or_else(|| format!("{context}: expected RGB hex string"))?;
    let digits = hex
        .strip_prefix('#')
        .ok_or_else(|| format!("{context}: expected #RRGGBB, got {hex:?}"))?;
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{context}: expected #RRGGBB, got {hex:?}"));
    }
    let channel = |offset| {
        u8::from_str_radix(&digits[offset..offset + 2], 16)
            .map_err(|error| format!("{context}: invalid RGB hex {hex:?}: {error}"))
    };
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

fn compare_rgb(
    actual: tekes_ui_tokens::Rgba,
    expected: &Value,
    context: &str,
) -> Result<(), String> {
    let [r, g, b] = expected_rgb(expected, context)?;
    if !actual.in_range() {
        return Err(format!(
            "{context}: actual color is out of range: {actual:?}"
        ));
    }
    let actual_rgb = [actual.r, actual.g, actual.b].map(|channel| (channel * 255.0).round() as u8);
    if actual_rgb != [r, g, b] {
        return Err(format!(
            "{context}: RGB mismatch: actual #{:02X}{:02X}{:02X}, expected #{r:02X}{g:02X}{b:02X}",
            actual_rgb[0], actual_rgb[1], actual_rgb[2]
        ));
    }
    Ok(())
}

fn compare_optional_rgb(
    actual: Option<tekes_ui_tokens::Rgba>,
    expected: &Value,
    context: &str,
) -> Result<(), String> {
    match (actual, expected) {
        (None, Value::Null) => Ok(()),
        (Some(actual), Value::String(_)) => compare_rgb(actual, expected, context),
        (None, _) => Err(format!("{context}: actual channel is absent")),
        (Some(actual), Value::Null) => Err(format!(
            "{context}: expected absent channel, got {actual:?}"
        )),
        (_, other) => Err(format!("{context}: invalid expected channel {other}")),
    }
}

fn compare_surface_packet(
    actual: SurfaceColors,
    expected: &Value,
    context: &str,
) -> Result<(), String> {
    let expected = object(expected, context)?;
    let required_fields = [
        "background",
        "boundary",
        "focus",
        "helper",
        "icon",
        "marker",
        "text",
    ];
    let actual_fields: BTreeSet<_> = expected.keys().map(String::as_str).collect();
    let required_fields: BTreeSet<_> = required_fields.into_iter().collect();
    if actual_fields != required_fields {
        return Err(format!(
            "{context}: packet channel set is incomplete or unexpected"
        ));
    }
    compare_rgb(
        actual.background,
        &expected["background"],
        &format!("{context}/background"),
    )?;
    compare_rgb(actual.text, &expected["text"], &format!("{context}/text"))?;
    compare_rgb(actual.icon, &expected["icon"], &format!("{context}/icon"))?;
    compare_optional_rgb(
        actual.boundary,
        &expected["boundary"],
        &format!("{context}/boundary"),
    )?;
    compare_optional_rgb(
        actual.focus,
        &expected["focus"],
        &format!("{context}/focus"),
    )?;
    compare_optional_rgb(
        actual.marker,
        &expected["marker"],
        &format!("{context}/marker"),
    )?;
    compare_optional_rgb(
        actual.helper,
        &expected["helper"],
        &format!("{context}/helper"),
    )?;
    Ok(())
}

fn load_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let artifact_root = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: theme_identity <artifact-root> [theme-id]")?;
    let theme_id = args
        .next()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "theme-id must be valid UTF-8".to_owned())
        })
        .transpose()?
        .unwrap_or_else(|| "blue".to_owned());
    if args.next().is_some() {
        return Err("usage: theme_identity <artifact-root> [theme-id]".into());
    }

    let instances_path = artifact_root.join("generated/instances.json");
    let rules_path = artifact_root.join("rules.json");
    let instances_document = load_json(&instances_path)?;
    let rules_document = load_json(&rules_path)?;
    let instances = instances_document["instances"]
        .as_array()
        .ok_or("instances.json has no instances array")?;
    let selected_index = instances
        .iter()
        .position(|instance| instance["id"].as_str() == Some(theme_id.as_str()));
    let Some(selected_index) = selected_index else {
        if Palette::for_theme_definition(Theme::LIGHT, &theme_id).is_some() {
            return Err(format!(
                "tokens build recognizes {theme_id:?}, but the expected artifact does not"
            ));
        }
        return Err(format!("unknown Theme definition id: {theme_id}"));
    };
    let expected_instance = &instances[selected_index];
    if expected_instance["id"].as_str() != Some(theme_id.as_str()) {
        return Err("selected artifact id differs from requested id".into());
    }
    if Palette::for_theme_definition(Theme::LIGHT, "__theme_identity_missing_id__").is_some() {
        return Err("tokens build unexpectedly accepts an unknown Theme id".into());
    }

    let rules_surfaces = object(&rules_document["surfaces"], "rules.surfaces")?;
    let theme_modes = object(
        &expected_instance["compiled_color_roles"],
        &format!("{theme_id}/compiled_color_roles"),
    )?;
    let expected_mode_names: BTreeSet<_> = MODES.iter().map(|(name, _)| *name).collect();
    let artifact_mode_names: BTreeSet<_> = theme_modes.keys().map(String::as_str).collect();
    if artifact_mode_names != expected_mode_names {
        return Err(format!(
            "{theme_id}: expected exactly the four compiled modes"
        ));
    }
    let mut compared_roles = 0usize;
    let mut compared_packets = 0usize;
    for (mode_name, theme) in MODES {
        let palette = Palette::for_theme_definition(theme, &theme_id)
            .ok_or_else(|| format!("tokens build does not contain Theme id {theme_id:?}"))?;
        if palette.source()
            != (PaletteSource::ThemeDefinition {
                index: selected_index,
            })
        {
            return Err(format!(
                "{mode_name}: source {:?} does not match {theme_id:?} at artifact index {selected_index}",
                palette.source()
            ));
        }
        if palette.brand_index().is_some() {
            return Err(format!(
                "{mode_name}: Theme definition was reduced to a Gallery brand"
            ));
        }

        let expected_roles = object(
            theme_modes
                .get(mode_name)
                .ok_or_else(|| format!("{theme_id}/{mode_name}: missing compiled mode"))?,
            &format!("{theme_id}/{mode_name}/compiled_color_roles"),
        )?;
        for (role, expected_hex) in expected_roles {
            let actual = palette.color(role).ok_or_else(|| {
                format!("{mode_name}/{role}: Palette did not resolve compiled role")
            })?;
            compare_rgb(actual, expected_hex, &format!("{mode_name}/{role}"))?;
            let proposal_color = proposal::render_color(role, theme).ok_or_else(|| {
                format!("{mode_name}/{role}: original proposal alpha is unavailable")
            })?;
            if actual.a.to_bits() != proposal_color.a.to_bits() {
                return Err(format!(
                    "{mode_name}/{role}: alpha changed from proposal {:?} to {:?}",
                    proposal_color.a, actual.a
                ));
            }
            compared_roles += 1;
        }

        let expected_mode = object(
            &expected_instance["modes"][mode_name],
            &format!("{theme_id}/{mode_name}/modes"),
        )?;
        let rule_surface_names: BTreeSet<_> = rules_surfaces.keys().map(String::as_str).collect();
        let artifact_surface_names: BTreeSet<_> =
            expected_mode.keys().map(String::as_str).collect();
        if rule_surface_names != artifact_surface_names {
            return Err(format!(
                "{mode_name}: artifact surfaces do not match supplied rules"
            ));
        }
        for (surface, surface_definition) in rules_surfaces {
            let usages = object(
                &surface_definition["usages"],
                &format!("rules.surfaces.{surface}.usages"),
            )?;
            let expected_surface = object(
                expected_mode
                    .get(surface)
                    .ok_or_else(|| format!("{mode_name}/{surface}: missing artifact surface"))?,
                &format!("{theme_id}/{mode_name}/{surface}"),
            )?;
            let rule_usage_names: BTreeSet<_> = usages.keys().map(String::as_str).collect();
            let artifact_usage_names: BTreeSet<_> =
                expected_surface.keys().map(String::as_str).collect();
            if rule_usage_names != artifact_usage_names {
                return Err(format!(
                    "{mode_name}/{surface}: artifact usages do not match the supplied rules"
                ));
            }
            for usage in usages.keys() {
                let states = object(
                    expected_surface
                        .get(usage)
                        .ok_or_else(|| format!("{mode_name}/{surface}/{usage}: missing states"))?,
                    &format!("{theme_id}/{mode_name}/{surface}/{usage}"),
                )?;
                if states.is_empty() {
                    return Err(format!("{mode_name}/{surface}/{usage}: no compiled states"));
                }
                for (state, expected_packet) in states {
                    let actual =
                        resolve_surface(palette, surface, usage, state).map_err(|error| {
                            format!("{mode_name}/{surface}/{usage}/{state}: {error:?}")
                        })?;
                    compare_surface_packet(
                        actual,
                        expected_packet,
                        &format!("{mode_name}/{surface}/{usage}/{state}"),
                    )?;
                    compared_packets += 1;
                }
            }
        }
    }

    println!(
        "THEME_IDENTITY_OK id={theme_id} definition_index={selected_index} modes=4 compiled_role_checks={compared_roles} surface_packet_checks={compared_packets}"
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("THEME_IDENTITY_FAILED: {error}");
        std::process::exit(1);
    }
}
