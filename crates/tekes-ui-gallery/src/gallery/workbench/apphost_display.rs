//! Display/reading fixtures retain the original models and View/Content painters.
use super::StoryState;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    collections_view::{CodeBlockView, DiffViewRuntime, DocumentLayout, MarkdownView},
    composition::*,
    extended::{Axis, CodeBlock, DiffView, Markdown, Stack, StatusBadge},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal::TextRole};

pub(super) fn supports(renderer: &str) -> bool {
    matches!(
        renderer,
        "markdown"
            | "code-block"
            | "diff-view"
            | "surface"
            | "skeleton"
            | "status-badge"
            | "progress"
    )
}
#[derive(Debug)]
enum ReadingModel {
    Markdown(Markdown),
    Code(CodeBlock),
    Diff(DiffView),
}
#[derive(Debug)]
struct ReadingSpecimen {
    model: ReadingModel,
    markdown: MarkdownView,
    code: CodeBlockView,
    diff: DiffViewRuntime,
}
/// The legacy document Views expose layout through render. Measure with their
/// real algorithm and the receiving text metrics, suppressing glyph emission.
/// This is a borrowed metric adapter, not a second text engine or native owner.
struct MeasuringText<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for MeasuringText<'_> {
    fn measure(&mut self, value: &str, role: TextRole, width: f32) -> Size {
        self.0.measure(value, role, width)
    }
    fn resolved_font_size(&mut self, role: TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
    fn first_line_height(&mut self, value: &str, role: TextRole, width: f32) -> f32 {
        self.0.first_line_height(value, role, width)
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn draw(&mut self, _: &mut Scene, _: &str, _: TextRole, _: Rect, _: ButtonTextAlign, _: Rgba) {}
}
impl ReadingSpecimen {
    fn render(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> DocumentLayout {
        let scale = text.resolved_font_size(TextRole::UiBody) / TextRole::UiBody.style().size;
        match &self.model {
            ReadingModel::Markdown(model) => {
                self.markdown
                    .render(scene, &mut text, model, frame, Some(clip), palette, scale)
            }
            ReadingModel::Code(model) => {
                self.code
                    .render(scene, &mut text, model, frame, Some(clip), palette, scale)
            }
            ReadingModel::Diff(model) => {
                self.diff
                    .render(scene, &mut text, model, frame, Some(clip), palette, scale)
            }
        }
    }
}
impl Component for ReadingSpecimen {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let width = available.width.min(match self.model {
            ReadingModel::Code(_) => 680.,
            _ => 700.,
        });
        let frame = Rect::new(0., 0., width, 100_000.);
        let layout = self.render(
            &mut Scene::new(),
            &mut MeasuringText(text),
            frame,
            frame,
            Theme::LIGHT.into(),
        );
        MeasuredChild::new(Size::new(
            width,
            layout
                .content_height
                .max(if matches!(self.model, ReadingModel::Code(_)) {
                    80.
                } else {
                    0.
                }),
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.render(scene, text, frame, clip, theme.palette);
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(
            ContentRole::Group,
            match self.model {
                ReadingModel::Markdown(_) => "Markdown document",
                ReadingModel::Code(_) => "Code block",
                ReadingModel::Diff(_) => "Line difference",
            },
        )
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![InspectionProperty::new(
            "model",
            format!("{:?}", self.model),
        )])
    }
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn log(state: &StoryState) -> Content {
    text(
        92003,
        state.log.iter().cloned().collect::<Vec<_>>().join("\n"),
    )
}
pub(super) fn children(renderer: &str, state: &StoryState, variants: bool) -> Vec<Content> {
    match renderer {
        "markdown" | "code-block" | "diff-view" => {
            let model = match renderer {
                "markdown" => ReadingModel::Markdown(state.extended.markdown.clone()),
                "code-block" => ReadingModel::Code(state.extended.code.clone()),
                _ => ReadingModel::Diff(state.extended.diff.clone()),
            };
            let footer = match renderer {
                "markdown" => "Supported subset: headings, paragraphs, bullets and fenced code.",
                "code-block" => {
                    "Plain light surface; syntax coloring and editing are not part of this component."
                }
                _ => {
                    "Deterministic line-based LCS fixture; large inputs fall back to delete/add blocks."
                }
            };
            let mut items = vec![
                Content::new(
                    ComponentId(96020),
                    ReadingSpecimen {
                        model,
                        markdown: MarkdownView::new(),
                        code: CodeBlockView::new(),
                        diff: DiffViewRuntime::new(),
                    },
                ),
                text(96021, footer),
            ];
            if !variants && renderer == "markdown" {
                items.push(text(
                    96029,
                    "Interactive Live Preview Editor (MarkdownEditorContent)",
                ));
                items.push(Content::new(
                    ComponentId(96030),
                    MarkdownEditorContent::new(
                        "gallery-markdown-playground",
                        1,
                        "# Agent Notes\n\nFocus any block to edit **bold**, `inline code`, or [architecture docs](https://example.com/docs).\n\n- Live Preview block expansion\n- Canonical Markdown undo & copy\n\n```rust\nfn status() -> &'static str {\n    \"ready\"\n}\n```",
                    )
                    .expect("valid gallery markdown editor")
                    .with_viewport(TextViewport::Fixed(220.)),
                ));
            } else if !variants && renderer == "diff-view" {
                items.push(text(
                    96029,
                    "Selectable Diff Reader (RichContent::from_diff)",
                ));
                items.push(Content::new(
                    ComponentId(96031),
                    RichContent::new(
                        "gallery-diff-playground",
                        1,
                        RichDocument::from_diff(&state.extended.diff),
                    )
                    .expect("valid gallery diff reader"),
                ));
            }
            items
        }
        "surface" if variants => vec![text(
            96000,
            "Surface / Panel share the same semantic container; Panel adds the border.",
        )],
        "surface" => vec![
            Content::new(
                ComponentId(96020),
                PanelContent::new(Content::new(
                    ComponentId(96022),
                    StackContent {
                        model: Stack {
                            axis: Axis::Vertical,
                            gap: 24.,
                        },
                        flexible_child: None,
                        children: vec![
                            text(96021, "Panel — semantic surface and border"),
                            Content::new(ComponentId(92120), Button::new("Nested real action")),
                        ],
                    },
                ))
                .expect("finite Gallery Panel"),
            ),
            log(state),
        ],
        "skeleton" => (0..4)
            .map(|i| {
                Content::new(
                    ComponentId(96020 + i),
                    SkeletonContent::default().with_max_width(500. - (i % 2) as f32 * 80.),
                )
            })
            .chain([log(state)])
            .collect(),
        "status-badge" => [
            StatusBadge::Info,
            StatusBadge::Running,
            StatusBadge::Succeeded,
            StatusBadge::Failed,
        ]
        .into_iter()
        .zip(["Info", "Running", "Succeeded", "Failed"])
        .enumerate()
        .map(|(i, (status, label))| {
            Content::new(
                ComponentId(96020 + i as u64),
                StatusBadgeContent::new(status, label).expect("nonempty original badge label"),
            )
        })
        .collect(),
        "progress" if variants => [
            ("0% · determinate", 0.),
            ("45% · determinate", 0.45),
            ("100% · determinate", 1.),
            ("Indeterminate", 0.45),
        ]
        .into_iter()
        .enumerate()
        .flat_map(|(i, (label, value))| {
            let mut progress = ProgressContent::new("Progress", value).with_max_width(500.);
            progress.model.indeterminate = i == 3;
            [
                text(96030 + i as u64, label),
                Content::new(ComponentId(96040 + i as u64), progress),
            ]
        })
        .collect(),
        "progress" => {
            let mut advance = Button::new("Advance 10%");
            advance.button.state.disabled = state.extended.progress.value() >= 1.;
            let mut progress = ProgressContent::new("Progress", state.extended.progress.value())
                .with_max_width(500.);
            progress.model = state.extended.progress;
            vec![
                Content::new(ComponentId(96020), progress),
                Content::new(ComponentId(92120), advance),
                text(
                    96021,
                    format!("{:.0}%", state.extended.progress.value() * 100.),
                ),
                log(state),
            ]
        }
        _ => unreachable!("display story"),
    }
}
pub(super) fn after_action(renderer: &str, state: &StoryState, host: &mut ContentHost) {
    if renderer == "progress" {
        host.get_mut(ComponentId(96020))
            .unwrap()
            .component_mut::<ProgressContent>()
            .unwrap()
            .model = state.extended.progress;
        host.get_mut(ComponentId(96021))
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = format!("{:.0}%", state.extended.progress.value() * 100.);
        host.get_mut(ComponentId(92120))
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = state.extended.progress.value() >= 1.;
    }
    host.get_mut(ComponentId(92003))
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .text = state.log.iter().cloned().collect::<Vec<_>>().join("\n");
}
