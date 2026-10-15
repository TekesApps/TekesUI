//! 无头快照：用 `Renderer::new_headless` 把每一页在每个主题、每个缩放下渲染成 PNG。
//!
//! 文件名 `<页>-<主题>@<缩放>x.png`，例如 `primitives-dark-ic@2x.png`。
//! 页名与主题名见 `Page::slug`、`theme_slug`。尺寸是固定框 1200x800 pt 乘缩放。
//! 这是以后自动对比（章程第 2 节）共用的固定夹具：同样的输入逐字节得到同样的 PNG。
//!
//! 静态文字页固定为默认状态：夹具 sentences-a、宽度 700，**不带参照目录**（参照窗格是占位文字，
//! 差异窗格写“需要参照图才能比较”），所以快照与环境变量 `TEKESUI_REFERENCE_DIR` 无关。

use std::fs;
use std::path::{Path, PathBuf};

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

use super::layout::{FRAME_H, FRAME_W};
use super::static_text::StaticText;
use super::text::TextEngine;
use super::theme::theme_slug;
use super::{Gallery, Page, PageCtx, compose};

/// 默认要输出的缩放。
pub const SCALES: [u32; 2] = [2, 1];

#[derive(Debug)]
pub enum SnapshotError {
    Render(RenderError),
    Png(png::EncodingError),
    Io(std::io::Error),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Render(e) => write!(f, "渲染失败：{e}"),
            SnapshotError::Png(e) => write!(f, "PNG 编码失败：{e}"),
            SnapshotError::Io(e) => write!(f, "写文件失败：{e}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<RenderError> for SnapshotError {
    fn from(e: RenderError) -> Self {
        SnapshotError::Render(e)
    }
}
impl From<png::EncodingError> for SnapshotError {
    fn from(e: png::EncodingError) -> Self {
        SnapshotError::Png(e)
    }
}
impl From<std::io::Error> for SnapshotError {
    fn from(e: std::io::Error) -> Self {
        SnapshotError::Io(e)
    }
}

pub fn file_name(page: Page, theme: Theme, scale: u32) -> String {
    format!("{}-{}@{}x.png", page.slug(), theme_slug(theme), scale)
}

/// 渲染一页，返回 PNG 字节。`static_text` 用 `StaticText::new(None)` 的默认状态。
pub fn render_png(
    renderer: &mut Renderer,
    text: &mut TextEngine,
    static_text: &mut StaticText,
    page: Page,
    theme: Theme,
    scale: u32,
) -> Result<Vec<u8>, SnapshotError> {
    text.set_scale(scale as f32);
    text.begin_frame();
    let mut scene = Scene::new();
    {
        let mut ctx = PageCtx {
            theme,
            text: &mut *text,
            static_text,
        };
        compose(page, None, &mut ctx, &mut scene);
    }
    let size = (FRAME_W as u32 * scale, FRAME_H as u32 * scale);
    let rgba = renderer.render_to_rgba8(
        &scene,
        text.atlas_mut(),
        size,
        scale as f32,
        Gallery::background(theme),
    )?;
    encode_png(&rgba, size.0, size.1)
}

fn encode_png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, SnapshotError> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header()?;
        writer.write_image_data(rgba)?;
    }
    Ok(out)
}

/// 渲染全部页 x 主题 x 缩放，写进 `dir`，返回写出的文件。
pub fn write_all(dir: &Path) -> Result<Vec<PathBuf>, SnapshotError> {
    fs::create_dir_all(dir)?;
    let mut renderer = Renderer::new_headless()?;
    let mut text = TextEngine::new();
    let mut static_text = StaticText::new(None);
    let mut written = Vec::new();
    for page in Page::ALL {
        for theme in Theme::ALL {
            for scale in SCALES {
                let png = render_png(
                    &mut renderer,
                    &mut text,
                    &mut static_text,
                    page,
                    theme,
                    scale,
                )?;
                let path = dir.join(file_name(page, theme, scale));
                fs::write(&path, png)?;
                written.push(path);
            }
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names() {
        assert_eq!(
            file_name(Page::Primitives, Theme::LIGHT, 2),
            "primitives-light@2x.png"
        );
        assert_eq!(
            file_name(Page::Tokens, Theme::DARK_INCREASED, 1),
            "tokens-dark-ic@1x.png"
        );
        assert_eq!(
            file_name(Page::StaticText, Theme::DARK, 2),
            "static-text-dark@2x.png"
        );
    }

    /// 同一输入渲染两次，PNG 逐字节相同。没有 GPU 适配器时跳过。
    #[test]
    fn rendering_twice_is_byte_identical() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            eprintln!("没有可用的 GPU 适配器，跳过");
            return;
        };
        let mut text = TextEngine::new();
        let mut st = StaticText::new(None);
        for (page, theme, scale) in [
            (Page::Primitives, Theme::LIGHT, 2),
            (Page::Tokens, Theme::DARK_INCREASED, 1),
            (Page::StaticText, Theme::LIGHT, 2),
            (Page::StaticText, Theme::DARK, 1),
            (Page::StaticText, Theme::DARK_INCREASED, 2),
        ] {
            let a = render_png(&mut renderer, &mut text, &mut st, page, theme, scale).unwrap();
            let b = render_png(&mut renderer, &mut text, &mut st, page, theme, scale).unwrap();
            assert_eq!(a, b, "{page:?} {theme:?} {scale}x");
            // 用新的渲染器、新的文字引擎再来一次，排除内部状态（atlas 打包顺序、标签缓存）的影响。
            let mut fresh = Renderer::new_headless().unwrap();
            let c = render_png(
                &mut fresh,
                &mut TextEngine::new(),
                &mut StaticText::new(None),
                page,
                theme,
                scale,
            )
            .unwrap();
            assert_eq!(a, c, "{page:?} {theme:?} {scale}x（新渲染器）");
        }
    }

    #[test]
    fn png_has_expected_size() {
        let Ok(mut renderer) = Renderer::new_headless() else {
            return;
        };
        let bytes = render_png(
            &mut renderer,
            &mut TextEngine::new(),
            &mut StaticText::new(None),
            Page::Tokens,
            Theme::LIGHT,
            1,
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        assert_eq!((info.width, info.height), (FRAME_W as u32, FRAME_H as u32));
    }
}
