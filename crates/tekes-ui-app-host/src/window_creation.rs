//! Creation appends original mounted Container records; closed records are never recycled.
use super::*;
impl Host {
    pub(crate) fn with_container_factories(
        mut self,
        factories: Vec<crate::ContainerFactory>,
    ) -> Self {
        self.container_factories = factories
            .into_iter()
            .map(|definition| crate::container_factory::RegisteredFactory {
                definition,
                next_instance: 1,
                single: None,
            })
            .collect();
        self
    }
    pub(super) fn creation_choices(&self) -> Vec<(String, String, String, bool)> {
        let mut choices = Vec::new();
        if let Some(state) = &self.presentation {
            for panel in &self.panels {
                if panel.dispose_when_closed
                    || panel.retired
                    || state.nav_bar_id() == Some(panel.module_id.as_str())
                    || state.focus_id() == panel.module_id
                    || self
                        .container_factories
                        .iter()
                        .any(|factory| factory.single.as_ref() == Some(&panel.module_id))
                {
                    continue;
                }
                let title = panel.options.title.as_deref().unwrap_or(&panel.module_id);
                choices.push((
                    format!("open:{}", panel.module_id),
                    format!("Open {title}"),
                    panel
                        .options
                        .switcher_icon
                        .clone()
                        .unwrap_or_else(|| "tekes-file-generic".into()),
                    false,
                ));
            }
        }
        choices.extend(self.container_factories.iter().map(|factory| {
            (
                format!("create:{}", factory.definition.id),
                factory.definition.title.clone(),
                factory.definition.icon.clone(),
                factory.single.as_ref().is_some_and(|id| {
                    self.presentation
                        .as_ref()
                        .is_some_and(|s| s.focus_id() == id)
                }),
            )
        }));
        choices
    }
    pub(super) fn creation_entries(&self) -> Vec<tekes_ui_controls::composition::MenuEntry> {
        self.creation_choices()
            .into_iter()
            .map(
                |(value, label, _, disabled)| tekes_ui_controls::composition::MenuEntry::Item {
                    value,
                    label,
                    checked: None,
                    disabled,
                },
            )
            .collect()
    }
    pub(super) fn open_creation_choice(&mut self, value: &str) -> Result<()> {
        ensure!(
            self.creation_choices()
                .iter()
                .any(|(id, _, _, disabled)| id == value && !disabled),
            "creation choice is no longer available"
        );
        if let Some(id) = value.strip_prefix("open:") {
            let state = self.presentation.as_mut().unwrap();
            state.open(id)?;
            if !state.others_visible() {
                state.toggle_others()?;
            }
        } else if let Some(id) = value.strip_prefix("create:") {
            self.create_container(id)?;
        }
        Ok(())
    }
    pub(super) fn close_container(&mut self, id: &str) -> Result<()> {
        self.presentation
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("container close requires presentation"))?
            .close(id)?;
        let Some(index) = self
            .panels
            .iter()
            .position(|panel| panel.module_id == id && panel.dispose_when_closed)
        else {
            return Ok(());
        };
        let renderer = &mut self.renderer;
        self.panels[index].retire(|image| {
            if let Some(renderer) = renderer {
                renderer.unregister_image(image);
            }
        });
        self.presentation.as_mut().unwrap().retire_created(id);
        Ok(())
    }
    pub(super) fn create_container(&mut self, factory_id: &str) -> Result<String> {
        self.mount_initial_containers();
        let index = self
            .container_factories
            .iter()
            .position(|f| f.definition.id == factory_id)
            .ok_or_else(|| anyhow::anyhow!("unknown container factory: {factory_id}"))?;
        ensure!(
            self.presentation
                .as_ref()
                .is_some_and(|s| s.others().is_some()),
            "creation requires Others"
        );
        if let Some(id) = self.container_factories[index].single.clone() {
            let state = self.presentation.as_mut().unwrap();
            ensure!(state.focus_id() != id, "single instance is already Focus");
            state.open(&id)?;
            if !state.others_visible() {
                state.toggle_others()?;
            }
            return Ok(id);
        }
        let factory = self.container_factories[index].definition.clone();
        let (id, ordinal) = loop {
            let serial = self.container_factories[index].next_instance;
            self.container_factories[index].next_instance += 1;
            let id = format!("{}-instance-{serial}", factory.id);
            if !self.panels.iter().any(|p| p.module_id == id) {
                break (id, serial);
            }
        };
        let container = factory.create(&id, ordinal)?;
        crate::columns::validate_ids(
            self.panels
                .iter()
                .map(|p| (p.module_id.as_str(), &p.options))
                .chain(std::iter::once((
                    container.id(),
                    container.container_options(),
                ))),
        )?;
        let state = self.presentation.as_mut().unwrap();
        state.register_created(&container, &factory);
        state.open(&id)?;
        if !state.others_visible() {
            state.toggle_others()?;
        }
        let mut panel = self.mount_column(self.panels.len(), container.into());
        panel.dispose_when_closed = factory.multiplicity == crate::ContainerMultiplicity::Multiple;
        self.module_ids.push(id.clone());
        self.order.push(self.panels.len());
        self.panels.push(panel);
        if factory.multiplicity == crate::ContainerMultiplicity::Single {
            self.container_factories[index].single = Some(id.clone());
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ContainerFactory, ContainerMultiplicity, LocalContainer, OthersOptions, PresentationOptions,
    };
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_controls::composition::{Content, TextContent};
    fn local(id: &str) -> LocalContainer {
        LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
    }
    fn host(factories: Vec<ContainerFactory>) -> Host {
        crate::AppWindow::from_containers(
            vec![local("focus").into()],
            PresentationOptions::new("focus").others(OthersOptions::new(Vec::new())),
        )
        .unwrap()
        .container_factories(factories)
        .unwrap()
        .into_host(false)
    }
    #[test]
    fn registered_single_reopens_original_panel_while_multiple_creates_independent_instances() {
        let constructed = Rc::new(Cell::new(0));
        let calls = constructed.clone();
        let mut host = host(vec![
            ContainerFactory::local(
                "inspector",
                "Inspector",
                ContainerMultiplicity::Single,
                move |id| {
                    calls.set(calls.get() + 1);
                    Ok(local(id))
                },
            ),
            ContainerFactory::local("notes", "Notes", ContainerMultiplicity::Multiple, |id| {
                Ok(local(id))
            }),
        ]);
        let single = host.create_container("inspector").unwrap();
        let original_instance = host.panels[1].instance;
        host.panels[1].seq = 27;
        host.close_container(&single).unwrap();
        assert_eq!(host.create_container("inspector").unwrap(), single);
        assert_eq!(constructed.get(), 1);
        assert_eq!(host.panels[1].instance, original_instance);
        assert_eq!(host.panels[1].seq, 27);
        let first = host.create_container("notes").unwrap();
        let second = host.create_container("notes").unwrap();
        assert_ne!(first, second);
        assert_ne!(host.panels[2].instance, host.panels[3].instance);
        host.close_container(&first).unwrap();
        assert!(host.panels[2].local.is_none());
        assert!(host.panels[2].process.is_cancelled());
        assert!(host.presentation.as_mut().unwrap().open(&first).is_err());
        let third = host.create_container("notes").unwrap();
        assert_ne!(third, first);
        assert!(
            host.presentation
                .as_ref()
                .unwrap()
                .others_ids()
                .contains(&second)
        );
        assert!(
            !host
                .presentation
                .as_ref()
                .unwrap()
                .others_ids()
                .contains(&first)
        );
        assert_eq!(host.panels[2].module_id, first);
    }
    #[test]
    fn registered_process_instances_keep_worker_factory_separate_from_container_identity() {
        fn worker() -> Box<dyn tekes_ui_module_runtime::PanelModule> {
            panic!("identity test must not launch a worker")
        }
        let definition = crate::ModuleDefinition {
            id: "registered-worker",
            create: worker,
        };
        let mut host = host(vec![ContainerFactory::process(
            "documents",
            "Document",
            ContainerMultiplicity::Multiple,
            definition,
            crate::ContainerOptions::default(),
        )]);
        let a = host.create_container("documents").unwrap();
        let b = host.create_container("documents").unwrap();
        assert_ne!(a, b);
        assert_eq!(host.panels[1].worker_id, definition.id);
        assert_eq!(host.panels[2].worker_id, definition.id);
        assert_eq!(host.panels[1].module_id, a);
        assert_eq!(host.panels[2].module_id, b);
        assert_ne!(host.panels[1].instance, host.panels[2].instance);
        host.close_container(&a).unwrap();
        assert!(host.panels[1].process.is_cancelled());
        assert!(!host.panels[2].process.is_cancelled());
        assert_eq!(host.presentation.as_ref().unwrap().others_ids(), &[b]);
    }
    #[test]
    fn retired_multiple_releases_pending_images_and_rejects_late_input_at_original_boundaries() {
        let mut panel = Panel::new(0, "retired".into());
        let (tx, rx) = mpsc::sync_channel(1);
        panel.rx = rx;
        tx.send(Err("already queued result".into())).unwrap();
        panel.image = Some(ImageId(73));
        panel.launch_tx = None;
        panel.request(InputKind::Cancel, 0., 0.).unwrap();
        assert_eq!(panel.inbox.queue.lock().unwrap().len(), 1);
        let mut removed = Vec::new();
        panel.retire(|image| removed.push(image));
        assert_eq!(removed, vec![ImageId(73)]);
        assert!(panel.image.is_none());
        assert!(panel.next_pending().is_none());
        assert!(tx.send(Err("late result".into())).is_err());
        for _ in 0..300 {
            panel.request(InputKind::Cancel, 0., 0.).unwrap();
        }
        panel.enqueue(InputKind::Command, 0., 0., Some(1)).unwrap();
        assert!(panel.inbox.queue.lock().unwrap().is_empty());
        assert!(panel.retired);
    }
    fn refresh(host: &mut Host) -> Scene {
        host.mount_initial_containers();
        let whole = Rect::new(0., 0., 1200., 700.);
        let layout = host
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, 48.);
        host.apply_presentation_layout(layout).unwrap();
        host.sync_container_toolbars(
            true,
            whole,
            super::super::window_toolbar_mount::ToolbarBand {
                y: 0.,
                height: 48.,
                native_leading: 0.,
            },
            40.,
        )
        .unwrap();
        for panel in &mut host.panels {
            super::super::window_presentation::render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        host.reconcile_toolbar_focus(true);
        let mut scene = Scene::new();
        host.paint_presentation(&mut scene, whole, Theme::LIGHT.into())
            .unwrap();
        scene
    }
    fn node(
        host: &Host,
        name: &str,
        grid: bool,
    ) -> (
        tekes_ui_controls::composition::ToolbarAccessibilityTarget,
        Rect,
    ) {
        let nodes = if grid {
            host.presentation_tabs.as_ref().unwrap().semantics()
        } else {
            host.panels
                .iter()
                .flat_map(|p| p.mounted_toolbar.host.semantics())
                .collect()
        };
        nodes
            .into_iter()
            .find_map(|node| {
                if node.node.semantics.name == name {
                    Some((node.target, node.node.frame))
                } else {
                    node.parts
                        .into_iter()
                        .find(|(_, part)| part.semantics.name == name)
                        .map(|(target, part)| (target, part.frame))
                }
            })
            .unwrap_or_else(|| panic!("missing original control {name}"))
    }
    fn click(host: &mut Host, point: Point, grid: bool) {
        let target = if grid {
            None
        } else {
            Some(
                host.open_toolbar()
                    .unwrap_or_else(|| host.toolbar_target_at(point).unwrap()),
            )
        };
        if let Some(target) = target {
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
        } else {
            assert!(
                host.presentation_tab_input(&ContentEvent::PointerDown(point))
                    .unwrap()
            );
        }
        refresh(host);
        if let Some(target) = target {
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
                .unwrap();
        } else {
            assert!(
                host.presentation_tab_input(&ContentEvent::PointerUp(point))
                    .unwrap()
            );
        }
        refresh(host);
    }
    fn local_factories() -> Vec<ContainerFactory> {
        vec![
            ContainerFactory::local(
                "scratch",
                "Scratchpad",
                ContainerMultiplicity::Single,
                |id| Ok(local(id)),
            ),
            ContainerFactory::local(
                "notes",
                "Working Notes",
                ContainerMultiplicity::Multiple,
                |id| Ok(local(id)),
            ),
        ]
    }
    #[test]
    fn registered_empty_grid_first_press_survives_paint_and_retired_ax_cannot_create_new_instance()
    {
        use tekes_ui_controls::composition::AccessibilityAction;
        let mut host = host(local_factories());
        let scene = refresh(&mut host);
        assert!(
            !scene.primitives().is_empty(),
            "real grid paints original controls"
        );
        let (old, bounds) = node(&host, "Create Working Notes", true);
        assert!(bounds.min_y() >= 48.);
        click(&mut host, bounds.center(), true);
        assert_eq!(host.panels.len(), 2);
        let first = host.presentation.as_ref().unwrap().others_ids()[0].clone();
        assert_eq!(host.order, vec![0, 1]);
        host.presentation.as_mut().unwrap().toggle_others().unwrap();
        refresh(&mut host);
        assert_eq!(
            host.presentation.as_ref().unwrap().others_ids(),
            std::slice::from_ref(&first)
        );
        host.presentation.as_mut().unwrap().toggle_others().unwrap();
        host.close_container(&first).unwrap();
        refresh(&mut host);
        assert!(
            !host
                .presentation_tabs
                .as_mut()
                .unwrap()
                .accessibility_action(old, AccessibilityAction::Press)
                .consumed
        );
        assert!(host.presentation.as_ref().unwrap().others_ids().is_empty());
        let fresh = node(&host, "Create Working Notes", true).1.center();
        click(&mut host, fresh, true);
        assert_eq!(host.panels.len(), 3);
        assert_ne!(host.presentation.as_ref().unwrap().others_ids()[0], first);
        assert!(host.panels[1].retired);
    }
    #[test]
    fn registered_creation_chevron_menu_cancel_preserves_selection_and_choice_creates_once() {
        let mut host = host(local_factories());
        let first = host.create_container("notes").unwrap();
        refresh(&mut host);
        let menu = node(&host, "Create Container", false).1.center();
        click(&mut host, menu, false);
        assert!(host.open_toolbar().is_some());
        let target = host.toolbar_focus.unwrap();
        host.dispatch_toolbar(
            target,
            &ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            },
        )
        .unwrap();
        refresh(&mut host);
        assert_eq!(host.panels.len(), 2);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&crate::OthersTab::Container(first.clone()))
        );
        click(&mut host, menu, false);
        let row = node(&host, "Working Notes", false).1.center();
        click(&mut host, row, false);
        assert_eq!(host.panels.len(), 3);
        let second = host.presentation.as_ref().unwrap().others_ids()[1].clone();
        assert_ne!(first, second);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&crate::OthersTab::Container(second.clone()))
        );
        let family = node(&host, "Working Notes", true).1.center();
        click(&mut host, family, true);
        let old = node(&host, "Working Notes 1", true).0;
        host.close_container(&first).unwrap();
        let third = host.create_container("notes").unwrap();
        refresh(&mut host);
        assert!(
            !host
                .presentation_tabs
                .as_mut()
                .unwrap()
                .accessibility_action(
                    old,
                    tekes_ui_controls::composition::AccessibilityAction::Press
                )
                .consumed
        );
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&crate::OthersTab::Container(third))
        );
        assert!(
            host.presentation
                .as_ref()
                .unwrap()
                .others_ids()
                .contains(&second)
        );
    }
    #[test]
    fn registered_fixed_container_close_then_grid_open_keeps_original_instance_and_edited_content()
    {
        use tekes_ui_controls::composition::TextFieldContent;
        let mut host = crate::AppWindow::from_containers(
            vec![
                local("focus").into(),
                LocalContainer::new(
                    "inspector",
                    Content::new(ComponentId(1), TextFieldContent::new("Title", "")),
                )
                .unwrap()
                .title("Inspector")
                .into(),
            ],
            PresentationOptions::new("focus").others(OthersOptions::new(vec!["inspector".into()])),
        )
        .unwrap()
        .into_host(false);
        refresh(&mut host);
        let instance = host.panels[1].instance;
        let text = &mut host.text;
        let local = host.panels[1].local.as_mut().unwrap();
        let frame = Rect::new(0., 0., 300., 200.);
        local
            .render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        local.with_host(&mut |host| {
            host.focus(Some(ComponentId(1)), false);
        });
        local
            .event(
                &ContentEvent::Commit {
                    text: "Retained edit".into(),
                    replacement: None,
                },
                text,
            )
            .unwrap();
        host.close_container("inspector").unwrap();
        refresh(&mut host);
        assert!(host.creation_entries().iter().any(|entry| matches!(entry, tekes_ui_controls::composition::MenuEntry::Item { value, .. } if value == "open:inspector")));
        let point = node(&host, "Open Inspector", true).1.center();
        click(&mut host, point, true);
        assert_eq!(host.panels.len(), 2);
        assert_eq!(host.panels[1].instance, instance);
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert_eq!(
                    host.get(ComponentId(1))
                        .unwrap()
                        .component::<TextFieldContent>()
                        .unwrap()
                        .surface
                        .input
                        .text,
                    "Retained edit"
                );
            });
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&crate::OthersTab::Container("inspector".into()))
        );
    }
    #[test]
    fn registered_multiple_instances_dispatch_same_command_only_to_current_original_owner() {
        let calls = Rc::new(std::cell::RefCell::new(Vec::new()));
        let output = calls.clone();
        let mut host = host(vec![ContainerFactory::local(
            "notes",
            "Notes",
            ContainerMultiplicity::Multiple,
            move |id| {
                let instance = id.to_owned();
                let output = output.clone();
                Ok(local(id)
                    .toolbar(vec![ToolbarItem {
                        id: 7,
                        title: "Touch".into(),
                        enabled: true,
                        geometry_dependent: false,
                    }])
                    .on_command(move |_, _| {
                        output.borrow_mut().push(instance.clone());
                        Ok(())
                    }))
            },
        )]);
        let first = host.create_container("notes").unwrap();
        let second = host.create_container("notes").unwrap();
        refresh(&mut host);
        let point = node(&host, "Touch", false).1.center();
        click(&mut host, point, false);
        assert_eq!(*calls.borrow(), vec![second.clone()]);
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(crate::OthersTab::Container(first.clone()))
            .unwrap();
        refresh(&mut host);
        let (old, frame) = node(&host, "Touch", false);
        click(&mut host, frame.center(), false);
        assert_eq!(*calls.borrow(), vec![second, first.clone()]);
        host.close_container(&first).unwrap();
        host.create_container("notes").unwrap();
        refresh(&mut host);
        assert!(
            !host.panels[0]
                .mounted_toolbar
                .host
                .accessibility_action(
                    old,
                    tekes_ui_controls::composition::AccessibilityAction::Press
                )
                .consumed
        );
        assert_eq!(calls.borrow().len(), 2);
    }
    #[test]
    fn registered_creation_preserves_declared_list_eligibility_and_real_list_layout() {
        let mut host = host(vec![
            ContainerFactory::local(
                "scratch",
                "Scratchpad",
                ContainerMultiplicity::Single,
                |id| Ok(local(id)),
            ),
            ContainerFactory::local(
                "reading",
                "Reading",
                ContainerMultiplicity::Multiple,
                |id| Ok(local(id).list_height(240.)),
            ),
        ]);
        let scratch = host.create_container("scratch").unwrap();
        let state = host.presentation.as_mut().unwrap();
        assert!(
            !state.tabs().contains(&crate::OthersTab::List),
            "creation without list_height must not invent aggregate eligibility"
        );
        assert!(state.list_ids().is_empty());
        assert!(state.select_tab(crate::OthersTab::List).is_err());
        let whole = Rect::new(0., 0., 1200., 700.);
        let layout = state.layout_with_toolbar(whole, 48.);
        assert!(
            layout
                .frames
                .iter()
                .any(|(id, frame)| id == &scratch && frame.size.height > 0.)
        );
        host.apply_presentation_layout(layout).unwrap();

        let reading = host.create_container("reading").unwrap();
        let state = host.presentation.as_mut().unwrap();
        assert!(state.tabs().contains(&crate::OthersTab::List));
        assert_eq!(state.list_ids(), vec![reading.as_str()]);
        state.select_tab(crate::OthersTab::List).unwrap();
        let layout = state.layout_with_toolbar(whole, 48.);
        assert_eq!(
            layout
                .frames
                .iter()
                .find(|(id, _)| id == &reading)
                .unwrap()
                .1
                .size
                .height,
            240.
        );
        assert!(!layout.frames.iter().any(|(id, _)| id == &scratch));
        let frames = host.apply_presentation_layout(layout).unwrap();
        assert_eq!(frames[1], Rect::default());
        assert_eq!(frames[2].size.height, 240.);
    }
}
