#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, extended::StatusBadge,
};
use tekes_ui_scene::{CornerRadii, Primitive, Rect, Scene};
use tekes_ui_tokens::{
    Palette, Theme,
    surface_palette::{resolve_surface, surface_names},
};
// Reuse the actual emitted glyphs, atlas, layout and clipping. Only the solid
// background is squared to expose otherwise invisible glyphs on the parent.
fn oracle(source: &Scene, square: bool, glyphs: bool) -> Scene {
    let mut result = Scene::new();
    for p in source.primitives() {
        match p {
            Primitive::Glyphs(_) if !glyphs => {}
            Primitive::Quad(q) if square => {
                let mut q = *q;
                q.corner_radii = CornerRadii::ZERO;
                result.push_quad(q);
            }
            _ => result.push(p.clone()),
        }
    }
    result
}
#[test]
#[ignore = "explicit acceptance: requires Theme selection and fresh SSD output"]
fn normal_labels_all_four_tones_have_solid_under_every_actual_glyph_pixel() {
    let out = std::env::var_os("TEKESUI_BADGE_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(p) = &out {
        assert!(!p.exists());
        std::fs::create_dir_all(p).unwrap();
    }
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut rows = vec![];
    let theme_set = std::env::var("TEKESUI_BADGE_THEME_SET")
        .expect("set TEKESUI_BADGE_THEME_SET to default or external");
    for (mi, mode) in Theme::ALL.into_iter().enumerate() {
        let palettes: Vec<(String, Palette)> = match theme_set.as_str() {
            "default" => vec![("default-brand0".into(), mode.into())],
            "external" => {
                let blue = Palette::for_theme_definition(mode, "blue")
                    .expect("selected external Theme definition blue must exist");
                let blue_inputs = Palette::for_theme_definition(mode, "blue-inputs")
                    .expect("selected external Theme definition blue-inputs must exist");
                assert_ne!(
                    blue, blue_inputs,
                    "external definitions must remain distinct"
                );
                vec![("blue".into(), blue), ("blue-inputs".into(), blue_inputs)]
            }
            other => panic!("unsupported TEKESUI_BADGE_THEME_SET: {other}"),
        };
        for (palette_id, p) in palettes {
            for surface in surface_names() {
                for (tone, label) in [
                    (StatusBadge::Info, "Info"),
                    (StatusBadge::Running, "Running"),
                    (StatusBadge::Succeeded, "Succeeded"),
                    (StatusBadge::Failed, "Failed"),
                ] {
                    for scale in [2., 1.] {
                        let mut text = coretext::Text::new(1.);
                        text.backing = scale;
                        let f = Rect::new(12.25, 14.5, 124., 28.);
                        let clip = Rect::new(0., 0., 160., 100.);
                        let mut host = ContentHost::new(
                            Content::new(
                                ComponentId(76),
                                StatusBadgeContent::new(tone, label).unwrap(),
                            )
                            .with_surface(surface)
                            .unwrap(),
                        )
                        .unwrap();
                        let mut s = Scene::new();
                        host.render(&mut s, &mut text, f, clip, p).unwrap();
                        let size = ((160. * scale) as u32, (100. * scale) as u32);
                        let ground = resolve_surface(p, surface, "content", "rest")
                            .unwrap()
                            .background;
                        let actual = gpu
                            .render_to_rgba8(&s, &mut text.atlas, size, scale, ground)
                            .unwrap();
                        let bare = gpu
                            .render_to_rgba8(
                                &oracle(&s, false, false),
                                &mut text.atlas,
                                size,
                                scale,
                                ground,
                            )
                            .unwrap();
                        let square = gpu
                            .render_to_rgba8(
                                &oracle(&s, true, false),
                                &mut text.atlas,
                                size,
                                scale,
                                ground,
                            )
                            .unwrap();
                        let square_label = gpu
                            .render_to_rgba8(
                                &oracle(&s, true, true),
                                &mut text.atlas,
                                size,
                                scale,
                                ground,
                            )
                            .unwrap();
                        let mut glyph_pixels = 0;
                        let mut off_solid = 0;
                        let mut missing_glyph_pixels = 0;
                        for (((a, b), sq), ink) in actual
                            .chunks_exact(4)
                            .zip(bare.chunks_exact(4))
                            .zip(square.chunks_exact(4))
                            .zip(square_label.chunks_exact(4))
                        {
                            if sq != ink {
                                glyph_pixels += 1;
                                if b != sq {
                                    off_solid += 1;
                                }
                                if a != ink {
                                    missing_glyph_pixels += 1;
                                }
                            }
                        }
                        let key = format!("m{mi}-{palette_id}-{surface}-{tone:?}@{scale}x");
                        let row = serde_json::json!({"case":key,"actual_glyph_pixels":glyph_pixels,"glyph_pixels_outside_opaque_solid":off_solid,"missing_or_changed_glyph_pixels":missing_glyph_pixels});
                        if let Some(out) = &out {
                            if surface == &"canvas" {
                                std::fs::write(out.join(format!("{key}.rgba8")), &actual).unwrap();
                            }
                            if off_solid > 0 || missing_glyph_pixels > 0 {
                                std::fs::write(out.join("actual-failure.rgba8"), &actual).unwrap();
                                std::fs::write(
                                    out.join("square-label-failure.rgba8"),
                                    &square_label,
                                )
                                .unwrap();
                                std::fs::write(
                                    out.join("failure.json"),
                                    serde_json::to_vec_pretty(&row).unwrap(),
                                )
                                .unwrap();
                            }
                        }
                        let mut centered = oracle(&s, true, false);
                        text.draw(
                            &mut centered,
                            label,
                            tekes_ui_tokens::proposal::TextRole::UiCaption,
                            f,
                            ButtonTextAlign::Center,
                            tekes_ui_controls::feedback_view::StatusBadgeView::label_color(p),
                        );
                        let centered_rgba = gpu
                            .render_to_rgba8(&centered, &mut text.atlas, size, scale, ground)
                            .unwrap();
                        if square_label != centered_rgba
                            && let Some(out) = &out
                        {
                            std::fs::write(out.join("alignment-failure-actual.rgba8"), &actual)
                                .unwrap();
                            std::fs::write(
                                out.join("alignment-failure-centered.rgba8"),
                                &centered_rgba,
                            )
                            .unwrap();
                            std::fs::write(
                                out.join("alignment-failure.json"),
                                serde_json::to_vec_pretty(&row).unwrap(),
                            )
                            .unwrap();
                        }
                        assert!(
                            square_label == centered_rgba,
                            "{key}: default Content must use the original text service's accepted Center alignment"
                        );
                        assert!(glyph_pixels > 0);
                        assert_eq!(
                            off_solid, 0,
                            "{key}: actual glyphs require opaque solid underneath; positive ink alone is insufficient"
                        );
                        assert_eq!(
                            missing_glyph_pixels, 0,
                            "{key}: every actual glyph pixel must remain readable over the solid"
                        );
                        rows.push(row);
                    }
                }
            }
        }
    }
    println!("normal opaque-solid readability GPU cases: {}", rows.len());
    if let Some(out) = out {
        std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"theme_set":theme_set,"cases":rows,"normal_frame_only":true,"narrow_case_full_text_readability_claim":false})).unwrap()).unwrap();
    }
}
