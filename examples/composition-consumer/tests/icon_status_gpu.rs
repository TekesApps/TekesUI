#![cfg(all(target_os = "macos", feature = "svg-icons"))]
//! Actual canonical SVG pixels and bounded image identity through semantic edits.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{composition::*, extended::StatusBadge, layout_view::IconRenderer};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::resolve_surface};
#[test]
fn passive_status_real_svg_pixels_match_original_asset_oracle_and_disabled_variants_share() {
    let tones = [
        (StatusBadge::Info, "content-status-info"),
        (StatusBadge::Running, "content-status-running"),
        (StatusBadge::Succeeded, "content-status-succeeded"),
        (StatusBadge::Failed, "content-status-failed"),
    ];
    let mut cases = vec![];
    let output = std::env::var_os("TEKESUI_ICON_STATUS_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        assert!(!p.exists());
        std::fs::create_dir_all(p).unwrap();
    }
    for (mode, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in [0, 1, 2] {
            for backing in [1., 2.] {
                let palette = Palette::new(theme, brand).unwrap();
                let ground = resolve_surface(palette, "canvas", "content", "rest")
                    .unwrap()
                    .background;
                let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
                let mut cache = SvgIconCache::default();
                let mut text = coretext::Text::new(1.);
                text.backing = backing;
                let mut host = ContentHost::new(Content::new(
                    ComponentId(1),
                    IconContent::new("tekes-back")
                        .unwrap()
                        .with_accessible_name("Result"),
                ))
                .unwrap();
                let mount = host.semantic_mount(ComponentId(1));
                let f = Rect::new(0., 0., 32., 32.);
                let mut disabled_id = None;
                for disabled in [false, true] {
                    for (tone, usage) in tones {
                        let icon = host
                            .get_mut(ComponentId(1))
                            .unwrap()
                            .component_mut::<IconContent>()
                            .unwrap();
                        icon.set_status_tone(Some(tone));
                        icon.set_disabled(disabled);
                        let mut actual = Scene::new();
                        {
                            let mut service = cache.renderer(&mut gpu, backing);
                            host.render(
                                &mut actual,
                                &mut WithIconRenderer::new(&mut text, &mut service),
                                f,
                                f,
                                palette,
                            )
                            .unwrap();
                        }
                        let (id, rect) = actual
                            .primitives()
                            .iter()
                            .find_map(|p| {
                                if let Primitive::Image(i) = p {
                                    Some((i.image, i.rect))
                                } else {
                                    None
                                }
                            })
                            .unwrap();
                        let rgba = gpu
                            .render_to_rgba8(
                                &actual,
                                &mut text.atlas,
                                ((32. * backing) as u32, (32. * backing) as u32),
                                backing,
                                ground,
                            )
                            .unwrap();
                        assert_eq!(gpu.image_count(), 1);
                        assert_eq!(host.semantic_mount(ComponentId(1)), mount);
                        assert!(host.focus_targets().is_empty());
                        let packet = resolve_surface(
                            palette,
                            "canvas",
                            usage,
                            if disabled { "disabled" } else { "rest" },
                        )
                        .unwrap();
                        // Resolve the oracle explicitly, with no IconContent status routing involved.
                        let mut expected = Scene::new();
                        {
                            let mut service = cache.renderer(&mut gpu, backing);
                            let image = service
                                .prepare_icon("tekes-back", rect.size.width, backing, packet.icon)
                                .unwrap();
                            expected.push_image(tekes_ui_scene::ImageQuad {
                                rect,
                                image: image.id(),
                                opacity: 1.,
                            });
                        }
                        let oracle = gpu
                            .render_to_rgba8(
                                &expected,
                                &mut text.atlas,
                                ((32. * backing) as u32, (32. * backing) as u32),
                                backing,
                                ground,
                            )
                            .unwrap();
                        assert_eq!(rgba, oracle);
                        if disabled {
                            if let Some(old) = disabled_id {
                                assert_eq!(
                                    id, old,
                                    "same resolved disabled pixels must reuse original registration"
                                );
                            } else {
                                disabled_id = Some(id);
                            }
                        }
                        let key = format!(
                            "mode{mode}-brand{brand}-{tone:?}-disabled{disabled}@{backing}x"
                        );
                        if let Some(p) = &output {
                            std::fs::write(p.join(format!("{key}.rgba8")), &rgba).unwrap();
                        }
                        cases.push(serde_json::json!({"case":key,"image":id.0,"registered_images":gpu.image_count(),"width":(32.*backing)as u32,"height":(32.*backing)as u32,"actual_RGBA_equals_explicit_resource_oracle":true}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 192);
    if let Some(p) = output {
        std::fs::write(p.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"actual_GPU_frames":384,"component_and_explicit_asset_oracle_pairs":192,"physical_or_native":false,"full_Gold7":false})).unwrap()).unwrap();
    }
}
