//! Theme/Surface rule compiler. Color generation remains in the existing brand compiler.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
const CHANNELS: [&str; 7] = [
    "background",
    "text",
    "icon",
    "boundary",
    "focus",
    "marker",
    "helper",
];
const STATES: [&str; 8] = [
    "rest",
    "hover",
    "pressed",
    "disabled",
    "selected",
    "selected-hover",
    "selected-pressed",
    "selected-disabled",
];
const USES: [&str; 31] = [
    "content",
    "content-selectable",
    "action-quiet",
    "action-primary",
    "input",
    "selection",
    "link",
    "text-selection",
    "input-readonly",
    "input-error",
    "input-readonly-error",
    "action-quiet-error",
    "action-primary-error",
    "selection-error",
    "check-unchecked",
    "check-checked",
    "check-mixed",
    "check-unchecked-error",
    "check-checked-error",
    "check-mixed-error",
    "control-label",
    "toggle-track",
    "toggle-track-error",
    "toggle-thumb",
    "range-track",
    "range-thumb",
    "surface-frame",
    "content-status-info",
    "content-status-running",
    "content-status-succeeded",
    "content-status-failed",
];

fn validate_use_channels(usage: &str, state: &str, pair: &Value) -> Result<(), String> {
    if usage == "link" && ["marker", "helper"].iter().any(|k| !pair[k].is_null()) {
        return Err(format!("Link has no marker/helper anatomy: {state}"));
    }
    if usage.starts_with("content-status-")
        && (pair["background"] != "$surface"
            || ["boundary", "focus", "marker", "helper"]
                .iter()
                .any(|k| !pair[k].is_null()))
    {
        return Err("passive status content requires parent ground and forbids boundary/focus/marker/helper".into());
    }
    let focus_free = usage.starts_with("content-status-")
        || [
            "content",
            "text-selection",
            "control-label",
            "toggle-thumb",
            "range-track",
            "surface-frame",
        ]
        .contains(&usage);
    let needs_focus = !focus_free && !state.contains("disabled");
    if needs_focus == pair["focus"].is_null() {
        return Err(format!("incomplete or inapplicable focus: {usage}/{state}"));
    }
    if (usage == "range-track"
        || usage.starts_with("check-checked")
        || usage.starts_with("check-mixed"))
        && pair["marker"].is_null()
    {
        return Err(format!("missing required marker: {usage}/{state}"));
    }
    if usage.starts_with("input") && pair["helper"].is_null() {
        return Err(format!("missing required helper: {usage}/{state}"));
    }
    Ok(())
}

fn object<'a>(v: &'a Value, keys: &[&str]) -> Result<&'a serde_json::Map<String, Value>, String> {
    let map = v.as_object().ok_or("expected object")?;
    if map.keys().map(String::as_str).collect::<BTreeSet<_>>() != keys.iter().copied().collect() {
        return Err(format!("expected exact fields {keys:?}"));
    }
    Ok(map)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn read_safe_file(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || path.to_str() != Some(relative)
        || relative.contains('\\')
    {
        return Err(format!("unsafe Theme input path: {relative}"));
    }
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = root
        .join(path)
        .canonicalize()
        .map_err(|e| format!("{relative}: {e}"))?;
    if !canonical_path.starts_with(&canonical_root) || !canonical_path.is_file() {
        return Err(format!(
            "Theme input escapes source root or is not a file: {relative}"
        ));
    }
    fs::read(canonical_path).map_err(|e| format!("{relative}: {e}"))
}

fn theme_input_contract(root: &Path, rules: &Value) -> Result<Value, String> {
    let load = |path: &str| -> Result<Value, String> {
        serde_json::from_slice(&read_safe_file(root, path)?).map_err(|e| format!("{path}: {e}"))
    };
    let system = load("design/system.json")?;
    let system_schema: Value = load("design/system.schema.json")?;
    let definition_schema: Value = load("design/theme/theme.schema.json")?;
    let surface_schema: Value = load("design/theme/rules.schema.json")?;
    if system["$schema"] != "./system.schema.json"
        || system["format"] != "tekesui-design-system/2"
        || system_schema["$schema"].as_str().is_none()
    {
        return Err("unsupported Design System root/schema".into());
    }
    let contract = &system["theme_system"]["theme_input_contract"];
    object(
        contract,
        &[
            "contract_version",
            "definition_schema_version",
            "instance_format",
            "surface_schema_version",
            "rules_version",
            "supported_modes",
            "pinned_inputs",
            "neutral_base_allowed_roles",
            "status_base_allowed_roles",
            "constraints_ref",
        ],
    )?;
    if contract["contract_version"] != "1.0.0"
        || contract["definition_schema_version"]
            != definition_schema["properties"]["schema_version"]["const"]
        || contract["definition_schema_version"] != "2.0.0"
        || contract["instance_format"] != "tekesui.theme-instances/2"
        || contract["surface_schema_version"]
            != surface_schema["properties"]["schema_version"]["const"]
        || contract["surface_schema_version"] != rules["schema_version"]
        || contract["rules_version"] != rules["rules_version"]
        || contract["supported_modes"]
            != serde_json::json!(["light", "dark", "light-ic", "dark-ic"])
        || rules["modes"] != contract["supported_modes"]
        || contract["constraints_ref"] != "design/brand/policy.json"
    {
        return Err("stale or unsupported Theme input contract".into());
    }
    let pin_paths = [
        "design/brand/policy.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/brand.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/policy.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/quality/contrast-pairs.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/tokens.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/requirements.txt",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand_color.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/validate.py",
        "design/source/base.tokens.json",
        "design/source/token-metadata.json",
        "design/system.schema.json",
        "design/theme/artifact-lock.schema.json",
        "design/theme/rules.json",
        "design/theme/rules.schema.json",
        "design/theme/theme.schema.json",
        "scripts/generate-theme-system.py",
        "scripts/merge-design-system.py",
    ];
    let pins = contract["pinned_inputs"]
        .as_object()
        .ok_or("missing Theme contract pins")?;
    if pins.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != pin_paths.iter().copied().collect::<BTreeSet<_>>()
    {
        return Err("Theme contract has missing or unknown shared input pins".into());
    }
    for path in pin_paths {
        let digest = sha256_hex(&read_safe_file(root, path)?);
        if pins[path].as_str() != Some(digest.as_str()) {
            return Err(format!("stale Theme contract pin: {path}"));
        }
    }
    for field in ["neutral_base_allowed_roles", "status_base_allowed_roles"] {
        let values = contract[field]
            .as_array()
            .ok_or("invalid allowed override roles")?;
        if values.is_empty() {
            return Err("empty Theme override vocabulary".into());
        }
        let mut roles = BTreeSet::new();
        for value in values {
            let role = value.as_str().ok_or("invalid allowed override role")?;
            if !roles.insert(role) || !role.starts_with("p-") {
                return Err("duplicate or invalid allowed override role".into());
            }
        }
    }
    let neutral = contract["neutral_base_allowed_roles"].as_array().unwrap();
    let status = contract["status_base_allowed_roles"].as_array().unwrap();
    if neutral.iter().any(|role| status.contains(role)) {
        return Err("Theme override vocabularies overlap".into());
    }
    let base = load("design/source/base.tokens.json")?;
    let metadata = load("design/source/token-metadata.json")?;
    let base_tokens = base["color"]["tokens"]
        .as_array()
        .ok_or("missing base color tokens")?;
    for field in ["neutral_base_allowed_roles", "status_base_allowed_roles"] {
        for role in contract[field].as_array().unwrap() {
            let name = role.as_str().ok_or("invalid allowed override role")?;
            let metadata_key = format!("color:{name}");
            if metadata["tokens"][metadata_key.as_str()]["tier"] != "primitive"
                || base_tokens
                    .iter()
                    .filter(|token| token["name"] == name)
                    .count()
                    != 1
            {
                return Err(format!(
                    "allowed Theme override role is not a unique base primitive: {name}"
                ));
            }
        }
    }
    Ok(contract.clone())
}

fn valid_opaque_hex(value: &Value) -> bool {
    value.as_str().is_some_and(|color| {
        color.len() == 7
            && color.starts_with('#')
            && color.is_ascii()
            && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
    })
}

fn validate_theme_overrides(
    overrides: &Value,
    contract: &Value,
    base: &Value,
    metadata: &Value,
) -> Result<(), String> {
    object(overrides, &["neutral_base", "status_base"])?;
    let base_tokens = base["color"]["tokens"]
        .as_array()
        .ok_or("missing base color tokens")?;
    for (category, vocabulary) in [
        ("neutral_base", "neutral_base_allowed_roles"),
        ("status_base", "status_base_allowed_roles"),
    ] {
        let rows = overrides[category]
            .as_object()
            .ok_or("invalid Theme color overrides")?;
        for (name, modes) in rows {
            let allowed = contract[vocabulary]
                .as_array()
                .ok_or("missing Theme override vocabulary")?;
            if !allowed.iter().any(|v| v.as_str() == Some(name.as_str())) {
                return Err(format!("Theme override role is not allowed: {name}"));
            }
            let metadata_key = format!("color:{name}");
            if metadata["tokens"][&metadata_key]["tier"] != "primitive" {
                return Err(format!(
                    "Theme override must name a primitive color: {name}"
                ));
            }
            let base_matches = base_tokens
                .iter()
                .filter(|t| t["name"].as_str() == Some(name.as_str()))
                .collect::<Vec<_>>();
            if base_matches.len() != 1
                || base_matches[0]["value"]
                    .as_str()
                    .is_some_and(|v| v.starts_with('{'))
            {
                return Err(format!(
                    "Theme override names an absent, duplicate, or aliased base color: {name}"
                ));
            }
            let mode_map = modes
                .as_object()
                .ok_or("Theme override requires all four modes")?;
            if mode_map.keys().map(String::as_str).collect::<BTreeSet<_>>()
                != ["light", "dark", "light-ic", "dark-ic"]
                    .into_iter()
                    .collect()
            {
                return Err(format!("Theme override requires all four modes: {name}"));
            }
            for mode in ["light", "dark", "light-ic", "dark-ic"] {
                if !valid_opaque_hex(&modes[mode]) {
                    return Err(format!("invalid opaque sRGB hex override: {name}/{mode}"));
                }
            }
        }
    }
    Ok(())
}

fn validate_compiled_roles(role_maps: &Value, overrides: &Value) -> Result<Value, String> {
    object(role_maps, &["light", "dark", "light-ic", "dark-ic"])?;
    let first_roles = role_maps["light"]
        .as_object()
        .filter(|roles| !roles.is_empty())
        .ok_or("missing compiled Theme roles")?;
    let expected_names = first_roles
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut normalized = serde_json::Map::new();
    for mode in ["light", "dark", "light-ic", "dark-ic"] {
        let roles = role_maps[mode]
            .as_object()
            .ok_or("invalid compiled Theme role map")?;
        if roles.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_names {
            return Err(format!(
                "compiled Theme role vocabulary is incomplete or unknown: {mode}"
            ));
        }
        for (name, color) in roles {
            if !valid_opaque_hex(color) {
                return Err(format!(
                    "compiled Theme role must be opaque sRGB: {mode}/{name}"
                ));
            }
        }
        normalized.insert(mode.into(), Value::Object(roles.clone()));
    }
    for category in ["neutral_base", "status_base"] {
        for (name, modes) in overrides[category]
            .as_object()
            .ok_or("invalid Theme color overrides")?
        {
            for mode in ["light", "dark", "light-ic", "dark-ic"] {
                if role_maps[mode][name] != modes[mode] {
                    return Err(format!(
                        "compiled role map omits declared base input: {name}/{mode}"
                    ));
                }
            }
        }
    }
    // Rust validates the complete role map and derives Surface packets without
    // inferring Themes from Gallery palettes. Color generation is independently
    // recomputed by the offline design check; ordinary builds bind artifacts to
    // the reviewed source lock.
    Ok(Value::Object(normalized))
}

fn theme_provenance(
    contract: &Value,
    overrides: &Value,
    config_path: &str,
    config_bytes: &[u8],
) -> Result<Value, String> {
    Ok(serde_json::json!({
        "contract_version": contract["contract_version"],
        "contract_sha256": sha256_hex(&canonical_json_bytes(contract)?),
        "overrides": overrides,
        "brand_config": config_path,
        "brand_config_sha256": sha256_hex(config_bytes),
    }))
}

fn canonical_compact_bytes(value: &Value) -> Result<Vec<u8>, String> {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut normalized = serde_json::Map::new();
                let mut keys = map.keys().collect::<Vec<_>>();
                keys.sort();
                for key in keys {
                    normalized.insert(key.clone(), sorted(&map[key]));
                }
                Value::Object(normalized)
            }
            Value::Array(values) => Value::Array(values.iter().map(sorted).collect()),
            value => value.clone(),
        }
    }
    serde_json::to_vec(&sorted(value)).map_err(|e| e.to_string())
}

fn is_sha256(value: &Value) -> bool {
    value.as_str().is_some_and(|hex| {
        hex.len() == 64
            && hex.is_ascii()
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn validate_reviewed_artifact_lock(
    lock_bytes: &[u8],
    manifest_bytes: &[u8],
    surface_bytes: &[u8],
    definitions_bytes: &[u8],
    contract: &Value,
    instances: &Value,
) -> Result<Value, String> {
    let lock: Value = serde_json::from_slice(lock_bytes)
        .map_err(|error| format!("invalid Theme artifact lock: {error}"))?;
    object(
        &lock,
        &[
            "format",
            "instance_format",
            "contract_sha256",
            "definition_source_sha256",
            "source_inputs_sha256",
            "source_inputs_sha256_digest",
            "instances_sha256",
            "surface_rules_sha256",
            "themes",
        ],
    )?;
    if lock["format"] != "tekesui.theme-artifact-lock/1"
        || lock["instance_format"] != "tekesui.theme-instances/2"
        || instances["format"] != lock["instance_format"]
    {
        return Err("unsupported or stale Theme artifact lock format".into());
    }
    for field in [
        "contract_sha256",
        "definition_source_sha256",
        "source_inputs_sha256_digest",
        "instances_sha256",
        "surface_rules_sha256",
    ] {
        if !is_sha256(&lock[field]) {
            return Err(format!("invalid Theme artifact lock digest: {field}"));
        }
    }
    if lock["contract_sha256"] != sha256_hex(&canonical_compact_bytes(contract)?)
        || lock["definition_source_sha256"] != sha256_hex(definitions_bytes)
        || lock["instances_sha256"] != sha256_hex(manifest_bytes)
        || lock["surface_rules_sha256"] != sha256_hex(surface_bytes)
    {
        return Err("Theme artifact does not match its reviewed source lock".into());
    }
    let inputs = instances["inputs_sha256"]
        .as_object()
        .ok_or("missing Theme input hashes")?;
    if lock["source_inputs_sha256"] != instances["inputs_sha256"]
        || lock["source_inputs_sha256_digest"]
            != sha256_hex(&canonical_compact_bytes(&instances["inputs_sha256"])?)
    {
        return Err("Theme source inputs differ from the reviewed artifact lock".into());
    }
    if inputs.is_empty() {
        return Err("empty Theme source input identity".into());
    }
    let generated_instances = instances["instances"]
        .as_array()
        .ok_or("missing generated Theme instances")?;
    let mut themes = Vec::with_capacity(generated_instances.len());
    for instance in generated_instances {
        themes.push(serde_json::json!({
            "id": instance["id"],
            "version": instance["version"],
            "schema_version": instance["schema_version"],
            "brand_config": instance["brand_config"],
        }));
    }
    if lock["themes"] != Value::Array(themes) {
        return Err("Theme definitions differ from the reviewed artifact lock".into());
    }
    Ok(lock)
}

fn theme_config_paths(definitions: &Value) -> Result<BTreeSet<String>, String> {
    let definitions = definitions
        .as_array()
        .ok_or("Theme definitions must be an array")?;
    if definitions.is_empty() {
        return Err("Theme definitions cannot be empty".into());
    }
    // The projection's selected BrandConfig remains a variable input even
    // when no Theme definition references it directly.
    let mut paths = BTreeSet::from([
        "design/theme/themes.json".to_owned(),
        "design/brand/config.json".to_owned(),
    ]);
    for definition in definitions {
        object(
            definition,
            &["schema_version", "id", "version", "brand_config", "inputs"],
        )?;
        let config = definition["brand_config"]
            .as_str()
            .ok_or("Theme definition is missing brand_config")?;
        if !paths.insert(config.to_owned()) && config != "design/theme/themes.json" {
            // Reusing one BrandConfig across definitions is permitted.
            continue;
        }
        if config == "design/theme/themes.json" {
            return Err("Theme BrandConfig cannot alias its definitions file".into());
        }
    }
    Ok(paths)
}

fn validate_consumer_theme_source_identity(
    package_inputs: &Value,
    artifact_inputs: &Value,
    package_definitions: &Value,
    artifact_definitions: &Value,
) -> Result<(), String> {
    let package_map = package_inputs
        .as_object()
        .ok_or("package Theme input receipt is missing")?;
    let artifact_map = artifact_inputs
        .as_object()
        .ok_or("consumer Theme input receipt is missing")?;
    let package_variable = theme_config_paths(package_definitions)?;
    let artifact_variable = theme_config_paths(artifact_definitions)?;
    if package_variable
        .iter()
        .any(|path| !package_map.contains_key(path))
        || artifact_variable
            .iter()
            .any(|path| !artifact_map.contains_key(path))
    {
        return Err("Theme definition or BrandConfig is absent from source inputs".into());
    }
    let package_shared = package_map
        .keys()
        .filter(|path| !package_variable.contains(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let artifact_shared = artifact_map
        .keys()
        .filter(|path| !artifact_variable.contains(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    if package_shared != artifact_shared {
        return Err("consumer Theme artifact changes the shared source input set".into());
    }
    for path in package_shared {
        if package_map.get(&path) != artifact_map.get(&path) {
            return Err(format!(
                "consumer Theme artifact changes a shared source input: {path}"
            ));
        }
    }
    Ok(())
}

fn expanded_profiles(rules: &Value) -> Result<serde_json::Map<String, Value>, String> {
    fn expand(
        name: &str,
        all: &serde_json::Map<String, Value>,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, String> {
        if !stack.insert(name.into()) {
            return Err(format!("cyclic shared profile: {name}"));
        }
        let raw = all
            .get(name)
            .ok_or_else(|| format!("unknown profile: {name}"))?;
        let result = if let Some(parent) = raw.get("extends") {
            let map = raw.as_object().ok_or("invalid profile")?;
            if map
                .keys()
                .any(|k| !["extends", "overrides", "state_overrides"].contains(&k.as_str()))
            {
                return Err("unknown inherited profile property".into());
            }
            let mut states = expand(parent.as_str().ok_or("invalid profile parent")?, all, stack)?;
            let states_map = states.as_object_mut().ok_or("invalid base profile")?;
            if let Some(overrides) = raw.get("overrides") {
                let overrides = overrides
                    .as_object()
                    .filter(|m| !m.is_empty())
                    .ok_or("invalid overrides")?;
                if overrides.keys().any(|k| !CHANNELS.contains(&k.as_str())) {
                    return Err("unknown override channel".into());
                }
                for pair in states_map.values_mut() {
                    pair.as_object_mut()
                        .ok_or("invalid base state channels")?
                        .extend(overrides.clone());
                }
            }
            if let Some(overrides) = raw.get("state_overrides") {
                for (state, values) in overrides.as_object().ok_or("invalid state overrides")? {
                    let values = values
                        .as_object()
                        .filter(|m| !m.is_empty())
                        .ok_or("invalid state override")?;
                    if values.keys().any(|k| !CHANNELS.contains(&k.as_str())) {
                        return Err("unknown override channel".into());
                    }
                    states_map
                        .get_mut(state)
                        .ok_or("override of inapplicable state")?
                        .as_object_mut()
                        .ok_or("invalid base state channels")?
                        .extend(values.clone());
                }
            }
            states
        } else {
            raw.clone()
        };
        stack.remove(name);
        Ok(result)
    }
    let all = rules["profiles"]
        .as_object()
        .filter(|m| !m.is_empty())
        .ok_or("missing profiles")?;
    all.keys()
        .map(|name| Ok((name.clone(), expand(name, all, &mut BTreeSet::new())?)))
        .collect()
}
/// Compile complete shared role references; unknown and incomplete definitions fail.
/// Surface names are explicit identities; their RGB values are never inspected.
pub fn compile_surface_rules(rules: &Value) -> Result<String, String> {
    object(
        rules,
        &[
            "format",
            "schema_version",
            "rules_version",
            "modes",
            "surfaces",
            "profiles",
        ],
    )?;
    if rules["format"] != "tekesui.theme-rules/1"
        || rules["schema_version"] != "1.2.0"
        || rules["rules_version"] != "1.6.1"
        || rules["modes"] != serde_json::json!(["light", "dark", "light-ic", "dark-ic"])
    {
        return Err("unsupported Theme schema/rules/modes".into());
    }
    let profiles = expanded_profiles(rules)?;
    let surfaces = rules["surfaces"]
        .as_object()
        .filter(|v| !v.is_empty())
        .ok_or("missing surfaces")?;
    // Reject malformed unused profiles too; no hidden invalid definitions.
    for profile in profiles.values() {
        let states = profile
            .as_object()
            .filter(|v| !v.is_empty())
            .ok_or("empty profile")?;
        if !states.contains_key("rest") || !states.contains_key("disabled") {
            return Err("profile requires rest and disabled".into());
        }
        for (state, pair) in states {
            if !STATES.contains(&state.as_str()) {
                return Err("unknown state".into());
            }
            object(pair, &CHANNELS)?;
            for channel in CHANNELS {
                let value = &pair[channel];
                if !value.is_null() && value.as_str().is_none_or(|s| s.is_empty()) {
                    return Err("invalid role".into());
                }
                if matches!(channel, "background" | "text" | "icon") && value.is_null() {
                    return Err("missing visible channel".into());
                }
            }
            if state.contains("disabled") && !pair["focus"].is_null() {
                return Err("disabled focus forbidden".into());
            }
        }
    }
    let names = surfaces
        .keys()
        .map(|name| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!(
        "// Generated by scripts/generate-theme-system.py; do not edit.\nconst SURFACE_NAMES: &[&str] = &[{names}];\nfn surface_roles(surface: &str, usage: &str, state: &str) -> Option<[Option<&'static str>; 7]> {{\n    Some(match (surface, usage, state) {{\n"
    );
    for (surface, definition) in surfaces {
        if surface.is_empty()
            || !surface
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("invalid Surface identity".into());
        }
        object(definition, &["background", "usages"])?;
        let background = definition["background"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("missing Surface background")?;
        let usages = object(&definition["usages"], &USES)?;
        for (usage, name) in usages {
            let profile = profiles
                .get(name.as_str().ok_or("invalid profile reference")?)
                .ok_or("unknown profile")?
                .as_object()
                .ok_or("invalid profile")?;
            let expected: &[&str] = match usage.as_str() {
                "content"
                | "text-selection"
                | "content-selectable"
                | "surface-frame"
                | "content-status-info"
                | "content-status-running"
                | "content-status-succeeded"
                | "content-status-failed" => &["rest", "disabled"],
                "link" | "range-track" | "range-thumb" => &STATES[..4],
                _ => &STATES,
            };
            if profile.keys().map(String::as_str).collect::<BTreeSet<_>>()
                != expected.iter().copied().collect()
            {
                return Err(format!("incomplete states for {surface}/{usage}"));
            }
            for (state, pair) in profile {
                if usage == "surface-frame"
                    && (pair["boundary"].is_null()
                        || ["focus", "marker", "helper"]
                            .iter()
                            .any(|k| !pair[k].is_null()))
                {
                    return Err(
                        "surface frame requires boundary and forbids focus/marker/helper".into(),
                    );
                }
                if usage == "range-track" && state != "disabled" && pair["boundary"].is_null() {
                    return Err("enabled range track requires a visible boundary".into());
                }
                validate_use_channels(usage, state, pair)?;
                let values = CHANNELS
                    .map(|k| match pair[k].as_str() {
                        None => "None".into(),
                        Some("$surface") => format!("Some({background:?})"),
                        Some(role) => format!("Some({role:?})"),
                    })
                    .join(", ");
                out.push_str(&format!(
                    "        ({surface:?}, {usage:?}, {state:?}) => [{values}],\n"
                ));
            }
        }
    }
    out.push_str("        _ => return None,\n    })\n}\n");
    Ok(out)
}
/// Pure-Rust source-build validation of the reviewed Theme artifact, contract,
/// role maps and Surface packets. Returns Cargo watch paths.
pub fn verify_theme_system(root: &Path, tokens: &Value) -> Result<Vec<PathBuf>, String> {
    let read = |p: &str| fs::read(root.join(p)).map_err(|e| format!("{p}: {e}"));
    let system: Value = serde_json::from_slice(&read_safe_file(root, "design/system.json")?)
        .map_err(|error| format!("design/system.json: {error}"))?;
    if system["theme_system"]["reviewed_artifact_lock"]
        != serde_json::json!({"$ref":"./theme/artifact.lock.json"})
        || system["theme_system"]["artifact_lock_schema"]
            != serde_json::json!({"$ref":"./theme/artifact-lock.schema.json"})
    {
        return Err(
            "Design System must reference the fixed reviewed Theme artifact lock and schema".into(),
        );
    }
    let lock_path = "design/theme/artifact.lock.json";
    let lock_schema_path = "design/theme/artifact-lock.schema.json";
    let lock_schema: Value = serde_json::from_slice(&read_safe_file(root, lock_schema_path)?)
        .map_err(|error| format!("{lock_schema_path}: {error}"))?;
    let lock_fields = [
        "format",
        "instance_format",
        "contract_sha256",
        "definition_source_sha256",
        "source_inputs_sha256",
        "source_inputs_sha256_digest",
        "instances_sha256",
        "surface_rules_sha256",
        "themes",
    ];
    if lock_schema["$schema"].as_str().is_none()
        || lock_schema["type"] != "object"
        || lock_schema["properties"]["format"]["const"] != "tekesui.theme-artifact-lock/1"
        || lock_schema["required"] != serde_json::json!(lock_fields)
        || lock_schema["properties"]
            .as_object()
            .is_none_or(|properties| {
                properties
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>()
                    != lock_fields.iter().copied().collect::<BTreeSet<_>>()
            })
        || lock_schema["additionalProperties"] != false
    {
        return Err("unsupported Theme artifact lock schema".into());
    }
    let rules: Value =
        serde_json::from_slice(&read("design/theme/rules.json")?).map_err(|e| e.to_string())?;
    let input_contract = theme_input_contract(root, &rules)?;
    let generated = compile_surface_rules(&rules)?;
    let output = "design/theme/generated/surface_rules.rs";
    let surface_bytes = read(output)?;
    if surface_bytes != generated.as_bytes() {
        return Err("stale generated Surface bindings".into());
    }
    let manifest = "design/theme/generated/instances.json";
    let manifest_bytes = read(manifest)?;
    let instances: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if instances["theme_input_contract"] != input_contract {
        return Err("generated Theme contract differs from Design System authority".into());
    }
    let definitions_bytes = read("design/theme/themes.json")?;
    let _lock = validate_reviewed_artifact_lock(
        &read_safe_file(root, lock_path)?,
        &manifest_bytes,
        &surface_bytes,
        &definitions_bytes,
        &input_contract,
        &instances,
    )?;
    let hashes = instances["inputs_sha256"]
        .as_object()
        .ok_or("missing Theme input hashes")?;
    let mut paths = vec![root.join(output), root.join(manifest)];
    for required in [
        "design/theme/rules.json",
        "design/theme/rules.schema.json",
        "design/theme/theme.schema.json",
        "design/theme/artifact-lock.schema.json",
        "design/theme/themes.json",
        "design/system.json",
        "design/system.schema.json",
        "design/brand/config.json",
        "scripts/merge-design-system.py",
        "scripts/generate-theme-system.py",
        "design/source/base.tokens.json",
        "design/source/token-metadata.json",
        "design/brand/policy.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand_color.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/validate.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/requirements.txt",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/tokens.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/brand.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/policy.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/quality/contrast-pairs.json",
    ] {
        if !hashes.contains_key(required) {
            return Err(format!("missing Theme input: {required}"));
        }
    }
    for (path, expected) in hashes {
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("unsafe Theme input path".into());
        }
        let actual = sha256_hex(&read_safe_file(root, path)?);
        if expected.as_str() != Some(actual.as_str()) {
            return Err(format!("stale Theme input: {path}"));
        }
        paths.push(root.join(path));
    }
    let names: BTreeSet<_> = tokens["proposal"]["tokens"]["color"]["tokens"]
        .as_array()
        .ok_or("missing compiled color vocabulary")?
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for profile in expanded_profiles(&rules)?.values() {
        for pair in profile.as_object().unwrap().values() {
            for k in CHANNELS {
                if let Some(role) = pair[k].as_str()
                    && role != "$surface"
                    && !names.contains(role)
                {
                    return Err(format!("unknown color role: {role}"));
                }
            }
        }
    }
    for surface in rules["surfaces"].as_object().unwrap().values() {
        if !names.contains(surface["background"].as_str().unwrap()) {
            return Err("unknown Surface background role".into());
        }
    }
    // Check the actual selected palette too, including explicit offline consumer artifacts.
    verify_range_boundaries(&rules, &tokens["proposal"]["tokens"])?;
    verify_surface_foregrounds(&rules, &tokens["proposal"]["tokens"])?;
    let gallery: Value =
        serde_json::from_slice(&read("design/gallery/brands.json")?).map_err(|e| e.to_string())?;
    for preset in gallery["presets"]
        .as_array()
        .ok_or("missing Gallery palettes")?
    {
        verify_range_boundaries(&rules, &serde_json::json!({"color": preset["color"]}))?;
        verify_surface_foregrounds(&rules, &serde_json::json!({"color": preset["color"]}))?;
    }
    verify_instances(root, &rules, &instances, &input_contract)?;
    paths.extend([
        root.join("design/gallery/brands.json"),
        root.join("design/brand/gallery.json"),
        root.join("design/tokens.json"),
        root.join(lock_path),
        root.join(lock_schema_path),
    ]);
    Ok(paths)
}

/// Verify a consumer-owned Theme artifact against this package's immutable
/// shared Theme inputs. The external lock is explicit so an artifact cannot
/// select or replace its own trust anchor. Only versioned definitions and the
/// BrandConfig files named by those definitions may vary from the package.
pub fn verify_consumer_theme_system(
    package_root: &Path,
    artifact_root: &Path,
    lock_path: &Path,
    tokens: &Value,
) -> Result<Vec<PathBuf>, String> {
    if !package_root.is_absolute() || !artifact_root.is_absolute() || !lock_path.is_absolute() {
        return Err("consumer Theme package, artifact, and lock paths must be absolute".into());
    }
    if fs::symlink_metadata(lock_path)
        .map_err(|error| format!("{}: {error}", lock_path.display()))?
        .file_type()
        .is_symlink()
    {
        return Err("consumer Theme source lock must not be a symlink".into());
    }
    let package_root = package_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let artifact_root = artifact_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let lock_path = lock_path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !package_root.is_dir()
        || !artifact_root.is_dir()
        || package_root == artifact_root
        || !lock_path.is_file()
        || lock_path.starts_with(&package_root)
        || lock_path.starts_with(&artifact_root)
    {
        return Err("consumer Theme lock must be an independent source file outside package and artifact roots".into());
    }
    let artifact_tokens: Value =
        serde_json::from_slice(&read_safe_file(&artifact_root, "design/tokens.json")?)
            .map_err(|error| format!("consumer design/tokens.json: {error}"))?;
    if artifact_tokens != *tokens {
        return Err("consumer Theme verifier received a different token document".into());
    }
    let mut watched = crate::verify_consumer_inputs(&package_root, &artifact_root, tokens)
        .map_err(|errors| errors.join("\n"))?;

    // Keep the package's ordinary reviewed-lock validation in force. This also
    // provides the exact shared contract, rules, and source map for comparison.
    watched.extend(verify_theme_system(&package_root, tokens)?);
    let package_manifest_bytes =
        read_safe_file(&package_root, "design/theme/generated/instances.json")?;
    let package_manifest: Value = serde_json::from_slice(&package_manifest_bytes)
        .map_err(|error| format!("package Theme instances: {error}"))?;
    let package_definitions: Value =
        serde_json::from_slice(&read_safe_file(&package_root, "design/theme/themes.json")?)
            .map_err(|error| format!("package Theme definitions: {error}"))?;
    let package_rules_bytes = read_safe_file(&package_root, "design/theme/rules.json")?;
    let rules: Value = serde_json::from_slice(&package_rules_bytes)
        .map_err(|error| format!("package Theme rules: {error}"))?;
    let contract = theme_input_contract(&package_root, &rules)?;

    let artifact_rules_bytes = read_safe_file(&artifact_root, "design/theme/rules.json")?;
    if artifact_rules_bytes != package_rules_bytes {
        return Err("consumer Theme artifact changes shared Surface rules".into());
    }
    let artifact_contract = theme_input_contract(&artifact_root, &rules)?;
    if artifact_contract != contract {
        return Err("consumer Theme artifact changes the shared input contract".into());
    }
    let surface_path = "design/theme/generated/surface_rules.rs";
    let surface_bytes = read_safe_file(&artifact_root, surface_path)?;
    if surface_bytes != compile_surface_rules(&rules)?.as_bytes() {
        return Err("consumer Theme artifact has stale Surface bindings".into());
    }
    let manifest_path = "design/theme/generated/instances.json";
    let manifest_bytes = read_safe_file(&artifact_root, manifest_path)?;
    let instances: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("consumer Theme instances: {error}"))?;
    if instances["theme_input_contract"] != contract {
        return Err("consumer Theme artifact has stale contract provenance".into());
    }
    let definitions_path = "design/theme/themes.json";
    let definitions_bytes = read_safe_file(&artifact_root, definitions_path)?;
    let definitions: Value = serde_json::from_slice(&definitions_bytes)
        .map_err(|error| format!("consumer Theme definitions: {error}"))?;
    validate_consumer_theme_source_identity(
        &package_manifest["inputs_sha256"],
        &instances["inputs_sha256"],
        &package_definitions,
        &definitions,
    )?;
    let lock_bytes =
        fs::read(&lock_path).map_err(|error| format!("{}: {error}", lock_path.display()))?;
    validate_reviewed_artifact_lock(
        &lock_bytes,
        &manifest_bytes,
        &surface_bytes,
        &definitions_bytes,
        &contract,
        &instances,
    )?;
    let artifact_inputs = instances["inputs_sha256"]
        .as_object()
        .ok_or("consumer Theme input receipt is missing")?;
    let mut expected_paths = Vec::with_capacity(artifact_inputs.len());
    for (relative, expected) in artifact_inputs {
        let actual = sha256_hex(&read_safe_file(&artifact_root, relative)?);
        if expected.as_str() != Some(actual.as_str()) {
            return Err(format!("stale consumer Theme source input: {relative}"));
        }
        expected_paths.push(artifact_root.join(relative));
    }
    verify_instances(&artifact_root, &rules, &instances, &contract)?;
    expected_paths.extend([
        artifact_root.join(surface_path),
        artifact_root.join(manifest_path),
        artifact_root.join(definitions_path),
        lock_path.clone(),
    ]);
    watched.extend(expected_paths);
    watched.sort();
    watched.dedup();
    Ok(watched)
}

fn resolved(tokens: &Value, role: &str, mode: &str, depth: usize) -> Result<Value, String> {
    if depth > 64 {
        return Err("cyclic color reference".into());
    }
    let all = tokens["color"]["tokens"]
        .as_array()
        .ok_or("missing colors")?;
    let value = &all
        .iter()
        .find(|t| t["name"] == role)
        .ok_or_else(|| format!("unknown role {role}"))?["value"];
    let value = if value.is_object() {
        &value[mode]
    } else {
        value
    };
    let text = value.as_str().ok_or("incomplete color mode")?;
    if let Some(alias) = text.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        resolved(tokens, alias, mode, depth + 1)
    } else {
        Ok(value.clone())
    }
}

fn color_luminance(color: &Value) -> Result<f64, String> {
    let hex = color
        .as_str()
        .filter(|s| s.len() == 7 && s.starts_with('#') && s.is_ascii())
        .ok_or("Surface contrast requires opaque sRGB colors")?;
    let mut linear = [0.; 3];
    for (i, value) in linear.iter_mut().enumerate() {
        let c = f64::from(
            u8::from_str_radix(&hex[1 + i * 2..3 + i * 2], 16)
                .map_err(|_| "invalid Surface contrast color")?,
        ) / 255.;
        *value = if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        };
    }
    Ok(linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722)
}

/// Mirror the shared generator's foreground constraints in the ordinary build,
/// even when an author has consistently regenerated every output and hash.
fn verify_surface_foregrounds(rules: &Value, palette: &Value) -> Result<(), String> {
    let profiles = expanded_profiles(rules)?;
    for mode in ["light", "dark", "light-ic", "dark-ic"] {
        let mut cache = std::collections::BTreeMap::<String, f64>::new();
        let mut luminance = |role: &str| -> Result<f64, String> {
            if let Some(value) = cache.get(role) {
                return Ok(*value);
            }
            let value = color_luminance(&resolved(palette, role, mode, 0)?)?;
            cache.insert(role.to_owned(), value);
            Ok(value)
        };
        for (surface, definition) in rules["surfaces"].as_object().ok_or("missing surfaces")? {
            let parent_role = definition["background"]
                .as_str()
                .ok_or("missing parent ground")?;
            let parent = luminance(parent_role)?;
            for (usage, profile) in definition["usages"].as_object().ok_or("missing usages")? {
                let states = profiles[profile.as_str().ok_or("invalid profile")?]
                    .as_object()
                    .ok_or("missing profile")?;
                for (state, pair) in states {
                    if state.contains("disabled") {
                        continue;
                    }
                    let role = |channel: &str| -> Result<&str, String> {
                        let role = pair[channel].as_str().ok_or("missing visible foreground")?;
                        Ok(if role == "$surface" {
                            parent_role
                        } else {
                            role
                        })
                    };
                    let background = luminance(role("background")?)?;
                    for channel in ["text", "icon", "marker", "helper", "focus"] {
                        if pair[channel].is_null() {
                            continue;
                        }
                        let ink = luminance(role(channel)?)?;
                        let ground = if channel == "helper" || channel == "focus" {
                            parent
                        } else {
                            background
                        };
                        let minimum = if channel == "marker" || channel == "focus" {
                            3.0
                        } else {
                            4.5
                        };
                        if (ink.max(ground) + 0.05) / (ink.min(ground) + 0.05) < minimum {
                            return Err(format!(
                                "insufficient Surface foreground contrast: {mode}/{surface}/{usage}/{state}/{channel}"
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// A necessary range rail must remain visible against both its own fill and parent.
/// This is a compiler constraint, never an RGB-to-Surface identity inference.
fn verify_range_boundaries(rules: &Value, palette: &Value) -> Result<(), String> {
    let profiles = expanded_profiles(rules)?;
    for mode in ["light", "dark", "light-ic", "dark-ic"] {
        for (surface, definition) in rules["surfaces"].as_object().ok_or("missing surfaces")? {
            let parent = definition["background"]
                .as_str()
                .ok_or("missing parent ground")?;
            let profile = definition["usages"]["range-track"]
                .as_str()
                .ok_or("missing range track")?;
            for (state, pair) in profiles[profile]
                .as_object()
                .ok_or("missing range profile")?
            {
                if state == "disabled" {
                    continue;
                }
                let luminance = |channel: &str| -> Result<f64, String> {
                    let role = pair[channel]
                        .as_str()
                        .ok_or("missing required range boundary")?;
                    color_luminance(&resolved(
                        palette,
                        if role == "$surface" { parent } else { role },
                        mode,
                        0,
                    )?)
                };
                let edge = luminance("boundary")?;
                for ground in [
                    luminance("background")?,
                    color_luminance(&resolved(palette, parent, mode, 0)?)?,
                ] {
                    if (edge.max(ground) + 0.05) / (edge.min(ground) + 0.05) < 3.0 {
                        return Err(format!(
                            "insufficient range boundary contrast: {mode}/{surface}/{state}"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
/// Theme instances are full outputs from the existing BrandConfig compiler;
/// Palette consumes their complete role maps as its color inputs. Rust validates
/// the reviewed lock and reconstructs every Surface packet without duplicating
/// the BrandConfig numerical generation algorithm.
fn verify_instances(
    root: &Path,
    rules: &Value,
    output: &Value,
    contract: &Value,
) -> Result<(), String> {
    let load = |p: &str| -> Result<Value, String> {
        serde_json::from_slice(&read_safe_file(root, p)?).map_err(|e| e.to_string())
    };
    object(
        output,
        &[
            "format",
            "schema_version",
            "rules_version",
            "inputs_sha256",
            "theme_input_contract",
            "instances",
        ],
    )?;
    if output["format"] != "tekesui.theme-instances/2"
        || output["schema_version"] != rules["schema_version"]
        || output["rules_version"] != rules["rules_version"]
        || output["theme_input_contract"] != *contract
    {
        return Err("incompatible generated instance format".into());
    }
    let definitions = load("design/theme/themes.json")?;
    let definitions = definitions.as_array().ok_or("missing theme definitions")?;
    let instances = output["instances"]
        .as_array()
        .ok_or("missing generated instances")?;
    if definitions.len() != instances.len() || definitions.is_empty() {
        return Err("incomplete generated Theme instances".into());
    }
    let mut expected_input_paths = [
        "design/system.json",
        "design/system.schema.json",
        "design/brand/config.json",
        "scripts/merge-design-system.py",
        "design/theme/rules.json",
        "design/theme/rules.schema.json",
        "design/theme/theme.schema.json",
        "design/theme/artifact-lock.schema.json",
        "design/theme/themes.json",
        "scripts/generate-theme-system.py",
        "design/source/base.tokens.json",
        "design/source/token-metadata.json",
        "design/brand/policy.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/brand_color.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/tools/validate.py",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/requirements.txt",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/tokens/tokens.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/brand.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/brand/policy.schema.json",
        "design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/quality/contrast-pairs.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    for definition in definitions {
        expected_input_paths.insert(
            definition["brand_config"]
                .as_str()
                .ok_or("missing Theme config")?
                .to_owned(),
        );
    }
    let actual_input_paths = output["inputs_sha256"]
        .as_object()
        .ok_or("missing Theme input hashes")?
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_input_paths != expected_input_paths {
        return Err("Theme generation receipt has missing or unknown source paths".into());
    }
    let profiles = expanded_profiles(rules)?;
    let mut identities = BTreeSet::new();
    for (definition, instance) in definitions.iter().zip(instances) {
        object(
            definition,
            &["schema_version", "id", "version", "brand_config", "inputs"],
        )?;
        object(
            instance,
            &[
                "schema_version",
                "id",
                "version",
                "brand_config",
                "inputs",
                "brand",
                "compiled_color_roles",
                "provenance",
                "modes",
            ],
        )?;
        let id = definition["id"].as_str().ok_or("missing Theme id")?;
        let version = definition["version"]
            .as_str()
            .ok_or("missing Theme definition version")?;
        if definition["schema_version"] != "2.0.0"
            || !id.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            || !id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || version.split('.').count() != 3
            || !version
                .split('.')
                .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("unsupported or invalid Theme definition".into());
        }
        if !identities.insert(id) {
            return Err("duplicate Theme id".into());
        }
        for key in ["schema_version", "id", "version", "brand_config", "inputs"] {
            if instance[key] != definition[key] {
                return Err("stale Theme definition".into());
            }
        }
        let config_path = definition["brand_config"]
            .as_str()
            .ok_or("missing Theme config")?;
        if !output["inputs_sha256"]
            .as_object()
            .unwrap()
            .contains_key(config_path)
        {
            return Err("missing Theme config receipt".into());
        }
        let config = load(config_path)?;
        if instance["brand"] != config {
            return Err("stale instance BrandConfig".into());
        }
        object(
            &definition["inputs"],
            &["contract_version", "shared_contract_ref", "overrides"],
        )?;
        if definition["inputs"]["contract_version"] != contract["contract_version"]
            || definition["inputs"]["shared_contract_ref"]
                != "design/system.json#/theme_system/theme_input_contract"
        {
            return Err("Theme definition has an unknown or stale input contract reference".into());
        }
        let overrides = &definition["inputs"]["overrides"];
        object(overrides, &["neutral_base", "status_base"])?;
        let config_bytes = read_safe_file(root, config_path)?;
        let expected_provenance =
            theme_provenance(contract, overrides, config_path, &config_bytes)?;
        if instance["provenance"] != expected_provenance {
            return Err(format!("modified or stale Theme input provenance: {id}"));
        }
        let base = load("design/source/base.tokens.json")?;
        let metadata = load("design/source/token-metadata.json")?;
        validate_theme_overrides(overrides, contract, &base, &metadata)?;
        let role_maps = validate_compiled_roles(&instance["compiled_color_roles"], overrides)?;
        let role_names = role_maps["light"]
            .as_object()
            .ok_or("invalid compiled Theme role map")?
            .keys()
            .collect::<Vec<_>>();
        let mut effective_tokens = Vec::with_capacity(role_names.len());
        for name in role_names {
            let mut values = serde_json::Map::new();
            for mode in ["light", "dark", "light-ic", "dark-ic"] {
                values.insert(mode.into(), role_maps[mode][name.as_str()].clone());
            }
            effective_tokens.push(serde_json::json!({"name":name,"value":values}));
        }
        let effective_roles = serde_json::json!({"color":{"tokens":effective_tokens}});
        verify_range_boundaries(rules, &effective_roles)?;
        verify_surface_foregrounds(rules, &effective_roles)?;
        let mut expected_modes = serde_json::Map::new();
        for mode in ["light", "dark", "light-ic", "dark-ic"] {
            let mut surfaces = serde_json::Map::new();
            for (surface, definition) in rules["surfaces"].as_object().unwrap() {
                let mut usages = serde_json::Map::new();
                for (usage, profile) in definition["usages"].as_object().unwrap() {
                    let mut states = serde_json::Map::new();
                    for (state, pair) in profiles[profile.as_str().unwrap()].as_object().unwrap() {
                        let mut colors = serde_json::Map::new();
                        for k in CHANNELS {
                            let color = match pair[k].as_str() {
                                None => Value::Null,
                                Some(role) => resolved(
                                    &effective_roles,
                                    if role == "$surface" {
                                        definition["background"].as_str().unwrap()
                                    } else {
                                        role
                                    },
                                    mode,
                                    0,
                                )?,
                            };
                            colors.insert(k.into(), color);
                        }
                        states.insert(state.clone(), Value::Object(colors));
                    }
                    usages.insert(usage.clone(), Value::Object(states));
                }
                surfaces.insert(surface.clone(), Value::Object(usages));
            }
            expected_modes.insert(mode.into(), Value::Object(surfaces));
        }
        if instance["modes"] != Value::Object(expected_modes) {
            return Err(format!(
                "modified/incomplete generated Theme instance: {id}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod central_theme_input_tests {
    use super::*;

    fn modes(light: &str, dark: &str, light_ic: &str, dark_ic: &str) -> Value {
        serde_json::json!({
            "light": light,
            "dark": dark,
            "light-ic": light_ic,
            "dark-ic": dark_ic,
        })
    }

    fn contract() -> Value {
        serde_json::json!({
            "neutral_base_allowed_roles": ["p-surface-0"],
            "status_base_allowed_roles": ["p-status-info"],
            "contract_version": "1.0.0"
        })
    }

    fn base_bundle() -> Value {
        serde_json::json!({
            "color": {"tokens": [
                {"name":"p-surface-0", "value": modes("#FFFFFF", "#000000", "#FFFFFF", "#000000")},
                {"name":"p-status-info", "value": modes("#0000FF", "#0000FF", "#0000FF", "#0000FF")},
                {"name":"surface-0", "value":"{p-surface-0}"}
            ]}
        })
    }

    #[test]
    fn theme_inputs_validate_complete_rolemap_and_declared_primitive_values() {
        let base = base_bundle();
        let metadata = serde_json::json!({"tokens": {
            "color:p-surface-0": {"tier":"primitive"},
            "color:p-status-info": {"tier":"primitive"},
            "color:surface-0": {"tier":"semantic"}
        }});
        let overrides = serde_json::json!({
            "neutral_base":{"p-surface-0":modes("#F4F5F3", "#0C0C0A", "#F3F4F0", "#010101")},
            "status_base":{"p-status-info":modes("#2A5D90", "#AAD4FF", "#16416E", "#D2E8FF")}
        });
        validate_theme_overrides(&overrides, &contract(), &base, &metadata).unwrap();
        let roles = serde_json::json!({
            "light": {"p-surface-0":"#F4F5F3","p-status-info":"#2A5D90","surface-0":"#F4F5F3"},
            "dark": {"p-surface-0":"#0C0C0A","p-status-info":"#AAD4FF","surface-0":"#0C0C0A"},
            "light-ic": {"p-surface-0":"#F3F4F0","p-status-info":"#16416E","surface-0":"#F3F4F0"},
            "dark-ic": {"p-surface-0":"#010101","p-status-info":"#D2E8FF","surface-0":"#010101"}
        });
        let checked = validate_compiled_roles(&roles, &overrides).unwrap();
        assert_eq!(checked, roles);
        assert_eq!(roles["dark-ic"]["surface-0"], "#010101");
        assert_eq!(roles["light"]["p-status-info"], "#2A5D90");
        assert!(
            roles["light"]
                .as_object()
                .unwrap()
                .contains_key("p-surface-0")
        );
    }

    #[test]
    fn theme_inputs_reject_unknown_roles_partial_modes_alpha_alias_and_rolemap_corruption() {
        let base = base_bundle();
        let metadata = serde_json::json!({"tokens": {
            "color:p-surface-0": {"tier":"primitive"},
            "color:p-status-info": {"tier":"primitive"},
            "color:surface-0": {"tier":"semantic"}
        }});
        let good = modes("#FFFFFF", "#000000", "#FFFFFF", "#000000");
        let unknown = serde_json::json!({"neutral_base":{"p-unknown":good},"status_base":{}});
        assert!(validate_theme_overrides(&unknown, &contract(), &base, &metadata).is_err());
        let partial = serde_json::json!({"light":"#FFFFFF","dark":"#000000","light-ic":"#FFFFFF"});
        let incomplete =
            serde_json::json!({"neutral_base":{"p-surface-0":partial},"status_base":{}});
        assert!(validate_theme_overrides(&incomplete, &contract(), &base, &metadata).is_err());
        let alpha = serde_json::json!({"neutral_base":{"p-surface-0":modes("#FFFFFF80", "#000000", "#FFFFFF", "#000000")},"status_base":{}});
        assert!(validate_theme_overrides(&alpha, &contract(), &base, &metadata).is_err());
        let alias = serde_json::json!({"neutral_base":{"surface-0":good},"status_base":{}});
        assert!(validate_theme_overrides(&alias, &contract(), &base, &metadata).is_err());
        let mut aliased_base = base.clone();
        aliased_base["color"]["tokens"][0]["value"] = Value::String("{surface-0}".into());
        let valid_name = serde_json::json!({"neutral_base":{"p-surface-0":good},"status_base":{}});
        assert!(
            validate_theme_overrides(&valid_name, &contract(), &aliased_base, &metadata).is_err()
        );
        let invalid_roles = serde_json::json!({
            "light":{"p-surface-0":"#FFFFFF","p-status-info":"#FFFFFF","surface-0":"#FFFFFF","extra":"#FFFFFF"},
            "dark":{"p-surface-0":"#000000","p-status-info":"#FFFFFF","surface-0":"#000000"},
            "light-ic":{"p-surface-0":"#FFFFFF","p-status-info":"#FFFFFF","surface-0":"#FFFFFF"},
            "dark-ic":{"p-surface-0":"#000000","p-status-info":"#FFFFFF","surface-0":"#000000"}
        });
        assert!(
            validate_compiled_roles(
                &invalid_roles,
                &serde_json::json!({"neutral_base":{},"status_base":{}})
            )
            .is_err()
        );
    }

    #[test]
    fn theme_provenance_binds_contract_overrides_and_brand_config_bytes() {
        let contract = serde_json::json!({"contract_version":"1.0.0","a":1,"z":"x"});
        let overrides = serde_json::json!({"neutral_base":{},"status_base":{}});
        let first =
            theme_provenance(&contract, &overrides, "design/brand/config.json", b"brand").unwrap();
        let same =
            theme_provenance(&contract, &overrides, "design/brand/config.json", b"brand").unwrap();
        let changed = theme_provenance(
            &contract,
            &overrides,
            "design/brand/config.json",
            b"changed",
        )
        .unwrap();
        assert_eq!(first, same);
        assert_ne!(first["brand_config_sha256"], changed["brand_config_sha256"]);
        assert_eq!(
            first["contract_sha256"],
            sha256_hex(&canonical_json_bytes(&contract).unwrap())
        );
    }

    #[test]
    fn consumer_theme_source_identity_allows_only_definition_and_named_config_inputs() {
        let package_definitions = serde_json::json!([{
            "schema_version":"2.0.0",
            "id":"default",
            "version":"1.0.0",
            "brand_config":"design/brand/config.json",
            "inputs":{}
        }]);
        let consumer_definitions = serde_json::json!([{
            "schema_version":"2.0.0",
            "id":"acme-blue",
            "version":"3.2.1",
            "brand_config":"consumer/brand/acme-blue.json",
            "inputs":{}
        }]);
        let package_inputs = serde_json::json!({
            "design/system.json":"sys",
            "design/theme/rules.json":"rules",
            "design/theme/themes.json":"package-definitions",
            "design/brand/config.json":"package-config"
        });
        let consumer_inputs = serde_json::json!({
            "design/system.json":"sys",
            "design/theme/rules.json":"rules",
            "design/theme/themes.json":"consumer-definitions",
            "design/brand/config.json":"selected-projection-config",
            "consumer/brand/acme-blue.json":"consumer-config"
        });
        validate_consumer_theme_source_identity(
            &package_inputs,
            &consumer_inputs,
            &package_definitions,
            &consumer_definitions,
        )
        .unwrap();

        let mut changed_rules = consumer_inputs.clone();
        changed_rules["design/theme/rules.json"] = Value::String("attacker-rules".into());
        assert!(
            validate_consumer_theme_source_identity(
                &package_inputs,
                &changed_rules,
                &package_definitions,
                &consumer_definitions,
            )
            .is_err()
        );

        let mut unknown_input = consumer_inputs.clone();
        unknown_input["design/source/base.tokens.json"] = Value::String("changed-base".into());
        assert!(
            validate_consumer_theme_source_identity(
                &package_inputs,
                &unknown_input,
                &package_definitions,
                &consumer_definitions,
            )
            .is_err()
        );

        let missing_config = serde_json::json!({
            "design/system.json":"sys",
            "design/theme/rules.json":"rules",
            "design/theme/themes.json":"consumer-definitions"
        });
        assert!(
            validate_consumer_theme_source_identity(
                &package_inputs,
                &missing_config,
                &package_definitions,
                &consumer_definitions,
            )
            .is_err()
        );
    }

    fn lock_fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Value, Value) {
        let contract = serde_json::json!({"contract_version":"1.0.0","rules_version":"1.6.1"});
        let definitions = br#"{"themes":[{"id":"blue","version":"1.0.0"}]}"#.to_vec();
        let surface = b"generated surface bindings\n".to_vec();
        let instances = serde_json::json!({
            "format":"tekesui.theme-instances/2",
            "inputs_sha256":{"design/system.json":"aabb"},
            "instances":[{"id":"blue","version":"1.0.0","schema_version":"2.0.0","brand_config":"design/brand/config.json","compiled_color_roles":{"light":{"p-text":"#111111"}},"modes":{"light":{"canvas":"#FFFFFF"}}}]
        });
        let manifest = serde_json::to_vec(&instances).unwrap();
        let lock = serde_json::json!({
            "format":"tekesui.theme-artifact-lock/1",
            "instance_format":"tekesui.theme-instances/2",
            "contract_sha256":sha256_hex(&canonical_compact_bytes(&contract).unwrap()),
            "definition_source_sha256":sha256_hex(&definitions),
            "source_inputs_sha256":instances["inputs_sha256"],
            "source_inputs_sha256_digest":sha256_hex(&canonical_compact_bytes(&instances["inputs_sha256"]).unwrap()),
            "instances_sha256":sha256_hex(&manifest),
            "surface_rules_sha256":sha256_hex(&surface),
            "themes":[{"id":"blue","version":"1.0.0","schema_version":"2.0.0","brand_config":"design/brand/config.json"}]
        });
        let lock_bytes = serde_json::to_vec(&lock).unwrap();
        (
            lock_bytes,
            manifest,
            surface,
            definitions,
            contract,
            instances,
        )
    }

    #[test]
    fn reviewed_artifact_lock_binds_contract_inputs_definitions_and_raw_outputs() {
        let (lock, manifest, surface, definitions, contract, instances) = lock_fixture();
        validate_reviewed_artifact_lock(
            &lock,
            &manifest,
            &surface,
            &definitions,
            &contract,
            &instances,
        )
        .unwrap();

        let changed_contract =
            serde_json::json!({"contract_version":"1.0.0","rules_version":"1.6.2"});
        assert!(
            validate_reviewed_artifact_lock(
                &lock,
                &manifest,
                &surface,
                &definitions,
                &changed_contract,
                &instances,
            )
            .is_err()
        );
        assert!(
            validate_reviewed_artifact_lock(
                &lock,
                &manifest,
                &surface,
                b"changed definitions",
                &contract,
                &instances,
            )
            .is_err()
        );
    }

    #[test]
    fn reviewed_artifact_lock_rejects_combined_rolemap_packet_and_receipt_tampering() {
        let (lock, _manifest, surface, definitions, contract, mut instances) = lock_fixture();
        instances["instances"][0]["compiled_color_roles"]["light"]["p-text"] =
            Value::String("#FFFFFF".into());
        instances["instances"][0]["modes"]["light"]["canvas"] = Value::String("#000000".into());
        instances["inputs_sha256"]["design/system.json"] = Value::String("ccdd".into());
        let tampered_manifest = serde_json::to_vec(&instances).unwrap();
        assert!(
            validate_reviewed_artifact_lock(
                &lock,
                &tampered_manifest,
                &surface,
                &definitions,
                &contract,
                &instances,
            )
            .is_err()
        );

        let mut tampered_lock: Value = serde_json::from_slice(&lock).unwrap();
        tampered_lock["source_inputs_sha256"] = instances["inputs_sha256"].clone();
        tampered_lock["source_inputs_sha256_digest"] = Value::String(
            sha256_hex(&canonical_compact_bytes(&instances["inputs_sha256"]).unwrap()),
        );
        let tampered_lock_bytes = serde_json::to_vec(&tampered_lock).unwrap();
        assert!(
            validate_reviewed_artifact_lock(
                &tampered_lock_bytes,
                &tampered_manifest,
                &surface,
                &definitions,
                &contract,
                &instances,
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod range_boundary_tests {
    use super::*;
    fn rules() -> Value {
        serde_json::json!({"profiles":{"track":{"rest":{"background":"rail","boundary":"edge"},"disabled":{"background":"rail","boundary":"rail"}}},"surfaces":{"canvas":{"background":"parent","usages":{"range-track":"track"}}}})
    }
    fn palette(parent: &str, rail: &str, edge: &str) -> Value {
        serde_json::json!({"color":{"tokens":[{"name":"parent","value":parent},{"name":"rail","value":rail},{"name":"edge","value":edge}]}})
    }
    #[test]
    fn normal_build_rejects_missing_required_range_boundary() {
        let mut r: Value =
            serde_json::from_str(include_str!("../../../design/theme/rules.json")).unwrap();
        r["profiles"]["range-track"]["overrides"]["boundary"] = Value::Null;
        assert!(
            compile_surface_rules(&r)
                .unwrap_err()
                .contains("requires a visible boundary")
        );
    }
    #[test]
    fn normal_build_checks_boundary_against_both_rail_and_parent() {
        let r = rules();
        for p in [
            palette("#FFFFFF", "#FFFFFF", "#FFFFFF"),
            palette("#666666", "#FFFFFF", "#666666"),
        ] {
            assert!(
                verify_range_boundaries(&r, &p)
                    .unwrap_err()
                    .contains("insufficient range boundary contrast")
            );
        }
    }
    #[test]
    fn active_boundary_passes_without_imposing_enabled_contrast_on_disabled() {
        assert!(
            verify_range_boundaries(&rules(), &palette("#FFFFFF", "#FFFFFF", "#666666")).is_ok()
        );
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    fn rules() -> Value {
        serde_json::from_str(include_str!("../../../design/theme/rules.json")).unwrap()
    }
    #[test]
    fn normal_compiler_rejects_missing_frame_use_and_unsupported_states() {
        let mut r = rules();
        r["surfaces"]["canvas"]["usages"]
            .as_object_mut()
            .unwrap()
            .remove("surface-frame");
        assert!(compile_surface_rules(&r).is_err());
        let mut r = rules();
        r["surfaces"]["canvas"]["usages"]["surface-frame"] = Value::from("quiet");
        assert!(
            compile_surface_rules(&r)
                .unwrap_err()
                .contains("incomplete states")
        );
    }
    #[test]
    fn normal_compiler_rejects_missing_edge_and_invented_frame_indicators() {
        for (key, value) in [
            ("boundary", Value::Null),
            ("focus", Value::from("border-focused")),
            ("marker", Value::from("primary-text")),
            ("helper", Value::from("primary-text")),
        ] {
            let mut r = rules();
            if key == "focus" {
                r["profiles"]["surface-frame"]["state_overrides"] =
                    serde_json::json!({"rest":{"focus":value}});
            } else {
                r["profiles"]["surface-frame"]["overrides"][key] = value;
            }
            assert!(
                compile_surface_rules(&r)
                    .unwrap_err()
                    .contains("surface frame requires boundary")
            );
        }
    }
}

#[cfg(test)]
mod foreground_gate_tests {
    use super::*;
    fn rules() -> Value {
        let pair = serde_json::json!({"background":"ground","text":"ink","icon":"ink","boundary":null,"focus":null,"marker":null,"helper":null});
        serde_json::json!({"profiles":{"input":{"rest":pair.clone(),"disabled":pair}},"surfaces":{"canvas":{"background":"parent","usages":{"input":"input"}}}})
    }
    fn palette(parent: &str, ink: &str) -> Value {
        serde_json::json!({"color":{"tokens":[{"name":"ground","value":"#FFFFFF"},{"name":"parent","value":parent},{"name":"ink","value":ink}]}})
    }
    #[test]
    fn consistently_known_low_contrast_text_icon_and_marker_are_rejected() {
        let r = rules();
        assert!(
            verify_surface_foregrounds(&r, &palette("#FFFFFF", "#FFFFFF"))
                .unwrap_err()
                .ends_with("/text")
        );
        for channel in ["icon", "marker"] {
            let mut r = rules();
            r["profiles"]["input"]["rest"][channel] = Value::from("ground");
            assert!(
                verify_surface_foregrounds(&r, &palette("#FFFFFF", "#111111"))
                    .unwrap_err()
                    .ends_with(&format!("/{channel}"))
            );
        }
    }
    #[test]
    fn helper_contrast_uses_parent_instead_of_input_ground() {
        let mut r = rules();
        r["profiles"]["input"]["rest"]["helper"] = Value::from("parent");
        assert!(
            verify_surface_foregrounds(&r, &palette("#333333", "#111111"))
                .unwrap_err()
                .ends_with("/helper")
        );
    }
    #[test]
    fn valid_active_foregrounds_and_existing_disabled_exception_pass() {
        let mut r = rules();
        r["profiles"]["input"]["disabled"]["text"] = Value::from("ground");
        r["profiles"]["input"]["disabled"]["icon"] = Value::from("ground");
        assert!(verify_surface_foregrounds(&r, &palette("#FFFFFF", "#111111")).is_ok());
    }
}

#[cfg(test)]
mod required_channel_tests {
    use super::*;
    fn rules() -> Value {
        serde_json::from_str(include_str!("../../../design/theme/rules.json")).unwrap()
    }
    fn set_rest(r: &mut Value, p: &str, k: &str, value: Value) {
        if r["profiles"][p].get("extends").is_some() {
            r["profiles"][p]["state_overrides"]["rest"][k] = value;
        } else {
            r["profiles"][p]["rest"][k] = value;
        }
    }
    #[test]
    fn focus_owner_and_non_owner_applicability_is_enforced() {
        for p in [
            "quiet",
            "readonly",
            "check-checked",
            "toggle-track",
            "range-thumb",
            "link",
            "content-selectable",
            "selection",
        ] {
            let mut r = rules();
            set_rest(&mut r, p, "focus", Value::Null);
            assert!(compile_surface_rules(&r).unwrap_err().contains("focus"));
        }
        for p in [
            "content",
            "text-selection",
            "control-label",
            "toggle-thumb",
            "range-track",
        ] {
            let mut r = rules();
            set_rest(&mut r, p, "focus", Value::from("border-focused"));
            assert!(compile_surface_rules(&r).unwrap_err().contains("focus"));
        }
    }
    #[test]
    fn required_marker_and_helper_parts_cannot_compile_as_null() {
        for (usage, channel) in [
            ("range-track", "marker"),
            ("check-checked", "marker"),
            ("check-mixed", "marker"),
            ("input", "helper"),
            ("input-error", "helper"),
            ("input-readonly", "helper"),
        ] {
            let mut r = rules();
            let base = r["surfaces"]["canvas"]["usages"][usage].clone();
            r["profiles"]["missing-part"] =
                serde_json::json!({"extends":base,"state_overrides":{"rest":{}}});
            r["profiles"]["missing-part"]["state_overrides"]["rest"][channel] = Value::Null;
            r["surfaces"]["canvas"]["usages"][usage] = Value::from("missing-part");
            assert!(
                compile_surface_rules(&r)
                    .unwrap_err()
                    .contains("missing required")
            );
        }
    }
    #[test]
    fn focus_contrast_is_against_the_parent() {
        let pair = serde_json::json!({"background":"ground","text":"ink","icon":"ink","boundary":null,"focus":"parent","marker":null,"helper":null});
        let r = serde_json::json!({"profiles":{"input":{"rest":pair}},"surfaces":{"canvas":{"background":"parent","usages":{"input":"input"}}}});
        let p = serde_json::json!({"color":{"tokens":[{"name":"ground","value":"#FFFFFF"},{"name":"ink","value":"#111111"},{"name":"parent","value":"#FFFFFF"}]}});
        assert!(
            verify_surface_foregrounds(&r, &p)
                .unwrap_err()
                .ends_with("/focus")
        );
    }
}

#[cfg(test)]
mod status_content_tests {
    use super::*;
    fn rules() -> Value {
        serde_json::from_str(include_str!("../../../design/theme/rules.json")).unwrap()
    }
    #[test]
    fn passive_status_requires_all_named_uses_and_only_rest_disabled() {
        for tone in ["info", "running", "succeeded", "failed"] {
            let usage = format!("content-status-{tone}");
            let mut r = rules();
            r["surfaces"]["canvas"]["usages"]
                .as_object_mut()
                .unwrap()
                .remove(&usage);
            assert!(compile_surface_rules(&r).is_err());
            let mut r = rules();
            r["surfaces"]["canvas"]["usages"][&usage] = Value::from("quiet");
            assert!(
                compile_surface_rules(&r)
                    .unwrap_err()
                    .contains("incomplete states")
            );
        }
    }
    #[test]
    fn passive_status_rejects_own_ground_and_action_indicators() {
        for (key, value) in [
            ("background", "surface-0"),
            ("boundary", "separator"),
            ("focus", "border-focused"),
            ("marker", "primary-text"),
            ("helper", "primary-text"),
        ] {
            let mut r = rules();
            r["profiles"]["content-status-info"]["state_overrides"]["rest"][key] =
                Value::from(value);
            assert!(
                compile_surface_rules(&r)
                    .unwrap_err()
                    .contains("passive status content requires parent ground")
            );
        }
    }
}

#[cfg(test)]
mod link_anatomy_tests {
    use super::*;
    #[test]
    fn link_rejects_invented_marker_helper_in_every_original_state() {
        for state in ["rest", "hover", "pressed", "disabled"] {
            for channel in ["marker", "helper"] {
                let mut r: Value =
                    serde_json::from_str(include_str!("../../../design/theme/rules.json")).unwrap();
                r["profiles"]["link"][state][channel] = Value::from("primary-text");
                assert!(
                    compile_surface_rules(&r)
                        .unwrap_err()
                        .contains("Link has no marker/helper anatomy")
                );
            }
        }
    }
}
