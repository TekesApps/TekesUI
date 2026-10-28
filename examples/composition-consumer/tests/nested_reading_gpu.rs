//! Actual renderer evidence, separate from native and physical-input acceptance.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::nested_reading::*;
use tekes_ui_controls::composition::*;
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};
#[test]
fn original_nested_detail_survives_append_on_actual_gpu_across_themes_and_scale() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let output = std::env::var_os("TEKESUI_NESTED_READING_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    let mut reports = vec![];
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for width in [320., 600.] {
            for zoom in [1., 1.5] {
                for backing in [1., 2.] {
                    let (mut host, mut state) = build().unwrap();
                    let mut text = coretext::Text::new(zoom);
                    text.services = true;
                    text.backing = backing;
                    let frame = Rect::new(0., 0., width, 440.);
                    let palette = Palette::from(theme);
                    let paint = |h: &mut ContentHost, t: &mut coretext::Text| {
                        t.atlas.begin_frame();
                        let mut scene = Scene::new();
                        h.render(&mut scene, t, frame, frame, palette).unwrap();
                        scene
                    };
                    paint(&mut host, &mut text);
                    host.focus(Some(COMPOSER), true);
                    let input = &mut host
                        .get_mut(COMPOSER)
                        .unwrap()
                        .component_mut::<TextFieldContent>()
                        .unwrap()
                        .surface
                        .input;
                    input.preedit("ni", 0..2, None);
                    let baseline = input.external_baseline();
                    let point = host
                        .get(INNER)
                        .unwrap()
                        .component::<TextFieldContent>()
                        .unwrap()
                        .surface
                        .inner
                        .center();
                    for (delta, time) in [(160., 1.), (-60., 2.)] {
                        host.event(&ContentEvent::Scroll {
                            position: point,
                            delta: Point::new(0., delta),
                            precise: true,
                            begin: true,
                            end: false,
                            momentum: false,
                            time,
                        });
                    }
                    assert!(
                        !host
                            .get(PARENT)
                            .unwrap()
                            .component::<ScrollContent>()
                            .unwrap()
                            .is_following_latest()
                    );
                    let before = host.get(INNER).unwrap().frame();
                    let outer = host
                        .get(PARENT)
                        .unwrap()
                        .component::<ScrollContent>()
                        .unwrap()
                        .scroll
                        .offset;
                    let mut images = vec![];
                    for phase in ["before", "after"] {
                        if phase == "after" {
                            assert!(
                                host.accessibility_action(APPEND, None, AccessibilityAction::Press)
                                    .activated
                            );
                            state.process_actions(&mut host);
                            assert_eq!(state.appended, 1);
                        }
                        let scene = paint(&mut host, &mut text);
                        let size = ((width * backing) as u32, (440. * backing) as u32);
                        let rgba = gpu
                            .render_to_rgba8(
                                &scene,
                                &mut text.atlas,
                                size,
                                backing,
                                palette.color("surface-1").unwrap(),
                            )
                            .unwrap();
                        assert!(
                            rgba.chunks_exact(4).any(|p| p != &rgba[..4]),
                            "real glyph/content pixels missing"
                        );
                        if let Some(out) = &output {
                            resvg::tiny_skia::Pixmap::from_vec(
                                rgba.clone(),
                                resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                            )
                            .unwrap()
                            .save_png(
                                out.join(format!(
                                    "nested-{ti}-{width}-{zoom}-{backing}-{phase}.png"
                                )),
                            )
                            .unwrap();
                        }
                        images.push(rgba);
                    }
                    assert!(
                        (host.get(INNER).unwrap().frame().min_y() - before.min_y()).abs() <= 1.
                    );
                    assert_eq!(
                        host.get(PARENT)
                            .unwrap()
                            .component::<ScrollContent>()
                            .unwrap()
                            .scroll
                            .offset,
                        outer
                    );
                    assert_eq!(host.focused(), Some(COMPOSER));
                    assert_eq!(
                        host.text_surface(COMPOSER)
                            .unwrap()
                            .input
                            .external_baseline(),
                        baseline
                    );
                    assert!(host.text_surface(COMPOSER).unwrap().input.marked.is_some());
                    // A glyph-bearing crop inside the original detail stays pixel exact;
                    // the appended lower document may legitimately paint elsewhere.
                    let stride = (width * backing) as usize * 4;
                    let x0 = ((before.min_x() + 6.) * backing) as usize;
                    let x1 =
                        ((before.min_x() + before.size.width.min(160.) - 6.) * backing) as usize;
                    let y0 = ((before.min_y() + 4.) * backing) as usize;
                    let y1 = ((before.max_y() - 4.) * backing) as usize;
                    let mut visible_ink = false;
                    for y in y0..y1 {
                        let range = y * stride + x0 * 4..y * stride + x1 * 4;
                        assert_eq!(
                            images[0][range.clone()],
                            images[1][range.clone()],
                            "original detail pixels moved after append"
                        );
                        visible_ink |= images[0][range]
                            .chunks_exact(4)
                            .any(|p| p != &images[0][..4]);
                    }
                    assert!(visible_ink);
                    reports.push(serde_json::json!({"theme":format!("{theme:?}"),"width":width,"zoom":zoom,"backing":backing,"frames":2,"anchor_unchanged":true,"detail_crop_pixel_equal":true,"original_preedit_preserved":true,"physical_IME":false}));
                }
            }
        }
    }
    assert_eq!(reports.len(), 32);
    if let Some(out) = output {
        std::fs::write(
            out.join("receipt.json"),
            serde_json::to_vec_pretty(&reports).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn original_scrollbar_position_survives_release_and_append_in_real_gpu_frames() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let output = std::env::var_os("TEKESUI_NESTED_READING_GPU_OUT").map(std::path::PathBuf::from);
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for backing in [1., 2.] {
            let (mut host, mut state) = build().unwrap();
            let mut text = coretext::Text::new(1.5);
            text.services = true;
            text.backing = backing;
            let frame = Rect::new(0., 0., 420., 440.);
            let palette = Palette::from(theme);
            let paint = |h: &mut ContentHost, t: &mut coretext::Text| {
                t.atlas.begin_frame();
                let mut scene = Scene::new();
                h.render(&mut scene, t, frame, frame, palette).unwrap();
                scene
            };
            paint(&mut host, &mut text);
            host.focus(Some(INNER), true);
            let surface = host.text_surface(INNER).unwrap();
            let point = surface
                .scroll
                .thumb(true, surface.track(true), 24.)
                .center();
            host.event(&ContentEvent::PointerDown(point));
            host.event(&ContentEvent::PointerMove(Point::new(
                point.x,
                point.y + 20.,
            )));
            let offset = host.text_surface(INNER).unwrap().scroll.offset;
            assert!(offset.y > 50.);
            host.event(&ContentEvent::PointerUp(point));
            assert_eq!(host.text_surface(INNER).unwrap().scroll.offset, offset);
            assert!(
                !host
                    .get(PARENT)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
                    .is_following_latest()
            );
            let before = host.get(INNER).unwrap().frame();
            let parent_offset = host
                .get(PARENT)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset;
            let mut images = vec![];
            for phase in ["before", "after"] {
                if phase == "after" {
                    host.accessibility_action(APPEND, None, AccessibilityAction::Press);
                    state.process_actions(&mut host);
                    assert_eq!(state.appended, 1);
                }
                let scene = paint(&mut host, &mut text);
                let size = ((420. * backing) as u32, (440. * backing) as u32);
                let rgba = gpu
                    .render_to_rgba8(
                        &scene,
                        &mut text.atlas,
                        size,
                        backing,
                        palette.color("surface-1").unwrap(),
                    )
                    .unwrap();
                if let Some(out) = &output {
                    std::fs::create_dir_all(out).unwrap();
                    resvg::tiny_skia::Pixmap::from_vec(
                        rgba.clone(),
                        resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                    )
                    .unwrap()
                    .save_png(out.join(format!("scrollbar-{ti}-{backing}-{phase}.png")))
                    .unwrap();
                }
                images.push(rgba);
            }
            assert_eq!(host.text_surface(INNER).unwrap().scroll.offset, offset);
            assert_eq!(
                host.get(PARENT)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
                    .scroll
                    .offset,
                parent_offset
            );
            assert_eq!(host.focused(), Some(INNER));
            assert!(host.pointer_capture().is_none());
            let stride = (420. * backing) as usize * 4;
            for y in ((before.min_y() + 4.) * backing) as usize
                ..((before.max_y() - 4.) * backing) as usize
            {
                let range = y * stride + ((before.min_x() + 6.) * backing) as usize * 4
                    ..y * stride + ((before.min_x() + 150.) * backing) as usize * 4;
                assert_eq!(
                    images[0][range.clone()],
                    images[1][range],
                    "scrollbar-owned detail jumped after append"
                );
            }
        }
    }
}
