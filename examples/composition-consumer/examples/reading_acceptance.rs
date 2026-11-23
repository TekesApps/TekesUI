//! Opt-in standalone native host for the independent public reading fixture.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("reading_acceptance requires macOS");
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
    use tekes_ui_composition_consumer::reading_prepend::{EDITOR, ReadingFixture};
    use tekes_ui_controls::composition::ContentEvent as C;
    use tekes_ui_macos_bridge::content_input::{
        translate_key, translate_pointer, translate_text_command, translate_text_key,
    };
    use tekes_ui_macos_bridge::coretext_host::CoreTextHost;
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, MenuBar, ScrollPhase, Window, WindowConfig, WindowHandler,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Palette, Spacing, Theme};

    pub(super) fn try_run() -> bool {
        let Some(out) = std::env::var_os("TEKESUI_READING_PREPEND_AUDIT_OUT") else {
            return false;
        };
        App::new(AppConfig {
            name: Some("TekesUI 阅读锚点".into()),
            quit_after_last_window_closed: true,
        })
        .run(
            WindowConfig {
                title: "TekesUI · 历史前插".into(),
                size: tekes_ui_platform_macos::Size {
                    width: 800.,
                    height: 720.,
                },
                ..Default::default()
            },
            MenuBar::default(),
            Host {
                fixture: {
                    let mut fixture = ReadingFixture::new();
                    if std::env::var_os("TEKESUI_TEXT_EDIT_AUDIT").is_some() {
                        use tekes_ui_controls::composition::{
                            TextFieldContent, TextSubmitPolicy, TextTabPolicy,
                        };
                        let field = fixture
                            .controls
                            .get_mut(EDITOR)
                            .unwrap()
                            .component_mut::<TextFieldContent>()
                            .unwrap();
                        field.set_read_only(false);
                        field.set_multiline(true);
                        field.set_scroll_policy(
                            tekes_ui_controls::ScrollPolicy::Disabled,
                            tekes_ui_controls::ScrollPolicy::Auto,
                        );
                        field.tab_policy = TextTabPolicy::Insert;
                        field.submit_policy = TextSubmitPolicy::CommandEnter;
                    }
                    fixture
                },
                text: CoreTextHost::new()
                    .with_backing_scale(2.0)
                    .with_text_services(true),
                scene: Scene::new(),
                renderer: None,
                theme: Theme::LIGHT,
                start: std::time::Instant::now(),
                reduced: false,
                active: true,
                out: out.into(),
                expected: None,
                reports: Vec::new(),
                pointer_reports: Vec::new(),
                text_key_reports: Vec::new(),
            },
        );
        true
    }

    struct Host {
        fixture: ReadingFixture,
        text: CoreTextHost,
        scene: Scene,
        renderer: Option<Renderer>,
        theme: Theme,
        start: std::time::Instant,
        reduced: bool,
        active: bool,
        out: std::path::PathBuf,
        expected: Option<(usize, f32)>,
        reports: Vec<serde_json::Value>,
        pointer_reports: Vec<serde_json::Value>,
        text_key_reports: Vec<serde_json::Value>,
    }

    impl Host {
        fn paint(&mut self, window: &Window) {
            self.text.scale = window.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.begin_frame(self.start.elapsed(), self.reduced);
            self.scene.clear();
            let size = window.size();
            let frame = Rect::new(0., 0., size.width as f32, size.height as f32);
            self.fixture.paint(
                &mut self.text,
                &mut self.scene,
                frame.inset(Spacing::Loose.value()),
                frame,
                Palette::from(self.theme),
            );
            if let Some((index, before)) = self.expected.take() {
                let after = self.fixture.surface.caret_doc(index).min_y()
                    - self.fixture.surface.scroll().offset.y;
                self.reports.push(serde_json::json!({
                    "pages": self.fixture.pages,
                    "utf16": index,
                    "before_y": before,
                    "after_y": after,
                    "error_pt": (before - after).abs()
                }));
            }
            if let Some(renderer) = &mut self.renderer {
                renderer
                    .render(
                        &self.scene,
                        &mut self.text.atlas,
                        Palette::from(self.theme).color("surface-1").unwrap(),
                    )
                    .unwrap();
            }
            window.set_title(&format!("TekesUI · 历史前插 {} 页", self.fixture.pages));
            window.set_animating(self.text.animation_requested);
            self.publish(window);
            self.save();
        }

        fn publish(&self, window: &Window) {
            window.set_text_input_state(
                self.fixture
                    .controls
                    .focused_text()
                    .filter(|_| self.active)
                    .map(|s| {
                        let caret = s.caret();
                        let origin = s.origin();
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
                            carets: s
                                .paragraph()
                                .map(|p| {
                                    p.lines
                                        .iter()
                                        .filter(|l| {
                                            origin.y + (l.top + l.height) as f32
                                                >= s.inner().min_y()
                                                && origin.y + l.top as f32 <= s.inner().max_y()
                                        })
                                        .flat_map(|l| {
                                            l.carets.iter().map(move |(i, x)| {
                                                (
                                                    *i,
                                                    tekes_ui_platform_macos::Point {
                                                        x: f64::from(origin.x) + x,
                                                        y: f64::from(origin.y) + l.top,
                                                    },
                                                    tekes_ui_platform_macos::Size {
                                                        width: 1.,
                                                        height: l.height,
                                                    },
                                                )
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        }
                    }),
            );
        }

        fn input(&mut self, event: Event) -> bool {
            let event = match event {
                Event::Mouse(m) => translate_pointer(m),
                Event::Scroll(s) => {
                    let p = Point::new(s.position.x as f32, s.position.y as f32);
                    if self.fixture.surface.contains_visible(p) {
                        let delta = Point::new(s.delta.x as f32, s.delta.y as f32);
                        self.fixture.reading.observe_scroll(
                            &mut self.fixture.surface,
                            delta,
                            s.modifiers.shift,
                        );
                        return self
                            .fixture
                            .surface
                            .wheel_at(
                                p,
                                delta,
                                s.precise,
                                matches!(s.phase, ScrollPhase::Began | ScrollPhase::MayBegin),
                                matches!(s.phase, ScrollPhase::Ended | ScrollPhase::Cancelled),
                                s.momentum_phase != ScrollPhase::None,
                                s.timestamp,
                                s.modifiers.shift,
                            )
                            .unwrap_or(false);
                    }
                    return false;
                }
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
                Event::TextCommand(c) => C::TextCommand(translate_text_command(c)),
                Event::Key(k) => C::KeyInput(translate_key(k)),
                _ => return false,
            };
            let before = self
                .fixture
                .controls
                .text_surface(EDITOR)
                .map(|s| s.input().selection());
            let changed = self.fixture.controls.event(&event).changed;
            if let C::Pointer(p) = &event
                && self.pointer_reports.len() < 512
            {
                self.pointer_reports.push(serde_json::json!({
                    "phase": format!("{:?}", p.phase),
                    "button": format!("{:?}", p.button),
                    "click_count": p.click_count,
                    "shift": p.modifiers.shift,
                    "position": [p.position.x, p.position.y],
                    "before_selection": before,
                    "after_selection": self.fixture.controls.text_surface(EDITOR).map(|s| s.input().selection()),
                    "focus": self.fixture.controls.focused().map(|id| id.0)
                }));
                self.save();
            }
            changed
        }

        fn save(&self) {
            std::fs::create_dir_all(&self.out).unwrap();
            std::fs::write(
                self.out.join("reading-events.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "pages": self.fixture.pages,
                    "reports": self.reports,
                    "pointer_reports": self.pointer_reports,
                    "text_key_reports": self.text_key_reports,
                    "editor_selection": self.fixture.controls.text_surface(EDITOR).map(|s| s.input().selection()),
                    "following": self.fixture.reading.is_following_latest(),
                    "version": self.fixture.reading.document_version().map(|v| v.revision()),
                    "draft": self.fixture.controls.text_surface(EDITOR).map(|s| s.text()),
                    "focus": self.fixture.controls.focused().map(|v| v.0),
                    "scope": "native paging and real layout; not physical IME/VoiceOver/performance acceptance"
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }

    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match event {
                Event::TextKey(report) => {
                    let response = self
                        .fixture
                        .controls
                        .event(&translate_text_key(report.clone()));
                    self.publish(window);
                    if response.changed {
                        window.request_redraw();
                    }
                    if self.text_key_reports.len() < 512 {
                        self.text_key_reports.push(serde_json::json!({
                            "event": format!("{report:?}"),
                            "outcome": format!("{:?}", response.text_key),
                            "submit_intent": response.activated,
                            "draft": self.fixture.controls.text_surface(EDITOR).map(|s| s.text()),
                            "focus": self.fixture.controls.focused().map(|id| id.0),
                        }));
                    }
                    self.save();
                }
                Event::Created => {
                    assert!(window.set_text_key_events(true));
                    // SAFETY: this handler owns the renderer until the window run returns;
                    // the existing view retains the layer.
                    self.renderer = Some(
                        unsafe {
                            Renderer::new_for_metal_layer(
                                window.metal_layer(),
                                window.drawable_size(),
                                window.scale() as f32,
                            )
                        }
                        .unwrap(),
                    );
                    if std::env::var_os("TEKESUI_READING_PREPEND_BACKGROUND").is_some() {
                        let wake = window.wake_proxy();
                        std::thread::spawn(move || {
                            // One finite arrival, not a data/frame polling loop.
                            std::thread::sleep(std::time::Duration::from_secs(10));
                            let _ = wake.wake();
                        });
                    }
                }
                Event::Wake => {
                    let before = self.fixture.surface.reflow_anchor().map(|a| {
                        (
                            a.utf16,
                            self.fixture.surface.caret_doc(a.utf16).min_y()
                                - self.fixture.surface.scroll().offset.y,
                        )
                    });
                    let len = self.fixture.surface.input().len();
                    self.fixture.prepend().unwrap();
                    self.expected =
                        before.map(|(i, y)| (i + self.fixture.surface.input().len() - len, y));
                    window.request_redraw();
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
                Event::Redraw | Event::AnimationFrame(_) => self.paint(window),
                Event::CloseRequested => window.close(),
                Event::SystemMotionChanged(v) => {
                    self.reduced = v;
                    window.request_redraw();
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
                    window.request_redraw();
                }
                Event::Focus(active) => {
                    self.active = active;
                    if !active {
                        self.fixture.controls.event(&C::Cancel);
                        self.fixture.surface.cancel_interaction();
                    }
                    self.publish(window);
                    window.request_redraw();
                }
                event => {
                    let before = self.fixture.surface.reflow_anchor().map(|a| {
                        (
                            a.utf16,
                            self.fixture.surface.caret_doc(a.utf16).min_y()
                                - self.fixture.surface.scroll().offset.y,
                        )
                    });
                    let len = self.fixture.surface.input().len();
                    let pages = self.fixture.pages;
                    let changed = self.input(event);
                    let action = self.fixture.process_actions().unwrap();
                    if self.fixture.pages != pages {
                        self.expected =
                            before.map(|(i, y)| (i + self.fixture.surface.input().len() - len, y));
                    }
                    self.publish(window);
                    if changed || action {
                        window.request_redraw();
                    }
                }
            }
        }
    }
}
