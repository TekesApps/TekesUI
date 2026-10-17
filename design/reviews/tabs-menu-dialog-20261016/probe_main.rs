#[path = "../gallery/mod.rs"]
pub mod gallery;

use gallery::{Command, Gallery};
use std::{env, fs, path::PathBuf};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

fn png_bytes(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("write PNG header");
        writer.write_image_data(rgba).expect("write PNG data");
    }
    bytes
}

fn capture(
    renderer: &mut Renderer,
    gallery: &mut Gallery,
    output: &PathBuf,
    route_id: &str,
    kind: &str,
    theme: Theme,
    scale: u32,
    text_scale: usize,
) -> serde_json::Value {
    let text_scale_percent = [100, 125, 150][text_scale];
    gallery.prepare(renderer, scale as f32);
    let mut scene = Scene::new();
    gallery.build_scene(&mut scene);
    let logical = (1440_u32, 960_u32);
    let pixels = (logical.0 * scale, logical.1 * scale);
    let rgba = renderer
        .render_to_rgba8(
            &scene,
            gallery.atlas_mut(),
            pixels,
            scale as f32,
            Gallery::background(theme),
        )
        .expect("render actual Gallery scene");
    let state = gallery::workbench::probe::state(gallery, kind);
    match kind {
        "tabs" => assert_eq!(state["selected"].as_u64(), Some(2), "actual Tabs input must select the third tab"),
        "menu" => {
            assert_eq!(state["open"].as_bool(), Some(true), "actual Menu input must leave it open");
            assert_eq!(state["selected"].as_u64(), Some(1), "actual Menu Down input must select item two");
        }
        "select" => assert_eq!(state["open"].as_bool(), Some(true), "actual Select input must open its menu"),
        "dialog" | "danger" => assert_eq!(state["open"].as_bool(), Some(true), "actual button input must open the Dialog"),
        _ => unreachable!(),
    }
    let stem = format!(
        "{}-{}-{}-t{}-{}x",
        route_id.replace('/', "-"),
        if theme.is_dark() { "dark" } else { "light" },
        if theme.is_increased() { "increased" } else { "standard" },
        [100, 125, 150][text_scale],
        scale
    );
    fs::write(output.join(format!("{stem}.png")), png_bytes(&rgba, pixels.0, pixels.1))
        .expect("write PNG");
    serde_json::json!({
        "id": stem,
        "route": route_id,
        "theme": format!("{}-{}", if theme.is_dark() {"dark"} else {"light"}, if theme.is_increased() {"increased"} else {"standard"}),
        "scale": scale,
        "text_scale_percent": text_scale_percent,
        "logical_size": logical,
        "pixel_size": pixels,
        "scene_primitive_count": scene.primitives().len(),
        "state": state,
    })
}

fn run_case(
    renderer: &mut Renderer,
    output: &PathBuf,
    theme: Theme,
    scale: u32,
    route: &str,
    kind: &str,
    text_scale: usize,
) -> serde_json::Value {
    let mut gallery = Gallery::new();
    assert!(gallery.open_route(route), "registered route: {route}");
    gallery.set_view_size(1440.0, 960.0);
    gallery.apply(Command::TextScale(text_scale));
    gallery.set_system_theme(theme);

    // Materialize the real production layout first. The target-only wrapper reads the
    // rendered hit frame, then these events pass through Gallery::handle_input.
    gallery.prepare(renderer, scale as f32);
    gallery.build_scene(&mut Scene::new());

    match kind {
        "tabs" => gallery::workbench::probe::activate_tab(&mut gallery, 2),
        "menu" | "select" | "dialog" | "danger" => {
            gallery::workbench::probe::activate_button(&mut gallery, 20);
            gallery.prepare(renderer, scale as f32);
            gallery.build_scene(&mut Scene::new());
            if kind == "menu" {
                gallery::workbench::probe::press(&mut gallery, 125); // Down selects the second item.
            }
        }
        _ => panic!("unknown case: {kind}"),
    }

    capture(
        renderer,
        &mut gallery,
        output,
        route,
        kind,
        theme,
        scale,
        text_scale,
    )
}

fn main() {
    let mut args = env::args_os().skip(1);
    let output = args
        .next()
        .map(PathBuf::from)
        .expect("usage: offscreen-overlay-probe OUTPUT_DIR [--text-scale 0|1|2]");
    let mut text_scale = 0_usize;
    while let Some(option) = args.next() {
        if option == "--text-scale" {
            let value = args.next().expect("--text-scale value");
            text_scale = value
                .to_string_lossy()
                .parse()
                .expect("--text-scale must be 0, 1, or 2");
            assert!(text_scale <= 2, "--text-scale must be 0, 1, or 2");
        } else {
            panic!("unknown argument: {}", option.to_string_lossy());
        }
    }
    fs::create_dir_all(&output).expect("create output directory");
    let mut renderer = Renderer::new_headless().expect("create headless renderer");
    let cases = [
        ("component-tabs/playground", "tabs"),
        ("component-menu/playground", "menu"),
        ("component-select/playground", "select"),
        ("component-dialog/playground", "dialog"),
        ("pattern-danger/playground", "danger"),
    ];
    let mut receipts = Vec::new();
    for theme in Theme::ALL {
        for scale in [1, 2] {
            for (route, kind) in cases {
                let receipt = run_case(
                    &mut renderer,
                    &output,
                    theme,
                    scale,
                    route,
                    kind,
                    text_scale,
                );
                eprintln!("{}", serde_json::to_string(&receipt).unwrap());
                receipts.push(receipt);
            }
        }
    }
    fs::write(
        output.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&receipts).unwrap()),
    )
    .expect("write receipt manifest");
}

