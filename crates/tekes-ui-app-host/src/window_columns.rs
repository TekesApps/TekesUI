//! Local column routing uses the original ContentHost; process columns keep IPC.
use super::*;
impl Host {
    /// Preserve non-primary native gestures for local Content owners. Capture
    /// lives in the original Host, so hiding/unmounting/cancelling that owner
    /// retires it without a second window-level gesture state.
    pub(super) fn local_secondary_pointer(&mut self, event: &Event) -> Result<bool> {
        let Event::Mouse(mouse) = event else {
            return Ok(false);
        };
        let content_event = toolbar_pointer(*mouse);
        let ContentEvent::Pointer(pointer) = &content_event else {
            unreachable!()
        };
        if pointer.button.is_none() || pointer.is_primary() {
            return Ok(false);
        }
        if self.settings_mgr.is_settings_window
            || self.layout.sidebar_motion.needs_frame()
            || self.open_toolbar().is_some()
            || self
                .nav_mgr
                .navigation
                .as_ref()
                .is_some_and(|n| n.busy || !n.host.input_open)
        {
            return Ok(false);
        }
        if matches!(pointer.phase, PointerPhase::Drag | PointerPhase::Up) {
            for panel in &mut self.panel_mgr.panels {
                if let Some(local) = &mut panel.local
                    && local
                        .captured_pointer_event(*pointer, &mut self.text)?
                        .is_some()
                {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let index = if pointer.phase == PointerPhase::Down {
            self.panel_mgr.panels.iter().position(|panel| {
                panel.local.is_some()
                    && panel.content_frame().contains(pointer.position)
                    && (self.layout.presentation.is_none() || panel.clip.contains(pointer.position))
                    && panel.frame_viewport == panel.viewport
                    && panel.frame_size == panel.content_size()
            })
        } else {
            None
        };
        let Some(index) = index else { return Ok(false) };
        let response = self.panel_mgr.panels[index]
            .local
            .as_mut()
            .unwrap()
            .event_with_response(&content_event, &mut self.text)?;
        Ok(response.consumed)
    }
    pub(super) fn is_search_shortcut(event: &Event) -> bool {
        matches!(event, Event::Key(key) if key.kind == KeyEventKind::Down
            && key.modifiers.command && !key.modifiers.control && !key.modifiers.alt && !key.modifiers.shift && !key.modifiers.function
            && key.characters_ignoring_modifiers.eq_ignore_ascii_case("f"))
    }
    /// Resolve the declared destination only while ordinary input may change focus.
    fn available_search_entry(&mut self) -> Option<(usize, ComponentId)> {
        if self.layout.sidebar_motion.needs_frame()
            || self.open_toolbar().is_some()
            || self
                .nav_mgr
                .navigation
                .as_ref()
                .is_some_and(|n| n.busy || !n.host.input_open)
            || self.composition_in_progress()
            || self
                .layout
                .presentation_tabs
                .as_ref()
                .is_some_and(|tabs| tabs.content_host().input_overlay_owner().is_some())
            || self
                .layout
                .presentation_creation_grid
                .as_ref()
                .is_some_and(|grid| grid.host.input_overlay_owner().is_some())
        {
            return None;
        }
        for panel in &mut self.panel_mgr.panels {
            if let Some(local) = &mut panel.local {
                let mut overlay = false;
                local.with_host(&mut |host| overlay = host.input_overlay_owner().is_some());
                if overlay {
                    return None;
                }
            }
        }
        self.panel_mgr
            .panels
            .iter_mut()
            .enumerate()
            .find_map(|(index, panel)| {
                let local = panel.local.as_mut()?;
                let target = local.options.search_entry?;
                let mut enabled = false;
                local.with_host(&mut |host| {
                    enabled = host.text_surface(target).is_some_and(|surface| {
                        !surface.input().is_disabled() && !surface.input().is_read_only()
                    })
                });
                enabled.then_some((index, target))
            })
    }
    /// Use the existing presentation operations before the normal layout pass.
    /// A hidden Nav or Others destination retains its original backing Host.
    pub(super) fn reveal_registered_search(&mut self) -> Result<bool> {
        let Some((index, _)) = self.available_search_entry() else {
            return Ok(false);
        };
        let id = self.panel_mgr.panels[index].module_id.clone();
        if let Some(state) = &mut self.layout.presentation {
            if state.nav_bar_id() == Some(id.as_str()) {
                if !state.nav_bar_visible() {
                    state.toggle_nav_bar()?;
                }
            } else if state.focus_id() == id {
                if state.others_expanded() {
                    state.toggle_others_expanded()?;
                }
            } else {
                state.open(&id)?;
                if !state.others_visible() {
                    state.toggle_others()?;
                }
            }
            self.cancel_toolbar_input();
            self.layout.drag_split = None;
            self.layout.outer_view.cancel(&mut self.layout.outer);
            self.layout.inner_view.cancel(&mut self.layout.inner);
        }
        Ok(true)
    }
    /// Admit focus after layout and publish through the ordinary local text route.
    pub(super) fn focus_registered_search(&mut self) -> Result<bool> {
        let Some((index, target)) = self.available_search_entry() else {
            return Ok(false);
        };
        if self.panel_mgr.panels[index].bounds.is_empty() {
            return Ok(false);
        }
        let mut accepted = false;
        self.panel_mgr.panels[index]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                host.focus(Some(target), true);
                accepted = host.focused() == Some(target);
            });
        if !accepted {
            return Ok(false);
        }
        self.set_toolbar_focus(None, false);
        if let Some(tabs) = &mut self.layout.presentation_tabs {
            tabs.blur();
        }
        if let Some(grid) = &mut self.layout.presentation_creation_grid {
            grid.host.focus(None, false);
        }
        for (other, panel) in self.panel_mgr.panels.iter_mut().enumerate() {
            if other != index {
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |host| {
                        host.focus(None, false);
                    });
                } else {
                    panel.request(InputKind::Cancel, 0., 0.)?;
                }
            }
        }
        self.layout.focused = index;
        Ok(true)
    }
    /// Prepare all original Hosts in actual paint order; toolbar is composited last.
    pub(super) fn content_accessibility_publication(
        &mut self,
        active: bool,
    ) -> Option<tekes_ui_macos_bridge::content_accessibility::WindowAccessibilityPublication> {
        use tekes_ui_macos_bridge::content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot, WindowAccessibilityPublication,
        };
        if !active {
            return WindowAccessibilityPublication::prepare(Vec::new());
        }
        self.toolbar_accessibility_nodes(active); // refresh original stamped target map
        let mut fragments = Vec::new();
        if self.open_toolbar().is_none() {
            for (i, panel) in self.panel_mgr.panels.iter_mut().enumerate() {
                if panel.bounds.size.width <= 0.
                    || (self.settings_mgr.is_settings_window
                        && i == 0
                        && (!self.layout.leading_visible
                            || self.layout.sidebar_motion.needs_frame()))
                {
                    continue;
                }
                if let Some(local) = &mut panel.local {
                    let mut snapshot = None;
                    local.with_host(&mut |h| snapshot = ContentAccessibilitySnapshot::from_host(h));
                    fragments.push((ContentAccessibilityOwner(i as u64 + 1), snapshot?));
                }
            }
        }
        let modal = self
            .panel_mgr
            .panels
            .iter()
            .position(|panel| panel.mounted_toolbar.modal());
        for (index, panel) in self.panel_mgr.panels.iter().enumerate() {
            if modal.is_some_and(|owner| owner != index)
                || panel.mounted_toolbar.host.semantics().is_empty()
            {
                continue;
            }
            let mut keys = std::collections::HashMap::new();
            for node in panel.mounted_toolbar.host.semantics() {
                for (target, part) in std::iter::once((node.target, None)).chain(
                    node.parts
                        .iter()
                        .map(|(target, part)| (*target, Some(part.index))),
                ) {
                    if let Some((key, _, _)) = self
                        .toolbar_mgr
                        .toolbar_ax
                        .iter()
                        .find(|(_, i, t)| *i == index && *t == target)
                    {
                        keys.insert(
                            tekes_ui_macos_bridge::content_accessibility::content_key(
                                panel.mounted_toolbar.host.content_host(),
                                node.node.id,
                                part,
                            )?,
                            *key,
                        );
                    }
                }
            }
            let toolbar =
                ContentAccessibilitySnapshot::from_host(panel.mounted_toolbar.host.content_host())?
                    .with_native_keys(&keys)?;
            fragments.push((
                ContentAccessibilityOwner(Self::toolbar_accessibility_owner(index)),
                toolbar,
            ));
        }
        if modal.is_none()
            && let Some(tabs) = self.presentation_tabs_publication()
        {
            fragments.push(tabs);
        }
        if modal.is_none()
            && let Some(grid) = self.presentation_creation_grid_publication()
        {
            fragments.push(grid);
        }
        WindowAccessibilityPublication::prepare(fragments)
    }
    pub(super) fn local_text_state(
        &mut self,
        active: bool,
    ) -> Option<tekes_ui_platform_macos::TextInputState> {
        if !active
            || self.layout.sidebar_motion.needs_frame()
            || self.open_toolbar().is_some()
            || self
                .layout
                .presentation_tabs
                .as_ref()
                .is_some_and(|tabs| tabs.content_host().focused().is_some())
            || self
                .layout
                .presentation_creation_grid
                .as_ref()
                .is_some_and(|grid| grid.host.focused().is_some())
        {
            return None;
        }
        if let Some(state) = crate::columns::text_state(
            self.panel_mgr.panels[self.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .content_host(),
        ) {
            return Some(state);
        }
        if self.panel_mgr.panels[self.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_some()
        {
            return None;
        }
        let mut state = None;
        self.panel_mgr
            .panels
            .get_mut(self.layout.focused)?
            .local
            .as_mut()?
            .with_host(&mut |h| state = crate::columns::text_state(h));
        state
    }
    pub(super) fn handle_local_caret_blink(&mut self, active: bool) -> bool {
        if self
            .local_text_state(active)
            .is_some_and(|s| !s.read_only && s.selection.is_empty())
        {
            self.text.0.caret_visible = !self.text.0.caret_visible;
            true
        } else {
            self.text.0.caret_visible = true;
            false
        }
    }
    pub(super) fn local_content_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        if !w.is_active()
            || self
                .layout
                .presentation_tabs
                .as_ref()
                .is_some_and(|tabs| tabs.content_host().focused().is_some())
            || self
                .layout
                .presentation_creation_grid
                .as_ref()
                .is_some_and(|grid| grid.host.focused().is_some())
            || self
                .layout
                .presentation_creation_grid
                .as_ref()
                .is_some_and(|grid| grid.host.focused().is_some())
            || self.layout.sidebar_motion.needs_frame()
            || self.open_toolbar().is_some()
            || self
                .nav_mgr
                .navigation
                .as_ref()
                .is_some_and(|n| n.busy || !n.host.input_open)
        {
            return Ok(false);
        }
        if matches!(event, Event::CaretBlink) {
            return Ok(self.handle_local_caret_blink(w.is_active()));
        }
        if let Event::Accessibility { target, action } = event {
            use tekes_ui_macos_bridge::WindowContentAccessibilityExt;
            let Some(publication) = w.content_accessibility_publication() else {
                return Ok(false);
            };
            let Some(owner) = publication.owner(*target) else {
                return Ok(false);
            };
            let Some(index) = owner.0.checked_sub(1).and_then(|i| usize::try_from(i).ok()) else {
                return Ok(false);
            };
            let Some(panel) = self.panel_mgr.panels.get_mut(index) else {
                return Ok(false);
            };
            if panel.bounds.size.width <= 0. {
                return Ok(false);
            }
            let Some(local) = &mut panel.local else {
                return Ok(false);
            };
            let mut consumed = false;
            local.with_host(&mut |h| {
                consumed = publication
                    .dispatch(owner, h, *target, action.clone())
                    .consumed
            });
            if consumed {
                local.dispatch_actions();
                self.layout.focused = index;
                self.set_toolbar_focus(None, false);
            }
            return Ok(consumed);
        }
        self.local_content_input(event, w)
    }
    // Called after window activity, modal/navigation and accessibility routing.
    fn local_content_input(
        &mut self,
        event: &Event,
        w: &dyn tekes_ui_module_runtime::ContentClipboard,
    ) -> Result<bool> {
        let index = if let Event::Scroll(s) = event {
            let point = Point::new(s.position.x as f32, s.position.y as f32);
            self.panel_mgr.panels.iter().position(|p| {
                p.local.is_some()
                    && p.content_frame().contains(point)
                    && (self.layout.presentation.is_none() || p.clip.contains(point))
            })
        } else if self.panel_mgr.panels[self.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_none()
        {
            Some(self.layout.focused)
        } else {
            None
        };
        let Some(local) = index
            .and_then(|i| self.panel_mgr.panels.get_mut(i))
            .and_then(|p| p.local.as_mut())
        else {
            return Ok(false);
        };
        // Unclaimed application/window shortcuts must retain the outer fallback.
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Down
            && key.modifiers.command
            && (key.modifiers.control
                || key.modifiers.alt
                || (!matches!(key.key_code, 36 | 76)
                    && !matches!(
                        key.characters_ignoring_modifiers
                            .to_ascii_lowercase()
                            .as_str(),
                        "a" | "z" | "c" | "x" | "v"
                    )))
        {
            return Ok(false);
        }
        match event {
            Event::Key(_)
            | Event::TextKey(_)
            | Event::TextCommit { .. }
            | Event::TextPreedit { .. }
            | Event::TextUnmark
            | Event::TextCommand(_) => local.window_event(event, w, &mut self.text),
            Event::Scroll(s) => {
                if std::env::var_os("TEKESUI_APPHOST_BODY_SCROLL_TRACE").is_some() {
                    host_diag!(
                        "APPHOST_BODY_SCROLL owner={} position={:?} delta={:?} precise={} phase={:?} momentum={:?} timestamp={}",
                        local.id(),
                        s.position,
                        s.delta,
                        s.precise,
                        s.phase,
                        s.momentum_phase,
                        s.timestamp
                    );
                }
                local.window_event(event, w, &mut self.text)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_color_source_preserves_each_windows_appearance() {
        let source = std::rc::Rc::new(std::cell::Cell::new(tekes_ui_tokens::Theme::LIGHT.into()));
        let make = |theme| {
            crate::AppWindow::new(crate::AppProfile { modules: vec![] })
                .theme(theme)
                .palette_source(source.clone())
                .into_host(false)
        };
        let dark = make(tekes_ui_tokens::Theme::DARK);
        let light = make(tekes_ui_tokens::Theme::LIGHT);
        fn custom_accent(
            _id: usize,
            role: &str,
            _theme: tekes_ui_tokens::Theme,
        ) -> Option<tekes_ui_tokens::Rgba> {
            (role == "action-primary-ground").then(|| tekes_ui_tokens::Rgba::rgb8(18, 52, 86))
        }
        let custom =
            tekes_ui_tokens::CustomPaletteSource::new(9, "window-column-test", custom_accent);
        source.set(Palette::from_custom(
            tekes_ui_tokens::Theme::LIGHT_INCREASED,
            custom,
        ));
        assert_eq!(
            dark.palette(),
            Palette::from_custom(tekes_ui_tokens::Theme::DARK, custom)
        );
        assert_eq!(
            light.palette(),
            Palette::from_custom(tekes_ui_tokens::Theme::LIGHT, custom)
        );
        assert_eq!(
            source.get().theme,
            tekes_ui_tokens::Theme::LIGHT_INCREASED,
            "painting one window cannot rewrite another window's source"
        );
    }
    #[derive(Debug, Default)]
    struct PointerRecorder(Vec<PointerEvent>);
    impl tekes_ui_controls::composition::Component for PointerRecorder {
        fn measure(
            &mut self,
            _: &mut dyn tekes_ui_controls::ButtonTextRenderer,
            size: tekes_ui_scene::Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(size)
        }
        fn paint(
            &mut self,
            _: &mut Scene,
            _: &mut dyn tekes_ui_controls::ButtonTextRenderer,
            _: Rect,
            _: Rect,
            _: tekes_ui_controls::composition::ContentTheme,
        ) {
        }
        fn interactive(&self) -> bool {
            true
        }
        fn pointer_event(
            &mut self,
            p: &PointerEvent,
        ) -> tekes_ui_controls::composition::ContentResponse {
            self.0.push(*p);
            tekes_ui_controls::composition::ContentResponse {
                capture_pointer: p.phase == PointerPhase::Down,
                ..tekes_ui_controls::composition::ContentResponse::consumed(true)
            }
        }
    }
    #[test]
    fn local_secondary_drag_retains_metadata_across_toolbar_and_other_container() {
        use crate::{AppWindow, LocalContainer};
        use tekes_ui_controls::composition::*;
        let columns = ["first", "second"]
            .into_iter()
            .map(|name| {
                LocalContainer::new(
                    name,
                    Content::new(ComponentId(1), PointerRecorder::default()),
                )
                .unwrap()
                .into()
            })
            .collect();
        let mut window = AppWindow::from_columns(columns).unwrap().into_host(false);
        assert!(window.mount_columns());
        for (i, panel) in window.panel_mgr.panels.iter_mut().enumerate() {
            panel
                .geometry_with_header(Rect::new(i as f32 * 200., 0., 200., 260.), 1., 52., 0.)
                .unwrap();
            panel.frame_viewport = panel.viewport;
            panel.frame_size = panel.content_size();
            let frame = panel.content_frame();
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut window.text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
        }
        let mouse = |kind, x, y| {
            Event::Mouse(MouseEvent {
                kind,
                position: tekes_ui_platform_macos::Point { x, y },
                modifiers: tekes_ui_platform_macos::Modifiers {
                    shift: true,
                    ..Default::default()
                },
            })
        };
        let down = mouse(
            MouseEventKind::Down {
                button: MouseButton::Right,
                click_count: 2,
            },
            70.,
            100.,
        );
        assert!(window.local_secondary_pointer(&down).unwrap());
        assert!(
            window
                .local_secondary_pointer(&mouse(
                    MouseEventKind::Dragged {
                        button: MouseButton::Right
                    },
                    300.,
                    20.
                ))
                .unwrap()
        );
        assert!(
            window
                .local_secondary_pointer(&mouse(
                    MouseEventKind::Up {
                        button: MouseButton::Right,
                        click_count: 2
                    },
                    300.,
                    100.
                ))
                .unwrap()
        );
        window.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                let events = &host
                    .get(ComponentId(1))
                    .unwrap()
                    .component::<PointerRecorder>()
                    .unwrap()
                    .0;
                assert_eq!(events.len(), 3);
                assert!(
                    events
                        .iter()
                        .all(|e| e.button == Some(PointerButton::Secondary) && e.modifiers.shift)
                );
                assert_eq!(events[1].position, Point::new(300., 20.));
                assert!(host.pointer_capture().is_none());
            });
        window.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert!(
                    host.get(ComponentId(1))
                        .unwrap()
                        .component::<PointerRecorder>()
                        .unwrap()
                        .0
                        .is_empty()
                )
            });
        assert!(window.local_secondary_pointer(&down).unwrap());
        window.panel_mgr.panels[0].local.as_mut().unwrap().cancel();
        assert!(
            !window
                .local_secondary_pointer(&mouse(
                    MouseEventKind::Up {
                        button: MouseButton::Right,
                        click_count: 1
                    },
                    300.,
                    100.
                ))
                .unwrap()
        );
        window.settings_mgr.is_settings_window = true;
        assert!(!window.local_secondary_pointer(&down).unwrap());
    }
    #[test]
    fn mixed_columns_mount_without_launching_local_workers() {
        use crate::{AppWindow, LocalContainer, ProcessContainer};
        use tekes_ui_controls::composition::{Content, TextFieldContent};
        let local = |id| {
            LocalContainer::new(
                id,
                Content::new(ComponentId(1), TextFieldContent::new("Notes", "local")),
            )
            .unwrap()
        };
        let process = crate::ModuleDefinition {
            id: "remote",
            create: || panic!("not in host"),
        };
        let app = AppWindow::from_columns(vec![
            local("left").into(),
            ProcessContainer::new(process).into(),
            local("right").into(),
        ])
        .unwrap();
        let mut host = app.into_host(false);
        assert!(host.mount_columns());
        assert_eq!(host.panel_mgr.panels.len(), 3);
        assert!(host.panel_mgr.panels[0].local.is_some());
        assert!(host.panel_mgr.panels[1].local.is_none());
        assert!(host.panel_mgr.panels[2].local.is_some());
        assert!(!host.panel_mgr.panels[0].needs_launch());
        assert!(host.panel_mgr.panels[1].needs_launch());
        assert!(host.panel_mgr.panels.iter().all(|p| p.pid.is_none()));
    }
}

#[cfg(test)]
mod window_publication_tests {
    use super::*;
    use crate::window_toolbar::{Contents, Declaration, Source};
    use crate::{AppWindow, LocalContainer};
    use tekes_ui_controls::composition::*;
    use tekes_ui_controls::composition::{
        ToolbarButtonDeclaration as ButtonDecl, ToolbarGroupOverflow as Overflow,
    };
    use tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner;
    use tekes_ui_platform_macos::{AccessibilityAction, AccessibilityRole};
    use tekes_ui_tokens::Theme;
    #[test]
    fn actual_apphost_three_hosts_keep_original_owners_and_open_toolbar_excludes_background() {
        let columns = vec![
            LocalContainer::new(
                "curve",
                Content::new(
                    ComponentId(8100),
                    PanelContent::new(Content::new(
                        ComponentId(8101),
                        TextFieldContent::new("Composer", "draft"),
                    ))
                    .unwrap(),
                ),
            )
            .unwrap()
            .into(),
            LocalContainer::new(
                "plain",
                Content::new(ComponentId(8200), Button::new("Body button")),
            )
            .unwrap()
            .into(),
        ];
        let mut window = AppWindow::from_columns(columns).unwrap().into_host(false);
        assert!(window.mount_columns());
        for (i, panel) in window.panel_mgr.panels.iter_mut().enumerate() {
            panel.bounds = Rect::new(i as f32 * 180., 40., 180., 150.);
            let mut result = None;
            panel.local.as_mut().unwrap().with_host(&mut |h| {
                result = Some(h.render(
                    &mut Scene::new(),
                    &mut window.text,
                    panel.bounds,
                    panel.bounds,
                    Theme::LIGHT.into(),
                ))
            });
            result.unwrap().unwrap();
        }
        let source = Source::Panel {
            instance: window.panel_mgr.panels[0].instance,
            index: 0,
            module: "curve".into(),
            pid: None,
            generation: 0,
        };
        window.panel_mgr.panels[window.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .sync(
                Rect::new(0., 0., 360., 200.),
                vec![ToolbarRegion::new(
                    ComponentId(10),
                    Rect::new(0., 0., 70., 40.),
                )],
                vec![Declaration {
                    id: ComponentId(100),
                    region: ComponentId(10),
                    placement: ToolbarPlacement::Navigation,
                    order: 0,
                    contents: Contents::Commands(
                        vec![ButtonDecl {
                            id: 7,
                            icon: None,
                            text: "Long original command".into(),
                            hint: None,
                            enabled: true,
                        }],
                        Overflow::Together,
                    ),
                    source,
                }],
            )
            .unwrap();
        window.panel_mgr.panels[window.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        assert!(
            !window.panel_mgr.panels[window.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .modal()
        );
        let publication = window.content_accessibility_publication(true).unwrap();
        let composer = publication
            .nodes()
            .iter()
            .find(|n| n.label == "Composer")
            .unwrap()
            .key;
        let body = publication
            .nodes()
            .iter()
            .find(|n| n.label == "Body button")
            .unwrap()
            .key;
        assert_eq!(
            publication.owner(composer),
            Some(ContentAccessibilityOwner(1))
        );
        assert_eq!(publication.owner(body), Some(ContentAccessibilityOwner(2)));
        assert!(publication.text_layouts().unwrap().contains_key(&composer));
        let overflow = publication
            .nodes()
            .iter()
            .find(|n| n.role == AccessibilityRole::MenuButton)
            .unwrap()
            .key;
        assert_eq!(
            publication.owner(overflow),
            Some(ContentAccessibilityOwner(
                Host::toolbar_accessibility_owner(window.toolbar_mgr.toolbar_index)
            ))
        );
        assert!(
            window
                .toolbar_accessibility_action(true, overflow, AccessibilityAction::Press)
                .unwrap()
        );
        window.panel_mgr.panels[window.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        assert!(
            window.panel_mgr.panels[window.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .modal()
        );
        let modal = window.content_accessibility_publication(true).unwrap();
        assert!(modal.owner(composer).is_none());
        assert!(modal.owner(body).is_none());
        assert!(!modal.nodes().is_empty());
        assert!(modal.nodes().iter().all(|n| modal.owner(n.key)
            == Some(ContentAccessibilityOwner(
                Host::toolbar_accessibility_owner(window.toolbar_mgr.toolbar_index)
            ))));
        assert!(
            window
                .content_accessibility_publication(false)
                .unwrap()
                .nodes()
                .is_empty()
        );
    }
}

#[cfg(test)]
mod local_clipboard_tests {
    use super::*;
    use crate::{AppWindow, LocalContainer};
    use std::cell::RefCell;
    use tekes_ui_controls::composition::{Content, TextFieldContent};
    use tekes_ui_module_runtime::ContentClipboard;
    use tekes_ui_tokens::Theme;

    struct Clipboard {
        text: RefCell<Option<String>>,
        writes: RefCell<Vec<String>>,
    }
    impl Clipboard {
        fn new(text: Option<&str>) -> Self {
            Self {
                text: RefCell::new(text.map(str::to_owned)),
                writes: RefCell::new(vec![]),
            }
        }
    }
    impl ContentClipboard for Clipboard {
        fn clipboard_text(&self) -> Option<String> {
            self.text.borrow().clone()
        }
        fn set_clipboard_text(&self, text: &str) {
            self.writes.borrow_mut().push(text.into());
            *self.text.borrow_mut() = Some(text.into());
        }
    }
    fn host(selection: std::ops::Range<usize>) -> Host {
        let local = LocalContainer::new(
            "editor",
            Content::new(ComponentId(1), TextFieldContent::new("Notes", "abc")),
        )
        .unwrap();
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let local = host.panel_mgr.panels[0].local.as_mut().unwrap();
        let frame = Rect::new(0., 40., 300., 160.);
        local
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        local.with_host(&mut |h| {
            h.focus(Some(ComponentId(1)), true);
            assert!(
                h.get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<TextFieldContent>()
                    .unwrap()
                    .select_range(selection.clone())
            );
        });
        host
    }
    #[test]
    fn registered_search_shortcut_routes_container_scoped_id_and_native_text() {
        use tekes_ui_controls::composition::SearchFieldContent;
        // Both containers deliberately use ID 1; registration must route by
        // container identity, never by a global ComponentId scan.
        let search = LocalContainer::new(
            "navigation",
            Content::new(ComponentId(1), SearchFieldContent::new("Find pages", "")),
        )
        .unwrap()
        .search_entry(ComponentId(1));
        let editor = LocalContainer::new(
            "editor",
            Content::new(ComponentId(1), TextFieldContent::new("Draft", "retained")),
        )
        .unwrap();
        let mut host = AppWindow::from_columns(vec![search.into(), editor.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        for (index, panel) in host.panel_mgr.panels.iter_mut().enumerate() {
            let frame = Rect::new(index as f32 * 320., 40., 300., 160.);
            panel.bounds = frame;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        host.layout.focused = 1;
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert!(host.focus(Some(ComponentId(1)), true));
            });
        assert!(Host::is_search_shortcut(&shortcut("f")));
        assert!(!Host::is_search_shortcut(&shortcut("g")));
        assert!(host.focus_registered_search().unwrap());
        assert_eq!(host.layout.focused, 0);
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| assert!(host.focused().is_none()));
        // Match the production redraw before subsequent native text input.
        for panel in &mut host.panel_mgr.panels {
            let frame = panel.bounds;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        let clipboard = Clipboard::new(None);
        assert!(
            host.local_content_input(
                &Event::TextCommit {
                    text: "query".into(),
                    replacement: None
                },
                &clipboard
            )
            .unwrap()
        );
        let frame = host.panel_mgr.panels[0].bounds;
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(host.local_text_state(true).is_some());
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert_eq!(host.text_surface(ComponentId(1)).unwrap().text(), "query");
                assert!(crate::columns::text_state(host).is_some());
            });
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert_eq!(
                    host.text_surface(ComponentId(1)).unwrap().text(),
                    "retained"
                )
            });
        // Marked IME text is a real cross-container focus barrier.
        host.local_content_input(
            &Event::TextPreedit {
                text: "中".into(),
                selection: 0..1,
                clauses: vec![],
                replacement: None,
            },
            &clipboard,
        )
        .unwrap();
        assert!(!host.focus_registered_search().unwrap());
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| assert!(host.is_composing()));
    }
    #[test]
    fn registered_search_reveals_hidden_navigation_before_focus_and_text_input() {
        use tekes_ui_controls::composition::SearchFieldContent;
        let search = LocalContainer::new(
            "nav",
            Content::new(ComponentId(1), SearchFieldContent::new("Find pages", "")),
        )
        .unwrap()
        .search_entry(ComponentId(1));
        let editor = LocalContainer::new(
            "editor",
            Content::new(ComponentId(1), TextFieldContent::new("Draft", "retained")),
        )
        .unwrap();
        let mut host = AppWindow::from_containers(
            vec![search.into(), editor.into()],
            crate::PresentationOptions::new("editor").nav_bar("nav"),
        )
        .unwrap()
        .into_host(false);
        assert!(host.mount_columns());
        let paint = |host: &mut Host| {
            let layout = host
                .layout
                .presentation
                .as_ref()
                .unwrap()
                .layout(Rect::new(0., 40., 900., 600.));
            let frames = host.apply_presentation_layout(layout).unwrap();
            for (panel, frame) in host.panel_mgr.panels.iter_mut().zip(frames) {
                panel.bounds = frame;
                if !frame.is_empty() {
                    panel
                        .local
                        .as_mut()
                        .unwrap()
                        .render(
                            &mut Scene::new(),
                            &mut host.text,
                            frame,
                            panel.clip,
                            Theme::LIGHT.into(),
                        )
                        .unwrap();
                }
            }
        };
        paint(&mut host);
        host.toggle_visibility(0).unwrap();
        paint(&mut host);
        assert!(!host.layout.presentation.as_ref().unwrap().nav_bar_visible());
        assert!(host.panel_mgr.panels[0].bounds.is_empty());
        assert!(host.reveal_registered_search().unwrap());
        paint(&mut host);
        assert!(host.layout.presentation.as_ref().unwrap().nav_bar_visible());
        assert!(!host.panel_mgr.panels[0].bounds.is_empty());
        assert!(host.focus_registered_search().unwrap());
        paint(&mut host);
        assert_eq!(host.layout.focused, 0);
        assert!(host.local_text_state(true).is_some());
        assert!(
            host.local_content_input(
                &Event::TextCommit {
                    text: "found".into(),
                    replacement: None
                },
                &Clipboard::new(None)
            )
            .unwrap()
        );
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                assert_eq!(h.focused(), Some(ComponentId(1)));
                assert_eq!(h.text_surface(ComponentId(1)).unwrap().text(), "found");
            });
    }
    #[test]
    fn registered_search_rejects_duplicate_missing_and_disabled_targets() {
        use tekes_ui_controls::composition::SearchFieldContent;
        let search = |name| {
            LocalContainer::new(
                name,
                Content::new(ComponentId(1), SearchFieldContent::new("Search", "")),
            )
            .unwrap()
            .search_entry(ComponentId(1))
        };
        assert!(AppWindow::from_columns(vec![search("a").into(), search("b").into()]).is_err());
        let mut host = AppWindow::from_columns(vec![search("a").into()])
            .unwrap()
            .into_host(false);
        host.mount_columns();
        let frame = Rect::new(0., 40., 300., 100.);
        host.panel_mgr.panels[0].bounds = frame;
        let local = host.panel_mgr.panels[0].local.as_mut().unwrap();
        local
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        local.options.search_entry = Some(ComponentId(99));
        assert!(!host.focus_registered_search().unwrap());
        let local = host.panel_mgr.panels[0].local.as_mut().unwrap();
        local.options.search_entry = Some(ComponentId(1));
        local.with_host(&mut |host| {
            host.get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<SearchFieldContent>()
                .unwrap()
                .set_availability(false, true);
        });
        assert!(!host.focus_registered_search().unwrap());
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| assert!(host.focused().is_none()));
    }
    fn value(host: &mut Host) -> String {
        let mut value = String::new();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                value = h
                    .get(ComponentId(1))
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap()
                    .text()
                    .to_owned();
            });
        value
    }
    #[test]
    fn local_clipboard_empty_cut_preserves_text_and_clipboard() {
        let mut host = host(3..3);
        let clipboard = Clipboard::new(Some("keep"));
        assert!(
            host.local_content_input(
                &Event::TextCommand(tekes_ui_platform_macos::TextEditCommand::Cut),
                &clipboard,
            )
            .unwrap()
        );
        assert_eq!(value(&mut host), "abc");
        assert_eq!(clipboard.clipboard_text().as_deref(), Some("keep"));
        assert!(clipboard.writes.borrow().is_empty());
    }
    #[test]
    fn local_clipboard_no_text_paste_preserves_selection_and_clipboard() {
        let mut host = host(1..3);
        let clipboard = Clipboard::new(None);
        assert!(
            host.local_content_input(
                &Event::TextCommand(tekes_ui_platform_macos::TextEditCommand::Paste),
                &clipboard,
            )
            .unwrap()
        );
        assert_eq!(value(&mut host), "abc");
        assert_eq!(clipboard.clipboard_text(), None);
        assert!(clipboard.writes.borrow().is_empty());
    }
    fn shortcut(letter: &str) -> Event {
        Event::Key(tekes_ui_platform_macos::KeyEvent {
            kind: KeyEventKind::Down,
            key_code: if letter == "x" { 7 } else { 9 },
            characters: letter.into(),
            characters_ignoring_modifiers: letter.into(),
            modifiers: tekes_ui_platform_macos::Modifiers {
                command: true,
                ..Default::default()
            },
            is_repeat: false,
        })
    }
    #[test]
    fn local_clipboard_raw_empty_cut_and_no_text_paste_are_noops() {
        for (letter, selection) in [("x", 3..3), ("v", 1..3)] {
            let mut host = host(selection.clone());
            let clipboard = Clipboard::new(if letter == "x" { Some("keep") } else { None });
            let before = clipboard.clipboard_text();
            assert!(
                host.local_content_input(&shortcut(letter), &clipboard)
                    .unwrap()
            );
            assert_eq!(value(&mut host), "abc");
            host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .with_host(&mut |h| {
                    assert_eq!(h.focused_text().unwrap().input().selection(), selection);
                });
            assert_eq!(clipboard.clipboard_text(), before);
            assert!(clipboard.writes.borrow().is_empty());
        }
    }
    #[test]
    fn local_clipboard_unclaimed_command_shortcuts_keep_window_fallback() {
        for (letter, control, alt) in [("q", false, false), ("x", true, false), ("v", false, true)]
        {
            let mut host = host(1..3);
            let clipboard = Clipboard::new(Some("keep"));
            let Event::Key(mut key) = shortcut(letter) else {
                unreachable!()
            };
            key.modifiers.control = control;
            key.modifiers.alt = alt;
            assert!(
                !host
                    .local_content_input(&Event::Key(key), &clipboard)
                    .unwrap()
            );
            assert_eq!(value(&mut host), "abc");
            assert_eq!(clipboard.clipboard_text().as_deref(), Some("keep"));
            assert!(clipboard.writes.borrow().is_empty());
        }
    }
    #[test]
    fn local_rich_link_command_enter_native_span_activates_once() {
        use std::rc::Rc;
        use tekes_ui_controls::composition::{AccessibilityAction, RichContent, RichDocument};
        use tekes_ui_platform_macos::{TextKeyContext, TextKeyDisposition, TextKeyEvent};
        for code in [36, 76] {
            for (control, alt, expected) in [(false, false, 1), (true, false, 0), (false, true, 0)]
            {
                let actions = Rc::new(RefCell::new(Vec::new()));
                let received = actions.clone();
                let local = LocalContainer::new(
                    "reader",
                    Content::new(
                        ComponentId(1),
                        RichContent::new(
                            "manual",
                            1,
                            RichDocument::from_markdown("[Number](number-field.md)"),
                        )
                        .unwrap(),
                    ),
                )
                .unwrap()
                .on_action(move |host, _| {
                    received.borrow_mut().extend(
                        host.get_mut(ComponentId(1))
                            .unwrap()
                            .component_mut::<RichContent>()
                            .unwrap()
                            .drain_link_actions(),
                    );
                });
                let mut host = AppWindow::from_columns(vec![local.into()])
                    .unwrap()
                    .into_host(false);
                assert!(host.mount_columns());
                let local = host.panel_mgr.panels[0].local.as_mut().unwrap();
                let frame = Rect::new(0., 40., 300., 160.);
                local
                    .render(
                        &mut Scene::new(),
                        &mut host.text,
                        frame,
                        frame,
                        Theme::LIGHT.into(),
                    )
                    .unwrap();
                local.with_host(&mut |h| {
                    assert!(h.focus(Some(ComponentId(1)), true));
                    h.accessibility_action(
                        ComponentId(1),
                        None,
                        AccessibilityAction::SetSelection(2..2),
                    );
                });
                let Event::Key(mut key) = shortcut("\r") else {
                    unreachable!()
                };
                key.key_code = code;
                key.modifiers.control = control;
                key.modifiers.alt = alt;
                let context = TextKeyContext {
                    id: 292,
                    key: key.clone(),
                    composing_at_start: false,
                };
                let clipboard = Clipboard::new(Some("keep"));
                for event in [
                    Event::TextKey(TextKeyEvent::Started(context.clone())),
                    Event::Key(key),
                    Event::TextKey(TextKeyEvent::Finished {
                        context,
                        input_context_handled: None,
                        disposition: TextKeyDisposition::NotOffered,
                        text_callbacks: 0,
                        command_callbacks: 0,
                    }),
                ] {
                    host.local_content_input(&event, &clipboard).unwrap();
                }
                let actions = actions.borrow();
                assert_eq!(
                    actions.len(),
                    expected,
                    "code={code} ctrl={control} alt={alt}"
                );
                if expected == 1 {
                    assert_eq!(actions[0].target, "number-field.md");
                }
                assert!(clipboard.writes.borrow().is_empty());
                assert_eq!(clipboard.clipboard_text().as_deref(), Some("keep"));
            }
        }
    }
    #[test]
    fn local_clipboard_native_span_raw_and_selector_edit_once() {
        use tekes_ui_platform_macos::{TextKeyContext, TextKeyDisposition, TextKeyEvent};
        for (letter, command, expected) in [
            ("x", tekes_ui_platform_macos::TextEditCommand::Cut, "a"),
            ("v", tekes_ui_platform_macos::TextEditCommand::Paste, "aNEW"),
        ] {
            let mut host = host(1..3);
            let clipboard = Clipboard::new(Some("NEW"));
            let Event::Key(key) = shortcut(letter) else {
                unreachable!()
            };
            let context = TextKeyContext {
                id: 270,
                key: key.clone(),
                composing_at_start: false,
            };
            for event in [
                Event::TextKey(TextKeyEvent::Started(context.clone())),
                Event::Key(key),
                Event::TextCommand(command),
                Event::TextKey(TextKeyEvent::Finished {
                    context,
                    input_context_handled: None,
                    disposition: TextKeyDisposition::NotOffered,
                    text_callbacks: 0,
                    command_callbacks: 1,
                }),
            ] {
                assert!(host.local_content_input(&event, &clipboard).unwrap());
            }
            assert_eq!(value(&mut host), expected);
            assert_eq!(
                clipboard.writes.borrow().as_slice(),
                if letter == "x" { vec!["bc"] } else { vec![] }
            );
            // One undo restores the complete edit; a second native callback did not edit.
            host.local_content_input(
                &Event::TextCommand(tekes_ui_platform_macos::TextEditCommand::Undo),
                &clipboard,
            )
            .unwrap();
            assert_eq!(value(&mut host), "abc");
        }
    }
    #[test]
    fn local_clipboard_readonly_disabled_and_marked_reject_edits() {
        for mode in ["readonly", "disabled", "marked"] {
            for event in [
                shortcut("x"),
                shortcut("v"),
                Event::TextCommand(tekes_ui_platform_macos::TextEditCommand::Cut),
                Event::TextCommand(tekes_ui_platform_macos::TextEditCommand::Paste),
            ] {
                let mut host = host(1..3);
                let clipboard = Clipboard::new(Some("keep"));
                host.panel_mgr.panels[0]
                    .local
                    .as_mut()
                    .unwrap()
                    .with_host(&mut |h| {
                        let field = h
                            .get_mut(ComponentId(1))
                            .unwrap()
                            .component_mut::<TextFieldContent>()
                            .unwrap();
                        match mode {
                            "readonly" => field.set_read_only(true),
                            "disabled" => field.set_disabled(true),
                            _ => assert!(field.set_marked_range(Some(1..3))),
                        }
                    });
                assert!(host.local_content_input(&event, &clipboard).unwrap());
                assert_eq!(value(&mut host), "abc", "{mode}");
                assert_eq!(
                    clipboard.clipboard_text().as_deref(),
                    Some("keep"),
                    "{mode}"
                );
                assert!(clipboard.writes.borrow().is_empty(), "{mode}");
            }
        }
    }
    #[test]
    fn local_clipboard_cut_uses_exact_utf16_selection_and_copy_keeps_readonly_text() {
        for command in [
            tekes_ui_platform_macos::TextEditCommand::Cut,
            tekes_ui_platform_macos::TextEditCommand::Copy,
        ] {
            let mut host = host(1..3);
            let clipboard = Clipboard::new(Some("keep"));
            host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .with_host(&mut |h| {
                    let field = h
                        .get_mut(ComponentId(1))
                        .unwrap()
                        .component_mut::<TextFieldContent>()
                        .unwrap();
                    field.replace_value(
                        "A😀B",
                        tekes_ui_controls::ExternalSelection::Reset,
                        tekes_ui_controls::ExternalUndo::Reset,
                    );
                    field.select_range(1..3);
                    field.set_read_only(command == tekes_ui_platform_macos::TextEditCommand::Copy);
                });
            host.local_content_input(&Event::TextCommand(command), &clipboard)
                .unwrap();
            assert_eq!(
                value(&mut host),
                if command == tekes_ui_platform_macos::TextEditCommand::Cut {
                    "AB"
                } else {
                    "A😀B"
                }
            );
            assert_eq!(clipboard.writes.borrow().as_slice(), ["😀"]);
        }
    }
    #[test]
    fn local_clipboard_keeps_text_service_layout_for_noop_and_edit() {
        let mut host = host(3..3);
        let clipboard = Clipboard::new(Some("keep"));
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                h.get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<TextFieldContent>()
                    .unwrap()
                    .invalidate_text_layout();
            });
        host.local_content_input(
            &Event::TextCommand(tekes_ui_platform_macos::TextEditCommand::Cut),
            &clipboard,
        )
        .unwrap();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                assert!(h.focused_text().unwrap().paragraph().is_some());
                // The same service scope prepares before a platform snapshot/claim,
                // and refreshes again after the dispatch mutates the original field.
                h.get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<TextFieldContent>()
                    .unwrap()
                    .invalidate_text_layout();
                h.with_text_input(&mut host.text, |h| {
                    assert!(h.focused_text().unwrap().paragraph().is_some());
                    h.event(&ContentEvent::Commit {
                        text: "!".into(),
                        replacement: None,
                    });
                })
                .unwrap();
                assert!(h.focused_text().unwrap().paragraph().is_some());
                assert_eq!(h.focused_text().unwrap().text(), "abc!");
                assert_eq!(h.focused_text().unwrap().paragraph().unwrap().utf16_len, 4);
            });
    }
}

#[cfg(test)]
mod local_scroll_tests {
    use super::*;
    use crate::{AppWindow, LocalContainer};
    use std::{cell::RefCell, rc::Rc};
    use tekes_ui_controls::{
        ButtonTextRenderer, ScrollArea,
        composition::{Component, Content, ContentResponse, ScrollContent},
        layout::MeasuredChild,
    };
    use tekes_ui_module_runtime::ContentClipboard;
    use tekes_ui_platform_macos::{ScrollEvent, ScrollPhase, Vec2};
    use tekes_ui_scene::Size;
    use tekes_ui_tokens::Theme;

    struct NoClipboard;
    impl ContentClipboard for NoClipboard {
        fn clipboard_text(&self) -> Option<String> {
            None
        }
        fn set_clipboard_text(&self, _: &str) {
            panic!("scroll must not write clipboard")
        }
    }
    #[derive(Debug)]
    struct Document(Rc<RefCell<Vec<ContentEvent>>>);
    impl Component for Document {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(300., 1200.))
        }
        fn interactive(&self) -> bool {
            true
        }
        fn event(&mut self, event: &ContentEvent) -> ContentResponse {
            if matches!(event, ContentEvent::Scroll { .. }) {
                self.0.borrow_mut().push(event.clone());
            }
            ContentResponse::default() // bubble to the original ScrollContent
        }
    }
    fn host() -> (Host, Rc<RefCell<Vec<ContentEvent>>>) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let local = LocalContainer::new(
            "reader",
            Content::new(
                ComponentId(1),
                ScrollContent::new(
                    ScrollArea::default(),
                    Content::new(ComponentId(2), Document(events.clone())),
                ),
            ),
        )
        .unwrap();
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let frame = Rect::new(0., 40., 300., 160.);
        let panel = &mut host.panel_mgr.panels[0];
        panel.bounds = frame;
        panel.geometry.content = frame;
        panel.content_padding = 0.;
        panel
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        (host, events)
    }
    fn scroll(host: &mut Host, mut visit: impl FnMut(&mut ScrollContent)) {
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                visit(
                    h.get_mut(ComponentId(1))
                        .unwrap()
                        .component_mut::<ScrollContent>()
                        .unwrap(),
                );
            });
    }
    fn native(phase: ScrollPhase, momentum_phase: ScrollPhase, y: f64) -> ScrollEvent {
        ScrollEvent {
            timestamp: 123.5,
            delta: Vec2 { x: 0., y },
            precise: true,
            phase,
            momentum_phase,
            inverted: true,
            position: tekes_ui_platform_macos::Point { x: 100., y: 100. },
            modifiers: tekes_ui_platform_macos::Modifiers::NONE,
        }
    }
    fn send(host: &mut Host, event: ScrollEvent) {
        assert!(
            host.local_content_input(&Event::Scroll(event), &NoClipboard)
                .unwrap()
        );
    }
    #[test]
    fn local_body_area_native_gesture_keeps_owner_inside_same_container() {
        use tekes_ui_controls::composition::{TextFieldContent, TextViewport};
        let field = TextFieldContent::from_surface(
            "Area",
            tekes_ui_controls::TextSurface::new(&"editable line\n".repeat(80), true, false),
        )
        .with_show_metadata(false)
        .with_viewport(TextViewport::Fixed(90.));
        let local = LocalContainer::new("area", Content::new(ComponentId(1), field)).unwrap();
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let frame = Rect::new(0., 40., 300., 160.);
        let panel = &mut host.panel_mgr.panels[0];
        panel.bounds = frame;
        panel.geometry.content = frame;
        panel.content_padding = 0.;
        panel
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        panel.local.as_mut().unwrap().with_host(&mut |h| {
            h.focus(Some(ComponentId(1)), true);
        });
        let offset = |host: &mut Host| {
            let mut y = 0.;
            host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .with_host(&mut |h| {
                    y = h.text_surface(ComponentId(1)).unwrap().scroll().offset.y;
                });
            y
        };
        send(&mut host, native(ScrollPhase::Began, ScrollPhase::None, 0.));
        assert_eq!(offset(&mut host), 0.);
        let mut event = native(ScrollPhase::Changed, ScrollPhase::None, -20.);
        event.position.y = 145.; // Outside the 90pt editor, inside the same 160pt container.
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                assert!(
                    !h.text_surface(ComponentId(1))
                        .unwrap()
                        .outer()
                        .contains(Point::new(100., 145.))
                );
            });
        assert!(
            host.panel_mgr.panels[0]
                .content_frame()
                .contains(Point::new(100., 145.))
        );
        send(&mut host, event);
        let changed = offset(&mut host);
        assert!(changed > 0.);
        event.phase = ScrollPhase::Ended;
        event.delta.y = 0.;
        send(&mut host, event);
        event.phase = ScrollPhase::None;
        event.momentum_phase = ScrollPhase::Changed;
        event.delta.y = -20.;
        send(&mut host, event);
        assert!(offset(&mut host) > changed);
        event.momentum_phase = ScrollPhase::Ended;
        event.delta.y = 0.;
        send(&mut host, event);
        let ended = offset(&mut host);
        event.momentum_phase = ScrollPhase::Changed;
        event.delta.y = -20.;
        send(&mut host, event);
        assert_eq!(
            offset(&mut host),
            ended,
            "local native tail cannot revive the retired owner"
        );
    }

    #[test]
    fn body_scroll_preserves_native_phases_momentum_time_and_zero_end_packets() {
        let (mut host, events) = host();
        for (phase, momentum, begin, end, delta) in [
            (ScrollPhase::MayBegin, ScrollPhase::None, true, false, 0.),
            (ScrollPhase::Began, ScrollPhase::None, true, false, -3.),
            (ScrollPhase::Changed, ScrollPhase::None, false, false, -4.),
            (ScrollPhase::Ended, ScrollPhase::None, false, true, 0.),
            (ScrollPhase::None, ScrollPhase::Began, false, false, -5.),
            (ScrollPhase::None, ScrollPhase::Changed, false, false, -6.),
            (ScrollPhase::None, ScrollPhase::Ended, false, true, 0.),
            (ScrollPhase::Cancelled, ScrollPhase::None, false, true, 0.),
        ] {
            send(&mut host, native(phase, momentum, delta));
            let seen = events.borrow();
            let ContentEvent::Scroll {
                position,
                delta: mapped,
                precise,
                begin: b,
                end: e,
                momentum: m,
                time,
            } = seen
                .last()
                .expect("body receives even zero-delta terminal packets")
            else {
                panic!("scroll")
            };
            assert_eq!(*position, Point::new(100., 100.));
            assert_eq!(*mapped, Point::new(0., -delta as f32));
            assert!(*precise);
            assert_eq!(
                (*b, *e, *m, *time),
                (begin, end, momentum != ScrollPhase::None, 123.5)
            );
        }
        assert_eq!(events.borrow().len(), 8);
    }
    #[test]
    fn body_scroll_original_owner_ends_gestures_and_applies_line_units_once() {
        for terminal in [ScrollPhase::Ended, ScrollPhase::Cancelled] {
            let (mut host, _) = host();
            send(
                &mut host,
                native(ScrollPhase::Began, ScrollPhase::None, -20.),
            );
            scroll(&mut host, |s| {
                assert!(
                    s.scroll().needs_animation(),
                    "Began establishes the gesture"
                )
            });
            send(&mut host, native(terminal, ScrollPhase::None, 0.));
            scroll(&mut host, |s| {
                assert!(
                    !s.scroll().needs_animation(),
                    "zero-delta terminal ends the gesture"
                )
            });
            let mut wheel = native(ScrollPhase::None, ScrollPhase::None, -2.);
            wheel.precise = false;
            send(&mut host, wheel);
            let line = tekes_ui_tokens::proposal::metric("size", "size-row-height").unwrap();
            scroll(&mut host, |s| assert_eq!(s.offset().y, 20. + 2. * line));
        }
    }
    #[test]
    fn body_scroll_reveal_and_reading_fences_reject_old_momentum_but_allow_new_fingers() {
        for reading in [false, true] {
            let (mut host, _) = host();
            if !reading {
                host.panel_mgr.panels[0]
                    .local
                    .as_mut()
                    .unwrap()
                    .with_host(&mut |h| {
                        let plan = h
                            .plan_host_reveal(
                                ComponentId(2),
                                tekes_ui_controls::ScrollRevealOptions {
                                    placement: tekes_ui_controls::ScrollRevealPlacement::Centered,
                                    animated: true,
                                    ..Default::default()
                                },
                                &[],
                                tekes_ui_controls::composition::HostRevealBudget::default(),
                            )
                            .unwrap();
                        assert!(h.commit_host_reveal(plan).unwrap().animated);
                    });
            }
            if reading {
                host.panel_mgr.panels[0]
                    .local
                    .as_mut()
                    .unwrap()
                    .with_host(&mut |h| {
                        h.set_scroll_reading_document(
                            ComponentId(1),
                            "reader",
                            1,
                            vec![ComponentId(2)],
                            |_| (),
                        )
                        .unwrap();
                    });
                scroll(&mut host, |s| {
                    assert!(s.follow_latest());
                });
            }
            let mut before = Point::default();
            scroll(&mut host, |s| before = s.offset());
            send(
                &mut host,
                native(ScrollPhase::None, ScrollPhase::Changed, 15.),
            );
            scroll(&mut host, |s| {
                assert_eq!(
                    s.offset(),
                    before,
                    "old OS momentum cannot take over navigation"
                );
                if reading {
                    assert!(s.is_following_latest());
                } else {
                    assert!(s.scroll().needs_animation());
                }
            });
            // Zero-delta Began itself releases the fence before a new delta.
            send(&mut host, native(ScrollPhase::Began, ScrollPhase::None, 0.));
            send(
                &mut host,
                native(
                    ScrollPhase::Changed,
                    ScrollPhase::None,
                    if reading { 10. } else { -10. },
                ),
            );
            scroll(&mut host, |s| {
                assert_eq!(s.offset().y, before.y + if reading { -10. } else { 10. });
                if reading {
                    assert!(!s.is_following_latest());
                }
            });
        }
    }
    #[test]
    fn body_scroll_boundary_rebound_respects_original_reduced_motion() {
        for reduced in [false, true] {
            let (mut host, _) = host();
            scroll(&mut host, |s| {
                s.set_reduced_motion(reduced);
            });
            send(
                &mut host,
                native(ScrollPhase::Began, ScrollPhase::None, 60.),
            );
            send(&mut host, native(ScrollPhase::Ended, ScrollPhase::None, 0.));
            scroll(&mut host, |s| {
                assert_eq!(s.offset().y, 0.);
                if reduced {
                    assert_eq!(s.scroll().overscroll.y, 0.);
                } else {
                    assert!(s.scroll().overscroll.y < 0.);
                    assert!(s.scroll().needs_animation());
                    for _ in 0..120 {
                        s.tick(1. / 60.);
                    }
                    assert!(s.scroll().overscroll.y.abs() < 0.01);
                }
            });
        }
    }
}

#[cfg(test)]
mod redraw_efficiency_tests {
    use crate::{AppWindow, LocalContainer};
    use tekes_ui_controls::composition::{
        Button, ComponentId, Content, ContentEvent, PointerEvent, PointerPhase, StackContent,
        TextFieldContent,
    };
    use tekes_ui_module_runtime::ContentClipboard;
    use tekes_ui_platform_macos::{
        Event, KeyEvent, KeyEventKind, Modifiers, TextKeyContext, TextKeyDisposition, TextKeyEvent,
    };
    use tekes_ui_scene::{Point, Rect, Scene};
    use tekes_ui_tokens::Theme;

    struct NoClipboard;
    impl ContentClipboard for NoClipboard {
        fn clipboard_text(&self) -> Option<String> {
            None
        }
        fn set_clipboard_text(&self, _: &str) {}
    }

    fn key_event(key_code: u16, chars: &str) -> KeyEvent {
        KeyEvent {
            key_code,
            kind: KeyEventKind::Down,
            modifiers: Modifiers::default(),
            characters: chars.to_owned(),
            characters_ignoring_modifiers: chars.to_owned(),
            is_repeat: false,
        }
    }

    #[test]
    fn caret_blink_produces_two_real_scene_phases_and_skips_readonly_unfocused_and_selected() {
        let editable = TextFieldContent::new("Edit", "abc");
        let content = Content::new(ComponentId(1), editable);
        let local = LocalContainer::new("edit", content).unwrap();
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let frame = Rect::new(0., 40., 320., 120.);
        {
            let panel = &mut host.panel_mgr.panels[0];
            panel.bounds = frame;
            panel.geometry.content = frame;
            panel.content_padding = 0.;
            panel.local.as_mut().unwrap().with_host(&mut |h| {
                h.focus(Some(ComponentId(1)), false);
            });
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        assert!(!host.handle_local_caret_blink(false));
        assert!(host.text.0.caret_visible);

        // Collapsed selection + editable + active -> alternates visible/hidden phases.
        assert!(host.handle_local_caret_blink(true));
        assert!(!host.text.0.caret_visible);
        let mut scene_hidden = Scene::new();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut scene_hidden,
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();

        assert!(host.handle_local_caret_blink(true));
        assert!(host.text.0.caret_visible);
        let mut scene_visible = Scene::new();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut scene_visible,
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();

        assert_ne!(
            scene_visible.primitives().len(),
            scene_hidden.primitives().len(),
            "two caret blink phases must emit distinct scene primitives"
        );

        // Non-empty selection must not emit periodic blink redraws.
        let selected = Content::new(ComponentId(1), TextFieldContent::new("Edit", "abc"));
        let sel_local = LocalContainer::new("sel", selected).unwrap();
        let mut sel_host = AppWindow::from_columns(vec![sel_local.into()])
            .unwrap()
            .into_host(false);
        assert!(sel_host.mount_columns());
        sel_host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                h.focus(Some(ComponentId(1)), false);
                h.get_mut(ComponentId(1))
                    .and_then(|n| n.component_mut::<TextFieldContent>())
                    .unwrap()
                    .set_selection(0, 2);
            });
        sel_host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut sel_host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(!sel_host.handle_local_caret_blink(true));

        // Read-only focused field must not blink/redraw.
        let ro_field = TextFieldContent::new("RO", "abc").with_read_only(true);
        let ro_local = LocalContainer::new("ro", Content::new(ComponentId(1), ro_field)).unwrap();
        let mut ro_host = AppWindow::from_columns(vec![ro_local.into()])
            .unwrap()
            .into_host(false);
        assert!(ro_host.mount_columns());
        ro_host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                h.focus(Some(ComponentId(1)), false);
            });
        ro_host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut ro_host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(!ro_host.handle_local_caret_blink(true));
    }

    #[test]
    fn blank_mouse_move_and_unhandled_key_skip_redraw_while_hover_and_typing_redraw() {
        let content = Content::new(
            ComponentId(10),
            StackContent::vertical(
                16.,
                vec![
                    Content::new(ComponentId(11), Button::new("HoverTarget")),
                    Content::new(ComponentId(12), TextFieldContent::new("Input", "abc")),
                ],
            ),
        );
        let local = LocalContainer::new("main", content).unwrap();
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let frame = Rect::new(0., 40., 360., 280.);
        let mut button_center = None;
        {
            let panel = &mut host.panel_mgr.panels[0];
            panel.bounds = frame;
            panel.geometry.content = frame;
            panel.content_padding = 0.;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            panel.local.as_mut().unwrap().with_host(&mut |h| {
                button_center = h
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == ComponentId(11))
                    .map(|n| {
                        let b = n.frame;
                        Point::new(
                            b.min_x() + b.size.width / 2.,
                            b.min_y() + b.size.height / 2.,
                        )
                    });
            });
        }
        let button_center = button_center.expect("button bounds");
        let blank_point = Point::new(frame.max_x() - 10., frame.max_y() - 10.);
        // Blank move causes no state/render change and preserves AX publication currency.
        let publication_before_blank = host.content_accessibility_publication(true).unwrap();
        assert!(publication_before_blank.is_current());
        let panel = host.panel_mgr.panels[0].local.as_mut().unwrap();
        let blank_move = ContentEvent::Pointer(PointerEvent {
            position: blank_point,
            phase: PointerPhase::Move,
            button: None,
            click_count: 0,
            modifiers: Default::default(),
        });
        assert!(
            !panel
                .hover_event_needs_redraw(&blank_move, &mut host.text)
                .unwrap()
        );
        assert!(publication_before_blank.is_current());

        // Real hover enter redraws; moving within blank after hover exit redraws once on exit.
        let enter_btn = ContentEvent::Pointer(PointerEvent {
            position: button_center,
            phase: PointerPhase::Move,
            button: None,
            click_count: 0,
            modifiers: Default::default(),
        });
        assert!(
            panel
                .hover_event_needs_redraw(&enter_btn, &mut host.text)
                .unwrap()
        );
        panel
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(
            panel
                .hover_event_needs_redraw(&blank_move, &mut host.text)
                .unwrap()
        );
        panel
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(
            !panel
                .hover_event_needs_redraw(&blank_move, &mut host.text)
                .unwrap()
        );

        // Unhandled key on button focus must not request redraw and preserves AX currency.
        panel.with_host(&mut |h| {
            h.focus(Some(ComponentId(11)), false);
        });
        panel
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let publication_before_key = host.content_accessibility_publication(true).unwrap();
        assert!(publication_before_key.is_current());
        assert!(
            !host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .window_event(
                    &Event::Key(key_event(105, "")),
                    &NoClipboard,
                    &mut host.text
                )
                .unwrap()
        );
        assert!(publication_before_key.is_current());
        let ctx = TextKeyContext {
            id: 1,
            key: key_event(105, ""),
            composing_at_start: false,
        };
        assert!(
            !host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .window_event(
                    &Event::TextKey(TextKeyEvent::Started(ctx.clone())),
                    &NoClipboard,
                    &mut host.text,
                )
                .unwrap()
        );
        assert!(
            !host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .window_event(
                    &Event::TextKey(TextKeyEvent::Finished {
                        context: ctx,
                        input_context_handled: Some(false),
                        disposition: TextKeyDisposition::Unhandled,
                        text_callbacks: 0,
                        command_callbacks: 0,
                    }),
                    &NoClipboard,
                    &mut host.text,
                )
                .unwrap()
        );
        assert!(publication_before_key.is_current());
        let panel = host.panel_mgr.panels[0].local.as_mut().unwrap();

        // Valid text commit on focused TextField requests redraw.
        panel.with_host(&mut |h| {
            h.focus(Some(ComponentId(12)), false);
        });
        assert!(
            panel
                .window_event(
                    &Event::TextCommit {
                        text: "d".into(),
                        replacement: None,
                    },
                    &NoClipboard,
                    &mut host.text,
                )
                .unwrap()
        );
    }

    #[test]
    fn on_action_callback_and_rich_link_drain_drive_redraw_and_ax_currency() {
        use std::cell::Cell;
        use std::rc::Rc;
        use tekes_ui_controls::composition::{
            AccessibilityAction, RichContent, RichDocument, TextContent,
        };
        let handled_links = Rc::new(Cell::new(0usize));
        let handled_links_cb = handled_links.clone();
        let reader_id = ComponentId(701);
        let notice_id = ComponentId(702);
        let btn_id = ComponentId(703);
        let body_id = ComponentId(704);
        let content = Content::new(
            ComponentId(700),
            StackContent::vertical(
                8.,
                vec![
                    Content::new(notice_id, TextContent::new("initial")),
                    Content::new(
                        reader_id,
                        RichContent::new(
                            "manual",
                            1,
                            RichDocument::from_markdown("[Number](number-field.md)"),
                        )
                        .unwrap(),
                    ),
                    Content::new(btn_id, Button::new("Trigger")),
                    Content::new(body_id, TextContent::new("body-initial")),
                ],
            ),
        );
        let local = LocalContainer::new("doc-panel", content)
            .unwrap()
            .on_action(move |host, records| {
                // Case (a)/(b): read-only check before get_mut for RichContent links.
                if host
                    .get(reader_id)
                    .and_then(|c| c.component::<RichContent>())
                    .is_some_and(RichContent::has_link_actions)
                {
                    let links = host
                        .get_mut(reader_id)
                        .unwrap()
                        .component_mut::<RichContent>()
                        .unwrap()
                        .drain_link_actions();
                    handled_links_cb.set(handled_links_cb.get() + links.len());
                    host.get_mut(notice_id)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .set_text("link-handled");
                }
                // Case (c): ordinary action callback mutates body.
                if records.iter().any(|r| r.action.target == btn_id) {
                    host.get_mut(body_id)
                        .unwrap()
                        .component_mut::<TextContent>()
                        .unwrap()
                        .set_text("body-mutated-by-callback");
                }
            });
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let frame = Rect::new(0., 40., 360., 320.);
        {
            let panel = &mut host.panel_mgr.panels[0];
            panel.bounds = frame;
            panel.geometry.content = frame;
            panel.content_padding = 0.;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }

        // Case (a): both action and rich link queues empty -> no redraw and AX stays current.
        let pub_a = host.content_accessibility_publication(true).unwrap();
        assert!(pub_a.is_current());
        let blank_move = ContentEvent::Pointer(PointerEvent {
            position: Point::new(frame.max_x() - 4., frame.max_y() - 4.),
            phase: PointerPhase::Move,
            button: None,
            click_count: 0,
            modifiers: Default::default(),
        });
        assert!(
            !host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .hover_event_needs_redraw(&blank_move, &mut host.text)
                .unwrap()
        );
        assert!(pub_a.is_current());
        assert!(
            !host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .window_event(
                    &Event::Key(key_event(105, "")),
                    &NoClipboard,
                    &mut host.text,
                )
                .unwrap()
        );
        assert!(pub_a.is_current());
        assert_eq!(handled_links.get(), 0);

        // Case (b): ordinary action queue empty, RichContent link queued -> handled once & redraws.
        {
            let panel = host.panel_mgr.panels[0].local.as_mut().unwrap();
            panel.with_host(&mut |h| {
                assert!(h.focus(Some(reader_id), false));
                h.accessibility_action(reader_id, None, AccessibilityAction::SetSelection(2..2));
            });
            panel
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            let mut enter = key_event(36, "\r");
            enter.modifiers.command = true;
            assert!(
                panel
                    .window_event(&Event::Key(enter), &NoClipboard, &mut host.text)
                    .unwrap()
            );
            assert_eq!(handled_links.get(), 1);
            panel.with_host(&mut |h| {
                assert_eq!(
                    h.get(notice_id)
                        .unwrap()
                        .component::<TextContent>()
                        .unwrap()
                        .text(),
                    "link-handled"
                );
            });
            panel
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            assert!(
                !panel
                    .hover_event_needs_redraw(&blank_move, &mut host.text)
                    .unwrap()
            );
            assert_eq!(handled_links.get(), 1);
        }

        // Case (c): on_action callback mutates body on keyboard Enter press AND on hover callback path -> redraws & publishes new AX.
        let mut btn_center = None;
        {
            let panel = host.panel_mgr.panels[0].local.as_mut().unwrap();
            panel.with_host(&mut |h| {
                btn_center = h
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == btn_id)
                    .map(|n| Point::new(n.frame.min_x() + 4., n.frame.min_y() + 4.));
                assert!(h.focus(Some(btn_id), false));
            });
            panel
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        let pub_before_cb = host.content_accessibility_publication(true).unwrap();
        assert!(pub_before_cb.is_current());
        assert!(
            host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .window_event(
                    &Event::Key(key_event(36, "\r")),
                    &NoClipboard,
                    &mut host.text,
                )
                .unwrap()
        );
        assert!(!pub_before_cb.is_current());
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let pub_after_cb = host.content_accessibility_publication(true).unwrap();
        assert!(pub_after_cb.is_current());
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| {
                assert_eq!(
                    h.get(body_id)
                        .unwrap()
                        .component::<TextContent>()
                        .unwrap()
                        .text(),
                    "body-mutated-by-callback"
                );
            });

        // Verify hover_event_needs_redraw also accounts for mutations made inside on_action.
        let hover_cb_local = LocalContainer::new(
            "hover-cb",
            Content::new(ComponentId(710), TextContent::new("hover-before")),
        )
        .unwrap()
        .on_action(|h, _| {
            if h.get(ComponentId(710))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text()
                == "hover-before"
            {
                h.get_mut(ComponentId(710))
                    .unwrap()
                    .component_mut::<TextContent>()
                    .unwrap()
                    .set_text("hover-after");
            }
        });
        let mut hover_host = AppWindow::from_columns(vec![hover_cb_local.into()])
            .unwrap()
            .into_host(false);
        assert!(hover_host.mount_columns());
        {
            let panel = &mut hover_host.panel_mgr.panels[0];
            panel.bounds = frame;
            panel.geometry.content = frame;
            panel.content_padding = 0.;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut hover_host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        let hover_pub_before = hover_host.content_accessibility_publication(true).unwrap();
        assert!(hover_pub_before.is_current());
        assert!(
            hover_host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .hover_event_needs_redraw(&blank_move, &mut hover_host.text)
                .unwrap()
        );
        assert!(!hover_pub_before.is_current());
        hover_host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut hover_host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        let hover_pub_after = hover_host.content_accessibility_publication(true).unwrap();
        assert!(hover_pub_after.is_current());
        assert!(
            !hover_host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .hover_event_needs_redraw(&blank_move, &mut hover_host.text)
                .unwrap()
        );
        assert!(hover_pub_after.is_current());
        let _ = btn_center;
    }
}
