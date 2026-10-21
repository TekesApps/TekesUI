//! Measures the real component update, including full caret maps and drawing.
use tekes_ui_app_protocol::{InputKind, Request, VERSION};
use tekes_ui_panel_module_demo::definitions;
use tekes_ui_scene::{GlyphAtlas, Scene};
fn main() {
    for definition in definitions() {
        let mut module = (definition.create)();
        let mut atlas = GlyphAtlas::default();
        let mut forward = Vec::new();
        let mut backward = Vec::new();
        for i in 0..120 {
            let request = Request {
                version: VERSION,
                seq: i,
                viewport: i,
                kind: InputKind::Layout,
                command: None,
                x: 0.,
                y: 0.,
                width: 240 + (if i < 60 { i } else { 119 - i }) as u32 * 5,
                height: 584,
                identity: 0,
                scale: 2.,
            };
            atlas.begin_frame();
            let mut scene = Scene::new();
            let start = std::time::Instant::now();
            module.update(&request, &mut scene, &mut atlas).unwrap();
            if i >= 20 {
                if i < 60 {
                    forward.push(start.elapsed().as_secs_f64() * 1000.);
                } else {
                    backward.push(start.elapsed().as_secs_f64() * 1000.);
                }
            }
        }
        for (phase, mut times) in [("new-width", forward), ("return", backward)] {
            times.sort_by(f64::total_cmp);
            println!(
                "{} {phase} component update median={:.3}ms p95={:.3}ms",
                definition.id,
                times[times.len() / 2],
                times[times.len() * 95 / 100]
            );
        }
    }
}
