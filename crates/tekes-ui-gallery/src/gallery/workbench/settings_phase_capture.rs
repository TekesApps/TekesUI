//! #12's non-Ready Settings page capture; phase transitions use the real story owner.
use super::{Phase, entries};
use crate::gallery::{Gallery, Page, static_text::StaticText};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

#[test]
#[ignore = "requires an allocated GPU window and a fresh TEKESUI_SETTINGS_PHASE_OUT directory"]
fn settings_non_ready_phases_render_real_gallery_page_across_themes() {
    let output = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SETTINGS_PHASE_OUT")
            .expect("fresh Settings phase output directory"),
    );
    std::fs::create_dir(&output).expect("Settings phase output must not already exist");
    let mut renderer = Renderer::new_headless().expect("actual GPU required for Settings phases");
    let index = entries()
        .iter()
        .position(|entry| entry.slug() == "example-settings")
        .expect("registered Settings example");
    assert_eq!(entries()[index].get("kind"), "example");
    assert_eq!(entries()[index].renderer(), "settings");

    let route = "example-settings/playground";
    let mut gallery = Gallery::with_static_text(StaticText::new(None));
    gallery.set_reduced_motion(true);
    assert!(gallery.open_route(route));
    assert_eq!(gallery.page, Page::Workbench(index));
    gallery.set_view_size(720.0, 512.0);
    gallery.workbench.text_scale = 1.5;
    let state = gallery.workbench.state(index);
    assert!(
        state.draft_fixture.is_none(),
        "Gallery Settings uses its local fixture"
    );
    assert_eq!(state.phase, Phase::Ready);

    // Drive the current StoryState's original controller transitions. The
    // Gallery still builds and paints its actual Settings route for each phase.
    state.input.set_value("");
    state.submit();
    assert_eq!(gallery.workbench.state(index).phase, Phase::Invalid);
    let phases = [Phase::Invalid, Phase::Pending, Phase::Failed, Phase::Saved];
    let labels = ["invalid", "pending", "failed", "saved"];
    let mut manifest = Vec::new();

    for (position, phase) in phases.into_iter().enumerate() {
        let state = gallery.workbench.state(index);
        assert_eq!(state.phase, phase);
        for theme in Theme::ALL {
            gallery.set_system_theme(theme);
            for scale in [1.0_f32, 2.0] {
                gallery.text.set_scale(scale);
                gallery.build_scene(&mut Scene::new());
                let range = gallery.page_scroll_range().max(0.0);
                for (scroll_position, fraction) in [("top", 0.0), ("bottom", 1.0)] {
                    gallery.scroll_page_to(range * fraction);
                    let mut scene = Scene::new();
                    gallery.build_scene(&mut scene);
                    let offset = gallery.viewport.offset.y;
                    let actual_range = gallery.page_scroll_range().max(0.0);
                    assert!((offset - actual_range * fraction).abs() < 0.5);
                    let width = (720.0 * scale) as u32;
                    let height = (512.0 * scale) as u32;
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            gallery.atlas_mut(),
                            (width, height),
                            scale,
                            Gallery::background(theme),
                        )
                        .expect("Settings non-Ready real Gallery GPU frame");
                    assert!(rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]));
                    let file = format!(
                        "example-settings__playground__{}__{}__720x512__type-150__{}@{}x.png",
                        theme_slug(theme),
                        labels[position],
                        scroll_position,
                        scale as u32
                    );
                    let path = output.join(&file);
                    let mut encoder = png::Encoder::new(
                        std::fs::File::create(&path).expect("create fresh Settings phase PNG"),
                        width,
                        height,
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .expect("write PNG header")
                        .write_image_data(&rgba)
                        .expect("write Settings phase pixels");
                    let state = gallery.workbench.state(index);
                    manifest.push(serde_json::json!({
                        "file": file,
                        "route": route,
                        "renderer": "settings",
                        "phase": format!("{:?}", state.phase),
                        "status": labels[position],
                        "theme": theme_slug(theme),
                        "backing_scale": scale,
                        "viewport": {"width": 720, "height": 512, "type_scale": 1.5},
                        "position": scroll_position,
                        "actual_offset": offset,
                        "actual_scroll_range": actual_range,
                        "document_height": gallery.page_document_height(),
                        "input": state.input.query(),
                        "attempts": state.attempts,
                        "event_log": state.log,
                        "visual_review": "unverified",
                        "evidence": "actual Gallery Settings scene on headless GPU; not native input or performance acceptance"
                    }));
                }
            }
        }
        match phase {
            Phase::Invalid => {
                let state = gallery.workbench.state(index);
                state.input.set_value("Studio");
                state.submit();
                assert_eq!(state.phase, Phase::Pending);
            }
            Phase::Pending => {
                gallery.workbench.state(index).resolve();
                assert_eq!(gallery.workbench.state(index).phase, Phase::Failed);
            }
            Phase::Failed => {
                let state = gallery.workbench.state(index);
                state.submit();
                assert_eq!(state.phase, Phase::Pending);
                state.resolve();
                assert_eq!(state.phase, Phase::Saved);
            }
            Phase::Ready | Phase::Saved => {}
        }
    }
    assert_eq!(manifest.len(), 64);
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("serialize Settings phase manifest"),
    )
    .expect("write Settings phase manifest");
}

fn theme_slug(theme: Theme) -> String {
    format!("{:?}-{:?}", theme.appearance, theme.contrast).to_lowercase()
}
