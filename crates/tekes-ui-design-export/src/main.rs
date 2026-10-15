use std::{env, fs, process};
use tekes_ui_design_export::{
    canonical_json, check_generated, export_schema, generate_rust, numeric_channels,
    resolved_document, validate_document, verify_inputs,
};
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [flag, input, rest @ ..] = args.as_slice() else {
        return Err("Usage: tekes-ui-design-export --validate <tokens.json> | --generate <tokens.json> <out.rs> | --check <tokens.json> <out.rs> | --write-schema <tokens.json> <schema.json> | --resolved <tokens.json> <out.json> | --channels <tokens.json> <out.txt>".into());
    };
    let text = fs::read_to_string(input).map_err(|e| e.to_string())?;
    let d = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    verify_inputs(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        &d,
    )
    .map_err(|e| e.join("\n"))?;
    match (flag.as_str(), rest) {
        ("--validate", []) => validate_document(&d).map_err(|e| e.join("\n")),
        ("--resolved", [path]) => fs::write(
            path,
            canonical_json(&resolved_document(&d).map_err(|e| e.join("\n"))?),
        )
        .map_err(|e| e.to_string()),
        ("--channels", [path]) => fs::write(
            path,
            numeric_channels(&d)
                .map_err(|e| e.join("\n"))?
                .into_iter()
                .map(|(path, bits)| format!("{path} {bits:08x}\n"))
                .collect::<String>(),
        )
        .map_err(|e| e.to_string()),
        ("--generate", [path]) => {
            fs::write(path, generate_rust(&d).map_err(|e| e.join("\n"))?).map_err(|e| e.to_string())
        }
        ("--check", [path]) => {
            check_generated(&d, &fs::read_to_string(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.join("\n"))
        }
        ("--write-schema", [path]) => fs::write(
            path,
            canonical_json(&export_schema(&d).map_err(|e| e.join("\n"))?),
        )
        .map_err(|e| e.to_string()),
        _ => Err("invalid arguments (no output was updated)".into()),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
