//! 渲染层，基于 wgpu。
//!
//! 接口保持窄，留出换成 objc2-metal 的余地。
//! 例如 mac 上需要更直接地控制 CAMetalLayer 和帧节奏时。
//! 公开 API 里没有 wgpu 的类型；输入是 [`tekes_ui_scene::Scene`]，错误是 [`RenderError`]。
//!
//! # 颜色管线：在 sRGB 编码空间里混合
//!
//! 渲染器**不做线性空间混合**，而是和 AppKit / CoreAnimation 一样，直接在 sRGB 编码
//! （gamma）空间里混合：
//!
//! - surface 和离屏目标用**非 sRGB** 格式 `Bgra8Unorm`，写进去的数值就是最终的 sRGB 编码值；
//! - token 里的 sRGB 颜色**原样**传给着色器，着色器只做预乘（`rgb * a`），不调用任何
//!   sRGB 到线性的转换，也不用 `*Srgb` 格式让硬件在写入时编码；
//! - 混合状态是 premultiplied alpha：`out = src + dst * (1 - src.a)`，直接作用在编码值上；
//! - 平台层把 `CAMetalLayer` 的 colorspace 设为 sRGB（本 crate 在每次配置 surface 后
//!   自己设置，见 [`Renderer::new_for_metal_layer`]），系统再把 sRGB 转成显示器色彩空间。
//!
//! 原因：对照对象 TekesMetalTextKit 与 AppKit 就是这样合成界面的。同样是黑色 50% 叠在白色
//! 上，gamma 空间混合得到 sRGB 值 128/255（视觉上偏深），线性空间混合得到约 188/255（偏浅）。
//! 想让半透明色（`hoverWash`、`Scrim`）、抗锯齿的边缘、文字边缘在 Gallery 里和参照图像素级
//! 对得上，就必须用同一种混合空间。线性混合在物理上更"正确"，但那样所有半透明颜色都要
//! 重新调，而且与系统控件并排时一眼能看出差别。
//!
//! 因此 [`tekes_ui_tokens::Rgba::to_linear`] 不在 UI 混合路径上使用。
//!
//! 抗锯齿也在编码空间里做：边缘覆盖率直接当 alpha，和 AppKit 画圆角矩形时一样。
//!
//! # 抗锯齿与几何
//!
//! 每个 quad 用实例化绘制：4 个顶点从 `vertex_index` 生成（triangle strip，没有顶点缓冲），
//! 每个实例一条记录。片元着色器按圆角矩形的有符号距离场（SDF）算覆盖率，
//! 过渡带约 1 个设备像素宽（距离边界 ±0.5 像素）；包围盒向外多画 1 个像素以容纳外侧
//! 的半个像素。与设备像素对齐的矩形边缘是干净的（不会糊），不对齐的边缘按覆盖率渐变。
//! 描边画在矩形内部，先把描边叠在 fill 上，再整体乘外轮廓的覆盖率，所以外缘只抗锯齿一次。
//!
//! 坐标：scene 是逻辑点，渲染器在着色器里乘 `scale` 变成设备像素；uniform 里放的是
//! 目标的像素尺寸和 `scale`（用实际像素尺寸转 NDC，避免 `点 * scale` 与 drawable
//! 尺寸取整不一致造成偏移）。
//!
//! # 字形
//!
//! 字形位图存在 [`tekes_ui_scene::GlyphAtlas`] 里（CPU 页缓冲是真值，规则见它的模块文档）。
//! [`Renderer::render`] 和 [`Renderer::render_to_rgba8`] 都接收 `&mut GlyphAtlas`：
//! 画之前先把 atlas 的脏区用 `Queue::write_texture` 上传到 GPU，然后画。
//!
//! - **纹理**：一张 `Bgra8Unorm` 的 `texture_2d_array`，一页一层，层数按 2 的幂增长到
//!   `max_pages`；增长时新建纹理并整页重传（CPU 页是真值，不从旧纹理拷贝）。选数组而不是
//!   "每页一张纹理"，是因为实例里带页号后一个 draw 能画任意页，而字形叠放顺序不能按页重排。
//! - **共享**：一个 atlas 可以被多个渲染器使用（各自一份 GPU 纹理）。atlas 记着上一次是谁
//!   取走了脏区，换了渲染器就整页重传，所以交替使用也是对的，只是多传一些。
//! - **位置**：位图左上角的设备像素坐标是 `round(origin.x * scale) + left`、
//!   `round(origin.y * scale) + top`，`round` 是四舍五入、0.5 向远离 0 的方向进位；
//!   取整在 CPU 侧用 Rust 的 `f32::round` 做（WGSL 的 `round` 是银行家舍入，不能用）。
//!   位图按 width x height 个设备像素原样贴，**不缩放、没有亚像素偏移**，
//!   所以片元着色器用 `textureLoad` 按整数坐标取（等价于 nearest 采样，没有过滤和 uv 取整的歧义）。
//! - **着色**：单色（`Mask`）输出 `(rgb * a * s.a, a * s.a)`，`a` 是 run 颜色的 alpha；
//!   彩色（`Color`）输出 `s * a`（采样值已预乘，`a` 只当不透明度）。不做伽马修正和对比度增强，
//!   覆盖率直接当 alpha，在 sRGB 编码空间里混合，和 quad 一样。
//! - **无效字形**：页号超出当前纹理层数的字形（不是这个 atlas 编出来的）被跳过。
//!
//! # 图像
//!
//! [`Renderer::register_image`] 登记一张预乘 alpha 的 RGBA8 图像（`Rgba8Unorm`，数值原样当作
//! sRGB 编码值），返回 [`tekes_ui_scene::ImageId`]；scene 里的 `Image` 图元把它线性采样、
//! 钳边地画进矩形，整体乘 `opacity`。矩形边缘不做抗锯齿，用于 Gallery 的参照图。
//! 没登记或已注销的 id 被跳过。
//!
//! # 批
//!
//! scene 是有序列表。一帧里线性扫描，把**连续的同类图元**并成一批（一次实例化 draw），
//! 不同种类交错时按原顺序排批，所以叠放顺序跨种类成立：多个相邻的 `GlyphRun` 并成一次 draw
//! （颜色在实例里），相邻的 `Image` 只有同一张图才并。每种图元一个实例缓冲，各自按需增长。
//!
//! # 帧节奏
//!
//! - 只在调用 [`Renderer::render`] 时工作。没有内部循环、定时器或线程；空闲时 0 CPU。
//! - 呈现模式 `Fifo`（垂直同步，不撕裂，Metal 上是 `displaySyncEnabled = true`）。
//! - `desired_maximum_frame_latency = 1`（Metal 上 `maximumDrawableCount = 2`）：
//!   每帧 GPU 工作很少，瓶颈是输入到上屏的延迟而不是吞吐，所以取最短队列。
//!   实测 120Hz 吃紧时可以改成 2。
//! - 透明度模式 `Opaque`：窗口背景由我们自己画，layer 标记为不透明，合成器可以跳过混合。
//!
//! # 帧计时
//!
//! [`Renderer::set_frame_timing`] 打开后，每帧量 `nextDrawable` 的等待、编码、提交三段 CPU 时间，
//! 设备支持 `TIMESTAMP_QUERY` 时（本机 Apple M4 支持）再量 render pass 的 GPU 时间，
//! 用 [`Renderer::drain_frame_timings`] 取走 [`FrameTiming`]。默认关闭，关闭时每帧只多几次
//! `Option` 判断。[`stats`] 模块是宿主汇总用的纯函数：分位数和基于显示链接时间戳的掉帧检测。
//! wgpu 30 不暴露 `CAMetalDrawable`，拿不到上屏时刻（`presentedTime`），细节见 [`stats`] 的文档。
//!
//! # 实例缓冲
//!
//! 每种图元（quad、字形、图像）各一个 `VERTEX | COPY_DST` 的实例缓冲；下面以 quad 为例，初始容量 256 个实例，按需增长：不够时分配
//! `max(需要的数量, 旧容量 * 2)`，够用就原样复用，**不缩小**。每帧只用 `queue.write_buffer`
//! 往里写，CPU 侧的打包暂存也是复用的，稳定状态下每帧没有堆分配和 GPU 缓冲分配。
//! [`Renderer::instance_buffer_allocations`] 可以观察到这一点。
//!
//! # 平台层要做的事（macOS）
//!
//! wgpu 30 的 Metal 后端（wgpu-hal 的 `metal/surface.rs`，`Surface::configure`）在
//! 每次配置时会改 layer 的这些属性：`opaque`（由 alpha_mode 决定）、`device`、
//! `pixelFormat`、`framebufferOnly`、`colorspace`（对 `Srgb` 重置为 nil）、
//! `maximumDrawableCount`、`drawableSize`、`allowsNextDrawableTimeout = false`、
//! `displaySyncEnabled`。它**不会**改 `bounds`、`contentsScale`、`frame`、`presentsWithTransaction`。
//! 因此：
//!
//! - **`drawableSize` 归渲染器管**：平台层不要自己设，窗口或 view 尺寸、scale 变化时调
//!   [`Renderer::resize`] 传入新的像素尺寸（`bounds * contentsScale`，取整后）。
//!   drawable 和 `bounds * contentsScale` 不一致时 Core Animation 会缩放内容，画面会糊。
//! - **`contentsScale` 归平台层管**：要和传给渲染器的 `scale` 一致（通常是窗口的
//!   `backingScaleFactor`），换屏时两边一起更新。
//! - **`colorspace`**：wgpu 在每次配置时把它重置为 nil，所以由本 crate 在每次配置之后
//!   重新设成 sRGB；平台层不要自己设，设了也会被覆盖。
//! - **`presentsWithTransaction`**：wgpu 在每次取 drawable 时读取 layer 当前的值：为 true 时，
//!   `present` 改用 `commit` + `waitUntilScheduled` + `drawable.present()`，这是 Apple 文档
//!   要求的、让 Core Animation 事务和 Metal 内容同步上屏的做法，实时缩放窗口时不会出现
//!   内容与窗口边框错位。所以平台层可以在实时缩放期间（`viewWillStartLiveResize` 到
//!   `viewDidEndLiveResize`）把它设为 true，并在主线程、同步地调用 `resize` 和 `render`，
//!   结束后改回 false。
//! - **遮挡**：窗口被遮挡（最小化、被完全盖住）时，wgpu 取 drawable 返回 `Occluded`，
//!   `render` 跳过这一帧并返回 `Ok`。窗口重新可见时平台层要主动触发一次重绘
//!   （`NSWindowDidChangeOcclusionState`），否则会一直停留在旧画面。
//! - 所有调用都在主线程。

mod common;
mod error;
mod frame;
mod glyph_pass;
mod image_pass;
mod instance;
mod quad_pass;
mod renderer;
pub mod stats;
mod timing;

pub use error::RenderError;
pub use renderer::Renderer;
pub use stats::{DROP_THRESHOLD, Distribution, DropReport, FrameSample, FrameStats};
pub use timing::FrameTiming;
