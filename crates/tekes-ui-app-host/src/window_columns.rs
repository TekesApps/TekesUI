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
        if self.open_toolbar().is_none() {
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
        let modal = self
            .panels
            .iter()
            .position(|panel| panel.mounted_toolbar.modal());
        for (index, panel) in self.panels.iter().enumerate() {
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
                        .toolbar_ax
                        .iter()
                        .find(|(_, i, t)| *i == index && *t == target)
                    {
                        keys.insert(
                            tekes_ui_platform_macos::content_accessibility::content_key(
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
        WindowAccessibilityPublication::prepare(fragments)
    }
    pub(super) fn local_text_state(
        &mut self,
        active: bool,
    ) -> Option<tekes_ui_platform_macos::TextInputState> {
        if !active
            || self.sidebar_motion.needs_frame()
            || self.open_toolbar().is_some()
            || self
                .presentation_tabs
                .as_ref()
                .is_some_and(|tabs| tabs.content_host().focused().is_some())
        {
            return None;
        }
        if let Some(state) = crate::columns::text_state(
            self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .content_host(),
        ) {
            return Some(state);
        }
        if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_some()
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
            || self
                .presentation_tabs
                .as_ref()
                .is_some_and(|tabs| tabs.content_host().focused().is_some())
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
            self.panels.iter().position(|p| {
                p.local.is_some()
                    && p.content_frame().contains(point)
                    && (self.presentation.is_none() || p.clip.contains(point))
            })
        } else if self.panels[self.toolbar_index]
            .mounted_toolbar
            .host
            .content_host()
            .focused()
            .is_none()
        {
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
        // Unclaimed application/window shortcuts must retain the outer fallback.
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Down
            && key.modifiers.command
            && (key.modifiers.control
                || key.modifiers.alt
                || !matches!(
                    key.characters_ignoring_modifiers
                        .to_ascii_lowercase()
                        .as_str(),
                    "a" | "z" | "c" | "x" | "v"
                ))
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
                    eprintln!(
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
            Event::CaretBlink => Ok(true),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    use crate::{AppWindow, LocalContainer};
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
        window.panels[window.toolbar_index]
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
        window.panels[window.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        assert!(!window.panels[window.toolbar_index].mounted_toolbar.modal());
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
                Host::toolbar_accessibility_owner(window.toolbar_index)
            ))
        );
        assert!(
            window
                .toolbar_accessibility_action(true, overflow, AccessibilityAction::Press)
                .unwrap()
        );
        window.panels[window.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut window.text, Theme::LIGHT.into())
            .unwrap();
        assert!(window.panels[window.toolbar_index].mounted_toolbar.modal());
        let modal = window.content_accessibility_publication(true).unwrap();
        assert!(modal.owner(composer).is_none());
        assert!(modal.owner(body).is_none());
        assert!(!modal.nodes().is_empty());
        assert!(modal.nodes().iter().all(|n| modal.owner(n.key)
            == Some(ContentAccessibilityOwner(
                Host::toolbar_accessibility_owner(window.toolbar_index)
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
        let local = host.panels[0].local.as_mut().unwrap();
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
                    .surface
                    .input
                    .select_range(selection.clone())
            );
        });
        host
    }
    fn value(host: &mut Host) -> String {
        let mut value = String::new();
        host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
            value = h
                .get(ComponentId(1))
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text
                .clone();
        });
        value
    }
    #[test]
    fn local_clipboard_empty_cut_preserves_text_and_clipboard() {
        let mut host = host(3..3);
        let clipboard = Clipboard::new(Some("keep"));
        assert!(
            host.local_content_input(&Event::TextCommand("cut:".into()), &clipboard)
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
            host.local_content_input(&Event::TextCommand("paste:".into()), &clipboard)
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
            host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
                assert_eq!(h.focused_text().unwrap().input.selection(), selection);
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
    fn local_clipboard_native_span_raw_and_selector_edit_once() {
        use tekes_ui_platform_macos::{TextKeyContext, TextKeyDisposition, TextKeyEvent};
        for (letter, command, expected) in [("x", "cut:", "a"), ("v", "paste:", "aNEW")] {
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
                Event::TextCommand(command.into()),
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
            host.local_content_input(&Event::TextCommand("undo:".into()), &clipboard)
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
                Event::TextCommand("cut:".into()),
                Event::TextCommand("paste:".into()),
            ] {
                let mut host = host(1..3);
                let clipboard = Clipboard::new(Some("keep"));
                host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
                    let input = &mut h
                        .get_mut(ComponentId(1))
                        .unwrap()
                        .component_mut::<TextFieldContent>()
                        .unwrap()
                        .surface
                        .input;
                    match mode {
                        "readonly" => input.read_only = true,
                        "disabled" => input.disabled = true,
                        _ => input.marked = Some(1..3),
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
        for command in ["cut:", "copy:"] {
            let mut host = host(1..3);
            let clipboard = Clipboard::new(Some("keep"));
            host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
                let input = &mut h
                    .get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<TextFieldContent>()
                    .unwrap()
                    .surface
                    .input;
                input.text = "A😀B".into();
                input.select_range(1..3);
                input.read_only = command == "copy:";
            });
            host.local_content_input(&Event::TextCommand(command.into()), &clipboard)
                .unwrap();
            assert_eq!(
                value(&mut host),
                if command == "cut:" { "AB" } else { "A😀B" }
            );
            assert_eq!(clipboard.writes.borrow().as_slice(), ["😀"]);
        }
    }
    #[test]
    fn local_clipboard_keeps_text_service_layout_for_noop_and_edit() {
        let mut host = host(3..3);
        let clipboard = Clipboard::new(Some("keep"));
        host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
            h.get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap()
                .surface
                .invalidate_text_layout();
        });
        host.local_content_input(&Event::TextCommand("cut:".into()), &clipboard)
            .unwrap();
        host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
            assert!(h.focused_text().unwrap().paragraph.is_some());
            // The same service scope prepares before a platform snapshot/claim,
            // and refreshes again after the dispatch mutates the original field.
            h.get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap()
                .surface
                .invalidate_text_layout();
            h.with_text_input(&mut host.text, |h| {
                assert!(h.focused_text().unwrap().paragraph.is_some());
                h.event(&ContentEvent::Commit {
                    text: "!".into(),
                    replacement: None,
                });
            })
            .unwrap();
            assert!(h.focused_text().unwrap().paragraph.is_some());
            assert_eq!(h.focused_text().unwrap().input.text, "abc!");
            assert_eq!(
                h.focused_text()
                    .unwrap()
                    .paragraph
                    .as_ref()
                    .unwrap()
                    .utf16_len,
                4
            );
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
        let panel = &mut host.panels[0];
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
        host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
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
        let mut field = TextFieldContent::new("Area", "");
        field.show_metadata = false;
        field.viewport = TextViewport::Fixed(90.);
        field.surface =
            tekes_ui_controls::TextSurface::new(&"editable line\n".repeat(80), true, false);
        let local = LocalContainer::new("area", Content::new(ComponentId(1), field)).unwrap();
        let mut host = AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        assert!(host.mount_columns());
        let frame = Rect::new(0., 40., 300., 160.);
        let panel = &mut host.panels[0];
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
            host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
                y = h.text_surface(ComponentId(1)).unwrap().scroll.offset.y;
            });
            y
        };
        send(&mut host, native(ScrollPhase::Began, ScrollPhase::None, 0.));
        assert_eq!(offset(&mut host), 0.);
        let mut event = native(ScrollPhase::Changed, ScrollPhase::None, -20.);
        event.position.y = 145.; // Outside the 90pt editor, inside the same 160pt container.
        host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
            assert!(
                !h.text_surface(ComponentId(1))
                    .unwrap()
                    .outer
                    .contains(Point::new(100., 145.))
            );
        });
        assert!(
            host.panels[0]
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
                assert!(s.scroll.needs_animation(), "Began establishes the gesture")
            });
            send(&mut host, native(terminal, ScrollPhase::None, 0.));
            scroll(&mut host, |s| {
                assert!(
                    !s.scroll.needs_animation(),
                    "zero-delta terminal ends the gesture"
                )
            });
            let mut wheel = native(ScrollPhase::None, ScrollPhase::None, -2.);
            wheel.precise = false;
            send(&mut host, wheel);
            let line = tekes_ui_tokens::proposal::metric("size", "size-row-height").unwrap();
            scroll(&mut host, |s| {
                assert_eq!(s.scroll.offset.y, 20. + 2. * line)
            });
        }
    }
    #[test]
    fn body_scroll_reveal_and_reading_fences_reject_old_momentum_but_allow_new_fingers() {
        for reading in [false, true] {
            let (mut host, _) = host();
            if !reading {
                scroll(&mut host, |s| {
                    assert!(
                        s.scroll
                            .reveal_with(Rect::new(0., 600., 10., 20.), Default::default())
                            .unwrap()
                            .animated
                    );
                });
            }
            if reading {
                host.panels[0].local.as_mut().unwrap().with_host(&mut |h| {
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
            scroll(&mut host, |s| before = s.scroll.offset);
            send(
                &mut host,
                native(ScrollPhase::None, ScrollPhase::Changed, 15.),
            );
            scroll(&mut host, |s| {
                assert_eq!(
                    s.scroll.offset, before,
                    "old OS momentum cannot take over navigation"
                );
                if reading {
                    assert!(s.is_following_latest());
                } else {
                    assert!(s.scroll.needs_animation());
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
                assert_eq!(
                    s.scroll.offset.y,
                    before.y + if reading { -10. } else { 10. }
                );
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
                s.scroll.set_reduced_motion(reduced);
            });
            send(
                &mut host,
                native(ScrollPhase::Began, ScrollPhase::None, 60.),
            );
            send(&mut host, native(ScrollPhase::Ended, ScrollPhase::None, 0.));
            scroll(&mut host, |s| {
                assert_eq!(s.scroll.offset.y, 0.);
                if reduced {
                    assert_eq!(s.scroll.overscroll.y, 0.);
                } else {
                    assert!(s.scroll.overscroll.y < 0.);
                    assert!(s.scroll.needs_animation());
                    for _ in 0..120 {
                        s.scroll.tick(1. / 60.);
                    }
                    assert!(s.scroll.overscroll.y.abs() < 0.01);
                }
            });
        }
    }
}
