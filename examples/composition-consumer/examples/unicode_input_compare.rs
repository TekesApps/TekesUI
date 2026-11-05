//! Opt-in #41 diagnostic. Uses the original TextFieldContent/editor and native bridge.
//! No production text logging, character compensation, input-source change or retry.
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
    use serde_json::{Value, json};
    use tekes_ui_controls::composition::*;
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, KeyEventKind, Menu, MenuBar, MenuCommand, MenuItem,
        Window, WindowConfig, WindowHandler, content_accessibility as ax, selftest::Action,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};
    const FIELD: ComponentId = ComponentId(41001);
    const READY: u32 = 41000;
    const FINISH: u32 = 41001;
    struct Host {
        tree: ContentHost,
        text: coretext::Text,
        gpu: Option<Renderer>,
        source: String,
        case: String,
        expected: &'static str,
        seq: u64,
        focused: bool,
        ready: bool,
        finished: bool,
        actions: usize,
        native_key_ends: usize,
        clock: std::time::Instant,
    }
    impl Host {
        fn record(&mut self, kind: &str, detail: Value, window: &Window) {
            self.seq += 1;
            let input = &self.tree.text_surface(FIELD).unwrap().input;
            eprintln!(
                "UNICODE41 {}",
                json!({"control":"sdk", "seq":self.seq, "kind":kind,
                "pid":std::process::id(), "declared_source":self.source, "case":self.case,
                "handler_elapsed_seconds":self.clock.elapsed().as_secs_f64(),
                "detail":detail, "value":input.text,"selection":[input.selection().start,input.selection().end],
                "marked":input.marked.as_ref().map(|r|[r.start,r.end]),"active":window.is_active(),
                "focused":self.tree.focused()==Some(FIELD),"actions":self.actions,
                "physical_ime":"not_run"})
            );
        }
        fn input(&mut self, event: ContentEvent, window: &Window) {
            let response = self.tree.event_with_text(&event, &mut self.text).unwrap();
            self.actions += self.tree.drain_action_records().len();
            self.record(
                "editor",
                json!({"event":format!("{event:?}"),"response":format!("{response:?}")}),
                window,
            );
        }
        // As in the ordinary native consumer, pasteboard access belongs to the
        // host; insertion still uses the original editor transaction.
        fn paste(&mut self, window: &Window) {
            if !window.is_active() {
                return;
            }
            let Some(surface) = self.tree.focused_text() else {
                return;
            };
            if surface.input.disabled || surface.input.read_only || surface.input.marked.is_some() {
                return;
            }
            if let Some(text) = window.clipboard_text() {
                // This is host clipboard input, not an observed AppKit text callback.
                self.record("clipboard", json!({"command":"paste:","text":text}), window);
                self.input(
                    ContentEvent::Commit {
                        text,
                        replacement: None,
                    },
                    window,
                );
            }
        }
        fn draw(&mut self, window: &mut Window) {
            self.text.now = self.clock.elapsed();
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.requested = false;
            let size = window.size();
            let viewport = Rect::new(0., 0., size.width as f32, size.height as f32);
            let palette = Palette::from(Theme::LIGHT);
            let mut scene = Scene::new();
            self.tree
                .render(
                    &mut scene,
                    &mut self.text,
                    Rect::new(30., 30., 600., 80.),
                    viewport,
                    palette,
                )
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
            if !self.focused && window.is_active() {
                self.focused = true;
                p::selftest::run_script(vec![
                    (
                        0,
                        Action::ClickPublished {
                            label: "Unicode41 input",
                            secondary: false,
                        },
                    ),
                    (100, Action::MenuCommand(READY)),
                ]);
            }
        }
        fn finish(&mut self, window: &Window) {
            if self.finished {
                return;
            }
            self.finished = true;
            let input = &self.tree.text_surface(FIELD).unwrap().input;
            self.record(
                "result",
                json!({"expected":self.expected,"matches":input.text==self.expected,
                "no_marked":input.marked.is_none(),"no_submit":self.actions==0,"ready":self.ready}),
                window,
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
                Event::Command(p::CommandId(READY)) => {
                    assert!(
                        !self.ready && self.tree.focused() == Some(FIELD) && window.is_active()
                    );
                    self.ready = true;
                    self.record(
                        "ready",
                        json!({"expected":self.expected,"source_is_operator_declared":true}),
                        window,
                    );
                    if self.source == "local-key-characters" {
                        let chars: Vec<_> = self
                            .expected
                            .chars()
                            .map(|c| match c {
                                'A' => "A",
                                's' => "s",
                                'c' => "c",
                                'i' => "i",
                                '-' => "-",
                                '7' => "7",
                                '原' => "原",
                                '生' => "生",
                                '草' => "草",
                                '稿' => "稿",
                                '保' => "保",
                                '留' => "留",
                                _ => unreachable!(),
                            })
                            .collect();
                        let mut steps = chars
                            .into_iter()
                            .enumerate()
                            .map(|(i, chars)| {
                                (i as u64 * 100, Action::KeyCharacters { key_code: 0, chars })
                            })
                            .collect::<Vec<_>>();
                        steps.push((1500, Action::MenuCommand(FINISH)));
                        p::selftest::run_script(steps);
                    }
                }
                Event::Command(p::CommandId(FINISH)) => {
                    self.finish(window);
                    window.close();
                    return;
                }
                Event::TextKey(key) => {
                    if matches!(&key, p::TextKeyEvent::Finished { .. }) {
                        self.native_key_ends += 1;
                        if self.source == "pid-hardware-key" && self.native_key_ends == 6 {
                            // Complete the diagnostic after the original space key-up can arrive.
                            // This never warms up or retries an input context.
                            p::selftest::run_script(vec![(250, Action::MenuCommand(FINISH))]);
                        }
                    }
                    let detail = match &key {
                        p::TextKeyEvent::Started(c) => {
                            json!({"phase":"start","context_id":c.id,"native_code":c.key.key_code,"characters":c.key.characters,"characters_ignoring_modifiers":c.key.characters_ignoring_modifiers,"composing":c.composing_at_start})
                        }
                        p::TextKeyEvent::Finished {
                            context,
                            input_context_handled,
                            disposition,
                            text_callbacks,
                            command_callbacks,
                        } => {
                            json!({"phase":"end","context_id":context.id,"input_context_handled":input_context_handled,"disposition":format!("{disposition:?}"),"text_callbacks":text_callbacks,"command_callbacks":command_callbacks})
                        }
                    };
                    self.record("key_context", detail, window);
                    self.input(native_key::translate(key), window);
                }
                Event::Mouse(mouse) => self.input(native_pointer::translate(mouse), window),
                Event::Key(key)
                    if key.kind == KeyEventKind::Down
                        && key.modifiers.command
                        && key.key_code == 9 =>
                {
                    self.paste(window);
                }
                Event::Key(key) => self.input(ContentEvent::KeyInput(native_key::key(key)), window),
                Event::TextCommit { text, replacement } => {
                    self.record("commit",json!({"text":text,"replacement":replacement.as_ref().map(|r|[r.start,r.end])}),window);
                    self.input(ContentEvent::Commit { text, replacement }, window);
                }
                Event::TextPreedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                } => {
                    self.record("preedit",json!({"text":text,"selection":[selection.start,selection.end],"replacement":replacement.as_ref().map(|r|[r.start,r.end])}),window);
                    self.input(
                        ContentEvent::Preedit {
                            text,
                            selection,
                            clauses,
                            replacement,
                        },
                        window,
                    );
                }
                Event::TextUnmark => {
                    self.record("unmark", json!({}), window);
                    self.input(ContentEvent::Unmark, window);
                }
                Event::TextCommand(command) => {
                    self.record("command", json!({"selector":command}), window);
                    if command == "paste:" {
                        self.paste(window);
                    } else {
                        self.input(ContentEvent::TextCommand(command), window);
                    }
                }
                Event::Accessibility { target, action } => {
                    ax::content_action(&mut self.tree, target, action);
                    self.actions += self.tree.drain_action_records().len();
                }
                Event::Focus(active) => self.tree.set_window_active(active),
                Event::CloseRequested => {
                    self.finish(window);
                    window.close();
                    return;
                }
                Event::Redraw | Event::AnimationFrame(_) => {}
                _ => return,
            }
            self.draw(window);
        }
    }
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "usage: unicode_input_compare SOURCE CASE; see scripts/diagnostics/unicode41/README.md"
    );
    let source = args[1].clone();
    let case = args[2].clone();
    assert!(
        [
            "cua-typeText",
            "pid-hardware-key",
            "local-key-characters",
            "paste",
            "physical-keyboard"
        ]
        .contains(&source.as_str())
    );
    let expected = match case.as_str() {
        "ascii" => "Ascii-A7",
        "cjk" => "原生草稿保留",
        "mixed" => "原生A草稿保留",
        "pinyin-first" => "你好",
        _ => panic!("unknown fixed case"),
    };
    assert!(
        source != "local-key-characters" || case != "pinyin-first",
        "Unicode NSEvent injection cannot substitute for Pinyin keys"
    );
    let mut tree = ContentHost::new(Content::new(
        FIELD,
        TextFieldContent::new("Unicode41 input", ""),
    ))
    .unwrap();
    tree.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    App::new(AppConfig {
        name: Some("Unicode41 SDK".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: format!("Unicode41 SDK · {source} · {case}"),
            size: p::Size {
                width: 660.,
                height: 180.,
            },
            ..Default::default()
        },
        MenuBar {
            menus: vec![Menu::new(
                "Diagnostic",
                vec![
                    MenuItem::Command(MenuCommand::new("Record ready", p::CommandId(READY))),
                    MenuItem::Command(MenuCommand::new(
                        "Record result and close",
                        p::CommandId(FINISH),
                    )),
                ],
            )],
        },
        Host {
            tree,
            text,
            gpu: None,
            source,
            case,
            expected,
            seq: 0,
            focused: false,
            ready: false,
            finished: false,
            actions: 0,
            native_key_ends: 0,
            clock: std::time::Instant::now(),
        },
    );
}
