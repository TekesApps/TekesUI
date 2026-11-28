//! Window presentation routes stable container identities. Each Panel retains
//! its original ContentHost/process and its independently owned Toolbar.
use super::*;
use crate::{
    OthersTab, PresentationState,
    window_toolbar::{
        ButtonDeclaration, CapsuleGroupDeclaration, CapsuleGroupMember, Contents, Declaration,
        MenuDeclaration, Source, build_capsule_group_content, build_menu_content,
    },
};
use std::collections::BTreeMap;
use tekes_ui_controls::composition::{
    ButtonImagePosition, Component, ContentActionPayload, MenuEntry, TextContent,
    ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement, ToolbarRegion,
    ToolbarWindowEdge,
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
use tekes_ui_controls::composition::{Content, ToolbarHost};
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
    render_test_tabs_with_extensions(host, scene, text, palette, &[])
}
#[cfg(test)]
pub(super) fn render_test_tabs_with_extensions<B: Clone + std::fmt::Debug>(
    host: &mut ToolbarHost<B>,
    scene: &mut Scene,
    text: &mut Text,
    palette: Palette,
    extensions: &[tekes_ui_icons::Glyph],
) -> Result<()> {
    with_test_icons_with_extensions(text, extensions, |text| host.render(scene, text, palette))
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
    with_test_icons_with_extensions(text, &[], render)
}
#[cfg(test)]
fn with_test_icons_with_extensions(
    text: &mut Text,
    extensions: &[tekes_ui_icons::Glyph],
    render: impl FnOnce(
        &mut dyn tekes_ui_controls::ButtonTextRenderer,
    )
        -> std::result::Result<(), tekes_ui_controls::composition::CompositionError>,
) -> Result<()> {
    struct Icons<'a> {
        extensions: &'a [tekes_ui_icons::Glyph],
    }
    impl tekes_ui_controls::layout_view::IconRenderer for Icons<'_> {
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
            assert!(
                tekes_ui_icons::Glyph::resolve_with_extensions(name, self.extensions).is_some()
            );
            Ok(tekes_ui_controls::composition::IconImage::new(
                ImageId(99),
                std::rc::Rc::new(()),
            ))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("Tabs must retain a prepared icon lease");
        }
    }
    let mut icons = Icons { extensions };
    render(&mut tekes_ui_controls::composition::WithIconRenderer::new(
        text, &mut icons,
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
fn toolbar_spacing(text: &mut Text, token: &str) -> f32 {
    use tekes_ui_controls::ButtonTextRenderer;
    use tekes_ui_tokens::proposal;
    let scale = ButtonTextRenderer::resolved_font_size(text, proposal::TextRole::UiBody)
        / proposal::TOOLBAR_REFERENCE_TITLE_SIZE;
    let scale = if scale.is_finite() && scale > 0. {
        scale
    } else {
        1.
    };
    proposal::metric("spacing", token).unwrap_or(tekes_ui_tokens::Spacing::Regular.value()) * scale
}
fn non_empty_menu_label(label: &str, fallback: &str) -> String {
    if label.trim().is_empty() {
        fallback.to_owned()
    } else {
        label.to_owned()
    }
}
fn local_menu_declaration(
    menu: &crate::LocalToolbarMenu,
    enabled: bool,
    include_icons: bool,
) -> MenuDeclaration {
    let fallback = menu.tooltip.as_deref().unwrap_or("Menu");
    let (title, accessible_name, icon, icon_position, icon_slot) = if include_icons {
        if let Some(icon) = &menu.icon {
            let icon_slot = menu.title.trim().is_empty();
            let accessible_name =
                icon_slot.then(|| menu.tooltip.clone().unwrap_or_else(|| "Menu".to_owned()));
            (
                menu.title.clone(),
                accessible_name,
                Some(icon.clone()),
                ButtonImagePosition::Leading,
                icon_slot,
            )
        } else if menu.dropdown_indicator {
            (
                non_empty_menu_label(&menu.title, fallback),
                None,
                Some("tekes-expand".to_owned()),
                ButtonImagePosition::Trailing,
                false,
            )
        } else {
            (
                non_empty_menu_label(&menu.title, fallback),
                None,
                None,
                ButtonImagePosition::Leading,
                false,
            )
        }
    } else {
        (
            non_empty_menu_label(&menu.title, fallback),
            None,
            None,
            ButtonImagePosition::Leading,
            false,
        )
    };
    MenuDeclaration {
        id: menu.id,
        title,
        accessible_name,
        icon,
        icon_position,
        icon_slot,
        tooltip: menu.tooltip.clone(),
        enabled,
        selected: menu.selected,
        tone: menu.tone.into(),
        entries: menu.entries.clone(),
    }
}
fn local_button_declaration(
    button: &crate::LocalToolbarButton,
    enabled: bool,
    title: &str,
    hint: &str,
    icon: Option<&str>,
    selected: bool,
) -> ButtonDeclaration {
    let icon_slot = icon.is_some() && title.trim().is_empty();
    let accessible_name = if icon_slot {
        button
            .tooltip
            .clone()
            .or_else(|| (!hint.trim().is_empty()).then(|| hint.to_owned()))
            .or_else(|| Some("Action".to_owned()))
    } else {
        None
    };
    let resolved_title = if icon.is_some() {
        title.to_owned()
    } else {
        non_empty_menu_label(title, button.tooltip.as_deref().unwrap_or("Action"))
    };
    ButtonDeclaration {
        id: button.id,
        title: resolved_title,
        accessible_name,
        icon: icon.map(str::to_owned),
        tooltip: (!hint.trim().is_empty()).then(|| hint.to_owned()),
        enabled,
        selected,
        tone: button.tone.into(),
    }
}
fn overflow_leaf_entry(item: &crate::LocalToolbarItem, enabled: bool, title: &str) -> MenuEntry {
    match item {
        crate::LocalToolbarItem::Button(button) => MenuEntry::Item {
            value: format!("cmd:{}", button.id),
            label: non_empty_menu_label(title, button.tooltip.as_deref().unwrap_or("Action")),
            checked: button.selected.then_some(true),
            disabled: !enabled,
        },
        crate::LocalToolbarItem::Menu(menu) if menu.entries.is_empty() => MenuEntry::Item {
            value: format!("noop:{}", menu.id),
            label: non_empty_menu_label(&menu.title, menu.tooltip.as_deref().unwrap_or("Menu")),
            checked: menu.selected.then_some(true),
            disabled: true,
        },
        crate::LocalToolbarItem::Menu(menu) => MenuEntry::Submenu {
            value: format!("menu:{}", menu.id),
            label: non_empty_menu_label(&menu.title, menu.tooltip.as_deref().unwrap_or("Menu")),
            disabled: !enabled,
            entries: menu.entries.clone(),
        },
        crate::LocalToolbarItem::Group(_) => unreachable!("nested group rejected by validation"),
    }
}
fn overflow_menu_declaration(
    id: u64,
    items: &[LocalOverflowEntry],
    include_icons: bool,
) -> MenuDeclaration {
    let mut entries: Vec<MenuEntry> = Vec::new();
    for entry in items {
        match &entry.item {
            crate::LocalToolbarItem::Button(_) | crate::LocalToolbarItem::Menu(_) => {
                entries.push(overflow_leaf_entry(
                    &entry.item,
                    entry.enabled,
                    &entry.title,
                ));
            }
            crate::LocalToolbarItem::Group(group) => {
                if entry.sub_entries.is_empty() {
                    continue;
                }
                let member_entries: Vec<MenuEntry> = entry
                    .sub_entries
                    .iter()
                    .map(|member| overflow_leaf_entry(&member.item, member.enabled, &member.title))
                    .collect();
                let group_label = group
                    .label
                    .as_deref()
                    .or(group.tooltip.as_deref())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                if let Some(label) = group_label {
                    let any_enabled = entry.sub_entries.iter().any(|m| m.enabled);
                    entries.push(MenuEntry::Submenu {
                        value: format!("group:{}", group.id),
                        label: label.to_owned(),
                        disabled: !any_enabled,
                        entries: member_entries,
                    });
                } else {
                    if !entries.is_empty() && !matches!(entries.last(), Some(MenuEntry::Separator))
                    {
                        entries.push(MenuEntry::Separator);
                    }
                    entries.extend(member_entries);
                    entries.push(MenuEntry::Separator);
                }
            }
        }
    }
    if matches!(entries.last(), Some(MenuEntry::Separator)) {
        entries.pop();
    }
    MenuDeclaration {
        id,
        title: if include_icons {
            String::new()
        } else {
            "More".to_owned()
        },
        accessible_name: Some("More actions".to_owned()),
        icon: include_icons.then(|| "tekes-more".to_owned()),
        icon_position: ButtonImagePosition::Leading,
        icon_slot: include_icons,
        tooltip: Some("More actions".to_owned()),
        enabled: true,
        selected: false,
        tone: tekes_ui_controls::composition::ButtonTone::Default,
        entries,
    }
}
fn capsule_member_component_id(base: u64, slot: u64, is_menu: bool) -> ComponentId {
    ComponentId(base + 500_000 + slot * 4 + u64::from(is_menu))
}
fn build_capsule_members(
    panel: &window_panels::Panel,
    base: u64,
    sub_entries: &[LocalBucketEntry],
    include_icons: bool,
) -> Vec<CapsuleGroupMember> {
    sub_entries
        .iter()
        .enumerate()
        .filter_map(|(idx, member)| {
            let slot = panel
                .slot_for_local_item(member.item.id())
                .unwrap_or(idx as u64);
            match &member.item {
                crate::LocalToolbarItem::Button(button) => Some(CapsuleGroupMember::Button {
                    component_id: capsule_member_component_id(base, slot, false),
                    decl: local_button_declaration(
                        button,
                        member.enabled,
                        &member.title,
                        &member.hint,
                        member.icon.as_deref(),
                        member.selected,
                    ),
                }),
                crate::LocalToolbarItem::Menu(menu) => Some(CapsuleGroupMember::Menu {
                    component_id: capsule_member_component_id(base, slot, true),
                    decl: local_menu_declaration(menu, member.enabled, include_icons),
                }),
                crate::LocalToolbarItem::Group(_) => None,
            }
        })
        .collect()
}
type LocalButtonPart = (ToolbarButtonDeclaration, Option<(u64, String)>, Option<u64>);
fn local_button_contents(
    buttons: Vec<LocalButtonPart>,
    overflow: ToolbarGroupOverflow,
) -> Contents {
    let mut decls = Vec::with_capacity(buttons.len());
    let mut templates = Vec::new();
    let mut selected = Vec::new();
    for (decl, icon, selected_id) in buttons {
        decls.push(decl);
        if let Some(template) = icon {
            templates.push(template);
        }
        if let Some(id) = selected_id {
            selected.push(id);
        }
    }
    if selected.is_empty() {
        Contents::TemplateCommands(decls, overflow, templates)
    } else {
        Contents::SelectedTemplateCommands {
            buttons: decls,
            overflow,
            templates,
            selected,
        }
    }
}
fn measure_menu_width(
    text: &mut Text,
    decl: &MenuDeclaration,
    extensions: &[tekes_ui_icons::Glyph],
) -> f32 {
    let mut menu = build_menu_content(decl, Rect::default(), extensions);
    Component::measure(&mut menu, text, tekes_ui_scene::Size::new(100_000., 64.))
        .size
        .width
}
fn measure_contents_width(
    text: &mut Text,
    contents: &Contents,
    extensions: &[tekes_ui_icons::Glyph],
) -> f32 {
    match contents {
        Contents::Menu(decl) => measure_menu_width(text, decl, extensions),
        Contents::CapsuleGroup(decl) => {
            let mut group = build_capsule_group_content(
                ToolbarPlacement::PrimaryAction,
                decl,
                Rect::default(),
                extensions,
            );
            Component::measure(&mut group, text, tekes_ui_scene::Size::new(100_000., 64.))
                .size
                .width
        }
        Contents::Title(title) => {
            let mut label = TextContent::label(title.clone());
            Component::measure(&mut label, text, tekes_ui_scene::Size::new(100_000., 64.))
                .size
                .width
        }
        Contents::Commands(buttons, overflow)
        | Contents::TemplateCommands(buttons, overflow, _)
        | Contents::SelectedTemplateCommands {
            buttons, overflow, ..
        } => {
            let mut content = tekes_ui_controls::composition::toolbar_commands(
                ComponentId(1),
                buttons.clone(),
                *overflow,
            );
            if let Some(control) = content.component_mut::<ToolbarControl>() {
                for (id, name) in contents.templates() {
                    let _ = control.set_action_template_icon_with_extensions(
                        *id,
                        Some(name),
                        extensions,
                    );
                }
                if let Contents::SelectedTemplateCommands { selected, .. } = contents {
                    control.set_selected_ids(selected);
                }
            }
            content
                .measure(text, tekes_ui_scene::Size::new(100_000., 64.))
                .size
                .width
        }
    }
}
fn occupied_region_width(
    text: &mut Text,
    existing: &[&Declaration],
    new_placement: ToolbarPlacement,
    extensions: &[tekes_ui_icons::Glyph],
) -> f32 {
    let gap = toolbar_spacing(text, "space-regular");
    let tight = toolbar_spacing(text, "space-tight");
    let mut sec0 = Vec::new();
    let mut sec1 = Vec::new();
    let mut sec2 = Vec::new();
    for decl in existing {
        let width = measure_contents_width(text, &decl.contents, extensions);
        match decl.placement {
            ToolbarPlacement::Navigation => sec0.push(width),
            ToolbarPlacement::PrimaryAction | ToolbarPlacement::SecondaryAction => sec2.push(width),
            _ => sec1.push(width),
        }
    }
    let w0: f32 = sec0.iter().sum::<f32>() + tight * sec0.len().saturating_sub(1) as f32;
    let w1: f32 = sec1.iter().sum::<f32>() + gap * sec1.len().saturating_sub(1) as f32;
    let w2: f32 = sec2.iter().sum::<f32>() + gap * sec2.len().saturating_sub(1) as f32;
    let has_sec0 = !sec0.is_empty() || matches!(new_placement, ToolbarPlacement::Navigation);
    let has_sec2 = !sec2.is_empty()
        || matches!(
            new_placement,
            ToolbarPlacement::PrimaryAction | ToolbarPlacement::SecondaryAction
        );
    let has_sec1 = !sec1.is_empty() || (!has_sec0 && !has_sec2);
    let sections = usize::from(has_sec0) + usize::from(has_sec1) + usize::from(has_sec2);
    let inter_sections = gap * sections.saturating_sub(1) as f32;
    let existing_intra = match new_placement {
        ToolbarPlacement::Navigation if !sec0.is_empty() => tight,
        ToolbarPlacement::PrimaryAction | ToolbarPlacement::SecondaryAction if !sec2.is_empty() => {
            gap
        }
        _ if !sec1.is_empty()
            && !matches!(
                new_placement,
                ToolbarPlacement::Navigation
                    | ToolbarPlacement::PrimaryAction
                    | ToolbarPlacement::SecondaryAction
            ) =>
        {
            gap
        }
        _ => 0.,
    };
    2. * gap + w0 + w1 + w2 + inter_sections + existing_intra
}
fn fitting_prefix_count(item_widths: &[f32], overflow_width: f32, budget: f32, gap: f32) -> usize {
    let count = item_widths.len();
    let total: f32 = item_widths.iter().sum::<f32>() + gap * count.saturating_sub(1) as f32;
    if total <= budget {
        return count;
    }
    for k in (0..count).rev() {
        let prefix: f32 =
            item_widths[..k].iter().sum::<f32>() + if k > 0 { gap * k as f32 } else { 0. };
        if prefix + overflow_width <= budget {
            return k;
        }
    }
    0
}
fn horizontal_region_budgets(regions: &[Vec<ToolbarRegion>]) -> BTreeMap<ComponentId, f32> {
    let mut budgets = BTreeMap::new();
    for region in regions.iter().flatten() {
        if region.axis == Axis::Horizontal && region.available.size.width > 0. {
            let usable = (region.available.size.width
                - region.native_safe_area.left
                - region.native_safe_area.right)
                .max(0.);
            budgets.insert(region.id, usable);
        }
    }
    budgets
}
fn presentation_window_business_region(
    state: &PresentationState,
    panel: &window_panels::Panel,
) -> ComponentId {
    let id = panel.module_id.as_str();
    if state.nav_bar_id() == Some(id) {
        WINDOW_NAV_BUSINESS
    } else if state.focus_id() == id && !state.others_expanded() {
        WINDOW_BUSINESS
    } else {
        WINDOW_OTHERS_BUSINESS
    }
}
fn presentation_business_destination(
    state: &PresentationState,
    panels: &[window_panels::Panel],
    owner: usize,
    retained: Option<usize>,
    focus: usize,
) -> (usize, ComponentId) {
    let target = retained.unwrap_or(owner);
    match panels[target].options.toolbar_host {
        crate::ContainerToolbarHost::Window => (
            focus,
            presentation_window_business_region(state, &panels[target]),
        ),
        crate::ContainerToolbarHost::Container { .. } => {
            (target, ComponentId(91000 + target as u64))
        }
    }
}
fn build_navigation_toolbar_declaration(
    state: &PresentationState,
    panels: &[window_panels::Panel],
    index: usize,
    focus: usize,
    source: crate::toolbar_source::ComponentSource,
    buttons: Vec<ToolbarButtonDeclaration>,
    templates: Vec<(u64, String)>,
) -> (usize, Declaration) {
    let (presenter, destination) =
        presentation_business_destination(state, panels, index, None, focus);
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
#[derive(Clone)]
struct LocalOverflowEntry {
    item: crate::LocalToolbarItem,
    enabled: bool,
    title: String,
    sub_entries: Vec<LocalBucketEntry>,
}
#[derive(Clone)]
struct LocalBucketEntry {
    item: crate::LocalToolbarItem,
    enabled: bool,
    title: String,
    hint: String,
    icon: Option<String>,
    selected: bool,
    sub_entries: Vec<LocalBucketEntry>,
}
fn is_rich_local_item(item: &crate::LocalToolbarItem) -> bool {
    matches!(
        item,
        crate::LocalToolbarItem::Menu(_) | crate::LocalToolbarItem::Group(_)
    )
}
impl LocalBucketEntry {
    fn from_panel_item(
        panel: &window_panels::Panel,
        item: &crate::LocalToolbarItem,
        state: Option<&PresentationState>,
        visible: bool,
        open: bool,
        include_icons: bool,
    ) -> Self {
        let is_switch = panel.options.focus_switch_command == Some(item.id());
        let switch_target = is_switch
            .then(|| state.and_then(|s| s.focus_switch_target(&panel.module_id)))
            .flatten();
        let sub_entries = match item {
            crate::LocalToolbarItem::Group(group) => group
                .items
                .iter()
                .map(|member| {
                    Self::from_panel_item(panel, member, state, visible, open, include_icons)
                })
                .collect(),
            _ => Vec::new(),
        };
        let enabled = open
            && (!is_switch || switch_target.is_some())
            && panel.failed.is_none()
            && item.enabled()
            && match item {
                crate::LocalToolbarItem::Button(button) => {
                    !button.geometry_dependent
                        || (visible && panel.frame_viewport == panel.viewport)
                }
                crate::LocalToolbarItem::Menu(_) => true,
                crate::LocalToolbarItem::Group(_) => {
                    sub_entries.iter().any(|member| member.enabled)
                }
            };
        let (title, hint, icon, selected) = match item {
            crate::LocalToolbarItem::Button(button) => {
                let title = if is_switch {
                    if state.is_some_and(|s| panel.module_id == s.focus_id()) {
                        "Switch to Others".to_owned()
                    } else {
                        "Switch to Focus".to_owned()
                    }
                } else {
                    button.title.clone()
                };
                let hint = if is_switch {
                    switch_target.map_or_else(
                        || "Select an ordinary container Tab to exchange".to_owned(),
                        |id| format!("Exchange with {id}"),
                    )
                } else {
                    button.tooltip.clone().unwrap_or_else(|| title.clone())
                };
                let icon = if !include_icons {
                    None
                } else if button.icon.is_some() {
                    button.icon.clone()
                } else if is_switch {
                    Some("tekes-swap-horizontal".to_owned())
                } else {
                    None
                };
                (title, hint, icon, button.selected)
            }
            crate::LocalToolbarItem::Menu(menu) => (
                menu.title.clone(),
                menu.tooltip.clone().unwrap_or_else(|| menu.title.clone()),
                menu.icon.clone().filter(|_| include_icons),
                menu.selected,
            ),
            crate::LocalToolbarItem::Group(group) => (
                group
                    .label
                    .clone()
                    .or_else(|| group.tooltip.clone())
                    .unwrap_or_default(),
                group
                    .tooltip
                    .clone()
                    .or_else(|| group.label.clone())
                    .unwrap_or_default(),
                None,
                false,
            ),
        };
        Self {
            item: item.clone(),
            enabled,
            title,
            hint,
            icon,
            selected,
            sub_entries,
        }
    }
    fn to_single_contents(
        &self,
        panel: &window_panels::Panel,
        base: u64,
        include_icons: bool,
    ) -> Contents {
        match &self.item {
            crate::LocalToolbarItem::Button(button) => local_button_contents(
                vec![(
                    ToolbarButtonDeclaration {
                        id: button.id,
                        icon: None,
                        text: self.title.clone(),
                        hint: Some(self.hint.clone()),
                        enabled: self.enabled,
                    },
                    self.icon.clone().map(|name| (button.id, name)),
                    self.selected.then_some(button.id),
                )],
                ToolbarGroupOverflow::Together,
            ),
            crate::LocalToolbarItem::Menu(menu) => {
                Contents::Menu(local_menu_declaration(menu, self.enabled, include_icons))
            }
            crate::LocalToolbarItem::Group(group) => {
                Contents::CapsuleGroup(CapsuleGroupDeclaration {
                    divided: group.divided,
                    label: group.label.clone(),
                    tooltip: group.tooltip.clone(),
                    members: build_capsule_members(panel, base, &self.sub_entries, include_icons),
                })
            }
        }
    }
    fn to_overflow_entry(&self) -> LocalOverflowEntry {
        LocalOverflowEntry {
            item: self.item.clone(),
            enabled: self.enabled,
            title: self.title.clone(),
            sub_entries: self.sub_entries.clone(),
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn emit_local_bucket_declarations(
    panel: &window_panels::Panel,
    index: usize,
    entries: &[LocalBucketEntry],
    region: ComponentId,
    primary_placement: ToolbarPlacement,
    secondary: bool,
    button_only_id: ComponentId,
    button_only_order: i32,
    overflow_role: u64,
    include_icons: bool,
    extensions: &[tekes_ui_icons::Glyph],
    budget: Option<(&mut Text, f32)>,
    out: &mut Vec<Declaration>,
) {
    if entries.is_empty() {
        return;
    }
    let source_panel = || Source::Panel {
        instance: panel.instance,
        index,
        module: panel.module_id.clone(),
        pid: panel.pid,
        generation: panel.toolbar_generation,
    };
    let has_rich = entries.iter().any(|entry| is_rich_local_item(&entry.item));
    if !has_rich {
        let parts = entries
            .iter()
            .filter_map(|entry| match &entry.item {
                crate::LocalToolbarItem::Button(button) => Some((
                    ToolbarButtonDeclaration {
                        id: button.id,
                        icon: None,
                        text: entry.title.clone(),
                        hint: Some(entry.hint.clone()),
                        enabled: entry.enabled,
                    },
                    entry.icon.clone().map(|name| (button.id, name)),
                    entry.selected.then_some(button.id),
                )),
                _ => None,
            })
            .collect();
        out.push(Declaration {
            id: button_only_id,
            region,
            placement: if secondary {
                ToolbarPlacement::SecondaryAction
            } else {
                primary_placement
            },
            order: button_only_order,
            contents: local_button_contents(
                parts,
                if secondary {
                    ToolbarGroupOverflow::Menu
                } else {
                    ToolbarGroupOverflow::Split
                },
            ),
            source: source_panel(),
        });
        return;
    }
    let base = 100_000_000u64 + (panel.instance & 0xFFFF_FFFF) * 1_000_000;
    let visible_entries: Vec<&LocalBucketEntry> = entries
        .iter()
        .filter(|entry| {
            !matches!(entry.item, crate::LocalToolbarItem::Group(_))
                || !entry.sub_entries.is_empty()
        })
        .collect();
    if visible_entries.is_empty() {
        return;
    }
    let overflow_entries: Vec<_> = visible_entries
        .iter()
        .map(|entry| entry.to_overflow_entry())
        .collect();
    if secondary {
        out.push(Declaration {
            id: ComponentId(base + overflow_role * 10 + 8_000_000),
            region,
            placement: ToolbarPlacement::SecondaryAction,
            order: 50 + index as i32,
            contents: Contents::Menu(overflow_menu_declaration(
                0,
                &overflow_entries,
                include_icons,
            )),
            source: Source::PanelOverflow {
                instance: panel.instance,
                index,
                module: panel.module_id.clone(),
                pid: panel.pid,
                generation: panel.toolbar_generation,
            },
        });
        return;
    }
    let item_contents: Vec<_> = visible_entries
        .iter()
        .map(|entry| entry.to_single_contents(panel, base, include_icons))
        .collect();
    let (k, split_capsule) = if let Some((text, available)) = budget {
        let gap = toolbar_spacing(
            text,
            if matches!(primary_placement, ToolbarPlacement::Navigation) {
                "space-tight"
            } else {
                "space-regular"
            },
        );
        let item_widths: Vec<_> = item_contents
            .iter()
            .map(|contents| measure_contents_width(text, contents, extensions))
            .collect();
        let overflow_decl = overflow_menu_declaration(0, &overflow_entries, include_icons);
        let overflow_width = measure_menu_width(text, &overflow_decl, extensions);
        let k = fitting_prefix_count(&item_widths, overflow_width, available, gap);
        let mut split_choice: Option<(Contents, LocalOverflowEntry)> = None;
        if let Some(boundary) = visible_entries.get(k)
            && let crate::LocalToolbarItem::Group(group) = &boundary.item
            && group.overflow == crate::LocalToolbarGroupOverflow::Split
            && boundary.sub_entries.len() > 1
        {
            let prefix_used: f32 =
                item_widths[..k].iter().sum::<f32>() + if k > 0 { gap * k as f32 } else { 0. };
            for keep in (1..boundary.sub_entries.len()).rev() {
                let prefix_contents = Contents::CapsuleGroup(CapsuleGroupDeclaration {
                    divided: group.divided,
                    label: group.label.clone(),
                    tooltip: group.tooltip.clone(),
                    members: build_capsule_members(
                        panel,
                        base,
                        &boundary.sub_entries[..keep],
                        include_icons,
                    ),
                });
                let prefix_w = measure_contents_width(text, &prefix_contents, extensions);
                let tail_subs = boundary.sub_entries[keep..].to_vec();
                let tail_entry = LocalOverflowEntry {
                    item: crate::LocalToolbarItem::Group(crate::LocalToolbarGroup {
                        id: group.id,
                        items: tail_subs.iter().map(|s| s.item.clone()).collect(),
                        divided: group.divided,
                        label: group.label.clone(),
                        tooltip: group.tooltip.clone(),
                        overflow: group.overflow,
                    }),
                    enabled: tail_subs.iter().any(|s| s.enabled),
                    title: boundary.title.clone(),
                    sub_entries: tail_subs,
                };
                let mut split_overflow = vec![tail_entry.clone()];
                split_overflow.extend(
                    visible_entries[k + 1..]
                        .iter()
                        .map(|entry| entry.to_overflow_entry()),
                );
                let split_decl = overflow_menu_declaration(0, &split_overflow, include_icons);
                let split_overflow_w = measure_menu_width(text, &split_decl, extensions);
                if prefix_used + prefix_w + gap + split_overflow_w <= available {
                    split_choice = Some((prefix_contents, tail_entry));
                    break;
                }
            }
        }
        (k, split_choice)
    } else {
        (item_contents.len(), None)
    };
    for (idx, (entry, contents)) in visible_entries
        .iter()
        .zip(item_contents)
        .take(k)
        .enumerate()
    {
        let slot_idx = panel
            .slot_for_local_item(entry.item.id())
            .unwrap_or(idx as u64);
        out.push(Declaration {
            id: ComponentId(base + 100 + slot_idx * 10),
            region,
            placement: primary_placement,
            order: -10_000 + (index as i32) * 256 + (idx as i32),
            contents,
            source: source_panel(),
        });
    }
    let tail_start = if let Some((prefix_contents, _)) = &split_capsule {
        let entry = visible_entries[k];
        let slot_idx = panel
            .slot_for_local_item(entry.item.id())
            .unwrap_or(k as u64);
        out.push(Declaration {
            id: ComponentId(base + 100 + slot_idx * 10),
            region,
            placement: primary_placement,
            order: -10_000 + (index as i32) * 256 + (k as i32),
            contents: prefix_contents.clone(),
            source: source_panel(),
        });
        k + 1
    } else {
        k
    };
    if let Some((_, tail_group)) = split_capsule {
        let mut combined = vec![tail_group];
        combined.extend(overflow_entries.into_iter().skip(tail_start));
        out.push(Declaration {
            id: ComponentId(base + overflow_role * 10),
            region,
            placement: primary_placement,
            order: -10_000 + (index as i32) * 256 + 250,
            contents: Contents::Menu(overflow_menu_declaration(0, &combined, include_icons)),
            source: Source::PanelOverflow {
                instance: panel.instance,
                index,
                module: panel.module_id.clone(),
                pid: panel.pid,
                generation: panel.toolbar_generation,
            },
        });
    } else if k < visible_entries.len() {
        out.push(Declaration {
            id: ComponentId(base + overflow_role * 10),
            region,
            placement: primary_placement,
            order: -10_000 + (index as i32) * 256 + 250,
            contents: Contents::Menu(overflow_menu_declaration(
                0,
                &overflow_entries[k..],
                include_icons,
            )),
            source: Source::PanelOverflow {
                instance: panel.instance,
                index,
                module: panel.module_id.clone(),
                pid: panel.pid,
                generation: panel.toolbar_generation,
            },
        });
    }
}
impl Host {
    pub(super) fn presentation_menu(&self) -> Menu {
        let mut entries = Vec::new();
        if self
            .layout
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
            .layout
            .presentation
            .as_ref()
            .is_some_and(|state| state.others().is_some())
        {
            entries.push(MenuItem::Command(MenuCommand::new(
                "Show / Hide Others",
                CommandId(COMMAND_BASE),
            )));
            for (index, id) in self.panel_mgr.module_ids.iter().enumerate() {
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
        self.presentation_frames_with_safe_area(
            whole,
            toolbar_height,
            band,
            native_height as f32,
            leading as f32,
        )
    }
    pub(super) fn presentation_frames_with_safe_area(
        &mut self,
        whole: Rect,
        toolbar_height: f32,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
        leading: f32,
    ) -> Result<Vec<Rect>> {
        #[cfg(test)]
        {
            self.diagnostics.presentation_layout_calls += 1;
        }
        let mut layout = self
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, toolbar_height);
        layout.tabs = layout.tabs.map(|frame| {
            avoid_native_controls(
                Rect::new(frame.min_x(), band.y, frame.size.width, band.height),
                Rect::new(whole.min_x(), whole.min_y(), leading, native_height),
            )
        });
        self.apply_presentation_layout(layout)
    }
    pub(super) fn apply_presentation_layout(
        &mut self,
        layout: crate::PresentationLayout,
    ) -> Result<Vec<Rect>> {
        let state = self.layout.presentation.as_ref().unwrap();
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
        let mut frames = vec![Rect::default(); self.panel_mgr.panels.len()];
        for (index, panel) in self.panel_mgr.panels.iter_mut().enumerate() {
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
                host_diag!(
                    "CONTAINER_GEOMETRY id={} frame={:?} clip={:?} instance={} pid={:?}",
                    panel.module_id,
                    frame,
                    panel.clip,
                    panel.instance,
                    panel.pid
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
                if self.layout.pressed == Some(index) {
                    self.layout.pressed = None;
                }
            }
            panel.bounds = frame;
            frames[index] = frame;
        }
        if layout.tabs.is_none()
            && let Some(tabs) = &mut self.layout.presentation_tabs
        {
            tabs.blur();
        }
        self.layout.presentation_layout = layout;
        Ok(frames)
    }
    pub(super) fn paint_presentation(
        &mut self,
        scene: &mut Scene,
        whole: Rect,
        palette: Palette,
    ) -> Result<()> {
        let mut renderer = self.renderer.take();
        let result =
            self.paint_presentation_with_renderer(scene, whole, palette, renderer.as_mut());
        self.renderer = renderer;
        result
    }
    pub(super) fn paint_presentation_with_renderer(
        &mut self,
        scene: &mut Scene,
        whole: Rect,
        palette: Palette,
        mut renderer: Option<&mut Renderer>,
    ) -> Result<()> {
        let focus = self
            .layout
            .presentation_layout
            .frames
            .iter()
            .find(|(id, _)| id == self.layout.presentation.as_ref().unwrap().focus_id())
            .map(|(_, frame)| *frame)
            .unwrap();
        let area = Rect::new(
            whole.min_x(),
            self.layout
                .container_window_boundary
                .unwrap_or(focus.min_y()),
            whole.size.width,
            (whole.max_y()
                - self
                    .layout
                    .container_window_boundary
                    .unwrap_or(focus.min_y()))
            .max(0.),
        );
        let mut right = area;
        self.layout.outer.min_pane = MIN_COLUMN;
        self.layout.inner.min_pane = MIN_COLUMN;
        self.layout.outer.divider = 1.;
        self.layout.inner.divider = 1.;
        if let Some(nav) = self
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .visible_nav_bar_id()
            && let Some((_, frame)) = self
                .layout
                .presentation_layout
                .frames
                .iter()
                .find(|(id, _)| id == nav)
        {
            self.layout
                .outer
                .set_ratio(frame.size.width / (area.size.width - 1.).max(1.));
            let layout = self
                .layout
                .outer_view
                .arrange(&mut self.layout.outer, area, None);
            right = layout.trailing.frame();
            self.layout.outer_view.paint_presented_divider(
                scene,
                &self.layout.outer,
                layout.divider.frame(),
                palette,
                self.text.scale,
            );
        } else {
            self.layout
                .outer_view
                .arrange(&mut self.layout.outer, Rect::default(), None);
        }
        if self.layout.presentation.as_ref().unwrap().others_visible()
            && !self.layout.presentation.as_ref().unwrap().others_expanded()
        {
            self.layout
                .inner
                .set_ratio(focus.size.width / (right.size.width - 1.).max(1.));
            let layout = self
                .layout
                .inner_view
                .arrange(&mut self.layout.inner, right, None);
            self.layout.inner_view.paint_presented_divider(
                scene,
                &self.layout.inner,
                layout.divider.frame(),
                palette,
                self.text.scale,
            );
        } else {
            self.layout
                .inner_view
                .arrange(&mut self.layout.inner, Rect::default(), None);
        }
        if (self.layout.drag_split == Some(2) && !self.layout.outer_view.is_dragging())
            || (self.layout.drag_split == Some(3) && !self.layout.inner_view.is_dragging())
        {
            self.layout.drag_split = None;
        }
        self.sync_presentation_switcher(whole)?;
        let switcher_frame = self.presentation_switcher_frame();
        let switcher_regions =
            switcher_frame.map(|frame| self.presentation_switcher_regions(frame, whole));
        if let (Some(host), Some(_frame)) = (&mut self.layout.presentation_tabs, switcher_frame) {
            host.set_regions(TABS_OWNER, whole, switcher_regions.unwrap())
                .map_err(|e| anyhow::anyhow!("container tabs region: {e:?}"))?;
            if let Some(renderer) = renderer.as_deref_mut() {
                with_content_icons(
                    renderer,
                    &mut self.layout.presentation_icons,
                    &mut self.text,
                    |text| {
                        host.render(scene, text, palette)
                            .map_err(|e| anyhow::anyhow!("container tabs paint: {e:?}"))
                    },
                )?;
            } else {
                #[cfg(test)]
                render_test_tabs_with_extensions(
                    host,
                    scene,
                    &mut self.text,
                    palette,
                    &self.extension_icons,
                )?;
                #[cfg(not(test))]
                host.render(scene, &mut self.text, palette)
                    .map_err(|e| anyhow::anyhow!("container tabs paint: {e:?}"))?;
            }
        }
        if let Some(grid) = &mut self.layout.presentation_creation_grid
            && !grid.frame.is_empty()
        {
            let grid_frame = grid.frame;
            if let Some(renderer) = renderer {
                with_content_icons(
                    renderer,
                    &mut self.layout.presentation_icons,
                    &mut self.text,
                    |text| {
                        grid.host
                            .render(scene, text, grid_frame, grid_frame, palette)
                            .map_err(|e| anyhow::anyhow!("creation grid paint: {e:?}"))
                    },
                )?;
            } else {
                #[cfg(test)]
                with_test_icons_with_extensions(&mut self.text, &self.extension_icons, |text| {
                    grid.host
                        .render(scene, text, grid_frame, grid_frame, palette)
                })?;
                #[cfg(not(test))]
                grid.host
                    .render(scene, &mut self.text, grid_frame, grid_frame, palette)
                    .map_err(|e| anyhow::anyhow!("creation grid paint: {e:?}"))?;
            }
        }
        self.trace_presentation_scroll("after-paint");
        Ok(())
    }
    pub(super) fn presentation_resize_regions(&self) -> Vec<(Rect, Axis)> {
        let mut regions = Vec::new();
        if self
            .layout
            .presentation
            .as_ref()
            .is_some_and(|state| state.nav_bar_visible())
        {
            regions.extend(self.layout.outer_view.resize_regions(&self.layout.outer));
        }
        if self
            .layout
            .presentation
            .as_ref()
            .is_some_and(|state| state.others_visible() && !state.others_expanded())
        {
            regions.extend(self.layout.inner_view.resize_regions(&self.layout.inner));
        }
        regions
    }
    pub(super) fn toggle_others_expansion(&mut self, id: &str, expanded: bool) -> Result<bool> {
        if self.composition_in_progress() {
            return Ok(false);
        }
        let Some(state) = &mut self.layout.presentation else {
            return Ok(false);
        };
        if state.selected_tab() != Some(&OthersTab::Container(id.into()))
            || !state.others_visible()
            || state.others_expanded() != expanded
        {
            return Ok(false);
        }
        state.toggle_others_expanded()?;
        self.layout.focused = self
            .panel_mgr
            .panels
            .iter()
            .position(|panel| panel.module_id == id)
            .unwrap();
        self.cancel_toolbar_input();
        if let Some(header) = &mut self.layout.presentation_tabs {
            header.event(&ContentEvent::Cancel);
        }
        if let Some(grid) = &mut self.layout.presentation_creation_grid {
            grid.host.event(&ContentEvent::Cancel);
        }
        self.layout.drag_split = None;
        self.layout.outer_view.cancel(&mut self.layout.outer);
        self.layout.inner_view.cancel(&mut self.layout.inner);
        Ok(true)
    }
    pub(super) fn switch_workspace_focus(
        &mut self,
        focus: u64,
        other: Option<u64>,
    ) -> Result<bool> {
        let current = self
            .layout
            .presentation
            .as_ref()
            .filter(|state| state.workspace_switch_enabled() && state.others_visible());
        let focus_index = current.and_then(|state| {
            self.panel_mgr
                .panels
                .iter()
                .position(|panel| panel.module_id == state.focus_id() && panel.instance == focus)
        });
        let target = current
            .and_then(|state| state.focus_switch_target(state.focus_id()))
            .and_then(|id| {
                self.panel_mgr
                    .panels
                    .iter()
                    .find(|panel| panel.module_id == id)
            });
        if self
            .nav_mgr
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
            host_diag!("PRESENTATION_REJECTED composition-in-progress");
            return Ok(false);
        }
        let owner = &self.panel_mgr.panels[index].module_id;
        let Some(target) = self
            .layout
            .presentation
            .as_ref()
            .and_then(|state| state.focus_switch_target(owner))
            .map(str::to_owned)
        else {
            return Ok(false);
        };
        self.layout
            .presentation
            .as_mut()
            .unwrap()
            .promote(&target)?;
        host_diag!(
            "PRESENTATION_SWITCH owner={owner} target={target} focus={}",
            self.layout.presentation.as_ref().unwrap().focus_id()
        );
        self.cancel_toolbar_input();
        Ok(true)
    }
    fn move_presentation_split(&mut self, point: Point) -> Result<()> {
        if self.layout.drag_split == Some(2) {
            if !self
                .layout
                .outer_view
                .pointer_move(point, &mut self.layout.outer)
            {
                return Ok(());
            }
            let area = self.layout.outer_view.layout().frame.frame();
            self.layout.presentation.as_mut().unwrap().resize_nav_width(
                (area.size.width - self.layout.outer.divider).max(0.) * self.layout.outer.ratio(),
                area,
                MIN_COLUMN,
            );
        } else {
            if !self
                .layout
                .inner_view
                .pointer_move(point, &mut self.layout.inner)
            {
                return Ok(());
            }
            let visible = self.layout.inner_view.layout().frame.frame();
            let whole = Rect::new(0., visible.min_y(), visible.max_x(), visible.size.height);
            self.layout
                .presentation
                .as_mut()
                .unwrap()
                .resize_focus_ratio(
                    self.layout.inner.ratio().clamp(0.01, 0.99),
                    whole,
                    MIN_COLUMN,
                )?;
        }
        Ok(())
    }
    pub(super) fn composition_in_progress(&mut self) -> bool {
        self.panel_mgr.panels.iter_mut().any(|panel| {
            let mut marked = false;
            if let Some(local) = &mut panel.local {
                local.with_host(&mut |host| {
                    marked = host
                        .focused_text()
                        .is_some_and(|surface| surface.marked().is_some())
                });
            }
            marked
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PresentationEventResult {
    Unhandled,
    ConsumedNoDraw,
    Handled,
}

impl Host {
    pub(super) fn presentation_event(
        &mut self,
        event: &Event,
        w: &Window,
    ) -> Result<PresentationEventResult> {
        let mut draw_needed = true;
        let handled = self.presentation_event_inner(event, w, &mut draw_needed)?;
        Ok(if !handled {
            PresentationEventResult::Unhandled
        } else if draw_needed {
            PresentationEventResult::Handled
        } else {
            PresentationEventResult::ConsumedNoDraw
        })
    }
    fn presentation_event_inner(
        &mut self,
        event: &Event,
        w: &Window,
        draw_needed: &mut bool,
    ) -> Result<bool> {
        if let Event::Scroll(scroll) = event
            && std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_some()
        {
            let point = Point::new(scroll.position.x as f32, scroll.position.y as f32);
            eprintln!(
                "OTHERS_SCROLL_ROUTE point={point:?} delta={:?} tabs={:?} in_tabs={} list={:?} pressed={} toolbar_overlay={}",
                scroll.delta,
                self.layout.presentation_layout.tabs,
                self.layout
                    .presentation_layout
                    .tabs
                    .is_some_and(|frame| frame.contains(point)),
                self.layout.presentation_layout.list_viewport,
                self.layout.pressed.is_some(),
                self.open_toolbar().is_some()
            );
            self.trace_presentation_scroll("host-route");
        }
        if self.layout.presentation.is_none() {
            return Ok(false);
        }
        if let Event::Command(CommandId(command)) = event
            && *command >= COMMAND_BASE - 2
            && *command <= COMMAND_BASE + self.panel_mgr.module_ids.len() as u32 * 6
        {
            if self.composition_in_progress() {
                host_diag!("PRESENTATION_REJECTED composition-in-progress");
                return Ok(true);
            }
            if *command > COMMAND_BASE {
                let value = (*command - COMMAND_BASE - 1) as usize;
                if value % 6 == 3 {
                    let id = self.panel_mgr.module_ids[value / 6].clone();
                    if let Err(error) = self.close_container(&id) {
                        host_diag!("PRESENTATION_REJECTED {error:#}");
                    }
                    return Ok(true);
                }
            }
            let state = self.layout.presentation.as_mut().unwrap();
            let result = if *command == COMMAND_BASE - 2 {
                state.toggle_nav_bar()
            } else if *command == COMMAND_BASE {
                state.toggle_others()
            } else if *command == COMMAND_BASE - 1 {
                state.select_tab(OthersTab::List)
            } else {
                let value = (*command - COMMAND_BASE - 1) as usize;
                let index = value / 6;
                let id = &self.panel_mgr.module_ids[index];
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
                host_diag!("PRESENTATION_REJECTED {error:#}");
            } else {
                host_diag!(
                    "PRESENTATION_CHANGED focus={} others={:?}",
                    state.focus_id(),
                    state.others()
                );
            }
            return Ok(true);
        }
        let switcher_modal = self
            .layout
            .presentation_tabs
            .as_ref()
            .is_some_and(|host| host.content_host().input_overlay_owner().is_some());
        if switcher_modal {
            if let Some(input) = super::window_toolbar_mount::native_toolbar_event(event) {
                self.presentation_tab_input(&input)?;
                return Ok(true);
            }
            if let Event::Accessibility { target, .. } = event {
                use tekes_ui_macos_bridge::WindowContentAccessibilityExt;
                let owned = w.content_accessibility_publication().is_some_and(|publication| publication.owner(*target) == Some(tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner(TABS_OWNER)));
                if !owned {
                    return Ok(true);
                }
            }
        }
        if self.open_toolbar().is_some() {
            return Ok(false);
        }
        if let Event::Scroll(scroll) = event
            && self.layout.pressed.is_none()
            && self.open_toolbar().is_none()
            && self
                .layout
                .presentation_layout
                .list_viewport
                .is_some_and(|frame| {
                    frame.contains(Point::new(
                        scroll.position.x as f32,
                        scroll.position.y as f32,
                    ))
                })
        {
            let height = self
                .layout
                .presentation_layout
                .list_viewport
                .unwrap()
                .size
                .height;
            *draw_needed = self
                .layout
                .presentation
                .as_mut()
                .unwrap()
                .scroll_list_wheel(-scroll.delta.y as f32, scroll.precise, height);
            return Ok(true);
        }
        if let Event::Mouse(mouse) = event {
            let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
            match mouse.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } if self.layout.pressed.is_none() && self.open_toolbar().is_none() => {
                    let nav = self.layout.presentation.as_ref().unwrap().nav_bar_visible();
                    let others = self.layout.presentation.as_ref().unwrap().others_visible();
                    if nav
                        && self
                            .layout
                            .outer_view
                            .pointer_down(point, &mut self.layout.outer)
                    {
                        self.cancel_toolbar_input();
                        self.layout.drag_split = Some(2);
                        return Ok(true);
                    }
                    if others
                        && !self.layout.presentation.as_ref().unwrap().others_expanded()
                        && self
                            .layout
                            .inner_view
                            .pointer_down(point, &mut self.layout.inner)
                    {
                        self.cancel_toolbar_input();
                        self.layout.drag_split = Some(3);
                        return Ok(true);
                    }
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } if self.layout.drag_split == Some(2) || self.layout.drag_split == Some(3) => {
                    self.move_presentation_split(point)?;
                    return Ok(true);
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } if self.layout.drag_split == Some(2) || self.layout.drag_split == Some(3) => {
                    self.layout.outer_view.pointer_up(&mut self.layout.outer);
                    self.layout.inner_view.pointer_up(&mut self.layout.inner);
                    self.layout.drag_split = None;
                    return Ok(true);
                }
                _ => {}
            }
        }
        if self.layout.presentation_layout.tabs.is_none() {
            return Ok(false);
        }
        let creation_frame = self.presentation_creation_frame();
        if matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down {..}) && (self.layout.presentation_layout.tabs.is_some_and(|frame| frame.contains(Point::new(mouse.position.x as f32, mouse.position.y as f32))) || creation_frame.is_some_and(|frame| frame.contains(Point::new(mouse.position.x as f32, mouse.position.y as f32)))))
            && self.composition_in_progress()
        {
            host_diag!("PRESENTATION_REJECTED composition-in-progress");
            return Ok(true);
        }
        if !self.settings_mgr.is_settings_window
            && Self::is_key_or_text_event(event)
            && self.local_overlay_panel().is_some()
        {
            return Ok(false);
        }
        let Some(host) = &mut self.layout.presentation_tabs else {
            return Ok(false);
        };
        use tekes_ui_macos_bridge::WindowContentAccessibilityExt;
        if let Event::Accessibility { target, action } = event
            && let Some(publication) = w.content_accessibility_publication()
        {
            if publication.owner(*target)
                == Some(
                    tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner(
                        super::window_switcher::CREATION_GRID_OWNER,
                    ),
                )
            {
                let consumed = if let Some(grid) = &mut self.layout.presentation_creation_grid {
                    publication
                        .dispatch(
                            tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner(
                                super::window_switcher::CREATION_GRID_OWNER,
                            ),
                            &mut grid.host,
                            *target,
                            action.clone(),
                        )
                        .consumed
                } else {
                    false
                };
                if consumed {
                    self.apply_creation_grid_actions()?;
                }
                return Ok(consumed);
            }
            if publication.owner(*target)
                == Some(
                    tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner(
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
                            (tekes_ui_macos_bridge::content_accessibility::content_key(
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
                let consumed = stamped
                    .is_some_and(|target| host.accessibility_action(target, action).consumed);
                if consumed {
                    self.apply_tab_selection()?;
                }
                return Ok(consumed);
            }
        }
        let in_grid = match event {
            Event::Mouse(mouse) => {
                self.layout.pressed.is_none()
                    && self
                        .layout
                        .presentation_creation_grid
                        .as_ref()
                        .is_some_and(|grid| {
                            grid.host.pointer_capture().is_some()
                                || grid.frame.contains(Point::new(
                                    mouse.position.x as f32,
                                    mouse.position.y as f32,
                                ))
                        })
            }
            Event::Key(_) | Event::TextKey(_) => self
                .layout
                .presentation_creation_grid
                .as_ref()
                .is_some_and(|grid| grid.host.focused().is_some()),
            _ => false,
        };
        if in_grid {
            let Some(input) = super::window_toolbar_mount::native_toolbar_event(event) else {
                return Ok(false);
            };
            if let ContentEvent::KeyInput(key) = input {
                return self.toolbar_key_input(key);
            }
            let consumed = self.presentation_creation_grid_input(&input)?;
            return Ok(consumed || matches!(event, Event::Key(_) | Event::TextKey(_)));
        }
        let in_tabs = match event {
            Event::Mouse(mouse) => {
                self.layout.pressed.is_none()
                    && (host.content_host().pointer_capture().is_some()
                        || self.layout.presentation_layout.tabs.is_some_and(|frame| {
                            frame.contains(Point::new(
                                mouse.position.x as f32,
                                mouse.position.y as f32,
                            ))
                        }))
            }
            Event::Scroll(scroll) => self.layout.presentation_layout.tabs.is_some_and(|frame| {
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
        let Some(host) = &mut self.layout.presentation_tabs else {
            return Ok(false);
        };
        let consumed = host
            .event_with_text(input, &mut self.text)
            .map_err(|e| anyhow::anyhow!("container tabs input: {e:?}"))?
            .consumed;
        if consumed {
            if host.content_host().focused().is_some() {
                if let Some(grid) = &mut self.layout.presentation_creation_grid {
                    grid.host.focus(None, false);
                }
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
    pub(super) fn presentation_creation_grid_input(
        &mut self,
        input: &ContentEvent,
    ) -> Result<bool> {
        if self.open_toolbar().is_some() {
            return Ok(false);
        }
        let Some(grid) = &mut self.layout.presentation_creation_grid else {
            return Ok(false);
        };
        let response = grid
            .host
            .event_with_text(input, &mut self.text)
            .map_err(|e| anyhow::anyhow!("creation grid input: {e:?}"))?;
        if grid.host.focused().is_some() {
            if let Some(tabs) = &mut self.layout.presentation_tabs {
                tabs.blur();
            }
            self.set_toolbar_focus(None, false);
        }
        self.apply_creation_grid_actions()?;
        Ok(response.consumed)
    }
    pub(super) fn apply_creation_grid_actions(&mut self) -> Result<()> {
        let Some(grid) = &mut self.layout.presentation_creation_grid else {
            return Ok(());
        };
        let activated: Vec<String> = grid
            .host
            .drain_actions()
            .into_iter()
            .filter_map(|record| {
                grid.choices
                    .iter()
                    .find(|(id, _)| *id == record.target)
                    .map(|(_, value)| value.clone())
            })
            .collect();
        for value in activated {
            if self.composition_in_progress() {
                host_diag!("PRESENTATION_REJECTED composition-in-progress");
                continue;
            }
            self.open_creation_choice(&value)?;
            self.set_toolbar_focus(None, false);
        }
        Ok(())
    }
    pub(super) fn apply_tab_selection(&mut self) -> Result<()> {
        let records = self
            .layout
            .presentation_tabs
            .as_mut()
            .unwrap()
            .drain_commands();
        for record in records {
            if self.composition_in_progress() {
                host_diag!("PRESENTATION_REJECTED composition-in-progress");
                continue;
            }
            use super::window_switcher::SwitcherAction;
            match record.source {
                SwitcherAction::Tabs => {
                    let Some(ContentActionPayload::TabStrip(intent)) = record.payload else {
                        continue;
                    };
                    let Some(tab) = self
                        .layout
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
                        self.layout.presentation.as_mut().unwrap().select_tab(tab)?;
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
        tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner,
        tekes_ui_macos_bridge::content_accessibility::ContentAccessibilitySnapshot,
    )> {
        use tekes_ui_macos_bridge::content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot,
        };
        self.layout.presentation_layout.tabs?;
        Some((
            ContentAccessibilityOwner(TABS_OWNER),
            ContentAccessibilitySnapshot::from_host(
                self.layout.presentation_tabs.as_ref()?.content_host(),
            )?,
        ))
    }
    pub(super) fn presentation_creation_grid_publication(
        &self,
    ) -> Option<(
        tekes_ui_macos_bridge::content_accessibility::ContentAccessibilityOwner,
        tekes_ui_macos_bridge::content_accessibility::ContentAccessibilitySnapshot,
    )> {
        use tekes_ui_macos_bridge::content_accessibility::{
            ContentAccessibilityOwner, ContentAccessibilitySnapshot,
        };
        let grid = self.layout.presentation_creation_grid.as_ref()?;
        (!grid.frame.is_empty()).then_some(())?;
        Some((
            ContentAccessibilityOwner(super::window_switcher::CREATION_GRID_OWNER),
            ContentAccessibilitySnapshot::from_host(&grid.host)?,
        ))
    }
    #[cfg(test)]
    fn business_destination(
        &self,
        owner: usize,
        retained: Option<usize>,
        focus: usize,
    ) -> (usize, ComponentId) {
        let state = self.layout.presentation.as_ref().unwrap();
        presentation_business_destination(state, &self.panel_mgr.panels, owner, retained, focus)
    }
    pub(super) fn sync_local_toolbars(&mut self) -> Result<()> {
        for panel in &mut self.panel_mgr.panels {
            panel.sync_local_toolbar()?;
        }
        Ok(())
    }
    pub(super) fn legacy_local_panel_contents(
        &self,
        index: usize,
        include_icons: bool,
    ) -> Vec<Contents> {
        let panel = &self.panel_mgr.panels[index];
        let mut out = Vec::new();
        for secondary in [false, true] {
            let entries: Vec<_> = panel
                .local_toolbar
                .iter()
                .filter(|item| panel.options.secondary_commands.contains(&item.id()) == secondary)
                .map(|item| {
                    LocalBucketEntry::from_panel_item(panel, item, None, true, true, include_icons)
                })
                .collect();
            if entries.is_empty() {
                continue;
            }
            let has_menu = entries.iter().any(|entry| is_rich_local_item(&entry.item));
            if !has_menu {
                let parts = entries
                    .iter()
                    .filter_map(|entry| match &entry.item {
                        crate::LocalToolbarItem::Button(button) => Some((
                            ToolbarButtonDeclaration {
                                id: button.id,
                                icon: None,
                                text: entry.title.clone(),
                                hint: Some(entry.hint.clone()),
                                enabled: entry.enabled,
                            },
                            entry.icon.clone().map(|name| (button.id, name)),
                            entry.selected.then_some(button.id),
                        )),
                        _ => None,
                    })
                    .collect();
                out.push(local_button_contents(
                    parts,
                    if secondary {
                        ToolbarGroupOverflow::Menu
                    } else {
                        ToolbarGroupOverflow::Split
                    },
                ));
            } else if secondary {
                let overflow_entries: Vec<_> = entries
                    .iter()
                    .map(LocalBucketEntry::to_overflow_entry)
                    .collect();
                out.push(Contents::Menu(overflow_menu_declaration(
                    0,
                    &overflow_entries,
                    include_icons,
                )));
            } else {
                let base = 100_000_000u64 + (panel.instance & 0xFFFF_FFFF) * 1_000_000;
                for entry in &entries {
                    out.push(entry.to_single_contents(panel, base, include_icons));
                }
            }
        }
        out
    }
    pub(super) fn emit_legacy_local_panel_declarations(
        &mut self,
        index: usize,
        slot: usize,
        open: bool,
        include_icons: bool,
        budget: Option<f32>,
        declarations: &mut Vec<Declaration>,
    ) {
        let motion_busy = self.layout.sidebar_motion.needs_frame();
        let is_window_host = matches!(
            self.panel_mgr.panels[index].options.toolbar_host,
            crate::ContainerToolbarHost::Window
        );
        let is_vertical = matches!(
            self.panel_mgr.panels[index].options.toolbar_host,
            crate::ContainerToolbarHost::Container {
                position: crate::ContainerToolbarPosition::Leading
                    | crate::ContainerToolbarPosition::Trailing
            }
        );
        for secondary in [false, true] {
            let entries: Vec<_> = {
                let panel = &self.panel_mgr.panels[index];
                panel
                    .local_toolbar
                    .iter()
                    .filter(|item| {
                        panel.options.secondary_commands.contains(&item.id()) == secondary
                    })
                    .map(|item| {
                        LocalBucketEntry::from_panel_item(
                            panel,
                            item,
                            None,
                            true,
                            open && !motion_busy,
                            include_icons,
                        )
                    })
                    .collect()
            };
            if entries.is_empty() {
                continue;
            }
            let has_menu = entries.iter().any(|entry| is_rich_local_item(&entry.item));
            let budget_ctx = if !secondary && !is_vertical && has_menu {
                budget.map(|usable_w| {
                    let used = {
                        let matching: Vec<&Declaration> = declarations
                            .iter()
                            .filter(|decl| match &decl.source {
                                Source::Panel { index: owner, .. }
                                | Source::PanelOverflow { index: owner, .. } => {
                                    *owner == index
                                        || (is_window_host
                                            && matches!(
                                                self.panel_mgr.panels[*owner].options.toolbar_host,
                                                crate::ContainerToolbarHost::Window
                                            ))
                                }
                                _ => false,
                            })
                            .collect();
                        if matching.is_empty() {
                            2. * toolbar_spacing(&mut self.text, "space-regular")
                        } else {
                            occupied_region_width(
                                &mut self.text,
                                &matching,
                                ToolbarPlacement::PrimaryAction,
                                &self.extension_icons,
                            )
                        }
                    };
                    (usable_w - used).max(0.)
                })
            } else {
                None
            };
            let panel = &self.panel_mgr.panels[index];
            let button_only_id =
                ComponentId(92400 + index as u64 * 10 + if secondary { 8_000_000 } else { 0 });
            emit_local_bucket_declarations(
                panel,
                index,
                &entries,
                ComponentId(91000 + slot as u64),
                ToolbarPlacement::PrimaryAction,
                secondary,
                button_only_id,
                10,
                if is_window_host { 0 } else { 3 },
                include_icons,
                &self.extension_icons,
                budget_ctx.map(|avail| (&mut self.text, avail)),
                declarations,
            );
        }
    }
    fn window_partition_frame(
        &self,
        region: ComponentId,
        whole: Rect,
        band: &window_toolbar_mount::ToolbarBand,
    ) -> Rect {
        let state = self.layout.presentation.as_ref().unwrap();
        let column = if region == WINDOW_OTHERS_BUSINESS {
            self.layout.presentation_layout.tabs.unwrap_or_default()
        } else {
            let id = if region == WINDOW_NAV_BUSINESS {
                state.visible_nav_bar_id()
            } else {
                Some(state.focus_id())
            };
            id.and_then(|id| {
                self.panel_mgr
                    .panels
                    .iter()
                    .find(|panel| panel.module_id == id)
            })
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
        let mut frame = self.layout.presentation_layout.tabs?;
        let mut region = ToolbarRegion::new(ComponentId(TABS_OWNER), frame);
        reserve_window_edges(&mut region, self.layout.presentation_chrome_edges);
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
            .panel_mgr
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
    fn compute_others_tabs_reserve(
        &mut self,
        focus: usize,
        whole: Rect,
        band: &window_toolbar_mount::ToolbarBand,
        include_icons: bool,
    ) -> f32 {
        let declarations = Self::build_presentation_declarations(
            &self.panel_mgr.panels,
            &self.layout,
            &self.nav_mgr,
            &self.extension_icons,
            include_icons,
            None,
        );
        let others_decls: Vec<_> = declarations[focus]
            .iter()
            .filter(|decl| decl.region == WINDOW_OTHERS_BUSINESS)
            .collect();
        if others_decls.is_empty() {
            return 0.;
        }
        let frame = self.window_partition_frame(WINDOW_OTHERS_BUSINESS, whole, band);
        if frame.size.width <= 0. {
            return 0.;
        }
        let mut region = ToolbarRegion::new(WINDOW_OTHERS_BUSINESS, frame);
        region.native_safe_area.left =
            (band.native_leading - frame.min_x()).clamp(0., frame.size.width);
        reserve_window_edges(&mut region, self.layout.presentation_chrome_edges);
        let usable_others_w = (region.available.size.width
            - region.native_safe_area.left
            - region.native_safe_area.right)
            .max(0.);
        if usable_others_w <= 0. {
            return 0.;
        }
        let gap = toolbar_spacing(&mut self.text, "space-regular");
        let tight = toolbar_spacing(&mut self.text, "space-tight");
        let (mut c0, mut c2) = (0usize, 0usize);
        let mut sum = 0.0f32;
        for decl in &others_decls {
            sum += measure_contents_width(&mut self.text, &decl.contents, &self.extension_icons);
            if matches!(decl.placement, ToolbarPlacement::Navigation) {
                c0 += 1;
            } else {
                c2 += 1;
            }
        }
        let business_w = 2. * gap
            + sum
            + tight * c0.saturating_sub(1) as f32
            + gap * c2.saturating_sub(1) as f32
            + if c0 > 0 && c2 > 0 { gap } else { 0. };
        let empty_overflow = overflow_menu_declaration(0, &[], include_icons);
        let overflow_trigger_w =
            measure_menu_width(&mut self.text, &empty_overflow, &self.extension_icons);
        let min_business_w = (2. * gap + overflow_trigger_w).min(business_w);
        let (full_tabs_w, min_tabs_w) = self.measure_switcher_widths();
        if full_tabs_w <= 0. {
            return 0.;
        }
        if full_tabs_w + business_w <= usable_others_w {
            full_tabs_w
        } else {
            let max_tabs = (usable_others_w - min_business_w).max(0.);
            let target_tabs = full_tabs_w.min(min_tabs_w.max(usable_others_w * 0.45));
            target_tabs.min(max_tabs)
        }
    }
    #[cfg(test)]
    fn container_toolbar_regions(
        &self,
        index: usize,
        focus: usize,
        whole: Rect,
        band: &window_toolbar_mount::ToolbarBand,
    ) -> Vec<ToolbarRegion> {
        self.container_toolbar_regions_with_reserve(index, focus, whole, band, 0.)
    }
    fn container_toolbar_regions_with_reserve(
        &self,
        index: usize,
        focus: usize,
        whole: Rect,
        band: &window_toolbar_mount::ToolbarBand,
        others_tabs_reserve: f32,
    ) -> Vec<ToolbarRegion> {
        let panel = &self.panel_mgr.panels[index];
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
                reserve_window_edges(&mut region, self.layout.presentation_chrome_edges);
                if id == WINDOW_OTHERS_BUSINESS && others_tabs_reserve > 0. {
                    let max_left = (frame.size.width - region.native_safe_area.right)
                        .max(region.native_safe_area.left);
                    region.native_safe_area.left =
                        (region.native_safe_area.left + others_tabs_reserve).min(max_left);
                }
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
    #[cfg(test)]
    fn navigation_toolbar_declaration(
        &self,
        index: usize,
        focus: usize,
        source: crate::toolbar_source::ComponentSource,
        buttons: Vec<ToolbarButtonDeclaration>,
        templates: Vec<(u64, String)>,
    ) -> (usize, Declaration) {
        let state = self.layout.presentation.as_ref().unwrap();
        build_navigation_toolbar_declaration(
            state,
            &self.panel_mgr.panels,
            index,
            focus,
            source,
            buttons,
            templates,
        )
    }
    #[cfg(test)]
    pub(super) fn presentation_toolbar_declarations(&self) -> Vec<Vec<Declaration>> {
        self.presentation_toolbar_declarations_with_icons(self.renderer.is_some())
    }
    // Projection depends on whether the receiving text service can prepare the
    // original toolkit icons; it does not require constructing a GPU renderer.
    pub(super) fn presentation_toolbar_declarations_with_icons(
        &self,
        include_icons: bool,
    ) -> Vec<Vec<Declaration>> {
        Self::build_presentation_declarations(
            &self.panel_mgr.panels,
            &self.layout,
            &self.nav_mgr,
            &self.extension_icons,
            include_icons,
            None,
        )
    }
    fn build_presentation_declarations(
        panels: &[window_panels::Panel],
        layout: &window_layout::WindowLayoutState,
        nav_mgr: &NavigationCoordinator,
        extension_icons: &[tekes_ui_icons::Glyph],
        include_icons: bool,
        mut budget_ctx: Option<(&mut Text, &BTreeMap<ComponentId, f32>)>,
    ) -> Vec<Vec<Declaration>> {
        let input_open = nav_mgr
            .navigation
            .as_ref()
            .is_none_or(|nav| nav.host.input_open && !nav.busy && nav.error.is_none());
        let state = layout.presentation.as_ref().unwrap();
        let mut declarations = vec![Vec::new(); panels.len()];
        let focus = panels
            .iter()
            .position(|panel| panel.module_id == state.focus_id())
            .unwrap();
        let mut local_menu_panels = Vec::new();
        for (index, panel) in panels.iter().enumerate() {
            let visible = panel.bounds.size.width > 0. && panel.bounds.size.height > 0.;
            let in_others = state.others_ids().contains(&panel.module_id)
                || (state.others_expanded() && state.focus_id() == panel.module_id);
            if visible
                && !in_others
                && matches!(
                    panel.options.toolbar_host,
                    crate::ContainerToolbarHost::Window
                )
                && let Some(title) = &panel.options.title
            {
                let (presenter, region) =
                    presentation_business_destination(state, panels, index, None, focus);
                declarations[presenter].push(Declaration {
                    id: ComponentId(2_000_000 + index as u64 * 10),
                    region,
                    placement: ToolbarPlacement::Navigation,
                    order: 20,
                    contents: Contents::Title(title.clone()),
                    source: Source::Title,
                });
            }
            if panel.local.is_some() && !panel.local_toolbar.is_empty() {
                local_menu_panels.push(index);
                continue;
            }
            type TargetCommands = (Vec<ToolbarButtonDeclaration>, bool, Vec<(u64, String)>);
            let mut targets: BTreeMap<(usize, u64, bool), TargetCommands> = BTreeMap::new();
            for action in &panel.toolbar {
                if state.workspace_switch_enabled()
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
                            panels.iter().position(|target| {
                                target.module_id == retained.target
                                    && target.bounds.size.width > 0.
                                    && target.bounds.size.height > 0.
                                    && (target.has_toolbar_items()
                                        || target.options.title.is_some())
                            })
                        })
                };
                if let Some(target) = target {
                    let retained_target = !visible;
                    let (target, region) = presentation_business_destination(
                        state,
                        panels,
                        index,
                        (!visible).then_some(target),
                        focus,
                    );
                    let switch = panel.options.focus_switch_command == Some(action.id);
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
        if state.workspace_switch_enabled() {
            let other = state
                .others_visible()
                .then(|| state.focus_switch_target(state.focus_id()))
                .flatten()
                .and_then(|id| panels.iter().find(|panel| panel.module_id == id));
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
                    focus: panels[focus].instance,
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
        if let Some(nav) = &nav_mgr.navigation
            && let Some(source) = nav.toolbar_source()
            && let Some(id) = state.navigation_toolbar_presenter_id()
            && let Some(index) = panels.iter().position(|panel| panel.module_id == id)
            && panels[index].bounds.size.width > 0.
            && panels[index].bounds.size.height > 0.
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
            let (presenter, declaration) = build_navigation_toolbar_declaration(
                state, panels, index, focus, source, buttons, templates,
            );
            declarations[presenter].push(declaration);
        }
        // Emit rich local container toolbar buckets after all window siblings so
        // budget accounting observes Title, Navigation, and WorkspaceSwitch widths.
        let mut local_buckets: Vec<(usize, usize, u64, bool, bool, Vec<LocalBucketEntry>)> =
            Vec::new();
        for index in local_menu_panels {
            let panel = &panels[index];
            let visible = panel.bounds.size.width > 0. && panel.bounds.size.height > 0.;
            let mut grouped: BTreeMap<(usize, u64, bool), (bool, Vec<LocalBucketEntry>)> =
                BTreeMap::new();
            for item in &panel.local_toolbar {
                if state.workspace_switch_enabled()
                    && panel.options.focus_switch_command == Some(item.id())
                {
                    continue;
                }
                let secondary = panel.options.secondary_commands.contains(&item.id());
                let target = if visible {
                    Some(index)
                } else {
                    panel
                        .options
                        .retained_buttons
                        .iter()
                        .find(|retained| retained.command == item.id())
                        .and_then(|retained| {
                            panels.iter().position(|target| {
                                target.module_id == retained.target
                                    && target.bounds.size.width > 0.
                                    && target.bounds.size.height > 0.
                                    && (target.has_toolbar_items()
                                        || target.options.title.is_some())
                            })
                        })
                };
                if let Some(target) = target {
                    let retained_target = !visible;
                    let (target_presenter, region) = presentation_business_destination(
                        state,
                        panels,
                        index,
                        retained_target.then_some(target),
                        focus,
                    );
                    let entry = LocalBucketEntry::from_panel_item(
                        panel,
                        item,
                        Some(state),
                        visible,
                        input_open,
                        include_icons,
                    );
                    let bucket = grouped
                        .entry((target_presenter, region.0, secondary))
                        .or_default();
                    bucket.0 |= retained_target;
                    bucket.1.push(entry);
                }
            }
            for ((target_presenter, region, secondary), (retained_target, entries)) in grouped {
                local_buckets.push((
                    index,
                    target_presenter,
                    region,
                    secondary,
                    retained_target,
                    entries,
                ));
            }
        }
        local_buckets.sort_by_key(|(index, _, _, secondary, _, entries)| {
            let has_menu = entries.iter().any(|entry| is_rich_local_item(&entry.item));
            (!*secondary, has_menu, *index)
        });
        for (index, target_presenter, region, secondary, retained_target, entries) in local_buckets
        {
            let panel = &panels[index];
            let window_region = [
                WINDOW_BUSINESS.0,
                WINDOW_NAV_BUSINESS.0,
                WINDOW_OTHERS_BUSINESS.0,
            ]
            .contains(&region);
            let primary_placement = if window_region {
                ToolbarPlacement::PrimaryAction
            } else {
                ToolbarPlacement::Navigation
            };
            let base_id = if retained_target && window_region {
                let role = u64::MAX - 18 - region;
                ComponentId(4_000_000 + index as u64 * 40 + role * 10)
            } else {
                ComponentId(92400 + index as u64 * 10)
            };
            let button_only_id = if secondary {
                ComponentId(base_id.0 + 8_000_000)
            } else {
                base_id
            };
            let overflow_role = if window_region {
                u64::MAX - 18 - region
            } else {
                3 + u64::from(retained_target)
            };
            let has_menu = entries.iter().any(|entry| is_rich_local_item(&entry.item));
            let budget = if !secondary && has_menu {
                match &mut budget_ctx {
                    Some((text, budgets)) => budgets.get(&ComponentId(region)).map(|&usable_w| {
                        let existing: Vec<&Declaration> = declarations[target_presenter]
                            .iter()
                            .filter(|decl| decl.region == ComponentId(region))
                            .collect();
                        let used = occupied_region_width(
                            text,
                            &existing,
                            primary_placement,
                            extension_icons,
                        );
                        (&mut **text, (usable_w - used).max(0.))
                    }),
                    None => None,
                }
            } else {
                None
            };
            emit_local_bucket_declarations(
                panel,
                index,
                &entries,
                ComponentId(region),
                primary_placement,
                secondary,
                button_only_id,
                index as i32,
                overflow_role,
                include_icons,
                extension_icons,
                budget,
                &mut declarations[target_presenter],
            );
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
        let mut renderer = self.renderer.take();
        let result = self.sync_container_toolbars_with_renderer(
            active,
            whole,
            band,
            native_height,
            renderer.as_mut(),
        );
        self.renderer = renderer;
        result
    }
    #[cfg(test)]
    pub(super) fn sync_container_toolbars_with_icons(
        &mut self,
        active: bool,
        whole: Rect,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
        include_icons: bool,
    ) -> Result<()> {
        let mut renderer = self.renderer.take();
        let result = self.sync_container_toolbars_inner(
            active,
            whole,
            band,
            native_height,
            include_icons,
            renderer.as_mut(),
        );
        self.renderer = renderer;
        result
    }
    pub(super) fn sync_container_toolbars_with_renderer(
        &mut self,
        active: bool,
        whole: Rect,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
        renderer: Option<&mut Renderer>,
    ) -> Result<()> {
        let include_icons = renderer.is_some();
        self.sync_container_toolbars_inner(
            active,
            whole,
            band,
            native_height,
            include_icons,
            renderer,
        )
    }
    fn sync_container_toolbars_inner(
        &mut self,
        active: bool,
        whole: Rect,
        band: window_toolbar_mount::ToolbarBand,
        native_height: f32,
        include_icons: bool,
        mut renderer: Option<&mut Renderer>,
    ) -> Result<()> {
        self.sync_local_toolbars()?;
        let boundary = self
            .layout
            .container_window_boundary
            .unwrap_or((band.y + band.height).max(native_height));
        for index in 0..self.panel_mgr.panels.len() {
            let bounds = self.panel_mgr.panels[index].bounds;
            if bounds.size.width > 0. && bounds.size.height > 0. {
                let geometry =
                    self.container_geometry_with_icons(index, bounds, boundary, include_icons)?;
                self.panel_geometry_layout(
                    index,
                    bounds,
                    self.panel_mgr.panels[index].scale,
                    geometry,
                )?;
            }
        }
        let focus = self
            .panel_mgr
            .panels
            .iter()
            .position(|panel| {
                panel.module_id == self.layout.presentation.as_ref().unwrap().focus_id()
            })
            .unwrap();
        let others_tabs_reserve =
            self.compute_others_tabs_reserve(focus, whole, &band, include_icons);
        let regions: Vec<_> = (0..self.panel_mgr.panels.len())
            .map(|index| {
                self.container_toolbar_regions_with_reserve(
                    index,
                    focus,
                    whole,
                    &band,
                    others_tabs_reserve,
                )
            })
            .collect();
        let budgets = horizontal_region_budgets(&regions);
        let mut declarations = Self::build_presentation_declarations(
            &self.panel_mgr.panels,
            &self.layout,
            &self.nav_mgr,
            &self.extension_icons,
            include_icons,
            Some((&mut self.text, &budgets)),
        );
        for (index, panel) in self.panel_mgr.panels.iter_mut().enumerate() {
            let keep: Vec<_> = declarations[index]
                .iter()
                .map(|declaration| declaration.id)
                .collect();
            panel.mounted_toolbar.retain_groups(&keep)?;
        }
        let list_members: Vec<_> = (0..self.panel_mgr.panels.len())
            .map(|index| self.is_list_member(index))
            .collect();
        for (index, panel) in self.panel_mgr.panels.iter_mut().enumerate() {
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
        let previous_edges = self.layout.presentation_chrome_edges;
        let palette = self.palette();
        measure_presentation_toolbar_host(
            &mut self.panel_mgr.panels[focus].mounted_toolbar,
            &mut self.text,
            palette,
            renderer.as_deref_mut(),
        )?;
        let toolbar = &self.panel_mgr.panels[focus].mounted_toolbar;
        self.layout.presentation_chrome_edges = [WINDOW_LEADING, WINDOW_TRAILING].map(|id| {
            toolbar
                .host
                .content_host()
                .get(id)
                .map_or(Rect::default(), |node| node.frame())
        });
        // A second identical sync is an original-host no-op; changed safe areas
        // retire stale geometry before any pointer release can be delivered.
        if self.layout.presentation_chrome_edges != previous_edges {
            let others_tabs_reserve =
                self.compute_others_tabs_reserve(focus, whole, &band, include_icons);
            let regions: Vec<_> = (0..self.panel_mgr.panels.len())
                .map(|index| {
                    self.container_toolbar_regions_with_reserve(
                        index,
                        focus,
                        whole,
                        &band,
                        others_tabs_reserve,
                    )
                })
                .collect();
            let budgets = horizontal_region_budgets(&regions);
            let mut updated_declarations = Self::build_presentation_declarations(
                &self.panel_mgr.panels,
                &self.layout,
                &self.nav_mgr,
                &self.extension_icons,
                include_icons,
                Some((&mut self.text, &budgets)),
            );
            let focus_declarations = std::mem::take(&mut updated_declarations[focus]);
            let keep: Vec<_> = focus_declarations
                .iter()
                .map(|declaration| declaration.id)
                .collect();
            let panel = &mut self.panel_mgr.panels[focus];
            panel.mounted_toolbar.retain_groups(&keep)?;
            panel.mounted_toolbar.sync(
                if list_members[focus] {
                    panel.clip
                } else {
                    whole
                },
                regions[focus].clone(),
                focus_declarations,
            )?;
            panel.mounted_toolbar.host.set_window_active(active);
            measure_presentation_toolbar_host(
                &mut panel.mounted_toolbar,
                &mut self.text,
                palette,
                renderer,
            )?;
        } else {
            let toolbar = &mut self.panel_mgr.panels[focus].mounted_toolbar;
            toolbar
                .host
                .set_regions(toolbar.window, whole, regions[focus].clone())
                .map_err(|error| anyhow::anyhow!("window toolbar reservation: {error:?}"))?;
        }
        let _ = native_height;
        Ok(())
    }
}
fn measure_presentation_toolbar_host(
    toolbar: &mut crate::window_toolbar::MountedToolbar,
    text: &mut Text,
    palette: Palette,
    renderer: Option<&mut Renderer>,
) -> Result<()> {
    let mut measure = Scene::new();
    if let Some(renderer) = renderer {
        with_content_icons(renderer, &mut toolbar.icons, text, |text| {
            toolbar
                .host
                .render(&mut measure, text, palette)
                .map_err(|error| anyhow::anyhow!("window toolbar measure: {error:?}"))
        })
    } else {
        #[cfg(test)]
        {
            render_test_tabs_with_extensions(
                &mut toolbar.host,
                &mut measure,
                text,
                palette,
                toolbar.icons.extensions(),
            )
        }
        #[cfg(not(test))]
        {
            toolbar
                .host
                .render(&mut measure, text, palette)
                .map_err(|error| anyhow::anyhow!("window toolbar measure: {error:?}"))
        }
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
        host.panel_mgr.panels[0].bounds = Rect::new(0., 0., 600., 300.);
        host.panel_mgr.panels[0].clip = host.panel_mgr.panels[0].bounds;

        (host, calls)
    }
    fn paint(host: &mut Host) {
        host.sync_container_toolbars(true, Rect::new(0., 0., 600., 600.), test_band(), 40.)
            .unwrap();
        for panel in &mut host.panel_mgr.panels {
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
        host.panel_mgr.panels[0].options.toolbar_host = crate::ContainerToolbarHost::Window;
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others()
            .unwrap();
        host.toggle_others_expansion("other", false).unwrap();
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("focus".into()))
            .unwrap();
        let whole = Rect::new(0., 0., 600., 600.);
        let layout = host
            .layout
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
            host.layout
                .presentation
                .as_ref()
                .unwrap()
                .focus_switch_target("focus"),
            None
        );
        assert!(host.toggle_others_expansion("focus", true).unwrap());
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().selected_tab(),
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
            &mut host.panel_mgr.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            palette,
        )
        .unwrap();
        let target = host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run focus")
            .unwrap()
            .1
            .frame;
        host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .event(&ContentEvent::PointerMove(target.center()));
        host.text.begin_frame(Duration::from_millis(10), false);
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        let mut scene = Scene::new();
        render_test_tabs(
            &mut host.panel_mgr.panels[0].mounted_toolbar.host,
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
        let original: Vec<_> = host
            .panel_mgr
            .panels
            .iter()
            .map(|panel| panel.instance)
            .collect();
        let area = Rect::new(0., 0., 1200., 600.);
        let layout = host.layout.presentation.as_ref().unwrap().layout(area);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (panel, frame) in host.panel_mgr.panels.iter_mut().zip(frames) {
            panel.bounds = frame;
        }
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        host.toolbar_mgr.toolbar_index = 0;
        host.paint_presentation(&mut Scene::new(), area, Theme::LIGHT.into())
            .unwrap();
        let header = host.layout.presentation_tabs.as_mut().unwrap();
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
            host.layout.presentation.as_ref().unwrap().others_ids(),
            &["second"]
        );
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .open("first")
            .unwrap();
        assert_eq!(
            host.panel_mgr
                .panels
                .iter()
                .map(|panel| panel.instance)
                .collect::<Vec<_>>(),
            original
        );
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().others_ids(),
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
            let layout = host.layout.presentation.as_ref().unwrap().layout(area);
            let frames = host.apply_presentation_layout(layout).unwrap();
            for (panel, frame) in host.panel_mgr.panels.iter_mut().zip(frames) {
                panel.bounds = frame;
            }
            host.sync_container_toolbars(true, area, test_band(), 40.)
                .unwrap();
            for panel in &mut host.panel_mgr.panels {
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
        let part = host.panel_mgr.panels[0]
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
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::List)
            .unwrap();
        render(&mut host);
        host.dispatch_toolbar(target, &ContentEvent::PointerUp(down))
            .unwrap();
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().focus_id(),
            "focus"
        );
        assert!(
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .any(|(_, part)| part.semantics.name == "Switch to Others"
                    && part.semantics.disabled)
        );
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("other".into()))
            .unwrap();
        render(&mut host);
        let point = host.panel_mgr.panels[1]
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
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().focus_id(),
            "other"
        );
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
            host.panel_mgr.panels[1]
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
        host.panel_mgr.panels[0].bounds = Rect::new(9., 36., 591., 250.);
        host.panel_mgr.panels[0].clip = host.panel_mgr.panels[0].bounds;

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
            &mut host.panel_mgr.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        for (_, part) in host.panel_mgr.panels[0]
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
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others()
            .unwrap();
        let area = Rect::new(0., 0., 600., 600.);
        let layout = host.layout.presentation.as_ref().unwrap().layout(area);
        host.apply_presentation_layout(layout).unwrap();
        host.paint_presentation(&mut Scene::new(), area, Theme::LIGHT.into())
            .unwrap();
        let divider = host.layout.inner_view.layout().divider.frame();
        let down = divider.center();
        assert!(
            host.layout
                .inner_view
                .pointer_down(down, &mut host.layout.inner)
        );
        host.layout.drag_split = Some(3);
        let original = host
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .layout(area)
            .frames;
        host.move_presentation_split(Point::new(down.x + 1., down.y))
            .unwrap();
        assert_eq!(
            host.layout
                .presentation
                .as_ref()
                .unwrap()
                .layout(area)
                .frames,
            original
        );
        host.move_presentation_split(Point::new(down.x + 60., down.y))
            .unwrap();
        let frame = host
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .layout(area)
            .frames[0]
            .1;
        assert!(
            (frame.size.width - host.layout.inner.ratio() * (area.size.width - 1.)).abs() < 0.01
        );
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
            .panel_mgr
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
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .promote("second")
            .unwrap();
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().others_ids(),
            &["first", "focus"]
        );
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().selected_tab(),
            Some(&OthersTab::Container("first".into()))
        );
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::List)
            .unwrap();
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .promote("first")
            .unwrap();
        let area = Rect::new(0., 0., 900., 600.);
        let layout = host.layout.presentation.as_ref().unwrap().layout(area);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (panel, frame) in host.panel_mgr.panels.iter_mut().zip(frames) {
            panel.bounds = frame;
        }
        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for (index, panel) in host.panel_mgr.panels.iter_mut().enumerate() {
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
        let point = host.panel_mgr.panels[1]
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
        host.panel_mgr.panels[0].bounds = area;
        host.panel_mgr.panels[0].clip = area;

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
            &mut host.panel_mgr.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let parts: Vec<_> = host.panel_mgr.panels[0]
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
                    host.panel_mgr.panels[0]
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
            host.panel_mgr.panels[1]
                .mounted_toolbar
                .host
                .semantics()
                .iter()
                .all(|node| node.parts.is_empty())
        );
        let target = host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap()
            .0;
        assert!(
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.toolbar_mgr.toolbar_index = 0;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 1);
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 1);
        assert!(
            host.panel_mgr.panels[0].mounted_toolbar.window
                != host.panel_mgr.panels[1].mounted_toolbar.window
        );
    }
    #[test]
    fn retained_move_revokes_old_press_and_old_ax_target_before_rebinding() {
        let (mut host, calls) = fixture();
        paint(&mut host);
        let (target, part) = host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap();
        let old_point = part.frame.center();
        host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .event(&ContentEvent::PointerDown(old_point));
        host.panel_mgr.panels[1].bounds = Rect::new(0., 320., 600., 200.);
        host.panel_mgr.panels[1].clip = host.panel_mgr.panels[1].bounds;

        paint(&mut host);
        assert!(
            !host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .event(&ContentEvent::PointerUp(old_point));
        host.toolbar_mgr.toolbar_index = 0;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 0);
        let target = host.panel_mgr.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap()
            .0;
        assert!(
            host.panel_mgr.panels[1]
                .mounted_toolbar
                .host
                .accessibility_action(target, AccessibilityAction::Press)
                .consumed
        );
        host.toolbar_mgr.toolbar_index = 1;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 1);
    }
    #[test]
    fn retained_target_without_toolbar_is_not_invented_and_disabled_owner_cannot_run() {
        let (mut host, calls) = fixture();
        host.panel_mgr.panels[0].toolbar.clear();
        host.panel_mgr.panels[0].options.title = None;

        paint(&mut host);
        assert!(
            !host.panel_mgr.panels[0]
                .mounted_toolbar
                .sources()
                .any(|source| matches!(source, Source::Panel { index: 0, .. })),
            "removed business toolbar has no mounted original command source"
        );
        assert!(
            host.panel_mgr.panels[0].geometry.toolbar.is_none(),
            "no business declaration adds an internal bar"
        );
        assert!(host.presentation_menu().items.iter().any(|item| matches!(item, MenuItem::Command(command) if command.command == CommandId(COMMAND_BASE))), "menu recovery remains available without inventing a toolbar row");
        assert!(
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .sources()
                .any(|source| matches!(source, Source::Visibility(_))),
            "window chrome remains available independently of an internal business bar"
        );
        host.panel_mgr.panels[0].options.title = Some("Host".into());

        host.panel_mgr.panels[1].toolbar[0].enabled = false;
        paint(&mut host);
        let target = host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Run other")
            .unwrap()
            .0;
        host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .accessibility_action(target, AccessibilityAction::Press);
        host.toolbar_mgr.toolbar_index = 0;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(calls.get(), 0);
        assert!(
            host.panel_mgr.panels[0]
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
        let layout = host.layout.presentation.as_ref().unwrap().layout(area);
        let frames = host.apply_presentation_layout(layout).unwrap();
        for (index, frame) in frames.into_iter().enumerate() {
            let panel = &mut host.panel_mgr.panels[index];
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
        host.layout.container_window_boundary = Some(48.);
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
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others_expanded()
            .unwrap();
        refresh_window_chrome(&mut host);
        assert!(host.switch_workspace_focus(focus, other).unwrap());
        assert!(!host.layout.presentation.as_ref().unwrap().others_expanded());
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().focus_id(),
            "other"
        );
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
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others()
            .unwrap();
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
                host.layout
                    .presentation
                    .as_mut()
                    .unwrap()
                    .toggle_others_expanded()
                    .unwrap();
                refresh_window_chrome(&mut host);
            }
            let point = host.panel_mgr.panels[0]
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
            let row = host.panel_mgr.panels[0]
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
                host.layout.presentation.as_ref().unwrap().focus_id(),
                "other",
                "expanded={expanded}"
            );
            assert!(!host.layout.presentation.as_ref().unwrap().others_expanded());
            host.drain_toolbar_commands().unwrap();
            assert_eq!(
                host.layout.presentation.as_ref().unwrap().focus_id(),
                "other"
            );
            assert_eq!(host.open_toolbar(), None);
        }
    }
    #[test]
    fn expanded_workspace_menu_survives_real_hidden_draw_between_press_and_release() {
        let mut host = workspace_secondary_fixture();
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others_expanded()
            .unwrap();
        refresh_window_chrome(&mut host);
        let presenter = host
            .panel_mgr
            .panels
            .iter()
            .position(|panel| panel.module_id == "focus")
            .unwrap();
        let point = host.panel_mgr.panels[presenter]
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
        let action = host.panel_mgr.panels[presenter]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Switch Focus and Others")
            .unwrap()
            .0;
        host.panel_mgr.panels[presenter]
            .mounted_toolbar
            .host
            .accessibility_action(action, AccessibilityAction::Press);
        host.toolbar_mgr.toolbar_index = presenter;
        host.drain_toolbar_commands().unwrap();
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().focus_id(),
            "other"
        );
        assert!(!host.layout.presentation.as_ref().unwrap().others_expanded());
        refresh_window_chrome(&mut host);
        assert!(
            host.panel_mgr
                .panels
                .iter()
                .all(|panel| panel.bounds.size.width > 0.)
        );
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
        host.layout.container_window_boundary = Some(48.);
        refresh_window_chrome(&mut host);
        host
    }
    fn refresh_window_chrome(host: &mut Host) {
        let whole = Rect::new(0., 0., 1200., 700.);
        render_layout(host, whole);
        for index in 0..host.panel_mgr.panels.len() {
            let bounds = host.panel_mgr.panels[index].bounds;
            if bounds.size.width <= 0. {
                host.retire_hidden_content(index, bounds);
            }
        }
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panel_mgr.panels {
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
            let state = host.layout.presentation.as_ref().unwrap();
            let target = host
                .panel_mgr
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
            host.panel_mgr.panels[target].options.toolbar_host =
                crate::ContainerToolbarHost::Container {
                    position: crate::ContainerToolbarPosition::Bottom,
                };
            assert_eq!(
                host.business_destination(target, None, index),
                (target, ComponentId(91000 + target as u64)),
                "follows-focus must not override explicit internal host"
            );
            host.layout
                .presentation
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
                    .panel_mgr
                    .panels
                    .iter()
                    .find(|panel| panel.module_id == id)
                    .unwrap()
                    .bounds;
                if expected.size.width > 0. {
                    let title = host
                        .panel_mgr
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
            host.layout
                .presentation
                .as_mut()
                .unwrap()
                .toggle_nav_bar()
                .unwrap();
            refresh_window_chrome(&mut host);
            let expected = if host.layout.presentation.as_ref().unwrap().nav_bar_visible() {
                "Hide NavBar"
            } else {
                "Show NavBar"
            };
            assert!(
                host.panel_mgr
                    .panels
                    .iter()
                    .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                    .flat_map(|node| node.parts)
                    .any(|(_, part)| part.semantics.name == expected && part.frame.size.width > 0.)
            );
        }
        host.layout.presentation.as_mut().unwrap().resize_nav_width(
            300.,
            Rect::new(0., 0., 1200., 700.),
            MIN_COLUMN,
        );
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .set_focus_fraction(0.6)
            .unwrap();
        refresh_window_chrome(&mut host);
        let focus = host.panel_mgr.panels[1].bounds;
        let region = host.panel_mgr.panels[1]
            .mounted_toolbar
            .host
            .content_host()
            .get(WINDOW_BUSINESS)
            .unwrap()
            .frame();
        assert!(region.min_x() >= focus.min_x() && region.max_x() <= focus.max_x());
        for _ in 0..2 {
            host.layout
                .presentation
                .as_mut()
                .unwrap()
                .toggle_others()
                .unwrap();
            refresh_window_chrome(&mut host);
            let expected = if host.layout.presentation.as_ref().unwrap().others_visible() {
                "Hide Others"
            } else {
                "Show Others"
            };
            assert!(
                host.panel_mgr
                    .panels
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
            host.layout
                .presentation
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
            window: host.panel_mgr.panels[1].mounted_toolbar.window,
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
        host.panel_mgr.panels[presenter]
            .mounted_toolbar
            .sync(whole, regions, std::mem::take(&mut declarations[presenter]))
            .unwrap();
        render_test_tabs(
            &mut host.panel_mgr.panels[presenter].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let palette = host.palette();
        host.paint_presentation(&mut Scene::new(), whole, palette)
            .unwrap();
        let frame = host.panel_mgr.panels[presenter]
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
            .layout
            .presentation_tabs
            .as_ref()
            .unwrap()
            .content_host()
            .get(ComponentId(999_999))
            .unwrap()
            .frame();
        let others = host.panel_mgr.panels[2].bounds;
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
            for panel in &mut host.panel_mgr.panels {
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
            let others = host.panel_mgr.panels[2].bounds;
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
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(whole, 48.);
        host.apply_presentation_layout(layout).unwrap();
        host.sync_container_toolbars(true, whole, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panel_mgr.panels {
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
        host.panel_mgr
            .panels
            .iter()
            .flat_map(|panel| panel.mounted_toolbar.host.semantics())
            .chain(
                host.layout
                    .presentation_tabs
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
        if host
            .layout
            .presentation_tabs
            .as_ref()
            .is_some_and(|header| {
                header.semantics().iter().any(|node| {
                    node.parts
                        .iter()
                        .any(|(_, part)| part.semantics.name == name)
                })
            })
        {
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
        host.panel_mgr.panels[1].toolbar = vec![ToolbarItem {
            id: 1,
            title: "Switch".into(),
            enabled: true,
            geometry_dependent: false,
        }];
        host.panel_mgr.panels[1].options.focus_switch_command = Some(1);
        t52_refresh(&mut host);
        let whole = Rect::new(0., 0., 1200., 700.);
        let paint_icons = |host: &mut Host, now| {
            host.text.begin_frame(now, false);
            // Real production declaration projection with icon capability; use
            // existing prepared-image services, no GPU renderer or alternate widget.
            let mut declarations = host.presentation_toolbar_declarations_with_icons(true);
            let regions: Vec<_> = (0..host.panel_mgr.panels.len())
                .map(|index| host.container_toolbar_regions(index, 1, whole, &test_band()))
                .collect();
            for (index, panel) in host.panel_mgr.panels.iter_mut().enumerate() {
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
                &mut host.panel_mgr.panels[1].mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                palette,
            )
            .unwrap();
            let mut toolbar = Scene::new();
            for panel in &mut host.panel_mgr.panels {
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
            host.panel_mgr.panels[2].geometry.close.is_none(),
            "Close migration leaves no internal close chrome"
        );
        assert_eq!(host.panel_mgr.panels[2].geometry.content.min_y(), 48.);
        assert!(
            !host
                .panel_mgr
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .any(|node| node.node.semantics.name == "Close focus")
        );
        t52_click(&mut host, "Close other");
        assert!(
            host.layout
                .presentation
                .as_ref()
                .unwrap()
                .others_ids()
                .is_empty()
        );
    }
    #[test]
    fn t52_expand_and_collapse_preserve_original_focus_and_split_allocations() {
        let mut host = window_chrome_fixture();
        t52_refresh(&mut host);
        let original: Vec<_> = host
            .panel_mgr
            .panels
            .iter()
            .map(|panel| (panel.instance, panel.bounds))
            .collect();
        let mut original_focus = 0;
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| original_focus = h as *mut ContentHost as usize);
        t52_click(&mut host, "Expand Others");
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().focus_id(),
            "focus",
            "expanding does not promote or replace Focus"
        );
        assert_eq!(
            host.panel_mgr.panels[0].bounds, original[0].1,
            "Nav allocation is unchanged"
        );
        assert_eq!(
            host.panel_mgr.panels[1].bounds.size.width, 0.,
            "Focus body is hidden"
        );
        assert_eq!(
            host.panel_mgr.panels[2].bounds.min_x(),
            original[1].1.min_x()
        );
        assert_eq!(
            host.panel_mgr.panels[2].bounds.max_x(),
            original[2].1.max_x()
        );
        let mut retained_focus = 0;
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |h| retained_focus = h as *mut ContentHost as usize);
        assert_eq!(
            retained_focus, original_focus,
            "original hidden Focus ContentHost survives"
        );
        t52_click(&mut host, "Collapse Others");
        for (panel, before) in host.panel_mgr.panels.iter().zip(original) {
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
        let header = host.layout.presentation_tabs.as_ref().unwrap();
        let group = header
            .semantics()
            .into_iter()
            .find(|node| node.node.semantics.role == ContentRole::TabList)
            .unwrap();
        assert_eq!(group.node.semantics.role, ContentRole::TabList);
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().selected_tab(),
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
                .panel_mgr
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.host.semantics())
                .find(|node| node.node.semantics.name == name)
                .expect("original title")
                .node
                .frame;
            let column = host.panel_mgr.panels[index].bounds;
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
        let column = host.panel_mgr.panels[2].bounds;
        let group = host
            .layout
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
            host.panel_mgr.panels[2].options.toolbar_host =
                crate::ContainerToolbarHost::Container { position };
            host.panel_mgr.panels[2].toolbar = vec![ToolbarItem {
                id: 7,
                title: "Business".into(),
                enabled: true,
                geometry_dependent: false,
            }];
            refresh_window_chrome(&mut host);
            let bar = host.panel_mgr.panels[2].geometry.toolbar.unwrap();
            let frame = host.panel_mgr.panels[2]
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
            host.panel_mgr.panels[2].options.toolbar_host = host_option;
            t52_refresh(&mut host);
            let panel = &host.panel_mgr.panels[2];
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
        host.panel_mgr.panels[2].options.toolbar_host = crate::ContainerToolbarHost::Container {
            position: Position::Top,
        };
        host.panel_mgr.panels[2].toolbar = vec![ToolbarItem {
            id: 1,
            title: "Business".into(),
            enabled: true,
            geometry_dependent: false,
        }];
        t52_refresh(&mut host);
        let geometry = host.panel_mgr.panels[2].geometry;
        let bar = geometry.toolbar.unwrap();
        assert!(geometry.close.is_none());
        assert!(geometry.content.min_y() >= bar.max_y());
        assert!(t52_button(&host, "Close other").max_y() <= bar.min_y());
    }
    #[test]
    fn partition_chrome_repeated_exchange_uses_real_original_press_paint_release() {
        let mut host = window_chrome_fixture();
        for index in [1, 2] {
            host.panel_mgr.panels[index].toolbar = vec![ToolbarItem {
                id: 1,
                title: "Switch".into(),
                enabled: true,
                geometry_dependent: false,
            }];
            host.panel_mgr.panels[index].options.focus_switch_command = Some(1);
        }
        for expected in ["other", "focus", "other", "focus"] {
            refresh_window_chrome(&mut host);
            let point = host
                .panel_mgr
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
            assert!(host.panel_mgr.panels.iter().any(|panel| {
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
            assert_eq!(
                host.layout.presentation.as_ref().unwrap().focus_id(),
                expected
            );
            refresh_window_chrome(&mut host);
            assert!(host.panel_mgr.panels.iter().all(|panel| {
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
            host.panel_mgr
                .panels
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
                        .panel_mgr
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
                .panel_mgr
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
                host.panel_mgr.panels.iter().any(|panel| panel
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
            let state = host.layout.presentation.as_ref().unwrap();
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
        let viewport_height = host
            .layout
            .presentation_layout
            .list_viewport
            .unwrap()
            .size
            .height;
        host.panel_mgr.panels[1]
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
        host.layout.focused = 1;
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .scroll_list(36., viewport_height);
        render_layout(&mut host, area);
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| {
                assert_eq!(content.focused(), Some(ComponentId(1)));
                let text = content.focused_text().unwrap();
                assert_eq!(text.marked(), Some(0..2));
                assert_eq!(text.text(), "かな");
                content.event(&ContentEvent::Unmark);
            });
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .scroll_list(400., viewport_height);
        render_layout(&mut host, area);
        assert_eq!(host.panel_mgr.panels[1].bounds, Rect::default());
        host.panel_mgr.panels[1]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| assert_eq!(content.focused(), Some(ComponentId(1))));
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others()
            .unwrap();
        render_layout(&mut host, area);
        host.panel_mgr.panels[1]
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
        for panel in &mut host.panel_mgr.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        let point = host.panel_mgr.panels[1]
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
        assert_eq!(host.toolbar_mgr.toolbar_index, 0);
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
        for panel in &mut host.panel_mgr.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        assert!(
            host.panel_mgr.panels[0].geometry.toolbar.is_none()
                && host.panel_mgr.panels[1].geometry.toolbar.is_none()
        );
        assert_eq!(
            host.panel_mgr
                .panels
                .iter()
                .flat_map(|panel| panel.mounted_toolbar.sources())
                .filter(|source| matches!(source, Source::Panel { index: 2, .. }))
                .count(),
            2,
            "two column destinations retain two groups with the same real original owner"
        );
        for (name, command, column) in [("To A", 10, 0), ("To B", 11, 1)] {
            let frame = host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == name)
                .unwrap()
                .1
                .frame;
            let bounds = host.panel_mgr.panels[column].bounds;
            assert!(frame.min_x() >= bounds.min_x() && frame.max_x() <= bounds.max_x());
            let point = frame.center();
            let target = host.toolbar_target_at(point).unwrap();
            assert!(matches!(
                target,
                ToolbarTarget::Contribution { presenter: 0, .. }
            ));
            assert!(matches!(
                host.panel_mgr.panels[0]
                    .mounted_toolbar
                    .source(Host::toolbar_group(target)),
                Some(Source::Panel { index: 2, .. })
            ));
            host.dispatch_toolbar(target, &ContentEvent::PointerDown(point))
                .unwrap();
            host.sync_container_toolbars_with_icons(true, area, test_band(), 40., true)
                .unwrap();
            for panel in &mut host.panel_mgr.panels {
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
            let point = host.panel_mgr.panels[0]
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
        for panel in &mut host.panel_mgr.panels {
            render_test_tabs(
                &mut panel.mounted_toolbar.host,
                &mut Scene::new(),
                &mut host.text,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        let parts = host.panel_mgr.panels[0]
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
                assert!(
                    host.panel_mgr.panels[0]
                        .geometry
                        .toolbar
                        .unwrap()
                        .contains(point)
                );
            } else {
                assert!(point.y < host.panel_mgr.panels[0].geometry.toolbar.unwrap().min_y());
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
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .set_focus_fraction(0.2)
            .unwrap();
        let area = Rect::new(0., 0., 600., 700.);
        render_layout(&mut host, area);
        // Match the production native row: window-edge controls are on the
        // Toolbar row, while the background Tabs are below that row.
        let layout = host
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .layout_with_toolbar(area, 48.);
        host.apply_presentation_layout(layout).unwrap();

        host.sync_container_toolbars(true, area, test_band(), 40.)
            .unwrap();
        for panel in &mut host.panel_mgr.panels {
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
        let overflow = host.panel_mgr.panels[0]
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
        host.panel_mgr.panels[0]
            .mounted_toolbar
            .host
            .accessibility_action(overflow, AccessibilityAction::Press);
        render_test_tabs(
            &mut host.panel_mgr.panels[0].mounted_toolbar.host,
            &mut Scene::new(),
            &mut host.text,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.open_toolbar().is_some());
        let point = host
            .layout
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
        let selected = host
            .layout
            .presentation
            .as_ref()
            .unwrap()
            .selected_tab()
            .cloned();
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
            host.layout
                .presentation
                .as_ref()
                .unwrap()
                .selected_tab()
                .cloned(),
            selected
        );
        assert!(host.open_toolbar().is_some());
    }

    #[test]
    fn others_containers_host_editable_markdown_and_readonly_diff_views() {
        use tekes_ui_controls::{
            ExternalSelection, ExternalUndo, ExternalValueResult, TextEditCommand,
            composition::{
                MarkdownEditorContent, RichBlockKind, RichContent, RichDocument, TextViewport,
            },
            extended::DiffView,
        };
        use tekes_ui_macos_bridge::content_accessibility::ContentAccessibilitySnapshot;

        let focus_panel = LocalContainer::new(
            "focus",
            Content::new(ComponentId(5000), TextContent::new("Agent Focus")),
        )
        .unwrap()
        .title("Focus");

        let md_editor = MarkdownEditorContent::new(
            "agent-notes",
            1,
            "# Notes\n\n- Check `diff`\n\n[Spec](spec.md)",
        )
        .unwrap()
        .with_viewport(TextViewport::Fixed(240.));
        let md_id = ComponentId(5001);
        let md_panel = LocalContainer::new("agent-markdown", Content::new(md_id, md_editor))
            .unwrap()
            .title("Notes");

        let diff = DiffView::between("let x = 1;\n", "let x = 2;\n");
        let diff_doc = RichDocument::from_diff(&diff);
        let diff_id = ComponentId(5002);
        let diff_viewer = RichContent::new("agent-diff-doc", 1, diff_doc).unwrap();
        let diff_panel = LocalContainer::new("agent-diff", Content::new(diff_id, diff_viewer))
            .unwrap()
            .title("Diff");

        let mut host = Host::from_containers(
            vec![focus_panel.into(), md_panel.into(), diff_panel.into()],
            false,
            None,
            Some(
                PresentationOptions::new("focus").others(OthersOptions::new(vec![
                    "agent-markdown".into(),
                    "agent-diff".into(),
                ])),
            ),
        );
        assert!(host.mount_columns());
        let area = Rect::new(0., 0., 1100., 680.);
        render_layout(&mut host, area);

        let md_panel_idx = host
            .panel_mgr
            .panels
            .iter()
            .position(|p| p.module_id == "agent-markdown")
            .unwrap();
        let diff_panel_idx = host
            .panel_mgr
            .panels
            .iter()
            .position(|p| p.module_id == "agent-diff")
            .unwrap();

        // Active `agent-markdown` others panel exposes valid accessibility & unfocused collapsed text.
        host.panel_mgr.panels[md_panel_idx]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |ch| {
                let ax = ContentAccessibilitySnapshot::from_host(ch).unwrap();
                assert!(ax.is_valid());
                assert!(ch.focus(Some(md_id), true));

                let ed = ch
                    .get(md_id)
                    .unwrap()
                    .component::<MarkdownEditorContent>()
                    .unwrap();
                assert_eq!(ed.display_text(), "# Notes\n\n• Check diff\n\nSpec");

                ch.event(&ContentEvent::Commit {
                    text: "Updated ".into(),
                    replacement: None,
                });
                let ed = ch
                    .get(md_id)
                    .unwrap()
                    .component::<MarkdownEditorContent>()
                    .unwrap();
                assert_eq!(
                    ed.source(),
                    "Updated # Notes\n\n- Check `diff`\n\n[Spec](spec.md)"
                );

                ch.event(&ContentEvent::TextCommand(TextEditCommand::Undo));
                let ed = ch
                    .get(md_id)
                    .unwrap()
                    .component::<MarkdownEditorContent>()
                    .unwrap();
                assert_eq!(ed.source(), "# Notes\n\n- Check `diff`\n\n[Spec](spec.md)");

                let v = ed.version();
                let res = ch
                    .get_mut(md_id)
                    .unwrap()
                    .component_mut::<MarkdownEditorContent>()
                    .unwrap()
                    .apply_external_source(
                        &v,
                        "# Notes\n\n- Check `diff`\n\n- Streamed item",
                        2,
                        ExternalSelection::Preserve,
                        ExternalUndo::Preserve,
                    )
                    .unwrap();
                assert!(matches!(
                    res,
                    ExternalValueResult::Applied { changed: true, .. }
                ));
            });

        // Re-render `agent-markdown` after streaming update, then switch tab to `agent-diff`.
        render_layout(&mut host, area);
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(OthersTab::Container("agent-diff".into()))
            .unwrap();
        render_layout(&mut host, area);

        host.panel_mgr.panels[diff_panel_idx]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |ch| {
                let ax = ContentAccessibilitySnapshot::from_host(ch).unwrap();
                assert!(ax.is_valid());
                assert!(ch.focus(Some(diff_id), true));
                let before_text = ch.semantic_text_input_snapshot(diff_id).unwrap().text;
                // Strictly read-only diff ignores Commit edits, while SelectAll selects lines cleanly.
                ch.event(&ContentEvent::Commit {
                    text: "MUTATE".into(),
                    replacement: None,
                });
                ch.event(&ContentEvent::TextCommand(TextEditCommand::SelectAll));
                let snap = ch.semantic_text_input_snapshot(diff_id).unwrap();
                assert_eq!(snap.text, before_text);
                assert!(snap.read_only);
                assert_eq!(snap.selection, 0..before_text.encode_utf16().count());
                let diff_rc = ch.get(diff_id).unwrap().component::<RichContent>().unwrap();
                assert!(diff_rc.document().blocks().iter().any(|b| matches!(
                    b.kind(),
                    RichBlockKind::Diff {
                        kind: tekes_ui_controls::extended::DiffKind::Removed
                    }
                )));
                assert!(diff_rc.document().blocks().iter().any(|b| matches!(
                    b.kind(),
                    RichBlockKind::Diff {
                        kind: tekes_ui_controls::extended::DiffKind::Added
                    }
                )));
                assert!(snap.text.contains("let x = 1;") && snap.text.contains("let x = 2;"));
            });
    }

    mod local_window_toolbar_tests {
        use super::*;
        use crate::{
            Container, LocalContainer, LocalToolbarAction, LocalToolbarButton, LocalToolbarItem,
            LocalToolbarMenu, OthersOptions, PresentationOptions, columns::ContainerContent,
        };
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        use tekes_ui_controls::composition::{
            ContentHost, ContentRole, MenuContent, MenuEntry, TextFieldContent,
        };
        use tekes_ui_platform_macos::{AccessibilityAction, AccessibilityKey, AccessibilityNode};

        #[derive(Clone, Debug)]
        struct SessionVm {
            harness: String,
            harness_locked: bool,
            approval: String,
            model: String,
            effort: String,
            pending_model_from_render: Option<String>,
            button_enabled: bool,
            actions: Vec<LocalToolbarAction>,
        }

        impl Default for SessionVm {
            fn default() -> Self {
                Self {
                    harness: "codex".into(),
                    harness_locked: false,
                    approval: "workspace".into(),
                    model: "gpt-5".into(),
                    effort: "medium".into(),
                    pending_model_from_render: None,
                    button_enabled: true,
                    actions: Vec::new(),
                }
            }
        }

        struct SessionContent {
            host: ContentHost,
            vm: Rc<RefCell<SessionVm>>,
        }

        impl SessionContent {
            fn new(vm: Rc<RefCell<SessionVm>>) -> Self {
                let field = TextFieldContent::new("Prompt", "draft prompt");
                let mut host = ContentHost::new(Content::new(ComponentId(1), field)).unwrap();
                let _ = host.focus(Some(ComponentId(1)), true);
                Self { host, vm }
            }

            fn build_items(vm: &SessionVm) -> Vec<LocalToolbarItem> {
                let harness_label = match vm.harness.as_str() {
                    "claude" => "Claude",
                    _ => "Codex",
                };
                let harness_entries = vec![
                    MenuEntry::Item {
                        value: "codex".into(),
                        label: "Codex".into(),
                        checked: Some(vm.harness == "codex"),
                        disabled: vm.harness_locked,
                    },
                    MenuEntry::Item {
                        value: "claude".into(),
                        label: "Claude".into(),
                        checked: Some(vm.harness == "claude"),
                        disabled: vm.harness_locked,
                    },
                ];
                let approval_label = match vm.approval.as_str() {
                    "read-only" => "Read-only",
                    "full" => "Full Auto",
                    _ => "Workspace",
                };
                let approval_entries = vec![
                    MenuEntry::Item {
                        value: "read-only".into(),
                        label: "Read-only".into(),
                        checked: Some(vm.approval == "read-only"),
                        disabled: false,
                    },
                    MenuEntry::Item {
                        value: "workspace".into(),
                        label: "Workspace".into(),
                        checked: Some(vm.approval == "workspace"),
                        disabled: false,
                    },
                    MenuEntry::Separator,
                    MenuEntry::Submenu {
                        value: "elevated".into(),
                        label: "Elevated".into(),
                        disabled: false,
                        entries: vec![MenuEntry::Item {
                            value: "full".into(),
                            label: "Full Auto".into(),
                            checked: Some(vm.approval == "full"),
                            disabled: false,
                        }],
                    },
                ];
                let model_label = match vm.model.as_str() {
                    "o3" => "o3",
                    "sonnet" => "Sonnet",
                    _ => "GPT-5",
                };
                let effort_label = match vm.effort.as_str() {
                    "low" => "Low",
                    "high" => "High",
                    _ => "Medium",
                };
                vec![
                    LocalToolbarItem::Menu(
                        LocalToolbarMenu::new(
                            1,
                            format!("Harness: {harness_label}"),
                            harness_entries,
                        )
                        .tooltip("Harness")
                        .dropdown_indicator(!vm.harness_locked),
                    ),
                    LocalToolbarItem::Menu(
                        LocalToolbarMenu::new(
                            2,
                            format!("Approval: {approval_label}"),
                            approval_entries,
                        )
                        .tooltip("Approval"),
                    ),
                    LocalToolbarItem::Menu(
                        LocalToolbarMenu::new(
                            3,
                            format!("Model: {model_label}"),
                            vec![
                                MenuEntry::Item {
                                    value: "gpt-5".into(),
                                    label: "GPT-5".into(),
                                    checked: Some(vm.model == "gpt-5"),
                                    disabled: false,
                                },
                                MenuEntry::Item {
                                    value: "sonnet".into(),
                                    label: "Sonnet".into(),
                                    checked: Some(vm.model == "sonnet"),
                                    disabled: false,
                                },
                                MenuEntry::Item {
                                    value: "o3".into(),
                                    label: "o3".into(),
                                    checked: Some(vm.model == "o3"),
                                    disabled: false,
                                },
                            ],
                        )
                        .tooltip("Model"),
                    ),
                    LocalToolbarItem::Menu(
                        LocalToolbarMenu::new(
                            4,
                            format!("Effort: {effort_label}"),
                            vec![
                                MenuEntry::Item {
                                    value: "low".into(),
                                    label: "Low".into(),
                                    checked: Some(vm.effort == "low"),
                                    disabled: false,
                                },
                                MenuEntry::Item {
                                    value: "medium".into(),
                                    label: "Medium".into(),
                                    checked: Some(vm.effort == "medium"),
                                    disabled: false,
                                },
                                MenuEntry::Item {
                                    value: "high".into(),
                                    label: "High".into(),
                                    checked: Some(vm.effort == "high"),
                                    disabled: false,
                                },
                            ],
                        )
                        .tooltip("Effort")
                        .selected(vm.effort == "high"),
                    ),
                    LocalToolbarItem::Button(
                        LocalToolbarButton::new(5, "Run")
                            .tooltip("Run session")
                            .enabled(vm.button_enabled)
                            .selected(vm.effort == "high"),
                    ),
                ]
            }
        }

        impl ContainerContent for SessionContent {
            fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
                visit(&mut self.host);
            }

            fn prepare_for_render(&mut self) {
                let mut vm = self.vm.borrow_mut();
                if let Some(next_model) = vm.pending_model_from_render.take() {
                    vm.model = next_model;
                }
            }

            fn toolbar_items(&mut self) -> Option<Vec<LocalToolbarItem>> {
                Some(Self::build_items(&self.vm.borrow()))
            }

            fn on_toolbar_action(&mut self, action: &LocalToolbarAction) -> Result<bool> {
                let mut vm = self.vm.borrow_mut();
                vm.actions.push(action.clone());
                match (action.id, action.menu_value()) {
                    (1, Some(v)) if !vm.harness_locked => vm.harness = v.to_owned(),
                    (2, Some(v)) => vm.approval = v.to_owned(),
                    (3, Some(v)) => vm.model = v.to_owned(),
                    (4, Some(v)) => vm.effort = v.to_owned(),
                    _ => {}
                }
                Ok(true)
            }
        }

        fn render_presentation_cycle(host: &mut Host, whole: Rect, theme: Theme) {
            host.theme = theme;
            let band = test_band();
            let boundary = band.y + band.height;
            host.layout.container_window_boundary = Some(boundary);
            let frames = host
                .presentation_frames_with_safe_area(whole, boundary, band, 0., 0.)
                .unwrap();
            let palette = host.palette();
            for (index, frame) in frames.into_iter().enumerate() {
                if frame.size.width > 0. {
                    let clip = host.panel_mgr.panels[index].clip;
                    let content = host.panel_mgr.panels[index].content_frame();
                    if let Some(local) = &mut host.panel_mgr.panels[index].local {
                        local
                            .render(&mut Scene::new(), &mut host.text, content, clip, palette)
                            .unwrap();
                    }
                }
            }
            host.sync_container_toolbars_with_icons(true, whole, test_band(), 0., true)
                .unwrap();
            for panel in &mut host.panel_mgr.panels {
                render_test_tabs_layers(
                    &mut panel.mounted_toolbar.host,
                    &mut Scene::new(),
                    &mut Scene::new(),
                    &mut host.text,
                    palette,
                )
                .unwrap();
            }
            host.sync_presentation_switcher(whole).unwrap();
            if let Some(tabs) = &mut host.layout.presentation_tabs {
                render_test_tabs_layers(
                    tabs,
                    &mut Scene::new(),
                    &mut Scene::new(),
                    &mut host.text,
                    palette,
                )
                .unwrap();
            }
        }

        fn focus_index(host: &Host) -> usize {
            let focus_id = host.layout.presentation.as_ref().unwrap().focus_id();
            host.panel_mgr
                .panels
                .iter()
                .position(|p| p.module_id == focus_id)
                .unwrap()
        }

        fn find_trigger_frame_and_id(
            host: &Host,
            prefix: &str,
        ) -> Option<(ComponentId, Rect, bool)> {
            let focus = focus_index(host);
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .find_map(|n| {
                    (n.node.semantics.role == ContentRole::MenuButton
                        && n.node.semantics.name.starts_with(prefix))
                    .then_some((n.node.id, n.node.frame, n.node.semantics.disabled))
                })
        }

        fn find_ax_part(
            host: &mut Host,
            label_prefix: &str,
        ) -> Option<(AccessibilityKey, AccessibilityNode)> {
            host.toolbar_accessibility_nodes(true)
                .into_iter()
                .find(|node| node.label.starts_with(label_prefix))
                .map(|node| (node.key, node))
        }

        fn rects_overlap(a: Rect, b: Rect) -> bool {
            a.size.width > 0.
                && a.size.height > 0.
                && b.size.width > 0.
                && b.size.height > 0.
                && a.min_x() + 0.5 < b.max_x()
                && a.max_x() > b.min_x() + 0.5
                && a.min_y() + 0.5 < b.max_y()
                && a.max_y() > b.min_y() + 0.5
        }

        #[test]
        fn container_content_dynamic_refresh_and_legacy_command_compatibility() {
            let vm = Rc::new(RefCell::new(SessionVm::default()));
            let legacy_calls = Rc::new(Cell::new(0u32));
            let legacy_ref = legacy_calls.clone();
            let dyn_provider_title = Rc::new(RefCell::new(String::from("Provider: v1")));
            let dyn_read = dyn_provider_title.clone();
            let dyn_clicks = Rc::new(Cell::new(0u32));
            let dyn_clicks_ref = dyn_clicks.clone();

            let members: Vec<Container> = vec![
                LocalContainer::from_source("session", SessionContent::new(vm.clone()))
                    .title("Session")
                    .into(),
                LocalContainer::new(
                    "legacy",
                    Content::new(ComponentId(1), TextContent::new("Legacy")),
                )
                .unwrap()
                .toolbar(vec![ToolbarItem {
                    id: 77,
                    title: "Legacy Ping".into(),
                    enabled: true,
                    geometry_dependent: false,
                }])
                .on_command(move |_, id| {
                    if id == 77 {
                        legacy_ref.set(legacy_ref.get() + 1);
                    }
                    Ok(())
                })
                .into(),
                LocalContainer::new("dyn", Content::new(ComponentId(1), TextContent::new("Dyn")))
                    .unwrap()
                    .dynamic_toolbar(move |_| {
                        vec![LocalToolbarItem::button(88, dyn_read.borrow().clone())]
                    })
                    .on_toolbar_action(move |_, action| {
                        if action.id == 88 {
                            dyn_clicks_ref.set(dyn_clicks_ref.get() + 1);
                        }
                        Ok(())
                    })
                    .into(),
            ];
            let options = PresentationOptions::new("session")
                .others(OthersOptions::new(vec!["legacy".into(), "dyn".into()]));
            let mut host = Host::from_containers(members, false, None, Some(options));
            assert!(host.mount_columns());

            let whole = Rect::new(0., 0., 1280., 720.);
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);

            // Capture original mount identities of Session's Model menu & Run button.
            let (model_id, _, _) = find_trigger_frame_and_id(&host, "Model: GPT-5").unwrap();
            let mount_before = host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .content_host()
                .get(model_id)
                .unwrap()
                .mount_identity();
            let (run_key, run_node) = find_ax_part(&mut host, "Run").unwrap();
            assert!(run_node.enabled);
            assert!(!run_node.selected);

            // Mutate VM inside prepare_for_render path + disable Run + set Effort=high.
            {
                let mut state = vm.borrow_mut();
                state.pending_model_from_render = Some("sonnet".into());
                state.effort = "high".into();
                state.button_enabled = false;
            }
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);

            let (updated_model_id, _, _) =
                find_trigger_frame_and_id(&host, "Model: Sonnet").expect("updated model trigger");
            assert_eq!(updated_model_id, model_id);
            let mount_after = host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .content_host()
                .get(updated_model_id)
                .unwrap()
                .mount_identity();
            assert_eq!(
                mount_after, mount_before,
                "dynamic update must preserve MenuContent mount identity"
            );
            let effort_menu = host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .content_host()
                .get(find_trigger_frame_and_id(&host, "Effort: High").unwrap().0)
                .unwrap()
                .component::<MenuContent>()
                .unwrap();
            assert!(effort_menu.trigger().is_selected());
            let (_, updated_run) = find_ax_part(&mut host, "Run").unwrap();
            assert!(!updated_run.enabled);
            assert!(updated_run.selected);
            let _ = run_key;

            // Backward compatibility: switch Others tab to legacy container and trigger command 77.
            host.layout
                .presentation
                .as_mut()
                .unwrap()
                .select_tab(OthersTab::Container("legacy".into()))
                .unwrap();
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let (legacy_key, _) = find_ax_part(&mut host, "Legacy Ping").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, legacy_key, AccessibilityAction::Press)
                    .unwrap()
            );
            assert_eq!(legacy_calls.get(), 1);

            // Switch to dynamic_toolbar closure container, mutate label, and invoke button 88.
            host.layout
                .presentation
                .as_mut()
                .unwrap()
                .select_tab(OthersTab::Container("dyn".into()))
                .unwrap();
            *dyn_provider_title.borrow_mut() = "Provider: v2".into();
            render_presentation_cycle(&mut host, whole, Theme::DARK);
            let (dyn_key, _) = find_ax_part(&mut host, "Provider: v2").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, dyn_key, AccessibilityAction::Press)
                    .unwrap()
            );
            assert_eq!(dyn_clicks.get(), 1);
        }

        #[test]
        fn session_menus_deliver_actions_to_owner_enforce_locked_state_and_dismiss_on_blur() {
            let vm = Rc::new(RefCell::new(SessionVm::default()));
            let members: Vec<Container> = vec![
                LocalContainer::from_source("session", SessionContent::new(vm.clone()))
                    .title("Session")
                    .into(),
                LocalContainer::new(
                    "other",
                    Content::new(ComponentId(1), TextContent::new("Other")),
                )
                .unwrap()
                .title("Other")
                .into(),
            ];
            let options = PresentationOptions::new("session")
                .others(OthersOptions::new(vec!["other".into()]));
            let mut host = Host::from_containers(members, false, None, Some(options));
            assert!(host.mount_columns());

            let whole = Rect::new(0., 0., 1280., 760.);
            for theme in Theme::ALL {
                render_presentation_cycle(&mut host, whole, theme);
            }

            // 1. Open Model menu via pointer click and select "o3".
            let (model_cid, model_frame, _) =
                find_trigger_frame_and_id(&host, "Model: GPT-5").unwrap();
            let focus = focus_index(&host);
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(model_frame.center()));
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(model_frame.center()));
            render_presentation_cycle(&mut host, whole, Theme::DARK);
            assert!(host.panel_mgr.panels[focus].mounted_toolbar.modal());

            let (o3_key, o3_node) = find_ax_part(&mut host, "o3").unwrap();
            assert!(o3_node.enabled);
            assert!(
                host.toolbar_accessibility_action(true, o3_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert!(!host.panel_mgr.panels[focus].mounted_toolbar.modal());
            assert_eq!(vm.borrow().model, "o3");
            assert_eq!(vm.borrow().actions.last().unwrap().id, 3);
            assert_eq!(vm.borrow().actions.last().unwrap().menu_value(), Some("o3"));
            assert!(find_trigger_frame_and_id(&host, "Model: o3").is_some());
            let _ = model_cid;

            // 2. Open Approval submenu ("Elevated" -> "Full Auto") and verify full value path.
            let (_, approval_frame, _) =
                find_trigger_frame_and_id(&host, "Approval: Workspace").unwrap();
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(approval_frame.center()));
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(approval_frame.center()));
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let (elevated_key, _) = find_ax_part(&mut host, "Elevated").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, elevated_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let (full_key, _) = find_ax_part(&mut host, "Full Auto").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, full_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert_eq!(vm.borrow().approval, "full");
            assert_eq!(
                vm.borrow().actions.last().unwrap().menu_value_path(),
                Some(["elevated".into(), "full".into()].as_slice())
            );

            // 3. Lock Harness after session start: trigger remains clear/enabled (no chevron),
            //    inner items are disabled with checked=Some(true), and clicks reject safely.
            vm.borrow_mut().harness_locked = true;
            render_presentation_cycle(&mut host, whole, Theme::DARK);
            let (harness_cid, harness_frame, harness_disabled) =
                find_trigger_frame_and_id(&host, "Harness: Codex").unwrap();
            assert!(
                !harness_disabled,
                "locked identity trigger should remain readable without disabled dimming"
            );
            let action_count_before = vm.borrow().actions.len();
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(harness_frame.center()));
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(harness_frame.center()));
            render_presentation_cycle(&mut host, whole, Theme::DARK);
            assert!(host.panel_mgr.panels[focus].mounted_toolbar.modal());

            let (codex_key, codex_ax) = find_ax_part(&mut host, "Codex").unwrap();
            assert!(!codex_ax.enabled);
            assert!(codex_ax.selected);
            let claude_part = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .find(|n| n.node.id == harness_cid)
                .unwrap()
                .parts
                .into_iter()
                .find(|(_, p)| p.semantics.name == "Claude")
                .unwrap()
                .1;
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(claude_part.frame.center()));
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(claude_part.frame.center()));
            host.drain_toolbar_commands().unwrap();
            assert!(
                !host
                    .toolbar_accessibility_action(true, codex_key, AccessibilityAction::Press)
                    .unwrap()
            );
            assert_eq!(vm.borrow().actions.len(), action_count_before);

            // 4. Outside click dismisses popup; re-open & deactivate window closes all panels.
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(Point::new(20., 300.)));
            assert!(!host.panel_mgr.panels[focus].mounted_toolbar.modal());

            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(harness_frame.center()));
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(harness_frame.center()));
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert!(host.panel_mgr.panels[focus].mounted_toolbar.modal());
            for panel in &mut host.panel_mgr.panels {
                panel.mounted_toolbar.host.set_window_active(false);
            }
            assert!(!host.panel_mgr.panels[focus].mounted_toolbar.modal());
        }

        #[test]
        fn focus_others_swap_navbar_titleless_and_narrow_overflow_preserve_tabs_and_body_state() {
            let vm = Rc::new(RefCell::new(SessionVm::default()));
            let members: Vec<Container> = vec![
                // NavBar without .title(...) should emit no redundant Source::Title.
                LocalContainer::new(
                    "nav",
                    Content::new(ComponentId(1), TextContent::new("Workspace Brand")),
                )
                .unwrap()
                .into(),
                LocalContainer::from_source("session", SessionContent::new(vm.clone()))
                    .title("Session")
                    .into(),
                LocalContainer::new(
                    "inspector",
                    Content::new(ComponentId(1), TextContent::new("Inspector")),
                )
                .unwrap()
                .title("Inspector")
                .into(),
            ];
            let options = PresentationOptions::new("session")
                .nav_bar("nav")
                .others(OthersOptions::new(vec!["inspector".into()]))
                .workspace_switch(true);
            let mut host = Host::from_containers(members, false, None, Some(options));
            assert!(host.mount_columns());

            let wide = Rect::new(0., 0., 1280., 760.);
            render_presentation_cycle(&mut host, wide, Theme::LIGHT);

            // Titleless NavBar emits zero Source::Title declarations.
            let decls = host.presentation_toolbar_declarations_with_icons(true);
            assert!(
                decls[focus_index(&host)].iter().any(|d| matches!(
                    (&d.source, d.region),
                    (Source::Title, r) if r == WINDOW_BUSINESS
                )),
                "Focus container with .title(\"Session\") keeps its title"
            );
            assert!(
                !decls[focus_index(&host)].iter().any(|d| matches!(
                    (&d.source, d.region),
                    (Source::Title, r) if r == WINDOW_NAV_BUSINESS
                )),
                "Titleless NavBar must not emit redundant window title"
            );

            // Verify body TextFieldContent mount identity, active focus & text, then swap Focus -> Others.
            let mut body_mount_before = None;
            host.panel_mgr.panels[1]
                .local
                .as_mut()
                .unwrap()
                .with_host(&mut |ch| {
                    body_mount_before = Some(ch.get(ComponentId(1)).unwrap().mount_identity());
                    assert_eq!(ch.focused(), Some(ComponentId(1)));
                    assert_eq!(ch.focused_text().unwrap().text(), "draft prompt");
                });

            assert!(host.switch_container_focus(1).unwrap());
            render_presentation_cycle(&mut host, wide, Theme::DARK);

            // Body ContentHost mount identity & active input state survived Focus <-> Others swap.
            host.panel_mgr.panels[1]
                .local
                .as_mut()
                .unwrap()
                .with_host(&mut |ch| {
                    assert_eq!(
                        Some(ch.get(ComponentId(1)).unwrap().mount_identity()),
                        body_mount_before
                    );
                    assert_eq!(ch.focused(), Some(ComponentId(1)));
                    assert_eq!(ch.focused_text().unwrap().text(), "draft prompt");
                });

            // Now in Others at narrow window width: tabs remain unobstructed and trailing items overflow.
            let narrow_others = Rect::new(0., 0., 760., 760.);
            render_presentation_cycle(&mut host, narrow_others, Theme::LIGHT);

            let tabs_frame = host
                .presentation_switcher_frame()
                .expect("Others tabs frame");
            assert!(
                tabs_frame.size.width >= 40.,
                "Others tabs bar must retain space at narrow width, got {:?}",
                tabs_frame
            );
            let focus = focus_index(&host);
            for occupied in host.panel_mgr.panels[focus]
                .mounted_toolbar
                .occupied_frames()
            {
                assert!(
                    !rects_overlap(occupied, tabs_frame),
                    "Toolbar item {:?} must not overlap Others switcher tabs {:?}",
                    occupied,
                    tabs_frame
                );
            }
            assert!(
                host.panel_mgr.panels[focus].mounted_toolbar.sources().any(
                    |s| matches!(s, Source::PanelOverflow { module, .. } if module == "session")
                ),
                "Narrow Others lane must fold overflowed session menus into Source::PanelOverflow"
            );

            // Activate an overflowed submenu inside "More actions" -> selects Effort: High cleanly.
            let (_, more_frame, _) = find_trigger_frame_and_id(&host, "More").unwrap();
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(more_frame.center()));
            host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(more_frame.center()));
            render_presentation_cycle(&mut host, narrow_others, Theme::LIGHT);

            let (effort_sub_key, _) = find_ax_part(&mut host, "Effort").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, effort_sub_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, narrow_others, Theme::LIGHT);
            let (high_key, _) = find_ax_part(&mut host, "High").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, high_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, narrow_others, Theme::LIGHT);
            assert_eq!(vm.borrow().effort, "high");
            assert_eq!(vm.borrow().actions.last().unwrap().id, 4);
            assert_eq!(
                vm.borrow().actions.last().unwrap().menu_value_path(),
                Some(["high".into()].as_slice())
            );

            // Swap session back to Focus and verify narrow Focus overflow as well.
            assert!(host.switch_container_focus(1).unwrap());
            let narrow_focus = Rect::new(0., 0., 680., 760.);
            render_presentation_cycle(&mut host, narrow_focus, Theme::DARK);
            let focus = focus_index(&host);
            assert!(
                host.panel_mgr.panels[focus].mounted_toolbar.sources().any(
                    |s| matches!(s, Source::PanelOverflow { module, .. } if module == "session")
                ),
                "Narrow Focus lane must overflow trailing session menus"
            );
            let focus_panel_frames: Vec<_> = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .occupied_frames();
            for i in 0..focus_panel_frames.len() {
                for j in (i + 1)..focus_panel_frames.len() {
                    assert!(
                        !rects_overlap(focus_panel_frames[i], focus_panel_frames[j]),
                        "Toolbar items {:?} and {:?} must not overlap",
                        focus_panel_frames[i],
                        focus_panel_frames[j]
                    );
                }
            }
        }

        #[test]
        fn capsule_group_mixed_segments_pointer_keyboard_ax_and_action_parity() {
            use crate::{LocalToolbarGroup, LocalToolbarTone};
            use tekes_ui_controls::ControlKey;
            use tekes_ui_controls::composition::ContentActionSource;
            use tekes_ui_controls::composition::ContentKeyEvent;
            use tekes_ui_controls::composition::ToolbarGroup;
            use tekes_ui_platform_macos::AccessibilityRole;

            let actions: Rc<RefCell<Vec<LocalToolbarAction>>> = Rc::new(RefCell::new(Vec::new()));
            let actions_cb = actions.clone();

            let access_entries = vec![
                MenuEntry::Item {
                    value: "ws".into(),
                    label: "Workspace".into(),
                    checked: Some(true),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "full".into(),
                    label: "Full Access".into(),
                    checked: Some(false),
                    disabled: false,
                },
            ];
            let model_entries = vec![
                MenuEntry::Item {
                    value: "claude-sonnet-4".into(),
                    label: "claude-sonnet-4".into(),
                    checked: Some(true),
                    disabled: false,
                },
                MenuEntry::Item {
                    value: "claude-opus-4".into(),
                    label: "claude-opus-4".into(),
                    checked: Some(false),
                    disabled: false,
                },
            ];

            let focus_items = vec![
                LocalToolbarItem::Group(
                    LocalToolbarGroup::new(
                        300,
                        vec![
                            LocalToolbarItem::Button(
                                LocalToolbarButton::new(301, "")
                                    .icon("tekes-back")
                                    .tooltip("Back"),
                            ),
                            LocalToolbarItem::Button(
                                LocalToolbarButton::new(302, "")
                                    .icon("tekes-forward")
                                    .tooltip("Forward"),
                            ),
                        ],
                    )
                    .divided(false)
                    .label("History"),
                ),
                LocalToolbarItem::Group(
                    LocalToolbarGroup::new(
                        100,
                        vec![
                            LocalToolbarItem::Menu(
                                LocalToolbarMenu::new(101, "", access_entries)
                                    .icon("tekes-folder-edit")
                                    .tooltip("Access")
                                    .dropdown_indicator(false),
                            ),
                            LocalToolbarItem::Menu(
                                LocalToolbarMenu::new(102, "claude-sonnet-4", model_entries)
                                    .tooltip("Model")
                                    .dropdown_indicator(false),
                            ),
                            LocalToolbarItem::Button(
                                LocalToolbarButton::new(103, "High")
                                    .tooltip("Reasoning effort")
                                    .tone(LocalToolbarTone::Secondary),
                            ),
                        ],
                    )
                    .label("Session controls")
                    .tooltip("Session controls"),
                ),
                LocalToolbarItem::Button(LocalToolbarButton::new(201, "Run")),
            ];

            let members: Vec<Container> = vec![
                LocalContainer::new(
                    "focus",
                    Content::new(ComponentId(1), TextContent::new("Main body")),
                )
                .unwrap()
                .title("Focus")
                .toolbar_items(focus_items)
                .on_toolbar_action(move |_, act| {
                    actions_cb.borrow_mut().push(act.clone());
                    Ok(())
                })
                .into(),
                LocalContainer::new(
                    "other",
                    Content::new(ComponentId(1), TextContent::new("Side body")),
                )
                .unwrap()
                .title("Other")
                .into(),
            ];
            let options =
                PresentationOptions::new("focus").others(OthersOptions::new(vec!["other".into()]));
            let mut host = Host::from_containers(members, false, None, Some(options));
            assert!(host.mount_columns());

            let whole = Rect::new(0., 0., 1280., 760.);
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let focus = focus_index(&host);

            let capsule_group_cid = |cmd_title_or_id: &str| -> ComponentId {
                host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .semantics()
                    .into_iter()
                    .find_map(|n| (n.semantics.name == cmd_title_or_id).then_some(n.id))
                    .unwrap()
            };
            let g100_cid = capsule_group_cid("Session controls");
            let seg_ids_and_frames = |host: &Host| -> (Rect, Vec<(ComponentId, Rect)>) {
                let ch = host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host();
                let group = ch
                    .get(g100_cid)
                    .unwrap()
                    .component::<ToolbarGroup>()
                    .unwrap();
                let frames = group
                    .children
                    .iter()
                    .map(|c| (c.id(), c.frame()))
                    .collect::<Vec<_>>();
                (ch.get(g100_cid).unwrap().frame(), frames)
            };
            let (g_frame, segs) = seg_ids_and_frames(&host);
            assert_eq!(segs.len(), 3);
            assert!(segs[0].1.size.width < g_frame.size.width);
            assert!((segs[0].1.max_x() - segs[1].1.min_x()).abs() < 0.01);
            assert!((segs[1].1.max_x() - segs[2].1.min_x()).abs() < 0.01);

            // 1. Pointer activation on each segment + anchor check for menus + action parity for button.
            let click_point = |host: &mut Host, pt: Point| {
                let target = host
                    .toolbar_target_at(pt)
                    .expect("toolbar target at segment");
                host.dispatch_toolbar(target, &ContentEvent::PointerMove(pt))
                    .unwrap();
                host.dispatch_toolbar(target, &ContentEvent::PointerDown(pt))
                    .unwrap();
                host.dispatch_toolbar(target, &ContentEvent::PointerUp(pt))
                    .unwrap();
            };

            // Click icon-only menu 101 -> anchor equals seg[0] (not full group), choose "Full Access".
            click_point(&mut host, segs[0].1.center());
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let m101 = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .content_host()
                .get(segs[0].0)
                .unwrap()
                .component::<MenuContent>()
                .unwrap();
            assert!(m101.is_open());
            assert_eq!(m101.popup_layout().unwrap().anchor, segs[0].1);
            let (full_key, _) = find_ax_part(&mut host, "Full Access").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, full_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert_eq!(actions.borrow().last().unwrap().id, 101);
            assert_eq!(actions.borrow().last().unwrap().menu_value(), Some("full"));

            // Click compact text menu 102 -> anchor equals seg[1], choose "claude-opus-4".
            click_point(&mut host, segs[1].1.center());
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let m102 = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .content_host()
                .get(segs[1].0)
                .unwrap()
                .component::<MenuContent>()
                .unwrap();
            assert!(m102.is_open());
            assert_eq!(m102.popup_layout().unwrap().anchor, segs[1].1);
            let (opus_key, _) = find_ax_part(&mut host, "claude-opus-4").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, opus_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert_eq!(actions.borrow().last().unwrap().id, 102);
            assert_eq!(
                actions.borrow().last().unwrap().menu_value(),
                Some("claude-opus-4")
            );

            // Click grouped button 103 and standalone button 201 -> identical ToolbarCommand payload parity.
            click_point(&mut host, segs[2].1.center());
            let act103 = actions.borrow().last().unwrap().clone();
            assert_eq!(act103.id, 103);
            assert_eq!(act103.source, ContentActionSource::Pointer);
            assert!(matches!(
                act103.payload,
                Some(tekes_ui_controls::composition::ContentActionPayload::ToolbarCommand(103))
            ));

            let run_frame = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .find_map(|n| {
                    if n.node.semantics.name == "Run" {
                        Some(n.node.frame)
                    } else {
                        n.parts
                            .iter()
                            .find_map(|(_, p)| (p.semantics.name == "Run").then_some(p.frame))
                    }
                })
                .unwrap();
            click_point(&mut host, run_frame.center());
            let act201 = actions.borrow().last().unwrap().clone();
            assert_eq!(act201.id, 201);
            assert_eq!(act201.source, ContentActionSource::Pointer);
            assert!(matches!(
                act201.payload,
                Some(tekes_ui_controls::composition::ContentActionPayload::ToolbarCommand(201))
            ));

            // 2. Keyboard navigation: Tab/Left/Right traverses individual segments and activates cleanly.
            let key_ev = |key: ControlKey, down: bool| ContentKeyEvent {
                key: Some(key),
                down,
                repeat: false,
                modifiers: Default::default(),
                native_code: 0,
                characters: String::new(),
                characters_ignoring_modifiers: String::new(),
            };
            host.set_toolbar_focus(None, false);
            // Focus group 100 directly via Tab until segs[0] owns focus, then Right -> segs[1] -> segs[2].
            let mut reached = false;
            for _ in 0..8 {
                assert!(
                    host.toolbar_key_input(key_ev(ControlKey::Tab { reverse: false }, true))
                        .unwrap()
                );
                if host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused()
                    == Some(segs[0].0)
                {
                    reached = true;
                    break;
                }
            }
            assert!(
                reached,
                "Tab must reach first segment of Session controls capsule"
            );
            assert!(
                host.toolbar_key_input(key_ev(ControlKey::Right, true))
                    .unwrap()
            );
            assert_eq!(
                host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused(),
                Some(segs[1].0)
            );
            assert!(
                host.toolbar_key_input(key_ev(ControlKey::Right, true))
                    .unwrap()
            );
            assert_eq!(
                host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused(),
                Some(segs[2].0)
            );
            assert!(
                host.toolbar_key_input(key_ev(ControlKey::Space, true))
                    .unwrap()
            );
            assert!(
                host.toolbar_key_input(key_ev(ControlKey::Space, false))
                    .unwrap()
            );
            let kb103 = actions.borrow().last().unwrap().clone();
            assert_eq!(kb103.id, 103);
            assert_eq!(kb103.source, ContentActionSource::Keyboard);
            assert!(matches!(
                kb103.payload,
                Some(tekes_ui_controls::composition::ContentActionPayload::ToolbarCommand(103))
            ));
            // ArrowLeft returns to middle segment, and Shift+Tab moves cleanly back.
            assert!(
                host.toolbar_key_input(key_ev(ControlKey::Left, true))
                    .unwrap()
            );
            assert_eq!(
                host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused(),
                Some(segs[1].0)
            );
            assert!(
                host.toolbar_key_input(key_ev(ControlKey::Tab { reverse: true }, true))
                    .unwrap()
            );
            assert_ne!(
                host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .focused(),
                Some(segs[1].0)
            );

            // 3. AX Group label & segment roles + Press activation.
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let ax_nodes = host.toolbar_accessibility_nodes(true);
            let group_ax = ax_nodes
                .iter()
                .find(|n| n.label == "Session controls")
                .expect("AX Group node");
            assert_eq!(group_ax.role, AccessibilityRole::Group);
            let access_ax = ax_nodes
                .iter()
                .find(|n| n.label == "Access")
                .expect("Access segment AX");
            let model_ax = ax_nodes
                .iter()
                .find(|n| n.label == "claude-sonnet-4")
                .expect("Model segment AX");
            let high_ax = ax_nodes
                .iter()
                .find(|n| n.label == "High")
                .expect("High segment AX");
            assert_eq!(access_ax.parent, Some(group_ax.key));
            assert_eq!(model_ax.parent, Some(group_ax.key));
            assert_eq!(high_ax.parent, Some(group_ax.key));

            assert!(
                host.toolbar_accessibility_action(true, high_ax.key, AccessibilityAction::Press)
                    .unwrap()
            );
            let ax103 = actions.borrow().last().unwrap().clone();
            assert_eq!(ax103.id, 103);
            assert_eq!(ax103.source, ContentActionSource::Accessibility);

            let access_key = access_ax.key;
            assert!(
                host.toolbar_accessibility_action(true, access_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let m101_ax = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .content_host()
                .get(segs[0].0)
                .unwrap()
                .component::<MenuContent>()
                .unwrap();
            assert!(m101_ax.is_open());
            assert_eq!(m101_ax.popup_layout().unwrap().anchor, segs[0].1);
            let (ws_key, _) = find_ax_part(&mut host, "Workspace").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, ws_key, AccessibilityAction::Press)
                    .unwrap()
            );
            assert_eq!(actions.borrow().last().unwrap().id, 101);
            assert_eq!(
                actions.borrow().last().unwrap().source,
                ContentActionSource::Accessibility
            );
        }

        #[test]
        fn capsule_group_dynamic_updates_and_invalid_declaration_rejection() {
            use crate::{AppWindow, LocalToolbarGroup, LocalToolbarTone};
            use tekes_ui_controls::composition::ToolbarGroup;

            struct DynVm {
                icon: String,
                model: String,
                tone: LocalToolbarTone,
                include_fourth: bool,
                include_second: bool,
                inject_invalid: Option<Vec<LocalToolbarItem>>,
                actions: Vec<LocalToolbarAction>,
            }
            struct DynSource {
                host: ContentHost,
                vm: Rc<RefCell<DynVm>>,
            }
            impl DynSource {
                fn items_for(vm: &DynVm) -> Vec<LocalToolbarItem> {
                    if let Some(bad) = &vm.inject_invalid {
                        return bad.clone();
                    }
                    let mut group_items = vec![LocalToolbarItem::Menu(
                        LocalToolbarMenu::new(
                            101,
                            "",
                            vec![MenuEntry::Item {
                                value: "ws".into(),
                                label: "Workspace".into(),
                                checked: Some(true),
                                disabled: false,
                            }],
                        )
                        .icon(&vm.icon)
                        .tooltip("Access")
                        .dropdown_indicator(false),
                    )];
                    if vm.include_second {
                        group_items.push(LocalToolbarItem::Menu(
                            LocalToolbarMenu::new(
                                102,
                                &vm.model,
                                vec![MenuEntry::Item {
                                    value: "m".into(),
                                    label: vm.model.clone(),
                                    checked: Some(true),
                                    disabled: false,
                                }],
                            )
                            .tooltip("Model")
                            .dropdown_indicator(false)
                            .tone(vm.tone),
                        ));
                    }
                    if vm.include_fourth {
                        group_items.push(LocalToolbarItem::Button(
                            LocalToolbarButton::new(104, "Fast").tone(LocalToolbarTone::Warning),
                        ));
                    }
                    group_items.push(LocalToolbarItem::Button(LocalToolbarButton::new(
                        103, "High",
                    )));
                    vec![
                        LocalToolbarItem::Group(
                            LocalToolbarGroup::new(100, group_items).label("Session controls"),
                        ),
                        LocalToolbarItem::Menu(LocalToolbarMenu::new(
                            202,
                            "Mode",
                            vec![MenuEntry::Item {
                                value: "auto".into(),
                                label: "Auto".into(),
                                checked: Some(true),
                                disabled: false,
                            }],
                        )),
                    ]
                }
            }
            impl ContainerContent for DynSource {
                fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
                    visit(&mut self.host);
                }
                fn toolbar_items(&mut self) -> Option<Vec<LocalToolbarItem>> {
                    Some(Self::items_for(&self.vm.borrow()))
                }
                fn on_toolbar_action(&mut self, action: &LocalToolbarAction) -> Result<bool> {
                    self.vm.borrow_mut().actions.push(action.clone());
                    Ok(true)
                }
            }

            let vm = Rc::new(RefCell::new(DynVm {
                icon: "tekes-folder-edit".into(),
                model: "claude-sonnet-4".into(),
                tone: LocalToolbarTone::Secondary,
                include_fourth: false,
                include_second: true,
                inject_invalid: None,
                actions: Vec::new(),
            }));
            let members: Vec<Container> = vec![
                LocalContainer::from_source(
                    "dyn",
                    DynSource {
                        host: ContentHost::new(Content::new(
                            ComponentId(1),
                            TextContent::new("Dyn body"),
                        ))
                        .unwrap(),
                        vm: vm.clone(),
                    },
                )
                .title("Dyn")
                .into(),
                LocalContainer::new(
                    "other",
                    Content::new(ComponentId(1), TextContent::new("Other")),
                )
                .unwrap()
                .title("Other")
                .into(),
            ];
            let options =
                PresentationOptions::new("dyn").others(OthersOptions::new(vec!["other".into()]));
            let mut host = Host::from_containers(members, false, None, Some(options));
            assert!(host.mount_columns());
            let whole = Rect::new(0., 0., 1280., 720.);
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);

            let seg_ids = |host: &Host| -> Vec<(ComponentId, Rect, u64)> {
                let ch = host.panel_mgr.panels[0].mounted_toolbar.host.content_host();
                let group_cid = ch
                    .semantics()
                    .into_iter()
                    .find_map(|n| (n.semantics.name == "Session controls").then_some(n.id))
                    .unwrap();
                ch.get(group_cid)
                    .unwrap()
                    .component::<ToolbarGroup>()
                    .unwrap()
                    .children
                    .iter()
                    .map(|c| (c.id(), c.frame(), c.mount_identity()))
                    .collect()
            };

            let before = seg_ids(&host);
            assert_eq!(before.len(), 3);
            let (m101_id, f101, mount101) = before[0];
            let (m102_id, _, mount102) = before[1];
            let (m103_id, _, mount103) = before[2];
            let (mode_cid, mode_frame, _) = find_trigger_frame_and_id(&host, "Mode").unwrap();
            let mode_mount = host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .content_host()
                .get(mode_cid)
                .unwrap()
                .mount_identity();

            // Open standalone Mode menu (and keep it open across property + structural updates? Actually open 101 first!).
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(f101.center()));
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(f101.center()));
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert!(
                host.panel_mgr.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(m101_id)
                    .unwrap()
                    .component::<MenuContent>()
                    .unwrap()
                    .is_open()
            );

            // Sibling property update (model + tone) while 101 open -> all mounts preserved, 101 stays open.
            {
                let mut state = vm.borrow_mut();
                state.model = "claude-opus-4".into();
                state.tone = LocalToolbarTone::Error;
            }
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let after_prop = seg_ids(&host);
            assert_eq!(after_prop.len(), 3);
            assert_eq!(after_prop[0].0, m101_id);
            assert_eq!(after_prop[0].2, mount101);
            assert_eq!(after_prop[1].0, m102_id);
            assert_eq!(after_prop[1].2, mount102);
            assert_eq!(after_prop[2].0, m103_id);
            assert_eq!(after_prop[2].2, mount103);
            assert_eq!(
                host.panel_mgr.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(mode_cid)
                    .unwrap()
                    .mount_identity(),
                mode_mount
            );
            assert!(
                host.panel_mgr.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(m101_id)
                    .unwrap()
                    .component::<MenuContent>()
                    .unwrap()
                    .is_open(),
                "updating sibling segment must keep 101 popup open"
            );

            // Updating 101's own icon also updates in place without rebuilding mount identity.
            vm.borrow_mut().icon = "tekes-access-full".into();
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert_eq!(seg_ids(&host)[0].2, mount101);

            // Dismiss 101, open Mode, then mutate 3 -> 4 segments (insert 104) -> all unchanged stay mounted and Mode stays open.
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(Point::new(10., 10.)));
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(Point::new(10., 10.)));
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(mode_frame.center()));
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(mode_frame.center()));
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            assert!(
                host.panel_mgr.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(mode_cid)
                    .unwrap()
                    .component::<MenuContent>()
                    .unwrap()
                    .is_open()
            );

            vm.borrow_mut().include_fourth = true;
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let four = seg_ids(&host);
            assert_eq!(four.len(), 4);
            assert_eq!(four[0].2, mount101);
            assert_eq!(four[1].2, mount102);
            assert_eq!(four[3].2, mount103);
            assert!(
                host.panel_mgr.panels[0]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(mode_cid)
                    .unwrap()
                    .component::<MenuContent>()
                    .unwrap()
                    .is_open(),
                "unrelated Mode menu must stay open across 3 -> 4 segment change"
            );

            // Dismiss Mode, click newly added segment 104 -> action 104 arrives.
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerDown(Point::new(10., 10.)));
            host.panel_mgr.panels[0]
                .mounted_toolbar
                .host
                .event(&ContentEvent::PointerUp(Point::new(10., 10.)));
            let target104 = host.toolbar_target_at(four[2].1.center()).unwrap();
            host.dispatch_toolbar(target104, &ContentEvent::PointerDown(four[2].1.center()))
                .unwrap();
            host.dispatch_toolbar(target104, &ContentEvent::PointerUp(four[2].1.center()))
                .unwrap();
            assert_eq!(vm.borrow().actions.last().unwrap().id, 104);

            // Shrink 4 -> 3 removing 102.
            vm.borrow_mut().include_second = false;
            render_presentation_cycle(&mut host, whole, Theme::LIGHT);
            let three = seg_ids(&host);
            assert_eq!(three.len(), 3);
            assert_eq!(three[0].2, mount101);
            assert_eq!(three[1].2, four[2].2);
            assert_eq!(three[2].2, mount103);

            // Invalid dynamic update rejection (duplicate ID, 0, nested group).
            for bad in [
                vec![LocalToolbarItem::Group(LocalToolbarGroup::new(
                    100,
                    vec![
                        LocalToolbarItem::button(100, "DupGroup"),
                        LocalToolbarItem::button(103, "High"),
                    ],
                ))],
                vec![LocalToolbarItem::Group(LocalToolbarGroup::new(
                    100,
                    vec![LocalToolbarItem::button(0, "Zero")],
                ))],
                vec![LocalToolbarItem::Group(LocalToolbarGroup::new(
                    100,
                    vec![LocalToolbarItem::group(
                        101,
                        vec![LocalToolbarItem::button(102, "Nested")],
                    )],
                ))],
            ] {
                vm.borrow_mut().inject_invalid = Some(bad.clone());
                assert!(host.sync_local_toolbars().is_err());
                // Static validation via AppWindow::from_containers also rejects invalid declarations.
                let bad_container: Container = LocalContainer::new(
                    "bad",
                    Content::new(ComponentId(1), TextContent::new("Bad")),
                )
                .unwrap()
                .toolbar_items(bad)
                .into();
                assert!(
                    AppWindow::from_containers(
                        vec![bad_container],
                        PresentationOptions::new("bad"),
                    )
                    .is_err()
                );
            }
            vm.borrow_mut().inject_invalid = None;
        }

        #[test]
        fn capsule_group_narrow_others_overflow_together_and_split() {
            use crate::{LocalToolbarGroup, LocalToolbarGroupOverflow, LocalToolbarTone};
            use tekes_ui_controls::composition::ToolbarGroup;

            let actions: Rc<RefCell<Vec<LocalToolbarAction>>> = Rc::new(RefCell::new(Vec::new()));
            let overflow_mode = Rc::new(Cell::new(LocalToolbarGroupOverflow::Together));
            let mode_read = overflow_mode.clone();
            let actions_cb = actions.clone();

            let build_other_items = move || {
                vec![LocalToolbarItem::Group(
                    LocalToolbarGroup::new(
                        500,
                        vec![
                            LocalToolbarItem::Menu(
                                LocalToolbarMenu::new(
                                    501,
                                    "",
                                    vec![MenuEntry::Item {
                                        value: "ws".into(),
                                        label: "Workspace".into(),
                                        checked: Some(true),
                                        disabled: false,
                                    }],
                                )
                                .icon("tekes-folder-edit")
                                .tooltip("Access")
                                .dropdown_indicator(false),
                            ),
                            LocalToolbarItem::Menu(
                                LocalToolbarMenu::new(
                                    502,
                                    "claude-sonnet-4",
                                    vec![
                                        MenuEntry::Item {
                                            value: "claude-sonnet-4".into(),
                                            label: "claude-sonnet-4".into(),
                                            checked: Some(true),
                                            disabled: false,
                                        },
                                        MenuEntry::Item {
                                            value: "claude-opus-4".into(),
                                            label: "claude-opus-4".into(),
                                            checked: Some(false),
                                            disabled: false,
                                        },
                                    ],
                                )
                                .tooltip("Model")
                                .dropdown_indicator(false)
                                .tone(LocalToolbarTone::Secondary),
                            ),
                            LocalToolbarItem::Button(
                                LocalToolbarButton::new(503, "High").tooltip("Reasoning effort"),
                            ),
                        ],
                    )
                    .label("Model controls")
                    .overflow(mode_read.get()),
                )]
            };

            let members: Vec<Container> = vec![
                LocalContainer::new(
                    "focus",
                    Content::new(ComponentId(1), TextContent::new("Focus")),
                )
                .unwrap()
                .title("Focus")
                .into(),
                LocalContainer::new(
                    "other",
                    Content::new(ComponentId(1), TextContent::new("Other")),
                )
                .unwrap()
                .title("Other")
                .dynamic_toolbar(move |_| build_other_items())
                .on_toolbar_action(move |_, act| {
                    actions_cb.borrow_mut().push(act.clone());
                    Ok(())
                })
                .into(),
            ];
            let options =
                PresentationOptions::new("focus").others(OthersOptions::new(vec!["other".into()]));
            let mut host = Host::from_containers(members, false, None, Some(options));
            assert!(host.mount_columns());

            // Width where Others (~35% of 840 = ~294 minus tabs/inset) fits icon+More or overflows full capsule (~220pt).
            let narrow = Rect::new(0., 0., 760., 760.);
            render_presentation_cycle(&mut host, narrow, Theme::LIGHT);

            let tabs_frame = host.presentation_switcher_frame().unwrap();
            let focus = focus_index(&host);
            assert!(
                host.panel_mgr.panels[focus].mounted_toolbar.sources().any(
                    |s| matches!(s, Source::PanelOverflow { module, .. } if module == "other")
                ),
                "Together policy in narrow Others must fold group into Source::PanelOverflow"
            );
            assert!(
                !host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .sources()
                    .any(|s| matches!(s, Source::Panel { module, .. } if module == "other")),
                "Together policy must keep zero partial capsule groups on narrow toolbar"
            );
            for occ in host.panel_mgr.panels[focus]
                .mounted_toolbar
                .occupied_frames()
            {
                assert!(!rects_overlap(occ, tabs_frame));
            }

            // Open More -> Model controls -> High button AND Model -> claude-opus-4 deliver exact member_ids.
            let (_, more_frame, _) = find_trigger_frame_and_id(&host, "More").unwrap();
            let more_target = host.toolbar_target_at(more_frame.center()).unwrap();
            host.dispatch_toolbar(more_target, &ContentEvent::PointerDown(more_frame.center()))
                .unwrap();
            host.dispatch_toolbar(more_target, &ContentEvent::PointerUp(more_frame.center()))
                .unwrap();
            render_presentation_cycle(&mut host, narrow, Theme::LIGHT);

            let (ctrl_key, _) = find_ax_part(&mut host, "Model controls").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, ctrl_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, narrow, Theme::LIGHT);
            let (high_key, _) = find_ax_part(&mut host, "High").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, high_key, AccessibilityAction::Press)
                    .unwrap()
            );
            assert_eq!(actions.borrow().last().unwrap().id, 503);
            assert!(matches!(
                actions.borrow().last().unwrap().payload,
                Some(tekes_ui_controls::composition::ContentActionPayload::ToolbarCommand(503))
            ));

            // Re-open More -> Model controls -> Model -> claude-opus-4.
            host.dispatch_toolbar(more_target, &ContentEvent::PointerDown(more_frame.center()))
                .unwrap();
            host.dispatch_toolbar(more_target, &ContentEvent::PointerUp(more_frame.center()))
                .unwrap();
            render_presentation_cycle(&mut host, narrow, Theme::LIGHT);
            let (ctrl_key, _) = find_ax_part(&mut host, "Model controls").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, ctrl_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, narrow, Theme::LIGHT);
            let (model_key, _) = find_ax_part(&mut host, "claude-sonnet-4").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, model_key, AccessibilityAction::Press)
                    .unwrap()
            );
            render_presentation_cycle(&mut host, narrow, Theme::LIGHT);
            let (opus_key, _) = find_ax_part(&mut host, "claude-opus-4").unwrap();
            assert!(
                host.toolbar_accessibility_action(true, opus_key, AccessibilityAction::Press)
                    .unwrap()
            );
            assert_eq!(actions.borrow().last().unwrap().id, 502);
            assert_eq!(
                actions.borrow().last().unwrap().menu_value(),
                Some("claude-opus-4")
            );

            // Switch to Split at a width fitting prefix (501) + More (~920pt window -> ~320pt Others minus tabs ~80pt).
            overflow_mode.set(LocalToolbarGroupOverflow::Split);
            let mid = Rect::new(0., 0., 860., 760.);
            render_presentation_cycle(&mut host, mid, Theme::LIGHT);
            assert!(
                host.panel_mgr.panels[focus]
                    .mounted_toolbar
                    .sources()
                    .any(|s| matches!(s, Source::Panel { module, .. } if module == "other")),
                "Split policy must keep fitting capsule prefix directly on toolbar"
            );
            assert!(
                host.panel_mgr.panels[focus].mounted_toolbar.sources().any(
                    |s| matches!(s, Source::PanelOverflow { module, .. } if module == "other")
                ),
                "Split policy must send remaining segments to overflow"
            );
            let group_seg_count = host.panel_mgr.panels[focus]
                .mounted_toolbar
                .host
                .content_host()
                .semantics()
                .into_iter()
                .find_map(|n| {
                    (n.semantics.name == "Model controls").then(|| {
                        host.panel_mgr.panels[focus]
                            .mounted_toolbar
                            .host
                            .content_host()
                            .get(n.id)
                            .unwrap()
                            .component::<ToolbarGroup>()
                            .unwrap()
                            .children
                            .len()
                    })
                })
                .unwrap();
            assert!((1..3).contains(&group_seg_count));
        }

        fn write_rgba8_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) {
            fn crc32(bytes: &[u8]) -> u32 {
                let mut crc = 0xFFFF_FFFFu32;
                for &b in bytes {
                    crc ^= u32::from(b);
                    for _ in 0..8 {
                        crc = if crc & 1 != 0 {
                            (crc >> 1) ^ 0xEDB8_8320
                        } else {
                            crc >> 1
                        };
                    }
                }
                !crc
            }
            fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
                out.extend_from_slice(&(data.len() as u32).to_be_bytes());
                out.extend_from_slice(tag);
                out.extend_from_slice(data);
                let mut crc_input = Vec::with_capacity(4 + data.len());
                crc_input.extend_from_slice(tag);
                crc_input.extend_from_slice(data);
                out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
            }
            let row_bytes = width as usize * 4;
            let mut raw = Vec::with_capacity((row_bytes + 1) * height as usize);
            for y in 0..height as usize {
                raw.push(0);
                raw.extend_from_slice(&rgba[y * row_bytes..(y + 1) * row_bytes]);
            }
            let mut adler_a = 1u32;
            let mut adler_b = 0u32;
            for &b in &raw {
                adler_a = (adler_a + u32::from(b)) % 65521;
                adler_b = (adler_b + adler_a) % 65521;
            }
            let mut zlib = Vec::with_capacity(raw.len() + 6 + (raw.len() / 65535 + 1) * 5);
            zlib.push(0x78);
            zlib.push(0x01);
            for (i, chunk) in raw.chunks(65535).enumerate() {
                let last = (i + 1) * 65535 >= raw.len();
                zlib.push(u8::from(last));
                let len = chunk.len() as u16;
                zlib.extend_from_slice(&len.to_le_bytes());
                zlib.extend_from_slice(&(!len).to_le_bytes());
                zlib.extend_from_slice(chunk);
            }
            zlib.extend_from_slice(&((adler_b << 16) | adler_a).to_be_bytes());

            let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
            let mut ihdr = Vec::with_capacity(13);
            ihdr.extend_from_slice(&width.to_be_bytes());
            ihdr.extend_from_slice(&height.to_be_bytes());
            ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
            push_chunk(&mut png, b"IHDR", &ihdr);
            push_chunk(&mut png, b"IDAT", &zlib);
            push_chunk(&mut png, b"IEND", &[]);
            std::fs::write(path, png).unwrap();
        }

        #[test]
        fn capsule_group_retina_visual_snapshots() {
            use crate::{AppWindow, LocalToolbarGroup, LocalToolbarTone};

            let out_dir =
                std::env::var_os("TEKESUI_CAPSULE_SCREENSHOT_DIR").map(std::path::PathBuf::from);
            let Ok(mut renderer) = tekes_ui_render::Renderer::new_headless() else {
                if out_dir.is_some() {
                    panic!("headless GPU renderer required to write screenshots");
                }
                return;
            };
            if let Some(dir) = &out_dir {
                std::fs::create_dir_all(dir).unwrap();
            }

            let build_window = || {
                let access_entries = vec![MenuEntry::Item {
                    value: "ws".into(),
                    label: "Workspace".into(),
                    checked: Some(true),
                    disabled: false,
                }];
                let model_entries = vec![MenuEntry::Item {
                    value: "claude-sonnet-4".into(),
                    label: "claude-sonnet-4".into(),
                    checked: Some(true),
                    disabled: false,
                }];
                let containers: Vec<Container> = vec![
                    LocalContainer::new(
                        "focus",
                        Content::new(ComponentId(1), TextContent::new("Focused container body")),
                    )
                    .unwrap()
                    .title("Workspace")
                    .toolbar_items(vec![
                        LocalToolbarItem::Group(
                            LocalToolbarGroup::new(
                                10,
                                vec![
                                    LocalToolbarItem::Button(
                                        LocalToolbarButton::new(11, "")
                                            .icon("tekes-back")
                                            .tooltip("Back"),
                                    ),
                                    LocalToolbarItem::Button(
                                        LocalToolbarButton::new(12, "")
                                            .icon("tekes-forward")
                                            .tooltip("Forward"),
                                    ),
                                ],
                            )
                            .divided(false)
                            .label("Navigation"),
                        ),
                        LocalToolbarItem::Group(
                            LocalToolbarGroup::new(
                                20,
                                vec![
                                    LocalToolbarItem::Menu(
                                        LocalToolbarMenu::new(21, "", access_entries.clone())
                                            .icon("tekes-folder-edit")
                                            .tooltip("Access")
                                            .dropdown_indicator(false),
                                    ),
                                    LocalToolbarItem::Menu(
                                        LocalToolbarMenu::new(
                                            22,
                                            "claude-sonnet-4",
                                            model_entries.clone(),
                                        )
                                        .tooltip("Model")
                                        .dropdown_indicator(false)
                                        .tone(LocalToolbarTone::Secondary),
                                    ),
                                    LocalToolbarItem::Button(
                                        LocalToolbarButton::new(23, "High").tooltip("Effort"),
                                    ),
                                ],
                            )
                            .label("Controls"),
                        ),
                        LocalToolbarItem::Button(LocalToolbarButton::new(30, "Run").tooltip("Run")),
                    ])
                    .into(),
                    LocalContainer::new(
                        "other",
                        Content::new(ComponentId(1), TextContent::new("Secondary inspector body")),
                    )
                    .unwrap()
                    .title("Inspector")
                    .toolbar_items(vec![LocalToolbarItem::Group(
                        LocalToolbarGroup::new(
                            40,
                            vec![
                                LocalToolbarItem::Menu(
                                    LocalToolbarMenu::new(41, "", access_entries)
                                        .icon("tekes-folder-edit")
                                        .tooltip("Access")
                                        .dropdown_indicator(false),
                                ),
                                LocalToolbarItem::Menu(
                                    LocalToolbarMenu::new(42, "claude-sonnet-4", model_entries)
                                        .tooltip("Model")
                                        .dropdown_indicator(false),
                                ),
                                LocalToolbarItem::Button(LocalToolbarButton::new(43, "High")),
                            ],
                        )
                        .label("Inspector controls"),
                    )])
                    .into(),
                ];
                let options = PresentationOptions::new("focus")
                    .others(OthersOptions::new(vec!["other".into()]));
                AppWindow::from_containers(containers, options)
                    .unwrap()
                    .into_window_handler()
                    .unwrap()
                    .1
            };

            for (name, size_pt, theme) in [
                ("capsule_focus_light_2x.png", (1340, 240), Theme::LIGHT),
                ("capsule_focus_dark_2x.png", (1340, 240), Theme::DARK),
                (
                    "capsule_others_overflow_light_2x.png",
                    (760, 240),
                    Theme::LIGHT,
                ),
                (
                    "capsule_others_overflow_dark_2x.png",
                    (760, 240),
                    Theme::DARK,
                ),
            ] {
                let mut win = build_window();
                win.set_theme(theme);
                let rgba = win
                    .render_headless_rgba8(&mut renderer, size_pt, 2)
                    .unwrap();
                let w_px = size_pt.0 * 2;
                let h_px = size_pt.1 * 2;
                assert_eq!(rgba.len(), (w_px * h_px * 4) as usize);
                assert!(rgba.windows(4).any(|px| px != &rgba[0..4]));
                if let Some(dir) = &out_dir {
                    write_rgba8_png(&dir.join(name), w_px, h_px, &rgba);
                }
            }
        }
    }
}
