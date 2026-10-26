//! Gallery 的文字引擎：一份字体登记表、一个排版器、一个光栅化器、一个 `GlyphAtlas`，都在主线程。
//!
//! - 标签（目录行、节标题、列头、窗格标题、色块名）按 `(文字, TextRole, 换行宽度)` 排一次、缓存段落，
//!   以后每帧只做 `push_paragraph`（atlas 命中，不再光栅化）。段落与主题无关，颜色在编图元时才给，
//!   所以切主题、切页都不会重新排版；
//! - 文字原点一律落在整数 pt 上（水平取 `round`，垂直居中后取 `round`）；
//! - 窗口用的 atlas 在这里；离屏对比（静态文字页的差异图）用另一个私有 atlas，避免互相干扰；
//! - scene 里的 `AtlasGlyph` 只在编出它的那一帧有效，而 Gallery 每次重绘都整个重编 scene，
//!   所以 atlas 淘汰（`stats().evictions` 变化）不需要额外处理。

use std::rc::Rc;
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use tekes_ui_controls::{
    Button, ButtonLayout, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlMotionContext,
    TextSurfaceTextRenderer,
};
use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
use tekes_ui_text::coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper, PreparedText};
use tekes_ui_text::model::{Paragraph, ShapeOptions};
use tekes_ui_text::push::push_paragraph;
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::Shaper;
use tekes_ui_tokens::{ColorRole, Rgba, TextRole, Theme, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LabelRole {
    Legacy(TextRole),
    TickingPreview(u32),
    Proposal(proposal::TextRole),
}

/// 标签缓存的键。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LabelKey {
    text: String,
    role: LabelRole,
    /// 换行宽度的 1/64 pt（整数，便于做键）；`None` 不换行。
    width_64: Option<u32>,
    locale: super::locale::Locale,
    type_scale_bits: u32,
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
    type_scale: f32,
    locale: super::locale::Locale,
    motion_enabled: bool,
    motion_context: Option<ControlMotionContext>,
    motion_frame_requested: bool,
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
            type_scale: 1.0,
            locale: super::locale::Locale::default(),
            motion_enabled: false,
            motion_context: None,
            motion_frame_requested: false,
        }
    }

    pub fn type_scale(&self) -> f32 {
        self.type_scale
    }
    pub fn set_type_scale(&mut self, scale: f32) {
        if scale.is_finite() {
            self.type_scale = scale.clamp(1.0, 1.5);
        }
    }
    pub fn locale(&self) -> super::locale::Locale {
        self.locale
    }
    pub fn set_locale(&mut self, locale: super::locale::Locale) {
        self.locale = locale;
    }

    #[cfg(test)]
    pub fn untranslated_labels(&self) -> Vec<String> {
        self.labels
            .keys()
            .filter(|key| key.locale == super::locale::Locale::En)
            .filter_map(|key| {
                let translated = key.locale.text(&key.text);
                super::locale::has_cjk(&translated).then(|| translated.into_owned())
            })
            .collect()
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
        self.motion_frame_requested = false;
    }

    /// Enable component motion for the live Gallery. Standalone fixtures keep
    /// the default static behavior unless a host explicitly opts in.
    pub fn set_motion_enabled(&mut self, enabled: bool) {
        self.motion_enabled = enabled;
        if enabled && self.motion_context.is_none() {
            self.motion_context = Some(ControlMotionContext {
                now: Duration::ZERO,
                reduced_motion: false,
            });
        } else if !enabled {
            self.motion_context = None;
            self.motion_frame_requested = false;
        }
    }

    pub fn set_motion_context(&mut self, now: Duration, reduced_motion: bool) {
        self.motion_context = self.motion_enabled.then_some(ControlMotionContext {
            now,
            reduced_motion,
        });
    }

    /// Static comparison fixtures borrow the host clock and snap their local
    /// states. The caller restores this exact optional context afterwards.
    pub(super) fn replace_fixture_motion_context(
        &mut self,
        context: Option<ControlMotionContext>,
    ) -> Option<ControlMotionContext> {
        std::mem::replace(&mut self.motion_context, context)
    }

    pub fn motion_frame_requested(&self) -> bool {
        self.motion_frame_requested
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
            locale: self.locale,
            type_scale_bits: self.type_scale.to_bits(),
        };
        if let Some(p) = self.labels.get(&key) {
            return p.clone();
        }
        let options = ShapeOptions {
            width: width.map(f64::from),
            line_spacing: 0.0,
        };
        let localized = self.locale.text(text);
        let mut style = role_style(role);
        style.size *= self.type_scale;
        let p = Rc::new(self.shape(&localized, &style, &options));
        self.cache_label(key, p.clone());
        p
    }

    /// Large counter glyphs are shaped at their final size and cached per digit.
    fn ticking_preview_digit(&mut self, digit: &str, size: f32) -> Rc<Paragraph> {
        let key = LabelKey {
            text: digit.to_owned(),
            role: LabelRole::TickingPreview(size.to_bits()),
            width_64: None,
            locale: self.locale,
            type_scale_bits: self.type_scale.to_bits(),
        };
        if let Some(paragraph) = self.labels.get(&key) {
            return paragraph.clone();
        }
        let style = TextStyle::system(size);
        let paragraph = Rc::new(self.shape(digit, &style, &ShapeOptions::default()));
        self.cache_label(key, paragraph.clone());
        paragraph
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
        self.proposal_label_with_locale(text, role, width, self.locale)
    }

    /// Measure a Button using the same cached paragraph backend used by rendering.
    /// The returned size includes the component's token-defined insets and minimum height.
    pub fn measure_button(
        &mut self,
        label: &str,
        max_width: f32,
        size: ButtonSize,
        role: proposal::TextRole,
    ) -> Size {
        Button::measure(self, label, max_width, size, role)
    }

    /// Render the shared production Button through Gallery's CoreText adapter.
    #[allow(clippy::too_many_arguments)]
    pub fn render_button(
        &mut self,
        scene: &mut Scene,
        button: &mut Button,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<tekes_ui_tokens::Palette>,
        variant: tekes_ui_controls::paint::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
    ) -> ButtonLayout {
        button.render(
            scene, self, label, frame, clip, palette, variant, size, role, align,
        )
    }

    /// Cached document geometry. The original UTF-16 text must stay intact for
    /// selection and reveal ranges, regardless of the interface language.
    pub fn proposal_document(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        self.proposal_label_with_locale(text, role, width, super::locale::Locale::Reference)
    }

    fn proposal_label_with_locale(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
        locale: super::locale::Locale,
    ) -> Rc<Paragraph> {
        let key = LabelKey {
            text: text.into(),
            role: LabelRole::Proposal(role),
            width_64: width.map(|w| (w * 64.0).round() as u32),
            locale,
            type_scale_bits: self.type_scale.to_bits(),
        };
        if let Some(paragraph) = self.labels.get(&key) {
            return paragraph.clone();
        }
        let localized = locale.text(text);
        let paragraph = self.shape_proposal(&localized, role, width);
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
        let prepared = self.prepare_proposal(text, role);
        self.reflow_proposal(&prepared, role, width)
    }

    pub fn prepare_proposal(&self, text: &str, role: proposal::TextRole) -> PreparedText {
        let metrics = role.style();
        let style = if proposal::text_family(role.id()) == Some("mono") {
            TextStyle::named("Menlo", metrics.size * self.type_scale)
        } else {
            TextStyle::system(metrics.size * self.type_scale)
        }
        .with_weight(metrics.weight);
        self.shaper.prepare(text, &style)
    }

    pub fn reflow_proposal(
        &mut self,
        prepared: &PreparedText,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Paragraph {
        let metrics = role.style();
        self.shapes += 1;
        let mut paragraph = self.shaper.shape_prepared(
            prepared,
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.0,
            },
        );
        let mut top = 0.0;
        for line in &mut paragraph.lines {
            let height = f64::from(metrics.line_height * self.type_scale).max(line.height);
            line.top = top;
            line.height = height;
            line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
            top += height;
        }
        paragraph.height = top.max(f64::from(metrics.line_height * self.type_scale));
        paragraph.default_line_height = f64::from(metrics.line_height * self.type_scale);
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

impl tekes_ui_controls::TextSurfaceHost for TextEngine {
    fn shape_label(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<Rc<tekes_ui_text::LabelLayout>> {
        let metrics = role.style();
        let style = if proposal::text_family(role.id()) == Some("mono") {
            TextStyle::named("Menlo", metrics.size * self.type_scale)
        } else {
            TextStyle::system(metrics.size * self.type_scale)
        }
        .with_weight(metrics.weight);
        Some(Rc::new(
            self.shaper
                .shape_label(
                    text,
                    &style,
                    &ShapeOptions::with_width(f64::from(width)),
                    options,
                )
                .with_min_line_height(f64::from(metrics.line_height * self.type_scale)),
        ))
    }

    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        Rc::new(self.shape_proposal(text, role, width))
    }
    fn type_scale(&self) -> f32 {
        self.type_scale
    }
}
/// Gallery-only large typography adapter; fixed slots remain owned by TickingNumber.
pub(super) struct TickingPreviewText<'a>(pub &'a mut TextEngine, pub f32);
impl ButtonTextRenderer for TickingPreviewText<'_> {
    fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
        let paragraph = self.0.ticking_preview_digit(text, self.1);
        Size::new(paragraph.width as f32, paragraph.height as f32)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        _: proposal::TextRole,
        rect: Rect,
        _: ButtonTextAlign,
        ink: Rgba,
    ) {
        let paragraph = self.0.ticking_preview_digit(text, self.1);
        let origin = label_origin(rect, &paragraph, HAlign::Center);
        self.0.push(scene, &paragraph, origin, ink);
    }
}

impl ButtonTextRenderer for TextEngine {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }

    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.motion_context
    }

    fn request_animation_frame(&mut self) {
        if self.motion_enabled {
            self.motion_frame_requested = true;
        }
    }

    fn backing_scale(&self) -> f32 {
        self.scale
    }

    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        let paragraph = self.button_paragraph(text, role, max_width);
        Size::new(paragraph.width as f32, paragraph.height as f32)
    }

    fn first_line_height(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> f32 {
        let paragraph = self.button_paragraph(text, role, max_width);
        paragraph
            .lines
            .first()
            .map_or(paragraph.default_line_height as f32, |line| {
                line.height as f32
            })
    }

    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        let paragraph = self.button_paragraph(text, role, rect.size.width);
        let align = match align {
            ButtonTextAlign::Leading => HAlign::Left,
            ButtonTextAlign::Center => HAlign::Center,
        };
        let origin = label_origin(rect, &paragraph, align);
        // The production Button already clips its complete contents to the visible outer frame.
        // Clipping again to the logical wrap slot truncates negative glyph bearings and marks
        // that extend into the token-defined horizontal inset, even when the button has room.
        self.push(scene, &paragraph, origin, ink);
    }
}

impl TextSurfaceTextRenderer for TextEngine {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &Paragraph,
        origin: Point,
        color: Rgba,
        clip: Rect,
    ) {
        TextEngine::push_visible(self, scene, paragraph, origin, color, clip);
    }
}

impl TextEngine {
    /// Keep single-line labels on their unwrapped cache entry while the Button's frame animates.
    /// Only labels that exceed the available slot acquire a width-dependent wrapped paragraph.
    fn button_paragraph(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        available_width: f32,
    ) -> Rc<Paragraph> {
        let natural = self.proposal_label(text, role, None);
        let available_width = available_width.max(0.0);
        // Widths are stored at 1/64 pt precision. Match that quantization at the fit boundary so
        // a fractional frame does not spuriously reflow due to a sub-quantum rounding difference.
        if natural.width as f32 <= available_width + (1.0 / 64.0) {
            natural
        } else {
            self.proposal_label(text, role, Some(available_width))
        }
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
    fn motion_context_is_opt_in_and_frame_requests_reset_at_frame_start() {
        let mut engine = TextEngine::new();
        assert_eq!(
            engine.motion_context(),
            None,
            "standalone fixtures remain static by default"
        );
        engine.request_animation_frame();
        assert!(!engine.motion_frame_requested());

        engine.set_motion_enabled(true);
        engine.set_motion_context(Duration::from_millis(42), true);
        assert_eq!(
            engine.motion_context(),
            Some(ControlMotionContext {
                now: Duration::from_millis(42),
                reduced_motion: true,
            })
        );
        engine.request_animation_frame();
        assert!(engine.motion_frame_requested());
        engine.begin_frame();
        assert!(!engine.motion_frame_requested());
        assert_eq!(
            engine.motion_context().unwrap().now,
            Duration::from_millis(42)
        );
    }

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
    fn localized_motion_controls_fit_the_existing_compact_buttons() {
        let mut text = TextEngine::new();
        for locale in super::super::locale::Locale::ALL {
            text.set_locale(locale);
            for copy in [
                "目标 A / 反向",
                "目标 B / 展开",
                "演示中途改目标",
                "暂停",
                "重播",
                "继续当前",
            ] {
                let paragraph = text.proposal_label(copy, proposal::TextRole::UiBody, None);
                assert!(
                    paragraph.width
                        <= if copy == "演示中途改目标" {
                            132.0
                        } else {
                            76.0
                        },
                    "{locale:?}: {copy} is {}pt",
                    paragraph.width
                );
            }
            let source = "Selected 中英文 👩‍💻 e\u{301}";
            let paragraph = text.proposal_document(source, proposal::TextRole::UiBody, Some(300.0));
            assert_eq!(paragraph.utf16_len, source.encode_utf16().count());
        }
    }

    #[test]
    fn shared_button_measure_and_render_keep_wrapped_labels_in_the_same_geometry() {
        use tekes_ui_controls::{Button, ButtonSize, ButtonTextAlign, paint};
        use tekes_ui_scene::Primitive;

        let labels = [
            "A long English label that wraps across several lines",
            "长文本按钮标签需要自然换行并保持完整可见",
            "Review 👩‍💻 changes e\u{301} before applying",
        ];
        let mut text = TextEngine::new();
        for type_scale in [1.0, 1.25, 1.5] {
            text.set_type_scale(type_scale);
            for label in labels {
                for max_width in [71.3, 157.7, 240.3] {
                    let measured = text.measure_button(
                        label,
                        max_width,
                        ButtonSize::Md,
                        proposal::TextRole::UiBody,
                    );
                    let mut button = Button::default();
                    let mut scene = Scene::new();
                    let layout = text.render_button(
                        &mut scene,
                        &mut button,
                        label,
                        Rect::new(13.0, 17.0, measured.width, measured.height),
                        None,
                        tekes_ui_tokens::Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
                        paint::ButtonVariant::Quiet,
                        ButtonSize::Md,
                        proposal::TextRole::UiBody,
                        ButtonTextAlign::Center,
                    );

                    assert!(
                        (layout.frame.size.height - measured.height).abs() <= 0.02,
                        "measurement/render height drift for {label:?} at {max_width}pt, scale {type_scale}: {} vs {}",
                        layout.frame.size.height,
                        measured.height
                    );
                    assert!(layout.frame.max_y() >= layout.content.max_y());
                    assert!(layout.content.min_x() >= layout.frame.min_x());
                    assert!(layout.content.max_x() <= layout.frame.max_x());
                    assert!(
                        scene
                            .primitives()
                            .iter()
                            .any(|p| matches!(p, Primitive::Glyphs(_)))
                    );
                    // Compare the component's clipped output to the exact same shaped paragraph
                    // pushed without clipping, catching real glyph bitmap truncation.
                    let paragraph = text.proposal_label(
                        label,
                        proposal::TextRole::UiBody,
                        Some(layout.content.size.width),
                    );
                    let origin = label_origin(layout.content, &paragraph, HAlign::Center);
                    let mut uncropped = Scene::new();
                    text.push(&mut uncropped, &paragraph, origin, Rgba::gray(0.0));
                    let glyph_records = |scene: &Scene| {
                        scene
                            .primitives()
                            .iter()
                            .flat_map(|primitive| match primitive {
                                Primitive::Glyphs(run) => run.glyphs.iter().collect::<Vec<_>>(),
                                _ => Vec::new(),
                            })
                            .map(|glyph| {
                                (
                                    glyph.origin.x.to_bits(),
                                    glyph.origin.y.to_bits(),
                                    glyph.glyph.page,
                                    glyph.glyph.x,
                                    glyph.glyph.y,
                                    glyph.glyph.width,
                                    glyph.glyph.height,
                                    glyph.glyph.left,
                                    glyph.glyph.top,
                                    format!("{:?}", glyph.glyph.format),
                                )
                            })
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        glyph_records(&scene),
                        glyph_records(&uncropped),
                        "Button clipped glyphs for {label:?} at {max_width}pt, scale {type_scale}"
                    );
                    for primitive in scene.primitives() {
                        let Primitive::Glyphs(run) = primitive else {
                            continue;
                        };
                        for glyph in &run.glyphs {
                            let scale = text.scale();
                            let x = (glyph.origin.x * scale).round() + glyph.glyph.left as f32;
                            let y = (glyph.origin.y * scale).round() + glyph.glyph.top as f32;
                            assert!(x >= layout.frame.min_x() * scale - 1.0);
                            assert!(y >= layout.frame.min_y() * scale - 1.0);
                            assert!(
                                x + glyph.glyph.width as f32 <= layout.frame.max_x() * scale + 1.0
                            );
                            assert!(
                                y + glyph.glyph.height as f32 <= layout.frame.max_y() * scale + 1.0
                            );
                        }
                    }

                    let shaped = text.shape_count();
                    let mut repeated = Scene::new();
                    text.render_button(
                        &mut repeated,
                        &mut button,
                        label,
                        layout.frame,
                        None,
                        tekes_ui_tokens::Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
                        paint::ButtonVariant::Quiet,
                        ButtonSize::Md,
                        proposal::TextRole::UiBody,
                        ButtonTextAlign::Center,
                    );
                    assert_eq!(
                        text.shape_count(),
                        shaped,
                        "button text should hit the shape cache"
                    );
                    assert_eq!(
                        button.layout().unwrap().label_size.height,
                        layout.label_size.height
                    );
                }
            }
        }
    }

    #[test]
    fn shared_button_single_line_cache_survives_animated_frame_widths() {
        use tekes_ui_controls::{Button, ButtonSize, ButtonTextAlign, paint};

        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let label = "Smooth motion preview";
        let mut button = Button::default();
        let palette = tekes_ui_tokens::Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut scene = Scene::new();
        let mut first_shapes = None;
        for frame in 0..120 {
            // Simulate the Motion story changing the Button frame every display tick while its
            // one-line label remains comfortably inside the content slot.
            let width = 320.0 + (frame as f32 * 0.15).sin() * 24.0;
            text.render_button(
                &mut scene,
                &mut button,
                label,
                Rect::new(0.0, 0.0, width, 48.0),
                None,
                palette,
                paint::ButtonVariant::Quiet,
                ButtonSize::Md,
                proposal::TextRole::UiBody,
                ButtonTextAlign::Center,
            );
            let shapes = text.shape_count();
            if let Some(first) = first_shapes {
                assert_eq!(shapes, first, "fitting label reshaped at frame {frame}");
            } else {
                first_shapes = Some(shapes);
            }
            scene.clear();
        }
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

#[cfg(test)]
mod gold7_focus_raster {
    use super::*;
    use tekes_ui_controls::{Button, ButtonSize, ButtonTextAlign, paint::ButtonVariant};
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};

    #[test]
    fn actual_button_focus_ring_keeps_the_gap_error_border_and_label_pixels() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("SKIP: Gold7 focus raster needs a GPU adapter");
            return;
        };
        for theme in Theme::ALL {
            for scale in [1u32, 2] {
                let mut text = TextEngine::new();
                text.set_scale(scale as f32);
                text.set_type_scale(1.5);
                let frame = Rect::new(20., 20., 140., 40.);
                let palette = tekes_ui_tokens::Palette::from(theme);
                let background = palette.color("surface-1").unwrap();
                let render = |focused: bool, text: &mut TextEngine, renderer: &mut Renderer| {
                    text.begin_frame();
                    let mut scene = Scene::new();
                    let mut button = Button::default();
                    button.state.error = true;
                    button.state.focused = focused;
                    button.state.focus_visible = focused;
                    let actual = text.render_button(
                        &mut scene,
                        &mut button,
                        "Clear filter",
                        frame,
                        Some(Rect::new(0., 0., 180., 80.)),
                        palette,
                        ButtonVariant::Quiet,
                        ButtonSize::Body,
                        proposal::TextRole::UiBody,
                        ButtonTextAlign::default(),
                    );
                    let pixels = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            (180 * scale, 80 * scale),
                            scale as f32,
                            background,
                        )
                        .unwrap();
                    (pixels, actual.frame)
                };
                let (normal, actual) = render(false, &mut text, &mut renderer);
                let (focused, focused_frame) = render(true, &mut text, &mut renderer);
                assert_eq!(actual, focused_frame, "focus must not change layout");
                let pixel = |image: &[u8], x: u32, y: u32| {
                    let index = ((y * scale * 180 * scale + x * scale) * 4) as usize;
                    image[index..index + 4].to_vec()
                };
                assert_ne!(
                    pixel(&normal, 90, 17),
                    pixel(&focused, 90, 17),
                    "ring is visible"
                );
                assert_eq!(
                    pixel(&normal, 90, 19),
                    pixel(&focused, 90, 19),
                    "2pt gap stays clear"
                );
                // The outer rounded ring may enter the bounding-box corners
                // outside the Button's actual rounded shape. Compare only the
                // base shape and its one-pixel antialias fringe, not that cutout.
                let radius =
                    proposal::metric("radius", "c-button-md-radius").unwrap() * text.type_scale();
                for y in
                    (actual.min_y() * scale as f32) as u32..(actual.max_y() * scale as f32) as u32
                {
                    for x in (actual.min_x() * scale as f32) as u32
                        ..(actual.max_x() * scale as f32) as u32
                    {
                        let px = (x as f32 + 0.5) / scale as f32;
                        let py = (y as f32 + 0.5) / scale as f32;
                        let qx =
                            (px - actual.center().x).abs() - (actual.size.width * 0.5 - radius);
                        let qy =
                            (py - actual.center().y).abs() - (actual.size.height * 0.5 - radius);
                        let distance = qx.max(0.).hypot(qy.max(0.)) + qx.max(qy).min(0.) - radius;
                        if distance <= 0.5 / scale as f32 {
                            let start = ((y * 180 * scale + x) * 4) as usize;
                            assert_eq!(
                                &normal[start..start + 4],
                                &focused[start..start + 4],
                                "focus changed Button ink at {x},{y}: {theme:?} {scale}x"
                            );
                        }
                    }
                }
                if let Some(output) = std::env::var_os("TEKESUI_GOLD7_FOCUS_OUT") {
                    let dir = std::path::PathBuf::from(output);
                    std::fs::create_dir_all(&dir).unwrap();
                    let path = dir.join(format!("focus-{theme:?}-{scale}x.png"));
                    let mut encoder = png::Encoder::new(
                        std::fs::File::create(path).unwrap(),
                        180 * scale,
                        80 * scale,
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&focused)
                        .unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod label_content_tests {
    use super::*;
    use tekes_ui_controls::composition::*;
    use tekes_ui_text::{LabelOptions, LabelOverflow};

    fn node(kind: usize, original: bool, state: tekes_ui_controls::ControlState) -> Content {
        let source = if original { "A\nB" } else { "A…" };
        let label = |id| {
            let node = Content::new(ComponentId(id), TextContent::new(source));
            if original {
                node.with_label_layout(LabelOptions::single_line(LabelOverflow::TailEllipsis))
                    .unwrap()
            } else {
                node
            }
        };
        let mut node = match kind {
            0 => label(1),
            1 => {
                let node = Content::new(ComponentId(1), ButtonContent::new(source));
                if original {
                    node.with_label_layout(LabelOptions::single_line(LabelOverflow::TailEllipsis))
                        .unwrap()
                } else {
                    node
                }
            }
            _ => Content::new(
                ComponentId(1),
                CompoundButton::new(source, label(2)).unwrap(),
            ),
        };
        if let Some(button) = node.component_mut::<ButtonContent>() {
            button.button.state = state;
        }
        if let Some(button) = node.component_mut::<CompoundButton>() {
            button.action.button.state = state;
        }
        node
    }

    #[test]
    fn production_gallery_label_slots_match_explicit_native_reference_pixels() {
        let mut renderer = Renderer::new_headless().unwrap();
        let mut cases = 0;
        use tekes_ui_controls::ControlState;
        let states = [
            ControlState::default(),
            ControlState {
                hovered: true,
                ..Default::default()
            },
            ControlState {
                pressed: true,
                focused: true,
                ..Default::default()
            },
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
            ControlState {
                disabled: true,
                ..Default::default()
            },
            ControlState {
                loading: true,
                ..Default::default()
            },
        ];
        for theme in Theme::ALL {
            for scale in [1u32, 2] {
                for type_scale in [1., 1.5, 2.] {
                    for kind in 0..3 {
                        for (state_index, state) in states.into_iter().enumerate() {
                            let mut text = TextEngine::new();
                            text.set_scale(scale as f32);
                            text.set_type_scale(type_scale);
                            let palette = tekes_ui_tokens::Palette::from(theme);
                            let mut render = |original: bool| {
                                text.begin_frame();
                                let mut host =
                                    ContentHost::new(node(kind, original, state)).unwrap();
                                let mut scene = Scene::new();
                                let frame = Rect::new(20., 20., 200., 60.);
                                host.render(
                                    &mut scene,
                                    &mut text,
                                    frame,
                                    Rect::new(0., 0., 240., 100.),
                                    palette,
                                )
                                .unwrap();
                                assert_eq!(
                                    host.semantics()[0].semantics.name,
                                    if original { "A\nB" } else { "A…" }
                                );
                                renderer
                                    .render_to_rgba8(
                                        &scene,
                                        text.atlas_mut(),
                                        (240 * scale, 100 * scale),
                                        scale as f32,
                                        palette.color("surface-1").unwrap(),
                                    )
                                    .unwrap()
                            };
                            let actual = render(true);
                            let reference = render(false);
                            assert!(
                                actual == reference,
                                "pixels differ: kind={kind},theme={theme:?},scale={scale},type={type_scale}; changed channels={}",
                                actual
                                    .iter()
                                    .zip(&reference)
                                    .filter(|(a, b)| a != b)
                                    .count()
                            );
                            if let Some(dir) = std::env::var_os("TEKESUI_LABEL_CONTENT_GPU_OUT") {
                                let dir = std::path::PathBuf::from(dir);
                                std::fs::create_dir_all(&dir).unwrap();
                                if scale == 2 && type_scale == 1.5 {
                                    let name = format!("kind-{kind}-state-{state_index}-{theme:?}")
                                        .replace([' ', ':', '{', '}', ','], "_");
                                    let mut encoder = png::Encoder::new(
                                        std::fs::File::create(dir.join(format!("{name}.png")))
                                            .unwrap(),
                                        240 * scale,
                                        100 * scale,
                                    );
                                    encoder.set_color(png::ColorType::Rgba);
                                    encoder.set_depth(png::BitDepth::Eight);
                                    encoder
                                        .write_header()
                                        .unwrap()
                                        .write_image_data(&actual)
                                        .unwrap();
                                }
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 432);
        if let Some(dir) = std::env::var_os("TEKESUI_LABEL_CONTENT_GPU_OUT") {
            std::fs::write(std::path::PathBuf::from(dir).join("result.json"),"{\"cases\":432,\"GPU_frames\":864,\"comparison\":\"production Gallery Content opt-in versus independent original shaper literal A…\"}\n").unwrap();
        }
    }
}
