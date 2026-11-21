use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlMotionContext};
use tekes_ui_macos_bridge::coretext_host::CoreTextHost;
pub(crate) use tekes_ui_macos_bridge::coretext_host::ParagraphLayoutDiagnostic;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, proposal};

/// Shared CoreText host adapter for AppHost windows, delegating shaping,
/// glyph atlas rasterization, and composition text-service traits to [`CoreTextHost`].
pub struct Text(pub CoreTextHost);

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Text").field(&self.0).finish()
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Text {
    type Target = CoreTextHost;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Text {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Text {
    pub fn new() -> Self {
        let diagnostic = std::env::var("TEKESUI_APPHOST_DRAW_DIAGNOSTIC").as_deref() == Ok("1");
        Self(CoreTextHost::new().with_diagnostic(diagnostic))
    }

    pub fn animation_requested(&self) -> bool {
        self.0.animation_requested
    }

    pub fn type_scale(&self) -> f32 {
        self.0.type_scale
    }

    pub(crate) fn paragraph_layout_diagnostic(&self) -> Option<ParagraphLayoutDiagnostic> {
        self.0.paragraph_layout_diagnostic()
    }

    #[cfg(test)]
    pub(crate) fn trimmed_advance(paragraph: &tekes_ui_text::Paragraph) -> f32 {
        tekes_ui_macos_bridge::coretext_host::trimmed_advance(paragraph)
    }
}

impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.0.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.0.request_animation_frame();
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        self.0.measure(text, role, width)
    }
    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, width: f32) -> f32 {
        self.0.first_line_height(text, role, width)
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
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
        self.0.draw(scene, text, role, r, align, color);
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
        self.0.push_visible(scene, paragraph, origin, color, clip);
    }
}

impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_system_text_with_weight(
        &mut self,
        value: &str,
        final_logical_point_size: f32,
        weight: u16,
        width: Option<f32>,
    ) -> Option<Rc<tekes_ui_text::Paragraph>> {
        self.0
            .shape_system_text_with_weight(value, final_logical_point_size, weight, width)
    }
    fn shape_system_text(
        &mut self,
        value: &str,
        final_logical_point_size: f32,
        width: Option<f32>,
    ) -> Option<Rc<tekes_ui_text::Paragraph>> {
        self.0
            .shape_system_text(value, final_logical_point_size, width)
    }
    fn shape_label(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        self.0.shape_label(value, role, width, options)
    }
    fn shape_semantic_label(
        &mut self,
        value: &tekes_ui_text::SemanticLabel,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        self.0.shape_semantic_label(value, role, width, options)
    }
    fn type_scale(&self) -> f32 {
        self.0.type_scale()
    }
    fn shape_surface(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        self.0.shape_surface(value, role, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use tekes_ui_controls::{composition::*, layout_view::IconRenderer};
    use tekes_ui_scene::GlyphAtlas;
    use tekes_ui_text::{
        ShapeOptions, Shaper, TextStyle,
        coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
        push_paragraph,
    };
    use tekes_ui_tokens::Theme;

    #[test]
    fn final_system_size_shapes_real_digits_without_role_minimum_or_double_scale() {
        use tekes_ui_controls::{TextSurfaceHost, TextSurfaceTextRenderer};
        let mut text = Text::new();
        for scale in [1., 1.5] {
            text.set_body_size(proposal::TextRole::UiBody.style().size * scale);
            let large = text.shape_system_text("8", 64., None).unwrap();
            let small = text.shape_system_text("8", 12., None).unwrap();
            assert!((large.fonts[0].size - 64.).abs() < 1e-6);
            assert!((small.fonts[0].size - 12.).abs() < 1e-6);
            assert!(small.width < large.width && small.height < large.height);
            let mut scene = Scene::new();
            text.push_visible(
                &mut scene,
                &large,
                Point::new(0., 0.),
                Rgba::rgb8(0, 0, 0),
                Rect::new(0., 0., 200., 200.),
            );
            assert!(
                !scene.primitives().is_empty(),
                "same host font registry and atlas must rasterize its shaped digit"
            );
        }
    }

    #[test]
    fn calibration_final_style_preserves_layout_across_font_registries_and_ui_scale() {
        use tekes_ui_controls::{TextSurfaceHost, TextSurfaceTextRenderer};
        let mut text = Text::new();
        text.shaper.shape(
            "already registered mono",
            &TextStyle::named("Menlo", 19.),
            &ShapeOptions::default(),
        );
        text.set_body_size(proposal::TextRole::UiBody.style().size * 1.5);
        let raw = include_str!("../../../fixtures/text/mixed-cjk-latin.txt");
        let mut independent = CoreTextShaper::new(CoreTextFonts::new());
        for weight in [400, 600] {
            let actual = text
                .shape_system_text_with_weight(raw, 14., weight, Some(320.))
                .unwrap();
            let expected = independent.shape(
                raw,
                &TextStyle::system(14.).with_weight(weight),
                &ShapeOptions::with_width(320.),
            );
            assert_eq!(actual.height, expected.height);
            assert_eq!(actual.default_line_height, expected.default_line_height);
            assert_eq!(actual.lines.len(), expected.lines.len());
            for (a, b) in actual.lines.iter().zip(&expected.lines) {
                assert_eq!(a.utf16, b.utf16);
                assert_eq!((a.top, a.height, a.width), (b.top, b.height, b.width));
                for (ar, br) in a.runs.iter().zip(&b.runs) {
                    let af = actual.font(ar.font).unwrap();
                    let bf = expected.font(br.font).unwrap();
                    assert_eq!(
                        (&af.post_script_name, af.size),
                        (&bf.post_script_name, bf.size)
                    );
                    assert_eq!(ar.glyphs, br.glyphs);
                }
            }
            let whole_lines = |p: &tekes_ui_text::Paragraph| {
                p.lines
                    .iter()
                    .take_while(|line| line.top + line.height <= 170.)
                    .count()
            };
            assert_eq!(whole_lines(&actual), whole_lines(&expected));
            let mut scene = Scene::new();
            text.push_visible(
                &mut scene,
                &actual,
                Point::new(0., 0.),
                Rgba::gray(0.),
                Rect::new(0., 0., 320., 170.),
            );
            assert!(
                !scene.primitives().is_empty(),
                "visible text must rasterize with its own font registry"
            );
        }
        let regular = text
            .shape_system_text_with_weight("MMMM WWWW", 14., 400, None)
            .unwrap();
        let semibold = text
            .shape_system_text_with_weight("MMMM WWWW", 14., 600, None)
            .unwrap();
        assert_ne!(
            regular.width, semibold.width,
            "requested weight must affect actual shaping"
        );
    }

    #[test]
    #[ignore = "requires the registered GPU acceptance lane"]
    fn actual_window_text_survives_measurement_atlas_switch_on_shared_renderer() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let mut text = Text::new();
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            TextContent::new("AppHost window — 文本恢复，office ffi 😀"),
        ))
        .unwrap();
        let window_pixels =
            |renderer: &mut tekes_ui_render::Renderer, text: &mut Text, host: &mut ContentHost| {
                text.begin_frame(std::time::Duration::ZERO, false);
                let frame = Rect::new(0., 0., 600., 200.);
                let mut scene = Scene::new();
                host.render(&mut scene, text, frame, frame, Theme::LIGHT.into())
                    .unwrap();
                renderer
                    .render_to_rgba8(&scene, &mut text.atlas, (600, 200), 1., Rgba::gray(1.))
                    .unwrap()
            };
        let before = window_pixels(&mut renderer, &mut text, &mut host);
        assert!(before.chunks_exact(4).any(|p| p[0] < 200));
        let fonts = CoreTextFonts::new();
        let mut measurement = CoreTextShaper::new(fonts.clone());
        let mut raster = CoreTextRasterizer::new(fonts);
        let paragraph = measurement.shape(
            include_str!("../../../fixtures/text/mixed-cjk-latin.txt"),
            &TextStyle::system(14.),
            &ShapeOptions::with_width(320.),
        );
        let mut atlas = GlyphAtlas::default();
        atlas.begin_frame();
        let mut scene = Scene::new();
        push_paragraph(
            &mut scene,
            &mut atlas,
            &mut raster,
            &paragraph,
            Point::new(0., 0.),
            1.,
            Rgba::gray(0.),
        );
        renderer
            .render_to_rgba8(
                &scene,
                &mut atlas,
                (320, paragraph.height.ceil() as u32),
                1.,
                Rgba::gray(1.),
            )
            .unwrap();
        assert_eq!(window_pixels(&mut renderer, &mut text, &mut host), before);
    }

    #[test]
    fn actual_apphost_text_respects_code_family_and_ui_glyph_widths() {
        use tekes_ui_controls::TextSurfaceHost;
        use tekes_ui_text::{LabelSourceVersion, SemanticLabel};
        let mut text = Text::new();
        let semantic = SemanticLabel::new(
            LabelSourceVersion {
                source: 1,
                revision: 1,
            },
            "iiiiWWWW",
            vec![],
        )
        .unwrap();
        for scale in [1., 1.5] {
            text.set_body_size(proposal::TextRole::UiBody.style().size * scale);
            for role in [proposal::TextRole::Code, proposal::TextRole::UiBody] {
                let (narrow, narrow_width) = text.paragraph_layout("iiii", role, f32::INFINITY);
                let (wide, wide_width) = text.paragraph_layout("WWWW", role, f32::INFINITY);
                let family = &narrow.fonts[0].post_script_name;
                assert_eq!(family.contains("Menlo"), role == proposal::TextRole::Code);
                assert_eq!(&wide.fonts[0].post_script_name, family);
                assert!(narrow_width > 0. && wide_width > 0.);
                assert!(!narrow.lines[0].runs[0].glyphs.is_empty());
                if role == proposal::TextRole::Code {
                    assert!(
                        (narrow_width - wide_width).abs() < 1. / 64.,
                        "equal character counts keep equal mono advances"
                    );
                } else {
                    assert!(
                        wide_width - narrow_width > 1.,
                        "ordinary UI text retains proportional glyph advances"
                    );
                }
                let label = text
                    .shape_label("iiiiWWWW", role, 400., Default::default())
                    .unwrap();
                let styled = text
                    .shape_semantic_label(&semantic, role, 400., Default::default())
                    .unwrap();
                let surface = text.shape_surface("iiiiWWWW", role, Some(400.));
                for paragraph in [label.paragraph(), styled.paragraph(), surface.as_ref()] {
                    assert_eq!(
                        &paragraph.fonts[0].post_script_name, family,
                        "every original text entry point consumes the role family"
                    );
                    assert!(
                        (paragraph.fonts[0].size - f64::from(role.style().size * scale)).abs()
                            < 1e-6
                    );
                }
            }
        }
    }

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
        assert!(!text.has_frame_paragraphs());
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
                &mut text.0.atlas,
                &mut text.0.raster,
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
        assert_eq!(text.frame_paragraph_count(), Some(0));
        let (larger, _) = text.paragraph_layout(source, role, 180.);
        assert!(larger.height > original.height);
        text.begin_frame(std::time::Duration::from_millis(1), false);
        assert_eq!(text.frame_paragraph_count(), Some(0));
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
            tekes_ui_controls::ScrollArea::default(), Content::new(ComponentId(2), StackContent::vertical(8., vec![
                Content::new(ComponentId(3), TextContent::new("A long document paragraph wraps across many lines while the reader makes the window narrower. 中文と日本語も同じ本文に表示します。")),
                Content::new(ComponentId(4), TextContent::new("Another paragraph remains below the first.")),
            ]))))).unwrap();
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
                            &mut text.0.atlas,
                            &mut text.0.raster,
                            &reference,
                            Point::new(frame.min_x().round(), (frame.min_y() + 10.).round()),
                            text.0.scale,
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
            StackContent::vertical(
                8.,
                vec![
                    Content::new(ComponentId(2), TextContent::new(source)),
                    Content::new(ComponentId(3), TextContent::new("Next paragraph")),
                ],
            ),
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
            StackContent::vertical(12., vec![disclosure, toolbar]),
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
