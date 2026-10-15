use serde_json::Value;

pub(crate) fn validate(
    root: &Value,
    schema: &Value,
    value: &Value,
    path: &str,
    errors: &mut Vec<String>,
) {
    if let Some(options) = schema["oneOf"].as_array() {
        let accepted = options
            .iter()
            .filter(|option| {
                let mut trial = Vec::new();
                validate(root, option, value, path, &mut trial);
                trial.is_empty()
            })
            .count();
        if accepted != 1 {
            errors.push(format!(
                "{path}: expected exactly one supported value/expression shape"
            ));
        }
        return;
    }
    if let Some(reference) = schema["$ref"].as_str() {
        match reference
            .strip_prefix('#')
            .and_then(|pointer| root.pointer(pointer))
        {
            Some(target) => validate(root, target, value, path, errors),
            None => errors.push(format!("{path}: unresolved schema reference {reference}")),
        }
        return;
    }
    if let Some(expected) = schema.get("const")
        && expected != value
    {
        errors.push(format!("{path}: expected constant {expected}"));
    }
    let valid_type = match schema["type"].as_str() {
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("number") => value.is_number(),
        Some("integer") => value.as_f64().is_some_and(|number| number.fract() == 0.0),
        None => true,
        _ => false,
    };
    if !valid_type {
        errors.push(format!("{path}: wrong type"));
        return;
    }
    if let Some(properties) = schema["properties"].as_object()
        && let Some(values) = value.as_object()
    {
        if let Some(required) = schema["required"].as_array() {
            for key in required.iter().filter_map(Value::as_str) {
                if !values.contains_key(key) {
                    errors.push(format!("{path}/{key}: missing required value"));
                }
            }
        }
        for (key, item) in values {
            if let Some(shape) = properties.get(key) {
                validate(root, shape, item, &format!("{path}/{key}"), errors);
            } else if schema["additionalProperties"] == false {
                errors.push(format!("{path}/{key}: unexpected value"));
            }
        }
    }
    if let Some(items) = value.as_array() {
        for bound in ["minItems", "maxItems"] {
            if let Some(limit) = schema[bound].as_u64()
                && ((bound == "minItems" && items.len() < limit as usize)
                    || (bound == "maxItems" && items.len() > limit as usize))
            {
                errors.push(format!("{path}: array length violates {bound}"));
            }
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                validate(root, item_schema, item, &format!("{path}/{index}"), errors);
            }
        }
    }
    if let Some(number) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|min| number < min)
            || schema["maximum"].as_f64().is_some_and(|max| number > max))
    {
        errors.push(format!("{path}: number outside schema bounds"));
    }
    if let Some(text) = value.as_str()
        && (schema["minLength"]
            .as_u64()
            .is_some_and(|min| text.chars().count() < min as usize)
            || schema["maxLength"]
                .as_u64()
                .is_some_and(|max| text.chars().count() > max as usize))
    {
        errors.push(format!("{path}: string length outside schema bounds"));
    }
}

pub fn canonical_json(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("finite token JSON");
    text.push('\n');
    text
}
