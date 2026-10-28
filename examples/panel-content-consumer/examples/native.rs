//! Native acceptance entry for the unchanged original Issue #82 Panel consumer tree.
#[cfg(target_os = "macos")]
#[path = "../tests/support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[path = "../../composition-consumer/examples/support/native_key.rs"]
mod native_key;
#[cfg(target_os = "macos")]
#[path = "../../composition-consumer/examples/support/native_pointer.rs"]
mod native_pointer;
#[cfg(target_os = "macos")]
#[path = "../tests/support/panel_fixture.rs"]
mod panel_fixture;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Issue #82 native consumer requires macOS");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_controls::composition::{
        Content, ContentEvent, ContentHost, ContentResponse, PanelContent, PointerPhase,
        ScrollContent, TextFieldContent,
    };
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler,
        content_accessibility::ContentAccessibilitySnapshot,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};

    struct Host {
        tree: ContentHost,
        text: coretext::Text,
        gpu: Option<Renderer>,
        evidence: Option<std::path::PathBuf>,
        closed: Rc<Cell<bool>>,
        script_started: bool,
        curve_probe_points: Option<(Point, Point)>,
        scroll_events: usize,
        action_records: usize,
        curve_outside_seen: bool,
        curve_inside_seen: bool,
        curve_outside_actions: usize,
        curve_inside_actions: usize,
        curve_inside_original_actions: usize,
        verified_stages: Vec<u32>,
    }
    impl Host {
        fn text_state(surface: &tekes_ui_controls::TextSurface) -> p::TextInputState {
            let caret = surface.caret();
            let carets = surface
                .paragraph
                .as_ref()
                .map_or_else(Vec::new, |paragraph| {
                    paragraph
                        .lines
                        .iter()
                        .flat_map(|line| {
                            line.carets.iter().map(move |(index, x)| {
                                (
                                    *index,
                                    p::Point {
                                        x: surface.origin().x as f64 + x,
                                        y: surface.origin().y as f64 + line.top,
                                    },
                                    p::Size {
                                        width: tekes_ui_tokens::Stroke::Rule.value() as f64,
                                        height: line.height,
                                    },
                                )
                            })
                        })
                        .collect()
                });
            p::TextInputState {
                text: surface.input.text.clone(),
                caret_index: surface.input.caret,
                selection: surface.input.selection(),
                marked: surface.input.marked.clone(),
                read_only: surface.input.read_only,
                caret_origin: p::Point {
                    x: caret.min_x() as f64,
                    y: caret.min_y() as f64,
                },
                caret_size: p::Size {
                    width: caret.size.width as f64,
                    height: caret.size.height as f64,
                },
                carets,
            }
        }
        fn state(
            &self,
            window: &Window,
            event: &str,
            response: Option<ContentResponse>,
            point: Option<Point>,
        ) -> serde_json::Value {
            let editor = self
                .tree
                .get(panel_fixture::EDITOR)
                .and_then(|node| node.component::<TextFieldContent>());
            let input = self.tree.text_input_snapshot();
            let scroll_node = self.tree.get(panel_fixture::SCROLL);
            let scroll = scroll_node.and_then(|node| node.component::<ScrollContent>());
            let reading = scroll.and_then(ScrollContent::reading_result);
            let panel_geometry = self
                .tree
                .get(panel_fixture::ROOT)
                .and_then(|node| node.component::<PanelContent>())
                .and_then(PanelContent::geometry);
            let hit = point.and_then(|point| {
                self.tree
                    .geometry_snapshot()
                    .filter(|g| g.is_current())
                    .and_then(|g| g.hit_test(point))
            });
            serde_json::json!({
                "event": event,
                "window_active": window.is_active(),
                "focused": self.tree.focused().map(|id| id.0),
                "editor_mount": self.tree.semantic_mount(panel_fixture::EDITOR),
                "native_key_binding": self.tree.native_key_binding(),
                "text": input.as_ref().map(|s| s.text.as_str()),
                "selection_utf16": input.as_ref().map(|s| format!("{}..{}", s.selection.start, s.selection.end)),
                "marked_utf16": input.as_ref().and_then(|s| s.marked.as_ref()).map(|r| format!("{}..{}", r.start, r.end)),
                "is_composing": self.tree.is_composing(),
                "editor_internal_scroll_offset": editor.map(|field| format!("{:?}", field.surface.scroll.offset)),
                "scroll_mount": self.tree.semantic_mount(panel_fixture::SCROLL),
                "scroll_offset": scroll.map(|owner| format!("{:?}", owner.scroll.offset)),
                "scroll_limit": scroll.map(|owner| format!("{:?}", owner.scroll.limit())),
                "scroll_following_latest": scroll.map(ScrollContent::is_following_latest),
                "scroll_reading_result": reading.map(|result| format!("{:?}", result)),
                "reading_scroll_source": self.tree.reading_scroll_source().map(|(id, mount)| (id.0, mount)),
                "panel_geometry_generation": panel_geometry.map(|geometry| geometry.generation),
                "panel_geometry": panel_geometry.map(|geometry| format!("{:?}", geometry)),
                "geometry_current": self.tree.geometry_snapshot().map(|snapshot| snapshot.is_current()),
                "text_now_seconds": self.text.now.as_secs_f64(),
                "text_reduced_motion": self.text.reduced_motion,
                "point": point.map(|point| format!("{:?}", point)),
                "curve_hit": hit.map(|(id, mount, part)| (id.0, mount, part)),
                "response": response.map(|response| format!("{:?}", response)),
                "semantic_mounts": ([panel_fixture::ROOT, panel_fixture::ACTION, panel_fixture::EDITOR, panel_fixture::SCROLL]
                    .into_iter().map(|id| (id.0, self.tree.semantic_mount(id))).collect::<Vec<_>>()),
            })
        }
        fn record(
            &self,
            window: &Window,
            event: &str,
            response: Option<ContentResponse>,
            point: Option<Point>,
        ) {
            let value = self.state(window, event, response, point);
            self.emit(value);
        }
        fn emit(&self, value: serde_json::Value) {
            eprintln!("PANEL82_NATIVE {value}");
            if let Some(directory) = &self.evidence {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(directory.join("events.jsonl"))
                    .expect("open native event journal");
                writeln!(file, "{value}").expect("write native event journal");
            }
        }
        fn verify_stage(&mut self, stage: u32, window: &Window) {
            let input = self.tree.text_input_snapshot();
            match stage {
                82_001 => {
                    let offset = self
                        .tree
                        .get(panel_fixture::SCROLL)
                        .and_then(|node| node.component::<ScrollContent>())
                        .map(|owner| owner.scroll.offset.y)
                        .unwrap_or_default();
                    assert!(self.scroll_events > 0, "native wheel reached Event::Scroll");
                    assert!(
                        offset.abs() > 0.01,
                        "native wheel changed original ScrollContent offset: {offset}"
                    );
                }
                82_002 => {
                    assert!(
                        self.curve_outside_seen,
                        "real pointer path exercised the curved outside corner"
                    );
                    assert!(
                        self.curve_inside_seen,
                        "real pointer path exercised an inside Button target"
                    );
                    assert_eq!(
                        self.curve_outside_actions, 0,
                        "curved outside click dispatched no action"
                    );
                    assert!(
                        self.curve_inside_actions == 1 && self.curve_inside_original_actions == 1,
                        "inside Button click dispatched an action owned by its original ID/mount"
                    );
                    assert!(
                        self.action_records > 0,
                        "original action owner produced a receipt"
                    );
                }
                82_003 => {
                    assert_eq!(
                        input.as_ref().map(|s| s.text.as_str()),
                        Some("draft 0123456789")
                    );
                    assert_eq!(input.as_ref().map(|s| s.selection.clone()), Some(6..10));
                    assert!(input.as_ref().is_some_and(|s| s.marked.is_none()));
                }
                82_004 => {
                    let text = input.as_ref().map(|s| s.text.as_str()).unwrap_or_default();
                    assert!(
                        text.starts_with("scroll reveal row 0\n"),
                        "long replacement remains installed before undo"
                    );
                    let offset = self
                        .tree
                        .get(panel_fixture::EDITOR)
                        .and_then(|node| node.component::<TextFieldContent>())
                        .map(|field| field.surface.scroll.offset.y)
                        .unwrap_or_default();
                    assert!(
                        offset.abs() > 0.01,
                        "long multiline caret reveal moved TextField scroll offset: {offset}"
                    );
                }
                82_005 => {
                    assert_eq!(
                        input.as_ref().map(|s| s.text.as_str()),
                        Some("draft 0123456789")
                    );
                    assert!(input.as_ref().is_some_and(|s| s.marked.is_none()));
                }
                82_006 => {
                    assert!(
                        self.tree
                            .focused()
                            .is_some_and(|id| id != panel_fixture::EDITOR),
                        "native Tab moved focus from the editor"
                    );
                }
                _ => panic!("unknown Issue #82 acceptance phase {stage}"),
            }
            self.verified_stages.push(stage);
            self.record(window, &format!("verified-stage-{stage}"), None, None);
        }
        fn draw(&mut self, window: &mut Window) {
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.requested = false;
            let width = window.size().width as f32;
            let height = window.size().height as f32;
            let viewport = Rect::new(0., 0., width.max(0.), height.max(0.));
            // The application owns the outside inset (#45); keep the actual
            // Panel corner inside the content view, away from NSWindow resize chrome.
            let inset = tekes_ui_tokens::proposal::metric("spacing", "space-loose")
                .expect("canonical application outside inset");
            let frame = Rect::new(
                inset,
                inset,
                (width - 2. * inset).max(0.),
                (height - 2. * inset).max(0.),
            );
            let palette = Palette::from(Theme::LIGHT);
            let mut scene = Scene::new();
            if let Err(error) =
                self.tree
                    .render(&mut scene, &mut self.text, frame, viewport, palette)
            {
                panic!("original Panel ContentHost render failed: {error:?}");
            }
            if let Some(gpu) = &mut self.gpu
                && let Err(error) = gpu.render(
                    &scene,
                    &mut self.text.atlas,
                    tekes_ui_tokens::surface_palette::resolve_surface(
                        palette, "canvas", "content", "rest",
                    )
                    .expect("canonical canvas content")
                    .background,
                )
            {
                let glyphs: usize = scene
                    .primitives()
                    .iter()
                    .map(|primitive| {
                        if let tekes_ui_scene::Primitive::Glyphs(run) = primitive {
                            run.glyphs.len()
                        } else {
                            0
                        }
                    })
                    .sum();
                eprintln!(
                    "PANEL82_RENDER_FAILURE error={error:?} backing={} logical={}x{} drawable={:?} primitives={} glyphs={glyphs}",
                    window.scale(),
                    width,
                    height,
                    window.drawable_size(),
                    scene.primitives().len()
                );
                if let Some(directory) = &self.evidence {
                    std::fs::write(directory.join("failed-scene.txt"), format!("{scene:#?}"))
                        .expect("preserve actual rejected Scene");
                }
                panic!("render original Panel scene through Metal: {error:?}");
            }
            let focused = self.tree.focused();
            let input = self
                .tree
                .is_window_active()
                .then(|| focused.and_then(|id| self.tree.text_surface(id)))
                .flatten()
                .map(Self::text_state);
            window.set_text_input_state(input);
            if self.tree.is_window_active() {
                let snapshot = ContentAccessibilitySnapshot::from_host(&self.tree)
                    .expect("current curved Panel geometry produces an accessibility snapshot");
                assert!(
                    window.set_content_accessibility_snapshot(snapshot),
                    "native Window accepted the current original Host accessibility snapshot"
                );
            } else {
                let _ = window.set_content_accessibility_publication(None);
            }
            window.set_animating(self.text.requested);
            self.record(window, "frame", None, None);

            if !self.script_started
                && window.is_active()
                && std::env::var_os("TEKESUI_PANEL82_NATIVE_SELFTEST").is_some()
            {
                use p::selftest::Action;
                let scroll_point = self
                    .tree
                    .get(panel_fixture::SCROLL)
                    .and_then(|node| node.visible())
                    .expect("original ScrollContent has a laid out visible viewport")
                    .center();
                let button_point = self
                    .tree
                    .get(panel_fixture::ACTION)
                    .and_then(|node| node.visible())
                    .expect("original Button has a laid out visible frame")
                    .center();
                let panel_geometry = self
                    .tree
                    .get(panel_fixture::ROOT)
                    .and_then(|node| node.component::<PanelContent>())
                    .and_then(PanelContent::geometry)
                    .expect("original root Panel has current rounded geometry");
                let corner_radius = panel_geometry.shape.radii().top_left;
                assert!(
                    corner_radius > 2.,
                    "the original Panel top-left is actually rounded: {corner_radius}"
                );
                let curve_outside = Point::new(
                    panel_geometry.outer.min_x() + corner_radius * 0.25,
                    panel_geometry.outer.min_y() + corner_radius * 0.25,
                );
                let before_pointer_script = self
                    .tree
                    .geometry_snapshot()
                    .expect("original Panel geometry is current before pointer script");
                assert!(before_pointer_script.is_current());
                let outside_hit = before_pointer_script.hit_test(curve_outside);
                assert!(
                    outside_hit.is_none(),
                    "the actual rounded corner rejects its bounding-box point: {outside_hit:?}"
                );
                let inside_hit = before_pointer_script.hit_test(button_point);
                assert!(
                    inside_hit.is_some_and(|(id, mount, _)| {
                        id == panel_fixture::ACTION
                            && Some(mount) == self.tree.semantic_mount(panel_fixture::ACTION)
                    }),
                    "the actual inside probe belongs to the original Button owner/mount: {inside_hit:?}"
                );
                self.curve_probe_points = Some((curve_outside, button_point));
                let editor_point = self
                    .tree
                    .get(panel_fixture::EDITOR)
                    .and_then(|node| node.visible())
                    .expect("original TextField has a laid out visible frame")
                    .center();
                let editor_mount = self
                    .tree
                    .get(panel_fixture::EDITOR)
                    .expect("original TextField owner")
                    .mount_identity();
                let long_text = (0..80)
                    .map(|i| format!("scroll reveal row {i}\n"))
                    .collect::<String>();
                self.script_started = true;
                p::selftest::run_script(vec![
                    (
                        200,
                        Action::ScrollWheel {
                            x: f64::from(scroll_point.x),
                            y: f64::from(scroll_point.y),
                            delta_x: 0,
                            delta_y: -6,
                        },
                    ),
                    (500, Action::MenuCommand(82_001)),
                    (
                        700,
                        Action::Click {
                            x: f64::from(curve_outside.x),
                            y: f64::from(curve_outside.y),
                        },
                    ),
                    (
                        1000,
                        Action::Click {
                            x: f64::from(button_point.x),
                            y: f64::from(button_point.y),
                        },
                    ),
                    (1400, Action::MenuCommand(82_002)),
                    (
                        1700,
                        Action::Click {
                            x: f64::from(editor_point.x),
                            y: f64::from(editor_point.y),
                        },
                    ),
                    (
                        2050,
                        Action::AXSelect {
                            component: editor_mount,
                            range: 6..10,
                        },
                    ),
                    (
                        2350,
                        Action::ExpectTextState {
                            text: "draft 0123456789".into(),
                            selection: 6..10,
                            marked: None,
                            query: 6..10,
                            actual: 6..10,
                        },
                    ),
                    (
                        2650,
                        Action::TextRange {
                            text: "é😀".into(),
                            replacement: Some(6..10),
                        },
                    ),
                    (
                        3000,
                        Action::ExpectTextState {
                            text: "draft é😀456789".into(),
                            selection: 9..9,
                            marked: None,
                            query: 7..9,
                            actual: 7..9,
                        },
                    ),
                    (3300, Action::TextCommand("undo:")),
                    (
                        3600,
                        Action::ExpectTextState {
                            text: "draft 0123456789".into(),
                            selection: 6..10,
                            marked: None,
                            query: 6..10,
                            actual: 6..10,
                        },
                    ),
                    (3900, Action::MenuCommand(82_003)),
                    (
                        4200,
                        Action::TextRange {
                            text: long_text,
                            replacement: Some(0..16),
                        },
                    ),
                    (5000, Action::MenuCommand(82_004)),
                    (5300, Action::TextCommand("undo:")),
                    (5700, Action::MenuCommand(82_005)),
                    (
                        6000,
                        Action::KeyCharacters {
                            key_code: 53,
                            chars: "\u{1b}",
                        },
                    ),
                    (
                        6300,
                        Action::KeyCharacters {
                            key_code: 48,
                            chars: "\t",
                        },
                    ),
                    (6600, Action::MenuCommand(82_006)),
                    (7000, Action::Close),
                ]);
            }
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::Created => {
                    assert!(
                        window.set_text_key_events(true),
                        "enable native text-key provenance"
                    );
                    self.text.services = true;
                    self.tree.set_window_active(true);
                    self.gpu = Some(unsafe {
                        Renderer::new_for_metal_layer(
                            window.metal_layer(),
                            window.drawable_size(),
                            window.scale() as f32,
                        )
                        .expect("create Metal renderer for native Panel window")
                    });
                    self.record(window, "created", None, None);
                }
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => {
                    if let Some(gpu) = &mut self.gpu {
                        gpu.resize(drawable_size, scale as f32);
                    }
                    self.record(window, "resized", None, None);
                }
                Event::Focus(active) => {
                    self.tree.set_window_active(active);
                    self.record(
                        window,
                        if active {
                            "focus-active"
                        } else {
                            "focus-inactive"
                        },
                        None,
                        None,
                    );
                }
                Event::Mouse(mouse) => {
                    let event = native_pointer::translate(mouse);
                    let point = match &event {
                        ContentEvent::Pointer(pointer) => Some(pointer.position),
                        _ => None,
                    };
                    let focused_before = self.tree.focused();
                    let geometry_before = self.tree.geometry_snapshot();
                    let geometry_current_before = geometry_before
                        .as_ref()
                        .is_some_and(|geometry| geometry.is_current());
                    let pre_hit = point.and_then(|point| {
                        geometry_before
                            .as_ref()
                            .and_then(|geometry| geometry.hit_test(point))
                    });
                    let pointer_down = matches!(
                        &event,
                        ContentEvent::Pointer(pointer) if pointer.phase == PointerPhase::Down
                    );
                    let probe = point.and_then(|point| {
                        self.curve_probe_points.and_then(|(outside, inside)| {
                            if (point.x - outside.x).abs() < 0.01
                                && (point.y - outside.y).abs() < 0.01
                            {
                                Some("curve-outside")
                            } else if (point.x - inside.x).abs() < 0.01
                                && (point.y - inside.y).abs() < 0.01
                            {
                                Some("curve-inside-button")
                            } else {
                                None
                            }
                        })
                    });
                    if pointer_down && let Some(probe) = probe {
                        assert!(
                            geometry_before
                                .as_ref()
                                .is_some_and(|geometry| geometry.is_current()),
                            "{probe} pointer event begins with current frozen geometry"
                        );
                        match probe {
                            "curve-outside" => assert!(
                                pre_hit.is_none(),
                                "actual rounded corner has no route target before dispatch: {pre_hit:?}"
                            ),
                            "curve-inside-button" => assert!(
                                pre_hit.is_some_and(|(id, mount, _)| {
                                    id == panel_fixture::ACTION
                                        && Some(mount)
                                            == self.tree.semantic_mount(panel_fixture::ACTION)
                                }),
                                "actual inside point belongs to Button before dispatch: {pre_hit:?}"
                            ),
                            _ => unreachable!(),
                        }
                    }
                    let response = self.tree.event(&event);
                    let actions = self.tree.drain_action_records();
                    self.action_records += actions.len();
                    if let Some(probe) = probe {
                        match probe {
                            "curve-outside" => {
                                self.curve_outside_seen |= pointer_down;
                                self.curve_outside_actions += actions.len();
                            }
                            "curve-inside-button" => {
                                self.curve_inside_seen |= pointer_down;
                                self.curve_inside_actions += actions.len();
                                let button_mount = self.tree.semantic_mount(panel_fixture::ACTION);
                                self.curve_inside_original_actions += actions
                                    .iter()
                                    .filter(|record| {
                                        record.action.target == panel_fixture::ACTION
                                            && Some(record.mount) == button_mount
                                    })
                                    .count();
                            }
                            _ => unreachable!(),
                        }
                    }
                    let mut value = self.state(
                        window,
                        &format!("mouse actions={actions:?}"),
                        Some(response),
                        point,
                    );
                    value["pointer_probe"] = serde_json::json!(probe);
                    value["pre_event_geometry_current"] =
                        serde_json::json!(geometry_current_before);
                    value["pre_event_curve_hit"] =
                        serde_json::json!(pre_hit.map(|(id, mount, part)| (id.0, mount, part)));
                    value["focused_before"] = serde_json::json!(focused_before.map(|id| id.0));
                    value["focused_after"] = serde_json::json!(self.tree.focused().map(|id| id.0));
                    value["action_records"] = serde_json::json!(
                        actions.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>()
                    );
                    self.emit(value);
                }
                Event::Scroll(scroll) => {
                    self.scroll_events += 1;
                    let point = Point::new(scroll.position.x as f32, scroll.position.y as f32);
                    let response = self
                        .tree
                        .event_with_text(
                            &ContentEvent::Scroll {
                                position: point,
                                delta: Point::new(-scroll.delta.x as f32, -scroll.delta.y as f32),
                                precise: scroll.precise,
                                begin: matches!(
                                    scroll.phase,
                                    p::ScrollPhase::Began | p::ScrollPhase::MayBegin
                                ),
                                end: matches!(
                                    scroll.phase,
                                    p::ScrollPhase::Ended | p::ScrollPhase::Cancelled
                                ),
                                momentum: !matches!(scroll.momentum_phase, p::ScrollPhase::None),
                                time: scroll.timestamp,
                            },
                            &mut self.text,
                        )
                        .expect("original ContentHost accepted native wheel event");
                    self.record(
                        window,
                        &format!(
                            "scroll native_delta={:?} precise={}",
                            scroll.delta, scroll.precise
                        ),
                        Some(response),
                        Some(point),
                    );
                }
                Event::Key(key) => {
                    let response = self
                        .tree
                        .event_with_text(
                            &ContentEvent::KeyInput(native_key::key(key)),
                            &mut self.text,
                        )
                        .expect("original ContentHost accepted native key event");
                    let actions = self.tree.drain_action_records();
                    self.record(
                        window,
                        &format!("key actions={actions:?}"),
                        Some(response),
                        None,
                    );
                }
                Event::TextKey(event) => {
                    let response = self
                        .tree
                        .event_with_text(&native_key::translate(event), &mut self.text)
                        .expect("original ContentHost accepted native text-key marker");
                    self.record(window, "text-key", Some(response), None);
                }
                Event::TextCommit { text, replacement } => {
                    self.tree
                        .event_with_text(
                            &ContentEvent::Commit { text, replacement },
                            &mut self.text,
                        )
                        .expect("original ContentHost accepted native text commit");
                    self.record(window, "text-commit", None, None);
                }
                Event::TextPreedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                } => {
                    self.tree
                        .event_with_text(
                            &ContentEvent::Preedit {
                                text,
                                selection,
                                clauses,
                                replacement,
                            },
                            &mut self.text,
                        )
                        .expect("original ContentHost accepted native marked-text update");
                    self.record(window, "text-preedit", None, None);
                }
                Event::TextUnmark => {
                    self.tree
                        .event_with_text(&ContentEvent::Unmark, &mut self.text)
                        .expect("original ContentHost accepted native unmark");
                    self.record(window, "text-unmark", None, None);
                }
                Event::TextCommand(command) => {
                    self.tree
                        .event_with_text(&ContentEvent::TextCommand(command), &mut self.text)
                        .expect("original ContentHost accepted native text command");
                    let actions = self.tree.drain_action_records();
                    self.record(
                        window,
                        &format!("text-command actions={actions:?}"),
                        None,
                        None,
                    );
                }
                Event::Accessibility { target, action } => {
                    let action_label = format!("{action:?}");
                    let response = window
                        .content_accessibility_publication()
                        .and_then(|publication| {
                            let owner = publication.owner(target)?;
                            Some(publication.dispatch(owner, &mut self.tree, target, action))
                        })
                        .unwrap_or_default();
                    let actions = self.tree.drain_action_records();
                    self.record(
                        window,
                        &format!(
                            "accessibility target={target:?} action={action_label} actions={actions:?}"
                        ),
                        Some(response),
                        None,
                    );
                }
                Event::SystemAppearanceChanged { dark, .. } => {
                    self.record(
                        window,
                        if dark {
                            "appearance-dark"
                        } else {
                            "appearance-light"
                        },
                        None,
                        None,
                    );
                }
                Event::SystemMotionChanged(reduced_motion) => {
                    self.text.reduced_motion = reduced_motion;
                    self.record(
                        window,
                        &format!("system-motion reduced_motion={reduced_motion}"),
                        None,
                        None,
                    );
                }
                Event::Command(p::CommandId(command @ 82_001..=82_006)) if self.script_started => {
                    self.verify_stage(command, window);
                }
                Event::Command(p::CommandId(command)) => {
                    self.record(window, &format!("menu-command {command}"), None, None);
                }
                Event::CloseRequested => {
                    if self.script_started {
                        assert_eq!(
                            self.verified_stages,
                            (82_001..=82_006).collect::<Vec<_>>(),
                            "all native Panel acceptance stages completed before scripted close"
                        );
                    }
                    self.tree.set_window_active(false);
                    window.set_text_input_state(None);
                    let _ = window.set_content_accessibility_publication(None);
                    self.record(window, "close-requested", None, None);
                    window.close();
                    self.closed.set(true);
                    return;
                }
                Event::Redraw | Event::AnimationFrame(_) | Event::Wake => {}
                other => self.record(window, &format!("other={other:?}"), None, None),
            }
            self.tree.set_window_active(window.is_active());
            self.draw(window);
        }
    }

    let row_ids = panel_fixture::row_ids();
    let tree = Content::new(
        panel_fixture::ROOT,
        PanelContent::new(panel_fixture::panel_body(
            panel_fixture::composer(),
            &row_ids,
        ))?,
    );
    let mut host = ContentHost::new(tree)?;
    host.set_action_events(true);
    host.set_scroll_reading_document(
        panel_fixture::SCROLL,
        "issue82-main-row",
        1,
        row_ids,
        |_| {},
    )
    .expect("attach the original reading document to its ScrollContent owner");
    let evidence =
        std::env::var_os("TEKESUI_PANEL82_NATIVE_EVIDENCE").map(std::path::PathBuf::from);
    if let Some(directory) = &evidence {
        std::fs::create_dir_all(directory)?;
    }
    let closed = Rc::new(Cell::new(false));
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let menu_bar = MenuBar {
        menus: vec![p::Menu::new(
            "Panel Acceptance",
            (82_001..=82_006)
                .map(|id| {
                    p::MenuItem::Command(p::MenuCommand::new(
                        format!("Acceptance phase {id}"),
                        p::CommandId(id),
                    ))
                })
                .collect(),
        )],
    };
    App::new(AppConfig {
        name: Some("TekesUI Issue 82 Panel Consumer".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI Issue 82 Panel Consumer".into(),
            size: p::Size {
                width: 720.,
                height: 640.,
            },
            min_size: Some(p::Size {
                width: 360.,
                height: 360.,
            }),
            placement: p::WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
            ..Default::default()
        },
        menu_bar,
        Host {
            tree: host,
            text,
            gpu: None,
            evidence,
            closed: closed.clone(),
            script_started: false,
            curve_probe_points: None,
            scroll_events: 0,
            action_records: 0,
            curve_outside_seen: false,
            curve_inside_seen: false,
            curve_outside_actions: 0,
            curve_inside_actions: 0,
            curve_inside_original_actions: 0,
            verified_stages: Vec::new(),
        },
    );
    if !closed.get() {
        return Err("native window run returned without CloseRequested -> Window::close".into());
    }
    Ok(())
}
