//! Public read-only TextSurface history beside an independent composed editor.
//! Uses the existing direct viewport path; configurable TextField slots are #20.
use tekes_ui_controls::{
    ButtonTextRenderer, ReadingController, ReadingUpdate, ReadingUpdateError, TextRangeMapping,
    TextSurface,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Spacing};

pub const ROOT: ComponentId = ComponentId(8600);
pub const EARLIER: ComponentId = ComponentId(8601);
pub const APPEND: ComponentId = ComponentId(8602);
pub const LATEST: ComponentId = ComponentId(8603);
pub const EDITOR: ComponentId = ComponentId(8604);

pub struct ReadingFixture {
    pub surface: TextSurface,
    pub reading: ReadingController,
    pub controls: ContentHost,
    pub pages: usize,
    counts: [usize; 3],
    start_mid: bool,
}
impl Default for ReadingFixture {
    fn default() -> Self {
        Self::new()
    }
}
impl ReadingFixture {
    pub fn new() -> Self {
        let body: String = (0..100).map(|i| format!("记录 {i:03}｜中文 e\u{301} 😀 Reading remains at the same content after earlier history arrives.\n")).collect();
        let mut surface = TextSurface::new("", true, true);
        let mut reading = ReadingController::default();
        reading
            .set_document(&mut surface, "public-history-A", 1, &body)
            .unwrap();
        let controls = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: None,
                children: vec![
                    Content::new(
                        ComponentId(8605),
                        StackContent {
                            model: Stack {
                                axis: Axis::Vertical,
                                gap: Spacing::Regular.value(),
                            },
                            flexible_child: None,
                            children: vec![
                                Content::new(EARLIER, ButtonContent::new("加载更早")),
                                Content::new(APPEND, ButtonContent::new("追加一段")),
                                Content::new(LATEST, ButtonContent::new("回到最新"))
                                    .preserving_focus(),
                            ],
                        },
                    ),
                    Content::new(
                        EDITOR,
                        TextFieldContent::new("草稿", "Keep this draft 中文😀"),
                    ),
                ],
            },
        ))
        .unwrap();
        Self {
            surface,
            reading,
            controls,
            pages: 0,
            counts: [0; 3],
            start_mid: true,
        }
    }
    pub fn prepend(&mut self) -> Result<ReadingUpdate, ReadingUpdateError> {
        let prefix: String = (0..12)
            .map(|i| format!("历史页 {}/{i}｜中文 😀 e\u{301}\n", self.pages + 1))
            .collect();
        let len = self.surface.input.len();
        let delta = prefix.encode_utf16().count();
        let next = format!("{prefix}{}", self.surface.input.text);
        let version = self.reading.document_version().unwrap().clone();
        let update = self.reading.update_document(
            &mut self.surface,
            &version,
            version.revision() + 1,
            &next,
            &[TextRangeMapping {
                old: 0..len,
                new: delta..delta + len,
            }],
        )?;
        self.pages += 1;
        Ok(update)
    }
    pub fn append(&mut self) -> Result<ReadingUpdate, ReadingUpdateError> {
        let len = self.surface.input.len();
        let next = format!(
            "{}追加内容｜new arrival does not change reading intent.\n",
            self.surface.input.text
        );
        let version = self.reading.document_version().unwrap().clone();
        self.reading.update_document(
            &mut self.surface,
            &version,
            version.revision() + 1,
            &next,
            &[TextRangeMapping {
                old: 0..len,
                new: 0..len,
            }],
        )
    }
    /// Uses the existing activation counters until #21 supplies typed actions.
    pub fn process_actions(&mut self) -> Result<bool, ReadingUpdateError> {
        let counts = [EARLIER, APPEND, LATEST].map(|id| {
            self.controls
                .get(id)
                .unwrap()
                .component::<ButtonContent>()
                .unwrap()
                .activations
        });
        let old = self.counts;
        self.counts = counts;
        if counts[0] > old[0] {
            self.prepend()?;
        }
        if counts[1] > old[1] {
            self.append()?;
        }
        if counts[2] > old[2] {
            self.reading.follow_latest(&mut self.surface);
        }
        Ok(counts != old)
    }
    pub fn paint(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        scene: &mut Scene,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) {
        let gap = Spacing::Loose.value();
        let control_size = self
            .controls
            .get_mut(ROOT)
            .unwrap()
            .measure(text, Size::new(frame.size.width, frame.size.height))
            .size;
        let reading_frame = Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            (frame.size.height - control_size.height - gap).max(1.),
        );
        let role = self.surface.role;
        let scale = text.backing_scale();
        if let Some(host) = text.text_surface_host() {
            let type_scale = host.type_scale();
            self.reading.layout_with(
                &mut self.surface,
                reading_frame,
                Some(clip),
                scale,
                type_scale,
                |s, w| host.shape_surface(s, role, w),
            );
            if self.start_mid {
                self.surface.scroll.scroll_by(0., 713.25);
                self.start_mid = false;
                self.reading.layout_with(
                    &mut self.surface,
                    reading_frame,
                    Some(clip),
                    scale,
                    type_scale,
                    |s, w| host.shape_surface(s, role, w),
                );
            }
            self.surface.draw(host, scene, palette, false, false);
        }
        self.controls
            .render(
                scene,
                text,
                Rect::new(
                    frame.min_x(),
                    reading_frame.max_y() + gap,
                    frame.size.width,
                    control_size.height,
                ),
                clip,
                palette,
            )
            .unwrap();
    }
}
