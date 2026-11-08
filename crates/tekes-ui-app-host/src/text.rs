use std::{collections::HashMap, rc::Rc};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlMotionContext};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    push_paragraph,
};
use tekes_ui_tokens::{Rgba, proposal};
/// Opt-in per-draw CPU observations, reset by the original begin_frame call.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ParagraphLayoutDiagnostic {
    pub requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub unique_entries: usize,
    pub prepare_count: u64,
    pub prepare_ns: u128,
    pub natural_count: u64,
    pub natural_ns: u128,
    pub bounded_count: u64,
    pub bounded_ns: u128,
}
/// Exact logical layout inputs. Draw position, color, clip and backing scale
/// do not affect CoreText's point-space paragraph and are applied on every draw.
#[derive(Debug, PartialEq, Eq, Hash)]
struct ParagraphLayoutKey {
    text: String,
    role: proposal::TextRole,
    type_scale: u32,
    width: u32,
}
pub struct Text {
    pub atlas: GlyphAtlas,
    shaper: CoreTextShaper,
    raster: CoreTextRasterizer,
    pub scale: f32,
    type_scale: f32,
    motion_context: Option<ControlMotionContext>,
    animation_requested: bool,
    paragraph_diagnostic: Option<ParagraphLayoutDiagnostic>,
    frame_paragraphs: Option<HashMap<ParagraphLayoutKey, (Rc<tekes_ui_text::Paragraph>, f32)>>,
}
impl Text {
    pub fn new() -> Self {
        let f = CoreTextFonts::new();
        Self {
            atlas: GlyphAtlas::default(),
            shaper: CoreTextShaper::new(f.clone()),
            raster: CoreTextRasterizer::new(f),
            scale: 1.,
            type_scale: 1.,
            motion_context: None,
            animation_requested: false,
            frame_paragraphs: None,
            paragraph_diagnostic: (std::env::var("TEKESUI_APPHOST_DRAW_DIAGNOSTIC").as_deref()
                == Ok("1"))
            .then(ParagraphLayoutDiagnostic::default),
        }
    }
    fn trimmed_advance(paragraph: &tekes_ui_text::Paragraph) -> f32 {
        paragraph
            .lines
            .iter()
            .map(|line| line.width_trimmed)
            .fold(0., f64::max) as f32
    }
    // Keep native advances intact. Wrapped display layout occupies the requested
    // slot: trailing blanks cannot enlarge it or shrink it before draw/reflow.
    fn paragraph_layout(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> (Rc<tekes_ui_text::Paragraph>, f32) {
        // Direct public measure/draw callers need not have a frame lifecycle.
        // Only an explicit begin_frame opts into this frame's retained layouts.
        let key = self.frame_paragraphs.as_ref().map(|_| ParagraphLayoutKey {
            text: value.to_owned(),
            role,
            type_scale: self.type_scale.to_bits(),
            width: width.to_bits(),
        });
        if let Some(stats) = &mut self.paragraph_diagnostic {
            stats.requests += 1;
        }
        if let Some(layout) = key
            .as_ref()
            .and_then(|key| self.frame_paragraphs.as_ref()?.get(key))
        {
            if let Some(stats) = &mut self.paragraph_diagnostic {
                stats.hits += 1;
            }
            return layout.clone();
        }
        if let Some(stats) = &mut self.paragraph_diagnostic {
            stats.misses += 1;
        }
        let metrics = role.style();
        let style = TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight);
        let begin = self.paragraph_diagnostic.map(|_| std::time::Instant::now());
        // Both width queries describe the same text/style. Keep its native
        // attributed string and typesetter for this miss only; retain just the layout.
        let prepared = self.shaper.prepare(value, &style);
        if let (Some(begin), Some(stats)) = (begin, &mut self.paragraph_diagnostic) {
            stats.prepare_count += 1;
            stats.prepare_ns += begin.elapsed().as_nanos();
        }
        let begin = self.paragraph_diagnostic.map(|_| std::time::Instant::now());
        let natural = self
            .shaper
            .shape_prepared(&prepared, &ShapeOptions::default());
        if let (Some(begin), Some(stats)) = (begin, &mut self.paragraph_diagnostic) {
            stats.natural_count += 1;
            stats.natural_ns += begin.elapsed().as_nanos();
        }
        let intrinsic = Self::trimmed_advance(&natural);
        let (paragraph, slot) = if !width.is_finite() || intrinsic <= width.max(0.) + 1. / 64. {
            (natural, intrinsic)
        } else {
            let slot = width.max(1.);
            let begin = self.paragraph_diagnostic.map(|_| std::time::Instant::now());
            let bounded = self
                .shaper
                .shape_prepared(&prepared, &ShapeOptions::with_width(f64::from(slot)));
            if let (Some(begin), Some(stats)) = (begin, &mut self.paragraph_diagnostic) {
                stats.bounded_count += 1;
                stats.bounded_ns += begin.elapsed().as_nanos();
            }
            (bounded, slot)
        };
        let paragraph = Rc::new(paragraph);
        if let (Some(cache), Some(key)) = (&mut self.frame_paragraphs, key) {
            cache.insert(key, (paragraph.clone(), slot));
            if let Some(stats) = &mut self.paragraph_diagnostic {
                stats.unique_entries = cache.len();
            }
        }
        (paragraph, slot)
    }
    pub(crate) fn paragraph_layout_diagnostic(&self) -> Option<ParagraphLayoutDiagnostic> {
        self.paragraph_diagnostic
    }
    pub fn type_scale(&self) -> f32 {
        self.type_scale
    }
    pub fn begin_frame(&mut self, now: std::time::Duration, reduced_motion: bool) {
        self.motion_context = Some(ControlMotionContext {
            now,
            reduced_motion,
        });
        self.animation_requested = false;
        self.frame_paragraphs
            .get_or_insert_with(HashMap::new)
            .clear();
        if let Some(stats) = &mut self.paragraph_diagnostic {
            *stats = ParagraphLayoutDiagnostic::default();
        }
    }
    pub fn animation_requested(&self) -> bool {
        self.animation_requested
    }
    pub fn set_body_size(&mut self, size: f32) -> bool {
        let scale = size / proposal::TextRole::UiBody.style().size;
        if !scale.is_finite() || scale <= 0. || self.type_scale == scale {
            return false;
        }
        self.type_scale = scale;
        if let Some(cache) = &mut self.frame_paragraphs {
            cache.clear();
        }
        if let Some(stats) = &mut self.paragraph_diagnostic {
            stats.unique_entries = 0;
        }
        true
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.motion_context
    }
    fn request_animation_frame(&mut self) {
        self.animation_requested = true;
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * self.type_scale
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let (paragraph, slot) = self.paragraph_layout(text, role, width);
        Size::new(slot, paragraph.height as f32)
    }
    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, width: f32) -> f32 {
        let (paragraph, _) = self.paragraph_layout(text, role, width);
        paragraph
            .lines
            .first()
            .map_or(paragraph.default_line_height as f32, |line| {
                line.height as f32
            })
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        r: Rect,
        align: ButtonTextAlign,
        color: Rgba,
    ) {
        let (p, _) = self.paragraph_layout(text, role, r.size.width);
        let x = if align == ButtonTextAlign::Center {
            r.min_x() + (r.size.width - Self::trimmed_advance(&p)) * 0.5
        } else {
            r.min_x()
        };
        let y = r.min_y() + (r.size.height - p.height as f32) * 0.5;
        scene.push_clip(r);
        push_paragraph(
            scene,
            &mut self.atlas,
            &mut self.raster,
            &p,
            Point::new(x.round(), y.round()),
            self.scale,
            color,
        );
        scene.pop_clip();
    }
}

impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &tekes_ui_text::Paragraph,
        origin: Point,
        color: Rgba,
        clip: Rect,
    ) {
        tekes_ui_text::push_paragraph_visible(
            scene,
            &mut self.atlas,
            &mut self.raster,
            paragraph,
            origin,
            self.scale,
            color,
            clip.min_y()..clip.max_y(),
        );
    }
}
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_label(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        let metrics = role.style();
        Some(std::rc::Rc::new(
            self.shaper
                .shape_label(
                    value,
                    &TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight),
                    &ShapeOptions::with_width(f64::from(width)),
                    options,
                )
                .with_min_line_height(f64::from(metrics.line_height * self.type_scale)),
        ))
    }
    fn shape_semantic_label(
        &mut self,
        value: &tekes_ui_text::SemanticLabel,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        let metrics = role.style();
        Some(std::rc::Rc::new(
            self.shaper
                .shape_semantic_label(
                    value,
                    &TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight),
                    &ShapeOptions::with_width(f64::from(width)),
                    options,
                )
                .with_min_line_height(f64::from(metrics.line_height * self.type_scale)),
        ))
    }

    fn type_scale(&self) -> f32 {
        self.type_scale
    }
    fn shape_surface(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        let metrics = role.style();
        let mut paragraph = self.shaper.shape(
            value,
            &TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        );
        let mut top = 0.;
        for line in &mut paragraph.lines {
            let height = f64::from(metrics.line_height * self.type_scale).max(line.height);
            line.top = top;
            line.height = height;
            line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
            top += height;
        }
        paragraph.height = top.max(f64::from(metrics.line_height * self.type_scale));
        paragraph.default_line_height = f64::from(metrics.line_height * self.type_scale);
        std::rc::Rc::new(paragraph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tekes_ui_controls::{
        composition::*,
        extended::{Axis, Stack},
        layout_view::IconRenderer,
    };
    use tekes_ui_tokens::Theme;

    #[derive(Default)]
    struct Icons(Vec<String>);
    impl IconRenderer for Icons {
        fn prepare_icon(
            &mut self,
            name: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> Result<IconImage, IconRenderError> {
            assert!(tekes_ui_icons::Glyph::named(name).is_some());
            self.0.push(name.into());
            Ok(IconImage::new(tekes_ui_scene::ImageId(99), Rc::new(())))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("original Content must prepare its icon resource");
        }
    }
    #[test]
    fn frame_memo_repaints_current_scene_and_expires_on_frame_or_font_change() {
        let source = "A shared paragraph keeps reading stable while a narrow document changes its position and color. 中文と日本語も保持します。";
        let role = proposal::TextRole::UiBody;
        let mut text = Text::new();
        text.paragraph_diagnostic = Some(ParagraphLayoutDiagnostic::default());
        // Legacy/direct callers without begin_frame never retain arbitrary keys.
        for width in [180., 181., 182.] {
            text.measure(source, role, width);
        }
        assert!(text.frame_paragraphs.is_none());
        text.begin_frame(std::time::Duration::ZERO, false);
        let (original, _) = text.paragraph_layout(source, role, 180.);
        text.measure(source, role, 180.);
        text.first_line_height(source, role, 180.);
        for (scale, frame, color, align) in [
            (
                1.,
                Rect::new(12.25, 14.75, 180., 320.),
                Rgba::gray(0.),
                ButtonTextAlign::Leading,
            ),
            (
                2.,
                Rect::new(54.5, 28.25, 180., 340.),
                Rgba::new(0.2, 0.4, 0.8, 1.),
                ButtonTextAlign::Center,
            ),
        ] {
            text.scale = scale;
            let mut expected = Scene::new();
            expected.push_clip(frame);
            let x = if align == ButtonTextAlign::Center {
                frame.min_x() + (frame.size.width - Text::trimmed_advance(&original)) * 0.5
            } else {
                frame.min_x()
            };
            let y = frame.min_y() + (frame.size.height - original.height as f32) * 0.5;
            push_paragraph(
                &mut expected,
                &mut text.atlas,
                &mut text.raster,
                &original,
                Point::new(x.round(), y.round()),
                scale,
                color,
            );
            expected.pop_clip();
            let mut actual = Scene::new();
            text.draw(&mut actual, source, role, frame, align, color);
            assert_eq!(
                actual, expected,
                "memo must never retain a Scene, position, color, clip or raster scale"
            );
        }
        let stats = text.paragraph_layout_diagnostic().unwrap();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.prepare_count, 1);
        assert_eq!(stats.hits, 4);
        let (resized, _) = text.paragraph_layout(source, role, 90.);
        assert!(resized.height > original.height);
        let (edited, _) = text.paragraph_layout("A changed paragraph", role, 180.);
        assert_ne!(edited.as_ref(), original.as_ref());
        let (heading, _) = text.paragraph_layout(source, proposal::TextRole::UiHeading, 180.);
        assert!(!Rc::ptr_eq(&heading, &original));
        assert!(text.set_body_size(20.));
        assert!(text.frame_paragraphs.as_ref().unwrap().is_empty());
        let (larger, _) = text.paragraph_layout(source, role, 180.);
        assert!(larger.height > original.height);
        text.begin_frame(std::time::Duration::from_millis(1), false);
        assert!(text.frame_paragraphs.as_ref().unwrap().is_empty());
        let (next_frame, _) = text.paragraph_layout(source, role, 180.);
        assert!(!Rc::ptr_eq(&larger, &next_frame));
        assert_eq!(larger, next_frame);
    }
    #[test]
    fn actual_scroll_stack_measure_and_paint_reuse_frame_layout_without_stale_reflow() {
        let mut text = Text::new();
        text.paragraph_diagnostic = Some(ParagraphLayoutDiagnostic::default());
        text.begin_frame(std::time::Duration::ZERO, false);
        let mut host = ContentHost::new(Content::new(ComponentId(1), ScrollContent::new(
            tekes_ui_controls::ScrollArea::default(), Content::new(ComponentId(2), StackContent {
                model: Stack { axis: Axis::Vertical, gap: 8. }, flexible_child: None,
                children: vec![
                    Content::new(ComponentId(3), TextContent::new("A long document paragraph wraps across many lines while the reader makes the window narrower. 中文と日本語も同じ本文に表示します。")),
                    Content::new(ComponentId(4), TextContent::new("Another paragraph remains below the first.")),
                ],
            })))).unwrap();
        let frame = Rect::new(10., 20., 180., 160.);
        let mut first = Scene::new();
        host.render(&mut first, &mut text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        let wide_height = host.get(ComponentId(3)).unwrap().frame().size.height;
        let stats = text.paragraph_layout_diagnostic().unwrap();
        assert!(
            stats.hits > 0,
            "original Scroll/Stack measure/arrange/paint must reach the same layout more than once"
        );
        assert_eq!(stats.requests, stats.hits + stats.misses);
        assert_eq!(stats.prepare_count, stats.misses);
        assert_eq!(stats.natural_count, stats.misses);
        assert_eq!(stats.unique_entries as u64, stats.misses);
        let mut repeated = Scene::new();
        host.render(&mut repeated, &mut text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        assert_eq!(first, repeated);
        let narrow = Rect::new(10., 20., 120., 160.);
        let mut resized = Scene::new();
        host.render(&mut resized, &mut text, narrow, narrow, Theme::LIGHT.into())
            .unwrap();
        assert!(host.get(ComponentId(3)).unwrap().frame().size.height > wide_height);
        text.begin_frame(std::time::Duration::ZERO, false);
        let mut rebuilt = Scene::new();
        host.render(&mut rebuilt, &mut text, narrow, narrow, Theme::LIGHT.into())
            .unwrap();
        assert_eq!(
            resized, rebuilt,
            "new frame rebuilds layout with unchanged actual output"
        );
    }
    #[test]
    fn one_prepared_paragraph_matches_original_shape_entry_for_lines_glyphs_and_draw() {
        let mut text = Text::new();
        text.begin_frame(std::time::Duration::ZERO, false);
        for source in [
            "A quieter workspace keeps reading calm while the window becomes narrower.   ",
            "中文与日本語の段落。改行しても文字と余白を保持します。\n次の行 🧑🏽‍💻 fi ffi a\u{301}",
            "العربية עברית reading 123 with mixed directions and a final line.\n",
        ] {
            for body_size in [proposal::TextRole::UiBody.style().size, 20.] {
                text.set_body_size(body_size);
                let role = proposal::TextRole::UiBody;
                let metrics = role.style();
                let style =
                    TextStyle::system(metrics.size * text.type_scale).with_weight(metrics.weight);
                for width in [f32::INFINITY, 700., 333.3333, 180., 1., 0., -1.] {
                    // Reference the previous production entry: each shape owns
                    // its prepare, with the original width decision unchanged.
                    let natural = text.shaper.shape(source, &style, &ShapeOptions::default());
                    let intrinsic = Text::trimmed_advance(&natural);
                    let (reference, slot) =
                        if !width.is_finite() || intrinsic <= width.max(0.) + 1. / 64. {
                            (natural, intrinsic)
                        } else {
                            let slot = width.max(1.);
                            (
                                text.shaper.shape(
                                    source,
                                    &style,
                                    &ShapeOptions::with_width(f64::from(slot)),
                                ),
                                slot,
                            )
                        };
                    let (actual, actual_slot) = text.paragraph_layout(source, role, width);
                    assert_eq!(actual_slot, slot);
                    // Includes line and glyph identity, UTF ranges, native
                    // advances, baselines, carets and complete paragraph geometry.
                    assert_eq!(
                        actual.as_ref(),
                        &reference,
                        "width={width} body_size={body_size} text={source:?}"
                    );
                    if width.is_finite() && width > 0. {
                        let frame = Rect::new(12.25, 14.75, width, reference.height as f32 + 20.);
                        let mut expected_scene = Scene::new();
                        expected_scene.push_clip(frame);
                        push_paragraph(
                            &mut expected_scene,
                            &mut text.atlas,
                            &mut text.raster,
                            &reference,
                            Point::new(frame.min_x().round(), (frame.min_y() + 10.).round()),
                            text.scale,
                            Rgba::gray(0.),
                        );
                        expected_scene.pop_clip();
                        let mut actual_scene = Scene::new();
                        text.draw(
                            &mut actual_scene,
                            source,
                            role,
                            frame,
                            ButtonTextAlign::Leading,
                            Rgba::gray(0.),
                        );
                        assert_eq!(
                            actual_scene, expected_scene,
                            "draw width={width} body_size={body_size}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn display_intrinsic_and_centering_exclude_trailing_blanks_but_native_advances_do_not() {
        let mut text = Text::new();
        let role = proposal::TextRole::UiBody;
        assert_eq!(
            text.measure("A", role, 200.),
            text.measure("A    ", role, 200.)
        );
        let (native, slot) = text.paragraph_layout("A    ", role, 200.);
        assert!(native.width > f64::from(slot));
        assert_eq!(slot, Text::trimmed_advance(&native));
        let frame = Rect::new(0., 0., 200., 40.);
        let mut origins = Vec::new();
        for value in ["A", "A    "] {
            let mut scene = Scene::new();
            text.draw(
                &mut scene,
                value,
                role,
                frame,
                ButtonTextAlign::Center,
                Rgba::gray(0.),
            );
            origins.push(
                scene
                    .primitives()
                    .iter()
                    .find_map(|primitive| {
                        if let tekes_ui_scene::Primitive::Glyphs(run) = primitive {
                            run.glyphs.first().map(|glyph| glyph.origin.x)
                        } else {
                            None
                        }
                    })
                    .unwrap(),
            );
        }
        assert_eq!(origins[0], origins[1]);
    }
    #[test]
    fn actual_apphost_text_reflows_at_finite_width_with_matching_first_line_and_draw() {
        let mut text = Text::new();
        text.begin_frame(std::time::Duration::ZERO, false);
        let source = "Readable paragraphs keep their normal type size while the available document width changes. 中文内容也应随实际窗口宽度自然换行，不能被当成一行裁掉。";
        let role = proposal::TextRole::UiBody;
        let original_size = text.resolved_font_size(role);
        let wide = text.measure(source, role, 900.);
        let narrow = text.measure(source, role, 180.);
        assert!(narrow.height > wide.height);
        let (diagnostic, _) = text.paragraph_layout(source, role, 180.);
        for line in &diagnostic.lines {
            let mut ink: Option<(f64, f64)> = None;
            for run in &line.runs {
                for glyph in &run.glyphs {
                    use tekes_ui_text::Rasterizer;
                    if let Some(bitmap) = text.raster.rasterize(run.font, glyph.id, 2.) {
                        for (index, pixel) in bitmap.pixels.chunks_exact(4).enumerate() {
                            if pixel[3] != 0 {
                                let x = (index % bitmap.width as usize) as f64;
                                let left = glyph.x + (f64::from(bitmap.left) + x) / 2.;
                                let right = left + 0.5;
                                ink =
                                    Some(ink.map_or((left, right), |(a, b)| {
                                        (a.min(left), b.max(right))
                                    }));
                            }
                        }
                    }
                }
            }
            assert!(
                line.width_trimmed <= 180. + 1. / 64.,
                "trimmed advance exceeds requested slot: {line:?}"
            );
            assert!(
                ink.is_none_or(|(_, right)| right <= 180. + 1. / 64.),
                "raster ink exceeds requested slot: {ink:?}"
            );
            eprintln!(
                "APPHOST_TEXT_LINE index={} utf16={:?} bytes={:?} advance={} trimmed_advance={} raster_ink_2x={:?} text={:?}",
                line.index,
                line.utf16,
                line.bytes,
                line.width,
                line.width_trimmed,
                ink,
                &source[line.bytes.clone()]
            );
        }
        assert!(
            narrow.width <= 180. + 1. / 64.,
            "measured={narrow:?} raw_paragraph_width={} lines={:?}",
            diagnostic.width,
            diagnostic
                .lines
                .iter()
                .map(|line| (line.width, line.width_trimmed, line.bytes.clone()))
                .collect::<Vec<_>>()
        );
        assert_eq!(text.resolved_font_size(role), original_size);
        let (paragraph, slot) = text.paragraph_layout(source, role, 180.);
        assert!(paragraph.lines.len() > 1);
        assert_eq!(
            text.first_line_height(source, role, 180.),
            paragraph.lines[0].height as f32
        );
        assert_eq!(narrow, Size::new(slot, paragraph.height as f32));
        assert_eq!(slot, 180.);
        assert_eq!(
            paragraph.width,
            paragraph
                .lines
                .iter()
                .map(|line| line.width)
                .fold(0., f64::max),
            "native advance metadata remains intact"
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                children: vec![
                    Content::new(ComponentId(2), TextContent::new(source)),
                    Content::new(ComponentId(3), TextContent::new("Next paragraph")),
                ],
                flexible_child: None,
            },
        ))
        .unwrap();
        let frame = Rect::new(0., 0., 180., 400.);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let actual = host.get(ComponentId(2)).unwrap().frame();
        assert_eq!(
            actual.size, narrow,
            "leading Stack passes the same measured wrap width to draw"
        );
        assert_eq!(
            host.get(ComponentId(3)).unwrap().frame().min_y(),
            actual.max_y() + 8.
        );
        let mut scene = Scene::new();
        text.draw(
            &mut scene,
            source,
            role,
            Rect::new(0., 0., 180., narrow.height),
            ButtonTextAlign::Leading,
            Rgba::gray(0.),
        );
        assert!(!scene.primitives().is_empty());
        assert!(text.set_body_size(original_size * 1.25));
        assert!(text.measure(source, role, 180.).height > narrow.height);
    }
    #[test]
    fn actual_apphost_text_and_icon_service_render_original_disclosure_and_toolbar_slots() {
        let disclosure = Content::new(
            ComponentId(7002),
            DisclosureContent::new(
                ComponentId(7003),
                Content::new(ComponentId(7004), TextContent::new("Document properties")),
                Content::new(
                    ComponentId(100),
                    TextFieldContent::new("Title", "A real document"),
                ),
                true,
            ),
        );
        let toolbar = Content::new(
            ComponentId(7100),
            ToolbarContent::new(vec![Content::new(
                ComponentId(7101),
                ToolbarGroup::new(
                    ToolbarPlacement::PrimaryAction,
                    vec![Content::new(
                        ComponentId(7102),
                        Button::new(Content::new(
                            ComponentId(7103),
                            IconContent::new("tekes-settings").unwrap(),
                        ))
                        .with_accessible_name("Settings"),
                    )],
                ),
            )]),
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(7200),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 12.,
                },
                children: vec![disclosure, toolbar],
                flexible_child: None,
            },
        ))
        .unwrap();
        let mut text = Text::new();
        text.begin_frame(std::time::Duration::ZERO, false);
        let frame = Rect::new(16., 16., 360., 420.);
        assert!(matches!(
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into()
            ),
            Err(CompositionError::MissingIconServices(_))
        ));
        let mut icons = Icons::default();
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut text, &mut icons),
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(icons.0.iter().any(|name| name == "tekes-settings"));
        assert!(
            icons.0.len() >= 2,
            "Disclosure and Toolbar both consume the original icon service"
        );
        assert!(
            scene
                .primitives()
                .iter()
                .any(|primitive| matches!(primitive, tekes_ui_scene::Primitive::Image(_)))
        );
        assert!(
            host.semantics()
                .iter()
                .any(|node| node.id == ComponentId(7003) && node.semantics.expanded == Some(true))
        );
    }
}
