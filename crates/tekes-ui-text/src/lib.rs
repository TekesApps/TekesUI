//! 文字：排版、字体回退、光栅化。
//!
//! # 三个 trait
//!
//! | trait | 作用 |
//! |---|---|
//! | [`FontFallback`] | 一段文字在某个样式下落在哪些字体上（按 UTF-16 / 字节范围分段） |
//! | [`Shaper`] | 按可选的换行宽度排版，得到 [`Paragraph`]：每行的范围、ascent/descent/leading、行高、基线、含/不含行尾空白的宽度、字形 run |
//! | [`Rasterizer`] | `(字体, 字形, scale)` 到 BGRA8 预乘位图 [`RasterGlyph`](tekes_ui_scene::RasterGlyph) |
//!
//! 排版结果用 [`push::push_paragraph`] 编进 [`Scene`](tekes_ui_scene::Scene)，字形位图经
//! [`GlyphAtlas`](tekes_ui_scene::GlyphAtlas) 缓存，由 `tekes-ui-render` 画出。
//!
//! 三个 trait 平台无关。macOS 上用 CoreText 实现（[`coretext`] 模块，只在 macOS 目标编译），
//! 以后 Windows 用 DirectWrite、Linux 用 Parley。mac 上排版、字体回退、光栅化都走 CoreText
//! 是 charter 的原则 2：macOS 15 苹方的 hvgl 字形表 FreeType 读不了。
//!
//! # CoreText 实现与参照实现的关系
//!
//! 行为对照 TekesMetalTextKit 的 `MetalTextShaper`（多行）与 `MetalGlyphAtlas.rasterize`，
//! 只借行为，不借代码。细节与有意的差异见 [`coretext`] 各模块的文档和
//! `docs/handoffs/F1-text.md`。
//!
//! # 其他模块
//!
//! - [`export`]：排版数据导出，JSON schema `tekesui-layout/0`（docs/schemas/layout-export.md）；
//! - [`compare`]：带容差的像素对比和差异热图（comparison-method 第 2.1 节）；
//! - [`coretext::render_oracle`]：用 CoreText 直绘的对照图，检验 atlas 和 GPU 路径。

pub mod compare;
pub mod export;
mod geometry;
pub mod model;
pub mod push;
pub mod style;
pub mod traits;
pub mod utf16;

#[cfg(target_os = "macos")]
pub mod coretext;

pub use model::{FontInfo, FontRun, Line, Paragraph, ShapeOptions, ShapedGlyph, ShapedRun};
pub use push::{push_paragraph, push_paragraph_visible};
pub use style::{FontFamily, TextStyle};
pub use traits::{FontFallback, Rasterizer, Shaper};
