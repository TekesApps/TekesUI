//! Documents and Inspector panel hosts for the AppHost Gallery workbench.

use super::apphost_specimens::{Selection, specimens};
use super::apphost_workbench::{CHOOSER, SharedWorkbench, list, rows, stack, text};
use super::workbench;
use std::{cell::Cell, rc::Rc};
use tekes_ui_app_host::{ContainerContent, LocalContainer};
use tekes_ui_controls::{
    ScrollArea,
    composition::{
        ComponentId, Content, ContentActionPayload, ContentActionRecord, ContentHost,
        InspectionSnapshot, KeyValueTableContent, ListIntentKind, RichContent, RichDocument,
        ScrollContent, StackContent, TextContent, TextFieldContent,
    },
    extended::{KeyedListItem, KeyedListSnapshot, ListDatasetKey, ListItemKey},
};
use tekes_ui_tokens::proposal::TextRole;

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
const DOCUMENTS: &[(&str, &str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/component_documents.rs"));
const SUPPLEMENTAL_DOCUMENTS: &[(&str, &str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/supplemental_documents.rs"));
pub(super) const DOCUMENT_READER: ComponentId = ComponentId(21);
pub(super) const DOCUMENT_SCROLL: ComponentId = ComponentId(22);
pub(super) const DOCUMENT_NOTICE: ComponentId = ComponentId(23);
pub(super) const DOCUMENT_HELP: &str =
    "⌘-click a link to read a bundled manual. External links are not supported.";

pub(super) fn document_source(id: &str) -> Option<&'static str> {
    DOCUMENTS
        .iter()
        .find(|(key, _, _)| *key == id)
        .map(|(_, _, source)| *source)
}

pub(super) fn document_body(selection: &Selection) -> String {
    match selection.document_id.as_deref() {
        Some(id) => match document_source(id) {
            Some(source) => source.into(),
            None => format!("Component manual not provided: {id}"),
        },
        None if workbench::apphost_quality::category_for_slug(&selection.page).is_some() => {
            let category = workbench::apphost_quality::category_for_slug(&selection.page).unwrap();
            let group = super::catalog::group(category);
            format!(
                "Page: {}\nOriginal cross-component state illustrations. Selecting an illustrated state is not live control acceptance.\nComponents: {}\nChoose a component to read its bundled manual.",
                group.name,
                group
                    .component_ids
                    .iter()
                    .map(|id| super::catalog::component(id).name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        None => format!(
            "Page: {}\n{}\n{}",
            selection.page,
            workbench::entries()
                .iter()
                .find(|entry| entry.slug() == selection.page)
                .map_or("", workbench::story_docs),
            if selection.specimen.is_some() {
                "This specimen has page guidance rather than a registered component manual."
            } else {
                "Select an available specimen to inspect its live instance and component manual."
            }
        ),
    }
}
pub(super) fn document_key(selection: &Selection) -> String {
    selection
        .document_id
        .clone()
        .unwrap_or_else(|| format!("page:{}", selection.page))
}

pub(super) fn document_scroll(key: &str, source: &str) -> Content {
    Content::new(
        DOCUMENT_SCROLL,
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                DOCUMENT_READER,
                RichContent::new(key, 1, RichDocument::from_markdown(source))
                    .expect("nonempty Gallery document identity"),
            ),
        ),
    )
}

/// Same document publications preserve the original text owner and selection.
/// Different documents retire the complete reader through the Host lifecycle.
pub(super) fn publish_document(
    host: &mut ContentHost,
    key: &str,
    source: &str,
    pending_focus: &Cell<Option<u64>>,
) {
    let current = host
        .get(DOCUMENT_READER)
        .unwrap()
        .component::<RichContent>()
        .unwrap();
    let expected = current.version();
    if expected.identity() == key {
        if current.document().source() == source {
            return;
        }
        host.get_mut(DOCUMENT_READER)
            .unwrap()
            .component_mut::<RichContent>()
            .unwrap()
            .update(
                &expected,
                expected.revision() + 1,
                RichDocument::from_markdown(source),
            )
            .expect("synchronous Gallery document publication");
    } else {
        let replacement = document_scroll(key, source);
        let focused = host.focused() == Some(DOCUMENT_READER);
        host.remove(DOCUMENT_SCROLL)
            .expect("mounted Gallery document scroll");
        host.insert(ComponentId(20), replacement)
            .expect("Gallery document slot");
        pending_focus.set(if focused {
            host.semantic_mount(DOCUMENT_READER)
        } else {
            None
        });
    }
}

/// A replacement has no focusable geometry until the original Host arranges it.
/// Retain only that mount's one pending transfer, never a second focus owner.
fn finish_document_focus(host: &mut ContentHost, pending_focus: &Cell<Option<u64>>) {
    let Some(mount) = pending_focus.get() else {
        return;
    };
    if host.semantic_mount(DOCUMENT_READER) != Some(mount)
        || host.focused().is_some_and(|id| id != DOCUMENT_READER)
    {
        pending_focus.set(None);
        return;
    }
    if host
        .semantic_text_layout_snapshot(DOCUMENT_READER, None)
        .is_none()
    {
        return;
    }
    let restored =
        host.focus(Some(DOCUMENT_READER), false) || host.focused() == Some(DOCUMENT_READER);
    pending_focus.set(None);
    if !restored {
        document_notice(host, "Click the document to continue keyboard reading.");
    }
}
/// Resolve only bundled paths relative to the captured source manual. Never read
/// the filesystem or treat an arbitrary target as a command/URL to execute.
pub(super) fn linked_document(
    document: &str,
    target: &str,
) -> Option<(&'static str, &'static str)> {
    let manuals = || DOCUMENTS.iter().chain(SUPPLEMENTAL_DOCUMENTS);
    let (_, path, _) = manuals().find(|(id, _, _)| *id == document)?;
    let joined = std::path::Path::new(path).parent()?.join(target);
    let mut resolved = std::path::PathBuf::new();
    for part in joined.components() {
        match part {
            std::path::Component::Normal(part) => resolved.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            _ => return None,
        }
    }
    manuals()
        .find(|(_, path, _)| std::path::Path::new(path) == resolved.as_path())
        .map(|(id, _, source)| (*id, *source))
}

fn document_notice(host: &mut ContentHost, message: &str) {
    if host
        .get(DOCUMENT_NOTICE)
        .unwrap()
        .component::<TextContent>()
        .unwrap()
        .text
        != message
    {
        host.get_mut(DOCUMENT_NOTICE)
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = message.into();
    }
}

/// Called only by the existing post-input action callback, even when the ordinary
/// action queue is empty. Rich links intentionally have their own owned queue.
pub(super) fn consume_document_links(host: &mut ContentHost, pending_focus: &Cell<Option<u64>>) {
    let links = host
        .get_mut(DOCUMENT_READER)
        .unwrap()
        .component_mut::<RichContent>()
        .unwrap()
        .drain_link_actions();
    for link in links {
        if let Some((key, source)) = linked_document(&link.document, &link.target) {
            publish_document(host, key, source, pending_focus);
            document_notice(host, DOCUMENT_HELP);
        } else {
            document_notice(
                host,
                &format!(
                    "Link not opened: {} — only bundled manual links are supported.",
                    link.target
                ),
            );
        }
    }
}
/// Presentation data only: all values come from the selected mount's snapshot.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct InspectorGroup {
    title: &'static str,
    rows: Vec<(String, String)>,
}
impl InspectorGroup {
    fn new(title: &'static str) -> Self {
        Self {
            title,
            rows: Vec::new(),
        }
    }
    fn row(&mut self, label: impl Into<String>, value: impl ToString) {
        self.rows.push((label.into(), value.to_string()));
    }
}
pub(super) fn inspector_body(
    selection: &Selection,
    snapshot: Option<InspectionSnapshot>,
) -> Vec<InspectorGroup> {
    let mut identity = InspectorGroup::new("Identity");
    identity.row("Page", &selection.page);
    identity.row("Specimen", selection.specimen.as_deref().unwrap_or("none"));
    identity.row(
        "Document",
        selection.document_id.as_deref().unwrap_or("page guidance"),
    );
    let Some(snapshot) = snapshot else {
        identity.row("Live snapshot", "Unavailable. Select a mounted specimen.");
        return vec![identity];
    };
    identity.row("Current instance", format!("{:?}", snapshot.id));
    identity.row("Mount", snapshot.mount);
    identity.row("Current semantics", format!("{:?}", snapshot.semantics));
    identity.row("Specimen focus", snapshot.focused);
    let mut properties = InspectorGroup::new("Properties");
    match snapshot.properties {
        Some(values) => {
            properties.row("Coverage", "Reported public properties only");
            for property in values {
                properties.row(property.name, property.value);
            }
        }
        None => properties.row("Coverage", "Not provided by this component."),
    }
    let mut layout = InspectorGroup::new("Layout");
    layout.row("Sizing", "Measured bounds, not declared sizing");
    // Keep the row schema stable across ordinary layout invalidation. A click
    // must still address the same text owner while the next paint is pending.
    let geometry = snapshot.geometry.as_ref();
    for (label, value) in [
        ("Frame", geometry.map(|g| format!("{:?}", g.frame))),
        (
            "Rectangular clip",
            geometry.map(|g| format!("{:?}", g.rectangular_clip)),
        ),
        (
            "Visible rectangle",
            geometry.map(|g| format!("{:?}", g.visible)),
        ),
        (
            "Additional curve geometry",
            geometry.map(|g| g.has_curve_geometry.to_string()),
        ),
    ] {
        layout.row(label, value.unwrap_or_else(|| "Unavailable".into()));
    }
    layout.row(
        "Current measured layout",
        if geometry.is_some() {
            "Available"
        } else {
            "Unavailable (not rendered or invalidated)."
        },
    );
    let mut tokens = InspectorGroup::new("Observed tokens");
    tokens.row(
        "Evidence",
        "Last paint (historical; not recomputed from current properties)",
    );
    tokens.row(
        "Limits",
        "Lookup calls do not prove every rendered pixel. Text and images are not sampled.",
    );
    tokens.row("Alias / override provenance", "Not provided");
    let mut quads = InspectorGroup::new("Emitted primitives");
    quads.row("Evidence", "Last paint, before GPU composition; quads only");
    match snapshot.last_paint {
        Some(paint) => {
            tokens.row("Theme", format!("{:?}", paint.theme));
            tokens.row("Includes owned children", paint.includes_owned_children);
            tokens.row("Lookup count", paint.tokens.lookups.len());
            for (family, name) in paint.tokens.lookups {
                tokens.row(format!("Lookup · {family}"), name);
            }
            tokens.row("Resolved color count", paint.tokens.colors.len());
            for color in paint.tokens.colors {
                tokens.row(
                    color.role,
                    format!("{:?}\nPalette: {:?}", color.value, color.palette),
                );
            }
            quads.row("Quad count", paint.quads.len());
            for (index, quad) in paint.quads.iter().enumerate() {
                quads.row(format!("Quad {}", index + 1), format!("{quad:#?}"));
            }
        }
        None => {
            tokens.row(
                "Recording",
                "Not recorded yet; wait for the specimen's normal paint.",
            );
            quads.row("Recording", "Not recorded yet");
        }
    }
    vec![identity, properties, layout, tokens, quads]
}

/// Shared key/value table owns aligned columns, padded rows and separators. The existing
/// ScrollContent owns overflow. Values use the shared selectable read-only text
/// owner; focusing them does not focus the specimen. IDs are panel-local.
fn inspector_sections(groups: &[InspectorGroup], mut next_id: u64) -> Vec<Content> {
    let mut sections = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let heading = Content::new(
            ComponentId(10_000 + index as u64 * 2),
            TextContent {
                text: group.title.into(),
                role: TextRole::UiHeading,
            },
        );
        let table_id = ComponentId(10_001 + index as u64 * 2);
        let mut rows = Vec::new();
        for (label, value) in &group.rows {
            rows.push((
                text(next_id, label),
                Content::new(
                    ComponentId(next_id + 1),
                    TextFieldContent::read_only_content(label, value, TextRole::UiBody),
                ),
            ));
            next_id += 2;
        }
        sections.push(heading);
        sections.push(Content::new(table_id, KeyValueTableContent::new(rows)));
    }
    sections
}
/// Both panels retain their own content and Gallery-owned chooser.
pub(super) struct PanelContent {
    pub(super) host: ContentHost,
    pub(super) state: SharedWorkbench,
    pub(super) selection: Option<Selection>,
    pub(super) inspector: bool,
    pub(super) inspector_groups: Vec<InspectorGroup>,
    pub(super) inspector_target: Option<(ComponentId, u64)>,
    pub(super) inspector_snapshot: Option<InspectionSnapshot>,
    pub(super) document_focus: Rc<Cell<Option<u64>>>,
}
impl PanelContent {
    fn publish(&mut self) {
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
        if self.inspector {
            let snapshot = self.state.borrow().selected_snapshot();
            let target = snapshot
                .as_ref()
                .map(|snapshot| (snapshot.id, snapshot.mount));
            if selection_changed
                || target != self.inspector_target
                || self.inspector_groups.is_empty()
                || snapshot != self.inspector_snapshot
            {
                self.inspector_snapshot = snapshot.clone();
                let groups = inspector_body(&selection, snapshot);
                if groups != self.inspector_groups {
                    if selection_changed || target != self.inspector_target {
                        self.host
                            .remove(DOCUMENT_READER)
                            .expect("mounted Inspector groups");
                        self.host
                            .insert(
                                DOCUMENT_SCROLL,
                                stack(DOCUMENT_READER.0, inspector_sections(&groups, 10_010)),
                            )
                            .expect("Inspector scroll content");
                    } else {
                        // AppHost cancels other panels before a click, invalidating
                        // specimen geometry until its next paint. Keep unchanged
                        // groups mounted through that transition, including the
                        // value receiving this click and its native/AX text owner.
                        let next_id = self
                            .host
                            .get(DOCUMENT_READER)
                            .unwrap()
                            .component::<StackContent>()
                            .unwrap()
                            .children
                            .iter()
                            .filter_map(|node| node.component::<KeyValueTableContent>())
                            .flat_map(|table| table.cells().iter().map(|cell| cell.id().0))
                            .max()
                            .map_or(10_010, |id| id + 1);
                        for (index, section) in
                            inspector_sections(&groups, next_id).into_iter().enumerate()
                        {
                            let group = index / 2;
                            if self.inspector_groups.get(group) == groups.get(group) {
                                continue;
                            }
                            if self.inspector_groups[group]
                                .rows
                                .iter()
                                .map(|(label, _)| label)
                                .eq(groups[group].rows.iter().map(|(label, _)| label))
                            {
                                if index % 2 == 1 {
                                    let values: Vec<_> = self
                                        .host
                                        .get(section.id())
                                        .unwrap()
                                        .component::<KeyValueTableContent>()
                                        .unwrap()
                                        .cells()
                                        .chunks_exact(2)
                                        .zip(&groups[group].rows)
                                        .map(|(cells, (_, value))| (cells[1].id(), value.clone()))
                                        .collect();
                                    for (id, value) in values {
                                        if self.host.text_surface(id).unwrap().text() == value {
                                            continue;
                                        }
                                        let field = self
                                            .host
                                            .get_mut(id)
                                            .unwrap()
                                            .component_mut::<TextFieldContent>()
                                            .unwrap();
                                        let baseline = field.surface.input().external_baseline();
                                        let revision = tekes_ui_controls::DraftRevision {
                                            identity: "gallery-inspector-value".into(),
                                            version: field
                                                .surface
                                                .input()
                                                .external_value()
                                                .map_or(1, |v| v.version + 1),
                                            payload: value,
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
                                            "synchronous Inspector value publication: {result:?}"
                                        );
                                    }
                                }
                                continue;
                            }
                            self.host
                                .remove(section.id())
                                .expect("mounted Inspector section");
                            self.host
                                .insert(DOCUMENT_READER, section)
                                .expect("Inspector section");
                        }
                        let order: Vec<_> = (0..groups.len() * 2)
                            .map(|index| ComponentId(10_000 + index as u64))
                            .collect();
                        self.host
                            .reorder(DOCUMENT_READER, &order)
                            .expect("Inspector section order");
                    }
                    self.inspector_groups = groups;
                }
            }
            self.inspector_target = target;
        } else {
            // Only an explicit chooser activation requests a return after link browsing.
            // Ordinary reads and select(same) keep the linked reader intact.
            let return_requested =
                std::mem::take(&mut self.state.borrow_mut().document_return_requested);
            if selection_changed || return_requested {
                publish_document(
                    &mut self.host,
                    &document_key(&selection),
                    &document_body(&selection),
                    &self.document_focus,
                );
                document_notice(&mut self.host, DOCUMENT_HELP);
            }
        }
        self.selection = Some(selection);
    }
}
impl ContainerContent for PanelContent {
    fn prepare_for_render(&mut self) {
        if self.inspector {
            self.publish();
        }
    }
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        if !self.inspector {
            self.publish();
        }
        visit(&mut self.host);
        if !self.inspector {
            finish_document_focus(&mut self.host, &self.document_focus);
        }
    }
}
pub(super) fn consume_chooser_actions(state: &SharedWorkbench, records: Vec<ContentActionRecord>) {
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
                let mut state = state.borrow_mut();
                let specimen = key
                    .0
                    .checked_sub(2)
                    .and_then(|index| specimens(&state.selection().page).get(index as usize))
                    .map(|item| item.key);
                state.select(specimen);
                if intent.kind == ListIntentKind::Activated {
                    state.document_return_requested = true;
                }
            }
        }
    }
}

pub(super) fn panel_source(
    state: SharedWorkbench,
    inspector: bool,
) -> anyhow::Result<PanelContent> {
    let snapshot = chooser_snapshot(state.borrow().selection());
    let mut host = ContentHost::new(stack(
        20,
        vec![
            Content::new(CHOOSER, list("Inspect specimen", &snapshot, 2000)),
            if inspector {
                Content::new(
                    DOCUMENT_SCROLL,
                    ScrollContent::new(ScrollArea::default(), stack(DOCUMENT_READER.0, Vec::new()))
                        .scrollbar(true),
                )
            } else {
                let selection = state.borrow().selection().clone();
                document_scroll(&document_key(&selection), &document_body(&selection))
            },
        ],
    ))
    .map_err(|error| anyhow::anyhow!("Gallery panel: {error:?}"))?;
    if !inspector {
        host.insert(
            ComponentId(20),
            Content::new(DOCUMENT_NOTICE, TextContent::new(DOCUMENT_HELP)),
        )
        .expect("Gallery document help");
        host.reorder(
            ComponentId(20),
            &[CHOOSER, DOCUMENT_NOTICE, DOCUMENT_SCROLL],
        )
        .expect("Gallery document order");
    }
    host.set_action_events(true);
    host.get_mut(ComponentId(20))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .flexible_child = Some(ComponentId(22));
    Ok(PanelContent {
        host,
        state,
        selection: None,
        inspector,
        inspector_groups: Vec::new(),
        inspector_target: None,
        inspector_snapshot: None,
        document_focus: Rc::new(Cell::new(None)),
    })
}
pub(super) fn panel(
    id: &str,
    title: &str,
    state: SharedWorkbench,
    inspector: bool,
) -> anyhow::Result<LocalContainer> {
    let source = panel_source(state.clone(), inspector)?;
    let document_focus = source.document_focus.clone();
    Ok(LocalContainer::from_source(id, source)
        .title(title)
        .on_action(move |host, records| {
            if !inspector {
                consume_document_links(host, &document_focus);
            }
            consume_chooser_actions(&state, records);
        }))
}
