//! Native exercise for the original ContentHost-owned interactive Popover.
#[cfg(target_os = "macos")]
#[path = "../tests/support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[path = "support/native_key.rs"]
mod native_key;
#[cfg(target_os = "macos")]
#[path = "support/native_pointer.rs"]
mod native_pointer;
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use tekes_ui_composition_consumer::popover_content::Scenario;
    use tekes_ui_controls::composition::{ContentEvent, TextFieldContent};
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler,
        content_accessibility as ax,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};

    struct Host {
        scenario: Scenario,
        text: coretext::Text,
        gpu: Option<Renderer>,
        clock: std::time::Instant,
        review_started: bool,
        review_phase: u32,
    }
    impl Host {
        fn actions(&mut self) {
            self.scenario.process();
        }
        fn draw(&mut self, window: &mut Window) {
            self.text.now = self.clock.elapsed();
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.requested = false;
            let width = window.size().width as f32;
            let height = window.size().height as f32;
            let frame = Rect::new(16., 16., (width - 32.).max(0.), (height - 32.).max(0.));
            let viewport = Rect::new(0., 0., width.max(0.), height.max(0.));
            let palette = Palette::from(Theme::LIGHT);
            let mut scene = Scene::new();
            self.scenario
                .host
                .render(&mut scene, &mut self.text, frame, viewport, palette)
                .unwrap();
            if let Some(gpu) = &mut self.gpu {
                gpu.render(
                    &scene,
                    &mut self.text.atlas,
                    tekes_ui_tokens::surface_palette::resolve_surface(
                        palette, "canvas", "content", "rest",
                    )
                    .expect("canonical canvas content")
                    .background,
                )
                .unwrap();
            }
            let layouts = ax::content_text_layouts(&self.scenario.host);
            let input = self
                .scenario
                .host
                .focused()
                .and_then(|id| ax::content_key(&self.scenario.host, id, None))
                .and_then(|key| layouts.get(&key))
                .map(|layout| layout.state.clone());
            window.set_text_input_state(input);
            window.set_accessibility_text_layouts(layouts.into_iter().collect());
            window.set_accessibility_nodes(ax::content_nodes(&self.scenario.host));
            window.set_animating(self.text.requested);
            if !self.review_started
                && window.is_active()
                && std::env::var_os("TEKESUI_POPOVER_NATIVE_SELFTEST").is_some()
            {
                use p::selftest::Action;
                use tekes_ui_composition_consumer::popover_content::{CALLER, EDITOR, SAVE};
                let initial_len = self
                    .scenario
                    .host
                    .get(EDITOR)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap()
                    .surface
                    .input
                    .text
                    .encode_utf16()
                    .count();
                let phase = Action::MenuCommand;
                self.review_started = true;
                p::selftest::run_script(vec![
                    (
                        200,
                        Action::AXPress {
                            component: self.scenario.host.get(CALLER).unwrap().mount_identity(),
                        },
                    ),
                    (600, phase(49_001)),
                    (
                        800,
                        Action::TextRange {
                            text: "draft😀".into(),
                            replacement: Some(0..initial_len),
                        },
                    ),
                    (
                        1100,
                        Action::PreeditRange {
                            text: "拼音".into(),
                            selection: 2..2,
                            replacement: None,
                        },
                    ),
                    (
                        1400,
                        Action::ExpectTextState {
                            text: "draft😀拼音".into(),
                            selection: 9..9,
                            marked: Some(7..9),
                            query: 7..9,
                            actual: 7..9,
                        },
                    ),
                    (1600, phase(49_002)),
                    (
                        1800,
                        Action::KeyCharacters {
                            key_code: 53,
                            chars: "\u{1b}",
                        },
                    ),
                    (2200, phase(49_003)),
                    (2400, Action::Unmark),
                    (2600, phase(49_004)),
                    (
                        2800,
                        Action::AXSelect {
                            component: self.scenario.host.get(EDITOR).unwrap().mount_identity(),
                            range: 0..2,
                        },
                    ),
                    (
                        3100,
                        Action::TextRange {
                            text: "OK".into(),
                            replacement: Some(0..2),
                        },
                    ),
                    (3300, phase(49_005)),
                    (3500, Action::TextCommand("undo:")),
                    (3800, phase(49_006)),
                    (
                        4000,
                        Action::KeyCharacters {
                            key_code: 48,
                            chars: "\t",
                        },
                    ),
                    (4300, phase(49_007)),
                    (4500, Action::Click { x: 8., y: 8. }),
                    (4900, phase(49_008)),
                    (
                        5200,
                        Action::AXPress {
                            component: self.scenario.host.get(CALLER).unwrap().mount_identity(),
                        },
                    ),
                    (5600, phase(49_009)),
                    (
                        5800,
                        Action::AXPress {
                            component: self.scenario.host.get(SAVE).unwrap().mount_identity(),
                        },
                    ),
                    (6200, phase(49_010)),
                    (
                        6500,
                        Action::AXPress {
                            component: self.scenario.host.get(CALLER).unwrap().mount_identity(),
                        },
                    ),
                    (6900, phase(49_011)),
                    (7200, Action::Miniaturize),
                    (8000, Action::Deminiaturize),
                    (8600, phase(49_013)),
                    (9000, Action::Close),
                ]);
            }

            if let Some(path) = std::env::var_os("TEKESUI_POPOVER_EVIDENCE") {
                let path = std::path::PathBuf::from(path);
                std::fs::create_dir_all(&path).unwrap();
                let editor = self
                    .scenario
                    .host
                    .get(tekes_ui_composition_consumer::popover_content::EDITOR)
                    .and_then(|node| node.component::<TextFieldContent>());
                let state = serde_json::json!({
                    "pid": std::process::id(),
                    "phase": self.review_phase,
                    "nodes": self.scenario.host.semantics().iter().map(|node| serde_json::json!({"id":node.id.0,"name":node.semantics.name,"role":format!("{:?}",node.semantics.role)})).collect::<Vec<_>>(),
                    "active": window.is_active(),
                    "popover_open": self.scenario.is_open(),
                    "popover_presented": self.scenario.is_presented(),
                    "focused": self.scenario.host.focused().map(|id| id.0),
                    "editor_mount": self.scenario.host.semantic_mount(tekes_ui_composition_consumer::popover_content::EDITOR),
                    "text": editor.map(|field| field.surface.input.text.as_str()),
                    "selection": editor.map(|field| field.surface.input.selection()),
                    "marked": editor.and_then(|field| field.surface.input.marked.clone()),
                    "semantic_nodes": ax::content_nodes(&self.scenario.host).len(),
                });
                std::fs::write(
                    path.join("state.json"),
                    serde_json::to_vec_pretty(&state).unwrap(),
                )
                .unwrap();
                use std::io::Write;
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path.join("journal.jsonl"))
                        .unwrap(),
                    "{state}"
                )
                .unwrap();
            }
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::Created => {
                    window.set_text_key_events(true);
                    self.text.services = true;
                    self.gpu = Some(unsafe {
                        Renderer::new_for_metal_layer(
                            window.metal_layer(),
                            window.drawable_size(),
                            window.scale() as f32,
                        )
                        .unwrap()
                    });
                }
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => {
                    if let Some(gpu) = &mut self.gpu {
                        gpu.resize(drawable_size, scale as f32);
                    }
                }
                Event::SystemMotionChanged(reduced) => self.text.reduced_motion = reduced,
                Event::Focus(active) => {
                    self.scenario.host.set_window_active(active);
                    if self.review_started && !active {
                        self.review_phase = 49_012;
                    }
                }
                Event::Command(p::CommandId(id))
                    if self.review_started && (49_001..=49_013).contains(&id) =>
                {
                    self.review_phase = id;
                }
                Event::Mouse(mouse) => {
                    self.scenario.host.event(&native_pointer::translate(mouse));
                    self.actions();
                }
                Event::Scroll(scroll) => {
                    self.scenario
                        .host
                        .event_with_text(
                            &ContentEvent::Scroll {
                                position: Point::new(
                                    scroll.position.x as f32,
                                    scroll.position.y as f32,
                                ),
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
                        .unwrap();
                }
                Event::Key(key) => {
                    self.scenario
                        .host
                        .event_with_text(
                            &ContentEvent::KeyInput(native_key::key(key)),
                            &mut self.text,
                        )
                        .unwrap();
                    self.actions();
                }
                Event::TextKey(event) => {
                    self.scenario
                        .host
                        .event_with_text(&native_key::translate(event), &mut self.text)
                        .unwrap();
                    self.actions();
                }
                Event::TextCommit { text, replacement } => {
                    self.scenario
                        .host
                        .event_with_text(
                            &ContentEvent::Commit { text, replacement },
                            &mut self.text,
                        )
                        .unwrap();
                }
                Event::TextPreedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                } => {
                    self.scenario
                        .host
                        .event_with_text(
                            &ContentEvent::Preedit {
                                text,
                                selection,
                                clauses,
                                replacement,
                            },
                            &mut self.text,
                        )
                        .unwrap();
                }
                Event::TextUnmark => {
                    self.scenario
                        .host
                        .event_with_text(&ContentEvent::Unmark, &mut self.text)
                        .unwrap();
                }
                Event::TextCommand(command) => {
                    self.scenario
                        .host
                        .event_with_text(&ContentEvent::TextCommand(command), &mut self.text)
                        .unwrap();
                    self.actions();
                }
                Event::Accessibility { target, action } => {
                    ax::content_action(&mut self.scenario.host, target, action);
                    self.actions();
                }
                Event::CloseRequested => {
                    self.scenario.deactivate();
                    window.set_accessibility_text_layouts(Default::default());
                    window.set_accessibility_nodes(Vec::new());
                    eprintln!("POPOVER_NATIVE_CLOSE_REQUESTED");
                    window.close();
                    return;
                }
                Event::Redraw | Event::AnimationFrame(_) => {}
                _ => return,
            }
            self.scenario.host.set_window_active(window.is_active());
            self.draw(window);
        }
    }
    let mut scenario = Scenario::new().unwrap();
    scenario.host.set_window_active(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    App::new(AppConfig {
        name: Some("TekesUI PopoverContent Review".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI PopoverContent Review".into(),
            size: p::Size {
                width: 720.,
                height: 640.,
            },
            min_size: Some(p::Size {
                width: 360.,
                height: 480.,
            }),
            placement: p::WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
            ..Default::default()
        },
        if std::env::var_os("TEKESUI_POPOVER_NATIVE_SELFTEST").is_some() {
            MenuBar {
                menus: vec![p::Menu::new(
                    "Acceptance",
                    (49_001..=49_013)
                        .map(|id| {
                            p::MenuItem::Command(p::MenuCommand::new(
                                format!("Acceptance {id}"),
                                p::CommandId(id),
                            ))
                        })
                        .collect(),
                )],
            }
        } else {
            MenuBar::default()
        },
        Host {
            scenario,
            text,
            gpu: None,
            clock: std::time::Instant::now(),
            review_started: false,
            review_phase: 0,
        },
    );
}
