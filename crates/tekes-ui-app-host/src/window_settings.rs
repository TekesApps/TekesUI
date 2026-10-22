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
    pub(super) fn drain_settings_back(&mut self) {
        if self
            .settings_back
            .as_mut()
            .is_some_and(|h| !h.drain_actions().is_empty())
        {
            self.settings
                .as_ref()
                .unwrap()
                .borrow_mut()
                .close_requested();
        }
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
        let back = surface.back_button.take();
        let surface = std::rc::Rc::new(std::cell::RefCell::new(surface));
        let columns = settings_columns(&surface);
        let app = crate::AppWindow::from_columns(columns)?
            .navigation(surface.borrow().navigation_options());
        let mut host = app.into_host(false);
        host.is_settings_window = true;
        host.leading_visible = true;
        host.trailing_visible = false;
        host.sidebar_motion = crate::sidebar_motion::SidebarMotion::new(true, false);
        host.outer.set_ratio(0.25);
        host.settings_back = back;
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
    pub(super) fn settings_accessibility_nodes(
        &mut self,
        active: bool,
    ) -> Vec<tekes_ui_platform_macos::AccessibilityNode> {
        let mut nodes = self.toolbar_accessibility_nodes(active);
        let active = active && self.open_toolbar().is_none();
        for (i, panel) in self.panels.iter_mut().enumerate() {
            if i == 0 && (!self.leading_visible || self.sidebar_motion.needs_frame()) {
                continue;
            }
            if let Some(local) = &mut panel.local {
                local.with_host(&mut |h| {
                    nodes.extend(crate::accessibility::content_nodes(h, active))
                });
            }
        }
        nodes
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
        while let Some(command) = navigation.take_external_action() {
            settings.undo_action(command)?;
        }
        navigation.set_external_actions(settings.toolbar_actions());
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

        let size = self.visibility_bars[0].prepare(
            renderer,
            w.scale() as f32,
            whole,
            &mut self.text,
            self.leading_visible,
            if self.leading_visible {
                "Hide leading panel"
            } else {
                "Show leading panel"
            },
        )?;
        navigation.set_breadcrumb(&settings.breadcrumb());
        let navigation_size =
            navigation.prepare(renderer, w.scale() as f32, whole, &mut self.text)?;
        let editing_size =
            navigation.prepare_external(renderer, w.scale() as f32, whole, &mut self.text)?;
        let title_size = navigation.prepare_title(&mut self.text, whole);

        let measured = size
            .height
            .max(navigation_size.height)
            .max(editing_size.height)
            .max(title_size.height);
        let native_center = w.set_toolbar_row_height(f64::from(measured));
        let (native, leading) = w.titlebar_safe_area();
        let (top, row_y, paint) = window_toolbar_geometry(
            native as f32,
            measured,
            native_center.map(|y| y as f32),
            w.scale() as f32,
        );
        let navigation_x = leading as f32 + size.width;
        let navigation_end = if self.leading_visible {
            frames[0].max_x()
        } else {
            whole.max_x()
        };
        let editing_width = editing_size
            .width
            .min((whole.max_x() - navigation_x).max(0.));
        let editing_x = whole.max_x() - editing_width;
        let navigation_width = navigation_size
            .width
            .min((navigation_end.min(editing_x) - navigation_x).max(0.));
        self.settings_top = top;
        self.visibility_bars[0].render(
            &mut toolbar_scene,
            &mut self.text,
            Rect::new(leading as f32, row_y, size.width, paint),
            whole,
        )?;
        navigation.render(
            &mut toolbar_scene,
            &mut self.text,
            Rect::new(navigation_x, row_y, navigation_width, paint),
            whole,
        )?;
        navigation.render_external(
            &mut toolbar_scene,
            &mut self.text,
            Rect::new(editing_x, row_y, editing_width, paint),
            whole,
        )?;
        let title_x = frames[1].min_x().max(navigation_x + navigation_width);
        navigation.render_title(
            &mut toolbar_scene,
            &mut self.text,
            Rect::new(
                title_x,
                row_y,
                title_size.width.min((editing_x - title_x).max(0.)),
                paint,
            ),
            whole,
        )?;
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
            let mut list_top = left.min_y() + top_gap;
            if let Some(back) = &mut self.settings_back {
                crate::settings::settings_back::prepare(
                    back,
                    renderer,
                    &self.text,
                    w.scale() as f32,
                    &mut self.toolbar_images,
                )?;
                let measured = back
                    .get_mut(ComponentId(1))
                    .unwrap()
                    .measure(&mut self.text, left.size)
                    .size;
                let frame = Rect::new(
                    left.min_x() + 16.,
                    list_top,
                    measured.width.min((left.size.width - 32.).max(1.)),
                    measured.height,
                );
                back.render(&mut toolbar_scene, &mut self.text, frame, left, palette)
                    .map_err(|e| anyhow::anyhow!("Settings Back: {e:?}"))?;
                list_top = frame.max_y() + top_gap;
            }
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
        let mut toolbar_controls = navigation.toolbar.pointer_regions();
        toolbar_controls.extend(navigation.external_toolbar.pointer_regions());
        toolbar_controls.extend(self.visibility_bars[0].host.pointer_regions());
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
        // The pinned release paints help in the original ContentHost render.
        // Composite that complete toolbar layer above ordinary content.
        for primitive in toolbar_scene.primitives() {
            scene.push(primitive.clone());
        }
        navigation.overlay(&mut scene, &mut self.text);

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
        let input_open =
            navigation.host.input_open && !navigation.busy && !shared_settings.borrow().restoring();
        let _ = navigation;
        let active = w.is_active();
        w.set_accessibility_nodes(self.settings_accessibility_nodes(active && input_open));
        w.set_text_input_state(if active && input_open {
            self.settings.as_ref().unwrap().borrow().text_state()
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
        let animate = (self.sidebar_motion.needs_frame()
            || self.text.animation_requested()
            || self.settings.as_ref().unwrap().borrow().needs_frame())
            && active;
        if !w.set_animating(animate) && animate {
            w.request_redraw();
        }
        Ok(())
    }

    fn settings_shell_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        if let Event::Accessibility { target, action } = event
            && self.toolbar_accessibility_action(w.is_active(), *target, action.clone())?
        {
            if matches!(
                action,
                tekes_ui_platform_macos::AccessibilityAction::Focus
                    | tekes_ui_platform_macos::AccessibilityAction::Press
            ) {
                let shared_settings = self.settings.as_ref().unwrap().clone();
                let mut state = shared_settings.borrow_mut();
                let settings = &mut *state;
                settings.form.host.focus(None, false);
                settings.sidebar.host.focus(None, false);
            }

            w.request_redraw();
            return Ok(true);
        }
        if self.open_toolbar().is_some() && matches!(event, Event::Accessibility { .. }) {
            // The original toolbar menu owns this input scope; AX must not
            // activate a form control behind it when the toolbar rejected a key.
            return Ok(true);
        }
        if w.is_active() && (self.open_toolbar().is_some() || self.toolbar_focus.is_some()) {
            match event {
                Event::Key(key) => {
                    if self.typography_key_input(key) {
                        w.request_redraw();
                        return Ok(true);
                    }
                    if !key.modifiers.command && !key.modifiers.control && !key.modifiers.alt {
                        self.toolbar_key_input(toolbar_key(key.clone()))?;
                        w.request_redraw();
                        return Ok(true);
                    }
                }
                Event::TextCommand(command) if command == "cancelOperation:" => {
                    self.cancel_toolbar_input();
                    w.request_redraw();
                    return Ok(true);
                }
                _ => {}
            }
        }
        let Event::Mouse(mouse) = event else {
            return Ok(false);
        };
        if !w.is_active() {
            return Ok(true);
        }
        let p = Point::new(mouse.position.x as f32, mouse.position.y as f32);
        let input = toolbar_pointer(*mouse);
        let mut handled = false;
        match mouse.kind {
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count,
            } => {
                if let Some(target) = self.toolbar_target_at(p) {
                    self.toolbar_pressed = Some(target);
                    self.set_toolbar_focus(Some(target), false);
                    let shared_settings = self.settings.as_ref().unwrap().clone();
                    let mut state = shared_settings.borrow_mut();
                    let settings = &mut *state;
                    settings.form.host.focus(None, false);
                    settings.sidebar.host.focus(None, false);
                    drop(state);
                    if self.dispatch_toolbar(target, &input)? {
                        self.cancel_toolbar_input();
                        if click_count == 2 {
                            w.try_titlebar_double_click();
                        } else {
                            w.try_begin_titlebar_drag();
                        }
                    }
                    handled = true;
                } else if self.leading_visible
                    && !self.sidebar_motion.needs_frame()
                    && self.outer_view.pointer_down(p, &mut self.outer)
                {
                    self.drag_split = Some(0);
                    handled = true;
                } else if p.y < self.settings_top {
                    if click_count == 2 {
                        w.try_titlebar_double_click();
                    } else {
                        w.try_begin_titlebar_drag();
                    }
                    handled = true;
                } else {
                    self.set_toolbar_focus(None, false);
                }
            }
            MouseEventKind::Up {
                button: MouseButton::Left,
                ..
            } => {
                if self.drag_split.take().is_some() {
                    self.outer_view.pointer_up(&mut self.outer);
                    handled = true;
                }
                if let Some(target) = self.toolbar_pressed.take() {
                    self.dispatch_toolbar(target, &input)?;
                    handled = true;
                }
            }
            MouseEventKind::Dragged {
                button: MouseButton::Left,
            } => {
                if self.drag_split == Some(0) {
                    self.outer_view.pointer_move(p, &mut self.outer);
                    handled = true;
                } else if let Some(target) = self.toolbar_pressed {
                    self.dispatch_toolbar(target, &input)?;
                    handled = true;
                }
            }
            MouseEventKind::Moved | MouseEventKind::Entered | MouseEventKind::Exited => {
                if self.toolbar_available(ToolbarTarget::SettingsBack) {
                    self.dispatch_toolbar(ToolbarTarget::SettingsBack, &input)?;
                }
                self.visibility_bars[0].event(&input);
                if let Some(nav) = &mut self.navigation {
                    nav.event(&input)?;
                    nav.external_event(&input)?;
                }
            }
            _ => {}
        }
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
        if matches!(
            event,
            Event::Accessibility {
                action: tekes_ui_platform_macos::AccessibilityAction::Focus
                    | tekes_ui_platform_macos::AccessibilityAction::Press,
                ..
            }
        ) {
            self.set_toolbar_focus(None, false);
        }
        let shared_settings = self.settings.as_ref().unwrap().clone();
        let mut state = shared_settings.borrow_mut();
        let settings = &mut *state;
        if let Event::Accessibility { target, action } = event
            && matches!(
                action,
                tekes_ui_platform_macos::AccessibilityAction::Focus
                    | tekes_ui_platform_macos::AccessibilityAction::Press
            )
        {
            if crate::accessibility::content_nodes(&settings.sidebar.host, true)
                .iter()
                .any(|n| n.key == *target)
            {
                settings.form.host.focus(None, false);
                let owner = settings
                    .sidebar
                    .host
                    .semantics()
                    .into_iter()
                    .find_map(|node| {
                        (settings.sidebar.host.semantic_mount(node.id) == Some(target.component))
                            .then_some(node.id)
                    });
                settings.sidebar.host.focus(owner, false);
            } else {
                settings.sidebar.host.focus(None, false);
            }
        }
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
                        settings.undo_action(if key.modifiers.shift {
                            crate::settings_undo::REDO
                        } else {
                            crate::settings_undo::UNDO
                        })?;
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
                crate::accessibility::content_action(
                    &mut settings.sidebar.host,
                    w.is_active() && self.leading_visible,
                    *target,
                    action.clone(),
                );
                if let Some(item) = settings.sidebar.take_selection() {
                    if self
                        .navigation
                        .as_ref()
                        .is_some_and(|n| n.host.input_open && !n.busy)
                    {
                        let entry = settings.entry(&item)?;
                        self.navigation.as_mut().unwrap().send(
                            crate::navigation::Command::Navigate(crate::navigation::Action::Push(
                                entry,
                            )),
                        )?;
                    }
                } else {
                    crate::accessibility::content_action(
                        &mut settings.form.host,
                        w.is_active(),
                        *target,
                        action.clone(),
                    );
                    intents.extend(
                        settings
                            .form
                            .collect(&settings.view_schema, &mut settings.draft),
                    );
                }
            }
            Event::Focus(false) => {
                settings.form.host.cancel();
                settings.flush(w.wake_proxy())?;
                w.set_accessibility_nodes(crate::accessibility::content_nodes(
                    &settings.form.host,
                    false,
                ));
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
            self.panels[0]
                .local
                .as_mut()
                .unwrap()
                .event(&event, &mut self.text)?;
            state = shared_settings.borrow_mut();
            let settings = &mut *state;
            if let Some(item) = settings.sidebar.take_selection() {
                let entry = settings.entry(&item)?;
                self.navigation
                    .as_mut()
                    .unwrap()
                    .send(crate::navigation::Command::Navigate(
                        crate::navigation::Action::Push(entry),
                    ))?;
            }
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
mod back_button_tests {
    use super::*;
    use crate::settings::{SettingsDisplayMode, SettingsSurface, SettingsWindow};
    use tekes_ui_platform_macos::AccessibilityAction;
    use tekes_ui_settings::{FileProvider, Snapshot};

    fn settings_host(mode: SettingsDisplayMode) -> Host {
        let path = std::env::temp_dir()
            .join(format!(
                "settings-back-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(
            include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml"),
            path.clone(),
        )
        .unwrap()
        .display_mode(mode);
        let provider = FileProvider::open(path, options.schema.namespace.clone()).unwrap();
        let mut surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
        surface.show();
        let mut host = Host::new(vec![], false, None);
        host.is_settings_window = true;
        host.leading_visible = true;
        host.sidebar_motion = crate::sidebar_motion::SidebarMotion::new(true, false);
        host.settings_back = surface.back_button.take();
        let surface = std::rc::Rc::new(std::cell::RefCell::new(surface));
        let columns = settings_columns(&surface);
        assert!(
            columns
                .iter()
                .all(|c| matches!(c, crate::Column::Container(_)))
        );
        let mut common = crate::AppWindow::from_columns(columns)
            .unwrap()
            .into_host(false);
        assert!(common.mount_columns());
        assert_eq!(common.panels.len(), 2);
        assert!(
            common
                .panels
                .iter()
                .all(|p| p.local.is_some() && p.launch_tx.is_none())
        );
        host.panels = std::mem::take(&mut common.panels);
        host.settings = Some(surface);
        if let Some(back) = host.settings_back.as_mut() {
            back.render(
                &mut Scene::new(),
                &mut host.text,
                Rect::new(16., 60., 80., 32.),
                Rect::new(0., 0., 900., 640.),
                Palette::from(Theme::LIGHT),
            )
            .unwrap();
        }
        host
    }
    #[test]
    fn exclusive_back_is_a_button_closes_settings_and_is_absent_in_independent() {
        let mut host = settings_host(SettingsDisplayMode::Exclusive);
        let back = host
            .toolbar_host(ToolbarTarget::SettingsBack)
            .unwrap()
            .get(ComponentId(1))
            .unwrap();
        assert!(back.component::<tekes_ui_controls::Button>().is_some());
        assert_eq!(back.help_text(), Some("返回主窗口"));

        let node = crate::accessibility::content_nodes(
            host.toolbar_host(ToolbarTarget::SettingsBack).unwrap(),
            true,
        )
        .into_iter()
        .find(|n| n.label == "Back")
        .unwrap();
        assert!(
            host.toolbar_accessibility_action(true, node.key, AccessibilityAction::Press)
                .unwrap()
        );
        assert!(!host.settings.as_ref().unwrap().borrow().open);
        let independent = settings_host(SettingsDisplayMode::Independent);
        assert!(independent.settings_back.is_none());
        assert!(
            !independent
                .toolbar_groups()
                .contains(&ToolbarTarget::SettingsBack)
        );
    }
    #[test]
    fn back_keeps_dirty_save_barrier_and_hidden_button_cannot_activate() {
        let mut host = settings_host(SettingsDisplayMode::Exclusive);
        let node = crate::accessibility::content_nodes(
            host.toolbar_host(ToolbarTarget::SettingsBack).unwrap(),
            true,
        )
        .into_iter()
        .find(|n| n.label == "Back")
        .unwrap();
        host.leading_visible = false;
        assert!(
            !host
                .toolbar_accessibility_action(true, node.key, AccessibilityAction::Press)
                .unwrap()
        );
        assert!(host.settings.as_ref().unwrap().borrow().open);
        host.leading_visible = true;
        let mut state = host.settings.as_ref().unwrap().borrow_mut();
        let settings = &mut *state;
        settings
            .draft
            .edit(&settings.schema.settings[0], "changed".into());
        drop(state);
        assert!(
            host.toolbar_accessibility_action(true, node.key, AccessibilityAction::Press)
                .unwrap()
        );
        assert!(host.settings.as_ref().unwrap().borrow().open);
        assert!(host.settings.as_ref().unwrap().borrow().draft.dirty());
    }
}
