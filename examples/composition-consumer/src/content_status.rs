//! Caller-owned Tabs/Disclosure content-status scenarios using public APIs only.
//! These are application fixtures, not new built-in controls or status protocols.
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{Axis, Banner, Disclosure, EmptyState, NoticeTone, Progress, Stack},
    feedback_view::{BannerView, EmptyStateView, ProgressView},
    layout::MeasuredChild,
    layout_view::{DisclosureView, IconRenderer, IconView, TabsView},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Spacing, proposal};

pub const HEADER: ComponentId = ComponentId(8101);
pub const PANEL: ComponentId = ComponentId(8102);
pub const FEEDBACK: ComponentId = ComponentId(8103);
pub const EDITOR_GATE: ComponentId = ComponentId(8104);
pub const EDITOR: ComponentId = ComponentId(8105);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Tabs,
    Disclosure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Loading,
    Error,
    Empty,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Loading => "Loading content",
            Self::Error => "Content could not be loaded",
            Self::Empty => "No content yet",
        }
    }
}
#[derive(Debug)]
enum HeaderView {
    Tabs(Box<TabsView>),
    Disclosure {
        model: Disclosure,
        view: Box<DisclosureView>,
    },
}
#[derive(Debug)]
struct Header {
    tabs_layout: tekes_ui_controls::layout_view::TabsLayout,
    icon: Option<IconContent>,
    view: HeaderView,
    visible: Rc<Cell<bool>>,
    selected: Rc<Cell<usize>>,
}
impl Header {
    fn project(&self) {
        match &self.view {
            HeaderView::Tabs(view) => {
                self.selected.set(view.selected());
                self.visible.set(view.selected() == 0);
            }
            HeaderView::Disclosure { model, .. } => self.visible.set(model.open),
        }
    }
}
impl Component for Header {
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn requires_icon_services(&self) -> bool {
        self.icon.is_some()
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.icon.as_ref().and_then(Component::icon_error)
    }

    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let width = match self.view {
            HeaderView::Tabs(_) => (a.width / 3.).min(160.) - 8.,
            _ => a.width - 24.,
        };
        let h = ["Content", "History", "Activity"]
            .into_iter()
            .map(|s| {
                ButtonModel::measure(
                    &mut &mut *t,
                    s,
                    width.max(0.),
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .height
            })
            .fold(48., f32::max);
        MeasuredChild::new(Size::new(a.width, h))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        mut t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        match &mut self.view {
            HeaderView::Tabs(view) => {
                self.tabs_layout = view.render(
                    s,
                    &mut t,
                    &["Content", "History", "Activity"],
                    f,
                    Some(c),
                    theme.palette,
                    proposal::TextRole::UiBody,
                    false,
                );
            }
            HeaderView::Disclosure { model, view } => {
                view.render(
                    s,
                    &mut t,
                    model,
                    "Details",
                    f,
                    Some(c),
                    theme.palette,
                    proposal::TextRole::UiBody,
                );
            }
        }
        if let (Some(icon), HeaderView::Disclosure { model, view }) = (&mut self.icon, &self.view) {
            icon.set_glyph(if model.open {
                "tekes-expand"
            } else {
                "tekes-forward"
            })
            .expect("canonical disclosure icon");
            let mut theme = theme;
            theme.icon_foreground = Some(theme.palette.color("primary-text").unwrap());
            icon.paint(s, t, view.icon_frame(), c, theme);
        }
        self.project();
    }
    fn focusable(&self) -> bool {
        true
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        match &mut self.view {
            HeaderView::Tabs(view) => {
                if focused {
                    view.focus(view.selected(), visible);
                } else {
                    view.clear_focus();
                }
            }
            HeaderView::Disclosure { view, .. } => {
                if focused {
                    view.focus(visible);
                } else {
                    view.blur();
                }
            }
        }
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let mut r = ContentResponse::default();
        match &mut self.view {
            HeaderView::Tabs(view) => match e {
                ContentEvent::PointerMove(p) => {
                    view.pointer_move_at(*p);
                    r = ContentResponse::consumed(true);
                }
                ContentEvent::PointerDown(p) => {
                    let hit = view.pointer_down_at(*p).is_some();
                    r = ContentResponse {
                        consumed: hit,
                        capture_pointer: hit,
                        ..Default::default()
                    };
                }
                ContentEvent::PointerUp(p) => {
                    r.activated = view.pointer_up_at(*p).is_some();
                    r.changed = r.activated;
                    r.consumed = r.activated;
                }
                ContentEvent::Key { key, down } => {
                    r.changed = view.key(*key, *down).is_some();
                    r.consumed = matches!(
                        key,
                        ControlKey::Left
                            | ControlKey::Right
                            | ControlKey::Home
                            | ControlKey::End
                            | ControlKey::Space
                            | ControlKey::Enter
                    );
                }
                ContentEvent::Cancel => view.cancel(),
                _ => {}
            },
            HeaderView::Disclosure { model, view } => match e {
                ContentEvent::PointerMove(p) => {
                    view.pointer_move(*p);
                    r = ContentResponse::consumed(true);
                }
                ContentEvent::PointerDown(p) => {
                    let hit = view.pointer_down(*p);
                    r = ContentResponse {
                        consumed: hit,
                        capture_pointer: hit,
                        ..Default::default()
                    };
                }
                ContentEvent::PointerUp(p) => {
                    r.changed = view.pointer_up(*p, model);
                    r.consumed = r.changed;
                    r.activated = r.changed;
                }
                ContentEvent::Key { key, down } => {
                    let result = view.key_event(*key, *down, model);
                    r = ContentResponse::consumed(result.changed);
                    r.consumed = result.consumed;
                }
                ContentEvent::Cancel => {
                    view.cancel();
                }
                _ => {}
            },
        }
        self.project();
        r
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if !matches!(action, AccessibilityAction::Press) {
            return ContentResponse::default();
        }
        let changed = match &mut self.view {
            HeaderView::Tabs(view) => part.is_some_and(|i| view.select(i)),
            HeaderView::Disclosure { model, view } => part.is_none() && view.activate(model),
        };
        self.project();
        ContentResponse::consumed(changed)
    }
    fn semantics(&self) -> ContentSemantics {
        match &self.view {
            HeaderView::Tabs(view) => {
                let mut s = ContentSemantics::new(ContentRole::Group, "Tabs content scenario");
                s.value = Some(view.selected().to_string());
                s
            }
            HeaderView::Disclosure { model, .. } => {
                let mut s = ContentSemantics::new(ContentRole::Button, "Details");
                s.expanded = Some(model.open);
                s
            }
        }
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        match &self.view {
            HeaderView::Tabs(view) => self
                .tabs_layout
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let mut semantics = ContentSemantics::new(
                        ContentRole::Button,
                        ["Content", "History", "Activity"][index],
                    );
                    semantics.selected = view.selected() == index;
                    SemanticPart {
                        index,
                        frame: item.frame,
                        focused: false,
                        semantics,
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}
#[derive(Debug)]
struct Panel {
    visible: Rc<Cell<bool>>,
    children: Vec<Content>,
}
impl Component for Panel {
    fn active(&self) -> bool {
        self.visible.get()
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width, if self.active() { 228. } else { 0. }))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        for child in &mut self.children {
            child.arrange(t, f, c);
        }
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[derive(Debug)]
struct EditorGate {
    status: Rc<Cell<Status>>,
    children: Vec<Content>,
}
impl Component for EditorGate {
    fn active(&self) -> bool {
        self.status.get() == Status::Ready
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[derive(Debug)]
pub(crate) struct Feedback {
    status: Rc<Cell<Status>>,
    progress: ProgressView,
    banner: BannerView,
    empty: EmptyStateView,
}
impl Feedback {
    pub(crate) fn new(status: Rc<Cell<Status>>) -> Self {
        Self {
            status,
            progress: Default::default(),
            banner: Default::default(),
            empty: Default::default(),
        }
    }
}
impl Component for Feedback {
    fn active(&self) -> bool {
        self.status.get() != Status::Ready
    }
    fn measure(&mut self, mut text: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let size = match self.status.get() {
            Status::Ready => Size::new(a.width, 0.),
            Status::Loading => Size::new(a.width, 56.),
            Status::Error => BannerView::measure(
                &mut text,
                Status::Error.label(),
                a.width,
                false,
                proposal::TextRole::UiBody,
            ),
            Status::Empty => EmptyStateView::measure(
                &mut text,
                &EmptyState { title: Status::Empty.label().into(),
                    detail: "The current panel is empty. Other panels and the retained draft are unchanged.".into(),
                    action: None },
                a.width, proposal::TextRole::UiBody,
            ),
        };
        MeasuredChild::new(size)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        mut t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        let status = self.status.get();
        match status {
            Status::Loading => {
                t.draw(
                    s,
                    status.label(),
                    proposal::TextRole::UiBody,
                    Rect::new(f.min_x(), f.min_y(), f.size.width, 40.),
                    ButtonTextAlign::Leading,
                    theme.palette.color("primary-text").unwrap(),
                );
                let mut progress = Progress::new(0.);
                progress.indeterminate = true;
                self.progress.render(
                    s,
                    progress,
                    Rect::new(f.min_x(), f.min_y() + 48., f.size.width, 8.),
                    Some(c),
                    theme.palette,
                );
            }
            Status::Error => {
                let model = Banner {
                    message: status.label().into(),
                    tone: NoticeTone::Error,
                    dismissible: false,
                    visible: true,
                };
                self.banner.render(
                    s,
                    &mut t,
                    &model,
                    f,
                    Some(c),
                    theme.palette,
                    proposal::TextRole::UiBody,
                );
            }
            Status::Empty => {
                let model=EmptyState{title:status.label().into(),detail:"The current panel is empty. Other panels and the retained draft are unchanged.".into(),action:None};
                self.empty.render(
                    s,
                    &mut t,
                    &model,
                    f,
                    Some(c),
                    theme.palette,
                    proposal::TextRole::UiBody,
                );
            }
            Status::Ready => {}
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let status = self.status.get();
        let mut s = ContentSemantics::new(ContentRole::Group, status.label());
        s.invalid = status == Status::Error;
        s
    }
}
/// Mount the original caller-owned fixture directly in another application's Host.
/// `embedded_icon` assigns the disclosure glyph to this subtree instead of Scenario's external painter.
#[allow(clippy::too_many_arguments)]
pub fn fixture_content(
    kind: Kind,
    ids: [ComponentId; 6],
    status: Rc<Cell<Status>>,
    visible: Rc<Cell<bool>>,
    selected: Rc<Cell<usize>>,
    embedded_icon: bool,
    focus: bool,
) -> Content {
    let [
        root,
        header_id,
        panel_id,
        feedback_id,
        editor_gate,
        editor_id,
    ] = ids;
    let mut header = Header {
        tabs_layout: Default::default(),
        icon: embedded_icon
            .then(|| IconContent::new("tekes-forward").expect("canonical disclosure icon")),
        view: match kind {
            Kind::Tabs => {
                let mut view = TabsView::new(3);
                view.select(selected.get());
                HeaderView::Tabs(Box::new(view))
            }
            Kind::Disclosure => HeaderView::Disclosure {
                model: Disclosure {
                    open: visible.get(),
                    disabled: false,
                },
                view: Box::new(DisclosureView::default()),
            },
        },
        visible: visible.clone(),
        selected: selected.clone(),
    };
    if focus {
        header.set_focus(true, true);
    }
    let editor = Content::new(
        editor_gate,
        EditorGate {
            status: status.clone(),
            children: vec![Content::new(
                editor_id,
                TextFieldContent::new("Retained draft", "Caller draft"),
            )],
        },
    );
    let feedback = Content::new(feedback_id, Feedback::new(status.clone()));
    let panel = Content::new(
        panel_id,
        Panel {
            visible: visible.clone(),
            children: vec![feedback, editor],
        },
    );
    Content::new(
        root,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            children: vec![Content::new(header_id, header), panel],
            flexible_child: None,
        },
    )
}

/// The caller changes content status independently from the production header
/// selection/expanded model. No status update forces opening or panel selection.
#[derive(Debug)]
pub struct Scenario {
    pub host: ContentHost,
    kind: Kind,
    status: Rc<Cell<Status>>,
    visible: Rc<Cell<bool>>,
    selected: Rc<Cell<usize>>,
}
impl Scenario {
    pub fn new(kind: Kind) -> Result<Self, CompositionError> {
        let status = Rc::new(Cell::new(Status::Ready));
        let visible = Rc::new(Cell::new(kind == Kind::Tabs));
        let selected = Rc::new(Cell::new(0));
        let host = ContentHost::new(fixture_content(
            kind,
            [
                ComponentId(8100),
                HEADER,
                PANEL,
                FEEDBACK,
                EDITOR_GATE,
                EDITOR,
            ],
            status.clone(),
            visible.clone(),
            selected.clone(),
            false,
            false,
        ))?;
        Ok(Self {
            host,
            kind,
            status,
            visible,
            selected,
        })
    }
    pub fn set_status(&mut self, status: Status) {
        self.status.set(status);
        self.host.get_mut(PANEL);
    }
    pub fn status(&self) -> Status {
        self.status.get()
    }
    pub fn visible(&self) -> bool {
        self.visible.get()
    }
    pub fn selected(&self) -> usize {
        self.selected.get()
    }
    pub fn set_open(&mut self, open: bool) -> bool {
        if self.kind != Kind::Disclosure {
            return false;
        }
        let header = self
            .host
            .get_mut(HEADER)
            .unwrap()
            .component_mut::<Header>()
            .unwrap();
        let changed = match &mut header.view {
            HeaderView::Disclosure { model, .. } => model.set_open(open),
            _ => false,
        };
        header.project();
        changed
    }
    pub fn select(&mut self, index: usize) -> bool {
        self.host
            .accessibility_action(HEADER, Some(index), AccessibilityAction::Press)
            .changed
    }
    /// Host-owned approved icon backend; read the production header's resolved
    /// slot without mutating the ContentHost after its render acknowledgement.
    pub fn draw_disclosure_icon(
        &self,
        s: &mut Scene,
        r: &mut impl IconRenderer,
        clip: Rect,
        palette: Palette,
    ) {
        let header = self
            .host
            .get(HEADER)
            .unwrap()
            .component::<Header>()
            .unwrap();
        if let HeaderView::Disclosure { model, view } = &header.view {
            IconView::default().render_tinted(
                s,
                r,
                if model.open {
                    "chevron-down"
                } else {
                    "chevron-right"
                },
                view.icon_frame(),
                proposal::metric("size", "size-icon-standard").unwrap(),
                Some(clip),
                palette,
                palette.color("primary-text").unwrap(),
            );
        }
    }
}
