use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlMotionContext};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    push_paragraph,
};
use tekes_ui_tokens::{Rgba, proposal};
pub struct Text {
    pub atlas: GlyphAtlas,
    shaper: CoreTextShaper,
    raster: CoreTextRasterizer,
    pub scale: f32,
    type_scale: f32,
    motion_context: Option<ControlMotionContext>,
    animation_requested: bool,
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
    ) -> (tekes_ui_text::Paragraph, f32) {
        let metrics = role.style();
        let style = TextStyle::system(metrics.size * self.type_scale).with_weight(metrics.weight);
        let natural = self.shaper.shape(value, &style, &ShapeOptions::default());
        let intrinsic = Self::trimmed_advance(&natural);
        if !width.is_finite() || intrinsic <= width.max(0.) + 1. / 64. {
            (natural, intrinsic)
        } else {
            let slot = width.max(1.);
            (
                self.shaper
                    .shape(value, &style, &ShapeOptions::with_width(f64::from(slot))),
                slot,
            )
        }
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
