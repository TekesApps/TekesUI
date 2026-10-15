//! 渲染层的错误类型。不暴露 wgpu 的类型，换后端时调用方不受影响。

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// 没有可用的 GPU 适配器。
    NoAdapter,
    /// 创建设备失败。
    Device(String),
    /// 创建或配置 surface 失败，或 surface 丢失后无法恢复。
    Surface(String),
    /// 离屏读回失败。
    Readback(String),
    /// 参数不合法（例如尺寸为 0 或 scale 不是正数）。
    InvalidArgument(&'static str),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NoAdapter => write!(f, "没有可用的 GPU 适配器"),
            RenderError::Device(e) => write!(f, "创建 GPU 设备失败：{e}"),
            RenderError::Surface(e) => write!(f, "surface 错误：{e}"),
            RenderError::Readback(e) => write!(f, "离屏读回失败：{e}"),
            RenderError::InvalidArgument(e) => write!(f, "参数不合法：{e}"),
        }
    }
}

impl std::error::Error for RenderError {}
