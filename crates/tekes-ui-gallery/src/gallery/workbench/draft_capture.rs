//! Real Gallery form/phase scenes, separate from native input acceptance.
use super::{Phase, entries, entry};
use crate::gallery::{Gallery, static_text::StaticText};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "tekesui-draft-gpu-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn draft_protocol_form_scenes_use_current_journal_and_real_gpu() {
    let mut renderer = tekes_ui_render::Renderer::new_headless().expect("actual GPU required");
    let output = std::env::var_os("TEKESUI_DRAFT_PAGE_OUT").map(PathBuf::from);
    if let Some(p) = &output {
        std::fs::create_dir_all(p).unwrap();
    }
    let i = entries()
        .iter()
        .position(|e| e.renderer() == "form")
        .unwrap();
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for scale in [1_u32, 2] {
            for (width, height, type_scale) in [(1440_u32, 960_u32, 0), (720, 512, 2)] {
                for storage in ["memory", "file"] {
                    let directory = Directory::new();
                    let mut gallery = Gallery::with_static_text(StaticText::new(None));
                    gallery.set_system_theme(theme);
                    gallery.set_reduced_motion(true);
                    gallery.text.set_scale(scale as f32);
                    gallery.workbench.text_scale = type_scale;
                    assert!(gallery.open_route(&format!("{}/playground", entry(i).slug())));
                    gallery.set_view_size(width as f32, height as f32);
                    if storage == "file" {
                        gallery
                            .workbench
                            .state(i)
                            .open_draft_fixture(directory.0.join("draft.json"));
                    }
                    let mut first = 0;
                    for stage in ["pending", "newer-pending", "late-ack", "acknowledged"] {
                        let state = gallery.workbench.state(i);
                        match stage {
                            "pending" => {
                                state.input.set_value("First immutable payload");
                                state.persist_draft();
                                state.submit();
                                first = state.draft_attempt.unwrap().0;
                                assert_eq!(state.phase, Phase::Pending);
                            }
                            "newer-pending" => {
                                state.input.set_value("Newer editor payload retained");
                                state.persist_draft();
                                assert_eq!(state.phase, Phase::Pending);
                            }
                            "late-ack" => {
                                state.resolve();
                                assert_eq!(state.phase, Phase::Failed);
                                state.submit();
                                assert_eq!(state.draft_attempt, Some((first, 2)));
                                state.resolve();
                                assert_eq!(state.phase, Phase::Ready);
                                assert_eq!(state.input.query(), "Newer editor payload retained");
                                assert!(
                                    !state
                                        .draft_fixture
                                        .as_mut()
                                        .unwrap()
                                        .acknowledge(first)
                                        .unwrap()
                                );
                            }
                            "acknowledged" => {
                                state.submit();
                                state.resolve();
                                assert_eq!(state.phase, Phase::Failed);
                                state.submit();
                                state.resolve();
                                assert_eq!(state.phase, Phase::Saved);
                            }
                            _ => unreachable!(),
                        }
                        super::state_capture::stabilize_document(&mut gallery);
                        for (position, fraction) in [("top", 0.), ("bottom", 1.)] {
                            let requested = gallery.page_scroll_range() * fraction;
                            let (offset, range) = super::state_capture::settle_scroll(
                                &mut gallery,
                                fraction,
                                requested,
                            );
                            let mut scene = Scene::new();
                            gallery.build_scene(&mut scene);
                            let rgba = renderer
                                .render_to_rgba8(
                                    &scene,
                                    gallery.atlas_mut(),
                                    (width * scale, height * scale),
                                    scale as f32,
                                    Gallery::background(theme),
                                )
                                .unwrap();
                            let theme_name = format!("{:?}-{:?}", theme.appearance, theme.contrast)
                                .to_lowercase();
                            let name = format!(
                                "form-{storage}-{stage}-{theme_name}-{width}x{height}-{position}@{scale}x.png"
                            );
                            if let Some(output) = &output {
                                let mut encoder = png::Encoder::new(
                                    std::fs::File::create(output.join(&name)).unwrap(),
                                    width * scale,
                                    height * scale,
                                );
                                encoder.set_color(png::ColorType::Rgba);
                                encoder.set_depth(png::BitDepth::Eight);
                                encoder
                                    .write_header()
                                    .unwrap()
                                    .write_image_data(&rgba)
                                    .unwrap();
                            }
                            let state = gallery.workbench.state(i);
                            let store = state.draft_fixture.as_ref().unwrap();
                            cases.push(serde_json::json!({"file":name,"storage":storage,"stage":stage,"theme":theme_name,"scale":scale,"width":width,"height":height,"type_scale":type_scale,"position":position,"offset":offset,"range":range,"phase":format!("{:?}",state.phase),"current_revision":store.draft().revision,"pending":store.has_pending(),"acknowledged":store.is_acknowledged(),"editor":state.input.query(),"actual_gpu":true,"native_acceptance":false}));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 256);
    if let Some(output) = output {
        std::fs::write(
            output.join("manifest.json"),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
}
