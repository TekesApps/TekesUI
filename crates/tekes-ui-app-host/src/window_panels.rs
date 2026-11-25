use super::*;
use tekes_ui_module_runtime::{
    ProcessControl, ProcessStatus, ViewConnection, ViewFrame, ViewImage,
};

pub(super) struct PanelManagerState {
    pub(super) module_ids: Vec<String>,
    pub(super) column_definitions: Option<Vec<crate::Column>>,
    pub(super) container_factories: Vec<crate::container_factory::RegisteredFactory>,
    pub(super) panels: Vec<Panel>,
}

impl PanelManagerState {
    pub(super) fn new(module_ids: Vec<String>) -> Self {
        Self {
            module_ids,
            column_definitions: None,
            container_factories: Vec::new(),
            panels: Vec::new(),
        }
    }
}

pub(super) struct Panel {
    pub(super) content_icons: tekes_ui_controls::composition::SvgIconCache,
    pub(super) id: u32,
    pub(super) instance: u64,
    pub(super) local: Option<crate::LocalContainer>,
    pub(super) connection: Option<ViewConnection>,
    pub(super) view_image: Option<ViewImage>,
    pub(super) process: ProcessControl,
    pub(super) image: Option<ImageId>,
    pub(super) frame_size: (u32, u32),
    pub(super) frame_pixels: (u32, u32),
    pub(super) viewport: u64,
    pub(super) frame_viewport: u64,
    pub(super) frame_seq: u64,
    pub(super) scale: f32,
    pub(super) bounds: Rect,
    pub(super) mounted_toolbar: crate::window_toolbar::MountedToolbar,
    pub(super) options: crate::ContainerOptions,
    pub(super) clip: Rect,
    pub(super) logically_visible: bool,
    pub(super) toolbar_generation: u64,
    pub(super) geometry: window_container_geometry::ContainerGeometry,
    pub(super) content_padding: f32,
    pub(super) toolbar: Vec<ToolbarItem>,
    pub(super) local_toolbar: Vec<crate::LocalToolbarItem>,
    pub(super) local_toolbar_slots: std::collections::BTreeMap<u64, u64>,
    pub(super) local_toolbar_next_slot: u64,
    pub(super) seq: u64,
    pub(super) failed: Option<String>,
    pub(super) pid: Option<u32>,
    pub(super) module_id: String,
    pub(super) worker_id: String,
    pub(super) dispose_when_closed: bool,
    pub(super) retired: bool,
    pub(super) needs_launch: bool,
    pub(super) acceptance_commands: Option<Vec<(u64, u64)>>,
}

impl Panel {
    pub(super) fn new(id: u32, module_id: String) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let instance = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let connection = ViewConnection::new_inert();
        let process = connection.control().clone();
        Self {
            content_icons: Default::default(),
            id,
            instance,
            local: None,
            worker_id: module_id.clone(),
            dispose_when_closed: false,
            retired: false,
            module_id,
            needs_launch: true,
            acceptance_commands: None,
            connection: Some(connection),
            view_image: None,
            process,
            image: None,
            frame_size: (0, 0),
            frame_pixels: (0, 0),
            viewport: 0,
            frame_viewport: 0,
            frame_seq: 0,
            scale: 1.,
            bounds: Rect::default(),
            mounted_toolbar: crate::window_toolbar::MountedToolbar::new(),
            options: crate::ContainerOptions::default(),
            clip: Rect::default(),
            logically_visible: true,
            toolbar_generation: 0,
            geometry: Default::default(),
            content_padding: 0.,
            toolbar: Vec::new(),
            local_toolbar: Vec::new(),
            local_toolbar_slots: Default::default(),
            local_toolbar_next_slot: 0,
            seq: 0,
            failed: None,
            pid: None,
        }
    }

    pub(super) fn from_column(
        index: usize,
        column: crate::Column,
        _presentation: Option<&crate::PresentationState>,
    ) -> Self {
        let mut panel = Self::new(index as u32, column.id().to_owned());
        panel.options = column.options().clone();
        if let Some(padding) = panel.options.content_padding {
            panel.content_padding = padding;
        }
        match column {
            crate::Column::Container(container) => {
                panel.toolbar = container.toolbar.clone();
                panel.local_toolbar = container.toolbar_items.clone();
                panel.ensure_local_toolbar_slots();
                panel.local = Some(container);
                panel.needs_launch = false;
                panel.connection = None;
            }
            crate::Column::ProcessContainer(container) => match container.backing {
                crate::columns::ProcessBacking::V1(definition) => {
                    panel.worker_id = definition.id.into();
                }
                crate::columns::ProcessBacking::V2 {
                    definition,
                    component_options,
                } => {
                    panel.worker_id = definition.id.into();
                    let local = crate::LocalContainer::from_source(
                        container.instance_id,
                        crate::columns::V2ProcessContainerContent::new(
                            definition.id,
                            panel.instance,
                            panel.mounted_toolbar.window,
                            component_options,
                        ),
                    )
                    .options(container.options);
                    panel.toolbar = local.toolbar.clone();
                    panel.local_toolbar = local.toolbar_items.clone();
                    panel.ensure_local_toolbar_slots();
                    panel.local = Some(local);
                    panel.needs_launch = false;
                    panel.connection = None;
                }
            },
        }
        panel
    }

    pub(super) fn start(&mut self, wake: WindowWake) {
        if self.retired {
            return;
        }
        if let Some(local) = &mut self.local {
            if let Err(error) = local.start_process(wake.clone()) {
                host_diag!("PANEL_FAILED id={} {error:#}", self.id);
                self.failed = Some(format!("{error:#}"));
                wake.wake();
            }
            return;
        }
        if !self.needs_launch {
            return;
        }
        self.needs_launch = false;
        let queued = self
            .connection
            .as_ref()
            .map_or_else(Vec::new, ViewConnection::queued_requests);
        let wake_proxy = wake.clone();
        match ViewConnection::spawn_with_control(
            self.worker_id.clone(),
            self.process.clone(),
            move || {
                wake_proxy.wake();
            },
        ) {
            Ok(connection) => {
                for request in queued {
                    if let Err(error) = connection.enqueue(request) {
                        let message = format!("{error:#}");
                        host_diag!("PANEL_FAILED id={} {message}", self.id);
                        self.failed = Some(message);
                        break;
                    }
                }
                self.connection = Some(connection);
            }
            Err(error) => {
                let message = format!("{error:#}");
                host_diag!("PANEL_FAILED id={} {message}", self.id);
                self.failed = Some(message);
                self.connection = None;
                wake.wake();
            }
        }
    }

    pub(super) fn poll_status_and_frames(&mut self) -> Vec<ViewFrame> {
        if self.retired {
            return Vec::new();
        }
        let Some(connection) = &self.connection else {
            return Vec::new();
        };
        if let ProcessStatus::Failed { message } = connection.status()
            && self.failed.is_none()
        {
            host_diag!("PANEL_FAILED id={} {message}", self.id);
            self.failed = Some(message);
        }
        connection.drain_frames()
    }

    pub(super) fn set_extension_icons(&mut self, icons: Vec<tekes_ui_icons::Glyph>) {
        self.content_icons.set_extensions(icons.clone());
        self.mounted_toolbar.icons.set_extensions(icons);
    }

    pub(super) fn retire(&mut self, mut unregister: impl FnMut(ImageId)) {
        self.retired = true;
        self.needs_launch = false;
        self.mounted_toolbar.host.blur();
        let toolbar_extensions = self.mounted_toolbar.icons.extensions().to_vec();
        self.mounted_toolbar = crate::window_toolbar::MountedToolbar::new();
        self.mounted_toolbar
            .icons
            .set_extensions(toolbar_extensions);
        if let Some(local) = &mut self.local {
            local.set_active(false);
            local.cancel();
        }
        self.local = None;
        self.toolbar.clear();
        self.local_toolbar.clear();
        self.local_toolbar_slots.clear();
        // Unregister before dropping ViewImage so GPU retirement waits only after no
        // Scene draw can reference the old ImageId.
        if let Some(image) = self.image.take() {
            unregister(image);
        }
        self.view_image = None;
        if let Some(connection) = self.connection.take() {
            connection.close();
        }
        self.process.cancel();
        self.content_icons.clear();
        self.logically_visible = false;
    }

    pub(super) fn request(&mut self, kind: InputKind, x: f32, y: f32) -> Result<()> {
        if self.retired || self.failed.is_some() || self.needs_launch {
            return Ok(());
        }
        if kind == InputKind::Command {
            if let Some(item) = self.toolbar.first() {
                return self.command(item.id);
            }
            return Ok(());
        }
        self.enqueue(kind, x, y, None)
    }

    pub(super) fn command(&mut self, id: u64) -> Result<()> {
        let Some(item) = self.toolbar.iter().find(|item| item.id == id) else {
            return Ok(());
        };
        if self.failed.is_some()
            || !item.enabled
            || (item.geometry_dependent && self.frame_viewport != self.viewport)
        {
            return Ok(());
        }
        self.enqueue(InputKind::Command, 0., 0., Some(id))
    }

    pub(super) fn ensure_local_toolbar_slots(&mut self) {
        let mut alloc = |id: u64| {
            if !self.local_toolbar_slots.contains_key(&id) {
                self.local_toolbar_slots
                    .insert(id, self.local_toolbar_next_slot);
                self.local_toolbar_next_slot = self
                    .local_toolbar_next_slot
                    .checked_add(1)
                    .expect("local toolbar slot exhausted");
            }
        };
        for item in &self.local_toolbar {
            alloc(item.id());
            if let crate::LocalToolbarItem::Group(group) = item {
                for member in &group.items {
                    alloc(member.id());
                }
            }
        }
    }

    pub(super) fn slot_for_local_item(&self, id: u64) -> Option<u64> {
        self.local_toolbar_slots.get(&id).copied()
    }

    pub(super) fn has_toolbar_items(&self) -> bool {
        !self.toolbar.is_empty() || !self.local_toolbar.is_empty()
    }

    pub(super) fn sync_local_toolbar(&mut self) -> Result<()> {
        if self.retired {
            return Ok(());
        }
        if let Some(local) = &mut self.local
            && let Some(items) = local.poll_toolbar_items()
        {
            crate::local_toolbar::validate_unique_ids(&self.module_id, &items)?;
            if self.local_toolbar != items {
                self.toolbar = crate::local_toolbar::collect_protocol_buttons(&items);
                self.local_toolbar = items;
            }
        }
        self.ensure_local_toolbar_slots();
        Ok(())
    }

    pub(super) fn toolbar_action(&mut self, action: crate::LocalToolbarAction) -> Result<()> {
        if self.retired || self.failed.is_some() {
            return Ok(());
        }
        if self.local.is_some() {
            match crate::local_toolbar::find_member(&self.local_toolbar, action.id) {
                Some(crate::LocalToolbarItem::Button(button)) => {
                    if !button.enabled
                        || (button.geometry_dependent && self.frame_viewport != self.viewport)
                    {
                        return Ok(());
                    }
                    let id = action.id;
                    self.local.as_mut().unwrap().toolbar_action(action)?;
                    self.seq += 1;
                    if let Some(trace) = &mut self.acceptance_commands {
                        trace.push((id, self.seq));
                    }
                    return Ok(());
                }
                Some(crate::LocalToolbarItem::Menu(menu)) => {
                    if !menu.enabled {
                        return Ok(());
                    }
                    let Some(selection) = action.menu_selection() else {
                        return Ok(());
                    };
                    if let Some(path) = selection.value_path()
                        && !crate::local_toolbar::entry_enabled(&menu.entries, path)
                    {
                        return Ok(());
                    }
                    self.local.as_mut().unwrap().toolbar_action(action)?;
                    self.seq += 1;
                    return Ok(());
                }
                Some(crate::LocalToolbarItem::Group(_)) | None => return self.command(action.id),
            }
        }
        self.command(action.id)
    }

    pub(super) fn enqueue(
        &mut self,
        kind: InputKind,
        x: f32,
        y: f32,
        command: Option<u64>,
    ) -> Result<()> {
        if self.retired {
            return Ok(());
        }
        if let Some(local) = &mut self.local {
            if let Some(command) = command {
                local.command(command)?;
            }
            if kind == InputKind::Cancel {
                local.cancel();
            }
            self.seq += 1;
            if let Some(command) = command
                && let Some(trace) = &mut self.acceptance_commands
            {
                trace.push((command, self.seq));
            }
            return Ok(());
        }
        self.seq += 1;
        let scale = self.scale.clamp(0.5, 4.);
        let max_logical = (8192. / scale).floor().clamp(1., 4096.) as u32;
        let (width, height) = self.content_size();
        let request = Request {
            version: VERSION,
            command,
            seq: self.seq,
            viewport: self.viewport,
            kind,
            x,
            y,
            width: width.clamp(1, max_logical),
            height: height.clamp(1, max_logical),
            identity: self.id,
            scale,
        };
        if let Some(connection) = &self.connection {
            match connection.status() {
                ProcessStatus::Failed { message } => {
                    if self.failed.is_none() {
                        self.failed = Some(message);
                    }
                    return Ok(());
                }
                ProcessStatus::Closed | ProcessStatus::Closing => return Ok(()),
                ProcessStatus::Starting | ProcessStatus::Ready { .. } => {}
            }
            if let Err(error) = connection.enqueue(request) {
                match connection.status() {
                    ProcessStatus::Failed { message } => {
                        if self.failed.is_none() {
                            self.failed = Some(message);
                        }
                        return Ok(());
                    }
                    ProcessStatus::Closed | ProcessStatus::Closing => return Ok(()),
                    ProcessStatus::Starting | ProcessStatus::Ready { .. } => return Err(error),
                }
            }
        }
        if let Some(command) = command
            && let Some(trace) = &mut self.acceptance_commands
        {
            trace.push((command, self.seq));
        }
        Ok(())
    }

    pub(super) fn cancel_geometry_input(&mut self) -> Result<()> {
        if let Some(local) = &mut self.local {
            local.with_host(&mut |host| {
                host.cancel_pointer_capture();
            });
            self.seq += 1;
            Ok(())
        } else {
            self.request(InputKind::Cancel, 0., 0.)
        }
    }

    pub(super) fn content_frame(&self) -> Rect {
        let body = self.geometry.content;
        let inset = self.content_padding;
        Rect::new(
            body.min_x() + inset,
            body.min_y() + inset,
            (body.size.width - 2. * inset).max(0.),
            (body.size.height - 2. * inset).max(0.),
        )
    }

    pub(super) fn content_size(&self) -> (u32, u32) {
        let content = self.content_frame();
        (
            content.size.width.max(1.) as u32,
            content.size.height.max(1.) as u32,
        )
    }

    pub(super) fn content_point(&self, point: Point) -> Point {
        let content = self.content_frame();
        Point::new(point.x - content.min_x(), point.y - content.min_y())
    }

    pub(super) fn content_inset(&self) -> f32 {
        self.geometry.insets.top
    }

    #[cfg(test)]
    pub(super) fn geometry_with_header(
        &mut self,
        bounds: Rect,
        scale: f32,
        toolbar_height: f32,
        header_extra: f32,
    ) -> Result<()> {
        self.geometry_with_layout(
            bounds,
            scale,
            window_container_geometry::ContainerGeometry::legacy(
                bounds,
                toolbar_height + header_extra,
            ),
        )
    }

    pub(super) fn geometry_with_layout(
        &mut self,
        bounds: Rect,
        scale: f32,
        geometry: window_container_geometry::ContainerGeometry,
    ) -> Result<()> {
        let previous = self.content_size();
        let scale_changed = self.scale != scale;
        self.bounds = bounds;
        self.scale = scale;
        self.geometry = geometry;
        let changed = previous != self.content_size() || scale_changed;
        if changed {
            // A height-only content resize can leave the toolbar paint frame
            // unchanged. Revoke old arms before advancing viewport ownership.
            self.viewport += 1;
            host_diag!(
                "GEOMETRY panel={} viewport={} width={:.3} height={:.3} toolbar_height={:.3} scale={:.3}",
                self.id,
                self.viewport,
                bounds.size.width,
                bounds.size.height,
                self.geometry.insets.top,
                scale
            );
            host_diag!(
                "CONTAINER_CONTENT id={} padding={} toolbar_boundary={} frame={:?}",
                self.module_id,
                self.content_padding,
                self.bounds.min_y() + self.content_inset(),
                self.content_frame()
            );
            self.request(InputKind::Layout, 0., 0.)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn needs_launch(&self) -> bool {
        self.needs_launch
    }

    #[cfg(test)]
    pub(super) fn mark_started_inert_for_test(&mut self) {
        self.needs_launch = false;
    }

    #[cfg(test)]
    pub(super) fn queued_requests(&self) -> Vec<Request> {
        self.connection
            .as_ref()
            .map_or_else(Vec::new, ViewConnection::queued_requests)
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        self.view_image = None;
        if let Some(connection) = self.connection.take() {
            connection.close();
        }
        self.process.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppWindow, ComponentDefinition, ContainerFactory, ContainerMultiplicity, ContainerOptions,
        LocalContainer, OthersOptions, PresentationOptions, ProcessContainer,
    };
    use tekes_ui_controls::composition::{ComponentId, Content, TextContent};
    use tekes_ui_module_runtime::{
        ComponentEmitter, ProcessModule,
        component::{Capabilities, RequestBody},
    };

    struct DummyV2Module;
    impl ProcessModule for DummyV2Module {
        fn handle(&mut self, _: RequestBody, _: &ComponentEmitter) -> Result<serde_json::Value> {
            Ok(serde_json::Value::Null)
        }
    }

    fn v2_def_named(id: &'static str) -> ComponentDefinition {
        ComponentDefinition {
            id,
            capabilities: Capabilities {
                services: true,
                toolbar: true,
                view: true,
            },
            create: || Box::new(DummyV2Module),
        }
    }

    #[test]
    fn panel_draw_error_after_import_keeps_connection_alive_without_silent_failure() {
        let mut panel = Panel::new(0, "remote".into());
        panel.mark_started_inert_for_test();
        panel.bounds = Rect::new(0., 0., 400., 300.);
        panel.geometry = window_container_geometry::ContainerGeometry::legacy(panel.bounds, 0.);
        panel.enqueue(InputKind::Layout, 0., 0., None).unwrap();

        // Simulate an interrupted draw pass before presentation completes: the
        // panel retains its connection and seq without marking `failed` or
        // killing its worker via dropped ack channels.
        let interrupted_draw: Result<()> = Err(anyhow::anyhow!("transient render failure"));
        assert!(interrupted_draw.is_err());
        assert!(panel.failed.is_none());
        assert!(!panel.process.is_cancelled());
        assert_eq!(panel.queued_requests().len(), 1);
        panel
            .enqueue(InputKind::PointerDown, 12., 18., None)
            .unwrap();
        assert_eq!(panel.queued_requests().len(), 2);
    }

    #[test]
    fn v2_process_component_assembles_lazily_and_registers_worker_dispatch() {
        let local =
            LocalContainer::new("focus", Content::new(ComponentId(1), TextContent::new("F")))
                .unwrap();
        let def = v2_def_named("v2-remote");
        let created = v2_def_named("v2-created");
        let mut host = AppWindow::from_containers(
            vec![
                local.into(),
                ProcessContainer::from_component(def)
                    .instance_id("v2-pane")
                    .into(),
            ],
            PresentationOptions::new("focus").others(OthersOptions::new(vec!["v2-pane".into()])),
        )
        .unwrap()
        .container_factories(vec![ContainerFactory::process_component(
            "v2-factory",
            "V2 Item",
            ContainerMultiplicity::Single,
            created,
            ContainerOptions::default(),
        )])
        .unwrap()
        .into_host(false);

        host.mount_columns();
        assert_eq!(host.panel_mgr.panels.len(), 2);
        assert!(host.panel_mgr.panels[1].local.is_some());
        assert!(host.panel_mgr.panels[1].connection.is_none());
        assert!(!host.panel_mgr.panels[1].needs_launch());
        assert!(host.panel_mgr.panels[1].pid.is_none());

        // Unknown/unlaunched worker args fail cleanly at the argument gate.
        let app = AppWindow::from_containers(
            vec![ProcessContainer::from_component(def).into()],
            PresentationOptions::new("v2-remote"),
        )
        .unwrap();
        assert_eq!(app.components.len(), 1);
        assert!(
            app.run_with_args(&["app".into(), "--unknown".into()])
                .is_err()
        );
    }
}
