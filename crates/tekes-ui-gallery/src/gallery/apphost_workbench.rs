//! Candidate Gallery shell. AppHost is the sole input, layout and window owner.
//! Unmigrated routes remain available in the legacy default Gallery.
use super::workbench;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};
use tekes_ui_app_host::{
    AppWindow, ContainerContent, LocalContainer, NavigationOptions, OthersOptions,
    PresentationOptions,
};
use tekes_ui_controls::{
    CheckState, ScrollArea,
    composition::{
        CheckboxContent, ComponentId, Content, ContentActionPayload, ContentHost,
        InspectionSnapshot, KeyedSelectableListContent, ListContent, ListDisplayUpdate,
        ListIntentKind, ListRowDisplay, RichDocument, ScrollContent, SearchFieldContent,
        StackContent, TextContent, TextFieldContent,
    },
    extended::{
        Axis, KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey, Stack,
    },
};
use tekes_ui_tokens::{Spacing, proposal::TextRole};

const FILTER: ComponentId = ComponentId(1);
const ROUTES: ComponentId = ComponentId(2);
const CHOOSER: ComponentId = ComponentId(3);
const COMMAND: u64 = 0x4757_0000;
const SEARCH_ROUTE: &str = "component-search-field";
const FORM_ROUTE: &str = "component-text-field";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub page: String,
    pub specimen: Option<String>,
    pub document_id: Option<String>,
}
#[derive(Clone, Copy)]
pub struct Specimen {
    pub key: &'static str,
    pub component_id: ComponentId,
    pub document_id: &'static str,
    pub label: &'static str,
}
const SEARCH: &[Specimen] = &[Specimen {
    key: "query",
    component_id: ComponentId(8801),
    document_id: "tui.search-field",
    label: "Search query",
}];
const FORM: &[Specimen] = &[
    Specimen {
        key: "name",
        component_id: ComponentId(100),
        document_id: "tui.text-field",
        label: "Project name",
    },
    Specimen {
        key: "owner",
        component_id: ComponentId(102),
        document_id: "tui.text-field",
        label: "Project owner",
    },
    Specimen {
        key: "published",
        component_id: ComponentId(101),
        document_id: "tui.checkbox",
        label: "Published",
    },
];
pub fn specimens(page: &str) -> &'static [Specimen] {
    match page {
        SEARCH_ROUTE => SEARCH,
        FORM_ROUTE => FORM,
        _ => &[],
    }
}
impl Selection {
    fn new(page: &str, specimen: Option<&str>) -> Self {
        let available = specimens(page);
        let selected = specimen
            .and_then(|key| available.iter().find(|item| item.key == key))
            .or_else(|| (specimen.is_none() && available.len() == 1).then(|| &available[0]));
        Self {
            page: page.into(),
            specimen: selected.map(|item| item.key.into()),
            document_id: selected.map(|item| item.document_id.into()),
        }
    }
}
/// Shared by the Focus and the two read-only panels. A host is never cloned.
/// Panels read the selection and an observational snapshot of its live host.
pub struct WorkbenchState {
    current: u64,
    visits: Vec<Selection>,
    pages: HashMap<String, Rc<RefCell<ContentHost>>>,
    pending: VecDeque<tekes_ui_app_host::navigation::Entry>,
    inspected: Option<(String, ComponentId, u64)>,
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
    fn sync_inspection(&mut self) {
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
    fn selected_snapshot(&self) -> Option<InspectionSnapshot> {
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
    fn navigate(&mut self, page: &str) {
        if self.selection().page == page {
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
        self.pending.push_back(self.entry(visit));
    }
    fn select(&mut self, key: Option<&str>) {
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
            self.pages
                .insert(page.clone(), Rc::new(RefCell::new(page_host(&page)?)));
        }
        self.current = visit;
        Ok(())
    }
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn stack(id: u64, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            children,
            flexible_child: None,
        },
    )
}
fn page_host(page: &str) -> anyhow::Result<ContentHost> {
    if page == SEARCH_ROUTE {
        return Ok(workbench::apphost_search());
    }
    // A new composition fixture using the original controls; this does not claim
    // migration of the legacy TextField story controls or its fixture toolbar.
    let content = if page == FORM_ROUTE {
        Content::new(
            ComponentId(200),
            ListContent::scrollable(
                "Composition example",
                ComponentId(201),
                vec![
                    text(202, "Project form — composition example"),
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
    } else {
        let entry = workbench::entries()
            .iter()
            .find(|entry| entry.slug() == page)
            .ok_or_else(|| anyhow::anyhow!("unknown Gallery route: {page}"))?;
        stack(
            200,
            vec![
                text(201, entry.title()),
                text(
                    202,
                    "This route has not migrated to AppHost yet. Open the default Gallery to use its existing stories.",
                ),
            ],
        )
    };
    let mut host =
        ContentHost::new(content).map_err(|error| anyhow::anyhow!("Gallery page: {error:?}"))?;
    host.set_action_events(true);
    Ok(host)
}
struct FocusContent(SharedWorkbench);
impl ContainerContent for FocusContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        self.0.borrow_mut().sync_inspection();
        let host = {
            let state = self.0.borrow();
            state
                .pages
                .get(&state.selection().page)
                .expect("current page mounted")
                .clone()
        };
        visit(&mut host.borrow_mut());
    }
}
fn list_snapshot(query: &str) -> KeyedListSnapshot {
    let items: Vec<_> = workbench::entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| KeyedListItem {
            key: ListItemKey(index as u64 + 1),
            name: format!("{} · {}", entry.section(), entry.title()),
        })
        .collect();
    let display_items = workbench::entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.matches(&query.to_lowercase()))
        .map(|(index, _)| ListItemKey(index as u64 + 1))
        .collect();
    KeyedListSnapshot {
        dataset: ListDatasetKey(1),
        items,
        display_items,
    }
}
fn rows(snapshot: &KeyedListSnapshot, base: u64) -> Vec<ListRowDisplay> {
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
fn list(name: &str, snapshot: &KeyedListSnapshot, base: u64) -> KeyedSelectableListContent {
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
}
impl ContainerContent for NavContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        let page = self.state.borrow().selection().page.clone();
        if self.page != page {
            let key = workbench::entries()
                .iter()
                .position(|entry| entry.slug() == page)
                .map(|index| ListItemKey(index as u64 + 1));
            self.host
                .set_keyed_list_selection(ROUTES, key)
                .expect("registered route selection");
            self.page = page;
        }
        visit(&mut self.host);
        let query = self
            .host
            .get(FILTER)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
            .input()
            .text
            .clone();
        if query != self.query {
            let snapshot = list_snapshot(&query);
            self.host
                .replace_keyed_list(ROUTES, &snapshot, &mut rows(&snapshot, 1000), None)
                .expect("registered route filter");
            self.query = query;
        }
    }
    fn take_navigation_request(&mut self) -> Option<tekes_ui_app_host::navigation::Entry> {
        self.state.borrow_mut().pending.pop_front()
    }
}
fn navigation(state: SharedWorkbench) -> anyhow::Result<LocalContainer> {
    let mut routes = list("Gallery pages", &list_snapshot(""), 1000);
    routes.set_navigation_layout(true);
    routes.set_navigation_selection_fill_only(true);
    let mut host = ContentHost::new(stack(
        10,
        vec![
            Content::new(FILTER, SearchFieldContent::new("Filter Gallery pages", "")),
            Content::new(
                ComponentId(11),
                ListContent::scrollable(
                    "Page directory",
                    ComponentId(12),
                    vec![Content::new(ROUTES, routes)],
                ),
            ),
        ],
    ))
    .map_err(|error| anyhow::anyhow!("Gallery navigation: {error:?}"))?;
    host.get_mut(ComponentId(10))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .flexible_child = Some(ComponentId(11));
    host.set_action_events(true);
    let actions = state.clone();
    Ok(LocalContainer::from_source(
        "gallery-navigation",
        NavContent {
            host,
            state,
            query: String::new(),
            page: String::new(),
        },
    )
    .on_action(move |_, records| {
        for record in records {
            if record.action.target != ROUTES {
                continue;
            }
            if let Some(ContentActionPayload::ListIntent(intent)) = record.payload {
                let key = match intent.kind {
                    ListIntentKind::Activated => Some(intent.key),
                    ListIntentKind::SelectionChanged => intent.selected_key,
                };
                if let Some(entry) = key
                    .and_then(|key| key.0.checked_sub(1))
                    .and_then(|index| workbench::entries().get(index as usize))
                {
                    actions.borrow_mut().navigate(entry.slug());
                }
            }
        }
    }))
}
fn chooser_snapshot(selection: &Selection) -> KeyedListSnapshot {
    let items = std::iter::once(KeyedListItem {
        key: ListItemKey(1),
        name: "Page guidance".into(),
    })
    .chain(
        specimens(&selection.page)
            .iter()
            .enumerate()
            .map(|(index, item)| KeyedListItem {
                key: ListItemKey(index as u64 + 2),
                name: item.label.into(),
            }),
    )
    .collect::<Vec<_>>();
    KeyedListSnapshot {
        dataset: ListDatasetKey(2),
        display_items: items.iter().map(|item| item.key).collect(),
        items,
    }
}
const DOCUMENTS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/component_documents.rs"));
fn document_source(id: &str) -> Option<&'static str> {
    DOCUMENTS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, source)| *source)
}

fn document_body(selection: &Selection) -> String {
    match selection.document_id.as_deref() {
        Some(id) => match document_source(id) {
            Some(source) => format!(
                "Document: {id}\n\n{}",
                RichDocument::from_markdown(source).text()
            ),
            None => format!("Component manual not provided: {id}"),
        },
        None => format!(
            "Page: {}\nSelect a specimen to read its component manual.\n{}",
            selection.page,
            if specimens(&selection.page).is_empty() {
                "This route has not migrated to AppHost; use the default Gallery for its existing stories."
            } else {
                "This composition uses independently selectable original components."
            }
        ),
    }
}
fn inspector_body(selection: &Selection, snapshot: Option<InspectionSnapshot>) -> String {
    let mut body = format!(
        "Page: {}\nSpecimen: {}\nDocument: {}\n",
        selection.page,
        selection.specimen.as_deref().unwrap_or("none"),
        selection.document_id.as_deref().unwrap_or("page guidance")
    );
    let Some(snapshot) = snapshot else {
        body.push_str("\nLive snapshot unavailable. Select a mounted specimen.");
        return body;
    };
    use std::fmt::Write;
    writeln!(
        body,
        "\nCurrent instance: {:?}, mount {}\nCurrent semantics: {:?}\nCurrent specimen focus: {}",
        snapshot.id, snapshot.mount, snapshot.semantics, snapshot.focused
    )
    .unwrap();
    body.push_str("\nCurrent public properties:\n");
    match snapshot.properties {
        Some(properties) => {
            for property in properties {
                writeln!(body, "{}: {}", property.name, property.value).unwrap();
            }
        }
        None => body.push_str("Not provided by this component.\n"),
    }
    match snapshot.geometry {
        Some(geometry) => writeln!(body, "\nCurrent measured layout (not declared sizing):\nFrame: {:?}\nRectangular clip: {:?}\nVisible rectangle: {:?}\nAdditional curve geometry: {}", geometry.frame, geometry.rectangular_clip, geometry.visible, geometry.has_curve_geometry).unwrap(),
        None => body.push_str("\nCurrent measured layout unavailable (not rendered or invalidated).\n"),
    }
    body.push_str("\nLast paint (historical; not recomputed from current properties):\n");
    match snapshot.last_paint {
        Some(paint) => {
            writeln!(
                body,
                "Theme: {:?}\nIncludes owned children: {}\nActual token lookup calls: {:?}",
                paint.theme, paint.includes_owned_children, paint.tokens.lookups
            )
            .unwrap();
            for color in paint.tokens.colors {
                writeln!(
                    body,
                    "Resolved role {}: {:?}; palette {:?}",
                    color.role, color.value, color.palette
                )
                .unwrap();
            }
            writeln!(
                body,
                "Emitted quads before GPU composition: {:#?}",
                paint.quads
            )
            .unwrap();
            body.push_str("Lookup calls do not prove every rendered pixel. Text and images are not sampled.\n");
        }
        None => body.push_str("Not recorded yet; wait for the specimen's normal paint.\n"),
    }
    body.push_str("Token alias chains and explicit override provenance: not provided.");
    body
}
/// Both panels retain their own selectable reader and Gallery-owned chooser.
struct PanelContent {
    host: ContentHost,
    state: SharedWorkbench,
    selection: Option<Selection>,
    inspector: bool,
    body: String,
}
impl ContainerContent for PanelContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        self.state.borrow_mut().sync_inspection();
        let selection = self.state.borrow().selection().clone();
        let selection_changed = self.selection.as_ref() != Some(&selection);
        if selection_changed {
            let snapshot = chooser_snapshot(&selection);
            self.host
                .replace_keyed_list(CHOOSER, &snapshot, &mut rows(&snapshot, 2000), None)
                .expect("registered specimen chooser");
            let key = selection
                .specimen
                .as_deref()
                .and_then(|key| {
                    specimens(&selection.page)
                        .iter()
                        .position(|item| item.key == key)
                })
                .map_or(1, |index| index as u64 + 2);
            self.host
                .set_keyed_list_selection(CHOOSER, Some(ListItemKey(key)))
                .expect("registered specimen selection");
        }
        let body = if self.inspector {
            inspector_body(&selection, self.state.borrow().selected_snapshot())
        } else if selection_changed {
            document_body(&selection)
        } else {
            self.body.clone()
        };
        // Do not invalidate layout on reads or erase selection on an unchanged
        // snapshot. Only an actual publication obtains a mutable component.
        if body != self.body {
            let field = self
                .host
                .get_mut(ComponentId(21))
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap();
            let baseline = field.surface.input.external_baseline();
            let revision = tekes_ui_controls::DraftRevision {
                identity: "gallery-panel-reader".into(),
                version: field
                    .surface
                    .input
                    .external_value()
                    .map_or(1, |value| value.version + 1),
                payload: body.clone(),
            };
            let result = field.surface.apply_external_value(
                &baseline,
                &revision,
                tekes_ui_controls::ExternalSelection::Reset,
                tekes_ui_controls::ExternalUndo::Reset,
            );
            assert!(
                matches!(
                    result,
                    tekes_ui_controls::ExternalValueResult::Applied { .. }
                ),
                "synchronous readonly reader update: {result:?}"
            );
            self.body = body;
        }
        self.selection = Some(selection);
        visit(&mut self.host);
    }
}
fn panel(
    id: &str,
    title: &str,
    state: SharedWorkbench,
    inspector: bool,
) -> anyhow::Result<LocalContainer> {
    let snapshot = chooser_snapshot(state.borrow().selection());
    let mut host = ContentHost::new(stack(
        20,
        vec![
            Content::new(CHOOSER, list("Inspect specimen", &snapshot, 2000)),
            Content::new(
                ComponentId(22),
                ScrollContent::new(
                    ScrollArea::default(),
                    Content::new(
                        ComponentId(21),
                        TextFieldContent::read_only_content(title, "", TextRole::UiBody),
                    ),
                ),
            ),
        ],
    ))
    .map_err(|error| anyhow::anyhow!("Gallery panel: {error:?}"))?;
    host.set_action_events(true);
    host.get_mut(ComponentId(20))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .flexible_child = Some(ComponentId(22));
    let actions = state.clone();
    Ok(LocalContainer::from_source(
        id,
        PanelContent {
            host,
            state,
            selection: None,
            inspector,
            body: String::new(),
        },
    )
    .title(title)
    .on_action(move |_, records| {
        for record in records {
            if record.action.target != CHOOSER {
                continue;
            }
            if let Some(ContentActionPayload::ListIntent(intent)) = record.payload {
                let key = match intent.kind {
                    ListIntentKind::Activated => Some(intent.key),
                    ListIntentKind::SelectionChanged => intent.selected_key,
                };
                if let Some(key) = key {
                    let mut state = actions.borrow_mut();
                    let specimen = key
                        .0
                        .checked_sub(2)
                        .and_then(|index| specimens(&state.selection().page).get(index as usize))
                        .map(|item| item.key);
                    state.select(specimen);
                }
            }
        }
    }))
}
pub fn assemble() -> anyhow::Result<AppWindow> {
    let page = std::env::var("TEKESUI_GALLERY_PAGE").unwrap_or_else(|_| SEARCH_ROUTE.into());
    let host = page_host(&page)?;
    let state = Rc::new(RefCell::new(WorkbenchState {
        current: 0,
        visits: vec![Selection::new(&page, None)],
        pages: HashMap::from([(page, Rc::new(RefCell::new(host)))]),
        pending: VecDeque::new(),
        inspected: None,
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
    let root = std::env::var_os("TEKESUI_APPHOST_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("tekesui-gallery-workbench"));
    let history_id = format!(
        "workbench-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    Ok(AppWindow::from_containers(
        vec![
            navigation(state.clone())?.into(),
            focus.into(),
            panel("gallery-documents", "Documents", state.clone(), false)?.into(),
            panel("gallery-inspector", "Inspector", state, true)?.into(),
        ],
        presentation,
    )?
    .navigation(NavigationOptions {
        directory: root.join(&history_id),
        history_id,
        initial_entries: vec![initial],
        view_command: Some(|entry| {
            anyhow::ensure!(
                workbench::entries()
                    .iter()
                    .any(|route| route.slug() == entry.route),
                "unknown Gallery route"
            );
            entry
                .params
                .get("visit")
                .and_then(serde_json::Value::as_u64)
                .and_then(|visit| COMMAND.checked_add(visit))
                .ok_or_else(|| anyhow::anyhow!("invalid Gallery history visit"))
        }),
    })
    .shared_settings(super::apphost::settings_owner()?))
}
pub fn run() -> anyhow::Result<()> {
    assemble()?.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::composition::ContentEvent;

    fn state(page: &str) -> SharedWorkbench {
        Rc::new(RefCell::new(WorkbenchState {
            current: 0,
            visits: vec![Selection::new(page, None)],
            pages: HashMap::from([(page.into(), Rc::new(RefCell::new(page_host(page).unwrap())))]),
            pending: VecDeque::new(),
            inspected: None,
        }))
    }
    fn render_text_host(host: &mut ContentHost, text: &mut super::super::text::TextEngine) {
        let frame = tekes_ui_scene::Rect::new(0., 0., 740., 900.);
        host.render(
            &mut tekes_ui_scene::Scene::new(),
            text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
    }
    fn publish(state: &SharedWorkbench, page: &str) {
        state.borrow_mut().navigate(page);
        let entry = state.borrow_mut().pending.pop_front().unwrap();
        let visit = entry.params["visit"].as_u64().unwrap();
        // LocalContainer invokes its command while borrowing the previous host.
        // The shell must not retain a borrow of the application model here.
        FocusContent(state.clone()).with_host(&mut |_| {
            state.borrow_mut().restore(COMMAND + visit).unwrap();
        });
    }
    #[test]
    fn apphost_workbench_history_retains_each_visit_and_live_input() {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let mut focus = FocusContent(state.clone());
        let mut text = super::super::text::TextEngine::new();
        focus.with_host(&mut |host| {
            // Formal focus/reveal admission needs the real laid-out text host.
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(100)), false));
            render_text_host(host, &mut text);
            assert_eq!(host.focused(), Some(ComponentId(100)));
            host.event_with_text(
                &ContentEvent::Commit {
                    text: " edited".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
            assert!(
                host.text_surface(ComponentId(100))
                    .unwrap()
                    .input
                    .text
                    .contains("edited")
            );
        });
        assert_eq!(state.borrow().selection().specimen.as_deref(), Some("name"));
        publish(&state, SEARCH_ROUTE);
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.search-field")
        );
        publish(&state, FORM_ROUTE);
        state.borrow_mut().select(Some("published"));
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.checkbox")
        );
        state.borrow_mut().restore(COMMAND).unwrap();
        assert_eq!(state.borrow().selection().specimen.as_deref(), Some("name"));
        focus.with_host(&mut |host| {
            assert!(
                host.text_surface(ComponentId(100))
                    .unwrap()
                    .input
                    .text
                    .contains("edited")
            )
        });
        state.borrow_mut().restore(COMMAND + 2).unwrap();
        assert_eq!(
            state.borrow().selection().specimen.as_deref(),
            Some("published")
        );
        // Choosing an inspection target never inserts a page visit.
        assert_eq!(state.borrow().visits.len(), 3);
        assert!(state.borrow().pending.is_empty());
    }
    #[test]
    fn apphost_workbench_selection_rejects_retired_specimens_and_visits() {
        let state = state(FORM_ROUTE);
        assert_eq!(state.borrow().selection().specimen, None);
        state.borrow_mut().select(Some("retired-field"));
        assert_eq!(state.borrow().selection().document_id, None);
        let before = state.borrow().selection().clone();
        assert!(state.borrow_mut().restore(COMMAND + 99).is_err());
        assert_eq!(*state.borrow().selection(), before);
        assert!(page_host("retired-route").is_err());
        for route in [SEARCH_ROUTE, FORM_ROUTE] {
            for specimen in specimens(route) {
                assert!(
                    workbench::entries()
                        .iter()
                        .any(|entry| entry.component() == specimen.document_id)
                );
            }
        }
    }
    #[test]
    fn apphost_workbench_filter_retains_registry_identity() {
        let all = list_snapshot("");
        assert_eq!(all.items.len(), workbench::entries().len());
        let filtered = list_snapshot("SEARCH-FIELD");
        assert!(!filtered.display_items.is_empty());
        for key in &filtered.display_items {
            let original = all.items.iter().find(|item| item.key == *key).unwrap();
            assert_eq!(
                filtered
                    .items
                    .iter()
                    .find(|item| item.key == *key)
                    .unwrap()
                    .name,
                original.name
            );
        }
        assert!(
            list_snapshot("no-such-gallery-component-291")
                .display_items
                .is_empty()
        );
    }
    #[test]
    fn apphost_workbench_manual_and_snapshot_follow_exact_instance() {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        state.borrow_mut().sync_inspection();
        let name = state.borrow().selected_snapshot().unwrap();
        let field_manual = document_body(state.borrow().selection());
        assert!(field_manual.contains("TextField"));
        assert_eq!(
            name.properties
                .as_ref()
                .unwrap()
                .iter()
                .find(|property| property.name == "text")
                .unwrap()
                .value,
            "Gallery"
        );
        state.borrow_mut().select(Some("owner"));
        state.borrow_mut().sync_inspection();
        let owner = state.borrow().selected_snapshot().unwrap();
        assert_eq!(document_body(state.borrow().selection()), field_manual);
        assert_ne!(name.id, owner.id);
        assert_ne!(name.mount, owner.mount);
        assert_eq!(
            owner
                .properties
                .as_ref()
                .unwrap()
                .iter()
                .find(|property| property.name == "text")
                .unwrap()
                .value,
            "TekesUI team"
        );
        assert_eq!(
            state.borrow().inspected,
            Some((FORM_ROUTE.into(), ComponentId(102), owner.mount))
        );
        state.borrow_mut().select(Some("published"));
        state.borrow_mut().sync_inspection();
        assert_eq!(
            state.borrow().selection().document_id.as_deref(),
            Some("tui.checkbox")
        );
        let checkbox_manual = document_body(state.borrow().selection());
        assert!(checkbox_manual.contains("Checkbox"));
        assert_ne!(checkbox_manual, field_manual);
        assert_eq!(
            state.borrow().selected_snapshot().unwrap().id,
            ComponentId(101)
        );
        state.borrow_mut().select(None);
        state.borrow_mut().sync_inspection();
        assert!(state.borrow().selected_snapshot().is_none());
        assert!(state.borrow().inspected.is_none());
        assert!(document_source("retired-component").is_none());
    }
    #[test]
    fn apphost_workbench_readers_preserve_selection_and_specimen_focus() {
        let state = state(FORM_ROUTE);
        state.borrow_mut().select(Some("name"));
        let unpainted = state.borrow().selected_snapshot().unwrap();
        assert!(unpainted.geometry.is_none());
        assert!(unpainted.last_paint.is_none());
        let body = inspector_body(state.borrow().selection(), Some(unpainted));
        assert!(body.contains("Current measured layout unavailable"));
        assert!(body.contains("Last paint (historical"));
        assert!(body.contains("Not recorded yet"));
        let mut text = super::super::text::TextEngine::new();
        FocusContent(state.clone()).with_host(&mut |host| {
            render_text_host(host, &mut text);
            assert!(host.focus(Some(ComponentId(100)), false));
            render_text_host(host, &mut text);
            assert_eq!(host.focused(), Some(ComponentId(100)));
        });
        let mut documents = panel("test-documents", "Documents", state.clone(), false).unwrap();
        let mut inspector = panel("test-inspector", "Inspector", state.clone(), true).unwrap();
        let mut version = 0;
        documents.with_host(&mut |host| {
            let field = host
                .get_mut(ComponentId(21))
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap();
            assert!(field.surface.input.read_only);
            field.surface.input.select_range(0..8);
            version = field.surface.input.external_value().unwrap().version;
        });
        for _ in 0..3 {
            inspector.with_host(&mut |_| {});
            documents.with_host(&mut |host| {
                let field = host
                    .get(ComponentId(21))
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap();
                assert_eq!(field.surface.input.selection(), 0..8);
                assert_eq!(
                    field.surface.input.external_value().unwrap().version,
                    version
                );
            });
        }
        let snapshot = state.borrow().selected_snapshot().unwrap();
        assert!(snapshot.focused);
        assert!(snapshot.geometry.is_some());
        assert!(snapshot.last_paint.is_some());
        let body = inspector_body(state.borrow().selection(), Some(snapshot));
        assert!(body.contains("Current measured layout (not declared sizing)"));
        assert!(body.contains("Last paint (historical"));
    }
    #[test]
    fn apphost_workbench_reenables_inspection_after_same_id_remount() {
        use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
        use tekes_ui_scene::{Rect, Scene, Size};
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, role: TextRole, width: f32) -> Size {
                Size::new(60_f32.min(width), role.style().line_height)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: tekes_ui_tokens::Rgba,
            ) {
            }
        }
        let state = state(FORM_ROUTE);
        // Keep only the selected original control so this regression needs no
        // native text resources. Replacements use the public host transaction.
        state.borrow_mut().pages.insert(
            FORM_ROUTE.into(),
            Rc::new(RefCell::new(
                ContentHost::new(stack(
                    200,
                    vec![Content::new(
                        ComponentId(101),
                        CheckboxContent::new("Published", CheckState::Unchecked),
                    )],
                ))
                .unwrap(),
            )),
        );
        state.borrow_mut().select(Some("published"));
        let render = |state: &SharedWorkbench| {
            state.borrow_mut().with_selected_host(&mut |host, _| {
                let frame = Rect::new(0., 0., 240., 100.);
                host.render(
                    &mut Scene::new(),
                    &mut Text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
            });
        };
        state.borrow_mut().sync_inspection();
        render(&state);
        let original = state.borrow().selected_snapshot().unwrap();
        assert!(original.last_paint.is_some());
        let selection = state.borrow().selection().clone();
        state
            .borrow_mut()
            .with_selected_host(&mut |host, specimen| {
                host.remove(specimen.component_id).unwrap();
                host.insert(
                    ComponentId(200),
                    Content::new(
                        specimen.component_id,
                        CheckboxContent::new("Replacement", CheckState::Checked),
                    ),
                )
                .unwrap();
            });
        render(&state);
        let replacement = state.borrow().selected_snapshot().unwrap();
        assert_ne!(replacement.mount, original.mount);
        assert!(replacement.last_paint.is_none());
        state.borrow_mut().sync_inspection();
        render(&state);
        assert!(
            state
                .borrow()
                .selected_snapshot()
                .unwrap()
                .last_paint
                .is_some()
        );
        assert_eq!(*state.borrow().selection(), selection);
        state
            .borrow_mut()
            .with_selected_host(&mut |host, specimen| {
                host.remove(specimen.component_id).unwrap();
            });
        state.borrow_mut().sync_inspection();
        assert!(state.borrow().inspected.is_none());
        assert!(state.borrow().selected_snapshot().is_none());
    }
}
