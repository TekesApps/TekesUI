//! Independent public API consumer. Real child Content and CoreText geometry;
//! the draw backend intentionally emits no glyph pixels. Not native/visual UAT.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Remote Content View consumer currently requires macOS");
}
#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    consumer::run()
}

#[cfg(target_os = "macos")]
mod consumer {
    mod accessibility_consumer;
    mod native_host;
    mod popup;
    use popup::Popup;
    mod overlay_consumer;
    mod projection_consumer;
    mod scroll_consumer;
    use anyhow::{Context, Result, ensure};
    use std::time::{Duration, Instant};
    use tekes_ui_app_protocol::{InputKind, Request, VERSION};
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
        composition::*,
    };
    use tekes_ui_module_runtime::component::content as wire;
    use tekes_ui_module_runtime::{
        ComponentDefinition, ComponentEmitter, ModuleState, PanelModule, ProcessComponent,
        ProcessComponentOptions, ProcessModule, ProcessStatus, component::*,
        run_component_worker_from_args,
    };
    use tekes_ui_scene::{GlyphAtlas, Rect, Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
    const MODULE: &str = "original-content-input";
    const TOOLBAR_MODULE: &str = "original-content-with-toolbar";
    const EDIT: ComponentId = ComponentId(2);
    const OTHER: ComponentId = ComponentId(4);
    const CAPS: ViewCapabilities = ViewCapabilities {
        keyboard: true,
        text_ime: true,
        primary_pointer: false,
        content_pointer: true,
        scroll: false,
        accessibility: false,
        independent_toolbar: false,
    };
    struct Text;
    impl ButtonTextRenderer for Text {
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
            Some(self)
        }
        fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (s.chars().count() as f32 * role.style().size).min(width),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    impl TextSurfaceTextRenderer for Text {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &tekes_ui_text::Paragraph,
            _: tekes_ui_scene::Point,
            _: Rgba,
            _: Rect,
        ) {
        }
    }
    impl TextSurfaceHost for Text {
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            width: Option<f32>,
        ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
            use tekes_ui_text::traits::Shaper;
            let mut s = tekes_ui_text::coretext::CoreTextShaper::new(
                tekes_ui_text::coretext::CoreTextFonts::new(),
            );
            std::rc::Rc::new(s.shape(
                text,
                &tekes_ui_text::style::TextStyle::system(role.style().size),
                &tekes_ui_text::model::ShapeOptions {
                    width: width.map(f64::from),
                    line_spacing: 0.,
                },
            ))
        }
    }
    struct Module {
        host: ContentHost,
        text: Text,
        revision: u64,
        toolbar_enabled: bool,
        toolbar_calls: u64,
        submits: u64,
    }
    fn original_module(toolbar_enabled: bool) -> Module {
        let mut editor = TextFieldContent::new("Message", "A😀B");
        editor.submit_policy = TextSubmitPolicy::Enter;
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            ListContent::new(
                "Original child",
                vec![
                    Content::new(EDIT, editor),
                    Content::new(ComponentId(3), Button::new("Auxiliary")).preserving_focus(),
                    Content::new(OTHER, TextFieldContent::new("Other", "other")),
                    Content::new(
                        ComponentId(5),
                        Popup::new(
                            "Pointer scope",
                            Content::new(ComponentId(6), Button::new("Overlay action")),
                        ),
                    ),
                ],
            ),
        ))
        .unwrap();
        host.set_focus_wrap(false);
        host.set_action_events(true);
        Module {
            host,
            text: Text,
            revision: 0,
            toolbar_enabled,
            toolbar_calls: 0,
            submits: 0,
        }
    }
    fn create_base(toolbar_enabled: bool) -> Box<dyn ProcessModule> {
        Box::new(original_module(toolbar_enabled))
    }
    fn create() -> Box<dyn ProcessModule> {
        create_base(false)
    }
    fn create_toolbar() -> Box<dyn ProcessModule> {
        create_base(true)
    }
    fn toolbar_event(revision: u64, enabled: bool) -> ComponentEvent {
        ComponentEvent::Toolbar {
            revision,
            scope: ToolbarScope::View,
            items: vec![ToolbarAction {
                command: tekes_ui_app_protocol::ToolbarItem {
                    id: 9001,
                    title: "History tools".into(),
                    enabled,
                    geometry_dependent: false,
                },
                icon: None,
                accessible_name: Some("History tools".into()),
                tooltip: Some("Original toolbar action".into()),
            }],
        }
    }
    impl ProcessModule for Module {
        fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
            Some((&mut self.host, &mut self.text))
        }
        fn view(&mut self) -> Option<&mut dyn PanelModule> {
            Some(self)
        }
        fn view_capabilities(&self) -> ViewCapabilities {
            ViewCapabilities {
                independent_toolbar: self.toolbar_enabled,
                ..CAPS
            }
        }
        fn started(&mut self, emitter: &ComponentEmitter) -> Result<()> {
            if self.toolbar_enabled {
                emitter.emit(toolbar_event(1, true))?;
            }
            emitter.emit(ComponentEvent::Message {
                topic: "started".into(),
                payload: serde_json::json!({"pid": std::process::id()}),
            })
        }
        fn handle(
            &mut self,
            input: RequestBody,
            emitter: &ComponentEmitter,
        ) -> Result<serde_json::Value> {
            match input {
                RequestBody::Message { topic, .. }
                    if self.toolbar_enabled
                        && (topic == "disable_toolbar" || topic == "enable_toolbar") =>
                {
                    let enabled = topic == "enable_toolbar";
                    emitter.emit(toolbar_event(if enabled { 3 } else { 2 }, enabled))?;
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Invoke { command: 9001, .. } if self.toolbar_enabled => {
                    self.toolbar_calls += 1;
                    Ok(serde_json::json!({"toolbar_calls":self.toolbar_calls}))
                }
                RequestBody::Message { topic, .. } if topic == "inspect" => {
                    self.submits += self
                        .host
                        .drain_actions()
                        .iter()
                        .filter(|a| a.kind == ContentActionKind::Submit)
                        .count() as u64;
                    let aux = self.host.get(ComponentId(3)).unwrap();
                    let f = aux.frame();
                    Ok(
                        serde_json::json!({"aux_frame":[f.min_x(),f.min_y(),f.size.width,f.size.height],
                        "aux_activations":aux.component::<Button>().unwrap().activations,
                        "aux_pressed":aux.component::<Button>().unwrap().button.state.pressed,
                        "view_revision":self.revision,"overlay_activations":self.host.get(ComponentId(6)).unwrap().component::<Button>().unwrap().activations,
                        "toolbar_calls":self.toolbar_calls,"submits":self.submits,"focused":self.host.focused().map(|id|id.0),"composing":self.host.is_composing(),"edit_text":self.host.get(EDIT).unwrap().component::<TextFieldContent>().unwrap().surface.input.text}),
                    )
                }
                RequestBody::Message { topic, .. } if topic == "bind_same_value" => {
                    let input = &mut self
                        .host
                        .get_mut(EDIT)
                        .unwrap()
                        .component_mut::<TextFieldContent>()
                        .unwrap()
                        .surface
                        .input;
                    let baseline = input.external_baseline();
                    let value = tekes_ui_controls::DraftRevision {
                        identity: "wire-document".into(),
                        version: 1,
                        payload: input.text.clone(),
                    };
                    ensure!(matches!(
                        input.apply_external_value(
                            &baseline,
                            &value,
                            tekes_ui_controls::ExternalSelection::Preserve,
                            tekes_ui_controls::ExternalUndo::Reset
                        ),
                        tekes_ui_controls::ExternalValueResult::Applied { .. }
                    ));
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Message { topic, .. } if topic == "clear_focus" => {
                    self.host.focus(None, false);
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Message { topic, .. } if topic == "refocus_same_editor" => {
                    self.host.focus(None, false);
                    self.host.focus(Some(EDIT), true);
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Message { topic, .. } if topic == "remount_aux" => {
                    self.host.remove(ComponentId(3))?;
                    self.host.insert(
                        ComponentId(1),
                        Content::new(ComponentId(3), Button::new("Auxiliary")).preserving_focus(),
                    )?;
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Message { topic, .. } if topic == "open_pointer_scope" => {
                    let anchor = self.host.get(ComponentId(3)).unwrap().frame();
                    self.host
                        .get_mut(ComponentId(5))
                        .unwrap()
                        .component_mut::<Popup>()
                        .unwrap()
                        .show(anchor);
                    self.revision += 1;
                    emitter.invalidate_view(self.revision)?;
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Message { topic, .. } if topic == "close_pointer_scope" => {
                    ensure!(
                        self.host
                            .get_mut(ComponentId(5))
                            .unwrap()
                            .component_mut::<Popup>()
                            .unwrap()
                            .close()
                    );
                    self.revision += 1;
                    emitter.invalidate_view(self.revision)?;
                    Ok(serde_json::Value::Null)
                }
                RequestBody::Message { topic, .. } if topic == "remount" => {
                    self.host.remove(EDIT)?;
                    self.host.insert(
                        ComponentId(1),
                        Content::new(EDIT, TextFieldContent::new("Message", "replacement")),
                    )?;
                    Ok(serde_json::Value::Null)
                }
                _ => anyhow::bail!("unsupported business request"),
            }
        }
    }
    impl PanelModule for Module {
        fn update(
            &mut self,
            input: &Request,
            scene: &mut Scene,
            _: &mut GlyphAtlas,
        ) -> Result<ModuleState> {
            let frame = Rect::new(0., 0., input.width as f32, input.height as f32);
            self.host
                .render(scene, &mut self.text, frame, frame, Theme::LIGHT.into())?;
            self.revision += 1;
            Ok(ModuleState {
                revision: self.revision,
                toolbar: vec![],
            })
        }
    }
    fn wait(mut ready: impl FnMut() -> bool) -> Result<()> {
        let end = Instant::now() + Duration::from_secs(10);
        while !ready() {
            ensure!(Instant::now() < end, "correctness deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    #[derive(Default, Clone)]
    struct Owner {
        target: Option<wire::Target>,
        epoch: u64,
        binding: Option<Vec<u8>>,
    }
    fn request(
        p: &ProcessComponent,
        owner: &mut Owner,
        view: u64,
        viewport: u64,
        input: wire::Input,
    ) -> Result<wire::Reply> {
        let reply = p.content_request(
            p.session(),
            wire::Request {
                view,
                viewport,
                owner: owner.target,
                focus_epoch: owner.epoch,
                text_binding: owner.binding.clone(),
                capture: None,
                input,
            },
            Duration::from_secs(5),
        )?;
        *owner = Owner {
            target: reply.focused,
            epoch: reply.focus_epoch,
            binding: reply.text_binding.clone(),
        };
        Ok(reply)
    }
    fn layout(p: &ProcessComponent, seq: u64, viewport: u64, width: u32) -> Result<()> {
        p.enqueue_view(Request {
            version: VERSION,
            seq,
            viewport,
            kind: InputKind::Layout,
            command: None,
            x: 0.,
            y: 0.,
            width,
            height: 300,
            identity: 1,
            scale: 1.,
        })?;
        let mut matched = false;
        wait(|| {
            for frame in p.drain_view_frames() {
                matched |= frame.metadata().viewport == viewport;
            }
            matched
        })
    }
    fn native_roundtrip(
        p: &ProcessComponent,
        view: u64,
        edit: wire::Target,
        other: wire::Target,
    ) -> Result<serde_json::Value> {
        use tekes_ui_module_runtime::{NativeViewOptions, ProcessContentBridge};
        use tekes_ui_platform_macos as native;
        let options = NativeViewOptions {
            window: 7,
            view,
            required: CAPS,
        };
        let frame = Rect::new(100., 60., 240., 300.);
        let bridge =
            ProcessContentBridge::new(p, &options, 2, frame, frame, Duration::from_secs(5))?;
        let snapshot = bridge.snapshot()?;
        let focused = bridge.focus(&snapshot, Some(edit), true)?;
        let stale_context = native::TextKeyContext {
            id: 89,
            key: native::KeyEvent {
                kind: native::KeyEventKind::Down,
                key_code: 36,
                characters: "\r".into(),
                characters_ignoring_modifiers: "\r".into(),
                modifiers: native::Modifiers::NONE,
                is_repeat: false,
            },
            composing_at_start: false,
        };
        let started = bridge
            .event(
                &focused,
                &native::Event::TextKey(native::TextKeyEvent::Started(stale_context.clone())),
            )?
            .unwrap();
        let blurred = bridge.focus(&started, None, false)?;
        let refocused = bridge.focus(&blurred, Some(edit), true)?;
        ensure!(
            bridge
                .event(
                    &focused,
                    &native::Event::TextCommit {
                        text: "LATE-ABA".into(),
                        replacement: None
                    }
                )
                .is_err(),
            "old callback edited a refocused same-mounted editor"
        );
        let finished = bridge.event(
            &started,
            &native::Event::TextKey(native::TextKeyEvent::Finished {
                context: stale_context,
                input_context_handled: Some(false),
                disposition: native::TextKeyDisposition::Unhandled,
                text_callbacks: 0,
                command_callbacks: 0,
            }),
        );
        ensure!(
            finished.is_ok(),
            "stale native key finish was rejected instead of cancelled"
        );
        ensure!(
            finished?
                .unwrap()
                .reply()
                .key
                .as_ref()
                .is_some_and(|k| k.disposition == wire::KeyDisposition::Cancelled),
            "stale native key finish did not cancel original provenance"
        );
        process_message(p, "bind_same_value")?;
        let rebound = bridge.snapshot()?;
        ensure!(
            rebound.reply().focus_epoch == refocused.reply().focus_epoch
                && rebound.reply().text_binding != refocused.reply().text_binding,
            "binding test changed focus instead of binding"
        );
        ensure!(
            bridge
                .event(
                    &refocused,
                    &native::Event::TextCommit {
                        text: "LATE-BINDING".into(),
                        replacement: None
                    }
                )
                .is_err(),
            "old callback edited a rebound same-focused editor"
        );
        let focused = rebound;
        let preedit = bridge
            .event(
                &focused,
                &native::Event::TextPreedit {
                    text: "ni".into(),
                    selection: 2..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: Some(1..2),
                },
            )?
            .unwrap();
        ensure!(
            preedit.reply().composing && preedit.text_input_state().unwrap().marked == Some(1..3)
        );
        ensure!(
            bridge.focus(&preedit, Some(other), true).is_err(),
            "native focus escaped preedit"
        );
        let key = native::TextKeyContext {
            id: 91,
            key: native::KeyEvent {
                kind: native::KeyEventKind::Down,
                key_code: 49,
                characters: " ".into(),
                characters_ignoring_modifiers: " ".into(),
                modifiers: native::Modifiers::NONE,
                is_repeat: false,
            },
            composing_at_start: true,
        };
        let started = bridge
            .event(
                &preedit,
                &native::Event::TextKey(native::TextKeyEvent::Started(key.clone())),
            )?
            .unwrap();
        let committed = bridge
            .event(
                &started,
                &native::Event::TextCommit {
                    text: "你".into(),
                    replacement: None,
                },
            )?
            .unwrap();
        let confirmed = bridge
            .event(
                &committed,
                &native::Event::TextKey(native::TextKeyEvent::Finished {
                    context: key,
                    input_context_handled: Some(true),
                    disposition: native::TextKeyDisposition::TextDispatched,
                    text_callbacks: 1,
                    command_callbacks: 0,
                }),
            )?
            .unwrap();
        ensure!(
            !confirmed.reply().composing && confirmed.text_input_state().unwrap().text == "A你B"
        );
        ensure!(
            confirmed
                .reply()
                .key
                .as_ref()
                .is_some_and(|k| k.disposition == wire::KeyDisposition::InputConsumed),
            "candidate confirmation misclassified as submit"
        );
        let text = confirmed.text_input_state().unwrap();
        let child = confirmed.reply().text.as_ref().unwrap();
        ensure!((text.caret_origin.x - f64::from(child.caret[0]) - 100.).abs() < 0.001);
        ensure!((text.caret_origin.y - f64::from(child.caret[1]) - 60.).abs() < 0.001);
        let rect = text
            .first_rect_for_range(text.caret_index..text.caret_index)
            .ok_or_else(|| anyhow::anyhow!("missing native candidate geometry"))?;
        let moved_frame = frame.offset(20., 10.);
        let moved = ProcessContentBridge::new(
            p,
            &options,
            2,
            moved_frame,
            moved_frame,
            Duration::from_secs(5),
        )?;
        ensure!(
            moved
                .event(
                    &confirmed,
                    &native::Event::TextCommit {
                        text: "BAD".into(),
                        replacement: None
                    }
                )
                .is_err(),
            "old allocation callback accepted"
        );
        let moved_snapshot = moved.snapshot()?;
        ensure!(
            (moved_snapshot.text_input_state().unwrap().caret_origin.x - text.caret_origin.x - 20.)
                .abs()
                < 0.001
        );
        let other_snapshot = bridge.focus(&confirmed, Some(other), true)?;
        ensure!(
            bridge
                .event(
                    &confirmed,
                    &native::Event::TextCommit {
                        text: "LATE".into(),
                        replacement: None
                    }
                )
                .is_err(),
            "stale native owner retargeted"
        );
        let edited = bridge.focus(&other_snapshot, Some(edit), true)?;
        let unicode = bridge
            .event(
                &edited,
                &native::Event::TextCommit {
                    text: "😀".into(),
                    replacement: Some(1..2),
                },
            )?
            .unwrap();
        ensure!(
            unicode.text_input_state().unwrap().text == "A😀B"
                && unicode.text_input_state().unwrap().selection == (3..3)
        );
        layout(p, 3, 3, 210)?;
        ensure!(
            bridge.event(&unicode, &native::Event::TextUnmark).is_err(),
            "old viewport callback accepted"
        );
        let resized_frame = Rect::new(100., 60., 210., 300.);
        let resized_bridge = ProcessContentBridge::new(
            p,
            &options,
            3,
            resized_frame,
            resized_frame,
            Duration::from_secs(5),
        )?;
        let resized = resized_bridge.snapshot()?;
        ensure!(resized.text_input_state().unwrap().text == "A😀B");
        p.request(RequestBody::Message {
            topic: "remount".into(),
            payload: serde_json::Value::Null,
        })?;
        ensure!(
            resized_bridge
                .event(
                    &resized,
                    &native::Event::TextCommit {
                        text: "LATE".into(),
                        replacement: None
                    }
                )
                .is_err(),
            "unmounted editor callback accepted"
        );
        layout(p, 4, 4, 210)?;
        let final_bridge = ProcessContentBridge::new(
            p,
            &options,
            4,
            resized_frame,
            resized_frame,
            Duration::from_secs(5),
        )?;
        let final_snapshot = final_bridge.snapshot()?;
        let replacement = *final_snapshot
            .reply()
            .focus_targets
            .iter()
            .find(|t| t.id == EDIT.0)
            .unwrap();
        ensure!(replacement.mount != edit.mount);
        let replacement = final_bridge.focus(&final_snapshot, Some(replacement), true)?;
        ensure!(replacement.text_input_state().unwrap().text == "replacement");
        ensure!(
            final_bridge
                .event(&replacement, &native::Event::Redraw)?
                .is_none()
        );
        Ok(
            serde_json::json!({"native_text_bridge":"passed","session":p.session(),"view":view,
            "parent_candidate_origin":[rect.1.x,rect.1.y],"replacement_mount":replacement.reply().focused,
            "scope":"real platform callback values over3child IPC; original CoreText query geometry, no real window/physical IME/AX/scroll/visual claim"}),
        )
    }
    fn inspect_pointer(p: &ProcessComponent) -> Result<serde_json::Value> {
        // This consumer owns its service drain. Content round trips themselves
        // still consume only their own correlated reply (verified above).
        process_message(p, "inspect")
    }
    fn process_message(p: &ProcessComponent, topic: &str) -> Result<serde_json::Value> {
        let id = p.request(RequestBody::Message {
            topic: topic.into(),
            payload: serde_json::Value::Null,
        })?;
        let mut result = None;
        wait(|| {
            for packet in p.drain_events() {
                if let Output::Response {
                    request,
                    result: value,
                } = packet.output
                    && request == id
                {
                    result = Some(value);
                }
            }
            result.is_some()
        })?;
        result.unwrap().map_err(anyhow::Error::msg)
    }
    fn pointer_roundtrip(p: &ProcessComponent, view: u64) -> Result<serde_json::Value> {
        use tekes_ui_module_runtime::{NativeViewOptions, ProcessContentBridge};
        use tekes_ui_platform_macos as n;
        let options = NativeViewOptions {
            window: 7,
            view,
            required: CAPS,
        };
        let frame = Rect::new(100., 60., 210., 300.);
        let bridge =
            ProcessContentBridge::new(p, &options, 4, frame, frame, Duration::from_secs(5))?;
        let snapshot = bridge.snapshot()?;
        let edit = snapshot.reply().focused.unwrap();
        ensure!(edit.id == EDIT.0);
        let text = snapshot.text_input_state().unwrap();
        let length = text.text.encode_utf16().count();
        let start = text.first_rect_for_range(0..0).unwrap();
        let end = text.first_rect_for_range(length..length).unwrap();
        let mouse = |point: n::Point, kind: n::MouseEventKind| {
            n::Event::Mouse(n::MouseEvent {
                position: point,
                kind,
                modifiers: n::Modifiers::NONE,
            })
        };
        let begin = n::Point {
            x: start.1.x,
            y: start.1.y + start.2.height / 2.,
        };
        let finish = n::Point {
            x: end.1.x,
            y: end.1.y + end.2.height / 2.,
        };
        let down = bridge
            .event(
                &snapshot,
                &mouse(
                    begin,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            down.reply().consumed
                && down.reply().capture_pointer
                && down.reply().captured.map(|c| c.target) == Some(edit)
        );
        let drag = bridge
            .event(
                &down,
                &mouse(
                    finish,
                    n::MouseEventKind::Dragged {
                        button: n::MouseButton::Left,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            drag.reply().captured.map(|c| c.target) == Some(edit)
                && drag.text_input_state().unwrap().selection == (0..length)
        );
        let released = bridge
            .event(
                &drag,
                &mouse(
                    finish,
                    n::MouseEventKind::Up {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            released.reply().captured.is_none()
                && released.text_input_state().unwrap().selection == (0..length)
        );
        let outside = n::Point {
            x: f64::from(frame.max_x()) + 20.,
            y: f64::from(frame.max_y()) + 20.,
        };
        let miss = bridge
            .event(
                &released,
                &mouse(
                    outside,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            !miss.reply().consumed && miss.reply().bubbles && miss.reply().captured.is_none(),
            "blank outside became an opaque sink"
        );
        let info = inspect_pointer(p)?;
        let f = info["aux_frame"].as_array().unwrap();
        let aux = n::Point {
            x: frame.min_x() as f64 + f[0].as_f64().unwrap() + f[2].as_f64().unwrap() / 2.,
            y: frame.min_y() as f64 + f[1].as_f64().unwrap() + f[3].as_f64().unwrap() / 2.,
        };
        let preedit = bridge
            .event(
                &miss,
                &n::Event::TextPreedit {
                    text: "ni".into(),
                    selection: 2..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: Some(0..0),
                },
            )?
            .unwrap();
        let aux_down = bridge
            .event(
                &preedit,
                &mouse(
                    aux,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            aux_down.reply().captured.is_some_and(|c| c.target.id == 3)
                && aux_down.reply().focused == Some(edit)
                && aux_down.reply().composing
        );
        let aux_up = bridge
            .event(
                &aux_down,
                &mouse(
                    aux,
                    n::MouseEventKind::Up {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            aux_up.reply().activated
                && aux_up.reply().captured.is_none()
                && aux_up.reply().focused == Some(edit)
                && aux_up.text_input_state().unwrap().marked == Some(0..2)
        );
        ensure!(inspect_pointer(p)?["aux_activations"] == 1);
        ensure!(
            bridge
                .event(
                    &aux_down,
                    &mouse(
                        aux,
                        n::MouseEventKind::Up {
                            button: n::MouseButton::Left,
                            click_count: 1
                        }
                    )
                )
                .is_err(),
            "late release reused an old capture"
        );
        let committed = bridge
            .event(
                &aux_up,
                &n::Event::TextCommit {
                    text: "你".into(),
                    replacement: None,
                },
            )?
            .unwrap();
        let right_down = bridge
            .event(
                &committed,
                &mouse(
                    aux,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Right,
                        click_count: 2,
                    },
                ),
            )?
            .unwrap();
        let right_up = bridge
            .event(
                &right_down,
                &mouse(
                    aux,
                    n::MouseEventKind::Up {
                        button: n::MouseButton::Right,
                        click_count: 2,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            !right_up.reply().activated && inspect_pointer(p)?["aux_activations"] == 1,
            "secondary click activated an ordinary action"
        );
        let active = bridge.snapshot()?;
        let t = active.text_input_state().unwrap();
        let caret = t
            .first_rect_for_range(t.caret_index..t.caret_index)
            .unwrap();
        let point = n::Point {
            x: caret.1.x,
            y: caret.1.y + caret.2.height / 2.,
        };
        let held = bridge
            .event(
                &active,
                &mouse(
                    point,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(held.reply().captured.map(|c| c.target) == Some(edit));
        p.request(RequestBody::Message {
            topic: "open_pointer_scope".into(),
            payload: serde_json::Value::Null,
        })?;
        layout(p, 5, 5, 210)?;
        let overlay_bridge =
            ProcessContentBridge::new(p, &options, 5, frame, frame, Duration::from_secs(5))?;
        let overlay = overlay_bridge.snapshot()?;
        ensure!(
            overlay.reply().captured.map(|c| c.target) == Some(edit)
                && !overlay.reply().focus_targets.iter().any(|t| t.id == EDIT.0),
            "captured identity was incorrectly restricted to overlay AX scope"
        );
        let ended = overlay_bridge
            .event(
                &overlay,
                &mouse(
                    outside,
                    n::MouseEventKind::Up {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(ended.reply().captured.is_none() && !ended.reply().activated);
        p.request(RequestBody::Message {
            topic: "close_pointer_scope".into(),
            payload: serde_json::Value::Null,
        })?;
        layout(p, 6, 6, 210)?;
        let fresh =
            ProcessContentBridge::new(p, &options, 6, frame, frame, Duration::from_secs(5))?;
        let restored = fresh.snapshot()?;
        let restored = fresh.focus(&restored, Some(edit), true)?;
        let aux_old = fresh
            .event(
                &restored,
                &mouse(
                    aux,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(aux_old.reply().captured.is_some_and(|c| c.target.id == 3));
        p.request(RequestBody::Message {
            topic: "remount_aux".into(),
            payload: serde_json::Value::Null,
        })?;
        ensure!(
            fresh
                .event(
                    &aux_old,
                    &mouse(
                        aux,
                        n::MouseEventKind::Up {
                            button: n::MouseButton::Left,
                            click_count: 1
                        }
                    )
                )
                .is_err(),
            "old capture retargeted after replacement"
        );
        layout(p, 7, 7, 210)?;
        let final_bridge =
            ProcessContentBridge::new(p, &options, 7, frame, frame, Duration::from_secs(5))?;
        let next = final_bridge.snapshot()?;
        ensure!(inspect_pointer(p)?["aux_activations"] == 0);
        let info = inspect_pointer(p)?;
        let f = info["aux_frame"].as_array().unwrap();
        let new_aux = n::Point {
            x: 100. + f[0].as_f64().unwrap() + f[2].as_f64().unwrap() / 2.,
            y: 60. + f[1].as_f64().unwrap() + f[3].as_f64().unwrap() / 2.,
        };
        let pressed = final_bridge
            .event(
                &next,
                &mouse(
                    new_aux,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(pressed.reply().captured.is_some_and(|c| c.target.id == 3));
        let cancelled = final_bridge.request(&pressed, wire::Input::Cancel)?;
        ensure!(cancelled.reply().captured.is_none());
        ensure!(
            cancelled.reply().changed && inspect_pointer(p)?["aux_pressed"] == false,
            "cancel did not revoke pressed paint/invalidation"
        );
        ensure!(
            final_bridge
                .event(
                    &pressed,
                    &mouse(
                        new_aux,
                        n::MouseEventKind::Up {
                            button: n::MouseButton::Left,
                            click_count: 1
                        }
                    )
                )
                .is_err()
        );
        ensure!(inspect_pointer(p)?["aux_activations"] == 0);
        let rearmed = final_bridge
            .event(
                &cancelled,
                &mouse(
                    new_aux,
                    n::MouseEventKind::Down {
                        button: n::MouseButton::Left,
                        click_count: 1,
                    },
                ),
            )?
            .unwrap();
        ensure!(
            final_bridge
                .event(
                    &pressed,
                    &mouse(
                        new_aux,
                        n::MouseEventKind::Up {
                            button: n::MouseButton::Left,
                            click_count: 1
                        }
                    )
                )
                .is_err(),
            "old release activated a new gesture on the same mounted button"
        );
        let _ = final_bridge.request(&rearmed, wire::Input::Cancel)?;
        ensure!(inspect_pointer(p)?["aux_activations"] == 0);
        let current = final_bridge.snapshot()?;
        ensure!(
            final_bridge
                .request(
                    &current,
                    wire::Input::Pointer(wire::PointerEvent {
                        position: [f32::NAN, 0.],
                        phase: wire::PointerPhase::Move,
                        button: None,
                        modifiers: wire::Modifiers::default(),
                        click_count: 0
                    })
                )
                .is_err()
        );
        ensure!(
            matches!(p.status(), ProcessStatus::Ready { .. })
                && final_bridge.snapshot()?.reply().captured.is_none(),
            "invalid pointer terminated the worker"
        );
        Ok(
            serde_json::json!({"pointer":"passed","capture_distinct_from_focus":true,"aux_preedit_preserved":true,
            "existing_capture_survives_overlay_scope":true,"stale_capture_and_cancel_rejected":true,"no_opaque_sink":true,
            "scope":"native MouseEvent values / original child pointer path over real IPC; not physical mouse or automatic parent capture/focus integration"}),
        )
    }
    fn inspect_container(
        c: &tekes_ui_module_runtime::ProcessContainer,
    ) -> Result<serde_json::Value> {
        container_message(c, "inspect")
    }
    fn container_message(
        c: &tekes_ui_module_runtime::ProcessContainer,
        topic: &str,
    ) -> Result<serde_json::Value> {
        let id = c.request(RequestBody::Message {
            topic: topic.into(),
            payload: serde_json::Value::Null,
        })?;
        let mut result = None;
        wait(|| {
            for packet in c.drain_events() {
                if let Output::Response {
                    request,
                    result: value,
                } = packet.output
                    && request == id
                {
                    result = Some(value);
                }
            }
            result.is_some()
        })?;
        result.unwrap().map_err(anyhow::Error::msg)
    }
    fn mounted_container_roundtrip() -> Result<serde_json::Value> {
        use tekes_ui_module_runtime::ProcessContainer;
        let ids = [ComponentId(101), ComponentId(102), ComponentId(103)];
        let mut children = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            let container = ProcessContainer::new_view(
                ProcessComponentOptions::new(MODULE, 51).with_view(7, 30 + i as u64, CAPS),
                || {},
            )?;
            ensure!(
                container
                    .native_content_bridge(Duration::from_secs(1))
                    .is_err(),
                "unmounted bridge available"
            );
            children.push(Content::new(*id, container));
        }
        let mut host = ContentHost::new(Content::new(
            ComponentId(100),
            GridContent {
                model: tekes_ui_controls::extended::Grid::new(3),
                children,
            },
        ))?;
        let mut renderer = tekes_ui_render::Renderer::new_headless()?;
        let frame = Rect::new(10., 20., 900., 300.);
        fn tick(
            host: &mut ContentHost,
            renderer: &mut tekes_ui_render::Renderer,
            ids: &[ComponentId],
            frame: Rect,
        ) -> Result<bool> {
            tick_clipped(host, renderer, ids, frame, frame)
        }
        fn tick_clipped(
            host: &mut ContentHost,
            renderer: &mut tekes_ui_render::Renderer,
            ids: &[ComponentId],
            frame: Rect,
            clip: Rect,
        ) -> Result<bool> {
            for id in ids {
                host.get_mut(*id)
                    .unwrap()
                    .component_mut::<ProcessContainer>()
                    .unwrap()
                    .prepare(renderer)?;
            }
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                clip,
                Theme::LIGHT.into(),
            )?;
            Ok(ids.iter().all(|id| {
                host.get(*id)
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap()
                    .input_ready()
            }))
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, frame)? {
            ensure!(
                Instant::now() < deadline,
                "container geometry acknowledgement deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        host.set_action_events(true);
        let mut result = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            ensure!(
                host.focus(Some(*id), true),
                "parent could not focus original container"
            );
            let node = host.get(*id).unwrap();
            let bounds = node.frame();
            let container = node.component::<ProcessContainer>().unwrap();
            let pid = match container.status().unwrap() {
                ProcessStatus::Ready { pid, .. } => pid,
                _ => anyhow::bail!("container not ready"),
            };
            let session = container.session().cloned();
            // Read-only query only. All actual input below enters the one
            // original parent ContentHost; there is no per-container forwarding.
            let initial = container
                .native_content_bridge(Duration::from_secs(5))?
                .snapshot()?;
            let text = initial.text_input_state().unwrap();
            let point = tekes_ui_scene::Point::new(
                text.caret_origin.x as f32,
                (text.caret_origin.y + text.caret_size.height / 2.) as f32,
            );
            let down = host.event(&ContentEvent::PointerDown(point));
            ensure!(
                down.consumed && down.capture_pointer,
                "parent pointer failed to acquire child capture"
            );
            ensure!(
                host.pointer_focus_requested(),
                "accepted child pointer focus was not published"
            );
            ensure!(
                host.pointer_capture().is_some_and(|c| c.0 == *id),
                "original parent capture missing"
            );
            // Repaint while held must not change viewport, lose input readiness,
            // or cancel the original captured gesture.
            tick(&mut host, &mut renderer, &ids, frame)?;
            let drag = host.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
                frame.max_x() + 20.,
                point.y,
            )));
            ensure!(
                drag.capture_pointer,
                "capture lost while crossing sibling containers"
            );
            let up = host.event(&ContentEvent::PointerUp(point));
            ensure!(
                up.consumed && host.pointer_capture().is_none(),
                "parent capture release failed"
            );
            host.event(&ContentEvent::Commit {
                text: format!("mount-{i}"),
                replacement: Some(0..4),
            });
            let container = host
                .get(*id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(
                container.error().is_none(),
                "component input error: {:?}",
                container.error()
            );
            let changed = container
                .native_content_bridge(Duration::from_secs(5))?
                .snapshot()?;
            ensure!(
                changed.text_input_state().unwrap().text == format!("mount-{i}"),
                "component text delivery missing"
            );
            // An ordinary key uses the child's existing caret state machine.
            host.event(&ContentEvent::KeyInput(ContentKeyEvent {
                key: Some(tekes_ui_controls::ControlKey::Left),
                down: true,
                repeat: false,
                modifiers: InputModifiers {
                    command: true,
                    ..Default::default()
                },
                native_code: 123,
                characters: String::new(),
                characters_ignoring_modifiers: String::new(),
            }));
            let state = host
                .get(*id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap()
                .native_content_bridge(Duration::from_secs(5))?
                .snapshot()?;
            ensure!(
                state.text_input_state().unwrap().caret_index == 0,
                "original child Command+Left key not routed"
            );
            ensure!(
                host.drain_actions().is_empty(),
                "child activation emitted a duplicate parent business action"
            );
            result.push(serde_json::json!({"pid":pid,"session":session,"frame":[bounds.min_x(),bounds.min_y(),bounds.size.width,bounds.size.height],"text":changed.text_input_state().unwrap().text,"parent_component_input":true}));
        }
        // Native first Tab has no prior editor/button ownership. It enters the
        // original tree rather than inventing a focused editor in the consumer.
        let entry_tab = |id| ContentEvent::TextKeyStart {
            id,
            composing: false,
            key: ContentKeyEvent {
                key: Some(tekes_ui_controls::ControlKey::Tab { reverse: false }),
                down: true,
                repeat: false,
                modifiers: Default::default(),
                native_code: 48,
                characters: "\t".into(),
                characters_ignoring_modifiers: "\t".into(),
            },
        };
        host.focus(None, false);
        host.event(&entry_tab(998));
        host.event(&ContentEvent::TextCommand("insertTab:".into()));
        ensure!(
            host.event(&ContentEvent::TextKeyEnd {
                id: 998,
                input_context_consumed: false
            })
            .text_key
            .is_some_and(|key| key.disposition == TextKeyDisposition::Navigated)
        );
        ensure!(host.focused() == Some(ids[0]));
        container_message(
            host.get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap(),
            "clear_focus",
        )?;
        host.event(&entry_tab(999));
        host.event(&ContentEvent::TextCommand("insertTab:".into()));
        ensure!(
            host.event(&ContentEvent::TextKeyEnd {
                id: 999,
                input_context_consumed: false
            })
            .text_key
            .is_some_and(|key| key.disposition == TextKeyDisposition::Navigated)
        );
        ensure!(
            host.focused() == Some(ids[0])
                && inspect_container(
                    host.get(ids[0])
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap()
                )?["focused"]
                    == EDIT.0
        );
        ensure!(
            host.drain_actions().is_empty(),
            "first Tab activated a business control"
        );
        // Native window adapter uses one original Host and one query entry.
        // There is deliberately no concrete parent TextSurface for these editors.
        host.enter_focus(false);
        ensure!(
            host.focused_text().is_none() && host.text_input_snapshot().is_some(),
            "remote text query requires a fake local surface"
        );
        let query = tekes_ui_module_runtime::content_text_input_state(&host).unwrap();
        ensure!(query.text == "mount-0" && query.caret_size.height > 0.);
        let start = |id, composing| {
            tekes_ui_platform_macos::Event::TextKey(tekes_ui_platform_macos::TextKeyEvent::Started(
                tekes_ui_platform_macos::TextKeyContext {
                    id,
                    key: tekes_ui_platform_macos::KeyEvent {
                        kind: tekes_ui_platform_macos::KeyEventKind::Down,
                        key_code: 36,
                        characters: "\r".into(),
                        characters_ignoring_modifiers: "\r".into(),
                        modifiers: tekes_ui_platform_macos::Modifiers::NONE,
                        is_repeat: false,
                    },
                    composing_at_start: composing,
                },
            ))
        };
        let finish = |id, handled| ContentEvent::TextKeyEnd {
            id,
            input_context_consumed: handled,
        };
        host.event(&tekes_ui_module_runtime::content_event(&start(1001, false)).unwrap());
        host.event(&ContentEvent::TextCommand("insertNewline:".into()));
        let submitted = host.event(&finish(1001, false));
        ensure!(
            submitted
                .text_key
                .is_some_and(|k| k.disposition == TextKeyDisposition::Submitted)
                && !submitted.activated,
            "child submit acknowledgement was lost or echoed"
        );
        ensure!(
            inspect_container(
                host.get(ids[0])
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap()
            )?["submits"]
                == 1
        );
        ensure!(
            host.drain_actions().is_empty(),
            "native submit duplicated parent business action"
        );
        // The explicit wire completion query cannot replay a finished action.
        {
            let bridge = host
                .get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap()
                .native_content_bridge(Duration::from_secs(5))?;
            let current = bridge.snapshot()?;
            let retired = bridge.request(&current, wire::Input::TextKeyResult { id: 1001 })?;
            ensure!(
                retired
                    .reply()
                    .key
                    .as_ref()
                    .is_some_and(|key| key.disposition == wire::KeyDisposition::Cancelled)
            );
        }
        // Candidate-confirming Enter is not a second submit.
        host.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: Some(0..0),
        });
        host.event(&tekes_ui_module_runtime::content_event(&start(1002, true)).unwrap());
        host.event(&ContentEvent::Commit {
            text: "你".into(),
            replacement: None,
        });
        host.event(&ContentEvent::TextCommand("insertNewline:".into()));
        let confirmed = host.event(&finish(1002, true));
        ensure!(
            confirmed
                .text_key
                .is_some_and(|k| k.disposition == TextKeyDisposition::InputConsumed)
        );
        ensure!(
            inspect_container(
                host.get(ids[0])
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap()
            )?["submits"]
                == 1
        );
        host.event(&ContentEvent::Commit {
            text: String::new(),
            replacement: Some(0..1),
        });
        // Parent native provenance rejects a same-component inner blur/refocus.
        host.event(&tekes_ui_module_runtime::content_event(&start(1003, false)).unwrap());
        container_message(
            host.get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap(),
            "refocus_same_editor",
        )?;
        host.event(&ContentEvent::Commit {
            text: "PARENT-LATE".into(),
            replacement: None,
        });
        let cancelled = host.event(&finish(1003, false));
        ensure!(
            cancelled
                .text_key
                .is_some_and(|k| k.disposition == TextKeyDisposition::Cancelled)
        );
        ensure!(
            tekes_ui_module_runtime::content_text_input_state(&host)
                .unwrap()
                .text
                == "mount-0"
        );
        // Observe current original identity without migrating a stale input.
        host.focus(Some(ids[0]), true);
        // The prior auxiliary scene below expects the last container focused.
        host.enter_focus(true);
        // Auxiliary controls in a DIFFERENT process preserve the original
        // composing parent's focus. They capture without becoming a text owner.
        host.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: Some(0..0),
        });
        let source = *ids.last().unwrap();
        let aux_container = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let info = inspect_container(aux_container)?;
        let aux = info["aux_frame"].as_array().unwrap();
        let bounds = host.get(ids[0]).unwrap().frame();
        let aux_point = tekes_ui_scene::Point::new(
            bounds.min_x() + aux[0].as_f64().unwrap() as f32 + 5.,
            bounds.min_y() + aux[1].as_f64().unwrap() as f32 + 5.,
        );
        ensure!(
            host.event(&ContentEvent::PointerDown(aux_point))
                .capture_pointer
        );
        ensure!(
            host.focused() == Some(source),
            "auxiliary press stole composing parent focus"
        );
        ensure!(
            !host.pointer_focus_requested(),
            "preserving-focus auxiliary published activation focus"
        );
        ensure!(host.pointer_capture().is_some_and(|c| c.0 == ids[0]));
        host.event(&ContentEvent::PointerUp(aux_point));
        let source_state = host
            .get(source)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            source_state.reply().composing,
            "auxiliary click ended another container's composition"
        );
        let info = inspect_container(
            host.get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap(),
        )?;
        ensure!(info["aux_activations"] == 1 && info["aux_pressed"] == false);
        ensure!(
            host.drain_actions().is_empty(),
            "duplicate auxiliary action escaped child"
        );
        host.event(&ContentEvent::Commit {
            text: String::new(),
            replacement: Some(0..2),
        });
        host.event(&ContentEvent::PointerDown(aux_point));
        ensure!(host.pointer_capture().is_some());
        host.event(&ContentEvent::Cancel);
        ensure!(host.pointer_capture().is_none());
        host.event(&ContentEvent::PointerUp(aux_point));
        let info = inspect_container(
            host.get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap(),
        )?;
        ensure!(
            info["aux_activations"] == 1 && info["aux_pressed"] == false,
            "cancelled release activated auxiliary: {info}"
        );
        // Ordinary parent Tab traverses original child targets, then transfers
        // to another container through the existing outer focus traversal.
        host.focus(Some(ids[0]), true);
        for _ in 0..3 {
            host.event(&ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Tab { reverse: false },
                down: true,
            });
        }
        ensure!(
            host.focused() == Some(ids[1]),
            "Tab trapped in the child instead of traversing parent"
        );
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Tab { reverse: true },
            down: true,
        });
        ensure!(
            host.focused() == Some(ids[0]),
            "reverse boundary did not return to previous container"
        );
        let reverse_entry = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            reverse_entry
                .reply()
                .focused
                .is_some_and(|target| target.id == OTHER.0),
            "reverse traversal entered first child instead of last"
        );
        // Native selectors must cross the same boundary without replaying keys.
        host.enter_focus(false);
        for key_id in 1004..1007 {
            let mut native = start(key_id, false);
            if let tekes_ui_platform_macos::Event::TextKey(
                tekes_ui_platform_macos::TextKeyEvent::Started(context),
            ) = &mut native
            {
                context.key.key_code = 48;
                context.key.characters = "\t".into();
                context.key.characters_ignoring_modifiers = "\t".into();
            }
            host.event(&tekes_ui_module_runtime::content_event(&native).unwrap());
            host.event(&ContentEvent::TextCommand("insertTab:".into()));
            host.event(&finish(key_id, false));
        }
        ensure!(
            host.focused() == Some(ids[1]),
            "native Tab trapped at original child boundary"
        );
        host.enter_focus(false);
        host.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: vec![],
            replacement: Some(0..0),
        });
        let moved_frame = Rect::new(40., 45., 720., 280.);
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, moved_frame)? {
            ensure!(Instant::now() < end, "moved composition geometry deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        let moved = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            moved.reply().composing && moved.reply().focused.is_some_and(|t| t.id == EDIT.0),
            "geometry cancelled original composition"
        );
        let ancestor_clip = Rect::new(70., 55., 650., 220.);
        let end = Instant::now() + Duration::from_secs(15);
        while !tick_clipped(&mut host, &mut renderer, &ids, moved_frame, ancestor_clip)? {
            ensure!(Instant::now() < end, "clipped composition deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        let clipped = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            clipped.reply().composing,
            "ancestor clip change ended composition"
        );
        ensure!(
            !clipped.text_input_state().unwrap().carets.is_empty(),
            "clip assertion has no visible caret samples"
        );
        for (_, point, size) in &clipped.text_input_state().unwrap().carets {
            ensure!(
                point.x >= f64::from(ancestor_clip.min_x())
                    && point.y >= f64::from(ancestor_clip.min_y())
                    && point.x + size.width <= f64::from(ancestor_clip.max_x()) + 0.001
                    && point.y + size.height <= f64::from(ancestor_clip.max_y()) + 0.001,
                "native caret escaped actual ancestor clip"
            );
        }
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, moved_frame)? {
            ensure!(Instant::now() < end, "clip restoration deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        let before_scale = moved.text_input_state().unwrap().clone();
        host.get_mut(ids[0])
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .set_scale(2.)?;
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, moved_frame)? {
            ensure!(Instant::now() < end, "scaled composition deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        let scaled = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            scaled.reply().composing,
            "backing-scale change ended composition"
        );
        let after_scale = scaled.text_input_state().unwrap();
        ensure!(
            (after_scale.caret_origin.x - before_scale.caret_origin.x).abs() < 0.001
                && (after_scale.caret_size.height - before_scale.caret_size.height).abs() < 0.001,
            "backing scale applied twice to native geometry"
        );
        host.event(&tekes_ui_module_runtime::content_event(&start(1014, true)).unwrap());
        let reflow_binding = host.native_key_binding();
        let pending_frame = Rect::new(70., 50., 660., 260.);
        ensure!(
            !tick(&mut host, &mut renderer, &ids, pending_frame)?,
            "new allocation unexpectedly ready without frame import"
        );
        ensure!(
            host.is_composing(),
            "pending layout lost parent composition barrier"
        );
        let pending_text = tekes_ui_module_runtime::content_text_input_state(&host)
            .context("pending geometry lost native text owner")?;
        ensure!(pending_text.marked.is_some() && pending_text.caret_size.height == 0.);
        ensure!(
            host.native_key_binding() == reflow_binding,
            "geometry alone replaced original native key owner"
        );
        let c = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        ensure!(
            inspect_container(c)?["composing"] == true,
            "pending layout ended child composition"
        );
        for value in ["你", "."] {
            ensure!(
                host.event(&ContentEvent::Commit {
                    text: value.into(),
                    replacement: None
                })
                .consumed,
                "pending text callback not retained"
            );
        }
        let c = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        ensure!(
            inspect_container(c)?["edit_text"] == "nimount-0",
            "deferred text applied before geometry acknowledgement"
        );
        let queued_end = host.event(&finish(1014, true));
        ensure!(
            queued_end
                .text_key
                .is_some_and(|k| k.disposition == TextKeyDisposition::Deferred)
        );
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, pending_frame)? {
            ensure!(Instant::now() < end, "deferred composition deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        ensure!(
            host.text_key_result(1014)
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::InputConsumed)
        );
        ensure!(
            host.text_key_result(1014)
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Cancelled),
            "deferred key result replayed"
        );
        let queued = host
            .get(ids[0])
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            !queued.reply().composing && queued.text_input_state().unwrap().text == "你.mount-0",
            "deferred text order/value changed"
        );
        let length = queued
            .text_input_state()
            .unwrap()
            .text
            .encode_utf16()
            .count();
        host.event(&ContentEvent::Commit {
            text: "mount-0".into(),
            replacement: Some(0..length),
        });
        host.event(&tekes_ui_module_runtime::content_event(&start(1015, false)).unwrap());
        let stale_frame = Rect::new(90., 60., 630., 250.);
        ensure!(!tick(&mut host, &mut renderer, &ids, stale_frame)?);
        host.event(&ContentEvent::Commit {
            text: "STALE-QUEUED".into(),
            replacement: None,
        });
        container_message(
            host.get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap(),
            "refocus_same_editor",
        )?;
        host.event(&finish(1015, false));
        let end = Instant::now() + Duration::from_secs(15);
        let rejection = loop {
            match tick(&mut host, &mut renderer, &ids, stale_frame) {
                Err(error) => break error.to_string(),
                Ok(_) => {
                    ensure!(
                        Instant::now() < end,
                        "stale deferred callback was not rejected"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        };
        ensure!(
            rejection.contains("lease") || rejection.contains("stale deferred"),
            "unexpected deferred error: {rejection}"
        );
        ensure!(
            inspect_container(
                host.get(ids[0])
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap()
            )?["edit_text"]
                == "mount-0",
            "old deferred callback edited refocused owner"
        );
        host.focus(Some(ids[0]), true);
        host.event(&ContentEvent::Preedit {
            text: "x".into(),
            selection: 1..1,
            clauses: vec![],
            replacement: Some(0..0),
        });
        ensure!(
            host.text_input_snapshot()
                .is_some_and(|s| s.marked.is_some()),
            "rejected queued edit stranded the original native key frame"
        );
        host.event(&ContentEvent::Commit {
            text: String::new(),
            replacement: Some(0..1),
        });
        let native_tab = |id| {
            let mut event = start(id, false);
            if let tekes_ui_platform_macos::Event::TextKey(
                tekes_ui_platform_macos::TextKeyEvent::Started(context),
            ) = &mut event
            {
                context.key.key_code = 48;
                context.key.characters = "\t".into();
                context.key.characters_ignoring_modifiers = "\t".into();
            }
            event
        };
        host.enter_focus(false);
        host.event(&tekes_ui_module_runtime::content_event(&native_tab(1016)).unwrap());
        let inner_frame = Rect::new(99., 69., 615., 246.);
        ensure!(!tick(&mut host, &mut renderer, &ids, inner_frame)?);
        host.event(&ContentEvent::TextCommand("insertTab:".into()));
        ensure!(
            host.event(&finish(1016, false))
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Deferred)
        );
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, inner_frame)? {
            ensure!(
                Instant::now() < end,
                "pending inner native boundary deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        ensure!(
            host.focused() == Some(ids[0])
                && inspect_container(
                    host.get(ids[0])
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap()
                )?["focused"]
                    == 3,
            "original deferred child navigation moved the wrong parent"
        );
        ensure!(
            host.text_key_result(1016)
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Navigated),
            "child's own navigation invalidated its original receipt"
        );
        host.enter_focus(false);
        for _ in 0..2 {
            host.event(&ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Tab { reverse: false },
                down: true,
            });
        }
        host.event(&tekes_ui_module_runtime::content_event(&native_tab(1017)).unwrap());
        let pending_tab_frame = Rect::new(100., 70., 610., 245.);
        ensure!(!tick(&mut host, &mut renderer, &ids, pending_tab_frame)?);
        host.event(&ContentEvent::TextCommand("insertTab:".into()));
        ensure!(
            host.event(&finish(1017, false))
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Deferred)
        );
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, pending_tab_frame)? {
            ensure!(Instant::now() < end, "pending native boundary deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        ensure!(
            host.focused() == Some(ids[1]),
            "geometry-pending native Tab lost original outer continuation"
        );
        ensure!(
            host.text_key_result(1017)
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Navigated)
        );
        ensure!(
            host.text_key_result(1017)
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Cancelled),
            "outer navigation acknowledged twice"
        );
        // The existing one-packet transport budget also revokes native spans
        // on exhaustion, rather than retaining a frame whose End cannot queue.
        host.enter_focus(false);
        host.event(&tekes_ui_module_runtime::content_event(&start(1018, false)).unwrap());
        let overflow_frame = Rect::new(101., 71., 605., 240.);
        ensure!(!tick(&mut host, &mut renderer, &ids, overflow_frame)?);
        host.event(&ContentEvent::Commit {
            text: "x".repeat(1_048_576),
            replacement: None,
        });
        ensure!(
            host.get(ids[0])
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap()
                .error()
                .is_some_and(|error| error.contains("transport budget"))
        );
        ensure!(
            host.event(&finish(1018, false))
                .text_key
                .is_some_and(|key| key.disposition == TextKeyDisposition::Cancelled)
        );
        let end = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, overflow_frame)? {
            ensure!(
                Instant::now() < end,
                "overflow cancellation acknowledgement deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        host.enter_focus(false);
        host.event(&ContentEvent::Preedit {
            text: "x".into(),
            selection: 1..1,
            clauses: vec![],
            replacement: Some(0..0),
        });
        ensure!(
            host.text_input_snapshot()
                .is_some_and(|query| query.marked.is_some()),
            "transport exhaustion stranded an original key frame"
        );
        host.event(&ContentEvent::Commit {
            text: String::new(),
            replacement: Some(0..1),
        });
        ensure!(
            inspect_container(
                host.get(ids[0])
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap()
            )?["edit_text"]
                == "mount-0"
        );
        let first = ids[0];
        host.get_mut(first)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .set_visible(false)?;
        ensure!(
            host.get(first)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap()
                .native_content_bridge(Duration::from_secs(1))
                .is_err(),
            "hidden bridge available"
        );
        host.get_mut(first)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .set_visible(true)?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while !tick(&mut host, &mut renderer, &ids, frame)? {
            ensure!(
                Instant::now() < deadline,
                "reshown container geometry deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let bridge = host
            .get(first)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .native_content_bridge(Duration::from_secs(5))?;
        let shown = bridge.snapshot()?;
        ensure!(
            shown.reply().focused.is_none() && shown.reply().captured.is_none(),
            "hide/show retained active input ownership"
        );
        let edit = *shown
            .reply()
            .focus_targets
            .iter()
            .find(|t| t.id == EDIT.0)
            .unwrap();
        let shown = bridge.focus(&shown, Some(edit), true)?;
        ensure!(
            shown.text_input_state().unwrap().text == "mount-0",
            "hide/show lost original editor state"
        );
        for id in ids {
            let removed = host.remove(id)?;
            ensure!(
                removed
                    .component::<ProcessContainer>()
                    .unwrap()
                    .session()
                    .is_none(),
                "unmount retained process"
            );
        }
        renderer.collect_retired_images();
        Ok(
            serde_json::json!({"result":"THREE_MOUNTED_CONTAINERS_OK","runs":result,"hidden_bridge_rejected":true,"hide_show_text_preserved":true,"unmounted":true,"geometry_composition_preserved":true,"ancestor_clip_preserved":true,"backing_scale_logical_geometry":true,"native_parent_query":true,"native_no_owner_entry_parent_and_child":true,"native_submit_once":true,"candidate_enter_no_submit":true,"native_tab_outer_boundary":true,"deferred_native_tab_inner_and_outer":true,"deferred_result_once":true,"transport_exhaustion_cancels_original_key":true,"pending_query_owner_retained":true,"rejected_edit_finishes_old_key":true,"queued_text_order_preserved":true,"stale_queued_lease_rejected":true,"scope":"original parent ContentHost automatically routes pointer/capture, focus, control keys and text callbacks across three ProcessContainers; no native window/glyph/AX/scroll or physical IME acceptance"}),
        )
    }
    fn toolbar_focus_roundtrip() -> Result<serde_json::Value> {
        use tekes_ui_module_runtime::ProcessContainer;
        let owner = ComponentId(202);
        let mut host = ContentHost::new(Content::new(
            ComponentId(200),
            GridContent {
                model: tekes_ui_controls::extended::Grid::new(3),
                children: vec![
                    Content::new(ComponentId(201), Button::new("Before")),
                    Content::new(
                        owner,
                        ProcessContainer::new_view(
                            ProcessComponentOptions::new(TOOLBAR_MODULE, 61).with_view(
                                7,
                                41,
                                ViewCapabilities {
                                    independent_toolbar: true,
                                    ..CAPS
                                },
                            ),
                            || {},
                        )?,
                    ),
                    Content::new(ComponentId(203), Button::new("After")),
                ],
            },
        ))?;
        host.set_action_events(true);
        host.set_focus_wrap(false);
        let mut renderer = tekes_ui_render::Renderer::new_headless()?;
        fn render_ready(
            host: &mut ContentHost,
            renderer: &mut tekes_ui_render::Renderer,
            frame: Rect,
        ) -> Result<()> {
            let end = Instant::now() + Duration::from_secs(15);
            loop {
                let c = host
                    .get_mut(ComponentId(202))
                    .unwrap()
                    .component_mut::<ProcessContainer>()
                    .unwrap();
                c.prepare(renderer)?;
                host.render(
                    &mut Scene::new(),
                    &mut Text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )?;
                let c = host
                    .get(ComponentId(202))
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap();
                ensure!(
                    c.error().is_none(),
                    "toolbar container failed: {:?}",
                    c.error()
                );
                if c.input_ready() && c.local_toolbar_frame().is_some() {
                    return Ok(());
                }
                ensure!(
                    Instant::now() < end,
                    "toolbar container layout acknowledgement deadline"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        fn state(host: &ContentHost) -> Result<serde_json::Value> {
            inspect_container(
                host.get(ComponentId(202))
                    .unwrap()
                    .component::<ProcessContainer>()
                    .unwrap(),
            )
        }
        fn tab(host: &mut ContentHost, reverse: bool) {
            host.event(&ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Tab { reverse },
                down: true,
            });
        }
        render_ready(&mut host, &mut renderer, Rect::new(0., 0., 900., 300.))?;
        let pid = match host
            .get(owner)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .status()
            .unwrap()
        {
            ProcessStatus::Ready { pid, .. } => pid,
            _ => anyhow::bail!("toolbar process not ready"),
        };
        host.enter_focus(false);
        ensure!(host.focused() == Some(ComponentId(201)));
        tab(&mut host, false);
        ensure!(
            host.focused() == Some(owner) && state(&host)?["focused"].is_null(),
            "forward entry skipped toolbar"
        );
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Enter,
            down: true,
        });
        ensure!(
            state(&host)?["toolbar_calls"] == 1,
            "toolbar command not invoked exactly once"
        );
        ensure!(
            host.drain_actions().is_empty(),
            "toolbar invocation also became parent business action"
        );
        for child in [EDIT, ComponentId(3), OTHER] {
            tab(&mut host, false);
            ensure!(
                host.focused() == Some(owner) && state(&host)?["focused"] == child.0,
                "forward toolbar/body traversal wrong"
            );
            if child == EDIT {
                host.event(&ContentEvent::Preedit {
                    text: "ni".into(),
                    selection: 2..2,
                    clauses: vec![],
                    replacement: Some(0..0),
                });
                for reverse in [true, false] {
                    tab(&mut host, reverse);
                    let current = state(&host)?;
                    ensure!(
                        host.focused() == Some(owner)
                            && current["focused"] == EDIT.0
                            && current["composing"] == true,
                        "candidate Tab escaped original editor into toolbar/body traversal"
                    );
                }
                for topic in ["disable_toolbar", "enable_toolbar"] {
                    container_message(
                        host.get(owner)
                            .unwrap()
                            .component::<ProcessContainer>()
                            .unwrap(),
                        topic,
                    )?;
                    render_ready(&mut host, &mut renderer, Rect::new(0., 0., 900., 300.))?;
                    ensure!(
                        state(&host)?["composing"] == true && state(&host)?["focused"] == EDIT.0,
                        "toolbar state update ended body composition"
                    );
                }
                host.event(&ContentEvent::Commit {
                    text: String::new(),
                    replacement: Some(0..2),
                });
            }
        }
        tab(&mut host, false);
        ensure!(
            host.focused() == Some(ComponentId(203)),
            "forward body boundary trapped"
        );
        for child in [OTHER, ComponentId(3), EDIT] {
            tab(&mut host, true);
            ensure!(
                host.focused() == Some(owner) && state(&host)?["focused"] == child.0,
                "reverse body traversal wrong"
            );
        }
        tab(&mut host, true);
        ensure!(
            host.focused() == Some(owner) && state(&host)?["focused"].is_null(),
            "reverse body boundary skipped toolbar"
        );
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Enter,
            down: true,
        });
        ensure!(state(&host)?["toolbar_calls"] == 2);
        tab(&mut host, true);
        ensure!(
            host.focused() == Some(ComponentId(201)),
            "reverse toolbar boundary trapped"
        );
        // Original More/menu retains Tab priority. It must not enter the body
        // until Escape closes the menu through the same ToolbarControl.
        render_ready(&mut host, &mut renderer, Rect::new(0., 0., 90., 300.))?;
        tab(&mut host, false);
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Enter,
            down: true,
        });
        ensure!(
            host.get(owner)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap()
                .overlay_open(),
            "narrow toolbar did not open More"
        );
        tab(&mut host, false);
        ensure!(
            host.focused() == Some(owner) && state(&host)?["focused"].is_null(),
            "More Tab leaked into body"
        );
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Escape,
            down: true,
        });
        ensure!(
            !host
                .get(owner)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap()
                .overlay_open()
        );
        tab(&mut host, false);
        ensure!(state(&host)?["focused"] == EDIT.0);
        ensure!(state(&host)?["toolbar_calls"] == 2);
        host.event(&ContentEvent::TextKeyStart {
            id: 8001,
            key: tekes_ui_controls::composition::ContentKeyEvent {
                key: Some(tekes_ui_controls::ControlKey::Tab { reverse: true }),
                down: true,
                repeat: false,
                modifiers: tekes_ui_controls::composition::InputModifiers {
                    shift: true,
                    ..Default::default()
                },
                native_code: 48,
                characters: "\t".into(),
                characters_ignoring_modifiers: "\t".into(),
            },
            composing: false,
        });
        host.event(&ContentEvent::TextCommand("insertBacktab:".into()));
        let boundary = host.event(&ContentEvent::TextKeyEnd {
            id: 8001,
            input_context_consumed: false,
        });
        ensure!(
            boundary
                .text_key
                .is_some_and(|k| k.disposition
                    == tekes_ui_controls::composition::TextKeyDisposition::Navigated)
                && host.focused() == Some(owner)
                && state(&host)?["focused"].is_null(),
            "native reverse body boundary skipped original local toolbar"
        );
        ensure!(state(&host)?["toolbar_calls"] == 2);
        let removed = host.remove(owner)?;
        ensure!(
            removed
                .component::<ProcessContainer>()
                .unwrap()
                .session()
                .is_none()
        );
        renderer.collect_retired_images();
        Ok(
            serde_json::json!({"result":"TOOLBAR_BODY_TRAVERSAL_OK","pid":pid,"forward_and_reverse":true,"single_toolbar_invocation":true,"more_tab_scope":true,"native_reverse_body_toolbar_boundary":true,"scope":"one original parent ContentHost and real child IPC; no native window or AX/glyph acceptance"}),
        )
    }
    fn read_only_query_wake() -> Result<serde_json::Value> {
        use std::sync::{
            Arc, Condvar, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        let armed = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (notified, notifications) = std::sync::mpsc::channel();
        let (a, c, g) = (armed.clone(), count.clone(), gate.clone());
        let p = ProcessComponent::spawn(
            ProcessComponentOptions::new(MODULE, 91).with_view(7, 61, CAPS),
            move || {
                let _ = notified.send(());
                if a.load(Ordering::SeqCst) {
                    c.fetch_add(1, Ordering::SeqCst);
                    let (lock, changed) = &*g;
                    let mut released = lock.lock().unwrap();
                    while !*released {
                        released = changed.wait(released).unwrap();
                    }
                }
            },
        )?;
        wait(|| matches!(p.status(), ProcessStatus::Ready { .. }))?;
        let pid = match p.status() {
            ProcessStatus::Ready { pid, .. } => pid,
            _ => unreachable!(),
        };
        layout(&p, 1, 1, 300)?;
        let frame = Rect::new(0., 0., 300., 300.);
        let bridge = tekes_ui_module_runtime::ProcessContentBridge::new(
            &p,
            &tekes_ui_module_runtime::NativeViewOptions {
                window: 7,
                view: 61,
                required: CAPS,
            },
            1,
            frame,
            frame,
            Duration::from_secs(1),
        )?;
        ensure!(
            !bridge.snapshot()?.reply().changed,
            "query fixture still invalidated"
        );
        armed.store(true, Ordering::SeqCst);
        // If a read-only RPC invokes the UI wake, this deliberately blocked
        // callback prevents the next RPC completing. No frame-rate/timing claim.
        let result = (|| -> Result<()> {
            ensure!(!bridge.snapshot()?.reply().changed);
            ensure!(!bridge.snapshot()?.reply().changed);
            Ok(())
        })();
        armed.store(false, Ordering::SeqCst);
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        let async_result = if result.is_ok() {
            notifications.try_iter().for_each(drop);
            let id = p.request(RequestBody::Content(wire::Request {
                view: 61,
                viewport: 1,
                owner: None,
                focus_epoch: 0,
                text_binding: None,
                capture: None,
                input: wire::Input::Snapshot,
            }))?;
            (|| -> Result<()> {
                loop {
                    notifications.recv_timeout(Duration::from_secs(2))?;
                    if p.drain_events().iter().any(|packet| {
                        matches!(packet.output,
                        Output::Response {request,..} if request==id)
                    }) {
                        break;
                    }
                }
                Ok(())
            })()
        } else {
            Ok(())
        };
        p.close();
        wait(|| {
            matches!(
                p.status(),
                ProcessStatus::Closed | ProcessStatus::Failed { .. }
            )
        })?;
        ensure!(
            count.load(Ordering::SeqCst) == 0,
            "read-only native query invoked UI wake and blocked its own next RPC"
        );
        result?;
        async_result?;
        Ok(
            serde_json::json!({"result":"READ_ONLY_QUERY_NO_UI_WAKE","pid":pid,
            "query_host_wakes":0,"explicit_async_content_reply_notified":true,"second_query_completed_without_ui_callback":true,
            "scope":"real IPC notification/lifecycle test; no native window or performance acceptance"}),
        )
    }
    pub fn run() -> Result<()> {
        let args: Vec<_> = std::env::args().collect();
        let definitions = [
            ComponentDefinition {
                id: accessibility_consumer::MODULE,
                capabilities: Capabilities {
                    view: true,
                    ..Capabilities::HEADLESS
                },
                create: accessibility_consumer::create,
            },
            ComponentDefinition {
                id: scroll_consumer::MODULE,
                capabilities: Capabilities {
                    view: true,
                    ..Capabilities::HEADLESS
                },
                create: scroll_consumer::create,
            },
            ComponentDefinition {
                id: native_host::MODULE,
                capabilities: Capabilities {
                    view: true,
                    ..Capabilities::HEADLESS
                },
                create: native_host::create,
            },
            ComponentDefinition {
                id: MODULE,
                capabilities: Capabilities {
                    view: true,
                    ..Capabilities::HEADLESS
                },
                create,
            },
            ComponentDefinition {
                id: TOOLBAR_MODULE,
                capabilities: Capabilities {
                    view: true,
                    ..Capabilities::HEADLESS
                },
                create: create_toolbar,
            },
        ];
        if run_component_worker_from_args(&definitions, &args)? {
            return Ok(());
        }
        if args.iter().any(|v| v == "--projection-only") {
            println!("{}", projection_consumer::roundtrip()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--accessibility-only") {
            println!("{}", accessibility_consumer::roundtrip()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--scroll-only") {
            println!("{}", scroll_consumer::roundtrip()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--query-wake-only") {
            println!("{}", read_only_query_wake()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--view-context-only") {
            println!("{}", native_host::context_roundtrip()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--three-surfaces-only") {
            println!("{}", native_host::three_surface_roundtrip()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--overlay-only") {
            println!("{}", overlay_consumer::roundtrip()?);
            return Ok(());
        }
        if args.iter().any(|v| v == "--native-overlay") {
            return native_host::run_overlay();
        }
        if args.iter().any(|v| v == "--native") {
            return native_host::run();
        }
        if args.iter().any(|v| v == "--geometry-only") {
            println!("{}", mounted_container_roundtrip()?);
            return Ok(());
        }
        let mut processes = Vec::new();
        for view in 10..13 {
            processes.push(ProcessComponent::spawn(
                ProcessComponentOptions::new(MODULE, 41).with_view(7, view, CAPS),
                || {},
            )?);
        }
        wait(|| {
            processes
                .iter()
                .all(|p| matches!(p.status(), ProcessStatus::Ready { .. }))
        })?;
        let pids: Vec<_> = processes
            .iter()
            .map(|p| match p.status() {
                ProcessStatus::Ready { pid, .. } => pid,
                _ => unreachable!(),
            })
            .collect();
        ensure!(
            pids.iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len()
                == 3,
            "not three isolated workers"
        );
        let mut results = Vec::new();
        for (i, p) in processes.iter().enumerate() {
            let view = 10 + i as u64;
            let mut owner = Owner::default();
            ensure!(
                request(p, &mut owner, view, 1, wire::Input::Snapshot).is_err(),
                "unapplied geometry accepted"
            );
            layout(p, 1, 1, 400)?;
            let initial = request(p, &mut owner, view, 1, wire::Input::Snapshot)?;
            let edit = *initial
                .focus_targets
                .iter()
                .find(|t| t.id == EDIT.0)
                .unwrap();
            let other = *initial
                .focus_targets
                .iter()
                .find(|t| t.id == OTHER.0)
                .unwrap();
            request(
                p,
                &mut owner,
                view,
                1,
                wire::Input::Focus {
                    target: Some(edit),
                    visible: true,
                },
            )?;
            let preedit = request(
                p,
                &mut owner,
                view,
                1,
                wire::Input::Preedit {
                    text: "ni".into(),
                    selection: 2..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: Some(1..3),
                },
            )?;
            ensure!(preedit.composing && preedit.text.as_ref().unwrap().marked == Some(1..3));
            ensure!(
                request(
                    p,
                    &mut owner,
                    view,
                    1,
                    wire::Input::Focus {
                        target: Some(other),
                        visible: true
                    }
                )
                .is_err(),
                "composition focus transfer accepted"
            );
            ensure!(
                request(
                    p,
                    &mut owner,
                    view,
                    1,
                    wire::Input::EnterFocus { reverse: true }
                )
                .is_err(),
                "composition traversal entry accepted"
            );
            let stable = request(p, &mut owner, view, 1, wire::Input::Snapshot)?;
            ensure!(
                stable.text == preedit.text,
                "rejected focus mutated original editor"
            );
            let confirmed = request(
                p,
                &mut owner,
                view,
                1,
                wire::Input::Commit {
                    text: "你".into(),
                    replacement: None,
                },
            )?;
            let text = confirmed.text.as_ref().unwrap();
            ensure!(!confirmed.composing && text.text == "A你B" && text.selection == (2..2));
            ensure!(
                text.caret[3] > 0. && !text.carets.is_empty(),
                "missing original shaped geometry"
            );
            layout(p, 2, 2, 240)?;
            ensure!(
                request(
                    p,
                    &mut owner,
                    view,
                    1,
                    wire::Input::Commit {
                        text: "BAD".into(),
                        replacement: None
                    }
                )
                .is_err(),
                "stale geometry edited content"
            );
            let resized = request(p, &mut owner, view, 2, wire::Input::Snapshot)?;
            ensure!(resized.text.as_ref().unwrap().text == "A你B");
            let stale = Session {
                instance: p.session().instance + 1,
                ..p.session().clone()
            };
            ensure!(
                p.content_request(
                    &stale,
                    wire::Request {
                        view,
                        viewport: 2,
                        owner: owner.target,
                        focus_epoch: owner.epoch,
                        text_binding: owner.binding.clone(),
                        capture: None,
                        input: wire::Input::Snapshot
                    },
                    Duration::from_secs(1)
                )
                .is_err()
            );
            request(
                p,
                &mut owner,
                view,
                2,
                wire::Input::Focus {
                    target: Some(other),
                    visible: true,
                },
            )?;
            ensure!(
                p.content_request(
                    p.session(),
                    wire::Request {
                        view,
                        viewport: 2,
                        owner: Some(edit),
                        focus_epoch: owner.epoch,
                        text_binding: owner.binding.clone(),
                        capture: None,
                        input: wire::Input::Commit {
                            text: "LATE".into(),
                            replacement: None
                        },
                    },
                    Duration::from_secs(1)
                )
                .is_err(),
                "late editor callback retargeted"
            );
            let boundary = request(
                p,
                &mut owner,
                view,
                2,
                wire::Input::Key(wire::KeyEvent {
                    key: Some(wire::Key::Tab { reverse: false }),
                    down: true,
                    repeat: false,
                    modifiers: wire::Modifiers::default(),
                    native_code: 48,
                    characters: "\t".into(),
                    characters_ignoring_modifiers: "\t".into(),
                }),
            )?;
            ensure!(!boundary.consumed && boundary.bubbles && boundary.focused == Some(other));
            ensure!(p.drain_events().iter().any(|m| matches!(&m.output, Output::Event(ComponentEvent::Message { topic, .. }) if topic == "started")), "round trip consumed unrelated event");
            let native_bridge = native_roundtrip(p, view, edit, other)?;
            let pointer = pointer_roundtrip(p, view)?;
            results.push(serde_json::json!({"pointer": pointer, "native_bridge": native_bridge, "pid": pids[i], "session": p.session(), "view": view, "reply": resized, "boundary": boundary}));
        }
        for p in &processes {
            p.close();
        }
        wait(|| {
            processes
                .iter()
                .all(|p| matches!(p.status(), ProcessStatus::Closed))
        })?;
        let container_mounts = mounted_container_roundtrip()?;
        let toolbar_focus = toolbar_focus_roundtrip()?;
        println!(
            "{}",
            serde_json::json!({"toolbar_focus":toolbar_focus,"container_mounts":container_mounts,"result": "REMOTE_ORIGINAL_CONTENT_INPUT_OK", "runs": results, "scope": "three real child processes; CoreText query geometry and original editor/key/focus transport only; no native/visual/AX/scroll acceptance", "performance": false})
        );
        Ok(())
    }
}
