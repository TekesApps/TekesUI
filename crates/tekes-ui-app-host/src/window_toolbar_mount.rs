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
        let center = usize::from(frames.len() >= 2);
        let mut declarations = vec![];
        let renderer = self.renderer.as_mut().unwrap();
        let scale = w.scale() as f32 * self.text.type_scale();
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
                "apphost-leading-collapsed",
                "leading",
            ),
            (
                1,
                self.trailing_visible,
                ComponentId(91021),
                "tekes-side-panel",
                "apphost-trailing-collapsed",
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
            let image = crate::window_toolbar::icon_image(
                &mut self.toolbar_images,
                renderer,
                if on { icon } else { collapsed },
                scale,
            )?;
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
                Contents::Commands(
                    vec![ButtonDecl {
                        id: 0x5601 + side as u64,
                        icon: Some(image),
                        text: hint.clone(),
                        hint: Some(hint),
                        enabled: true,
                    }],
                    Overflow::Together,
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
            for action in &nav.actions {
                let icon = action
                    .icon
                    .as_deref()
                    .map(|name| {
                        crate::window_toolbar::icon_image(
                            &mut self.toolbar_images,
                            renderer,
                            name,
                            scale,
                        )
                    })
                    .transpose()?;
                buttons.push(ButtonDecl {
                    id: action.command.id,
                    icon,
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
                Contents::Commands(buttons, Overflow::Together),
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
                for action in state.toolbar_actions() {
                    let icon = action
                        .icon
                        .as_deref()
                        .map(|name| {
                            crate::window_toolbar::icon_image(
                                &mut self.toolbar_images,
                                renderer,
                                name,
                                scale,
                            )
                        })
                        .transpose()?;
                    buttons.push(ButtonDecl {
                        id: action.command.id,
                        icon,
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
                    Contents::Commands(buttons, Overflow::Together),
                    Source::Undo(source.clone()),
                );
            }
        }
        for (slot, &index) in self.order.iter().enumerate() {
            let p = &self.panels[index];
            if frames[slot].size.width <= 0. || p.toolbar.is_empty() {
                continue;
            }
            let buttons = p
                .toolbar
                .iter()
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
            add(
                92400 + index as u64 * 10,
                ComponentId(91000 + slot as u64),
                Placement::PrimaryAction,
                10,
                Contents::Commands(buttons, Overflow::Split),
                Source::Panel {
                    instance: p.instance,
                    index,
                    module: p.module_id.clone(),
                    pid: p.pid,
                    generation: p.toolbar_generation,
                },
            );
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
        let fallback = self.order.iter().copied().find(|&i| visible(i));
        let mut owned = vec![Vec::new(); self.panels.len()];
        for mut declaration in declarations {
            let owner = match declaration.source {
                Source::Panel { index, .. } => Some(index),
                Source::Visibility(0) => self
                    .order
                    .first()
                    .copied()
                    .filter(|&i| visible(i))
                    .or(fallback),
                Source::Visibility(_) => self
                    .order
                    .last()
                    .copied()
                    .filter(|&i| visible(i))
                    .or(fallback),
                Source::Navigation(_) => fallback,
                Source::Undo(_) | Source::Title => self
                    .order
                    .get(center)
                    .copied()
                    .filter(|&i| visible(i))
                    .or(fallback),
            };
            if let Some(owner) = owner {
                declaration.region = ComponentId(91000 + owner as u64);
                owned[owner].push(declaration);
            }
        }
        for (index, panel) in self.panels.iter_mut().enumerate() {
            let slot = self.order.iter().position(|&i| i == index);
            let frame = slot.map(|slot| frames[slot]).unwrap_or_default();
            let mut region = ToolbarRegion::new(
                ComponentId(91000 + index as u64),
                Rect::new(frame.min_x(), y, frame.size.width, row),
            );
            if frame.min_x() == whole.min_x() {
                region.native_safe_area.left = safe;
            }
            let declarations = std::mem::take(&mut owned[index]);
            panel.mounted_toolbar.sync(
                whole,
                if frame.size.width > 0. {
                    vec![region]
                } else {
                    Vec::new()
                },
                declarations,
            )?;
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
                    Source::Title => Err(anyhow::anyhow!("title has no command binding")),
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
                        } else if let Some(p) = self.panels.get_mut(index)
                            && p.instance == instance
                            && p.module_id == module
                            && p.pid == pid
                            && p.toolbar_generation == generation
                        {
                            p.command(invocation.command)
                        } else {
                            Ok(())
                        }
                    }
                };
            if let Err(error) = result {
                eprintln!("TOOLBAR_COMMAND_REJECTED {error:#}");
            }
        }
        Ok(())
    }
    /// Single shared event entry before any body fallback. Existing body/splitter
    /// captures keep their original owner, including when crossing toolbar chrome.
    pub(super) fn route_toolbar_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
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
        // An original body editor owns its complete native key/IME sequence.
        // A toolbar without keyboard focus must not consume its key markers.
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_none()
            && !self.panels[self.toolbar_index].mounted_toolbar.modal()
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
            let mut body_editor = false;
            for panel in &mut self.panels {
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |h| body_editor |= h.focused_text().is_some());
                }
            }
            if body_editor {
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
            self.toolbar_key_input(toolbar_key(key.clone()))?;
            !self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .event_bubbled()
        } else if let Some(owner) = self.open_toolbar().or_else(|| {
            if let Event::Mouse(mouse) = event {
                self.toolbar_target_at(Point::new(mouse.position.x as f32, mouse.position.y as f32))
            } else {
                self.actual_toolbar_owner()
            }
        }) {
            !self.dispatch_toolbar(owner, &input)?
        } else {
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
        if let ToolbarTarget::Panel(index) = target {
            return self
                .panels
                .get(index)
                .filter(|panel| panel.mounted_toolbar.source(group).is_some())
                .map(|_| index);
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
                for source in panel.mounted_toolbar.sources() {
                    let target = match source {
                        Source::Panel { index, .. } if *index == presenter => {
                            Some(ToolbarTarget::Panel(*index))
                        }
                        Source::Panel { index, .. } => Some(ToolbarTarget::Retained {
                            owner: *index,
                            presenter,
                        }),
                        Source::Navigation(_) => Some(ToolbarTarget::Navigation),
                        _ => None,
                    };
                    if let Some(target) = target
                        && !groups.contains(&target)
                    {
                        groups.push(target);
                    }
                }
            }
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
    pub(super) fn toolbar_available(&self, target: ToolbarTarget) -> bool {
        if let ToolbarTarget::Contribution { presenter, group } = target {
            return self.panels.get(presenter).is_some_and(|panel| {
                panel.bounds.size.width > 0. && panel.mounted_toolbar.source(group).is_some()
            });
        }
        if self.presentation.is_some() {
            return match target {
                ToolbarTarget::Panel(index) | ToolbarTarget::Retained { owner: index, .. } => {
                    self.toolbar_container(target).is_some_and(|presenter| {
                        let panel = &self.panels[index];
                        let target = &self.panels[presenter];
                        panel.failed.is_none()
                            && self.navigation.as_ref().is_none_or(|nav| {
                                nav.host.input_open && !nav.busy && nav.error.is_none()
                            })
                            && target.bounds.size.width > 0.
                            && target.bounds.size.height > 0.
                    })
                }
                ToolbarTarget::Navigation => self
                    .navigation
                    .as_ref()
                    .is_some_and(|nav| nav.host.input_open && !nav.busy && nav.error.is_none()),
                ToolbarTarget::Visibility(_) | ToolbarTarget::Contribution { .. } => false,
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
        if self.presentation.is_some() {
            return match target {
                ToolbarTarget::Panel(index) | ToolbarTarget::Retained { owner: index, .. } => {
                    self.toolbar_container(target).is_some_and(|presenter| {
                        let panel = &self.panels[index];
                        let visible = &self.panels[presenter];
                        panel.failed.is_none()
                            && visible.bounds.size.width > 0.
                            && visible.bounds.size.height > 0.
                            && (self
                                .navigation
                                .as_ref()
                                .is_some_and(|nav| nav.busy && nav.error.is_none())
                                || panel.frame_viewport != panel.viewport)
                    })
                }
                _ => false,
            };
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
            tabs.cancel();
            tabs.focus(None, false);
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
            if let ToolbarTarget::Panel(i) | ToolbarTarget::Retained { owner: i, .. } = owner {
                self.focused = i;
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
    pub(super) fn cancel_toolbar_input(&mut self) {
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
        if !self.toolbar_available(owner) {
            return Ok(true);
        }
        let Some(index) = self.toolbar_container(owner) else {
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
    pub(super) fn toolbar_key_input(&mut self, key: ContentKeyEvent) -> Result<()> {
        if matches!(key.key, Some(ControlKey::Tab { .. }))
            && (key.modifiers.command || key.modifiers.control || key.modifiers.alt)
        {
            return Ok(());
        }
        if key.down
            && let Some(ControlKey::Tab { reverse }) = key.key
            && self.open_toolbar().is_none()
        {
            let targets = self.toolbar_focus_targets();
            let groups = self.toolbar_groups();
            let current = self.actual_toolbar_owner().or(self.toolbar_focus);
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
            return Ok(());
        }
        if key.down
            && key.key == Some(ControlKey::Escape)
            && !self.panels[self.toolbar_index].mounted_toolbar.modal()
        {
            self.cancel_toolbar_input();
            self.reconcile_toolbar_focus(true);
            return Ok(());
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
        Ok(())
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
                            .is_some_and(|n| n.frame().contains(point))
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
                                if !node.node.visible.is_some_and(|clip| clip.contains(point))
                                    || !node
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
    pub(super) fn panel_geometry(
        &mut self,
        index: usize,
        bounds: Rect,
        scale: f32,
        toolbar_height: f32,
    ) -> Result<()> {
        self.remember_toolbar_focus(None);
        let toolbar_index = self
            .toolbar_container(ToolbarTarget::Panel(index))
            .unwrap_or(index);
        let previous = self.panels[index].viewport;
        self.panels[index].geometry(bounds, scale, toolbar_height)?;
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
