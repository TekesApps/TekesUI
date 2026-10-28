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
    use tekes_ui_controls::composition::Component;
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, TextSurfaceHost,
        TextSurfaceTextRenderer, composition::ContentHost,
    };
    use tekes_ui_platform_macos::{
        App, AppConfig, CommandId, Event, KeyEventKind, Menu, MenuBar, MenuCommand, MenuItem,
        ScrollPhase, Size, Window, WindowConfig, WindowHandler,
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
        fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
            role.style().size * self.type_scale
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
        nested_scroll_events: usize,
        help_removed: Option<tekes_ui_controls::composition::Content>,
        menu_actions: Vec<tekes_ui_controls::composition::ContentActionRecord>,
        reading: Option<tekes_ui_composition_consumer::reading_stream::StreamState>,
        nested_reading: Option<tekes_ui_composition_consumer::nested_reading::State>,
        reading_width: bool,
        item_version: Option<tekes_ui_controls::ReadingDocumentVersion>,
        item_error: Option<String>,
        layout_collapsed: bool,
        layout_error: Option<String>,
        layout_baseline: Option<tekes_ui_controls::TextEditBaseline>,
        plain_value: Option<tekes_ui_controls::DraftRevision>,
        split_icon: Option<(u32, tekes_ui_tokens::Rgba, tekes_ui_scene::ImageId)>,
        gpu: Option<Renderer>,
        text: TextBackend,
        scene: Scene,
        palette: Palette,
        mode: String,
        invoker: tekes_ui_controls::composition::ComponentId,
        dialog: tekes_ui_controls::composition::ComponentId,
        toolbar_commands: Vec<u64>,
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
        fn publish(&self, w: &Window, rendered: bool) {
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
                if node.semantics.role == R::Slider
                    && let Some(domain) = self.tree.value_range(node.id)
                {
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
                    "pid": std::process::id(), "mode": self.mode, "window_active": w.is_active(),
                    "window_size": {"width":w.size().width,"height":w.size().height},
                    "screen": screen.map(|s| serde_json::json!({"name":s.name,"builtin":s.builtin,"scale":s.scale,"visible_frame":{"x":s.visible_frame.origin.x,"y":s.visible_frame.origin.y,"width":s.visible_frame.size.width,"height":s.visible_frame.size.height}})),
                    "nodes": nodes.iter().map(|n| serde_json::json!({
                        "component":n.key.component,"part":n.key.part,"role":format!("{:?}",n.role),
                        "label":n.label,
                        "value":n.value,
                        "description":(self.mode == "content-help").then(||n.description.clone()).flatten(),
                        "required":n.required,"enabled":n.enabled,"focused":n.focused,"selected":n.selected,"expanded":n.expanded,
                        "parent":n.parent.map(|key|serde_json::json!({"component":key.component,"part":key.part})),
                        "frame":{"x":n.frame.origin.x,"y":n.frame.origin.y,"width":n.frame.size.width,"height":n.frame.size.height}
                    })).collect::<Vec<_>>()
                });
                if self.mode == "content-help" {
                    use tekes_ui_composition_consumer::help_list::FIRST;
                    let first = self.tree.get(FIRST);
                    let activations = first.and_then(|node| {
                        node.component::<tekes_ui_controls::composition::Button>()
                            .map(|button| button.activations)
                    });
                    let help = self.tree.help_tooltip();
                    let scroll = self
                        .tree
                        .get(tekes_ui_composition_consumer::help_list::SCROLL)
                        .and_then(|node| {
                            node.component::<tekes_ui_controls::composition::ScrollContent>()
                        })
                        .map(|content| content.scroll.offset.y);
                    let visible_rows = nodes
                        .iter()
                        .filter(|node| {
                            node.key.part.is_none()
                                && (FIRST.0..FIRST.0 + 8).contains(&node.key.component)
                        })
                        .map(|node| {
                            serde_json::json!({
                                "id": node.key.component,
                                "y": node.frame.origin.y,
                                "height": node.frame.size.height,
                                "description": node.description
                            })
                        })
                        .collect::<Vec<_>>();
                    snapshot["content_help"] = serde_json::json!({
                        "owner": help.as_ref().map(|(id, _, _)| id.0),
                        "text": help.as_ref().map(|(_, text, _)| *text),
                        "first_description": nodes
                            .iter()
                            .find(|node| node.key.component == FIRST.0 && node.key.part.is_none())
                            .and_then(|node| node.description.clone()),
                        "first_mounted": first.is_some(),
                        "first_activations": activations,
                        "scroll_offset_y": scroll,
                        "row_frames": visible_rows,
                        "instructions": "Hover the first result and wait for help. Use Help Fixture > Set help query B, Revoke help, or Remove or remount first row. Remount stays unbound until Set help query B."
                    });
                }
                if let Some(state) = &self.nested_reading {
                    use tekes_ui_composition_consumer::nested_reading::TAIL;
                    use tekes_ui_composition_consumer::nested_reading::{COMPOSER, INNER, PARENT};
                    use tekes_ui_controls::composition::{ScrollContent, TextFieldContent};
                    let parent_content = self
                        .tree
                        .get(PARENT)
                        .unwrap()
                        .component::<ScrollContent>()
                        .unwrap();
                    let inner_content = self
                        .tree
                        .get(INNER)
                        .unwrap()
                        .component::<TextFieldContent>()
                        .unwrap();
                    let composer_content = self.tree.get(COMPOSER).unwrap();
                    let composer = &composer_content
                        .component::<TextFieldContent>()
                        .unwrap()
                        .surface;
                    let rect = |r: Rect| {
                        serde_json::json!({
                            "x": r.min_x(), "y": r.min_y(),
                            "width": r.size.width, "height": r.size.height
                        })
                    };
                    let detail = self.tree.get(INNER).unwrap();
                    let tail = self.tree.get(TAIL).unwrap();
                    snapshot["nested_reading"] = serde_json::json!({
                        "rendered": rendered,
                        "scroll_events": self.nested_scroll_events,
                        "following": parent_content.is_following_latest(),
                        "parent_offset": parent_content.scroll.offset.y,
                        "inner_offset": inner_content.surface.scroll.offset.y,
                        "inner_following": inner_content.reading.as_ref().map(|r| r.is_following_latest()),
                        "inner_visible": detail.visible().is_some(),
                        "inner_frame": rect(detail.frame()),
                        "inner_visible_frame": detail.visible().map(rect),
                        "inner_mount": detail.mount_identity(),
                        "parent_frame": rect(self.tree.get(PARENT).unwrap().frame()),
                        "parent_visible_frame": self.tree.get(PARENT).unwrap().visible().map(rect),
                        "tail_visible_frame": tail.visible().map(rect),
                        "tail_mount": tail.mount_identity(),
                        "appended": state.appended,
                        "version": state.version.revision(),
                        "focus": self.tree.focused().map(|id| id.0),
                        "composer_mount": composer_content.mount_identity(),
                        "composer": composer.input.text,
                        "marked": composer.input.marked,
                        "selection": composer.input.selection(),
                        "baseline": format!("{:?}", composer.input.external_baseline()),
                    });
                    if rendered
                        && let Some(directory) = std::env::var_os("TEKESUI_COMPONENT_EVIDENCE")
                    {
                        use std::io::Write;
                        let mut journal = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(
                                std::path::PathBuf::from(directory)
                                    .join("nested-reading-journal.jsonl"),
                            )
                            .unwrap();
                        writeln!(journal, "{}", snapshot).unwrap();
                    }
                }
                if self.mode == "editable-number" {
                    use tekes_ui_composition_consumer::editable_number::PERCENT;
                    let field = self
                        .tree
                        .get(PERCENT)
                        .unwrap()
                        .component::<tekes_ui_controls::composition::NumberFieldContent>()
                        .unwrap();
                    let state = field.edit_state();
                    snapshot["number"] = serde_json::json!({"committed":state.committed_value,"draft":state.text,"composing":state.composing,"dirty":state.dirty,"validation":format!("{:?}",state.validation)});
                }
                if self.mode == "toolbar" {
                    snapshot["toolbar_typography"] = serde_json::json!({"title_size":proposal::TextRole::UiBody.style().size*self.text.type_scale,"toolbar_frame":self.tree.get(tekes_ui_composition_consumer::toolbar_semantics::TOOLBAR).unwrap().frame().size.height,"native_center":w.titlebar_control_center_y(),"safe_area":w.titlebar_safe_area(),"native_overlap":w.titlebar_content_height()});
                    snapshot["toolbar_commands"] = serde_json::json!(self.toolbar_commands);
                    snapshot["toolbar_action_count"] = serde_json::json!(self.menu_actions.len());
                    snapshot["draft"] = serde_json::json!(
                        self.tree
                            .get(tekes_ui_composition_consumer::toolbar_semantics::EDITOR)
                            .unwrap()
                            .component::<tekes_ui_controls::composition::TextFieldContent>()
                            .unwrap()
                            .surface
                            .input
                            .text
                    );
                }
                if matches!(
                    self.mode.as_str(),
                    "menu-tree" | "toolbar" | "toolbar-groups" | "toolbar-split"
                ) {
                    snapshot["menu_actions"]=serde_json::json!(self.menu_actions.iter().map(|record|{
                        let (selection, toolbar_command) = match record.payload.as_ref() {
                            Some(tekes_ui_controls::composition::ContentActionPayload::MenuSelection(value)) => (Some(serde_json::json!({"index":value.index(),"label":value.label(),"model_revision":value.model_revision(),"value_path":value.value_path()})), None),
                            Some(tekes_ui_controls::composition::ContentActionPayload::ToolbarCommand(command)) => (None, Some(*command)),
                            None => (None, None),
                        };
                        serde_json::json!({"target":record.action.target.0,"source":format!("{:?}",record.action.source),"kind":format!("{:?}",record.action.kind),"mount":record.mount,"selection":selection,"toolbar_command":toolbar_command})
                    }).collect::<Vec<_>>());
                }
                if self.mode == "toolbar-groups" {
                    use tekes_ui_composition_consumer::toolbar_groups::{
                        ACTION_GROUP, BAR, NAV, STATUS_GROUP,
                    };
                    snapshot["toolbar_typography"] = serde_json::json!({
                        "title_size": proposal::TextRole::UiBody.style().size * self.text.type_scale,
                        "toolbar_frame": self.tree.get(BAR).unwrap().frame().size.height,
                        "native_center": w.titlebar_control_center_y(),
                        "safe_area": w.titlebar_safe_area(),
                        "native_overlap": w.titlebar_content_height(),
                        "groups": ([NAV, STATUS_GROUP, ACTION_GROUP].map(|id| {
                            let frame = self.tree.get(id).unwrap().frame();
                            serde_json::json!({"id": id.0, "x":frame.min_x(), "y":frame.min_y(), "width":frame.size.width, "height":frame.size.height})
                        }))
                    });
                }
                if self.mode == "toolbar-split" {
                    use tekes_ui_composition_consumer::split_toolbar::{
                        LEFT_BAR, NAV, RIGHT_BAR, ROOT, SplitToolbar,
                    };
                    let f = self
                        .tree
                        .get(ROOT)
                        .unwrap()
                        .component::<SplitToolbar>()
                        .unwrap();
                    let rect = |r: Rect| [r.min_x(), r.min_y(), r.size.width, r.size.height];
                    snapshot["split_toolbar"] = serde_json::json!({
                        "title_size": proposal::TextRole::UiBody.style().size * self.text.type_scale,
                        "row":rect(f.row), "native_center": w.titlebar_control_center_y(),
                        "divider":rect(f.view.layout().divider.frame()), "ratio": f.model.ratio(),
                        "held": f.held, "minimum":f.model.min_pane,
                        "marker": self.scene.primitives().iter().find_map(|p| {
                            if let tekes_ui_scene::Primitive::Quad(q) = p
                                && q.rect.size.width <= 1. && q.corner_radii.top_left > 0.
                                && q.rect.min_y() >= f.row.min_y() && q.rect.max_y() <= f.row.max_y()
                            { return Some(rect(q.rect)); }
                            None
                        }),
                        "image": self.scene.primitives().iter().find_map(|p| {
                            if let tekes_ui_scene::Primitive::Image(i) = p { Some(rect(i.rect)) } else { None }
                        }),
                        "image_identity": self.split_icon.map(|(_, _, id)| format!("{id:?}")),
                        "left_bar":rect(self.tree.get(LEFT_BAR).unwrap().frame()),
                        "right_bar":rect(self.tree.get(RIGHT_BAR).unwrap().frame()),
                        "navigation": self.tree.get(NAV).unwrap().component::<tekes_ui_controls::ToolbarControl>().unwrap()
                            .pointer_regions(self.tree.get(NAV).unwrap().frame(), self.tree.get(NAV).unwrap().visible().unwrap_or_default()).into_iter().map(rect).collect::<Vec<_>>()
                    });
                    snapshot["rendered"] = serde_json::json!(rendered);
                    if rendered {
                        use std::io::Write;
                        let mut journal = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(directory.join("split-journal.jsonl"))
                            .unwrap();
                        writeln!(journal, "{}", snapshot).unwrap();
                    }
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
                    use tekes_ui_controls::composition::{Button, ComponentId};
                    snapshot["label_review"] = serde_json::json!({
                        "type_scale": self.text.type_scale, "width": w.size().width,
                        "buttons": ([9902,9903].into_iter().map(|id| {
                            let node=self.tree.get(ComponentId(id)).unwrap();
                            let button=node.component::<Button>().unwrap();
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
                if self.mode == "content-help" {
                    use std::io::Write;
                    let mut history = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(directory.join("content-help-history.jsonl"))
                        .expect("content-help native history");
                    writeln!(history, "{}", snapshot).expect("content-help snapshot record");
                }
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
            if self.mode == "toolbar" {
                self.tree
                    .get_mut(tekes_ui_composition_consumer::toolbar_semantics::TOOLBAR)
                    .unwrap()
                    .component_mut::<tekes_ui_controls::ToolbarControl>()
                    .unwrap()
                    .set_viewport(Rect::new(
                        0.,
                        0.,
                        w.size().width as f32,
                        w.size().height as f32,
                    ));
            }
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
            if self.mode == "toolbar-split" {
                use tekes_ui_composition_consumer::split_toolbar::{NAV, ROOT, SplitToolbar};
                let row = tekes_ui_controls::ToolbarControl::window().row_height(&mut self.text);
                let center = w.set_toolbar_row_height(f64::from(row));
                let leading = w.titlebar_safe_area().1;
                let fixture = self
                    .tree
                    .get_mut(ROOT)
                    .unwrap()
                    .component_mut::<SplitToolbar>()
                    .unwrap();
                fixture.native_center = center.map(|y| y as f32);
                fixture.native_leading = leading as f32;
                fixture
                    .view
                    .set_window_active(w.is_active(), &mut fixture.model);
                let side = (proposal::metric("size", "size-icon-standard").unwrap()
                    * self.text.type_scale
                    * w.scale() as f32)
                    .ceil()
                    .max(1.) as u32;
                let ink = self.palette.color("primary-text").unwrap();
                if self
                    .split_icon
                    .is_none_or(|(cached, color, _)| cached != side || color != ink)
                {
                    let glyph = tekes_ui_icons::Glyph::ALL
                        .iter()
                        .find(|g| g.name == "tekes-back")
                        .unwrap();
                    let channel = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
                    let options = resvg::usvg::Options {
                        style_sheet: Some(format!(
                            "svg {{ color: rgb({}, {}, {}); }}",
                            channel(ink.r),
                            channel(ink.g),
                            channel(ink.b)
                        )),
                        ..Default::default()
                    };
                    let tree = resvg::usvg::Tree::from_str(glyph.svg, &options).unwrap();
                    let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
                    let scale = side as f32 / tree.size().width();
                    resvg::render(
                        &tree,
                        resvg::tiny_skia::Transform::from_scale(scale, scale),
                        &mut pixels.as_mut(),
                    );
                    let renderer = self.gpu.as_mut().unwrap();
                    let image = renderer.register_image(pixels.data(), side, side).unwrap();
                    self.tree
                        .get_mut(NAV)
                        .unwrap()
                        .component_mut::<tekes_ui_controls::ToolbarControl>()
                        .unwrap()
                        .set_icon_actions(vec![tekes_ui_controls::ToolbarIconAction {
                            id: 1,
                            image,
                            label: "Right container navigation".into(),
                            enabled: true,
                        }]);
                    if let Some((_, _, old)) = self.split_icon.replace((side, ink, image)) {
                        renderer.unregister_image(old);
                    }
                }
            }
            let clip = Rect::new(0., 0., w.size().width as f32, w.size().height as f32);
            let mut frame = clip.inset(tekes_ui_tokens::Spacing::Regular.value());
            if matches!(
                self.mode.as_str(),
                "toolbar" | "toolbar-groups" | "toolbar-split"
            ) {
                let leading = w.titlebar_safe_area().1 as f32;
                frame = Rect::new(
                    leading,
                    0.,
                    (clip.size.width - leading).max(0.),
                    clip.size.height,
                );
            }
            if self.mode == "toolbar-groups" {
                frame.size.height = self
                    .tree
                    .get_mut(tekes_ui_composition_consumer::toolbar_groups::BAR)
                    .unwrap()
                    .measure(&mut self.text, frame.size)
                    .size
                    .height;
            }
            if matches!(
                self.mode.as_str(),
                "toolbar" | "toolbar-groups" | "toolbar-split"
            ) {
                let row = tekes_ui_controls::ToolbarControl::window().row_height(&mut self.text);
                if let Some(center) = w.set_toolbar_row_height(f64::from(row)) {
                    frame.origin.y = center as f32 - row * 0.5;
                }
            }
            if self.mode == "toolbar-split" {
                frame = clip;
            }
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
            if self.mode == "toolbar-split" {
                use tekes_ui_composition_consumer::split_toolbar::{ROOT, SplitToolbar};
                let fixture = self
                    .tree
                    .get(ROOT)
                    .unwrap()
                    .component::<SplitToolbar>()
                    .unwrap();
                w.set_cursor_regions(
                    fixture
                        .view
                        .resize_regions(&fixture.model)
                        .into_iter()
                        .map(|(rect, _)| tekes_ui_platform_macos::CursorRegion {
                            origin: tekes_ui_platform_macos::Point {
                                x: rect.min_x() as f64,
                                y: rect.min_y() as f64,
                            },
                            size: Size {
                                width: rect.size.width as f64,
                                height: rect.size.height as f64,
                            },
                            icon: tekes_ui_platform_macos::CursorIcon::ResizeHorizontal,
                        })
                        .collect(),
                );
            }
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
            self.publish(w, true);
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
            if self.mode == "toolbar" {
                self.menu_actions.extend(self.tree.drain_action_records());
                if let Some(command) = self
                    .tree
                    .get_mut(tekes_ui_composition_consumer::toolbar_semantics::TOOLBAR)
                    .unwrap()
                    .component_mut::<tekes_ui_controls::ToolbarControl>()
                    .unwrap()
                    .take_action()
                {
                    self.toolbar_commands.push(command);
                }
            }
            if matches!(
                self.mode.as_str(),
                "menu-tree" | "toolbar-groups" | "toolbar-split"
            ) {
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
            if let Some(state) = &mut self.nested_reading {
                state.process_actions(&mut self.tree);
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
                        .and_then(|c| c.component::<tekes_ui_controls::composition::Button>())
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
        // Host clipboard access stays outside the reusable editor. Actual edits
        // go through the same ContentHost/TextInput transactions as typed text.
        fn clipboard_command(&mut self, w: &Window, command: &str) {
            if !w.is_active() {
                return;
            }
            let Some(surface) = self.tree.focused_text() else {
                return;
            };
            if surface.input.disabled || surface.input.marked.is_some() {
                return;
            }
            if command == "copy:" {
                self.copy_selection(w);
                return;
            }
            if surface.input.read_only {
                return;
            }
            let replacement = surface.input.selection();
            match command {
                "cut:" if !replacement.is_empty() => {
                    self.copy_selection(w);
                    self.dispatch(Event::TextCommit {
                        text: String::new(),
                        replacement: Some(replacement),
                    });
                }
                "paste:" => {
                    if let Some(text) = w.clipboard_text() {
                        self.dispatch(Event::TextCommit {
                            text,
                            replacement: None,
                        });
                    }
                }
                _ => {}
            }
        }
        fn dispatch(&mut self, e: Event) -> bool {
            use tekes_ui_controls::composition::ContentEvent as C;
            if let Event::Scroll(scroll) = &e
                && self.mode == "nested-reading"
            {
                self.nested_scroll_events += 1;
                eprintln!(
                    "nested-reading-native-scroll count={} position={:?} delta={:?} phase={:?} momentum={:?}",
                    self.nested_scroll_events,
                    scroll.position,
                    scroll.delta,
                    scroll.phase,
                    scroll.momentum_phase
                );
            }
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
            let nested_scroll_before =
                (self.mode == "nested-reading" && matches!(&ev, C::Scroll { .. })).then(|| {
                    use tekes_ui_composition_consumer::nested_reading::{INNER, PARENT};
                    use tekes_ui_controls::composition::{ScrollContent, TextFieldContent};
                    let parent = self
                        .tree
                        .get(PARENT)
                        .unwrap()
                        .component::<ScrollContent>()
                        .unwrap();
                    let inner = self
                        .tree
                        .get(INNER)
                        .unwrap()
                        .component::<TextFieldContent>()
                        .unwrap();
                    (
                        parent.scroll.offset.y,
                        inner.surface.scroll.offset.y,
                        parent.is_following_latest(),
                        inner
                            .reading
                            .as_ref()
                            .is_some_and(|reading| reading.is_following_latest()),
                    )
                });
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
            if let Some((parent_before, inner_before, following_before, inner_following_before)) =
                nested_scroll_before
            {
                use tekes_ui_composition_consumer::nested_reading::{INNER, PARENT};
                use tekes_ui_controls::composition::{ScrollContent, TextFieldContent};
                let parent = self
                    .tree
                    .get(PARENT)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap();
                let inner = self
                    .tree
                    .get(INNER)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap();
                eprintln!(
                    "nested-reading-after-scroll event={} parent_offset={}->{} inner_offset={}->{} following={}->{} inner_following={}->{}",
                    self.nested_scroll_events,
                    parent_before,
                    parent.scroll.offset.y,
                    inner_before,
                    inner.surface.scroll.offset.y,
                    following_before,
                    parent.is_following_latest(),
                    inner_following_before,
                    inner
                        .reading
                        .as_ref()
                        .is_some_and(|reading| reading.is_following_latest()),
                );
            }
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
                self.publish(w, false);
                return;
            }
            if self.mode == "content-help"
                && let Event::Command(CommandId(command)) = &e
                && matches!(*command, 35_001..=35_003)
            {
                use tekes_ui_composition_consumer::help_list::{FIRST, LIST};
                match *command {
                    35_001 => {
                        if let Some(row) = self.tree.get_mut(FIRST) {
                            let help = Some("query B · 本次搜索摘要：组合输入、焦点与阅读位置应保持不变；结果覆盖 multilingual résumé, 東京, 搜索😀与/folder/1 中的原始条目。".to_owned());
                            row.set_help(help.clone());
                            eprintln!("content-help-query owner={} help={help:?}", FIRST.0);
                        } else {
                            eprintln!("content-help-query owner={} result=unmounted", FIRST.0);
                        }
                    }
                    35_002 => {
                        if let Some(row) = self.tree.get_mut(FIRST) {
                            row.set_help(None);
                            eprintln!("content-help-revoke owner={}", FIRST.0);
                        } else {
                            eprintln!("content-help-revoke owner={} result=unmounted", FIRST.0);
                        }
                    }
                    35_003 => {
                        if self.tree.get(FIRST).is_some() {
                            self.help_removed =
                                Some(self.tree.remove(FIRST).expect("remove original help row"));
                            eprintln!("content-help-row owner={} mounted=false", FIRST.0);
                        } else if let Some(row) = self.help_removed.take() {
                            self.tree
                                .insert(LIST, row)
                                .expect("remount original help row");
                            let order = (0..8)
                                .map(|index| {
                                    tekes_ui_controls::composition::ComponentId(FIRST.0 + index)
                                })
                                .collect::<Vec<_>>();
                            self.tree
                                .reorder(LIST, &order)
                                .expect("restore original List order");
                            eprintln!(
                                "content-help-row owner={} mounted=true help_requires_rebind=true",
                                FIRST.0
                            );
                        }
                    }
                    _ => unreachable!(),
                }
                self.redraw(w);
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
                    self.publish(w, false);
                    return;
                }
            }
            match e {
                Event::Key(key)
                    if matches!(
                        self.mode.as_str(),
                        "toolbar" | "toolbar-groups" | "toolbar-split"
                    ) && key.kind == KeyEventKind::Down
                        && key.modifiers.command
                        && matches!(key.key_code, 18 | 19 | 24 | 27 | 29) =>
                {
                    let baseline = proposal::TextRole::UiBody.style().size;
                    let size = baseline * self.text.type_scale;
                    self.text.type_scale = match key.key_code {
                        18 | 29 => 1.,
                        19 => 20. / baseline,
                        24 => (size + 1.) / baseline,
                        _ => (size - 1.).max(1.) / baseline,
                    };
                    self.redraw(w);
                }
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
                            use tekes_ui_controls::composition::{Button, ComponentId};
                            for id in [9902, 9903] {
                                let node = self.tree.get_mut(ComponentId(id)).unwrap();
                                if let Some(button) = node.component_mut::<Button>() {
                                    button.button.state.disabled = code == 21;
                                    button.button.state.loading = code == 23;
                                }
                            }
                        }
                    }
                    self.redraw(w);
                }
                Event::Key(key)
                    if key.kind == KeyEventKind::Down
                        && key.modifiers.command
                        && matches!(key.key_code, 7..=9) =>
                {
                    let command = match key.key_code {
                        7 => "cut:",
                        8 => "copy:",
                        _ => "paste:",
                    };
                    self.clipboard_command(w, command);
                    self.redraw(w);
                }
                Event::TextCommand(command)
                    if matches!(command.as_str(), "copy:" | "cut:" | "paste:") =>
                {
                    self.clipboard_command(w, &command);
                    self.redraw(w);
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
                    self.publish(w, false);
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
                    if self.mode == "content-help"
                        && std::env::var("TEKES_COMPOSITION_SELFTEST").as_deref()
                            == Ok("content-help")
                    {
                        use tekes_ui_composition_consumer::help_list::{COMPOSER, FIRST};
                        use tekes_ui_platform_macos::selftest::Action;
                        let row = self.tree.get(FIRST).unwrap().visible().unwrap().center();
                        let composer = self.tree.get(COMPOSER).unwrap().visible().unwrap().center();
                        let text = "Keep A😀B拼音";
                        self.selftest_script = Some(vec![
                            (300, Action::WindowDark(false)),
                            (
                                400,
                                Action::Click {
                                    x: f64::from(composer.x),
                                    y: f64::from(composer.y),
                                },
                            ),
                            (
                                520,
                                Action::TextRange {
                                    text: "Keep A😀B".into(),
                                    replacement: Some(0..9),
                                },
                            ),
                            (
                                650,
                                Action::PreeditRange {
                                    text: "拼音".into(),
                                    selection: 2..2,
                                    replacement: None,
                                },
                            ),
                            (
                                780,
                                Action::ExpectTextState {
                                    text: text.into(),
                                    selection: 11..11,
                                    marked: Some(9..11),
                                    query: 9..11,
                                    actual: 9..11,
                                },
                            ),
                            (
                                900,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (
                                1_550,
                                Action::ExpectTextState {
                                    text: text.into(),
                                    selection: 11..11,
                                    marked: Some(9..11),
                                    query: 9..11,
                                    actual: 9..11,
                                },
                            ),
                            (
                                1_600,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE")
                                            .expect("evidence directory"),
                                    )
                                    .join("help-A-light.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (1_800, Action::MenuCommand(35_001)),
                            (1_900, Action::MouseExit),
                            (
                                2_000,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (
                                2_650,
                                Action::ExpectTextState {
                                    text: text.into(),
                                    selection: 11..11,
                                    marked: Some(9..11),
                                    query: 9..11,
                                    actual: 9..11,
                                },
                            ),
                            (
                                2_700,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap(),
                                    )
                                    .join("help-B-light.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (2_900, Action::MenuCommand(35_002)),
                            (
                                3_000,
                                Action::ExpectTextState {
                                    text: text.into(),
                                    selection: 11..11,
                                    marked: Some(9..11),
                                    query: 9..11,
                                    actual: 9..11,
                                },
                            ),
                            (3_100, Action::MenuCommand(35_001)),
                            (3_200, Action::MouseExit),
                            (
                                3_300,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (
                                3_950,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap(),
                                    )
                                    .join("help-long-light.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (
                                4_100,
                                Action::ScrollWheel {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                    delta_x: 0,
                                    delta_y: -1,
                                },
                            ),
                            (
                                4_300,
                                Action::ExpectTextState {
                                    text: text.into(),
                                    selection: 11..11,
                                    marked: Some(9..11),
                                    query: 9..11,
                                    actual: 9..11,
                                },
                            ),
                            (
                                4_400,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap(),
                                    )
                                    .join("help-scrolled.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (
                                4_500,
                                Action::ScrollWheel {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                    delta_x: 0,
                                    delta_y: -20,
                                },
                            ),
                            (
                                4_700,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap(),
                                    )
                                    .join("help-clipped.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (
                                4_800,
                                Action::ScrollWheel {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                    delta_x: 0,
                                    delta_y: 21,
                                },
                            ),
                            (5_000, Action::Unmark),
                            (
                                5_100,
                                Action::ExpectText {
                                    text: "Keep A😀B拼音".into(),
                                    marked: false,
                                },
                            ),
                            (
                                5_200,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (
                                5_850,
                                Action::Click {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (6_050, Action::Key { key_code: 36 }),
                            (6_250, Action::AXPress { component: FIRST.0 }),
                            (6_500, Action::MenuCommand(35_003)),
                            (6_800, Action::MenuCommand(35_003)),
                            (7_000, Action::MenuCommand(35_001)),
                            (7_100, Action::MouseExit),
                            (
                                7_200,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (7_850, Action::WindowDark(true)),
                            (7_950, Action::MouseExit),
                            (
                                8_050,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (
                                8_700,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap(),
                                    )
                                    .join("help-long-dark.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (9_000, Action::Miniaturize),
                            (9_800, Action::Deminiaturize),
                            (10_500, Action::MouseExit),
                            (
                                10_600,
                                Action::MouseMove {
                                    x: f64::from(row.x),
                                    y: f64::from(row.y),
                                },
                            ),
                            (
                                11_250,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE").unwrap(),
                                    )
                                    .join("help-reactivated-dark.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (11_500, Action::Close),
                        ]);
                    }
                    if self.mode == "nested-reading"
                        && std::env::var("TEKES_COMPOSITION_SELFTEST").as_deref()
                            == Ok("nested-reading")
                    {
                        use tekes_ui_composition_consumer::nested_reading::{
                            APPEND, COMPOSER, INNER, LATEST,
                        };
                        use tekes_ui_platform_macos::selftest::Action;
                        let center = |id| {
                            self.tree
                                .get(id)
                                .and_then(|content| content.visible())
                                .expect("nested-reading rendered visible target")
                                .center()
                        };
                        let composer = center(COMPOSER);
                        let detail = center(INNER);
                        let append = center(APPEND);
                        let latest = center(LATEST);
                        let expect_composer = || Action::ExpectTextState {
                            text: "draft拼音".into(),
                            selection: 7..7,
                            marked: Some(5..7),
                            query: 5..7,
                            actual: 5..7,
                        };
                        self.selftest_script = Some(vec![
                            (250, Action::Log("nested-reading-baseline-frame".into())),
                            (
                                300,
                                Action::Click {
                                    x: f64::from(composer.x),
                                    y: f64::from(composer.y),
                                },
                            ),
                            (
                                550,
                                Action::PreeditRange {
                                    text: "拼音".into(),
                                    selection: 2..2,
                                    replacement: None,
                                },
                            ),
                            (700, expect_composer()),
                            (
                                800,
                                Action::Log("nested-reading-before-inner-scroll".into()),
                            ),
                            (
                                850,
                                Action::ScrollWheel {
                                    x: f64::from(detail.x),
                                    y: f64::from(detail.y),
                                    delta_x: 0,
                                    delta_y: -16,
                                },
                            ),
                            (
                                950,
                                Action::ScrollWheel {
                                    x: f64::from(detail.x),
                                    y: f64::from(detail.y),
                                    delta_x: 0,
                                    delta_y: 6,
                                },
                            ),
                            (1_050, expect_composer()),
                            (
                                1_100,
                                Action::Log("nested-reading-after-inner-scroll".into()),
                            ),
                            (
                                1_200,
                                Action::Click {
                                    x: f64::from(append.x),
                                    y: f64::from(append.y),
                                },
                            ),
                            (1_400, expect_composer()),
                            (1_500, Action::Log("nested-reading-after-append".into())),
                            (1_600, Action::Resize { w: 320., h: 640. }),
                            (1_850, expect_composer()),
                            (2_100, Action::Resize { w: 420., h: 640. }),
                            (2_350, expect_composer()),
                            (2_600, Action::Resize { w: 600., h: 640. }),
                            (2_850, expect_composer()),
                            (3_100, Action::Resize { w: 720., h: 640. }),
                            (
                                3_250,
                                Action::Log("nested-reading-returned-to-initial-size".into()),
                            ),
                            (
                                3_350,
                                Action::Click {
                                    x: f64::from(latest.x),
                                    y: f64::from(latest.y),
                                },
                            ),
                            (3_550, expect_composer()),
                            (
                                3_700,
                                Action::Capture {
                                    path: std::path::PathBuf::from(
                                        std::env::var_os("TEKESUI_COMPONENT_EVIDENCE")
                                            .expect("evidence directory"),
                                    )
                                    .join("nested-reading-final.png")
                                    .to_string_lossy()
                                    .into_owned(),
                                },
                            ),
                            (3_900, Action::Close),
                        ]);
                    }
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
                Event::Mouse(m)
                    if matches!(
                        self.mode.as_str(),
                        "toolbar" | "toolbar-groups" | "toolbar-split"
                    ) =>
                {
                    let eligible = matches!(
                        m.kind,
                        tekes_ui_platform_macos::MouseEventKind::Down {
                            button: tekes_ui_platform_macos::MouseButton::Left,
                            ..
                        } | tekes_ui_platform_macos::MouseEventKind::Dragged {
                            button: tekes_ui_platform_macos::MouseButton::Left
                        }
                    ) && m.position.y >= 0.
                        && m.position.y
                            < tekes_ui_controls::ToolbarControl::window().row_height(&mut self.text)
                                as f64;
                    self.dispatch(Event::Mouse(m));
                    if eligible && self.tree.event_bubbled() {
                        self.tree
                            .event(&tekes_ui_controls::composition::ContentEvent::Cancel);
                        if matches!(
                            m.kind,
                            tekes_ui_platform_macos::MouseEventKind::Down { click_count: 2, .. }
                        ) {
                            w.try_titlebar_double_click();
                        } else {
                            w.try_begin_titlebar_drag();
                        }
                    }
                    self.redraw(w);
                }
                Event::AnimationFrame(_) => self.redraw(w),
                Event::Redraw => self.redraw(w),
                Event::CloseRequested => {
                    if self.mode == "nested-reading" {
                        eprintln!("nested-reading-close-requested->Window.close");
                    }
                    w.close();
                }
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
                | "nested-reading"
                | "plain"
                | "number"
                | "editable-number"
                | "field"
                | "label"
                | "layout"
                | "item"
                | "menu-tree"
                | "toolbar"
                | "toolbar-groups"
                | "toolbar-split"
                | "content-help"
        ),
        "usage: native [list|dialog|slider|command|reading|nested-reading|plain|number|field|label|layout|item|menu-tree|toolbar|toolbar-groups|toolbar-split|content-help]"
    );
    let mut reading = None;
    let mut nested_reading = None;
    let mut plain_value = None;
    let mut item_version = None;
    let (tree, invoker, row_activations) = if mode == "toolbar" {
        (
            tekes_ui_composition_consumer::toolbar_semantics::build().unwrap(),
            tekes_ui_composition_consumer::toolbar_semantics::EDITOR,
            None,
        )
    } else if mode == "toolbar-split" {
        (
            tekes_ui_composition_consumer::split_toolbar::build().unwrap(),
            tekes_ui_composition_consumer::split_toolbar::NAV,
            None,
        )
    } else if mode == "toolbar-groups" {
        (
            tekes_ui_composition_consumer::toolbar_groups::build().unwrap(),
            tekes_ui_composition_consumer::toolbar_groups::SAVE,
            None,
        )
    } else if mode == "menu-tree" {
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
    } else if mode == "editable-number" {
        (
            tekes_ui_composition_consumer::editable_number::build().unwrap(),
            tekes_ui_composition_consumer::editable_number::PERCENT,
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
    } else if mode == "nested-reading" {
        let (tree, state) = tekes_ui_composition_consumer::nested_reading::build().unwrap();
        nested_reading = Some(state);
        (
            tree,
            tekes_ui_composition_consumer::nested_reading::APPEND,
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
    } else if mode == "content-help" {
        use tekes_ui_composition_consumer::help_list::{self, FIRST};
        (
            help_list::build().expect("public help List fixture"),
            FIRST,
            None,
        )
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
        nested_scroll_events: 0,
        help_removed: None,
        menu_actions: Vec::new(),
        reading,
        nested_reading,
        reading_width: false,
        item_version,
        item_error: None,
        layout_collapsed: mode == "layout",
        layout_error: None,
        layout_baseline,
        plain_value,
        split_icon: None,
        gpu: None,
        text: backend,
        scene: Scene::new(),
        palette: Palette::from(Theme::LIGHT),
        mode: mode.clone(),
        invoker,
        dialog: ids::DIALOG,
        toolbar_commands: Vec::new(),
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
            title: if mode == "toolbar-split" {
                "TekesUI Toolbar + Splitter Review".into()
            } else {
                format!("TekesUI Composition Consumer · {mode}")
            },
            unified_titlebar: matches!(
                mode.as_str(),
                "toolbar" | "toolbar-groups" | "toolbar-split"
            ),
            size: Size {
                width: if mode == "toolbar-split" { 960. } else { 720. },
                height: if mode == "toolbar-split" { 560. } else { 640. },
            },
            min_size: Some(Size {
                width: if mode == "nested-reading" { 320. } else { 360. },
                height: 480.,
            }),
            placement: if mode == "toolbar-split" {
                tekes_ui_platform_macos::WindowPlacement::Screen(
                    tekes_ui_platform_macos::ScreenChoice::Builtin,
                )
            } else {
                tekes_ui_platform_macos::WindowPlacement::Auto {
                    need_120hz: false,
                    size_check: false,
                }
            },
            ..Default::default()
        },
        if mode == "content-help" {
            MenuBar {
                menus: vec![Menu::new(
                    "Help Fixture",
                    vec![
                        MenuItem::Command(MenuCommand::new("Set help query B", CommandId(35_001))),
                        MenuItem::Command(MenuCommand::new("Revoke help", CommandId(35_002))),
                        MenuItem::Command(MenuCommand::new(
                            "Remove or remount first row",
                            CommandId(35_003),
                        )),
                    ],
                )],
            }
        } else {
            MenuBar::default()
        },
        host,
    );
}
