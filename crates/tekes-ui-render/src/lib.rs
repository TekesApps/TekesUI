//! 渲染层，基于 wgpu。
//!
//! 接口保持窄，留出换成 objc2-metal 的余地。
//! 例如 mac 上需要更直接地控制 CAMetalLayer 和帧节奏时。
