//! Original fixture with real glyph rendering and one native window adapter.
use super::*;
use tekes_ui_controls::{TextSurfaceHost, TextSurfaceTextRenderer};
use tekes_ui_platform_macos as native;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Point, Size};
#[path = "native_fonts.rs"]
mod fonts;

const CAPS: ViewCapabilities = ViewCapabilities {
    accessibility: true,
    ..super::CAPS
};
pub(super) const MODULE: &str = "original-content-native-glyphs";
pub(super) struct NativeText {
    fonts: fonts::Fonts,
    pub(super) atlas: GlyphAtlas,
    pub(super) scale: f32,
    pub(super) context: ViewContext,
    animation_requested: bool,
}
impl NativeText {
    pub(super) fn new() -> Self {
        Self {
            fonts: fonts::Fonts::new(),
            atlas: GlyphAtlas::default(),
            scale: 1.,
            context: ViewContext::default(),
            animation_requested: false,
        }
    }
    pub(super) fn apply_context(&mut self, context: &ViewContext) -> Result<()> {
        tekes_ui_module_runtime::view_context_palette(context)?;
        self.fonts.set_type_scale(context.type_scale);
        self.context = context.clone();
        Ok(())
    }
    fn adapter(&mut self) -> fonts::TextRenderer<'_> {
        fonts::TextRenderer {
            fonts: &mut self.fonts,
            atlas: &mut self.atlas,
            scale: self.scale,
        }
    }
}
impl ButtonTextRenderer for NativeText {
    fn request_animation_frame(&mut self) {
        self.animation_requested = true;
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: Duration::from_nanos(self.context.elapsed_nanos),
            reduced_motion: self.context.reduced_motion,
        })
    }

    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        self.adapter().measure(s, role, width)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.adapter().draw(scene, s, role, rect, align, ink);
    }
}
impl TextSurfaceTextRenderer for NativeText {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        p: &tekes_ui_text::Paragraph,
        origin: Point,
        ink: Rgba,
        clip: Rect,
    ) {
        self.adapter().push_visible(scene, p, origin, ink, clip);
    }
}
impl TextSurfaceHost for NativeText {
    fn caret_visible(&self) -> bool {
        self.context.caret_visible
    }

    fn type_scale(&self) -> f32 {
        self.fonts.type_scale
    }
    fn shape_surface(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        self.adapter().shape_surface(s, role, width)
    }
}
struct NativeModule {
    original: Module,
    text: NativeText,
}
pub(super) fn create() -> Box<dyn ProcessModule> {
    Box::new(NativeModule {
        original: accessibility_consumer::original_semantic_module(),
        text: NativeText::new(),
    })
}
impl ProcessModule for NativeModule {
    fn set_view_context(&mut self, context: &ViewContext) -> Result<()> {
        self.text.apply_context(context)?;
        self.original.revision += 1;
        Ok(())
    }

    fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
        Some((&mut self.original.host, &mut self.text))
    }
    fn view(&mut self) -> Option<&mut dyn PanelModule> {
        Some(self)
    }
    fn view_capabilities(&self) -> ViewCapabilities {
        CAPS
    }
    fn started(&mut self, emitter: &ComponentEmitter) -> Result<()> {
        self.original.started(emitter)
    }
    fn handle(
        &mut self,
        request: RequestBody,
        emitter: &ComponentEmitter,
    ) -> Result<serde_json::Value> {
        if let RequestBody::Message { topic, .. } = &request
            && topic == "view-context-inspect"
        {
            return Ok(serde_json::json!({
                "context":self.text.context,
                "render_revision":self.original.revision,
                "resolved_type_scale":self.text.type_scale(),
                "caret_visible":self.text.caret_visible(),
                "motion_reduced":self.text.motion_context().unwrap().reduced_motion,
                "motion_nanos":self.text.motion_context().unwrap().now.as_nanos(),
                "surface":Some(tekes_ui_module_runtime::view_context_background(&self.text.context)?).map(|c|[c.r,c.g,c.b,c.a]),
                "inherited_surface":self.original.host.inherited_surface(),
                "pid":std::process::id(),
            }));
        }
        self.original.handle(request, emitter)
    }
}
impl PanelModule for NativeModule {
    fn update(
        &mut self,
        input: &Request,
        scene: &mut Scene,
        atlas: &mut GlyphAtlas,
    ) -> Result<ModuleState> {
        self.text.scale = input.scale;
        // The worker owns its original persistent atlas; the adapter only
        // borrows it during this same original Content render.
        std::mem::swap(atlas, &mut self.text.atlas);
        let frame = Rect::new(0., 0., input.width as f32, input.height as f32);
        let palette = tekes_ui_module_runtime::view_context_palette(&self.text.context)?;
        let rendered = self
            .original
            .host
            .render(scene, &mut self.text, frame, frame, palette);
        std::mem::swap(atlas, &mut self.text.atlas);
        rendered?;
        self.original.revision += 1;
        Ok(ModuleState {
            revision: self.original.revision,
            toolbar: vec![],
        })
    }
}
struct Host {
    module: &'static str,
    capabilities: ViewCapabilities,
    tree: Option<ContentHost>,
    renderer: Option<Renderer>,
    text: NativeText,
    ids: Vec<ComponentId>,
    type_scale: f32,
    logged_ready: bool,
    context: ViewContext,
    started: Instant,
    overlay_review: bool,
    review_directory: Option<std::path::PathBuf>,
    review_started: bool,
}
impl Host {
    fn created(&mut self, w: &native::Window) -> Result<()> {
        ensure!(
            w.set_text_key_events(true),
            "native ordered key markers unavailable"
        );
        self.renderer = Some(unsafe {
            Renderer::new_for_metal_layer(w.metal_layer(), w.drawable_size(), w.scale() as f32)?
        });
        let wake = w.wake_proxy();
        let window = u64::try_from(w.native_window_number())?;
        let children = (0..3)
            .map(|index| {
                let id = ComponentId(200 + index);
                self.ids.push(id);
                let wake = wake.clone();
                let mut container = tekes_ui_module_runtime::ProcessContainer::new_view(
                    ProcessComponentOptions::new(self.module, 81).with_view(
                        window,
                        10 + index,
                        self.capabilities,
                    ),
                    move || {
                        wake.wake();
                    },
                )?;
                container.set_scale(w.scale() as f32)?;
                container.set_view_context(self.context.clone())?;
                Ok(Content::new(id, container))
            })
            .collect::<Result<Vec<_>>>()?;
        self.tree = Some(ContentHost::new(Content::new(
            ComponentId(100),
            GridContent {
                model: tekes_ui_controls::extended::Grid::new(3),
                children,
            },
        ))?);
        self.tree.as_mut().unwrap().set_focus_wrap(false);
        self.tree.as_mut().unwrap().set_window_active(w.is_active());
        eprintln!(
            "PROCESS_NATIVE_CREATED parent_pid={} window={} screen={:?}",
            std::process::id(),
            window,
            w.placement_report()
        );
        self.redraw(w)
    }
    fn publish(&mut self, w: &native::Window) {
        // Keep the original Host geometry lease with its complete semantic tree.
        // A clipped child may have zero visible bounds; raw node publication
        // deliberately cannot authorize those nodes and rejects the whole batch.
        let publication = self.tree.as_ref().and_then(|tree| {
            use native::content_accessibility::{
                ContentAccessibilityOwner, ContentAccessibilitySnapshot,
                WindowAccessibilityPublication,
            };
            ContentAccessibilitySnapshot::from_host(tree).and_then(|snapshot| {
                WindowAccessibilityPublication::prepare(vec![(
                    ContentAccessibilityOwner(0),
                    snapshot,
                )])
            })
        });
        w.set_content_accessibility_publication(publication);
        w.set_text_input_state(
            self.tree
                .as_ref()
                .and_then(tekes_ui_module_runtime::content_text_input_state),
        );
    }
    fn redraw(&mut self, w: &native::Window) -> Result<()> {
        let (Some(tree), Some(renderer)) = (&mut self.tree, &mut self.renderer) else {
            return Ok(());
        };
        tree.set_window_active(w.is_active());
        for id in &self.ids {
            for event in tree
                .get(*id)
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .drain_events()
            {
                eprintln!("PROCESS_NATIVE_SERVICE component={} {event:?}", id.0);
            }
            tree.get_mut(*id)
                .unwrap()
                .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .prepare(renderer)?;
        }
        self.text.scale = w.scale() as f32;
        let size = w.size();
        let frame = Rect::new(0., 0., size.width as f32, size.height as f32);
        let mut scene = Scene::new();
        self.text.animation_requested = false;
        tree.set_inherited_surface(&self.context.surface)
            .expect("validated window Surface");
        tree.render(
            &mut scene,
            &mut self.text,
            frame,
            frame,
            tekes_ui_module_runtime::view_context_palette(&self.context)?,
        )?;
        renderer.render(
            &scene,
            &mut self.text.atlas,
            tekes_ui_module_runtime::view_context_background(&self.context)?,
        )?;
        w.set_animating(w.is_active() && self.text.animation_requested);
        if !self.logged_ready
            && self.ids.iter().all(|id| {
                tree.get(*id)
                    .unwrap()
                    .component::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    .input_ready()
            })
        {
            self.logged_ready = true;
            let children:Vec<_>=self.ids.iter().map(|id| {
                let c=tree.get(*id).unwrap().component::<tekes_ui_module_runtime::ProcessContainer>().unwrap();
                serde_json::json!({"id":id.0,"session":c.session(),"status":format!("{:?}",c.status())})
            }).collect();
            eprintln!("PROCESS_NATIVE_READY {}", serde_json::to_string(&children)?);
        }
        self.publish(w);
        self.start_native_review()?;
        Ok(())
    }
    fn start_native_review(&mut self) -> Result<()> {
        if self.review_started || self.review_directory.is_none() || !self.logged_ready {
            return Ok(());
        }
        let nodes = native::content_accessibility::content_nodes(self.tree.as_ref().unwrap());
        let named = |label: &str| {
            let mut found: Vec<_> = nodes
                .iter()
                .filter(|n| n.label == label && n.frame.size.width > 0. && n.frame.size.height > 0.)
                .collect();
            found.sort_by(|a, b| a.frame.origin.x.total_cmp(&b.frame.origin.x));
            found
        };
        let editors = named("Message");
        let others = named("Other");
        let auxiliary = named("Auxiliary");
        let history = named("Original history");
        if [editors.len(), others.len(), auxiliary.len(), history.len()] != [3, 3, 3, 3] {
            return Ok(());
        }
        let center = |n: &native::accessibility::AccessibilityNode| {
            (
                n.frame.origin.x + n.frame.size.width * 0.5,
                n.frame.origin.y + n.frame.size.height * 0.5,
            )
        };
        let editor = center(editors[0]);
        let other = center(others[0]);
        let auxiliary_point = center(auxiliary[0]);
        let reader = center(history[0]);
        let third = center(editors[2]);
        let foreign_auxiliary = auxiliary[1].key;
        use native::selftest::Action;
        if std::env::var_os("TEKESUI_PROCESS_NATIVE_READING_SELFTEST").is_some() {
            self.review_started = true;
            native::selftest::run_script(vec![
                (200, Action::MenuCommand(48_001)),
                (
                    400,
                    Action::ScrollWheel {
                        x: reader.0,
                        y: reader.1,
                        delta_x: 0,
                        delta_y: 4,
                    },
                ),
                (1_000, Action::MenuCommand(48_008)),
                (
                    1_200,
                    Action::ScrollWheel {
                        x: reader.0,
                        y: reader.1,
                        delta_x: 0,
                        delta_y: -4,
                    },
                ),
                (1_800, Action::MenuCommand(48_009)),
                (
                    2_000,
                    Action::ScrollWheel {
                        x: reader.0,
                        y: reader.1,
                        delta_x: 0,
                        delta_y: -4,
                    },
                ),
                (2_600, Action::MenuCommand(48_010)),
                (3_000, Action::Close),
            ]);
            return Ok(());
        }
        let marked = || Action::ExpectTextState {
            text: "Keep A😀B拼音".into(),
            selection: 11..11,
            marked: Some(9..11),
            query: 9..11,
            actual: 9..11,
        };
        let third_marked = || Action::ExpectTextState {
            text: "draft拼音".into(),
            selection: 7..7,
            marked: Some(5..7),
            query: 5..7,
            actual: 5..7,
        };
        self.review_started = true;
        native::selftest::run_script(vec![
            (200, Action::MenuCommand(48_001)),
            (
                400,
                Action::Click {
                    x: editor.0,
                    y: editor.1,
                },
            ),
            (
                800,
                Action::TextRange {
                    text: "Keep A😀B".into(),
                    replacement: Some(0..5),
                },
            ),
            (
                1_200,
                Action::PreeditRange {
                    text: "拼音".into(),
                    selection: 2..2,
                    replacement: None,
                },
            ),
            (1_700, marked()),
            (
                1_900,
                Action::Click {
                    x: auxiliary_point.0,
                    y: auxiliary_point.1,
                },
            ),
            (2_300, marked()),
            (2_500, Action::MenuCommand(48_002)),
            (3_000, marked()),
            (
                3_200,
                Action::ScrollWheel {
                    x: reader.0,
                    y: reader.1,
                    delta_x: 0,
                    delta_y: 4,
                },
            ),
            (3_600, marked()),
            (
                3_800,
                Action::AXPressKey {
                    key: foreign_auxiliary,
                },
            ),
            (4_300, marked()),
            (4_500, Action::MenuCommand(48_003)),
            (4_700, Action::Unmark),
            (
                5_100,
                Action::ExpectText {
                    text: "Keep A😀B拼音".into(),
                    marked: false,
                },
            ),
            (
                5_300,
                Action::Click {
                    x: other.0,
                    y: other.1,
                },
            ),
            (
                5_650,
                Action::KeyCharacters {
                    key_code: 48,
                    chars: "\t",
                },
            ),
            (
                5_950,
                Action::KeyCharacters {
                    key_code: 48,
                    chars: "\t",
                },
            ),
            (6_400, Action::MenuCommand(48_004)),
            (
                6_600,
                Action::Click {
                    x: third.0,
                    y: third.1,
                },
            ),
            (
                7_000,
                Action::PreeditRange {
                    text: "拼音".into(),
                    selection: 2..2,
                    replacement: None,
                },
            ),
            (7_500, third_marked()),
            (7_700, Action::Resize { w: 1000., h: 360. }),
            (8_400, third_marked()),
            (8_600, Action::MenuCommand(48_005)),
            (8_800, Action::Miniaturize),
            (9_600, Action::Deminiaturize),
            (10_400, Action::MenuCommand(48_006)),
            // Window deactivation deliberately cancels focus. Re-enter with
            // actual user input; do not invent automatic focus restoration.
            (10_600, Action::Resize { w: 1000., h: 500. }),
            (
                11_000,
                Action::Click {
                    x: third.0,
                    y: third.1,
                },
            ),
            (11_400, Action::Unmark),
            (
                11_800,
                Action::ExpectText {
                    text: "draft拼音".into(),
                    marked: false,
                },
            ),
            (12_100, Action::MenuCommand(48_007)),
            (12_400, Action::Close),
        ]);
        Ok(())
    }
    fn native_review_snapshot(&self, w: &native::Window, phase: &str) -> Result<()> {
        use std::io::Write;
        let directory = self
            .review_directory
            .as_ref()
            .context("native review directory")?;
        let tree = self.tree.as_ref().context("native mixed tree")?;
        let mut children = Vec::new();
        for id in &self.ids {
            let c = tree
                .get(*id)
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap();
            children.push(serde_json::json!({
                "id": id.0, "session": c.session(), "status": format!("{:?}",c.status()),
                "input_ready": c.input_ready(), "error":c.error(),
                "reading": container_message(c, "reading-inspect")?,
                "actions": container_message(c, "inspect")?
            }));
        }
        let text = tekes_ui_module_runtime::content_text_input_state(tree).map(|s| {
            serde_json::json!({
                "text": s.text, "selection":[s.selection.start,s.selection.end],
                "marked":s.marked.map(|r|[r.start,r.end]),
                "caret_origin":[s.caret_origin.x,s.caret_origin.y],
                "caret_size":[s.caret_size.width,s.caret_size.height],"read_only":s.read_only
            })
        });
        let nodes = native::content_accessibility::content_nodes(tree);
        let snapshot = serde_json::json!({"phase":phase,"parent_pid":std::process::id(),
            "focused_container":tree.focused().map(|id|id.0),"window_active":w.is_active(),
            "window_size":[w.size().width,w.size().height],"native_text":text,"children":children,
            "nodes":nodes.iter().map(|n|serde_json::json!({"component":n.key.component,"part":n.key.part,
                "role":format!("{:?}",n.role),"label":n.label,"focused":n.focused,"value":n.value,
                "frame":[n.frame.origin.x,n.frame.origin.y,n.frame.size.width,n.frame.size.height]})).collect::<Vec<_>>()});
        std::fs::create_dir_all(directory)?;
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("native-mixed-history.jsonl"))?,
            "{snapshot}"
        )?;
        if phase == "after_tab" {
            ensure!(
                tree.focused() == Some(self.ids[1])
                    && children[1]["actions"]["focused"] == EDIT.0
                    && text.as_ref().is_some_and(|s| s["text"] == "draft"),
                "native Tab did not enter the next Reader and advance from history to its editor"
            );
            ensure!(
                children[0]["actions"]["edit_text"] == "Keep A😀B拼音"
                    && children[0]["actions"]["submits"] == 0,
                "native Tab changed or submitted the old editor"
            );
        }
        if phase == "after_reactivate" {
            ensure!(
                tree.focused().is_none() && text.is_none(),
                "inactive focus was not revoked"
            );
            ensure!(
                children[2]["actions"]["edit_text"] == "draft拼音"
                    && children[2]["actions"]["submits"] == 0,
                "window lifecycle lost or submitted the original text"
            );
        }
        eprintln!("PROCESS_NATIVE_REVIEW phase={phase}");
        Ok(())
    }
    fn zoom(&mut self, w: &native::Window, delta: f32) -> Result<()> {
        self.type_scale = (self.type_scale + delta).clamp(0.5, 4.);
        self.text.fonts.set_type_scale(self.type_scale);
        self.context.type_scale = self.type_scale;
        self.sync_context()?;
        eprintln!("PROCESS_NATIVE_FONT {}", self.type_scale);
        w.request_redraw();
        Ok(())
    }
    fn sync_context(&mut self) -> Result<()> {
        self.context.elapsed_nanos = u64::try_from(self.started.elapsed().as_nanos())?;
        self.text.context = self.context.clone();
        if let Some(tree) = &mut self.tree {
            for id in &self.ids {
                tree.get_mut(*id)
                    .unwrap()
                    .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    .set_view_context(self.context.clone())?;
            }
        }
        eprintln!("PROCESS_NATIVE_CONTEXT {:?}", self.context);
        Ok(())
    }
    fn handle_event(&mut self, event: native::Event, w: &mut native::Window) -> Result<()> {
        // Sample the original clock at real input, not at arbitrary IPC wakes.
        // Otherwise a later hover/open starts from an old caret-blink instant.
        if w.is_active()
            && matches!(
                &event,
                native::Event::Mouse(_)
                    | native::Event::Scroll(_)
                    | native::Event::Key(_)
                    | native::Event::TextKey(_)
                    | native::Event::Accessibility { .. }
            )
        {
            self.sync_context()?;
        }
        match event {
            native::Event::Created => self.created(w)?,
            native::Event::AnimationFrame(_) => {
                if w.is_active() {
                    self.sync_context()?;
                    self.redraw(w)?;
                } else {
                    w.set_animating(false);
                }
            }
            native::Event::CaretBlink => {
                self.context.caret_visible = !self.context.caret_visible;
                self.sync_context()?;
                w.request_redraw();
            }
            native::Event::SystemAppearanceChanged {
                dark,
                increase_contrast,
            } => {
                self.context.dark = dark;
                self.context.increased_contrast = increase_contrast;
                self.sync_context()?;
                w.request_redraw();
            }
            native::Event::SystemMotionChanged(reduced) => {
                self.context.reduced_motion = reduced;
                self.sync_context()?;
                w.request_redraw();
            }
            native::Event::Redraw | native::Event::Wake => self.redraw(w)?,
            native::Event::Resized {
                drawable_size,
                scale,
                ..
            } => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(drawable_size, scale as f32);
                }
                if let Some(tree) = &mut self.tree {
                    for id in &self.ids {
                        tree.get_mut(*id)
                            .unwrap()
                            .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                            .unwrap()
                            .set_scale(scale as f32)?;
                    }
                }
                self.redraw(w)?;
            }
            native::Event::Focus(active) => {
                if let Some(tree) = &mut self.tree {
                    tree.set_window_active(active && w.is_active());
                }
                eprintln!(
                    "PROCESS_NATIVE_FOCUS active={active} actual={}",
                    w.is_active()
                );
                self.publish(w);
                w.request_redraw();
            }
            native::Event::Command(native::CommandId(command))
                if self.review_directory.is_some() && (48_001..=48_010).contains(&command) =>
            {
                if command == 48_002 {
                    let tree = self.tree.as_ref().context("native mixed tree")?;
                    let c = tree
                        .get(self.ids[0])
                        .unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>()
                        .unwrap();
                    let _ = container_message(c, "reading-append")?;
                    w.request_redraw();
                } else {
                    let phase = match command {
                        48_001 => "ready",
                        48_003 => "after_aux_append_scroll",
                        48_004 => "after_tab",
                        48_005 => "after_resize",
                        48_006 => "after_reactivate",
                        48_007 => "final",
                        48_008 => "reading_inner",
                        48_009 => "reading_restored",
                        48_010 => "reading_boundary",
                        _ => unreachable!(),
                    };
                    self.native_review_snapshot(w, phase)?;
                }
            }
            native::Event::CloseRequested => {
                if self.review_started {
                    self.native_review_snapshot(w, "before_close")?;
                }
                w.set_animating(false);
                self.tree.take();
                w.set_text_input_state(None);
                w.set_content_accessibility_publication(None);
                w.close();
            }
            native::Event::Key(ref key)
                if self.overlay_review
                    && key.kind == native::KeyEventKind::Down
                    && key.modifiers.command
                    && key.characters_ignoring_modifiers == "o" =>
            {
                if w.is_active()
                    && let Some(tree) = &self.tree
                {
                    tree.get(self.ids[0])
                        .unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>()
                        .unwrap()
                        .request(RequestBody::Message {
                            topic: "open_pointer_scope".into(),
                            payload: serde_json::Value::Null,
                        })?;
                    eprintln!("PROCESS_NATIVE_OPEN_REMOTE_POPOVER");
                }
                w.request_redraw();
            }
            native::Event::Key(ref key)
                if key.kind == native::KeyEventKind::Down
                    && key.modifiers.command
                    && matches!(key.characters_ignoring_modifiers.as_str(), "+" | "=" | "-") =>
            {
                if w.is_active() {
                    self.zoom(
                        w,
                        if key.characters_ignoring_modifiers == "-" {
                            -0.1
                        } else {
                            0.1
                        },
                    )?;
                }
            }
            other => {
                if let Some(tree) = &mut self.tree
                    && let Some(response) =
                        tekes_ui_module_runtime::dispatch_content_window_event(tree, &other, w)
                {
                    eprintln!(
                        "PROCESS_NATIVE_INPUT native={other:?} response={response:?} focused={:?}",
                        tree.focused()
                    );
                    self.publish(w);
                    w.request_redraw();
                }
            }
        }
        Ok(())
    }
}
impl native::WindowHandler for Host {
    fn handle(&mut self, event: native::Event, w: &mut native::Window) {
        if let Err(error) = self.handle_event(event, w) {
            eprintln!("PROCESS_NATIVE_ERROR {error:#}");
            if self.review_directory.is_some() {
                panic!("native mixed acceptance failed: {error:#}");
            }
            w.set_text_input_state(None);
        }
    }
}
pub(super) fn run() -> Result<()> {
    run_mode(false, MODULE, CAPS)
}
pub(super) fn run_overlay() -> Result<()> {
    run_mode(true, MODULE, CAPS)
}
pub(super) fn run_scroll() -> Result<()> {
    run_mode(
        false,
        scroll_consumer::NATIVE_MODULE,
        ViewCapabilities {
            scroll: true,
            ..CAPS
        },
    )
}
fn run_mode(
    overlay_review: bool,
    module: &'static str,
    capabilities: ViewCapabilities,
) -> Result<()> {
    let review_directory =
        std::env::var_os("TEKESUI_PROCESS_NATIVE_SELFTEST_EVIDENCE").map(std::path::PathBuf::from);
    ensure!(
        review_directory.is_none() || module == scroll_consumer::NATIVE_MODULE,
        "native mixed selftest requires the real mixed-reader module"
    );
    ensure!(
        std::env::var_os("TEKESUI_PROCESS_NATIVE_READING_SELFTEST").is_none()
            || review_directory.is_some(),
        "native reading selftest requires its evidence directory"
    );
    let menu = if review_directory.is_some() {
        native::MenuBar {
            menus: vec![native::Menu::new(
                "Acceptance",
                (48_001..=48_010)
                    .map(|id| {
                        native::MenuItem::Command(native::MenuCommand::new(
                            format!("Acceptance {id}"),
                            native::CommandId(id),
                        ))
                    })
                    .collect(),
            )],
        }
    } else {
        native::MenuBar::default()
    };
    native::App::new(native::AppConfig {
        name: Some("TekesUI Process Content Review".into()),
        ..Default::default()
    })
    .run(
        native::WindowConfig {
            title: "TekesUI Process Content Review".into(),
            size: native::Size {
                width: 1000.,
                height: 500.,
            },
            placement: native::WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
            ..Default::default()
        },
        menu,
        Host {
            module,
            capabilities,
            tree: None,
            renderer: None,
            text: NativeText::new(),
            ids: Vec::new(),
            type_scale: 1.,
            logged_ready: false,
            context: ViewContext::default(),
            started: Instant::now(),
            overlay_review,
            review_directory,
            review_started: false,
        },
    );
    Ok(())
}

/// Real isolated worker + original Content/backend; no window or physical IME claim.
pub(super) fn context_roundtrip() -> Result<serde_json::Value> {
    use tekes_ui_module_runtime::ProcessContainer;
    let id = ComponentId(410);
    let mut child = ProcessContainer::new_view(
        ProcessComponentOptions::new(MODULE, 91).with_view(9, 81, CAPS),
        || {},
    )?;
    let initial = ViewContext::default();
    ensure!(
        child.set_view_context(initial.clone())?,
        "first context ignored"
    );
    ensure!(
        !child.set_view_context(initial.clone())?,
        "identical context creates work"
    );
    let mut host = ContentHost::new(Content::new(id, child))?;
    let mut renderer = Renderer::new_headless()?;
    let mut text = Text;
    let frame = Rect::new(0., 0., 320., 500.);
    fn ready(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        text: &mut Text,
        frame: Rect,
    ) -> Result<()> {
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            host.get_mut(ComponentId(410))
                .unwrap()
                .component_mut::<ProcessContainer>()
                .unwrap()
                .prepare(renderer)?;
            host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())?;
            let c = host
                .get(ComponentId(410))
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(c.error().is_none(), "context worker failed {:?}", c.error());
            if c.input_ready() {
                return Ok(());
            }
            ensure!(
                Instant::now() < end,
                "context layout acknowledgement deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    ready(&mut host, &mut renderer, &mut text, frame)?;
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let first = container_message(c, "view-context-inspect")?;
    let pid = first["pid"].as_u64().context("worker PID")?;
    let session = c.session().unwrap().clone();
    let (owner, original_binding) = {
        let bridge = c.native_content_bridge(Duration::from_secs(5))?;
        let snapshot = bridge.snapshot()?;
        let editor = *snapshot
            .reply()
            .focus_targets
            .iter()
            .find(|t| t.id == EDIT.0)
            .unwrap();
        let focused = bridge.focus(&snapshot, Some(editor), true)?;
        let marked = bridge.request(
            &focused,
            wire::Input::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: Some(1..3),
            },
        )?;
        ensure!(
            marked.text_input_state().unwrap().marked.is_some(),
            "original preedit absent"
        );
        (editor, marked.reply().text_binding.clone())
    };
    let mut contexts = Vec::new();
    let mut revision = 0;
    for surface in tekes_ui_tokens::surface_palette::surface_names() {
        for type_scale in [1., 1.5, 2.] {
            for dark in [false, true] {
                for increased_contrast in [false, true] {
                    revision += 1;
                    let context = ViewContext {
                        type_scale,
                        dark,
                        increased_contrast,
                        brand: 0,
                        surface: (*surface).into(),
                        elapsed_nanos: revision,
                        caret_visible: revision % 2 == 0,
                        reduced_motion: revision % 2 != 0,
                    };
                    host.get_mut(id)
                        .unwrap()
                        .component_mut::<ProcessContainer>()
                        .unwrap()
                        .set_view_context(context.clone())?;
                    ready(&mut host, &mut renderer, &mut text, frame)?;
                    let c = host
                        .get(id)
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap();
                    let mut report = container_message(c, "view-context-inspect")?;
                    ensure!(
                        report["inherited_surface"] == *surface,
                        "SDK did not inherit Surface into original ContentHost"
                    );
                    ensure!(
                        report["context"] == serde_json::to_value(&context)?,
                        "environment changed in transit"
                    );
                    ensure!(
                        report["resolved_type_scale"].as_f64() == Some(f64::from(type_scale)),
                        "original font backend did not scale"
                    );
                    ensure!(
                        report["caret_visible"].as_bool() == Some(context.caret_visible)
                            && report["motion_reduced"].as_bool() == Some(context.reduced_motion),
                        "original backend dropped caret/motion"
                    );
                    let color = tekes_ui_module_runtime::view_context_background(&context)?;
                    ensure!(
                        report["surface"]
                            == serde_json::json!([color.r, color.g, color.b, color.a]),
                        "palette bypasses compiled JSON"
                    );
                    let revision = report["render_revision"]
                        .as_u64()
                        .context("context revision")?;
                    let end = Instant::now() + Duration::from_secs(15);
                    loop {
                        host.get_mut(id)
                            .unwrap()
                            .component_mut::<ProcessContainer>()
                            .unwrap()
                            .prepare(&mut renderer)?;
                        let mut scene = Scene::new();
                        host.render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())?;
                        if host
                            .get(id)
                            .unwrap()
                            .component::<ProcessContainer>()
                            .unwrap()
                            .revision()
                            .is_some_and(|v| v >= revision)
                        {
                            let pixels = renderer.render_to_rgba8(
                                &scene,
                                &mut GlyphAtlas::default(),
                                (320, 500),
                                1.,
                                Rgba::gray(1.),
                            )?;
                            let offset = (480 * 320 + 300) * 4;
                            let expected = [color.r, color.g, color.b, color.a]
                                .map(|v| (v * 255.).round() as u8);
                            ensure!(
                                pixels[offset..offset + 4]
                                    .iter()
                                    .zip(expected)
                                    .all(|(a, b)| a.abs_diff(b) <= 1),
                                "remote surface background bypasses View context: actual {:?}, expected {:?}",
                                &pixels[offset..offset + 4],
                                expected
                            );
                            report["gpu_corner"] = serde_json::json!(&pixels[offset..offset + 4]);
                            report["expected_corner"] = serde_json::json!(expected);
                            break;
                        }
                        ensure!(Instant::now() < end, "context GPU frame deadline");
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    let c = host
                        .get(id)
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap();
                    let snapshot = c
                        .native_content_bridge(Duration::from_secs(5))?
                        .snapshot()?;
                    let state = snapshot
                        .text_input_state()
                        .context("original editor after context change")?;
                    ensure!(
                        state.marked.is_some() && state.text.contains("ni"),
                        "context update interrupted original preedit"
                    );
                    ensure!(
                        snapshot.reply().focused == Some(owner)
                            && snapshot.reply().text_binding == original_binding,
                        "environment changed original editor authority"
                    );
                    contexts.push(report);
                }
            }
        }
    }
    let c = host
        .get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap();
    ensure!(
        c.set_view_context(ViewContext {
            type_scale: f32::NAN,
            ..initial.clone()
        })
        .is_err(),
        "NaN scale accepted"
    );
    ensure!(
        c.set_view_context(initial.clone()).is_err(),
        "backward clock accepted"
    );
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    ensure!(
        c.session() == Some(&session),
        "environment replaced original process"
    );
    let bridge = c.native_content_bridge(Duration::from_secs(5))?;
    let snapshot = bridge.snapshot()?;
    // Bypass the convenience setter to exercise the original actor's own
    // validation boundary. A rejected request must not terminate or mutate it.
    let before_invalid = container_message(c, "view-context-inspect")?;
    let bad = c.request(RequestBody::SetViewContext {
        view: 81,
        context: ViewContext {
            surface: "unknown".into(),
            type_scale: 4.,
            elapsed_nanos: revision + 1,
            ..Default::default()
        },
    })?;
    let mut rejected = None;
    wait(|| {
        for packet in c.drain_events() {
            if let Output::Response { request, result } = packet.output
                && request == bad
            {
                rejected = Some(result);
            }
        }
        rejected.is_some()
    })?;
    ensure!(rejected.unwrap().is_err(), "invalid actor Surface accepted");
    let after_invalid = container_message(c, "view-context-inspect")?;
    ensure!(
        before_invalid == after_invalid,
        "invalid actor context changed original backend"
    );
    let preserved = bridge.snapshot()?;
    ensure!(
        preserved.reply().text_binding == snapshot.reply().text_binding
            && preserved.text_input_state() == snapshot.text_input_state(),
        "invalid context changed original IME authority"
    );
    let confirmed = bridge.request(
        &snapshot,
        wire::Input::Commit {
            text: "你".into(),
            replacement: None,
        },
    )?;
    ensure!(
        confirmed.text_input_state().unwrap().marked.is_none()
            && confirmed.text_input_state().unwrap().text.contains('你'),
        "original preedit cannot confirm after environment changes"
    );
    let point = confirmed.reply().text.as_ref().unwrap().caret;
    let pointer = |phase| PointerEvent {
        position: tekes_ui_scene::Point::new(point[0], point[1] + 1.),
        phase,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 1,
    };
    let held = bridge.pointer(&confirmed, &pointer(PointerPhase::Down))?;
    let capture = held.reply().captured;
    ensure!(capture.is_some(), "original text pointer did not capture");
    let visual = ViewContext {
        type_scale: 2.,
        elapsed_nanos: revision + 1,
        caret_visible: false,
        reduced_motion: true,
        ..Default::default()
    };
    host.get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
        .set_view_context(visual.clone())?;
    ready(&mut host, &mut renderer, &mut text, frame)?;
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let report = container_message(c, "view-context-inspect")?;
    ensure!(
        report["context"] == serde_json::to_value(&visual)?,
        "visual context not applied"
    );
    let retained = c
        .native_content_bridge(Duration::from_secs(5))?
        .snapshot()?;
    ensure!(
        retained.reply().captured == capture,
        "visual-only environment cancelled held pointer"
    );
    let zoom = ViewContext {
        type_scale: 1.,
        elapsed_nanos: revision + 2,
        ..visual.clone()
    };
    host.get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
        .set_view_context(zoom.clone())?;
    ready(&mut host, &mut renderer, &mut text, frame)?;
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let after = c
        .native_content_bridge(Duration::from_secs(5))?
        .snapshot()?;
    ensure!(
        after.reply().captured.is_none(),
        "typography change retained old pointer geometry"
    );
    ensure!(
        after.text_input_state().unwrap().text.contains('你'),
        "geometry cancellation lost original editor value"
    );
    let mut motion_checks = Vec::new();
    for (phase, nanos, reduced, topic, expected) in [
        (
            "enter",
            1_000_000_000,
            false,
            Some("open_pointer_scope"),
            true,
        ),
        ("settled", 3_000_000_000, false, None, false),
        (
            "exit",
            3_000_000_001,
            false,
            Some("close_pointer_scope"),
            true,
        ),
        ("reduced", 3_000_000_002, true, None, false),
    ] {
        let old_revision = host
            .get(id)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .revision();
        let c = host
            .get_mut(id)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap();
        c.set_view_context(ViewContext {
            elapsed_nanos: nanos,
            reduced_motion: reduced,
            ..zoom.clone()
        })?;
        if let Some(topic) = topic {
            container_message(c, topic)?;
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            host.get_mut(id)
                .unwrap()
                .component_mut::<ProcessContainer>()
                .unwrap()
                .prepare(&mut renderer)?;
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            let c = host
                .get(id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(c.error().is_none(), "motion worker error {:?}", c.error());
            if c.input_ready() && c.revision() > old_revision {
                let observed = c
                    .native_content_bridge(Duration::from_secs(5))?
                    .snapshot()?;
                ensure!(
                    !observed.reply().changed,
                    "motion query created render work"
                );
                if observed.reply().animation_frame_requested == expected
                    && host.animation_frame_requested() == expected
                {
                    motion_checks.push(
                        serde_json::json!({"phase":phase,"requested":expected,"clock":nanos}),
                    );
                    break;
                }
            }
            ensure!(
                Instant::now() < deadline,
                "original remote {phase} frame demand failed"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    drop(host);
    renderer.collect_retired_images();
    Ok(
        serde_json::json!({"result":"ORIGINAL_VIEW_CONTEXT_OK","pid":pid,"session":session,"contexts":contexts,"original_preedit_and_binding_preserved":true,"confirmed_after_updates":true,"visual_context_preserves_capture":true,"typography_cancels_only_capture":true,"invalid_input_rejected":true,"invalid_actor_context_keeps_original_backend_alive":true,"original_frame_demand":motion_checks,"scope":"real IPC + original CoreText/GlyphAtlas backend and editor; no physical IME/native/AX/Gold7 acceptance"}),
    )
}

/// Three original SDK trees: independent process/session context, not three
/// copies of a consumer input/focus implementation. No desktop window is opened.
pub(super) fn three_surface_roundtrip() -> Result<serde_json::Value> {
    use tekes_ui_module_runtime::ProcessContainer;
    let ids = [ComponentId(510), ComponentId(511), ComponentId(512)];
    let names = ["sidebar", "panel", "popover"];
    let mut children = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let mut c = ProcessContainer::new_view(
            ProcessComponentOptions::new(MODULE, 101).with_view(9, 91 + index as u64, CAPS),
            || {},
        )?;
        c.set_view_context(ViewContext {
            surface: names[index].into(),
            ..Default::default()
        })?;
        children.push(Content::new(*id, c));
    }
    let mut host = ContentHost::new(Content::new(
        ComponentId(500),
        GridContent {
            model: tekes_ui_controls::extended::Grid::new(3),
            children,
        },
    ))?;
    let frame = Rect::new(0., 0., 960., 500.);
    let mut renderer = Renderer::new_headless()?;
    let mut text = Text;
    let prepare =
        |host: &mut ContentHost, renderer: &mut Renderer, text: &mut Text| -> Result<Scene> {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                for id in ids {
                    host.get_mut(id)
                        .unwrap()
                        .component_mut::<ProcessContainer>()
                        .unwrap()
                        .prepare(renderer)?;
                }
                let mut scene = Scene::new();
                host.render(&mut scene, text, frame, frame, Theme::LIGHT.into())?;
                let mut ready = true;
                for id in ids {
                    let c = host
                        .get(id)
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap();
                    ensure!(
                        c.error().is_none(),
                        "three-context worker failed {:?}",
                        c.error()
                    );
                    ready &= c.input_ready();
                }
                if ready {
                    return Ok(scene);
                }
                ensure!(
                    Instant::now() < deadline,
                    "three-context layout acknowledgement deadline"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        };
    prepare(&mut host, &mut renderer, &mut text)?;
    let mut before = Vec::new();
    let mut bindings = Vec::new();
    let mut sessions = Vec::new();
    for id in ids {
        let c = host
            .get(id)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let report = container_message(c, "view-context-inspect")?;
        before.push(report);
        sessions.push(c.session().unwrap().clone());
        let bridge = c.native_content_bridge(Duration::from_secs(5))?;
        let snapshot = bridge.snapshot()?;
        let owner = *snapshot
            .reply()
            .focus_targets
            .iter()
            .find(|target| target.id == EDIT.0)
            .context("original editor")?;
        let focused = bridge.focus(&snapshot, Some(owner), true)?;
        let marked = bridge.request(
            &focused,
            wire::Input::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: Some(1..3),
            },
        )?;
        ensure!(
            marked.text_input_state().unwrap().marked.is_some(),
            "original preedit missing"
        );
        bindings.push(marked.reply().text_binding.clone());
    }
    let pids: std::collections::BTreeSet<_> =
        before.iter().map(|v| v["pid"].as_u64().unwrap()).collect();
    ensure!(
        pids.len() == 3 && !pids.contains(&u64::from(std::process::id())),
        "process isolation missing"
    );
    let mut reports = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let next = ViewContext {
            surface: names[(index + 1) % names.len()].into(),
            dark: index % 2 == 0,
            elapsed_nanos: 1,
            ..Default::default()
        };
        host.get_mut(*id)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .set_view_context(next.clone())?;
        prepare(&mut host, &mut renderer, &mut text)?;
        let c = host
            .get(*id)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let report = container_message(c, "view-context-inspect")?;
        ensure!(
            report["context"] == serde_json::to_value(&next)?
                && report["inherited_surface"] == next.surface,
            "original SDK tree lost its own Surface"
        );
        ensure!(
            report["pid"] == before[index]["pid"] && c.session() == Some(&sessions[index]),
            "Surface replaced process"
        );
        let snapshot = c
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            snapshot.reply().text_binding == bindings[index]
                && snapshot.text_input_state().unwrap().marked.is_some(),
            "Surface update interrupted original preedit/binding"
        );
        for (other, other_id) in ids.iter().enumerate().skip(index + 1) {
            let untouched = host
                .get(*other_id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(
                container_message(untouched, "view-context-inspect")?["context"]
                    == before[other]["context"],
                "context was misrouted to another process"
            );
        }
        reports.push(report);
    }
    // Each worker's real shared image must use its own generated background.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut pixels_checked;
    loop {
        let scene = prepare(&mut host, &mut renderer, &mut text)?;
        let pixels = renderer.render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            (960, 500),
            1.,
            Rgba::gray(1.),
        )?;
        pixels_checked = 0;
        for (index, id) in ids.iter().enumerate() {
            let rect = host.get(*id).unwrap().frame();
            let x = (rect.max_x() - 20.).floor() as usize;
            let y = (rect.max_y() - 20.).floor() as usize;
            let offset = (y * 960 + x) * 4;
            let expected = reports[index]["surface"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v.as_f64().unwrap() * 255.).round() as u8)
                .collect::<Vec<_>>();
            let c = host
                .get(*id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            let revision = reports[index]["render_revision"]
                .as_u64()
                .context("original context revision")?;
            if c.revision().is_some_and(|current| current >= revision)
                && pixels[offset..offset + 4]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(b) <= 1)
            {
                pixels_checked += 1;
            }
        }
        if pixels_checked == 3 {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "three original GPU Surface backgrounds did not settle"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(host);
    renderer.collect_retired_images();
    Ok(
        serde_json::json!({"result":"THREE_ORIGINAL_SURFACES_OK","processes":reports,"sessions":sessions,
        "gpu_backgrounds_checked":pixels_checked,"preedit_and_bindings_preserved":true,"context_misrouting":false,
        "scope":"three real child PIDs/sessions + shared GPU images; no native/physical IME/AX/scroll/Gold7 acceptance"}),
    )
}
