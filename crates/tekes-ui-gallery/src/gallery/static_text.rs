//! 静态文字页的状态与数据：夹具、参照图、差异。页面怎么画见 `pages/static_text.rs`。
//!
//! # 夹具
//!
//! 夹具文件和 manifest 在编译时嵌进来（`include_str!`，来自仓库的 `fixtures/text/`），与运行目录无关。
//! 文本原样使用，包括文件末尾的换行。选择器里的夹具顺序是 [`FIXTURES`] 的顺序，宽度取自 manifest。
//!
//! # 参照图
//!
//! 环境变量 `TEKESUI_REFERENCE_DIR` 指向一个本地目录（私有伴随仓库的 `reference/`，**不进本仓库**），
//! 目录里按 `<夹具>/<夹具>-w<宽度>@<倍率>x-<light|dark>.png` 放图，另有可选的 `appearances.json`
//! 给出两种外观的前景与背景色。倍率是窗口 backing scale 四舍五入；深色与深色加高对比度都取 dark，
//! 浅色与浅色加高对比度都取 light（参照侧没有高对比度版本）。
//!
//! # 差异
//!
//! 参照图存在、尺寸与预期一致（宽度 x 倍率，高度 ceil(段落高度) x 倍率）时，把同一个段落离屏渲染成
//! 同样大小的画布，用 `compare::diff_rgba8` 比较，门槛取 `DiffOptions::for_scale`。
//! 结果按 `(夹具, 宽度, 倍率, 浅/深)` 缓存，状态不变时每帧只读缓存。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;

use tekes_ui_controls::{ButtonModel, ControlKey};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Point};
use tekes_ui_text::compare::{DiffOptions, diff_rgba8};
use tekes_ui_text::model::{Paragraph, ShapeOptions};
use tekes_ui_text::style::TextStyle;
use tekes_ui_tokens::{Appearance, Rgba, Theme};

use super::text::TextEngine;

/// 环境变量名。
pub const REFERENCE_ENV: &str = "TEKESUI_REFERENCE_DIR";

const MANIFEST: &str = include_str!("../../../../fixtures/text/manifest.json");

/// 选择器里的夹具：名字和嵌入的文本。
#[derive(Clone, Copy, Debug)]
pub struct FixtureDef {
    pub name: &'static str,
    pub text: &'static str,
}

macro_rules! fixture {
    ($name:literal) => {
        FixtureDef {
            name: $name,
            text: include_str!(concat!("../../../../fixtures/text/", $name, ".txt")),
        }
    };
}

/// 默认夹具在最前。
pub const FIXTURES: [FixtureDef; 8] = [
    fixture!("sentences-a"),
    fixture!("mixed-cjk-latin"),
    fixture!("kinsoku"),
    fixture!("emoji"),
    fixture!("combining"),
    fixture!("rtl"),
    fixture!("long-paragraph"),
    fixture!("gallery-reading-en"),
];

pub const DEFAULT_WIDTH: u32 = 700;
/// 静态文字页的字体：系统界面字体 14pt，行间距 0。
pub const FIXTURE_FONT_SIZE: f32 = 14.0;
/// 每个窗格显示的高度（pt）。排版结果比它高时只显示放得下的前几行整行。
pub const VIEW_H: f32 = 170.0;

/// manifest 里每个夹具的宽度，顺序同 [`FIXTURES`]。
pub fn manifest_widths() -> &'static [Vec<u32>] {
    static WIDTHS: OnceLock<Vec<Vec<u32>>> = OnceLock::new();
    WIDTHS.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap_or_default();
        FIXTURES
            .iter()
            .map(|f| {
                let found = v["fixtures"].as_array().and_then(|a| {
                    a.iter()
                        .find(|x| x["name"].as_str() == Some(f.name))
                        .and_then(|x| x["widths"].as_array())
                });
                let widths: Vec<u32> = found
                    .map(|a| {
                        a.iter()
                            .filter_map(|w| w.as_u64().map(|w| w as u32))
                            .collect()
                    })
                    .unwrap_or_default();
                if widths.is_empty() {
                    vec![DEFAULT_WIDTH]
                } else {
                    widths
                }
            })
            .collect()
    })
}

// ---------------------------------------------------------------- 参照图路径与外观

/// 参照图文件名：`<夹具>-w<宽度>@<倍率>x-<light|dark>.png`。
pub fn reference_file_name(fixture: &str, width: u32, scale: u32, dark: bool) -> String {
    format!(
        "{fixture}-w{width}@{scale}x-{}.png",
        if dark { "dark" } else { "light" }
    )
}

/// 参照图路径：`<dir>/<夹具>/<文件名>`。
pub fn reference_path(dir: &Path, fixture: &str, width: u32, scale: u32, dark: bool) -> PathBuf {
    dir.join(fixture)
        .join(reference_file_name(fixture, width, scale, dark))
}

/// 窗口 backing scale 四舍五入到整数倍率（至少 1）。
pub fn scale_to_int(scale: f32) -> u32 {
    (scale.round() as i64).max(1) as u32
}

/// 主题对应的参照外观：深色（含高对比度）取 dark，浅色取 light。
pub fn is_dark(theme: Theme) -> bool {
    theme.appearance == Appearance::Dark
}

/// 一种外观的前景与背景（8 位 sRGB 编码值）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub fg: [u8; 3],
    pub bg: [u8; 3],
}

impl Colors {
    pub fn fg(self) -> Rgba {
        Rgba::rgb8(self.fg[0], self.fg[1], self.fg[2])
    }
    pub fn bg(self) -> Rgba {
        Rgba::rgb8(self.bg[0], self.bg[1], self.bg[2])
    }
    pub fn bg_rgba8(self) -> [u8; 4] {
        [self.bg[0], self.bg[1], self.bg[2], 255]
    }
}

/// 没有 `appearances.json` 时的默认颜色。
pub fn default_colors(dark: bool) -> Colors {
    if dark {
        Colors {
            fg: [230, 230, 230],
            bg: [30, 30, 30],
        }
    } else {
        Colors {
            fg: [0, 0, 0],
            bg: [255, 255, 255],
        }
    }
}

/// 读 `<dir>/appearances.json` 里的颜色；没有目录、没有文件或格式不对都退回默认值。
pub fn load_colors(dir: Option<&Path>, dark: bool) -> Colors {
    let fallback = default_colors(dark);
    let Some(dir) = dir else { return fallback };
    let Ok(text) = std::fs::read_to_string(dir.join("appearances.json")) else {
        return fallback;
    };
    parse_colors(&text, dark).unwrap_or(fallback)
}

fn parse_colors(text: &str, dark: bool) -> Option<Colors> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let name = if dark { "dark" } else { "light" };
    let entry = v["appearances"]
        .as_array()?
        .iter()
        .find(|a| a["name"].as_str() == Some(name))?;
    let rgb = |key: &str| -> Option<[u8; 3]> {
        let a = entry[key].as_array()?;
        if a.len() != 3 {
            return None;
        }
        let mut out = [0u8; 3];
        for (o, x) in out.iter_mut().zip(a) {
            *o = u8::try_from(x.as_u64()?).ok()?;
        }
        Some(out)
    };
    Some(Colors {
        fg: rgb("foreground")?,
        bg: rgb("background")?,
    })
}

// ---------------------------------------------------------------- PNG

/// 解码后的参照图：RGBA8，预乘 alpha，行紧密排列。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug)]
pub enum LoadError {
    NotFound,
    Invalid(String),
}

/// 读一张 PNG（8 位或 16 位，灰度/灰度加 alpha/RGB/RGBA/调色板），转成预乘 RGBA8。
pub fn load_png(path: &Path) -> Result<Bitmap, LoadError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::NotFound),
        Err(e) => return Err(LoadError::Invalid(e.to_string())),
    };
    decode_png(&bytes)
}

fn decode_png(bytes: &[u8]) -> Result<Bitmap, LoadError> {
    let invalid = |e: png::DecodingError| LoadError::Invalid(e.to_string());
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(invalid)?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(invalid)?;
    let (w, h) = (info.width, info.height);
    let data = &buf[..info.buffer_size()];
    let n = (w as usize) * (h as usize);
    let mut rgba = Vec::with_capacity(n * 4);
    match info.color_type {
        png::ColorType::Rgba => {
            for px in data.chunks_exact(4) {
                rgba.extend_from_slice(&premul(px[0], px[1], px[2], px[3]));
            }
        }
        png::ColorType::Rgb => {
            for px in data.chunks_exact(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in data.chunks_exact(2) {
                rgba.extend_from_slice(&premul(px[0], px[0], px[0], px[1]));
            }
        }
        png::ColorType::Grayscale => {
            for &g in data {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => {
            return Err(LoadError::Invalid("调色板图未展开".into()));
        }
    }
    if rgba.len() != n * 4 {
        return Err(LoadError::Invalid("像素数据长度不符".into()));
    }
    Ok(Bitmap {
        width: w,
        height: h,
        rgba,
    })
}

fn premul(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
    let m = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
    [m(r), m(g), m(b), a]
}

// ---------------------------------------------------------------- 结果

/// 缓存键：`(夹具, 宽度, 倍率, 浅/深)`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResultKey {
    pub fixture: usize,
    pub width: u32,
    pub scale: u32,
    pub dark: bool,
}

/// 参照图的状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceStatus {
    /// 没设环境变量。
    NoDir,
    /// 目录里没有这张图（只记文件名，不记目录）。
    Missing {
        file: String,
    },
    Invalid {
        file: String,
        reason: String,
    },
    SizeMismatch {
        file: String,
        got: (u32, u32),
        expected: (u32, u32),
    },
    /// 离屏渲染失败。
    RenderFailed {
        reason: String,
    },
    Loaded,
}

/// 差异的三个数字和判定。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Numbers {
    pub ratio: f64,
    pub max_ratio: f64,
    pub max_component: usize,
    pub max_component_limit: usize,
    pub max_pixel_diff: u8,
    pub passed: bool,
}

/// 一个键下算出的全部东西。
#[derive(Clone, Debug, PartialEq)]
pub struct Computed {
    pub key: ResultKey,
    pub colors: Colors,
    pub status: ReferenceStatus,
    /// 登记到渲染器的参照图（已裁到显示高度）。
    pub reference_image: Option<ImageId>,
    pub heat_image: Option<ImageId>,
    pub numbers: Option<Numbers>,
    /// 登记的图像的像素尺寸（宽，裁剪后的高）。
    pub image_px: (u32, u32),
}

impl Computed {
    fn without_images(key: ResultKey, colors: Colors, status: ReferenceStatus) -> Computed {
        Computed {
            key,
            colors,
            status,
            reference_image: None,
            heat_image: None,
            numbers: None,
            image_px: (0, 0),
        }
    }

    fn images(&self) -> impl Iterator<Item = ImageId> {
        [self.reference_image, self.heat_image]
            .into_iter()
            .flatten()
    }
}

/// 排好的段落和它的显示裁剪。
#[derive(Debug)]
pub struct ShapedFixture {
    pub paragraph: Paragraph,
    /// 只含放得下的前几行整行，高度为 `crop_pt`。
    pub cropped: Paragraph,
    pub total_lines: usize,
    pub shown_lines: usize,
    /// 显示区高度（pt，整数）：最后一个显示行的底，向上取整。
    pub crop_pt: f32,
    /// 画布高度（pt，整数）：`ceil(段落高度)`。
    pub canvas_h: u32,
}

/// 按 [`VIEW_H`] 把段落裁成前几行整行（至少一行）。
pub fn crop_paragraph(paragraph: &Paragraph) -> ShapedFixture {
    let mut shown = 0;
    let mut bottom = 0.0f64;
    for line in &paragraph.lines {
        let b = line.top + line.height;
        if shown > 0 && b > f64::from(VIEW_H) + 1e-6 {
            break;
        }
        bottom = b;
        shown += 1;
    }
    let mut cropped = paragraph.clone();
    cropped.lines.truncate(shown);
    let crop_pt = bottom.ceil().max(1.0) as f32;
    cropped.height = f64::from(crop_pt);
    ShapedFixture {
        total_lines: paragraph.lines.len(),
        shown_lines: shown,
        crop_pt,
        canvas_h: paragraph.height.ceil().max(1.0) as u32,
        cropped,
        paragraph: paragraph.clone(),
    }
}

/// 取参照图并比较的纯逻辑部分（不依赖 GPU）：读文件、核对尺寸。
pub fn resolve_reference(
    dir: &Path,
    key: ResultKey,
    expected_px: (u32, u32),
) -> Result<Bitmap, ReferenceStatus> {
    let name = FIXTURES[key.fixture].name;
    let file = reference_file_name(name, key.width, key.scale, key.dark);
    let path = reference_path(dir, name, key.width, key.scale, key.dark);
    let bmp = load_png(&path).map_err(|e| match e {
        LoadError::NotFound => ReferenceStatus::Missing { file: file.clone() },
        LoadError::Invalid(reason) => ReferenceStatus::Invalid {
            file: file.clone(),
            reason,
        },
    })?;
    if (bmp.width, bmp.height) != expected_px {
        return Err(ReferenceStatus::SizeMismatch {
            file,
            got: (bmp.width, bmp.height),
            expected: expected_px,
        });
    }
    Ok(bmp)
}

/// 算出一个键下的结果，并把要显示的图像登记到渲染器。
///
/// 没有参照目录时不渲染也不登记任何东西。
pub fn compute(
    engine: &mut TextEngine,
    renderer: &mut Renderer,
    key: ResultKey,
    shaped: &ShapedFixture,
    dir: Option<&Path>,
) -> Computed {
    let colors = load_colors(dir, key.dark);
    let Some(dir) = dir else {
        return Computed::without_images(key, colors, ReferenceStatus::NoDir);
    };
    let expected = (key.width * key.scale, shaped.canvas_h * key.scale);
    let reference = match resolve_reference(dir, key, expected) {
        Ok(b) => b,
        Err(status) => return Computed::without_images(key, colors, status),
    };
    let live = match engine.render_paragraph_offscreen(
        renderer,
        &shaped.paragraph,
        expected,
        key.scale as f32,
        colors.fg(),
        colors.bg(),
    ) {
        Ok(p) => p,
        Err(e) => {
            return Computed::without_images(
                key,
                colors,
                ReferenceStatus::RenderFailed {
                    reason: e.to_string(),
                },
            );
        }
    };
    let opts = DiffOptions::for_scale(
        f64::from(key.scale),
        f64::from(FIXTURE_FONT_SIZE) * f64::from(key.scale),
        colors.bg_rgba8(),
    );
    let report = diff_rgba8(&reference.rgba, &live, expected.0, expected.1, opts);
    let numbers = Numbers {
        ratio: report.ratio,
        max_ratio: report.options.max_ratio,
        max_component: report.max_component_area,
        max_component_limit: report.options.max_component_area,
        max_pixel_diff: report.max_pixel_diff,
        passed: report.passed(),
    };
    // 只登记显示区那几行：每行 4 字节，取前 crop_px 行。
    let crop_px = ((shaped.crop_pt as u32) * key.scale).min(expected.1);
    let prefix = (expected.0 as usize) * (crop_px as usize) * 4;
    let reg = |renderer: &mut Renderer, rgba: &[u8]| {
        renderer
            .register_image(&rgba[..prefix], expected.0, crop_px)
            .ok()
    };
    Computed {
        key,
        colors,
        status: ReferenceStatus::Loaded,
        reference_image: reg(renderer, &reference.rgba),
        heat_image: reg(renderer, &report.heatmap),
        numbers: Some(numbers),
        image_px: (expected.0, crop_px),
    }
}

// ---------------------------------------------------------------- 状态

/// 选择器里的一次点击。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Fixture(usize),
    Width(u32),
}

/// 结果缓存最多留几项（超出丢最旧的，并注销它的图像）。
const CACHE_CAP: usize = 6;

#[derive(Debug)]
pub struct StaticText {
    fixture: usize,
    width: u32,
    fixture_buttons: Vec<ButtonModel>,
    width_buttons: Vec<ButtonModel>,
    focus_index: Option<usize>,
    reference_dir: Option<PathBuf>,
    shaped: HashMap<(usize, u32), Rc<ShapedFixture>>,
    results: Vec<Computed>,
    /// 到目前为止算过几次（测试用：状态不变时不增加）。
    computations: u64,
}

impl StaticText {
    pub fn new(reference_dir: Option<PathBuf>) -> StaticText {
        StaticText {
            fixture: 0,
            width: DEFAULT_WIDTH,
            fixture_buttons: (0..FIXTURES.len())
                .map(|_| ButtonModel::default())
                .collect(),
            width_buttons: (0..manifest_widths().iter().map(Vec::len).max().unwrap_or(0))
                .map(|_| ButtonModel::default())
                .collect(),
            focus_index: None,
            reference_dir,
            shaped: HashMap::new(),
            results: Vec::new(),
            computations: 0,
        }
    }

    /// 读 `TEKESUI_REFERENCE_DIR`；空串当作没设。
    pub fn from_env() -> StaticText {
        let dir = std::env::var_os(REFERENCE_ENV)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        StaticText::new(dir)
    }

    pub fn fixture(&self) -> usize {
        self.fixture
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn reference_dir(&self) -> Option<&Path> {
        self.reference_dir.as_deref()
    }

    #[cfg(test)]
    pub fn computations(&self) -> u64 {
        self.computations
    }

    /// 当前夹具可选的宽度。
    pub fn widths(&self) -> &'static [u32] {
        &manifest_widths()[self.fixture]
    }

    /// 选夹具。宽度不在新夹具的列表里就退回默认宽度。返回是否有变化。
    pub fn select_fixture(&mut self, i: usize) -> bool {
        if i >= FIXTURES.len() || i == self.fixture {
            return false;
        }
        self.fixture = i;
        if !self.widths().contains(&self.width) {
            self.width = DEFAULT_WIDTH;
        }
        true
    }

    /// 选宽度（必须在当前夹具的列表里）。返回是否有变化。
    pub fn select_width(&mut self, w: u32) -> bool {
        if w == self.width || !self.widths().contains(&w) {
            return false;
        }
        self.width = w;
        true
    }

    pub fn apply(&mut self, pick: Pick) -> bool {
        match pick {
            Pick::Fixture(i) => self.select_fixture(i),
            Pick::Width(w) => self.select_width(w),
        }
    }

    pub fn fixture_button_mut(&mut self, index: usize) -> Option<&mut ButtonModel> {
        self.fixture_buttons.get_mut(index)
    }

    pub fn width_button_mut(&mut self, index: usize) -> Option<&mut ButtonModel> {
        self.width_buttons.get_mut(index)
    }

    pub(super) fn fixture_buttons_mut(&mut self) -> &mut [ButtonModel] {
        &mut self.fixture_buttons
    }

    pub(super) fn width_buttons_mut(&mut self) -> &mut [ButtonModel] {
        &mut self.width_buttons
    }

    pub(super) fn focus_index(&self) -> Option<usize> {
        self.focus_index
    }

    pub fn fixture_buttons(&self) -> &[ButtonModel] {
        &self.fixture_buttons
    }

    pub fn width_buttons(&self) -> &[ButtonModel] {
        &self.width_buttons
    }

    /// Update hover through each retained production Button placement.
    pub fn pointer_move(&mut self, point: Point) -> bool {
        let mut changed = false;
        for button in self
            .fixture_buttons
            .iter_mut()
            .chain(self.width_buttons.iter_mut())
        {
            let was_hovered = button.state.hovered;
            button.pointer_move_at(point);
            changed |= was_hovered != button.state.hovered;
        }
        changed
    }

    /// Arm the visible selector Button under the pointer.
    pub fn pointer_down(&mut self, point: Point) -> bool {
        self.cancel_pointer();
        self.pointer_move(point);
        let order = self.focus_order();
        let index = order.iter().position(|pick| {
            self.button(*pick)
                .and_then(ButtonModel::layout)
                .is_some_and(|layout| layout.hit_test(point))
        });
        let Some(index) = index else {
            return false;
        };
        let pick = order[index];
        self.set_focus(Some(index), false);
        self.button_mut(pick).is_some_and(|button| {
            // A pointer press changes the activation source intentionally; discard a held
            // keyboard activation on this same target before arming the pointer.
            button.cancel();
            button.pointer_down_at(point)
        })
    }

    /// Activate only the Button that was armed and released within its rendered bounds.
    pub fn pointer_up(&mut self, point: Point) -> bool {
        let mut activated = None;
        let widths = self.widths().to_vec();
        for (index, button) in self.fixture_buttons.iter_mut().enumerate() {
            if button.pointer_up_at(point) {
                activated = Some(Pick::Fixture(index));
            }
        }
        for (index, button) in self.width_buttons.iter_mut().enumerate() {
            if button.pointer_up_at(point)
                && let Some(width) = widths.get(index)
            {
                activated = Some(Pick::Width(*width));
            }
        }
        activated.is_some_and(|pick| self.apply(pick))
    }

    pub fn cancel_pointer(&mut self) {
        for button in self
            .fixture_buttons
            .iter_mut()
            .chain(self.width_buttons.iter_mut())
        {
            button.cancel_pointer();
        }
    }

    pub(super) fn focus_order(&self) -> Vec<Pick> {
        self.fixture_buttons
            .iter()
            .enumerate()
            .filter(|(_, button)| {
                button
                    .layout()
                    .is_some_and(|layout| layout.visible.is_some())
            })
            .map(|(index, _)| Pick::Fixture(index))
            .chain(
                self.width_buttons
                    .iter()
                    .enumerate()
                    .filter(|(_, button)| {
                        button
                            .layout()
                            .is_some_and(|layout| layout.visible.is_some())
                    })
                    .filter_map(|(index, _)| self.widths().get(index).copied().map(Pick::Width)),
            )
            .collect()
    }

    fn button_mut(&mut self, pick: Pick) -> Option<&mut ButtonModel> {
        match pick {
            Pick::Fixture(index) => self.fixture_buttons.get_mut(index),
            Pick::Width(width) => {
                let index = self
                    .widths()
                    .iter()
                    .position(|candidate| *candidate == width)?;
                self.width_buttons.get_mut(index)
            }
        }
    }

    fn button(&self, pick: Pick) -> Option<&ButtonModel> {
        match pick {
            Pick::Fixture(index) => self.fixture_buttons.get(index),
            Pick::Width(width) => {
                let index = self
                    .widths()
                    .iter()
                    .position(|candidate| *candidate == width)?;
                self.width_buttons.get(index)
            }
        }
    }

    pub(super) fn set_focus(&mut self, index: Option<usize>, keyboard: bool) {
        if self.focus_index != index
            && let Some(previous) = self
                .focus_index
                .and_then(|focused| self.focus_order().get(focused).copied())
            && let Some(button) = self.button_mut(previous)
        {
            button.cancel();
        }
        for button in self
            .fixture_buttons
            .iter_mut()
            .chain(self.width_buttons.iter_mut())
        {
            button.state.focused = false;
            button.state.focus_visible = false;
        }
        self.focus_index = index;
        if let Some(pick) = index.and_then(|index| self.focus_order().get(index).copied())
            && let Some(button) = self.button_mut(pick)
        {
            button.state.focused = true;
            button.state.focus_visible = keyboard;
        }
    }

    /// Route keyboard focus and activation through the same retained Button models.
    pub fn handle_key(&mut self, key: ControlKey, down: bool) -> bool {
        let order = self.focus_order();
        if let ControlKey::Tab { reverse } = key {
            if !down || order.is_empty() {
                return false;
            }
            let next = match self.focus_index {
                Some(current) if reverse => current.checked_sub(1).unwrap_or(order.len() - 1),
                Some(current) => (current + 1) % order.len(),
                None if reverse => order.len() - 1,
                None => 0,
            };
            self.set_focus(Some(next), true);
            return true;
        }
        let Some(index) = self.focus_index.filter(|index| *index < order.len()) else {
            return false;
        };
        let pick = order[index];
        let activated = self
            .button_mut(pick)
            .is_some_and(|button| button.key(key, down));
        if activated {
            self.apply(pick);
        }
        activated || matches!(key, ControlKey::Enter | ControlKey::Space)
    }

    pub fn blur(&mut self) {
        self.cancel_pointer();
        for button in self
            .fixture_buttons
            .iter_mut()
            .chain(self.width_buttons.iter_mut())
        {
            button.blur();
        }
        self.focus_index = None;
    }

    pub fn key(&self, theme: Theme, scale: f32) -> ResultKey {
        ResultKey {
            fixture: self.fixture,
            width: self.width,
            scale: scale_to_int(scale),
            dark: is_dark(theme),
        }
    }

    /// 当前夹具与宽度排好的段落（第一次用时排版，之后读缓存）。
    pub fn shaped(&mut self, engine: &mut TextEngine) -> Rc<ShapedFixture> {
        let k = (self.fixture, self.width);
        if let Some(s) = self.shaped.get(&k) {
            return s.clone();
        }
        let style = TextStyle::system(FIXTURE_FONT_SIZE);
        let options = ShapeOptions::with_width(f64::from(self.width));
        let p = engine.shape(FIXTURES[self.fixture].text, &style, &options);
        let s = Rc::new(crop_paragraph(&p));
        self.shaped.insert(k, s.clone());
        s
    }

    /// 当前键下已经算好的结果。
    pub fn result(&self, theme: Theme, scale: f32) -> Option<&Computed> {
        let key = self.key(theme, scale);
        self.results.iter().find(|c| c.key == key)
    }

    /// 保证 `key` 有结果；没有就调用 `f` 算一次。返回要注销的图像（被挤出缓存的）。
    pub fn ensure(&mut self, key: ResultKey, f: impl FnOnce() -> Computed) -> Vec<ImageId> {
        if self.results.iter().any(|c| c.key == key) {
            return Vec::new();
        }
        self.computations += 1;
        let c = f();
        let mut released = Vec::new();
        if self.results.len() >= CACHE_CAP {
            released.extend(self.results.remove(0).images());
        }
        self.results.push(c);
        released
    }

    /// 为当前状态备好结果（排版、读参照图、离屏渲染、差异、登记图像）。状态没变时什么都不做。
    pub fn prepare(
        &mut self,
        engine: &mut TextEngine,
        renderer: &mut Renderer,
        theme: Theme,
        scale: f32,
    ) {
        let key = self.key(theme, scale);
        if self.results.iter().any(|c| c.key == key) {
            return;
        }
        let shaped = self.shaped(engine);
        let dir = self.reference_dir.clone();
        let released = self.ensure(key, || {
            compute(engine, renderer, key, &shaped, dir.as_deref())
        });
        for id in released {
            renderer.unregister_image(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "tekes-gallery-test-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_png(path: &Path, w: u32, h: u32, color: png::ColorType, data: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut enc = png::Encoder::new(std::fs::File::create(path).unwrap(), w, h);
        enc.set_color(color);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(data).unwrap();
    }

    #[test]
    fn fixtures_match_manifest() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let names: Vec<&str> = v["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        for f in FIXTURES {
            assert!(names.contains(&f.name), "{}", f.name);
            assert!(f.text.ends_with('\n'), "{}：夹具应以换行结尾", f.name);
        }
        assert_eq!(FIXTURES[0].name, "sentences-a");
        let w = manifest_widths();
        assert_eq!(w[0], vec![700]);
        assert_eq!(w[1], vec![700, 500, 320]);
        assert_eq!(w.len(), FIXTURES.len());
    }

    #[test]
    fn file_name_and_path() {
        assert_eq!(
            reference_file_name("sentences-a", 700, 2, false),
            "sentences-a-w700@2x-light.png"
        );
        assert_eq!(
            reference_path(Path::new("/r"), "emoji", 320, 1, true),
            PathBuf::from("/r/emoji/emoji-w320@1x-dark.png")
        );
        assert_eq!(scale_to_int(2.0), 2);
        assert_eq!(scale_to_int(1.0), 1);
        assert_eq!(scale_to_int(1.5), 2);
        assert_eq!(scale_to_int(0.2), 1);
        assert!(is_dark(Theme::DARK) && is_dark(Theme::DARK_INCREASED));
        assert!(!is_dark(Theme::LIGHT) && !is_dark(Theme::LIGHT_INCREASED));
    }

    #[test]
    fn reference_resolution_with_a_fake_png() {
        let dir = temp_dir("ref");
        let key = ResultKey {
            fixture: 0,
            width: 700,
            scale: 2,
            dark: true,
        };
        // 没有文件。
        assert_eq!(
            resolve_reference(&dir, key, (4, 2)).unwrap_err(),
            ReferenceStatus::Missing {
                file: "sentences-a-w700@2x-dark.png".into()
            }
        );
        // 放一张 4x2 的 RGB 假图。
        let path = reference_path(&dir, "sentences-a", 700, 2, true);
        let rgb: Vec<u8> = (0..4 * 2 * 3).map(|i| i as u8).collect();
        write_png(&path, 4, 2, png::ColorType::Rgb, &rgb);
        let bmp = resolve_reference(&dir, key, (4, 2)).unwrap();
        assert_eq!((bmp.width, bmp.height), (4, 2));
        assert_eq!(&bmp.rgba[..8], &[0, 1, 2, 255, 3, 4, 5, 255]);
        // 尺寸不符。
        assert!(matches!(
            resolve_reference(&dir, key, (8, 4)),
            Err(ReferenceStatus::SizeMismatch {
                got: (4, 2),
                expected: (8, 4),
                ..
            })
        ));
        // 浅色那张不存在：深浅各取各的文件。
        let light = ResultKey { dark: false, ..key };
        assert!(matches!(
            resolve_reference(&dir, light, (4, 2)),
            Err(ReferenceStatus::Missing { .. })
        ));
        // 坏文件。
        std::fs::write(&path, b"not a png").unwrap();
        assert!(matches!(
            resolve_reference(&dir, key, (4, 2)),
            Err(ReferenceStatus::Invalid { .. })
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rgba_png_is_premultiplied() {
        let dir = temp_dir("premul");
        let path = dir.join("a.png");
        write_png(
            &path,
            2,
            1,
            png::ColorType::Rgba,
            &[200, 100, 50, 128, 1, 2, 3, 255],
        );
        let b = load_png(&path).unwrap();
        assert_eq!(b.rgba, vec![100, 50, 25, 128, 1, 2, 3, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn colors_default_and_from_appearances_json() {
        assert_eq!(default_colors(false).fg, [0, 0, 0]);
        assert_eq!(default_colors(true).bg, [30, 30, 30]);
        assert_eq!(load_colors(None, true), default_colors(true));
        let dir = temp_dir("colors");
        // 没有文件：默认。
        assert_eq!(load_colors(Some(&dir), false), default_colors(false));
        let json = r#"{"appearances":[
            {"name":"light","foreground":[1,2,3],"background":[250,251,252]},
            {"name":"dark","foreground":[200,201,202],"background":[10,11,12]}]}"#;
        std::fs::write(dir.join("appearances.json"), json).unwrap();
        assert_eq!(
            load_colors(Some(&dir), false),
            Colors {
                fg: [1, 2, 3],
                bg: [250, 251, 252]
            }
        );
        assert_eq!(load_colors(Some(&dir), true).bg, [10, 11, 12]);
        // 格式不对：默认。
        std::fs::write(dir.join("appearances.json"), "{}").unwrap();
        assert_eq!(load_colors(Some(&dir), true), default_colors(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selection_rules() {
        let mut s = StaticText::new(None);
        assert_eq!((s.fixture(), s.width()), (0, 700));
        // sentences-a 只有 700。
        assert!(!s.select_width(500));
        assert!(s.select_fixture(1));
        assert!(s.select_width(500));
        assert!(!s.select_width(500));
        assert!(!s.select_fixture(1));
        // 切到只有 700 的夹具，宽度退回 700。
        assert!(s.select_fixture(0));
        assert_eq!(s.width(), 700);
        assert!(!s.select_fixture(99));
        assert!(s.apply(Pick::Fixture(2)));
    }

    #[test]
    fn ensure_computes_once_per_key_and_evicts_oldest() {
        let mut s = StaticText::new(None);
        let key = |fixture, scale| ResultKey {
            fixture,
            width: 700,
            scale,
            dark: false,
        };
        let colors = default_colors(false);
        let mut calls = 0;
        let mut run = |s: &mut StaticText, k: ResultKey| {
            s.ensure(k, || {
                calls += 1;
                Computed {
                    reference_image: Some(ImageId(calls)),
                    ..Computed::without_images(k, colors, ReferenceStatus::NoDir)
                }
            })
        };
        assert!(run(&mut s, key(0, 2)).is_empty());
        // 同一个键：不再计算。
        for _ in 0..10 {
            assert!(run(&mut s, key(0, 2)).is_empty());
        }
        assert_eq!(s.computations(), 1);
        // 不同的键各算一次，直到超出容量，最旧的被挤出，图像交还给调用方注销。
        for i in 1..CACHE_CAP {
            assert!(run(&mut s, key(i, 2)).is_empty());
        }
        let released = run(&mut s, key(0, 1));
        assert_eq!(released, vec![ImageId(1)]);
        assert_eq!(s.computations(), CACHE_CAP as u64 + 1);
    }

    #[test]
    fn crop_keeps_whole_lines_that_fit() {
        let mut e = TextEngine::new();
        let style = TextStyle::system(FIXTURE_FONT_SIZE);
        let p = e.shape(
            FIXTURES[0].text,
            &style,
            &ShapeOptions::with_width(f64::from(700u32)),
        );
        let s = crop_paragraph(&p);
        assert_eq!(s.total_lines, p.lines.len());
        assert!(s.shown_lines < s.total_lines);
        assert!(s.crop_pt <= VIEW_H);
        assert_eq!(s.cropped.lines.len(), s.shown_lines);
        // 下一行放不下。
        let next = &p.lines[s.shown_lines];
        assert!(next.top + next.height > f64::from(VIEW_H));
        assert_eq!(s.canvas_h, p.height.ceil() as u32);
        // 短文本不裁。
        let short = e.shape("abc", &style, &ShapeOptions::with_width(100.0));
        let c = crop_paragraph(&short);
        assert_eq!(c.shown_lines, c.total_lines);
    }

    /// 端到端：用自己的离屏渲染结果当“参照图”写进临时目录，`prepare` 应当读到它、
    /// 算出零差异并通过；再 `prepare` 一次不重新计算。没有 GPU 适配器时跳过。
    #[test]
    fn prepare_with_a_generated_reference_passes_and_is_cached() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("没有可用的 GPU 适配器，跳过");
            return;
        };
        let dir = temp_dir("e2e");
        let mut engine = TextEngine::new();
        let mut st = StaticText::new(Some(dir.clone()));
        let (theme, scale) = (Theme::LIGHT, 2.0);
        let shaped = st.shaped(&mut engine);
        let colors = default_colors(false);
        let size = (700 * 2, shaped.canvas_h * 2);
        let live = engine
            .render_paragraph_offscreen(
                &mut renderer,
                &shaped.paragraph,
                size,
                2.0,
                colors.fg(),
                colors.bg(),
            )
            .unwrap();
        // 画布不透明，RGBA 里的 A 全是 255，写成 RGB 的 PNG。
        let rgb: Vec<u8> = live
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        write_png(
            &reference_path(&dir, "sentences-a", 700, 2, false),
            size.0,
            size.1,
            png::ColorType::Rgb,
            &rgb,
        );
        st.prepare(&mut engine, &mut renderer, theme, scale);
        let c = st.result(theme, scale).unwrap();
        assert_eq!(c.status, ReferenceStatus::Loaded);
        let n = c.numbers.unwrap();
        assert!(n.passed);
        assert_eq!((n.ratio, n.max_component, n.max_pixel_diff), (0.0, 0, 0));
        assert!(c.reference_image.is_some() && c.heat_image.is_some());
        assert_eq!(renderer.image_count(), 2);
        // 状态没变：不重算，不再登记图像。
        st.prepare(&mut engine, &mut renderer, theme, scale);
        st.prepare(&mut engine, &mut renderer, theme, scale);
        assert_eq!(st.computations(), 1);
        assert_eq!(renderer.image_count(), 2);
        // 深色那张图不存在：算一次，状态是 Missing，不登记图像。
        st.prepare(&mut engine, &mut renderer, Theme::DARK, scale);
        assert!(matches!(
            st.result(Theme::DARK, scale).unwrap().status,
            ReferenceStatus::Missing { .. }
        ));
        assert_eq!(st.computations(), 2);
        assert_eq!(renderer.image_count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
