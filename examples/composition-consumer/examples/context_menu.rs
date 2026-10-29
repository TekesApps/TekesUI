//! Native original-owner context menu consumer. Desktop execution is separate
//! acceptance: building this program does not prove physical IME or VoiceOver.
#[cfg(target_os = "macos")]
#[path = "../tests/support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[path = "support/native_key.rs"]
mod native_key;
#[cfg(target_os = "macos")]
#[path = "support/native_pointer.rs"]
mod native_pointer;
#[cfg(target_os = "macos")]
#[path = "support/context_menu_review.rs"]
mod review;
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use tekes_ui_composition_consumer::context_menu::{DRAFT, MENU_A, MENU_B, Sidebar};
    use tekes_ui_controls::composition::{ContentEvent, MenuContent, TextFieldContent};
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler,
        content_accessibility as ax,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};
    struct Host {
        sidebar: Sidebar,
        text: coretext::Text,
        gpu: Option<Renderer>,
        clock: std::time::Instant,
        review_started: bool,
        review: Option<review::Review>,
    }
    impl Host {
        fn draw(&mut self, window: &mut Window) {
            self.text.now = self.clock.elapsed();
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            self.text.requested = false;
            let width = window.size().width as f32;
            let height = window.size().height as f32;
            let viewport = Rect::new(0., 0., width.max(0.), height.max(0.));
            let frame = Rect::new(16., 16., (width - 32.).max(0.), (height - 32.).max(0.));
            let palette = Palette::from(Theme::LIGHT);
            let mut scene = Scene::new();
            self.sidebar
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
                    .unwrap()
                    .background,
                )
                .unwrap();
            }
            let layouts = ax::content_text_layouts(&self.sidebar.host);
            let input = self
                .sidebar
                .host
                .focused()
                .and_then(|id| ax::content_key(&self.sidebar.host, id, None))
                .and_then(|key| layouts.get(&key))
                .map(|layout| layout.state.clone());
            window.set_text_input_state(input);
            window.set_accessibility_text_layouts(layouts.into_iter().collect());
            window.set_accessibility_nodes(ax::content_nodes(&self.sidebar.host));
            window.set_animating(self.text.requested);
            if !self.review_started
                && window.is_active()
                && std::env::var_os("TEKESUI_CONTEXT_MENU_NATIVE_SELFTEST").is_some()
            {
                self.review_started = true;
                p::selftest::run_script(self.review.as_ref().unwrap().script(&self.sidebar));
            }
            if let Some(path) = std::env::var_os("TEKESUI_CONTEXT_MENU_EVIDENCE") {
                use std::io::Write;
                let path = std::path::PathBuf::from(path);
                std::fs::create_dir_all(&path).unwrap();
                let input = &self
                    .sidebar
                    .host
                    .get(DRAFT)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap()
                    .surface
                    .input;
                let menus=[MENU_A,MENU_B].into_iter().map(|id| {let menu=self.sidebar.host.get(id).unwrap().component::<MenuContent>().unwrap();serde_json::json!({"id":id.0,"open":menu.menu.is_open(),"popup":menu.popup.as_ref().map(|layout|format!("{:?}",layout.popup))})}).collect::<Vec<_>>();
                let state = serde_json::json!({"pid":std::process::id(),"phase":self.review.as_ref().and_then(review::Review::phase),"active":window.is_active(),"show_menu_owners":ax::content_nodes(&self.sidebar.host).iter().filter(|node|node.can_show_menu).map(|node|node.key.component).collect::<Vec<_>>(),"primary_selection":self.sidebar.primary_selection,"commands":self.sidebar.commands,"selections":review::selection_records(&self.sidebar),"focus":self.sidebar.host.focused().map(|id|id.0),"text":input.text,"selection":input.selection(),"marked":input.marked,"menus":menus,"nodes":self.sidebar.host.semantics().iter().map(|node|serde_json::json!({"id":node.id.0,"mount":self.sidebar.host.semantic_mount(node.id),"role":format!("{:?}",node.semantics.role),"name":node.semantics.name,"frame":format!("{:?}",node.frame),"parts":node.parts.iter().map(|part|serde_json::json!({"index":part.index,"name":part.semantics.name,"disabled":part.semantics.disabled,"frame":format!("{:?}",part.frame)})).collect::<Vec<_>>()})).collect::<Vec<_>>()});
                std::fs::write(
                    path.join("state.json"),
                    serde_json::to_vec_pretty(&state).unwrap(),
                )
                .unwrap();
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
            if let Some(review) = &mut self.review {
                let evidence =
                    std::env::var_os("TEKESUI_CONTEXT_MENU_EVIDENCE").map(std::path::PathBuf::from);
                review.verify(&self.sidebar, viewport, evidence.as_deref());
            }
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            if let Some(path) = std::env::var_os("TEKESUI_CONTEXT_MENU_EVIDENCE") {
                use std::io::Write;
                let path = std::path::PathBuf::from(path);
                std::fs::create_dir_all(&path).unwrap();
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path.join("native-events.jsonl"))
                        .unwrap(),
                    "{}",
                    serde_json::json!({"native_event":format!("{event:?}")})
                )
                .unwrap();
            }
            match event {
                Event::Command(p::CommandId(id))
                    if self
                        .review
                        .as_mut()
                        .is_some_and(|review| review.command(id, &mut self.sidebar)) => {}
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
                Event::Focus(active) => self.sidebar.host.set_window_active(active),
                Event::Mouse(mouse) => {
                    self.sidebar.host.event(&native_pointer::translate(mouse));
                }
                Event::Scroll(scroll) => {
                    self.sidebar
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
                    self.sidebar
                        .host
                        .event_with_text(
                            &ContentEvent::KeyInput(native_key::key(key)),
                            &mut self.text,
                        )
                        .unwrap();
                }
                Event::TextKey(event) => {
                    self.sidebar
                        .host
                        .event_with_text(&native_key::translate(event), &mut self.text)
                        .unwrap();
                }
                Event::TextCommit { text, replacement } => {
                    self.sidebar
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
                    self.sidebar
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
                    self.sidebar
                        .host
                        .event_with_text(&ContentEvent::Unmark, &mut self.text)
                        .unwrap();
                }
                Event::TextCommand(command) => {
                    self.sidebar
                        .host
                        .event_with_text(&ContentEvent::TextCommand(command), &mut self.text)
                        .unwrap();
                }
                Event::Accessibility { target, action } => {
                    ax::content_action(&mut self.sidebar.host, target, action);
                }
                Event::CloseRequested => {
                    self.sidebar.host.set_window_active(false);
                    window.set_accessibility_text_layouts(Default::default());
                    window.set_accessibility_nodes(Vec::new());
                    window.close();
                    return;
                }
                Event::Redraw | Event::AnimationFrame(_) => {}
                _ => return,
            }
            self.sidebar.collect();
            self.sidebar.host.set_window_active(window.is_active());
            self.draw(window);
        }
    }
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut sidebar = Sidebar::new();
    let review = std::env::var_os("TEKESUI_CONTEXT_MENU_NATIVE_SELFTEST")
        .map(|_| review::Review::prepare(&mut sidebar));
    let menu = if review.is_some() {
        MenuBar {
            menus: vec![p::Menu::new(
                "Acceptance",
                (review::FIRST..=review::LAST)
                    .map(|id| {
                        p::MenuItem::Command(p::MenuCommand::new(
                            format!("Context checkpoint {id}"),
                            p::CommandId(id),
                        ))
                    })
                    .collect(),
            )],
        }
    } else {
        MenuBar::default()
    };
    App::new(AppConfig {
        name: Some("TekesUI Context Menu Review".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI Context Menu Review".into(),
            size: p::Size {
                width: 720.,
                height: 480.,
            },
            min_size: Some(p::Size {
                width: 320.,
                height: 260.,
            }),
            ..Default::default()
        },
        menu,
        Host {
            sidebar,
            text,
            gpu: None,
            clock: std::time::Instant::now(),
            review_started: false,
            review,
        },
    );
}
