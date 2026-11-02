//! Narrow, large-type captures of the documented Filter, Async and Danger patterns.
//!
//! These ignored GPU fixtures drive the registered Gallery stories and current
//! story-state owners. They record state and geometry; PNGs remain unreviewed
//! until a person inspects each image.

use super::state_capture::{settle_scroll, stabilize_document};
use super::{ControlKey, Event, entries};
use crate::gallery::{Gallery, Page, static_text::StaticText};
use tekes_ui_controls::DialogEvent;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;

const POSITIONS: [(&str, f32); 2] = [("top", 0.0), ("bottom", 1.0)];

struct CaptureSpec<'a> {
    index: usize,
    renderer: &'a str,
    state: &'a str,
    theme: Theme,
    scale: f32,
    position: &'a str,
    offset: f32,
    scroll_range: f32,
    state_record: serde_json::Value,
    pre_modal_scroll: bool,
}

#[test]
#[ignore = "requires allocated GPU and fresh TEKESUI_PATTERN_READING_OUT child directories"]
fn pattern_filter_reading_states_capture_real_gallery_pages() {
    let output = fresh_output("filter");
    let mut renderer = Renderer::new_headless().expect("actual GPU required for Filter captures");
    let index = route_index("pattern-filter", "filter");
    let cases = [
        (
            "notes",
            "NOTES",
            &["Studio notes.md", "Research notes.txt"][..],
        ),
        (
            "not-found",
            "not-found",
            &["No results. Clear the filter to see all files."][..],
        ),
        (
            "clear-all",
            "",
            &["Studio notes.md", "Research notes.txt", "项目计划.md"][..],
        ),
    ];
    let mut manifest = Vec::new();

    for (case, query, expected) in cases {
        for theme in Theme::ALL {
            for scale in [1.0_f32, 2.0] {
                for (position, fraction) in POSITIONS {
                    let mut gallery = new_gallery("pattern-filter/playground", index, theme, scale);
                    if case == "clear-all" {
                        set_query(&mut gallery, index, "NOTES");
                        gallery.workbench.state(index).activate(22, "filter");
                    } else {
                        set_query(&mut gallery, index, query);
                    }
                    assert_eq!(gallery.workbench.state(index).input.query(), query);
                    if case == "clear-all" {
                        assert_eq!(gallery.workbench.state(index).log.len(), 1);
                        assert_eq!(gallery.workbench.state(index).log[0], "filter: cleared");
                    } else {
                        assert!(gallery.workbench.state(index).log.is_empty());
                    }

                    stabilize_document(&mut gallery);
                    let range = gallery.page_scroll_range().max(0.0);
                    let (offset, actual_range) =
                        settle_scroll(&mut gallery, fraction, range * fraction);
                    let mut scene = Scene::new();
                    gallery.build_scene(&mut scene);
                    let state = gallery
                        .workbench
                        .states
                        .get(&index)
                        .expect("mounted Filter state");
                    assert_eq!(state.phase, super::Phase::Ready);
                    assert_eq!(state.attempts, 0);
                    let input = state.input.frame_at(0).expect("real filter input frame");
                    let clear = state.buttons[&22]
                        .layout()
                        .expect("real Clear filter layout")
                        .frame;
                    let log = state.event_log_frame.expect("real Filter event-log frame");
                    assert_eq!(state.input.query(), query);
                    assert_eq!(state.brand, 0);
                    assert_eq!(gallery.page, Page::Workbench(index));
                    assert_eq!(gallery.theme(), theme);
                    assert!((offset - actual_range * fraction).abs() < 0.5);

                    let geometry = serde_json::json!({
                        "input": rect(input),
                        "clear": rect(clear),
                        "event_log": rect(log)
                    });
                    let state_record = serde_json::json!({
                        "query": query,
                        "phase": format!("{:?}", state.phase),
                        "attempts": state.attempts,
                        "expected_results": expected,
                        "expected_result_count": expected.len(),
                        "event_log": state.log.iter().cloned().collect::<Vec<_>>(),
                        "frames": geometry
                    });
                    manifest.push(capture_page(
                        &mut renderer,
                        &mut gallery,
                        &output,
                        CaptureSpec {
                            index,
                            renderer: "filter",
                            state: case,
                            theme,
                            scale,
                            position,
                            offset,
                            scroll_range: actual_range,
                            state_record,
                            pre_modal_scroll: false,
                        },
                    ));
                }
            }
        }
    }

    assert_eq!(manifest.len(), 48);
    write_manifest(&output, &manifest);
}

#[test]
#[ignore = "requires allocated GPU and fresh TEKESUI_PATTERN_READING_OUT child directories"]
fn pattern_async_reading_phases_capture_real_gallery_pages() {
    let output = fresh_output("async");
    let mut renderer = Renderer::new_headless().expect("actual GPU required for Async captures");
    let index = route_index("pattern-async", "async");
    let phases = ["ready", "pending", "failed", "saved"];
    let mut manifest = Vec::new();

    for phase in phases {
        for theme in Theme::ALL {
            for scale in [1.0_f32, 2.0] {
                for (position, fraction) in POSITIONS {
                    let mut gallery = new_gallery("pattern-async/playground", index, theme, scale);
                    set_query(&mut gallery, index, "Async draft");
                    match phase {
                        "ready" => {}
                        "pending" => gallery.workbench.state(index).activate(20, "async"),
                        "failed" => {
                            gallery.workbench.state(index).activate(20, "async");
                            gallery.workbench.state(index).activate(21, "async");
                        }
                        "saved" => {
                            gallery.workbench.state(index).activate(20, "async");
                            gallery.workbench.state(index).activate(21, "async");
                            gallery.workbench.state(index).activate(20, "async");
                            gallery.workbench.state(index).activate(21, "async");
                        }
                        _ => unreachable!(),
                    }
                    let expected_phase = match phase {
                        "ready" => super::Phase::Ready,
                        "pending" => super::Phase::Pending,
                        "failed" => super::Phase::Failed,
                        "saved" => super::Phase::Saved,
                        _ => unreachable!(),
                    };
                    stabilize_document(&mut gallery);
                    assert_async_state(&gallery, index, expected_phase);
                    let range = gallery.page_scroll_range().max(0.0);
                    let (offset, actual_range) =
                        settle_scroll(&mut gallery, fraction, range * fraction);
                    let mut scene = Scene::new();
                    gallery.build_scene(&mut scene);
                    assert_async_state(&gallery, index, expected_phase);
                    let state = gallery
                        .workbench
                        .states
                        .get(&index)
                        .expect("mounted Async state");
                    let input = state.input.frame_at(0).expect("real Async input frame");
                    let notification = state.buttons[&25]
                        .layout()
                        .expect("notification action layout")
                        .frame;
                    let save = state.buttons[&20]
                        .layout()
                        .expect("save action layout")
                        .frame;
                    let phase_frame = state.phase_frame.expect("Async phase frame");
                    let log = state.event_log_frame.expect("Async event-log frame");
                    assert!(notification.max_y() + 11.9 <= save.min_y());
                    assert!(save.max_y() + 11.9 <= phase_frame.min_y());
                    assert!(phase_frame.max_y() + 24.0 <= log.min_y());
                    assert!((offset - actual_range * fraction).abs() < 0.5);
                    let state_record = serde_json::json!({
                        "phase": phase,
                        "attempts": state.attempts,
                        "edited_input": state.input.query(),
                        "draft_fixture": state.draft_fixture.is_some(),
                        "save_disabled": state.buttons[&20].state.disabled,
                        "save_label": state.button_labels[&20],
                        "event_log": state.log.iter().cloned().collect::<Vec<_>>(),
                        "frames": {
                            "input": rect(input),
                            "notification": rect(notification),
                            "save": rect(save),
                            "phase": rect(phase_frame),
                            "event_log": rect(log)
                        }
                    });
                    manifest.push(capture_page(
                        &mut renderer,
                        &mut gallery,
                        &output,
                        CaptureSpec {
                            index,
                            renderer: "async",
                            state: phase,
                            theme,
                            scale,
                            position,
                            offset,
                            scroll_range: actual_range,
                            state_record,
                            pre_modal_scroll: false,
                        },
                    ));
                    assert_async_state(&gallery, index, expected_phase);
                }
            }
        }
    }

    assert_eq!(manifest.len(), 64);
    write_manifest(&output, &manifest);
}

#[test]
#[ignore = "requires allocated GPU and fresh TEKESUI_PATTERN_READING_OUT child directories"]
fn pattern_danger_reading_states_capture_real_gallery_pages() {
    let output = fresh_output("danger");
    let mut renderer = Renderer::new_headless().expect("actual GPU required for Danger captures");
    let index = route_index("pattern-danger", "danger");
    let cases = ["modal-open", "cancelled", "pending", "failed", "saved"];
    let mut manifest = Vec::new();

    for case in cases {
        for theme in Theme::ALL {
            for scale in [1.0_f32, 2.0] {
                for (position, fraction) in POSITIONS {
                    let mut gallery = new_gallery("pattern-danger/playground", index, theme, scale);
                    let mut pre_modal_offset = None;
                    let mut pre_modal_range = None;
                    let mut preparation_offset = None;
                    let mut preparation_range = None;

                    if case == "modal-open" {
                        stabilize_document(&mut gallery);
                        let range = gallery.page_scroll_range().max(0.0);
                        let (before_offset, before_range) =
                            settle_scroll(&mut gallery, fraction, range * fraction);
                        pre_modal_offset = Some(before_offset);
                        pre_modal_range = Some(before_range);
                        gallery.workbench.state(index).activate(20, "danger");
                        stabilize_document(&mut gallery);
                        assert!(gallery.workbench.state(index).dialog_view.dialog().open);
                        assert_eq!(gallery.workbench.state(index).phase, super::Phase::Ready);
                        assert_eq!(gallery.workbench.state(index).attempts, 0);
                        assert_eq!(gallery.viewport.offset.y, before_offset);
                    } else {
                        stabilize_document(&mut gallery);
                        let range = gallery.page_scroll_range().max(0.0);
                        let (bottom_offset, actual_range) = settle_scroll(&mut gallery, 1.0, range);
                        assert!((bottom_offset - actual_range).abs() < 0.5);
                        preparation_offset = Some(bottom_offset);
                        preparation_range = Some(actual_range);
                        drive_danger_state(&mut gallery, index, case);
                        stabilize_document(&mut gallery);
                        let range = gallery.page_scroll_range().max(0.0);
                        let (offset, actual_range) =
                            settle_scroll(&mut gallery, fraction, range * fraction);
                        assert!((offset - actual_range * fraction).abs() < 0.5);
                    }

                    let mut scene = Scene::new();
                    gallery.build_scene(&mut scene);
                    let state = gallery
                        .workbench
                        .states
                        .get(&index)
                        .expect("mounted Danger state");
                    assert_danger_state(state, case);
                    let phase_frame = state.phase_frame.expect("Danger phase frame");
                    let log = state.event_log_frame.expect("Danger event-log frame");
                    assert!(phase_frame.max_y() + 24.0 <= log.min_y());
                    let dialog = state.dialog_layout.as_ref().map(|layout| {
                        serde_json::json!({
                            "visible": layout.visible.map(rect),
                            "modal": rect(layout.modal),
                            "title": rect(layout.title),
                            "focus": layout.focus,
                            "buttons_visible": layout.buttons.iter()
                                .map(|button| button.visible.map(rect))
                                .collect::<Vec<_>>()
                        })
                    });
                    let offset = gallery.viewport.offset.y;
                    let actual_range = gallery.page_scroll_range().max(0.0);
                    if case == "modal-open" {
                        assert_eq!(offset, pre_modal_offset.expect("recorded modal origin"));
                    } else {
                        assert!((offset - actual_range * fraction).abs() < 0.5);
                    }
                    let state_record = serde_json::json!({
                        "state": case,
                        "phase": format!("{:?}", state.phase),
                        "attempts": state.attempts,
                        "dialog_open": state.dialog_view.dialog().open,
                        "event_log": state.log.iter().cloned().collect::<Vec<_>>(),
                        "position_origin": if case == "modal-open" { "pre_modal_scroll" } else { "closed_modal_scroll" },
                        "pre_modal_offset": pre_modal_offset,
                        "pre_modal_scroll_range": pre_modal_range,
                        "dialog_preparation_offset": preparation_offset,
                        "dialog_preparation_scroll_range": preparation_range,
                        "frames": {
                            "phase": rect(phase_frame),
                            "event_log": rect(log),
                            "dialog": dialog
                        }
                    });
                    manifest.push(capture_page(
                        &mut renderer,
                        &mut gallery,
                        &output,
                        CaptureSpec {
                            index,
                            renderer: "danger",
                            state: case,
                            theme,
                            scale,
                            position,
                            offset,
                            scroll_range: actual_range,
                            state_record,
                            pre_modal_scroll: case == "modal-open",
                        },
                    ));
                }
            }
        }
    }

    assert_eq!(manifest.len(), 80);
    write_manifest(&output, &manifest);
}

fn fresh_output(name: &str) -> std::path::PathBuf {
    let root = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_PATTERN_READING_OUT")
            .expect("TEKESUI_PATTERN_READING_OUT must name a fresh parent directory"),
    );
    assert!(root.is_dir(), "the parent must provision {root:?}");
    let output = root.join(name);
    std::fs::create_dir(&output).expect("refuse to overwrite existing pattern capture output");
    output
}

fn route_index(slug: &str, renderer: &str) -> usize {
    let index = entries()
        .iter()
        .position(|entry| entry.slug() == slug)
        .unwrap_or_else(|| panic!("registered Gallery route missing: {slug}"));
    assert_eq!(entries()[index].get("kind"), "pattern");
    assert_eq!(entries()[index].renderer(), renderer);
    index
}

fn new_gallery(route: &str, index: usize, theme: Theme, scale: f32) -> Gallery {
    let mut gallery = Gallery::with_static_text(StaticText::new(None));
    gallery.set_system_theme(theme);
    gallery.set_reduced_motion(true);
    gallery.workbench.text_scale = 2;
    gallery.text.set_scale(scale);
    assert!(
        gallery.open_route(route),
        "registered pattern route: {route}"
    );
    assert_eq!(gallery.page, Page::Workbench(index));
    gallery.set_view_size(720.0, 512.0);
    gallery.build_scene(&mut Scene::new());
    assert_eq!(gallery.theme(), theme);
    assert_eq!(gallery.text.type_scale(), 1.5);
    assert_eq!(gallery.workbench.state(index).brand, 0);
    gallery
}

fn set_query(gallery: &mut Gallery, index: usize, query: &str) {
    gallery.build_scene(&mut Scene::new());
    assert!(gallery.workbench.focus_control(index, 1000).is_some());
    gallery.handle_input(&Event::TextCommand("selectAll:".into()));
    gallery.handle_input(&Event::TextCommit {
        text: query.to_owned(),
        replacement: None,
    });
    assert_eq!(gallery.workbench.state(index).input.query(), query);
}

fn assert_async_state(gallery: &Gallery, index: usize, expected: super::Phase) {
    let state = gallery
        .workbench
        .states
        .get(&index)
        .expect("mounted Async state");
    let attempts = match expected {
        super::Phase::Ready => 0,
        super::Phase::Pending | super::Phase::Failed => 1,
        super::Phase::Saved => 2,
        super::Phase::Invalid => unreachable!(),
    };
    assert_eq!(state.phase, expected);
    assert_eq!(state.attempts, attempts);
    assert_eq!(state.input.query(), "Async draft");
    assert!(state.draft_fixture.is_none());
    assert_eq!(
        state.buttons[&20].state.disabled,
        expected == super::Phase::Pending
    );
    assert_eq!(
        state.button_labels[&20],
        if expected == super::Phase::Failed {
            "Retry save"
        } else {
            "Save settings"
        }
    );
    assert_eq!(
        state.buttons.contains_key(&21),
        expected == super::Phase::Pending
    );
    assert!(state.field.error.is_none());
    let expected_log: &[&str] = match expected {
        super::Phase::Ready => &[],
        super::Phase::Pending => &["submit: pending (Resolve fixture to complete)"],
        super::Phase::Failed => &[
            "submit: pending (Resolve fixture to complete)",
            "result: simulated failure; retry is available",
        ],
        super::Phase::Saved => &[
            "submit: pending (Resolve fixture to complete)",
            "result: simulated failure; retry is available",
            "submit: pending (Resolve fixture to complete)",
            "result: saved",
        ],
        super::Phase::Invalid => unreachable!(),
    };
    assert_eq!(
        state.log.iter().map(String::as_str).collect::<Vec<_>>(),
        expected_log
    );
}

fn drive_danger_state(gallery: &mut Gallery, index: usize, case: &str) {
    let open = |gallery: &mut Gallery| {
        gallery.workbench.state(index).activate(20, "danger");
        gallery.build_scene(&mut Scene::new());
        let state = gallery
            .workbench
            .states
            .get(&index)
            .expect("mounted Danger state");
        assert!(state.dialog_view.dialog().open);
        assert!(
            state
                .dialog_layout
                .as_ref()
                .and_then(|layout| layout.buttons.get(1))
                .is_some_and(|button| button.visible.is_some()),
            "the Confirm button must be visible to its real DialogView key event"
        );
    };
    let confirm = |gallery: &mut Gallery| {
        let blocked = gallery
            .workbench
            .state(index)
            .dialog_view
            .key(ControlKey::Tab { reverse: false }, true);
        assert_eq!(blocked, DialogEvent::Blocked);
        gallery
            .workbench
            .state(index)
            .dialog_event(blocked, "danger", false);
        assert_eq!(gallery.workbench.state(index).dialog_view.dialog().focus, 1);
        let event = gallery
            .workbench
            .state(index)
            .dialog_view
            .key(ControlKey::Enter, true);
        assert!(matches!(event, DialogEvent::Confirmed { .. }));
        gallery
            .workbench
            .state(index)
            .dialog_event(event, "danger", false);
        gallery.build_scene(&mut Scene::new());
        assert!(!gallery.workbench.state(index).dialog_view.dialog().open);
    };
    match case {
        "cancelled" => {
            open(gallery);
            let event = gallery
                .workbench
                .state(index)
                .dialog_view
                .key(ControlKey::Escape, true);
            assert!(matches!(event, DialogEvent::Cancelled { .. }));
            gallery
                .workbench
                .state(index)
                .dialog_event(event, "danger", false);
            assert!(!gallery.workbench.state(index).dialog_view.dialog().open);
        }
        "pending" => {
            open(gallery);
            confirm(gallery);
        }
        "failed" => {
            open(gallery);
            confirm(gallery);
            gallery.workbench.state(index).activate(21, "danger");
        }
        "saved" => {
            open(gallery);
            confirm(gallery);
            gallery.workbench.state(index).activate(21, "danger");
            assert_eq!(gallery.workbench.state(index).phase, super::Phase::Failed);
            open(gallery);
            confirm(gallery);
            gallery.workbench.state(index).activate(21, "danger");
        }
        _ => unreachable!("modal-open is scrolled before opening"),
    }
}

fn assert_danger_state(state: &super::StoryState, case: &str) {
    let (phase, attempts, dialog_open, expected_log): (super::Phase, usize, bool, &[&str]) =
        match case {
            "modal-open" => (super::Phase::Ready, 0, true, &["dialog: open"]),
            "cancelled" => (
                super::Phase::Ready,
                0,
                false,
                &["dialog: open", "dialog: cancel; focus restored"],
            ),
            "pending" => (
                super::Phase::Pending,
                1,
                false,
                &[
                    "dialog: open",
                    "submit: pending (Resolve fixture to complete)",
                    "dialog: confirm; focus restored",
                ],
            ),
            "failed" => (
                super::Phase::Failed,
                1,
                false,
                &[
                    "dialog: open",
                    "submit: pending (Resolve fixture to complete)",
                    "dialog: confirm; focus restored",
                    "result: simulated failure; retry is available",
                ],
            ),
            "saved" => (
                super::Phase::Saved,
                2,
                false,
                &[
                    "dialog: open",
                    "submit: pending (Resolve fixture to complete)",
                    "dialog: confirm; focus restored",
                    "result: simulated failure; retry is available",
                    "dialog: open",
                    "submit: pending (Resolve fixture to complete)",
                    "dialog: confirm; focus restored",
                    "result: saved",
                ],
            ),
            _ => unreachable!(),
        };
    assert_eq!(state.phase, phase);
    assert_eq!(state.attempts, attempts);
    assert_eq!(state.dialog_view.dialog().open, dialog_open);
    assert!(state.draft_fixture.is_none());
    assert_eq!(
        state.log.iter().map(String::as_str).collect::<Vec<_>>(),
        expected_log
    );
}

fn capture_page(
    renderer: &mut Renderer,
    gallery: &mut Gallery,
    output: &std::path::Path,
    spec: CaptureSpec<'_>,
) -> serde_json::Value {
    let CaptureSpec {
        index,
        renderer: renderer_name,
        state: state_name,
        theme,
        scale,
        position,
        offset: expected_offset,
        scroll_range,
        state_record,
        pre_modal_scroll,
    } = spec;
    let route = format!("{}/playground", entries()[index].slug());
    assert_eq!(gallery.page, Page::Workbench(index));
    assert_eq!(gallery.theme(), theme);
    assert_eq!(
        gallery
            .workbench
            .states
            .get(&index)
            .expect("mounted capture state")
            .brand,
        0
    );
    assert_eq!(gallery.text.type_scale(), 1.5);
    let mut scene = Scene::new();
    gallery.build_scene(&mut scene);
    assert!(
        !scene.is_empty(),
        "real Gallery scene required for {route}/{state_name}"
    );
    let actual_offset = gallery.viewport.offset.y;
    let actual_range = gallery.page_scroll_range().max(0.0);
    assert!((actual_offset - expected_offset).abs() < 0.5);
    if !pre_modal_scroll {
        let fraction = if position == "top" { 0.0 } else { 1.0 };
        assert!((actual_offset - actual_range * fraction).abs() < 0.5);
    }

    let pixel_width = (720.0 * scale) as u32;
    let pixel_height = (512.0 * scale) as u32;
    let rgba = renderer
        .render_to_rgba8(
            &scene,
            gallery.atlas_mut(),
            (pixel_width, pixel_height),
            scale,
            Gallery::background(theme),
        )
        .expect("actual Gallery pattern GPU raster");
    let non_background_pixels = rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]);
    assert!(
        non_background_pixels,
        "blank real Gallery raster for {route}/{state_name}"
    );
    let file = format!(
        "{}__{}__{}__720x512__type-150__{}@{}x.png",
        entries()[index].slug(),
        state_name,
        theme_slug(theme),
        position,
        scale as u32
    );
    let path = output.join(&file);
    let mut encoder = png::Encoder::new(
        std::fs::File::create(&path).expect("create fresh pattern PNG"),
        pixel_width,
        pixel_height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("write pattern PNG header")
        .write_image_data(&rgba)
        .expect("write pattern PNG pixels");
    serde_json::json!({
        "file": file,
        "route": route,
        "entry_index": index,
        "renderer": renderer_name,
        "state": state_name,
        "theme": theme_slug(theme),
        "brand": 0,
        "viewport": {"width": 720, "height": 512, "type_scale": 1.5},
        "pixels": {"width": pixel_width, "height": pixel_height},
        "backing_scale": scale,
        "position": position,
        "requested_offset": expected_offset,
        "settled_scroll_range": scroll_range,
        "actual_offset": actual_offset,
        "actual_scroll_range": actual_range,
        "document_height": gallery.page_document_height(),
        "position_origin": if pre_modal_scroll { "scroll_completed_before_dialog_open" } else { "closed_modal_page_scroll" },
        "non_background_pixels": non_background_pixels,
        "state_record": state_record,
        "visual_review": "unverified",
        "evidence": "real Gallery scene on headless GPU; fixture events are programmatic owner/view calls, not native mouse, keyboard, IME, VoiceOver or performance acceptance"
    })
}

fn rect(rect: Rect) -> [f32; 4] {
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

fn write_manifest(output: &std::path::Path, manifest: &[serde_json::Value]) {
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(manifest).expect("serialize pattern capture manifest"),
    )
    .expect("write pattern capture manifest");
}
