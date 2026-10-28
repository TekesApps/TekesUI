#![cfg_attr(not(feature = "svg-icons"), allow(dead_code))]
//! Small native host: one ordinary ContentHost, no per-document input/AX wiring.
#[cfg(all(target_os = "macos", feature = "svg-icons"))]
#[path = "../tests/support/coretext.rs"]
mod coretext;
#[cfg(all(target_os = "macos", feature = "svg-icons"))]
#[path = "support/native_key.rs"]
mod native_key;
#[cfg(all(target_os = "macos", feature = "svg-icons"))]
#[path = "support/native_pointer.rs"]
mod native_pointer;
#[cfg(not(all(target_os = "macos", feature = "svg-icons")))]
fn main() {}
#[cfg(all(target_os = "macos", feature = "svg-icons"))]
fn main() {
    use tekes_ui_composition_consumer::icon_content::*;
    use tekes_ui_controls::composition::*;
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler,
        content_accessibility as ax,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme, proposal, surface_palette::resolve_surface};
    struct Host {
        root: ComponentId,
        host: ContentHost,
        text: coretext::Text,
        gpu: Option<Renderer>,
        icons: SvgIconCache,
        commands: Vec<serde_json::Value>,
        clock: std::time::Instant,
        review_started: bool,
        review_phase: u32,
    }
    impl Host {
        fn actions(&mut self) {
            let records = process_actions(&mut self.host);
            for record in records {
                self.commands.push(serde_json::json!({"target":record.action.target.0,"source":format!("{:?}",record.action.source),"mount":record.mount}));
            }
        }
        fn draw(&mut self, w: &mut Window) {
            self.text.now = self.clock.elapsed();
            self.host.set_window_active(w.is_active());
            self.text.backing = w.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.requested = false;
            let width = w.size().width as f32;
            let height = w.size().height as f32;
            let frame = Rect::new(16., 16., (width - 32.).max(0.), (height - 32.).max(0.));
            let viewport = Rect::new(0., 0., width.max(0.), height.max(0.));
            let palette = Palette::from(Theme::LIGHT);
            let mut scene = Scene::new();
            {
                let mut icons = self
                    .icons
                    .renderer(self.gpu.as_mut().unwrap(), self.text.backing);
                self.host
                    .render(
                        &mut scene,
                        &mut WithIconRenderer::new(&mut self.text, &mut icons),
                        frame,
                        viewport,
                        palette,
                    )
                    .unwrap();
            }
            let surface = self
                .host
                .get(self.root)
                .unwrap()
                .surface()
                .unwrap_or("canvas");
            let ground = resolve_surface(palette, surface, "content", "rest")
                .unwrap()
                .background;
            if let Some(gpu) = &mut self.gpu {
                gpu.render(&scene, &mut self.text.atlas, ground).unwrap();
            }
            let layouts = ax::content_text_layouts(&self.host);
            let input = self
                .host
                .focused()
                .and_then(|id| ax::content_key(&self.host, id, None))
                .and_then(|key| layouts.get(&key))
                .map(|layout| layout.state.clone());
            w.set_text_input_state(input);
            w.set_accessibility_text_layouts(layouts.into_iter().collect());
            w.set_accessibility_nodes(ax::content_nodes(&self.host));
            w.set_animating(self.text.requested);
            if !self.review_started
                && std::env::var_os("TEKESUI_ICON_NATIVE_SELFTEST").is_some()
                && w.is_active()
            {
                use p::selftest::Action;
                let editor = self.host.get(COMPOSER).unwrap().frame().center();
                let back = self.host.get(BACK).unwrap().frame().center();
                let press = |id| Action::AXPressKey {
                    key: ax::content_key(&self.host, id, None).unwrap(),
                };
                let marked = || Action::ExpectTextState {
                    text: "draft😀拼音".into(),
                    selection: 9..9,
                    marked: Some(7..9),
                    query: 7..9,
                    actual: 7..9,
                };
                let steps = vec![
                    (200, Action::MenuCommand(47_001)),
                    (
                        400,
                        Action::Click {
                            x: editor.x.into(),
                            y: editor.y.into(),
                        },
                    ),
                    (
                        700,
                        Action::TextRange {
                            text: "draft😀".into(),
                            replacement: Some(0..7),
                        },
                    ),
                    (
                        1000,
                        Action::PreeditRange {
                            text: "拼音".into(),
                            selection: 2..2,
                            replacement: None,
                        },
                    ),
                    (1400, marked()),
                    (1600, Action::MenuCommand(47_002)),
                    (1800, press(RUN)),
                    (2200, marked()),
                    (2400, Action::MenuCommand(47_003)),
                    (2600, press(SWAP)),
                    (2800, press(TONE)),
                    (
                        3000,
                        Action::Click {
                            x: back.x.into(),
                            y: back.y.into(),
                        },
                    ),
                    (3400, marked()),
                    (3600, Action::MenuCommand(47_004)),
                    (3800, press(RUN)),
                    (4000, press(HIDE)),
                    (4300, marked()),
                    (4500, Action::MenuCommand(47_005)),
                    (4700, press(HIDE)),
                    (4900, press(RUN)),
                    (5200, marked()),
                    (5400, Action::MenuCommand(47_006)),
                    // Per-window preference input, not a global macOS setting.
                    (5600, Action::MenuCommand(47_007)),
                    (5900, marked()),
                    (6100, Action::MenuCommand(47_008)),
                    (6500, Action::Miniaturize),
                    (7300, Action::Deminiaturize),
                    (7900, Action::MenuCommand(47_010)),
                    (8300, Action::Close),
                ];
                self.review_started = true;
                p::selftest::run_script(steps);
            }
            if let Some(path) = std::env::var_os("TEKESUI_COMPONENT_EVIDENCE")
                .or_else(|| std::env::var_os("TEKESUI_ICON_EVIDENCE"))
            {
                let path = std::path::PathBuf::from(path);
                std::fs::create_dir_all(&path).unwrap();
                let screen = w.screen_info();
                let nodes = self.host.semantics();
                let icon = self
                    .host
                    .get(ICON)
                    .and_then(|node| node.component::<IconContent>());
                let composer = &self
                    .host
                    .get(COMPOSER)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap()
                    .surface
                    .input;
                let state = serde_json::json!({"pid":std::process::id(),"phase":self.review_phase,"composer":{"text":composer.text,"selection":composer.selection(),"marked":composer.marked,"mount":self.host.semantic_mount(COMPOSER)},"icon_visible":icon.map(Component::active),"icon_mount":self.host.semantic_mount(ICON),"active":w.is_active(),"status_tone":format!("{:?}",icon.and_then(IconContent::status_tone)),"icon_disabled":icon.map(IconContent::is_disabled),"indeterminate":icon.map(IconContent::is_indeterminate),"reduced_motion":self.text.reduced_motion,"frame_requested":self.host.animation_frame_requested(),"scale":self.text.scale,"width":width,"height":height,"root_surface":surface,"glyph":icon.map(|icon|icon.glyph().name),"focused":self.host.focused().map(|id|id.0),"commands":self.commands,"screen":screen.map(|s|serde_json::json!({"name":s.name,"builtin":s.builtin})),"nodes":nodes.iter().map(|n|serde_json::json!({"id":n.id.0,"mount":self.host.semantic_mount(n.id),"name":n.semantics.name,"role":format!("{:?}",n.semantics.role),"frame":[n.frame.min_x(),n.frame.min_y(),n.frame.size.width,n.frame.size.height],"visible":n.visible.map(|r|[r.min_x(),r.min_y(),r.size.width,r.size.height])})).collect::<Vec<_>>()});
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
        fn handle(&mut self, e: Event, w: &mut Window) {
            match e {
                Event::Created => {
                    w.set_text_key_events(true);
                    self.gpu = Some(unsafe {
                        Renderer::new_for_metal_layer(
                            w.metal_layer(),
                            w.drawable_size(),
                            w.scale() as f32,
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
                    self.host.set_window_active(active);
                    if self.review_started && !active {
                        self.review_phase = 47_009;
                    }
                }
                Event::Command(p::CommandId(id))
                    if self.review_started && (47_001..=47_010).contains(&id) =>
                {
                    self.review_phase = id;
                    if id == 47_007 {
                        self.text.reduced_motion = true;
                    }
                    if id == 47_002 || id == 47_008 {
                        self.text.reduced_motion = false;
                    }
                }
                Event::Mouse(m) => {
                    self.host.event(&native_pointer::translate(m));
                    self.actions();
                }
                Event::Scroll(s) => {
                    self.host
                        .event_with_text(
                            &ContentEvent::Scroll {
                                position: tekes_ui_scene::Point::new(
                                    s.position.x as f32,
                                    s.position.y as f32,
                                ),
                                delta: tekes_ui_scene::Point::new(
                                    -s.delta.x as f32,
                                    -s.delta.y as f32,
                                ),
                                precise: s.precise,
                                begin: matches!(
                                    s.phase,
                                    p::ScrollPhase::Began | p::ScrollPhase::MayBegin
                                ),
                                end: matches!(
                                    s.phase,
                                    p::ScrollPhase::Ended | p::ScrollPhase::Cancelled
                                ),
                                momentum: !matches!(s.momentum_phase, p::ScrollPhase::None),
                                time: s.timestamp,
                            },
                            &mut self.text,
                        )
                        .unwrap();
                }
                Event::Key(k)
                    if k.kind == p::KeyEventKind::Down
                        && k.modifiers.command
                        && matches!(k.key_code, 24 | 27 | 29) =>
                {
                    let body = proposal::TextRole::UiBody.style().size;
                    self.text.scale = match k.key_code {
                        24 => (body * self.text.scale + 1.) / body,
                        27 => (body * self.text.scale - 1.).max(body * 0.5) / body,
                        _ => 1.,
                    };
                }
                Event::Key(k) => {
                    self.host
                        .event_with_text(
                            &ContentEvent::KeyInput(native_key::key(k)),
                            &mut self.text,
                        )
                        .unwrap();
                    self.actions();
                }
                Event::TextCommit { text, replacement } => {
                    self.host
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
                    self.host
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
                    self.host
                        .event_with_text(&ContentEvent::Unmark, &mut self.text)
                        .unwrap();
                }
                Event::TextCommand(command) => {
                    self.host
                        .event_with_text(&ContentEvent::TextCommand(command), &mut self.text)
                        .unwrap();
                    self.actions();
                }
                Event::TextKey(e) => {
                    self.host
                        .event_with_text(&native_key::translate(e), &mut self.text)
                        .unwrap();
                    self.actions();
                }
                Event::Accessibility { target, action } => {
                    ax::content_action(&mut self.host, target, action);
                    self.actions();
                }
                Event::CloseRequested => {
                    self.host.set_window_active(false);
                    w.set_accessibility_text_layouts(Default::default());
                    w.set_accessibility_nodes(vec![]);
                    eprintln!("ICON_NATIVE_CLOSE_REQUESTED");
                    w.close();
                    return;
                }
                Event::Redraw | Event::AnimationFrame(_) => {}
                _ => return,
            }
            self.draw(w);
        }
    }
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let (title, root, host) = ("TekesUI Icon Content Review", ROOT, build().unwrap());
    App::new(AppConfig {
        name: Some(title.into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: title.into(),
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
        if std::env::var_os("TEKESUI_ICON_NATIVE_SELFTEST").is_some() {
            MenuBar {
                menus: vec![p::Menu::new(
                    "Acceptance",
                    (47_001..=47_010)
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
            root,
            host,
            text,
            gpu: None,
            icons: SvgIconCache::default(),
            commands: vec![],
            clock: std::time::Instant::now(),
            review_started: false,
            review_phase: 0,
        },
    );
}
