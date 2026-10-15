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
//! 现在只有 [`Quad`]，但 [`Scene`] 存的是 [`Primitive`] 枚举的有序列表，
//! 而不是一个 `Vec<Quad>`，也不是"每种图元一个批次"。原因是 painter's order
//! 要跨图元种类成立：F1 以后 quad、文字、图片会交错出现（文字压在 quad 上，
//! 另一个 quad 又压在文字上），分开成几个批次就无法保持叠放顺序。
//!
//! 取舍：
//!
//! - 单一有序列表，保证叠放顺序是显式的、唯一的；渲染层在提交时把**连续的同类
//!   图元**合并成一批（instanced draw），代价只是一次线性扫描。
//! - [`Primitive`] 标了 `#[non_exhaustive]`，以后加变体不算破坏性改动。
//!   计划中的变体：
//!   - F1：`GlyphRun`（引用 glyph atlas 里的字形，按 run 打包）；
//!   - 裁剪：`PushClip(Rect)` / `PopClip`，作为列表里的状态指令，而不是每个图元
//!     自带 clip，这样嵌套裁剪不需要给每个图元多存一份矩形；
//!   - 参照图槽位：`Image { rect, id }`，图像本身由渲染层按 id 管理。
//! - 图元体积小（`Quad` 约 60 字节），枚举的额外开销可以忽略。
//!
//! 不做的事：scene 不做脏区跟踪、不做命中测试、不做排序。上层每帧（按需）整体
//! 重建，渲染层只负责画。

mod geometry;
mod quad;
mod scene;

pub use geometry::{Point, Rect, Size};
pub use quad::{CornerRadii, Quad};
pub use scene::{Primitive, Scene};
