//! Standalone native consumer smoke for the heterogeneous List and Rich Dialog.
//! This executable imports the consumer fixture, never Gallery.
#[cfg(target_os = "macos")]
#[path = "support/native_pointer.rs"]
mod native_pointer;

#[cfg(target_os = "macos")]
#[path = "support/native_key.rs"]
mod native_key;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the native composition consumer requires macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_composition_consumer::scenarios::{self, ids};
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, TextSurfaceHost,
        TextSurfaceTextRenderer, composition::ContentHost,
    };
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, KeyEventKind, MenuBar, ScrollPhase, Size, Window, WindowConfig,
        WindowHandler,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size as SSize};
    use tekes_ui_text::{
        coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
        model::{Paragraph, ShapeOptions},
        push::push_paragraph_visible,
        style::TextStyle,
        traits::Shaper,
    };
    use tekes_ui_tokens::{Palette, Theme, proposal};

    struct TextBackend {
        shaper: CoreTextShaper,
        rasterizer: CoreTextRasterizer,
        scale: f32,
        type_scale: f32,
        atlas: GlyphAtlas,
        started: std::time::Instant,
        frame_requested: bool,
        reduced_motion: bool,
        window_active: bool,
        caret_on: bool,
        services_enabled: bool,
    }
    impl TextBackend {
        fn shape(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> Rc<Paragraph> {
            let m = role.style();
            let style = if proposal::text_family(role.id()) == Some("mono") {
                TextStyle::named("Menlo", m.size * self.type_scale)
            } else {
                TextStyle::system(m.size * self.type_scale)
            }
            .with_weight(m.weight);
            Rc::new(self.shaper.shape(
                text,
                &style,
                &ShapeOptions {
                    width: width.map(f64::from),
                    line_spacing: 0.,
                },
            ))
        }
        fn push(
            &mut self,
            s: &mut Scene,
            p: &Paragraph,
            o: Point,
            c: tekes_ui_tokens::Rgba,
            clip: Rect,
        ) {
            push_paragraph_visible(
                s,
                &mut self.atlas,
                &mut self.rasterizer,
                p,
                o,
                self.scale,
                c,
                clip.min_y()..clip.max_y(),
            );
        }
    }
    impl TextSurfaceTextRenderer for TextBackend {
        fn push_visible(
            &mut self,
            s: &mut Scene,
            p: &Paragraph,
            o: Point,
            c: tekes_ui_tokens::Rgba,
            clip: Rect,
        ) {
            self.push(s, p, o, c, clip)
        }
    }
    impl TextSurfaceHost for TextBackend {
        fn shape_label(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: f32,
            options: tekes_ui_text::LabelOptions,
        ) -> Option<Rc<tekes_ui_text::LabelLayout>> {
            let metrics = role.style();
            let style = if proposal::text_family(role.id()) == Some("mono") {
                TextStyle::named("Menlo", metrics.size * self.type_scale)
            } else {
                TextStyle::system(metrics.size * self.type_scale)
            }
            .with_weight(metrics.weight);
            Some(Rc::new(
                self.shaper
                    .shape_label(
                        text,
                        &style,
                        &ShapeOptions::with_width(f64::from(width)),
                        options,
                    )
                    .with_min_line_height(f64::from(metrics.line_height * self.type_scale)),
            ))
        }

        fn shape_surface(
            &mut self,
            s: &str,
            r: proposal::TextRole,
            w: Option<f32>,
        ) -> Rc<Paragraph> {
            self.shape(s, r, w)
        }
        fn type_scale(&self) -> f32 {
            self.type_scale
        }
        fn caret_visible(&self) -> bool {
            self.caret_on
        }
    }
    impl ButtonTextRenderer for TextBackend {
        fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> SSize {
            let p = self.shape(s, r, Some(w));
            SSize::new(p.width as f32, p.height as f32)
        }
        fn draw(
            &mut self,
            s: &mut Scene,
            text: &str,
            r: proposal::TextRole,
            rect: Rect,
            a: ButtonTextAlign,
            color: tekes_ui_tokens::Rgba,
        ) {
            let p = self.shape(text, r, Some(rect.size.width));
            let x = match a {
                ButtonTextAlign::Leading => rect.min_x(),
                ButtonTextAlign::Center => rect.min_x() + (rect.size.width - p.width as f32) * 0.5,
            };
            let y = rect.min_y() + (rect.size.height - p.height as f32) * 0.5;
            self.push(s, &p, Point::new(x.round(), y.round()), color, rect)
        }
        fn backing_scale(&self) -> f32 {
            self.scale
        }
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: self.started.elapsed(),
                // Inactive windows settle effects without changing the system preference.
                reduced_motion: self.reduced_motion || !self.window_active,
            })
        }
        fn request_animation_frame(&mut self) {
            self.frame_requested = true;
        }
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            if self.services_enabled {
                Some(self)
            } else {
                None
            }
        }
    }
    struct Host {
        selftest_script: Option<Vec<(u64, tekes_ui_platform_macos::selftest::Action)>>,
        tree: ContentHost,
        menu_actions: Vec<tekes_ui_controls::composition::ContentActionRecord>,
        reading: Option<tekes_ui_composition_consumer::reading_stream::StreamState>,
        reading_width: bool,
        item_version: Option<tekes_ui_controls::ReadingDocumentVersion>,
        item_error: Option<String>,
        layout_collapsed: bool,
        layout_error: Option<String>,
        layout_baseline: Option<tekes_ui_controls::TextEditBaseline>,
        plain_value: Option<tekes_ui_controls::DraftRevision>,
        gpu: Option<Renderer>,
        text: TextBackend,
        scene: Scene,
        palette: Palette,
        mode: String,
        invoker: tekes_ui_controls::composition::ComponentId,
        dialog: tekes_ui_controls::composition::ComponentId,
        invoker_activations: usize,
        cancel_activations: usize,
        save_activations: usize,
        system_theme: Theme,
        row_activations: Option<Rc<Cell<usize>>>,
    }
    impl Host {
        fn text_state(
            s: &tekes_ui_controls::TextSurface,
            visible_only: bool,
        ) -> tekes_ui_platform_macos::TextInputState {
            let r = s.caret();
            let origin = s.origin();
            tekes_ui_platform_macos::TextInputState {
                caret_index: s.input.caret,
                text: s.input.text.clone(),
                selection: s.input.selection(),
                marked: s.input.marked.clone(),
                caret_origin: tekes_ui_platform_macos::Point {
                    x: r.min_x() as f64,
                    y: r.min_y() as f64,
                },
                caret_size: Size {
                    width: r.size.width as f64,
                    height: r.size.height as f64,
                },
                read_only: s.input.read_only,
                carets: s
                    .paragraph
                    .as_ref()
                    .map(|p| {
                        p.lines
                            .iter()
                            .filter(|line| {
                                !visible_only
                                    || origin.y + (line.top + line.height) as f32 >= s.inner.min_y()
                                        && origin.y + line.top as f32 <= s.inner.max_y()
                            })
                            .flat_map(|line| {
                                line.carets.iter().map(move |(index, x)| {
                                    (
                                        *index,
                                        tekes_ui_platform_macos::Point {
                                            x: f64::from(origin.x) + x,
                                            y: f64::from(origin.y) + line.top,
                                        },
                                        Size {
                                            width: f64::from(tekes_ui_tokens::Stroke::Rule.value()),
                                            height: line.height,
                                        },
                                    )
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        }
        fn publish(&self, w: &Window) {
            // A failed frame must not publish its partially arranged text/AX tree.
            let valid = self.layout_error.is_none();
            let state = valid
                .then(|| self.tree.focused_text())
                .flatten()
                .map(|s| Self::text_state(s, true));
            w.set_text_input_state(state);
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
                        size: Size {
                            width: frame.size.width as f64,
                            height: frame.size.height as f64,
                        },
                    },
                    label: semantics.name.clone(),
                    value: semantics.value.clone(),
                    description: semantics.description.clone(),
                    enabled: !semantics.disabled,
                    focused: focused && w.is_active(),
                    selected: semantics.selected,
                    expanded: semantics.expanded,
                    invalid: semantics.invalid,
                    required: semantics.required,
                    text_selection: None,
                    read_only: false,
                };
            let mut nodes = Vec::new();
            for node in self.tree.semantics().into_iter().filter(|_| valid) {
                let mut native = project(
                    key(node.id, None),
                    node.parent.map(|p| key(p, None)),
                    node.frame,
                    node.focused && !node.parts.iter().any(|part| part.focused),
                    &node.semantics,
                );
                if let Some(domain) = self.tree.value_range(node.id) {
                    let range = tekes_ui_platform_macos::AccessibilityRange::new(
                        *domain.start(),
                        *domain.end(),
                    )
                    .expect("finite public range");
                    native.role = A::Slider(range);
                }
                if let Some(surface) = self.tree.text_surface(node.id) {
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
                            self.tree.semantic_part_parent(node.id, part.index),
                        )),
                        part.frame,
                        part.focused,
                        &part.semantics,
                    ));
                }
            }
            if let Some(directory) = std::env::var_os("TEKESUI_COMPONENT_EVIDENCE") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).expect("component evidence directory");
                let screen = w.screen_info();
                let mut snapshot = serde_json::json!({
                    "mode": self.mode, "window_active": w.is_active(),
                    "screen": screen.map(|s| serde_json::json!({"name":s.name,"builtin":s.builtin,"scale":s.scale})),
                    "nodes": nodes.iter().map(|n| serde_json::json!({
                        "component":n.key.component,"part":n.key.part,"role":format!("{:?}",n.role),
                        "label":n.label,"value":n.value,"required":n.required,"enabled":n.enabled,"focused":n.focused,"selected":n.selected,"expanded":n.expanded,
                        "parent":n.parent.map(|key|serde_json::json!({"component":key.component,"part":key.part})),
                        "frame":{"x":n.frame.origin.x,"y":n.frame.origin.y,"width":n.frame.size.width,"height":n.frame.size.height}
                    })).collect::<Vec<_>>()
                });
                if self.mode == "menu-tree" {
                    snapshot["menu_actions"]=serde_json::json!(self.menu_actions.iter().map(|record|{
                        let selection=record.payload.as_ref().map(|tekes_ui_controls::composition::ContentActionPayload::MenuSelection(value)|serde_json::json!({"index":value.index(),"label":value.label(),"model_revision":value.model_revision(),"value_path":value.value_path()}));
                        serde_json::json!({"target":record.action.target.0,"source":format!("{:?}",record.action.source),"kind":format!("{:?}",record.action.kind),"mount":record.mount,"selection":selection})
                    }).collect::<Vec<_>>());
                }
                snapshot["text_input"] = valid.then(|| self.tree.focused_text()).flatten().map_or(
                    serde_json::Value::Null,
                    |surface| {
                        let selection = surface.input.selection();
                        serde_json::json!({
                            "text": surface.input.text,
                            "selection": [selection.start, selection.end],
                            "marked": surface.input.marked.as_ref().map(|range| [range.start, range.end]),
                        })
                    },
                );
                if let Some(value) = &self.plain_value {
                    use tekes_ui_composition_consumer::plain_text::{BODY, SCROLLER};
                    let surface = self.tree.text_surface(BODY).unwrap();
                    let parent = self
                        .tree
                        .get(SCROLLER)
                        .unwrap()
                        .component::<tekes_ui_controls::composition::ScrollContent>()
                        .unwrap();
                    snapshot["plain_document"] = serde_json::json!({
                        "version": value.version,
                        "parent_offset": parent.scroll.offset.y,
                        "nested_offset": surface.scroll.offset.y,
                        "wrap_width": surface.width,
                        "body_height": surface.outer.size.height,
                    });
                    if std::env::var("TEKES_SELFTEST").as_deref() == Ok("text") {
                        // The existing platform test pasteboard is private to
                        // this process; never log the user's general clipboard.
                        snapshot["plain_document"]["copied_text"] =
                            serde_json::json!(w.clipboard_text());
                    }
                }
                if self.mode == "item" {
                    use tekes_ui_composition_consumer::item_navigation::{
                        COMPOSER, INNER, OTHER, OWNER,
                    };
                    snapshot["item_review"] = serde_json::json!({
                        "error": self.item_error,
                        "result": self.tree.scroll_reading_reveal_result().map(|r| serde_json::json!({"owner":r.owner.0,"item":r.item.0,"version":format!("{:?}",r.version),"status":format!("{:?}",r.status),"changed":r.changed,"clamped":r.clamped})),
                        "focus":self.tree.focused().map(|id|id.0),"reduced_motion":self.text.reduced_motion,"type_scale":self.text.type_scale,
                        "composer_baseline":format!("{:?}",self.tree.get(COMPOSER).unwrap().component::<tekes_ui_controls::composition::TextFieldContent>().unwrap().surface.input.external_baseline()),
                        "scrolls":([OWNER,INNER,OTHER].into_iter().map(|id| {let scroll=&self.tree.get(id).unwrap().component::<tekes_ui_controls::composition::ScrollContent>().unwrap().scroll;serde_json::json!({"id":id.0,"x":scroll.offset.x,"y":scroll.offset.y,"limit_y":scroll.limit().y})}).collect::<Vec<_>>())
                    });
                }
                if self.mode == "layout" {
                    use tekes_ui_composition_consumer::layout_activation::{FIELD, PANE, Pane};
                    use tekes_ui_controls::composition::TextFieldContent;
                    let field = self
                        .tree
                        .get(FIELD)
                        .unwrap()
                        .component::<TextFieldContent>()
                        .unwrap();
                    snapshot["layout_review"] = serde_json::json!({
                        "collapsed": self.layout_collapsed, "services_enabled": self.text.services_enabled,
                        "error": self.layout_error, "focus": self.tree.focused().map(|id| id.0),
                        "pane_active": self.tree.get(PANE).unwrap().component::<Pane>().unwrap().layout_active(),
                        "original_baseline_retained": self.layout_baseline.as_ref() == Some(&field.surface.input.external_baseline()),
                        "original_value": field.surface.input.text,
                        "paragraph_present": field.surface.paragraph.is_some(),
                        "type_scale": self.text.type_scale,
                    });
                }
                if self.mode == "label" {
                    use tekes_ui_controls::composition::{
                        ButtonContent, ComponentId, CompoundButton,
                    };
                    snapshot["label_review"] = serde_json::json!({
                        "type_scale": self.text.type_scale, "width": w.size().width,
                        "buttons": ([9902,9903].into_iter().map(|id| {
                            let node=self.tree.get(ComponentId(id)).unwrap();
                            let button=node.component::<ButtonContent>().or_else(|| node.component::<CompoundButton>().map(|c| &c.action)).unwrap();
                            serde_json::json!({"id":id,"activations":button.activations,"hovered":button.button.state.hovered,"pressed":button.button.state.pressed,"focused":button.button.state.focused,"focus_visible":button.button.state.focus_visible,"disabled":button.button.state.disabled,"loading":button.button.state.loading})
                        }).collect::<Vec<_>>())
                    });
                }
                if let Some(reading) = &self.reading {
                    use tekes_ui_composition_consumer::reading_stream::{COMPOSER, TRANSCRIPT};
                    let surface = self.tree.text_surface(TRANSCRIPT).unwrap();
                    let field = self
                        .tree
                        .get(TRANSCRIPT)
                        .unwrap()
                        .component::<tekes_ui_controls::composition::TextFieldContent>()
                        .unwrap();
                    snapshot["reading"] = serde_json::json!({
                        "appended": reading.appended, "pending": reading.pending,
                        "following": field.reading.as_ref().unwrap().is_following_latest(),
                        "offset": surface.scroll.offset.y, "limit": surface.scroll.limit().y,
                        "anchor": surface.reflow_anchor.map(|a| serde_json::json!({"utf16":a.utf16,"within_line":a.within_line})),
                        "composer": self.tree.text_surface(COMPOSER).unwrap().input.text,
                        "focus": self.tree.focused().map(|id| id.0),
                        "type_scale": self.text.type_scale, "width": w.size().width,
                    });
                }
                if self.mode == "item" {
                    use std::io::Write;
                    snapshot["item_review"]["elapsed_ms"] =
                        serde_json::json!(self.text.started.elapsed().as_millis());
                    let mut history = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(directory.join("frame-history.jsonl"))
                        .expect("owned native phase evidence");
                    writeln!(history, "{}", serde_json::to_string(&snapshot).unwrap())
                        .expect("native phase record");
                }
                std::fs::write(
                    directory.join("current.json"),
                    serde_json::to_vec_pretty(&snapshot).unwrap(),
                )
                .expect("component snapshot");
            }
            w.set_accessibility_text_layouts(
                self.tree
                    .semantics()
                    .into_iter()
                    .filter(|_| valid)
                    .filter_map(|n| {
                        let surface = self.tree.text_surface(n.id)?;
                        let paragraph = surface.paragraph.as_ref()?;
                        Some((
                            key(n.id, None),
                            tekes_ui_platform_macos::AccessibilityTextLayout {
                                state: Self::text_state(surface, false),
                                lines: paragraph.lines.iter().map(|l| l.utf16.clone()).collect(),
                            },
                        ))
                    })
                    .collect(),
            );
            w.set_accessibility_nodes(nodes);
        }
        fn redraw(&mut self, w: &Window) {
            if let Some(reading) = &self.reading {
                let width = if reading.narrow { 480. } else { 720. };
                if reading.narrow != self.reading_width {
                    self.reading_width = reading.narrow;
                    w.set_content_size(Size {
                        width,
                        height: w.size().height,
                    });
                }
            }
            let stream_pending = self.reading.as_mut().is_some_and(|reading| {
                reading.tick(&mut self.tree, self.text.started.elapsed(), w.is_active())
            });
            let clip = Rect::new(0., 0., w.size().width as f32, w.size().height as f32);
            let mut frame = clip.inset(tekes_ui_tokens::Spacing::Regular.value());
            if self.mode == "layout" && self.layout_collapsed {
                frame.size.width = 0.;
            }
            self.text.window_active = w.is_active();
            self.text.scale = w.scale() as f32;
            self.text.frame_requested = false;
            self.text.atlas.begin_frame();
            self.scene.clear();
            let result =
                self.tree
                    .render(&mut self.scene, &mut self.text, frame, clip, self.palette);
            if self.mode == "layout" {
                self.layout_error = result.err().map(|error| format!("{error:?}"));
                if self.layout_error.is_some() {
                    self.scene.clear();
                }
            } else {
                result.expect("valid consumer scene");
            }
            self.publish(w);
            if std::env::var_os("TEKES_COMPOSITION_TRACE_PRESENCE").is_some()
                && self.mode == "dialog"
            {
                let dialog = self
                    .tree
                    .get(self.dialog)
                    .unwrap()
                    .component::<tekes_ui_controls::composition::DialogContent>()
                    .unwrap();
                eprintln!(
                    "presence open={} presented={} animating={} reduced={} frame_requested={}",
                    dialog.is_open(),
                    dialog.is_presented(),
                    dialog.needs_animation(),
                    self.text.reduced_motion,
                    self.text.frame_requested
                );
            }
            if let Some(r) = &mut self.gpu {
                r.render(
                    &self.scene,
                    &mut self.text.atlas,
                    self.palette.color("surface-1").unwrap(),
                )
                .expect("native render");
            }
            w.set_animating(self.text.frame_requested || stream_pending);
        }
        fn dialog_is_open(&self) -> bool {
            self.tree
                .get(self.dialog)
                .and_then(|content| {
                    content.component::<tekes_ui_controls::composition::DialogContent>()
                })
                .is_some_and(tekes_ui_controls::composition::DialogContent::is_open)
        }
        fn navigate_item(
            &mut self,
            placement: tekes_ui_controls::ScrollRevealPlacement,
            animated: bool,
        ) {
            use tekes_ui_composition_consumer::item_navigation::{OWNER, TARGET};
            let margin = tekes_ui_tokens::Spacing::Regular.value();
            let result = self.tree.reveal_scroll_reading_item(
                OWNER,
                self.item_version.as_ref().expect("item document version"),
                TARGET,
                tekes_ui_controls::ScrollRevealOptions {
                    placement,
                    margins: tekes_ui_controls::Insets::new(margin, margin, margin, margin),
                    animated,
                },
                &mut self.text,
            );
            self.item_error = result.err().map(|e| format!("{e:?}"));
        }
        fn update_dialog_actions(&mut self) {
            if self.mode == "menu-tree" {
                self.menu_actions.extend(self.tree.drain_action_records());
            }
            if self.mode == "item" {
                use tekes_ui_composition_consumer::item_navigation::{
                    ANIMATED, CENTERED, LATEST, NEAREST, OWNER,
                };
                for action in self.tree.drain_actions() {
                    if action.kind != tekes_ui_controls::composition::ContentActionKind::Activate {
                        continue;
                    }
                    match action.target {
                        NEAREST => self.navigate_item(
                            tekes_ui_controls::ScrollRevealPlacement::Nearest,
                            false,
                        ),
                        CENTERED => self.navigate_item(
                            tekes_ui_controls::ScrollRevealPlacement::Centered,
                            false,
                        ),
                        ANIMATED => self.navigate_item(
                            tekes_ui_controls::ScrollRevealPlacement::Centered,
                            true,
                        ),
                        LATEST => {
                            self.tree
                                .get_mut(OWNER)
                                .unwrap()
                                .component_mut::<tekes_ui_controls::composition::ScrollContent>()
                                .unwrap()
                                .follow_latest();
                        }
                        _ => {}
                    }
                }
            }

            if let Some(value) = &mut self.plain_value {
                tekes_ui_composition_consumer::plain_text::process_actions(&mut self.tree, value);
            }
            if let Some(reading) = &mut self.reading {
                reading.process_actions(&mut self.tree, self.text.started.elapsed());
                self.text.type_scale = if reading.large_text { 1.5 } else { 1. };
            }
            if self.mode == "dialog" {
                let dialog_open = self.dialog_is_open();
                let count = |id| {
                    self.tree
                        .get(id)
                        .and_then(|c| {
                            c.component::<tekes_ui_controls::composition::ButtonContent>()
                        })
                        .map_or(0, |b| b.activations)
                };
                let invoker = count(ids::DIALOG_INVOKER);
                let cancel = count(ids::DIALOG_CANCEL);
                let save = count(ids::DIALOG_SAVE);
                if !dialog_open && invoker > self.invoker_activations {
                    self.tree
                        .show_dialog(self.dialog)
                        .expect("open composed Dialog");
                } else if dialog_open
                    && (cancel > self.cancel_activations
                        || (save > self.save_activations
                            && scenarios::validate_dialog(&mut self.tree)))
                {
                    self.tree.close_dialog();
                }
                self.invoker_activations = invoker;
                self.cancel_activations = cancel;
                self.save_activations = save;
            }
        }
        fn copy_selection(&self, w: &Window) {
            if !w.is_active() {
                return;
            }
            if let Some(surface) = self.tree.focused_text()
                && !surface.input.disabled
                && surface.input.marked.is_none()
            {
                let selected = surface.input.selected_text();
                if !selected.is_empty() {
                    w.set_clipboard_text(&selected);
                }
            }
        }
        fn dispatch(&mut self, e: Event) -> bool {
            use tekes_ui_controls::composition::ContentEvent as C;
            let ev = match e {
                Event::Mouse(m) => crate::native_pointer::translate(m),
                Event::TextKey(report) => crate::native_key::translate(report),
                Event::Scroll(s) => C::Scroll {
                    position: Point::new(s.position.x as f32, s.position.y as f32),
                    delta: Point::new(-(s.delta.x as f32), -(s.delta.y as f32)),
                    precise: s.precise,
                    begin: matches!(s.phase, ScrollPhase::Began | ScrollPhase::MayBegin),
                    end: matches!(s.phase, ScrollPhase::Ended | ScrollPhase::Cancelled),
                    momentum: !matches!(s.momentum_phase, ScrollPhase::None),
                    time: s.timestamp,
                },
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
                Event::TextCommand(s) => C::TextCommand(s),
                Event::Key(k) => {
                    // This minimal host has no Edit menu. Dispatch standard
                    // editing shortcuts to the same public text command API.
                    if k.kind == KeyEventKind::Down && k.modifiers.command {
                        let command = match k.key_code {
                            0 => Some("selectAll:"),
                            6 if k.modifiers.shift => Some("redo:"),
                            6 => Some("undo:"),
                            _ => None,
                        };
                        if let Some(command) = command {
                            return self.dispatch(Event::TextCommand(command.into()));
                        }
                    }
                    C::KeyInput(crate::native_key::key(k))
                }
                Event::Focus(false) => C::Cancel,
                _ => return false,
            };
            if matches!(
                &ev,
                C::Commit { .. }
                    | C::Preedit { .. }
                    | C::Unmark
                    | C::TextCommand(_)
                    | C::Key { .. }
                    | C::PointerDown(_)
                    | C::PointerUp(_)
            ) {
                self.text.caret_on = true;
            }
            let response = self.tree.event(&ev);
            eprintln!(
                "composition-event mode={} event={:?} focused={:?} text={:?}",
                self.mode,
                ev,
                self.tree.focused(),
                self.tree.focused_text().map(|s| (
                    &s.input.text,
                    s.input.selection(),
                    s.input.marked.clone()
                ))
            );
            self.update_dialog_actions();
            let semantics = self.tree.semantics();
            let selected = |id| {
                semantics.iter().find(|node| node.id == id).map(|node| {
                    (
                        node.semantics.selected,
                        node.semantics.expanded,
                        node.semantics.value.clone(),
                    )
                })
            };
            eprintln!(
                "composition-state mode={} switch={:?} menu={:?} row_activations={} dialog.is_open={}",
                self.mode,
                selected(ids::LIST_SWITCH),
                selected(ids::LIST_MENU),
                self.row_activations.as_ref().map_or(0, |count| count.get()),
                self.dialog_is_open(),
            );
            let _ = response;
            true
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, e: Event, w: &mut Window) {
            // Focus(false) cancels current ownership, but delayed native input
            // must also be fenced so Tab cannot re-arm the inactive tree.
            if !w.is_active()
                && matches!(
                    e,
                    Event::Key(_)
                        | Event::TextKey(_)
                        | Event::Mouse(_)
                        | Event::Scroll(_)
                        | Event::TextCommit { .. }
                        | Event::TextPreedit { .. }
                        | Event::TextUnmark
                        | Event::TextCommand(_)
                        | Event::Accessibility { .. }
                        | Event::Command(_)
                        | Event::ModifiersChanged(_)
                )
            {
                self.tree
                    .event(&tekes_ui_controls::composition::ContentEvent::Cancel);
                self.tree.focus(None, false);
                self.publish(w);
                return;
            }
            if self.mode == "item"
                && let Event::Key(key) = &e
                && key.kind == KeyEventKind::Down
                && key.modifiers.command
                && matches!(key.key_code, 18..=23 | 26 | 28)
            {
                match key.key_code {
                    18 => {
                        self.navigate_item(tekes_ui_controls::ScrollRevealPlacement::Nearest, false)
                    }
                    19 => self
                        .navigate_item(tekes_ui_controls::ScrollRevealPlacement::Centered, false),
                    20 => {
                        self.navigate_item(tekes_ui_controls::ScrollRevealPlacement::Centered, true)
                    }
                    21 => self.text.reduced_motion = !self.text.reduced_motion,
                    23 => w.set_content_size(Size {
                        width: if w.size().width > 500. { 400. } else { 720. },
                        height: w.size().height,
                    }),
                    22 => self.text.type_scale = if self.text.type_scale == 1. { 1.5 } else { 1. },
                    26 => {
                        use tekes_ui_composition_consumer::item_navigation::{PANEL, Panel};
                        let panel = self
                            .tree
                            .get_mut(PANEL)
                            .unwrap()
                            .component_mut::<Panel>()
                            .unwrap();
                        panel.clipped = !panel.clipped;
                    }
                    28 => {
                        use tekes_ui_composition_consumer::item_navigation::{OWNER, PANEL, Panel};
                        self.tree
                            .get_mut(OWNER)
                            .unwrap()
                            .component_mut::<tekes_ui_controls::composition::ScrollContent>()
                            .unwrap()
                            .follow_latest();
                        self.navigate_item(
                            tekes_ui_controls::ScrollRevealPlacement::Centered,
                            true,
                        );
                        let panel = self
                            .tree
                            .get_mut(PANEL)
                            .unwrap()
                            .component_mut::<Panel>()
                            .unwrap();
                        panel.content_shift = if panel.content_shift == 0. {
                            tekes_ui_tokens::Spacing::Regular.value() * 3.
                        } else {
                            0.
                        };
                    }
                    _ => unreachable!(),
                }
                self.redraw(w);
                return;
            }
            // Review controls recover a rejected frame; all ordinary input stays fenced.
            if self.mode == "layout" {
                if let Event::Key(key) = &e
                    && key.kind == KeyEventKind::Down
                    && key.modifiers.command
                    && matches!(key.key_code, 18..=23)
                {
                    match key.key_code {
                        18 => {
                            self.text.services_enabled = true;
                            self.layout_collapsed = true;
                        }
                        19 => {
                            self.text.services_enabled = false;
                            self.layout_collapsed = false;
                        }
                        20 => {
                            self.text.services_enabled = true;
                            self.layout_collapsed = false;
                        }
                        21 => self.text.services_enabled = false,
                        23 => self.text.services_enabled = false,
                        22 => {
                            self.text.type_scale = if self.text.type_scale == 1. { 1.5 } else { 1. }
                        }
                        _ => unreachable!(),
                    }
                    self.redraw(w);
                    return;
                }
                // Finish is an acknowledgement: drain the old key chain even
                // when its frame failed. Only fresh input stays fenced.
                if self.layout_error.is_some()
                    && matches!(
                        e,
                        Event::Key(_)
                            | Event::TextKey(tekes_ui_platform_macos::TextKeyEvent::Started(_))
                            | Event::Mouse(_)
                            | Event::Scroll(_)
                            | Event::TextCommit { .. }
                            | Event::TextPreedit { .. }
                            | Event::TextUnmark
                            | Event::TextCommand(_)
                            | Event::Accessibility { .. }
                            | Event::Command(_)
                    )
                {
                    self.publish(w);
                    return;
                }
            }
            match e {
                Event::Key(key)
                    if self.mode == "label"
                        && key.kind == KeyEventKind::Down
                        && key.modifiers.command
                        && matches!(key.key_code, 18 | 19 | 20 | 21 | 23 | 29) =>
                {
                    match key.key_code {
                        18 => self.text.type_scale = 1.,
                        19 => self.text.type_scale = 1.5,
                        20 => self.text.type_scale = 2.,
                        code => {
                            use tekes_ui_controls::composition::{
                                ButtonContent, ComponentId, CompoundButton,
                            };
                            for id in [9902, 9903] {
                                let node = self.tree.get_mut(ComponentId(id)).unwrap();
                                if let Some(button) = node.component_mut::<ButtonContent>() {
                                    button.button.state.disabled = code == 21;
                                    button.button.state.loading = code == 23;
                                } else if let Some(compound) =
                                    node.component_mut::<CompoundButton>()
                                {
                                    compound.action.button.state.disabled = code == 21;
                                    compound.action.button.state.loading = code == 23;
                                }
                            }
                        }
                    }
                    self.redraw(w);
                }
                Event::Key(key)
                    if key.kind == KeyEventKind::Down
                        && key.modifiers.command
                        && key.key_code == 8 =>
                {
                    self.copy_selection(w);
                    self.publish(w);
                }
                Event::TextCommand(command) if command == "copy:" => {
                    self.copy_selection(w);
                    self.publish(w);
                }
                Event::Accessibility { target, action } => {
                    if !w.is_active() {
                        return;
                    }
                    use tekes_ui_controls::composition::{AccessibilityAction as A, ComponentId};
                    let action = match action {
                        // This example currently mounts no Tree content. Disclosure
                        // must never be reinterpreted as a generic Press.
                        tekes_ui_platform_macos::AccessibilityAction::SetExpanded(_) => return,
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
                    let response = self.tree.accessibility_action(
                        ComponentId(target.component),
                        target.part,
                        action.clone(),
                    );
                    self.update_dialog_actions();
                    eprintln!(
                        "native-accessibility target={target:?} action={action:?} response={response:?} focused={:?}",
                        self.tree.focused()
                    );
                    self.redraw(w);
                }
                Event::TextKey(report) => {
                    // Observe provenance only; callbacks keep their original once-only route.
                    eprintln!("native-text-key {report:?}");
                    if self.dispatch(Event::TextKey(report)) {
                        self.redraw(w);
                    }
                }
                Event::Focus(active) => {
                    if !active {
                        self.tree
                            .event(&tekes_ui_controls::composition::ContentEvent::Cancel);
                        self.tree.focus(None, false);
                    }
                    self.publish(w);
                    w.request_redraw();
                }
                Event::Created => {
                    assert!(w.set_text_key_events(true));
                    self.gpu = Some(unsafe {
                        Renderer::new_for_metal_layer(
                            w.metal_layer(),
                            w.drawable_size(),
                            w.scale() as f32,
                        )
                        .expect("Metal renderer")
                    });
                    if self.mode == "dialog" {
                        let _ = self.tree.focus(Some(self.invoker), false);
                    }
                    self.redraw(w);
                    if let Some(script) = self.selftest_script.take() {
                        tekes_ui_platform_macos::selftest::run_script(script);
                    }
                }
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => {
                    if let Some(r) = &mut self.gpu {
                        r.resize(drawable_size, scale as f32);
                    }
                    self.redraw(w)
                }
                Event::SystemAppearanceChanged {
                    dark,
                    increase_contrast,
                } => {
                    self.system_theme = Theme {
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
                    self.palette = Palette::from(self.system_theme);
                    self.redraw(w);
                }
                Event::SystemMotionChanged(reduced) => {
                    self.text.reduced_motion = std::env::var("TEKES_COMPOSITION_REDUCED_MOTION")
                        .ok()
                        .map_or(reduced, |value| value == "1");
                    self.redraw(w);
                }
                Event::CaretBlink => {
                    self.text.caret_on = !self.text.caret_on;
                    self.redraw(w);
                }
                Event::AnimationFrame(_) => self.redraw(w),
                Event::Redraw => self.redraw(w),
                Event::CloseRequested => w.close(),
                e => {
                    if self.dispatch(e) {
                        self.redraw(w)
                    }
                }
            }
        }
    }

    let mode = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("TEKESUI_COMPONENT_CASE").ok())
        .unwrap_or_else(|| "list".into());
    assert!(
        matches!(
            mode.as_str(),
            "list"
                | "dialog"
                | "slider"
                | "command"
                | "reading"
                | "plain"
                | "number"
                | "field"
                | "label"
                | "layout"
                | "item"
                | "menu-tree"
        ),
        "usage: native [list|dialog|slider|command|reading|plain|number|field|label|layout|item|menu-tree]"
    );
    let mut reading = None;
    let mut plain_value = None;
    let mut item_version = None;
    let (tree, invoker, row_activations) = if mode == "menu-tree" {
        (
            tekes_ui_composition_consumer::declared_menu::build().unwrap(),
            tekes_ui_composition_consumer::declared_menu::OPTIONS,
            None,
        )
    } else if mode == "item" {
        let fixture = tekes_ui_composition_consumer::item_navigation::Fixture::new().unwrap();
        item_version = Some(fixture.version);
        (
            fixture.host,
            tekes_ui_composition_consumer::item_navigation::NEAREST,
            None,
        )
    } else if mode == "layout" {
        let (tree, _, _) = tekes_ui_composition_consumer::layout_activation::fixture(false, false);
        (
            tree,
            tekes_ui_composition_consumer::layout_activation::FIELD,
            None,
        )
    } else if mode == "label" {
        (
            tekes_ui_composition_consumer::label_content::build().unwrap(),
            tekes_ui_composition_consumer::label_content::ACTION,
            None,
        )
    } else if mode == "field" {
        (
            tekes_ui_composition_consumer::field_metadata::build().unwrap(),
            tekes_ui_composition_consumer::field_metadata::NORMAL,
            None,
        )
    } else if mode == "number" {
        (
            tekes_ui_composition_consumer::number_field::build().unwrap(),
            tekes_ui_composition_consumer::number_field::DECREMENT,
            None,
        )
    } else if mode == "plain" {
        let scenario = tekes_ui_composition_consumer::plain_text::Scenario::new().unwrap();
        plain_value = Some(scenario.value);
        (
            scenario.host,
            tekes_ui_composition_consumer::plain_text::BODY,
            None,
        )
    } else if mode == "reading" {
        let (tree, state) = tekes_ui_composition_consumer::reading_stream::build().unwrap();
        reading = Some(state);
        (
            tree,
            tekes_ui_composition_consumer::reading_stream::APPEND,
            None,
        )
    } else if mode == "slider" {
        (
            tekes_ui_composition_consumer::slider_content::build().unwrap(),
            tekes_ui_composition_consumer::slider_content::RANGE,
            None,
        )
    } else if mode == "command" {
        let scenario = tekes_ui_composition_consumer::command_status::Scenario::new(vec![
            "Open file".into(),
            "Open settings".into(),
            "Close window".into(),
        ])
        .unwrap();
        (
            scenario.host,
            tekes_ui_composition_consumer::command_status::COMMAND,
            None,
        )
    } else if mode == "dialog" {
        let (tree, invoker) = scenarios::build_dialog();
        (tree, invoker, None)
    } else {
        let fixture =
            scenarios::heterogeneous_list().expect("independent heterogeneous List fixture");
        (
            fixture.host,
            ids::LIST_SWITCH,
            Some(fixture.row_activations),
        )
    };
    let _ = ids::DIALOG;
    let fonts = CoreTextFonts::new();
    let backend = TextBackend {
        shaper: CoreTextShaper::new(fonts.clone()),
        rasterizer: CoreTextRasterizer::new(fonts),
        scale: 2.,
        type_scale: 1.,
        atlas: GlyphAtlas::default(),
        started: std::time::Instant::now(),
        frame_requested: false,
        reduced_motion: false,
        window_active: true,
        caret_on: true,
        services_enabled: mode != "layout",
    };
    let layout_baseline = (mode == "layout").then(|| {
        tree.get(tekes_ui_composition_consumer::layout_activation::FIELD)
            .unwrap()
            .component::<tekes_ui_controls::composition::TextFieldContent>()
            .unwrap()
            .surface
            .input
            .external_baseline()
    });
    let mut host = Host {
        selftest_script: None,
        tree,
        menu_actions: Vec::new(),
        reading,
        reading_width: false,
        item_version,
        item_error: None,
        layout_collapsed: mode == "layout",
        layout_error: None,
        layout_baseline,
        plain_value,
        gpu: None,
        text: backend,
        scene: Scene::new(),
        palette: Palette::from(Theme::LIGHT),
        mode: mode.clone(),
        invoker,
        dialog: ids::DIALOG,
        invoker_activations: 0,
        cancel_activations: 0,
        save_activations: 0,
        system_theme: Theme::LIGHT,
        row_activations,
    };
    let app = App::new(AppConfig {
        name: Some("TekesUI Composition Consumer".into()),
        ..Default::default()
    });
    if std::env::var("TEKES_COMPOSITION_SELFTEST").as_deref() == Ok("ime") {
        use tekes_ui_platform_macos::selftest::Action;
        host.selftest_script = Some(vec![
            (300, Action::Click { x: 120., y: 56. }),
            (
                600,
                Action::TextRange {
                    text: "A😀B".into(),
                    replacement: Some(0..9),
                },
            ),
            (
                750,
                Action::ExpectTextState {
                    text: "A😀B".into(),
                    selection: 4..4,
                    marked: None,
                    query: 1..2,
                    actual: 1..3,
                },
            ),
            (
                900,
                Action::PreeditRange {
                    text: "拼音".into(),
                    selection: 1..1,
                    replacement: Some(1..3),
                },
            ),
            (
                1050,
                Action::ExpectTextState {
                    text: "A拼音B".into(),
                    selection: 2..2,
                    marked: Some(1..3),
                    query: 1..3,
                    actual: 1..3,
                },
            ),
            (1150, Action::TextCommand("insertTab:")),
            (
                1250,
                Action::ExpectTextState {
                    text: "A拼音B".into(),
                    selection: 2..2,
                    marked: Some(1..3),
                    query: 1..3,
                    actual: 1..3,
                },
            ),
            (
                1400,
                Action::TextRange {
                    text: "你😀".into(),
                    replacement: None,
                },
            ),
            (
                1550,
                Action::ExpectTextState {
                    text: "A你😀B".into(),
                    selection: 4..4,
                    marked: None,
                    query: 2..3,
                    actual: 2..4,
                },
            ),
            (1700, Action::TextCommand("undo:")),
            (
                1850,
                Action::ExpectTextState {
                    text: "A😀B".into(),
                    selection: 4..4,
                    marked: None,
                    query: 1..2,
                    actual: 1..3,
                },
            ),
            (
                2000,
                Action::PreeditRange {
                    text: "ni".into(),
                    selection: 2..2,
                    replacement: Some(1..3),
                },
            ),
            (2100, Action::TextCommand("cancelOperation:")),
            (
                2250,
                Action::ExpectTextState {
                    text: "A😀B".into(),
                    selection: 4..4,
                    marked: None,
                    query: 1..2,
                    actual: 1..3,
                },
            ),
            (2400, Action::TextCommand("cancelOperation:")),
            (2550, Action::ExpectNoText),
            (2700, Action::Key { key_code: 36 }),
            (
                2900,
                Action::PreeditRange {
                    text: "你".into(),
                    selection: 1..1,
                    replacement: Some(0..1),
                },
            ),
            (3050, Action::Unmark),
            (
                3200,
                Action::ExpectTextState {
                    text: "你😀B".into(),
                    selection: 1..1,
                    marked: None,
                    query: 1..2,
                    actual: 1..3,
                },
            ),
            (3450, Action::Close),
        ]);
    }
    if std::env::var("TEKES_COMPOSITION_SELFTEST").as_deref() == Ok("ax-text") {
        use tekes_ui_platform_macos::selftest::Action;
        let field = ids::DIALOG_FIELD.0;
        host.selftest_script = Some(vec![
            (300, Action::Click { x: 120., y: 56. }),
            (
                500,
                Action::TextRange {
                    text: "A😀B".into(),
                    replacement: Some(0..9),
                },
            ),
            (
                650,
                Action::AXSelect {
                    component: field,
                    range: 1..3,
                },
            ),
            (
                700,
                Action::ExpectTextRect {
                    range: 1..3,
                    actual: 1..3,
                    nonzero_width: true,
                },
            ),
            (
                750,
                Action::ExpectTextRect {
                    range: 3..3,
                    actual: 3..3,
                    nonzero_width: false,
                },
            ),
            (
                800,
                Action::ExpectAXText {
                    component: field,
                    text: "A😀B".into(),
                    selection: 1..3,
                    selected: "😀".into(),
                    query: 1..3,
                    substring: Some("😀".into()),
                },
            ),
            (
                900,
                Action::AXReplace {
                    component: field,
                    text: "你".into(),
                },
            ),
            (
                1050,
                Action::ExpectAXText {
                    component: field,
                    text: "A你B".into(),
                    selection: 2..2,
                    selected: "".into(),
                    query: 1..2,
                    substring: Some("你".into()),
                },
            ),
            (1150, Action::TextCommand("undo:")),
            (
                1300,
                Action::ExpectAXText {
                    component: field,
                    text: "A😀B".into(),
                    selection: 1..3,
                    selected: "😀".into(),
                    query: 1..2,
                    substring: None,
                },
            ),
            (
                1400,
                Action::PreeditRange {
                    text: "pin".into(),
                    selection: 3..3,
                    replacement: None,
                },
            ),
            (
                1500,
                Action::AXSelect {
                    component: field,
                    range: 0..0,
                },
            ),
            (
                1600,
                Action::AXReplace {
                    component: field,
                    text: "x".into(),
                },
            ),
            (
                1750,
                Action::ExpectTextState {
                    text: "ApinB".into(),
                    selection: 4..4,
                    marked: Some(1..4),
                    query: 1..4,
                    actual: 1..4,
                },
            ),
            (1850, Action::TextCommand("undo:")),
            (
                2000,
                Action::ExpectAXText {
                    component: field,
                    text: "A😀B".into(),
                    selection: 1..3,
                    selected: "😀".into(),
                    query: 1..3,
                    substring: Some("😀".into()),
                },
            ),
            (
                2100,
                Action::PreeditRange {
                    text: "pin".into(),
                    selection: 3..3,
                    replacement: None,
                },
            ),
            (
                2200,
                Action::PreeditRange {
                    text: "".into(),
                    selection: 0..0,
                    replacement: None,
                },
            ),
            (
                2350,
                Action::ExpectTextState {
                    text: "A😀B".into(),
                    selection: 1..3,
                    marked: None,
                    query: 1..3,
                    actual: 1..3,
                },
            ),
            (2450, Action::TextCommand("insertTab:")),
            (2550, Action::TextCommand("cancelOperation:")),
            (2700, Action::ExpectAXMissing(field)),
            (2850, Action::Close),
        ]);
    }
    app.run(
        WindowConfig {
            title: format!("TekesUI Composition Consumer · {mode}"),
            size: Size {
                width: 720.,
                height: 640.,
            },
            min_size: Some(Size {
                width: 360.,
                height: 480.,
            }),
            placement: tekes_ui_platform_macos::WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
            ..Default::default()
        },
        MenuBar::default(),
        host,
    );
}
