//! 与参照实现对比的运行器。
//!
//! 流程：读夹具清单，对每个 夹具 × 宽度 × 倍率 排版并导出 `tekesui-layout/0`，与参照侧的导出
//! 按 comparison-method 第 2.2 节逐项比较（[`layout`]）；再经 atlas 加 wgpu 离屏渲染，与参照侧的
//! 位图按第 2.1 节做带容差的像素差异（`tekes_ui_text::compare`）。输出报告 JSON、Markdown 摘要和热图。
//!
//! 参照数据是私有资产，只从命令行给的目录读取，输出目录由调用方选（应当是临时目录或私有目录）；
//! 本 crate 不内置任何参照数据。

pub mod layout;
pub mod manifest;
pub mod report;
#[cfg(target_os = "macos")]
pub mod runner;
