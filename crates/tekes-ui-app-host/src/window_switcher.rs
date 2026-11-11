//! Declared destination families presented by the original Toolbar/Button/Menu.
//! This header owns no container content and never stores a second selection.
use super::*;
use crate::OthersTab;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        Button, Content, IconContent, MenuContent, MenuEntry, ToolbarCommandBinding, ToolbarGroup,
        ToolbarHost, ToolbarPlacement, ToolbarRegion, ToolbarRegistration,
    },
    paint::ButtonVariant,
};
use tekes_ui_tokens::proposal;

const OWNER: u64 = u64::MAX - 1;
const GROUP: ComponentId = ComponentId(999_999);
const MORE: &str = ":more";
#[derive(Clone, Debug)]
pub(super) enum SwitcherAction {
    Select(OthersTab),
    Menu,
}
#[derive(Clone, Debug, PartialEq)]
struct Item {
    key: String,
    title: String,
    icon: String,
    tab: Option<OthersTab>,
    entries: Vec<MenuEntry>,
    selected: bool,
}
#[derive(Default)]
pub(super) struct SwitcherState {
    items: Vec<Item>,
}
fn entry(tab: &OthersTab, title: &str, selected: Option<&OthersTab>) -> MenuEntry {
    MenuEntry::Item {
        value: match tab {
            OthersTab::Container(id) => id.clone(),
            OthersTab::List => ":list".into(),
        },
        label: title.into(),
        checked: Some(selected == Some(tab)),
        disabled: false,
    }
}
fn slot(index: usize) -> ComponentId {
    ComponentId(1_000_000 + index as u64 * 10)
}
fn icon(id: ComponentId, name: &str) -> Result<Content> {
    Ok(Content::new(
        id,
        IconContent::new(name).map_err(|e| anyhow::anyhow!("switcher icon: {e:?}"))?,
    ))
}
fn control(item: &Item, id: ComponentId) -> Result<Content> {
    let mut button = if item.tab.is_some() {
        let mut button = Button::new(icon(ComponentId(id.0 + 1), &item.icon)?)
            .with_accessible_name(&item.title)
            .with_help(&item.title);
        button.variant = ButtonVariant::Toolbar;
        button.set_selection_fill_only(true);
        button.set_toolbar_icon_slot(true);
        button.button.state.selected = item.selected;
        return Ok(Content::new(id, button));
    } else {
        let trigger = if item.key == MORE {
            icon(ComponentId(id.0 + 1), &item.icon)?
        } else {
            Content::new(
                ComponentId(id.0 + 1),
                ToolbarGroup::new(
                    ToolbarPlacement::Navigation,
                    vec![
                        icon(ComponentId(id.0 + 2), &item.icon)?,
                        icon(ComponentId(id.0 + 3), "tekes-expand")?,
                    ],
                ),
            )
        };
        MenuContent::new(&item.title, Vec::new())
            .with_trigger_content(&item.title, trigger)
            .map_err(|e| anyhow::anyhow!("switcher trigger: {e:?}"))?
    };
    button.trigger.variant = ButtonVariant::Toolbar;
    button.trigger.set_selection_fill_only(true);
    button.trigger.set_toolbar_icon_slot(true);
    button.trigger.set_help(Some(item.title.clone()));
    button.trigger.button.state.selected = item.selected;
    button
        .set_entries(item.entries.clone())
        .map_err(|e| anyhow::anyhow!("switcher menu: {e:?}"))?;
    Ok(Content::new(id, button))
}
fn more(hidden: &[Item]) -> Item {
    let entries: Vec<_> = hidden
        .iter()
        .map(|item| {
            if let Some(tab) = &item.tab {
                entry(tab, &item.title, item.selected.then_some(tab))
            } else {
                MenuEntry::submenu(&item.key, &item.title, item.entries.clone())
            }
        })
        .collect();
    Item {
        key: MORE.into(),
        title: "More containers".into(),
        icon: "tekes-more".into(),
        tab: None,
        selected: hidden.iter().any(|item| item.selected),
        entries,
    }
}
/// Spend the real measured slot on the selected family first, then preserve the
/// declaration order. More always retains an entry for every hidden destination.
fn fit(widths: &[f32], selected: Option<usize>, width: f32, more: f32, gap: f32) -> Vec<usize> {
    if widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32 <= width {
        return (0..widths.len()).collect();
    }
    let mut chosen = Vec::new();
    let mut remaining = (width - more).max(0.);
    for index in selected.into_iter().chain(0..widths.len()) {
        if chosen.contains(&index) {
            continue;
        }
        let cost = widths[index] + gap;
        if cost <= remaining {
            chosen.push(index);
            remaining -= cost;
        }
    }
    chosen.sort_unstable();
    chosen
}
impl Host {
    fn switcher_items(&self) -> Vec<Item> {
        let state = self.presentation.as_ref().unwrap();
        let selected = state.selected_tab();
        let Some(options) = state.others() else {
            return Vec::new();
        };
        let title = |id: &str| {
            self.panels
                .iter()
                .find(|panel| panel.module_id == id)
                .and_then(|panel| panel.options.title.as_deref())
                .unwrap_or(id)
                .to_owned()
        };
        let mut items: Vec<Item> = Vec::new();
        for tab in state.tabs() {
            match tab {
                OthersTab::Container(ref id) => {
                    if let Some(group) = options
                        .groups
                        .iter()
                        .find(|group| group.containers.contains(id))
                    {
                        let key = format!(":group:{}", group.id);
                        if items.iter().any(|item| item.key == key) {
                            continue;
                        }
                        items.push(Item { key, title: group.title.clone(), icon: {
                            let current = selected.and_then(|tab| match tab { OthersTab::Container(id) if group.containers.contains(id) => Some(id.as_str()), _ => None })
                                .or_else(|| group.containers.iter().find(|id| state.others_ids().contains(id)).map(String::as_str));
                            current.and_then(|id| self.panels.iter().find(|panel| panel.module_id == id)).and_then(|panel| panel.options.switcher_icon.clone())
                                .unwrap_or_else(|| "tekes-file-generic".into())
                        }, tab: None,
                            entries: group.containers.iter().filter(|id| state.others_ids().contains(id))
                                .map(|id| entry(&OthersTab::Container(id.clone()), &title(id), selected)).collect(),
                            selected: selected.is_some_and(|tab| matches!(tab, OthersTab::Container(id) if group.containers.contains(id))),
                        });
                    } else {
                        items.push(Item {
                            key: id.clone(),
                            title: title(id),
                            icon: self
                                .panels
                                .iter()
                                .find(|panel| panel.module_id == *id)
                                .and_then(|panel| panel.options.switcher_icon.clone())
                                .unwrap_or_else(|| "tekes-file-generic".into()),
                            selected: selected == Some(&tab),
                            tab: Some(tab),
                            entries: Vec::new(),
                        });
                    }
                }
                OthersTab::List => items.push(Item {
                    key: ":list".into(),
                    title: "List".into(),
                    icon: "tekes-diff-list".into(),
                    selected: selected == Some(&tab),
                    tab: Some(tab),
                    entries: Vec::new(),
                }),
            }
        }
        items
    }
    pub(super) fn sync_presentation_switcher(&mut self, whole: Rect) -> Result<()> {
        let Some(frame) = self.presentation_switcher_frame() else {
            return Ok(());
        };
        let all = self.switcher_items();
        if all.is_empty() {
            self.presentation_tabs = None;
            self.presentation_switcher = SwitcherState::default();
            return Ok(());
        }
        let scale = self.text.resolved_font_size(proposal::TextRole::UiBody)
            / proposal::TOOLBAR_REFERENCE_TITLE_SIZE;
        let tight = proposal::metric("spacing", "space-tight").unwrap() * scale;
        let regular = proposal::metric("spacing", "space-regular").unwrap() * scale;
        let available = tekes_ui_scene::Size::new(100_000., frame.size.height);
        let mut widths = Vec::new();
        for (index, item) in all.iter().enumerate() {
            widths.push(
                control(item, slot(index))?
                    .measure(&mut self.text, available)
                    .size
                    .width,
            );
        }
        // ToolbarContent owns its regular outer pads and its section gap.
        let width = (frame.size.width - regular * 2.).max(0.);
        let more_width = control(&more(&all), slot(all.len()))?
            .measure(&mut self.text, available)
            .size
            .width;
        let budget = width;
        let chosen = fit(
            &widths,
            all.iter().position(|item| item.selected),
            budget,
            more_width,
            tight,
        );
        let mut visible: Vec<_> = chosen.iter().map(|&index| all[index].clone()).collect();
        let hidden: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(index, _)| !chosen.contains(index))
            .map(|(_, item)| item.clone())
            .collect();
        if !hidden.is_empty() {
            visible.push(more(&hidden));
        }
        let structural = self.presentation_tabs.is_none()
            || self
                .presentation_switcher
                .items
                .iter()
                .map(|item| &item.key)
                .ne(visible.iter().map(|item| &item.key));
        if self.presentation_tabs.is_none() {
            let mut host = ToolbarHost::new(ComponentId(999_998), OWNER, whole)
                .map_err(|e| anyhow::anyhow!("switcher toolbar: {e:?}"))?;
            host.set_regions(
                OWNER,
                whole,
                vec![ToolbarRegion::new(ComponentId(OWNER), frame)],
            )
            .map_err(|e| anyhow::anyhow!("switcher region: {e:?}"))?;
            self.presentation_tabs = Some(host);
        }
        let host = self.presentation_tabs.as_mut().unwrap();
        if structural {
            let mut children = Vec::new();
            let mut bindings = Vec::new();
            for (index, item) in visible.iter().enumerate() {
                let id = slot(index);
                children.push(control(item, id)?);
                bindings.push(ToolbarCommandBinding {
                    target: id,
                    action: None,
                    command: 0,
                    source: item
                        .tab
                        .clone()
                        .map_or(SwitcherAction::Menu, SwitcherAction::Select),
                });
            }
            host.register(ToolbarRegistration {
                region: ComponentId(OWNER),
                order: 0,
                group: Content::new(GROUP, {
                    let mut group = ToolbarGroup::new(ToolbarPlacement::Navigation, children);
                    group.set_radio(true);
                    group.set_selected_id(visible.iter().position(|item| item.selected).map(slot));
                    group
                }),
                bindings,
            })
            .map_err(|e| anyhow::anyhow!("switcher group: {e:?}"))?;
        } else {
            for (index, item) in visible.iter().enumerate() {
                if self.presentation_switcher.items[index] == *item {
                    continue;
                }
                if item.tab.is_some() {
                    host.update_component::<Button>(GROUP, slot(index), |button| {
                        if let Some(image) =
                            tekes_ui_controls::composition::Component::children_mut(button)
                                .and_then(|children| children.first_mut())
                                .and_then(|content| content.component_mut::<IconContent>())
                        {
                            image
                                .set_glyph(&item.icon)
                                .expect("Container icons validated before mount");
                        }
                    })
                    .map_err(|e| anyhow::anyhow!("switcher icon: {e:?}"))?;
                } else {
                    let mut result = Ok(());
                    host.update_component::<MenuContent>(GROUP, slot(index), |menu| {
                        if let Some(group) =
                            tekes_ui_controls::composition::Component::children_mut(menu)
                                .and_then(|children| children.first_mut())
                                .and_then(|content| content.component_mut::<ToolbarGroup>())
                            && let Some(image) = group
                                .children
                                .first_mut()
                                .and_then(|content| content.component_mut::<IconContent>())
                        {
                            image
                                .set_glyph(&item.icon)
                                .expect("Container icons validated before mount");
                        }
                        if result.is_ok() {
                            result = menu.set_entries(item.entries.clone());
                        }
                    })
                    .map_err(|e| anyhow::anyhow!("switcher menu update: {e:?}"))?;
                    result.map_err(|e| anyhow::anyhow!("switcher entries: {e:?}"))?;
                }
            }
        }
        if !structural
            && self
                .presentation_switcher
                .items
                .iter()
                .map(|item| item.selected)
                .ne(visible.iter().map(|item| item.selected))
        {
            host.update_component::<ToolbarGroup>(GROUP, GROUP, |group| {
                group.set_selected_id(visible.iter().position(|item| item.selected).map(slot));
            })
            .map_err(|e| anyhow::anyhow!("switcher radio selection: {e:?}"))?;
        }
        self.presentation_switcher = SwitcherState { items: visible };

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
                .list_height(180.)
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
    fn t52_container_icon_fallback_and_declared_icon_follow_identity_through_exchange() {
        let local = |id| {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
                .unwrap()
                .title(id)
        };
        let members: Vec<Container> = vec![
            local("focus").switcher_icon("tekes-note").into(),
            local("other").into(),
        ];
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(
                PresentationOptions::new("focus").others(OthersOptions::new(vec!["other".into()])),
            ),
        );
        host.mount_columns();
        let before: Vec<_> = host
            .panels
            .iter()
            .map(|panel| (panel.module_id.clone(), panel.instance))
            .collect();
        let items = host.switcher_items();
        assert_eq!(
            items[0].icon, "tekes-file-generic",
            "undeclared container uses existing ordinary document fallback"
        );
        host.presentation
            .as_mut()
            .unwrap()
            .promote("other")
            .unwrap();
        let items = host.switcher_items();
        assert_eq!(
            items[0].icon, "tekes-note",
            "former Focus carries its own declaration into Others"
        );
        assert_eq!(
            host.panels
                .iter()
                .map(|panel| (panel.module_id.clone(), panel.instance))
                .collect::<Vec<_>>(),
            before
        );
    }
    #[test]
    fn first_family_click_paints_then_releases_selects_once_and_cancel_keeps_content() {
        let mut host = fixture();
        let original: Vec<_> = host.panels.iter().map(|panel| panel.instance).collect();
        paint(&mut host, 600.);
        let before = host.presentation.as_ref().unwrap().selected_tab().cloned();
        click_with_paint(&mut host, "Documents", 600.);
        assert!(
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .content_host()
                .input_overlay_owner()
                .is_some()
        );
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab().cloned(),
            before,
            "opening is not selection"
        );
        click_with_paint(&mut host, "second", 600.);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("second".into()))
        );
        assert!(
            host.presentation_tabs
                .as_mut()
                .unwrap()
                .drain_commands()
                .is_empty(),
            "production consumed the one menu command"
        );
        click_with_paint(&mut host, "Documents", 600.);
        let parts = host.presentation_tabs.as_ref().unwrap().semantics();
        assert!(
            parts
                .iter()
                .flat_map(|node| &node.parts)
                .any(|(_, part)| part.semantics.name == "second" && part.semantics.selected)
        );
        host.presentation_tab_input(&ContentEvent::Key {
            key: ControlKey::Escape,
            down: true,
        })
        .unwrap();
        paint(&mut host, 600.);
        assert!(
            host.presentation_tabs
                .as_ref()
                .unwrap()
                .content_host()
                .input_overlay_owner()
                .is_none()
        );
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("second".into()))
        );
        click_with_paint(&mut host, "inspector", 600.);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("inspector".into()))
        );
        assert_eq!(
            host.panels
                .iter()
                .map(|panel| panel.instance)
                .collect::<Vec<_>>(),
            original
        );
    }
    #[test]
    fn declared_family_survives_one_member_and_exchange_preserves_instance_identity() {
        let mut host = fixture();
        let original: Vec<_> = host.panels.iter().map(|panel| panel.instance).collect();
        host.presentation.as_mut().unwrap().close("first").unwrap();
        paint(&mut host, 600.);
        let family = host
            .presentation_switcher
            .items
            .iter()
            .find(|item| item.title == "Documents")
            .unwrap();
        assert!(family.tab.is_none());
        assert_eq!(family.entries.len(), 1);
        let singleton = target(&host, "inspector").0;
        host.presentation_tabs
            .as_mut()
            .unwrap()
            .accessibility_action(singleton, AccessibilityAction::Press);
        host.apply_tab_selection().unwrap();
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("inspector".into()))
        );
        host.presentation
            .as_mut()
            .unwrap()
            .promote("second")
            .unwrap();
        paint(&mut host, 600.);
        let family = host
            .presentation_switcher
            .items
            .iter()
            .find(|item| item.title == "Documents")
            .unwrap();
        assert!(
            family
                .entries
                .iter()
                .any(|entry| matches!(entry,MenuEntry::Item {value,..} if value=="focus"))
        );
        assert!(
            !family
                .entries
                .iter()
                .any(|entry| matches!(entry,MenuEntry::Item {value,..} if value=="second"))
        );
        host.presentation.as_mut().unwrap().open("first").unwrap();
        assert_eq!(
            host.panels
                .iter()
                .map(|panel| panel.instance)
                .collect::<Vec<_>>(),
            original
        );
    }
    #[test]
    fn narrow_slot_prefers_current_family_and_overflow_keeps_every_hidden_destination() {
        let mut host = fixture();
        host.presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("second".into()))
            .unwrap();
        paint(&mut host, 120.);
        assert!(
            host.presentation_switcher
                .items
                .iter()
                .any(|item| item.key == MORE)
        );
        assert!(
            host.presentation_switcher
                .items
                .iter()
                .any(|item| item.title == "Documents"),
            "current family gets first claim on the measured slot"
        );
        click_with_paint(&mut host, "More containers", 120.);
        let hidden = host
            .presentation_switcher
            .items
            .iter()
            .find(|item| item.key == MORE)
            .unwrap();
        assert!(!hidden.entries.is_empty());
        let before = host.presentation.as_ref().unwrap().selected_tab().cloned();
        host.presentation_tab_input(&ContentEvent::Cancel).unwrap();
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab().cloned(),
            before
        );
        paint(&mut host, 120.);
        click_with_paint(&mut host, "More containers", 120.);
        click_with_paint(&mut host, "inspector", 120.);
        assert_eq!(
            host.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("inspector".into()))
        );
        // With only More fitting, a declared family is reachable as a submenu.
        paint(&mut host, 50.);
        assert!(host.presentation_switcher.items.iter().find(|item|item.key==MORE).unwrap().entries.iter().any(|entry|matches!(entry,MenuEntry::Submenu { label,entries,.. } if label=="Documents" && entries.len()==2)));
    }
    #[test]
    fn one_declared_family_keeps_radio_selection_with_one_remaining_instance() {
        let members = ["focus", "first", "second"]
            .into_iter()
            .map(|id| {
                LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
                    .unwrap()
                    .into()
            })
            .collect();
        let mut host = Host::from_containers(
            members,
            false,
            None,
            Some(PresentationOptions::new("focus").others(
                OthersOptions::new(vec!["first".into(), "second".into()]).groups(vec![
                    OthersGroup::new(
                        "documents",
                        "Documents",
                        "tekes-files",
                        vec!["first".into(), "second".into()],
                    ),
                ]),
            )),
        );
        host.mount_columns();
        host.presentation.as_mut().unwrap().close("second").unwrap();
        paint(&mut host, 600.);
        let menu = host
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(slot(0))
            .unwrap()
            .component::<MenuContent>()
            .unwrap();
        assert!(
            menu.trigger.button.state.selected,
            "the current destination remains selected in its radio group"
        );
        click_with_paint(&mut host, "Documents", 600.);
        assert!(target(&host, "first").1.size.width > 0.);
    }
    #[test]
    fn aggregate_list_has_no_close_all_or_internal_close_and_focus_is_not_closable() {
        let mut host = fixture();
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
            !host.panels[0]
                .mounted_toolbar
                .sources()
                .any(|source| matches!(source, crate::window_toolbar::Source::Close(_)))
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
