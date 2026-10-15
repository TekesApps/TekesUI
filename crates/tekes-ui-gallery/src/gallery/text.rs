//! Gallery 的文字引擎：一份字体登记表、一个排版器、一个光栅化器、一个 `GlyphAtlas`，都在主线程。
//!
//! - 标签（目录行、节标题、列头、窗格标题、色块名）按 `(文字, TextRole, 换行宽度)` 排一次、缓存段落，
//!   以后每帧只做 `push_paragraph`（atlas 命中，不再光栅化）。段落与主题无关，颜色在编图元时才给，
//!   所以切主题、切页都不会重新排版；
//! - 文字原点一律落在整数 pt 上（水平取 `round`，垂直居中后取 `round`）；
//! - 窗口用的 atlas 在这里；离屏对比（静态文字页的差异图）用另一个私有 atlas，避免互相干扰；
//! - scene 里的 `AtlasGlyph` 只在编出它的那一帧有效，而 Gallery 每次重绘都整个重编 scene，
//!   所以 atlas 淘汰（`stats().evictions` 变化）不需要额外处理。

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene};
use tekes_ui_text::coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper};
use tekes_ui_text::model::{Paragraph, ShapeOptions};
use tekes_ui_text::push::push_paragraph;
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::Shaper;
use tekes_ui_tokens::{ColorRole, Rgba, TextRole, Theme, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LabelRole {
    Legacy(TextRole),
    Proposal(proposal::TextRole),
}

/// 标签缓存的键。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LabelKey {
    text: String,
    role: LabelRole,
    /// 换行宽度的 1/64 pt（整数，便于做键）；`None` 不换行。
    width_64: Option<u32>,
}

/// 水平对齐。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
}

/// 标签的样式：字号字重来自 `TextRole`，颜色来自 `ColorRole`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelStyle {
    pub role: TextRole,
    pub color: ColorRole,
    pub align: HAlign,
}

impl LabelStyle {
    pub const fn new(role: TextRole, color: ColorRole) -> LabelStyle {
        LabelStyle {
            role,
            color,
            align: HAlign::Left,
        }
    }

    pub const fn centered(mut self) -> LabelStyle {
        self.align = HAlign::Center;
        self
    }
}

pub struct TextEngine {
    shaper: CoreTextShaper,
    rasterizer: CoreTextRasterizer,
    atlas: GlyphAtlas,
    /// 离屏渲染用的 atlas。
    offscreen_atlas: GlyphAtlas,
    labels: HashMap<LabelKey, Rc<Paragraph>>,
    label_order: VecDeque<LabelKey>,
    /// 到目前为止排过几次版（测试用：重绘不应增加它）。
    shapes: u64,
    scale: f32,
}

impl std::fmt::Debug for TextEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEngine")
            .field("labels", &self.labels.len())
            .field("shapes", &self.shapes)
            .field("scale", &self.scale)
            .finish_non_exhaustive()
    }
}

impl Default for TextEngine {
    fn default() -> Self {
        TextEngine::new()
    }
}

/// `TextRole` 对应的文字样式：系统界面字体，字号与字重取自 token。
pub fn role_style(role: TextRole) -> TextStyle {
    let s = role.style();
    TextStyle::system(s.size).with_weight(s.weight)
}

impl TextEngine {
    pub fn new() -> TextEngine {
        let fonts = CoreTextFonts::new();
        TextEngine {
            shaper: CoreTextShaper::new(fonts.clone()),
            rasterizer: CoreTextRasterizer::new(fonts),
            atlas: GlyphAtlas::default(),
            offscreen_atlas: GlyphAtlas::default(),
            labels: HashMap::new(),
            label_order: VecDeque::new(),
            shapes: 0,
            scale: 2.0,
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// 窗口用的 atlas（渲染器画之前上传它的脏区）。
    pub fn atlas_mut(&mut self) -> &mut GlyphAtlas {
        &mut self.atlas
    }

    /// 每帧开头调用一次。
    pub fn begin_frame(&mut self) {
        self.atlas.begin_frame();
    }

    #[cfg(test)]
    pub fn shape_count(&self) -> u64 {
        self.shapes
    }

    /// 排一段文字（不缓存）。
    pub fn shape(&mut self, text: &str, style: &TextStyle, options: &ShapeOptions) -> Paragraph {
        self.shapes += 1;
        self.shaper.shape(text, style, options)
    }

    fn cache_label(&mut self, key: LabelKey, paragraph: Rc<Paragraph>) {
        // Animated diagnostic values are not a finite fixture set. Bound their
        // cache without clearing/re-shaping the entire visible page at once.
        const LIMIT: usize = 4096;
        if self.labels.len() >= LIMIT
            && let Some(old) = self.label_order.pop_front()
        {
            self.labels.remove(&old);
        }
        self.label_order.push_back(key.clone());
        self.labels.insert(key, paragraph);
    }

    /// 标签段落：按 `(text, role, width)` 缓存。`width` 为换行宽度（pt）。
    pub fn label(&mut self, text: &str, role: TextRole, width: Option<f32>) -> Rc<Paragraph> {
        let key = LabelKey {
            text: text.to_string(),
            role: LabelRole::Legacy(role),
            width_64: width.map(|w| (w * 64.0).round() as u32),
        };
        if let Some(p) = self.labels.get(&key) {
            return p.clone();
        }
        let options = ShapeOptions {
            width: width.map(f64::from),
            line_spacing: 0.0,
        };
        let p = Rc::new(self.shape(text, &role_style(role), &options));
        self.cache_label(key, p.clone());
        p
    }

    /// 把段落编进 scene，原点 `origin`（调用方保证是整数 pt）。
    pub fn push(&mut self, scene: &mut Scene, paragraph: &Paragraph, origin: Point, color: Rgba) {
        push_paragraph(
            scene,
            &mut self.atlas,
            &mut self.rasterizer,
            paragraph,
            origin,
            self.scale,
            color,
        );
    }

    pub fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &Paragraph,
        origin: Point,
        color: Rgba,
        clip: Rect,
    ) {
        tekes_ui_text::push_paragraph_visible(
            scene,
            &mut self.atlas,
            &mut self.rasterizer,
            paragraph,
            origin,
            self.scale,
            color,
            clip.min_y()..clip.max_y(),
        );
    }

    /// 在 `slot` 里画一行标签：水平 `align`，垂直居中，原点取整到整数 pt。
    /// 返回标签的宽度（pt），测试用来核对没有超出槽。
    pub fn label_in(
        &mut self,
        scene: &mut Scene,
        theme: Theme,
        slot: Rect,
        text: &str,
        style: LabelStyle,
    ) -> f32 {
        let p = self.label(text, style.role, None);
        let origin = label_origin(slot, &p, style.align);
        self.push(scene, &p, origin, style.color.resolve(theme));
        p.width as f32
    }

    /// 多行标签（按 `slot` 宽度换行），顶对齐，原点取整。返回行数。
    pub fn wrapped_label_in(
        &mut self,
        scene: &mut Scene,
        theme: Theme,
        slot: Rect,
        text: &str,
        style: LabelStyle,
    ) -> usize {
        let p = self.label(text, style.role, Some(slot.size.width));
        let origin = Point::new(slot.min_x().round(), slot.min_y().round());
        self.push(scene, &p, origin, style.color.resolve(theme));
        p.lines.len()
    }

    /// A JSON-defined profile, cached separately from the legacy calibration roles.
    pub fn proposal_label(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        let key = LabelKey {
            text: text.into(),
            role: LabelRole::Proposal(role),
            width_64: width.map(|w| (w * 64.0).round() as u32),
        };
        if let Some(paragraph) = self.labels.get(&key) {
            return paragraph.clone();
        }
        let paragraph = self.shape_proposal(text, role, width);
        let paragraph = Rc::new(paragraph);
        self.cache_label(key, paragraph.clone());
        paragraph
    }

    /// Editable documents cache their latest layout themselves, so typed strings
    /// never accumulate in the Gallery's permanent fixture-label cache.
    pub fn shape_proposal(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Paragraph {
        let metrics = role.style();
        // Font choice is a platform strategy; sizes, weights and family roles come from JSON.
        let style = if proposal::text_family(role.id()) == Some("mono") {
            TextStyle::named("Menlo", metrics.size)
        } else {
            TextStyle::system(metrics.size)
        }
        .with_weight(metrics.weight);
        let mut paragraph = self.shape(
            text,
            &style,
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.0,
            },
        );
        let mut top = 0.0;
        for line in &mut paragraph.lines {
            let height = f64::from(metrics.line_height).max(line.height);
            line.top = top;
            line.height = height;
            line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
            top += height;
        }
        paragraph.height = top.max(f64::from(metrics.line_height));
        paragraph.default_line_height = f64::from(metrics.line_height);
        paragraph
    }

    /// Draw a one-line fixture label, cropped to its actual device-pixel slot.
    pub fn proposal_label_in(
        &mut self,
        scene: &mut Scene,
        _theme: Theme,
        slot: Rect,
        text: &str,
        role: proposal::TextRole,
        color: Rgba,
    ) -> f32 {
        let paragraph = self.proposal_label(text, role, None);
        let origin = label_origin(slot, &paragraph, HAlign::Left);
        self.push_bounded(scene, &paragraph, origin, color, slot);
        paragraph.width as f32
    }

    /// Selection backgrounds and text use the same shaped line geometry.
    pub fn proposal_selected_in(
        &mut self,
        scene: &mut Scene,
        theme: Theme,
        slot: Rect,
        text: &str,
        role: proposal::TextRole,
        wrapped: bool,
    ) {
        let paragraph = self.proposal_label(text, role, wrapped.then_some(slot.size.width));
        let origin = if wrapped {
            slot.origin
        } else {
            label_origin(slot, &paragraph, HAlign::Left)
        };
        scene.push_clip(slot);
        for rect in
            paragraph.selection_rects(0..paragraph.utf16_len, slot.offset(-origin.x, -origin.y))
        {
            scene.push_quad(tekes_ui_scene::Quad::new(
                rect.offset(origin.x, origin.y),
                proposal::render_color("text-selection", theme).unwrap(),
            ));
        }
        self.push(
            scene,
            &paragraph,
            origin,
            proposal::render_color("selected-text", theme).unwrap(),
        );
        scene.pop_clip();
    }

    pub fn proposal_wrapped_in(
        &mut self,
        scene: &mut Scene,
        _theme: Theme,
        slot: Rect,
        text: &str,
        role: proposal::TextRole,
        color: Rgba,
    ) -> usize {
        let paragraph = self.proposal_label(text, role, Some(slot.size.width));
        self.push_bounded(
            scene,
            &paragraph,
            Point::new(slot.min_x().round(), slot.min_y().round()),
            color,
            slot,
        );
        paragraph
            .lines
            .iter()
            .filter(|line| line.top < f64::from(slot.size.height))
            .count()
    }

    fn push_bounded(
        &mut self,
        scene: &mut Scene,
        paragraph: &Paragraph,
        origin: Point,
        color: Rgba,
        slot: Rect,
    ) {
        let mut labels = Scene::new();
        self.push(&mut labels, paragraph, origin, color);
        for primitive in labels.primitives() {
            if let tekes_ui_scene::Primitive::Glyphs(run) = primitive {
                let run = clip_glyphs(run, slot, self.scale);
                if !run.glyphs.is_empty() {
                    scene.push_glyphs(run);
                }
            }
        }
    }

    /// 离屏渲染一个段落：画布 `size_px`，原点 (0,0)，前景 `fg`、背景 `bg`。
    /// 用私有 atlas，每次调用先 `begin_frame`。
    pub fn render_paragraph_offscreen(
        &mut self,
        renderer: &mut Renderer,
        paragraph: &Paragraph,
        size_px: (u32, u32),
        scale: f32,
        fg: Rgba,
        bg: Rgba,
    ) -> Result<Vec<u8>, RenderError> {
        self.offscreen_atlas.begin_frame();
        let mut scene = Scene::new();
        push_paragraph(
            &mut scene,
            &mut self.offscreen_atlas,
            &mut self.rasterizer,
            paragraph,
            Point::new(0.0, 0.0),
            scale,
            fg,
        );
        renderer.render_to_rgba8(&scene, &mut self.offscreen_atlas, size_px, scale, bg)
    }
}

/// Crop atlas subrects without changing cached atlas entries or global rendering.
pub(super) fn clip_glyphs(
    run: &tekes_ui_scene::GlyphRun,
    slot: Rect,
    scale: f32,
) -> tekes_ui_scene::GlyphRun {
    let mut clipped = run.clone();
    clipped.glyphs.retain_mut(|positioned| {
        let glyph = &mut positioned.glyph;
        let left = (positioned.origin.x * scale).round() + glyph.left as f32;
        let top = (positioned.origin.y * scale).round() + glyph.top as f32;
        let x0 = left.max((slot.min_x() * scale).round());
        let y0 = top.max((slot.min_y() * scale).round());
        let x1 = (left + glyph.width as f32).min((slot.max_x() * scale).round());
        let y1 = (top + glyph.height as f32).min((slot.max_y() * scale).round());
        if x1 <= x0 || y1 <= y0 {
            return false;
        }
        let dx = (x0 - left) as u32;
        let dy = (y0 - top) as u32;
        glyph.x += dx;
        glyph.y += dy;
        glyph.left += dx as i32;
        glyph.top += dy as i32;
        glyph.width = (x1 - x0) as u32;
        glyph.height = (y1 - y0) as u32;
        true
    });
    clipped
}

/// 标签在 `slot` 里的原点：水平靠左取 `slot.min_x`，居中取槽中线减半宽；垂直居中；都取整。
pub fn label_origin(slot: Rect, p: &Paragraph, align: HAlign) -> Point {
    let x = match align {
        HAlign::Left => slot.min_x(),
        HAlign::Center => slot.min_x() + (slot.size.width - p.width as f32) * 0.5,
    };
    let y = slot.min_y() + (slot.size.height - p.height as f32) * 0.5;
    Point::new(x.round(), y.round())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_shaped_once_and_cached() {
        let mut e = TextEngine::new();
        let a = e.label("Primitives", TextRole::Body, None);
        let n = e.shape_count();
        let b = e.label("Primitives", TextRole::Body, None);
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(e.shape_count(), n);
        // 不同角色、不同宽度是不同的键。
        e.label("Primitives", TextRole::RowMeta, None);
        e.label("Primitives", TextRole::Body, Some(40.0));
        assert_eq!(e.shape_count(), n + 2);
    }

    #[test]
    fn label_origin_is_integer_and_centered() {
        let mut e = TextEngine::new();
        let p = e.label("Token Swatches", TextRole::Body, None);
        let slot = Rect::new(20.0, 31.0, 180.0, 16.0);
        for align in [HAlign::Left, HAlign::Center] {
            let o = label_origin(slot, &p, align);
            assert_eq!(o.x, o.x.round());
            assert_eq!(o.y, o.y.round());
            // 垂直居中的偏差不超过半个点（取整）。
            let centered = slot.min_y() + (slot.size.height - p.height as f32) * 0.5;
            assert!((o.y - centered).abs() <= 0.5);
        }
        assert_eq!(label_origin(slot, &p, HAlign::Left).x, 20.0);
    }

    #[test]
    fn proposal_profiles_cache_and_keep_family_and_line_height() {
        let mut engine = TextEngine::new();
        let a = engine.proposal_label("let value = 42;", proposal::TextRole::Code, Some(160.0));
        let n = engine.shape_count();
        let b = engine.proposal_label("let value = 42;", proposal::TextRole::Code, Some(160.0));
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(engine.shape_count(), n);
        assert_eq!(proposal::TextRole::Code.id(), "code");
        assert!(a.fonts[0].post_script_name.contains("Menlo"));
        assert!(a.height >= f64::from(proposal::TextRole::Code.style().line_height));
    }

    #[test]
    fn glyph_crop_adjusts_bitmap_region_without_mutating_the_cached_glyph() {
        use tekes_ui_scene::{AtlasGlyph, GlyphRun, PositionedGlyph, RasterFormat};
        let run = GlyphRun {
            color: Rgba::gray(0.0),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(10.0, 10.0),
                glyph: AtlasGlyph {
                    page: 0,
                    x: 10,
                    y: 20,
                    width: 20,
                    height: 20,
                    left: -2,
                    top: -4,
                    format: RasterFormat::Mask,
                },
            }],
        };
        let cut = clip_glyphs(&run, Rect::new(10.0, 10.0, 5.0, 4.0), 2.0);
        let glyph = cut.glyphs[0].glyph;
        assert_eq!(
            (glyph.x, glyph.y, glyph.width, glyph.height),
            (12, 24, 10, 8)
        );
        assert_eq!((glyph.left, glyph.top), (0, 0));
        assert_eq!(run.glyphs[0].glyph.width, 20);
        assert!(
            clip_glyphs(&run, Rect::new(100.0, 100.0, 5.0, 4.0), 2.0)
                .glyphs
                .is_empty()
        );
    }

    #[test]
    fn actual_showcase_labels_stay_inside_their_slots_at_both_scales() {
        let slot = Rect::new(10.0, 10.0, 38.0, 18.0);
        let mut text = TextEngine::new();
        for scale in [1.0, 2.0] {
            text.set_scale(scale);
            text.begin_frame();
            let mut scene = Scene::new();
            text.proposal_label_in(
                &mut scene,
                Theme::LIGHT,
                slot,
                "Long text 你好",
                proposal::TextRole::UiBody,
                Rgba::gray(0.0),
            );
            assert!(!scene.is_empty());
            for primitive in scene.primitives() {
                let tekes_ui_scene::Primitive::Glyphs(run) = primitive else {
                    panic!("expected glyphs")
                };
                for positioned in &run.glyphs {
                    let g = positioned.glyph;
                    let x = (positioned.origin.x * scale).round() + g.left as f32;
                    let y = (positioned.origin.y * scale).round() + g.top as f32;
                    assert!(x >= slot.min_x() * scale && y >= slot.min_y() * scale);
                    assert!(x + g.width as f32 <= slot.max_x() * scale);
                    assert!(y + g.height as f32 <= slot.max_y() * scale);
                }
            }
        }
    }

    #[test]
    fn role_style_follows_tokens() {
        assert_eq!(role_style(TextRole::Body).size, 13.0);
        assert_eq!(role_style(TextRole::RowMeta).size, 11.0);
        assert_eq!(role_style(TextRole::SectionHeader).size, 11.0);
        assert_eq!(role_style(TextRole::SectionHeader).weight, 700);
    }
}
