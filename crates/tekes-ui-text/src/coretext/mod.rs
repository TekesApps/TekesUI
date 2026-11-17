//! CoreText 实现（仅 macOS）。
mod fonts;
mod label;
#[cfg(feature = "qa")]
mod oracle;
mod rasterizer;
mod shaper;
pub use fonts::CoreTextFonts;
#[cfg(feature = "qa")]
pub use oracle::{OracleOptions, Placement, framesetter_line_ranges, render_oracle};
pub use rasterizer::{CoreTextRasterizer, PADDING};
pub use shaper::{CoreTextShaper, DisplayParagraph, PreparedText};
