//! Window presentation routes stable container identities. Each Panel retains
//! its original ContentHost/process and its independently owned Toolbar.
use super::*;
use crate::{
    OthersTab,
    window_toolbar::{Contents, Declaration, Source},
};
use tekes_ui_controls::composition::{
    ContentActionPayload, ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement,
    ToolbarRegion, ToolbarWindowEdge,
};
use tekes_ui_platform_macos::{CommandId, Menu, MenuCommand, MenuItem};
const COMMAND_BASE: u32 = 0x4300_0000;
const WINDOW_LEADING: ComponentId = ComponentId(u64::MAX - 16);
const WINDOW_TRAILING: ComponentId = ComponentId(u64::MAX - 17);
const WINDOW_BUSINESS: ComponentId = ComponentId(u64::MAX - 18);
const WINDOW_NAV_BUSINESS: ComponentId = ComponentId(u64::MAX - 19);
const WINDOW_OTHERS_BUSINESS: ComponentId = ComponentId(u64::MAX - 20);
const TABS_OWNER: u64 = u64::MAX - 1;
#[cfg(test)]
use tekes_ui_controls::composition::{Content, TextContent, ToolbarHost};
#[cfg(test)]
fn test_band() -> window_toolbar_mount::ToolbarBand {
    window_toolbar_mount::ToolbarBand {
        y: 0.,
        height: 48.,
        native_leading: 0.,
    }
}
#[cfg(test)]
pub(super) fn render_test_tabs<B: Clone + std::fmt::Debug>(
    host: &mut ToolbarHost<B>,
    scene: &mut Scene,
    text: &mut Text,
    palette: Palette,
) -> Result<()> {
    with_test_icons(text, |text| host.render(scene, text, palette))
}
#[cfg(test)]
pub(super) fn render_test_tabs_layers<B: Clone + std::fmt::Debug>(
    host: &mut ToolbarHost<B>,
    scene: &mut Scene,
    overlays: &mut Scene,
    text: &mut Text,
    palette: Palette,
) -> Result<()> {
    with_test_icons(text, |text| {
        host.render_layers(scene, overlays, text, palette)
    })
}
#[cfg(test)]
fn with_test_icons(
    text: &mut Text,
    render: impl FnOnce(
        &mut dyn tekes_ui_controls::ButtonTextRenderer,
    )
        -> std::result::Result<(), tekes_ui_controls::composition::CompositionError>,
) -> Result<()> {
    struct Icons;
    impl tekes_ui_controls::layout_view::IconRenderer for Icons {
        fn prepare_icon(
            &mut self,
            name: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> std::result::Result<
            tekes_ui_controls::composition::IconImage,
            tekes_ui_controls::composition::IconRenderError,
        > {
            assert!(tekes_ui_icons::Glyph::named(name).is_some());
            Ok(tekes_ui_controls::composition::IconImage::new(
                ImageId(99),
                std::rc::Rc::new(()),
            ))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("Tabs must retain a prepared icon lease");
        }
    }
    render(&mut tekes_ui_controls::composition::WithIconRenderer::new(
        text, &mut Icons,
    ))
    .map_err(|e| anyhow::anyhow!("test container tabs toolbar: {e:?}"))
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
fn reserve_window_edges(region: &mut ToolbarRegion, edges: [Rect; 2]) {
    let frame = region.available;
    let mut left = frame.min_x() + region.native_safe_area.left;
    let mut right = frame.max_x() - region.native_safe_area.right;
    for (index, edge) in edges.into_iter().enumerate() {
        if edge.size.width <= 0. || edge.min_y() >= frame.max_y() || edge.max_y() <= frame.min_y() {
            continue;
        }
        if index == 0 {
            left = left.max(edge.max_x()).min(right);
        } else {
            right = right.min(edge.min_x()).max(left);
        }
    }
    region.native_safe_area.left = (left - frame.min_x()).max(0.);
    region.native_safe_area.right = (frame.max_x() - right).max(0.);
}
impl Host {
    pub(super) fn presentation_menu(&self) -> Menu {
        let mut entries = Vec::new();
        if self
            .presentation
            .as_ref()
            .is_some_and(|state| state.nav_bar_id().is_some())
        {
            entries.push(MenuItem::Command(MenuCommand::new(
                "Show / Hide NavBar",
                CommandId(COMMAND_BASE - 2),
            )));
        }

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
    pub(super) fn presentation_frames(
        &mut self,
        whole: Rect,
        w: &Window,
        toolbar_height: f32,
        band: window_toolbar_mount::ToolbarBand,
    ) -> Result<Vec<Rect>> {
        let (native_height, leading) = w.titlebar_safe_area();
        let mut layout = self
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, toolbar_height);
        layout.tabs = layout.tabs.map(|frame| {
            avoid_native_controls(
                Rect::new(frame.min_x(), band.y, frame.size.width, band.height),
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
    pub(super) fn apply_presentation_layout(
        &mut self,
        layout: crate::PresentationLayout,
    ) -> Result<Vec<Rect>> {
        let state = self.presentation.as_ref().unwrap();
        let mut logical: Vec<String> = if state.others_expanded() {
            Vec::new()
        } else {
            vec![state.focus_id().into()]
        };
        if let Some(nav) = state.visible_nav_bar_id() {
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
            panel.bounds = frame;
            frames[index] = frame;
        }
        if layout.tabs.is_none()
            && let Some(tabs) = &mut self.presentation_tabs
        {
            tabs.blur();
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
        let focus = self
            .presentation_layout
            .frames
            .iter()
            .find(|(id, _)| id == self.presentation.as_ref().unwrap().focus_id())
            .map(|(_, frame)| *frame)
            .unwrap();
        let area = Rect::new(
            whole.min_x(),
            self.container_window_boundary.unwrap_or(focus.min_y()),
            whole.size.width,
            (whole.max_y() - self.container_window_boundary.unwrap_or(focus.min_y())).max(0.),
        );
        let mut right = area;
        self.outer.min_pane = MIN_COLUMN;
        self.inner.min_pane = MIN_COLUMN;
        self.outer.divider = 1.;
        self.inner.divider = 1.;
        if let Some(nav) = self.presentation.as_ref().unwrap().visible_nav_bar_id()
            && let Some((_, frame)) = self
                .presentation_layout
                .frames
                .iter()
                .find(|(id, _)| id == nav)
        {
            self.outer
                .set_ratio(frame.size.width / (area.size.width - 1.).max(1.));
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
        if self.presentation.as_ref().unwrap().others_visible()
            && !self.presentation.as_ref().unwrap().others_expanded()
        {
            self.inner
                .set_ratio(focus.size.width / (right.size.width - 1.).max(1.));
            let layout = self.inner_view.arrange(&mut self.inner, right, None);
            self.inner_view.paint_presented_divider(
                scene,
                &self.inner,
                layout.divider.frame(),
                palette,
                self.text.scale,
            );
        }
        self.sync_presentation_switcher(whole)?;
        let switcher_frame = self.presentation_switcher_frame();
        let switcher_regions =
            switcher_frame.map(|frame| self.presentation_switcher_regions(frame, whole));
        if let (Some(host), Some(_frame)) = (&mut self.presentation_tabs, switcher_frame) {
            host.set_regions(TABS_OWNER, whole, switcher_regions.unwrap())
                .map_err(|e| anyhow::anyhow!("container tabs region: {e:?}"))?;
            if let Some(renderer) = &mut self.renderer {
                with_content_icons(
                    renderer,
                    &mut self.presentation_icons,
                    &mut self.text,
                    |text| {
                        host.render(scene, text, palette)
                            .map_err(|e| anyhow::anyhow!("container tabs paint: {e:?}"))
                    },
                )?;
            } else {
                #[cfg(test)]
                render_test_tabs(host, scene, &mut self.text, palette)?;
                #[cfg(not(test))]
                host.render(scene, &mut self.text, palette)
                    .map_err(|e| anyhow::anyhow!("container tabs paint: {e:?}"))?;
            }
        }
        self.trace_presentation_scroll("after-paint");
        Ok(())
    }
    pub(super) fn presentation_resize_regions(&self) -> Vec<(Rect, Axis)> {
        let mut regions = Vec::new();
        if self
            .presentation
            .as_ref()
            .is_some_and(|state| state.nav_bar_visible())
        {
            regions.extend(self.outer_view.resize_regions(&self.outer));
        }
        if self
            .presentation
            .as_ref()
            .is_some_and(|state| state.others_visible() && !state.others_expanded())
        {
            regions.extend(self.inner_view.resize_regions(&self.inner));
        }
        regions
    }
    pub(super) fn toggle_others_expansion(&mut self, id: &str, expanded: bool) -> Result<bool> {
        if self.composition_in_progress() {
            return Ok(false);
        }
        let Some(state) = &mut self.presentation else {
            return Ok(false);
        };
        if state.selected_tab() != Some(&OthersTab::Container(id.into()))
            || !state.others_visible()
            || state.others_expanded() != expanded
        {
            return Ok(false);
        }
        state.toggle_others_expanded()?;
        self.focused = self
            .panels
            .iter()
            .position(|panel| panel.module_id == id)
            .unwrap();
        self.cancel_toolbar_input();
        if let Some(header) = &mut self.presentation_tabs {
            header.event(&ContentEvent::Cancel);
        }
        self.drag_split = None;
        self.outer_view.cancel(&mut self.outer);
        self.inner_view.cancel(&mut self.inner);
        Ok(true)
    }
    pub(super) fn switch_workspace_focus(
        &mut self,
        focus: u64,
        other: Option<u64>,
    ) -> Result<bool> {
        let current = self
            .presentation
            .as_ref()
            .filter(|state| state.workspace_switch_enabled() && state.others_visible());
        let focus_index = current.and_then(|state| {
            self.panels
                .iter()
                .position(|panel| panel.module_id == state.focus_id() && panel.instance == focus)
        });
        let target = current
            .and_then(|state| state.focus_switch_target(state.focus_id()))
            .and_then(|id| self.panels.iter().find(|panel| panel.module_id == id));
        if self
            .navigation
            .as_ref()
            .is_none_or(|nav| nav.host.input_open && !nav.busy && nav.error.is_none())
            && other.is_some()
            && target.map(|panel| panel.instance) == other
            && let Some(index) = focus_index
        {
            self.switch_container_focus(index)
        } else {
            Ok(false)
        }
    }
    pub(super) fn switch_container_focus(&mut self, index: usize) -> Result<bool> {
        if self.composition_in_progress() {
            eprintln!("PRESENTATION_REJECTED composition-in-progress");
            return Ok(false);
        }
        let owner = &self.panels[index].module_id;
        let Some(target) = self
            .presentation
            .as_ref()
            .and_then(|state| state.focus_switch_target(owner))
            .map(str::to_owned)
        else {
            return Ok(false);
        };
        self.presentation.as_mut().unwrap().promote(&target)?;
        eprintln!(
            "PRESENTATION_SWITCH owner={owner} target={target} focus={}",
            self.presentation.as_ref().unwrap().focus_id()
        );
        self.cancel_toolbar_input();
        Ok(true)
    }
    fn move_presentation_split(&mut self, point: Point) -> Result<()> {
        if self.drag_split == Some(2) {
            if !self.outer_view.pointer_move(point, &mut self.outer) {
                return Ok(());
            }
            let area = self.outer_view.layout().frame.frame();
            self.presentation.as_mut().unwrap().resize_nav_width(
                (area.size.width - self.outer.divider).max(0.) * self.outer.ratio(),
                area,
                MIN_COLUMN,
            );
        } else {
            if !self.inner_view.pointer_move(point, &mut self.inner) {
                return Ok(());
            }
            let visible = self.inner_view.layout().frame.frame();
            let whole = Rect::new(0., visible.min_y(), visible.max_x(), visible.size.height);
            self.presentation.as_mut().unwrap().resize_focus_ratio(
                self.inner.ratio().clamp(0.01, 0.99),
                whole,
                MIN_COLUMN,
            )?;
        }
        Ok(())
    }
    pub(super) fn composition_in_progress(&mut self) -> bool {
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
        if let Event::Scroll(scroll) = event
            && std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_some()
        {
            let point = Point::new(scroll.position.x as f32, scroll.position.y as f32);
            eprintln!(
                "OTHERS_SCROLL_ROUTE point={point:?} delta={:?} tabs={:?} in_tabs={} list={:?} pressed={} toolbar_overlay={}",
                scroll.delta,
                self.presentation_layout.tabs,
                self.presentation_layout
                    .tabs
                    .is_some_and(|frame| frame.contains(point)),
                self.presentation_layout.list_viewport,
                self.pressed.is_some(),
                self.open_toolbar().is_some()
            );
            self.trace_presentation_scroll("host-route");
        }
        if self.presentation.is_none() {
            return Ok(false);
        }
        if let Event::Command(CommandId(command)) = event
            && *command >= COMMAND_BASE - 2
            && *command <= COMMAND_BASE + self.module_ids.len() as u32 * 6
        {
            if self.composition_in_progress() {
                eprintln!("PRESENTATION_REJECTED composition-in-progress");
                return Ok(true);
            }
            if *command > COMMAND_BASE {
                let value = (*command - COMMAND_BASE - 1) as usize;
                if value % 6 == 3 {
                    let id = self.module_ids[value / 6].clone();
                    if let Err(error) = self.close_container(&id) {
                        eprintln!("PRESENTATION_REJECTED {error:#}");
                    }
                    return Ok(true);
                }
            }
            let state = self.presentation.as_mut().unwrap();
            let result = if *command == COMMAND_BASE - 2 {
                state.toggle_nav_bar()
            } else if *command == COMMAND_BASE {
                state.toggle_others()
            } else if *command == COMMAND_BASE - 1 {
                state.select_tab(OthersTab::List)
            } else {
                let value = (*command - COMMAND_BASE - 1) as usize;
                let index = value / 6;
                let id = &self.module_ids[index];
                match value % 6 {
                    0 => state.select_tab(if state.list_ids().contains(&id.as_str()) {
                        OthersTab::List
                    } else {
                        OthersTab::Container(id.clone())
                    }),
                    1 => state.promote(id),
                    2 => state.open(id),
                    3 => unreachable!("close handled through Host lifecycle"),
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
        let switcher_modal = self
            .presentation_tabs
            .as_ref()
            .is_some_and(|host| host.content_host().input_overlay_owner().is_some());
        if switcher_modal {
            if let Some(input) = super::window_toolbar_mount::native_toolbar_event(event) {
                self.presentation_tab_input(&input)?;
                return Ok(true);
            }
            if let Event::Accessibility { target, .. } = event {
                let owned = w.content_accessibility_publication().is_some_and(|publication| publication.owner(*target) == Some(tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(TABS_OWNER)));
                if !owned {
                    return Ok(true);
                }
            }
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
            self.presentation.as_mut().unwrap().scroll_list_wheel(
                -scroll.delta.y as f32,
                scroll.precise,
                height,
            );
            return Ok(true);
        }
        if let Event::Mouse(mouse) = event {
            let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
            match mouse.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } if self.pressed.is_none() && self.open_toolbar().is_none() => {
                    let nav = self.presentation.as_ref().unwrap().nav_bar_visible();
                    let others = self.presentation.as_ref().unwrap().others_visible();
                    if nav && self.outer_view.pointer_down(point, &mut self.outer) {
                        self.cancel_toolbar_input();
                        self.drag_split = Some(2);
                        return Ok(true);
                    }
                    if others
                        && !self.presentation.as_ref().unwrap().others_expanded()
                        && self.inner_view.pointer_down(point, &mut self.inner)
                    {
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
        let creation_frame = self.presentation_creation_frame();
        if matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down {..}) && (self.presentation_layout.tabs.is_some_and(|frame| frame.contains(Point::new(mouse.position.x as f32, mouse.position.y as f32))) || creation_frame.is_some_and(|frame| frame.contains(Point::new(mouse.position.x as f32, mouse.position.y as f32)))))
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
            let stamped = host.semantics().into_iter().find_map(|node| {
                let id = node.node.id;
                std::iter::once((node.target, None))
                    .chain(
                        node.parts
                            .into_iter()
                            .map(|(target, part)| (target, Some(part.index))),
                    )
                    .find_map(|(stamp, part)| {
                        (tekes_ui_platform_macos::content_accessibility::content_key(
                            host.content_host(),
                            id,
                            part,
                        ) == Some(*target))
                        .then_some(stamp)
                    })
            });
            use tekes_ui_controls::composition::AccessibilityAction as Action;
            use tekes_ui_platform_macos::AccessibilityAction as Native;
            let action = match action.clone() {
                Native::Focus => Action::Focus,
                Native::Press => Action::Press,
                Native::ShowMenu => Action::ShowMenu,
                Native::Increment => Action::Increment,
                Native::Decrement => Action::Decrement,
                Native::SetValue(value) => Action::SetValue(value),
                Native::SetSelection(range) => Action::SetSelection(range),
                Native::ReplaceSelectedText(text) => Action::ReplaceSelectedText(text),
                Native::SetExpanded(value) => Action::SetExpanded(value),
            };
            let consumed =
                stamped.is_some_and(|target| host.accessibility_action(target, action).consumed);
            if consumed {
                self.apply_tab_selection()?;
            }
            return Ok(consumed);
        }
        let in_tabs = match event {
            Event::Mouse(mouse) => {
                self.pressed.is_none()
                    && (host.content_host().pointer_capture().is_some()
                        || creation_frame.is_some_and(|frame| {
                            frame.contains(Point::new(
                                mouse.position.x as f32,
                                mouse.position.y as f32,
                            ))
                        })
                        || self.presentation_layout.tabs.is_some_and(|frame| {
                            frame.contains(Point::new(
                                mouse.position.x as f32,
                                mouse.position.y as f32,
                            ))
                        }))
            }
            Event::Scroll(scroll) => self.presentation_layout.tabs.is_some_and(|frame| {
                frame.contains(Point::new(
                    scroll.position.x as f32,
                    scroll.position.y as f32,
                ))
            }),
            Event::Key(_) | Event::TextKey(_) => host.content_host().focused().is_some(),
            _ => false,
        };
        if !in_tabs {
            return Ok(false);
        }
        let Some(input) = super::window_toolbar_mount::native_toolbar_event(event) else {
            return Ok(false);
        };
        if let ContentEvent::KeyInput(key) = input {
            return self.toolbar_key_input(key);
        }
        let consumed = self.presentation_tab_input(&input)?;
        Ok(consumed || matches!(event, Event::Key(_) | Event::TextKey(_)))
    }
    pub(super) fn presentation_tab_input(&mut self, input: &ContentEvent) -> Result<bool> {
        let trace_scroll = matches!(input, ContentEvent::Scroll { .. })
            && std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_some();
        if trace_scroll {
            self.trace_presentation_scroll("before-input");
        }
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
            if host.content_host().focused().is_some() {
                self.set_toolbar_focus(None, false);
            }
            self.apply_tab_selection()?;
        }
        if trace_scroll {
            eprintln!("OTHERS_SCROLL_HEADER consumed={consumed}");
            self.trace_presentation_scroll("after-input");
        }
        Ok(consumed)
    }
    pub(super) fn apply_tab_selection(&mut self) -> Result<()> {
        let records = self.presentation_tabs.as_mut().unwrap().drain_commands();
        for record in records {
            if self.composition_in_progress() {
                eprintln!("PRESENTATION_REJECTED composition-in-progress");
                continue;
            }
            use super::window_switcher::SwitcherAction;
            match record.source {
                SwitcherAction::Create(id) => {
                    self.open_creation_choice(&id)?;
                }
                SwitcherAction::Tabs => {
                    let Some(ContentActionPayload::TabStrip(intent)) = record.payload else {
                        continue;
                    };
                    let Some(tab) = self
                        .presentation_switcher
                        .items
                        .iter()
                        .find(|item| item.key == intent.key)
                        .map(|item| item.tab.clone())
                    else {
                        continue;
                    };
                    if intent.close {
                        if let OthersTab::Container(id) = tab {
                            self.close_container(&id)?;
                        }
                    } else {
                        self.presentation.as_mut().unwrap().select_tab(tab)?;
                    }
                }
                SwitcherAction::Menu => {
                    let Some(ContentActionPayload::MenuSelection(selection)) = record.payload
                    else {
                        continue;
                    };
                    let Some(value) = selection.value_path().and_then(|path| path.last()) else {
                        continue;
                    };
                    self.open_creation_choice(value)?;
                }
            }
            self.set_toolbar_focus(None, false);
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
            ContentAccessibilitySnapshot::from_host(
                self.presentation_tabs.as_ref()?.content_host(),
            )?,
        ))
    }
    fn business_destination(
        &self,
        owner: usize,
        retained: Option<usize>,
        focus: usize,
    ) -> (usize, ComponentId) {
        let target = retained.unwrap_or(owner);
        match self.panels[target].options.toolbar_host {
            crate::ContainerToolbarHost::Window => (focus, self.window_business_region(target)),
            crate::ContainerToolbarHost::Container { .. } => {
                (target, ComponentId(91000 + target as u64))
            }
        }
    }
    fn window_business_region(&self, target: usize) -> ComponentId {
        let state = self.presentation.as_ref().unwrap();
        let id = self.panels[target].module_id.as_str();
        if state.nav_bar_id() == Some(id) {
            WINDOW_NAV_BUSINESS
        } else if state.focus_id() == id && !state.others_expanded() {
            WINDOW_BUSINESS
        } else {
            WINDOW_OTHERS_BUSINESS
        }
    }
    fn window_partition_frame(
        &self,
        region: ComponentId,
        whole: Rect,
        band: &window_toolbar_mount::ToolbarBand,
    ) -> Rect {
        let state = self.presentation.as_ref().unwrap();
        let column = if region == WINDOW_OTHERS_BUSINESS {
            self.presentation_layout.tabs.unwrap_or_default()
        } else {
            let id = if region == WINDOW_NAV_BUSINESS {
                state.visible_nav_bar_id()
            } else {
                Some(state.focus_id())
            };
            id.and_then(|id| self.panels.iter().find(|panel| panel.module_id == id))
                .map_or(Rect::default(), |panel| panel.bounds)
        };
        Rect::new(
            column.min_x().max(whole.min_x()),
            band.y,
            column.size.width,
            band.height,
        )
    }
    pub(super) fn presentation_switcher_frame(&self) -> Option<Rect> {
        let mut frame = self.presentation_layout.tabs?;
        let mut region = ToolbarRegion::new(ComponentId(TABS_OWNER), frame);
        reserve_window_edges(&mut region, self.presentation_chrome_edges);
        frame = Rect::new(
            frame.min_x() + region.native_safe_area.left,
            frame.min_y(),
            (frame.size.width - region.native_safe_area.left - region.native_safe_area.right)
                .max(0.),
            frame.size.height,
        );
        // Original mounted group measurements reserve this same column's business
        // commands. Internal toolbars below the window band do not intersect it.
        for occupied in self
            .panels
            .iter()
            .flat_map(|panel| panel.mounted_toolbar.occupied_frames())
        {
            if occupied.min_y() < frame.max_y()
                && occupied.max_y() > frame.min_y()
                && occupied.min_x() < frame.max_x()
                && occupied.max_x() > frame.min_x()
                && occupied.min_x() > frame.min_x()
            {
                frame.size.width = (occupied.min_x() - frame.min_x()).max(0.);
            }
        }
        Some(frame)
    }
    fn container_toolbar_regions(
        &self,
        index: usize,
        focus: usize,
        whole: Rect,
        band: &window_toolbar_mount::ToolbarBand,
    ) -> Vec<ToolbarRegion> {
        let panel = &self.panels[index];
        let mut regions = Vec::new();
        if panel.bounds.size.width > 0.
            && panel.bounds.size.height > 0.
            && let Some(frame) = panel.geometry.toolbar
        {
            let mut region = ToolbarRegion::new(ComponentId(91000 + index as u64), frame);
            if matches!(
                panel.options.toolbar_host,
                crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Leading
                        | crate::ContainerToolbarPosition::Trailing
                }
            ) {
                region.axis = Axis::Vertical;
            }
            regions.push(region);
        }
        if index == focus {
            let frames = [WINDOW_NAV_BUSINESS, WINDOW_BUSINESS, WINDOW_OTHERS_BUSINESS]
                .map(|id| (id, self.window_partition_frame(id, whole, band)));
            for &(id, frame) in &frames {
                let mut region = ToolbarRegion::new(id, frame);
                region.native_safe_area.left =
                    (band.native_leading - frame.min_x()).clamp(0., frame.size.width);
                reserve_window_edges(&mut region, self.presentation_chrome_edges);
                regions.push(region);
            }
            for (id, edge, column) in [
                (WINDOW_LEADING, ToolbarWindowEdge::Leading, 0),
                (WINDOW_TRAILING, ToolbarWindowEdge::Trailing, 2),
            ] {
                let frame = if frames[column].1.size.width > 0. {
                    frames[column].1
                } else {
                    frames[1].1
                };
                let mut region = ToolbarRegion::window(id, frame, edge);
                region.native_safe_area.left =
                    (band.native_leading - frame.min_x()).clamp(0., frame.size.width);
                regions.push(region);
            }
        }
        regions
    }
    fn navigation_toolbar_declaration(
        &self,
        index: usize,
        focus: usize,
        source: crate::toolbar_source::ComponentSource,
        buttons: Vec<ToolbarButtonDeclaration>,
        templates: Vec<(u64, String)>,
    ) -> (usize, Declaration) {
        let (presenter, destination) = self.business_destination(index, None, focus);
        (
            presenter,
            Declaration {
                id: ComponentId(92100),
                region: destination,
                // Others content navigation owns leading; fixed history commands
                // share this partition's trailing business space instead.
                placement: if destination == WINDOW_OTHERS_BUSINESS {
                    ToolbarPlacement::PrimaryAction
                } else {
                    ToolbarPlacement::Navigation
                },
                order: 10,
                contents: Contents::TemplateCommands(
                    buttons,
                    ToolbarGroupOverflow::Together,
                    templates,
                ),
                source: Source::Navigation(source),
            },
        )
    }
    pub(super) fn presentation_toolbar_declarations(&self) -> Vec<Vec<Declaration>> {
        self.presentation_toolbar_declarations_with_icons(self.renderer.is_some())
    }
    // Projection depends on whether the receiving text service can prepare the
    // original toolkit icons; it does not require constructing a GPU renderer.
    fn presentation_toolbar_declarations_with_icons(
        &self,
        include_icons: bool,
    ) -> Vec<Vec<Declaration>> {
        let input_open = self
            .navigation
            .as_ref()
            .is_none_or(|nav| nav.host.input_open && !nav.busy && nav.error.is_none());
        let mut declarations = vec![Vec::new(); self.panels.len()];
        let focus = self
            .panels
            .iter()
            .position(|panel| panel.module_id == self.presentation.as_ref().unwrap().focus_id())
            .unwrap();
        for (index, panel) in self.panels.iter().enumerate() {
            let visible = panel.bounds.size.width > 0. && panel.bounds.size.height > 0.;
            let in_others = self.presentation.as_ref().is_some_and(|state| {
                state.others_ids().contains(&panel.module_id)
                    || (state.others_expanded() && state.focus_id() == panel.module_id)
            });
            if visible
                && !in_others
                && matches!(
                    panel.options.toolbar_host,
                    crate::ContainerToolbarHost::Window
                )
                && let Some(title) = &panel.options.title
            {
                let (presenter, region) = self.business_destination(index, None, focus);
                declarations[presenter].push(Declaration {
                    id: ComponentId(2_000_000 + index as u64 * 10),
                    region,
                    placement: ToolbarPlacement::Navigation,
                    order: 20,
                    contents: Contents::Title(title.clone()),
                    source: Source::Title,
                });
            }
            type TargetCommands = (Vec<ToolbarButtonDeclaration>, bool, Vec<(u64, String)>);
            let mut targets: std::collections::BTreeMap<(usize, u64, bool), TargetCommands> =
                std::collections::BTreeMap::new();
            for action in &panel.toolbar {
                if self
                    .presentation
                    .as_ref()
                    .unwrap()
                    .workspace_switch_enabled()
                    && panel.options.focus_switch_command == Some(action.id)
                {
                    continue;
                }
                let secondary = panel.options.secondary_commands.contains(&action.id);
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
                    let retained_target = !visible;
                    let (target, region) =
                        self.business_destination(index, (!visible).then_some(target), focus);
                    let switch = panel.options.focus_switch_command == Some(action.id);
                    let state = self.presentation.as_ref().unwrap();
                    let switch_target = switch
                        .then(|| state.focus_switch_target(&panel.module_id))
                        .flatten();
                    let title = if switch {
                        if panel.module_id == state.focus_id() {
                            "Switch to Others"
                        } else {
                            "Switch to Focus"
                        }
                    } else {
                        action.title.as_str()
                    };
                    let (buttons, retained, icons) =
                        targets.entry((target, region.0, secondary)).or_default();
                    *retained |= retained_target;
                    if switch && include_icons {
                        icons.push((action.id, "tekes-swap-horizontal".to_owned()));
                    }
                    let hint = if switch {
                        switch_target.map_or_else(
                            || "Select an ordinary container Tab to exchange".to_owned(),
                            |id| format!("Exchange with {id}"),
                        )
                    } else {
                        action.title.clone()
                    };
                    buttons.push(ToolbarButtonDeclaration {
                        id: action.id,
                        icon: None,
                        text: title.to_owned(),
                        hint: Some(hint),
                        enabled: input_open
                            && (!switch || switch_target.is_some())
                            && action.enabled
                            && panel.failed.is_none()
                            && (!action.geometry_dependent
                                || visible && panel.frame_viewport == panel.viewport),
                    });
                }
            }
            for ((target, region, secondary), (buttons, retained, icons)) in targets {
                // Fixed owner/role identities reserve forty IDs per owner and ten per group, as the
                // original groups do. A retained Window contribution may share
                // its presenter with an internal retained group from this owner.
                let window_region = [
                    WINDOW_BUSINESS.0,
                    WINDOW_NAV_BUSINESS.0,
                    WINDOW_OTHERS_BUSINESS.0,
                ]
                .contains(&region);
                let id = if retained && window_region {
                    let role = u64::MAX - 18 - region;
                    ComponentId(4_000_000 + index as u64 * 40 + role * 10)
                } else {
                    ComponentId(92400 + index as u64 * 10)
                };
                let id = if secondary {
                    ComponentId(id.0 + 8_000_000)
                } else {
                    id
                };
                let contents = Contents::TemplateCommands(
                    buttons,
                    if secondary {
                        ToolbarGroupOverflow::Menu
                    } else {
                        ToolbarGroupOverflow::Split
                    },
                    icons,
                );
                declarations[target].push(Declaration {
                    id,
                    region: ComponentId(region),
                    placement: if secondary {
                        ToolbarPlacement::SecondaryAction
                    } else if window_region {
                        ToolbarPlacement::PrimaryAction
                    } else {
                        ToolbarPlacement::Navigation
                    },
                    order: index as i32,
                    contents,
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
        let state = self.presentation.as_ref().unwrap();
        let focus = self
            .panels
            .iter()
            .position(|panel| panel.module_id == state.focus_id())
            .unwrap();
        if state.workspace_switch_enabled() {
            let other = state
                .others_visible()
                .then(|| state.focus_switch_target(state.focus_id()))
                .flatten()
                .and_then(|id| self.panels.iter().find(|panel| panel.module_id == id));
            declarations[focus].push(Declaration {
                id: ComponentId(92040),
                region: if state.others_expanded() {
                    WINDOW_OTHERS_BUSINESS
                } else {
                    WINDOW_BUSINESS
                },
                placement: ToolbarPlacement::SecondaryAction,
                order: 100,
                contents: Contents::Commands(
                    vec![ToolbarButtonDeclaration {
                        id: 0x5604,
                        icon: None,
                        text: "Switch Focus and Others".into(),
                        hint: Some("Exchange the current Focus and Others containers".into()),
                        enabled: input_open && other.is_some(),
                    }],
                    ToolbarGroupOverflow::Menu,
                ),
                source: Source::WorkspaceSwitch {
                    focus: self.panels[focus].instance,
                    other: other.map(|panel| panel.instance),
                },
            });
        }
        if state.others_visible()
            && let Some(OthersTab::Container(id)) = state.selected_tab()
        {
            let name = if state.others_expanded() {
                "Collapse Others"
            } else {
                "Expand Others"
            };
            declarations[focus].push(Declaration {
                id: ComponentId(92020),
                region: WINDOW_TRAILING,
                placement: ToolbarPlacement::PrimaryAction,
                order: 0,
                contents: Contents::TemplateCommands(
                    vec![ToolbarButtonDeclaration {
                        id: 0x5603,
                        icon: None,
                        text: name.into(),
                        hint: Some(name.into()),
                        enabled: input_open,
                    }],
                    ToolbarGroupOverflow::Together,
                    if include_icons {
                        vec![(0x5603, "tekes-fullscreen".into())]
                    } else {
                        Vec::new()
                    },
                ),
                source: Source::Expand {
                    container: id.clone(),
                    expanded: state.others_expanded(),
                },
            });
        }
        for (side, available, visible, name) in [
            (
                0,
                state.nav_bar_id().is_some(),
                state.nav_bar_visible(),
                "NavBar",
            ),
            (
                1,
                state.others().is_some(),
                state.others_visible(),
                "Others",
            ),
        ] {
            if available {
                let glyph = match (side, visible) {
                    (0, true) => "tekes-side-bar",
                    (0, false) => "tekes-side-bar-collapsed",
                    (_, true) => "tekes-side-panel",
                    (_, false) => "tekes-side-panel-collapsed",
                };
                declarations[focus].push(Declaration {
                    id: ComponentId(92000 + side as u64 * 10),
                    region: if side == 0 {
                        WINDOW_LEADING
                    } else {
                        WINDOW_TRAILING
                    },
                    placement: if side == 0 {
                        ToolbarPlacement::Navigation
                    } else {
                        ToolbarPlacement::PrimaryAction
                    },
                    order: if side == 0 { 0 } else { 2 },
                    contents: Contents::TemplateCommands(
                        vec![ToolbarButtonDeclaration {
                            id: 0x5601 + side as u64,
                            icon: None,
                            text: format!("{} {name}", if visible { "Hide" } else { "Show" }),
                            hint: Some(format!("{} {name}", if visible { "Hide" } else { "Show" })),
                            enabled: input_open,
                        }],
                        ToolbarGroupOverflow::Together,
                        if include_icons {
                            vec![(0x5601 + side as u64, glyph.to_owned())]
                        } else {
                            Vec::new()
                        },
                    ),
                    source: Source::Visibility(side),
                });
            }
        }
        if let Some(nav) = &self.navigation
            && let Some(source) = crate::toolbar_source::ComponentSource::current(&nav.process)
            && let Some(id) = self
                .presentation
                .as_ref()
                .unwrap()
                .navigation_toolbar_presenter_id()
            && let Some(index) = self.panels.iter().position(|panel| panel.module_id == id)
            && self.panels[index].bounds.size.width > 0.
            && self.panels[index].bounds.size.height > 0.
        {
            let mut buttons = Vec::new();
            let mut templates = Vec::new();
            for action in &nav.actions {
                if let Some(name) = &action.icon {
                    templates.push((action.command.id, name.clone()));
                }
                buttons.push(ToolbarButtonDeclaration {
                    id: action.command.id,
                    icon: None,
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
            let (presenter, declaration) =
                self.navigation_toolbar_declaration(index, focus, source, buttons, templates);
            declarations[presenter].push(declaration);
        }
        declarations
    }
    pub(super) fn sync_container_toolbars(
        &mut self,
        active: bool,
        whole: Rect,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
    ) -> Result<()> {
        self.sync_container_toolbars_with_icons(
            active,
            whole,
            band,
            native_height,
            self.renderer.is_some(),
        )
    }
    fn sync_container_toolbars_with_icons(
        &mut self,
        active: bool,
        whole: Rect,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
        include_icons: bool,
    ) -> Result<()> {
        let boundary = self
            .container_window_boundary
            .unwrap_or((band.y + band.height).max(native_height));
        for index in 0..self.panels.len() {
            let bounds = self.panels[index].bounds;
            if bounds.size.width > 0. && bounds.size.height > 0. {
                let geometry = self.container_geometry(index, bounds, boundary)?;
                self.panel_geometry_layout(index, bounds, self.panels[index].scale, geometry)?;
            }
        }
        let mut declarations = self.presentation_toolbar_declarations_with_icons(include_icons);
        let focus = self
            .panels
            .iter()
            .position(|panel| panel.module_id == self.presentation.as_ref().unwrap().focus_id())
            .unwrap();
        for (index, panel) in self.panels.iter_mut().enumerate() {
            let keep: Vec<_> = declarations[index]
                .iter()
                .map(|declaration| declaration.id)
                .collect();
            panel.mounted_toolbar.retain_groups(&keep)?;
        }
        let list_members: Vec<_> = (0..self.panels.len())
            .map(|index| self.is_list_member(index))
            .collect();
        let regions: Vec<_> = (0..self.panels.len())
            .map(|index| self.container_toolbar_regions(index, focus, whole, &band))
            .collect();
        for (index, panel) in self.panels.iter_mut().enumerate() {
            panel.mounted_toolbar.sync(
                if list_members[index] {
                    panel.clip
                } else {
                    whole
                },
                regions[index].clone(),
                std::mem::take(&mut declarations[index]),
            )?;
            panel.mounted_toolbar.host.set_window_active(active);
        }
        // Window edge allocation is measured by its original host. The business
        // region then reserves those actual groups, never a second estimate.
        let palette = self.palette();
        let toolbar = &mut self.panels[focus].mounted_toolbar;
        let mut measure = Scene::new();
        if let Some(renderer) = &mut self.renderer {
            with_content_icons(renderer, &mut toolbar.icons, &mut self.text, |text| {
                toolbar
                    .host
                    .render(&mut measure, text, palette)
                    .map_err(|error| anyhow::anyhow!("window toolbar measure: {error:?}"))
            })?;
        } else {
            #[cfg(test)]
            render_test_tabs(&mut toolbar.host, &mut measure, &mut self.text, palette)?;
            #[cfg(not(test))]
            toolbar
                .host
                .render(&mut measure, &mut self.text, palette)
                .map_err(|error| anyhow::anyhow!("window toolbar measure: {error:?}"))?;
        }
        self.presentation_chrome_edges = [WINDOW_LEADING, WINDOW_TRAILING].map(|id| {
            toolbar
                .host
                .content_host()
                .get(id)
                .map_or(Rect::default(), |node| node.frame())
        });
        // A second identical sync is an original-host no-op; changed safe areas
        // retire stale geometry before any pointer release can be delivered.
        let regions = self.container_toolbar_regions(focus, focus, whole, &band);
        let toolbar = &mut self.panels[focus].mounted_toolbar;
        toolbar
            .host
            .set_regions(toolbar.window, whole, regions)
            .map_err(|error| anyhow::anyhow!("window toolbar reservation: {error:?}"))?;
        let _ = native_height;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, LocalContainer, OthersOptions, PresentationOptions};
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_controls::composition::{AccessibilityAction, TextContent};
    fn local(id: &str, command: u64, calls: Rc<Cell<u32>>) -> LocalContainer {
        LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
            .unwrap()
            .toolbar_host(crate::ContainerToolbarHost::Container {
                position: crate::ContainerToolbarPosition::Top,
            })
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

        (host, calls)
    }
    fn paint(host: &mut Host) {
        host.sync_container_toolbars(true, Rect::new(0., 0., 600., 600.), test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
    }
    #[test]
    fn expanded_focus_tab_toolbar_routes_business_and_collapse_without_self_exchange() {
        let (mut host, _) = fixture();
        host.panels[0].options.toolbar_host = crate::ContainerToolbarHost::Window;
        host.presentation.as_mut().unwrap().toggle_others().unwrap();
        host.toggle_others_expansion("other", false).unwrap();
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("focus".into()))
            .unwrap();
        let whole = Rect::new(0., 0., 600., 600.);
        let layout = host
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, 48.);
        host.apply_presentation_layout(layout).unwrap();
        paint(&mut host);
        let declarations = host.presentation_toolbar_declarations();
        assert!(declarations[0].iter().any(|d| matches!(&d.source, Source::Expand { container, expanded: true } if container == "focus")));
        assert!(
            !declarations[0]
                .iter()
                .any(|d| matches!(&d.source, Source::Title))
        );
        let business = declarations[0]
            .iter()
            .find(|d| matches!(&d.source, Source::Panel { module, .. } if module == "focus"))
            .unwrap();
        assert_eq!(business.region, WINDOW_OTHERS_BUSINESS);
        assert_eq!(
            host.presentation
                .as_ref()
                .unwrap()
                .focus_switch_target("focus"),
            None
        );
        assert!(host.toggle_others_expansion("focus", true).unwrap());
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("other".into()))
        );
    }
    #[test]
    fn dark_toolbar_measurement_keeps_original_hover_transition_palette() {
        use std::time::Duration;
        use tekes_ui_scene::Primitive;
        let (mut host, _) = fixture();
        host = host.with_theme(Theme::DARK);
        let palette = host.palette();
        let whole = Rect::new(0., 0., 600., 600.);
        host.text.begin_frame(Duration::ZERO, false);
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        render_test_tabs(
            &mut host.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            palette,
        )
        .unwrap();
        let target = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run focus")
            .unwrap()
            .1
            .frame;
        host.panels[0]
            .mounted_toolbar
            .host
            .event(&ContentEvent::PointerMove(target.center()));
        host.text.begin_frame(Duration::from_millis(10), false);
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        let mut scene = Scene::new();
        render_test_tabs(
            &mut host.panels[0].mounted_toolbar.host,
            &mut scene,
            &mut host.text,
            palette,
        )
        .unwrap();
        let rest = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            "toolbar",
            "action-quiet",
            "rest",
        )
        .unwrap();
        let hover = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            "toolbar",
            "action-quiet",
            "hover",
        )
        .unwrap();
        assert_ne!(rest.background, hover.background);
        assert!(
            scene.primitives().iter().any(|p| matches!(p,
                Primitive::Quad(quad) if quad.rect == target && quad.fill == rest.background
            )),
            "the measurement pass must not snap hover by switching LIGHT/DARK"
        );
    }
    #[test]
    fn window_edge_reservations_keep_sibling_toolbar_and_tab_hits_outside_original_icons() {
        let edges = [
            Rect::new(100., 4., 120., 42.),
            Rect::new(1060., 4., 40., 42.),
        ];
        let mut left = ToolbarRegion::new(ComponentId(1), Rect::new(0., 4., 366., 42.));
        left.native_safe_area.left = 100.;
        reserve_window_edges(&mut left, edges);
        assert_eq!(left.native_safe_area.left, 220.);
        let mut center = ToolbarRegion::new(ComponentId(2), Rect::new(367., 4., 366., 42.));
        reserve_window_edges(&mut center, edges);
        assert_eq!(center.native_safe_area.left, 0.);
        assert_eq!(center.native_safe_area.right, 0.);
        let mut right = ToolbarRegion::new(ComponentId(3), Rect::new(734., 4., 366., 42.));
        reserve_window_edges(&mut right, edges);
        assert_eq!(right.native_safe_area.right, 40.);
        let before = right.clone();
        reserve_window_edges(&mut right, edges);
        assert_eq!(
            right, before,
            "same layout does not accumulate insets or move a held target"
        );
    }
    #[test]
    fn switcher_close_uses_original_container_key_and_keeps_its_host_for_reopen() {
        let calls = Rc::new(Cell::new(0));
        let members = ["focus", "first", "second"]
            .into_iter()
            .map(|id| local(id, 1, calls.clone()).into())
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
        let original: Vec<_> = host.panels.iter().map(|panel| panel.instance).collect();
        let area = Rect::new(0., 0., 1200., 600.);
        let layout = host.presentation.as_ref().unwrap().layout(area);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (panel, frame) in host.panels.iter_mut().zip(frames) {
            panel.bounds = frame;
        }
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        host.toolbar_index = 0;
        host.paint_presentation(&mut Scene::new(), area, Theme::LIGHT.into())
            .unwrap();
        let header = host.presentation_tabs.as_mut().unwrap();
        let target = header
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Close first")
            .unwrap()
            .0;
        assert!(
            header
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.apply_tab_selection().unwrap();
        assert_eq!(
            host.presentation.as_ref().unwrap().others_ids(),
            &["second"]
        );
        host.presentation.as_mut().unwrap().open("first").unwrap();
        assert_eq!(
            host.panels
                .iter()
                .map(|panel| panel.instance)
                .collect::<Vec<_>>(),
            original
        );
        assert_eq!(
            host.presentation.as_ref().unwrap().others_ids(),
            &["second", "first"]
        );
    }
    #[test]
    fn focus_switch_uses_original_button_and_selected_target_and_updates_revoke_old_press() {
        let calls = Rc::new(Cell::new(0));
        let mut members: Vec<Container> = ["focus", "other"]
            .into_iter()
            .map(|id| local(id, 1, calls.clone()).focus_switch(1).into())
            .collect();
        members.push(
            local("reading", 2, Rc::new(Cell::new(0)))
                .presentation(crate::ContainerPresentation::List { height: 240. })
                .into(),
        );
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .others(OthersOptions::new(vec!["other".into(), "reading".into()])),
            ),
        );
        host.mount_columns();
        let area = Rect::new(0., 0., 1100., 600.);
        let render = |host: &mut Host| {
            let layout = host.presentation.as_ref().unwrap().layout(area);
            let frames = host.apply_presentation_layout(layout).unwrap();
            for (panel, frame) in host.panels.iter_mut().zip(frames) {
                panel.bounds = frame;
            }
            host.sync_container_toolbars(true, area, test_band(), 40.)
                .unwrap();
            for panel in &mut host.panels {
                render_test_tabs(
                    &mut panel.mounted_toolbar.host,
                    &mut Scene::new(),
                    &mut host.text,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            }
        };
        render(&mut host);
        let part = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Switch to Others")
            .unwrap();
        let down = part.1.frame.center();
        let target = host.toolbar_target_at(down).unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(down))
            .unwrap();
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::List)
            .unwrap();
        render(&mut host);
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(down))
            .unwrap();
        assert_eq!(host.presentation.as_ref().unwrap().focus_id(), "focus");
        assert!(
            host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .any(|(_, part)| part.semantics.name == "Switch to Others"
                    && part.semantics.disabled)
        );
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("other".into()))
            .unwrap();
        render(&mut host);
        let point = host.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Switch to Focus")
            .unwrap()
            .1
            .frame
            .center();
        let target = host.toolbar_target_at(point).unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
            .unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(host.presentation.as_ref().unwrap().focus_id(), "other");
        assert_eq!(
            calls.get(),
            0,
            "presentation command is not mis-dispatched as owner business action"
        );
        host.toggle_visibility(1).unwrap();
        render(&mut host);
        assert!(
            host.toolbar_groups()
                .contains(&ToolbarTarget::Visibility(1))
        );
        assert!(
            host.panels[1]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .any(|(_, part)| part.semantics.name == "Show Others")
        );
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
        render_test_tabs(
            &mut host.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        for (_, part) in host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
        {
            assert!(
                part.frame.min_y() >= 56. || part.frame.min_x() >= 100.,
                "only frames intersecting the native controls need horizontal exclusion"
            );
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
        assert!((frame.size.width - host.inner.ratio() * (area.size.width - 1.)).abs() < 0.01);
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
                let container = local(id, index as u64 + 1, counts[index].clone());
                if *id == "first" {
                    container.into()
                } else {
                    container.list_height(240.).into()
                }
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
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
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
    fn native_window_row_and_explicit_container_top_have_distinct_safe_regions() {
        let (mut host, _) = fixture();
        let area = Rect::new(0., 0., 600., 600.);
        host.panels[0].bounds = area;
        host.panels[0].clip = area;

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
        render_test_tabs(
            &mut host.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
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
            if part.frame.min_y() < 52. {
                assert!(part.frame.min_x() >= 100.);
                assert!(
                    part.frame.min_y() >= 4. && part.frame.max_y() <= 52.,
                    "window contributions retain the actual native row"
                );
            } else {
                assert!(
                    host.panels[0]
                        .geometry
                        .toolbar
                        .unwrap()
                        .contains(part.frame.center()),
                    "explicit Container Top now uses its internal region below the window row"
                );
            }
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

        paint(&mut host);
        assert!(
            !host.panels[0]
                .mounted_toolbar
                .sources()
                .any(|source| matches!(source, Source::Panel { index: 0, .. })),
            "removed business toolbar has no mounted original command source"
        );
        assert!(
            host.panels[0].geometry.toolbar.is_none(),
            "no business declaration adds an internal bar"
        );
        assert!(host.presentation_menu().items.iter().any(|item| matches!(item, MenuItem::Command(command) if command.command == CommandId(COMMAND_BASE))), "menu recovery remains available without inventing a toolbar row");
        assert!(
            host.panels[0]
                .mounted_toolbar
                .sources()
                .any(|source| matches!(source, Source::Visibility(_))),
            "window chrome remains available independently of an internal business bar"
        );
        host.panels[0].options.title = Some("Host".into());

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
        assert!(
            host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .any(|(_, part)| part.semantics.name == "Show Others"),
            "an explicitly declared title/Toolbar may host the restore action"
        );
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
    fn workspace_secondary_fixture() -> Host {
        let local = |id: &str, command| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
                .unwrap()
                .toolbar(vec![ToolbarItem {
                    id: command,
                    title: id.into(),
                    enabled: true,
                    geometry_dependent: false,
                }])
        };
        let mut host = Host::from_containers(
            vec![
                local("focus", 1).focus_switch(1).into(),
                local("other", 2).secondary_commands(vec![2]).into(),
            ],
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .workspace_switch(true)
                    .others(OthersOptions::new(vec!["other".into()])),
            ),
        );
        assert!(host.mount_columns());
        host.container_window_boundary = Some(48.);
        refresh_window_chrome(&mut host);
        host
    }
    #[test]
    fn workspace_secondary_is_single_and_container_commands_keep_their_owner() {
        let mut host = workspace_secondary_fixture();
        let declarations = host.presentation_toolbar_declarations();
        let groups: Vec<_> = declarations.iter().flatten().collect();
        assert_eq!(
            groups
                .iter()
                .filter(|d| matches!(d.source, Source::WorkspaceSwitch { .. }))
                .count(),
            1
        );
        let secondary = groups
            .iter()
            .find(|d| matches!(d.source, Source::Panel { index: 1, .. }))
            .unwrap();
        assert_eq!(secondary.placement, ToolbarPlacement::SecondaryAction);
        assert!(
            matches!(&secondary.contents, Contents::TemplateCommands(buttons, ToolbarGroupOverflow::Menu, _) if buttons.len() == 1 && buttons[0].id == 2)
        );
        assert!(
            !groups
                .iter()
                .any(|d| matches!(d.source, Source::Panel { index: 0, .. }))
        );
        let original = groups
            .iter()
            .find(|d| matches!(d.source, Source::WorkspaceSwitch { .. }))
            .unwrap()
            .source
            .clone();
        let Source::WorkspaceSwitch { focus, other } = original.clone() else {
            unreachable!()
        };
        host.presentation
            .as_mut()
            .unwrap()
            .toggle_others_expanded()
            .unwrap();
        refresh_window_chrome(&mut host);
        assert!(host.switch_workspace_focus(focus, other).unwrap());
        assert!(!host.presentation.as_ref().unwrap().others_expanded());
        assert_eq!(host.presentation.as_ref().unwrap().focus_id(), "other");
        assert!(!host.switch_workspace_focus(focus, other).unwrap());

        refresh_window_chrome(&mut host);
        let changed = host.presentation_toolbar_declarations();
        let switch: Vec<_> = changed
            .iter()
            .flatten()
            .filter(|d| matches!(d.source, Source::WorkspaceSwitch { .. }))
            .collect();
        assert_eq!(switch.len(), 1);
        assert_ne!(original, switch[0].source);
        host.presentation.as_mut().unwrap().toggle_others().unwrap();
        refresh_window_chrome(&mut host);
        let hidden = host.presentation_toolbar_declarations();
        let switch = hidden
            .iter()
            .flatten()
            .find(|d| matches!(d.source, Source::WorkspaceSwitch { .. }))
            .unwrap();
        assert!(
            matches!(&switch.contents, Contents::Commands(buttons, ToolbarGroupOverflow::Menu) if !buttons[0].enabled)
        );
    }
    #[test]
    fn workspace_secondary_pointer_activates_original_action_after_redraw() {
        for expanded in [false, true] {
            let mut host = workspace_secondary_fixture();
            if expanded {
                host.presentation
                    .as_mut()
                    .unwrap()
                    .toggle_others_expanded()
                    .unwrap();
                refresh_window_chrome(&mut host);
            }
            let point = host.panels[0]
                .mounted_toolbar
                .host
                .content_host()
                .get(ComponentId(92041))
                .unwrap()
                .frame()
                .center();
            let target = host.toolbar_target_at(point).unwrap();
            let pointer = |position: Point, phase| {
                let kind = match phase {
                    PointerPhase::Down => MouseEventKind::Down {
                        button: MouseButton::Left,
                        click_count: 1,
                    },
                    PointerPhase::Up => MouseEventKind::Up {
                        button: MouseButton::Left,
                        click_count: 1,
                    },
                    _ => MouseEventKind::Moved,
                };
                window_toolbar_mount::native_toolbar_event(&Event::Mouse(MouseEvent {
                    kind,
                    position: tekes_ui_platform_macos::Point {
                        x: position.x as f64,
                        y: position.y as f64,
                    },
                    modifiers: Default::default(),
                }))
                .unwrap()
            };
            host.dispatch_toolbar(target, &pointer(point, PointerPhase::Down))
                .unwrap();
            refresh_window_chrome(&mut host);
            host.dispatch_toolbar(target, &pointer(point, PointerPhase::Up))
                .unwrap();
            refresh_window_chrome(&mut host);
            assert_eq!(host.open_toolbar(), Some(target));
            let row = host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == "Switch Focus and Others")
                .unwrap()
                .1
                .frame
                .center();
            for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
                // Use the same open-overlay precedence as route_toolbar_event,
                // including hits outside the original toolbar band.
                let owner = host
                    .open_toolbar()
                    .or_else(|| host.toolbar_target_at(row))
                    .unwrap();
                assert_eq!(owner, target);
                host.dispatch_toolbar(owner, &pointer(row, phase)).unwrap();
                if phase != PointerPhase::Up {
                    refresh_window_chrome(&mut host);
                }
            }
            assert_eq!(
                host.presentation.as_ref().unwrap().focus_id(),
                "other",
                "expanded={expanded}"
            );
            assert!(!host.presentation.as_ref().unwrap().others_expanded());
            host.drain_toolbar_commands().unwrap();
            assert_eq!(host.presentation.as_ref().unwrap().focus_id(), "other");
            assert_eq!(host.open_toolbar(), None);
        }
    }
    #[test]
    fn expanded_workspace_menu_survives_real_hidden_draw_between_press_and_release() {
        let mut host = workspace_secondary_fixture();
        host.presentation
            .as_mut()
            .unwrap()
            .toggle_others_expanded()
            .unwrap();
        refresh_window_chrome(&mut host);
        let presenter = host
            .panels
            .iter()
            .position(|panel| panel.module_id == "focus")
            .unwrap();
        let point = host.panels[presenter]
            .mounted_toolbar
            .host
            .content_host()
            .get(ComponentId(92041))
            .unwrap()
            .frame()
            .center();
        let target = host.toolbar_target_at(point).unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
            .unwrap();
        refresh_window_chrome(&mut host);
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
            .unwrap();
        refresh_window_chrome(&mut host);
        assert_eq!(
            host.open_toolbar(),
            Some(target),
            "same hidden geometry must not cancel the original More menu"
        );
        let action = host.panels[presenter]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Switch Focus and Others")
            .unwrap()
            .0;
        host.panels[presenter]
            .mounted_toolbar
            .host
            .accessibility_action(action, AccessibilityAction::Press);
        host.toolbar_index = presenter;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(host.presentation.as_ref().unwrap().focus_id(), "other");
        assert!(!host.presentation.as_ref().unwrap().others_expanded());
        refresh_window_chrome(&mut host);
        assert!(host.panels.iter().all(|panel| panel.bounds.size.width > 0.));
    }
    fn window_chrome_fixture() -> Host {
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
                .unwrap()
                .title(id)
                .into()
        };
        let mut host = Host::from_containers(
            vec![local("nav"), local("focus"), local("other")],
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .nav_bar("nav")
                    .navigation_toolbar("other")
                    .others(OthersOptions::new(vec!["other".into()])),
            ),
        );
        host.mount_columns();
        host.container_window_boundary = Some(48.);
        refresh_window_chrome(&mut host);
        host
    }
    fn refresh_window_chrome(host: &mut Host) {
        let whole = Rect::new(0., 0., 1200., 700.);
        render_layout(host, whole);
        for index in 0..host.panels.len() {
            let bounds = host.panels[index].bounds;
            if bounds.size.width <= 0. {
                host.retire_hidden_content(index, bounds);
            }
        }
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        host.reconcile_toolbar_focus(true);
    }
    #[test]
    fn partition_chrome_navigation_follows_identity_and_respects_declared_host() {
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
                .unwrap()
                .into()
        };
        let mut host = Host::from_containers(
            vec![local("nav"), local("focus"), local("other")],
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .nav_bar("nav")
                    .navigation_toolbar("focus")
                    .navigation_toolbar_follows_focus(true)
                    .others(OthersOptions::new(vec!["other".into()])),
            ),
        );
        host.mount_columns();
        refresh_window_chrome(&mut host);
        for index in [1, 2] {
            let state = host.presentation.as_ref().unwrap();
            let target = host
                .panels
                .iter()
                .position(|panel| {
                    Some(panel.module_id.as_str()) == state.navigation_toolbar_presenter_id()
                })
                .unwrap();
            assert_eq!(
                target, index,
                "navigation follows original current Focus identity"
            );
            assert_eq!(
                host.business_destination(target, None, index),
                (index, WINDOW_BUSINESS)
            );
            host.panels[target].options.toolbar_host = crate::ContainerToolbarHost::Container {
                position: crate::ContainerToolbarPosition::Bottom,
            };
            assert_eq!(
                host.business_destination(target, None, index),
                (target, ComponentId(91000 + target as u64)),
                "follows-focus must not override explicit internal host"
            );
            host.presentation
                .as_mut()
                .unwrap()
                .promote(if index == 1 { "other" } else { "focus" })
                .unwrap();
        }
    }
    #[test]
    fn partition_chrome_actual_column_bounds_follow_resize_and_visibility() {
        let mut host = window_chrome_fixture();
        for _ in 0..2 {
            for id in ["nav", "focus"] {
                let expected = host
                    .panels
                    .iter()
                    .find(|panel| panel.module_id == id)
                    .unwrap()
                    .bounds;
                if expected.size.width > 0. {
                    let title = host
                        .panels
                        .iter()
                        .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                        .find(|node| node.node.semantics.name == id)
                        .unwrap()
                        .node
                        .frame;
                    assert!(title.min_x() >= expected.min_x() && title.max_x() <= expected.max_x());
                }
            }
            host.presentation
                .as_mut()
                .unwrap()
                .toggle_nav_bar()
                .unwrap();
            refresh_window_chrome(&mut host);
            let expected = if host.presentation.as_ref().unwrap().nav_bar_visible() {
                "Hide NavBar"
            } else {
                "Show NavBar"
            };
            assert!(
                host.panels
                    .iter()
                    .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                    .flat_map(|node| node.parts)
                    .any(|(_, part)| part.semantics.name == expected && part.frame.size.width > 0.)
            );
        }
        host.presentation.as_mut().unwrap().resize_nav_width(
            300.,
            Rect::new(0., 0., 1200., 700.),
            MIN_COLUMN,
        );
        host.presentation
            .as_mut()
            .unwrap()
            .set_focus_fraction(0.6)
            .unwrap();
        refresh_window_chrome(&mut host);
        let focus = host.panels[1].bounds;
        let region = host.panels[1]
            .mounted_toolbar
            .host
            .content_host()
            .get(WINDOW_BUSINESS)
            .unwrap()
            .frame();
        assert!(region.min_x() >= focus.min_x() && region.max_x() <= focus.max_x());
        for _ in 0..2 {
            host.presentation.as_mut().unwrap().toggle_others().unwrap();
            refresh_window_chrome(&mut host);
            let expected = if host.presentation.as_ref().unwrap().others_visible() {
                "Hide Others"
            } else {
                "Show Others"
            };
            assert!(
                host.panels
                    .iter()
                    .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                    .flat_map(|node| node.parts)
                    .any(|(_, part)| part.semantics.name == expected && part.frame.size.width > 0.)
            );
        }
    }
    #[test]
    fn partition_chrome_fixed_navigation_in_others_does_not_overlap_leading_switcher() {
        let mut host = window_chrome_fixture();
        assert_eq!(
            host.presentation
                .as_ref()
                .unwrap()
                .navigation_toolbar_presenter_id(),
            Some("other")
        );
        let whole = Rect::new(0., 0., 1200., 700.);
        let source = crate::toolbar_source::ComponentSource {
            session: tekes_ui_module_runtime::component::Session {
                module: "navigation".into(),
                instance: 1,
                host_epoch: 1,
            },
            registration: 1,
            window: host.panels[1].mounted_toolbar.window,
        };
        let (presenter, declaration) = host.navigation_toolbar_declaration(
            2,
            1,
            source,
            vec![ToolbarButtonDeclaration {
                id: 71,
                icon: None,
                text: "Back".into(),
                hint: Some("Back".into()),
                enabled: true,
            }],
            Vec::new(),
        );
        assert_eq!((presenter, declaration.region), (1, WINDOW_OTHERS_BUSINESS));
        let mut declarations = host.presentation_toolbar_declarations();
        declarations[presenter].push(declaration);
        let regions = host.container_toolbar_regions(presenter, 1, whole, &test_band());
        host.panels[presenter]
            .mounted_toolbar
            .sync(whole, regions, std::mem::take(&mut declarations[presenter]))
            .unwrap();
        render_test_tabs(
            &mut host.panels[presenter].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let palette = host.palette();
        host.paint_presentation(&mut Scene::new(), whole, palette)
            .unwrap();
        let frame = host.panels[presenter]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Back")
            .unwrap()
            .1
            .frame;
        let switcher = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(ComponentId(999_999))
            .unwrap()
            .frame();
        let others = host.panels[2].bounds;
        assert!(
            switcher.size.width > 0.,
            "a fixed navigation source must not squeeze Others switcher away"
        );
        assert!(
            switcher.min_x() >= others.min_x()
                && switcher.min_x()
                    <= others.min_x()
                        + tekes_ui_tokens::proposal::metric("spacing", "space-regular").unwrap()
        );
        assert!(
            switcher.max_x() <= frame.min_x() || frame.max_x() <= switcher.min_x(),
            "original fixed navigation and switcher frames must not overlap: {switcher:?} {frame:?}"
        );
    }
    #[test]
    fn final_window_scene_keeps_others_icons_above_empty_business_chrome() {
        use tekes_ui_scene::Primitive;
        for theme in [Theme::LIGHT, Theme::DARK] {
            let mut host = window_chrome_fixture().with_theme(theme);
            let whole = Rect::new(0., 0., 1200., 700.);
            let palette = host.palette();
            let mut toolbar = Scene::new();
            for panel in &mut host.panels {
                render_test_tabs(
                    &mut panel.mounted_toolbar.host,
                    &mut toolbar,
                    &mut host.text,
                    palette,
                )
                .unwrap();
            }
            let mut presentation = Scene::new();
            host.paint_presentation(&mut presentation, whole, palette)
                .unwrap();
            let others = host.panels[2].bounds;
            let mut final_scene = Scene::new();
            // The exact production native draw composition, after all original
            // hosts have emitted their real ground and prepared-icon primitives.
            host.append_toolbar_layers(&mut final_scene, &toolbar, &presentation, &Scene::new());
            let images: Vec<_> = final_scene
                .primitives()
                .iter()
                .enumerate()
                .filter_map(|(index, primitive)| match primitive {
                    Primitive::Image(image)
                        if image.rect.center().x >= others.min_x()
                            && image.rect.center().y < 48. =>
                    {
                        Some((index, image.rect.center()))
                    }
                    _ => None,
                })
                .collect();
            assert!(!images.is_empty(), "original Others icon must be emitted");
            for (index, point) in images {
                assert!(
                    !final_scene.primitives()[index + 1..]
                        .iter()
                        .any(|primitive| matches!(primitive,
                    Primitive::Quad(quad) if quad.fill.a == 1. && quad.rect.contains(point))),
                    "later opaque toolbar ground must not erase the Others icon in the final native Scene"
                );
            }
        }
    }
    fn t52_refresh(host: &mut Host) {
        let whole = Rect::new(0., 0., 1200., 700.);
        let layout = host
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, 48.);
        host.apply_presentation_layout(layout).unwrap();
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        host.reconcile_toolbar_focus(true);
        let palette = host.palette();
        host.paint_presentation(&mut Scene::new(), whole, palette)
            .unwrap();
    }
    fn t52_button(host: &Host, name: &str) -> Rect {
        host.panels
            .iter()
            .flat_map(|panel| panel.mounted_toolbar.host.semantics())
            .chain(
                host.presentation_tabs
                    .iter()
                    .flat_map(|header| header.semantics()),
            )
            .find_map(|node| {
                if node.node.semantics.name == name {
                    Some(node.node.frame)
                } else {
                    node.parts
                        .into_iter()
                        .find(|(_, part)| part.semantics.name == name)
                        .map(|(_, part)| part.frame)
                }
            })
            .unwrap_or_else(|| panic!("expected original Window navigation button {name}"))
    }
    fn t52_click(host: &mut Host, name: &str) {
        let point = t52_button(host, name).center();
        if host.presentation_tabs.as_ref().is_some_and(|header| {
            header.semantics().iter().any(|node| {
                node.parts
                    .iter()
                    .any(|(_, part)| part.semantics.name == name)
            })
        }) {
            host.presentation_tab_input(&ContentEvent::PointerDown(point))
                .unwrap();
            t52_refresh(host);
            host.presentation_tab_input(&ContentEvent::PointerUp(point))
                .unwrap();
            t52_refresh(host);
            return;
        }
        let target = host
            .toolbar_target_at(point)
            .expect("original navigation pointer target");
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
            .unwrap();
        t52_refresh(host);
        let target = host
            .toolbar_target_at(point)
            .expect("navigation target after original draw");
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
            .unwrap();
        t52_refresh(host);
    }
    #[test]
    fn t52_window_switch_icon_hover_reaches_original_button_and_survives_measure_paint() {
        use std::time::Duration;
        use tekes_ui_scene::Primitive;
        let mut host = window_chrome_fixture();
        host.panels[1].toolbar = vec![ToolbarItem {
            id: 1,
            title: "Switch".into(),
            enabled: true,
            geometry_dependent: false,
        }];
        host.panels[1].options.focus_switch_command = Some(1);
        t52_refresh(&mut host);
        let whole = Rect::new(0., 0., 1200., 700.);
        let paint_icons = |host: &mut Host, now| {
            host.text.begin_frame(now, false);
            // Real production declaration projection with icon capability; use
            // existing prepared-image services, no GPU renderer or alternate widget.
            let mut declarations = host.presentation_toolbar_declarations_with_icons(true);
            let regions: Vec<_> = (0..host.panels.len())
                .map(|index| host.container_toolbar_regions(index, 1, whole, &test_band()))
                .collect();
            for (index, panel) in host.panels.iter_mut().enumerate() {
                panel
                    .mounted_toolbar
                    .sync(
                        whole,
                        regions[index].clone(),
                        std::mem::take(&mut declarations[index]),
                    )
                    .unwrap();
            }
            let palette = host.palette();
            render_test_tabs(
                &mut host.panels[1].mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                palette,
            )
            .unwrap();
            let mut toolbar = Scene::new();
            for panel in &mut host.panels {
                render_test_tabs(
                    &mut panel.mounted_toolbar.host,
                    &mut toolbar,
                    &mut host.text,
                    palette,
                )
                .unwrap();
            }
            let mut presentation = Scene::new();
            host.paint_presentation(&mut presentation, whole, palette)
                .unwrap();
            let mut scene = Scene::new();
            host.append_toolbar_layers(&mut scene, &toolbar, &presentation, &Scene::new());
            scene
        };
        paint_icons(&mut host, Duration::ZERO);
        let frame = t52_button(&host, "Switch to Others");
        let target = host.toolbar_target_at(frame.center()).unwrap();
        let pointer = ContentEvent::Pointer(PointerEvent {
            position: frame.center(),
            phase: PointerPhase::Move,
            button: None,
            click_count: 0,
            modifiers: Default::default(),
        });
        host.dispatch_toolbar(target, &pointer).unwrap();
        paint_icons(&mut host, Duration::from_millis(10));
        let scene = paint_icons(&mut host, Duration::from_millis(500));
        let hover = tekes_ui_tokens::surface_palette::resolve_surface(
            host.palette(),
            "toolbar",
            "action-quiet",
            "hover",
        )
        .unwrap()
        .background;
        assert!(
            scene
                .primitives()
                .iter()
                .any(|primitive| matches!(primitive,
            Primitive::Quad(quad) if quad.rect == frame && quad.fill == hover)),
            "Window switch hover must remain visible after original measurement and final composed paint"
        );
        host.dispatch_toolbar(target, &ContentEvent::PointerMove(Point::new(-1., -1.)))
            .unwrap();
        paint_icons(&mut host, Duration::from_millis(510));
        let scene = paint_icons(&mut host, Duration::from_millis(1000));
        let rest = tekes_ui_tokens::surface_palette::resolve_surface(
            host.palette(),
            "toolbar",
            "action-quiet",
            "rest",
        )
        .unwrap()
        .background;
        assert!(
            scene
                .primitives()
                .iter()
                .any(|primitive| matches!(primitive,
            Primitive::Quad(quad) if quad.rect == frame && quad.fill == rest)),
            "leave restores original rest state"
        );
    }
    #[test]
    fn t52_others_window_layout_controls_have_order_and_no_internal_close_strip() {
        let mut host = window_chrome_fixture();
        t52_refresh(&mut host);
        let expand = t52_button(&host, "Expand Others");
        let close = t52_button(&host, "Close other");
        let toggle = t52_button(&host, "Hide Others");
        assert!(close.max_x() <= expand.min_x() && expand.max_x() <= toggle.min_x());
        assert!(expand.max_y() <= 48. && close.max_y() <= 48. && toggle.max_y() <= 48.);
        assert!(
            host.panels[2].geometry.close.is_none(),
            "Close migration leaves no internal close chrome"
        );
        assert_eq!(host.panels[2].geometry.content.min_y(), 48.);
        assert!(
            !host
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .any(|node| node.node.semantics.name == "Close focus")
        );
        t52_click(&mut host, "Close other");
        assert!(host.presentation.as_ref().unwrap().others_ids().is_empty());
    }
    #[test]
    fn t52_expand_and_collapse_preserve_original_focus_and_split_allocations() {
        let mut host = window_chrome_fixture();
        t52_refresh(&mut host);
        let original: Vec<_> = host
            .panels
            .iter()
            .map(|panel| (panel.instance, panel.bounds))
            .collect();
        let mut original_focus = 0;
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| original_focus = h as *mut ContentHost as usize);
        t52_click(&mut host, "Expand Others");
        assert_eq!(
            host.presentation.as_ref().unwrap().focus_id(),
            "focus",
            "expanding does not promote or replace Focus"
        );
        assert_eq!(
            host.panels[0].bounds, original[0].1,
            "Nav allocation is unchanged"
        );
        assert_eq!(host.panels[1].bounds.size.width, 0., "Focus body is hidden");
        assert_eq!(host.panels[2].bounds.min_x(), original[1].1.min_x());
        assert_eq!(host.panels[2].bounds.max_x(), original[2].1.max_x());
        let mut retained_focus = 0;
        host.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| retained_focus = h as *mut ContentHost as usize);
        assert_eq!(
            retained_focus, original_focus,
            "original hidden Focus ContentHost survives"
        );
        t52_click(&mut host, "Collapse Others");
        for (panel, before) in host.panels.iter().zip(original) {
            assert_eq!(
                (panel.instance, panel.bounds),
                before,
                "collapse restores original instance and proportions"
            );
        }
    }
    #[test]
    fn t64_content_switcher_is_one_tab_list_with_authoritative_selection() {
        use tekes_ui_controls::composition::ContentRole;
        let mut host = window_chrome_fixture();
        t52_refresh(&mut host);
        let header = host.presentation_tabs.as_ref().unwrap();
        let group = header
            .semantics()
            .into_iter()
            .find(|node| node.node.semantics.role == ContentRole::TabList)
            .unwrap();
        assert_eq!(group.node.semantics.role, ContentRole::TabList);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("other".into()))
        );
        assert_eq!(
            header
                .semantics()
                .iter()
                .flat_map(|node| &node.parts)
                .filter(|(_, part)| part.semantics.selected)
                .count(),
            1,
            "even a sole current destination reflects the source selection"
        );
    }
    #[test]
    fn partition_chrome_window_titles_stay_in_their_container_columns() {
        let host = window_chrome_fixture();
        for (index, name) in [(0, "nav"), (1, "focus")] {
            let frame = host
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .find(|node| node.node.semantics.name == name)
                .expect("original title")
                .node
                .frame;
            let column = host.panels[index].bounds;
            assert!(
                frame.min_x() >= column.min_x() && frame.max_x() <= column.max_x(),
                "title {name} must stay in its actual column: title={frame:?}, column={column:?}"
            );
        }
    }
    #[test]
    fn partition_chrome_others_switcher_starts_at_its_own_leading_edge() {
        let mut host = window_chrome_fixture();
        let whole = Rect::new(0., 0., 1200., 700.);
        let palette = host.palette();
        host.paint_presentation(&mut Scene::new(), whole, palette)
            .unwrap();
        let column = host.panels[2].bounds;
        let group = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(ComponentId(999_999))
            .unwrap()
            .frame();
        let regular = tekes_ui_tokens::proposal::metric("spacing", "space-regular").unwrap();
        assert!(
            group.min_x() >= column.min_x() && group.min_x() <= column.min_x() + regular,
            "Others navigation must be leading, not centered/trailing: {group:?}, {column:?}"
        );
    }
    #[test]
    fn partition_chrome_internal_business_keeps_its_leading_edge_in_all_four_directions() {
        for position in [
            crate::ContainerToolbarPosition::Top,
            crate::ContainerToolbarPosition::Bottom,
            crate::ContainerToolbarPosition::Leading,
            crate::ContainerToolbarPosition::Trailing,
        ] {
            let mut host = window_chrome_fixture();
            host.panels[2].options.toolbar_host =
                crate::ContainerToolbarHost::Container { position };
            host.panels[2].toolbar = vec![ToolbarItem {
                id: 7,
                title: "Business".into(),
                enabled: true,
                geometry_dependent: false,
            }];
            refresh_window_chrome(&mut host);
            let bar = host.panels[2].geometry.toolbar.unwrap();
            let frame = host.panels[2]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == "Business")
                .unwrap()
                .1
                .frame;
            let regular = tekes_ui_tokens::proposal::metric("spacing", "space-regular").unwrap();
            if matches!(
                position,
                crate::ContainerToolbarPosition::Leading
                    | crate::ContainerToolbarPosition::Trailing
            ) {
                assert!(
                    frame.min_y() >= bar.min_y() && frame.min_y() <= bar.min_y() + regular * 2.,
                    "vertical internal business starts at its top: {frame:?} {bar:?}"
                );
            } else {
                assert!(
                    frame.min_x() >= bar.min_x() && frame.min_x() <= bar.min_x() + regular * 2.,
                    "horizontal internal business starts at its leading edge: {frame:?} {bar:?}"
                );
            }
        }
    }
    #[test]
    fn partition_chrome_no_business_bar_leaves_no_internal_close_strip() {
        for host_option in [
            crate::ContainerToolbarHost::Window,
            crate::ContainerToolbarHost::Container {
                position: crate::ContainerToolbarPosition::Top,
            },
            crate::ContainerToolbarHost::Container {
                position: crate::ContainerToolbarPosition::Trailing,
            },
        ] {
            let mut host = window_chrome_fixture();
            host.panels[2].options.toolbar_host = host_option;
            t52_refresh(&mut host);
            let panel = &host.panels[2];
            assert!(
                panel.geometry.toolbar.is_none(),
                "no business item creates no business bar"
            );
            assert!(panel.geometry.close.is_none());
            assert!(
                panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(ComponentId(95000 + 2))
                    .is_none()
            );
            assert!(t52_button(&host, "Close other").max_y() <= panel.geometry.content.min_y());
        }
    }
    #[test]
    fn partition_chrome_top_business_keeps_its_height_without_internal_close() {
        use crate::ContainerToolbarPosition as Position;
        let mut host = window_chrome_fixture();
        host.panels[2].options.toolbar_host = crate::ContainerToolbarHost::Container {
            position: Position::Top,
        };
        host.panels[2].toolbar = vec![ToolbarItem {
            id: 1,
            title: "Business".into(),
            enabled: true,
            geometry_dependent: false,
        }];
        t52_refresh(&mut host);
        let geometry = host.panels[2].geometry;
        let bar = geometry.toolbar.unwrap();
        assert!(geometry.close.is_none());
        assert!(geometry.content.min_y() >= bar.max_y());
        assert!(t52_button(&host, "Close other").max_y() <= bar.min_y());
    }
    #[test]
    fn partition_chrome_repeated_exchange_uses_real_original_press_paint_release() {
        let mut host = window_chrome_fixture();
        for index in [1, 2] {
            host.panels[index].toolbar = vec![ToolbarItem {
                id: 1,
                title: "Switch".into(),
                enabled: true,
                geometry_dependent: false,
            }];
            host.panels[index].options.focus_switch_command = Some(1);
        }
        for expected in ["other", "focus", "other", "focus"] {
            refresh_window_chrome(&mut host);
            let point = host
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == "Switch to Others")
                .expect("original Focus exchange button")
                .1
                .frame
                .center();
            let target = host
                .toolbar_target_at(point)
                .expect("real current pointer target");
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
            refresh_window_chrome(&mut host);
            assert!(host.panels.iter().any(|panel| {
                panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture()
                    .is_some()
            }));
            let target = host
                .toolbar_target_at(point)
                .expect("current target after paint");
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
                .unwrap();
            assert_eq!(host.presentation.as_ref().unwrap().focus_id(), expected);
            refresh_window_chrome(&mut host);
            assert!(host.panels.iter().all(|panel| {
                panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture()
                    .is_none()
            }));
        }
    }
    #[test]
    fn window_chrome_unified_blank_reaches_native_drag_handoff() {
        let host = window_chrome_fixture();
        assert!(
            host.panels
                .iter()
                .all(|panel| panel.geometry.toolbar.is_none())
        );
        // Pick actual blank unified chrome, rather than pretending an internal
        // toolbar exists. This is the production final native-down predicate.
        let point = (0..1200)
            .map(|x| Point::new(x as f32 + 0.5, 24.))
            .find(|point| {
                host.toolbar_target_at(*point).is_none()
                    && host
                        .panels
                        .iter()
                        .all(|panel| !panel.content_frame().contains(*point))
            })
            .expect("unified band has blank draggable chrome");
        assert!(
            host.toolbar_chrome_at(point),
            "blank Window toolbar must reach native drag handoff"
        );
        assert!(!host.toolbar_chrome_at(Point::new(point.x, 100.)));
        assert!(!host.toolbar_chrome_at(Point::new(point.x, -1.)));
    }
    #[test]
    fn window_chrome_original_layout_visibility_buttons_survive_down_paint_up_and_restore() {
        let mut host = window_chrome_fixture();
        for (name, nav, others) in [
            ("Hide NavBar", false, true),
            ("Show NavBar", true, true),
            ("Hide Others", true, false),
            ("Show Others", true, true),
        ] {
            let point = host
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == name)
                .unwrap_or_else(|| panic!("original layout action {name} must be visible"))
                .1
                .frame
                .center();
            let target = host.toolbar_target_at(point).expect("real pointer target");
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
            refresh_window_chrome(&mut host);
            assert!(
                host.panels.iter().any(|panel| panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture()
                    .is_some()),
                "draw must retain original layout-button capture"
            );
            let target = host
                .toolbar_target_at(point)
                .expect("same layout target after draw");
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
                .unwrap();
            let state = host.presentation.as_ref().unwrap();
            assert_eq!(
                (state.nav_bar_visible(), state.others_visible()),
                (nav, others),
                "{name}"
            );
            assert_eq!(state.focus_id(), "focus");
            refresh_window_chrome(&mut host);
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
        host.presentation.as_mut().unwrap().toggle_others().unwrap();
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
            local("a")
                .title("A")
                .toolbar_host(crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Top,
                })
                .into(),
            local("b")
                .title("B")
                .toolbar_host(crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Top,
                })
                .into(),
            local("owner")
                .toolbar_host(crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Top,
                })
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
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
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
    fn two_default_window_retained_targets_keep_their_columns_and_one_dispatch_per_action() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let recorded = calls.clone();
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
        };
        let action = |id, title: &str| ToolbarItem {
            id,
            title: title.into(),
            enabled: true,
            geometry_dependent: false,
        };
        let members: Vec<Container> = vec![
            local("a").title("A").into(),
            local("b").title("B").into(),
            local("owner")
                .toolbar(vec![action(10, "To A"), action(11, "To B")])
                .retain_button(10, "a")
                .retain_button(11, "b")
                .on_command(move |_, command| {
                    recorded.borrow_mut().push(command);
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
        host.sync_container_toolbars_with_icons(true, area, test_band(), 40., true)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        assert!(
            host.panels[0].geometry.toolbar.is_none() && host.panels[1].geometry.toolbar.is_none()
        );
        assert_eq!(
            host.panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.sources())
                .filter(|source| matches!(source, Source::Panel { index: 2, .. }))
                .count(),
            2,
            "two column destinations retain two groups with the same real original owner"
        );
        for (name, command, column) in [("To A", 10, 0), ("To B", 11, 1)] {
            let frame = host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == name)
                .unwrap()
                .1
                .frame;
            let bounds = host.panels[column].bounds;
            assert!(frame.min_x() >= bounds.min_x() && frame.max_x() <= bounds.max_x());
            let point = frame.center();
            let target = host.toolbar_target_at(point).unwrap();
            assert!(matches!(
                target,
                ToolbarTarget::Contribution { presenter: 0, .. }
            ));
            assert!(matches!(
                host.panels[0]
                    .mounted_toolbar
                    .source(Host::toolbar_group(target)),
                Some(Source::Panel { index: 2, .. })
            ));
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
            host.sync_container_toolbars_with_icons(true, area, test_band(), 40., true)
                .unwrap();
            for panel in &mut host.panels {
                render_test_tabs(
                    &mut panel.mounted_toolbar.host,
                    &mut Scene::new(),
                    &mut host.text,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            }
            host.reconcile_toolbar_focus(true);
            let target = host.toolbar_target_at(point).unwrap();
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
                .unwrap();
            assert_eq!(calls.borrow().last(), Some(&command));
        }
        for name in ["To A", "To B"] {
            let point = host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == name)
                .unwrap()
                .1
                .frame
                .center();
            let target = host.toolbar_target_at(point).unwrap();
            host.set_toolbar_focus(Some(target), true);
            for down in [true, false] {
                host.toolbar_key_input(ContentKeyEvent {
                    key: Some(ControlKey::Space),
                    down,
                    repeat: false,
                    modifiers: Default::default(),
                    native_code: 49,
                    characters: " ".into(),
                    characters_ignoring_modifiers: " ".into(),
                })
                .unwrap();
            }
        }
        assert_eq!(*calls.borrow(), vec![10, 11, 10, 11]);
    }
    #[test]
    fn mixed_retained_hosts_in_one_presenter_keep_both_commands_and_original_owner() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let recorded = calls.clone();
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
        };
        let action = |id, title: &str| ToolbarItem {
            id,
            title: title.into(),
            enabled: true,
            geometry_dependent: false,
        };
        let members: Vec<Container> = vec![
            local("a")
                .title("A")
                .toolbar_host(crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Top,
                })
                .into(),
            local("b").title("B").into(),
            local("owner")
                .toolbar(vec![action(10, "To A"), action(11, "To B")])
                .retain_button(10, "a")
                .retain_button(11, "b")
                .on_command(move |_, command| {
                    recorded.borrow_mut().push(command);
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
        host.sync_container_toolbars_with_icons(true, area, test_band(), 40., true)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        let parts = host.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .collect::<Vec<_>>();
        for (name, command) in [("To A", 10), ("To B", 11)] {
            let matches = parts
                .iter()
                .filter(|(_, part)| part.semantics.name == name)
                .collect::<Vec<_>>();
            assert_eq!(
                matches.len(),
                1,
                "each mixed-host retained action must have one live original control: {name}"
            );
            let point = matches[0].1.frame.center();
            if command == 10 {
                assert!(host.panels[0].geometry.toolbar.unwrap().contains(point));
            } else {
                assert!(point.y < host.panels[0].geometry.toolbar.unwrap().min_y());
            }
            let target = host.toolbar_target_at(point).unwrap();
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
                .unwrap();
        }
        assert_eq!(*calls.borrow(), vec![10, 11]);
        for name in ["To A", "To B"] {
            let point = parts
                .iter()
                .find(|(_, part)| part.semantics.name == name)
                .unwrap()
                .1
                .frame
                .center();
            let target = host.toolbar_target_at(point).unwrap();
            host.set_toolbar_focus(Some(target), true);
            assert_eq!(host.actual_toolbar_owner(), Some(target));
            for down in [true, false] {
                host.toolbar_key_input(ContentKeyEvent {
                    key: Some(ControlKey::Space),
                    down,
                    repeat: false,
                    modifiers: Default::default(),
                    native_code: 49,
                    characters: " ".into(),
                    characters_ignoring_modifiers: " ".into(),
                })
                .unwrap();
            }
        }
        assert_eq!(
            *calls.borrow(),
            vec![10, 11, 10, 11],
            "both original groups preserve keyboard source ownership as well as pointer ownership"
        );
    }
    #[test]
    fn mixed_retained_icon_templates_belong_to_their_actual_command_group() {
        for switch in [11, 10] {
            let local = |id| {
                LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
            };
            let action = |id, title: &str| ToolbarItem {
                id,
                title: title.into(),
                enabled: true,
                geometry_dependent: false,
            };
            let members: Vec<Container> = vec![
                local("a")
                    .title("A")
                    .toolbar_host(crate::ContainerToolbarHost::Container {
                        position: crate::ContainerToolbarPosition::Top,
                    })
                    .into(),
                local("b").title("B").into(),
                local("owner")
                    .toolbar(vec![action(10, "To A"), action(11, "To B")])
                    .focus_switch(switch)
                    .retain_button(10, "a")
                    .retain_button(11, "b")
                    .into(),
            ];
            let mut host = Host::from_containers(
                members,
                false,
                None,
                Some(PresentationOptions::new("a").others(OthersOptions::new(vec!["b".into()]))),
            );
            host.mount_columns();
            render_layout(&mut host, Rect::new(0., 0., 700., 700.));
            // Exercise the real production projection with icon capability,
            // including the Container-before-Window region iteration order.
            let projected = host.presentation_toolbar_declarations_with_icons(true);
            let groups = projected[0]
                .iter()
                .filter(|declaration| matches!(declaration.source, Source::Panel { index: 2, .. }))
                .collect::<Vec<_>>();
            assert_eq!(groups.len(), 2);
            let mut matched_switch = 0;
            for declaration in groups {
                let Contents::TemplateCommands(buttons, _, icons) = &declaration.contents else {
                    panic!("expected original retained command declaration");
                };
                assert_eq!(buttons.len(), 1);
                assert!(
                    icons
                        .iter()
                        .all(|(id, _)| buttons.iter().any(|button| button.id == *id)),
                    "a template must belong to an action in the same original group"
                );
                if buttons[0].id == switch {
                    assert_eq!(icons, &vec![(switch, "tekes-swap-horizontal".to_owned())]);
                    assert_eq!(
                        declaration.region,
                        if switch == 11 {
                            WINDOW_OTHERS_BUSINESS
                        } else {
                            ComponentId(91000)
                        }
                    );
                    matched_switch += 1;
                } else {
                    assert!(
                        icons.is_empty(),
                        "another retained group must not consume this switch icon"
                    );
                }
            }
            assert_eq!(matched_switch, 1);
            assert!(
                host.presentation_toolbar_declarations_with_icons(false)[0]
                    .iter()
                    .filter(|declaration| matches!(
                        declaration.source,
                        Source::Panel { index: 2, .. }
                    ))
                    .all(|declaration| declaration.contents.templates().is_empty())
            );
        }
    }
    #[test]
    fn original_overflow_blocks_background_tab_pointer_until_its_own_dismissal() {
        let member = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id))).unwrap()
        };
        let members: Vec<Container> = vec![
            member("focus")
                .title("Focus")
                .toolbar_host(crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Top,
                })
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
        host.presentation
            .as_mut()
            .unwrap()
            .set_focus_fraction(0.2)
            .unwrap();
        let area = Rect::new(0., 0., 600., 700.);
        render_layout(&mut host, area);
        // Match the production native row: window-edge controls are on the
        // Toolbar row, while the background Tabs are below that row.
        let layout = host
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(area, 48.);
        host.apply_presentation_layout(layout).unwrap();

        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
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
        render_test_tabs(
            &mut host.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.open_toolbar().is_some());
        let point = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "b")
            .unwrap()
            .1
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
