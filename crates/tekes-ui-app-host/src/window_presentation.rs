//! Window presentation routes stable container identities. Each Panel retains
//! its original ContentHost/process and its independently owned Toolbar.
use super::*;
use crate::{
    OthersTab,
    window_toolbar::{Contents, Declaration, Source},
};
use tekes_ui_controls::composition::{
    Content, ContentActionPayload, TabItem, TabsContent, TextContent, ToolbarButtonDeclaration,
    ToolbarGroupOverflow, ToolbarPlacement, ToolbarRegion,
};
use tekes_ui_platform_macos::{CommandId, Menu, MenuCommand, MenuItem};
const COMMAND_BASE: u32 = 0x4300_0000;
const TABS_OWNER: u64 = u64::MAX - 1;
#[cfg(test)]
fn test_band() -> window_toolbar_mount::ToolbarBand {
    window_toolbar_mount::ToolbarBand {
        y: 0.,
        height: 48.,
        native_leading: 0.,
    }
}
// Use this same adjusted frame for Tabs rendering, pointer routing and AX.
fn avoid_native_controls(frame: Rect, reserved: Rect) -> Rect {
    if frame.min_y() < reserved.max_y()
        && frame.max_y() > reserved.min_y()
        && frame.min_x() < reserved.max_x()
    {
        let left = frame.min_x().max(reserved.max_x()).min(frame.max_x());
        Rect::new(
            left,
            frame.min_y(),
            (frame.max_x() - left).max(0.),
            frame.size.height,
        )
    } else {
        frame
    }
}
impl Host {
    pub(super) fn presentation_menu(&self) -> Menu {
        let mut entries = Vec::new();
        if self
            .presentation
            .as_ref()
            .is_some_and(|state| state.others().is_some())
        {
            entries.push(MenuItem::Command(MenuCommand::new(
                "Show / Hide Others",
                CommandId(COMMAND_BASE),
            )));
            for (index, id) in self.module_ids.iter().enumerate() {
                let base = COMMAND_BASE + 1 + index as u32 * 6;
                for (offset, action) in
                    ["Select", "Promote", "Open", "Close", "Move up", "Move down"]
                        .iter()
                        .enumerate()
                {
                    entries.push(MenuItem::Command(MenuCommand::new(
                        format!("{action} {id}"),
                        CommandId(base + offset as u32),
                    )));
                }
            }
            entries.push(MenuItem::Command(MenuCommand::new(
                "Select List",
                CommandId(COMMAND_BASE - 1),
            )));
        }
        Menu::new("Containers", entries)
    }
    pub(super) fn container_toolbar_height(&mut self, index: usize, legacy: f32) -> f32 {
        if self.presentation.is_none() {
            return legacy;
        }
        if self.panels[index].toolbar.is_empty()
            && self.panels[index].options.title.is_none()
            && !(self.navigation.is_some()
                && self
                    .presentation
                    .as_ref()
                    .and_then(|state| state.navigation_toolbar_id())
                    == Some(self.panels[index].module_id.as_str()))
        {
            return 0.;
        }
        ToolbarControl::window().row_height(&mut self.text)
    }
    pub(super) fn presentation_frames(&mut self, whole: Rect, w: &Window) -> Result<Vec<Rect>> {
        let (native_height, leading) = w.titlebar_safe_area();
        let mut layout = self.presentation.as_ref().unwrap().layout(whole);
        layout.tabs = layout.tabs.map(|frame| {
            avoid_native_controls(
                frame,
                Rect::new(
                    whole.min_x(),
                    whole.min_y(),
                    leading as f32,
                    native_height as f32,
                ),
            )
        });
        self.apply_presentation_layout(layout)
    }
    fn apply_presentation_layout(
        &mut self,
        layout: crate::PresentationLayout,
    ) -> Result<Vec<Rect>> {
        let state = self.presentation.as_ref().unwrap();
        let mut logical: Vec<String> = vec![state.focus_id().into()];
        if let Some(nav) = state.nav_bar_id() {
            logical.push(nav.into());
        }
        if state.others_visible() {
            match state.selected_tab() {
                Some(OthersTab::Container(id)) => logical.push(id.clone()),
                Some(OthersTab::List) => {
                    logical.extend(state.list_ids().into_iter().map(str::to_owned))
                }
                None => {}
            }
        }
        let mut frames = vec![Rect::default(); self.panels.len()];
        for (index, panel) in self.panels.iter_mut().enumerate() {
            let frame = layout
                .frames
                .iter()
                .find(|(id, _)| id == &panel.module_id)
                .map(|(_, r)| *r)
                .unwrap_or_default();
            panel.clip = layout
                .clips
                .iter()
                .find(|(id, _)| id == &panel.module_id)
                .map(|(_, r)| *r)
                .unwrap_or_default();
            let was_visible = panel.logically_visible;
            panel.logically_visible = logical.contains(&panel.module_id);
            if was_visible && !panel.logically_visible {
                panel.mounted_toolbar.host.blur();
                if let Some(local) = &mut panel.local {
                    local.cancel();
                }
            }
            if panel.bounds != frame {
                eprintln!(
                    "CONTAINER_GEOMETRY id={} frame={:?} clip={:?} instance={} pid={:?}",
                    panel.module_id, frame, panel.clip, panel.instance, panel.pid
                );
                // Geometry changes revoke only the affected container's transient
                // input. Ordinary repaints do not touch its editor or IME state.
                panel.mounted_toolbar.host.event(&ContentEvent::Cancel);
                panel.cancel_geometry_input()?;
                if frame.size.width <= 0. || frame.size.height <= 0. {
                    panel.mounted_toolbar.host.blur();
                    if !panel.logically_visible
                        && let Some(local) = &mut panel.local
                    {
                        local.cancel();
                    }
                }
                if self.pressed == Some(index) {
                    self.pressed = None;
                }
            }
            frames[index] = frame;
        }
        if layout.tabs.is_none()
            && let Some(tabs) = &mut self.presentation_tabs
        {
            tabs.cancel();
            tabs.focus(None, false);
        }
        self.presentation_layout = layout;
        Ok(frames)
    }
    pub(super) fn paint_presentation(
        &mut self,
        scene: &mut Scene,
        whole: Rect,
        palette: Palette,
    ) -> Result<()> {
        let state = self.presentation.as_ref().unwrap();
        let keys = state.tabs();
        let selected = state.selected_tab().cloned();
        if self.presentation_tab_keys != keys || self.presentation_selected != selected {
            self.presentation_tabs = if keys.is_empty() {
                None
            } else {
                let items = keys
                    .iter()
                    .enumerate()
                    .map(|(index, tab)| {
                        let name = match tab {
                            OthersTab::Container(id) => id.as_str(),
                            OthersTab::List => "List",
                        };
                        TabItem::new(
                            Content::new(
                                ComponentId(1_000_001 + index as u64 * 2),
                                TextContent::new(name),
                            ),
                            Content::new(
                                ComponentId(1_000_002 + index as u64 * 2),
                                TextContent::new(""),
                            ),
                        )
                    })
                    .collect();
                let selected = selected
                    .as_ref()
                    .and_then(|tab| keys.iter().position(|key| key == tab))
                    .unwrap_or(0);
                let mut host = ContentHost::new(Content::new(
                    ComponentId(1_000_000),
                    TabsContent::new(items, selected),
                ))
                .map_err(|e| anyhow::anyhow!("container tabs: {e:?}"))?;
                host.set_action_events(true);
                Some(host)
            };
            self.presentation_tab_keys = keys;
            self.presentation_selected = selected;
        }
        if let (Some(host), Some(frame)) =
            (&mut self.presentation_tabs, self.presentation_layout.tabs)
        {
            host.render(scene, &mut self.text, frame, frame, palette)
                .map_err(|e| anyhow::anyhow!("container tabs paint: {e:?}"))?;
        }
        let focus = self
            .presentation_layout
            .frames
            .iter()
            .find(|(id, _)| id == self.presentation.as_ref().unwrap().focus_id())
            .map(|(_, frame)| *frame)
            .unwrap();
        let area = Rect::new(
            whole.min_x(),
            focus.min_y(),
            whole.size.width,
            (whole.max_y() - focus.min_y()).max(0.),
        );
        let mut right = area;
        self.outer.min_pane = 0.;
        self.inner.min_pane = 0.;
        self.outer.divider = 6.;
        self.inner.divider = 6.;
        if let Some(nav) = self.presentation.as_ref().unwrap().nav_bar_id()
            && let Some((_, frame)) = self
                .presentation_layout
                .frames
                .iter()
                .find(|(id, _)| id == nav)
        {
            self.outer
                .set_ratio(frame.size.width / (area.size.width - 6.).max(1.));
            let layout = self.outer_view.arrange(&mut self.outer, area, None);
            right = layout.trailing.frame();
            self.outer_view.paint_presented_divider(
                scene,
                &self.outer,
                layout.divider.frame(),
                palette,
                self.text.scale,
            );
        }
        if self.presentation.as_ref().unwrap().others_visible() {
            self.inner
                .set_ratio(focus.size.width / (right.size.width - 6.).max(1.));
            let layout = self.inner_view.arrange(&mut self.inner, right, None);
            self.inner_view.paint_presented_divider(
                scene,
                &self.inner,
                layout.divider.frame(),
                palette,
                self.text.scale,
            );
        }
        Ok(())
    }
    pub(super) fn presentation_resize_regions(&self) -> Vec<(Rect, Axis)> {
        let mut regions = Vec::new();
        if self
            .presentation
            .as_ref()
            .is_some_and(|state| state.nav_bar_id().is_some())
        {
            regions.extend(self.outer_view.resize_regions(&self.outer));
        }
        if self
            .presentation
            .as_ref()
            .is_some_and(|state| state.others_visible())
        {
            regions.extend(self.inner_view.resize_regions(&self.inner));
        }
        regions
    }
    fn move_presentation_split(&mut self, point: Point) -> Result<()> {
        if self.drag_split == Some(2) {
            if !self.outer_view.pointer_move(point, &mut self.outer) {
                return Ok(());
            }
            let area = self.outer_view.layout().frame.frame();
            self.presentation.as_mut().unwrap().set_nav_width(
                (area.size.width - self.outer.divider).max(0.) * self.outer.ratio(),
            )?;
        } else {
            if !self.inner_view.pointer_move(point, &mut self.inner) {
                return Ok(());
            }
            self.presentation
                .as_mut()
                .unwrap()
                .set_focus_fraction(self.inner.ratio().clamp(0.01, 0.99))?;
        }
        Ok(())
    }
    fn composition_in_progress(&mut self) -> bool {
        self.panels.iter_mut().any(|panel| {
            let mut marked = false;
            if let Some(local) = &mut panel.local {
                local.with_host(&mut |host| {
                    marked = host
                        .focused_text()
                        .is_some_and(|surface| surface.input.marked.is_some())
                });
            }
            marked
        })
    }
    pub(super) fn presentation_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        if self.presentation.is_none() {
            return Ok(false);
        }
        if let Event::Command(CommandId(command)) = event
            && *command >= COMMAND_BASE - 1
            && *command <= COMMAND_BASE + self.module_ids.len() as u32 * 6
        {
            if self.composition_in_progress() {
                eprintln!("PRESENTATION_REJECTED composition-in-progress");
                return Ok(true);
            }
            let state = self.presentation.as_mut().unwrap();
            let result = if *command == COMMAND_BASE {
                state.toggle_others()
            } else if *command == COMMAND_BASE - 1 {
                state.select_tab(OthersTab::List)
            } else {
                let value = (*command - COMMAND_BASE - 1) as usize;
                let index = value / 6;
                let id = &self.module_ids[index];
                match value % 6 {
                    0 => state.select_tab(OthersTab::Container(id.clone())),
                    1 => state.promote(id),
                    2 => state.open(id),
                    3 => state.close(id),
                    direction => {
                        let items = state.list_ids();
                        let position = items.iter().position(|candidate| *candidate == id);
                        let next = position.map(|position| {
                            if direction == 4 {
                                position.saturating_sub(1)
                            } else {
                                (position + 1).min(items.len().saturating_sub(1))
                            }
                        });
                        next.map_or_else(
                            || Err(anyhow::anyhow!("container is not a List item")),
                            |next| state.reorder_list(id, next),
                        )
                    }
                }
            };
            if let Err(error) = result {
                eprintln!("PRESENTATION_REJECTED {error:#}");
            } else {
                eprintln!(
                    "PRESENTATION_CHANGED focus={} others={:?}",
                    state.focus_id(),
                    state.others()
                );
            }
            return Ok(true);
        }
        if self.open_toolbar().is_some() {
            return Ok(false);
        }
        if let Event::Scroll(scroll) = event
            && self.pressed.is_none()
            && self.open_toolbar().is_none()
            && self.presentation_layout.list_viewport.is_some_and(|frame| {
                frame.contains(Point::new(
                    scroll.position.x as f32,
                    scroll.position.y as f32,
                ))
            })
        {
            let height = self.presentation_layout.list_viewport.unwrap().size.height;
            self.presentation
                .as_mut()
                .unwrap()
                .scroll_list(-scroll.delta.y as f32, height);
            return Ok(true);
        }
        if let Event::Mouse(mouse) = event {
            let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
            match mouse.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } if self.pressed.is_none() && self.open_toolbar().is_none() => {
                    let nav = self.presentation.as_ref().unwrap().nav_bar_id().is_some();
                    let others = self.presentation.as_ref().unwrap().others_visible();
                    if nav && self.outer_view.pointer_down(point, &mut self.outer) {
                        self.cancel_toolbar_input();
                        self.drag_split = Some(2);
                        return Ok(true);
                    }
                    if others && self.inner_view.pointer_down(point, &mut self.inner) {
                        self.cancel_toolbar_input();
                        self.drag_split = Some(3);
                        return Ok(true);
                    }
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } if self.drag_split == Some(2) || self.drag_split == Some(3) => {
                    self.move_presentation_split(point)?;
                    return Ok(true);
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } if self.drag_split == Some(2) || self.drag_split == Some(3) => {
                    self.outer_view.pointer_up(&mut self.outer);
                    self.inner_view.pointer_up(&mut self.inner);
                    self.drag_split = None;
                    return Ok(true);
                }
                _ => {}
            }
        }
        if self.presentation_layout.tabs.is_none() {
            return Ok(false);
        }
        if matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down {..}) && self.presentation_layout.tabs.is_some_and(|frame| frame.contains(Point::new(mouse.position.x as f32, mouse.position.y as f32))))
            && self.composition_in_progress()
        {
            eprintln!("PRESENTATION_REJECTED composition-in-progress");
            return Ok(true);
        }
        let Some(host) = &mut self.presentation_tabs else {
            return Ok(false);
        };
        if let Event::Accessibility { target, action } = event
            && let Some(publication) = w.content_accessibility_publication()
            && publication.owner(*target)
                == Some(
                    tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(
                        TABS_OWNER,
                    ),
                )
        {
            let consumed = publication
                .dispatch(
                    tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(
                        TABS_OWNER,
                    ),
                    host,
                    *target,
                    action.clone(),
                )
                .consumed;
            if consumed {
                self.apply_tab_selection()?;
            }
            return Ok(consumed);
        }
        let in_tabs = match event {
            Event::Mouse(mouse) => {
                self.pressed.is_none()
                    && (host.pointer_capture().is_some()
                        || self.presentation_layout.tabs.is_some_and(|frame| {
                            frame.contains(Point::new(
                                mouse.position.x as f32,
                                mouse.position.y as f32,
                            ))
                        }))
            }
            Event::Key(_) | Event::TextKey(_) => host.focused().is_some(),
            _ => false,
        };
        if !in_tabs {
            return Ok(false);
        }
        let Some(input) = super::window_toolbar_mount::native_toolbar_event(event) else {
            return Ok(false);
        };
        let consumed = self.presentation_tab_input(&input)?;
        Ok(consumed || matches!(event, Event::Key(_) | Event::TextKey(_)))
    }
    fn presentation_tab_input(&mut self, input: &ContentEvent) -> Result<bool> {
        if self.open_toolbar().is_some() {
            return Ok(false);
        }
        let Some(host) = &mut self.presentation_tabs else {
            return Ok(false);
        };
        let consumed = host
            .event_with_text(input, &mut self.text)
            .map_err(|e| anyhow::anyhow!("container tabs input: {e:?}"))?
            .consumed;
        if consumed {
            self.apply_tab_selection()?;
        }
        Ok(consumed)
    }
    fn apply_tab_selection(&mut self) -> Result<()> {
        let records = self
            .presentation_tabs
            .as_mut()
            .unwrap()
            .drain_action_records();
        for record in records {
            if let Some(ContentActionPayload::TabSelection { index, .. }) = record.payload {
                if self.composition_in_progress() {
                    self.presentation_selected = None;
                    eprintln!("PRESENTATION_REJECTED composition-in-progress");
                    continue;
                }
                if let Some(tab) = self.presentation_tab_keys.get(index).cloned() {
                    self.presentation.as_mut().unwrap().select_tab(tab)?;
                    self.set_toolbar_focus(None, false);
                    eprintln!(
                        "PRESENTATION_TAB selected={:?}",
                        self.presentation.as_ref().unwrap().selected_tab()
                    );
                }
            }
        }
        Ok(())
    }
    pub(super) fn presentation_tabs_publication(
        &self,
    ) -> Option<(
        tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner,
        tekes_ui_platform_macos::content_accessibility::ContentAccessibilitySnapshot,
    )> {
        use tekes_ui_platform_macos::content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot,
        };
        self.presentation_layout.tabs?;
        Some((
            ContentAccessibilityOwner(TABS_OWNER),
            ContentAccessibilitySnapshot::from_host(self.presentation_tabs.as_ref()?)?,
        ))
    }
    pub(super) fn sync_container_toolbars(
        &mut self,
        active: bool,
        whole: Rect,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
    ) -> Result<()> {
        let input_open = self
            .navigation
            .as_ref()
            .is_none_or(|nav| nav.host.input_open && !nav.busy && nav.error.is_none());
        let mut declarations = vec![Vec::new(); self.panels.len()];
        for (index, panel) in self.panels.iter().enumerate() {
            let visible = panel.bounds.size.width > 0. && panel.bounds.size.height > 0.;
            if visible && let Some(title) = &panel.options.title {
                declarations[index].push(Declaration {
                    id: ComponentId(2_000_000 + index as u64 * 10),
                    region: ComponentId(91000 + index as u64),
                    placement: ToolbarPlacement::Navigation,
                    order: 0,
                    contents: Contents::Title(title.clone()),
                    source: Source::Title,
                });
            }
            let mut targets: std::collections::BTreeMap<usize, Vec<ToolbarButtonDeclaration>> =
                std::collections::BTreeMap::new();
            for action in &panel.toolbar {
                let target = if visible {
                    Some(index)
                } else {
                    panel
                        .options
                        .retained_buttons
                        .iter()
                        .find(|retained| retained.command == action.id)
                        .and_then(|retained| {
                            self.panels.iter().position(|target| {
                                target.module_id == retained.target
                                    && target.bounds.size.width > 0.
                                    && target.bounds.size.height > 0.
                                    && (!target.toolbar.is_empty()
                                        || target.options.title.is_some())
                            })
                        })
                };
                if let Some(target) = target {
                    targets
                        .entry(target)
                        .or_default()
                        .push(ToolbarButtonDeclaration {
                            id: action.id,
                            icon: None,
                            text: action.title.clone(),
                            hint: Some(action.title.clone()),
                            enabled: input_open
                                && action.enabled
                                && panel.failed.is_none()
                                && (!action.geometry_dependent
                                    || visible && panel.frame_viewport == panel.viewport),
                        });
                }
            }
            for (target, buttons) in targets {
                declarations[target].push(Declaration {
                    id: ComponentId(92400 + index as u64 * 10),
                    region: ComponentId(91000 + target as u64),
                    placement: ToolbarPlacement::PrimaryAction,
                    order: index as i32,
                    contents: Contents::Commands(buttons, ToolbarGroupOverflow::Split),
                    source: Source::Panel {
                        instance: panel.instance,
                        index,
                        module: panel.module_id.clone(),
                        pid: panel.pid,
                        generation: panel.toolbar_generation,
                    },
                });
            }
        }
        if let Some(nav) = &self.navigation
            && let Some(source) = crate::toolbar_source::ComponentSource::current(&nav.process)
            && let Some(id) = self.presentation.as_ref().unwrap().navigation_toolbar_id()
            && let Some(index) = self.panels.iter().position(|panel| panel.module_id == id)
            && self.panels[index].bounds.size.width > 0.
            && self.panels[index].bounds.size.height > 0.
        {
            let mut buttons = Vec::new();
            for action in &nav.actions {
                let icon = action
                    .icon
                    .as_deref()
                    .map(|name| {
                        crate::window_toolbar::icon_image(
                            &mut self.toolbar_images,
                            self.renderer.as_mut().unwrap(),
                            name,
                            self.text.scale * self.text.type_scale(),
                        )
                    })
                    .transpose()?;
                buttons.push(ToolbarButtonDeclaration {
                    id: action.command.id,
                    icon,
                    text: action
                        .accessible_name
                        .clone()
                        .unwrap_or_else(|| action.command.title.clone()),
                    hint: action.tooltip.clone(),
                    enabled: nav.host.input_open
                        && !nav.busy
                        && nav.error.is_none()
                        && action.command.enabled,
                });
            }
            declarations[index].push(Declaration {
                id: ComponentId(92100),
                region: ComponentId(91000 + index as u64),
                placement: ToolbarPlacement::Navigation,
                order: 10,
                contents: Contents::Commands(buttons, ToolbarGroupOverflow::Together),
                source: Source::Navigation(source),
            });
        }
        // Revoke moved declarations in their old original hosts before binding
        // another presentation. No stale release can invoke the new location.
        for (index, panel) in self.panels.iter_mut().enumerate() {
            let keep: Vec<_> = declarations[index].iter().map(|d| d.id).collect();
            panel.mounted_toolbar.retain_groups(&keep)?;
        }
        for (index, panel) in self.panels.iter_mut().enumerate() {
            let native_row =
                panel.bounds.min_y() == whole.min_y() && panel.clip.min_y() == whole.min_y();
            let toolbar = Rect::new(
                panel.bounds.min_x(),
                panel.bounds.min_y() + if native_row { band.y } else { 0. },
                panel.bounds.size.width,
                if native_row {
                    band.height.min(panel.toolbar_height)
                } else {
                    panel.toolbar_height
                },
            );
            let top = toolbar.min_y().max(panel.clip.min_y());
            let bottom = toolbar.max_y().min(panel.clip.max_y());
            let visible = bottom > top && toolbar.size.width > 0.;
            let viewport = if visible {
                Rect::new(
                    whole.min_x(),
                    top,
                    whole.size.width,
                    (whole.max_y() - top).max(0.),
                )
            } else {
                Rect::default()
            };
            panel.mounted_toolbar.sync(
                viewport,
                if visible {
                    let mut region = ToolbarRegion::new(ComponentId(91000 + index as u64), toolbar);
                    if toolbar.min_y().max(panel.clip.min_y()) < whole.min_y() + native_height {
                        region.native_safe_area.left = (whole.min_x() + band.native_leading
                            - toolbar.min_x())
                        .clamp(0., toolbar.size.width);
                    }
                    vec![region]
                } else {
                    Vec::new()
                },
                if visible {
                    std::mem::take(&mut declarations[index])
                } else {
                    Vec::new()
                },
            )?;
            panel.mounted_toolbar.host.set_window_active(active);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, LocalContainer, OthersOptions, PresentationOptions};
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_controls::composition::AccessibilityAction;
    fn local(id: &str, command: u64, calls: Rc<Cell<u32>>) -> LocalContainer {
        LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
            .unwrap()
            .toolbar(vec![ToolbarItem {
                id: command,
                title: format!("Run {id}"),
                enabled: true,
                geometry_dependent: false,
            }])
            .on_command(move |_, _| {
                calls.set(calls.get() + 1);
                Ok(())
            })
    }
    fn fixture() -> (Host, Rc<Cell<u32>>) {
        let calls = Rc::new(Cell::new(0));
        let members: Vec<Container> = vec![
            local("focus", 1, Rc::new(Cell::new(0)))
                .title("Focus")
                .into(),
            local("other", 2, calls.clone())
                .retain_button(2, "focus")
                .into(),
        ];
        let mut options = OthersOptions::new(vec!["other".into()]);
        options.visible = false;
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(PresentationOptions::new("focus").others(options)),
        );
        assert!(host.mount_columns());
        host.panels[0].bounds = Rect::new(0., 0., 600., 300.);
        host.panels[0].clip = host.panels[0].bounds;
        host.panels[0].toolbar_height = 48.;
        (host, calls)
    }
    fn paint(host: &mut Host) {
        host.sync_container_toolbars(true, Rect::new(0., 0., 600., 600.), test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            panel
                .mounted_toolbar
                .host
                .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
                .unwrap();
        }
    }
    #[test]
    fn narrow_focus_tabs_and_internal_rows_exclude_actual_native_controls() {
        let reserved = Rect::new(0., 0., 100., 56.);
        let tabs = avoid_native_controls(Rect::new(9., 0., 591., 36.), reserved);
        assert_eq!(tabs, Rect::new(100., 0., 500., 36.));
        assert!(
            !tabs.contains(Point::new(12., 12.)),
            "paint/hit/AX use this same frame"
        );
        let below = Rect::new(9., 60., 591., 36.);
        assert_eq!(avoid_native_controls(below, reserved), below);
        let (mut host, _) = fixture();
        host.panels[0].bounds = Rect::new(9., 36., 591., 250.);
        host.panels[0].clip = host.panels[0].bounds;
        host.panels[0].toolbar_height = 48.;
        host.sync_container_toolbars(
            true,
            Rect::new(0., 0., 600., 600.),
            window_toolbar_mount::ToolbarBand {
                y: 4.,
                height: 48.,
                native_leading: 100.,
            },
            56.,
        )
        .unwrap();
        host.panels[0]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
        for (_, part) in host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
        {
            assert!(part.frame.min_x() >= 100.);
        }
    }
    #[test]
    fn horizontal_split_deadzone_and_real_drag_keep_model_and_rendered_ratio_equal() {
        let (mut host, _) = fixture();
        host.presentation.as_mut().unwrap().toggle_others().unwrap();
        let area = Rect::new(0., 0., 600., 600.);
        let layout = host.presentation.as_ref().unwrap().layout(area);
        host.apply_presentation_layout(layout).unwrap();
        host.paint_presentation(&mut Scene::new(), area, Theme::LIGHT.into())
            .unwrap();
        let divider = host.inner_view.layout().divider.frame();
        let down = divider.center();
        assert!(host.inner_view.pointer_down(down, &mut host.inner));
        host.drag_split = Some(3);
        let original = host.presentation.as_ref().unwrap().layout(area).frames;
        host.move_presentation_split(Point::new(down.x + 1., down.y))
            .unwrap();
        assert_eq!(
            host.presentation.as_ref().unwrap().layout(area).frames,
            original
        );
        host.move_presentation_split(Point::new(down.x + 60., down.y))
            .unwrap();
        let frame = host.presentation.as_ref().unwrap().layout(area).frames[0].1;
        assert!((frame.size.width - host.inner.ratio() * (area.size.width - 6.)).abs() < 0.01);
        assert!(frame.size.width > original[0].1.size.width + 40.);
        assert!(
            host.presentation_resize_regions()
                .iter()
                .all(|(_, axis)| *axis == Axis::Horizontal)
        );
    }
    #[test]
    fn nonselected_and_list_member_swap_preserves_original_hosts_and_toolbar_routes() {
        let counts = [
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
        ];
        let members = ["focus", "first", "second"]
            .iter()
            .enumerate()
            .map(|(index, id)| {
                local(id, index as u64 + 1, counts[index].clone())
                    .list_height(240.)
                    .into()
            })
            .collect();
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .others(OthersOptions::new(vec!["first".into(), "second".into()])),
            ),
        );
        host.mount_columns();
        let original: Vec<_> = host
            .panels
            .iter_mut()
            .map(|panel| {
                let mut address = 0;
                panel
                    .local
                    .as_mut()
                    .unwrap()
                    .with_host(&mut |content| address = content as *mut ContentHost as usize);
                (panel.module_id.clone(), panel.instance, address)
            })
            .collect();
        host.presentation
            .as_mut()
            .unwrap()
            .promote("second")
            .unwrap();
        assert_eq!(
            host.presentation.as_ref().unwrap().others_ids(),
            &["first", "focus"]
        );
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("first".into()))
        );
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::List)
            .unwrap();
        host.presentation
            .as_mut()
            .unwrap()
            .promote("first")
            .unwrap();
        let area = Rect::new(0., 0., 900., 600.);
        let layout = host.presentation.as_ref().unwrap().layout(area);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (panel, frame) in host.panels.iter_mut().zip(frames) {
            panel.bounds = frame;
            panel.toolbar_height = 48.;
        }
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for (index, panel) in host.panels.iter_mut().enumerate() {
            let mut address = 0;
            panel
                .local
                .as_mut()
                .unwrap()
                .with_host(&mut |content| address = content as *mut ContentHost as usize);
            assert_eq!(
                (panel.module_id.clone(), panel.instance, address),
                original[index]
            );
            panel
                .mounted_toolbar
                .host
                .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
                .unwrap();
        }
        let point = host.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run first")
            .unwrap()
            .1
            .frame
            .center();
        let target = host.toolbar_target_at(point).unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
            .unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(
            [counts[0].get(), counts[1].get(), counts[2].get()],
            [0, 1, 0]
        );
    }
    #[test]
    fn native_top_toolbar_uses_original_row_center_and_excludes_traffic_lights() {
        let (mut host, _) = fixture();
        let area = Rect::new(0., 0., 600., 600.);
        host.panels[0].bounds = area;
        host.panels[0].clip = area;
        host.panels[0].toolbar_height = 56.;
        host.sync_container_toolbars(
            true,
            area,
            window_toolbar_mount::ToolbarBand {
                y: 4.,
                height: 48.,
                native_leading: 100.,
            },
            40.,
        )
        .unwrap();
        host.panels[0]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
        let parts: Vec<_> = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .collect();
        assert!(!parts.is_empty());
        for (_, part) in parts {
            assert!(
                part.frame.min_x() >= 100.,
                "native reserved area must not contain an engine hit target: {:?}",
                part.frame
            );
            assert!(
                part.frame.min_y() >= 4. && part.frame.max_y() <= 52.,
                "toolbar stays on original native row: {:?}",
                part.frame
            );
        }
    }
    #[test]
    fn hidden_retained_button_is_presented_once_and_calls_original_local_owner() {
        let (mut host, calls) = fixture();
        paint(&mut host);
        assert!(
            host.panels[1]
                .mounted_toolbar
                .host
                .semantics()
                .iter()
                .all(|node| node.parts.is_empty())
        );
        let target = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap()
            .0;
        assert!(
            host.panels[0]
                .mounted_toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.toolbar_index = 0;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 1);
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 1);
        assert!(host.panels[0].mounted_toolbar.window != host.panels[1].mounted_toolbar.window);
    }
    #[test]
    fn retained_move_revokes_old_press_and_old_ax_target_before_rebinding() {
        let (mut host, calls) = fixture();
        paint(&mut host);
        let (target, part) = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap();
        let old_point = part.frame.center();
        host.panels[0]
            .mounted_toolbar
            .host
            .event(&ContentEvent::PointerDown(old_point));
        host.panels[1].bounds = Rect::new(0., 320., 600., 200.);
        host.panels[1].clip = host.panels[1].bounds;
        host.panels[1].toolbar_height = 48.;
        paint(&mut host);
        assert!(
            !host.panels[0]
                .mounted_toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.panels[0]
            .mounted_toolbar
            .host
            .event(&ContentEvent::PointerUp(old_point));
        host.toolbar_index = 0;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 0);
        let target = host.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap()
            .0;
        assert!(
            host.panels[1]
                .mounted_toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.toolbar_index = 1;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 1);
    }
    #[test]
    fn retained_target_without_toolbar_is_not_invented_and_disabled_owner_cannot_run() {
        let (mut host, calls) = fixture();
        host.panels[0].toolbar.clear();
        host.panels[0].options.title = None;
        host.panels[0].toolbar_height = 0.;
        paint(&mut host);
        assert!(host.panels.iter().all(|panel| {
            panel
                .mounted_toolbar
                .host
                .semantics()
                .iter()
                .all(|node| node.parts.is_empty())
        }));
        assert_eq!(host.container_toolbar_height(0, 99.), 0.);
        host.panels[0].options.title = Some("Host".into());
        host.panels[0].toolbar_height = 48.;
        host.panels[1].toolbar[0].enabled = false;
        paint(&mut host);
        let target = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap()
            .0;
        host.panels[0]
            .mounted_toolbar
            .host
            .accessibility_action(target, AccessibilityAction::Press);
        host.toolbar_index = 0;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 0);
    }
}

#[cfg(test)]
mod input_regressions {
    use super::*;
    use crate::{Container, LocalContainer, OthersOptions, PresentationOptions};
    use std::{cell::RefCell, rc::Rc};
    use tekes_ui_controls::composition::{AccessibilityAction, TextFieldContent};
    fn render_layout(host: &mut Host, area: Rect) {
        let layout = host.presentation.as_ref().unwrap().layout(area);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (index, frame) in frames.into_iter().enumerate() {
            let panel = &mut host.panels[index];
            panel.bounds = frame;
            if frame.size.width > 0.
                && let Some(local) = &mut panel.local
            {
                local
                    .render(
                        &mut Scene::new(),
                        &mut host.text,
                        frame,
                        panel.clip,
                        Theme::LIGHT.into(),
                    )
                    .unwrap();
            }
        }
    }
    #[test]
    fn list_scroll_preserves_original_editor_and_logical_hide_revokes_offscreen_focus() {
        let members: Vec<Container> = vec![
            LocalContainer::new(
                "focus",
                Content::new(ComponentId(1), TextContent::new("Focus")),
            )
            .unwrap()
            .into(),
            LocalContainer::new(
                "editor",
                Content::new(ComponentId(1), TextFieldContent::new("Editor", "")),
            )
            .unwrap()
            .list_height(300.)
            .into(),
            LocalContainer::new(
                "second",
                Content::new(ComponentId(1), TextContent::new("Second")),
            )
            .unwrap()
            .list_height(300.)
            .into(),
        ];
        let mut others = OthersOptions::new(vec!["editor".into(), "second".into()]);
        others.selected = Some(OthersTab::List);
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(PresentationOptions::new("focus").others(others)),
        );
        host.mount_columns();
        let area = Rect::new(0., 0., 600., 300.);
        render_layout(&mut host, area);
        let viewport_height = host.presentation_layout.list_viewport.unwrap().size.height;
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| {
                assert!(content.focus(Some(ComponentId(1)), true));
                content.event(&ContentEvent::Preedit {
                    text: "かな".into(),
                    selection: 0..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: None,
                });
            });
        host.focused = 1;
        host.presentation
            .as_mut()
            .unwrap()
            .scroll_list(36., viewport_height);
        render_layout(&mut host, area);
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| {
                assert_eq!(content.focused(), Some(ComponentId(1)));
                let text = content.focused_text().unwrap();
                assert_eq!(text.input.marked, Some(0..2));
                assert_eq!(text.input.text, "かな");
                content.event(&ContentEvent::Unmark);
            });
        host.presentation
            .as_mut()
            .unwrap()
            .scroll_list(400., viewport_height);
        render_layout(&mut host, area);
        assert_eq!(host.panels[1].bounds, Rect::default());
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| assert_eq!(content.focused(), Some(ComponentId(1))));
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("second".into()))
            .unwrap();
        render_layout(&mut host, area);
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| assert_eq!(content.focused(), None));
    }
    #[test]
    fn two_retained_targets_route_pointer_and_keyboard_to_the_same_original_owner() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let owner_calls = calls.clone();
        let toolbar = |id: u64, title: &str| ToolbarItem {
            id,
            title: title.into(),
            enabled: true,
            geometry_dependent: false,
        };
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
        };
        let members: Vec<Container> = vec![
            local("a").title("A").into(),
            local("b").title("B").into(),
            local("owner")
                .toolbar(vec![toolbar(10, "To A"), toolbar(11, "To B")])
                .retain_button(10, "a")
                .retain_button(11, "b")
                .on_command(move |_, command| {
                    owner_calls.borrow_mut().push(command);
                    Ok(())
                })
                .into(),
        ];
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(PresentationOptions::new("a").others(OthersOptions::new(vec!["b".into()]))),
        );
        host.mount_columns();
        let area = Rect::new(0., 0., 700., 700.);
        render_layout(&mut host, area);
        for panel in &mut host.panels {
            panel.toolbar_height = 48.;
        }
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            panel
                .mounted_toolbar
                .host
                .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
                .unwrap();
        }
        let point = host.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "To B")
            .unwrap()
            .1
            .frame
            .center();
        let target = host.toolbar_target_at(point).unwrap();
        assert_eq!(
            target,
            ToolbarTarget::Retained {
                owner: 2,
                presenter: 1
            }
        );
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
            .unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(*calls.borrow(), vec![11]);
        let first = ToolbarTarget::Retained {
            owner: 2,
            presenter: 0,
        };
        host.set_toolbar_focus(Some(first), true);
        assert_eq!(host.toolbar_index, 0);
        host.toolbar_key_input(ContentKeyEvent {
            key: Some(ControlKey::Space),
            down: true,
            repeat: false,
            modifiers: Default::default(),
            native_code: 49,
            characters: " ".into(),
            characters_ignoring_modifiers: " ".into(),
        })
        .unwrap();
        host.toolbar_key_input(ContentKeyEvent {
            key: Some(ControlKey::Space),
            down: false,
            repeat: false,
            modifiers: Default::default(),
            native_code: 49,
            characters: " ".into(),
            characters_ignoring_modifiers: " ".into(),
        })
        .unwrap();
        assert_eq!(*calls.borrow(), vec![11, 10]);
    }
    #[test]
    fn original_overflow_blocks_background_tab_pointer_until_its_own_dismissal() {
        let member = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
        };
        let members: Vec<Container> = vec![
            member("focus")
                .title("Focus")
                .toolbar(vec![ToolbarItem {
                    id: 1,
                    title: "Very long command that requires overflow".into(),
                    enabled: true,
                    geometry_dependent: false,
                }])
                .into(),
            member("a").into(),
            member("b").into(),
        ];
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .others(OthersOptions::new(vec!["a".into(), "b".into()])),
            ),
        );
        host.mount_columns();
        let area = Rect::new(0., 0., 180., 700.);
        render_layout(&mut host, area);
        host.panels[0].toolbar_height = 48.;
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            panel
                .mounted_toolbar
                .host
                .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
                .unwrap();
        }
        host.paint_presentation(&mut Scene::new(), area, Theme::LIGHT.into())
            .unwrap();
        let overflow = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| {
                part.semantics.role == tekes_ui_controls::composition::ContentRole::MenuButton
            })
            .unwrap()
            .0;
        host.panels[0]
            .mounted_toolbar
            .host
            .accessibility_action(overflow, AccessibilityAction::Press);
        host.panels[0]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
        assert!(host.open_toolbar().is_some());
        let point = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|part| part.semantics.name == "b")
            .unwrap()
            .frame
            .center();
        let selected = host.presentation.as_ref().unwrap().selected_tab().cloned();
        assert!(
            !host
                .presentation_tab_input(&ContentEvent::PointerDown(point))
                .unwrap()
        );
        assert!(
            !host
                .presentation_tab_input(&ContentEvent::PointerUp(point))
                .unwrap()
        );
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab().cloned(),
            selected
        );
        assert!(host.open_toolbar().is_some());
    }
}
