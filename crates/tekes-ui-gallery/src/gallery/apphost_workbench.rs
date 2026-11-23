//! Gallery shell. AppHost is the sole input, layout and window owner.
//! The original renderer remains available through the diagnostic `--legacy` entry.
use super::workbench;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::Rc,
};
use tekes_ui_app_host::{
    AppWindow, ContainerContent, ContainerRenderContext, LocalContainer, NavigationOptions,
    OthersOptions, PresentationOptions,
};
#[cfg(test)]
use tekes_ui_controls::composition::{KeyValueTableContent, RichContent, RichDocument};
use tekes_ui_controls::{
    CheckState,
    composition::{
        CheckboxContent, ComponentId, Content, ContentActionPayload, ContentHost,
        InspectionSnapshot, KeyedSelectableListContent, ListContent, ListDisplayUpdate,
        ListIntentKind, ListRowDisplay, ScrollContent, SearchFieldContent, StackContent,
        TextContent, TextFieldContent,
    },
    extended::{KeyedListSnapshot, ListBudget, ListItemKey},
};
#[cfg(test)]
use tekes_ui_tokens::proposal::TextRole;
use tekes_ui_tokens::{Palette, Spacing};

const FILTER: ComponentId = ComponentId(1);
pub(super) const CHOOSER: ComponentId = ComponentId(3);
const COMMAND: u64 = 0x4757_0000;
pub(super) use super::apphost_specimens::SEARCH_ROUTE;
#[cfg(test)]
pub use super::apphost_specimens::Specimen;
use super::apphost_specimens::directory_page;
#[cfg(test)]
use super::apphost_specimens::{AREA_ROUTE, COMBO_ROUTE, FORM_ROUTE, SELECT_ROUTE};
pub use super::apphost_specimens::{Selection, specimens};
fn story_section(story: &str) -> Option<usize> {
    match story {
        "docs" => Some(0),
        "playground" => Some(1),
        "states" => Some(2),
        "behaviors" => Some(3),
        "edge-cases" => Some(4),
        "variants" => Some(5),
        _ => None,
    }
}
pub(super) fn resolve_route(raw: &str) -> anyhow::Result<(String, usize)> {
    let (slug, story) = raw.split_once('/').unwrap_or((raw, "playground"));
    let section =
        story_section(story).ok_or_else(|| anyhow::anyhow!("unknown Gallery route: {raw}"))?;
    if workbench::apphost_quality::category_for_slug(slug).is_some() {
        anyhow::ensure!(
            matches!(story, "docs" | "playground"),
            "unknown Gallery route: {raw}"
        );
        return Ok((slug.to_owned(), section));
    }
    let entry = workbench::entries()
        .iter()
        .find(|entry| {
            entry.slug() == slug || (!entry.component().is_empty() && entry.component() == slug)
        })
        .ok_or_else(|| anyhow::anyhow!("unknown Gallery route: {raw}"))?;
    anyhow::ensure!(entry.supports_story(story), "unknown Gallery route: {raw}");
    Ok((entry.slug().to_owned(), section))
}
fn open_route_host(
    raw: &str,
    palette: &Rc<Cell<Palette>>,
) -> anyhow::Result<(String, MountedPage)> {
    let (page, section) = resolve_route(raw)?;
    let mut host = page_host(&page, palette)?;
    host.select_section(section);
    Ok((page, host))
}
fn known_page(page: &str) -> bool {
    resolve_route(page).is_ok()
}
/// Shared by the Focus and the two read-only panels. A host is never cloned.
/// Panels read the selection and an observational snapshot of its live host.
pub struct WorkbenchState {
    settings: Option<tekes_ui_app_host::SettingsOwner>,
    window_requests: usize,
    pub(super) palette: Rc<Cell<Palette>>,
    current: u64,
    visits: Vec<Selection>,
    visit_sections: Vec<Option<usize>>,
    pending_section: Option<(String, usize)>,
    pages: HashMap<String, Rc<RefCell<MountedPage>>>,
    pending: VecDeque<tekes_ui_app_host::navigation::Entry>,
    inspected: Option<(String, ComponentId, u64)>,
    pub(super) document_return_requested: bool,
    session_dirs: Vec<super::apphost::SessionTempDir>,
}
pub type SharedWorkbench = Rc<RefCell<WorkbenchState>>;
impl WorkbenchState {
    pub fn selection(&self) -> &Selection {
        &self.visits[self.current as usize]
    }
    #[cfg(test)]
    fn with_selected_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost, Specimen)) {
        let selection = self.selection().clone();
        if let Some(specimen) = specimens(&selection.page)
            .iter()
            .find(|item| Some(item.key) == selection.specimen.as_deref())
        {
            visit(
                &mut self
                    .pages
                    .get(&selection.page)
                    .expect("current page mounted")
                    .borrow_mut(),
                *specimen,
            );
        }
    }
    // Commands run inside a borrowed Focus host. Apply recording changes at the
    // next host operation, after that borrow ends, without replaying any event.
    pub(super) fn sync_inspection(&mut self) {
        if let Some((target_page, section)) = self.pending_section.clone()
            && let Some(mounted) = self.pages.get(&target_page)
            && let Ok(mut guard) = mounted.try_borrow_mut()
        {
            guard.select_section(section);
            self.pending_section = None;
        }
        let selection = self.selection();
        let target = specimens(&selection.page)
            .iter()
            .find(|item| Some(item.key) == selection.specimen.as_deref())
            .and_then(|item| {
                self.pages[&selection.page]
                    .borrow()
                    .get(item.component_id)
                    .map(|node| {
                        (
                            selection.page.clone(),
                            item.component_id,
                            node.mount_identity(),
                        )
                    })
            });
        if target == self.inspected {
            return;
        }
        if let Some((page, id, _)) = self.inspected.take() {
            self.pages[&page]
                .borrow_mut()
                .set_inspection_enabled(id, false);
        }
        if let Some((page, id, _)) = &target {
            self.pages[page]
                .borrow_mut()
                .set_inspection_enabled(*id, true);
        }
        self.inspected = target;
    }
    pub(super) fn selected_snapshot(&self) -> Option<InspectionSnapshot> {
        let selection = self.selection();
        let specimen = specimens(&selection.page)
            .iter()
            .find(|item| Some(item.key) == selection.specimen.as_deref())?;
        self.pages[&selection.page]
            .borrow()
            .inspection_snapshot(specimen.component_id)
    }
    fn entry(&self, visit: u64) -> tekes_ui_app_host::navigation::Entry {
        let selection = &self.visits[visit as usize];
        tekes_ui_app_host::navigation::Entry {
            id: format!("gallery-visit-{visit}"),
            route: selection.page.clone(),
            params: serde_json::json!({"visit": visit, "specimen": selection.specimen, "document_id": selection.document_id}),
        }
    }
    fn navigate(&mut self, route: &str) {
        let explicit = route.contains('/');
        let (canonical, section) = match resolve_route(route) {
            Ok((slug, section)) => (slug, explicit.then_some(section)),
            Err(_) => (route.to_owned(), None),
        };
        let page = canonical.as_str();
        if self.selection().page == page && section.is_none() {
            return;
        }
        // Each visit retains its own chooser selection, including repeated routes.
        let previous = self
            .visits
            .iter()
            .rev()
            .find(|selection| selection.page == page);
        let selection = Selection::new(
            page,
            previous.and_then(|selection| selection.specimen.as_deref()),
        );
        let visit = self.visits.len() as u64;
        self.visits.push(selection);
        self.visit_sections.push(section);
        self.pending.push_back(self.entry(visit));
    }
    pub(super) fn select(&mut self, key: Option<&str>) {
        let page = self.selection().page.clone();
        self.visits[self.current as usize] = if key.is_none() {
            Selection {
                page,
                specimen: None,
                document_id: None,
            }
        } else {
            Selection::new(&page, key)
        };
    }
    fn restore(&mut self, command: u64) -> anyhow::Result<()> {
        let visit = command
            .checked_sub(COMMAND)
            .filter(|visit| (*visit as usize) < self.visits.len())
            .ok_or_else(|| anyhow::anyhow!("unknown Gallery visit"))?;
        let page = self.visits[visit as usize].page.clone();
        if !self.pages.contains_key(&page) {
            self.pages.insert(
                page.clone(),
                Rc::new(RefCell::new(page_host(&page, &self.palette)?)),
            );
        }
        if let Some(Some(section)) = self.visit_sections.get(visit as usize).copied() {
            if let Ok(mut mounted) = self.pages[&page].try_borrow_mut() {
                mounted.select_section(section);
                self.pending_section = None;
            } else {
                self.pending_section = Some((page, section));
            }
        } else {
            self.pending_section = None;
        }
        self.current = visit;
        Ok(())
    }
    pub(crate) fn current_route(&self) -> &str {
        &self.selection().page
    }
    pub(crate) fn switch_to_route(&mut self, raw_route: &str) -> anyhow::Result<()> {
        let explicit = raw_route.contains('/');
        let (page, section) = resolve_route(raw_route)?;
        let already = self.selection().page == page;
        if !self.pages.contains_key(&page) {
            let (_, host) = open_route_host(raw_route, &self.palette)?;
            self.pages.insert(page.clone(), Rc::new(RefCell::new(host)));
        }
        self.pages[&page].borrow_mut().select_section(section);
        self.pending_section = None;
        if already {
            if let Some(slot) = self.visit_sections.get_mut(self.current as usize) {
                *slot = explicit.then_some(section);
            }
        } else {
            self.visits.push(Selection::new(&page, None));
            self.visit_sections.push(explicit.then_some(section));
            self.current = (self.visits.len() - 1) as u64;
        }
        self.sync_inspection();
        Ok(())
    }
    pub(crate) fn with_current_page<R>(&mut self, visit: impl FnOnce(&mut MountedPage) -> R) -> R {
        let page = self.selection().page.clone();
        let mut mounted = self.pages[&page].borrow_mut();
        visit(&mut mounted)
    }
    pub(crate) fn with_current_page_ref<R>(&self, visit: impl FnOnce(&MountedPage) -> R) -> R {
        let page = self.selection().page.as_str();
        let mounted = self.pages[page].borrow();
        visit(&mounted)
    }
    pub(crate) fn set_locale(&mut self, locale: super::locale::Locale) {
        self.with_current_page(|page| page.set_locale(locale));
    }
    pub(crate) fn clear_calibration_reference_dir(&mut self) {
        self.with_current_page(MountedPage::clear_calibration_reference_dir);
    }
}
pub(super) fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
pub(super) fn stack(id: u64, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent::vertical(Spacing::Regular.value(), children),
    )
}
/// Original hosted stories keep their owning StoryState; plain pages keep the
/// previously accepted Host. Both expose exactly one Host to AppHost.
pub(crate) enum MountedPage {
    Brand(Box<workbench::apphost_brand::BrandStory>),
    Feedback(Box<workbench::apphost_feedback::FeedbackStory>),
    Patterns(Box<workbench::apphost_patterns::AppHostPatterns>),
    Plain(Box<ContentHost>),
    Story(Box<workbench::apphost_story::AppHostStory>),
    Basic(Box<workbench::apphost_basic::BasicStory>),
    Scene3D(Box<workbench::apphost_scene3d::AppHostScene3D>),
    Motion(Box<workbench::apphost_motion::AppHostMotion>),
    Icons(Box<workbench::apphost_icons::AppHostIcons>),
    Documents(Box<workbench::apphost_documents::AppHostDocuments>),
    Foundations(Box<workbench::apphost_foundations::AppHostFoundations>),
}
impl std::ops::Deref for MountedPage {
    type Target = ContentHost;
    fn deref(&self) -> &ContentHost {
        match self {
            Self::Brand(story) => &story.host,
            Self::Feedback(story) => &story.host,
            Self::Patterns(story) => story.host(),
            Self::Plain(host) => host,
            Self::Story(story) => story.host(),
            Self::Basic(story) => &story.host,
            Self::Scene3D(story) => &story.host,
            Self::Motion(story) => &story.host,
            Self::Icons(story) => &story.host,
            Self::Documents(story) => &story.host,
            Self::Foundations(story) => &story.host,
        }
    }
}
impl std::ops::DerefMut for MountedPage {
    fn deref_mut(&mut self) -> &mut ContentHost {
        match self {
            Self::Brand(story) => &mut story.host,
            Self::Feedback(story) => &mut story.host,
            Self::Patterns(story) => story.host_mut(),
            Self::Plain(host) => host,
            Self::Story(story) => story.host_mut(),
            Self::Basic(story) => &mut story.host,
            Self::Scene3D(story) => &mut story.host,
            Self::Motion(story) => &mut story.host,
            Self::Icons(story) => &mut story.host,
            Self::Documents(story) => &mut story.host,
            Self::Foundations(story) => &mut story.host,
        }
    }
}
impl MountedPage {
    fn select_section(&mut self, section: usize) {
        match self {
            Self::Feedback(story) => story.select_section(section),
            Self::Patterns(story) => story.select_section(section),
            Self::Story(story) => story.select_section(section),
            Self::Basic(story) => story.select_section(section),
            Self::Scene3D(story) => story.select_section(section),
            Self::Icons(story) => story.select_section(section),
            Self::Documents(story) => story.select_section(section),
            Self::Brand(_) | Self::Plain(_) | Self::Motion(_) | Self::Foundations(_) => {}
        }
    }
    pub(crate) fn motion(&self) -> Option<&workbench::apphost_motion::AppHostMotion> {
        match self {
            Self::Motion(story) => Some(story),
            _ => None,
        }
    }
    pub(crate) fn motion_mut(&mut self) -> Option<&mut workbench::apphost_motion::AppHostMotion> {
        match self {
            Self::Motion(story) => Some(story),
            _ => None,
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scene3d(&self) -> Option<&workbench::apphost_scene3d::AppHostScene3D> {
        match self {
            Self::Scene3D(story) => Some(story),
            _ => None,
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scene3d_mut(
        &mut self,
    ) -> Option<&mut workbench::apphost_scene3d::AppHostScene3D> {
        match self {
            Self::Scene3D(story) => Some(story),
            _ => None,
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn icons(&self) -> Option<&workbench::apphost_icons::AppHostIcons> {
        match self {
            Self::Icons(story) => Some(story),
            _ => None,
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn icons_mut(&mut self) -> Option<&mut workbench::apphost_icons::AppHostIcons> {
        match self {
            Self::Icons(story) => Some(story),
            _ => None,
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn documents(&self) -> Option<&workbench::apphost_documents::AppHostDocuments> {
        match self {
            Self::Documents(story) => Some(story),
            _ => None,
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn documents_mut(
        &mut self,
    ) -> Option<&mut workbench::apphost_documents::AppHostDocuments> {
        match self {
            Self::Documents(story) => Some(story),
            _ => None,
        }
    }
    pub(crate) fn set_locale(&mut self, locale: super::locale::Locale) {
        match self {
            Self::Motion(story) => story.set_locale(locale),
            Self::Documents(story) => story.set_locale(locale),
            _ => {}
        }
    }
    pub(crate) fn clear_calibration_reference_dir(&mut self) {
        if let Self::Documents(story) = self {
            story.clear_calibration_reference_dir();
        }
    }
}
impl From<ContentHost> for MountedPage {
    fn from(host: ContentHost) -> Self {
        Self::Plain(Box::new(host))
    }
}
/// #291 composition fixture remains separately labelled inside the Field page.
pub(super) fn composition_fixture() -> Content {
    Content::new(
        ComponentId(200),
        ListContent::new(
            "Composition example",
            vec![
                text(
                    202,
                    "Additional composition fixture — not the original Playground",
                ),
                Content::new(
                    ComponentId(100),
                    TextFieldContent::new("Project name", "Gallery"),
                ),
                Content::new(
                    ComponentId(102),
                    TextFieldContent::new("Project owner", "TekesUI team"),
                ),
                Content::new(
                    ComponentId(101),
                    CheckboxContent::new("Published", CheckState::Unchecked),
                ),
            ],
        ),
    )
}
fn page_host(page: &str, palette: &Rc<Cell<Palette>>) -> anyhow::Result<MountedPage> {
    if page == "lab-matrix" {
        return Ok(MountedPage::Brand(Box::new(
            workbench::apphost_brand::BrandStory::new(palette.clone()),
        )));
    }
    if let Some(category) = workbench::apphost_quality::category_for_slug(page) {
        return Ok(workbench::apphost_quality::host_for(category)?.into());
    }

    let entry = workbench::entries()
        .iter()
        .find(|entry| entry.slug() == page)
        .ok_or_else(|| anyhow::anyhow!("unknown Gallery route: {page}"))?;
    if matches!(
        entry.renderer(),
        "tokens"
            | "spacing"
            | "shape"
            | "accessibility"
            | "typography"
            | "iconography"
            | "a11y"
            | "stress"
    ) {
        return Ok(MountedPage::Foundations(Box::new(
            workbench::apphost_foundations::AppHostFoundations::new(entry)?,
        )));
    }
    if entry.renderer() == "icon" {
        return Ok(MountedPage::Icons(Box::new(
            workbench::apphost_icons::AppHostIcons::new(entry)?,
        )));
    }
    if workbench::apphost_text_pages::supports(entry.renderer())
        || entry.renderer() == "calibration"
    {
        return Ok(MountedPage::Documents(Box::new(
            workbench::apphost_documents::AppHostDocuments::new(entry)?,
        )));
    }
    if matches!(
        entry.renderer(),
        "form" | "filter" | "master" | "async" | "danger" | "settings" | "field-meta"
    ) {
        return Ok(MountedPage::Patterns(Box::new(
            workbench::apphost_patterns::AppHostPatterns::new(entry)?,
        )));
    }
    if workbench::apphost_feedback::supports(entry.renderer()) {
        return Ok(MountedPage::Feedback(Box::new(
            workbench::apphost_feedback::FeedbackStory::new(entry),
        )));
    }
    if entry.renderer() == "apphost" {
        let intro = Content::new(
            ComponentId(124005),
            StackContent::vertical(
                StackContent::SECTION_HEADER_GAP,
                vec![
                    workbench::story_heading(124002, entry.title()),
                    text(124003, workbench::story_docs(entry)),
                ],
            ),
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(124000),
            ScrollContent::document(StackContent::document_sections(
                ComponentId(124001),
                vec![
                    intro,
                    text(
                        124004,
                        format!(
                            "{}\n{}",
                            workbench::api_reference(entry),
                            "Open the original AppHost demonstration in its own window."
                        ),
                    ),
                    Content::new(
                        ComponentId(124020),
                        tekes_ui_controls::composition::Button::new("Open AppHost demonstration"),
                    ),
                ],
            )),
        ))
        .map_err(|error| anyhow::anyhow!("AppHost launcher: {error:?}"))?;
        host.set_action_events(true);
        return Ok(host.into());
    }
    if entry.renderer() == "consistency" {
        return Ok(workbench::apphost_quality::index_host(entry)?.into());
    }
    if entry.renderer() == "geometry" {
        return Ok(workbench::apphost_quality::geometry_host(entry)?.into());
    }
    if page == "foundation-motion" {
        return Ok(MountedPage::Motion(Box::new(
            workbench::apphost_motion::AppHostMotion::new(entry)?,
        )));
    }
    if page == "component-scene3d" {
        return Ok(MountedPage::Scene3D(Box::new(
            workbench::apphost_scene3d::AppHostScene3D::new(entry)?,
        )));
    }
    if matches!(entry.renderer(), "docs" | "index") {
        let mut children = vec![Content::new(
            ComponentId(93005),
            StackContent::vertical(
                StackContent::SECTION_HEADER_GAP,
                vec![
                    workbench::story_heading(93001, entry.title()),
                    text(93002, workbench::story_docs(entry)),
                ],
            ),
        )];
        if entry.renderer() == "docs" {
            children.push(text(93003, workbench::overview_copy(entry)));
        } else {
            let items = workbench::entries()
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.component().is_empty())
                .map(|(i, item)| {
                    Content::new(
                        ComponentId(94000 + i as u64),
                        tekes_ui_controls::composition::Button::new(format!(
                            "{}{}",
                            item.title(),
                            if item.planned() { " · planned" } else { "" }
                        )),
                    )
                })
                .collect();
            children.push(Content::new(
                ComponentId(93006),
                ListContent::new(entry.title(), items),
            ));
        }
        let mut host = ContentHost::new(Content::new(
            ComponentId(93000),
            ScrollContent::document(StackContent::document_sections(
                ComponentId(93004),
                children,
            )),
        ))
        .map_err(|error| anyhow::anyhow!("Gallery reference: {error:?}"))?;
        host.set_action_events(true);
        return Ok(host.into());
    }
    if matches!(
        page,
        "component-checkbox"
            | "component-button"
            | "component-stack"
            | "component-grid"
            | "component-separator"
            | "component-link"
            | "component-segmented-control"
            | "component-breadcrumb"
            | "component-list"
            | "component-tree"
            | "component-data-table"
            | "component-chart"
            | "component-split-view"
            | "component-icon-button"
            | "component-command-palette"
            | "component-tabs"
            | "component-disclosure"
            | "component-toolbar"
            | "component-number-field"
            | "component-switch"
            | "component-slider"
            | "component-radio-group"
            | "component-markdown"
            | "component-code-block"
            | "component-diff-view"
            | "component-surface"
            | "component-skeleton"
            | "component-status-badge"
            | "component-progress"
    ) {
        return Ok(MountedPage::Basic(Box::new(
            workbench::apphost_basic::BasicStory::new(entry),
        )));
    }
    if workbench::apphost_story::supports(page) {
        return Ok(MountedPage::Story(Box::new(
            workbench::apphost_story::AppHostStory::new(entry),
        )));
    }
    let content = stack(
        200,
        vec![
            text(201, entry.title()),
            text(
                202,
                "This route has not migrated to AppHost yet. Open the default Gallery to use its existing stories.",
            ),
        ],
    );
    let mut host =
        ContentHost::new(content).map_err(|error| anyhow::anyhow!("Gallery page: {error:?}"))?;
    host.set_action_events(true);
    Ok(host.into())
}
struct FocusContent(SharedWorkbench);
impl FocusContent {
    fn current_page(&self) -> Rc<RefCell<MountedPage>> {
        let state = self.0.borrow();
        state
            .pages
            .get(&state.selection().page)
            .expect("current page mounted")
            .clone()
    }
}
impl ContainerContent for FocusContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        self.0.borrow_mut().sync_inspection();
        let visited_page = self.0.borrow().selection().page.clone();
        let index_page = visited_page == "component-index";
        let consistency_page = visited_page == "lab-consistency";
        let launcher_page = visited_page == "apphost";
        let host = self.current_page();
        let mut page = host.borrow_mut();
        visit(&mut page);
        let pending_section = self.0.borrow_mut().pending_section.take();
        if let Some((target_page, section)) = pending_section {
            if target_page == visited_page {
                page.select_section(section);
            } else if let Some(other) = self.0.borrow().pages.get(&target_page).cloned() {
                other.borrow_mut().select_section(section);
            }
        }
        if self.0.borrow().selection().page != visited_page {
            // Navigation commands execute while borrowing this departing Host.
            // Stop it here; borrowing it again through WorkbenchState would panic.
            page.set_window_active(false);
        }
        match &mut *page {
            MountedPage::Brand(story) => story.after_host_visit(),
            MountedPage::Feedback(story) => story.after_host_visit(),
            MountedPage::Patterns(story) => story.after_host_visit(),
            MountedPage::Story(story) => story.after_host_visit(),
            MountedPage::Basic(story) => story.after_host_visit(),
            MountedPage::Scene3D(story) => story.after_host_visit(),
            MountedPage::Motion(story) => story.after_host_visit(),
            MountedPage::Icons(story) => story.after_host_visit(),
            MountedPage::Documents(story) => story.after_host_visit(),
            MountedPage::Foundations(story) => story.after_host_visit(),
            MountedPage::Plain(_) => {}
        }
        if index_page || consistency_page || launcher_page {
            for record in page.drain_action_records() {
                if record.action.kind != tekes_ui_controls::composition::ContentActionKind::Activate
                {
                    continue;
                }
                if launcher_page {
                    if record.action.target == ComponentId(124020) {
                        self.0.borrow_mut().window_requests += 1;
                    }
                } else if consistency_page {
                    if let Some(destination) =
                        workbench::apphost_quality::destination(record.action.target)
                    {
                        self.0.borrow_mut().navigate(destination);
                    }
                } else if let Some(entry) = record
                    .action
                    .target
                    .0
                    .checked_sub(94000)
                    .and_then(|i| workbench::entries().get(i as usize))
                {
                    self.0.borrow_mut().navigate(entry.slug());
                }
            }
        }
    }
    fn take_window_open_request(
        &mut self,
        theme: tekes_ui_tokens::Theme,
    ) -> anyhow::Result<Option<tekes_ui_app_host::WindowOpenRequest>> {
        let settings = {
            let mut state = self.0.borrow_mut();
            if state.window_requests == 0 {
                return Ok(None);
            }
            state.window_requests -= 1;
            state
                .settings
                .as_ref()
                .expect("assembled Gallery Settings owner")
                .clone()
        };
        let (request, session_dir) = super::apphost::open_request(theme, settings)?;
        self.0.borrow_mut().session_dirs.push(session_dir);
        Ok(Some(request))
    }
    fn prepare_resources(
        &mut self,
        context: &mut ContainerRenderContext<'_>,
    ) -> anyhow::Result<()> {
        match &mut *self.current_page().borrow_mut() {
            MountedPage::Scene3D(story) => story.prepare_resources(context)?,
            MountedPage::Motion(story) => story.prepare_resources(context)?,
            MountedPage::Icons(story) => story.prepare_resources(context)?,
            MountedPage::Documents(story) => story.prepare_resources(context)?,
            _ => {}
        }
        Ok(())
    }
    fn finish_resources(&mut self, context: &mut ContainerRenderContext<'_>) -> anyhow::Result<()> {
        match &mut *self.current_page().borrow_mut() {
            MountedPage::Scene3D(story) => story.finish_resources(context)?,
            MountedPage::Motion(story) => story.finish_resources(context)?,
            MountedPage::Icons(story) => story.finish_resources(context)?,
            _ => {}
        }
        Ok(())
    }
    fn take_navigation_request(&mut self) -> Option<tekes_ui_app_host::navigation::Entry> {
        self.0.borrow_mut().pending.pop_front()
    }
}
fn navigation_sections(query: &str) -> Vec<super::navigation::Section> {
    if query.trim().is_empty() {
        super::navigation::sections().to_vec()
    } else {
        super::navigation::Navigation::filter_sections(query)
    }
}
pub(super) fn rows(snapshot: &KeyedListSnapshot, base: u64) -> Vec<ListRowDisplay> {
    snapshot
        .items
        .iter()
        .map(|item| ListRowDisplay {
            key: item.key,
            row_id: ComponentId(base + item.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect()
}
pub(super) fn list(
    name: &str,
    snapshot: &KeyedListSnapshot,
    base: u64,
) -> KeyedSelectableListContent {
    KeyedSelectableListContent::new(
        name,
        snapshot,
        &mut rows(snapshot, base),
        ListBudget {
            max_items: 1024,
            max_display_items: 1024,
            max_utf8_bytes: 1_000_000,
            max_staging_units: 100_000,
        },
    )
    .expect("bounded registered Gallery list")
}
struct NavContent {
    host: ContentHost,
    state: SharedWorkbench,
    query: String,
    page: String,
    sections: Vec<super::navigation::Section>,
}
impl ContainerContent for NavContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        use super::navigation::Navigation;
        let page = self.state.borrow().selection().page.clone();
        if self.page != page {
            for section in &self.sections {
                let key = section.items.iter().find_map(|candidate| {
                    let super::Page::Workbench(index) = candidate;
                    (workbench::entries()[*index].slug() == directory_page(&page))
                        .then_some(ListItemKey(*index as u64 + 1))
                });
                self.host
                    .set_keyed_list_selection(Navigation::list_id(section), key)
                    .expect("registered route selection");
            }
            self.page = page;
        }
        visit(&mut self.host);
        let query = self
            .host
            .get(FILTER)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
            .query()
            .to_owned();
        if query != self.query {
            // Only directory groups are replaced. The Search mount, IME state,
            // selection and focus remain owned by the same original Host.
            for section in &self.sections {
                self.host
                    .remove(Navigation::group_id(section))
                    .expect("mounted Gallery group");
            }
            self.sections = navigation_sections(&query);
            let selected =
                workbench::route(directory_page(&self.page)).expect("registered current page");
            for section in &self.sections {
                self.host
                    .insert(
                        super::navigation::LIST,
                        super::navigation::section_content(section, selected),
                    )
                    .expect("Gallery directory accepts original groups");
            }
            self.query = query;
        }
    }
    fn take_navigation_request(&mut self) -> Option<tekes_ui_app_host::navigation::Entry> {
        self.state.borrow_mut().pending.pop_front()
    }
}
fn navigation(state: SharedWorkbench) -> anyhow::Result<LocalContainer> {
    let sections = navigation_sections("");
    let selected = workbench::route(directory_page(&state.borrow().selection().page))
        .expect("registered current page");
    let section_contents = sections
        .iter()
        .map(|section| super::navigation::section_content(section, selected))
        .collect();
    let root = ListContent::scrollable_with_header(
        ComponentId(10),
        ComponentId(11),
        super::navigation::SCROLL,
        super::navigation::LIST,
        "Components",
        Content::new(FILTER, SearchFieldContent::new("Filter Gallery pages", "")),
        section_contents,
    );
    let mut host =
        ContentHost::new(root).map_err(|error| anyhow::anyhow!("Gallery navigation: {error:?}"))?;
    host.set_action_events(true);
    let actions = state.clone();
    Ok(LocalContainer::from_source(
        "gallery-navigation",
        NavContent {
            host,
            state,
            query: String::new(),
            page: String::new(),
            sections,
        },
    )
    .search_entry(FILTER)
    .on_action(move |_, records| {
        for record in records {
            let Some(section) = super::navigation::sections().iter().find(|section| {
                super::navigation::Navigation::list_id(section) == record.action.target
            }) else {
                continue;
            };
            if let Some(ContentActionPayload::ListIntent(intent)) = record.payload {
                let key = match intent.kind {
                    ListIntentKind::Activated => Some(intent.key),
                    ListIntentKind::SelectionChanged => intent.selected_key,
                };
                if let Some(index) = section.items.iter().find_map(|page| {
                    let super::Page::Workbench(index) = page;
                    (key == Some(ListItemKey(*index as u64 + 1))).then_some(*index)
                }) {
                    actions
                        .borrow_mut()
                        .navigate(workbench::entries()[index].slug());
                }
            }
        }
    }))
}
use super::apphost_panels::panel;
#[cfg(test)]
use super::apphost_panels::{
    DOCUMENT_HELP, DOCUMENT_NOTICE, DOCUMENT_READER, DOCUMENT_SCROLL, PanelContent,
    consume_chooser_actions, consume_document_links, document_body, document_key, document_scroll,
    document_source, inspector_body, linked_document, panel_source, publish_document,
};
fn assemble_with_session_path() -> anyhow::Result<(AppWindow, std::path::PathBuf)> {
    let page = std::env::var("TEKESUI_GALLERY_PAGE").unwrap_or_else(|_| SEARCH_ROUTE.into());
    assemble_for_route(&page)
}
fn assemble_for_route(raw_route: &str) -> anyhow::Result<(AppWindow, std::path::PathBuf)> {
    let (app, _, session_path) = assemble_for_route_with_state(raw_route)?;
    Ok((app, session_path))
}
pub(super) fn assemble_for_route_with_state(
    raw_route: &str,
) -> anyhow::Result<(AppWindow, SharedWorkbench, std::path::PathBuf)> {
    assemble_for_route_internal(raw_route, false)
}
fn assemble_for_route_internal(
    raw_route: &str,
    isolated_settings: bool,
) -> anyhow::Result<(AppWindow, SharedWorkbench, std::path::PathBuf)> {
    static NEXT_WORKBENCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let palette = Rc::new(Cell::new(Palette::from(tekes_ui_tokens::Theme::LIGHT)));
    let (page, host) = open_route_host(raw_route, &palette)?;
    let root = std::env::var_os("TEKESUI_APPHOST_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("tekesui-gallery-workbench"));
    let history_id = format!(
        "workbench-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
        NEXT_WORKBENCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    );
    let session_path = root.join(&history_id);
    let settings = if isolated_settings {
        super::apphost::settings_owner_in(session_path.clone())?
    } else {
        super::apphost::settings_owner()?
    };
    let state = Rc::new(RefCell::new(WorkbenchState {
        settings: Some(settings.clone()),
        window_requests: 0,
        palette: palette.clone(),
        current: 0,
        visits: vec![Selection::new(&page, None)],
        visit_sections: vec![story_section(
            raw_route.split_once('/').map_or("playground", |(_, s)| s),
        )],
        pending_section: None,
        pages: HashMap::from([(page, Rc::new(RefCell::new(host)))]),
        pending: VecDeque::new(),
        inspected: None,
        document_return_requested: false,
        session_dirs: vec![super::apphost::SessionTempDir::new(session_path.clone())],
    }));
    let initial = state.borrow().entry(0);
    let commands = state.clone();
    let focus = LocalContainer::from_source("gallery-focus", FocusContent(state.clone()))
        .title("Gallery")
        .on_command(move |_, command| commands.borrow_mut().restore(command));
    let presentation = PresentationOptions::new("gallery-focus")
        .nav_bar("gallery-navigation")
        .navigation_toolbar("gallery-navigation")
        .navigation_targets(vec!["gallery-focus".into()])
        .others(OthersOptions::new(vec![
            "gallery-documents".into(),
            "gallery-inspector".into(),
        ]));
    let app = AppWindow::from_containers(
        vec![
            navigation(state.clone())?.into(),
            focus.into(),
            panel("gallery-documents", "Documents", state.clone(), false)?.into(),
            panel("gallery-inspector", "Inspector", state.clone(), true)?.into(),
        ],
        presentation,
    )?
    .navigation(NavigationOptions {
        directory: session_path.clone(),
        history_id,
        initial_entries: vec![initial],
        view_command: Some(|entry| {
            anyhow::ensure!(known_page(&entry.route), "unknown Gallery route");
            entry
                .params
                .get("visit")
                .and_then(serde_json::Value::as_u64)
                .and_then(|visit| COMMAND.checked_add(visit))
                .ok_or_else(|| anyhow::anyhow!("invalid Gallery history visit"))
        }),
    })
    .app_name("TekesUI Gallery")
    .window_title("TekesUI Gallery")
    .palette_source(palette)
    .shared_settings(settings);
    Ok((app, state, session_path))
}
pub(crate) struct HeadlessGallery {
    window: tekes_ui_app_host::HostedWindow,
    state: SharedWorkbench,
    _session: super::apphost::SessionTempDir,
}
impl HeadlessGallery {
    pub(crate) fn open(raw_route: &str) -> anyhow::Result<Self> {
        let (app, state, session_path) = assemble_for_route_internal(raw_route, true)?;
        let (_, window) = app.into_window_handler()?;
        Ok(Self {
            window,
            state,
            _session: super::apphost::SessionTempDir::new(session_path),
        })
    }
    pub(crate) fn window_mut(&mut self) -> &mut tekes_ui_app_host::HostedWindow {
        &mut self.window
    }
    pub(crate) fn state(&self) -> &SharedWorkbench {
        &self.state
    }
    pub(crate) fn set_theme(&mut self, theme: tekes_ui_tokens::Theme) {
        self.window.set_theme(theme);
        let next = self.state.borrow().palette.get().with_theme(theme);
        self.state.borrow().palette.set(next);
    }
    pub(crate) fn switch_route(&mut self, raw_route: &str) -> anyhow::Result<()> {
        self.state.borrow_mut().switch_to_route(raw_route)
    }
}
/// Default to AppHost; retain the explicit selector and its original arguments.
pub fn apphost_arguments(args: &[String]) -> Option<Vec<String>> {
    match args {
        [_] => Some(args.to_vec()),
        [_, selector, ..] if selector == "--apphost" => {
            Some(args[..1].iter().chain(&args[2..]).cloned().collect())
        }
        _ => None,
    }
}
pub fn run(args: &[String]) -> anyhow::Result<()> {
    if tekes_ui_app_host::run_component_worker_from_args(args)? {
        return Ok(());
    }
    if let Ok(slug) = std::env::var("TEKESUI_GALLERY_PAGE")
        && resolve_route(&slug).is_err()
    {
        eprintln!("Unknown Gallery route: {slug}");
        std::process::exit(2);
    }
    #[cfg(not(feature = "selftest"))]
    if std::env::var_os("TEKES_SELFTEST").is_some() || std::env::var_os("TEKES_PERF").is_some() {
        eprintln!(
            "TEKES_SELFTEST and TEKES_PERF require building tekes-ui-gallery with --features selftest"
        );
        std::process::exit(2);
    }
    #[cfg(all(target_os = "macos", feature = "selftest"))]
    if std::env::var_os("TEKES_SELFTEST").is_some() || std::env::var_os("TEKES_PERF").is_some() {
        return super::apphost_harness::run_harness();
    }
    let (app, session_path) = assemble_with_session_path()?;
    let _session_dir = super::apphost::SessionTempDir::new(session_path);
    app.run_with_args(args)
}

#[cfg(test)]
#[path = "apphost_workbench_tests.rs"]
mod tests;
