#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::checkbox_content::{CHECKBOX, build};
use tekes_ui_controls::{CheckState, Checkbox, composition::*};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
#[test]
fn checkbox_actual_metal_coretext_matches_original_view_default_or_external_theme() {
    let mut renderer = tekes_ui_render::Renderer::new_headless().expect("actual Metal required");
    let out = std::env::var_os("TEKESUI_CHECKBOX_GPU_OUT")
        .map(std::path::PathBuf::from)
        .expect("explicit GPU evidence destination required");
    std::fs::create_dir_all(&out).unwrap();
    let ids = std::env::var("TEKESUI_CHECKBOX_THEME_IDS").ok();
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for surface in ["canvas", "sidebar", "panel"] {
            for scale in [1., 1.5, 2.] {
                for checked in [
                    CheckState::Unchecked,
                    CheckState::Checked,
                    CheckState::Mixed,
                ] {
                    for (width, backing, flags) in [(100., 1., 0), (360., 2., 1), (100., 2., 2)] {
                        let palette = if let Some(ids) = &ids {
                            Palette::for_theme_definition(theme, ids.split(',').next().unwrap())
                                .expect("actual external Theme ID")
                        } else {
                            theme.into()
                        };
                        let label = "Checkbox 第一行 · wrapping label";
                        let mut host = build(label, checked).unwrap();
                        host.set_inherited_surface(surface).unwrap();
                        {
                            let c = host
                                .get_mut(CHECKBOX)
                                .unwrap()
                                .component_mut::<CheckboxContent>()
                                .unwrap();
                            c.set_availability(flags == 2, false);
                            c.set_error(flags == 2);
                        }
                        let mut text = coretext::Text::new(scale);
                        text.backing = backing;
                        text.services = true;
                        text.atlas.begin_frame();
                        let frame = Rect::new(4.25, 8.5, width, 50.);
                        let clip = Rect::new(10., 0., 400., 280.);
                        host.render(&mut Scene::new(), &mut text, frame, clip, palette)
                            .unwrap();
                        if flags == 1 {
                            host.focus(Some(CHECKBOX), true);
                        }
                        text.atlas.begin_frame();
                        let mut scene = Scene::new();
                        host.render(&mut scene, &mut text, frame, clip, palette)
                            .unwrap();
                        let owner = host
                            .get(CHECKBOX)
                            .unwrap()
                            .component::<CheckboxContent>()
                            .unwrap();
                        let layout = owner.layout();
                        let mut original = Checkbox {
                            checked,
                            ..Default::default()
                        };
                        original.button.state = owner.model().button.state;
                        let mut reference_text = coretext::Text::new(scale);
                        reference_text.backing = backing;
                        reference_text.services = true;
                        reference_text.atlas.begin_frame();
                        let mut reference = Scene::new();
                        let expected = original
                            .render_on_surface(
                                &mut reference,
                                &mut reference_text,
                                label,
                                host.get(CHECKBOX).unwrap().frame(),
                                Some(clip),
                                palette,
                                proposal::TextRole::UiBody,
                                surface,
                            )
                            .unwrap();
                        assert_eq!(layout, expected);
                        let size = (880, 560);
                        let background = palette.color("surface-1").unwrap();
                        let actual = renderer
                            .render_to_rgba8(&scene, &mut text.atlas, size, backing, background)
                            .unwrap();
                        let expected = renderer
                            .render_to_rgba8(
                                &reference,
                                &mut reference_text.atlas,
                                size,
                                backing,
                                background,
                            )
                            .unwrap();
                        let different =
                            actual.iter().zip(&expected).filter(|(a, b)| a != b).count();
                        let max = actual
                            .iter()
                            .zip(&expected)
                            .map(|(a, b)| a.abs_diff(*b))
                            .max()
                            .unwrap_or(0);
                        assert_eq!(
                            different, 0,
                            "same original view pixels must match: {theme:?}/{surface}/{scale}/{checked:?}/{flags}; max {max}"
                        );
                        let index = cases.len();
                        for (name, pixels) in [("content", actual), ("original", expected)] {
                            resvg::tiny_skia::Pixmap::from_vec(
                                pixels,
                                resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                            )
                            .unwrap()
                            .save_png(out.join(format!("{index:03}-{name}.png")))
                            .unwrap();
                        }
                        cases.push(serde_json::json!({"theme":format!("{theme:?}"),"surface":surface,"type_scale":scale,"checked":format!("{checked:?}"),"flags":flags,"width":width,"backing":backing,"different_channels":different,"max_channel_delta":max,"renderer":"actual Metal + CoreText glyph atlas","external":ids.is_some()}));
                    }
                }
            }
        }
    }
    std::fs::write(
        out.join("receipt.json"),
        serde_json::to_vec_pretty(&cases).unwrap(),
    )
    .unwrap();
    assert!(!cases.is_empty());
}
