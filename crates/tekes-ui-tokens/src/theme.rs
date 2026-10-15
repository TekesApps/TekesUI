//! **临时占位：数值取自 Tekes 现有 Spacing / CornerRadius / Stroke / Motion / ControlSize / Alpha / ColorRole / TextRole，等 Design 交付的 tokens.json 生成后替换。**
//!
//! 主题：外观加对比度。

/// 外观。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Appearance {
    Light,
    Dark,
}

/// 对比度。`Increased` 对应系统的 Increase Contrast。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Contrast {
    Standard,
    Increased,
}

/// 一套主题。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Theme {
    pub appearance: Appearance,
    pub contrast: Contrast,
}

impl Theme {
    pub const LIGHT: Theme = Theme {
        appearance: Appearance::Light,
        contrast: Contrast::Standard,
    };
    pub const DARK: Theme = Theme {
        appearance: Appearance::Dark,
        contrast: Contrast::Standard,
    };
    pub const LIGHT_INCREASED: Theme = Theme {
        appearance: Appearance::Light,
        contrast: Contrast::Increased,
    };
    pub const DARK_INCREASED: Theme = Theme {
        appearance: Appearance::Dark,
        contrast: Contrast::Increased,
    };

    /// 四个预设，用于遍历。
    pub const ALL: [Theme; 4] = [
        Theme::LIGHT,
        Theme::DARK,
        Theme::LIGHT_INCREASED,
        Theme::DARK_INCREASED,
    ];

    pub const fn is_dark(self) -> bool {
        matches!(self.appearance, Appearance::Dark)
    }

    pub const fn is_increased(self) -> bool {
        matches!(self.contrast, Contrast::Increased)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::LIGHT
    }
}
