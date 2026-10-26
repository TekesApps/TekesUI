#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{composition::*, extended::Progress, feedback_view::ProgressView};
use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, surface_palette::resolve_surface};
fn reference(gpu: &mut tekes_ui_render::Renderer, t: &mut coretext::Text, c: Rgba) -> [u8; 4] {
    let mut s = Scene::new();
    s.push_quad(Quad::new(Rect::new(0., 0., 8., 8.), c));
    let a = gpu
        .render_to_rgba8(&s, &mut t.atlas, (8, 8), 1., Rgba::new(1., 1., 1., 1.))
        .unwrap();
    a[4 * 36..4 * 37].try_into().unwrap()
}
fn near(a: &[u8], b: [u8; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 3)
}
#[test]
#[ignore = "explicit GPU acceptance requires fresh SSD output and theme mode environment"]
fn final_progress_actual_gpu_readback() {
    let out = std::path::PathBuf::from(std::env::var("TEKESUI_FINAL_GPU_OUT").unwrap());
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let external = std::env::var("TEKESUI_FINAL_GPU_EXTERNAL").unwrap() == "1";
    let calibrate = std::env::var("TEKESUI_FINAL_GPU_CALIBRATE").unwrap() == "1";
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut t = coretext::Text::new(1.);
    let mut rows = vec![];
    let names = if external {
        vec!["blue", "blue-inputs"]
    } else {
        vec!["default-brand0"]
    };
    for name in names {
        for (mode, theme) in Theme::ALL.into_iter().enumerate() {
            for scale in [1., 2.] {
                for (kind, frame, clip) in [
                    (
                        "normal",
                        Rect::new(16., 16., 160., 12.),
                        Rect::new(0., 0., 208., 64.),
                    ),
                    (
                        "narrow-large-allocation",
                        Rect::new(16., 16., 24., 24.),
                        Rect::new(0., 0., 208., 64.),
                    ),
                    (
                        "fractional",
                        Rect::new(16.25, 16.5, 159.5, 13.25),
                        Rect::new(0., 0., 208., 64.),
                    ),
                    (
                        "parentclip",
                        Rect::new(16., 16., 160., 12.),
                        Rect::new(34., 16., 90., 8.),
                    ),
                ] {
                    for (state, value, unknown) in [
                        ("zero", 0., false),
                        ("partial", 0.45, false),
                        ("full", 1., false),
                        ("unknown", 0., true),
                    ] {
                        if calibrate
                            && !(mode == 2
                                && scale == 1.
                                && kind == "normal"
                                && (state == "zero" || state == "partial"))
                        {
                            continue;
                        }
                        let p = if external {
                            Palette::for_theme_definition(theme, name).unwrap()
                        } else {
                            Palette::new(theme, 0).unwrap()
                        };
                        let packet =
                            resolve_surface(p, "canvas", "progress-track", "rest").unwrap();
                        let ground = resolve_surface(p, "canvas", "content", "rest")
                            .unwrap()
                            .background;
                        let refs = [
                            reference(&mut gpu, &mut t, ground),
                            reference(&mut gpu, &mut t, packet.background),
                            reference(&mut gpu, &mut t, packet.marker.unwrap()),
                            reference(&mut gpu, &mut t, packet.boundary.unwrap()),
                        ];
                        let mut model = Progress::new(value);
                        model.indeterminate = unknown;
                        let mut scene = Scene::new();
                        ProgressView::default().render(&mut scene, model, frame, Some(clip), p);
                        let mut c = ProgressContent::new("GPU progress", value);
                        c.model.indeterminate = unknown;
                        let mut h = ContentHost::new(Content::new(ComponentId(1), c)).unwrap();
                        let mut content = Scene::new();
                        h.render(&mut content, &mut t, frame, clip, p).unwrap();
                        let size = ((208. * scale) as u32, (64. * scale) as u32);
                        let a = gpu
                            .render_to_rgba8(&scene, &mut t.atlas, size, scale, ground)
                            .unwrap();
                        let b = gpu
                            .render_to_rgba8(&content, &mut t.atlas, size, scale, ground)
                            .unwrap();
                        assert_eq!(
                            a, b,
                            "Content/View GPU parity {name} {mode} {scale} {kind} {state}"
                        );
                        let visible = frame.intersection(&clip).unwrap();
                        let mut counts = [0usize; 4];
                        let mut ink = 0;
                        let mut escape = 0;
                        let mut blended_boundary = 0;
                        for y in 0..size.1 {
                            for x in 0..size.0 {
                                let px = &a[((y * size.0 + x) * 4) as usize
                                    ..((y * size.0 + x) * 4 + 4) as usize];
                                let lx = (x as f32 + 0.5) / scale;
                                let ly = (y as f32 + 0.5) / scale;
                                let inside = lx >= visible.min_x()
                                    && lx <= visible.max_x()
                                    && ly >= visible.min_y()
                                    && ly <= visible.max_y();
                                if inside
                                    && lx > frame.min_x() + frame.size.height
                                    && lx < frame.max_x() - frame.size.height
                                    && (ly < frame.min_y() + 1.5 || ly > frame.max_y() - 1.5)
                                {
                                    let active =
                                        frame.size.width * if unknown { 0.3 } else { value };
                                    let base = if lx < frame.min_x() + active {
                                        refs[2]
                                    } else {
                                        refs[1]
                                    };
                                    let fit = (2..=20).any(|step| {
                                        let alpha = step as f32 / 20.;
                                        (0..3).all(|i| {
                                            (px[i] as f32
                                                - (base[i] as f32 * (1. - alpha)
                                                    + refs[3][i] as f32 * alpha))
                                                .abs()
                                                <= 4.
                                        })
                                    });
                                    if fit && !near(px, base) {
                                        blended_boundary += 1;
                                    }
                                }
                                if inside {
                                    for i in 0..4 {
                                        if near(px, refs[i]) {
                                            counts[i] += 1
                                        }
                                    }
                                    if !near(px, refs[0]) {
                                        ink += 1
                                    }
                                } else if (lx < visible.min_x() - 1. / scale
                                    || lx > visible.max_x() + 1. / scale
                                    || ly < visible.min_y() - 1. / scale
                                    || ly > visible.max_y() + 1. / scale)
                                    && !near(px, refs[0])
                                {
                                    escape += 1
                                }
                            }
                        }
                        let file = format!("{name}-{mode}-{scale}-{kind}-{state}.png");
                        resvg::tiny_skia::Pixmap::from_vec(
                            a,
                            resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                        )
                        .unwrap()
                        .save_png(out.join(&file))
                        .unwrap();
                        let row = serde_json::json!({"name":name,"external_definition":external,"mode":mode,"scale":scale,"geometry":kind,"state":state,"rgba_reference":refs,"pixel_counts_ground_track_fill_boundary":counts,"boundary_antialias_fit_pixels":blended_boundary,"positive_ink":ink,"escape":escape,"file":file,"Content_View_GPU_equal":true});
                        rows.push(row);
                        std::fs::write(
                            out.join("readback.json"),
                            serde_json::to_vec_pretty(&rows).unwrap(),
                        )
                        .unwrap();
                        assert!(ink > 0, "no positive ink");
                        assert_eq!(escape, 0, "escaped clip");
                        if kind != "parentclip" {
                            if state != "full" {
                                assert!(counts[1] > 0 || refs[1] == refs[0], "missing track");
                            }
                            if state != "zero" {
                                assert!(counts[2] > 0, "missing fill");
                            }
                            assert!(
                                counts[3] > 0 || (kind == "fractional" && blended_boundary > 0),
                                "missing boundary"
                            );
                        }
                    }
                }
            }
        }
    }
    println!("actual GPU readback cases {}", rows.len());
}
