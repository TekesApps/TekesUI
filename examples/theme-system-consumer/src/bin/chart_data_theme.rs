//! Independent public Theme consumer and data-pair contrast witness.
use serde_json::Value;
use std::path::{Path, PathBuf};
use tekes_ui_tokens::{
    Palette, Rgba, Theme,
    surface_palette::{SurfaceError, resolve_data_plot_surface, resolve_surface, surface_names},
};
fn read(p: &Path) -> Result<Value, String> {
    serde_json::from_slice(&std::fs::read(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
// Independent consumer calculation: no production contrast/luminance helper.
fn luminance(c: Rgba) -> f64 {
    assert_eq!(c.a, 1.);
    let linear = |v: f32| {
        let v = f64::from(v);
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}
fn contrast(a: Rgba, b: Rgba) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn verify(pkg: &Path, art: &Path, lock: &Path) -> Result<(), String> {
    let tokens = read(&art.join("design/tokens.json"))?;
    if pkg == art {
        tekes_ui_design_export::theme_system::verify_theme_system(pkg, &tokens)?;
    } else {
        tekes_ui_design_export::theme_system::verify_consumer_theme_system(
            pkg, art, lock, &tokens,
        )?;
    }
    Ok(())
}
fn main() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("verify") {
        if args.len() != 4 {
            return Err("verify package artifact lock".into());
        }
        verify(Path::new(&args[1]), Path::new(&args[2]), Path::new(&args[3]))?;
        println!("{{\"verified\":true}}");
        return Ok(());
    }
    let pkg = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let art = std::env::var_os("TEKESUI_DESIGN_ARTIFACT")
        .map(PathBuf::from)
        .unwrap_or_else(|| pkg.clone());
    let lock = std::env::var_os("TEKESUI_THEME_LOCK")
        .map(PathBuf::from)
        .unwrap_or_else(|| art.join("design/theme/artifact.lock.json"));
    verify(&pkg, &art, &lock)?;
    let contract = read(&pkg.join("design/theme/data-plot-pairs.json"))?;
    let instances = read(&art.join("design/theme/generated/instances.json"))?;
    let mut checked = 0;
    for definition in instances["instances"]
        .as_array()
        .ok_or("missing definitions")?
    {
        for mode in Theme::ALL {
            let palette = Palette::for_theme_definition(mode, definition["id"].as_str().unwrap())
                .ok_or("missing compiled public Theme")?;
            for surface in surface_names() {
                let p = resolve_data_plot_surface(palette, surface).map_err(|e| format!("{e:?}"))?;
                for pair in contract["pairs"].as_array().unwrap() {
                    if pair["applicability"] != "owner-band" {
                        continue;
                    }
                    let color = |selector: &str| match selector {
                        "parent" => p.plot.background,
                        "plot.text" => p.plot.text,
                        "category.text" => p.category.text,
                        "plot.marker" => p.plot.marker.unwrap(),
                        "plot.focus" => p.plot.focus.unwrap(),
                        "selection.background" => p.selection.background,
                        _ => panic!("unknown finite selector"),
                    };
                    let minimum = pair[if mode.is_increased() {
                        "increased"
                    } else {
                        "normal"
                    }]
                    .as_f64()
                    .unwrap();
                    assert!(
                        contrast(
                            color(pair["ink"].as_str().unwrap()),
                            color(pair["ground"].as_str().unwrap())
                        ) >= minimum,
                        "{} {surface} {:?}",
                        pair["id"],
                        mode
                    );
                    checked += 1;
                }
                for usage in ["data-plot", "data-category-label"] {
                    for state in ["hover", "selected", "disabled", "focus"] {
                        assert_eq!(
                            resolve_surface(palette, surface, usage, state),
                            Err(SurfaceError::UnknownCombination)
                        );
                    }
                }
                assert_eq!(
                    resolve_data_plot_surface(palette, "unknown"),
                    Err(SurfaceError::UnknownCombination)
                );
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"public_data_pairs":checked,"themes":instances["instances"].as_array().unwrap().len()})
    );
    Ok(())
}
