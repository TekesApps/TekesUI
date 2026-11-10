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
    Close(String),
    Panel {
        instance: u64,
        index: usize,
        module: String,
        pid: Option<u32>,
        generation: u64,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Contents {
    Commands(Vec<ToolbarButtonDeclaration>, ToolbarGroupOverflow),
    TemplateCommands(
        Vec<ToolbarButtonDeclaration>,
        ToolbarGroupOverflow,
        Vec<(u64, String)>,
    ),
    Title(String),
    Close(String),
}
impl Contents {
    pub(super) fn templates(&self) -> &[(u64, String)] {
        match self {
            Self::TemplateCommands(_, _, icons) => icons,
            _ => &[],
        }
    }
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
        }
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
                continue;
            }
            if let Some(old) = self.declarations.iter().find(|old| old.id == d.id)
                && self.host.content_host().get(d.id).is_some()
                && old.source == d.source
            {
                let target = ComponentId(d.id.0 + 1);
                let compatible = match (&old.contents, &d.contents) {
                    (
                        Contents::Commands(a, _) | Contents::TemplateCommands(a, _, _),
                        Contents::Commands(b, _) | Contents::TemplateCommands(b, _, _),
                    ) => a.iter().map(|b| b.id).eq(b.iter().map(|b| b.id)),
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
                            | Contents::TemplateCommands(buttons, overflow, _) => {
                                self.host
                                    .update_component::<tekes_ui_controls::ToolbarControl>(
                                        d.id,
                                        target,
                                        |control| {
                                            control.set_group_together(
                                                *overflow == ToolbarGroupOverflow::Together,
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
                            Contents::Close(_) => {
                                unreachable!("close changes replace its original declaration")
                            }
                            Contents::Title(title) => {
                                self.host
                                    .update_component::<TextContent>(d.id, target, |label| {
                                        label.text = title.clone()
                                    })
                            }
                        };
                        result.map_err(|e| anyhow::anyhow!("toolbar values: {e:?}"))?;
                    }
                    continue;
                }
            }
            let target = ComponentId(d.id.0 + 1);
            let (mut child, bindings) = match &d.contents {
                Contents::Commands(buttons, overflow)
                | Contents::TemplateCommands(buttons, overflow, _) => (
                    toolbar_commands(target, buttons.clone(), *overflow),
                    buttons
                        .iter()
                        .map(|b| ToolbarCommandBinding {
                            target,
                            action: Some(b.id),
                            command: b.id,
                            source: d.source.clone(),
                        })
                        .collect(),
                ),
                Contents::Title(title) => (
                    Content::new(target, TextContent::new(title.clone())),
                    vec![],
                ),
                Contents::Close(title) => {
                    let button = close_button(target, title);
                    (
                        button,
                        vec![ToolbarCommandBinding {
                            target,
                            action: None,
                            command: 0,
                            source: d.source.clone(),
                        }],
                    )
                }
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
        // differs from our window root; no expanded disclosure or group polling.
        self.host
            .semantics()
            .first()
            .is_some_and(|n| n.node.id != ComponentId(90000))
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
        toolbar.sync(viewport, vec![], vec![]).unwrap();
        assert!(toolbar.host.content_host().get(d.id).is_none());
    }
}

pub(crate) fn close_button(target: ComponentId, title: &str) -> Content {
    let icon = IconContent::new("tekes-remove-one").expect("canonical close icon");
    let mut button = Button::new(Content::new(ComponentId(target.0 + 1), icon))
        .with_accessible_name(format!("Close {title}"))
        .with_help(format!("Close {title}"));
    button.variant = tekes_ui_controls::paint::ButtonVariant::Toolbar;
    button.set_transparent_chrome(true);
    Content::new(target, button)
}
