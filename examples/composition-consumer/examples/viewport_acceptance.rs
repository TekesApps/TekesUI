//! Opt-in standalone host for the independent public text-viewport consumer.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("viewport_acceptance requires macOS");
}

#[cfg(target_os = "macos")]
pub fn try_run() -> bool {
    macos::try_run()
}

#[cfg(target_os = "macos")]
fn main() {
    let _ = try_run();
}

#[cfg(target_os = "macos")]
mod macos {
    use tekes_ui_composition_consumer::text_viewport::{COMPOSER, TRANSCRIPT, ViewportFixture};
    use tekes_ui_controls::composition::ContentEvent as C;
    use tekes_ui_macos_bridge::content_input::{
        translate_key, translate_pointer, translate_text_command, translate_text_key,
    };
    use tekes_ui_macos_bridge::coretext_host::CoreTextHost;
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, KeyEventKind, MenuBar, ScrollPhase, Window, WindowConfig,
        WindowHandler,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Palette, Spacing, Theme};

    pub(super) fn try_run() -> bool {
        let Some(out) = std::env::var_os("TEKESUI_TEXT_VIEWPORT_AUDIT_OUT") else {
            return false;
        };
        App::new(AppConfig {
            name: Some("TekesUI 文本视口".into()),
            ..Default::default()
        })
        .run(
            WindowConfig {
                title: "TekesUI · 文本视口".into(),
                size: tekes_ui_platform_macos::Size {
                    width: 760.,
                    height: 640.,
                },
                placement: tekes_ui_platform_macos::WindowPlacement::Auto {
                    need_120hz: false,
                    size_check: true,
                },
                ..Default::default()
            },
            MenuBar::default(),
            Host {
                fixture: ViewportFixture::new(),
                text: CoreTextHost::new()
                    .with_backing_scale(2.0)
                    .with_text_services(true),
                scene: Scene::new(),
                renderer: None,
                theme: Theme::LIGHT,
                out: out.into(),
                start: std::time::Instant::now(),
                reduced: false,
                active: true,
                reports: Vec::new(),
            },
        );
        true
    }

    struct Host {
        fixture: ViewportFixture,
        text: CoreTextHost,
        scene: Scene,
        renderer: Option<Renderer>,
        theme: Theme,
        out: std::path::PathBuf,
        start: std::time::Instant,
        reduced: bool,
        active: bool,
        reports: Vec<serde_json::Value>,
    }

    impl Host {
        fn publish(&self, w: &Window) {
            w.set_text_input_state(
                self.fixture
                    .host
                    .focused_text()
                    .filter(|_| self.active)
                    .map(|s| {
                        let caret = s.caret();
                        tekes_ui_platform_macos::TextInputState {
                            text: s.text().to_owned(),
                            selection: s.input().selection(),
                            marked: s.marked(),
                            caret_index: s.input().caret(),
                            caret_origin: tekes_ui_platform_macos::Point {
                                x: caret.min_x() as f64,
                                y: caret.min_y() as f64,
                            },
                            caret_size: tekes_ui_platform_macos::Size {
                                width: caret.size.width as f64,
                                height: caret.size.height as f64,
                            },
                            read_only: s.input().is_read_only(),
                            ..Default::default()
                        }
                    }),
            );
        }

        fn save(&self, w: &Window) {
            let rect = |r: Rect| {
                serde_json::json!({
                    "x": r.min_x(),
                    "y": r.min_y(),
                    "width": r.size.width,
                    "height": r.size.height
                })
            };
            let fields: Vec<_> = [TRANSCRIPT, COMPOSER]
                .into_iter()
                .map(|id| {
                    let field = self.fixture.field(id);
                    let s = &field.surface;
                    serde_json::json!({
                        "id": id.0,
                        "text": s.text(),
                        "selection": s.input().selection(),
                        "marked": s.marked(),
                        "viewport": format!("{:?}", field.viewport),
                        "outer": rect(s.outer()),
                        "inner": rect(s.inner()),
                        "caret": rect(s.caret()),
                        "scroll": {
                            "x": s.scroll().offset.x,
                            "y": s.scroll().offset.y,
                            "limit_x": s.scroll().limit().x,
                            "limit_y": s.scroll().limit().y
                        },
                        "following": field.reading.as_ref().map(|r| r.is_following_latest())
                    })
                })
                .collect();
            let semantics: Vec<_> = self
                .fixture
                .host
                .semantics()
                .into_iter()
                .map(|n| {
                    serde_json::json!({
                        "id": n.id.0,
                        "name": n.semantics.name,
                        "frame": rect(n.frame),
                        "visible": n.visible.map(rect),
                        "focused": n.focused
                    })
                })
                .collect();
            std::fs::create_dir_all(&self.out).unwrap();
            std::fs::write(
                self.out.join("viewport-events.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "fields": fields,
                    "screen": format!("{:?}", w.screen_info()),
                    "placement": format!("{:?}", w.placement_report()),
                    "backing_scale": w.scale(),
                    "focus": self.fixture.host.focused().map(|id| id.0),
                    "semantics": semantics,
                    "events": self.reports,
                    "scope": "real native stock public Field/Stack viewport + Metal render; logical semantics only; physicalIME/VoiceOver/performance unverified"
                }))
                .unwrap(),
            )
            .unwrap();
        }

        fn paint(&mut self, w: &Window) {
            self.text.scale = w.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.begin_frame(self.start.elapsed(), self.reduced);
            self.scene.clear();
            let s = w.size();
            let frame = Rect::new(0., 0., s.width as f32, s.height as f32);
            self.fixture
                .host
                .render(
                    &mut self.scene,
                    &mut self.text,
                    frame.inset(Spacing::Loose.value()),
                    frame,
                    Palette::from(self.theme),
                )
                .unwrap();
            if let Some(r) = &mut self.renderer {
                r.render(
                    &self.scene,
                    &mut self.text.atlas,
                    Palette::from(self.theme).color("surface-1").unwrap(),
                )
                .unwrap();
            }
            w.set_animating(self.text.animation_requested);
            self.publish(w);
            self.save(w);
        }

        fn input(&mut self, event: Event, w: &Window) {
            let e = match event {
                Event::Mouse(m) => translate_pointer(m),
                Event::TextKey(report) => translate_text_key(report),
                Event::TextCommit { text, replacement } => C::Commit { text, replacement },
                Event::TextPreedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                } => C::Preedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                },
                Event::TextUnmark => C::Unmark,
                Event::TextCommand(c) => {
                    if c == tekes_ui_platform_macos::TextEditCommand::Copy {
                        if let Some(surface) = self.fixture.host.focused_text() {
                            w.set_clipboard_text(&surface.input().selected_text());
                        }
                        return;
                    }
                    if c == tekes_ui_platform_macos::TextEditCommand::Paste {
                        if let Some(text) = w.clipboard_text() {
                            self.input(
                                Event::TextCommit {
                                    text,
                                    replacement: None,
                                },
                                w,
                            );
                        }
                        return;
                    }
                    C::TextCommand(translate_text_command(c))
                }
                Event::Key(k) => {
                    if k.kind == KeyEventKind::Down && k.modifiers.command {
                        let command = match k.key_code {
                            0 => Some(tekes_ui_platform_macos::TextEditCommand::SelectAll),
                            6 if k.modifiers.shift => {
                                Some(tekes_ui_platform_macos::TextEditCommand::Redo)
                            }
                            6 => Some(tekes_ui_platform_macos::TextEditCommand::Undo),
                            8 => Some(tekes_ui_platform_macos::TextEditCommand::Copy),
                            9 => Some(tekes_ui_platform_macos::TextEditCommand::Paste),
                            _ => None,
                        };
                        if let Some(command) = command {
                            return self.input(Event::TextCommand(command), w);
                        }
                    }
                    C::KeyInput(translate_key(k))
                }
                Event::Scroll(s) => C::Scroll {
                    position: Point::new(s.position.x as f32, s.position.y as f32),
                    delta: Point::new(-s.delta.x as f32, -s.delta.y as f32),
                    precise: s.precise,
                    begin: matches!(s.phase, ScrollPhase::Began | ScrollPhase::MayBegin),
                    end: matches!(s.phase, ScrollPhase::Ended | ScrollPhase::Cancelled),
                    momentum: !matches!(s.momentum_phase, ScrollPhase::None),
                    time: s.timestamp,
                },
                _ => return,
            };
            let response = self
                .fixture
                .host
                .event_with_text(&e, &mut self.text)
                .expect("existing CoreText backend");
            if self.reports.len() < 512 {
                self.reports.push(serde_json::json!({
                    "event": format!("{e:?}"),
                    "response": format!("{response:?}"),
                    "value": self.fixture.field(COMPOSER).surface.text()
                }));
            }
            self.publish(w);
            self.save(w);
            if response.changed || self.fixture.host.needs_render() {
                w.request_redraw();
            }
        }
    }

    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, w: &mut Window) {
            match event {
                Event::Created => {
                    assert!(w.set_text_key_events(true));
                    // SAFETY: this handler retains the renderer for the window lifetime.
                    self.renderer = Some(
                        unsafe {
                            Renderer::new_for_metal_layer(
                                w.metal_layer(),
                                w.drawable_size(),
                                w.scale() as f32,
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
                    if let Some(r) = &mut self.renderer {
                        r.resize(drawable_size, scale as f32);
                    }
                }
                Event::Redraw | Event::AnimationFrame(_) | Event::CaretBlink => self.paint(w),
                Event::CloseRequested => w.close(),
                Event::Focus(active) => {
                    self.active = active;
                    if !active {
                        self.fixture.event(&C::Cancel);
                    }
                    self.publish(w);
                    w.request_redraw();
                }
                Event::SystemMotionChanged(v) => {
                    self.reduced = v;
                    w.request_redraw();
                }
                Event::SystemAppearanceChanged {
                    dark,
                    increase_contrast,
                } => {
                    self.theme = Theme {
                        appearance: if dark {
                            tekes_ui_tokens::Appearance::Dark
                        } else {
                            tekes_ui_tokens::Appearance::Light
                        },
                        contrast: if increase_contrast {
                            tekes_ui_tokens::Contrast::Increased
                        } else {
                            tekes_ui_tokens::Contrast::Standard
                        },
                    };
                    w.request_redraw();
                }
                event => self.input(event, w),
            }
        }
    }
}
