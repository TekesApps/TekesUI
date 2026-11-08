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
    AppWindow, ContainerContent, LocalContainer, NavigationOptions, OthersOptions,
    PresentationOptions,
};
use tekes_ui_app_protocol::ToolbarItem;
use tekes_ui_controls::{
    CheckState, ScrollArea, TextInput,
    composition::{
        CheckboxContent, Component, ComponentId, Content, ContentActionPayload,
        ContentActionRecord, ContentHost, DecimalNumberFormat, DisclosureContent,
        NumberFieldContent, ScrollContent, SelectContent, SelectableListContent, SliderContent,
        StackContent, TextContent, TextFieldContent, TextSubmitPolicy,
    },
    extended::{Axis, Slider, Stack},
};
use tekes_ui_platform_macos::{Size as WindowSize, Window, WindowConfig, WindowHandle};
use tekes_ui_tokens::{Spacing, proposal::TextRole};
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
fn history_options() -> anyhow::Result<NavigationOptions> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let root = if let Some(root) = std::env::var_os("TEKESUI_APPHOST_DATA_DIR") {
        std::path::PathBuf::from(root)
    } else {
        std::path::PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is unavailable"))?,
        )
        .join("Library/Application Support/TekesUI Gallery/AppHost")
    };
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
        ScrollContent::new(ScrollArea::default(), child),
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
    children.extend(
        document
            .paragraphs
            .iter()
            .enumerate()
            .map(|(index, value)| text(10 + index as u64, value, TextRole::UiBody)),
    );
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
    for (index, value) in document.paragraphs.iter().enumerate() {
        host.get_mut(ComponentId(10 + index as u64))
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text
            .clone_from(value);
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
    scroll(
        7000,
        stack(
            7005,
            vec![
                text(7006, "Inspector", TextRole::UiHeading),
                text(
                    7007,
                    "Changes apply to the document selected in navigation.",
                    TextRole::UiCaption,
                ),
                disclosure,
            ],
        ),
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
fn assemble() -> anyhow::Result<AppWindow> {
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
    let mut resources = SelectableListContent::new(
        "Project",
        documents.borrow().items[2..]
            .iter()
            .map(|d| d.title.clone())
            .collect(),
    );
    resources.view.set_navigation_selection_fill_only(true);
    let navigation = scroll(
        8000,
        stack(
            8001,
            vec![
                text(8002, "Workspace", TextRole::UiHeading),
                text(8003, "WRITING", TextRole::UiCaption),
                Content::new(ComponentId(8004), writing),
                text(8005, "PROJECT", TextRole::UiCaption),
                Content::new(ComponentId(8006), resources),
            ],
        ),
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
    .title("Workspace")
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
    let reading = LocalContainer::new("gallery-list", scroll(6000, stack(6001, vec![text(6002, "Reading notes", TextRole::UiHeading), field(ComponentId(6003), "Notes", "Keep a short note beside the current document."), text(6004, "Capture a question, an idea or a useful reference before the next meeting.", TextRole::UiBody)])))?.title("Notes").list_height(320.).toolbar(switch_toolbar()).focus_switch(SWITCH);
    let mut presentation = PresentationOptions::new("gallery-focus")
        .nav_bar("gallery-nav")
        .nav_fraction(1. / 3.)
        .navigation_toolbar("gallery-focus")
        .navigation_toolbar_follows_focus(true)
        .navigation_targets(vec!["gallery-focus".into()])
        .others(OthersOptions::new(vec![
            "gallery-detail".into(),
            "gallery-list".into(),
        ]));
    presentation.focus_fraction = 0.5;
    Ok(AppWindow::from_containers(
        vec![nav.into(), focus.into(), inspector.into(), reading.into()],
        presentation,
    )?
    .navigation(history_options()?))
}
pub fn open(parent: &Window) -> anyhow::Result<WindowHandle> {
    assemble()?.open(
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
    use tekes_ui_tokens::Theme;

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
        assert!(
            host.get(ComponentId(13)).unwrap().visible().is_some(),
            "last paragraph is reachable at narrow width"
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
