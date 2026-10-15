//! 主题选择：外观与对比度各有“跟随系统 / 固定”的选项，合成出当前生效的 [`Theme`]。

use tekes_ui_tokens::{Appearance, Contrast, Theme};

/// 外观选项。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppearanceChoice {
    System,
    Light,
    Dark,
}

/// Increase Contrast 选项：跟随系统 / 强制开 / 强制关。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContrastChoice {
    System,
    On,
    Off,
}

impl AppearanceChoice {
    pub const ALL: [AppearanceChoice; 3] = [
        AppearanceChoice::System,
        AppearanceChoice::Light,
        AppearanceChoice::Dark,
    ];
}

impl ContrastChoice {
    pub const ALL: [ContrastChoice; 3] = [
        ContrastChoice::System,
        ContrastChoice::On,
        ContrastChoice::Off,
    ];
}

/// 用户的选择加上最近一次从系统读到的主题。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeState {
    pub appearance: AppearanceChoice,
    pub contrast: ContrastChoice,
    /// 平台层最近一次投递的系统外观。启动时在第一个 `SystemAppearanceChanged` 到达前是浅色、标准对比度。
    pub system: Theme,
}

impl Default for ThemeState {
    fn default() -> Self {
        ThemeState {
            appearance: AppearanceChoice::System,
            contrast: ContrastChoice::System,
            system: Theme::default(),
        }
    }
}

impl ThemeState {
    /// 当前生效的主题。
    pub fn effective(&self) -> Theme {
        let appearance = match self.appearance {
            AppearanceChoice::System => self.system.appearance,
            AppearanceChoice::Light => Appearance::Light,
            AppearanceChoice::Dark => Appearance::Dark,
        };
        let contrast = match self.contrast {
            ContrastChoice::System => self.system.contrast,
            ContrastChoice::On => Contrast::Increased,
            ContrastChoice::Off => Contrast::Standard,
        };
        Theme {
            appearance,
            contrast,
        }
    }
}

/// 窗口标题里的主题名。
pub fn theme_label(theme: Theme) -> &'static str {
    match (theme.is_dark(), theme.is_increased()) {
        (false, false) => "Light",
        (true, false) => "Dark",
        (false, true) => "Light (Increase Contrast)",
        (true, true) => "Dark (Increase Contrast)",
    }
}

/// 快照文件名里的主题名。
pub fn theme_slug(theme: Theme) -> &'static str {
    match (theme.is_dark(), theme.is_increased()) {
        (false, false) => "light",
        (true, false) => "dark",
        (false, true) => "light-ic",
        (true, true) => "dark-ic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_system_by_default() {
        let mut s = ThemeState::default();
        assert_eq!(s.effective(), Theme::LIGHT);
        s.system = Theme::DARK_INCREASED;
        assert_eq!(s.effective(), Theme::DARK_INCREASED);
    }

    #[test]
    fn fixed_choices_override_system() {
        let mut s = ThemeState {
            system: Theme::DARK_INCREASED,
            ..ThemeState::default()
        };
        s.appearance = AppearanceChoice::Light;
        assert_eq!(s.effective(), Theme::LIGHT_INCREASED);
        s.contrast = ContrastChoice::Off;
        assert_eq!(s.effective(), Theme::LIGHT);
        s.appearance = AppearanceChoice::Dark;
        s.contrast = ContrastChoice::On;
        assert_eq!(s.effective(), Theme::DARK_INCREASED);
    }

    #[test]
    fn names_are_distinct() {
        let labels: Vec<_> = Theme::ALL.iter().map(|t| theme_label(*t)).collect();
        let slugs: Vec<_> = Theme::ALL.iter().map(|t| theme_slug(*t)).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert_ne!(labels[i], labels[j]);
                assert_ne!(slugs[i], slugs[j]);
            }
        }
    }
}
