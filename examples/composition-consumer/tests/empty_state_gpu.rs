#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::empty_state_content::{ACTION, MESSAGE, model};
use tekes_ui_controls::{composition::*, feedback_view::EmptyStateView};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{
    Palette, Theme, proposal,
    surface_palette::{resolve_surface, surface_names},
};
#[test]
#[ignore = "explicit actual GPU output lane"]
fn actual_gpu_original_view_packets_and_clip() {
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_EMPTY_GPU_OUT").expect("unique evidence output"),
    );
    assert!(out.starts_with("/Volumes/HP P900/DEV/TekesUI/artifacts"), "SSD evidence output required");
    std::fs::create_dir(&out).expect("fresh output directory required");
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cases = vec![];
    let external = std::env::var_os("TEKESUI_EMPTY_EXTERNAL").is_some();
    for (mode, theme) in Theme::ALL.into_iter().enumerate() {
        let palettes = if external {
            ["blue", "blue-inputs"]
                .into_iter()
                .map(|id| {
                    (
                        id.to_owned(),
                        Palette::for_theme_definition(theme, id).unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            (0..3)
                .map(|brand| (format!("brand{brand}"), Palette::new(theme, brand).unwrap()))
                .collect()
        };
        for (label, palette) in palettes {
            for surface in surface_names() {
                for backing in [1., 2.] {
                    for (name, frame, clip, positive, scale) in [
                        (
                            "normal",
                            Rect::new(8., 10., 240., 210.),
                            Rect::new(0., 0., 280., 250.),
                            true,
                            1.,
                        ),
                        (
                            "fractional",
                            Rect::new(8.25, 10.5, 240.5, 210.25),
                            Rect::new(0., 0., 280., 250.),
                            true,
                            1.,
                        ),
                        (
                            "narrow",
                            Rect::new(8.25, 10.5, 64.5, 210.25),
                            Rect::new(0., 0., 280., 250.),
                            true,
                            1.,
                        ),
                        (
                            "partial",
                            Rect::new(8.25, 10.5, 240.5, 210.25),
                            Rect::new(30., 20., 150., 130.),
                            true,
                            1.,
                        ),
                        (
                            "CJKlarge",
                            Rect::new(8.25, 10.5, 240.5, 210.25),
                            Rect::new(0., 0., 280., 250.),
                            true,
                            1.5,
                        ),
                        (
                            "zero",
                            Rect::new(8.25, 10.5, 0., 0.),
                            Rect::new(0., 0., 280., 250.),
                            false,
                            1.,
                        ),
                        (
                            "disjoint",
                            Rect::new(8.25, 10.5, 240.5, 210.25),
                            Rect::new(270., 240., 5., 5.),
                            false,
                            1.,
                        ),
                    ] {
                        let key = format!("{label}-m{mode}-{surface}-{name}@{backing}");
                        let mut text = coretext::Text::new(scale);
                        text.backing = backing;
                        let m = model(Some("Create 创建"));
                        let mut host = ContentHost::new(
                            Content::new(
                                MESSAGE,
                                EmptyStateContent::new("Documents", m.clone(), ACTION).unwrap(),
                            )
                            .with_surface(surface)
                            .unwrap(),
                        )
                        .unwrap();
                        let mut actual = Scene::new();
                        host.render(&mut actual, &mut text, frame, clip, palette)
                            .unwrap();
                        let mut view = EmptyStateView::default();
                        view.set_surface(surface).unwrap();
                        let mut expected = Scene::new();
                        view.render(
                            &mut expected,
                            &mut text,
                            &m,
                            host.get(MESSAGE).unwrap().frame(),
                            Some(clip),
                            palette,
                            proposal::TextRole::UiBody,
                        );
                        let size = ((280. * backing) as u32, (250. * backing) as u32);
                        let ground = resolve_surface(palette, surface, "content", "rest")
                            .unwrap()
                            .background;
                        let empty = gpu
                            .render_to_rgba8(&Scene::new(), &mut text.atlas, size, backing, ground)
                            .unwrap();
                        let rgba = gpu
                            .render_to_rgba8(&actual, &mut text.atlas, size, backing, ground)
                            .unwrap();
                        let oracle = gpu
                            .render_to_rgba8(&expected, &mut text.atlas, size, backing, ground)
                            .unwrap();
                        let mut ink = 0;
                        let mut outside = 0;
                        for (i, (a, b)) in
                            rgba.chunks_exact(4).zip(empty.chunks_exact(4)).enumerate()
                        {
                            if a != b {
                                ink += 1;
                                let x = i as u32 % size.0;
                                let y = i as u32 / size.0;
                                if x < (clip.min_x() * backing).ceil().max(0.) as u32
                                    || x >= (clip.max_x() * backing).floor().max(0.) as u32
                                    || y < (clip.min_y() * backing).ceil().max(0.) as u32
                                    || y >= (clip.max_y() * backing).floor().max(0.) as u32
                                {
                                    outside += 1;
                                }
                            }
                        }
                        let equal = rgba == oracle;
                        let record = serde_json::json!({"key":key,"size":[size.0,size.1],"ink":ink,"outside_ancestor":outside,"original_view_rgba_equal":equal,"positive":positive});
                        std::fs::write(
                            out.join("latest-case.json"),
                            serde_json::to_vec_pretty(&record).unwrap(),
                        )
                        .unwrap();
                        if (name == "normal" && label.ends_with('0'))
                            || !equal
                            || outside != 0
                            || (positive && ink == 0)
                            || (!positive && ink != 0)
                        {
                            std::fs::write(out.join(format!("{key}.actual.rgba8")), &rgba).unwrap();
                            std::fs::write(out.join(format!("{key}.view.rgba8")), &oracle).unwrap();
                            std::fs::write(out.join(format!("{key}.empty.rgba8")), &empty).unwrap();
                        }
                        assert!(equal, "{key} original View mismatch");
                        assert_eq!(outside, 0, "{key} ancestor clip");
                        if positive {
                            assert!(ink > 0, "{key} positive ink");
                        } else {
                            assert_eq!(ink, 0, "{key} invisible");
                        }
                        cases.push(record);
                    }
                }
            }
        }
    }
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"actual_GPU_frames":cases.len()*3,"cases":cases,"native_window":false,"physical_IME":false,"VoiceOver":false})).unwrap()).unwrap();
}
