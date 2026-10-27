//! macOS 平台层，基于 objc2 的 AppKit 绑定，不使用 winit。
//!
//! 负责窗口、事件泵、菜单和系统外观，向引擎投递规整的 [`Event`]；
//! NSTextInputClient 和剪贴板桥接只传递数据；文本及选区由 Rust 管理。
//! 与引擎之间只通过窄接口通信：[`App`]、[`WindowHandler`]、[`Window`]、[`Event`]，
//! 引擎不接触任何 objc2 类型。见章程第 4、8 节与 docs/platform-boundary.md。
//! 引擎不依赖这个 crate；这个 crate 也不依赖渲染层。
//!
//! 纯数据部分（事件、菜单描述、配置）不含平台代码，在任何平台上都能编译和单测。

pub mod accessibility;
pub mod config;
#[cfg(feature = "content-accessibility")]
pub mod content_accessibility;
pub mod event;
pub mod menu;
pub mod placement;
mod text_key;
mod wake;
pub use text_key::{TextKeyContext, TextKeyDisposition, TextKeyEvent};
pub use wake::{WakeResult, WindowWake};

pub use accessibility::{
    AccessibilityAction, AccessibilityKey, AccessibilityNode, AccessibilityRange,
    AccessibilityRole, AccessibilitySortDirection, AccessibilityTextLayout,
};
pub use config::{AppConfig, WindowConfig};
pub use event::{
    AnimationFrame, CursorIcon, CursorRegion, Event, KeyEvent, KeyEventKind, Modifiers,
    MouseButton, MouseEvent, MouseEventKind, Point, ScrollEvent, ScrollPhase, Size, TextInputState,
    Vec2,
};
pub use menu::{CommandId, KeyEquivalent, Menu, MenuBar, MenuCommand, MenuError, MenuItem};
pub use placement::{Layout, PlacementReport, Rect, ScreenChoice, ScreenInfo, WindowPlacement};

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod appearance;
#[cfg(target_os = "macos")]
mod menu_build;
#[cfg(target_os = "macos")]
mod screens;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub mod selftest;
#[cfg(target_os = "macos")]
mod shared;
#[cfg(target_os = "macos")]
mod view;

#[cfg(target_os = "macos")]
pub use app::{App, WindowHandle};
#[cfg(target_os = "macos")]
pub use shared::{Window, WindowHandler};

#[cfg(target_os = "macos")]
mod presentation;
#[cfg(target_os = "macos")]
pub use presentation::PresentedFrame;
