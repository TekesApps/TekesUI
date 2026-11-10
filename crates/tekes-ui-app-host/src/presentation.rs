//! Stable container presentation. This state never owns Content, processes or
//! navigation history; changing a position cannot clone or retarget them.
use crate::columns::{Column, Container, ContainerOptions};
use anyhow::{Result, ensure};
use tekes_ui_scene::Rect;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OthersTab {
    Container(String),
    List,
}
/// A declared multi-instance destination. It remains a menu with one open member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OthersGroup {
    pub id: String,
    pub title: String,
    pub icon: String,
    /// Stable window identities, including currently closed or focused members.
    pub containers: Vec<String>,
}
impl OthersGroup {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        icon: impl Into<String>,
        containers: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: icon.into(),
            containers,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct OthersOptions {
    pub containers: Vec<String>,
    pub groups: Vec<OthersGroup>,
    pub visible: bool,
    pub selected: Option<OthersTab>,
}
impl OthersOptions {
    pub fn new(containers: Vec<String>) -> Self {
        let selected = containers.first().cloned().map(OthersTab::Container);
        Self {
            containers,
            groups: Vec::new(),
            visible: true,
            selected,
        }
    }
    pub fn groups(mut self, groups: Vec<OthersGroup>) -> Self {
        self.groups = groups;
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PresentationOptions {
    pub nav_bar: Option<String>,
    pub focus: String,
    /// None means no area and no toggle. Some(visible=false) retains state.
    pub others: Option<OthersOptions>,
    pub nav_width: f32,
    /// Optional relative width, preserving the original AppHost resize ratios.
    pub nav_fraction: Option<f32>,
    pub focus_fraction: f32,
    /// Explicit stable identities for the optional history service. Empty means
    /// no recipients; it never implicitly means all containers or focus.
    pub navigation_targets: Vec<String>,
    /// Explicit container receiving the optional headless service's controls.
    pub navigation_toolbar: Option<String>,
    /// Explicitly keep window history controls in the current focus Toolbar.
    /// Business navigation_targets and the service identity remain unchanged.
    pub navigation_toolbar_follows_focus: bool,
}
impl PresentationOptions {
    pub fn new(focus: impl Into<String>) -> Self {
        Self {
            nav_bar: None,
            focus: focus.into(),
            others: None,
            nav_width: 220.,
            nav_fraction: None,
            focus_fraction: 0.6,
            navigation_targets: Vec::new(),
            navigation_toolbar: None,
            navigation_toolbar_follows_focus: false,
        }
    }
    pub fn nav_bar(mut self, id: impl Into<String>) -> Self {
        self.nav_bar = Some(id.into());
        self
    }
    pub fn nav_fraction(mut self, fraction: f32) -> Self {
        self.nav_fraction = Some(fraction);
        self
    }
    pub fn others(mut self, others: OthersOptions) -> Self {
        self.others = Some(others);
        self
    }
    pub fn navigation_toolbar(mut self, id: impl Into<String>) -> Self {
        self.navigation_toolbar = Some(id.into());
        self
    }
    pub fn navigation_toolbar_follows_focus(mut self, enabled: bool) -> Self {
        self.navigation_toolbar_follows_focus = enabled;
        self
    }
    pub fn navigation_targets(mut self, ids: Vec<String>) -> Self {
        self.navigation_targets = ids;
        self
    }
}
#[derive(Clone, Debug)]
struct Member {
    id: String,
    list_height: Option<f32>,
}
#[derive(Clone, Debug)]
pub struct PresentationState {
    options: PresentationOptions,
    nav_visible: bool,
    members: Vec<Member>,
    list_order: Vec<String>,
    list_scroll: f32,
}
/// Container allocation and its visible clipping rectangle use the same IDs.
/// A List item can extend beyond its viewport; its own layout keeps its height.
#[derive(Clone, Debug, Default)]
pub struct PresentationLayout {
    pub frames: Vec<(String, Rect)>,
    pub clips: Vec<(String, Rect)>,
    pub tabs: Option<Rect>,
    pub list_viewport: Option<Rect>,
}
impl PresentationState {
    pub fn new(containers: &[Container], options: PresentationOptions) -> Result<Self> {
        crate::columns::validate_containers(containers)?;
        Self::assemble(
            containers.iter().map(|c| (c.id(), c.container_options())),
            options,
        )
    }
    pub(crate) fn from_columns(
        containers: &[Column],
        options: PresentationOptions,
    ) -> Result<Self> {
        crate::columns::validate_ids(containers.iter().map(|c| (c.id(), c.options())))?;
        Self::assemble(containers.iter().map(|c| (c.id(), c.options())), options)
    }
    fn assemble<'a>(
        containers: impl IntoIterator<Item = (&'a str, &'a ContainerOptions)>,
        options: PresentationOptions,
    ) -> Result<Self> {
        let members: Vec<_> = containers
            .into_iter()
            .map(|(id, options)| Member {
                id: id.into(),
                list_height: options.list_height,
            })
            .collect();
        let known = |id: &str| members.iter().any(|m| m.id == id);
        ensure!(
            known(&options.focus),
            "unknown focus container: {}",
            options.focus
        );
        ensure!(
            options.nav_width.is_finite() && options.nav_width > 0.,
            "invalid NavBar width"
        );
        ensure!(
            options
                .nav_fraction
                .is_none_or(|fraction| fraction.is_finite() && fraction > 0. && fraction < 1.),
            "invalid NavBar fraction"
        );
        ensure!(
            options.focus_fraction.is_finite()
                && options.focus_fraction > 0.
                && options.focus_fraction < 1.,
            "invalid focus fraction"
        );
        if let Some(nav) = &options.nav_bar {
            ensure!(known(nav), "unknown NavBar container: {nav}");
            ensure!(*nav != options.focus, "NavBar cannot also be focus");
        }
        if let Some(id) = &options.navigation_toolbar {
            ensure!(known(id), "unknown navigation toolbar container: {id}");
        }
        let mut targets = std::collections::HashSet::new();
        for id in &options.navigation_targets {
            ensure!(known(id), "unknown navigation target: {id}");
            ensure!(targets.insert(id), "duplicate navigation target: {id}");
        }
        if let Some(others) = &options.others {
            let mut ids = std::collections::HashSet::new();
            for id in &others.containers {
                ensure!(known(id), "unknown others container: {id}");
                ensure!(
                    *id != options.focus && options.nav_bar.as_ref() != Some(id),
                    "container has two presentation roles: {id}"
                );
                ensure!(ids.insert(id), "duplicate others container: {id}");
            }
            let mut group_ids = std::collections::HashSet::new();
            let mut grouped_members = std::collections::HashSet::new();
            for group in &others.groups {
                ensure!(
                    !group.id.is_empty()
                        && group
                            .id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
                    "invalid Others group ID"
                );
                ensure!(
                    group_ids.insert(&group.id),
                    "duplicate Others group: {}",
                    group.id
                );
                ensure!(!group.title.trim().is_empty(), "empty Others group title");
                ensure!(
                    tekes_ui_icons::Glyph::named(&group.icon).is_some(),
                    "unknown Others group icon: {}",
                    group.icon
                );
                ensure!(
                    !group.containers.is_empty(),
                    "empty Others group: {}",
                    group.id
                );
                for id in &group.containers {
                    ensure!(known(id), "unknown Others group member: {id}");
                    ensure!(
                        options.nav_bar.as_ref() != Some(id),
                        "NavBar cannot be an Others group member: {id}"
                    );
                    ensure!(
                        grouped_members.insert(id),
                        "container assigned to multiple Others groups: {id}"
                    );
                }
            }
            if let Some(OthersTab::Container(id)) = &others.selected {
                ensure!(
                    others.containers.contains(id),
                    "selected container is not open: {id}"
                );
            }
        }
        if options.others.is_none() {
            ensure!(
                members.iter().all(|member| member.id == options.focus
                    || options.nav_bar.as_ref() == Some(&member.id)),
                "containers outside focus/NavBar require an others area for reopening"
            );
        }
        let list_order = members
            .iter()
            .filter(|m| m.list_height.is_some())
            .map(|m| m.id.clone())
            .collect();
        let mut state = Self {
            options,
            nav_visible: true,
            members,
            list_order,
            list_scroll: 0.,
        };
        if state
            .options
            .others
            .as_ref()
            .is_some_and(|o| o.selected == Some(OthersTab::List))
        {
            ensure!(!state.list_ids().is_empty(), "cannot select an empty List");
        }
        state.reconcile_selection();
        Ok(state)
    }
    pub fn focus_id(&self) -> &str {
        &self.options.focus
    }
    pub fn nav_bar_id(&self) -> Option<&str> {
        self.options.nav_bar.as_deref()
    }
    pub fn nav_bar_visible(&self) -> bool {
        self.options.nav_bar.is_some() && self.nav_visible
    }
    pub fn visible_nav_bar_id(&self) -> Option<&str> {
        self.nav_bar_visible().then(|| self.nav_bar_id()).flatten()
    }
    pub fn toggle_nav_bar(&mut self) -> Result<()> {
        ensure!(self.options.nav_bar.is_some(), "NavBar is not configured");
        self.nav_visible = !self.nav_visible;
        Ok(())
    }
    pub fn navigation_toolbar_id(&self) -> Option<&str> {
        self.options.navigation_toolbar.as_deref()
    }
    pub(crate) fn navigation_uses_window_edge(&self) -> bool {
        self.options.navigation_toolbar_follows_focus
    }
    pub fn navigation_toolbar_presenter_id(&self) -> Option<&str> {
        self.navigation_toolbar_id().map(|configured| {
            if self.options.navigation_toolbar_follows_focus {
                self.focus_id()
            } else {
                configured
            }
        })
    }
    pub fn navigation_targets(&self) -> &[String] {
        &self.options.navigation_targets
    }
    pub fn others(&self) -> Option<&OthersOptions> {
        self.options.others.as_ref()
    }
    pub fn others_ids(&self) -> &[String] {
        self.others().map_or(&[], |o| o.containers.as_slice())
    }
    pub fn others_visible(&self) -> bool {
        self.others().is_some_and(|o| o.visible)
    }
    pub fn selected_tab(&self) -> Option<&OthersTab> {
        self.others().and_then(|o| o.selected.as_ref())
    }
    pub fn list_ids(&self) -> Vec<&str> {
        self.list_order
            .iter()
            .filter(|id| self.others_ids().contains(id))
            .map(String::as_str)
            .collect()
    }
    pub fn tabs(&self) -> Vec<OthersTab> {
        let mut tabs: Vec<_> = self
            .others_ids()
            .iter()
            .cloned()
            .map(OthersTab::Container)
            .collect();
        if !self.list_ids().is_empty() {
            tabs.push(OthersTab::List);
        }
        tabs
    }
    pub fn toggle_others(&mut self) -> Result<()> {
        let others = self
            .options
            .others
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("others is not configured"))?;
        others.visible = !others.visible;
        Ok(())
    }
    pub fn select_tab(&mut self, selected: OthersTab) -> Result<()> {
        ensure!(self.tabs().contains(&selected), "tab is not available");
        self.options
            .others
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("others is not configured"))?
            .selected = Some(selected);
        Ok(())
    }
    pub fn open(&mut self, id: &str) -> Result<()> {
        ensure!(
            self.members.iter().any(|m| m.id == id),
            "unknown container: {id}"
        );
        ensure!(
            id != self.focus_id() && self.nav_bar_id() != Some(id),
            "container already has a presentation role: {id}"
        );
        let others = self
            .options
            .others
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("others is not configured"))?;
        if !others.containers.iter().any(|i| i == id) {
            others.containers.push(id.into());
        }
        others.selected = Some(OthersTab::Container(id.into()));
        Ok(())
    }
    /// Closing a tab only removes its presentation; its backing remains mounted.
    pub fn close(&mut self, id: &str) -> Result<()> {
        let others = self
            .options
            .others
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("others is not configured"))?;
        let index = others
            .containers
            .iter()
            .position(|i| i == id)
            .ok_or_else(|| anyhow::anyhow!("container is not open: {id}"))?;
        others.containers.remove(index);
        self.reconcile_selection();
        Ok(())
    }
    /// A focus owner exchanges only with a selected ordinary Others tab.
    /// List aggregates and absent selections never imply a first-item fallback.
    pub fn focus_switch_target(&self, owner: &str) -> Option<&str> {
        if owner == self.focus_id() {
            match self.selected_tab() {
                Some(OthersTab::Container(id)) => Some(id.as_str()),
                _ => None,
            }
        } else {
            self.others_ids()
                .iter()
                .find(|id| id.as_str() == owner)
                .map(String::as_str)
        }
    }
    pub fn promote_tab(&mut self, tab: &OthersTab) -> Result<()> {
        match tab {
            OthersTab::Container(id) => self.promote(id),
            OthersTab::List => anyhow::bail!("List is not a container and cannot become focus"),
        }
    }
    pub fn promote(&mut self, id: &str) -> Result<()> {
        let others = self
            .options
            .others
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("others is not configured"))?;
        let index = others
            .containers
            .iter()
            .position(|i| i == id)
            .ok_or_else(|| anyhow::anyhow!("container is not in others: {id}"))?;
        let old_focus = std::mem::replace(&mut self.options.focus, id.into());
        others.containers[index] = old_focus.clone();
        if others.selected == Some(OthersTab::Container(id.into())) {
            others.selected = Some(OthersTab::Container(old_focus));
        }
        self.reconcile_selection();
        Ok(())
    }
    pub fn reorder_list(&mut self, id: &str, index: usize) -> Result<()> {
        let visible: Vec<_> = self.list_ids().into_iter().map(str::to_owned).collect();
        ensure!(
            visible.iter().any(|i| i == id),
            "container is not in List: {id}"
        );
        ensure!(index < visible.len(), "List index out of range");
        let old = self.list_order.iter().position(|i| i == id).unwrap();
        self.list_order.remove(old);
        let mut reordered = visible;
        reordered.retain(|i| i != id);
        let destination = if let Some(next) = reordered.get(index) {
            self.list_order.iter().position(|i| i == next).unwrap()
        } else if let Some(last) = reordered.last() {
            self.list_order.iter().position(|i| i == last).unwrap() + 1
        } else {
            0
        };
        self.list_order.insert(destination, id.into());
        Ok(())
    }
    pub fn scroll_list(&mut self, delta: f32, viewport_height: f32) {
        if !delta.is_finite() || !viewport_height.is_finite() {
            return;
        }
        let height: f32 = self.list_ids().iter().map(|id| self.list_height(id)).sum();
        self.list_scroll =
            (self.list_scroll + delta).clamp(0., (height - viewport_height.max(0.)).max(0.));
    }
    pub fn list_scroll(&self) -> f32 {
        self.list_scroll
    }
    pub fn set_nav_width(&mut self, width: f32) -> Result<()> {
        ensure!(width.is_finite() && width > 0., "invalid NavBar width");
        self.options.nav_width = width;
        Ok(())
    }
    pub fn set_focus_fraction(&mut self, fraction: f32) -> Result<()> {
        ensure!(
            fraction.is_finite() && fraction > 0. && fraction < 1.,
            "invalid focus fraction"
        );
        self.options.focus_fraction = fraction;
        Ok(())
    }
    fn reconcile_selection(&mut self) {
        let list_available = !self.list_ids().is_empty();
        if let Some(others) = &mut self.options.others {
            let valid = match &others.selected {
                Some(OthersTab::Container(id)) => others.containers.contains(id),
                Some(OthersTab::List) => list_available,
                None => false,
            };
            if !valid {
                others.selected = others.containers.first().cloned().map(OthersTab::Container);
            }
        }
    }
    fn list_height(&self, id: &str) -> f32 {
        self.members
            .iter()
            .find(|m| m.id == id)
            .and_then(|m| m.list_height)
            .unwrap_or(0.)
    }
    pub(crate) fn configured_nav_width(&self, area: Rect) -> f32 {
        if self.nav_bar_id().is_none() {
            return 0.;
        }
        self.options
            .nav_fraction
            .map_or(self.options.nav_width, |ratio| {
                (area.size.width - 1.).max(0.) * ratio
            })
            .min((area.size.width - 1.).max(0.))
    }
    fn reference_split_width(&self, area: Rect) -> f32 {
        // Preserve the old global SplitView fractions. The center receives the
        // second one-point seam; hiding NavBar cannot change the opposite pane.
        (area.size.width - self.configured_nav_width(area) - 1.).max(1.)
    }
    pub(crate) fn resize_nav_width(&mut self, width: f32, area: Rect, minimum: f32) {
        let others = self.reference_split_width(area) * (1. - self.options.focus_fraction);
        let maximum = (area.size.width - 2. - others - minimum).max(minimum);
        let width = width.clamp(minimum, maximum);
        if self.options.nav_fraction.is_some() {
            self.options.nav_fraction =
                Some((width / (area.size.width - 1.).max(1.)).clamp(0.01, 0.99));
        } else {
            self.options.nav_width = width;
        }
        self.options.focus_fraction =
            (1. - others / self.reference_split_width(area)).clamp(0.01, 0.99);
    }
    pub(crate) fn resize_focus_ratio(
        &mut self,
        ratio: f32,
        area: Rect,
        minimum: f32,
    ) -> Result<()> {
        let visible_right = area.size.width
            - if self.nav_bar_visible() {
                self.configured_nav_width(area) + 1.
            } else {
                0.
            };
        let others = (visible_right - 1.).max(0.) * (1. - ratio);
        let reference = self.reference_split_width(area);
        let minimum =
            ((minimum + if self.nav_bar_id().is_some() { 1. } else { 0. }) / reference).min(0.49);
        self.set_focus_fraction(
            (1. - others / reference).clamp(minimum.max(0.01), 1. - minimum.max(0.01)),
        )
    }
    pub fn frames(&self, area: Rect) -> Vec<(String, Rect)> {
        self.layout(area).frames
    }
    pub fn layout(&self, area: Rect) -> PresentationLayout {
        self.layout_with_toolbar(area, 0.)
    }
    pub(crate) fn layout_with_toolbar(
        &self,
        area: Rect,
        toolbar_height: f32,
    ) -> PresentationLayout {
        const SPLITTER: f32 = 1.;
        const TABS: f32 = 36.;
        let mut layout = PresentationLayout::default();
        let mut right = area;
        if let Some(nav) = self.visible_nav_bar_id() {
            let width = self.configured_nav_width(area);
            let frame = Rect::new(area.min_x(), area.min_y(), width, area.size.height);
            layout.frames.push((nav.into(), frame));
            layout.clips.push((nav.into(), frame));
            right = Rect::new(
                area.min_x() + width + SPLITTER.min(area.size.width),
                area.min_y(),
                (area.size.width - width - SPLITTER).max(0.),
                area.size.height,
            );
        }
        // Hiding NavBar expands focus only; the opposite Others allocation
        // retains the original AppHost width and reappears without a jump.
        let others_width = if self.others_visible() {
            self.reference_split_width(area) * (1. - self.options.focus_fraction)
        } else {
            0.
        };
        let focus_width = if self.others_visible() {
            (right.size.width - SPLITTER - others_width).max(0.)
        } else {
            right.size.width
        };
        let focus = Rect::new(right.min_x(), right.min_y(), focus_width, right.size.height);
        layout.frames.push((self.focus_id().into(), focus));
        layout.clips.push((self.focus_id().into(), focus));
        if !self.others_visible() {
            return layout;
        }
        // Anchor the unchanged opposite allocation to its original window edge.
        // Reconstructing x through focus introduces a different rounding path
        // when NavBar is hidden, although Others has not moved or resized.
        let x = area.max_x() - others_width;
        let header = if toolbar_height > 0. {
            toolbar_height
        } else {
            TABS
        }
        .min(right.size.height)
        .max(0.);
        let tabs_height = header;
        layout.tabs = Some(Rect::new(x, right.min_y(), others_width, tabs_height));
        let viewport = Rect::new(
            x,
            right.min_y() + header,
            others_width,
            (right.size.height - header).max(0.),
        );
        match self.selected_tab() {
            Some(OthersTab::Container(id)) => {
                let frame = Rect::new(x, right.min_y(), others_width, right.size.height);
                layout.frames.push((id.clone(), frame));
                layout.clips.push((id.clone(), frame));
            }
            Some(OthersTab::List) => {
                layout.list_viewport = Some(viewport);
                let ids = self.list_ids();
                let total: f32 = ids.iter().map(|id| self.list_height(id)).sum();
                let scroll = self.list_scroll.min((total - viewport.size.height).max(0.));
                let mut item_y = viewport.min_y() - scroll;
                for id in ids {
                    let height = self.list_height(id);
                    let frame = Rect::new(viewport.min_x(), item_y, viewport.size.width, height);
                    let top = frame.min_y().max(viewport.min_y());
                    let bottom = frame.max_y().min(viewport.max_y());
                    if bottom > top && viewport.size.width > 0. {
                        layout.frames.push((id.into(), frame));
                        layout.clips.push((
                            id.into(),
                            Rect::new(frame.min_x(), top, frame.size.width, bottom - top),
                        ));
                    }
                    item_y += height;
                }
            }
            None => {}
        }
        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LocalContainer, ProcessContainer};
    use tekes_ui_controls::composition::{ComponentId, Content, TextFieldContent};
    fn local(id: &str, list: Option<f32>) -> Container {
        LocalContainer::new(
            id,
            Content::new(ComponentId(1), TextFieldContent::new(id, id)),
        )
        .unwrap()
        .options(ContainerOptions {
            list_height: list,
            ..ContainerOptions::default()
        })
        .into()
    }
    fn members() -> Vec<Container> {
        vec![
            local("nav", None),
            local("a", Some(120.)),
            local("b", Some(180.)),
            local("c", None),
            ProcessContainer::new(crate::ModuleDefinition {
                id: "remote",
                create: || panic!("presentation must not create workers"),
            })
            .list_height(220.)
            .into(),
        ]
    }
    fn configured() -> PresentationState {
        PresentationState::new(
            &members(),
            PresentationOptions::new("a")
                .nav_bar("nav")
                .others(OthersOptions::new(vec![
                    "b".into(),
                    "c".into(),
                    "remote".into(),
                ]))
                .navigation_targets(vec!["c".into(), "remote".into()]),
        )
        .unwrap()
    }
    #[test]
    fn ordinary_toolbars_stay_on_native_row_and_list_content_uses_remaining_viewport() {
        let mut state = configured();
        let area = Rect::new(0., 0., 1100., 300.);
        let ordinary = state.layout_with_toolbar(area, 52.);
        assert!(ordinary.frames.iter().all(|(_, frame)| frame.min_y() == 0.));
        assert_eq!(ordinary.tabs.unwrap().min_y(), 0.);
        assert_eq!(ordinary.tabs.unwrap().max_y(), 52.);
        state.select_tab(OthersTab::List).unwrap();
        let viewport = state.layout_with_toolbar(area, 52.).list_viewport.unwrap();
        assert_eq!(viewport.min_y(), 52.);
        state.scroll_list(10000., viewport.size.height);
        let list = state.layout_with_toolbar(area, 52.);
        assert_eq!(list.frames.last().unwrap().1.max_y(), area.max_y());
        for (_, clip) in list
            .clips
            .iter()
            .filter(|(id, _)| id == "b" || id == "remote")
        {
            assert!(clip.min_y() >= viewport.min_y() && clip.max_y() <= viewport.max_y());
        }
    }
    #[test]
    fn original_relative_widths_and_side_hiding_keep_the_opposite_allocation_fixed() {
        let members = members();
        let mut options = PresentationOptions::new("a")
            .nav_bar("nav")
            .nav_fraction(1. / 3.)
            .others(OthersOptions::new(vec![
                "b".into(),
                "c".into(),
                "remote".into(),
            ]));
        options.focus_fraction = 0.5;
        let mut state = PresentationState::new(&members, options).unwrap();
        for area in [
            Rect::new(0., 0., 1100., 640.),
            Rect::new(0., 0., 900., 640.),
        ] {
            let shown = state.layout(area);
            let nav = shown.frames.iter().find(|(id, _)| id == "nav").unwrap().1;
            assert!((nav.size.width - (area.size.width - 1.) / 3.).abs() < 0.01);
            let others = shown.tabs.unwrap();
            let first = tekes_ui_controls::extended::SplitView::new(1. / 3.)
                .layout(area, tekes_ui_controls::extended::Axis::Horizontal);
            let second = tekes_ui_controls::extended::SplitView::new(2. / 3.)
                .layout(area, tekes_ui_controls::extended::Axis::Horizontal);
            assert!((nav.max_x() - first.0.max_x()).abs() < 0.01);
            assert!((others.min_x() - second.2.min_x()).abs() < 0.01);
            assert!((others.size.width - second.2.size.width).abs() < 0.01);
            state.toggle_nav_bar().unwrap();
            let hidden = state.layout(area);
            assert_eq!(hidden.tabs.unwrap(), others);
            assert_eq!(hidden.frames[0].1.min_x(), area.min_x());
            state.toggle_nav_bar().unwrap();
            assert_eq!(state.layout(area).frames, shown.frames);
        }
        let area = Rect::new(0., 0., 1100., 640.);
        let original = state.layout(area).tabs.unwrap();
        state.resize_nav_width(300., area, 120.);
        let dragged = state.layout(area).tabs.unwrap();
        assert!((dragged.min_x() - original.min_x()).abs() < 0.01);
        assert!((dragged.size.width - original.size.width).abs() < 0.01);
    }
    #[test]
    fn nav_visibility_and_explicit_follow_focus_keep_business_binding_and_original_members() {
        let mut state = configured();
        let before = state.layout(Rect::new(0., 0., 900., 600.));
        state.toggle_nav_bar().unwrap();
        assert_eq!(state.nav_bar_id(), Some("nav"));
        assert!(!state.nav_bar_visible());
        let hidden = state.layout(Rect::new(0., 0., 900., 600.));
        assert!(!hidden.frames.iter().any(|(id, _)| id == "nav"));
        assert_eq!(hidden.frames[0].1.min_x(), 0.);
        state.toggle_nav_bar().unwrap();
        assert_eq!(
            state.layout(Rect::new(0., 0., 900., 600.)).frames,
            before.frames
        );
        state.options.navigation_toolbar = Some("a".into());
        state.options.navigation_toolbar_follows_focus = true;
        state.promote("remote").unwrap();
        state.toggle_others().unwrap();
        assert_eq!(state.navigation_toolbar_id(), Some("a"));
        assert_eq!(state.navigation_toolbar_presenter_id(), Some("remote"));
        assert_eq!(state.navigation_targets(), &["c", "remote"]);
        assert_eq!(state.focus_switch_target("remote"), Some("b"));
        state.toggle_others().unwrap();
        state.select_tab(OthersTab::List).unwrap();
        assert_eq!(state.focus_switch_target("remote"), None);
        assert_eq!(state.focus_switch_target("b"), Some("b"));
        assert_eq!(state.focus_switch_target("nav"), None);
    }
    #[test]
    fn focus_and_others_split_width_and_any_single_member_can_swap() {
        let mut state = configured();
        for area in [
            Rect::new(0., 0., 1100., 640.),
            Rect::new(0., 0., 650., 300.),
        ] {
            let layout = state.layout(area);
            let focus = layout.frames.iter().find(|(id, _)| id == "a").unwrap().1;
            let tabs = layout.tabs.unwrap();
            assert_eq!(focus.min_y(), area.min_y());
            assert_eq!(focus.size.height, area.size.height);
            assert!(tabs.min_x() > focus.max_x());
            assert_eq!(tabs.min_y(), area.min_y());
            assert_eq!(tabs.max_x(), area.max_x());
        }
        assert_eq!(
            state.selected_tab(),
            Some(&OthersTab::Container("b".into()))
        );
        state.promote("remote").unwrap();
        assert_eq!(state.focus_id(), "remote");
        assert_eq!(
            state.selected_tab(),
            Some(&OthersTab::Container("b".into())),
            "non-selected target does not change the selected Tab"
        );
        assert_eq!(state.others_ids(), &["b", "c", "a"]);
        state.select_tab(OthersTab::List).unwrap();
        state.promote("b").unwrap();
        assert_eq!(state.focus_id(), "b");
        assert_eq!(state.others_ids(), &["remote", "c", "a"]);
        assert_eq!(state.selected_tab(), Some(&OthersTab::List));
        assert!(state.promote_tab(&OthersTab::List).is_err());
        assert_eq!(state.navigation_targets(), &["c", "remote"]);
    }
    #[test]
    fn four_startup_combinations_and_off_keep_identity_and_selection() {
        let area = Rect::new(10., 20., 800., 600.);
        for nav in [false, true] {
            for others in [false, true] {
                let mut options = PresentationOptions::new("a");
                if nav {
                    options.nav_bar = Some("nav".into());
                }
                if others {
                    options.others = Some(OthersOptions::new(vec!["b".into()]));
                }
                let containers: Vec<_> = members()
                    .into_iter()
                    .filter(|member| others || member.id() == "a" || nav && member.id() == "nav")
                    .collect();
                let mut state = PresentationState::new(&containers, options).unwrap();
                let before = state.layout(area);
                assert_eq!(
                    before.frames.len(),
                    1 + usize::from(nav) + usize::from(others)
                );
                assert_eq!(before.tabs.is_some(), others);
                if others {
                    let selection = state.selected_tab().cloned();
                    state.toggle_others().unwrap();
                    assert!(state.layout(area).tabs.is_none());
                    assert_eq!(state.selected_tab(), selection.as_ref());
                    state.toggle_others().unwrap();
                    assert_eq!(state.layout(area).frames, before.frames);
                } else {
                    assert!(state.toggle_others().is_err());
                }
            }
        }
    }
    #[test]
    fn switching_and_swapping_never_rebind_navigation_or_duplicate_focus() {
        let mut state = configured();
        state
            .select_tab(OthersTab::Container("remote".into()))
            .unwrap();
        assert_eq!(state.focus_id(), "a");
        state.promote("remote").unwrap();
        assert_eq!(state.focus_id(), "remote");
        assert_eq!(state.others_ids(), &["b", "c", "a"]);
        assert_eq!(state.navigation_targets(), &["c", "remote"]);
        assert_eq!(
            state.selected_tab(),
            Some(&OthersTab::Container("a".into()))
        );
        assert!(state.promote_tab(&OthersTab::List).is_err());
        state.select_tab(OthersTab::List).unwrap();
        assert_eq!(state.list_ids(), ["a", "b"]);
        assert_eq!(
            state
                .frames(Rect::new(0., 0., 600., 800.))
                .iter()
                .filter(|(id, _)| id == "remote")
                .count(),
            1
        );
        assert!(!state.list_ids().contains(&state.focus_id()));
    }
    #[test]
    fn list_order_is_independent_clipped_and_scrollable_without_recreating_members() {
        let mut state = configured();
        let tabs = state.others_ids().to_vec();
        state.reorder_list("remote", 0).unwrap();
        assert_eq!(state.list_ids(), ["remote", "b"]);
        assert_eq!(state.others_ids(), tabs);
        state.select_tab(OthersTab::List).unwrap();
        let area = Rect::new(0., 0., 900., 300.);
        let before = state.layout(area);
        let viewport = before.list_viewport.unwrap();
        state.scroll_list(10000., viewport.size.height);
        let after = state.layout(area);
        assert!(state.list_scroll() > 0.);
        for (id, clip) in after
            .clips
            .iter()
            .filter(|(id, _)| id == "remote" || id == "b")
        {
            assert!(clip.min_y() >= viewport.min_y(), "{id}");
            assert!(clip.max_y() <= viewport.max_y(), "{id}");
            assert!(
                after.frames.iter().any(
                    |(frame_id, frame)| frame_id == id && frame.size.height >= clip.size.height
                )
            );
        }
        assert!(state.reorder_list("c", 0).is_err());
    }
    #[test]
    fn absent_others_rejects_unreachable_extra_containers_but_configured_off_can_reopen() {
        let extra = vec![local("a", None), local("hidden", None)];
        let error = PresentationState::new(&extra, PresentationOptions::new("a")).unwrap_err();
        assert!(error.to_string().contains("require an others area"));
        let mut options = OthersOptions::new(Vec::new());
        options.visible = false;
        let mut state =
            PresentationState::new(&extra, PresentationOptions::new("a").others(options)).unwrap();
        state.open("hidden").unwrap();
        state.toggle_others().unwrap();
        assert_eq!(
            state.selected_tab(),
            Some(&OthersTab::Container("hidden".into()))
        );
    }
    #[test]
    fn empty_list_tab_disappears_and_returns_when_an_eligible_member_opens() {
        let mut state = configured();
        state.close("b").unwrap();
        state.close("remote").unwrap();
        assert!(!state.tabs().contains(&OthersTab::List));
        assert!(state.select_tab(OthersTab::List).is_err());
        state.open("b").unwrap();
        assert!(state.tabs().contains(&OthersTab::List));
        state.select_tab(OthersTab::List).unwrap();
        state.close("b").unwrap();
        assert_eq!(
            state.selected_tab(),
            Some(&OthersTab::Container("c".into()))
        );
        state.open("b").unwrap();
        assert_eq!(state.list_ids(), ["b"]);
    }
    #[test]
    fn invalid_roles_targets_and_list_metadata_are_rejected_at_assembly() {
        let mut options = PresentationOptions::new("a").nav_bar("a");
        assert!(PresentationState::new(&members(), options.clone()).is_err());
        options.nav_bar = None;
        options.others = Some(OthersOptions::new(vec!["a".into()]));
        assert!(PresentationState::new(&members(), options).is_err());
        assert!(PresentationState::new(&members(), PresentationOptions::new("missing")).is_err());
        assert!(
            PresentationState::new(
                &members(),
                PresentationOptions::new("a").navigation_targets(vec!["missing".into()])
            )
            .is_err()
        );
        assert!(
            PresentationState::new(
                &[local("bad", Some(f32::NAN))],
                PresentationOptions::new("bad")
            )
            .is_err()
        );
    }
}
