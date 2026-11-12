//! Header-only original Tabs; PresentationState owns every destination.
use super::*;
use crate::OthersTab;
use tekes_ui_controls::{
    ScrollArea,
    composition::{
        Button, Content, IconContent, MenuContent, MenuEntry, ScrollContent, TabStripContent,
        TabStripItem, TextContent, ToolbarCommandBinding, ToolbarGroup, ToolbarHost,
        ToolbarPlacement, ToolbarRegion, ToolbarRegistration,
    },
};
const OWNER: u64 = u64::MAX - 1;
const GROUP: ComponentId = ComponentId(999_999);
const SCROLL: ComponentId = ComponentId(999_996);
const STRIP: ComponentId = ComponentId(999_995);
const ROW: ComponentId = ComponentId(999_994);
const CREATE: ComponentId = ComponentId(999_992);
const GRID_REGION: ComponentId = ComponentId(u64::MAX - 100);
const GRID_GROUP: ComponentId = ComponentId(1_100_000);
#[derive(Clone, Debug)]
pub(super) enum SwitcherAction {
    Tabs,
    Menu,
    Create(String),
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Item {
    pub(super) key: ComponentId,
    title: String,
    pub(super) tab: OthersTab,
    selected: bool,
}
#[derive(Default)]
pub(super) struct SwitcherState {
    pub(super) items: Vec<Item>,
    grid: Vec<MenuEntry>,
    creation: Vec<MenuEntry>,
    width: f32,
    viewport: Rect,
}
fn icon(id: ComponentId, name: &str) -> Result<Content> {
    Ok(Content::new(
        id,
        IconContent::new(name).map_err(|e| anyhow::anyhow!("switcher icon: {e:?}"))?,
    ))
}
impl Host {
    /// Opt-in evidence for the native header scroll path; never logs labels or text.
    pub(super) fn trace_presentation_scroll(&self, stage: &str) {
        if std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_none() {
            return;
        }
        let Some(host) = &self.presentation_tabs else {
            eprintln!("OTHERS_SCROLL_STATE stage={stage} mounted=false");
            return;
        };
        let Some(node) = host.content_host().get(SCROLL) else {
            return;
        };
        let Some(scroll) = node.component::<ScrollContent>() else {
            return;
        };
        let selected = host
            .content_host()
            .get(STRIP)
            .and_then(|node| node.component::<TabStripContent>())
            .map(TabStripContent::selected);
        eprintln!(
            "OTHERS_SCROLL_STATE stage={stage} offset={:?} visual={:?} viewport={:?} document={:?} frame={:?} selected={selected:?} capture={} overlay={}",
            scroll.scroll.offset,
            scroll.scroll.visual_offset(),
            scroll.scroll.viewport_size(),
            scroll.scroll.document_size(),
            node.frame(),
            host.content_host().pointer_capture().is_some(),
            host.content_host().input_overlay_owner().is_some()
        );
    }
    fn switcher_items(&self) -> Vec<Item> {
        let state = self.presentation.as_ref().unwrap();
        state
            .tabs()
            .into_iter()
            .map(|tab| {
                let (key, title) = match &tab {
                    OthersTab::Container(id) => {
                        let panel = self
                            .panels
                            .iter()
                            .find(|panel| panel.module_id == *id)
                            .unwrap();
                        (
                            ComponentId(2_000_000 + panel.instance),
                            panel.options.title.clone().unwrap_or_else(|| id.clone()),
                        )
                    }
                    OthersTab::List => (ComponentId(999_990), "List".into()),
                };
                Item {
                    key,
                    title,
                    selected: state.selected_tab() == Some(&tab),
                    tab,
                }
            })
            .collect()
    }
    pub(super) fn presentation_creation_frame(&self) -> Option<Rect> {
        let state = self.presentation.as_ref()?;
        if !state.others_visible()
            || !state.others_ids().is_empty()
            || self.creation_choices().is_empty()
        {
            return None;
        }
        let tabs = self.presentation_layout.tabs?;
        let height = self
            .presentation_layout
            .frames
            .iter()
            .map(|(_, frame)| frame.max_y())
            .fold(tabs.max_y(), f32::max);
        Some(Rect::new(
            tabs.min_x(),
            self.container_window_boundary.unwrap_or(tabs.max_y()),
            tabs.size.width,
            (height - self.container_window_boundary.unwrap_or(tabs.max_y())).max(0.),
        ))
    }
    pub(super) fn presentation_switcher_regions(
        &self,
        frame: Rect,
        whole: Rect,
    ) -> Vec<ToolbarRegion> {
        let mut regions = vec![ToolbarRegion::new(ComponentId(OWNER), frame)];
        if let Some(mut frame) = self.presentation_creation_frame() {
            frame.size.height = (whole.max_y() - frame.min_y()).max(0.);
            let pad = tekes_ui_tokens::Spacing::Loose.value();
            frame = Rect::new(
                frame.min_x() + pad,
                frame.min_y() + pad,
                (frame.size.width - pad * 2.).max(0.),
                (frame.size.height - pad * 2.).max(0.),
            );
            let mut region = ToolbarRegion::new(GRID_REGION, frame);
            region.axis = Axis::Vertical;
            regions.push(region);
        }
        regions
    }
    pub(super) fn sync_presentation_switcher(&mut self, whole: Rect) -> Result<()> {
        let Some(frame) = self.presentation_switcher_frame() else {
            return Ok(());
        };
        let visible = self.switcher_items();
        let grid = if self.presentation_creation_frame().is_some() {
            self.creation_entries()
        } else {
            Vec::new()
        };
        let creation = self.creation_entries();
        let regular = tekes_ui_tokens::Spacing::Regular.value();
        let structural = self.presentation_tabs.is_none()
            || self
                .presentation_switcher
                .items
                .iter()
                .map(|item| (&item.key, &item.title))
                .ne(visible.iter().map(|item| (&item.key, &item.title)))
            || self.presentation_switcher.creation != creation;
        let selected = visible.iter().position(|item| item.selected).unwrap_or(0);
        let uncommitted_selection = self
            .presentation_tabs
            .as_ref()
            .and_then(|host| host.content_host().get(STRIP))
            .and_then(|node| node.component::<TabStripContent>())
            .is_some_and(|strip| strip.selected() != selected);
        let selection_changed = uncommitted_selection
            || structural
            || self.presentation_switcher.width != frame.size.width
            || self
                .presentation_switcher
                .items
                .iter()
                .map(|item| item.selected)
                .ne(visible.iter().map(|item| item.selected));
        if self.presentation_tabs.is_none() {
            self.presentation_tabs = Some(
                ToolbarHost::new(ComponentId(999_998), OWNER, whole)
                    .map_err(|e| anyhow::anyhow!("switcher toolbar: {e:?}"))?,
            );
        }
        let choices = self.creation_choices();
        let regions = self.presentation_switcher_regions(frame, whole);
        let host = self.presentation_tabs.as_mut().unwrap();
        host.set_regions(OWNER, whole, regions)
            .map_err(|e| anyhow::anyhow!("switcher regions: {e:?}"))?;
        if structural {
            let items = visible
                .iter()
                .map(|item| TabStripItem {
                    label: Content::new(item.key, TextContent::new(&item.title)),
                    name: item.title.clone(),
                    closable: matches!(item.tab, OthersTab::Container(_)),
                })
                .collect();
            let mut strip = TabStripContent::new(items, selected);
            // Resolve the reveal range using the same shared natural-title layout.
            tekes_ui_controls::composition::Component::measure(
                &mut strip,
                &mut self.text,
                frame.size,
            );
            let selected_rect = strip.selected_rect();
            let mut children = vec![Content::new(STRIP, strip)];
            let mut bindings = vec![ToolbarCommandBinding {
                target: STRIP,
                action: None,
                command: 0,
                source: SwitcherAction::Tabs,
            }];
            if !creation.is_empty() {
                let mut menu = MenuContent::new("Create Container", Vec::new())
                    .with_trigger_content(
                        "Create Container",
                        icon(ComponentId(CREATE.0 + 1), "tekes-new")?,
                    )
                    .map_err(|e| anyhow::anyhow!("creation trigger: {e:?}"))?;
                menu.set_popup_viewport(Some(whole));
                menu.trigger.variant = tekes_ui_controls::paint::ButtonVariant::Toolbar;
                menu.trigger.set_selection_fill_only(true);
                menu.trigger.set_toolbar_icon_slot(true);
                menu.set_entries(creation.clone())
                    .map_err(|e| anyhow::anyhow!("creation menu: {e:?}"))?;
                children.push(Content::new(CREATE, menu));
                bindings.push(ToolbarCommandBinding {
                    target: CREATE,
                    action: None,
                    command: 0,
                    source: SwitcherAction::Menu,
                });
            }
            let row = Content::new(
                ROW,
                ToolbarGroup::new(ToolbarPlacement::Navigation, children),
            );
            let mut scroll = ScrollContent::new(ScrollArea::default(), row).horizontal(true);
            scroll.reveal_horizontal(selected_rect);
            host.register(ToolbarRegistration {
                region: ComponentId(OWNER),
                order: 0,
                group: Content::new(
                    GROUP,
                    ToolbarGroup::new(
                        ToolbarPlacement::Navigation,
                        vec![Content::new(SCROLL, scroll)],
                    ),
                ),
                bindings,
            })
            .map_err(|e| anyhow::anyhow!("switcher strip: {e:?}"))?;
        } else if selection_changed {
            let mut rect = Rect::default();
            host.update_component::<TabStripContent>(GROUP, STRIP, |strip| {
                strip.select(selected);
                rect = strip.selected_rect();
            })
            .map_err(|e| anyhow::anyhow!("switcher selection: {e:?}"))?;
            host.update_component::<ScrollContent>(GROUP, SCROLL, |scroll| {
                scroll.reveal_horizontal(rect)
            })
            .map_err(|e| anyhow::anyhow!("switcher reveal: {e:?}"))?;
        }
        if !structural && self.presentation_switcher.viewport != whole && !creation.is_empty() {
            host.update_component::<MenuContent>(GROUP, CREATE, |menu| {
                menu.set_popup_viewport(Some(whole))
            })
            .map_err(|e| anyhow::anyhow!("creation popup viewport: {e:?}"))?;
        }
        if self.presentation_switcher.grid != grid {
            if !grid.is_empty() {
                let mut children = Vec::new();
                let mut bindings = Vec::new();
                for (index, entry) in grid.iter().enumerate() {
                    let MenuEntry::Item {
                        value,
                        label,
                        disabled,
                        ..
                    } = entry
                    else {
                        unreachable!()
                    };
                    let (_, _, image, _) =
                        choices.iter().find(|(id, _, _, _)| id == value).unwrap();
                    let id = ComponentId(GRID_GROUP.0 + 10 + index as u64 * 10);
                    let body = Content::new(
                        ComponentId(id.0 + 1),
                        ToolbarGroup::new(
                            ToolbarPlacement::Navigation,
                            vec![
                                icon(ComponentId(id.0 + 2), image)?,
                                Content::new(
                                    ComponentId(id.0 + 3),
                                    tekes_ui_controls::composition::TextContent::new(label.clone()),
                                ),
                            ],
                        ),
                    );
                    let mut button =
                        Button::new(body).with_accessible_name(&if value.starts_with("open:") {
                            label.clone()
                        } else {
                            format!("Create {label}")
                        });
                    button.button.state.disabled = *disabled;
                    children.push(Content::new(id, button));
                    bindings.push(ToolbarCommandBinding {
                        target: id,
                        action: None,
                        command: 0,
                        source: SwitcherAction::Create(value.clone()),
                    });
                }
                let mut model = tekes_ui_controls::extended::Grid::new(2);
                model.gap_x = regular;
                model.gap_y = regular;
                let grid_content = Content::new(
                    ComponentId(GRID_GROUP.0 + 1),
                    tekes_ui_controls::composition::GridContent { model, children },
                );
                host.register(ToolbarRegistration {
                    region: GRID_REGION,
                    order: 0,
                    group: Content::new(
                        GRID_GROUP,
                        ToolbarGroup::new(ToolbarPlacement::Navigation, vec![grid_content]),
                    ),
                    bindings,
                })
                .map_err(|e| anyhow::anyhow!("creation grid: {e:?}"))?;
            } else if host.content_host().get(GRID_GROUP).is_some() {
                host.remove(GRID_GROUP)
                    .map_err(|e| anyhow::anyhow!("creation grid remove: {e:?}"))?;
            }
        }
        self.presentation_switcher = SwitcherState {
            items: visible,
            grid,
            creation,
            width: frame.size.width,
            viewport: whole,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, LocalContainer, OthersGroup, OthersOptions, PresentationOptions};
    use tekes_ui_controls::composition::{
        AccessibilityAction, TextContent, ToolbarAccessibilityTarget,
    };
    fn members() -> Vec<Container> {
        ["focus", "inspector", "first", "second"]
            .into_iter()
            .enumerate()
            .map(|(index, id)| {
                LocalContainer::new(
                    id,
                    Content::new(
                        ComponentId(1),
                        TextContent::new(format!("Original {index}")),
                    ),
                )
                .unwrap()
                .title(id)
                .switcher_icon("tekes-file-text")
                .into()
            })
            .collect()
    }
    fn options() -> PresentationOptions {
        PresentationOptions::new("focus").others(
            OthersOptions::new(vec!["inspector".into(), "first".into(), "second".into()]).groups(
                vec![OthersGroup::new(
                    "documents",
                    "Documents",
                    "tekes-files",
                    vec!["focus".into(), "first".into(), "second".into()],
                )],
            ),
        )
    }
    fn fixture() -> Host {
        let mut host = Host::from_containers(members(), false, None, Some(options()));
        assert!(host.mount_columns());
        host
    }
    fn paint(host: &mut Host, width: f32) {
        let whole = Rect::new(0., 0., 1200., 700.);
        let layout = host
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, 48.);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (panel, frame) in host.panels.iter_mut().zip(frames) {
            panel.bounds = frame;
        }
        if let Some(frame) = &mut host.presentation_layout.tabs {
            frame.size.width = width;
        }
        host.paint_presentation(&mut Scene::new(), whole, Theme::LIGHT.into())
            .unwrap();
    }
    fn target(host: &Host, name: &str) -> (ToolbarAccessibilityTarget, Rect) {
        host.presentation_tabs
            .as_ref()
            .unwrap()
            .semantics()
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
            .unwrap_or_else(|| panic!("missing switcher target {name}"))
    }
    fn native_pointer(host: &mut Host, point: Point, down: bool) {
        let native = Event::Mouse(MouseEvent {
            kind: if down {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                }
            } else {
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                }
            },
            position: tekes_ui_platform_macos::Point {
                x: point.x as f64,
                y: point.y as f64,
            },
            modifiers: Default::default(),
        });
        let input = super::super::window_toolbar_mount::native_toolbar_event(&native).unwrap();
        assert!(host.presentation_tab_input(&input).unwrap());
    }
    fn click_with_paint(host: &mut Host, name: &str, width: f32) {
        let point = target(host, name).1.center();
        native_pointer(host, point, true);
        let capture = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .pointer_capture();
        let mount = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(GROUP)
            .unwrap()
            .mount_identity();
        paint(host, width);
        assert_eq!(
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .content_host()
                .get(GROUP)
                .unwrap()
                .mount_identity(),
            mount,
            "unchanged paint must not remount the original trigger"
        );
        assert_eq!(
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .content_host()
                .pointer_capture(),
            capture,
            "paint must preserve the original press"
        );
        native_pointer(host, point, false);
        paint(host, width);
    }
    #[test]
    fn t64_native_horizontal_wheel_survives_header_repaint_in_both_directions() {
        use tekes_ui_platform_macos::{ScrollEvent, ScrollPhase, Vec2};
        let mut host = fixture();
        paint(&mut host, 180.);
        let scroll_offset = |host: &Host| {
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .content_host()
                .get(SCROLL)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset
                .x
        };
        let initial = scroll_offset(&host);
        let original_title = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(host.presentation_switcher.items[0].key)
            .unwrap()
            .frame();
        let point = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(SCROLL)
            .unwrap()
            .frame()
            .center();
        for (delta, timestamp) in [(-0.1, 1.), (0.1, 2.)] {
            let event = Event::Scroll(ScrollEvent {
                timestamp,
                delta: Vec2 { x: delta, y: 0. },
                precise: false,
                phase: ScrollPhase::None,
                momentum_phase: ScrollPhase::None,
                inverted: false,
                position: tekes_ui_platform_macos::Point {
                    x: point.x as f64,
                    y: point.y as f64,
                },
                modifiers: Default::default(),
            });
            assert!(host.presentation_layout.tabs.unwrap().contains(point));
            let input = super::super::window_toolbar_mount::native_toolbar_event(&event).unwrap();
            assert!(host.presentation_tab_input(&input).unwrap());
            let after_input = scroll_offset(&host);
            paint(&mut host, 180.);
            assert_eq!(
                scroll_offset(&host),
                after_input,
                "draw must not reveal the selected tab over a user wheel"
            );
            let title = host
                .presentation_tabs
                .as_ref()
                .unwrap()
                .content_host()
                .get(host.presentation_switcher.items[0].key)
                .unwrap()
                .frame();
            if delta < 0. {
                let step = tekes_ui_tokens::proposal::metric("size", "size-row-height").unwrap();
                assert!((after_input - initial - 0.1 * step).abs() < 0.0001);
                assert!(title.min_x() < original_title.min_x());
            } else {
                assert_eq!(after_input, initial);
                assert_eq!(title.min_x(), original_title.min_x());
            }
        }
    }
    #[test]
    fn t64_each_instance_has_text_tab_and_close_does_not_select_sibling() {
        let mut host = fixture();
        let original: Vec<_> = host.panels.iter().map(|panel| panel.instance).collect();
        paint(&mut host, 600.);
        assert_eq!(host.presentation_switcher.items.len(), 3);
        let before = host.presentation.as_ref().unwrap().selected_tab().cloned();
        click_with_paint(&mut host, "Close second", 600.);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab().cloned(),
            before
        );
        assert!(
            !host
                .presentation
                .as_ref()
                .unwrap()
                .others_ids()
                .contains(&"second".into())
        );
        assert_eq!(
            host.panels
                .iter()
                .map(|panel| panel.instance)
                .collect::<Vec<_>>(),
            original
        );
        click_with_paint(&mut host, "first", 600.);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("first".into()))
        );
    }
    #[test]
    fn t64_horizontal_scroll_reveals_selection_and_rejects_retired_ax_close() {
        let mut host = fixture();
        paint(&mut host, 600.);
        let old = target(&host, "Close first").0;
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("second".into()))
            .unwrap();
        paint(&mut host, 120.);
        let header = host.presentation_tabs.as_ref().unwrap();
        let scroll = header
            .content_host()
            .get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap();
        assert!(scroll.scroll.document_size().width > scroll.scroll.viewport_size().width);
        assert!(scroll.scroll.offset.x > 0.);
        assert!(target(&host, "second").1.size.width > 0.);
        host.close_container("first").unwrap();
        paint(&mut host, 120.);
        assert!(
            !host
                .presentation_tabs
                .as_mut()
                .unwrap()
                .accessibility_action(old, AccessibilityAction::Press)
                .consumed
        );
        assert!(
            host.presentation
                .as_ref()
                .unwrap()
                .others_ids()
                .contains(&"second".into())
        );
    }
    #[test]
    fn t64_rejected_ime_selection_reprojects_authoritative_tab() {
        use tekes_ui_controls::composition::TextFieldContent;
        let mut members = members();
        members[0] = LocalContainer::new(
            "focus",
            Content::new(ComponentId(1), TextFieldContent::new("Editor", "")),
        )
        .unwrap()
        .into();
        let mut host = Host::from_containers(members, false, None, Some(options()));
        host.mount_columns();
        paint(&mut host, 600.);
        host.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| {
                let frame = Rect::new(0., 50., 500., 300.);
                content
                    .render(
                        &mut Scene::new(),
                        &mut host.text,
                        frame,
                        frame,
                        Theme::LIGHT.into(),
                    )
                    .unwrap();
                assert!(content.focus(Some(ComponentId(1)), true));
                content.event(&ContentEvent::Preedit {
                    text: "かな".into(),
                    selection: 0..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: None,
                });
            });
        assert!(host.composition_in_progress());
        let before = host.presentation.as_ref().unwrap().selected_tab().cloned();
        let other = target(&host, "first").0;
        assert!(
            host.presentation_tabs
                .as_mut()
                .unwrap()
                .accessibility_action(other, AccessibilityAction::Press)
                .consumed
        );
        host.apply_tab_selection().unwrap();
        paint(&mut host, 600.);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab().cloned(),
            before
        );
        let strip = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(STRIP)
            .unwrap()
            .component::<TabStripContent>()
            .unwrap();
        assert_eq!(strip.selected(), 0);
        assert!(host.composition_in_progress());
    }
    #[test]
    fn t64_title_lanes_are_natural_and_bounded_without_more_menu() {
        let mut host = fixture();
        host.panels[1].options.title = Some("I".into());
        host.panels[2].options.title =
            Some("A deliberately very long title repeated beyond the title maximum".into());
        paint(&mut host, 600.);
        let short = target(&host, "I").1;
        let long = target(&host, host.panels[2].options.title.as_deref().unwrap()).1;
        assert!(long.size.width > short.size.width);
        assert!(long.size.width < tekes_ui_controls::composition::TAB_TITLE_MAX_WIDTH + 80.);
        assert!(
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .semantics()
                .iter()
                .all(|node| node.node.semantics.name != "More containers")
        );
    }
    #[test]
    fn aggregate_list_has_no_close_all_or_internal_close_and_focus_is_not_closable() {
        let members = members()
            .into_iter()
            .map(|member| match member {
                Container::Local(container) if container.id() != "focus" => container
                    .presentation(crate::ContainerPresentation::List { height: 180. })
                    .into(),
                member => member,
            })
            .collect();
        let mut host = Host::from_containers(members, false, None, Some(options()));
        host.mount_columns();
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::List)
            .unwrap();
        paint(&mut host, 600.);
        let whole = Rect::new(0., 0., 1200., 700.);
        host.sync_container_toolbars(
            true,
            whole,
            super::super::window_toolbar_mount::ToolbarBand {
                y: 0.,
                height: 48.,
                native_leading: 0.,
            },
            48.,
        )
        .unwrap();
        assert!(
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .semantics()
                .iter()
                .flat_map(|node| &node.parts)
                .all(|(_, part)| !part.semantics.name.starts_with("Close "))
        );
        for index in 1..host.panels.len() {
            let panel = &mut host.panels[index];
            super::super::window_presentation::render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
            assert!(
                panel
                    .mounted_toolbar
                    .host
                    .semantics()
                    .into_iter()
                    .all(|node| node.node.semantics.name != format!("Close {}", panel.module_id))
            );
            assert!(panel.geometry.close.is_none());
        }
    }
    #[test]
    fn group_declarations_reject_unknown_or_duplicate_members_before_mounting() {
        let mut unknown = options();
        unknown.others.as_mut().unwrap().groups[0]
            .containers
            .push("missing".into());
        assert!(crate::PresentationState::new(&members(), unknown).is_err());
        let mut duplicate = options();
        duplicate
            .others
            .as_mut()
            .unwrap()
            .groups
            .push(OthersGroup::new(
                "other",
                "Other",
                "tekes-note",
                vec!["first".into()],
            ));
        assert!(crate::PresentationState::new(&members(), duplicate).is_err());
        let mut invalid = options();
        invalid.others.as_mut().unwrap().groups[0].icon = "unknown".into();
        assert!(crate::PresentationState::new(&members(), invalid).is_err());
    }
}
