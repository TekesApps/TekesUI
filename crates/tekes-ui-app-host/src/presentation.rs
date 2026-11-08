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
#[derive(Clone, Debug, PartialEq)]
pub struct OthersOptions {
    pub containers: Vec<String>,
    pub visible: bool,
    pub selected: Option<OthersTab>,
}
impl OthersOptions {
    pub fn new(containers: Vec<String>) -> Self {
        let selected = containers.first().cloned().map(OthersTab::Container);
        Self {
            containers,
            visible: true,
            selected,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PresentationOptions {
    pub nav_bar: Option<String>,
    pub focus: String,
    /// None means no area and no toggle. Some(visible=false) retains state.
    pub others: Option<OthersOptions>,
    pub nav_width: f32,
    pub focus_fraction: f32,
    /// Explicit stable identities for the optional history service. Empty means
    /// no recipients; it never implicitly means all containers or focus.
    pub navigation_targets: Vec<String>,
    /// Explicit container receiving the optional headless service's controls.
    pub navigation_toolbar: Option<String>,
}
impl PresentationOptions {
    pub fn new(focus: impl Into<String>) -> Self {
        Self {
            nav_bar: None,
            focus: focus.into(),
            others: None,
            nav_width: 220.,
            focus_fraction: 0.6,
            navigation_targets: Vec::new(),
            navigation_toolbar: None,
        }
    }
    pub fn nav_bar(mut self, id: impl Into<String>) -> Self {
        self.nav_bar = Some(id.into());
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
    pub fn navigation_toolbar_id(&self) -> Option<&str> {
        self.options.navigation_toolbar.as_deref()
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
    pub fn frames(&self, area: Rect) -> Vec<(String, Rect)> {
        self.layout(area).frames
    }
    pub fn layout(&self, area: Rect) -> PresentationLayout {
        const SPLITTER: f32 = 6.;
        const TABS: f32 = 36.;
        let mut layout = PresentationLayout::default();
        let mut right = area;
        if let Some(nav) = self.nav_bar_id() {
            let width = self
                .options
                .nav_width
                .min((area.size.width - SPLITTER).max(0.) * 0.5);
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
        // SplitView ratios describe the usable span, excluding the divider.
        let usable_width = (right.size.width - SPLITTER).max(0.);
        let focus_width = if self.others_visible() {
            usable_width * self.options.focus_fraction
        } else {
            right.size.width
        };
        let others_width = if self.others_visible() {
            usable_width - focus_width
        } else {
            0.
        };
        let focus = Rect::new(right.min_x(), right.min_y(), focus_width, right.size.height);
        layout.frames.push((self.focus_id().into(), focus));
        layout.clips.push((self.focus_id().into(), focus));
        if !self.others_visible() {
            return layout;
        }
        let x = right.min_x() + focus_width + SPLITTER.min(right.size.width);
        let tabs_height = TABS.min(right.size.height);
        layout.tabs = Some(Rect::new(x, right.min_y(), others_width, tabs_height));
        let viewport = Rect::new(
            x,
            right.min_y() + tabs_height,
            others_width,
            (right.size.height - tabs_height).max(0.),
        );
        match self.selected_tab() {
            Some(OthersTab::Container(id)) => {
                layout.frames.push((id.clone(), viewport));
                layout.clips.push((id.clone(), viewport));
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
