use super::*;
use tekes_ui_settings::FormIntent;
struct SettingsColumn {
    state: std::rc::Rc<std::cell::RefCell<crate::settings::SettingsSurface>>,
    sidebar: bool,
}
impl crate::ContainerContent for SettingsColumn {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        let mut state = self.state.borrow_mut();
        visit(if self.sidebar {
            &mut state.sidebar.host
        } else {
            &mut state.form.host
        });
    }
}
fn settings_columns(
    surface: &std::rc::Rc<std::cell::RefCell<crate::settings::SettingsSurface>>,
) -> Vec<crate::Column> {
    vec![
        crate::Column::from(crate::Container::from_source(
            "settings-leading",
            SettingsColumn {
                state: surface.clone(),
                sidebar: true,
            },
        )),
        crate::Column::from(crate::Container::from_source(
            "settings-center",
            SettingsColumn {
                state: surface.clone(),
                sidebar: false,
            },
        )),
    ]
}
impl Host {
    pub(super) fn prepare_settings_item_navigation(&mut self) -> bool {
        let allowed = self.settings.as_ref().is_none_or(|state| state.borrow_mut().prepare_item_navigation());
        if !allowed {
            // Reject before a request or cross-Host focus cancellation. No
            // history/input gate is changed, and the actual form keeps its IME.
            self.set_toolbar_focus(None, false);
        }
        allowed
    }
    fn sidebar_navigation_point(&self, point: Point) -> bool {
        self.settings.as_ref().is_some_and(|state| {
            let state = state.borrow();
            state.sidebar.host.semantics().iter().any(|node| {
                state.sidebar.host.get(node.id)
                    .and_then(|n| n.component::<tekes_ui_settings::SelectableListContent>())
                    .is_some_and(|list| list.view.layout().items.iter()
                        .any(|row| row.visible.is_some_and(|visible| visible.contains(point))))
            })
        })
    }
    pub(super) fn settings_navigation_focus_guard(
        &mut self,
        event: &Event,
        publication: Option<&tekes_ui_platform_macos::content_accessibility::WindowAccessibilityPublication>,
    ) -> bool {
        let Some(state) = &self.settings else { return false; };
        if self.navigation.as_ref().is_some_and(|nav| nav.busy || !nav.host.input_open)
            || state.borrow().restoring() { return false; }
        let mut back = false;
        let target = match event {
            Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down { button: MouseButton::Left, .. }) => {
                let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
                back = self.leading_visible && state.borrow().sidebar.host.get(ComponentId(4))
                    .and_then(|node| node.visible()).is_some_and(|visible| visible.contains(point));
                (self.toolbar.host.content_host().get(Self::toolbar_control(ToolbarTarget::Navigation)).is_some_and(|node| node.frame().contains(point))
                    || self.toolbar.host.content_host().input_overlay_owner().is_some_and(|(id, _)| id == Self::toolbar_control(ToolbarTarget::Navigation)))
                    || (self.leading_visible && self.sidebar_navigation_point(point)) || back
            }
            Event::Key(key) if key.kind == KeyEventKind::Down => {
                let key = toolbar_key(key.clone()).key;
                let sidebar = state.borrow();
                let focused_list = sidebar.sidebar.host.focused().and_then(|id| sidebar.sidebar.host.get(id))
                    .is_some_and(|node| node.component::<tekes_ui_settings::SelectableListContent>().is_some());
                back = self.leading_visible && sidebar.sidebar.host.focused() == Some(ComponentId(4))
                    && matches!(key, Some(ControlKey::Enter | ControlKey::Space));
                (focused_list && matches!(key, Some(ControlKey::Up | ControlKey::Down | ControlKey::Home | ControlKey::End | ControlKey::Enter)))
                    || (self.toolbar.host.content_host().focused() == Some(Self::toolbar_control(ToolbarTarget::Navigation))
                        && matches!(key, Some(ControlKey::Enter | ControlKey::Space))) || back
            }
            Event::Accessibility {target, action: tekes_ui_platform_macos::AccessibilityAction::Focus | tekes_ui_platform_macos::AccessibilityAction::Press} => {
                let owner = publication.and_then(|p| p.owner(*target));
                if owner == Some(tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(0)) {
                    let opaque = self.toolbar_ax.iter().find(|(key, _)| key == target).map(|(_, target)| *target);
                    self.toolbar.host.semantics().iter().any(|node| node.node.id == Self::toolbar_control(ToolbarTarget::Navigation)
                        && opaque.is_some_and(|target| node.target == target || node.parts.iter().any(|(part, _)| *part == target)))
                } else if self.leading_visible && owner == Some(tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(1)) {
                    let state = state.borrow();
                    back = state.sidebar.host.semantic_mount(ComponentId(4)) == Some(target.component)
                        && state.sidebar.host.get(ComponentId(4)).is_some();
                    back || state.sidebar.host.semantics().iter().any(|node| state.sidebar.host.semantic_mount(node.id) == Some(target.component)
                        && state.sidebar.host.get(node.id).is_some_and(|n| n.component::<tekes_ui_settings::SelectableListContent>().is_some()))
                } else { false }
            }
            _ => false,
        };
        let ready = state.borrow().navigation_input_ready();
        // Successful preflight is pure: this same native publication is still
        // needed by the actual action. Rejected input may update its feedback.
        if !target || ready { return false; }
        if back {
            let allowed = state.borrow_mut().prepare_back_focus();
            if !allowed { self.set_toolbar_focus(None, false); }
            !allowed
        } else { !self.prepare_settings_item_navigation() }
    }
    pub(super) fn settings_sidebar_input(&mut self, event: &ContentEvent) -> Result<Option<crate::navigation::Entry>> {
        self.panels[0].local.as_mut().unwrap().event(event, &mut self.text)?;
        self.settings.as_ref().unwrap().borrow_mut().take_item_navigation()
    }
    pub(super) fn open_settings(&mut self, w: &Window) -> Result<()> {
        if let Some(child) = &self.settings_child
            && !child.is_closed()
        {
            child.show();
            return Ok(());
        }
        self.settings_child.take();
        if self.settings_loading.is_some() {
            return Ok(());
        }
        let Some(provider) = self.settings_provider.clone() else {
            return Ok(());
        };
        let (tx, rx) = mpsc::channel();
        let wake = w.wake_proxy();
        std::thread::spawn(move || {
            let _ = tx.send(provider.load().map_err(|e| format!("{e:#}")));
            wake.wake();
        });
        self.settings_loading = Some(rx);
        Ok(())
    }
    pub(super) fn poll_settings_open(&mut self, w: &Window) -> Result<()> {
        let Some(result) = self
            .settings_loading
            .as_ref()
            .and_then(|r| match r.try_recv() {
                Ok(value) => Some(value),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Settings load worker disconnected".into()))
                }
            })
        else {
            return Ok(());
        };
        self.settings_loading = None;
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(e) => {
                w.set_title(&format!("Settings unavailable: {e}"));
                return Ok(());
            }
        };
        let options = self.settings_options.clone().unwrap();
        let mode = options.display_mode;
        let mut surface = crate::settings::SettingsSurface::new(
            options,
            self.settings_provider.clone().unwrap(),
            snapshot,
        )?;
        surface.show();
        let back = surface.exclusive;
        let surface = std::rc::Rc::new(std::cell::RefCell::new(surface));
        let columns = settings_columns(&surface);
        let app = crate::AppWindow::from_columns(columns)?
            .navigation(surface.borrow().navigation_options());
        let mut host = app.into_host(false);
        host.is_settings_window = true;
        host.acceptance_shared = self.acceptance_shared.clone();
        if self.acceptance_shared.is_some() { host.settings_options = self.settings_options.clone(); }
        host.leading_visible = true;
        host.trailing_visible = false;
        host.sidebar_motion = crate::sidebar_motion::SidebarMotion::new(true, false);
        host.outer.set_ratio(0.25);
        host.settings_back = back;
        surface.borrow_mut().toolbar_window = host.toolbar.window;
        host.settings = Some(surface);
        let config = WindowConfig {
            title: "Settings".into(),
            unified_titlebar: true,
            size: if mode == crate::SettingsDisplayMode::Exclusive {
                w.size()
            } else {
                WindowSize {
                    width: 900.,
                    height: 640.,
                }
            },
            min_size: Some(WindowSize {
                width: 600.,
                height: 360.,
            }),
            ..Default::default()
        };
        let child = if mode == crate::SettingsDisplayMode::Exclusive {
            w.open_exclusive_window(config, host)
        } else {
            w.open_window(config, host)
        };
        self.settings_child = Some(child);
        Ok(())
    }
    pub(super) fn drive_settings_acceptance(&mut self, w: &mut Window) -> Result<()> {
        let Some(shared) = self.acceptance_shared.clone() else { return Ok(()); };
        let stage = shared.stage();
        if matches!(stage, window_acceptance::Stage::Done | window_acceptance::Stage::Main) { return Ok(()); }
        if shared.0.lock().is_ok_and(|s| s.phase_started.elapsed() > std::time::Duration::from_secs(10)) {
            if stage == window_acceptance::Stage::AwaitingShown {
                let nav = self.navigation.as_ref();
                let publication = w.content_accessibility_publication();
                let text_size_targets: Vec<String> = publication.as_ref().map(|p| {
                    p.nodes().iter().filter(|n| n.label == "文字大小").map(|n| format!("label={:?},role={:?},enabled={},owner={:?}", n.label, n.role, n.enabled, p.owner(n.key).map(|owner| owner.0))).collect()
                }).unwrap_or_default();
                let text_field_count = publication.as_ref().map_or(0, |p| p.nodes().iter().filter(|n| n.role == tekes_ui_platform_macos::AccessibilityRole::TextField).count());
                let text_fields: Vec<String> = publication.as_ref().map(|p| {
                    p.nodes().iter().filter(|n| n.role == tekes_ui_platform_macos::AccessibilityRole::TextField).take(12)
                        .map(|n| format!("label={:?},role={:?},enabled={},owner={:?}", n.label, n.role, n.enabled, p.owner(n.key).map(|owner| owner.0))).collect()
                }).unwrap_or_default();
                eprintln!("SETTINGS_ACCEPTANCE_WAIT_TIMEOUT phase=AwaitingShown nav_present={} nav_busy={} nav_input_open={} nav_error={:?} active={} key={} occluded={} publication_present={} publication_current={} publication_valid={} publication_nodes={} text_size_targets={:?} text_field_count={} text_fields_first12={:?}",
                    nav.is_some(), nav.is_some_and(|n| n.busy), nav.is_some_and(|n| n.host.input_open), nav.and_then(|n| n.error.as_deref()),
                    w.is_active(), w.is_key_window(), w.is_occluded(), publication.is_some(),
                    publication.as_ref().is_some_and(|p| p.is_current()), publication.as_ref().is_some_and(|p| p.is_valid()),
                    publication.as_ref().map_or(0, |p| p.nodes().len()), text_size_targets, text_field_count, text_fields);
            }
            shared.fail(format!("10s timeout in {stage:?}"));
            w.close();
            if let Ok(s) = shared.0.lock() && let Some(wake) = &s.parent_wake { let _ = wake.wake(); }
            return Ok(());
        }
        if stage == window_acceptance::Stage::Failed {
            w.close();
            if let Ok(s) = shared.0.lock() && let Some(wake) = &s.parent_wake { let _ = wake.wake(); }
            return Ok(());
        }
        if shared.0.lock().is_ok_and(|s| s.started.elapsed() > std::time::Duration::from_secs(30)) {
            shared.fail("30s total acceptance timeout");
            w.close();
            return Ok(());
        }
        if let Ok(mut result) = shared.0.lock() { result.child_wake = Some(w.wake_proxy()); }
        let publication = w.content_accessibility_publication();
        match stage {
            window_acceptance::Stage::AwaitingShown => {
                let options = self.settings_options.as_ref().ok_or_else(|| anyhow::anyhow!("acceptance settings options absent"))?;
                let field = options.schema.settings.iter().find(|f| f.label == "文字大小")
                    .ok_or_else(|| anyhow::anyhow!("acceptance requires demo text-size field `文字大小`"))?;
                let nav_ready = self.navigation.as_ref().is_some_and(|nav| nav.host.input_open && !nav.busy && nav.error.is_none());
                if !w.is_active() || !w.is_key_window() || w.is_occluded() || !nav_ready {
                    w.request_redraw();
                    return Ok(());
                }
                let Some(publication) = publication else { w.request_redraw(); return Ok(()); };
                if !publication.is_current() || !publication.is_valid() {
                    w.request_redraw();
                    return Ok(());
                }
                let owner = tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(2);
                let Some(node) = publication.nodes().iter().find(|n| n.label == field.label
                    && n.enabled
                    && n.role == tekes_ui_platform_macos::AccessibilityRole::TextField
                    && publication.owner(n.key) == Some(owner)) else {
                    w.request_redraw();
                    return Ok(());
                };
                let original = self.settings.as_ref().unwrap().borrow().draft.display(field);
                let current: i64 = original.parse().map_err(|_| anyhow::anyhow!("text-size AX value is not integer: {original}"))?;
                let edited = if current + 10 <= 300 { current + 10 } else { current - 10 };
                {
                    let mut result = shared.0.lock().map_err(|e| anyhow::anyhow!("acceptance lock: {e}"))?;
                    result.child_window = Some(w.native_window_number());
                    result.mode = Some(options.display_mode);
                    result.field_path = field.path.clone();
                    result.original = Some(original);
                    result.edited = Some(edited.to_string());
                    result.stage = window_acceptance::Stage::Shown;
                    result.phase_started = std::time::Instant::now();
                }
                eprintln!("SETTINGS_ACCEPTANCE_SHOWN parent_window={} child_window={} active={} key={} occluded={} ax_nodes={} mode={:?} physical_input=false", shared.0.lock().unwrap().parent_window, w.native_window_number(), w.is_active(), w.is_key_window(), w.is_occluded(), publication.nodes().len(), options.display_mode);
                self.event(Event::Accessibility { target: node.key, action: tekes_ui_platform_macos::AccessibilityAction::Focus }, w)?;
                w.request_redraw();
            }
            window_acceptance::Stage::Shown => {
                let Some(publication) = publication else { return Ok(()); };
                let node = publication.nodes().iter().find(|n| n.label == "文字大小" && n.enabled && n.role == tekes_ui_platform_macos::AccessibilityRole::TextField)
                    .ok_or_else(|| anyhow::anyhow!("Settings AX field disappeared after Focus"))?;
                self.event(Event::Accessibility { target: node.key, action: tekes_ui_platform_macos::AccessibilityAction::Focus }, w)?;
                self.event(Event::TextCommand("selectAll:".into()), w)?;
                shared.transition(window_acceptance::Stage::Editing);
                w.request_redraw();
            }
            window_acceptance::Stage::Editing => {
                let value = shared.0.lock().unwrap().edited.clone().ok_or_else(|| anyhow::anyhow!("acceptance edited value missing"))?;
                self.event(Event::TextCommit { text: value, replacement: None }, w)?;
                self.event(Event::TextCommand("insertNewline:".into()), w)?;
                shared.transition(window_acceptance::Stage::Saving);
                w.request_redraw();
            }
            window_acceptance::Stage::Saving => {
                let options = self.settings_options.as_ref().ok_or_else(|| anyhow::anyhow!("acceptance settings options absent"))?;
                let (field_path, edited) = { let s = shared.0.lock().unwrap(); (s.field_path.clone(), s.edited.clone().unwrap()) };
                let source = match std::fs::read_to_string(&options.file) {
                    Ok(source) => source,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => { w.request_redraw(); return Ok(()); },
                    Err(e) => return Err(anyhow::anyhow!("read settings file: {e}")),
                };
                let saved = window_acceptance::read_integer_override(&source, &field_path).map_err(anyhow::Error::msg)?;
                let clean = self.settings.as_ref().is_some_and(|s| !s.borrow().draft.dirty() && !s.borrow().blocked());
                if saved.as_deref() == Some(edited.as_str()) && clean
                    && let Some(undo) = self.toolbar.host.semantics().into_iter().flat_map(|n| n.parts.into_iter()).find(|(_, p)| p.semantics.name == "Undo" && !p.semantics.disabled).map(|(_, p)| p.frame) {
                        eprintln!("SETTINGS_ACCEPTANCE_SAVED path={} value={} draft_clean=true undo_enabled=true", options.file.display(), edited);
                        shared.transition(window_acceptance::Stage::Undoing);
                        let point = tekes_ui_platform_macos::Point { x: undo.center().x as f64, y: undo.center().y as f64 };
                        for kind in [MouseEventKind::Down { button: MouseButton::Left, click_count: 1 }, MouseEventKind::Up { button: MouseButton::Left, click_count: 1 }] {
                            self.event(Event::Mouse(tekes_ui_platform_macos::MouseEvent { kind, position: point, modifiers: tekes_ui_platform_macos::Modifiers::default() }), w)?;
                        }
                        shared.transition(window_acceptance::Stage::Restoring);
                }
            }
            window_acceptance::Stage::Undoing => { w.request_redraw(); }
            window_acceptance::Stage::Restoring => {
                let options = self.settings_options.as_ref().ok_or_else(|| anyhow::anyhow!("acceptance settings options absent"))?;
                let s = shared.0.lock().unwrap();
                let original = s.original.clone().ok_or_else(|| anyhow::anyhow!("acceptance original value missing"))?;
                let source = match std::fs::read_to_string(&options.file) {
                    Ok(source) => source,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(e) => return Err(e.into()),
                };
                let field = options.schema.settings.iter().find(|f| f.label == "文字大小").ok_or_else(|| anyhow::anyhow!("text-size field disappeared"))?;
                let saved = window_acceptance::read_integer_override(&source, &s.field_path).map_err(anyhow::Error::msg)?.unwrap_or_else(|| field.display(&field.default));
                let (form_value, edit, clean) = {
                    let settings = self.settings.as_ref().unwrap().borrow();
                    let edit = settings.form.host.semantics().iter().find_map(|node| {
                        if node.semantics.name != field.label { return None; }
                        settings.form.host.get(node.id)
                            .and_then(|n| n.component::<tekes_ui_controls::composition::NumberFieldContent>())
                            .map(|number| number.edit_state())
                    }).ok_or_else(|| anyhow::anyhow!("actual Settings NumberField absent"))?;
                    (settings.draft.display(field), edit, !settings.draft.dirty() && !settings.blocked())
                };
                drop(s);
                if saved == original && form_value == original && edit.text == format!("{original}{}", field.suffix)
                    && !edit.dirty && !edit.composing && clean {
                    eprintln!("SETTINGS_ACCEPTANCE_UNDO_RESTORED path={} value={} form_value={} field_text={:?} draft_clean=true", options.file.display(), saved, form_value, edit.text);
                    let mode = shared.0.lock().unwrap().mode.unwrap();
                    if mode == crate::SettingsDisplayMode::Exclusive {
                        let back = publication.as_ref().and_then(|p| p.nodes().iter().find(|n| n.label == "Back" || n.label == "返回")).ok_or_else(|| anyhow::anyhow!("published original Settings Back action absent"))?;
                        shared.transition(window_acceptance::Stage::Closing);
                        self.event(Event::Accessibility { target: back.key, action: tekes_ui_platform_macos::AccessibilityAction::Press }, w)?;
                    } else {
                        shared.transition(window_acceptance::Stage::Closing);
                        self.event(Event::CloseRequested, w)?;
                    }
                    if let Some(wake) = shared.0.lock().unwrap().parent_wake.clone() { let _ = wake.wake(); }
                } else { w.request_redraw(); }
            }
            window_acceptance::Stage::Closing => { }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn draw_settings(&mut self, w: &Window) -> Result<()> {
        let progress = self.sidebar_motion.sample(
            self.motion_epoch.elapsed(),
            self.reduced_motion,
            w.is_active(),
        )?;
        let transitioning = self.sidebar_motion.needs_frame();
        let shared_settings = self.settings.as_ref().unwrap().clone();
        let mut state = shared_settings.borrow_mut();
        let settings = &mut *state;
        settings.poll(w.wake_proxy())?;
        if !settings.open {
            w.close();
            return Ok(());
        }
        let navigation = self.navigation.as_mut().unwrap();
        if self
            .navigation_ack
            .as_ref()
            .is_some_and(|ack| ack.try_recv().is_ok())
        {
            self.navigation_ack = None;
            self.navigation_order = None;
            navigation.published()?;
        }
        navigation.poll()?;
        if self.navigation_order.is_none()
            && let Some(publication) = &navigation.publication
        {
            if let Some(target) = publication.target() {
                ensure!(
                    target.route == "settings-item",
                    "route not installed in Settings"
                );
                let item = target
                    .params
                    .get("item_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing Settings item"))?;
                settings.select(item)?;
            }
            self.navigation_order = Some(vec![]);
        }
        self.text.scale = w.scale() as f32;
        let size = w.size();
        let whole = Rect::new(0., 0., size.width as f32, size.height as f32);
        let palette = Palette::from(Theme::LIGHT);
        let mut scene = Scene::new();
        let mut toolbar_scene = Scene::new();
        scene.push_quad(Quad::new(whole, palette.color("surface-panel").unwrap()));
        self.outer.min_pane = 160.;
        // Arrange now; paint only after current toolbar hit regions are known.
        let layout = self.outer_view.arrange(&mut self.outer, whole, None);
        let frames = presented_frames(
            &[layout.leading.frame(), layout.trailing.frame()],
            whole,
            progress,
            w.scale() as f32,
        );
        self.settings_frames = frames.clone();
        let renderer = self.renderer.as_mut().unwrap();

        let measured = ToolbarControl::window().row_height(&mut self.text);
        let native_center = w.set_toolbar_row_height(f64::from(measured));
        let (native, leading) = w.titlebar_safe_area();
        let (top, row_y, paint) = window_toolbar_geometry(
            native as f32,
            measured,
            native_center.map(|y| y as f32),
            w.scale() as f32,
        );
        self.settings_top = top;
        if self.settings_back {
            let image = crate::window_toolbar::icon_image(
                &mut self.toolbar_images, renderer, "tekes-back",
                w.scale() as f32 * self.text.type_scale(),
            )?;
            settings.sidebar.set_back_icon(image);
        }
        let breadcrumb = settings.breadcrumb();
        let _ = navigation;
        let _ = renderer;
        let form_content = content_geometry(frames[1], top);
        let pad = 24.;
        let top_gap = tekes_ui_tokens::proposal::metric("spacing", "space-regular").unwrap()
            * self.text.type_scale();
        let width = (form_content.size.width - 2. * pad).clamp(100., 620.);
        let form_frame = Rect::new(
            form_content.min_x() + (form_content.size.width - width) * 0.5,
            form_content.min_y() + top_gap,
            width,
            (form_content.size.height - top_gap - pad).max(80.),
        );
        let _ = settings;
        drop(state);
        self.sync_toolbar(
            w,
            whole,
            &frames,
            window_toolbar_mount::ToolbarBand {
                y: row_y,
                height: paint,
                native_leading: leading as f32,
            },
            breadcrumb,
        )?;
        self.toolbar
            .host
            .render(&mut toolbar_scene, &mut self.text, Theme::LIGHT.into())
            .map_err(|e| anyhow::anyhow!("settings toolbar: {e:?}"))?;
        self.panels[1].bounds = frames[1];
        self.panels[1].local.as_mut().unwrap().render(
            &mut scene,
            &mut self.text,
            form_frame,
            form_frame,
            palette,
        )?;
        if frames[0].size.width > 32. {
            let left = content_geometry(frames[0], top);
            let list_top = left.min_y() + top_gap;
            let rect = Rect::new(
                left.min_x() + 16.,
                list_top,
                (left.size.width - 32.).max(1.),
                (left.max_y() - 24. - list_top).max(1.),
            );
            self.panels[0].bounds = frames[0];
            self.panels[0].local.as_mut().unwrap().render(
                &mut scene,
                &mut self.text,
                rect,
                rect,
                palette,
            )?;
        }
        let row = Rect::new(whole.min_x(), row_y, whole.size.width, paint);
        let mut toolbar_controls = self.toolbar.host.content_host().pointer_regions();
        if leading > 0. {
            toolbar_controls.push(Rect::new(0., row_y, leading as f32, paint));
        }
        self.outer_view
            .set_toolbar_region(Some(row), &toolbar_controls, &mut self.text);
        if self.leading_visible && !transitioning {
            self.outer_view.paint_presented_divider(
                &mut scene,
                &self.outer,
                toolbar_divider_frame(layout.divider.frame(), row_y),
                palette,
                w.scale() as f32,
            );
        }
        // The shared ToolbarHost render includes help and all original overlays.
        // Composite that complete toolbar layer above ordinary content.
        for primitive in toolbar_scene.primitives() {
            scene.push(primitive.clone());
        }

        let renderer = self.renderer.as_mut().unwrap();
        renderer.resize(w.drawable_size(), w.scale() as f32);
        renderer.render(&scene, &mut self.text.atlas, Rgba::gray(1.))?;
        if self.navigation_order.is_some() && self.navigation_ack.is_none() {
            let (tx, rx) = mpsc::sync_channel(1);
            self.completion
                .as_ref()
                .unwrap()
                .try_send(vec![tx])
                .map_err(|_| anyhow::anyhow!("Settings GPU completion queue full"))?;
            self.navigation_ack = Some(rx);
        }
        let input_open = self
            .navigation
            .as_ref()
            .is_some_and(|n| n.host.input_open && !n.busy)
            && !shared_settings.borrow().restoring();
        let active = w.is_active();
        w.set_content_accessibility_publication(self.content_accessibility_publication(active && input_open));
        w.set_text_input_state(if active && input_open {
            crate::columns::text_state(self.toolbar.host.content_host()).or_else(|| {
                (!self.toolbar.modal() && self.toolbar.host.content_host().focused().is_none())
                    .then(|| self.settings.as_ref().unwrap().borrow().text_state())
                    .flatten()
            })
        } else {
            None
        });
        let resize_regions = if self.leading_visible && !transitioning {
            self.outer_view.resize_regions(&self.outer)
        } else {
            vec![]
        };
        w.set_cursor_regions(
            resize_regions
                .into_iter()
                .map(|(r, _)| tekes_ui_platform_macos::CursorRegion {
                    origin: tekes_ui_platform_macos::Point {
                        x: r.min_x().into(),
                        y: r.min_y().into(),
                    },
                    size: WindowSize {
                        width: r.size.width.into(),
                        height: r.size.height.into(),
                    },
                    icon: tekes_ui_platform_macos::CursorIcon::ResizeHorizontal,
                })
                .collect(),
        );
        let animate = (self.acceptance_shared.as_ref().is_some_and(|s| !matches!(s.stage(), window_acceptance::Stage::Done | window_acceptance::Stage::Failed))
            || self.sidebar_motion.needs_frame()
            || self.toolbar.host.needs_render()
            || self.text.animation_requested()
            || self.settings.as_ref().unwrap().borrow().needs_frame())
            && active;
        if !w.set_animating(animate) && animate {
            w.request_redraw();
        }
        Ok(())
    }

    fn settings_shell_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        let Event::Mouse(mouse) = event else {
            return Ok(false);
        };
        if !w.is_active() {
            return Ok(true);
        }
        let p = Point::new(mouse.position.x as f32, mouse.position.y as f32);
        let handled = match mouse.kind {
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count,
            } => {
                if self.leading_visible
                    && !self.sidebar_motion.needs_frame()
                    && self.outer_view.pointer_down(p, &mut self.outer)
                {
                    self.drag_split = Some(0);
                    true
                } else if p.y < self.settings_top {
                    if click_count == 2 {
                        w.try_titlebar_double_click();
                    } else {
                        w.try_begin_titlebar_drag();
                    }
                    true
                } else {
                    false
                }
            }
            MouseEventKind::Up {
                button: MouseButton::Left,
                ..
            } if self.drag_split.take().is_some() => {
                self.outer_view.pointer_up(&mut self.outer);
                true
            }
            MouseEventKind::Dragged {
                button: MouseButton::Left,
            } if self.drag_split == Some(0) => {
                self.outer_view.pointer_move(p, &mut self.outer);
                true
            }
            _ => false,
        };
        if handled {
            w.request_redraw();
        }
        Ok(handled)
    }
    pub(super) fn settings_event(&mut self, event: &Event, w: &mut Window) -> Result<bool> {
        if self
            .settings
            .as_ref()
            .is_some_and(|s| s.borrow().restoring())
            && matches!(
                event,
                Event::Mouse(_)
                    | Event::Key(_)
                    | Event::Accessibility { .. }
                    | Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextCommand(_)
            )
        {
            return Ok(true);
        }
        if self.settings_shell_event(event, w)? {
            return Ok(true);
        }
        if self
            .navigation
            .as_ref()
            .is_some_and(|n| n.busy || !n.host.input_open)
            && matches!(
                event,
                Event::Mouse(_)
                    | Event::Key(_)
                    | Event::Accessibility { .. }
                    | Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextCommand(_)
            )
        {
            return Ok(true);
        }
        // Header search is an original local editor, using the same text/IME
        // and clipboard adapter as every ordinary Container, not the form draft.
        let sidebar_editor = self.settings.as_ref().is_some_and(|s|
            s.borrow().sidebar.host.focused_text().is_some());
        if sidebar_editor && matches!(event,
            Event::Key(_) | Event::TextKey(_) | Event::TextCommit { .. }
            | Event::TextPreedit { .. } | Event::TextUnmark | Event::TextCommand(_)) {
            self.focused = 0;
            if self.local_content_event(event, w)? {
                self.draw(w)?;
                return Ok(true);
            }
        }
        let shared_settings = self.settings.as_ref().unwrap().clone();
        let mut state = shared_settings.borrow_mut();
        let settings = &mut *state;
        let mut intents = vec![];
        let mut routed = None;
        let mut sidebar_routed = None;
        match event {
            Event::Mouse(m) if w.is_active() => {
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                if matches!(m.kind, MouseEventKind::Down { .. }) {
                    let sidebar = self.leading_visible
                        && self.settings_frames.first().is_some_and(|r| r.contains(p));
                    self.settings_pointer = Some(sidebar);
                    if sidebar {
                        settings.form.host.focus(None, false);
                    } else {
                        settings.sidebar.host.focus(None, false);
                    }
                }
                if self.settings_pointer == Some(true) {
                    sidebar_routed = Some(toolbar_pointer(*m));
                } else {
                    routed = Some(toolbar_pointer(*m));
                }
                if matches!(m.kind, MouseEventKind::Up { .. }) {
                    self.settings_pointer = None;
                }
            }
            Event::Scroll(s) if w.is_active() => {
                let scroll = ContentEvent::Scroll {
                    position: Point::new(s.position.x as f32, s.position.y as f32),
                    delta: Point::new(-s.delta.x as f32, -s.delta.y as f32),
                    precise: s.precise,
                    begin: false,
                    end: false,
                    momentum: false,
                    time: self.motion_epoch.elapsed().as_secs_f64(),
                };
                if self.leading_visible
                    && self.settings_frames.first().is_some_and(|r| {
                        r.contains(Point::new(s.position.x as f32, s.position.y as f32))
                    })
                {
                    sidebar_routed = Some(scroll);
                } else {
                    routed = Some(scroll);
                }
            }
            Event::Key(key)
                if w.is_active()
                    && settings.sidebar.host.focused().is_some()
                    && !key.modifiers.command
                    && key.key_code != 53 =>
            {
                sidebar_routed = Some(ContentEvent::KeyInput(toolbar_key(key.clone())));
            }
            Event::Key(key) if w.is_active() => {
                let composing = settings
                    .form
                    .host
                    .focused_text()
                    .is_some_and(|s| s.input.marked.is_some());
                if key.kind == KeyEventKind::Down
                    && key.modifiers.command
                    && settings.form.host.focused_text().is_some()
                {
                    let command = match key
                        .characters_ignoring_modifiers
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "a" => Some("selectAll:"),
                        "z" => Some(if key.modifiers.shift {
                            "redo:"
                        } else {
                            "undo:"
                        }),
                        _ => None,
                    };
                    if key.characters_ignoring_modifiers == "z"
                        && settings.form.host.focused_text().is_none()
                    {
                        if let Some(source) = settings.toolbar_source.clone() {
                            settings.undo_action(
                                if key.modifiers.shift {
                                    crate::settings_undo::REDO
                                } else {
                                    crate::settings_undo::UNDO
                                },
                                &source,
                            )?;
                        }
                    } else if let Some(command) = command {
                        routed = Some(ContentEvent::TextCommand(command.into()));
                    } else if key.characters_ignoring_modifiers == "c"
                        || key.characters_ignoring_modifiers == "x"
                    {
                        if let Some(surface) = settings.form.host.focused_text()
                            && !surface.input.selection().is_empty()
                        {
                            w.set_clipboard_text(&surface.input.selected_text());
                            if key.characters_ignoring_modifiers == "x" {
                                routed = Some(ContentEvent::TextCommand("deleteBackward:".into()));
                            }
                        }
                    } else if key.characters_ignoring_modifiers == "v" {
                        if let Some(text) = w.clipboard_text() {
                            routed = Some(ContentEvent::Commit {
                                text,
                                replacement: None,
                            });
                        }
                    } else {
                        routed = Some(ContentEvent::KeyInput(toolbar_key(key.clone())));
                    }
                } else if key.kind == KeyEventKind::Down
                    && key.key_code == 53
                    && !composing
                    && !settings.form.has_open_menu()
                    && !settings.form.number_editing()
                {
                    intents.push(FormIntent::Cancel);
                } else {
                    routed = Some(ContentEvent::KeyInput(toolbar_key(key.clone())));
                }
            }
            Event::TextKey(marker) => {
                routed = window_toolbar_mount::native_toolbar_event(&Event::TextKey(marker.clone()))
            }
            Event::TextCommit { text, replacement } => {
                routed = Some(ContentEvent::Commit {
                    text: text.clone(),
                    replacement: replacement.clone(),
                })
            }
            Event::TextPreedit {
                text,
                selection,
                clauses,
                replacement,
            } => {
                routed = Some(ContentEvent::Preedit {
                    text: text.clone(),
                    selection: selection.clone(),
                    clauses: clauses.clone(),
                    replacement: replacement.clone(),
                })
            }
            Event::TextUnmark => routed = Some(ContentEvent::Unmark),
            Event::TextCommand(command)
                if command == "cancelOperation:"
                    && !settings.form.number_editing()
                    && !settings.form.has_open_menu()
                    && settings
                        .form
                        .host
                        .focused_text()
                        .is_none_or(|s| s.input.marked.is_none()) =>
            {
                intents.push(FormIntent::Cancel)
            }
            Event::TextCommand(command) => {
                if command == "copy:" || command == "cut:" {
                    if let Some(surface) = settings.form.host.focused_text()
                        && !surface.input.selection().is_empty()
                    {
                        w.set_clipboard_text(&surface.input.selected_text());
                        if command == "cut:" {
                            routed = Some(ContentEvent::TextCommand("deleteBackward:".into()));
                        }
                    }
                } else if command == "paste:" {
                    if let Some(text) = w.clipboard_text() {
                        routed = Some(ContentEvent::Commit {
                            text,
                            replacement: None,
                        });
                    }
                } else {
                    routed = Some(ContentEvent::TextCommand(command.clone()));
                }
            }
            Event::Accessibility { target, action } => {
                let Some(publication)=w.content_accessibility_publication() else{return Ok(false);};
                let Some(owner)=publication.owner(*target) else{return Ok(false);};
                if !settings.accessibility_input(&publication, owner, *target, action.clone(), self.leading_visible) {
                    return Ok(true);
                }
                // Do not revoke any participating Host until publication.dispatch
                // has delivered successfully to its stamped original owner.
                if matches!(action, tekes_ui_platform_macos::AccessibilityAction::Focus | tekes_ui_platform_macos::AccessibilityAction::Press) {
                    self.set_toolbar_focus(None, false);
                    self.focused = if owner.0 == 1 { 0 } else { 1 };
                }
                if owner.0 == 1 {
                    if let Some(entry) = settings.take_item_navigation()? {
                        if self.navigation.as_ref().is_some_and(|n| n.host.input_open && !n.busy) {
                            self.navigation.as_mut().unwrap().send(crate::navigation::Command::Navigate(crate::navigation::Action::Push(entry)))?;
                        }
                    } else if settings.sidebar.take_back() { settings.close_requested(); }
                } else {
                    intents.extend(settings.form.collect(&settings.view_schema, &mut settings.draft));
                }
            }
            Event::Focus(false) => {
                settings.form.host.cancel();
                settings.flush(w.wake_proxy())?;
                w.set_content_accessibility_publication(None);
                w.set_text_input_state(None);
                return Ok(false);
            }
            Event::QuitRequested => {
                settings.close_requested();
                settings.intents(vec![], w.wake_proxy())?;
                w.request_redraw();
                return Ok(true);
            }
            Event::CloseRequested => {
                if settings.close_requested() {
                    w.close();
                } else {
                    settings.intents(vec![], w.wake_proxy())?;
                }
                w.request_redraw();
                return Ok(true);
            }
            Event::CaretBlink => {
                w.request_redraw();
                return Ok(true);
            }
            Event::Created
            | Event::Redraw
            | Event::Resized { .. }
            | Event::Wake
            | Event::AnimationFrame(_)
            | Event::Focus(true)
            | Event::SystemMotionChanged(_) => return Ok(false),
            // No input or toolbar AX action may leak behind the Settings surface.
            _ => return Ok(true),
        }
        if let Some(event) = sidebar_routed {
            let _ = settings;
            drop(state);
            let entry = self.settings_sidebar_input(&event)?;
            state = shared_settings.borrow_mut();
            if let Some(entry) = entry {
                self.navigation
                    .as_mut()
                    .unwrap()
                    .send(crate::navigation::Command::Navigate(
                        crate::navigation::Action::Push(entry),
                    ))?;
            }
        }
        if state.sidebar.take_back() {
            state.close_requested();
        }
        if let Some(event) = routed {
            drop(state);
            self.panels[1]
                .local
                .as_mut()
                .unwrap()
                .event(&event, &mut self.text)?;
            state = shared_settings.borrow_mut();
            let settings = &mut *state;
            intents.extend(
                settings
                    .form
                    .collect(&settings.view_schema, &mut settings.draft),
            );
            if settings.form.host.focused().is_some() {
                settings.sidebar.host.focus(None, false);
            }
        }
        let settings = &mut *state;
        settings.edited();
        settings.intents(intents, w.wake_proxy())?;
        if !settings.open {
            w.close();
        }
        w.set_text_input_state(if settings.open {
            settings.text_state()
        } else {
            None
        });
        w.request_redraw();
        Ok(true)
    }
}

#[cfg(test)]
mod pinned_header_tests {
    use super::*;
    use tekes_ui_controls::composition::TextFieldContent;
    #[test]
    fn header_uses_toolkit_icon_and_fixed_original_editor_with_scrolling_list() {
        let schema = tekes_ui_settings::Schema::parse(include_str!("../../../examples/app-host-demo/settings.schema.toml")).unwrap();
        let mut sidebar = tekes_ui_settings::SettingsSidebar::new_with_back(&schema).unwrap();
        sidebar.set_back_icon(tekes_ui_scene::ImageId(777));
        let mut text = crate::text::Text::new();
        let frame = Rect::new(0., 0., 260., 180.);
        let draw = |s: &mut tekes_ui_settings::SettingsSidebar, t: &mut crate::text::Text| {
            s.render(&mut Scene::new(), t, frame, Theme::LIGHT.into()).unwrap();
        };
        draw(&mut sidebar, &mut text);
        let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
        let search = sidebar.host.get(ComponentId(6)).unwrap().frame();
        let scroll = sidebar.host.get(ComponentId(1)).unwrap().frame();
        assert!(back.max_x() < search.min_x());
        assert!(search.max_y() < scroll.min_y());
        assert!(sidebar.host.get(ComponentId(4)).unwrap().component::<tekes_ui_controls::Button>().unwrap().label.is_empty());
        sidebar.host.event_with_text(&ContentEvent::PointerDown(search.center()), &mut text).unwrap();
        sidebar.host.event_with_text(&ContentEvent::PointerUp(search.center()), &mut text).unwrap();
        sidebar.host.event_with_text(&ContentEvent::Commit { text: "搜索".into(), replacement: None }, &mut text).unwrap();
        sidebar.host.event_with_text(&ContentEvent::Scroll { position: scroll.center(), delta: Point::new(0., 100.), precise: true, begin: true, end: false, momentum: false, time: 0. }, &mut text).unwrap();
        draw(&mut sidebar, &mut text);
        assert_eq!(sidebar.host.get(ComponentId(6)).unwrap().frame(), search);
        assert_eq!(sidebar.host.get(ComponentId(6)).unwrap().component::<TextFieldContent>().unwrap().surface.input.text, "搜索");
        sidebar.event(&ContentEvent::PointerDown(back.center()));
        sidebar.event(&ContentEvent::PointerUp(back.center()));
        assert!(sidebar.take_back());
        assert!(!sidebar.take_back());
    }
}


#[cfg(test)]
mod navigation_input_tests {
    use super::*;
    use crate::settings::{SettingsDisplayMode,SettingsSurface,SettingsWindow};
    use crate::window_toolbar::{Contents,Declaration,Source};
    use crate::toolbar_source::ComponentSource;
    use tekes_ui_controls::composition::{ToolbarButtonDeclaration,ToolbarGroupOverflow,ToolbarPlacement,ToolbarRegion,AccessibilityAction as ContentAction};
    use tekes_ui_platform_macos::{AccessibilityAction,Modifiers,content_accessibility::ContentAccessibilityOwner};
    use tekes_ui_settings::{FileProvider,Snapshot,SelectableListContent};
    fn fixture(marked:bool)->(Host,std::path::PathBuf) { fixture_mode(marked,SettingsDisplayMode::Independent) }
    fn fixture_mode(marked:bool,mode:SettingsDisplayMode)->(Host,std::path::PathBuf) {
        let path=std::env::temp_dir().join(format!("settings-nav-input-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())).join("settings.toml");
        let schema=include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml").replace("min = 10\nmax = 32","min = 10\nmax = 32\nstep = 1")
            .replace("[[groups.items]]\nid = \"advanced\"", "[[groups.items.fields]]\nid = \"extra-number\"\npath = [\"reading\", \"extra_number\"]\nlabel = \"Extra number\"\ntype = \"integer\"\ndefault = 14\nmin = 10\nmax = 32\nstep = 1\n\n[[groups.items]]\nid = \"advanced\"");
        let options=SettingsWindow::from_toml(&schema,path.clone()).unwrap().display_mode(mode);
        let provider=FileProvider::open(path.clone(),options.schema.namespace.clone()).unwrap();
        let mut surface=SettingsSurface::new(options,provider,Snapshot::default()).unwrap();
        if !marked {assert!(surface.prepare_item_navigation());surface.select("reading/text").unwrap();}
        surface.show();
        let state=std::rc::Rc::new(std::cell::RefCell::new(surface));
        let mut host=crate::AppWindow::from_columns(settings_columns(&state)).unwrap().into_host(false);
        host.settings=Some(state);host.is_settings_window=true;host.leading_visible=true;
        assert!(host.mount_columns());
        let source=ComponentSource{session:tekes_ui_module_runtime::component::Session{module:crate::navigation::MODULE_ID.into(),instance:1,host_epoch:1},registration:1,window:host.toolbar.window};
        host.toolbar.sync(Rect::new(0.,0.,940.,600.),vec![ToolbarRegion::new(ComponentId(10),Rect::new(0.,0.,280.,52.))],vec![Declaration{id:ComponentId(92100),region:ComponentId(10),placement:ToolbarPlacement::Navigation,order:0,contents:Contents::Commands(vec![ToolbarButtonDeclaration{id:crate::navigation::BACK,icon:None,text:"Back".into(),hint:None,enabled:true},ToolbarButtonDeclaration{id:crate::navigation::FORWARD,icon:None,text:"Forward".into(),hint:None,enabled:true}],ToolbarGroupOverflow::Split),source:Source::Navigation(source)}]).unwrap();
        render(&mut host);
        {
            let mut s=host.settings.as_ref().unwrap().borrow_mut();
            let id=ComponentId(if marked {100}else{101});
            assert!(s.form.host.focus(Some(id),true));
            if marked {s.form.host.event(&ContentEvent::Preedit{text:"中文".into(),selection:0..2,clauses:std::iter::once(0..2).collect(),replacement:None});}
            else {s.form.host.event(&ContentEvent::TextCommand("selectAll:".into()));s.form.host.event(&ContentEvent::Commit{text:"99".into(),replacement:None});}
        }
        render(&mut host);
        (host,path)
    }
    fn render(host:&mut Host) {
        for (i,frame) in [Rect::new(16.,60.,260.,500.),Rect::new(300.,60.,620.,500.)].into_iter().enumerate() {
            host.panels[i].bounds=frame;
            host.panels[i].local.as_mut().unwrap().render(&mut Scene::new(),&mut host.text,frame,frame,Theme::LIGHT.into()).unwrap();
        }
        host.toolbar.host.render(&mut Scene::new(),&mut host.text,Theme::LIGHT.into()).unwrap();
    }
    fn correct(host:&mut Host,marked:bool) {
        let mut s=host.settings.as_ref().unwrap().borrow_mut();
        if marked {s.form.host.event(&ContentEvent::Unmark);}
        else {s.form.host.event(&ContentEvent::TextCommand("selectAll:".into()));s.form.host.event(&ContentEvent::Commit{text:"18".into(),replacement:None});}
        let state=&mut *s;
        state.form.collect(&state.view_schema,&mut state.draft);
        state.edited();
        drop(s);render(host);
    }
    fn row(host:&Host,marked:bool)->(ComponentId,Point,String) {
        let label=if marked {"文本与语言"}else{"高级排版"};
        let state=host.settings.as_ref().unwrap().borrow();
        for node in state.sidebar.host.semantics() {
            if let Some(list)=state.sidebar.host.get(node.id).and_then(|n|n.component::<SelectableListContent>())
                && let Some(index)=list.labels.iter().position(|s|s==label) {
                let point=list.view.layout().items.iter().find(|r|r.id==index).unwrap().visible.unwrap().center();
                return (node.id,point,label.into());
            }
        }
        panic!("original sidebar route missing");
    }
    fn key(code:u16)->Event { Event::Key(KeyEvent{kind:KeyEventKind::Down,key_code:code,characters:String::new(),characters_ignoring_modifiers:String::new(),modifiers:Modifiers::NONE,is_repeat:false}) }
    fn assert_retained(host:&Host,selected:&str,text:&str,marked:Option<std::ops::Range<usize>>) {
        let state=host.settings.as_ref().unwrap().borrow();
        assert_eq!(state.selected,selected);
        assert_eq!(state.form.host.focused_text().unwrap().input.text,text);
        assert_eq!(state.form.host.focused_text().unwrap().input.marked,marked);
        assert!(state.form.has_number_draft() || state.form.host.is_composing());
    }
    #[test]
    fn actual_sidebar_pointer_keyboard_and_ax_refuse_then_accept_corrected_input() {
        for marked in [false,true] { for channel in ["pointer","key","ax"] {
            let (mut host,path)=fixture(marked);
            let (list,point,label)=row(&host,marked);
            if channel=="key" {assert!(host.settings.as_ref().unwrap().borrow_mut().sidebar.host.focus(Some(list),true));render(&mut host);}
            let publication=host.content_accessibility_publication(true).unwrap();
            let target=publication.nodes().iter().find(|n|n.label==label).unwrap().key;
            let event=match channel {
                "pointer"=>Event::Mouse(MouseEvent{kind:MouseEventKind::Down{button:MouseButton::Left,click_count:1},position:tekes_ui_platform_macos::Point{x:point.x as f64,y:point.y as f64},modifiers:Modifiers::NONE}),
                "key"=>key(if marked {36}else{125}),
                _=>Event::Accessibility{target,action:AccessibilityAction::Press},
            };
            let (selected,text,composition)={let s=host.settings.as_ref().unwrap().borrow();(s.selected.clone(),s.form.host.focused_text().unwrap().input.text.clone(),s.form.host.focused_text().unwrap().input.marked.clone())};
            assert!(host.settings_navigation_focus_guard(&event,Some(&publication)),"{channel} must refuse before focus/unmark");
            assert_retained(&host,&selected,&text,composition.clone());
            if channel == "ax" {
                let mut state=host.settings.as_ref().unwrap().borrow_mut();
                let focused=state.sidebar.host.focused();
                assert!(!state.accessibility_input(&publication,ContentAccessibilityOwner(1),target,AccessibilityAction::Press,true),"stale/refused publication cannot dispatch");
                assert_eq!(state.sidebar.host.focused(),focused,"refusal must not transfer keyboard ownership");
            }
            assert_retained(&host,&selected,&text,composition);
            assert!(host.settings.as_ref().unwrap().borrow_mut().take_item_navigation().unwrap().is_none());
            correct(&mut host,marked);
            let publication=host.content_accessibility_publication(true).unwrap();
            assert!(publication.is_current());
            assert!(!host.settings_navigation_focus_guard(&event,Some(&publication)),"corrected {channel} must remain usable");
            assert!(publication.is_current(),"allowed preflight must preserve the same multi-Host AX publication");
            let entry=match &event {
                Event::Accessibility{target,action}=>{let mut state=host.settings.as_ref().unwrap().borrow_mut();assert!(state.accessibility_input(&publication,ContentAccessibilityOwner(1),*target,action.clone(),true));assert_eq!(state.sidebar.host.focused(),Some(list),"successful List Press keeps original keyboard owner");assert_eq!(state.form.host.focused(),None,"old form focus retires only after successful AX delivery");state.take_item_navigation().unwrap()},
                _=>host.settings_sidebar_input(&crate::window::window_toolbar_mount::native_toolbar_event(&event).unwrap()).unwrap(),
            }.unwrap_or_else(|| panic!("original sidebar must produce its real settings-item Entry: marked={marked} channel={channel}"));
            let expected=if marked {"reading/text"}else{"reading/advanced"};
            assert_eq!(entry.route,"settings-item");assert_eq!(entry.params["item_id"],expected);
            host.settings.as_ref().unwrap().borrow_mut().select(expected).unwrap();
            assert_eq!(host.settings.as_ref().unwrap().borrow().selected,expected);
            assert!(host.settings.as_ref().unwrap().borrow().draft.dirty(),"corrected edits survive item publication");
            assert!(!path.exists());drop(host);std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }}
    }
    #[test]
    fn original_back_forward_producers_preserve_invalid_or_marked_then_allow_corrected_commands() {
        for marked in [false,true] { for name in ["Back","Forward"] {
            let (mut host,path)=fixture(marked);
            let publication=host.content_accessibility_publication(true).unwrap();
            let key=publication.nodes().iter().find(|n|n.label==name).unwrap().key;
            let event=Event::Accessibility{target:key,action:AccessibilityAction::Press};
            let (selected,text,composition)={let s=host.settings.as_ref().unwrap().borrow();(s.selected.clone(),s.form.host.focused_text().unwrap().input.text.clone(),s.form.host.focused_text().unwrap().input.marked.clone())};
            assert!(host.settings_navigation_focus_guard(&event,Some(&publication)));
            assert_retained(&host,&selected,&text,composition);
            assert!(host.toolbar.host.drain_commands().is_empty(),"refusal creates no transport request payload");
            correct(&mut host,marked);
            let publication=host.content_accessibility_publication(true).unwrap();
            assert!(publication.is_current());
            assert!(!host.settings_navigation_focus_guard(&event,Some(&publication)));
            assert!(publication.is_current(),"allowed toolbar AX preflight must not revoke form geometry");
            let target=host.toolbar_ax.iter().find(|(k,_)|*k==key).unwrap().1;
            assert!(host.toolbar.host.accessibility_action(target,ContentAction::Press).consumed);
            let invocations=host.toolbar.host.drain_commands();
            assert_eq!(invocations.len(),1);
            assert_eq!(invocations[0].command,if name=="Back" {crate::navigation::BACK}else{crate::navigation::FORWARD});
            assert!(matches!(&invocations[0].source,Source::Navigation(_)));
            assert!(host.prepare_settings_item_navigation(),"same production pre-RPC admission accepts corrected input");
            assert!(host.settings.as_ref().unwrap().borrow().draft.dirty());
            assert!(!path.exists());drop(host);std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }}
    }
    #[test]
    fn refused_navigation_keeps_other_committed_fields_scheduled_with_visible_feedback() {
        let (mut host,path)=fixture(false);
        {
            let mut state=host.settings.as_ref().unwrap().borrow_mut();
            assert!(state.form.host.focus(Some(ComponentId(102)),true));
            state.form.host.event(&ContentEvent::TextCommand("selectAll:".into()));
            state.form.host.event(&ContentEvent::Commit{text:"18".into(),replacement:None});
            assert!(!state.draft.dirty());
        }
        assert!(!host.prepare_settings_item_navigation());
        {
            let state=host.settings.as_ref().unwrap().borrow();
            let field=state.schema.settings.iter().find(|f|f.id.ends_with("/extra-number")).unwrap();
            assert_eq!(state.draft.display(field),"18");
            assert!(state.draft.dirty());
            assert!(state.needs_frame(),"valid partial commits keep original autosave scheduling");
            assert!(state.form.host.get(ComponentId(2)).unwrap().component::<tekes_ui_controls::composition::TextContent>().unwrap().text.contains("修正数值"));
        }
        assert!(!path.exists());drop(host);std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn original_back_rejects_before_focus_then_activates_original_close_barrier() {
        for marked in [false,true] { for channel in ["pointer","key","ax-focus","ax-press"] {
            let (mut host,path)=fixture_mode(marked,SettingsDisplayMode::Exclusive);
            if channel=="key" {
                assert!(host.settings.as_ref().unwrap().borrow_mut().sidebar.host.focus(Some(ComponentId(4)),true));
                render(&mut host);
            }
            let (point,mount)={let s=host.settings.as_ref().unwrap().borrow();(s.sidebar.host.get(ComponentId(4)).unwrap().frame().center(),s.sidebar.host.semantic_mount(ComponentId(4)).unwrap())};
            let publication=host.content_accessibility_publication(true).unwrap();
            let target=publication.nodes().iter().find(|n|n.key.component==mount && n.key.part.is_none()).unwrap().key;
            let event=match channel {
                "pointer"=>Event::Mouse(MouseEvent{kind:MouseEventKind::Down{button:MouseButton::Left,click_count:1},position:tekes_ui_platform_macos::Point{x:point.x as f64,y:point.y as f64},modifiers:Modifiers::NONE}),
                "key"=>key(49),
                _=>Event::Accessibility{target,action:if channel=="ax-focus"{AccessibilityAction::Focus}else{AccessibilityAction::Press}},
            };
            let (selected,text,composition)={let s=host.settings.as_ref().unwrap().borrow();(s.selected.clone(),s.form.host.focused_text().unwrap().input.text.clone(),s.form.host.focused_text().unwrap().input.marked.clone())};
            assert!(host.settings_navigation_focus_guard(&event,Some(&publication)),"Back {channel} must preserve invalid/marked input before focus");
            assert_retained(&host,&selected,&text,composition);
            {let mut s=host.settings.as_ref().unwrap().borrow_mut();assert!(s.open);assert!(!s.sidebar.take_back());}
            correct(&mut host,marked);
            let publication=host.content_accessibility_publication(true).unwrap();
            assert!(!host.settings_navigation_focus_guard(&event,Some(&publication)));
            assert!(publication.is_current(),"accepted Back focus preflight is read-only");
            match &event {
                Event::Accessibility{target,action}=>{
                    {let mut s=host.settings.as_ref().unwrap().borrow_mut();assert!(s.accessibility_input(&publication,ContentAccessibilityOwner(1),*target,action.clone(),true));assert!(s.take_item_navigation().unwrap().is_none(),"Back AX uses the original action consumption boundary without an item request");assert!(s.open);if channel=="ax-focus"{assert!(!s.sidebar.take_back(),"AX Focus does not activate Back");}}
                    if channel=="ax-focus" {
                        // A completed focus is a real input change; the next native
                        // Press uses the subsequent ordinary rendered publication.
                        render(&mut host);
                        let next=host.content_accessibility_publication(true).unwrap();
                        let press=Event::Accessibility{target:*target,action:AccessibilityAction::Press};
                        assert!(!host.settings_navigation_focus_guard(&press,Some(&next)));
                        assert!(next.is_current());
                        assert!(host.settings.as_ref().unwrap().borrow_mut().accessibility_input(&next,ContentAccessibilityOwner(1),*target,AccessibilityAction::Press,true));
                        assert!(host.settings.as_ref().unwrap().borrow_mut().take_item_navigation().unwrap().is_none());
                    }
                }
                _=>{
                    assert!(host.settings_sidebar_input(&crate::window::window_toolbar_mount::native_toolbar_event(&event).unwrap()).unwrap().is_none(),"Back never creates an item request");
                    {let mut s=host.settings.as_ref().unwrap().borrow_mut();assert!(s.open);assert!(!s.sidebar.take_back(),"down does not activate or close");}
                    let up=if channel=="pointer" {
                        Event::Mouse(MouseEvent{kind:MouseEventKind::Up{button:MouseButton::Left,click_count:1},position:tekes_ui_platform_macos::Point{x:point.x as f64,y:point.y as f64},modifiers:Modifiers::NONE})
                    } else {
                        let Event::Key(mut k)=key(49) else {unreachable!()};k.kind=KeyEventKind::Up;Event::Key(k)
                    };
                    assert!(host.settings_sidebar_input(&crate::window::window_toolbar_mount::native_toolbar_event(&up).unwrap()).unwrap().is_none());
                }
            }
            {let mut s=host.settings.as_ref().unwrap().borrow_mut();assert!(s.open,"activation leaves close policy to original consumer");assert!(s.sidebar.take_back(),"original Back producer activates: marked={marked} channel={channel}");assert!(!s.close_requested(),"corrected edits await actual persistence, not an early focus-close");assert!(s.open);assert!(s.draft.dirty());}
            assert!(!path.exists());drop(host);std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }}
    }

}
