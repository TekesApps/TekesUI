//! Opt-in Gallery host for the independent public action consumer and existing AX bridge.
use crate::gallery::text::TextEngine;
use crate::{reading_key as native_key, reading_pointer as native_pointer};
use tekes_ui_composition_consumer::content_actions::{ActionFixture, FIRST, SECOND};
use tekes_ui_controls::composition::ContentEvent as C;
use tekes_ui_platform_macos::{
    App, AppConfig, Event, KeyEventKind, MenuBar, ScrollPhase, Window, WindowConfig, WindowHandler,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, Spacing, Theme};
pub(crate) fn try_run() -> bool {
    let Some(out) = std::env::var_os("TEKESUI_CONTENT_ACTIONS_AUDIT_OUT") else {
        return false;
    };
    let mut fixture = ActionFixture::new();
    fixture
        .host
        .get_mut(SECOND)
        .unwrap()
        .component_mut::<tekes_ui_controls::composition::ButtonContent>()
        .unwrap()
        .button
        .state
        .disabled = true;
    fixture.host.focus(Some(FIRST), true);
    App::new(AppConfig {
        name: Some("TekesUI Gallery 公共动作".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI Gallery · 公共动作".into(),
            size: tekes_ui_platform_macos::Size {
                width: 760.,
                height: 380.,
            },
            placement: tekes_ui_platform_macos::WindowPlacement::Auto {
                need_120hz: false,
                size_check: true,
            },
            ..Default::default()
        },
        MenuBar::default(),
        Host {
            fixture,
            text: TextEngine::new(),
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
    fixture: ActionFixture,
    text: TextEngine,
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
                        text: s.input.text.clone(),
                        selection: s.input.selection(),
                        marked: s.input.marked.clone(),
                        caret_index: s.input.caret,
                        caret_origin: tekes_ui_platform_macos::Point {
                            x: caret.min_x() as f64,
                            y: caret.min_y() as f64,
                        },
                        caret_size: tekes_ui_platform_macos::Size {
                            width: caret.size.width as f64,
                            height: caret.size.height as f64,
                        },
                        read_only: s.input.read_only,
                        ..Default::default()
                    }
                }),
        );
        use tekes_ui_controls::composition::ContentRole as R;
        use tekes_ui_platform_macos::{
            AccessibilityKey as K, AccessibilityNode as N, AccessibilityRole as A,
        };
        let key = |id: tekes_ui_controls::composition::ComponentId, part| K {
            component: id.0,
            part,
        };
        let project =
            |key,
             parent,
             frame: Rect,
             focused,
             semantics: &tekes_ui_controls::composition::ContentSemantics| N {
                key,
                parent,
                role: match semantics.role {
                    R::Group => A::Group,
                    R::Text => A::StaticText,
                    R::Button => A::Button,
                    R::Link => A::Link,
                    R::TextInput => A::TextField,
                    R::Switch => A::Switch,
                    R::List => A::List,
                    R::ListItem => A::ListItem,
                    R::Dialog => A::Dialog,
                    R::MenuButton => A::MenuButton,
                    R::Menu => A::Menu,
                    R::MenuItem => A::MenuItem,
                    _ => A::Group,
                },
                frame: tekes_ui_platform_macos::Rect {
                    origin: tekes_ui_platform_macos::Point {
                        x: frame.min_x() as f64,
                        y: frame.min_y() as f64,
                    },
                    size: tekes_ui_platform_macos::Size {
                        width: frame.size.width as f64,
                        height: frame.size.height as f64,
                    },
                },
                label: semantics.name.clone(),
                value: semantics.value.clone(),
                description: semantics.description.clone(),
                enabled: !semantics.disabled,
                focused,
                selected: semantics.selected,
                expanded: semantics.expanded,
                invalid: semantics.invalid,
                required: semantics.required,
                text_selection: None,
                read_only: false,
            };
        let mut nodes = Vec::new();
        for node in self.fixture.host.semantics() {
            let mut native = project(
                key(node.id, None),
                node.parent.map(|p| key(p, None)),
                node.frame,
                node.focused && !node.parts.iter().any(|part| part.focused),
                &node.semantics,
            );
            if let Some(domain) = self.fixture.host.value_range(node.id) {
                let range = tekes_ui_platform_macos::AccessibilityRange::new(
                    *domain.start(),
                    *domain.end(),
                )
                .expect("finite public range");
                native.role = A::Slider(range);
            }
            if let Some(surface) = self.fixture.host.text_surface(node.id) {
                if surface.input.multiline {
                    native.role = A::TextArea;
                }
                native.text_selection = Some(surface.input.selection());
                native.read_only = surface.input.read_only;
            }
            nodes.push(native);
            for part in node.parts {
                nodes.push(project(
                    key(node.id, Some(part.index)),
                    Some(key(
                        node.id,
                        self.fixture.host.semantic_part_parent(node.id, part.index),
                    )),
                    part.frame,
                    part.focused,
                    &part.semantics,
                ));
            }
        }
        w.set_accessibility_nodes(nodes);
    }
    fn save(&self, w: &Window) {
        let rect = |r: Rect| serde_json::json!({"x":r.min_x(),"y":r.min_y(),"width":r.size.width,"height":r.size.height});
        let actions:Vec<_>=self.fixture.delivered.iter().map(|a|serde_json::json!({"target":a.target.0,"kind":format!("{:?}",a.kind),"source":format!("{:?}",a.source)})).collect();
        let semantics:Vec<_>=self.fixture.host.semantics().into_iter().map(|n|serde_json::json!({"id":n.id.0,"name":n.semantics.name,"frame":rect(n.frame),"disabled":n.semantics.disabled,"focused":n.focused})).collect();
        std::fs::create_dir_all(&self.out).unwrap();
        std::fs::write(self.out.join("action-events.json"),serde_json::to_vec_pretty(&serde_json::json!({"actions":actions,"focus":self.fixture.host.focused().map(|id|id.0),"semantics":semantics,"events":self.reports,"screen":format!("{:?}",w.screen_info()),"backing_scale":w.scale(),"scope":"actual Gallery public controls/key/pointer + existing native accessibility projection + Metal; not physicalIME/VoiceOver/performance acceptance"})).unwrap()).unwrap();
    }
    fn paint(&mut self, w: &Window) {
        self.text.set_scale(w.scale() as f32);
        self.text.set_locale(crate::gallery::locale::Locale::ZhCn);
        self.text.set_motion_enabled(true);
        self.text
            .set_motion_context(self.start.elapsed(), self.reduced);
        self.text.begin_frame();
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
                self.text.atlas_mut(),
                Palette::from(self.theme).color("surface-1").unwrap(),
            )
            .unwrap();
        }
        w.set_animating(self.text.motion_frame_requested());
        self.publish(w);
        self.save(w);
    }
    fn input(&mut self, event: Event, w: &Window) {
        let e = match event {
            Event::Mouse(m) => native_pointer::translate(m),
            Event::TextKey(report) => native_key::translate(report),
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
                if c == "copy:" {
                    if let Some(surface) = self.fixture.host.focused_text() {
                        w.set_clipboard_text(&surface.input.selected_text());
                    }
                    return;
                }
                if c == "paste:" {
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
                C::TextCommand(c)
            }
            Event::Key(k) => {
                if k.kind == KeyEventKind::Down && k.modifiers.command {
                    let command = match k.key_code {
                        0 => Some("selectAll:"),
                        6 if k.modifiers.shift => Some("redo:"),
                        6 => Some("undo:"),
                        8 => Some("copy:"),
                        9 => Some("paste:"),
                        _ => None,
                    };
                    if let Some(command) = command {
                        return self.input(Event::TextCommand(command.into()), w);
                    }
                }
                C::KeyInput(native_key::key(k))
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
        let response = self.fixture.event(&e);
        if self.reports.len() < 512 {
            self.reports.push(serde_json::json!({"event":format!("{e:?}"),"response":format!("{response:?}"),"actions":format!("{:?}",self.fixture.delivered)}));
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
            Event::Accessibility { target, action } => {
                use tekes_ui_controls::composition::{AccessibilityAction as A, ComponentId};
                let action = match action {
                    tekes_ui_platform_macos::AccessibilityAction::Increment => A::Increment,
                    tekes_ui_platform_macos::AccessibilityAction::Decrement => A::Decrement,
                    tekes_ui_platform_macos::AccessibilityAction::Focus => A::Focus,
                    tekes_ui_platform_macos::AccessibilityAction::Press => A::Press,
                    tekes_ui_platform_macos::AccessibilityAction::SetValue(v) => A::SetValue(v),
                    tekes_ui_platform_macos::AccessibilityAction::SetSelection(v) => {
                        A::SetSelection(v)
                    }
                    tekes_ui_platform_macos::AccessibilityAction::ReplaceSelectedText(v) => {
                        A::ReplaceSelectedText(v)
                    }
                };
                let response = self.fixture.accessibility_action(
                    ComponentId(target.component),
                    target.part,
                    action.clone(),
                );
                self.reports.push(serde_json::json!({"native_accessibility":format!("{action:?}"),"target":target.component,"response":format!("{response:?}")}));
                self.publish(w);
                self.save(w);
                w.request_redraw();
            }
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
