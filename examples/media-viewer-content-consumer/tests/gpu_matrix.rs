#![cfg(target_os = "macos")]
use tekes_ui_controls::composition::*;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Rgba, Theme};
#[path = "support/coretext.rs"]
mod coretext;
#[path = "support/viewer_fixture.rs"]
mod fixture;
#[path = "support/pixel_oracle.rs"]
mod pixel_oracle;

/// Run only in an allocated GPU window. Generated PNGs require individual review;
/// this test records renders and structural assertions, not visual approval.
#[test]
#[ignore = "requires allocated GPU window and MEDIA_VIEWER_OUTPUT evidence directory"]
fn four_themes_two_backings_and_narrow_large_type() {
    let output = std::path::PathBuf::from(
        std::env::var_os("MEDIA_VIEWER_OUTPUT").expect("set output directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    assert!(
        std::fs::read_dir(&output).unwrap().next().is_none(),
        "use a fresh raw-evidence directory"
    );
    let binding = std::env::var_os("MEDIA_VIEWER_BINDING").expect("set verified run-binding JSON");
    std::fs::copy(binding, output.join("run-binding.json")).unwrap();
    std::fs::write(
        output.join("process.txt"),
        format!(
            "pid={}\ncurrent_exe={:?}\n",
            std::process::id(),
            std::env::current_exe().unwrap()
        ),
    )
    .unwrap();
    let mut gpu = Renderer::new_headless().expect("real GPU required");
    let mut all_failures = Vec::new();
    std::fs::create_dir(output.join("review")).unwrap();
    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
        for backing in [1., 2.] {
            for (layout, width, type_scale) in [("regular", 480., 1.), ("narrow150", 240., 1.5)] {
                for state in [
                    "fit", "zoom", "edge", "resize", "loading", "failed", "empty",
                ] {
                    let mut host = fixture::fixture(&mut gpu);
                    let mut text = coretext::Text::new(type_scale);
                    text.backing = backing;
                    let frame = Rect::new(0., 0., width, 720.);
                    let mut scene = Scene::new();
                    host.render(&mut scene, &mut text, frame, frame, theme.into())
                        .unwrap();
                    if state != "fit" {
                        fixture::viewer(&mut host)
                            .execute(MediaViewerCommand::ZoomAt {
                                relative_scale: 4.,
                                anchor: Point::new(width / 2., 50.),
                            })
                            .unwrap();
                    }
                    if state == "edge" {
                        fixture::viewer(&mut host)
                            .execute(MediaViewerCommand::PanBy {
                                delta: Point::new(-10000., -10000.),
                            })
                            .unwrap();
                    }
                    if state == "loading" || state == "failed" || state == "empty" {
                        let request = fixture::viewer(&mut host).begin_load().unwrap();
                        if state == "failed" {
                            fixture::viewer(&mut host)
                                .complete(
                                    request,
                                    ImageLoadOutcome::Failed("Fixture replacement failed".into()),
                                )
                                .unwrap();
                        } else if state == "empty" {
                            fixture::viewer(&mut host)
                                .complete(request, ImageLoadOutcome::Missing)
                                .unwrap();
                        }
                    }
                    fixture::refresh_buttons(&mut host);
                    fixture::refresh_status(&mut host);
                    let frame = if state == "resize" {
                        Rect::new(0., 0., width - 40., 640.)
                    } else {
                        frame
                    };
                    scene.clear();
                    host.render(&mut scene, &mut text, frame, frame, theme.into())
                        .unwrap();
                    assert!(
                        host.focus_targets()
                            .iter()
                            .any(|(id, rect)| *id == fixture::CLOSE
                                && rect.size.height > 0.
                                && rect.max_y() <= frame.max_y()),
                        "Close must remain visible"
                    );
                    let pixels = gpu
                        .render_to_rgba8(
                            &scene,
                            &mut text.atlas,
                            (
                                (frame.size.width * backing) as u32,
                                (frame.size.height * backing) as u32,
                            ),
                            backing,
                            Rgba::gray(0.125),
                        )
                        .unwrap();
                    let size = resvg::tiny_skia::IntSize::from_wh(
                        (frame.size.width * backing) as u32,
                        (frame.size.height * backing) as u32,
                    )
                    .unwrap();
                    let png = resvg::tiny_skia::Pixmap::from_vec(pixels, size).unwrap();
                    let stem = format!("theme{ti}-{backing}x-{layout}-{state}");
                    let expected = match state {
                        "fit" => {
                            "All RG/BY corner markers and transparent center visible; Fit centered."
                        }
                        "zoom" => {
                            "Manual enlarged image; expected cropping only inside original viewport; no blank drag-out."
                        }
                        "edge" => {
                            "Pan at bottom-right bound; parent rounded corner masks image; no yellow outside curve."
                        }
                        "resize" => {
                            "Viewport shrinks; Manual q stays 4, center reading anchor preserved except required clamp."
                        }
                        "loading" => {
                            "Previous image remains; loading copy visible; transform controls disabled; Close reachable."
                        }
                        "failed" => {
                            "Previous image remains; error and Retry visible; transform controls restored; Close reachable."
                        }
                        "empty" => {
                            "No image pixels; missing copy and Retry visible; transform controls disabled; Close reachable."
                        }
                        _ => unreachable!(),
                    };
                    std::fs::write(output.join("review").join(format!("{stem}.md")),format!(
                        "# {stem}\n\nReview status: NOT_REVIEWED\n\nExpected: {expected}\n\n- Check original PNG at native scale, then zoom for clip/alpha edges.\n- Check theme contrast, parent rounded clipping, complete Close/Retry labels and narrow toolbar layout.\n- Check image against scene/transform TXT and numeric oracle; record discrepancies independently.\n\nReviewer/input source/time: UNRECORDED\nObserved result and defects: UNRECORDED\n" )).unwrap();
                    png.save_png(output.join(format!("{stem}.png"))).unwrap();
                    std::fs::write(output.join(format!("{stem}.txt")),format!("theme={theme:?}\nbacking={backing}\ntype_scale={type_scale}\nframe={frame:?}\ntransform={:?}\nsemantics={:#?}\nscene={scene:#?}\nvisual_review=NOT_RUN\n",fixture::viewer(&mut host).transform(),host.semantics())).unwrap();
                    let (oracle, failures) =
                        pixel_oracle::inspect(&host, &scene, png.data(), frame, backing, state);
                    std::fs::write(output.join(format!("{stem}-oracle.txt")), oracle).unwrap();
                    all_failures.extend(
                        failures
                            .into_iter()
                            .map(|failure| format!("{stem}: {failure}")),
                    );
                }
            }
        }
    }
    assert!(
        all_failures.is_empty(),
        "Pixel oracle failures (raw PNG/scene retained): {all_failures:#?}"
    );
}
