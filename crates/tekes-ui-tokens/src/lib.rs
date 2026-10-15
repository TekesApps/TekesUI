//! TekesUI 设计 token。
//!
//! **临时占位：数值取自 Tekes 现有 Spacing / CornerRadius / Stroke / Motion / ControlSize / Alpha / ColorRole / TextRole，等 Design 交付的 tokens.json 生成后替换。**
//!
//! 内容：间距、圆角、描边、动效、控件尺寸、透明度、主题、颜色角色、文字角色。
//! 没有任何依赖。长度单位是 pt，时间单位是秒。

pub mod alpha;
pub mod color;
pub mod metrics;
pub mod text;
pub mod theme;

pub use alpha::Alpha;
pub use color::{ColorRole, Rgba};
pub use metrics::{ControlSize, CornerRadius, Motion, Spacing, Stroke};
pub use text::{TextRole, TextStyle};
pub use theme::{Appearance, Contrast, Theme};
