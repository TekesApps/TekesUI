#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{composition::*, extended::StatusBadge, feedback_view::StatusBadgeView};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{
    Palette, Theme,
    surface_palette::{resolve_surface, surface_names},
};
#[test]
#[ignore = "explicit acceptance: requires Theme selection and fresh SSD output"]
fn actual_gpu_original_bare_labelled_view_and_mounted_content() {
    let output = std::env::var_os("TEKESUI_BADGE_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &output {
        assert!(!p.exists());
        std::fs::create_dir_all(p).unwrap();
    }
    let mut gpu = tekes_ui_render::Renderer::new_headless().expect("actual GPU required");
    let mut rows = vec![];
    for (mi, mode) in Theme::ALL.into_iter().enumerate() {
        let theme_set = std::env::var("TEKESUI_BADGE_THEME_SET")
            .expect("set TEKESUI_BADGE_THEME_SET to default or external");
        let palettes: Vec<(String, Palette)> = match theme_set.as_str() {
            "default" => (0..3)
                .map(|brand| {
                    (
                        format!("gallery-brand{brand}"),
                        Palette::new(mode, brand).expect("registered Gallery brand"),
                    )
                })
                .collect(),
            "external" => ["blue", "blue-inputs"]
                .into_iter()
                .map(|id| {
                    (
                        id.to_string(),
                        Palette::for_theme_definition(mode, id)
                            .expect("selected external Theme definition must exist"),
                    )
                })
                .collect(),
            other => panic!("unsupported TEKESUI_BADGE_THEME_SET: {other}"),
        };
        for (brand, p) in palettes {
            for surface in surface_names() {
                for (tone, usage, label) in [
                    (StatusBadge::Info, "status-badge-info", "Info"),
                    (StatusBadge::Running, "status-badge-running", "Running"),
                    (
                        StatusBadge::Succeeded,
                        "status-badge-succeeded",
                        "Succeeded",
                    ),
                    (StatusBadge::Failed, "status-badge-failed", "Failed"),
                ] {
                    for scale in [1., 2.] {
                        for (variant, w, h, text_scale, label, partial) in [
                            ("normal", 124., 28., 1., label, false),
                            ("narrow-cjk", 27.5, 60., 1.5, "状态·未验证", false),
                            (
                                "fractional-clip",
                                98.25,
                                36.5,
                                1.5,
                                "e\u{301} · 未验证",
                                true,
                            ),
                        ] {
                            let mut text = coretext::Text::new(text_scale);
                            text.backing = scale;
                            text.atlas.begin_frame();
                            let f = Rect::new(12.25, 14.5, w, h);
                            let clip = if partial {
                                Rect::new(20.5, 16.25, 68.75, 29.5)
                            } else {
                                Rect::new(0., 0., 160., 100.)
                            };
                            let ground = resolve_surface(p, surface, "content", "rest")
                                .unwrap()
                                .background;
                            let packet = resolve_surface(p, surface, usage, "rest").unwrap();
                            let size = ((160. * scale) as u32, (100. * scale) as u32);
                            let blank = gpu
                                .render_to_rgba8(
                                    &Scene::new(),
                                    &mut text.atlas,
                                    size,
                                    scale,
                                    ground,
                                )
                                .unwrap();
                            let mut view = StatusBadgeView::default();
                            view.set_surface(surface).unwrap();
                            let mut bare = Scene::new();
                            view.render(&mut bare, tone, f, Some(clip), p);
                            let solid = gpu
                                .render_to_rgba8(&bare, &mut text.atlas, size, scale, ground)
                                .unwrap();
                            let mut original = Scene::new();
                            view.render_labelled(
                                &mut original,
                                &mut text,
                                tone,
                                label,
                                f,
                                f,
                                Some(clip),
                                p,
                            )
                            .unwrap();
                            let labelled = gpu
                                .render_to_rgba8(&original, &mut text.atlas, size, scale, ground)
                                .unwrap();
                            let id = ComponentId(76);
                            let mut host = ContentHost::new(
                                Content::new(id, StatusBadgeContent::new(tone, label).unwrap())
                                    .with_surface(surface)
                                    .unwrap(),
                            )
                            .unwrap();
                            let mount = host.semantic_mount(id);
                            let mut actual = Scene::new();
                            host.render(&mut actual, &mut text, f, clip, p).unwrap();
                            let rgba = gpu
                                .render_to_rgba8(&actual, &mut text.atlas, size, scale, ground)
                                .unwrap();
                            let key =
                                format!("m{mi}-{brand}-{surface}-{tone:?}-{variant}@{scale}x");
                            assert_eq!(
                                rgba, labelled,
                                "{key}: Content must match original labelled View pixels"
                            );
                            let fill_pixels = solid
                                .chunks_exact(4)
                                .zip(blank.chunks_exact(4))
                                .filter(|(a, b)| a != b)
                                .count();
                            let text_pixels = rgba
                                .chunks_exact(4)
                                .zip(solid.chunks_exact(4))
                                .filter(|(a, b)| a != b)
                                .count();
                            assert!(fill_pixels > 0, "{key} requires positive solid ink");
                            assert!(text_pixels > 0, "{key} requires positive label ink");
                            let normal_text_on_parent = rgba
                                .chunks_exact(4)
                                .zip(solid.chunks_exact(4))
                                .zip(blank.chunks_exact(4))
                                .filter(|((a, b), c)| a != b && b == c)
                                .count();
                            if variant == "normal" {
                                assert_eq!(
                                    normal_text_on_parent, 0,
                                    "{key}: normal label must remain over the solid"
                                );
                            }
                            let visible = f.intersection(&clip).unwrap();
                            let mut escaped = 0;
                            for (i, (a, b)) in
                                rgba.chunks_exact(4).zip(blank.chunks_exact(4)).enumerate()
                            {
                                let x = ((i as u32 % size.0) as f32 + 0.5) / scale;
                                let y = ((i as u32 / size.0) as f32 + 0.5) / scale;
                                // Interior/scissor boundary pixels may antialias. Strictly outside the
                                // physical enclosing clip must remain the original clear pixel.
                                if (x < (visible.min_x() * scale).floor() / scale
                                    || x > (visible.max_x() * scale).ceil() / scale
                                    || y < (visible.min_y() * scale).floor() / scale
                                    || y > (visible.max_y() * scale).ceil() / scale)
                                    && a != b
                                {
                                    escaped += 1;
                                }
                            }
                            assert_eq!(escaped, 0, "{key}: ink outside ancestor/frame clip");
                            assert_eq!(host.semantic_mount(id), mount);
                            assert!(host.focus_targets().is_empty());
                            // Sample the solid's actual opaque interior against the resolved packet;
                            // label comparison above separately demonstrates dedicated foreground ink.
                            let expected = [
                                (packet.background.r * 255.).round() as u8,
                                (packet.background.g * 255.).round() as u8,
                                (packet.background.b * 255.).round() as u8,
                                255,
                            ];
                            assert!(
                                solid.chunks_exact(4).any(|v| v == expected),
                                "{key}: packet solid color must reach GPU"
                            );
                            if let Some(out) = &output
                                && surface == &"canvas"
                                && tone == StatusBadge::Info
                            {
                                std::fs::write(out.join(format!("{key}.rgba8")), &rgba).unwrap();
                            }
                            rows.push(serde_json::json!({"case":key,"width":size.0,"height":size.1,"solid_pixels":fill_pixels,"label_pixels":text_pixels,"outside_ink":escaped,"normal_label_on_parent_pixels":normal_text_on_parent,"whole_label_readability_claim":variant=="normal","content_equals_labelled_View":true,"packet_solid_readback":true}));
                        }
                        // Zero and fully clipped allocations are legal and intentionally ink-free.
                        let mut text = coretext::Text::new(1.);
                        text.backing = scale;
                        for f in [
                            Rect::new(12.25, 14.5, 0., 0.),
                            Rect::new(300., 300., 120., 28.),
                        ] {
                            let mut s = Scene::new();
                            let mut h = ContentHost::new(
                                Content::new(
                                    ComponentId(1),
                                    StatusBadgeContent::new(tone, label).unwrap(),
                                )
                                .with_surface(surface)
                                .unwrap(),
                            )
                            .unwrap();
                            h.render(&mut s, &mut text, f, Rect::new(0., 0., 160., 100.), p)
                                .unwrap();
                            let ground = resolve_surface(p, surface, "content", "rest")
                                .unwrap()
                                .background;
                            let size = ((160. * scale) as u32, (100. * scale) as u32);
                            let a = gpu
                                .render_to_rgba8(&s, &mut text.atlas, size, scale, ground)
                                .unwrap();
                            let b = gpu
                                .render_to_rgba8(
                                    &Scene::new(),
                                    &mut text.atlas,
                                    size,
                                    scale,
                                    ground,
                                )
                                .unwrap();
                            assert_eq!(a, b);
                        }
                    }
                }
            }
        }
    }
    println!(
        "actual GPU positive triplets: {} (plus clear/zero/full-clip controls)",
        rows.len()
    );
    if let Some(out) = output {
        std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"positive_cases":rows.len(),"positive_GPU_frames":rows.len()*4,"cases":rows,"native_window":false,"physical_IME":false})).unwrap()).unwrap();
    }
}
