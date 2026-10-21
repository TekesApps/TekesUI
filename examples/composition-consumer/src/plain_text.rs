//! Caller-owned document inside an ordinary Surface, using the stock selectable
//! text adapter. No Agent-specific model or copied text event routing.
use tekes_ui_controls::{
    ButtonTextRenderer, DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult,
    ScrollArea,
    composition::*,
    extended::{Axis, Stack},
    feedback_view::SurfaceView,
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Spacing, Theme, proposal};

pub const ROOT: ComponentId = ComponentId(8900);
pub const SCROLLER: ComponentId = ComponentId(8901);
pub const PANEL: ComponentId = ComponentId(8902);
pub const BODY_STACK: ComponentId = ComponentId(8903);
pub const BODY: ComponentId = ComponentId(8904);
pub const APPEND: ComponentId = ComponentId(8905);
pub const FOOTNOTE: ComponentId = ComponentId(8906);
pub const FORM: ComponentId = ComponentId(8907);

#[derive(Debug)]
struct Card {
    view: SurfaceView,
    children: Vec<Content>,
}
impl Component for Card {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let frame = Rect::new(0., 0., available.width, available.height);
        let l = self.view.render_at_scale(
            &mut Scene::new(),
            frame,
            None,
            Palette::from(Theme::LIGHT),
            true,
            text.backing_scale(),
        );
        let size = self.children[0].measure(text, l.content.size).size;
        MeasuredChild::new(Size::new(
            available.width,
            size.height + (frame.size.height - l.content.size.height),
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let l = self.view.render_at_scale(
            &mut Scene::new(),
            frame,
            Some(clip),
            Palette::from(Theme::LIGHT),
            true,
            text.backing_scale(),
        );
        self.children[0].arrange(text, l.content, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view.render_at_scale(
            scene,
            frame,
            Some(clip),
            theme.palette,
            true,
            text.backing_scale(),
        );
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Document surface")
    }
}

#[derive(Debug)]
pub struct Scenario {
    pub host: ContentHost,
    pub value: DraftRevision,
}
impl Scenario {
    pub fn new() -> Result<Self, CompositionError> {
        let value=DraftRevision {identity:"plain-document".into(),version:1,payload:(0..28).map(|i|format!("Line {i:02}｜中文😀 e\u{301} selectable content in a parent-owned document.\n")).collect()};
        let mut body = TextFieldContent::read_only_content(
            "Selectable content",
            &value.payload,
            proposal::TextRole::UiBody,
        );
        let baseline = body.surface.input.external_baseline();
        assert!(matches!(
            body.surface.apply_external_value(
                &baseline,
                &value,
                ExternalSelection::Reset,
                ExternalUndo::Reset
            ),
            ExternalValueResult::Applied { .. }
        ));
        let contents = Content::new(
            BODY_STACK,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: None,
                children: vec![
                    Content::new(BODY, body),
                    Content::new(APPEND, ButtonContent::new("Append content")),
                    Content::new(
                        FOOTNOTE,
                        TextContent {
                            text: "Footnote · outer surface owns decoration and document scrolling"
                                .into(),
                            role: proposal::TextRole::UiCaption,
                        },
                    ),
                ],
            },
        );
        let panel = Content::new(
            PANEL,
            Card {
                view: SurfaceView::default(),
                children: vec![contents],
            },
        );
        let root = Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: Some(SCROLLER),
                children: vec![
                    Content::new(SCROLLER, ScrollContent::new(ScrollArea::default(), panel)),
                    Content::new(
                        FORM,
                        TextFieldContent::new("Ordinary field", "Form presentation unchanged"),
                    ),
                ],
            },
        );
        let mut host = ContentHost::new(root)?;
        host.set_action_events(true);
        Ok(Self { host, value })
    }
    /// The caller updates its own authoritative document through the existing
    /// controlled-value transaction; it never mutates selection or editor state.
    pub fn process_actions(&mut self) -> bool {
        process_actions(&mut self.host, &mut self.value)
    }
}

pub fn process_actions(host: &mut ContentHost, value: &mut DraftRevision) -> bool {
    let actions = host.drain_actions();
    let mut changed = false;
    for action in actions {
        if action.target == APPEND && action.kind == ContentActionKind::Activate {
            let next = DraftRevision {
                identity: value.identity.clone(),
                version: value.version + 1,
                payload: format!(
                    "{}Appended 中文😀 · version {}\n",
                    value.payload,
                    value.version + 1
                ),
            };
            let field = host
                .get_mut(BODY)
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap();
            let baseline = field.surface.input.external_baseline();
            if matches!(
                field.surface.apply_external_value(
                    &baseline,
                    &next,
                    ExternalSelection::Preserve,
                    ExternalUndo::Preserve
                ),
                ExternalValueResult::Applied { .. }
            ) {
                *value = next;
                changed = true;
            }
        }
    }
    changed
}
