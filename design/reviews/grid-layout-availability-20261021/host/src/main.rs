include!("../../../collection-focus-native-20261021/host/src/text_backend.rs");
use std::path::PathBuf;
use tekes_ui_controls::{
    Button, ButtonSize, extended::Grid, layout_view::GridView, paint::ButtonVariant,
};
use tekes_ui_tokens::{Palette, Theme};
fn main() {
    let out = PathBuf::from(std::env::var_os("TEKESUI_GRID_REVIEW_OUT").expect("explicit output"));
    std::fs::create_dir_all(&out).unwrap();
    let before = std::env::var("TEKESUI_GRID_REVIEW_PHASE").as_deref() == Ok("before");
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cases = Vec::new();
    let mut observations = Vec::new();
    let mut frames = 0;
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for width in [240., 480.] {
                for scale in [1., 2.] {
                    for empty in [false, true] {
                        let palette = Palette { theme, brand };
                        let mut text = TextBackend::new(scale, 1.);
                        text.atlas.begin_frame();
                        let mut grid = GridView::new(Grid {
                            columns: 2,
                            gap_x: 4.,
                            gap_y: 4.,
                        });
                        let bounds = Rect::new(8., 8., width - 16., if empty { 0. } else { 20. });
                        #[cfg(feature = "token-audit")]
                        let (layout, reads) = tekes_ui_tokens::audit::capture(|| {
                            grid.render(bounds, 4, 40., None).clone()
                        });
                        #[cfg(not(feature = "token-audit"))]
                        let (layout, reads) = (
                            grid.render(bounds, 4, 40., None).clone(),
                            Vec::<(String, String)>::new(),
                        );
                        let visible = layout
                            .items
                            .iter()
                            .filter(|c| c.visible_frame().is_some())
                            .count();
                        let expected = if before || !empty { 4 } else { 0 };
                        assert_eq!(visible, expected);
                        let mut scene = Scene::new();
                        for (i, c) in layout.items.iter().enumerate() {
                            if let Some(clip) = c.visible_frame() {
                                Button::default().render(
                                    &mut scene,
                                    &mut text,
                                    &format!("Item {} · 子项", i + 1),
                                    c.frame(),
                                    Some(clip),
                                    palette,
                                    ButtonVariant::Quiet,
                                    ButtonSize::Body,
                                    proposal::TextRole::UiBody,
                                    ButtonTextAlign::Center,
                                );
                            }
                        }
                        let size = ((width * scale) as u32, (112. * scale) as u32);
                        let clear = palette.color("surface-1").unwrap();
                        let pixels = gpu
                            .render_to_rgba8(&scene, &mut text.atlas, size, scale, clear)
                            .unwrap();
                        frames += 1;
                        let file = format!("t{ti}-b{brand}-w{width}-d{scale}-empty{empty}.png");
                        let mut encoder = png::Encoder::new(
                            std::fs::File::create(out.join(&file)).unwrap(),
                            size.0,
                            size.1,
                        );
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&pixels)
                            .unwrap();
                        let mut changed = 0;
                        if !before {
                            let mut reader = png::Decoder::new(
                                std::fs::File::open(
                                    out.parent().unwrap().join("before-gpu").join(&file),
                                )
                                .unwrap(),
                            )
                            .read_info()
                            .unwrap();
                            let mut old = vec![0; reader.output_buffer_size()];
                            let info = reader.next_frame(&mut old).unwrap();
                            old.truncate(info.buffer_size());
                            assert_eq!(old.len(), pixels.len());
                            changed = old
                                .chunks_exact(4)
                                .zip(pixels.chunks_exact(4))
                                .filter(|(a, b)| a != b)
                                .count();
                            if empty {
                                let blank = gpu
                                    .render_to_rgba8(
                                        &Scene::new(),
                                        &mut text.atlas,
                                        size,
                                        scale,
                                        clear,
                                    )
                                    .unwrap();
                                frames += 1;
                                assert_eq!(pixels, blank);
                                assert!(changed > 0, "actual prior leaked children must disappear");
                            } else {
                                assert_eq!(
                                    changed, 0,
                                    "positive natural overflow must be identical"
                                );
                            }
                        }
                        cases.push(serde_json::json!({"theme":ti,"brand":brand,"width":width,"scale":scale,"empty":empty,"visible_children":visible,"before_after_changed_pixels":changed,"image":file}));
                        observations.push(serde_json::json!({"component":"tui.grid","content_state":"rest","scenario":if empty {"zero-area"}else{"positive-natural-overflow"},"state_asserted":!before,"theme":format!("{theme:?}"),"brand":brand,"width":width,"scale":scale,"lookups":reads,"no_dynamic_lookups":true,"zero_lookup_reason":"GridView publishes geometry only and resolves no dynamic tokens; real themed child Button rendering is outside the captured GridView scope."}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 96);
    std::fs::write(out.parent().unwrap().join(if before {"before-gpu-result.json"}else{"gpu-result.json"}),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"frames":frames,"phase":if before {"before"}else{"after"},"scope":"actual GridView and real Button/CoreText/Metal consumer; empty availability vs preserved positive overflow"})).unwrap()).unwrap();
    if !before {
        std::fs::write(out.parent().unwrap().join("grid-token-raw.json"),serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","scope":"GridView geometry-only capture with actual four-theme Button consumers outside the scope; zero-lookups do not describe child paint dependencies.","complete":false,"excluded":["child Button paint dependencies","all caller geometry/identity combinations","native and whole-page/ClientKit experience"],"cases":observations})).unwrap()).unwrap();
    }
}
