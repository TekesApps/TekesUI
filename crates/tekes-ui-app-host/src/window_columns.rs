//! Local column routing uses the original ContentHost; process columns keep IPC.
use super::*;
impl Host {
    /// Prepare all original Hosts in actual paint order; toolbar is composited last.
    pub(super) fn content_accessibility_publication(
        &mut self,
        active: bool,
    ) -> Option<tekes_ui_platform_macos::content_accessibility::WindowAccessibilityPublication>
    {
        use tekes_ui_platform_macos::content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot, WindowAccessibilityPublication,
        };
        if !active {
            return WindowAccessibilityPublication::prepare(Vec::new());
        }
        self.toolbar_accessibility_nodes(active); // refresh original stamped target map
        let mut fragments = Vec::new();
        if !self.toolbar.modal() {
            for (i, panel) in self.panels.iter_mut().enumerate() {
                if panel.bounds.size.width <= 0.
                    || (self.is_settings_window
                        && i == 0
                        && (!self.leading_visible || self.sidebar_motion.needs_frame()))
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
        let mut keys = std::collections::HashMap::new();
        for node in self.toolbar.host.semantics() {
            for (target, part) in std::iter::once((node.target, None))
                .chain(node.parts.iter().map(|(t, p)| (*t, Some(p.index))))
            {
                if let Some((key, _)) = self.toolbar_ax.iter().find(|(_, t)| *t == target) {
                    keys.insert(
                        tekes_ui_platform_macos::content_accessibility::content_key(
                            self.toolbar.host.content_host(),
                            node.node.id,
                            part,
                        )?,
                        *key,
                    );
                }
            }
        }
        let toolbar = ContentAccessibilitySnapshot::from_host(self.toolbar.host.content_host())?
            .with_native_keys(&keys)?;
        fragments.push((ContentAccessibilityOwner(0), toolbar));
        WindowAccessibilityPublication::prepare(fragments)
    }
    pub(super) fn local_text_state(
        &mut self,
        active: bool,
    ) -> Option<tekes_ui_platform_macos::TextInputState> {
        if !active || self.sidebar_motion.needs_frame() || self.open_toolbar().is_some() {
            return None;
        }
        if let Some(state) = crate::columns::text_state(self.toolbar.host.content_host()) {
            return Some(state);
        }
        if self.toolbar.host.content_host().focused().is_some() {
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
            let Some(publication) = w.content_accessibility_publication() else {
                return Ok(false);
            };
            let Some(owner) = publication.owner(*target) else {
                return Ok(false);
            };
            let Some(index) = owner.0.checked_sub(1).and_then(|i| usize::try_from(i).ok()) else {
                return Ok(false);
            };
            let Some(panel) = self.panels.get_mut(index) else {
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
                self.focused = index;
                self.set_toolbar_focus(None, false);
            }
            return Ok(consumed);
        }
        let index = if let Event::Scroll(s) = event {
            let point = Point::new(s.position.x as f32, s.position.y as f32);
            self.panels.iter().position(|p| {
                p.local.is_some() && content_geometry(p.bounds, p.toolbar_height).contains(point)
            })
        } else if self.toolbar.host.content_host().focused().is_none() {
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
            Event::TextKey(marker) => {
                window_toolbar_mount::native_toolbar_event(&Event::TextKey(marker.clone())).unwrap()
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

#[cfg(test)]
mod window_publication_tests {
    use super::*;
    use crate::window_toolbar::{Contents, Declaration, Source};
    use crate::{AppWindow, Container};
    use tekes_ui_controls::composition::*;
    use tekes_ui_controls::composition::{
        ToolbarButtonDeclaration as ButtonDecl, ToolbarGroupOverflow as Overflow,
    };
    use tekes_ui_platform_macos::{
        AccessibilityAction, AccessibilityRole, content_accessibility::ContentAccessibilityOwner,
    };
    use tekes_ui_tokens::Theme;
    #[test]
    fn actual_apphost_three_hosts_keep_original_owners_and_open_toolbar_excludes_background() {
        let columns = vec![
            Container::new(
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
            Container::new(
                "plain",
                Content::new(ComponentId(8200), Button::new("Body button")),
            )
            .unwrap()
            .into(),
        ];
        let mut window = AppWindow::from_columns(columns).unwrap().into_host(false);
        assert!(window.mount_columns());
        for (i, panel) in window.panels.iter_mut().enumerate() {
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
            instance: window.panels[0].instance,
            index: 0,
            module: "curve".into(),
            pid: None,
            generation: 0,
        };
        window
            .toolbar
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
        window
            .toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        assert!(!window.toolbar.modal());
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
            Some(ContentAccessibilityOwner(0))
        );
        assert!(
            window
                .toolbar_accessibility_action(true, overflow, AccessibilityAction::Press)
                .unwrap()
        );
        window
            .toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        assert!(window.toolbar.modal());
        let modal = window.content_accessibility_publication(true).unwrap();
        assert!(modal.owner(composer).is_none());
        assert!(modal.owner(body).is_none());
        assert!(!modal.nodes().is_empty());
        assert!(
            modal
                .nodes()
                .iter()
                .all(|n| modal.owner(n.key) == Some(ContentAccessibilityOwner(0)))
        );
        assert!(
            window
                .content_accessibility_publication(false)
                .unwrap()
                .nodes()
                .is_empty()
        );
    }
}
