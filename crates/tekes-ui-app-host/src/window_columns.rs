//! Local column routing uses the original ContentHost; process columns keep IPC.
use super::*;
impl Host {
    pub(super) fn content_accessibility_nodes(
        &mut self,
        active: bool,
    ) -> Vec<tekes_ui_platform_macos::AccessibilityNode> {
        let mut nodes = self.toolbar_accessibility_nodes(active);
        if self.open_toolbar().is_some() {
            return nodes;
        }
        for panel in &mut self.panels {
            if panel.bounds.size.width <= 0. {
                continue;
            }
            if let Some(local) = &mut panel.local {
                local.with_host(&mut |h| {
                    nodes.extend(crate::accessibility::content_nodes(
                        h,
                        active && !self.sidebar_motion.needs_frame(),
                    ))
                });
            }
        }
        nodes
    }
    pub(super) fn local_text_state(
        &mut self,
        active: bool,
    ) -> Option<tekes_ui_platform_macos::TextInputState> {
        if !active
            || self.sidebar_motion.needs_frame()
            || self.toolbar_focus.is_some()
            || self.open_toolbar().is_some()
        {
            return None;
        }
        let mut state = None;
        self.panels
            .get_mut(self.focused)?
            .local
            .as_mut()?
            .with_host(&mut |h| state = crate::columns::text_state(h));
        state
    }
    pub(super) fn local_content_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        if !w.is_active()
            || self.sidebar_motion.needs_frame()
            || self.open_toolbar().is_some()
            || self
                .navigation
                .as_ref()
                .is_some_and(|n| n.busy || !n.host.input_open)
        {
            return Ok(false);
        }
        if let Event::Accessibility { target, action } = event {
            for (i, panel) in self.panels.iter_mut().enumerate() {
                if panel.bounds.size.width <= 0. {
                    continue;
                }
                if let Some(local) = &mut panel.local {
                    let mut consumed = false;
                    local.with_host(&mut |h| {
                        consumed =
                            crate::accessibility::content_action(h, true, *target, action.clone())
                                .consumed
                    });
                    if consumed {
                        local.dispatch_actions();
                        self.focused = i;
                        self.set_toolbar_focus(None, false);
                        return Ok(true);
                    }
                }
            }
            return Ok(false);
        }
        let index = if let Event::Scroll(s) = event {
            let point = Point::new(s.position.x as f32, s.position.y as f32);
            self.panels.iter().position(|p| {
                p.local.is_some() && content_geometry(p.bounds, p.toolbar_height).contains(point)
            })
        } else if self.toolbar_focus.is_none() {
            Some(self.focused)
        } else {
            None
        };
        let Some(local) = index
            .and_then(|i| self.panels.get_mut(i))
            .and_then(|p| p.local.as_mut())
        else {
            return Ok(false);
        };
        let input = match event {
            Event::Key(k) if k.modifiers.command && k.kind == KeyEventKind::Down => {
                match k.characters_ignoring_modifiers.to_lowercase().as_str() {
                    "a" => ContentEvent::TextCommand("selectAll:".into()),
                    "z" => ContentEvent::TextCommand(
                        if k.modifiers.shift { "redo:" } else { "undo:" }.into(),
                    ),
                    "c" | "x" => {
                        local.with_host(&mut |h| {
                            if let Some(s) = h.focused_text() {
                                w.set_clipboard_text(&s.input.selected_text());
                            }
                        });
                        if k.characters_ignoring_modifiers == "c" {
                            return Ok(true);
                        }
                        ContentEvent::TextCommand("deleteBackward:".into())
                    }
                    "v" => ContentEvent::Commit {
                        text: w.clipboard_text().unwrap_or_default(),
                        replacement: None,
                    },
                    _ => return Ok(false),
                }
            }
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
            Event::TextUnmark => ContentEvent::Unmark,
            Event::TextCommand(command) if command == "copy:" || command == "cut:" => {
                local.with_host(&mut |h| {
                    if let Some(s) = h.focused_text() {
                        w.set_clipboard_text(&s.input.selected_text());
                    }
                });
                if command == "copy:" {
                    return Ok(true);
                }
                ContentEvent::TextCommand("deleteBackward:".into())
            }
            Event::TextCommand(command) if command == "paste:" => ContentEvent::Commit {
                text: w.clipboard_text().unwrap_or_default(),
                replacement: None,
            },
            Event::TextCommand(command) => ContentEvent::TextCommand(command.clone()),
            Event::Scroll(s) => ContentEvent::Scroll {
                position: Point::new(s.position.x as f32, s.position.y as f32),
                delta: Point::new(-s.delta.x as f32, -s.delta.y as f32),
                precise: s.precise,
                begin: false,
                end: false,
                momentum: false,
                time: self.motion_epoch.elapsed().as_secs_f64(),
            },
            Event::CaretBlink => return Ok(true),
            _ => return Ok(false),
        };
        local.event(&input, &mut self.text)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_columns_mount_without_launching_local_workers() {
        use crate::{AppWindow, Container, ProcessContainer};
        use tekes_ui_controls::composition::{Content, TextFieldContent};
        let local = |id| {
            Container::new(
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
        assert_eq!(host.panels.len(), 3);
        assert!(host.panels[0].local.is_some());
        assert!(host.panels[1].local.is_none());
        assert!(host.panels[2].local.is_some());
        assert!(host.panels[0].launch_tx.is_none());
        assert!(host.panels[1].launch_tx.is_some());
        assert!(host.panels.iter().all(|p| p.pid.is_none()));
    }
}
