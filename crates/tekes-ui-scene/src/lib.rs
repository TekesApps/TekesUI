//! display list 加 glyph atlas。
//!
//! 上层把内容编译成绘制指令，这一层不关心 GPU API。
//!
//! # 坐标与单位
//!
//! 所有几何量都是**逻辑点（pt）**，原点在左上角，y 向下。
//! 逻辑点到设备像素的换算（乘 scale）由渲染层完成，scene 不知道 scale。
//!
//! # 颜色
//!
//! 颜色是 [`tekes_ui_tokens::Rgba`]：sRGB 编码、非预乘 alpha。
//! 渲染层在 sRGB 空间里混合，见 `tekes-ui-render` 的模块文档。
//!
//! # 设计：一个有序的 `Vec<Primitive>`
//!
//! [`Scene`] 存的是 [`Primitive`] 枚举的有序列表，
//! 而不是一个 `Vec<Quad>`，也不是"每种图元一个批次"。原因是 painter's order
//! 要跨图元种类成立：F1 以后 quad、文字、图片会交错出现（文字压在 quad 上，
//! 另一个 quad 又压在文字上），分开成几个批次就无法保持叠放顺序。
//!
//! 取舍：
//!
//! - 单一有序列表，保证叠放顺序是显式的、唯一的；渲染层在提交时把**连续的同类
//!   图元**合并成一批（instanced draw），代价只是一次线性扫描。
//! - [`Primitive`] 标了 `#[non_exhaustive]`，以后加变体不算破坏性改动。
//!   变体：
//!   - 已有：[`Quad`]、[`ShadowQuad`]、`Glyphs`（引用 [`GlyphAtlas`] 位图）、
//!     `Image`（渲染层登记的图像）和变换图像；
//!   - 裁剪：`PushClip(Rect)` / `PopClip`，作为列表里的状态指令，而不是每个图元
//!     自带 clip，这样嵌套裁剪不需要给每个图元多存一份矩形；
//! - 图元体积小（`Quad` 约 60 字节），枚举的额外开销可以忽略。
//!
//! 不做的事：scene 不做脏区跟踪、不做命中测试、不做排序。上层每帧（按需）整体
//! 重建，渲染层只负责画。
//!
//! # 有界 CPU 裁剪几何（#80）
//!
//! [`ClipShape`] 和 [`ClipRegion`] 提供轴对齐 Rect/RoundedRect 的规范化、
//! 点/截面/可见边界查询；超出数值域或预算时显式返回 [`ClipError`]。
//! [`ShapeKey`] 保留原 [`ShapeTable`] 的 owner，不能跨表借用同 generation/slot。
//! 这些 CPU 查询不启用 Scene shape 指令或 GPU 裁剪；renderer、片段 append
//! 与 Host 集成分别验收。历史 Root 的 GPU 证据不等于此 main 基线已接收。

mod atlas;
mod clip;
mod geometry;
mod glyph;
mod quad;
mod scene;
mod shadow;

pub use atlas::{AtlasConfig, AtlasStats, DirtyRect, GlyphAtlas};
pub use clip::{
    ClipBudget, ClipError, ClipInterval, ClipRegion, ClipShape, FullCoverageInnerRectPx,
    InnerClipShape, ShapeKey, ShapeTable, VisibleClipBounds,
};
pub use geometry::{Point, Rect, Size};
pub use glyph::{
    AtlasGlyph, FontKey, GlyphKey, GlyphRun, ImageId, ImageQuad, PositionedGlyph, RasterFormat,
    RasterGlyph, TransformedImageQuad,
};
pub use quad::{CornerRadii, Quad};
pub use scene::{
    FragmentBudget, FragmentError, FragmentTransform, OpaqueImageLease, Primitive, Scene,
};
pub use shadow::ShadowQuad;
