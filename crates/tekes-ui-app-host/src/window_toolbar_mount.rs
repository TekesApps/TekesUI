//! Native-window adapter: declaration data in, original invocations out.
use super::*;
use crate::{
    toolbar_source::ComponentSource,
    window_toolbar::{Contents, Declaration, Source},
};
use tekes_ui_controls::composition::{
    ToolbarButtonDeclaration as ButtonDecl, ToolbarGroupOverflow as Overflow,
    ToolbarPlacement as Placement, ToolbarRegion, ToolbarWindowEdge,
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
        let ToolbarBand {
            y,
            height: row,
            native_leading: safe,
        } = band;
        let mut regions: Vec<_> = frames
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut region = ToolbarRegion::new(
                    ComponentId(91000 + i as u64),
                    Rect::new(f.min_x(), y, f.size.width, row),
                );
                if f.min_x() == whole.min_x() {
                    region.native_safe_area.left = safe;
                }
                region
            })
            .collect();
        let center = usize::from(frames.len() >= 2);
        // These registrations belong to the window, not a visible panel slot.
        // The shared ToolbarHost measures/reserves both edge allocations.
        let mut leading_region = ToolbarRegion::window(ComponentId(91020),
            Rect::new(whole.min_x(), y, whole.size.width, row), ToolbarWindowEdge::Leading);
        leading_region.native_safe_area.left = safe;
        let mut trailing_region = ToolbarRegion::window(ComponentId(91021),
            Rect::new(whole.min_x(), y, whole.size.width, row), ToolbarWindowEdge::Trailing);
        trailing_region.native_safe_area.left = safe;
        regions.insert(0, leading_region);
        regions.push(trailing_region);
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
        add(92200, ComponentId(91000 + center as u64), Placement::Navigation, 20,
            Contents::Title(title), Source::Title);
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
        self.toolbar.sync(whole, regions, declarations)?;
        self.toolbar.host.set_window_active(w.is_active());
        Ok(())
    }
    pub(super) fn drain_toolbar_commands(&mut self) -> Result<()> {
        for invocation in self.toolbar.host.drain_commands() {
            if invocation.window != self.toolbar.window {
                continue;
            }
            eprintln!(
                "TOOLBAR_INVOCATION window={} command={} input={:?} source={:?}",
                invocation.window, invocation.command, invocation.input, invocation.source
            );
            let result = match invocation.source {
                Source::Visibility(side) => self.toggle_visibility(side),

                Source::Navigation(source) => self
                    .navigation
                    .as_mut()
                    .map_or(Ok(()), |nav| nav.invoke(&source, invocation.command)),
                Source::Title => Err(anyhow::anyhow!("title has no command binding")),
                Source::Panel {
                    instance,
                    index,
                    module,
                    pid,
                    generation,
                } => {
                    if let Some(p) = self.panels.get_mut(index)
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
        if let Event::Key(key) = event {
            let input = toolbar_key(key.clone());
            if matches!(input.key, Some(ControlKey::Tab { .. }))
                && (input.modifiers.command || input.modifiers.control || input.modifiers.alt)
            { return Ok(false); }
        }
        if let Event::Accessibility { target, action } = event {
            if w.content_accessibility_publication().and_then(|p|p.owner(*target))
                != Some(tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(0)) {return Ok(false);}
            let before = self.toolbar.host.content_host().focused();
            let handled =
                self.toolbar_accessibility_action(w.is_active(), *target, action.clone())?;
            if handled
                && self.toolbar.host.content_host().focused() != before
                && self.toolbar.host.content_host().focused().is_some()
            {
                for p in &mut self.panels {
                    if let Some(local) = &mut p.local {
                        local.with_host(&mut |h| h.cancel());
                    }
                }
            }
            return Ok(handled || self.toolbar.modal() || self.toolbar.host.semantics().is_empty());
        }
        let body_capture =
            self.drag_split.is_some() || self.pressed.is_some();
        if body_capture
            && matches!(event,Event::Mouse(m) if matches!(m.kind,MouseEventKind::Dragged{..}|MouseEventKind::Up{..}))
        {
            return Ok(false);
        }
        // An original body editor owns its complete native key/IME sequence.
        // A toolbar without keyboard focus must not consume its key markers.
        if self.toolbar.host.content_host().focused().is_none()
            && !self.toolbar.modal()
            && matches!(event, Event::Key(_) | Event::TextKey(_)
                | Event::TextCommit { .. } | Event::TextPreedit { .. }
                | Event::TextUnmark | Event::TextCommand(_))
        {
            let mut body_editor = false;
            for panel in &mut self.panels {
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |h| body_editor |= h.focused_text().is_some());
                }
            }
            if body_editor { return Ok(false); }
        }
        let Some(input) = native_toolbar_event(event) else {
            return Ok(false);
        };
        let previous_focus = self.toolbar.host.content_host().focused();
        let blocked = if let Event::Key(key) = event {
            self.toolbar_key_input(toolbar_key(key.clone()))?;
            !self.toolbar.host.event_bubbled()
        } else if let Some(owner) = self.open_toolbar().or_else(|| {
            if let Event::Mouse(mouse) = event { self.toolbar_target_at(Point::new(mouse.position.x as f32, mouse.position.y as f32)) } else { self.actual_toolbar_owner() }
        }) {
            !self.dispatch_toolbar(owner, &input)?
        } else {
            self.toolbar.host.event_with_text(&input, &mut self.text)
                .map_err(|e| anyhow::anyhow!("toolbar input: {e:?}"))?;
            self.drain_toolbar_commands()?;
            !self.toolbar.host.event_bubbled()
        };
        if blocked
            && self.toolbar.host.content_host().focused().is_some()
            && self.toolbar.host.content_host().focused() != previous_focus
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
        let focus_changed = self.toolbar.host.content_host().focused() != previous_focus;
        let pointer_down = matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down { .. }));
        self.remember_toolbar_focus(if blocked && pointer_down { Some(false) }
            else if focus_changed { Some(true) } else { None });
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
            ToolbarTarget::Panel(index) => 92400 + index as u64 * 10,
        })
    }
    pub(super) fn toolbar_control(target: ToolbarTarget) -> ComponentId {
        ComponentId(Self::toolbar_group(target).0 + 1)
    }
    pub(super) fn toolbar_groups(&self) -> Vec<ToolbarTarget> {
        let mut groups = Vec::new();
        if self.panels.len() >= 2 { groups.push(ToolbarTarget::Visibility(0)); }
        if self.navigation.is_some() { groups.push(ToolbarTarget::Navigation); }
        groups.extend(self.order.iter().filter(|&&i| i < self.panels.len()).map(|&i| ToolbarTarget::Panel(i)));
        if self.panels.len() == 3 { groups.push(ToolbarTarget::Visibility(1)); }
        groups
    }
    pub(super) fn actual_toolbar_owner(&self) -> Option<ToolbarTarget> {
        let focused = self.toolbar.host.content_host().focused()?;
        self.toolbar_groups().into_iter().find(|&owner| Self::toolbar_control(owner) == focused)
    }
    pub(super) fn remember_toolbar_focus(&mut self, visible: Option<bool>) {
        let Some(owner) = self.actual_toolbar_owner() else { return; };
        let Some(mount) = self.toolbar.host.content_host().semantic_mount(Self::toolbar_control(owner)) else { return; };
        let Some(source) = self.toolbar.source(Self::toolbar_group(owner)).cloned() else { return; };
        self.toolbar_focus = Some(owner);
        self.toolbar_focus_lease = Some((mount, source));
        if let Some(visible) = visible { self.toolbar_focus_visible = visible; }
    }
    pub(super) fn toolbar_available(&self, target: ToolbarTarget) -> bool {
        match target {
            ToolbarTarget::Visibility(side) => self.panels.len() == 3 || self.panels.len() == 2 && side == 0,
            ToolbarTarget::Navigation => self.navigation.as_ref().is_some_and(|n| n.host.input_open && !n.busy && n.error.is_none()),
            ToolbarTarget::Panel(i) => self.panels.get(i).is_some_and(|p| {
                p.bounds.size.width > 0. && !self.sidebar_motion.needs_frame()
                    && (self.panels.len() < 2 || (i != self.order[0] || self.leading_visible)
                        && (self.panels.len() != 3 || i != self.order[2] || self.trailing_visible))
                    && self.navigation.as_ref().is_none_or(|n| n.host.input_open && !n.busy)
            }),
        }
    }
    pub(super) fn toolbar_temporarily_blocked(&self, target: ToolbarTarget) -> bool {
        let navigating = self.navigation.as_ref().is_some_and(|n| n.busy && n.error.is_none());
        match target {
            ToolbarTarget::Navigation => navigating,
            ToolbarTarget::Panel(i) => self.panels.get(i).is_some_and(|p| {
                (navigating || self.sidebar_motion.needs_frame() || p.frame_viewport != p.viewport)
                    && p.failed.is_none() && p.bounds.size.width > 0.
                    && (self.panels.len() < 2 || (i != self.order[0] || self.leading_visible)
                        && (self.panels.len() != 3 || i != self.order[2] || self.trailing_visible))
            }),
            ToolbarTarget::Visibility(_) => false,
        }
    }
    pub(super) fn toolbar_focus_targets(&self) -> Vec<ToolbarTarget> {
        self.toolbar_groups().into_iter().filter(|&owner| self.toolbar_available(owner)
            && self.toolbar.host.content_host().get(Self::toolbar_control(owner))
                .is_some_and(|n| n.component::<ToolbarControl>().is_some_and(tekes_ui_controls::composition::Component::focusable)))
            .collect()
    }
    pub(super) fn set_toolbar_focus(&mut self, target: Option<ToolbarTarget>, visible: bool) {
        let target = target.filter(|t| self.toolbar_focus_targets().contains(t));
        let Some(owner) = target else {
            self.toolbar.host.event(&ContentEvent::Cancel);
            self.toolbar_focus = None;
            self.toolbar_focus_lease = None;
            self.toolbar_focus_visible = false;
            return;
        };
        let Some(target) = self.toolbar.host.semantics().into_iter()
            .find(|n| n.node.id == Self::toolbar_control(owner)).map(|n| n.target) else { return; };
        if self.toolbar.host.restore_focus(target, visible) {
            self.remember_toolbar_focus(Some(visible));
            if let ToolbarTarget::Panel(i) = owner { self.focused = i; }
        }
    }
    pub(super) fn reconcile_toolbar_focus(&mut self, active: bool) {
        if !active { self.cancel_toolbar_input(); return; }
        let Some(owner) = self.toolbar_focus else { return; };
        let Some((mount, source)) = self.toolbar_focus_lease.as_ref() else { return; };
        // Never renew a lease from a reused numeric ID or a replaced source.
        let same = self.toolbar.host.content_host().semantic_mount(Self::toolbar_control(owner)) == Some(*mount)
            && self.toolbar.source(Self::toolbar_group(owner)) == Some(source);
        if !same || (!self.toolbar_focus_targets().contains(&owner) && !self.toolbar_temporarily_blocked(owner)) {
            self.set_toolbar_focus(None, false);
            return;
        }
        if !self.toolbar_focus_targets().contains(&owner) {
            self.cancel_toolbar_input();
            return;
        }
        if self.toolbar.host.content_host().focused().is_none() {
            self.set_toolbar_focus(Some(owner), self.toolbar_focus_visible);
        }
    }
    pub(super) fn open_toolbar(&self) -> Option<ToolbarTarget> {
        let owner = self.toolbar.host.content_host().input_overlay_owner()?.0;
        self.toolbar_groups().into_iter().find(|&target| Self::toolbar_control(target) == owner)
    }
    pub(super) fn cancel_toolbar_input(&mut self) {
        self.remember_toolbar_focus(None);
        self.toolbar.host.event(&ContentEvent::Cancel);
    }
    pub(super) fn dispatch_toolbar(&mut self, owner: ToolbarTarget, event: &ContentEvent) -> Result<bool> {
        if !self.toolbar_available(owner) { return Ok(true); }
        let before = self.toolbar.host.content_host().focused();
        self.toolbar.host.event_with_text(event, &mut self.text)
            .map_err(|e| anyhow::anyhow!("toolbar input: {e:?}"))?;
        let bubbles = self.toolbar.host.event_bubbled();
        self.drain_toolbar_commands()?;
        if self.toolbar.host.content_host().focused() != before {
            self.remember_toolbar_focus(Some(!matches!(event,ContentEvent::Pointer(_) | ContentEvent::PointerDown(_) | ContentEvent::PointerUp(_))));
        }
        Ok(bubbles)
    }
    pub(super) fn toolbar_key_input(&mut self, key: ContentKeyEvent) -> Result<()> {
        if matches!(key.key, Some(ControlKey::Tab { .. }))
            && (key.modifiers.command || key.modifiers.control || key.modifiers.alt) { return Ok(()); }
        // Only a temporarily suspended window intent needs boundary selection.
        // Every ordinary traversal and roving event stays in the one Host.
        if self.toolbar.host.content_host().focused().is_none()
            && self.toolbar_focus.is_some_and(|owner| self.toolbar_temporarily_blocked(owner))
            && key.down && let Some(ControlKey::Tab { reverse }) = key.key
        {
            let groups = self.toolbar_groups();
            let targets = self.toolbar_focus_targets();
            let next = self.toolbar_focus.and_then(|owner| groups.iter().position(|&g|g==owner))
                .and_then(|i|(1..=groups.len()).map(|step|groups[if reverse {(i+groups.len()-step)%groups.len()}else{(i+step)%groups.len()}]).find(|g|targets.contains(g)));
            self.set_toolbar_focus(next, true);
            return Ok(());
        }
        if key.down && key.key == Some(ControlKey::Escape) && self.open_toolbar().is_none() {
            self.cancel_toolbar_input();
            self.reconcile_toolbar_focus(true);
            return Ok(());
        }
        let previous = self.toolbar.host.content_host().focused();
        let traversal = key.down && matches!(key.key, Some(ControlKey::Tab { .. }));
        self.toolbar.host.event_with_text(&ContentEvent::KeyInput(key), &mut self.text)
            .map_err(|e| anyhow::anyhow!("toolbar key: {e:?}"))?;
        self.drain_toolbar_commands()?;
        self.remember_toolbar_focus((traversal || self.toolbar.host.content_host().focused() != previous).then_some(true));
        Ok(())
    }
    pub(super) fn toolbar_target_at(&self, point: Point) -> Option<ToolbarTarget> {
        if let Some(owner) = self.open_toolbar() { return Some(owner); }
        self.toolbar_groups().into_iter().find(|&owner| self.toolbar_available(owner)
            && self.toolbar.host.content_host().get(Self::toolbar_control(owner)).is_some_and(|n| n.frame().contains(point)))
    }
    pub(super) fn panel_geometry(&mut self, index: usize, bounds: Rect, scale: f32, toolbar_height: f32) -> Result<()> {
        self.remember_toolbar_focus(None);
        let previous = self.panels[index].viewport;
        self.panels[index].geometry(bounds, scale, toolbar_height)?;
        if self.panels[index].viewport != previous {
            let owner = ToolbarTarget::Panel(index);
            if self.toolbar.host.content_host().get(Self::toolbar_control(owner)).is_some() {
                self.toolbar.host.update_component::<ToolbarControl>(Self::toolbar_group(owner), Self::toolbar_control(owner), |_| {})
                    .map_err(|e|anyhow::anyhow!("panel toolbar geometry revoke: {e:?}"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{AppWindow, Container};
    use tekes_ui_controls::composition::{Content, TextContent, ToolbarPlacement, ToolbarRegion};
    #[test]
    fn real_appwindow_ax_projection_routes_original_column_once_and_rejects_old_generation() {
        let counts = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let columns = (0..2)
            .map(|i| {
                let counts = counts.clone();
                Container::new(
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
        window
            .toolbar
            .sync(
                viewport,
                vec![ToolbarRegion::new(
                    ComponentId(10),
                    Rect::new(0., 0., 500., 40.),
                )],
                vec![declaration],
            )
            .unwrap();
        window
            .toolbar
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
        window.toolbar.host.set_window_active(false);
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
