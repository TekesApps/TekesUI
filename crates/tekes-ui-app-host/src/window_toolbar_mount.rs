//! Native-window adapter: declaration data in, original invocations out.
use super::*;
use crate::{
    toolbar_source::ComponentSource,
    window_toolbar::{Contents, Declaration, Source},
};
use tekes_ui_controls::composition::{
    ToolbarButtonDeclaration as ButtonDecl, ToolbarGroupOverflow as Overflow,
    ToolbarPlacement as Placement, ToolbarRegion,
};
pub(super) struct ToolbarBand {
    pub y: f32,
    pub height: f32,
    pub native_leading: f32,
}
impl Host {
    pub(super) fn sync_toolbar(
        &mut self,
        w: &Window,
        whole: Rect,
        frames: &[Rect],
        band: ToolbarBand,
        title: String,
    ) -> Result<()> {
        if self.presentation.is_some() {
            return self.sync_container_toolbars(
                w.is_active(),
                whole,
                band,
                w.titlebar_safe_area().0 as f32,
            );
        }
        let ToolbarBand {
            y,
            height: row,
            native_leading: safe,
        } = band;
        let center_slot = usize::from(frames.len() >= 2);
        let center = self.order[center_slot];
        let mut declarations = vec![];
        let mut add = |id, region: ComponentId, placement, order, contents, source| {
            declarations.push(Declaration {
                id: ComponentId(id),
                region,
                placement,
                order,
                contents,
                source,
            })
        };
        for (side, on, slot, icon, collapsed, label) in [
            (
                0,
                self.leading_visible,
                ComponentId(91020),
                "tekes-side-bar",
                "tekes-side-bar-collapsed",
                "leading",
            ),
            (
                1,
                self.trailing_visible,
                ComponentId(91021),
                "tekes-side-panel",
                "tekes-side-panel-collapsed",
                "trailing",
            ),
        ]
        .into_iter()
        .take(if frames.len() == 3 {
            2
        } else if frames.len() == 2 {
            1
        } else {
            0
        }) {
            let icon = if on { icon } else { collapsed };
            let hint = format!("{} {label} panel", if on { "Hide" } else { "Show" });
            add(
                92000 + side as u64 * 10,
                slot,
                if side == 0 {
                    Placement::Navigation
                } else {
                    Placement::PrimaryAction
                },
                0,
                Contents::TemplateCommands(
                    vec![ButtonDecl {
                        id: 0x5601 + side as u64,
                        icon: None,
                        text: hint.clone(),
                        hint: Some(hint),
                        enabled: true,
                    }],
                    Overflow::Together,
                    vec![(0x5601 + side as u64, icon.to_owned())],
                ),
                Source::Visibility(side),
            );
        }
        let open = self
            .navigation
            .as_ref()
            .is_none_or(|n| n.host.input_open && !n.busy && n.error.is_none());
        if let Some(nav) = &self.navigation
            && let Some(source) = ComponentSource::current(&nav.process)
        {
            let mut buttons = vec![];
            let mut templates = vec![];
            for action in &nav.actions {
                if let Some(name) = &action.icon {
                    templates.push((action.command.id, name.clone()));
                }
                buttons.push(ButtonDecl {
                    id: action.command.id,
                    icon: None,
                    text: action
                        .accessible_name
                        .clone()
                        .unwrap_or_else(|| action.command.title.clone()),
                    hint: action.tooltip.clone(),
                    enabled: open && action.command.enabled,
                });
            }
            add(
                92100,
                ComponentId(91020),
                Placement::Navigation,
                10,
                Contents::TemplateCommands(buttons, Overflow::Together, templates),
                Source::Navigation(source),
            );
        }
        // TextContent remains an original single-line toolbar descendant.
        add(
            92200,
            ComponentId(91000 + center as u64),
            Placement::Navigation,
            20,
            Contents::Title(title),
            Source::Title,
        );
        if let Some(state) = &self.settings {
            let state = state.borrow();
            if let Some(source) = &state.toolbar_source {
                let mut buttons = vec![];
                let mut templates = vec![];
                for action in state.toolbar_actions() {
                    if let Some(name) = &action.icon {
                        templates.push((action.command.id, name.clone()));
                    }
                    buttons.push(ButtonDecl {
                        id: action.command.id,
                        icon: None,
                        text: action.accessible_name.unwrap_or(action.command.title),
                        hint: action.tooltip,
                        enabled: action.command.enabled,
                    });
                }
                add(
                    92300,
                    ComponentId(91000 + center as u64),
                    Placement::SecondaryAction,
                    0,
                    Contents::TemplateCommands(buttons, Overflow::Together, templates),
                    Source::Undo(source.clone()),
                );
            }
        }
        for (slot, &index) in self.order.iter().enumerate() {
            let p = &self.panels[index];
            if frames[slot].size.width <= 0. || p.toolbar.is_empty() {
                continue;
            }
            for secondary in [false, true] {
                let buttons: Vec<_> = p
                    .toolbar
                    .iter()
                    .filter(|a| p.options.secondary_commands.contains(&a.id) == secondary)
                    .map(|a| ButtonDecl {
                        id: a.id,
                        icon: None,
                        text: a.title.clone(),
                        hint: Some(a.title.clone()),
                        enabled: open
                            && !self.sidebar_motion.needs_frame()
                            && p.failed.is_none()
                            && a.enabled
                            && (!a.geometry_dependent || p.frame_viewport == p.viewport),
                    })
                    .collect();
                if buttons.is_empty() {
                    continue;
                }
                add(
                    92400 + index as u64 * 10 + if secondary { 8_000_000 } else { 0 },
                    ComponentId(91000 + slot as u64),
                    if secondary {
                        Placement::SecondaryAction
                    } else {
                        Placement::PrimaryAction
                    },
                    10,
                    Contents::Commands(
                        buttons,
                        if secondary {
                            Overflow::Menu
                        } else {
                            Overflow::Split
                        },
                    ),
                    Source::Panel {
                        instance: p.instance,
                        index,
                        module: p.module_id.clone(),
                        pid: p.pid,
                        generation: p.toolbar_generation,
                    },
                );
            }
        }
        self.remember_toolbar_focus(None);
        // Containers own the shared Toolbar instances. Window contributes only
        // placement and authenticated command sources to the selected container.
        let visible = |index: usize| {
            self.order
                .iter()
                .position(|&i| i == index)
                .is_some_and(|slot| frames[slot].size.width > 0.)
        };
        let mut owned = vec![Vec::new(); self.panels.len()];
        for mut declaration in declarations {
            let (owner, region) = match &declaration.source {
                Source::Panel { index, .. } if visible(*index) => {
                    match self.panels[*index].options.toolbar_host {
                        crate::ContainerToolbarHost::Window => (center, ComponentId(u64::MAX - 18)),
                        crate::ContainerToolbarHost::Container { .. } => {
                            (*index, ComponentId(91000 + *index as u64))
                        }
                    }
                }
                Source::Panel { .. } => continue,
                _ => (center, ComponentId(u64::MAX - 18)),
            };
            declaration.region = region;
            owned[owner].push(declaration);
        }
        for (index, panel) in self.panels.iter_mut().enumerate() {
            let mut regions = Vec::new();
            // Settings syncs its native-top toolbar before publishing panel
            // bounds. The current allocation, not last draw's bounds, owns it.
            let frame = self
                .order
                .iter()
                .position(|&owner| owner == index)
                .map(|slot| frames[slot])
                .unwrap_or_default();
            if frame.size.width > 0. {
                if let Some(frame) = panel.geometry.toolbar {
                    let mut region = ToolbarRegion::new(ComponentId(91000 + index as u64), frame);
                    if matches!(
                        panel.options.toolbar_host,
                        crate::ContainerToolbarHost::Container {
                            position: crate::ContainerToolbarPosition::Leading
                                | crate::ContainerToolbarPosition::Trailing
                        }
                    ) {
                        region.axis = Axis::Vertical;
                    }
                    regions.push(region);
                }
                if index == center {
                    let mut region = ToolbarRegion::new(
                        ComponentId(u64::MAX - 18),
                        Rect::new(whole.min_x(), y, whole.size.width, row),
                    );
                    region.native_safe_area.left = safe;
                    regions.push(region);
                }
            }
            panel
                .mounted_toolbar
                .sync(whole, regions, std::mem::take(&mut owned[index]))?;
            panel.mounted_toolbar.host.set_window_active(w.is_active());
        }
        Ok(())
    }
    pub(super) fn drain_toolbar_commands(&mut self) -> Result<()> {
        for invocation in self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .drain_commands()
        {
            if invocation.window != self.panels[self.toolbar_index].mounted_toolbar.window {
                continue;
            }
            eprintln!(
                "TOOLBAR_INVOCATION window={} command={} input={:?} source={:?}",
                invocation.window, invocation.command, invocation.input, invocation.source
            );
            let mut exchanged = false;
            let result =
                match invocation.source {
                    Source::Visibility(side) => self.toggle_visibility(side),
                    Source::Navigation(source) => {
                        if self.prepare_settings_item_navigation() {
                            self.navigation
                                .as_mut()
                                .map_or(Ok(()), |nav| nav.invoke(&source, invocation.command))
                        } else {
                            Ok(())
                        }
                    }
                    Source::WorkspaceSwitch { focus, other } => self
                        .switch_workspace_focus(focus, other)
                        .map(|changed| exchanged = changed),
                    Source::Title => Err(anyhow::anyhow!("title has no command binding")),
                    Source::Expand {
                        container,
                        expanded,
                    } => self
                        .toggle_others_expansion(&container, expanded)
                        .map(|changed| exchanged = changed),
                    Source::Undo(source) => self.settings.as_ref().map_or(Ok(()), |s| {
                        s.borrow_mut().undo_action(invocation.command, &source)
                    }),
                    Source::Panel {
                        instance,
                        index,
                        module,
                        pid,
                        generation,
                    } => {
                        if self.navigation.as_ref().is_some_and(|nav| {
                            !nav.host.input_open || nav.busy || nav.error.is_some()
                        }) {
                            Ok(())
                        } else if let Some(p) = self.panels.get(index)
                            && p.instance == instance
                            && p.module_id == module
                            && p.pid == pid
                            && p.toolbar_generation == generation
                        {
                            if self.presentation.is_some()
                                && p.options.focus_switch_command == Some(invocation.command)
                            {
                                self.switch_container_focus(index)
                                    .map(|changed| exchanged = changed)
                            } else {
                                self.panels[index].command(invocation.command)
                            }
                        } else {
                            Ok(())
                        }
                    }
                };
            if let Err(error) = result {
                eprintln!("TOOLBAR_COMMAND_REJECTED {error:#}");
            }
            // A completed presentation change revokes remaining old-position
            // actions already drained in this input batch, as well as capture.
            if exchanged {
                break;
            }
        }
        Ok(())
    }
    /// Single shared event entry before any body fallback. Existing body/splitter
    /// captures keep their original owner, including when crossing toolbar chrome.
    pub(super) fn route_toolbar_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        if matches!(event, Event::Focus(false) | Event::CloseRequested) {
            self.clear_toolbar_hover();
        }
        if self.panels.is_empty() {
            return Ok(false);
        }
        if matches!(
            event,
            Event::Key(_)
                | Event::TextKey(_)
                | Event::TextCommit { .. }
                | Event::TextPreedit { .. }
                | Event::TextUnmark
                | Event::TextCommand(_)
        ) && let Some(index) = self
            .panels
            .iter()
            .position(|p| p.mounted_toolbar.host.content_host().focused().is_some())
        {
            self.toolbar_index = index;
        }
        if let Event::Key(key) = event {
            let input = toolbar_key(key.clone());
            if matches!(input.key, Some(ControlKey::Tab { .. }))
                && (input.modifiers.command || input.modifiers.control || input.modifiers.alt)
            {
                return Ok(false);
            }
        }
        if let Event::Accessibility { target, action } = event {
            if w.content_accessibility_publication()
                .and_then(|p| p.owner(*target))
                .is_none_or(|owner| {
                    owner.0 < Self::toolbar_accessibility_owner(0)
                        || owner.0 >= Self::toolbar_accessibility_owner(self.panels.len())
                })
            {
                return Ok(false);
            }
            let before = self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused();
            let handled =
                self.toolbar_accessibility_action(w.is_active(), *target, action.clone())?;
            if handled
                && self.panels[self.toolbar_index]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused()
                    != before
                && self.panels[self.toolbar_index]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused()
                    .is_some()
            {
                for p in &mut self.panels {
                    if let Some(local) = &mut p.local {
                        local.with_host(&mut |h| h.cancel());
                    }
                }
            }
            return Ok(handled
                || self.panels[self.toolbar_index].mounted_toolbar.modal()
                || self.panels[self.toolbar_index]
                    .mounted_toolbar
                    .host
                    .semantics()
                    .is_empty());
        }
        let body_capture =
            self.drag_split.is_some() || self.pressed.is_some() || self.settings_pointer.is_some();
        if body_capture
            && matches!(event,Event::Mouse(m) if matches!(m.kind,MouseEventKind::Dragged{..}|MouseEventKind::Up{..}))
        {
            return Ok(false);
        }
        // An original body control owns its complete key/IME sequence, even
        // when it is not a text editor (for example a Settings MenuButton).
        // Settings plain Tab first visits its existing cross-scope coordinator.
        let settings_tab = self.is_settings_window
            && matches!(event, Event::Key(key)
                if key.kind == KeyEventKind::Down && key.key_code == 48
                    && !key.modifiers.command && !key.modifiers.control
                    && !key.modifiers.alt);
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_none()
            && !self.panels[self.toolbar_index].mounted_toolbar.modal()
            && !settings_tab
            && matches!(
                event,
                Event::Key(_)
                    | Event::TextKey(_)
                    | Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextUnmark
                    | Event::TextCommand(_)
            )
        {
            let mut body_focus = false;
            for panel in &mut self.panels {
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |h| body_focus |= h.focused().is_some());
                }
            }
            if body_focus {
                return Ok(false);
            }
        }
        let Some(input) = native_toolbar_event(event) else {
            return Ok(false);
        };
        let previous_focus = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused();
        let blocked = if let Event::Key(key) = event {
            self.toolbar_key_input(toolbar_key(key.clone()))?
        } else if let Some(owner) = self.open_toolbar().or_else(|| {
            if let Event::Mouse(mouse) = event {
                self.toolbar_target_at(Point::new(mouse.position.x as f32, mouse.position.y as f32))
            } else {
                self.actual_toolbar_owner()
            }
        }) {
            !self.dispatch_toolbar(owner, &input)?
        } else {
            self.update_toolbar_hover(None, &input)?;
            self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .event_with_text(&input, &mut self.text)
                .map_err(|e| anyhow::anyhow!("toolbar input: {e:?}"))?;
            self.drain_toolbar_commands()?;
            !self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .event_bubbled()
        };
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_some()
        {
            self.blur_other_toolbars(self.toolbar_index);
        }
        if blocked
            && self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused()
                .is_some()
            && self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused()
                != previous_focus
        {
            for p in &mut self.panels {
                if let Some(local) = &mut p.local {
                    local.with_host(&mut |host| {
                        host.cancel();
                    });
                }
            }
        }
        if !blocked && matches!(event,Event::Mouse(m) if matches!(m.kind,MouseEventKind::Down{..}))
        {
            self.set_toolbar_focus(None, false);
        }
        let focus_changed = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            != previous_focus;
        let pointer_down = matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down { .. }));
        self.remember_toolbar_focus(if blocked && pointer_down {
            Some(false)
        } else if focus_changed {
            Some(true)
        } else {
            None
        });
        Ok(blocked)
    }
}
pub(super) fn native_toolbar_event(event: &Event) -> Option<ContentEvent> {
    Some(match event {
        Event::Mouse(m) => toolbar_pointer(*m),
        Event::Scroll(s) => ContentEvent::Scroll {
            position: Point::new(s.position.x as f32, s.position.y as f32),
            delta: Point::new(-s.delta.x as f32, -s.delta.y as f32),
            precise: s.precise,
            begin: matches!(s.phase, tekes_ui_platform_macos::ScrollPhase::Began),
            end: matches!(
                s.phase,
                tekes_ui_platform_macos::ScrollPhase::Ended
                    | tekes_ui_platform_macos::ScrollPhase::Cancelled
            ),
            momentum: s.momentum_phase != tekes_ui_platform_macos::ScrollPhase::None,
            time: s.timestamp,
        },
        Event::Key(k) => ContentEvent::KeyInput(toolbar_key(k.clone())),
        Event::TextCommit { text, replacement } => ContentEvent::Commit {
            text: text.clone(),
            replacement: replacement.clone(),
        },
        Event::TextPreedit {
            text,
            selection,
            clauses,
            replacement,
        } => ContentEvent::Preedit {
            text: text.clone(),
            selection: selection.clone(),
            clauses: clauses.clone(),
            replacement: replacement.clone(),
        },
        Event::TextKey(marker) => match marker.clone() {
            tekes_ui_platform_macos::TextKeyEvent::Started(context) => ContentEvent::TextKeyStart {
                id: context.id,
                key: toolbar_key(context.key),
                composing: context.composing_at_start,
            },
            tekes_ui_platform_macos::TextKeyEvent::Finished {
                context,
                disposition,
                ..
            } => ContentEvent::TextKeyEnd {
                id: context.id,
                input_context_consumed: disposition
                    == tekes_ui_platform_macos::TextKeyDisposition::InputContextConsumed,
            },
        },
        Event::TextUnmark => ContentEvent::Unmark,
        Event::TextCommand(command) => ContentEvent::TextCommand(command.clone()),
        _ => return None,
    })
}

impl Host {
    pub(super) fn toolbar_group(target: ToolbarTarget) -> ComponentId {
        ComponentId(match target {
            ToolbarTarget::Visibility(side) => 92000 + side as u64 * 10,
            ToolbarTarget::Navigation => 92100,
            ToolbarTarget::Contribution { group, .. } => return group,
            ToolbarTarget::Panel(index) | ToolbarTarget::Retained { owner: index, .. } => {
                92400 + index as u64 * 10
            }
        })
    }
    pub(super) fn toolbar_control(target: ToolbarTarget) -> ComponentId {
        ComponentId(Self::toolbar_group(target).0 + 1)
    }
    pub(super) fn toolbar_accessibility_owner(index: usize) -> u64 {
        (1 << 32) + index as u64
    }
    pub(super) fn toolbar_container(&self, target: ToolbarTarget) -> Option<usize> {
        let group = Self::toolbar_group(target);
        if let ToolbarTarget::Retained { presenter, .. }
        | ToolbarTarget::Contribution { presenter, .. } = target
        {
            return self
                .panels
                .get(presenter)
                .filter(|panel| panel.mounted_toolbar.source(group).is_some())
                .map(|_| presenter);
        }
        self.panels
            .iter()
            .position(|p| p.mounted_toolbar.source(group).is_some())
    }
    pub(super) fn select_toolbar_container(&mut self, index: usize) {
        self.toolbar_index = index;
    }
    pub(super) fn blur_other_toolbars(&mut self, index: usize) {
        for (other, panel) in self.panels.iter_mut().enumerate() {
            if other != index
                && panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused()
                    .is_some()
            {
                panel.mounted_toolbar.host.blur();
            }
        }
    }
    pub(super) fn toolbar_groups(&self) -> Vec<ToolbarTarget> {
        if self.presentation.is_some() {
            let mut groups = Vec::new();
            for (presenter, panel) in self.panels.iter().enumerate() {
                for (group, source) in panel.mounted_toolbar.contributions() {
                    let target = match source {
                        Source::Panel { index, .. }
                            if group != Self::toolbar_group(ToolbarTarget::Panel(*index)) =>
                        {
                            Some(ToolbarTarget::Contribution { presenter, group })
                        }
                        Source::Panel { index, .. } if *index == presenter => {
                            Some(ToolbarTarget::Panel(*index))
                        }
                        Source::Panel { index, .. } => Some(ToolbarTarget::Retained {
                            owner: *index,
                            presenter,
                        }),
                        Source::Expand { .. } | Source::WorkspaceSwitch { .. } => {
                            Some(ToolbarTarget::Contribution { presenter, group })
                        }
                        Source::Navigation(_) => Some(ToolbarTarget::Navigation),
                        Source::Visibility(side) => Some(ToolbarTarget::Visibility(*side)),
                        _ => None,
                    };
                    if let Some(target) = target
                        && !groups.contains(&target)
                    {
                        groups.push(target);
                    }
                }
            }
            groups.sort_by(|a, b| {
                let position = |target: ToolbarTarget| {
                    self.toolbar_container(target)
                        .and_then(|index| {
                            self.panels[index]
                                .mounted_toolbar
                                .host
                                .content_host()
                                .get(Self::toolbar_group(target))
                        })
                        .map_or(Point::default(), |content| content.frame().origin)
                };
                let a = position(*a);
                let b = position(*b);
                a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
            });
            return groups;
        }
        let mut groups = Vec::new();
        if self.presentation.is_none() && self.panels.len() >= 2 {
            groups.push(ToolbarTarget::Visibility(0));
        }
        if self.navigation.is_some() {
            groups.push(ToolbarTarget::Navigation);
        }
        groups.extend(
            self.order
                .iter()
                .filter(|&&i| i < self.panels.len())
                .map(|&i| ToolbarTarget::Panel(i)),
        );
        for (presenter, panel) in self.panels.iter().enumerate() {
            for (group, source) in panel.mounted_toolbar.contributions() {
                if let Source::Panel { index, .. } = source
                    && group != Self::toolbar_group(ToolbarTarget::Panel(*index))
                {
                    groups.push(ToolbarTarget::Contribution { presenter, group });
                }
            }
        }
        if self.presentation.is_none() && self.panels.len() == 3 {
            groups.push(ToolbarTarget::Visibility(1));
        }
        groups
    }
    pub(super) fn actual_toolbar_owner(&self) -> Option<ToolbarTarget> {
        let focused = self
            .panels
            .get(self.toolbar_index)?
            .mounted_toolbar
            .host
            .content_host()
            .focused()?;
        self.toolbar_groups().into_iter().find(|&owner| {
            self.toolbar_container(owner) == Some(self.toolbar_index)
                && Self::toolbar_control(owner) == focused
        })
    }
    pub(super) fn remember_toolbar_focus(&mut self, visible: Option<bool>) {
        let Some(owner) = self.actual_toolbar_owner() else {
            if self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused()
                .is_some()
            {
                // Another actual shared group (e.g. config Undo) now owns focus.
                // Old Panel return intent must not steal it on the next draw.
                self.toolbar_focus = None;
                self.toolbar_focus_lease = None;
            }
            return;
        };
        let Some(mount) = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .semantic_mount(Self::toolbar_control(owner))
        else {
            return;
        };
        let Some(source) = self.panels[self.toolbar_index]
            .mounted_toolbar
            .source(Self::toolbar_group(owner))
            .cloned()
        else {
            return;
        };
        self.toolbar_focus = Some(owner);
        self.toolbar_focus_lease = Some((mount, source));
        if let Some(visible) = visible {
            self.toolbar_focus_visible = visible;
        }
    }
    fn original_panel_binding(&self, target: ToolbarTarget) -> Option<(usize, usize)> {
        let presenter = self.toolbar_container(target)?;
        let Source::Panel { index, .. } = self.panels[presenter]
            .mounted_toolbar
            .source(Self::toolbar_group(target))?
        else {
            return None;
        };
        Some((*index, presenter))
    }
    fn toolbar_control_visible(&self, target: ToolbarTarget) -> bool {
        self.toolbar_container(target)
            .and_then(|index| {
                self.panels[index]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(Self::toolbar_control(target))
            })
            .is_some_and(|node| {
                node.frame().size.width > 0.
                    && node.frame().size.height > 0.
                    && node
                        .visible()
                        .is_some_and(|clip| clip.size.width > 0. && clip.size.height > 0.)
            })
    }
    pub(super) fn toolbar_available(&self, target: ToolbarTarget) -> bool {
        if self.presentation.is_some() || matches!(target, ToolbarTarget::Contribution { .. }) {
            if let Some((owner, _)) = self.original_panel_binding(target) {
                return self.panels[owner].failed.is_none()
                    && self.toolbar_control_visible(target)
                    && self
                        .navigation
                        .as_ref()
                        .is_none_or(|nav| nav.host.input_open && !nav.busy && nav.error.is_none());
            }
            return match target {
                ToolbarTarget::Contribution { presenter, group } => {
                    self.panels.get(presenter).is_some_and(|panel| {
                        panel.mounted_toolbar.source(group).is_some()
                            && self.toolbar_control_visible(target)
                    })
                }
                ToolbarTarget::Navigation => self
                    .navigation
                    .as_ref()
                    .is_some_and(|nav| nav.host.input_open && !nav.busy && nav.error.is_none()),
                ToolbarTarget::Visibility(side) => {
                    self.presentation.as_ref().is_some_and(|state| {
                        if side == 0 {
                            state.nav_bar_id().is_some()
                        } else {
                            state.others().is_some()
                        }
                    }) && self
                        .navigation
                        .as_ref()
                        .is_none_or(|nav| nav.host.input_open && !nav.busy && nav.error.is_none())
                }
                _ => false,
            };
        }
        match target {
            ToolbarTarget::Contribution { .. } => false,
            ToolbarTarget::Visibility(side) => {
                self.panels.len() == 3 || self.panels.len() == 2 && side == 0
            }
            ToolbarTarget::Navigation => self
                .navigation
                .as_ref()
                .is_some_and(|n| n.host.input_open && !n.busy && n.error.is_none()),
            ToolbarTarget::Panel(i) | ToolbarTarget::Retained { owner: i, .. } => {
                self.panels.get(i).is_some_and(|p| {
                    p.bounds.size.width > 0.
                        && !self.sidebar_motion.needs_frame()
                        && (self.panels.len() < 2
                            || (i != self.order[0] || self.leading_visible)
                                && (self.panels.len() != 3
                                    || i != self.order[2]
                                    || self.trailing_visible))
                        && self
                            .navigation
                            .as_ref()
                            .is_none_or(|n| n.host.input_open && !n.busy)
                })
            }
        }
    }
    pub(super) fn toolbar_temporarily_blocked(&self, target: ToolbarTarget) -> bool {
        if self.presentation.is_some() || matches!(target, ToolbarTarget::Contribution { .. }) {
            return self
                .original_panel_binding(target)
                .is_some_and(|(owner, _)| {
                    let panel = &self.panels[owner];
                    panel.failed.is_none()
                        && self.toolbar_control_visible(target)
                        && (self
                            .navigation
                            .as_ref()
                            .is_some_and(|nav| nav.busy && nav.error.is_none())
                            || panel.frame_viewport != panel.viewport)
                });
        }
        let navigating = self
            .navigation
            .as_ref()
            .is_some_and(|n| n.busy && n.error.is_none());
        match target {
            ToolbarTarget::Navigation => navigating,
            ToolbarTarget::Panel(i) | ToolbarTarget::Retained { owner: i, .. } => {
                self.panels.get(i).is_some_and(|p| {
                    (navigating
                        || self.sidebar_motion.needs_frame()
                        || p.frame_viewport != p.viewport)
                        && p.failed.is_none()
                        && p.bounds.size.width > 0.
                        && (self.panels.len() < 2
                            || (i != self.order[0] || self.leading_visible)
                                && (self.panels.len() != 3
                                    || i != self.order[2]
                                    || self.trailing_visible))
                })
            }
            ToolbarTarget::Visibility(_) | ToolbarTarget::Contribution { .. } => false,
        }
    }
    pub(super) fn toolbar_focus_targets(&self) -> Vec<ToolbarTarget> {
        self.toolbar_groups()
            .into_iter()
            .filter(|&owner| {
                self.toolbar_available(owner)
                    && self.toolbar_container(owner).is_some_and(|index| {
                        self.panels[index]
                            .mounted_toolbar
                            .host
                            .content_host()
                            .get(Self::toolbar_control(owner))
                            .is_some_and(|n| {
                                n.component::<ToolbarControl>().is_some_and(
                                    tekes_ui_controls::composition::Component::focusable,
                                )
                            })
                    })
            })
            .collect()
    }
    pub(super) fn set_toolbar_focus(&mut self, target: Option<ToolbarTarget>, visible: bool) {
        let target = target.filter(|t| self.toolbar_focus_targets().contains(t));
        let Some(owner) = target else {
            for panel in &mut self.panels {
                panel.mounted_toolbar.host.blur();
            }
            self.toolbar_focus = None;
            self.toolbar_focus_lease = None;
            self.toolbar_focus_visible = false;
            return;
        };
        let Some(index) = self.toolbar_container(owner) else {
            return;
        };
        self.blur_other_toolbars(index);
        if let Some(tabs) = &mut self.presentation_tabs {
            tabs.blur();
        }
        self.select_toolbar_container(index);
        let Some(target) = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .find(|n| n.node.id == Self::toolbar_control(owner))
            .map(|n| n.target)
        else {
            return;
        };
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .restore_focus(target, visible)
        {
            self.remember_toolbar_focus(Some(visible));
            if let Some((original, _)) = self.original_panel_binding(owner) {
                self.focused = original;
            }
        }
    }
    pub(super) fn reconcile_toolbar_focus(&mut self, active: bool) {
        if !active {
            self.cancel_toolbar_input();
            return;
        }
        let Some(owner) = self.toolbar_focus else {
            return;
        };
        let Some((mount, source)) = self.toolbar_focus_lease.as_ref() else {
            return;
        };
        // Never renew a lease from a reused numeric ID or a replaced source.
        let same = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .semantic_mount(Self::toolbar_control(owner))
            == Some(*mount)
            && self.panels[self.toolbar_index]
                .mounted_toolbar
                .source(Self::toolbar_group(owner))
                == Some(source);
        if !same
            || (!self.toolbar_focus_targets().contains(&owner)
                && !self.toolbar_temporarily_blocked(owner))
        {
            self.set_toolbar_focus(None, false);
            return;
        }
        if !self.toolbar_focus_targets().contains(&owner) {
            self.cancel_toolbar_input();
            return;
        }
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_none()
        {
            self.set_toolbar_focus(Some(owner), self.toolbar_focus_visible);
        }
    }
    pub(super) fn open_toolbar(&self) -> Option<ToolbarTarget> {
        self.panels.iter().enumerate().find_map(|(index, panel)| {
            let owner = panel
                .mounted_toolbar
                .host
                .content_host()
                .input_overlay_owner()?
                .0;
            self.toolbar_groups()
                .into_iter()
                .find(|&target| {
                    self.toolbar_container(target) == Some(index)
                        && Self::toolbar_control(target) == owner
                })
                .or_else(|| {
                    owner
                        .0
                        .checked_sub(1)
                        .map(ComponentId)
                        .filter(|group| panel.mounted_toolbar.source(*group).is_some())
                        .map(|group| ToolbarTarget::Contribution {
                            presenter: index,
                            group,
                        })
                })
        })
    }
    fn clear_toolbar_hover(&mut self) {
        if let Some(index) = self.toolbar_hover_index.take() {
            // A hover exit does not revoke the original keyboard focus owner.
            self.panels[index]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerMove(Point::new(f32::MIN, f32::MIN)));
        }
    }
    fn update_toolbar_hover(&mut self, target: Option<usize>, event: &ContentEvent) -> Result<()> {
        let hover = matches!(event, ContentEvent::Pointer(pointer)
            if pointer.button.is_none()
                && matches!(pointer.phase, PointerPhase::Move | PointerPhase::Enter | PointerPhase::Exit))
            || matches!(event, ContentEvent::PointerMove(_));
        if !hover
            || self.open_toolbar().is_some()
            || self.pressed.is_some()
            || self.drag_split.is_some()
            || self.settings_pointer.is_some()
            || self.panels.iter().any(|panel| {
                panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture()
                    .is_some()
            })
        {
            return Ok(());
        }
        if self.toolbar_hover_index != target {
            if let Some(index) = self.toolbar_hover_index.take() {
                // Container and keyboard routes are independent. Deliver the
                // real outside point only to the previous hover owner.
                self.panels[index]
                    .mounted_toolbar
                    .host
                    .event_with_text(event, &mut self.text)
                    .map_err(|error| anyhow::anyhow!("toolbar hover leave: {error:?}"))?;
            }
            self.toolbar_hover_index = target;
        }
        Ok(())
    }
    pub(super) fn cancel_toolbar_input(&mut self) {
        self.clear_toolbar_hover();
        self.remember_toolbar_focus(None);
        self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .event(&ContentEvent::Cancel);
    }
    pub(super) fn dispatch_toolbar(
        &mut self,
        owner: ToolbarTarget,
        event: &ContentEvent,
    ) -> Result<bool> {
        let available = self.toolbar_available(owner);
        let target = self.toolbar_container(owner).filter(|_| available);
        self.update_toolbar_hover(target, event)?;
        let Some(index) = target else {
            return Ok(true);
        };
        self.select_toolbar_container(index);
        let before = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused();
        self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .event_with_text(event, &mut self.text)
            .map_err(|e| anyhow::anyhow!("toolbar input: {e:?}"))?;
        let bubbles = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .event_bubbled();
        self.drain_toolbar_commands()?;
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            != before
        {
            self.remember_toolbar_focus(Some(!matches!(
                event,
                ContentEvent::Pointer(_)
                    | ContentEvent::PointerDown(_)
                    | ContentEvent::PointerUp(_)
            )));
        }
        Ok(bubbles)
    }
    pub(super) fn toolbar_key_input(&mut self, key: ContentKeyEvent) -> Result<bool> {
        if matches!(key.key, Some(ControlKey::Tab { .. }))
            && (key.modifiers.command || key.modifiers.control || key.modifiers.alt)
        {
            return Ok(false);
        }
        if let Some(consumed) = self.settings_tab_input(&key)? {
            return Ok(consumed);
        }
        let header_focused = self
            .presentation_tabs
            .as_ref()
            .is_some_and(|header| header.content_host().focused().is_some());
        if key.down
            && let Some(ControlKey::Tab { reverse }) = key.key
            && self.open_toolbar().is_none()
        {
            let targets = self.toolbar_focus_targets();
            if header_focused {
                if self.presentation_tab_input(&ContentEvent::KeyInput(key.clone()))? {
                    return Ok(true);
                }
                // The header uses non-wrapping original ContentHost traversal.
                // Its boundary resumes the window's existing toolbar scope.
                if targets.is_empty() {
                    return Ok(false);
                }
                self.presentation_tabs.as_mut().unwrap().blur();
                self.set_toolbar_focus(
                    if reverse {
                        targets.last()
                    } else {
                        targets.first()
                    }
                    .copied(),
                    true,
                );
                return Ok(self.actual_toolbar_owner().is_some());
            }
            let groups = self.toolbar_groups();
            let current = self.actual_toolbar_owner().or(self.toolbar_focus);
            let crosses_header = self.presentation_layout.tabs.is_some()
                && (if reverse {
                    current.is_none() || current == targets.first().copied()
                } else {
                    current.is_some() && current == targets.last().copied()
                });
            if crosses_header {
                self.set_toolbar_focus(None, false);
                if self.presentation_tab_input(&ContentEvent::KeyInput(key.clone()))? {
                    return Ok(true);
                }
            }
            let next = current
                .and_then(|current| groups.iter().position(|&target| target == current))
                .and_then(|index| {
                    (1..=groups.len())
                        .map(|step| {
                            groups[if reverse {
                                (index + groups.len() - step) % groups.len()
                            } else {
                                (index + step) % groups.len()
                            }]
                        })
                        .find(|target| targets.contains(target))
                })
                .or_else(|| {
                    if reverse {
                        targets.last().copied()
                    } else {
                        targets.first().copied()
                    }
                });
            self.set_toolbar_focus(next, true);
            return Ok(next.is_some()
                && self.panels[self.toolbar_index]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused()
                    .is_some());
        }
        if header_focused {
            return self.presentation_tab_input(&ContentEvent::KeyInput(key));
        }
        if key.down
            && key.key == Some(ControlKey::Escape)
            && !self.panels[self.toolbar_index].mounted_toolbar.modal()
        {
            self.cancel_toolbar_input();
            self.reconcile_toolbar_focus(true);
            return Ok(!self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .event_bubbled());
        }
        let previous = self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused();
        let traversal = key.down && matches!(key.key, Some(ControlKey::Tab { .. }));
        self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .event_with_text(&ContentEvent::KeyInput(key), &mut self.text)
            .map_err(|e| anyhow::anyhow!("toolbar key: {e:?}"))?;
        self.drain_toolbar_commands()?;
        self.remember_toolbar_focus(
            (traversal
                || self.panels[self.toolbar_index]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused()
                    != previous)
                .then_some(true),
        );
        Ok(!self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .event_bubbled())
    }
    pub(super) fn toolbar_target_at(&self, point: Point) -> Option<ToolbarTarget> {
        if let Some(owner) = self.open_toolbar() {
            return Some(owner);
        }
        self.toolbar_groups()
            .into_iter()
            .find(|&owner| {
                self.toolbar_available(owner)
                    && self.toolbar_container(owner).is_some_and(|index| {
                        self.panels[index]
                            .mounted_toolbar
                            .host
                            .content_host()
                            .get(Self::toolbar_control(owner))
                            .is_some_and(|n| {
                                n.frame().contains(point)
                                    && n.visible().is_some_and(|clip| clip.contains(point))
                            })
                    })
            })
            .or_else(|| {
                // Service/configuration contributions belong to their actual mounted
                // container too; the last keyboard route is not a pointer target.
                self.panels
                    .iter()
                    .enumerate()
                    .find_map(|(presenter, panel)| {
                        panel
                            .mounted_toolbar
                            .host
                            .semantics()
                            .into_iter()
                            .find_map(|node| {
                                if !node.node.visible.is_some_and(|clip| clip.contains(point)) {
                                    return None;
                                }
                                if !node
                                    .parts
                                    .iter()
                                    .any(|(_, part)| part.frame.contains(point))
                                {
                                    return None;
                                }
                                // MountedToolbar registers the original control at group+1,
                                // the same declaration identity used by overlay routing.
                                let group = ComponentId(node.node.id.0.checked_sub(1)?);
                                panel.mounted_toolbar.source(group)?;
                                let target = ToolbarTarget::Contribution { presenter, group };
                                self.toolbar_available(target).then_some(target)
                            })
                    })
            })
    }
    #[cfg(test)]
    pub(super) fn panel_geometry(
        &mut self,
        index: usize,
        bounds: Rect,
        scale: f32,
        toolbar_height: f32,
        header_extra: f32,
    ) -> Result<()> {
        let geometry = window_container_geometry::ContainerGeometry::legacy(
            bounds,
            toolbar_height + header_extra,
        );
        self.panel_geometry_layout(index, bounds, scale, geometry)
    }
    pub(super) fn panel_geometry_layout(
        &mut self,
        index: usize,
        bounds: Rect,
        scale: f32,
        geometry: window_container_geometry::ContainerGeometry,
    ) -> Result<()> {
        let toolbar_index = self
            .toolbar_container(ToolbarTarget::Panel(index))
            .unwrap_or(index);
        let previous = self.panels[index].viewport;
        self.panels[index].geometry_with_layout(bounds, scale, geometry)?;
        if self.panels[index].viewport != previous {
            let owner = ToolbarTarget::Panel(index);
            if self.panels[toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .get(Self::toolbar_control(owner))
                .is_some()
            {
                self.panels[toolbar_index]
                    .mounted_toolbar
                    .host
                    .update_component::<ToolbarControl>(
                        Self::toolbar_group(owner),
                        Self::toolbar_control(owner),
                        |_| {},
                    )
                    .map_err(|e| anyhow::anyhow!("panel toolbar geometry revoke: {e:?}"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{AppWindow, LocalContainer};
    use tekes_ui_controls::composition::{Content, TextContent, ToolbarPlacement, ToolbarRegion};
    fn mounted_contribution_fixture(
        service: bool,
    ) -> (Host, Declaration, std::rc::Rc<std::cell::Cell<usize>>) {
        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        let original = count.clone();
        let owner = LocalContainer::new(
            "owner",
            Content::new(ComponentId(1), TextContent::new("Owner")),
        )
        .unwrap()
        .toolbar_host(crate::ContainerToolbarHost::Container {
            position: crate::ContainerToolbarPosition::Top,
        })
        .toolbar(vec![ToolbarItem {
            id: 7,
            title: "Undo".into(),
            enabled: true,
            geometry_dependent: false,
        }])
        .on_command(move |_, command| {
            assert_eq!(command, 7);
            original.set(original.get() + 1);
            Ok(())
        });
        let presenter = LocalContainer::new(
            "presenter",
            Content::new(ComponentId(1), TextContent::new("Presenter")),
        )
        .unwrap();
        let mut host = AppWindow::from_columns(vec![owner.into(), presenter.into()])
            .unwrap()
            .into_host(false);
        host.mount_initial_containers();
        host.panels[0].bounds = Rect::new(0., 0., 280., 180.);
        host.panels[1].bounds = Rect::new(300., 0., 640., 180.);
        let source = if service {
            Source::Undo(ComponentSource {
                session: tekes_ui_module_runtime::component::Session {
                    module: crate::settings_undo::MODULE_ID.into(),
                    instance: 61,
                    host_epoch: 1,
                },
                registration: 4,
                window: host.panels[0].mounted_toolbar.window,
            })
        } else {
            Source::Panel {
                instance: host.panels[0].instance,
                index: 0,
                module: "owner".into(),
                pid: None,
                generation: 0,
            }
        };
        let declaration = Declaration {
            id: ComponentId(92300),
            region: ComponentId(93000),
            placement: Placement::SecondaryAction,
            order: 0,
            contents: Contents::Commands(
                vec![ButtonDecl {
                    id: 7,
                    icon: None,
                    text: "Undo".into(),
                    hint: None,
                    enabled: true,
                }],
                Overflow::Together,
            ),
            source,
        };
        mount_actual_contribution(&mut host, declaration.clone());
        (host, declaration, count)
    }
    fn mount_actual_contribution(host: &mut Host, declaration: Declaration) {
        host.panels[1]
            .mounted_toolbar
            .sync(
                Rect::new(0., 0., 1000., 200.),
                vec![ToolbarRegion::new(
                    ComponentId(93000),
                    Rect::new(300., 0., 640., 48.),
                )],
                vec![declaration],
            )
            .unwrap();
        host.panels[1]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
    }
    fn contribution_button_point(host: &Host) -> Point {
        host.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Undo")
            .unwrap()
            .1
            .frame
            .center()
    }
    fn crossing_container_hover_case(keyboard_interleave: bool) {
        use tekes_ui_scene::Primitive;
        for theme in Theme::ALL {
            let (mut host, declaration, calls) = mounted_contribution_fixture(false);
            host = host.with_theme(theme);
            host.panels[0]
                .mounted_toolbar
                .sync(
                    Rect::new(0., 0., 1000., 200.),
                    vec![ToolbarRegion::new(
                        ComponentId(93001),
                        Rect::new(0., 0., 280., 48.),
                    )],
                    vec![Declaration {
                        id: Host::toolbar_group(ToolbarTarget::Panel(0)),
                        region: ComponentId(93001),
                        placement: Placement::PrimaryAction,
                        order: 0,
                        contents: Contents::Commands(
                            vec![ButtonDecl {
                                id: 7,
                                icon: None,
                                text: "Owner A".into(),
                                hint: None,
                                enabled: true,
                            }],
                            Overflow::Together,
                        ),
                        source: declaration.source,
                    }],
                )
                .unwrap();
            let palette = host.palette();
            let paint = |host: &mut Host| {
                std::array::from_fn::<_, 2, _>(|index| {
                    let mut scene = Scene::new();
                    host.panels[index]
                        .mounted_toolbar
                        .host
                        .render(&mut scene, &mut host.text, palette)
                        .unwrap();
                    scene
                })
            };
            paint(&mut host);
            let frame = |host: &Host, index: usize, name: &str| {
                host.panels[index]
                    .mounted_toolbar
                    .host
                    .semantics()
                    .into_iter()
                    .flat_map(|node| node.parts)
                    .find(|(_, part)| part.semantics.name == name)
                    .unwrap()
                    .1
                    .frame
            };
            let a = frame(&host, 0, "Owner A");
            let b = frame(&host, 1, "Undo");
            let hover = |point| {
                ContentEvent::Pointer(PointerEvent {
                    position: point,
                    phase: PointerPhase::Move,
                    button: None,
                    modifiers: Default::default(),
                    click_count: 0,
                })
            };
            let colors = |state| {
                tekes_ui_tokens::surface_palette::resolve_surface(
                    palette,
                    "toolbar",
                    "action-quiet",
                    state,
                )
                .unwrap()
                .background
            };
            let has_fill = |scene: &Scene, frame, fill| {
                scene.primitives().iter().any(|primitive| {
                    matches!(primitive,
                        Primitive::Quad(quad) if quad.rect == frame && quad.fill == fill
                    )
                })
            };
            if keyboard_interleave {
                let target = host.panels[1]
                    .mounted_toolbar
                    .host
                    .semantics()
                    .into_iter()
                    .find(|node| {
                        node.parts
                            .iter()
                            .any(|(_, part)| part.semantics.name == "Undo")
                    })
                    .unwrap()
                    .target;
                assert!(
                    host.panels[1]
                        .mounted_toolbar
                        .host
                        .restore_focus(target, true)
                );
            }
            let focus_before: Vec<_> = host
                .panels
                .iter()
                .map(|panel| panel.mounted_toolbar.host.content_host().focused())
                .collect();
            let owner_a = host.toolbar_target_at(a.center()).unwrap();
            host.dispatch_toolbar(owner_a, &hover(a.center())).unwrap();
            let scenes = paint(&mut host);
            assert!(has_fill(&scenes[0], a, colors("hover")));
            assert!(has_fill(&scenes[1], b, colors("rest")));

            if keyboard_interleave {
                // The native key route chooses the actual focused ToolbarHost
                // before calling this original key dispatcher. Pointer hover
                // must retain its own owner through that keyboard selection.
                let focused = host
                    .panels
                    .iter()
                    .position(|panel| {
                        panel
                            .mounted_toolbar
                            .host
                            .content_host()
                            .focused()
                            .is_some()
                    })
                    .unwrap();
                host.select_toolbar_container(focused);
                host.toolbar_key_input(ContentKeyEvent {
                    key: Some(ControlKey::Left),
                    down: true,
                    repeat: false,
                    modifiers: Default::default(),
                    native_code: 123,
                    characters: String::new(),
                    characters_ignoring_modifiers: String::new(),
                })
                .unwrap();
                assert_eq!(host.toolbar_index, 1);
            }
            let owner_b = host.toolbar_target_at(b.center()).unwrap();
            host.dispatch_toolbar(owner_b, &hover(b.center())).unwrap();
            let scenes = paint(&mut host);
            assert!(
                has_fill(&scenes[0], a, colors("rest")),
                "moving directly to another container must clear the old Toolbar hover"
            );
            assert!(has_fill(&scenes[1], b, colors("hover")));
            assert_eq!(
                host.panels
                    .iter()
                    .map(|panel| { panel.mounted_toolbar.host.content_host().focused() })
                    .collect::<Vec<_>>(),
                focus_before
            );
            assert_eq!(calls.get(), 0);

            // This is route_toolbar_event's no-target body fallback: deliver
            // the real outside point to the last container, without Cancel.
            let outside = hover(Point::new(500., 150.));
            assert!(host.toolbar_target_at(Point::new(500., 150.)).is_none());
            host.panels[host.toolbar_index]
                .mounted_toolbar
                .host
                .event_with_text(&outside, &mut host.text)
                .unwrap();
            let scenes = paint(&mut host);
            assert!(has_fill(&scenes[1], b, colors("rest")));
            host.dispatch_toolbar(owner_a, &hover(a.center())).unwrap();
            host.select_toolbar_container(1);
            host.clear_toolbar_hover();
            let scenes = paint(&mut host);
            assert!(has_fill(&scenes[0], a, colors("rest")));
            assert_eq!(
                host.panels
                    .iter()
                    .map(|panel| { panel.mounted_toolbar.host.content_host().focused() })
                    .collect::<Vec<_>>(),
                focus_before,
                "hover-only cleanup must not blur the keyboard owner"
            );
            assert_eq!(host.toolbar_hover_index, None);
            // Whole-input cancellation retains its existing ContentHost
            // contract: unlike a pointer-only exit, it also clears focus.
            host.cancel_toolbar_input();
            assert_eq!(
                host.panels[1].mounted_toolbar.host.content_host().focused(),
                None
            );
            host.dispatch_toolbar(owner_b, &ContentEvent::PointerDown(b.center()))
                .unwrap();
            host.dispatch_toolbar(owner_b, &ContentEvent::PointerUp(b.center()))
                .unwrap();
            assert_eq!(
                calls.get(),
                1,
                "the original source still receives one activation"
            );
        }
    }
    #[test]
    fn pointer_crossing_container_toolbars_clears_previous_hover_without_changing_focus() {
        crossing_container_hover_case(false);
    }
    #[test]
    fn pointer_crossing_container_toolbars_after_keyboard_route_clears_actual_hover_owner() {
        crossing_container_hover_case(true);
    }
    #[test]
    fn mounted_undo_pointer_routes_to_its_actual_container_from_leading_route() {
        let (mut host, _, _) = mounted_contribution_fixture(true);
        assert_eq!(host.toolbar_index, 0);
        let point = contribution_button_point(&host);
        let owner = host
            .toolbar_target_at(point)
            .expect("the actual mounted center Undo must be discoverable from leading input route");
        assert_eq!(
            owner,
            ToolbarTarget::Contribution {
                presenter: 1,
                group: ComponentId(92300)
            }
        );
        // Shared pointer entry and Source dispatch are the production methods.
        // Native save/Undo worker completion remains a separate GUI receipt.
        assert!(
            !host
                .dispatch_toolbar(owner, &ContentEvent::PointerDown(point))
                .unwrap()
        );
        assert_eq!(host.toolbar_index, 1);
        assert_eq!(
            host.panels[1].mounted_toolbar.host.content_host().focused(),
            Some(ComponentId(92301))
        );
        assert!(
            host.panels[0]
                .mounted_toolbar
                .host
                .content_host()
                .focused()
                .is_none()
        );
        assert!(
            !host
                .dispatch_toolbar(owner, &ContentEvent::PointerUp(point))
                .unwrap()
        );
        assert!(
            host.panels[1]
                .mounted_toolbar
                .host
                .drain_commands()
                .is_empty()
        );
    }
    #[test]
    fn contribution_pointer_keeps_original_owner_and_rejects_disabled_or_replaced_source() {
        let (mut host, mut declaration, count) = mounted_contribution_fixture(false);
        let point = contribution_button_point(&host);
        let owner = host
            .toolbar_target_at(point)
            .expect("actual mounted contribution target");
        host.dispatch_toolbar(owner, &ContentEvent::PointerDown(point))
            .unwrap();
        host.dispatch_toolbar(owner, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(
            count.get(),
            1,
            "presenter must not replace original business owner"
        );
        if let Contents::Commands(buttons, _) = &mut declaration.contents {
            buttons[0].enabled = false;
        }
        mount_actual_contribution(&mut host, declaration.clone());
        let owner = host.toolbar_target_at(point).unwrap();
        host.dispatch_toolbar(owner, &ContentEvent::PointerDown(point))
            .unwrap();
        host.dispatch_toolbar(owner, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(count.get(), 1);
        if let Contents::Commands(buttons, _) = &mut declaration.contents {
            buttons[0].enabled = true;
        }
        mount_actual_contribution(&mut host, declaration.clone());
        host.dispatch_toolbar(owner, &ContentEvent::PointerDown(point))
            .unwrap();
        if let Source::Panel { generation, .. } = &mut declaration.source {
            *generation += 1;
        }
        mount_actual_contribution(&mut host, declaration);
        host.dispatch_toolbar(owner, &ContentEvent::PointerUp(point))
            .unwrap();
        host.dispatch_toolbar(owner, &ContentEvent::PointerDown(point))
            .unwrap();
        host.dispatch_toolbar(owner, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(
            count.get(),
            1,
            "retired source and its old pointer arm cannot execute"
        );
    }
    #[test]
    fn real_appwindow_ax_projection_routes_original_column_once_and_rejects_old_generation() {
        let counts = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let columns = (0..2)
            .map(|i| {
                let counts = counts.clone();
                LocalContainer::new(
                    format!("owner-{i}"),
                    Content::new(ComponentId(1), TextContent::new("Body")),
                )
                .unwrap()
                .toolbar_host(crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Top,
                })
                .toolbar(vec![ToolbarItem {
                    id: 7,
                    title: "Do work".into(),
                    enabled: true,
                    geometry_dependent: false,
                }])
                .on_command(move |_, command| {
                    counts.borrow_mut().push((i, command));
                    Ok(())
                })
                .into()
            })
            .collect();
        let mut window = AppWindow::from_columns(columns).unwrap().into_host(false);
        assert!(window.mount_columns());
        let viewport = Rect::new(0., 0., 500., 200.);
        let source = Source::Panel {
            instance: window.panels[0].instance,
            index: 0,
            module: "owner-0".into(),
            pid: None,
            generation: 0,
        };
        let declaration = Declaration {
            id: ComponentId(100),
            region: ComponentId(10),
            placement: ToolbarPlacement::Navigation,
            order: 0,
            contents: Contents::Commands(
                vec![ButtonDecl {
                    id: 7,
                    icon: None,
                    text: "Do work".into(),
                    hint: Some("Original owner".into()),
                    enabled: true,
                }],
                Overflow::Together,
            ),
            source,
        };
        window.panels[window.toolbar_index]
            .mounted_toolbar
            .sync(
                viewport,
                vec![ToolbarRegion::new(
                    ComponentId(10),
                    Rect::new(0., 0., 500., 40.),
                )],
                vec![declaration],
            )
            .unwrap();
        window.panels[window.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        let key = window
            .toolbar_accessibility_nodes(true)
            .iter()
            .find(|n| n.label == "Do work")
            .unwrap()
            .key;
        window.order.swap(0, 1); // presentation and current selection are not command sources
        window.focused = 1;
        assert!(
            window
                .toolbar_accessibility_action(
                    true,
                    key,
                    tekes_ui_platform_macos::AccessibilityAction::Press
                )
                .unwrap()
        );
        window.drain_toolbar_commands().unwrap();
        assert_eq!(&*counts.borrow(), &[(0, 7)]);
        window.panels[0].toolbar_generation += 1;
        assert!(
            window
                .toolbar_accessibility_action(
                    true,
                    key,
                    tekes_ui_platform_macos::AccessibilityAction::Press
                )
                .unwrap()
        );
        assert_eq!(
            &*counts.borrow(),
            &[(0, 7)],
            "retired generation was executed"
        );
        window.panels[window.toolbar_index]
            .mounted_toolbar
            .host
            .set_window_active(false);
        assert!(
            !window
                .toolbar_accessibility_action(
                    true,
                    key,
                    tekes_ui_platform_macos::AccessibilityAction::Press
                )
                .unwrap()
        );
        assert!(window.toolbar_accessibility_nodes(false).is_empty());
    }
}
