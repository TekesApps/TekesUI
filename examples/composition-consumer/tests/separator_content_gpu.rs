//! Original accepted a2 GPU scene/cases; explicitly opt in under the root lock.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout_view::SeparatorView,
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
const ROOT: ComponentId = ComponentId(1);
const SEP: ComponentId = ComponentId(3);
const WIDTH: f32 = 1040.;
const HEIGHT: f32 = 752.;
const SHIFT: f32 = 512.;
fn rect(r: Rect) -> serde_json::Value {
    serde_json::json!([r.min_x(), r.min_y(), r.size.width, r.size.height])
}
fn shifted(r: Rect) -> Rect {
    Rect::new(r.min_x() + SHIFT, r.min_y(), r.size.width, r.size.height)
}
fn host(axis: Axis, tree: bool, default: bool) -> ContentHost {
    let divider = if default {
        SeparatorContent::default()
    } else {
        SeparatorContent::new(axis)
    };
    let root = if tree {
        Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: if axis == Axis::Horizontal {
                        Axis::Vertical
                    } else {
                        Axis::Horizontal
                    },
                    gap: 4.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(
                        ComponentId(2),
                        TextContent::new(if axis == Axis::Horizontal {
                            "上文 中文"
                        } else {
                            "左文"
                        }),
                    ),
                    Content::new(SEP, divider),
                    Content::new(
                        ComponentId(4),
                        TextContent::new(if axis == Axis::Horizontal {
                            "下文 中文"
                        } else {
                            "右文"
                        }),
                    ),
                ],
            },
        )
    } else {
        Content::new(SEP, divider)
    };
    ContentHost::new(root).unwrap()
}
fn scissor(r: Rect, b: f32) -> [u32; 4] {
    let w = (WIDTH * b) as u32;
    let h = (HEIGHT * b) as u32;
    let x = (r.min_x() * b).ceil().max(0.).min(w as f32) as u32;
    let y = (r.min_y() * b).ceil().max(0.).min(h as f32) as u32;
    let right = (r.max_x() * b).floor().max(x as f32).min(w as f32) as u32;
    let bottom = (r.max_y() * b).floor().max(y as f32).min(h as f32) as u32;
    [x, y, right, bottom]
}
fn clips_at(scene: &Scene, index: usize) -> Rect {
    let mut stack = vec![Rect::new(0., 0., WIDTH, HEIGHT)];
    for p in &scene.primitives()[..index] {
        match p {
            Primitive::PushClip(r) => {
                stack.push(stack.last().unwrap().intersection(r).unwrap_or_default())
            }
            Primitive::PopClip if stack.len() > 1 => {
                stack.pop();
            }
            _ => {}
        }
    }
    *stack.last().unwrap()
}
#[test]
#[ignore = "explicit actual GPU output lane"]
fn actual_gpu_separator_original_a2_matrix() {
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SEPARATOR_GPU_OUT").expect("unique output"),
    );
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut rows = vec![];
    let mut pixel_frames = vec![];
    let mut failures = vec![];
    for (mode, appearance) in Theme::ALL.into_iter().enumerate() {
        for backing in [1., 2.] {
            let mut text = coretext::Text::new(1.);
            text.backing = backing;
            text.services = true;
            text.atlas.begin_frame();
            let mut scene = Scene::new();
            let mut retained = vec![];
            let mut cases = vec![];
            for (group, theme) in ["default", "blue", "clay", "blue-inputs"]
                .into_iter()
                .enumerate()
            {
                let p = Palette::for_theme_definition(appearance, theme).unwrap();
                let ground = p.color("document-canvas").unwrap();
                let ink = p.color("separator").unwrap();
                let oy = group as f32 * 184. + 8.;
                scene.push_quad(Quad::new(
                    Rect::new(0., group as f32 * 184., WIDTH, 184.),
                    ground,
                ));
                text.draw(
                    &mut scene,
                    &format!("{theme} — actual Content / original View"),
                    proposal::TextRole::UiCaption,
                    Rect::new(16., oy - 6., 1000., 16.),
                    ButtonTextAlign::Leading,
                    p.color("primary-text").unwrap(),
                );
                for (kind, axis, f, clip, tree, default, tiny) in [
                    (
                        "horizontal",
                        Axis::Horizontal,
                        Rect::new(16., oy + 20., 200., 48.),
                        Rect::new(8., oy + 16., 220., 56.),
                        true,
                        true,
                        false,
                    ),
                    (
                        "vertical",
                        Axis::Vertical,
                        Rect::new(252., oy + 20., 200., 48.),
                        Rect::new(244., oy + 16., 220., 56.),
                        true,
                        false,
                        false,
                    ),
                    (
                        "narrow-horizontal",
                        Axis::Horizontal,
                        Rect::new(16., oy + 90., 80., 76.),
                        Rect::new(8., oy + 86., 104., 84.),
                        true,
                        false,
                        false,
                    ),
                    (
                        "narrow-vertical",
                        Axis::Vertical,
                        Rect::new(128., oy + 90., 80., 48.),
                        Rect::new(120., oy + 86., 104., 60.),
                        true,
                        false,
                        false,
                    ),
                    (
                        "tiny-horizontal",
                        Axis::Horizontal,
                        Rect::new(252., oy + 104., 40., 0.125),
                        Rect::new(248., oy + 100., 48., 8.),
                        false,
                        false,
                        true,
                    ),
                    (
                        "tiny-vertical",
                        Axis::Vertical,
                        Rect::new(316., oy + 90., 0.125, 48.),
                        Rect::new(312., oy + 86., 8., 56.),
                        false,
                        false,
                        true,
                    ),
                    (
                        "clipped-horizontal",
                        Axis::Horizontal,
                        Rect::new(340., oy + 104., 120., 1.),
                        Rect::new(360.25, oy + 100., 60.5, 8.),
                        false,
                        false,
                        false,
                    ),
                    (
                        "clipped-vertical",
                        Axis::Vertical,
                        Rect::new(480., oy + 90., 1., 48.),
                        Rect::new(476., oy + 100.25, 8., 20.5),
                        false,
                        false,
                        false,
                    ),
                ] {
                    let mut h = host(axis, tree, default);
                    let start = scene.len();
                    h.render(&mut scene, &mut text, f, clip, p).unwrap();
                    let end = scene.len();
                    let allocated = h.get(SEP).unwrap().frame();
                    let actual_layout = h
                        .get(SEP)
                        .unwrap()
                        .component::<SeparatorContent>()
                        .unwrap()
                        .layout();
                    assert_eq!(actual_layout.frame, allocated);
                    assert!(h.focus_targets().is_empty());
                    assert!(!h.semantics().iter().any(|n| n.id == SEP));
                    assert!(h.drain_action_records().is_empty());
                    let marks = scene.primitives()[start..end]
                        .iter()
                        .enumerate()
                        .filter_map(|(n, q)| {
                            if let Primitive::Quad(q) = q {
                                Some((start + n, *q, clips_at(&scene, start + n)))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(marks.len(), 1);
                    assert_eq!(marks[0].1.fill, ink);
                    // Clip oracle follows the actual original ink geometry and Scene stack.
                    // Normal allocations preserve original ancestor clip; only ink
                    // wider than the short-axis allocation requires frame clipping.
                    let ink_fits_short_axis = if axis == Axis::Horizontal {
                        marks[0].1.rect.size.height <= allocated.size.height
                    } else {
                        marks[0].1.rect.size.width <= allocated.size.width
                    };
                    let allocation_clip = allocated.intersection(&clip).unwrap_or_default();
                    let expected_clip = if ink_fits_short_axis {
                        clip
                    } else {
                        allocation_clip
                    };
                    assert_eq!(marks[0].2, expected_clip);
                    let device = scissor(marks[0].2, backing);
                    let original_start = scene.len();
                    let mut view = SeparatorView::default();
                    let original = view.render_at_scale(
                        &mut scene,
                        shifted(allocated),
                        axis,
                        Some(shifted(clip)),
                        p,
                        backing,
                    );
                    let original_end = scene.len();
                    assert_eq!(original.frame, shifted(allocated));
                    let raw = scene.primitives()[original_start..original_end]
                        .iter()
                        .filter_map(|q| {
                            if let Primitive::Quad(q) = q {
                                Some(*q)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(raw.len(), 1);
                    assert_eq!(raw[0].fill, ink);
                    let expected = marks[0].1.rect;
                    assert_eq!(raw[0].rect, shifted(expected));
                    let glyphs = scene.primitives()[start..end]
                        .iter()
                        .filter(|p| matches!(p, Primitive::Glyphs(_)))
                        .count();
                    if tree {
                        assert!(
                            glyphs >= 2,
                            "actual original CoreText paragraphs are required"
                        );
                    }
                    let roi = Rect::new(
                        allocated.min_x() - 1.,
                        allocated.min_y() - 1.,
                        allocated.size.width + 2.,
                        allocated.size.height + 2.,
                    );
                    cases.push(serde_json::json!({"Theme":theme,"case":kind,"tree_with_two_CoreText_paragraphs":tree,"default_horizontal":default,"axis":format!("{axis:?}"),"parent_frame":rect(f),"ancestor_clip":rect(clip),"allocated_frame":rect(allocated),"actual_ink_quad":rect(marks[0].1.rect),"actual_effective_clip":rect(marks[0].2),"original_ink_fits_allocated_short_axis":ink_fits_short_axis,"allocation_logical_intersection":rect(allocation_clip),"actual_device_scissor_edges":device,"original_View_reference_frame":rect(original.frame),"original_View_reference_clip":rect(shifted(clip)),"ROI":rect(roi),"original_reference_shift_pt":SHIFT,"normal_View_pixel_equality_required":!tiny,"tiny_integer_scissor_can_be_empty":tiny,"actual_separator_role":format!("{ink:?}"),"Theme_ground":format!("{ground:?}"),"actual_original_CoreText_Glyph_primitives":glyphs,"new_Surface_packet":false}));
                    retained.push(h);
                }
            }
            assert_eq!(cases.len(), 32);
            let size = ((WIDTH * backing) as u32, (HEIGHT * backing) as u32);
            let clear = Palette::for_theme_definition(appearance, "default")
                .unwrap()
                .color("document-canvas")
                .unwrap();
            let rgba = gpu
                .render_to_rgba8(&scene, &mut text.atlas, size, backing, clear)
                .unwrap();
            // Original a2 review_pixels.py checks the entire unchanged ROI,
            // not a cropped line mask. Preserve its exact zero-difference oracle.
            let pixel = |x: usize, y: usize| {
                let offset = (y * size.0 as usize + x) * 4;
                &rgba[offset..offset + 4]
            };
            let mut reviews = vec![];
            for case in &cases {
                let group = ["default", "blue", "clay", "blue-inputs"]
                    .iter()
                    .position(|id| Some(*id) == case["Theme"].as_str())
                    .unwrap();
                let ground = pixel(
                    (4. * backing) as usize,
                    ((group as f32 * 184. + 178.) * backing) as usize,
                );
                let roi = case["ROI"].as_array().unwrap();
                let x = roi[0].as_f64().unwrap();
                let y = roi[1].as_f64().unwrap();
                let width = roi[2].as_f64().unwrap();
                let height = roi[3].as_f64().unwrap();
                let b = f64::from(backing);
                let edges = case["actual_device_scissor_edges"].as_array().unwrap();
                let [left, top, right, bottom] =
                    std::array::from_fn::<_, 4, _>(|i| edges[i].as_u64().unwrap() as usize);
                let shift = (case["original_reference_shift_pt"].as_f64().unwrap() * b) as usize;
                let mut ink = 0;
                let mut reference_ink = 0;
                let mut changed = 0;
                let mut maximum = 0;
                let mut outside = vec![];
                for py in (y * b).floor() as usize..((y + height) * b).ceil() as usize {
                    for px in (x * b).floor() as usize..((x + width) * b).ceil() as usize {
                        let actual = pixel(px, py);
                        let reference = pixel(px + shift, py);
                        if actual != ground {
                            ink += 1;
                            if !(left <= px && px < right && top <= py && py < bottom) {
                                outside.push([px, py]);
                            }
                        }
                        reference_ink += usize::from(reference != ground);
                        if actual != reference {
                            changed += 1;
                        }
                        maximum = maximum.max(
                            actual
                                .iter()
                                .zip(reference)
                                .map(|(a, b)| a.abs_diff(*b))
                                .max()
                                .unwrap(),
                        );
                    }
                }
                let normal = case["normal_View_pixel_equality_required"]
                    .as_bool()
                    .unwrap();
                let passed = outside.is_empty() && (!normal || (ink > 0 && changed == 0));
                let review = serde_json::json!({"Theme":case["Theme"],"case":case["case"],"actual_non_ground_pixels":ink,"original_View_non_ground_pixels":reference_ink,"outside_actual_integer_scissor_pixels":outside.len(),"outside_scissor_coordinates":outside,"normal_required":normal,"normal_vs_View_changed_pixels":changed,"normal_vs_View_max_channel_delta":maximum,"zero_width_or_height_scissor":right==left||bottom==top,"pass":passed,"allocated_frame":case["allocated_frame"],"scissor_edges":case["actual_device_scissor_edges"]});
                if !passed {
                    failures.push(serde_json::json!({"frame":format!("mode-{mode}@{backing}x.png"),"review":review}));
                }
                reviews.push(review);
            }
            let file = format!("mode-{mode}@{backing}x.png");
            pixel_frames.push(serde_json::json!({"file":file,"cases":reviews}));
            resvg::tiny_skia::Pixmap::from_vec(
                rgba,
                resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
            )
            .unwrap()
            .save_png(out.join(&file))
            .unwrap();
            rows.push(serde_json::json!({"file":file,"mode":mode,"backing":backing,"cases":cases}));
        }
    }
    assert_eq!(rows.len(), 8);
    assert_eq!(pixel_frames.len(), 8);
    let actual_cases: usize = pixel_frames
        .iter()
        .map(|row| row["cases"].as_array().unwrap().len())
        .sum();
    assert_eq!(actual_cases, 256);
    let normal_cases = pixel_frames
        .iter()
        .flat_map(|row| row["cases"].as_array().unwrap())
        .filter(|case| case["normal_required"] == true)
        .count();
    assert_eq!(normal_cases, 192);
    std::fs::write(out.join("pixel-review.json"),serde_json::to_vec_pretty(&serde_json::json!({"scope":"original a2 full ROI/zero-difference and integer scissor oracle", "actual_frames":8,"actual_cases":actual_cases,"frames":pixel_frames,"failures":failures,"pass":failures.is_empty(),"human_viewing_complete":false,"full_Gold7":false,"native":false,"performance":"not_run"})).unwrap()).unwrap();
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"frames":rows,"real_original_SeparatorContent":true,"real_original_SeparatorView":true,"actual_CoreText_and_GPU":true,"native":false,"performance":"not_run","full_Gold7":false})).unwrap()).unwrap();
    assert!(
        failures.is_empty(),
        "actual Separator GPU failures saved in pixel-review.json: {failures:?}"
    );
}
