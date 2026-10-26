//! Independent mixed reading document. The application supplies stable IDs and
//! initial/follow deduplication; all navigation geometry stays in ContentHost.
use tekes_ui_controls::{
    ButtonTextRenderer, ReadingDocumentVersion, ScrollArea, ScrollRevealOptions,
    composition::*,
    extended::{Axis, Stack},
    feedback_view::SurfaceView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, Theme, proposal};
pub const ROOT: ComponentId = ComponentId(11000);
pub const OWNER: ComponentId = ComponentId(11001);
pub const ITEMS: ComponentId = ComponentId(11002);
pub const INNER: ComponentId = ComponentId(11003);
pub const INNER_ITEMS: ComponentId = ComponentId(11004);
pub const PANEL: ComponentId = ComponentId(11005);
pub const TARGET: ComponentId = ComponentId(11006);
pub const COMPOSER: ComponentId = ComponentId(11007);
pub const OTHER: ComponentId = ComponentId(11008);
pub const TOP: ComponentId = ComponentId(11009);
pub const LAST: ComponentId = ComponentId(11010);
pub const NEAREST: ComponentId = ComponentId(11013);
pub const CENTERED: ComponentId = ComponentId(11014);
pub const ANIMATED: ComponentId = ComponentId(11015);
pub const LATEST: ComponentId = ComponentId(11016);
#[derive(Debug)]
pub struct Panel {
    pub clipped: bool,
    pub content_shift: f32,
    children: Vec<Content>,
    view: SurfaceView,
    arranged: Option<(bool, f32, Rect)>,
}
impl Panel {
    fn padding() -> f32 {
        proposal::metric("spacing", "space-loose").expect("original Panel padding")
    }
}
impl Component for Panel {
    fn host_reveal_passthrough(&self) -> bool {
        self.children.len() == 1 && self.arranged.is_some_and(|(clipped, shift, _)|
            clipped == self.clipped && shift == self.content_shift && shift.is_finite())
    }
    fn host_reveal_local_clip(&self) -> Option<Rect> {
        self.arranged.map(|(clipped, _, frame)| if clipped { Rect::default() } else { frame })
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        let pad = Self::padding();
        let measured =
            self.children[0].measure(t, Size::new((a.width - 2. * pad).max(0.), a.height));
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(
            a.width,
            measured.size.height + 2. * pad,
        ))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.arranged = None;
        let clip = if self.clipped {
            Rect::default()
        } else {
            f.intersection(&c).unwrap_or_default()
        };
        self.children[0].arrange(
            t,
            f.inset(Self::padding()).offset(0., self.content_shift),
            clip,
        );
        self.arranged = Some((self.clipped, self.content_shift, f));
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .render_at_scale(s, f, Some(c), theme.palette, true, t.backing_scale());
    }
}
fn column(id: ComponentId, children: Vec<Content>, flexible: Option<ComponentId>) -> Content {
    Content::new(
        id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: flexible,
            children,
        },
    )
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
pub struct Fixture {
    pub host: ContentHost,
    pub version: ReadingDocumentVersion,
    pub items: Vec<ComponentId>,
}
impl Fixture {
    pub fn new() -> Result<Self, CompositionError> {
        let mut inner = Vec::new();
        for index in 0..12 {
            inner.push(text(
                11200 + index,
                format!("Detail {index:02} · 中文 e\u{301} 😀 · original content"),
            ));
        }
        inner.push(Content::new(
            TARGET,
            ButtonContent::new("Inspect retained item · 检查"),
        ));
        for index in 12..24 {
            inner.push(text(
                11200 + index,
                format!("Detail {index:02} · original content follows"),
            ));
        }
        let mut nested = ScrollContent::new(
            ScrollArea::default(),
            Content::new(INNER_ITEMS, ListContent::new("Nested document", inner)),
        );
        nested.viewport_height = Some(140.);
        let panel = Content::new(
            PANEL,
            Panel {
                clipped: false,
                content_shift: 0.,
                view: SurfaceView::default(),
                arranged: None,
                children: vec![column(
                    ComponentId(11011),
                    vec![
                        text(11012, "Original Panel · 原面板"),
                        Content::new(INNER, nested),
                    ],
                    None,
                )],
            },
        );
        let mut rows = vec![Content::new(
            TOP,
            TextContent::new("Reading document · 阅读内容"),
        )];
        for index in 0..12 {
            rows.push(text(
                11100 + index,
                format!("Before {index:02} · mixed text, panels and real controls"),
            ));
        }
        rows.push(panel);
        for index in 12..24 {
            rows.push(text(
                11100 + index,
                format!("After {index:02} · retained stable reading item"),
            ));
        }
        rows.push(Content::new(
            LAST,
            ButtonContent::new("Last item · 最后一项"),
        ));
        let items = rows
            .iter()
            .map(|row| if row.id() == PANEL { TARGET } else { row.id() })
            .collect::<Vec<_>>();
        let outer = ScrollContent::new(
            ScrollArea::default(),
            Content::new(ITEMS, ListContent::new("Mixed document", rows)),
        );
        let mut other = ScrollContent::new(
            ScrollArea::default(),
            column(
                ComponentId(11400),
                (0..12)
                    .map(|i| text(11401 + i, format!("Unrelated {i:02} · 独立分支")))
                    .collect(),
                None,
            ),
        );
        other.viewport_height = Some(84.);
        let mut host = ContentHost::new(column(
            ROOT,
            vec![
                Content::new(
                    ComponentId(11017),
                    GridContent {
                        model: tekes_ui_controls::extended::Grid {
                            columns: 2,
                            gap_x: Spacing::Regular.value(),
                            gap_y: Spacing::Regular.value(),
                        },
                        children: [
                            (NEAREST, "Nearest item"),
                            (CENTERED, "Center item"),
                            (ANIMATED, "Animated jump"),
                            (LATEST, "Latest"),
                        ]
                        .into_iter()
                        .map(|(id, label)| {
                            Content::new(id, ButtonContent::new(label)).preserving_focus()
                        })
                        .collect(),
                    },
                ),
                Content::new(OWNER, outer),
                Content::new(OTHER, other),
                Content::new(
                    COMPOSER,
                    TextFieldContent::new("Independent draft", "Draft A😀B"),
                ),
            ],
            Some(OWNER),
        ))?;
        let (version, ()) = host
            .set_scroll_reading_document(OWNER, "document-A", 1, items.clone(), |_| {})
            .expect("valid original document");
        host.set_scroll_reading_document(
            OTHER,
            "document-other",
            1,
            vec![ComponentId(11401)],
            |_| {},
        )
        .expect("valid sibling document");
        host.set_action_events(true);
        Ok(Self {
            host,
            version,
            items,
        })
    }
    pub fn navigate(
        &mut self,
        item: ComponentId,
        options: ScrollRevealOptions,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<ContentRevealResult, ContentRevealError> {
        // Normal application frames are outside the read-only reveal transaction.
        // A document without a completed layout must still be rejected by Host;
        // a failed render has no allocation to reuse and is not auto-recovered.
        let allocation = self.host.last_layout_allocation();
        let document_laid_out = self.host.get(OWNER)
            .and_then(|owner| owner.component::<ScrollContent>())
            .is_some_and(|scroll| scroll.reading_result().is_some());
        if document_laid_out && self.host.needs_render()
            && let Some((frame, clip)) = allocation
        {
            self.host.render(&mut Scene::new(), text, frame, clip, Theme::LIGHT.into())
                .map_err(ContentRevealError::Layout)?;
        }
        let result = self.host
            .reveal_scroll_reading_item(OWNER, &self.version, item, options, text)?;
        if let Some((frame, clip)) = allocation {
            self.host.render(&mut Scene::new(), text, frame, clip, Theme::LIGHT.into())
                .map_err(ContentRevealError::Layout)?;
        }
        Ok(result)
    }
}
