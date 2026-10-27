//! Declarations and business bindings only. One shared ToolbarHost per window.
use crate::toolbar_source::ComponentSource;
use anyhow::{Context, Result};
use std::collections::HashMap;
use tekes_ui_controls::composition::*;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Rect};
pub(crate) fn icon_image(
    images: &mut HashMap<(String, u32), ImageId>,
    renderer: &mut Renderer,
    name: &str,
    scale: f32,
) -> Result<ImageId> {
    let key = (name.to_owned(), (scale * 1000.).round() as u32);
    if let std::collections::hash_map::Entry::Vacant(slot) = images.entry(key.clone()) {
        // Visibility artwork is a Host presentation choice, not a shared toggle style.
        let svg = match name {
            "apphost-leading-collapsed" => include_str!("../assets/leading-collapsed.svg"),
            "apphost-trailing-collapsed" => include_str!("../assets/trailing-collapsed.svg"),
            _ => {
                tekes_ui_icons::Glyph::ALL
                    .iter()
                    .find(|g| g.name == name)
                    .context("unknown toolkit icon")?
                    .svg
            }
        };
        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())?;
        let pixels = (24. * scale).ceil().max(1.) as u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(pixels, pixels).context("icon allocation")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(pixels as f32 / 24., pixels as f32 / 24.),
            &mut pixmap.as_mut(),
        );
        slot.insert(renderer.register_image(pixmap.data(), pixels, pixels)?);
    }
    Ok(images[&key])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Visibility(usize),
    Navigation(ComponentSource),
    Title,
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
    Title(String),
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
    pub host: ToolbarHost<Source>,
    pub window: u64,
    declarations: Vec<Declaration>,
}
impl MountedToolbar {
    pub fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let window = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            host: ToolbarHost::new(ComponentId(90000), window, Rect::default())
                .expect("toolbar root"),
            window,
            declarations: vec![],
        }
    }
    pub fn source(&self, group: ComponentId) -> Option<&Source> {
        self.declarations.iter().find(|d| d.id == group).map(|d| &d.source)
    }
    pub fn sync(
        &mut self,
        viewport: Rect,
        regions: Vec<ToolbarRegion>,
        declarations: Vec<Declaration>,
    ) -> Result<()> {
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
                    (Contents::Commands(a, _), Contents::Commands(b, _)) => {
                        a.iter().map(|b| b.id).eq(b.iter().map(|b| b.id))
                    }
                    (Contents::Title(_), Contents::Title(_))
                    => true,
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
                            Contents::Commands(buttons, overflow) => {
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
                                        },
                                    )
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
            let (child, bindings) = match &d.contents {
                Contents::Commands(buttons, overflow) => (
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

            };
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
