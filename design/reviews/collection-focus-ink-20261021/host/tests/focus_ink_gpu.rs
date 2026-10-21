include!("../../../collection-focus-availability-20261020/host/src/text_backend.rs");
include!("../../../collection-focus-availability-20261020/host/src/fixture.rs");
use tekes_ui_render::Renderer;
use tekes_ui_tokens::{Palette, Theme};
fn pixels(
    gpu: &mut Renderer,
    text: &mut TextBackend,
    story: &mut ExtendedStory,
    f: Family,
    width: f32,
    scale: f32,
    palette: Palette,
) -> (Vec<u8>, Option<tekes_ui_scene::Quad>, Scene) {
    text.atlas.begin_frame();
    let mut scene = Scene::new();
    story.paint(f, &mut scene, text, width, palette);
    let focus = scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            tekes_ui_scene::Primitive::Quad(q)
                if q.fill.a == 0.0
                    && q.border_width == proposal::metric("stroke", "c-focus-width").unwrap()
                    && q.border_color == palette.color("border-focused").unwrap() =>
            {
                Some(*q)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(focus.len() <= 1, "no duplicate focus paint");
    let rgba = gpu
        .render_to_rgba8(
            &scene,
            &mut text.atlas,
            ((width * scale) as u32, (220. * scale) as u32),
            scale,
            palette.color("surface-1").unwrap(),
        )
        .unwrap();
    (rgba, focus.first().copied(), scene)
}
#[test]
fn canonical_focus_ink_matches_full_reference_without_changing_geometry_or_body() {
    let out =
        std::path::PathBuf::from(std::env::var_os("TEKESUI_COLLECTION_FOCUS_GPU_OUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let mut gpu = Renderer::new_headless().unwrap();
    let mut cases = Vec::new();
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for (width, scale) in [(180., 1.), (300., 2.)] {
                for type_scale in [1., 1.5] {
                    for family in [
                        Family::List,
                        Family::Tree,
                        Family::TableRow,
                        Family::TableHeader,
                        Family::Chart,
                    ] {
                        for disabled in [false, true] {
                            if family == Family::Chart && disabled {
                                continue;
                            }
                            let mut story = ExtendedStory::new();
                            let mut text = TextBackend::new(scale, type_scale);
                            text.clock = Some(std::time::Duration::ZERO);
                            let palette = Palette { theme, brand };
                            let (baseline, _, rest_scene) = pixels(
                                &mut gpu, &mut text, &mut story, family, width, scale, palette,
                            );
                            if disabled {
                                story.disabled(family, true);
                            } else {
                                story.active(family, false);
                            }
                            story.focus(family);
                            if disabled {
                                story.disabled(family, false);
                            } else {
                                story.active(family, true);
                            }
                            let (restored, _, _) = pixels(
                                &mut gpu, &mut text, &mut story, family, width, scale, palette,
                            );
                            assert_eq!(
                                restored, baseline,
                                "{family:?} late focus must not leave paint after restore"
                            );
                            assert_eq!(story.key(family), E::Ignored);
                            story.focus(family);
                            let (fresh, focus_quad, _) = pixels(
                                &mut gpu, &mut text, &mut story, family, width, scale, palette,
                            );
                            assert_ne!(
                                fresh, baseline,
                                "{family:?} fresh keyboard focus must be visible"
                            );
                            let file_name = format!(
                                "t{ti}-b{brand}-w{width}-s{type_scale}-{family:?}-disabled{disabled}.png"
                            );
                            let reference_root = std::path::PathBuf::from(
                                std::env::var_os("TEKESUI_FOCUS_INK_BEFORE").unwrap(),
                            );
                            let mut reader = png::Decoder::new(
                                std::fs::File::open(reference_root.join(&file_name)).unwrap(),
                            )
                            .read_info()
                            .unwrap();
                            let mut old = vec![0; reader.output_buffer_size()];
                            let info = reader.next_frame(&mut old).unwrap();
                            old.truncate(info.buffer_size());
                            assert_eq!(old.len(), fresh.len());
                            let changed = old
                                .chunks_exact(4)
                                .zip(fresh.chunks_exact(4))
                                .filter(|(a, b)| a != b)
                                .count();
                            if family == Family::Chart {
                                assert_eq!(
                                    fresh, old,
                                    "Chart already owns internal focus margins; no scope/style change"
                                );
                            } else {
                                assert!(changed > 0, "actual prior clipped output must change");
                                let mut reference = rest_scene.clone();
                                reference.push_quad(focus_quad.expect("canonical full focus quad"));
                                let expected = gpu
                                    .render_to_rgba8(
                                        &reference,
                                        &mut text.atlas,
                                        ((width * scale) as u32, (220. * scale) as u32),
                                        scale,
                                        palette.color("surface-1").unwrap(),
                                    )
                                    .unwrap();
                                assert_eq!(
                                    fresh, expected,
                                    "complete final focus must equal unfocused original component + unobscured canonical focus quad"
                                );
                            }
                            let result = story.key(family);
                            assert_ne!(result, E::Ignored);
                            story.select(family);
                            let mut encoder=png::Encoder::new(std::fs::File::create(out.join(format!("t{ti}-b{brand}-w{width}-s{type_scale}-{family:?}-disabled{disabled}.png"))).unwrap(),(width*scale) as u32,(220.*scale) as u32);
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            encoder
                                .write_header()
                                .unwrap()
                                .write_image_data(&fresh)
                                .unwrap();
                            cases.push(serde_json::json!({"component":family.component(),"target":format!("{family:?}"),"renderer":family.renderer(),"theme":ti,"brand":brand,"width":width,"scale":scale,"type_scale":type_scale,"disabled":disabled,"restored_pixels_equal":true,"late_key_ignored":true,"fresh_focus_pixels_differ":true,"fresh_key":format!("{result:?}"),"before_after_changed_pixels":changed,"canonical_unoccluded_reference_equal":family!=Family::Chart,"chart_unchanged_exception":family==Family::Chart}));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 432);
    std::fs::write(out.parent().unwrap().join("gpu-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"frames":1680,"scope":"Current public Views/exact Gallery focus adapter/CoreText/GPU; actual before-after images and canonical full-ink composition. Scope is focus appearance/cancel only, not full native/Gold7/ClientKit parity."})).unwrap()).unwrap();
}
