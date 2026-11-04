//! #12's unrepresented viewport/type pairs and one existing non-default brand.
//! These are real Gallery scenes; every emitted PNG still requires visual review.
use super::{entries, settle_scroll, stabilize_document, theme_slug, write_png};
use crate::gallery::{Gallery, Page, static_text::StaticText};
use tekes_ui_platform_macos::{Event, Modifiers, MouseButton, MouseEvent, MouseEventKind, Point};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Primitive, Scene};
use tekes_ui_tokens::{Palette, Theme};

#[test]
#[ignore = "requires allocated GPU and fresh TEKESUI_VISUAL_REMAINDER_OUT"]
fn settings_complementary_viewports_capture_real_ready_page() {
    let output = fresh_output("settings");
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let mut manifest = Vec::new();
    let index = entries()
        .iter()
        .position(|e| e.slug() == "example-settings")
        .unwrap();
    // A48 already covered narrow/150% and wide/100%. Only these two pairs remain.
    for (width, height, text_scale, type_scale) in [(720, 512, 0, 1.0), (1440, 960, 2, 1.5)] {
        for theme in Theme::ALL {
            let mut gallery = Gallery::with_static_text(StaticText::new(None));
            gallery.set_system_theme(theme);
            gallery.set_reduced_motion(true);
            gallery.text.set_scale(2.);
            gallery.workbench.text_scale = text_scale;
            assert!(gallery.open_route("example-settings/playground"));
            gallery.set_view_size(width as f32, height as f32);
            stabilize_document(&mut gallery);
            assert_eq!(gallery.text.type_scale(), type_scale);
            let range = gallery.page_scroll_range().max(0.);
            for (position, fraction) in [("top", 0.), ("bottom", 1.)] {
                let (offset, actual_range) =
                    settle_scroll(&mut gallery, fraction, range * fraction);
                let mut scene = Scene::new();
                gallery.build_scene(&mut scene);
                assert_eq!(gallery.page, Page::Workbench(index));
                let state = gallery.workbench.state(index);
                assert_eq!(state.phase, super::super::Phase::Ready);
                assert_eq!(state.input.query(), "Studio");
                assert_eq!(state.brand, 0);
                assert!(state.draft_fixture.is_none());
                assert!((offset - actual_range * fraction).abs() < 0.5);
                let file = format!(
                    "settings-{}-{width}x{height}-type{text_scale}-{position}.png",
                    theme_slug(theme)
                );
                capture(
                    &mut renderer,
                    &mut gallery,
                    &scene,
                    &output.join(&file),
                    width,
                    height,
                    theme,
                );
                manifest.push(serde_json::json!({"file":file,"route":"example-settings/playground","brand":0,"theme":theme_slug(theme),"window":[width,height],"type_scale":type_scale,"backing_scale":2,"position":position,"offset":offset,"scroll_range":actual_range,"visual_review":"unverified"}));
            }
        }
    }
    assert_eq!(manifest.len(), 16);
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires allocated GPU and fresh TEKESUI_VISUAL_REMAINDER_OUT"]
fn nondefault_brand_menu_dialog_capture_original_open_layers() {
    let output = fresh_output("brand-overlays");
    let mut renderer = Renderer::new_headless().expect("real GPU required");
    let brand = 1;
    let preset =
        tekes_ui_tokens::palette::gallery_brand_preset(brand).expect("existing registered brand 1");
    let mut manifest = Vec::new();
    for kind in ["menu", "dialog"] {
        let index = entries().iter().position(|e| e.renderer() == kind).unwrap();
        let route = format!("{}/playground", entries()[index].slug());
        for theme in Theme::ALL {
            let mut gallery = Gallery::with_static_text(StaticText::new(None));
            gallery.set_system_theme(theme);
            gallery.set_reduced_motion(true);
            gallery.text.set_scale(2.);
            gallery.workbench.brand = brand;
            assert!(gallery.open_route(&route));
            gallery.set_view_size(1440., 960.);
            stabilize_document(&mut gallery);
            let trigger = gallery.workbench.state(index).buttons[&20]
                .layout()
                .unwrap()
                .frame
                .center();
            for down in [true, false] {
                gallery.handle_input(&Event::Mouse(MouseEvent {
                    position: Point {
                        x: trigger.x as f64,
                        y: trigger.y as f64,
                    },
                    modifiers: Modifiers::NONE,
                    kind: if down {
                        MouseEventKind::Down {
                            button: MouseButton::Left,
                            click_count: 1,
                        }
                    } else {
                        MouseEventKind::Up {
                            button: MouseButton::Left,
                            click_count: 1,
                        }
                    },
                }));
            }
            let mut scene = Scene::new();
            gallery.build_scene(&mut scene);
            let state = gallery.workbench.state(index);
            assert_eq!(state.brand, brand);
            let frame = if kind == "menu" {
                assert!(state.menu_view.is_open());
                state.menu_layout.as_ref().unwrap().popup
            } else {
                assert!(state.dialog_view.is_open());
                state.dialog_layout.as_ref().unwrap().modal
            };
            let palette = Palette::new(theme, brand).unwrap();
            let surface = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == frame => Some(q),
                    _ => None,
                })
                .expect("original overlay surface");
            assert_eq!(surface.fill, palette.color("surface-popover").unwrap());
            assert!(scene.composition_error().is_none());
            let geometry = serde_json::json!({"x":frame.origin.x,"y":frame.origin.y,"width":frame.size.width,"height":frame.size.height,"radius":surface.corner_radii.top_left});
            let file = format!("{kind}-brand{brand}-{}.png", theme_slug(theme));
            capture(
                &mut renderer,
                &mut gallery,
                &scene,
                &output.join(&file),
                1440,
                960,
                theme,
            );
            manifest.push(serde_json::json!({"file":file,"route":route,"brand":brand,"brand_id":preset.id,"brand_seed":preset.seed_srgb,"theme":theme_slug(theme),"window":[1440,960],"type_scale":1,"backing_scale":2,"overlay":geometry,"input":"programmatic original Gallery pointer Down/Up, not native/physical","visual_review":"unverified"}));
        }
    }
    assert_eq!(manifest.len(), 8);
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

fn fresh_output(child: &str) -> std::path::PathBuf {
    let root = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_VISUAL_REMAINDER_OUT").expect("allocated fresh output parent"),
    );
    assert!(root.is_dir());
    let output = root.join(child);
    std::fs::create_dir(&output).expect("refuse to overwrite historical captures");
    output
}

fn capture(
    renderer: &mut Renderer,
    gallery: &mut Gallery,
    scene: &Scene,
    path: &std::path::Path,
    width: u32,
    height: u32,
    theme: Theme,
) {
    let rgba = renderer
        .render_to_rgba8(
            scene,
            gallery.atlas_mut(),
            (width * 2, height * 2),
            2.,
            Gallery::background(theme),
        )
        .expect("real Gallery GPU capture");
    assert!(rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]));
    write_png(path.to_path_buf(), width * 2, height * 2, &rgba);
}
