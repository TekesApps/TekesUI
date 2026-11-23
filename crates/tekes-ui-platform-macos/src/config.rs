//! 应用与窗口的启动配置。纯数据。

use crate::event::Size;
use crate::placement::WindowPlacement;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppConfig {
    /// 应用名，用在标准应用菜单的“关于 / 隐藏 / 退出”标题里。
    /// `None` 时取进程名。
    pub name: Option<String>,
    /// 最后一个窗口关闭后是否结束 `run()`。默认 false（符合原生 macOS 应用习惯，关窗后驻留 Dock 与菜单栏）。
    pub quit_after_last_window_closed: bool,
}

impl AppConfig {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn quit_after_last_window_closed(mut self, quit: bool) -> Self {
        self.quit_after_last_window_closed = quit;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    /// Extend the TekesUI view into a transparent, unified native titlebar.
    /// Hosts must reserve space for the native window buttons. Default false.
    pub unified_titlebar: bool,
    /// 内容区（视图）的初始尺寸，单位 pt。
    pub size: Size,
    pub min_size: Option<Size>,
    /// 初始位置。已经从 autosave 恢复出位置时不再摆放。
    /// 默认 `Centered`（AppKit 居中）；开发时的示例和 Gallery 用 `Auto`，
    /// 并可以用环境变量 `TEKESUI_WINDOW_PLACEMENT` 覆盖，见 [`crate::placement`]。
    pub placement: WindowPlacement,
    /// 窗口位置与尺寸的自动保存名。默认 `None`（关闭）：
    /// Gallery 需要固定、可复现的窗口尺寸，不能被上次的拖动改变。
    pub frame_autosave_name: Option<String>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: String::from("TekesUI"),
            unified_titlebar: false,
            size: Size {
                width: 960.0,
                height: 640.0,
            },
            min_size: None,
            placement: WindowPlacement::Centered,
            frame_autosave_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_native_macos_lifecycle() {
        let config = AppConfig::default();
        assert_eq!(config.name, None);
        assert!(!config.quit_after_last_window_closed);
    }

    #[test]
    fn builders_override_name_and_quit_policy() {
        let config = AppConfig::default()
            .name("Tekes Studio")
            .quit_after_last_window_closed(true);
        assert_eq!(config.name.as_deref(), Some("Tekes Studio"));
        assert!(config.quit_after_last_window_closed);
    }
}
