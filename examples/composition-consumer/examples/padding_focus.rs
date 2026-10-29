//! Thin native host for the existing editable and additive selectable-readonly consumers.
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
    use tekes_ui_composition_consumer::padding_focus as f;
    use tekes_ui_controls::{TextEditBaseline, composition::*};
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, Menu, MenuBar, MenuCommand, MenuItem, Window,
        WindowConfig, WindowHandler, content_accessibility as ax, selftest::Action,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};

    struct Host {
        tree: ContentHost,
        text: coretext::Text,
        gpu: Option<Renderer>,
        clock: std::time::Instant,
        readonly: bool,
        started: bool,
        stages: Vec<u32>,
        baseline: Option<TextEditBaseline>,
        other_baseline: Option<TextEditBaseline>,
        editor_mount: u64,
        blocked_padding: usize,
        direct_text_down: usize,
        actions: Vec<ContentActionRecord>,
    }
    impl Host {
        fn edge_points(&self) -> [Point; 4] {
            let band = self.tree.get(f::BAND).unwrap().frame();
            [
                Point::new(band.min_x() + 1., band.center().y),
                Point::new(band.max_x() - 1., band.center().y),
                Point::new(band.center().x, band.min_y() + 1.),
                Point::new(band.center().x, band.max_y() - 1.),
            ]
        }
        fn click(point: Point) -> Action {
            Action::Click {
                x: point.x.into(),
                y: point.y.into(),
            }
        }
        fn field_baseline(&self) -> TextEditBaseline {
            self.tree
                .text_surface(f::EDITOR)
                .unwrap()
                .input
                .external_baseline()
        }
        fn record(&self, window: &Window, label: &str) {
            let input = &self.tree.text_surface(f::EDITOR).unwrap().input;
            let other = &self.tree.text_surface(f::OTHER).unwrap().input;
            let value = serde_json::json!({"event":label,"readonly_case":self.readonly,
                "window_active":window.is_active(),"stages":self.stages,
                "focus":self.tree.focused().map(|id|id.0),"editor_mount":self.tree.semantic_mount(f::EDITOR),
                "editor":{"text":input.text,"selection":input.selection(),"marked":input.marked,"read_only":input.read_only},
                "other":{"text":other.text,"selection":other.selection(),"marked":other.marked},
                "blocked_padding":self.blocked_padding,"direct_text_down":self.direct_text_down,
                "actions":self.actions.iter().map(|a|serde_json::json!({"target":a.action.target.0,"mount":a.mount,"source":format!("{:?}",a.action.source)})).collect::<Vec<_>>(),
                "software_AppKit":true,"physical_IME":false,"VoiceOver":false});
            eprintln!("PADDING30_NATIVE {value}");
            if let Some(directory) = std::env::var_os("TEKESUI_PADDING_NATIVE_EVIDENCE") {
                use std::io::Write;
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(directory.join("events.jsonl"))
                    .unwrap();
                writeln!(file, "{value}").unwrap();
            }
        }
        fn draw(&mut self, window: &mut Window) {
            self.text.now = self.clock.elapsed();
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.requested = false;
            let size = window.size();
            let viewport = Rect::new(0., 0., size.width as f32, size.height as f32);
            let frame = viewport.inset(20.);
            let palette = Palette::from(Theme::LIGHT);
            let mut scene = Scene::new();
            self.tree
                .render(&mut scene, &mut self.text, frame, viewport, palette)
                .unwrap();
            if let Some(gpu) = &mut self.gpu {
                gpu.render(
                    &scene,
                    &mut self.text.atlas,
                    palette.color("surface-1").unwrap(),
                )
                .unwrap();
            }
            let layouts = ax::content_text_layouts(&self.tree);
            let input = self
                .tree
                .focused()
                .and_then(|id| ax::content_key(&self.tree, id, None))
                .and_then(|key| layouts.get(&key))
                .map(|layout| layout.state.clone());
            window.set_text_input_state(input);
            window.set_accessibility_text_layouts(layouts.into_iter().collect());
            window.set_accessibility_nodes(ax::content_nodes(&self.tree));
            window.set_animating(self.text.requested);
            if !self.started
                && window.is_active()
                && std::env::var_os("TEKESUI_PADDING_NATIVE_SELFTEST").is_some()
            {
                self.started = true;
                p::selftest::run_script(vec![(200, Action::MenuCommand(30_001))]);
            }
        }
        fn phase(&mut self, stage: u32, window: &Window) -> Vec<(u64, Action)> {
            assert!(window.is_active());
            assert_eq!(stage, 30_001 + self.stages.len() as u32);
            let next = |id| Action::MenuCommand(id);
            let editor = ax::content_key(&self.tree, f::EDITOR, None)
                .unwrap()
                .component;
            let other = ax::content_key(&self.tree, f::OTHER, None)
                .unwrap()
                .component;
            let selection = if self.readonly { 1..3 } else { 7..9 };
            let script = match stage {
                30_001 => {
                    assert!(self.actions.is_empty());
                    assert_eq!(self.tree.focused(), None);
                    self.editor_mount = self.tree.semantic_mount(f::EDITOR).unwrap();
                    self.baseline = Some(self.field_baseline());
                    vec![
                        (100, Self::click(self.edge_points()[0])),
                        (350, next(30_002)),
                    ]
                }
                30_002 => {
                    assert_eq!(self.tree.focused(), Some(f::EDITOR));
                    assert_eq!(self.field_baseline(), *self.baseline.as_ref().unwrap());
                    assert_eq!(self.tree.semantic_mount(f::EDITOR), Some(self.editor_mount));
                    vec![
                        (
                            100,
                            Action::AXSelect {
                                component: editor,
                                range: selection.clone(),
                            },
                        ),
                        (350, next(30_003)),
                    ]
                }
                30_003 => {
                    assert_eq!(
                        self.tree.text_surface(f::EDITOR).unwrap().input.selection(),
                        selection
                    );
                    self.baseline = Some(self.field_baseline());
                    let mut steps = self
                        .edge_points()
                        .into_iter()
                        .enumerate()
                        .map(|(i, point)| (100 + i as u64 * 150, Self::click(point)))
                        .collect::<Vec<_>>();
                    steps.push((850, next(30_004)));
                    steps
                }
                30_004 => {
                    assert_eq!(self.field_baseline(), *self.baseline.as_ref().unwrap());
                    assert_eq!(
                        self.tree.text_surface(f::EDITOR).unwrap().input.read_only,
                        self.readonly
                    );
                    assert!(self.actions.is_empty());
                    vec![
                        (
                            100,
                            Self::click(self.tree.text_surface(f::EDITOR).unwrap().inner.center()),
                        ),
                        (350, next(30_005)),
                    ]
                }
                30_005 => {
                    assert!(self.direct_text_down > 0);
                    assert_eq!(self.tree.focused(), Some(f::EDITOR));
                    self.baseline = Some(self.field_baseline());
                    let mut steps = vec![(
                        100,
                        Action::PreeditRange {
                            text: "ni".into(),
                            selection: 2..2,
                            replacement: None,
                        },
                    )];
                    if self.readonly {
                        steps.push((250, Action::TypeAscii("x".into())));
                    }
                    steps.push((500, next(30_006)));
                    steps
                }
                30_006 => {
                    let input = &self.tree.text_surface(f::EDITOR).unwrap().input;
                    if self.readonly {
                        assert!(input.marked.is_none());
                        assert_eq!(self.field_baseline(), *self.baseline.as_ref().unwrap());
                    } else {
                        assert!(input.marked.is_some());
                    }
                    self.baseline = Some(self.field_baseline());
                    vec![
                        (100, Self::click(self.edge_points()[0])),
                        (350, next(30_007)),
                    ]
                }
                30_007 => {
                    assert_eq!(self.field_baseline(), *self.baseline.as_ref().unwrap());
                    vec![
                        (100, Action::Unmark),
                        (
                            300,
                            Action::AXSelect {
                                component: other,
                                range: 0..0,
                            },
                        ),
                        (
                            500,
                            Action::PreeditRange {
                                text: "hold".into(),
                                selection: 4..4,
                                replacement: None,
                            },
                        ),
                        (800, next(30_008)),
                    ]
                }
                30_008 => {
                    assert_eq!(self.tree.focused(), Some(f::OTHER));
                    assert!(self.tree.focused_text().unwrap().input.marked.is_some());
                    self.other_baseline =
                        Some(self.tree.focused_text().unwrap().input.external_baseline());
                    vec![
                        (100, Self::click(self.edge_points()[0])),
                        (350, next(30_009)),
                    ]
                }
                30_009 => {
                    assert!(self.blocked_padding > 0);
                    assert_eq!(self.tree.focused(), Some(f::OTHER));
                    assert_eq!(
                        self.tree.focused_text().unwrap().input.external_baseline(),
                        *self.other_baseline.as_ref().unwrap()
                    );
                    vec![
                        (100, Action::Unmark),
                        (350, Self::click(self.edge_points()[1])),
                        (600, next(30_010)),
                    ]
                }
                30_010 => {
                    assert_eq!(self.tree.focused(), Some(f::EDITOR));
                    assert!(self.actions.is_empty());
                    vec![
                        (
                            100,
                            Self::click(self.tree.get(f::ACTION).unwrap().frame().center()),
                        ),
                        (350, next(30_011)),
                    ]
                }
                30_011 => {
                    assert_eq!(self.actions.len(), 1);
                    assert_eq!(self.actions[0].action.target, f::ACTION);
                    assert_eq!(self.actions[0].action.source, ContentActionSource::Pointer);
                    assert_eq!(self.tree.focused(), Some(f::ACTION));
                    vec![
                        (
                            100,
                            Action::KeyCharacters {
                                key_code: 48,
                                chars: "\t",
                            },
                        ),
                        (350, next(30_012)),
                    ]
                }
                30_012 => {
                    assert_eq!(self.tree.focused(), Some(f::OTHER));
                    vec![
                        (
                            100,
                            Action::AXSelect {
                                component: editor,
                                range: selection,
                            },
                        ),
                        (350, next(30_013)),
                    ]
                }
                30_013 => {
                    assert_eq!(self.tree.focused(), Some(f::EDITOR));
                    assert_eq!(self.tree.semantic_mount(f::EDITOR), Some(self.editor_mount));
                    assert_eq!(
                        self.tree
                            .semantics()
                            .iter()
                            .filter(|node| node.id == f::EDITOR)
                            .count(),
                        1
                    );
                    if self.readonly {
                        assert_eq!(
                            self.tree.text_surface(f::EDITOR).unwrap().input.text,
                            f::READ_ONLY_TEXT
                        );
                    }
                    let mut steps = Vec::new();
                    if let Some(directory) = std::env::var_os("TEKESUI_PADDING_NATIVE_EVIDENCE") {
                        steps.push((
                            100,
                            Action::Capture {
                                path: std::path::PathBuf::from(directory)
                                    .join("original-owner-selection.png")
                                    .to_string_lossy()
                                    .into_owned(),
                            },
                        ));
                    }
                    steps.push((700, Action::Close));
                    steps
                }
                _ => unreachable!(),
            };
            self.stages.push(stage);
            self.record(window, &format!("verified-{stage}"));
            script
        }
        fn event(&mut self, event: ContentEvent, window: &Window) {
            let intent = self.tree.padding_focus_target(&event);
            if f::padding_ime_admission(&self.tree, &event) == Some(false) {
                self.blocked_padding += 1;
                self.record(window, "host-blocked-cross-owner-padding");
                return;
            }
            let direct = matches!(
                &event,
                ContentEvent::PointerDown(_)
                    | ContentEvent::Pointer(PointerEvent {
                        phase: PointerPhase::Down,
                        button: Some(PointerButton::Primary),
                        ..
                    })
            ) && intent.is_none();
            let response = self.tree.event_with_text(&event, &mut self.text).unwrap();
            if intent.is_some() {
                assert!(
                    !response.capture_pointer,
                    "padding must never acquire capture"
                );
            }
            if direct && self.tree.focused() == Some(f::EDITOR) {
                self.direct_text_down += 1;
            }
            self.actions.extend(self.tree.drain_action_records());
            self.record(
                window,
                &format!("input intent={intent:?} response={response:?}"),
            );
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            self.tree.set_window_active(window.is_active());
            match event {
                Event::Created => {
                    assert!(window.set_text_key_events(true));
                    self.gpu = Some(
                        unsafe {
                            Renderer::new_for_metal_layer(
                                window.metal_layer(),
                                window.drawable_size(),
                                window.scale() as f32,
                            )
                        }
                        .unwrap(),
                    );
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
                Event::Focus(active) => self.tree.set_window_active(active),
                Event::SystemMotionChanged(reduced) => self.text.reduced_motion = reduced,
                Event::Command(p::CommandId(stage))
                    if self.started && (30_001..=30_013).contains(&stage) =>
                {
                    let script = self.phase(stage, window);
                    self.draw(window);
                    p::selftest::run_script(script);
                    return;
                }
                Event::Mouse(mouse) => self.event(native_pointer::translate(mouse), window),
                Event::Key(key) => self.event(ContentEvent::KeyInput(native_key::key(key)), window),
                Event::TextKey(key) => self.event(native_key::translate(key), window),
                Event::TextCommit { text, replacement } => {
                    self.event(ContentEvent::Commit { text, replacement }, window)
                }
                Event::TextPreedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                } => self.event(
                    ContentEvent::Preedit {
                        text,
                        selection,
                        clauses,
                        replacement,
                    },
                    window,
                ),
                Event::TextUnmark => self.event(ContentEvent::Unmark, window),
                Event::TextCommand(command) => {
                    self.event(ContentEvent::TextCommand(command), window)
                }
                Event::Accessibility { target, action } => {
                    ax::content_action(&mut self.tree, target, action);
                    self.actions.extend(self.tree.drain_action_records());
                }
                Event::CloseRequested => {
                    if self.started {
                        assert_eq!(self.stages.len(), 13);
                        self.record(window, "normal-close");
                    }
                    self.tree.set_window_active(false);
                    window.set_accessibility_nodes(Vec::new());
                    window.set_accessibility_text_layouts(Default::default());
                    window.close();
                    return;
                }
                Event::Redraw | Event::AnimationFrame(_) => {}
                _ => return,
            }
            self.draw(window);
        }
    }
    let readonly = std::env::args().nth(1).as_deref() != Some("editable");
    let tree = if readonly {
        f::build_read_only()
    } else {
        f::build()
    }
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    App::new(AppConfig {
        name: Some("TekesUI Padding Focus Consumer".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: format!(
                "TekesUI Padding Focus · {}",
                if readonly { "readonly" } else { "editable" }
            ),
            size: p::Size {
                width: 720.,
                height: 480.,
            },
            ..Default::default()
        },
        if std::env::var_os("TEKESUI_PADDING_NATIVE_SELFTEST").is_some() {
            MenuBar {
                menus: vec![Menu::new(
                    "Padding acceptance",
                    (30_001..=30_013)
                        .map(|stage| {
                            MenuItem::Command(MenuCommand::new(
                                format!("Verify phase {stage}"),
                                p::CommandId(stage),
                            ))
                        })
                        .collect(),
                )],
            }
        } else {
            MenuBar::default()
        },
        Host {
            tree,
            text,
            gpu: None,
            clock: std::time::Instant::now(),
            readonly,
            started: false,
            stages: vec![],
            baseline: None,
            other_baseline: None,
            editor_mount: 0,
            blocked_padding: 0,
            direct_text_down: 0,
            actions: vec![],
        },
    );
}
