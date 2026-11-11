//! Local-only AppHost workbench: real original controls and a small shared
//! document model. Presentation moves never rebuild the owning ContentHosts.
use super::{Page, workbench};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_app_host::{
    AppWindow, ContainerContent, ContainerToolbarHost, ContainerToolbarPosition, LocalContainer,
    NavigationOptions, OthersGroup, OthersOptions, PresentationOptions, SettingsDisplayMode,
    SettingsOwner, SettingsWindow,
};
use tekes_ui_app_protocol::ToolbarItem;
use tekes_ui_controls::{
    CheckState, ScrollArea, TextInput,
    composition::{
        CheckboxContent, Component, ComponentId, Content, ContentActionPayload,
        ContentActionRecord, ContentHost, DecimalNumberFormat, DisclosureContent, ListContent,
        NumberFieldContent, ScrollContent, SelectContent, SelectableListContent, SliderContent,
        StackContent, TextContent, TextFieldContent, TextSubmitPolicy,
    },
    extended::{Axis, Slider, Stack},
};
use tekes_ui_platform_macos::{Size as WindowSize, Window, WindowConfig, WindowHandle};
use tekes_ui_tokens::{Spacing, Theme, proposal::TextRole};
const DOCUMENT_TITLES: [&str; 4] = [
    "A quieter workspace",
    "Notes on responsive reading",
    "Project overview",
    "Release checklist",
];
const SWITCH: u64 = 1;
const TITLE: ComponentId = ComponentId(100);
const AUTHOR: ComponentId = ComponentId(101);
const REVISION: ComponentId = ComponentId(102);
const PUBLISHED: ComponentId = ComponentId(103);
const STATUS: ComponentId = ComponentId(104);
const PRIORITY: ComponentId = ComponentId(105);
const STATUSES: [&str; 3] = ["Draft", "Review", "Published"];

pub fn is_page(page: Page) -> bool {
    matches!(page, Page::Workbench(index) if workbench::entries()[index].slug() == "apphost")
}
#[derive(Clone)]
struct Document {
    title: String,
    author: String,
    revision: f64,
    published: bool,
    status: usize,
    priority: f64,
    paragraphs: Vec<String>,
}
struct Documents {
    selected: usize,
    items: Vec<Document>,
}
struct SharedContent(Rc<RefCell<ContentHost>>);
impl ContainerContent for SharedContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        visit(&mut self.0.borrow_mut());
    }
}
struct NavigationContent {
    host: Rc<RefCell<ContentHost>>,
    pending: Rc<RefCell<VecDeque<tekes_ui_app_host::navigation::Entry>>>,
}
impl ContainerContent for NavigationContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        visit(&mut self.host.borrow_mut());
    }
    fn take_navigation_request(&mut self) -> Option<tekes_ui_app_host::navigation::Entry> {
        self.pending.borrow_mut().pop_front()
    }
}
const DOCUMENT_COMMAND: u64 = 0x4744_0000;
fn entry(index: usize) -> tekes_ui_app_host::navigation::Entry {
    tekes_ui_app_host::navigation::Entry {
        id: DOCUMENT_TITLES[index].into(),
        route: "gallery-document".into(),
        params: serde_json::json!({"index":index}),
    }
}
fn update_navigation(host: &mut ContentHost, documents: &Documents) {
    let selected = documents.selected;
    for (id, base) in [(ComponentId(8004), 0), (ComponentId(8006), 2)] {
        host.get_mut(id)
            .unwrap()
            .component_mut::<SelectableListContent>()
            .unwrap()
            .labels = documents.items[base..base + 2]
            .iter()
            .map(|item| item.title.clone())
            .collect();
        host.get_mut(id)
            .unwrap()
            .component_mut::<SelectableListContent>()
            .unwrap()
            .model
            .selected = if selected / 2 == base / 2 {
            Some(selected % 2)
        } else {
            None
        };
    }
}
fn data_root() -> anyhow::Result<std::path::PathBuf> {
    let root = if let Some(root) = std::env::var_os("TEKESUI_APPHOST_DATA_DIR") {
        std::path::PathBuf::from(root)
    } else {
        std::path::PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is unavailable"))?,
        )
        .join("Library/Application Support/TekesUI Gallery/AppHost")
    };
    Ok(root)
}
/// Consumer-owned definition and storage; the shared owner is held by Gallery.
/// General preferences apply to all AppHosts sharing this owner and Settings.
pub fn settings_owner() -> anyhow::Result<SettingsOwner> {
    settings_owner_in(data_root()?)
}
fn settings_owner_in(root: std::path::PathBuf) -> anyhow::Result<SettingsOwner> {
    SettingsOwner::new(
        SettingsWindow::from_toml(
            include_str!("apphost.settings.toml"),
            root.join("settings/general/settings.toml"),
        )?
        .display_mode(SettingsDisplayMode::Exclusive),
    )?
    .with_display_preferences(
        "general/appearance/appearance",
        "general/appearance/text-size",
    )
}
fn history_options() -> anyhow::Result<NavigationOptions> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let root = data_root()?;
    let history_id = format!(
        "gallery-window-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    Ok(NavigationOptions {
        directory: root.join(&history_id),
        history_id,
        initial_entries: vec![entry(0)],
        view_command: Some(|entry| {
            anyhow::ensure!(entry.route == "gallery-document", "unknown Gallery route");
            let index = entry
                .params
                .get("index")
                .and_then(serde_json::Value::as_u64)
                .filter(|index| *index < 4)
                .ok_or_else(|| anyhow::anyhow!("unknown Gallery document"))?;
            Ok(DOCUMENT_COMMAND + index)
        }),
    })
}
fn text(id: u64, value: impl Into<String>, role: TextRole) -> Content {
    let mut text = TextContent::new(value);
    text.role = role;
    Content::new(ComponentId(id), text)
}
fn stack(id: u64, children: Vec<Content>) -> Content {
    Content::new(
        ComponentId(id),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Relaxed.value(),
            },
            children,
            flexible_child: None,
        },
    )
}
fn scroll(id: u64, child: Content) -> Content {
    Content::new(
        ComponentId(id),
        ScrollContent::new(ScrollArea::default(), child).content_padding(Spacing::Loose.value()),
    )
}
fn documents() -> Documents {
    let paragraphs: Vec<String> = vec![
        "The studio begins with a simple idea: make room for the work that matters. A document sits at the center, with a small collection of references and practical tools nearby. On a quiet morning, a clear title and a few thoughtful paragraphs can be enough to begin a useful conversation.".into(),
        "Our first milestone brings the research notes together. We will compare the proposals, invite comments from the people who use the workspace every day, and agree on a short list of improvements. Each revision should explain what changed and why, so the next reader can follow the decisions without retracing every discussion.".into(),
        "项目的下一阶段从一次小范围试用开始。我们会记录参与者遇到的问题，整理清楚的建议，再决定下一轮优先完成哪些工作。好的说明文字应当在宽屏上舒展，在较窄的窗口里自然换行，让读者始终能够轻松找到上下文。".into(),
        "Before the end of the week, we will prepare a short handover: a summary of the completed work, the decisions still open, and the next meeting date. The final paragraph is a reminder to leave the workspace ready for the next person, with useful notes and a clear place to continue.".into(),
    ];
    Documents {
        selected: 0,
        items: DOCUMENT_TITLES
            .into_iter()
            .enumerate()
            .map(|(index, title)| Document {
                title: title.into(),
                author: "TekesUI team".into(),
                revision: (index + 1) as f64,
                published: false,
                status: 0,
                priority: 50.,
                paragraphs: {
                    let mut body = paragraphs.clone();
                    body[0] = format!("{} — {}", title, body[0]);
                    body
                },
            })
            .collect(),
    }
}
fn document_content(document: &Document) -> Content {
    let mut children = vec![
        text(1, &document.title, TextRole::UiHeading),
        text(2, summary(document), TextRole::UiCaption),
    ];
    children.push(Content::new(
        ComponentId(10),
        TextFieldContent::read_only_content(
            "Document body",
            &document.paragraphs.join("\n\n"),
            TextRole::UiBody,
        ),
    ));
    scroll(9000, stack(9001, children))
}
fn summary(document: &Document) -> String {
    format!(
        "{} · revision {} · {} · publication {} · priority {}",
        document.author,
        document.revision,
        STATUSES[document.status],
        if document.published { "on" } else { "off" },
        document.priority
    )
}
fn update_document(host: &mut ContentHost, document: &Document) {
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .text
        .clone_from(&document.title);
    host.get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .text = summary(document);
    let value = document.paragraphs.join("\n\n");
    let field = host
        .get_mut(ComponentId(10))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    // Inspector updates that leave the body unchanged must retain selection.
    if field.surface.input.text != value {
        let baseline = field.surface.input.external_baseline();
        let revision = tekes_ui_controls::DraftRevision {
            identity: "gallery-apphost-document-body".into(),
            version: field
                .surface
                .input
                .external_value()
                .map_or(1, |current| current.version + 1),
            payload: value,
        };
        let result = field.surface.apply_external_value(
            &baseline,
            &revision,
            tekes_ui_controls::ExternalSelection::Set(0..0),
            tekes_ui_controls::ExternalUndo::Reset,
        );
        assert!(
            matches!(
                result,
                tekes_ui_controls::ExternalValueResult::Applied { .. }
            ),
            "the synchronous Gallery body update must be accepted: {result:?}"
        );
    }
}
fn field(id: ComponentId, label: &str, value: &str) -> Content {
    let mut field = TextFieldContent::new(label, value);
    field.submit_policy = TextSubmitPolicy::Enter;
    Content::new(id, field)
}
fn inspector_content(document: &Document) -> Content {
    let mut status =
        SelectContent::new("Status", STATUSES.iter().map(|v| (*v).to_owned()).collect());
    status.set_selected(document.status);
    let properties = stack(
        7001,
        vec![
            field(TITLE, "Document title", &document.title),
            field(AUTHOR, "Author", &document.author),
            Content::new(
                REVISION,
                NumberFieldContent::new(
                    "Revision",
                    document.revision,
                    1.,
                    100.,
                    1.,
                    DecimalNumberFormat::default(),
                )
                .expect("finite fixture revision"),
            ),
            Content::new(
                PUBLISHED,
                CheckboxContent::new("Published", CheckState::Unchecked),
            ),
            Content::new(STATUS, status),
            text(106, "Priority", TextRole::UiBody),
            Content::new(
                PRIORITY,
                SliderContent::new(
                    "Priority",
                    Slider::new(document.priority, 0., 100., 1.).expect("finite fixture priority"),
                ),
            ),
        ],
    );
    let disclosure = Content::new(
        ComponentId(7002),
        DisclosureContent::new(
            ComponentId(7003),
            text(7004, "Document properties", TextRole::UiHeading),
            properties,
            true,
        ),
    );
    Content::new(
        ComponentId(7000),
        ListContent::scrollable(
            "Document properties",
            ComponentId(7005),
            vec![
                text(7006, "Inspector", TextRole::UiHeading),
                text(
                    7007,
                    "Changes apply to the document selected in navigation.",
                    TextRole::UiCaption,
                ),
                disclosure,
            ],
        )
        .content_padding(Spacing::Loose.value()),
    )
}
fn update_inspector(host: &mut ContentHost, document: &Document) {
    host.cancel();
    host.focus(None, false);
    for (id, value) in [(TITLE, &document.title), (AUTHOR, &document.author)] {
        host.get_mut(id)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input = TextInput::new(value, false);
    }
    host.get_mut(REVISION)
        .unwrap()
        .component_mut::<NumberFieldContent>()
        .unwrap()
        .set_value(document.revision)
        .expect("fixture revision stays within range");
    host.get_mut(PUBLISHED)
        .unwrap()
        .component_mut::<CheckboxContent>()
        .unwrap()
        .set_checked(if document.published {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        });
    host.get_mut(STATUS)
        .unwrap()
        .component_mut::<SelectContent>()
        .unwrap()
        .set_selected(document.status);
    host.get_mut(PRIORITY)
        .unwrap()
        .component_mut::<SliderContent>()
        .unwrap()
        .model = Slider::new(document.priority, 0., 100., 1.).unwrap();
}
fn confirm_inspector_disclosures(host: &mut ContentHost, records: &[ContentActionRecord]) -> bool {
    let mut property_change = false;
    for record in records {
        if let Some(ContentActionPayload::DisclosureExpansion(intent)) = &record.payload {
            host.get_mut(ComponentId(7002))
                .unwrap()
                .component_mut::<DisclosureContent>()
                .unwrap()
                .confirm(intent, true);
        } else if [TITLE, AUTHOR, REVISION, PUBLISHED, STATUS, PRIORITY]
            .contains(&record.action.target)
        {
            property_change = true;
        }
    }
    property_change
}
fn switch_toolbar() -> Vec<ToolbarItem> {
    vec![ToolbarItem {
        id: SWITCH,
        title: "Switch".into(),
        enabled: true,
        geometry_dependent: false,
    }]
}
fn assemble(theme: Theme, settings: SettingsOwner) -> anyhow::Result<AppWindow> {
    let documents = Rc::new(RefCell::new(documents()));
    let document = Rc::new(RefCell::new(
        ContentHost::new(document_content(&documents.borrow().items[0]))
            .map_err(|e| anyhow::anyhow!("document: {e:?}"))?,
    ));
    let inspector = Rc::new(RefCell::new(
        ContentHost::new(inspector_content(&documents.borrow().items[0]))
            .map_err(|e| anyhow::anyhow!("inspector: {e:?}"))?,
    ));
    inspector.borrow_mut().set_action_events(true);
    let mut writing = SelectableListContent::new(
        "Writing",
        documents.borrow().items[..2]
            .iter()
            .map(|d| d.title.clone())
            .collect(),
    );
    writing.model.selected = Some(0);
    writing.view.set_navigation_selection_fill_only(true);
    writing.view.set_navigation_layout(true);
    let mut resources = SelectableListContent::new(
        "Project",
        documents.borrow().items[2..]
            .iter()
            .map(|d| d.title.clone())
            .collect(),
    );
    resources.view.set_navigation_selection_fill_only(true);
    resources.view.set_navigation_layout(true);
    let navigation = Content::new(
        ComponentId(8000),
        ListContent::scrollable(
            "Workspace",
            ComponentId(8001),
            vec![
                text(8002, "Workspace", TextRole::UiHeading),
                Content::new(
                    ComponentId(8007),
                    StackContent {
                        model: Stack {
                            axis: Axis::Vertical,
                            gap: Spacing::Micro.value(),
                        },
                        flexible_child: None,
                        children: vec![
                            Content::new(
                                ComponentId(8008),
                                StackContent {
                                    model: Stack {
                                        axis: Axis::Vertical,
                                        gap: Spacing::Regular.value(),
                                    },
                                    flexible_child: None,
                                    children: vec![
                                        text(8003, "WRITING", TextRole::UiCaption),
                                        Content::new(ComponentId(8004), writing),
                                    ],
                                },
                            ),
                            Content::new(
                                ComponentId(8009),
                                StackContent {
                                    model: Stack {
                                        axis: Axis::Vertical,
                                        gap: Spacing::Regular.value(),
                                    },
                                    flexible_child: None,
                                    children: vec![
                                        text(8005, "PROJECT", TextRole::UiCaption),
                                        Content::new(ComponentId(8006), resources),
                                    ],
                                },
                            ),
                        ],
                    },
                ),
            ],
        )
        .content_padding(Spacing::Loose.value()),
    );
    let navigation = Rc::new(RefCell::new(
        ContentHost::new(navigation).map_err(|error| anyhow::anyhow!("navigation: {error:?}"))?,
    ));
    navigation.borrow_mut().set_action_events(true);
    let pending = Rc::new(RefCell::new(VecDeque::new()));
    let nav_documents = documents.clone();
    let nav_inspector = inspector.clone();
    let nav_pending = pending.clone();
    let nav = LocalContainer::from_source(
        "gallery-nav",
        NavigationContent {
            host: navigation.clone(),
            pending,
        },
    )
    .content_padding(0.)
    .on_action(move |host, records| {
        for record in records {
            let (id, offset) = if record.action.target == ComponentId(8004) {
                (ComponentId(8004), 0)
            } else if record.action.target == ComponentId(8006) {
                (ComponentId(8006), 2)
            } else {
                continue;
            };
            let Some(selected) = host
                .get(id)
                .and_then(|content| content.component::<SelectableListContent>())
                .and_then(|list| list.model.selected)
            else {
                continue;
            };
            if nav_inspector
                .borrow()
                .focused_text()
                .is_some_and(|surface| surface.input.marked.is_some())
            {
                update_navigation(host, &nav_documents.borrow());
                eprintln!("GALLERY_APPHOST_NAV_REJECTED composition-in-progress");
                continue;
            }
            nav_pending.borrow_mut().push_back(entry(selected + offset));
            eprintln!("GALLERY_APPHOST_NAV_REQUEST selected={}", selected + offset);
        }
    });
    let focus_documents = documents.clone();
    let focus_inspector = inspector.clone();
    let inspector_navigation = navigation.clone();
    let focus_navigation = navigation;
    let inspector_documents = documents;
    let inspector_document = document.clone();
    let inspector = LocalContainer::from_source("gallery-detail", SharedContent(inspector))
        .title("Inspector")
        .toolbar_host(ContainerToolbarHost::Container {
            position: ContainerToolbarPosition::Top,
        })
        .switcher_icon("tekes-side-panel")
        .content_padding(0.)
        .list_height(640.)
        .toolbar(switch_toolbar())
        .focus_switch(SWITCH)
        .on_action(move |host, records| {
            if !confirm_inspector_disclosures(host, &records) {
                return;
            }
            let mut state = inspector_documents.borrow_mut();
            let selected = state.selected;
            let item = &mut state.items[selected];
            item.title = host
                .get(TITLE)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text
                .clone();
            item.author = host
                .get(AUTHOR)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text
                .clone();
            item.revision = host
                .get(REVISION)
                .unwrap()
                .component::<NumberFieldContent>()
                .unwrap()
                .value();
            item.published = host
                .get(PUBLISHED)
                .unwrap()
                .component::<CheckboxContent>()
                .unwrap()
                .model()
                .checked
                == CheckState::Checked;
            let status = host
                .get(STATUS)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .semantics()
                .value;
            item.status = STATUSES
                .iter()
                .position(|value| status.as_deref() == Some(*value))
                .unwrap_or(item.status);
            item.priority = host
                .get(PRIORITY)
                .unwrap()
                .component::<SliderContent>()
                .unwrap()
                .model
                .value();
            update_document(&mut inspector_document.borrow_mut(), item);
            let description = summary(item);
            update_navigation(&mut inspector_navigation.borrow_mut(), &state);
            eprintln!(
                "GALLERY_APPHOST_PROPERTY selected={selected} summary={}",
                description
            );
        });
    let focus = LocalContainer::from_source("gallery-focus", SharedContent(document))
        .title("Document")
        .switcher_icon("tekes-file-text")
        .content_padding(0.)
        .list_height(700.)
        .toolbar(switch_toolbar())
        .focus_switch(SWITCH)
        .on_command(move |host, command| {
            let index = command
                .checked_sub(DOCUMENT_COMMAND)
                .filter(|index| *index < 4)
                .ok_or_else(|| anyhow::anyhow!("unknown Gallery document command"))?
                as usize;
            let mut state = focus_documents.borrow_mut();
            state.selected = index;
            update_document(host, &state.items[index]);
            update_inspector(&mut focus_inspector.borrow_mut(), &state.items[index]);
            update_navigation(&mut focus_navigation.borrow_mut(), &state);
            eprintln!(
                "GALLERY_APPHOST_DOCUMENT selected={index} title={}",
                state.items[index].title
            );
            Ok(())
        });
    let reading = LocalContainer::new("gallery-list", scroll(6000, stack(6001, vec![text(6002, "Reading notes", TextRole::UiHeading), field(ComponentId(6003), "Notes", "Keep a short note beside the current document."), text(6004, "Capture a question, an idea or a useful reference before the next meeting.", TextRole::UiBody)])))?.title("Notes").toolbar_host(ContainerToolbarHost::Container { position: ContainerToolbarPosition::Leading }).switcher_icon("tekes-note")
    .content_padding(0.).list_height(320.).toolbar(switch_toolbar()).focus_switch(SWITCH);
    let reference = LocalContainer::new("gallery-reference", scroll(6100, stack(6101, vec![
        text(6102, "Reference", TextRole::UiHeading),
        text(6103, "A second document instance. Switch between Notes and Reference from the Documents menu; each keeps its original content.", TextRole::UiBody),
    ])))?.title("Reference").toolbar_host(ContainerToolbarHost::Container { position: ContainerToolbarPosition::Trailing }).switcher_icon("tekes-file-text").content_padding(0.).toolbar(switch_toolbar()).focus_switch(SWITCH);
    let summary = LocalContainer::new("gallery-summary", scroll(6200, stack(6201, vec![
        text(6202, "Summary", TextRole::UiHeading),
        text(6203, "This document keeps its toolbar at the bottom. Use Documents to switch instances; use the swap action to exchange it with Focus.", TextRole::UiBody),
    ])))?.title("Summary").switcher_icon("tekes-file-text")
        .toolbar_host(ContainerToolbarHost::Container { position: ContainerToolbarPosition::Bottom })
        .content_padding(0.).toolbar(switch_toolbar()).focus_switch(SWITCH);
    let mut presentation = PresentationOptions::new("gallery-focus")
        .nav_bar("gallery-nav")
        .nav_fraction(1. / 3.)
        .navigation_toolbar("gallery-nav")
        .navigation_targets(vec!["gallery-focus".into()])
        .others(
            OthersOptions::new(vec![
                "gallery-detail".into(),
                "gallery-list".into(),
                "gallery-reference".into(),
                "gallery-summary".into(),
            ])
            .groups(vec![OthersGroup::new(
                "documents",
                "Documents",
                "tekes-files",
                vec![
                    "gallery-focus".into(),
                    "gallery-list".into(),
                    "gallery-reference".into(),
                    "gallery-summary".into(),
                ],
            )]),
        );
    presentation.focus_fraction = 0.5;
    Ok(AppWindow::from_containers(
        vec![
            nav.into(),
            focus.into(),
            inspector.into(),
            reading.into(),
            reference.into(),
            summary.into(),
        ],
        presentation,
    )?
    .theme(theme)
    .navigation(history_options()?)
    .shared_settings(settings))
}
pub fn open(
    parent: &Window,
    theme: Theme,
    settings: SettingsOwner,
) -> anyhow::Result<WindowHandle> {
    assemble(theme, settings)?.open(
        parent,
        WindowConfig {
            title: "TekesUI AppHost".into(),
            unified_titlebar: true,
            placement: tekes_ui_platform_macos::WindowPlacement::Auto {
                need_120hz: false,
                size_check: true,
            },
            size: WindowSize {
                width: 1100.,
                height: 640.,
            },
            min_size: Some(WindowSize {
                width: 600.,
                height: 320.,
            }),
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::{Rect, Scene};

    #[test]
    fn gallery_settings_owner_binds_real_schema_before_apphost_assembly() {
        let root = std::env::temp_dir().join(format!(
            "gallery-general-owner-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        // Same production helper, including actual TOML, storage suffix and
        // canonical display binding; no environment or user preference changes.
        let owner = settings_owner_in(root.clone()).unwrap();
        let first = assemble(Theme::LIGHT, owner.clone()).unwrap();
        let second = assemble(Theme::DARK, owner.clone()).unwrap();
        assert!(root.join("settings/general/settings.lock").is_file());
        drop(first);
        drop(second);
        drop(owner);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn settings_template_declares_valid_item_routes() {
        let options = SettingsWindow::from_toml(
            include_str!("apphost.settings.toml"),
            std::path::PathBuf::from("unused/settings.toml"),
        )
        .unwrap()
        .display_mode(SettingsDisplayMode::Exclusive);
        assert_eq!(options.schema.namespace, "gallery.apphost.general");
        assert_eq!(options.display_mode, SettingsDisplayMode::Exclusive);
        // Each declared navigation target must yield a real form schema, not
        // merely TOML that parses but cannot open through the Settings service.
        for (id, _) in options.schema.items() {
            assert!(!options.schema.for_item(&id).unwrap().settings.is_empty());
        }
    }

    #[test]
    fn inspector_disclosure_expands_and_collapses_without_submitting_properties() {
        use tekes_ui_controls::{
            composition::{AccessibilityAction, IconImage, IconRenderError, WithIconRenderer},
            layout_view::IconRenderer,
        };
        use tekes_ui_tokens::Rgba;
        struct Icons;
        impl IconRenderer for Icons {
            fn prepare_icon(
                &mut self,
                _: &str,
                _: f32,
                _: f32,
                _: Rgba,
            ) -> Result<IconImage, IconRenderError> {
                Ok(IconImage::new(tekes_ui_scene::ImageId(99), Rc::new(())))
            }
            fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
                panic!("prepared original icon required");
            }
        }
        let state = documents();
        let mut host = ContentHost::new(inspector_content(&state.items[0])).unwrap();
        host.set_action_events(true);
        let mut text = super::super::text::TextEngine::new();
        let frame = Rect::new(0., 0., 360., 600.);
        for (step, expanded) in [true, false, true].into_iter().enumerate() {
            host.render(
                &mut Scene::new(),
                &mut WithIconRenderer::new(&mut text, &mut Icons),
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
            assert!(
                host.semantics()
                    .iter()
                    .any(|node| node.id == ComponentId(7003)
                        && node.semantics.expanded == Some(expanded))
            );
            assert_eq!(
                host.get(TITLE)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap()
                    .surface
                    .input
                    .text,
                state.items[0].title
            );
            if step < 2 {
                assert!(
                    host.accessibility_action(ComponentId(7003), None, AccessibilityAction::Press)
                        .activated
                );
                let records = host.drain_action_records();
                assert!(!records.is_empty());
                assert!(
                    !confirm_inspector_disclosures(&mut host, &records),
                    "expansion does not submit fields or push history"
                );
            }
        }
    }
    #[test]
    fn apphost_document_reflows_at_real_width_and_last_paragraph_is_scrollable() {
        let state = documents();
        assert_ne!(state.items[0].paragraphs[0], state.items[1].paragraphs[0]);
        let mut host = ContentHost::new(document_content(&state.items[0])).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let wide = Rect::new(16., 16., 740., 340.);
        host.render(
            &mut Scene::new(),
            &mut text,
            wide,
            wide,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let wide_height = host.get(ComponentId(10)).unwrap().frame().size.height;
        let narrow = Rect::new(16., 16., 260., 340.);
        host.render(
            &mut Scene::new(),
            &mut text,
            narrow,
            narrow,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.get(ComponentId(10)).unwrap().frame().size.height > wide_height);
        let scroll = host
            .get_mut(ComponentId(9000))
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap();
        scroll.scroll.offset.y =
            (scroll.scroll.document_size().height - scroll.scroll.viewport_size().height).max(0.);
        host.render(
            &mut Scene::new(),
            &mut text,
            narrow,
            narrow,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let field = host
            .get(ComponentId(10))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let origin = field.surface.origin();
        let end = field
            .surface
            .caret_doc(field.surface.input.len())
            .offset(origin.x, origin.y);
        assert!(
            end.min_y() >= narrow.min_y() && end.max_y() <= narrow.max_y(),
            "the last text line is reachable at narrow width: {end:?}"
        );
    }
    #[test]
    fn apphost_body_selects_across_paragraphs_stays_readonly_and_keeps_selection_on_inspector_update()
     {
        use tekes_ui_controls::composition::{
            ContentEvent, PointerButton, PointerEvent, PointerModifiers, PointerPhase,
        };
        let state = documents();
        let mut host = ContentHost::new(document_content(&state.items[0])).unwrap();
        let mut text = super::super::text::TextEngine::new();
        let frame = Rect::new(0., 0., 740., 900.);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let end = state.items[0].paragraphs[0].encode_utf16().count() + 2 + 12;
        let (start_point, end_point, original) = {
            let field = host
                .get(ComponentId(10))
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap();
            let origin = field.surface.origin();
            let point = |index| {
                field
                    .surface
                    .caret_doc(index)
                    .offset(origin.x, origin.y)
                    .center()
            };
            (point(0), point(end), field.surface.input.text.clone())
        };
        for (phase, position) in [
            (PointerPhase::Down, start_point),
            (PointerPhase::Move, end_point),
            (PointerPhase::Up, end_point),
        ] {
            host.event(&ContentEvent::Pointer(PointerEvent {
                position,
                phase,
                button: Some(PointerButton::Primary),
                modifiers: PointerModifiers::default(),
                click_count: 1,
            }));
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        let field = host
            .get(ComponentId(10))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert!(field.surface.input.read_only);
        let selection = field.surface.input.selection();
        assert_eq!(selection, 0..end);
        assert_eq!(
            field.surface.input.selected_text(),
            String::from_utf16(&original.encode_utf16().take(end).collect::<Vec<_>>()).unwrap()
        );
        host.event(&ContentEvent::Commit {
            text: "overwrite".into(),
            replacement: None,
        });
        update_document(&mut host, &state.items[0]);
        let field = host
            .get(ComponentId(10))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert_eq!(field.surface.input.text, original);
        assert_eq!(
            field.surface.input.selection(),
            selection,
            "unrelated Inspector updates retain the original selection"
        );
        update_document(&mut host, &state.items[1]);
        let field = host
            .get(ComponentId(10))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert_eq!(
            field.surface.input.text,
            state.items[1].paragraphs.join("\n\n")
        );
    }

    #[test]
    fn apphost_navigation_requests_are_not_reemitted_by_publication_or_inspector_updates() {
        let state = documents();
        let host = Rc::new(RefCell::new(ContentHost::new(stack(1, vec![])).unwrap()));
        let pending = Rc::new(RefCell::new(VecDeque::from([entry(1), entry(2)])));
        let mut source = NavigationContent {
            host,
            pending: pending.clone(),
        };
        assert_eq!(source.take_navigation_request().unwrap().params["index"], 1);
        assert_eq!(source.take_navigation_request().unwrap().params["index"], 2);
        assert!(source.take_navigation_request().is_none());
        let options = history_options().unwrap();
        for index in 0..state.items.len() {
            assert_eq!(
                (options.view_command.unwrap())(&entry(index)).unwrap(),
                DOCUMENT_COMMAND + index as u64
            );
        }
        let mut document = ContentHost::new(document_content(&state.items[0])).unwrap();
        let mut inspector = ContentHost::new(inspector_content(&state.items[0])).unwrap();
        let original = document.get(ComponentId(1)).unwrap() as *const Content;
        update_document(&mut document, &state.items[2]);
        update_inspector(&mut inspector, &state.items[2]);
        assert_eq!(
            document.get(ComponentId(1)).unwrap() as *const Content,
            original
        );
        assert_eq!(
            inspector
                .get(TITLE)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text,
            state.items[2].title
        );
        assert!(
            pending.borrow().is_empty(),
            "publication updates selection/content without another Push"
        );
    }
}
