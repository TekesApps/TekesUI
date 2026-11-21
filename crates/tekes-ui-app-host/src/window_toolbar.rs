//! Container-owned shared Toolbar instances and original command bindings.
use crate::toolbar_source::ComponentSource;
use anyhow::{Context, Result};
use tekes_ui_controls::composition::*;
use tekes_ui_scene::Rect;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Visibility(usize),
    Navigation(ComponentSource),
    Undo(ComponentSource),
    Title,
    WorkspaceSwitch {
        focus: u64,
        other: Option<u64>,
    },
    Expand {
        container: String,
        expanded: bool,
    },
    Panel {
        instance: u64,
        index: usize,
        module: String,
        pid: Option<u32>,
        generation: u64,
    },
    PanelOverflow {
        instance: u64,
        index: usize,
        module: String,
        pid: Option<u32>,
        generation: u64,
    },
}
impl Source {
    pub(crate) fn panel_index(&self) -> Option<usize> {
        match self {
            Self::Panel { index, .. } | Self::PanelOverflow { index, .. } => Some(*index),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MenuDeclaration {
    pub id: u64,
    pub title: String,
    pub accessible_name: Option<String>,
    pub icon: Option<String>,
    pub icon_position: ButtonImagePosition,
    pub icon_slot: bool,
    pub tooltip: Option<String>,
    pub enabled: bool,
    pub selected: bool,
    pub entries: Vec<MenuEntry>,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Contents {
    Commands(Vec<ToolbarButtonDeclaration>, ToolbarGroupOverflow),
    TemplateCommands(
        Vec<ToolbarButtonDeclaration>,
        ToolbarGroupOverflow,
        Vec<(u64, String)>,
    ),
    SelectedTemplateCommands {
        buttons: Vec<ToolbarButtonDeclaration>,
        overflow: ToolbarGroupOverflow,
        templates: Vec<(u64, String)>,
        selected: Vec<u64>,
    },
    Menu(MenuDeclaration),
    Title(String),
}
impl Contents {
    pub(super) fn templates(&self) -> &[(u64, String)] {
        match self {
            Self::TemplateCommands(_, _, icons) => icons,
            Self::SelectedTemplateCommands { templates, .. } => templates,
            _ => &[],
        }
    }
}
fn validate_menu_entries(entries: &[MenuEntry]) -> Result<()> {
    let mut values = std::collections::BTreeSet::new();
    for entry in entries {
        match entry {
            MenuEntry::Item { value, label, .. } => {
                anyhow::ensure!(
                    !value.trim().is_empty(),
                    "menu entry value must not be empty"
                );
                anyhow::ensure!(
                    values.insert(value.as_str()),
                    "menu sibling values must be unique"
                );
                anyhow::ensure!(
                    !label.trim().is_empty(),
                    "menu entry label must not be empty"
                );
            }
            MenuEntry::Submenu {
                value,
                label,
                entries,
                ..
            } => {
                anyhow::ensure!(
                    !value.trim().is_empty(),
                    "menu entry value must not be empty"
                );
                anyhow::ensure!(
                    values.insert(value.as_str()),
                    "menu sibling values must be unique"
                );
                anyhow::ensure!(
                    !label.trim().is_empty(),
                    "menu entry label must not be empty"
                );
                validate_menu_entries(entries)?;
            }
            MenuEntry::Separator => {}
        }
    }
    Ok(())
}
pub(crate) fn apply_menu_declaration(
    menu: &mut MenuContent,
    decl: &MenuDeclaration,
    viewport: Rect,
) {
    menu.set_popup_viewport(Some(viewport));
    menu.set_trigger_variant(tekes_ui_controls::ButtonVariant::Toolbar);
    menu.set_trigger_selection_fill_only(true);
    menu.set_trigger_toolbar_icon_slot(decl.icon_slot);
    menu.set_trigger_image_position(decl.icon_position);
    if let Some(icon) = &decl.icon {
        menu.set_trigger_icon_label(icon, decl.title.clone())
            .expect("menu icon validated before sync");
    } else {
        menu.set_trigger_contents(decl.title.clone());
    }
    menu.set_trigger_accessible_name(decl.accessible_name.clone());
    menu.set_trigger_help(decl.tooltip.clone());
    menu.set_disabled(!decl.enabled);
    menu.set_trigger_selected(decl.selected);
    menu.set_entries(decl.entries.clone())
        .expect("menu entries validated before sync");
}
pub(crate) fn build_menu_content(decl: &MenuDeclaration, viewport: Rect) -> MenuContent {
    let mut menu = MenuContent::new(decl.title.clone(), Vec::new());
    apply_menu_declaration(&mut menu, decl, viewport);
    menu
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Declaration {
    pub id: ComponentId,
    pub region: ComponentId,
    pub placement: ToolbarPlacement,
    pub order: i32,
    pub contents: Contents,
    pub source: Source,
}
pub(crate) struct MountedToolbar {
    pub(crate) icons: SvgIconCache,
    pub host: ToolbarHost<Source>,
    pub window: u64,
    viewport: Rect,
    declarations: Vec<Declaration>,
}
impl MountedToolbar {
    pub fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let window = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            icons: SvgIconCache::default(),
            host: ToolbarHost::new(ComponentId(90000), window, Rect::default())
                .expect("toolbar root"),
            window,
            viewport: Rect::default(),
            declarations: vec![],
        }
    }
    pub(crate) fn occupied_frames(&self) -> Vec<Rect> {
        self.declarations
            .iter()
            .filter_map(|declaration| self.host.content_host().get(declaration.id))
            .map(|region| region.frame())
            .filter(|frame| frame.size.width > 0. && frame.size.height > 0.)
            .collect()
    }
    pub(crate) fn contributions(&self) -> impl Iterator<Item = (ComponentId, &Source)> {
        self.declarations
            .iter()
            .map(|declaration| (declaration.id, &declaration.source))
    }
    #[cfg(test)]
    pub(crate) fn sources(&self) -> impl Iterator<Item = &Source> {
        self.contributions().map(|(_, source)| source)
    }
    pub fn source(&self, group: ComponentId) -> Option<&Source> {
        self.declarations
            .iter()
            .find(|d| d.id == group)
            .map(|d| &d.source)
    }
    pub fn sync(
        &mut self,
        viewport: Rect,
        regions: Vec<ToolbarRegion>,
        declarations: Vec<Declaration>,
    ) -> Result<()> {
        for declaration in &declarations {
            for (_, name) in declaration.contents.templates() {
                tekes_ui_icons::Glyph::named(name).context("unknown toolkit template icon")?;
            }
            if let Contents::Menu(menu) = &declaration.contents {
                if let Some(name) = &menu.icon {
                    tekes_ui_icons::Glyph::named(name).context("unknown toolkit menu icon")?;
                }
                validate_menu_entries(&menu.entries)?;
            }
        }
        let viewport_changed = self.viewport != viewport;
        self.host
            .set_regions(self.window, viewport, regions)
            .map_err(|e| anyhow::anyhow!("toolbar regions: {e:?}"))?;
        for old in &self.declarations {
            if !declarations.iter().any(|d| d.id == old.id)
                && self.host.content_host().get(old.id).is_some()
            {
                self.host
                    .remove(old.id)
                    .map_err(|e| anyhow::anyhow!("toolbar remove: {e:?}"))?;
            }
        }
        for d in &declarations {
            if self.declarations.iter().any(|old| old == d) {
                if viewport_changed && matches!(d.contents, Contents::Menu(_)) {
                    let target = ComponentId(d.id.0 + 1);
                    self.host
                        .update_component::<MenuContent>(d.id, target, |menu| {
                            menu.set_popup_viewport(Some(viewport));
                        })
                        .map_err(|e| anyhow::anyhow!("toolbar viewport: {e:?}"))?;
                }
                continue;
            }
            if let Some(old) = self.declarations.iter().find(|old| old.id == d.id)
                && self.host.content_host().get(d.id).is_some()
                && old.source == d.source
            {
                let target = ComponentId(d.id.0 + 1);
                let compatible = match (&old.contents, &d.contents) {
                    (
                        Contents::Commands(a, _)
                        | Contents::TemplateCommands(a, _, _)
                        | Contents::SelectedTemplateCommands { buttons: a, .. },
                        Contents::Commands(b, _)
                        | Contents::TemplateCommands(b, _, _)
                        | Contents::SelectedTemplateCommands { buttons: b, .. },
                    ) => a.iter().map(|b| b.id).eq(b.iter().map(|b| b.id)),
                    (Contents::Menu(a), Contents::Menu(b)) => a.id == b.id,
                    (Contents::Title(_), Contents::Title(_)) => true,
                    _ => false,
                };
                if compatible {
                    if old.region != d.region
                        || old.placement != d.placement
                        || old.order != d.order
                    {
                        self.host
                            .move_group(d.id, d.region, d.placement, d.order)
                            .map_err(|e| anyhow::anyhow!("toolbar move: {e:?}"))?;
                    }
                    if old.contents != d.contents {
                        let result = match &d.contents {
                            Contents::Commands(buttons, overflow)
                            | Contents::TemplateCommands(buttons, overflow, _)
                            | Contents::SelectedTemplateCommands {
                                buttons, overflow, ..
                            } => {
                                let selected = match &d.contents {
                                    Contents::SelectedTemplateCommands { selected, .. } => {
                                        selected.as_slice()
                                    }
                                    _ => &[],
                                };
                                self.host
                                    .update_component::<tekes_ui_controls::ToolbarControl>(
                                        d.id,
                                        target,
                                        |control| {
                                            control.set_group_together(
                                                *overflow == ToolbarGroupOverflow::Together,
                                            );
                                            control.set_menu_only(
                                                *overflow == ToolbarGroupOverflow::Menu,
                                            );
                                            control.set_actions(
                                                buttons
                                                    .iter()
                                                    .map(|b| tekes_ui_controls::ToolbarAction {
                                                        id: b.id,
                                                        title: b.text.clone(),
                                                        enabled: b.enabled,
                                                    })
                                                    .collect(),
                                            );
                                            control.set_selected_ids(selected);
                                            for b in buttons {
                                                control.set_action_icon(b.id, b.icon);
                                                control.set_action_help(b.id, b.hint.clone());
                                            }
                                            for (id, name) in d.contents.templates() {
                                                control
                                                    .set_action_template_icon(*id, Some(name))
                                                    .expect("template names validated before sync");
                                            }
                                        },
                                    )
                            }
                            Contents::Menu(decl) => {
                                self.host
                                    .update_component::<MenuContent>(d.id, target, |menu| {
                                        apply_menu_declaration(menu, decl, viewport)
                                    })
                            }
                            Contents::Title(title) => {
                                self.host
                                    .update_component::<TextContent>(d.id, target, |label| {
                                        label.set_text(title.clone())
                                    })
                            }
                        };
                        result.map_err(|e| anyhow::anyhow!("toolbar values: {e:?}"))?;
                    } else if viewport_changed && matches!(d.contents, Contents::Menu(_)) {
                        self.host
                            .update_component::<MenuContent>(d.id, target, |menu| {
                                menu.set_popup_viewport(Some(viewport));
                            })
                            .map_err(|e| anyhow::anyhow!("toolbar viewport: {e:?}"))?;
                    }
                    continue;
                }
            }
            let target = ComponentId(d.id.0 + 1);
            let (mut child, bindings) = match &d.contents {
                Contents::Commands(buttons, overflow)
                | Contents::TemplateCommands(buttons, overflow, _)
                | Contents::SelectedTemplateCommands {
                    buttons, overflow, ..
                } => {
                    let mut content = toolbar_commands(target, buttons.clone(), *overflow);
                    if let Contents::SelectedTemplateCommands { selected, .. } = &d.contents
                        && let Some(control) =
                            content.component_mut::<tekes_ui_controls::ToolbarControl>()
                    {
                        control.set_selected_ids(selected);
                    }
                    (
                        content,
                        buttons
                            .iter()
                            .map(|b| ToolbarCommandBinding {
                                target,
                                action: Some(b.id),
                                command: b.id,
                                source: d.source.clone(),
                            })
                            .collect(),
                    )
                }
                Contents::Menu(decl) => (
                    Content::new(target, build_menu_content(decl, viewport)),
                    vec![ToolbarCommandBinding {
                        target,
                        action: None,
                        command: decl.id,
                        source: d.source.clone(),
                    }],
                ),
                Contents::Title(title) => (
                    Content::new(target, TextContent::label(title.clone())),
                    vec![],
                ),
            };
            if let Some(control) = child.component_mut::<tekes_ui_controls::ToolbarControl>() {
                for (id, name) in d.contents.templates() {
                    control
                        .set_action_template_icon(*id, Some(name))
                        .expect("template names validated before sync");
                }
            }
            self.host
                .register(ToolbarRegistration {
                    region: d.region,
                    order: d.order,
                    group: Content::new(d.id, ToolbarGroup::new(d.placement, vec![child])),
                    bindings,
                })
                .map_err(|e| anyhow::anyhow!("toolbar registration: {e:?}"))?;
        }
        self.viewport = viewport;
        self.declarations = declarations;
        Ok(())
    }
    pub(crate) fn retain_groups(&mut self, keep: &[ComponentId]) -> Result<()> {
        let remove: Vec<_> = self
            .declarations
            .iter()
            .filter(|declaration| !keep.contains(&declaration.id))
            .map(|declaration| declaration.id)
            .collect();
        for id in remove {
            if self.host.content_host().get(id).is_some() {
                self.host
                    .remove(id)
                    .map_err(|e| anyhow::anyhow!("container toolbar unbind: {e:?}"))?;
            }
        }
        self.declarations
            .retain(|declaration| keep.contains(&declaration.id));
        Ok(())
    }
    pub fn modal(&self) -> bool {
        // ContentHost projects only its original input scope. A scoped root
        // differs from our window root, and MenuContent opens via input_overlay_owner.
        self.host
            .semantics()
            .first()
            .is_some_and(|n| n.node.id != ComponentId(90000))
            || self.host.content_host().input_overlay_owner().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Text;
    use tekes_ui_controls::ControlKey;
    use tekes_ui_scene::{Point, Scene};
    fn setup() -> (MountedToolbar, Text, Rect, Vec<Declaration>) {
        let toolbar = MountedToolbar::new();
        let text = Text::new();
        let rect = Rect::new(0., 0., 600., 180.);
        let declarations = vec![
            Declaration {
                id: ComponentId(100),
                region: ComponentId(10),
                placement: ToolbarPlacement::Navigation,
                order: 0,
                contents: Contents::Commands(
                    vec![ToolbarButtonDeclaration {
                        id: 1,
                        icon: None,
                        text: "Back".into(),
                        hint: Some("Go back".into()),
                        enabled: true,
                    }],
                    ToolbarGroupOverflow::Together,
                ),
                source: Source::Visibility(0),
            },
            Declaration {
                id: ComponentId(200),
                region: ComponentId(20),
                placement: ToolbarPlacement::SecondaryAction,
                order: 0,
                contents: Contents::Commands(
                    vec![
                        ToolbarButtonDeclaration {
                            id: 2,
                            icon: None,
                            text: "Undo".into(),
                            hint: Some("Undo saved change".into()),
                            enabled: true,
                        },
                        ToolbarButtonDeclaration {
                            id: 3,
                            icon: None,
                            text: "Redo".into(),
                            hint: Some("Redo saved change".into()),
                            enabled: true,
                        },
                    ],
                    ToolbarGroupOverflow::Together,
                ),
                source: Source::Panel {
                    instance: 1,
                    index: 2,
                    module: "original-owner".into(),
                    pid: Some(99),
                    generation: 7,
                },
            },
        ];
        (toolbar, text, rect, declarations)
    }
    fn sync(toolbar: &mut MountedToolbar, text: &mut Text, rect: Rect, ds: Vec<Declaration>) {
        toolbar
            .sync(
                rect,
                vec![
                    ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., 300., 40.)),
                    ToolbarRegion::new(
                        ComponentId(20),
                        Rect::new(300., 0., rect.size.width - 300., 40.),
                    ),
                ],
                ds,
            )
            .unwrap();
        toolbar
            .host
            .render(
                &mut Scene::new(),
                text,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
    }
    fn button(toolbar: &MountedToolbar, name: &str) -> (ToolbarAccessibilityTarget, Rect) {
        toolbar
            .host
            .semantics()
            .iter()
            .flat_map(|n| n.parts.iter())
            .find(|(_, p)| p.semantics.name == name)
            .map(|(t, p)| (*t, p.frame))
            .expect(name)
    }
    #[test]
    fn independent_edit_group_pointer_keyboard_ax_deliver_once_to_original_source() {
        let (mut toolbar, mut text, rect, ds) = setup();
        sync(&mut toolbar, &mut text, rect, ds.clone());
        let (target, frame) = button(&toolbar, "Undo");
        assert!(
            toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        let commands = toolbar.host.drain_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].source, ds[1].source);
        assert_eq!(commands[0].command, 2);
        assert!(toolbar.host.drain_commands().is_empty());
        toolbar
            .host
            .event(&ContentEvent::PointerDown(frame.center()));
        toolbar.host.event(&ContentEvent::PointerUp(frame.center()));
        assert_eq!(toolbar.host.drain_commands().len(), 1);
        toolbar.host.event(&ContentEvent::Cancel);
        toolbar.host.event(&ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(ControlKey::Tab { reverse: false }),
            down: true,
            repeat: false,
            modifiers: InputModifiers::default(),
            native_code: 0,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        }));
        assert!(toolbar.host.content_host().focused().is_some());
        // The same public input entry traverses into the independent group.
        for _ in 0..3 {
            toolbar.host.event(&ContentEvent::KeyInput(ContentKeyEvent {
                key: Some(ControlKey::Tab { reverse: false }),
                down: true,
                repeat: false,
                modifiers: InputModifiers::default(),
                native_code: 0,
                characters: String::new(),
                characters_ignoring_modifiers: String::new(),
            }));
        }
        toolbar.host.event(&ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(ControlKey::Right),
            down: true,
            repeat: false,
            modifiers: InputModifiers::default(),
            native_code: 0,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        }));
        assert_eq!(
            toolbar.host.content_host().focused(),
            Some(ComponentId(201))
        );
    }
    #[test]
    fn unchanged_declarations_keep_mount_but_update_move_remove_remount_reject_old_targets() {
        let (mut toolbar, mut text, rect, mut ds) = setup();
        sync(&mut toolbar, &mut text, rect, ds.clone());
        let (old, _) = button(&toolbar, "Undo");
        sync(&mut toolbar, &mut text, rect, ds.clone());
        assert_eq!(button(&toolbar, "Undo").0, old);
        ds[1].region = ComponentId(10);
        sync(&mut toolbar, &mut text, rect, ds.clone());
        assert!(
            !toolbar
                .host
                .accessibility_action(old, AccessibilityAction::Press)
                .consumed
        );
        let (moved, _) = button(&toolbar, "Undo");
        ds.remove(1);
        sync(&mut toolbar, &mut text, rect, ds.clone());
        assert!(
            !toolbar
                .host
                .accessibility_action(moved, AccessibilityAction::Press)
                .consumed
        );
        let (_, _, _, all) = setup();
        sync(&mut toolbar, &mut text, rect, all);
        assert!(
            !toolbar
                .host
                .accessibility_action(moved, AccessibilityAction::Press)
                .consumed
        );
        let (fresh, _) = button(&toolbar, "Undo");
        toolbar.host.set_window_active(false);
        assert!(
            !toolbar
                .host
                .accessibility_action(fresh, AccessibilityAction::Press)
                .consumed
        );
        assert!(toolbar.host.semantics().is_empty());
    }
    #[test]
    fn narrow_more_owns_background_and_resize_cancels_old_pointer_and_ax() {
        let (mut toolbar, mut text, rect, ds) = setup();
        sync(&mut toolbar, &mut text, rect, ds.clone());
        let (old, frame) = button(&toolbar, "Undo");
        toolbar
            .host
            .event(&ContentEvent::PointerDown(frame.center()));
        sync(&mut toolbar, &mut text, Rect::new(0., 0., 365., 180.), ds);
        toolbar.host.event(&ContentEvent::PointerUp(frame.center()));
        assert!(toolbar.host.drain_commands().is_empty());
        assert!(
            !toolbar
                .host
                .accessibility_action(old, AccessibilityAction::Press)
                .consumed
        );
        let more = toolbar
            .host
            .semantics()
            .iter()
            .flat_map(|n| n.parts.iter())
            .find(|(_, p)| p.semantics.role == ContentRole::MenuButton)
            .map(|(t, _)| *t)
            .expect("More");
        assert!(
            toolbar
                .host
                .accessibility_action(more, AccessibilityAction::Press)
                .consumed
        );
        assert!(toolbar.modal());
        toolbar
            .host
            .render(
                &mut Scene::new(),
                &mut text,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
        toolbar
            .host
            .event(&ContentEvent::PointerDown(Point::new(20., 150.)));
        assert!(!toolbar.host.event_bubbled());
        assert!(!toolbar.modal());
        assert!(toolbar.host.drain_commands().is_empty());
    }

    fn menu_declaration(
        id: u64,
        title: &str,
        tooltip: &str,
        enabled: bool,
        selected: bool,
        entries: Vec<MenuEntry>,
    ) -> MenuDeclaration {
        MenuDeclaration {
            id,
            title: title.into(),
            accessible_name: None,
            icon: None,
            icon_position: ButtonImagePosition::Leading,
            icon_slot: false,
            tooltip: Some(tooltip.into()),
            enabled,
            selected,
            entries,
        }
    }

    fn menu_declarations(
        model_title: &str,
        effort_title: &str,
        effort_enabled: bool,
        effort_selected: bool,
        model_entries: Vec<MenuEntry>,
    ) -> Vec<Declaration> {
        vec![
            Declaration {
                id: ComponentId(100),
                region: ComponentId(10),
                placement: ToolbarPlacement::Navigation,
                order: 0,
                contents: Contents::Menu(menu_declaration(
                    11,
                    model_title,
                    "Model",
                    true,
                    false,
                    model_entries,
                )),
                source: Source::Panel {
                    instance: 7,
                    index: 0,
                    module: "session".into(),
                    pid: None,
                    generation: 0,
                },
            },
            Declaration {
                id: ComponentId(200),
                region: ComponentId(20),
                placement: ToolbarPlacement::PrimaryAction,
                order: 1,
                contents: Contents::Menu(menu_declaration(
                    12,
                    effort_title,
                    "Effort",
                    effort_enabled,
                    effort_selected,
                    vec![
                        MenuEntry::Item {
                            value: "low".into(),
                            label: "Low".into(),
                            checked: Some(false),
                            disabled: false,
                        },
                        MenuEntry::Item {
                            value: "high".into(),
                            label: "High".into(),
                            checked: Some(true),
                            disabled: false,
                        },
                    ],
                )),
                source: Source::Panel {
                    instance: 7,
                    index: 0,
                    module: "session".into(),
                    pid: None,
                    generation: 0,
                },
            },
        ]
    }

    fn render_theme(toolbar: &mut MountedToolbar, text: &mut Text, theme: tekes_ui_tokens::Theme) {
        toolbar
            .host
            .render(&mut Scene::new(), text, theme.into())
            .unwrap();
    }

    fn menu_trigger(
        toolbar: &MountedToolbar,
        target_id: ComponentId,
    ) -> (ToolbarAccessibilityTarget, Rect) {
        toolbar
            .host
            .semantics()
            .into_iter()
            .find(|n| n.node.id == target_id)
            .map(|n| (n.target, n.node.frame))
            .expect("menu trigger semantic node")
    }

    fn menu_item(
        toolbar: &MountedToolbar,
        menu_id: ComponentId,
        label: &str,
    ) -> (
        ToolbarAccessibilityTarget,
        tekes_ui_controls::composition::SemanticPart,
    ) {
        toolbar
            .host
            .semantics()
            .into_iter()
            .find(|n| n.node.id == menu_id)
            .and_then(|n| n.parts.into_iter().find(|(_, p)| p.semantics.name == label))
            .expect(label)
    }

    #[test]
    fn menu_in_place_updates_preserve_identity_and_unaffected_open_menu_across_themes() {
        let mut toolbar = MountedToolbar::new();
        let mut text = Text::new();
        let rect = Rect::new(0., 0., 640., 260.);
        let initial_models = vec![
            MenuEntry::Item {
                value: "gpt-5".into(),
                label: "GPT-5".into(),
                checked: Some(true),
                disabled: false,
            },
            MenuEntry::Item {
                value: "sonnet".into(),
                label: "Claude Sonnet".into(),
                checked: Some(false),
                disabled: false,
            },
        ];
        sync(
            &mut toolbar,
            &mut text,
            rect,
            menu_declarations(
                "Model: GPT-5",
                "Effort: High",
                true,
                true,
                initial_models.clone(),
            ),
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::DARK);

        let mount_1 = toolbar
            .host
            .content_host()
            .get(ComponentId(101))
            .unwrap()
            .mount_identity();
        let mount_2 = toolbar
            .host
            .content_host()
            .get(ComponentId(201))
            .unwrap()
            .mount_identity();

        // Open menu 1 via pointer click.
        let (_, trigger_frame) = menu_trigger(&toolbar, ComponentId(101));
        toolbar
            .host
            .event(&ContentEvent::PointerDown(trigger_frame.center()));
        toolbar
            .host
            .event(&ContentEvent::PointerUp(trigger_frame.center()));
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        assert!(toolbar.modal());

        // Updating sibling menu 2 refreshes in place without rebuilding or closing menu 1.
        sync(
            &mut toolbar,
            &mut text,
            rect,
            menu_declarations(
                "Model: GPT-5",
                "Effort: Low",
                false,
                false,
                initial_models.clone(),
            ),
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::DARK);
        assert_eq!(
            toolbar
                .host
                .content_host()
                .get(ComponentId(101))
                .unwrap()
                .mount_identity(),
            mount_1
        );
        assert_eq!(
            toolbar
                .host
                .content_host()
                .get(ComponentId(201))
                .unwrap()
                .mount_identity(),
            mount_2
        );
        assert!(toolbar.modal());
        assert_eq!(
            toolbar
                .host
                .content_host()
                .get(ComponentId(201))
                .unwrap()
                .component::<MenuContent>()
                .unwrap()
                .trigger()
                .label(),
            "Effort: Low"
        );

        // Updating menu 1's own declaration preserves mount identity and safely
        // cancels/closes the affected popup per ToolbarHost/MenuContent contract.
        sync(
            &mut toolbar,
            &mut text,
            rect,
            menu_declarations(
                "Model: Default",
                "Effort: Low",
                false,
                false,
                initial_models,
            ),
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        assert_eq!(
            toolbar
                .host
                .content_host()
                .get(ComponentId(101))
                .unwrap()
                .mount_identity(),
            mount_1
        );
        assert!(!toolbar.modal());
        assert_eq!(
            toolbar
                .host
                .content_host()
                .get(ComponentId(101))
                .unwrap()
                .component::<MenuContent>()
                .unwrap()
                .trigger()
                .label(),
            "Model: Default"
        );

        // Re-open menu 1, then change its entries -> safely closes popup while keeping mount identity.
        let (_, reopen_frame) = menu_trigger(&toolbar, ComponentId(101));
        toolbar
            .host
            .event(&ContentEvent::PointerDown(reopen_frame.center()));
        toolbar
            .host
            .event(&ContentEvent::PointerUp(reopen_frame.center()));
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        assert!(toolbar.modal());
        let updated_models = vec![MenuEntry::Item {
            value: "o3".into(),
            label: "o3".into(),
            checked: Some(true),
            disabled: false,
        }];
        sync(
            &mut toolbar,
            &mut text,
            rect,
            menu_declarations("Model: o3", "Effort: Low", false, false, updated_models),
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::DARK);
        assert_eq!(
            toolbar
                .host
                .content_host()
                .get(ComponentId(101))
                .unwrap()
                .mount_identity(),
            mount_1
        );
        assert!(!toolbar.modal());
    }

    #[test]
    fn menu_modal_dismissal_deactivation_and_locked_disabled_items_contract() {
        let mut toolbar = MountedToolbar::new();
        let mut text = Text::new();
        let rect = Rect::new(0., 0., 640., 300.);
        let entries = vec![
            MenuEntry::Item {
                value: "codex".into(),
                label: "Codex CLI".into(),
                checked: Some(true),
                disabled: true,
            },
            MenuEntry::Separator,
            MenuEntry::Item {
                value: "claude".into(),
                label: "Claude Code".into(),
                checked: Some(false),
                disabled: false,
            },
            MenuEntry::Submenu {
                value: "more".into(),
                label: "Custom".into(),
                disabled: false,
                entries: vec![MenuEntry::Item {
                    value: "local".into(),
                    label: "Local Runner".into(),
                    checked: None,
                    disabled: false,
                }],
            },
        ];
        sync(
            &mut toolbar,
            &mut text,
            rect,
            menu_declarations("Harness: Codex", "Effort: Medium", true, false, entries),
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);

        let (trigger_ax, trigger_frame) = menu_trigger(&toolbar, ComponentId(101));
        assert!(
            toolbar
                .host
                .accessibility_action(trigger_ax, AccessibilityAction::Press)
                .consumed
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        assert!(toolbar.modal());

        // Disabled item carries disabled/checked semantics and rejects pointer & AX clicks.
        let (disabled_ax, disabled_part) = menu_item(&toolbar, ComponentId(101), "Codex CLI");
        assert!(disabled_part.semantics.disabled);
        assert!(disabled_part.semantics.selected);
        toolbar
            .host
            .event(&ContentEvent::PointerDown(disabled_part.frame.center()));
        toolbar
            .host
            .event(&ContentEvent::PointerUp(disabled_part.frame.center()));
        assert!(toolbar.host.drain_commands().is_empty());
        assert!(
            !toolbar
                .host
                .accessibility_action(disabled_ax, AccessibilityAction::Press)
                .consumed
        );
        assert!(toolbar.host.drain_commands().is_empty());

        // Clicking outside closes the popup without bubbling or invoking commands.
        toolbar
            .host
            .event(&ContentEvent::PointerDown(Point::new(15., 260.)));
        assert!(!toolbar.host.event_bubbled());
        assert!(!toolbar.modal());
        assert!(toolbar.host.drain_commands().is_empty());

        // Re-open and deactivate window -> popup safely closes.
        toolbar
            .host
            .event(&ContentEvent::PointerDown(trigger_frame.center()));
        toolbar
            .host
            .event(&ContentEvent::PointerUp(trigger_frame.center()));
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::DARK);
        assert!(toolbar.modal());
        toolbar.host.set_window_active(false);
        assert!(!toolbar.modal());

        // Re-activate, open submenu, and select leaf item -> MenuSelection value path is delivered.
        toolbar.host.set_window_active(true);
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        let (trigger_ax, _) = menu_trigger(&toolbar, ComponentId(101));
        assert!(
            toolbar
                .host
                .accessibility_action(trigger_ax, AccessibilityAction::Press)
                .consumed
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        let (sub_ax, _) = menu_item(&toolbar, ComponentId(101), "Custom");
        assert!(
            toolbar
                .host
                .accessibility_action(sub_ax, AccessibilityAction::Press)
                .consumed
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        let (leaf_ax, _) = menu_item(&toolbar, ComponentId(101), "Local Runner");
        assert!(
            toolbar
                .host
                .accessibility_action(leaf_ax, AccessibilityAction::Press)
                .consumed
        );
        render_theme(&mut toolbar, &mut text, tekes_ui_tokens::Theme::LIGHT);
        assert!(!toolbar.modal());

        let invocations = toolbar.host.drain_commands();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].command, 11);
        match invocations[0].payload.as_ref() {
            Some(ContentActionPayload::MenuSelection(selection)) => {
                assert_eq!(
                    selection.value_path(),
                    Some(["more".into(), "local".into()].as_slice())
                );
            }
            other => panic!("expected MenuSelection payload, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod region_lifecycle_tests {
    use super::*;
    #[test]
    fn removing_a_region_already_unmounts_its_group_and_cache_reconciles_once() {
        let mut toolbar = MountedToolbar::new();
        let viewport = Rect::new(0., 0., 500., 200.);
        let region = ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., 300., 40.));
        let d = Declaration {
            id: ComponentId(100),
            region: region.id,
            placement: ToolbarPlacement::Navigation,
            order: 0,
            contents: Contents::Title("Back region".into()),
            source: Source::Title,
        };
        toolbar
            .sync(viewport, vec![region], vec![d.clone()])
            .unwrap();
        let title_node = toolbar
            .host
            .content_host()
            .get(ComponentId(d.id.0 + 1))
            .unwrap();
        assert_eq!(
            title_node.component::<TextContent>().unwrap().role(),
            tekes_ui_tokens::proposal::TextRole::UiLabel
        );
        toolbar.sync(viewport, vec![], vec![]).unwrap();
        assert!(toolbar.host.content_host().get(d.id).is_none());
    }
}
