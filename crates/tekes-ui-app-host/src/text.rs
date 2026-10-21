use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
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
}
impl Text {
    pub fn new() -> Self {
        let f = CoreTextFonts::new();
        Self {
            atlas: GlyphAtlas::default(),
            shaper: CoreTextShaper::new(f.clone()),
            raster: CoreTextRasterizer::new(f),
            scale: 1.,
        }
    }
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, role: proposal::TextRole, _: f32) -> Size {
        let style = role.style();
        let p = self.shaper.shape(
            text,
            &TextStyle::system(style.size).with_weight(style.weight),
            &ShapeOptions::default(),
        );
        Size::new(p.width as f32, p.height as f32)
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
        let style = role.style();
        let p = self.shaper.shape(
            text,
            &TextStyle::system(style.size).with_weight(style.weight),
            &ShapeOptions::default(),
        );
        let x = if align == ButtonTextAlign::Center {
            r.min_x() + (r.size.width - p.width as f32) * 0.5
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
