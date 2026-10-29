//! Real Metal oracle for the public original-action forwarding consumer.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::forwarded_actions::*;
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};

#[test]
fn direct_and_forwarded_exact_rows_keep_both_original_ring_edges_above_neighbors() {
    let mut gpu = tekes_ui_render::Renderer::new_headless().expect("real Metal renderer");
    let output = std::env::var_os("TEKESUI_FORWARDED_FOCUS_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    let mut reports = Vec::new();
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for type_scale in [1., 1.5, 2.] {
            for (width, backing) in [(160., 1.), (240., 2.)] {
                for id in [DIRECT, WRAPPED_RELEASE, WRAPPED_PRESS] {
                    let mut host = build();
                    let mut text = coretext::Text::new(type_scale);
                    text.services = true;
                    text.backing = backing;
                    let palette = Palette::from(theme);
                    let viewport = Rect::new(0., 0., width + 40., 160.);
                    let frame = Rect::new(20., 20., width, 120.);
                    let size = ((viewport.size.width * backing) as u32, 160 * backing as u32);
                    let clear = palette.color("surface-1").unwrap();
                    let mut reference = Scene::new();
                    host.render(&mut reference, &mut text, frame, viewport, palette)
                        .unwrap();
                    let before_frames = [DIRECT, WRAPPED_RELEASE, WRAPPED_PRESS, AFTER, NORMAL]
                        .map(|target| host.get(target).unwrap().frame());
                    let before_order = host.semantics().iter().map(|n| n.id).collect::<Vec<_>>();
                    let baseline = gpu
                        .render_to_rgba8(&reference, &mut text.atlas, size, backing, clear)
                        .unwrap();
                    host.focus(Some(id), true);
                    // Oracle: paint the SAME retained original subtree over unfocused
                    // neighbors using its public paint API. No copied ring or predicates.
                    // Only out-of-frame pixels are compared, so repainting the action's
                    // interior in this reference cannot make the assertion pass.
                    host.get_mut(id)
                        .unwrap()
                        .paint(&mut reference, &mut text, palette.into());
                    let expected = gpu
                        .render_to_rgba8(&reference, &mut text.atlas, size, backing, clear)
                        .unwrap();
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, viewport, palette)
                        .unwrap();
                    let actual = gpu
                        .render_to_rgba8(&scene, &mut text.atlas, size, backing, clear)
                        .unwrap();
                    assert_eq!(
                        before_frames,
                        [DIRECT, WRAPPED_RELEASE, WRAPPED_PRESS, AFTER, NORMAL]
                            .map(|target| host.get(target).unwrap().frame())
                    );
                    assert_eq!(
                        before_order,
                        host.semantics().iter().map(|n| n.id).collect::<Vec<_>>()
                    );
                    assert_eq!(host.get(NORMAL).unwrap().frame().size.height, 36.);
                    let action_frame = host.get(id).unwrap().frame();
                    let outset = proposal::metric("stroke", "c-focus-width").unwrap()
                        + proposal::metric("spacing", "c-focus-offset").unwrap()
                        + 1.;
                    let mut changed = [0usize; 2];
                    let mut checked = [0usize; 2];
                    for y in 0..size.1 {
                        let py = (y as f32 + 0.5) / backing;
                        let edge = if py >= action_frame.min_y() - outset
                            && py < action_frame.min_y()
                        {
                            Some(0)
                        } else if py >= action_frame.max_y() && py < action_frame.max_y() + outset {
                            Some(1)
                        } else {
                            None
                        };
                        let Some(edge) = edge else { continue };
                        for x in 0..size.0 {
                            let px = (x as f32 + 0.5) / backing;
                            if px < action_frame.min_x() - outset
                                || px > action_frame.max_x() + outset
                            {
                                continue;
                            }
                            let at = ((y * size.0 + x) * 4) as usize;
                            let want = &expected[at..at + 4];
                            let got = &actual[at..at + 4];
                            assert!(
                                want.iter().zip(got).all(|(a, b)| a.abs_diff(*b) <= 1),
                                "ring occluded: theme={theme:?} type={type_scale} backing={backing} id={id:?} edge={edge} pixel=({x},{y}) expected={want:?} actual={got:?}"
                            );
                            checked[edge] += 1;
                            if want
                                .iter()
                                .zip(&baseline[at..at + 4])
                                .any(|(a, b)| a.abs_diff(*b) > 8)
                            {
                                changed[edge] += 1;
                            }
                        }
                    }
                    assert!(
                        changed.into_iter().all(|count| count > width as usize / 2),
                        "oracle must observe actual ring ink on both edges: {changed:?}"
                    );
                    if let Some(out) = &output {
                        for (phase, pixels) in [
                            ("baseline", baseline),
                            ("reference", expected),
                            ("actual", actual),
                        ] {
                            resvg::tiny_skia::Pixmap::from_vec(
                                pixels,
                                resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                            )
                            .unwrap()
                            .save_png(out.join(format!(
                                "focus-{theme_index}-{type_scale}-{width}-{backing}-{}-{phase}.png",
                                id.0
                            )))
                            .unwrap();
                        }
                    }
                    reports.push(serde_json::json!({
                        "theme": format!("{theme:?}"), "type_scale": type_scale,
                        "width": width, "backing_scale": backing, "target": id.0,
                        "checked_edge_pixels": checked, "visible_ring_pixels": changed,
                        "original_subtree_oracle": true, "native_input": false
                    }));
                }
            }
        }
    }
    assert_eq!(reports.len(), 72);
    if let Some(out) = &output {
        std::fs::write(
            out.join("receipt.json"),
            serde_json::to_vec_pretty(&reports).unwrap(),
        )
        .unwrap();
    }
}
