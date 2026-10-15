//! CoreText 实现（仅 macOS）。
mod fonts;
mod rasterizer;
mod shaper;
pub use fonts::CoreTextFonts;
pub use rasterizer::{CoreTextRasterizer, PADDING};
pub use shaper::CoreTextShaper;
