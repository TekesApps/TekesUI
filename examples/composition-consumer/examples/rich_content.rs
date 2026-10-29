//! Independent rich-document native host; original input, AX and selftest adapters.
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
    use tekes_ui_composition_consumer::rich_content as f;
    use tekes_ui_controls::{TextEditBaseline, composition::*};
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, Menu, MenuBar, MenuCommand, MenuItem, Window,
        WindowConfig, WindowHandler, content_accessibility as ax, selftest::Action,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};

    struct Host {
        tree: ContentHost,
        text: coretext::Text,
        gpu: Option<Renderer>,
        clock: std::time::Instant,
        started: bool,
        stages: Vec<u32>,
        baseline: Option<TextEditBaseline>,
        composer_baseline: Option<TextEditBaseline>,
        mount: u64,
        theme: Theme,
        links: Vec<RichLinkAction>,
    }
    impl Host {
        fn record(&self, window: &Window, label: &str) {
            let input = &self.tree.text_surface(f::MARKDOWN).unwrap().input;
            let composer = &self.tree.text_surface(f::COMPOSER).unwrap().input;
            let value = serde_json::json!({"event":label,"window_active":window.is_active(),"stages":self.stages,
                "focus":self.tree.focused().map(|id|id.0),"mount":self.tree.semantic_mount(f::MARKDOWN),
                "markdown":{"text":input.text,"selection":input.selection(),"read_only":input.read_only,"marked":input.marked},
                "composer":{"text":composer.text,"selection":composer.selection(),"marked":composer.marked},
                "version":f::rich(&self.tree,f::MARKDOWN).version().revision(),
                "blocks":f::rich(&self.tree,f::MARKDOWN).block_geometry().len(),
                "links":self.links.iter().map(|link|serde_json::json!({"document":link.document,"revision":link.revision,"target":link.target,"source":format!("{:?}",link.source)})).collect::<Vec<_>>(),
                "software_AppKit":true,"physical_IME":false,"VoiceOver":false});
            eprintln!("RICH25_NATIVE {value}");
            if let Some(directory) = std::env::var_os("TEKESUI_RICH_NATIVE_EVIDENCE") {
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
        fn collect_links(&mut self) {
            for id in [f::MARKDOWN, f::CODE, f::DIFF] {
                self.links
                    .extend(f::rich_mut(&mut self.tree, id).drain_link_actions());
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
            let palette = Palette::from(self.theme);
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
                && std::env::var_os("TEKESUI_RICH_NATIVE_SELFTEST").is_some()
            {
                self.started = true;
                p::selftest::run_script(vec![(200, Action::MenuCommand(25_001))]);
            }
        }
        fn phase(&mut self, stage: u32, window: &Window) -> Vec<(u64, Action)> {
            assert!(window.is_active());
            assert_eq!(stage, 25_001 + self.stages.len() as u32);
            let md = ax::content_key(&self.tree, f::MARKDOWN, None)
                .unwrap()
                .component;
            let composer = ax::content_key(&self.tree, f::COMPOSER, None)
                .unwrap()
                .component;
            let text = f::rich(&self.tree, f::MARKDOWN)
                .document()
                .text()
                .to_owned();
            let length = text.encode_utf16().count();
            let byte = text.find("中文😀").unwrap();
            let start = text[..byte].encode_utf16().count();
            let range = start..start + 4;
            let next = |n| Action::MenuCommand(n);
            let script = match stage {
                25_001 => {
                    self.mount = self.tree.semantic_mount(f::MARKDOWN).unwrap();
                    vec![
                        (
                            100,
                            Action::AXSelect {
                                component: md,
                                range: range.clone(),
                            },
                        ),
                        (
                            180,
                            Action::ExpectAXText {
                                component: md,
                                text: text.clone(),
                                selection: range.clone(),
                                selected: "中文😀".into(),
                                query: range.clone(),
                                substring: Some("中文😀".into()),
                            },
                        ),
                        (
                            220,
                            Action::ExpectTextState {
                                text: text.clone(),
                                selection: range.clone(),
                                marked: None,
                                query: range.clone(),
                                actual: range.clone(),
                            },
                        ),
                        (
                            240,
                            Action::ExpectTextRect {
                                range: range.clone(),
                                actual: range.clone(),
                                nonzero_width: true,
                            },
                        ),
                        (280, Action::TextCommand("copy:")),
                        (350, next(25_002)),
                    ]
                }
                25_002 => {
                    assert_eq!(window.clipboard_text().as_deref(), Some("中文😀"));
                    self.baseline = Some(
                        self.tree
                            .text_surface(f::MARKDOWN)
                            .unwrap()
                            .input
                            .external_baseline(),
                    );
                    vec![
                        (100, Action::Text("forbidden".into())),
                        (150, Action::Preedit("ni".into())),
                        (
                            200,
                            Action::AXReplace {
                                component: md,
                                text: "forbidden".into(),
                            },
                        ),
                        (300, next(25_003)),
                    ]
                }
                25_003 => {
                    assert_eq!(
                        Some(
                            self.tree
                                .text_surface(f::MARKDOWN)
                                .unwrap()
                                .input
                                .external_baseline()
                        ),
                        self.baseline
                    );
                    let key = ax::content_key(&self.tree, f::MARKDOWN, Some(0)).unwrap();
                    vec![(100, Action::AXPressKey { key }), (250, next(25_004))]
                }
                25_004 => {
                    assert_eq!(self.links.len(), 1);
                    assert_eq!(self.links[0].source, ContentActionSource::Accessibility);
                    assert_eq!(self.links[0].target, "https://example.com/docs");
                    vec![
                        (
                            100,
                            Action::AXSelect {
                                component: composer,
                                range: 0..0,
                            },
                        ),
                        (200, Action::Preedit("hold".into())),
                        (320, next(25_005)),
                    ]
                }
                25_005 => {
                    assert_eq!(self.tree.focused(), Some(f::COMPOSER));
                    assert!(
                        self.tree
                            .text_surface(f::COMPOSER)
                            .unwrap()
                            .input
                            .marked
                            .is_some()
                    );
                    self.composer_baseline = Some(
                        self.tree
                            .text_surface(f::COMPOSER)
                            .unwrap()
                            .input
                            .external_baseline(),
                    );
                    f::append(&mut self.tree);
                    self.theme = Theme::DARK;
                    self.text.scale = 1.3;
                    vec![
                        (100, Action::Resize { w: 360., h: 760. }),
                        (350, next(25_006)),
                    ]
                }
                25_006 => {
                    assert_eq!(self.tree.focused(), Some(f::COMPOSER));
                    assert_eq!(
                        Some(
                            self.tree
                                .text_surface(f::COMPOSER)
                                .unwrap()
                                .input
                                .external_baseline()
                        ),
                        self.composer_baseline
                    );
                    assert!(self.tree.drain_actions().is_empty());
                    let mut actions = vec![];
                    if let Some(directory) = std::env::var_os("TEKESUI_RICH_NATIVE_EVIDENCE") {
                        actions.push((
                            100,
                            Action::Capture {
                                path: std::path::PathBuf::from(directory)
                                    .join("narrow-dark-composer-marked.png")
                                    .display()
                                    .to_string(),
                            },
                        ));
                    }
                    actions.extend([
                        (250, Action::Unmark),
                        (
                            350,
                            Action::AXSelect {
                                component: md,
                                range: 0..length,
                            },
                        ),
                        (
                            450,
                            Action::ExpectAXText {
                                component: md,
                                text: text.clone(),
                                selection: 0..length,
                                selected: text.clone(),
                                query: range.clone(),
                                substring: Some("中文😀".into()),
                            },
                        ),
                        (500, Action::TextCommand("copy:")),
                        (620, next(25_007)),
                    ]);
                    actions
                }
                25_007 => {
                    assert_eq!(window.clipboard_text().as_deref(), Some(text.as_str()));
                    let code = ax::content_key(&self.tree, f::CODE, None)
                        .unwrap()
                        .component;
                    let value = f::rich(&self.tree, f::CODE).document().text().to_owned();
                    let len = value.encode_utf16().count();
                    vec![
                        (
                            100,
                            Action::AXSelect {
                                component: code,
                                range: 0..len,
                            },
                        ),
                        (
                            250,
                            Action::ExpectAXText {
                                component: code,
                                text: value.clone(),
                                selection: 0..len,
                                selected: value.clone(),
                                query: 0..len,
                                substring: Some(value),
                            },
                        ),
                        (300, Action::TextCommand("copy:")),
                        (420, next(25_008)),
                    ]
                }
                25_008 => {
                    assert_eq!(
                        window.clipboard_text().as_deref(),
                        Some(f::rich(&self.tree, f::CODE).document().text())
                    );
                    let diff = ax::content_key(&self.tree, f::DIFF, None)
                        .unwrap()
                        .component;
                    let value = f::rich(&self.tree, f::DIFF).document().text().to_owned();
                    let len = value.encode_utf16().count();
                    vec![
                        (
                            100,
                            Action::AXSelect {
                                component: diff,
                                range: 0..len,
                            },
                        ),
                        (
                            250,
                            Action::ExpectAXText {
                                component: diff,
                                text: value.clone(),
                                selection: 0..len,
                                selected: value.clone(),
                                query: 0..len,
                                substring: Some(value),
                            },
                        ),
                        (300, Action::TextCommand("copy:")),
                        (420, next(25_009)),
                    ]
                }
                25_009 => {
                    assert_eq!(
                        window.clipboard_text().as_deref(),
                        Some(f::rich(&self.tree, f::DIFF).document().text())
                    );
                    self.baseline = Some(
                        self.tree
                            .text_surface(f::DIFF)
                            .unwrap()
                            .input
                            .external_baseline(),
                    );
                    vec![
                        (100, Action::Preedit("noedit".into())),
                        (150, Action::Text("noedit".into())),
                        (250, next(25_010)),
                    ]
                }
                25_010 => {
                    assert_eq!(
                        Some(
                            self.tree
                                .text_surface(f::DIFF)
                                .unwrap()
                                .input
                                .external_baseline()
                        ),
                        self.baseline
                    );
                    self.theme = Theme::LIGHT;
                    self.text.scale = 1.;
                    vec![
                        (100, Action::Resize { w: 720., h: 760. }),
                        (350, next(25_011)),
                    ]
                }
                25_011 => {
                    assert_eq!(self.tree.semantic_mount(f::MARKDOWN), Some(self.mount));
                    assert_eq!(self.links.len(), 1);
                    let mut actions = vec![];
                    if let Some(directory) = std::env::var_os("TEKESUI_RICH_NATIVE_EVIDENCE") {
                        actions.push((
                            100,
                            Action::Capture {
                                path: std::path::PathBuf::from(directory)
                                    .join("code-diff-selection.png")
                                    .display()
                                    .to_string(),
                            },
                        ));
                    }
                    actions.push((350, Action::Close));
                    actions
                }
                _ => unreachable!(),
            };
            self.stages.push(stage);
            self.record(window, &format!("verified-{stage}"));
            script
        }
        fn event(&mut self, event: ContentEvent, window: &Window) {
            if let ContentEvent::TextCommand(command) = &event
                && command == "copy:"
            {
                if let Some(surface) = self.tree.focused_text()
                    && surface.input.marked.is_none()
                {
                    window.set_clipboard_text(&surface.input.selected_text());
                }
                self.record(window, "original-selection-copied");
                return;
            }
            self.tree.event_with_text(&event, &mut self.text).unwrap();
            self.collect_links();
            self.record(window, "original-input");
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
                    if self.started && (25_001..=25_011).contains(&stage) =>
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
                    self.collect_links();
                }
                Event::CloseRequested => {
                    if self.started {
                        assert_eq!(self.stages.len(), 11);
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
    let tree = f::build().unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    App::new(AppConfig {
        name: Some("TekesUI Rich Content Consumer".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI Rich Content".into(),
            size: p::Size {
                width: 720.,
                height: 760.,
            },
            ..Default::default()
        },
        if std::env::var_os("TEKESUI_RICH_NATIVE_SELFTEST").is_some() {
            MenuBar {
                menus: vec![Menu::new(
                    "Rich content acceptance",
                    (25_001..=25_011)
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
            started: false,
            stages: vec![],
            baseline: None,
            composer_baseline: None,
            mount: 0,
            theme: Theme::LIGHT,
            links: vec![],
        },
    );
}
