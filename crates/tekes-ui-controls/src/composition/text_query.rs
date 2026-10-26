//! Read-only native query data from the original TextSurface, never an editor model.
use crate::TextSurface;
use std::ops::Range;
use tekes_ui_scene::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct ContentTextInput {
    /// Opaque original editor/document binding. Geometry changes do not change it.
    pub binding: Vec<u8>,
    pub text: String,
    pub caret_index: usize,
    pub selection: Range<usize>,
    pub marked: Option<Range<usize>>,
    pub read_only: bool,
    /// Original shaped geometry in host logical coordinates. None means the
    /// geometry is unavailable or clipped, not that the text owner is absent.
    pub caret: Option<Rect>,
    pub clip: Option<Rect>,
    pub carets: Vec<(usize, Rect)>,
}

impl ContentTextInput {
    pub(crate) fn local_binding(surface: &TextSurface) -> Vec<u8> {
        let (editor, document) = surface.input.controlled_binding();
        [editor, document]
            .into_iter()
            .flat_map(u64::to_le_bytes)
            .collect()
    }

    pub fn from_surface(surface: &TextSurface) -> Self {
        let clip = surface
            .scroll
            .geometry(surface.inner, surface.ancestor_clip())
            .visible_viewport_rect();
        let visible = |rect: Rect| {
            clip.and_then(|c| c.intersection(&rect))
                .filter(|r| r.size.height > 0.)
        };
        let origin = surface.origin();
        let carets = surface.paragraph.as_ref().map_or_else(Vec::new, |p| {
            p.lines
                .iter()
                .flat_map(|line| {
                    line.carets.iter().filter_map(|(index, x)| {
                        let rect = Rect::new(
                            origin.x + *x as f32,
                            origin.y + line.top as f32,
                            tekes_ui_tokens::Stroke::Rule.value(),
                            line.height as f32,
                        );
                        visible(rect).map(|rect| (*index, rect))
                    })
                })
                .collect()
        });
        Self {
            binding: Self::local_binding(surface),
            text: surface.input.text.clone(),
            caret_index: surface.input.caret,
            selection: surface.input.selection(),
            marked: surface.input.marked.clone(),
            read_only: surface.input.read_only,
            caret: visible(surface.caret()),
            clip,
            carets,
        }
    }
}

/// Complete original shaper output for native accessibility queries. Unlike
/// the IME snapshot, offscreen caret/line geometry is retained. This is a
/// read-only projection, never a second editor or a synthesized line model.
#[derive(Clone, Debug, PartialEq)]
pub struct ContentTextLayout {
    pub input: ContentTextInput,
    pub multiline: bool,
    pub lines: Vec<Range<usize>>,
    pub carets: Vec<(usize, Rect)>,
}
impl ContentTextLayout {
    pub fn from_surface(surface: &TextSurface) -> Option<Self> {
        if surface.shaped != surface.input.text {
            return None;
        }
        let paragraph = surface.paragraph.as_ref()?;
        let origin = surface.origin();
        Some(Self {
            input: ContentTextInput::from_surface(surface),
            multiline: surface.input.multiline,
            lines: paragraph
                .lines
                .iter()
                .map(|line| line.utf16.clone())
                .collect(),
            carets: paragraph
                .lines
                .iter()
                .flat_map(|line| {
                    line.carets.iter().map(move |(index, x)| {
                        (
                            *index,
                            Rect::new(
                                origin.x + *x as f32,
                                origin.y + line.top as f32,
                                tekes_ui_tokens::Stroke::Rule.value(),
                                line.height as f32,
                            ),
                        )
                    })
                })
                .collect(),
        })
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::super::*;
    use crate::{TextSurfaceHost, TextSurfaceTextRenderer};
    use std::rc::Rc;
    use tekes_ui_text::{Paragraph, ShapeOptions, Shaper, TextStyle, coretext::{CoreTextFonts, CoreTextShaper}};
    use tekes_ui_tokens::Theme;

    struct Text(bool);
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, role: proposal::TextRole, _: f32) -> Size { Size::new(80., role.style().line_height) }
        fn draw(&mut self, _: &mut Scene, _: &str, _: proposal::TextRole, _: Rect, _: ButtonTextAlign, _: Rgba) {}
        fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> { if self.0 { Some(self) } else { None } }
    }
    impl TextSurfaceHost for Text {
        fn shape_surface(&mut self, text: &str, role: proposal::TextRole, width: Option<f32>) -> Rc<Paragraph> {
            let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
            Rc::new(shaper.shape(text, &TextStyle::system(role.style().size), &ShapeOptions { width: width.map(f64::from), line_spacing: 0. }))
        }
    }
    impl TextSurfaceTextRenderer for Text {
        fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
    }
    fn render(host: &mut ContentHost, width: f32, height: f32) {
        let frame = Rect::new(0., 0., width, height);
        host.render(&mut Scene::new(), &mut Text(true), frame, frame, Theme::LIGHT.into()).unwrap();
    }
    const ROOT: ComponentId = ComponentId(82900);
    const FIELD: ComponentId = ComponentId(82901);
    const SCROLL: ComponentId = ComponentId(82902);
    fn field() -> Content { Content::new(FIELD, TextFieldContent::new("retained", "original text 😀")) }

    #[test]
    fn off_focus_layout_uses_original_paragraph_and_stable_selection_binding() {
        let mut host = ContentHost::new(field()).unwrap();
        render(&mut host, 280., 80.);
        let before = host.semantic_text_layout_snapshot(FIELD, None).unwrap();
        assert_eq!(host.focused(), None);
        assert!(!before.lines.is_empty() && !before.carets.is_empty());
        assert_eq!(before.input.text, "original text 😀");
        assert_eq!(host.semantic_text_layout_snapshot(FIELD, Some(0)), None);
        host.get_mut(FIELD).unwrap().component_mut::<TextFieldContent>().unwrap().surface.input.select(1, false);
        render(&mut host, 200., 80.);
        let after = host.semantic_text_layout_snapshot(FIELD, None).unwrap();
        assert_eq!(after.input.binding, before.input.binding);
        assert_eq!(after.input.selection, 1..1);
        assert_eq!(after.input.text, before.input.text);
        assert_eq!(host.focused(), None);
        host.set_window_active(false);
        assert_eq!(host.semantic_text_layout_snapshot(FIELD, None), None);
    }

    #[test]
    fn zero_split_pane_cannot_publish_retained_text_layout() {
        let split = SplitViewContent::try_new(SplitViewConstruction {
            config: SplitViewConfig::default(),
            slots: SplitViewSlotIds { leading: ComponentId(82910), trailing: ComponentId(82911), divider: ComponentId(82912) },
            leading: field(), trailing: Content::new(ComponentId(82913), ButtonContent::new("other")),
            budget: 64 * 1024 * 1024,
        }).unwrap();
        let mut host = ContentHost::try_new_prepared(Content::new(ROOT, split), 64 * 1024 * 1024).unwrap();
        render(&mut host, 600., 200.);
        let before = host.semantic_text_layout_snapshot(FIELD, None).unwrap();
        render(&mut host, 0., 0.);
        assert_eq!(host.semantic_text_layout_snapshot(FIELD, None), None);
        render(&mut host, 600., 200.);
        let after = host.semantic_text_layout_snapshot(FIELD, None).unwrap();
        assert_eq!(after.input.binding, before.input.binding);
        assert!(!after.lines.is_empty() && !after.carets.is_empty());
    }

    fn scroll() -> Content {
        let rows = Content::new(ComponentId(82920), StackContent {
            model: Stack { axis: Axis::Vertical, gap: 2. }, flexible_child: None,
            children: (0..24).map(|i| Content::new(ComponentId(83000+i), TextContent::new(format!("row {i}")))).collect(),
        });
        let mut owner = ScrollContent::new(ScrollArea::default(), rows);
        owner.viewport_height = Some(64.);
        Content::new(SCROLL, owner)
    }
    fn wheel(host: &mut ContentHost) {
        let point = host.get(SCROLL).unwrap().visible().unwrap().center();
        let response = host.event(&ContentEvent::Scroll { position: point, delta: Point::new(0., 24.), precise: true, begin: true, end: true, momentum: false, time: 1. });
        assert!(response.consumed);
        assert_eq!(host.reading_scroll_source(), Some((SCROLL, host.semantic_mount(SCROLL).unwrap())));
    }
    #[test]
    fn reading_source_is_actual_owner_clears_before_refusal_and_expires_on_remount() {
        let mut host = ContentHost::new(Content::new(ROOT, StackContent {
            model: Stack { axis: Axis::Vertical, gap: 2. }, flexible_child: None, children: vec![scroll(), field()],
        })).unwrap();
        render(&mut host, 300., 160.);
        wheel(&mut host);
        host.event(&ContentEvent::PointerMove(Point::new(-50., -50.)));
        assert_eq!(host.reading_scroll_source(), None);
        wheel(&mut host);
        assert!(matches!(host.event_with_text(&ContentEvent::Unmark, &mut Text(false)), Err(CompositionError::MissingTextServices(_))));
        assert_eq!(host.reading_scroll_source(), None);
        wheel(&mut host);
        let old_mount = host.semantic_mount(SCROLL).unwrap();
        host.remove(SCROLL).unwrap();
        host.insert(ROOT, scroll()).unwrap();
        assert_ne!(host.semantic_mount(SCROLL), Some(old_mount));
        assert_eq!(host.reading_scroll_source(), None);
        render(&mut host, 300., 160.);
        wheel(&mut host);
    }
}
