//! macOS 平台层，基于 objc2 的 AppKit 绑定。
//!
//! 负责窗口、事件泵、菜单、剪贴板和输入法（NSTextInputClient）。
//! 与引擎之间只通过窄接口通信，见章程第 8 节。
//! 引擎不依赖这个 crate。
