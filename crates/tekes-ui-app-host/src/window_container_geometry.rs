//! One allocation shared by Local paint, Process Layout and pointer conversion.
use super::*;
use crate::ContainerToolbarPosition as Position;
use tekes_ui_controls::Insets;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct ContainerGeometry {
    pub insets: Insets,
    pub toolbar: Option<Rect>,
    pub close: Option<Rect>,
    pub content: Rect,
}
impl ContainerGeometry {
    #[cfg(test)]
    pub fn legacy(bounds: Rect, top: f32) -> Self {
        Self {
            insets: Insets::new(top, 0., 0., 0.),
            toolbar: (top > 0.).then_some(Rect::new(
                bounds.min_x(),
                bounds.min_y(),
                bounds.size.width,
                top,
            )),
            close: None,
            content: content_geometry(bounds, top),
        }
    }
    pub fn arrange(
        bounds: Rect,
        window_boundary: f32,
        position: Option<Position>,
        thickness: f32,
        close_size: tekes_ui_scene::Size,
    ) -> Self {
        let top = (window_boundary - bounds.min_y()).clamp(0., bounds.size.height.max(0.));
        let inner = Rect::new(
            bounds.min_x(),
            bounds.min_y() + top,
            bounds.size.width,
            (bounds.size.height - top).max(0.),
        );
        let close_width = close_size.width.min(inner.size.width).max(0.);
        let close_height = close_size.height.min(inner.size.height).max(0.);
        let mut close = (close_width > 0. && close_height > 0.).then_some(Rect::new(
            inner.max_x() - close_width,
            inner.min_y(),
            close_width,
            close_height,
        ));
        let mut insets = Insets::new(top, 0., 0., 0.);
        // Top business/close share one strip. A trailing business bar already
        // clears the close column. Other placements reserve only natural close
        // height above the body, never an otherwise empty full-height rail.
        let close_top = if matches!(position, Some(Position::Top | Position::Trailing)) {
            0.
        } else {
            close_height
        };
        insets.top += close_top;
        let business = Rect::new(
            inner.min_x(),
            inner.min_y() + close_top,
            inner.size.width,
            (inner.size.height - close_top).max(0.),
        );
        let toolbar = position.map(|position| match position {
            Position::Top => {
                let height = thickness.max(close_height).min(inner.size.height);
                insets.top += height;
                if let Some(close) = &mut close {
                    close.size.height = height;
                }
                Rect::new(
                    inner.min_x(),
                    inner.min_y(),
                    (inner.size.width - close_width).max(0.),
                    height,
                )
            }
            Position::Bottom => {
                let height = thickness.min(business.size.height);
                insets.bottom = height;
                Rect::new(
                    business.min_x(),
                    business.max_y() - height,
                    business.size.width,
                    height,
                )
            }
            Position::Leading => {
                let width = thickness.min(business.size.width);
                insets.left = width;
                Rect::new(
                    business.min_x(),
                    business.min_y(),
                    width,
                    business.size.height,
                )
            }
            Position::Trailing => {
                let width = thickness.max(close_width).min(inner.size.width);
                insets.right = width;
                if let Some(close) = &mut close {
                    close.origin.x = inner.max_x() - width;
                    close.size.width = width;
                }
                Rect::new(
                    inner.max_x() - width,
                    inner.min_y() + close_height,
                    width,
                    (inner.size.height - close_height).max(0.),
                )
            }
        });
        let content = Rect::new(
            bounds.min_x() + insets.left,
            bounds.min_y() + insets.top,
            (bounds.size.width - insets.left - insets.right).max(0.),
            (bounds.size.height - insets.top - insets.bottom).max(0.),
        );
        Self {
            insets,
            toolbar,
            close,
            content,
        }
    }
}
impl Host {
    pub(super) fn is_list_member(&self, index: usize) -> bool {
        self.presentation.as_ref().is_some_and(|state| {
            state.selected_tab() == Some(&crate::OthersTab::List)
                && state
                    .list_ids()
                    .contains(&self.panels[index].module_id.as_str())
        })
    }
    pub(super) fn container_geometry(
        &mut self,
        index: usize,
        bounds: Rect,
        window_boundary: f32,
    ) -> Result<ContainerGeometry> {
        use crate::window_toolbar::Contents;
        use crate::{ContainerToolbarHost as Host, ContainerToolbarPosition as Position};
        use tekes_ui_controls::composition::Content;
        let declarations = if self.presentation.is_some() {
            self.presentation_toolbar_declarations()
        } else {
            Vec::new()
        };
        let own = ComponentId(91000 + index as u64);
        let internal = if self.presentation.is_some() {
            declarations[index]
                .iter()
                .filter(|declaration| declaration.region == own)
                .map(|declaration| declaration.contents.clone())
                .collect::<Vec<_>>()
        } else if matches!(
            self.panels[index].options.toolbar_host,
            Host::Container { .. }
        ) && !self.panels[index].toolbar.is_empty()
        {
            vec![Contents::Commands(
                self.panels[index]
                    .toolbar
                    .iter()
                    .map(
                        |action| tekes_ui_controls::composition::ToolbarButtonDeclaration {
                            id: action.id,
                            icon: None,
                            text: action.title.clone(),
                            hint: Some(action.title.clone()),
                            enabled: action.enabled,
                        },
                    )
                    .collect(),
                tekes_ui_controls::composition::ToolbarGroupOverflow::Split,
            )]
        } else {
            Vec::new()
        };
        let position = match self.panels[index].options.toolbar_host {
            Host::Container { position } if !internal.is_empty() => Some(position),
            Host::Window | Host::Container { .. } => None,
        };
        let axis = if matches!(position, Some(Position::Leading | Position::Trailing)) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        let mut thickness: f32 = 0.;
        for contents in internal {
            let templates = contents.templates().to_vec();
            let mut content = match contents {
                Contents::Commands(buttons, overflow)
                | Contents::TemplateCommands(buttons, overflow, _) => {
                    let mut content = tekes_ui_controls::composition::toolbar_commands(
                        ComponentId(1),
                        buttons,
                        overflow,
                    );
                    let control = content.component_mut::<ToolbarControl>().unwrap();
                    control.set_axis(axis);
                    for (id, name) in templates {
                        control
                            .set_action_template_icon(id, Some(&name))
                            .map_err(|error| anyhow::anyhow!("toolbar measure icon: {error:?}"))?;
                    }
                    content
                }
                Contents::Title(title) => Content::new(
                    ComponentId(1),
                    tekes_ui_controls::composition::TextContent::new(title),
                ),
                Contents::Close(_) => continue,
            };
            // Use the original action geometry; constraints affect main-axis
            // overflow later, never an invented vertical cross-axis estimate.
            let size = content
                .measure(
                    &mut self.text,
                    tekes_ui_scene::Size::new(100_000., 100_000.),
                )
                .size;
            thickness = thickness.max(if axis == Axis::Vertical {
                size.width
            } else {
                size.height
            });
        }
        if position.is_some() && axis == Axis::Horizontal {
            thickness = ToolbarControl::window().row_height(&mut self.text);
        }
        // Layout navigation Close lives in the Window toolbar, not the internal
        // business allocation. No internal Close strip or extra body inset.
        let close = tekes_ui_scene::Size::default();
        // List owns complete, translated member allocations. The window band
        // was already excluded by List's viewport; clipping must not resize a
        // partially scrolled member or make its chrome sticky.
        let window_boundary = if self.is_list_member(index) {
            bounds.min_y()
        } else {
            window_boundary
        };
        Ok(ContainerGeometry::arrange(
            bounds,
            window_boundary,
            position,
            thickness,
            close,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ContainerToolbarHost as ToolbarHost, LocalContainer, OthersOptions, PresentationOptions,
    };
    use std::{cell::Cell, rc::Rc};
    use tekes_ui_controls::composition::{Button, Content};
    fn item(id: u64, title: &str) -> ToolbarItem {
        ToolbarItem {
            id,
            title: title.into(),
            enabled: true,
            geometry_dependent: false,
        }
    }
    fn overlap(a: Rect, b: Rect) -> bool {
        a.intersection(&b)
            .is_some_and(|rect| rect.size.width > 0. && rect.size.height > 0.)
    }
    #[test]
    fn canonical_four_edges_drive_local_paint_and_process_layout_and_pointer_coordinates() {
        for position in [
            Position::Top,
            Position::Bottom,
            Position::Leading,
            Position::Trailing,
        ] {
            let calls = Rc::new(Cell::new(0));
            let count = calls.clone();
            let local =
                LocalContainer::new("local", Content::new(ComponentId(1), Button::new("Body")))
                    .unwrap()
                    .on_action(move |_, records| count.set(count.get() + records.len()));
            let mut local_host = Host::from_columns(vec![local.into()], false, None);
            local_host.mount_columns();
            let mut process = Panel::new(9, "process".into());
            // The original process queue accepts Layout after launch handoff.
            process.launch_tx = None;
            let bounds = Rect::new(100., 0., 600., 500.);
            let geometry = ContainerGeometry::arrange(
                bounds,
                48.,
                Some(position),
                72.,
                tekes_ui_scene::Size::new(48., 42.),
            );
            local_host.panels[0].content_padding = 16.;
            process.content_padding = 16.;
            local_host.panels[0]
                .geometry_with_layout(bounds, 1., geometry)
                .unwrap();
            process.geometry_with_layout(bounds, 1., geometry).unwrap();
            assert_eq!(
                local_host.panels[0].content_frame(),
                process.content_frame()
            );
            let content = process.content_frame();
            assert!(!overlap(content, geometry.toolbar.unwrap()));
            assert!(!overlap(content, geometry.close.unwrap()));
            let layout = process.inbox.queue.lock().unwrap().back().cloned().unwrap();
            assert_eq!(layout.kind, InputKind::Layout);
            assert_eq!((layout.width, layout.height), process.content_size());
            let clip = content.intersection(&bounds).unwrap();
            let panel = &mut local_host.panels[0];
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut local_host.text,
                    content,
                    clip,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            panel.local.as_mut().unwrap().with_host(&mut |host| {
                assert_eq!(host.get(ComponentId(1)).unwrap().frame(), content)
            });
            let point = content.center();
            panel
                .local
                .as_mut()
                .unwrap()
                .event(&ContentEvent::PointerDown(point), &mut local_host.text)
                .unwrap();
            panel
                .local
                .as_mut()
                .unwrap()
                .event(&ContentEvent::PointerUp(point), &mut local_host.text)
                .unwrap();
            assert_eq!(calls.get(), 1);
            let relative = process.content_point(point);
            process
                .request(InputKind::PointerDown, relative.x, relative.y)
                .unwrap();
            let request = process.inbox.queue.lock().unwrap().back().cloned().unwrap();
            assert_eq!(
                (request.x, request.y),
                (content.size.width * 0.5, content.size.height * 0.5)
            );
            assert_eq!(
                (request.width, request.height),
                (layout.width, layout.height)
            );
        }
    }
    fn fixture(position: Option<Position>) -> (Host, Rc<Cell<u32>>) {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let mut owner =
            LocalContainer::new("owner", Content::new(ComponentId(1), Button::new("Body")))
                .unwrap()
                .title("Owner")
                .toolbar(vec![item(1, "One"), item(2, "Two")])
                .on_command(move |_, _| {
                    count.set(count.get() + 1);
                    Ok(())
                });
        if let Some(position) = position {
            owner = owner.toolbar_host(ToolbarHost::Container { position });
        }
        let focus =
            LocalContainer::new("focus", Content::new(ComponentId(1), Button::new("Focus")))
                .unwrap()
                .title("Focus");
        let mut host = Host::from_containers(
            vec![focus.into(), owner.into()],
            false,
            None,
            Some(
                PresentationOptions::new("focus").others(OthersOptions::new(vec!["owner".into()])),
            ),
        );
        host.mount_columns();
        host.container_window_boundary = Some(48.);
        paint(&mut host);
        (host, calls)
    }
    fn paint(host: &mut Host) {
        let area = Rect::new(0., 0., 1200., 700.);
        let layout = host
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(area, 48.);
        host.apply_presentation_layout(layout).unwrap();
        host.sync_container_toolbars(
            true,
            area,
            super::super::window_toolbar_mount::ToolbarBand {
                y: 0.,
                height: 48.,
                native_leading: 0.,
            },
            48.,
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
        // The real native draw reconciles logical toolbar ownership after the
        // original host render, and repaints if that changed focus geometry.
        host.reconcile_toolbar_focus(true);
        if host
            .panels
            .iter()
            .any(|panel| panel.mounted_toolbar.host.needs_render())
        {
            for panel in &mut host.panels {
                super::super::window_presentation::render_test_tabs(
                    &mut panel.mounted_toolbar.host,
                    &mut Scene::new(),
                    &mut host.text,
                    Theme::LIGHT.into(),
                )
                .unwrap();
            }
        }
    }
    fn point(host: &Host, name: &str) -> Point {
        host.panels
            .iter()
            .flat_map(|panel| panel.mounted_toolbar.host.semantics())
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == name)
            .unwrap()
            .1
            .frame
            .center()
    }
    #[test]
    fn original_four_edge_buttons_and_independent_close_keep_capture_through_paint() {
        for position in [
            Position::Top,
            Position::Bottom,
            Position::Leading,
            Position::Trailing,
        ] {
            let (mut host, calls) = fixture(Some(position));
            let first = point(&host, "One");
            let second = point(&host, "Two");
            if matches!(position, Position::Leading | Position::Trailing) {
                assert_eq!(first.x, second.x);
                assert!(first.y < second.y);
            } else {
                assert_eq!(first.y, second.y);
                assert!(first.x < second.x);
            }
            let geometry = host.panels[1].geometry;
            assert!(geometry.close.is_none());
            let owner = host.toolbar_target_at(first).unwrap();
            host.dispatch_toolbar(owner, &ContentEvent::PointerDown(first))
                .unwrap();
            let presenter = host.toolbar_container(owner).unwrap();
            let capture = host.panels[presenter]
                .mounted_toolbar
                .host
                .content_host()
                .pointer_capture();
            paint(&mut host);
            assert_eq!(
                host.panels[presenter]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture(),
                capture
            );
            host.dispatch_toolbar(owner, &ContentEvent::PointerUp(first))
                .unwrap();
            assert_eq!(calls.get(), 1);
            // Close is a normal original Button, with no synthetic semantic parts.
            let close = host
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .find(|node| node.node.semantics.name == "Close Owner")
                .unwrap()
                .node
                .frame
                .center();
            let target = host
                .toolbar_target_at(close)
                .expect("original close Button is a real pointer target");
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(close))
                .unwrap();
            let presenter = host.toolbar_container(target).unwrap();
            let capture = host.panels[presenter]
                .mounted_toolbar
                .host
                .content_host()
                .pointer_capture();
            assert!(capture.is_some(), "Close Down must arm the original Button");
            assert_eq!(host.actual_toolbar_owner(), Some(target));
            paint(&mut host);
            assert_eq!(
                host.panels[presenter]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture(),
                capture,
                "the real draw focus reconciliation must preserve the original Close press"
            );
            assert_eq!(host.actual_toolbar_owner(), Some(target));
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(close))
                .unwrap();
            assert!(host.presentation.as_ref().unwrap().others_ids().is_empty());
        }
    }
    #[test]
    fn close_joins_original_tab_scope_and_space_release_closes_after_real_draw() {
        for position in [
            Position::Top,
            Position::Bottom,
            Position::Leading,
            Position::Trailing,
        ] {
            let (mut host, calls) = fixture(Some(position));
            let target = ToolbarTarget::Contribution {
                presenter: 0,
                group: ComponentId(3_000_010),
            };
            let key = |key, down| ContentKeyEvent {
                key: Some(key),
                down,
                repeat: false,
                modifiers: Default::default(),
                native_code: 0,
                characters: String::new(),
                characters_ignoring_modifiers: String::new(),
            };
            assert!(host.toolbar_focus_targets().contains(&target));
            for _ in 0..host.toolbar_groups().len() {
                assert!(
                    host.toolbar_key_input(key(ControlKey::Tab { reverse: false }, true))
                        .unwrap()
                );
                paint(&mut host);
                if host.actual_toolbar_owner() == Some(target) {
                    break;
                }
            }
            assert_eq!(
                host.actual_toolbar_owner(),
                Some(target),
                "unmodified Tab reaches the original Close Button"
            );
            assert!(
                host.toolbar_key_input(key(ControlKey::Space, true))
                    .unwrap()
            );
            assert_eq!(host.presentation.as_ref().unwrap().others_ids(), &["owner"]);
            paint(&mut host);
            assert_eq!(host.actual_toolbar_owner(), Some(target));
            assert!(
                host.toolbar_key_input(key(ControlKey::Space, false))
                    .unwrap()
            );
            assert!(
                host.presentation.as_ref().unwrap().others_ids().is_empty(),
                "Space release invokes the original Close source"
            );
            assert_eq!(calls.get(), 0, "Close never dispatches a business command");
        }
    }
    #[test]
    fn window_business_is_aggregated_once_and_retained_replaces_it_when_owner_hides() {
        let (mut host, calls) = fixture(None);
        assert!(host.panels[1].geometry.toolbar.is_none());
        assert_eq!(
            host.panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.sources())
                .filter(|source| matches!(
                    source,
                    crate::window_toolbar::Source::Panel { index: 1, .. }
                ))
                .count(),
            1
        );
        let first = point(&host, "One");
        let target = host.toolbar_target_at(first).unwrap();
        assert_eq!(
            host.toolbar_container(target),
            Some(0),
            "Window host aggregates into the focus presenter with original owner source"
        );
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(first))
            .unwrap();
        paint(&mut host);
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(first))
            .unwrap();
        assert_eq!(calls.get(), 1);
        // A held original source is retired before its explicit retained location opens.
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(first))
            .unwrap();
        host.panels[1]
            .options
            .retained_buttons
            .push(crate::RetainedButton {
                command: 1,
                target: "focus".into(),
            });
        host.presentation.as_mut().unwrap().close("owner").unwrap();
        paint(&mut host);
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(first))
            .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(
            host.panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.sources())
                .filter(|source| matches!(
                    source,
                    crate::window_toolbar::Source::Panel { index: 1, .. }
                ))
                .count(),
            1
        );
        assert!(
            host.panels[0].geometry.toolbar.is_none(),
            "an undeclared host remains Window when it receives retained actions"
        );
        assert!(
            !host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .iter()
                .flat_map(|node| &node.parts)
                .any(|(_, part)| part.semantics.name == "Two")
        );
        let retained = point(&host, "One");
        let target = host.toolbar_target_at(retained).unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerDown(retained))
            .unwrap();
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(retained))
            .unwrap();
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn retained_default_and_explicit_window_targets_never_create_an_internal_bar() {
        for explicit in [false, true] {
            let (mut host, calls) = fixture(None);
            if explicit {
                host.panels[0].options.toolbar_host = ToolbarHost::Window;
            }
            host.panels[1].options.retained_buttons = vec![
                crate::RetainedButton {
                    command: 1,
                    target: "focus".into(),
                },
                crate::RetainedButton {
                    command: 2,
                    target: "focus".into(),
                },
            ];
            paint(&mut host);
            let content = host.panels[0].content_frame();
            let source_generation = host.panels[1].toolbar_generation;
            host.presentation.as_mut().unwrap().close("owner").unwrap();
            paint(&mut host);
            assert!(
                host.panels[0].geometry.toolbar.is_none(),
                "retained target default/explicit Window must not invent internal Top"
            );
            assert_eq!(
                host.panels[0].content_frame(),
                content,
                "Window retention must not consume internal body space"
            );
            assert_eq!(host.panels[1].toolbar_generation, source_generation);
            let point = point(&host, "One");
            let target = host.toolbar_target_at(point).unwrap();
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
            paint(&mut host);
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(point))
                .unwrap();
            assert_eq!(calls.get(), 1);
        }
    }
    #[test]
    fn retained_window_rebinding_retires_old_press_and_ax_even_without_physical_move() {
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), Button::new(id)))
                .unwrap()
                .title(id)
        };
        // Both original and retained locations are now in the Others column.
        // Rebinding to Focus must move under the corrected partition contract;
        // a replacement Others instance preserves the genuine same-position case.
        let owner = local("owner")
            .toolbar(vec![item(1, "One"), item(2, "Two")])
            .retain_button(1, "replacement")
            .retain_button(2, "replacement")
            .on_command(move |_, _| {
                count.set(count.get() + 1);
                Ok(())
            });
        let mut host = Host::from_containers(
            vec![
                local("focus").into(),
                owner.into(),
                local("replacement").into(),
            ],
            false,
            None,
            Some(
                PresentationOptions::new("focus").others(OthersOptions::new(vec![
                    "owner".into(),
                    "replacement".into(),
                ])),
            ),
        );
        host.mount_columns();
        host.container_window_boundary = Some(48.);
        paint(&mut host);
        let source = host.panels[0]
            .mounted_toolbar
            .source(ComponentId(92410))
            .unwrap()
            .clone();
        let generation = host.panels[1].toolbar_generation;
        for hide in [true, false] {
            let old_point = point(&host, "One");
            let target = host.toolbar_target_at(old_point).unwrap();
            let old_ax = host.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == "One")
                .unwrap()
                .0;
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(old_point))
                .unwrap();
            assert!(
                host.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture()
                    .is_some()
            );
            if hide {
                host.presentation.as_mut().unwrap().close("owner").unwrap();
            } else {
                let state = host.presentation.as_mut().unwrap();
                state.open("owner").unwrap();
                state
                    .select_tab(crate::OthersTab::Container("owner".into()))
                    .unwrap();
            }
            paint(&mut host);
            assert_eq!(
                point(&host, "One"),
                old_point,
                "Window-to-Window rebind preserves the physical action location"
            );
            assert!(
                host.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .pointer_capture()
                    .is_none(),
                "logical retention rebind must revoke a held original press even in the same region"
            );
            assert!(
                !host.panels[0]
                    .mounted_toolbar
                    .host
                    .accessibility_action(
                        old_ax,
                        tekes_ui_controls::composition::AccessibilityAction::Press
                    )
                    .consumed,
                "old AX target must not execute after logical rebind"
            );
            let rebound = host.toolbar_target_at(old_point).unwrap();
            host.dispatch_toolbar(rebound, &ContentEvent::PointerUp(old_point))
                .unwrap();
            assert_eq!(calls.get(), if hide { 0 } else { 1 });
            assert_eq!(
                host.panels[0]
                    .mounted_toolbar
                    .source(Host::toolbar_group(rebound)),
                Some(&source)
            );
            assert_eq!(host.panels[1].toolbar_generation, generation);
            assert_eq!(
                host.panels
                    .iter()
                    .flat_map(|panel| panel.mounted_toolbar.sources())
                    .filter(|source| matches!(
                        source,
                        crate::window_toolbar::Source::Panel { index: 1, .. }
                    ))
                    .count(),
                1
            );
            let current = host.toolbar_target_at(old_point).unwrap();
            host.dispatch_toolbar(current, &ContentEvent::PointerDown(old_point))
                .unwrap();
            paint(&mut host);
            host.dispatch_toolbar(current, &ContentEvent::PointerUp(old_point))
                .unwrap();
            assert_eq!(calls.get(), if hide { 1 } else { 2 });
        }
    }
    #[test]
    fn retained_explicit_container_target_uses_each_declared_edge() {
        for position in [
            Position::Top,
            Position::Bottom,
            Position::Leading,
            Position::Trailing,
        ] {
            let (mut host, calls) = fixture(None);
            host.panels[0].options.toolbar_host = ToolbarHost::Container { position };
            host.panels[1].options.retained_buttons = vec![
                crate::RetainedButton {
                    command: 1,
                    target: "focus".into(),
                },
                crate::RetainedButton {
                    command: 2,
                    target: "focus".into(),
                },
            ];
            host.presentation.as_mut().unwrap().close("owner").unwrap();
            paint(&mut host);
            let bar = host.panels[0]
                .geometry
                .toolbar
                .expect("explicit Container target mounts its retained business bar");
            let body = host.panels[0].content_frame();
            assert!(!overlap(bar, body));
            assert!(
                host.panels[0].geometry.close.is_none(),
                "retention never gives Focus a Close action"
            );
            let first = point(&host, "One");
            let second = point(&host, "Two");
            match position {
                Position::Top => assert!(bar.max_y() <= body.min_y()),
                Position::Bottom => assert!(bar.min_y() >= body.max_y()),
                Position::Leading => assert!(bar.max_x() <= body.min_x()),
                Position::Trailing => assert!(bar.min_x() >= body.max_x()),
            }
            if matches!(position, Position::Leading | Position::Trailing) {
                assert_eq!(first.x, second.x);
                assert!(first.y < second.y);
            } else {
                assert_eq!(first.y, second.y);
                assert!(first.x < second.x);
            }
            let target = host.toolbar_target_at(first).unwrap();
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(first))
                .unwrap();
            paint(&mut host);
            host.dispatch_toolbar(target, &ContentEvent::PointerUp(first))
                .unwrap();
            assert_eq!(calls.get(), 1);
        }
    }
    #[test]
    fn partially_scrolled_list_keeps_member_allocation_and_process_viewport_and_clips_chrome() {
        use crate::{OthersTab, ProcessContainer};
        let focus =
            LocalContainer::new("focus", Content::new(ComponentId(1), Button::new("Focus")))
                .unwrap();
        let remote = ProcessContainer::new(crate::ModuleDefinition {
            id: "remote",
            create: || panic!("geometry regression does not launch a child"),
        })
        .title("Remote")
        .list_height(320.)
        .toolbar_host(ToolbarHost::Container {
            position: Position::Top,
        });
        let second = LocalContainer::new(
            "second",
            Content::new(ComponentId(1), Button::new("Second")),
        )
        .unwrap()
        .list_height(320.);
        let mut host = Host::from_containers(
            vec![focus.into(), remote.into(), second.into()],
            false,
            None,
            Some(
                PresentationOptions::new("focus")
                    .others(OthersOptions::new(vec!["remote".into(), "second".into()])),
            ),
        );
        host.mount_columns();
        // Same compiled-module toolbar declaration shape as a real publication;
        // Layout/pointer observations below use the original Process queue.
        host.panels[1].toolbar = vec![item(1, "Remote action")];
        host.panels[1].launch_tx = None;
        host.container_window_boundary = Some(48.);
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::List)
            .unwrap();
        let paint_list = |host: &mut Host| {
            let area = Rect::new(0., 0., 1200., 400.);
            let layout = host
                .presentation
                .as_ref()
                .unwrap()
                .layout_with_toolbar(area, 48.);
            host.apply_presentation_layout(layout).unwrap();
            host.sync_container_toolbars(
                true,
                area,
                super::super::window_toolbar_mount::ToolbarBand {
                    y: 0.,
                    height: 48.,
                    native_leading: 0.,
                },
                48.,
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
        };
        paint_list(&mut host);
        let before = host.panels[1].geometry;
        let content_before = host.panels[1].content_frame();
        let bounds_before = host.panels[1].bounds;
        let viewport_before = host.panels[1].viewport;
        let layout_count = host.panels[1]
            .inbox
            .queue
            .lock()
            .unwrap()
            .iter()
            .filter(|request| request.kind == InputKind::Layout)
            .count();
        assert!(
            layout_count > 0,
            "the ready original Process queue observed initial Layout"
        );
        assert_eq!(bounds_before.min_y(), 48.);
        host.presentation.as_mut().unwrap().scroll_list(20., 352.);
        paint_list(&mut host);
        let panel = &host.panels[1];
        let after = panel.geometry;
        assert_eq!(
            panel.bounds.min_y(),
            28.,
            "the original allocation oracle translates the whole member"
        );
        assert_eq!(
            panel.content_frame().size,
            content_before.size,
            "partial visibility cannot shrink process content"
        );
        assert_eq!(panel.content_frame().min_y(), content_before.min_y() - 20.);
        assert_eq!(
            after.toolbar.unwrap().min_y(),
            before.toolbar.unwrap().min_y() - 20.
        );
        assert!(before.close.is_none() && after.close.is_none());
        assert_eq!(
            panel.viewport, viewport_before,
            "pure origin translation does not advance the Process Layout viewport"
        );
        assert_eq!(
            panel
                .inbox
                .queue
                .lock()
                .unwrap()
                .iter()
                .filter(|request| request.kind == InputKind::Layout)
                .count(),
            layout_count,
            "scroll can revoke input capture but must not enqueue another Layout"
        );
        let control = panel
            .mounted_toolbar
            .host
            .content_host()
            .get(Host::toolbar_control(ToolbarTarget::Panel(1)))
            .unwrap();
        let outside = Point::new(control.frame().center().x, panel.clip.min_y() - 1.);
        assert!(control.frame().contains(outside));
        assert!(
            !control
                .visible()
                .is_some_and(|visible| visible.contains(outside))
        );
        assert!(
            host.toolbar_target_at(outside).is_none(),
            "clipped part of the translated original toolbar is not a hit target"
        );
        assert!(
            panel
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .all(|node| node.node.semantics.name != "Close Remote"),
            "aggregate List members do not create an internal Close strip"
        );
        let visible = control.visible().unwrap().center();
        assert_eq!(
            host.toolbar_target_at(visible),
            Some(ToolbarTarget::Panel(1)),
            "the visible portion retains its original owner"
        );
    }
}
