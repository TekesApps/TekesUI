//! Native review font adapter derived from the existing panel-module demo.
//! Uses original CoreText preparation and glyph pushing; no editor/layout engine.
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper, PreparedText},
    push_paragraph_visible,
};
use tekes_ui_tokens::{Rgba, proposal};

pub struct Fonts {
    shaper: CoreTextShaper,
    raster: CoreTextRasterizer,
    prepared: Vec<(String, proposal::TextRole, PreparedText)>,
    pub type_scale: f32,
}
impl Fonts {
    pub fn new() -> Self {
        let fonts = CoreTextFonts::new();
        Self {
            shaper: CoreTextShaper::new(fonts.clone()),
            raster: CoreTextRasterizer::new(fonts),
            prepared: Vec::new(),
            type_scale: 1.,
        }
    }
    pub fn set_type_scale(&mut self, scale: f32) {
        if self.type_scale != scale {
            self.prepared.clear();
            self.type_scale = scale;
        }
    }
    fn shape(&mut self, text: &str, role: proposal::TextRole, width: Option<f32>) -> Rc<Paragraph> {
        let mut metrics = role.style();
        metrics.size *= self.type_scale;
        metrics.line_height *= self.type_scale;
        let index = self
            .prepared
            .iter()
            .position(|(s, r, _)| s == text && *r == role)
            .unwrap_or_else(|| {
                // Bound transient status/button strings; the document's prepared
                // text remains reusable across consecutive resize frames.
                if self.prepared.len() >= 8 {
                    self.prepared.remove(1);
                }
                let style = if proposal::text_family(role.id()) == Some("mono") {
                    TextStyle::named("Menlo", metrics.size)
                } else {
                    TextStyle::system(metrics.size)
                }
                .with_weight(metrics.weight);
                self.prepared
                    .push((text.to_owned(), role, self.shaper.prepare(text, &style)));
                self.prepared.len() - 1
            });
        let mut p = self.shaper.shape_prepared(
            &self.prepared[index].2,
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        );
        // Same semantic role metrics as Gallery's native text adapter.
        let mut top = 0.;
        for line in &mut p.lines {
            let height = f64::from(metrics.line_height).max(line.height);
            line.top = top;
            line.height = height;
            line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
            top += height;
        }
        p.height = top.max(f64::from(metrics.line_height));
        p.default_line_height = f64::from(metrics.line_height);
        Rc::new(p)
    }
}
pub struct TextRenderer<'a> {
    pub fonts: &'a mut Fonts,
    pub atlas: &'a mut GlyphAtlas,
    pub scale: f32,
}
impl TextSurfaceHost for TextRenderer<'_> {
    fn shape_surface(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        self.fonts.shape(s, role, width)
    }
}
impl TextSurfaceTextRenderer for TextRenderer<'_> {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        p: &Paragraph,
        origin: Point,
        ink: Rgba,
        clip: Rect,
    ) {
        push_paragraph_visible(
            scene,
            self.atlas,
            &mut self.fonts.raster,
            p,
            origin,
            self.scale,
            ink,
            clip.min_y()..clip.max_y(),
        );
    }
}
impl ButtonTextRenderer for TextRenderer<'_> {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.fonts.shape(s, role, Some(width));
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        let p = self.fonts.shape(s, role, Some(rect.size.width));
        let x = match align {
            ButtonTextAlign::Leading => rect.min_x(),
            ButtonTextAlign::Center => rect.min_x() + (rect.size.width - p.width as f32) * 0.5,
        };
        let origin = Point::new(
            x.round(),
            (rect.min_y() + (rect.size.height - p.height as f32) * 0.5).round(),
        );
        scene.push_clip(rect);
        self.push_visible(scene, &p, origin, ink, rect);
        scene.pop_clip();
    }
}
