//! A single ToolbarHost per window; adding groups adds NO event/overlay code.
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
    use coretext::Text;
    use std::time::Instant;
    use tekes_ui_composition_consumer::toolbar_mount::*;
    use tekes_ui_controls::{ButtonTextRenderer, composition::*, paint};
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Size, Window, WindowConfig, WindowHandler,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::Theme;
    struct Host {
        toolbar: ToolbarHost<Source>,
        text: Text,
        gpu: Option<Renderer>,
        started: Instant,
        ax: Vec<(p::AccessibilityKey, ToolbarAccessibilityTarget)>,
        next: u64,
        visible: bool,
        removed: bool,
        moved: bool,
        commands: Vec<serde_json::Value>,
    }
    impl Host {
        fn publish(&mut self, w: &Window) {
            let snapshot = self.toolbar.semantics();
            let old = std::mem::take(&mut self.ax);
            let mut assigned = Vec::new();
            let mut assign = |target| {
                let key = old
                    .iter()
                    .find(|(_, t)| *t == target)
                    .map(|(k, _)| *k)
                    .unwrap_or_else(|| {
                        self.next += 1;
                        p::AccessibilityKey {
                            component: self.next,
                            part: None,
                        }
                    });
                assigned.push((key, target));
                key
            };
            let mut owners = Vec::new();
            let mut parts = Vec::new();
            for node in &snapshot {
                owners.push((node.node.id, assign(node.target)));
                for (target, part) in &node.parts {
                    parts.push((node.node.id, part.index, assign(*target)));
                }
            }
            let project =
                |key, parent, frame: Rect, focused, s: &ContentSemantics| p::AccessibilityNode {
                    key,
                    parent,
                    role: match s.role {
                        ContentRole::Text => p::AccessibilityRole::StaticText,
                        ContentRole::Button => p::AccessibilityRole::Button,
                        ContentRole::MenuButton => p::AccessibilityRole::MenuButton,
                        ContentRole::MenuItem => p::AccessibilityRole::MenuItem,
                        _ => p::AccessibilityRole::Group,
                    },
                    frame: p::Rect {
                        origin: p::Point {
                            x: frame.min_x() as f64,
                            y: frame.min_y() as f64,
                        },
                        size: Size {
                            width: frame.size.width as f64,
                            height: frame.size.height as f64,
                        },
                    },
                    label: s.name.clone(),
                    value: s.value.clone(),
                    description: s.description.clone(),
                    enabled: !s.disabled,
                    focused,
                    selected: s.selected,
                    expanded: s.expanded,
                    invalid: s.invalid,
                    required: s.required,
                    text_selection: None,
                    read_only: false,
                };
            let mut nodes = Vec::new();
            for node in snapshot {
                let key = owners.iter().find(|(id, _)| *id == node.node.id).unwrap().1;
                let parent = node
                    .node
                    .parent
                    .and_then(|p| owners.iter().find(|(id, _)| *id == p).map(|(_, k)| *k));
                nodes.push(project(
                    key,
                    parent,
                    node.node.frame,
                    node.node.focused,
                    &node.node.semantics,
                ));
                for (_, part) in node.parts {
                    let pk = parts
                        .iter()
                        .find(|(id, index, _)| *id == node.node.id && *index == part.index)
                        .unwrap()
                        .2;
                    nodes.push(project(
                        pk,
                        Some(key),
                        part.frame,
                        part.focused,
                        &part.semantics,
                    ));
                }
            }
            if let Some(dir) = std::env::var_os("TEKESUI_TOOLBAR_MOUNT_EVIDENCE") {
                let dir = std::path::PathBuf::from(dir);
                std::fs::create_dir_all(&dir).unwrap();
                let screen = w.screen_info();
                let data = serde_json::json!({"pid":std::process::id(),"active":w.is_active(),"type_scale":self.text.scale,"screen":screen.map(|s|serde_json::json!({"name":s.name,"builtin":s.builtin})),"width":w.size().width,"commands":self.commands,"hint":self.toolbar.help_tooltip().map(|(_,text,_)|text),"nodes":nodes.iter().map(|n|serde_json::json!({"key":n.key.component,"name":n.label,"role":format!("{:?}",n.role),"focused":n.focused,"x":n.frame.origin.x,"y":n.frame.origin.y,"width":n.frame.size.width,"height":n.frame.size.height})).collect::<Vec<_>>()});
                std::fs::write(
                    dir.join("state.json"),
                    serde_json::to_vec_pretty(&data).unwrap(),
                )
                .unwrap();
                use std::io::Write;
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(dir.join("journal.jsonl"))
                        .unwrap(),
                    "{data}"
                )
                .unwrap();
            }
            w.set_accessibility_nodes(nodes);
            self.ax = assigned;
        }
        fn draw(&mut self, w: &mut Window) {
            self.text.now = self.started.elapsed();
            self.text.backing = w.scale() as f32;
            self.text.requested = false;
            self.text.atlas.begin_frame();
            let width = w.size().width as f32;
            let height = w.size().height as f32;
            self.toolbar
                .set_regions(7, Rect::new(0., 0., width, height), regions(width))
                .unwrap();
            self.toolbar.set_window_active(w.is_active());
            let mut scene = Scene::new();
            paint::surface(
                &mut scene,
                Rect::new(0., 0., width, height),
                Theme::LIGHT,
                false,
            );
            let caption = "One ToolbarHost · Main + Settings + Progress\nCommand +/-: zoom · Command 1: hide/show · Command 2: remove/add · Command 3: move group";
            self.text.draw(
                &mut scene,
                caption,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                Rect::new(20., 120., width - 40., 80.),
                tekes_ui_controls::ButtonTextAlign::Leading,
                paint::color(Theme::LIGHT, "primary-text"),
            );
            self.toolbar
                .render(&mut scene, &mut self.text, Theme::LIGHT.into())
                .unwrap();
            if let Some(gpu) = &mut self.gpu {
                gpu.render(
                    &scene,
                    &mut self.text.atlas,
                    paint::color(Theme::LIGHT, "surface-1"),
                )
                .unwrap();
            }
            w.set_animating(self.text.requested);
            self.publish(w);
        }
        fn actions(&mut self) {
            for c in self.toolbar.drain_commands() {
                self.commands.push(serde_json::json!({"group":c.group.0,"command":c.command,"source":c.source.module,"generation":c.source.generation,"window":c.window,"input":format!("{:?}",c.input)}));
            }
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, w: &mut Window) {
            match event {
                Event::Created => {
                    self.gpu = Some(unsafe {
                        Renderer::new_for_metal_layer(
                            w.metal_layer(),
                            w.drawable_size(),
                            w.scale() as f32,
                        )
                        .unwrap()
                    });
                    self.draw(w);
                }
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => {
                    if let Some(gpu) = &mut self.gpu {
                        gpu.resize(drawable_size, scale as f32);
                    }
                    self.draw(w);
                }
                Event::Redraw | Event::AnimationFrame(_) => self.draw(w),
                Event::Focus(active) => {
                    self.toolbar.set_window_active(active);
                    self.draw(w);
                }
                Event::Mouse(mouse) => {
                    self.toolbar.event(&native_pointer::translate(mouse));
                    self.actions();
                    self.draw(w);
                }
                Event::Key(k)
                    if k.kind == p::KeyEventKind::Down
                        && k.modifiers.command
                        && matches!(k.key_code, 18 | 19 | 20 | 24 | 27 | 29) =>
                {
                    match k.key_code {
                        24 => self.text.scale += 0.1,
                        27 => self.text.scale = (self.text.scale - 0.1).max(0.5),
                        29 => self.text.scale = 1.,
                        18 if !self.removed => {
                            self.visible = !self.visible;
                            self.toolbar.set_visible(EDIT, self.visible).unwrap();
                        }
                        19 => {
                            if self.removed {
                                self.toolbar.register(undo_redo()).unwrap();
                            } else {
                                self.toolbar.remove(EDIT).unwrap();
                            }
                            self.removed = !self.removed;
                        }
                        20 if !self.removed => {
                            self.moved = !self.moved;
                            self.toolbar
                                .move_group(
                                    EDIT,
                                    if self.moved { MAIN } else { SETTINGS },
                                    ToolbarPlacement::PrimaryAction,
                                    0,
                                )
                                .unwrap();
                        }
                        _ => {}
                    }
                    self.draw(w);
                }
                Event::Key(k) => {
                    self.toolbar
                        .event(&ContentEvent::KeyInput(native_key::key(k)));
                    self.actions();
                    self.draw(w);
                }
                Event::Accessibility { target, action } => {
                    let Some((_, target)) = self.ax.iter().find(|(key, _)| *key == target).copied()
                    else {
                        return;
                    };
                    let action = match action {
                        p::AccessibilityAction::Focus => AccessibilityAction::Focus,
                        p::AccessibilityAction::Press => AccessibilityAction::Press,
                        _ => return,
                    };
                    self.toolbar.accessibility_action(target, action);
                    self.actions();
                    self.draw(w);
                }
                Event::TextKey(marker) => {
                    self.toolbar.event(&native_key::translate(marker));
                    self.actions();
                    self.draw(w);
                }
                Event::TextCommand(command) => {
                    self.toolbar.event(&ContentEvent::TextCommand(command));
                    self.actions();
                    self.draw(w);
                }
                _ => {}
            }
        }
    }
    let mut text = Text::new(1.);
    text.services = true;
    let app = App::new(AppConfig {
        name: Some("TekesUI Toolbar Mount Review".into()),
        ..Default::default()
    });
    app.run(
        WindowConfig {
            title: "TekesUI Toolbar Mount Review".into(),
            size: Size {
                width: 900.,
                height: 360.,
            },
            min_size: Some(Size {
                width: 300.,
                height: 220.,
            }),
            placement: p::WindowPlacement::Screen(p::ScreenChoice::Builtin),
            ..Default::default()
        },
        MenuBar::default(),
        Host {
            toolbar: build(900.).unwrap(),
            text,
            gpu: None,
            started: Instant::now(),
            ax: vec![],
            next: 0,
            visible: true,
            removed: false,
            moved: false,
            commands: vec![],
        },
    );
}
