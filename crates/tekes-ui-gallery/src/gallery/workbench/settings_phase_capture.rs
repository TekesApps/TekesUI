//! #12's non-Ready Settings captures use the existing Workbench state owner.
use super::state_capture::{settle_scroll, stabilize_document};
use super::{Phase, entries};
use crate::gallery::{Gallery, Page, static_text::StaticText};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

const SCROLL_POSITIONS: [(&str, f32); 2] = [("top", 0.0), ("bottom", 1.0)];

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
    let phases = [Phase::Invalid, Phase::Pending, Phase::Failed, Phase::Saved];
    let labels = ["invalid", "pending", "failed", "saved"];
    let mut manifest = Vec::new();

    for (phase_index, phase) in phases.into_iter().enumerate() {
        for theme in Theme::ALL {
            for scale in [1.0_f32, 2.0] {
                // Each phase/theme/scale starts with the real Settings defaults.
                let mut gallery = Gallery::with_static_text(StaticText::new(None));
                gallery.set_system_theme(theme);
                gallery.set_reduced_motion(true);
                assert!(gallery.open_route(route));
                assert_eq!(gallery.page, Page::Workbench(index));
                gallery.set_view_size(720.0, 512.0);
                gallery.workbench.text_scale = 2;
                gallery.text.set_scale(scale);
                let state = gallery.workbench.state(index);
                assert_eq!(state.phase, Phase::Ready);
                assert_eq!(state.input.query(), "Studio");
                assert_eq!(state.attempts, 0);
                assert!(state.draft_fixture.is_none());

                configure_phase(state, phase);
                let state = gallery.workbench.state(index);
                assert_eq!(state.phase, phase);
                assert_eq!(state.attempts, [0, 1, 1, 2][phase_index]);
                assert_eq!(
                    state.input.query(),
                    if phase == Phase::Invalid {
                        ""
                    } else {
                        "Studio"
                    }
                );
                assert!(state.draft_fixture.is_none());
                assert_eq!(gallery.theme(), theme);
                assert_eq!(gallery.page, Page::Workbench(index));

                stabilize_document(&mut gallery);
                assert_eq!(gallery.text.type_scale(), 1.5);
                let initial_range = gallery.page_scroll_range().max(0.0);
                for (position, fraction) in SCROLL_POSITIONS {
                    let (offset, actual_range) =
                        settle_scroll(&mut gallery, fraction, initial_range * fraction);
                    assert!((offset - actual_range * fraction).abs() < 0.5);
                    let mut scene = Scene::new();
                    gallery.build_scene(&mut scene);
                    let document_height = gallery.page_document_height();
                    let state_snapshot = {
                        let state = gallery.workbench.state(index);
                        assert_eq!(state.phase, phase);
                        assert_eq!(state.attempts, [0, 1, 1, 2][phase_index]);
                        assert_eq!(state.log.len(), [1, 1, 2, 4][phase_index]);
                        assert_eq!(
                            state.input.query(),
                            if phase == Phase::Invalid {
                                ""
                            } else {
                                "Studio"
                            }
                        );
                        assert_eq!(state.buttons[&20].state.disabled, phase == Phase::Pending);
                        assert_eq!(
                            state.button_labels[&20],
                            if phase == Phase::Failed {
                                "Retry save"
                            } else {
                                "Save settings"
                            }
                        );
                        assert_eq!(state.field.error.is_some(), phase == Phase::Invalid);
                        assert_eq!(state.buttons.contains_key(&21), phase == Phase::Pending);

                        let notification = state.buttons[&25]
                            .layout()
                            .expect("notification action layout")
                            .frame;
                        let save = state.buttons[&20]
                            .layout()
                            .expect("save action layout")
                            .frame;
                        let phase_frame = state.phase_frame.expect("Settings phase frame");
                        let log = state.event_log_frame.expect("Settings event log frame");
                        assert!(
                            notification.max_y() + 11.9 <= save.min_y(),
                            "notification/save order: {notification:?} {save:?}"
                        );
                        assert!(
                            save.max_y() + 11.9 <= phase_frame.min_y(),
                            "save/phase order: {save:?} {phase_frame:?}"
                        );
                        assert!(
                            phase_frame.max_y() + 24.0 <= log.min_y(),
                            "phase/event-log overlap: {phase_frame:?} {log:?}"
                        );
                        let resolve = state
                            .buttons
                            .get(&21)
                            .and_then(|button| button.layout())
                            .map(|layout| layout.frame);
                        assert_eq!(resolve.is_some(), phase == Phase::Pending);
                        serde_json::json!({
                            "phase": format!("{:?}", state.phase),
                            "attempts": state.attempts,
                            "brand": state.brand,
                            "input": state.input.query(),
                            "event_log": state.log,
                            "field_error": state.field.error,
                            "save_disabled": state.buttons[&20].state.disabled,
                            "save_label": state.button_labels[&20],
                            "frames": {
                                "notification": rect(notification),
                                "save": rect(save),
                                "phase": rect(phase_frame),
                                "event_log": rect(log),
                                "resolve": resolve.map(rect)
                            }
                        })
                    };
                    assert!(!scene.is_empty());

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
                        labels[phase_index],
                        position,
                        scale as u32
                    );
                    // The existing PNG helper is private to the frozen A/B module.
                    // Keep this one local write without widening that module's API.
                    let mut encoder = png::Encoder::new(
                        std::fs::File::create(output.join(&file))
                            .expect("create fresh Settings phase PNG"),
                        width,
                        height,
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .expect("write Settings phase PNG header")
                        .write_image_data(&rgba)
                        .expect("write Settings phase PNG pixels");
                    manifest.push(serde_json::json!({
                        "file": file,
                        "route": route,
                        "entry_index": index,
                        "renderer": "settings",
                        "status": labels[phase_index],
                        "theme": theme_slug(theme),
                        "backing_scale": scale,
                        "viewport": {"width": 720, "height": 512, "type_scale": 1.5},
                        "position": position,
                        "actual_offset": offset,
                        "actual_scroll_range": actual_range,
                        "document_height": document_height,
                        "state": state_snapshot,
                        "visual_review": "unverified",
                        "evidence": "actual Gallery Settings scene on headless GPU; fixture transitions are not native input, IME, VoiceOver or performance acceptance"
                    }));
                }
            }
        }
    }
    assert_eq!(manifest.len(), 64);
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("serialize Settings phase manifest"),
    )
    .expect("write Settings phase manifest");
}

fn configure_phase(state: &mut super::StoryState, phase: Phase) {
    match phase {
        Phase::Invalid => {
            state.input.set_value("");
            state.activate(20, "settings");
        }
        Phase::Pending => state.activate(20, "settings"),
        Phase::Failed => {
            state.activate(20, "settings");
            state.activate(21, "settings");
        }
        Phase::Saved => {
            state.activate(20, "settings");
            state.activate(21, "settings");
            state.activate(20, "settings");
            state.activate(21, "settings");
        }
        Phase::Ready => unreachable!("capture only non-Ready Settings phases"),
    }
}

fn rect(rect: tekes_ui_scene::Rect) -> [f32; 4] {
    [
        rect.min_x(),
        rect.min_y(),
        rect.size.width,
        rect.size.height,
    ]
}

fn theme_slug(theme: Theme) -> String {
    format!("{:?}-{:?}", theme.appearance, theme.contrast).to_lowercase()
}
