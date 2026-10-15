//! **临时占位：数值取自 Tekes 现有 Spacing / CornerRadius / Stroke / Motion / ControlSize / Alpha / ColorRole / TextRole，等 Design 交付的 tokens.json 生成后替换。**
//!
//! 颜色与颜色角色。标 `// 占位` 的分支是猜的，没有标的来自 Tekes 现状。

use crate::alpha::{Alpha, TEXT_SELECTION_INCREASED};
use crate::theme::Theme;

/// sRGB 颜色，**非预乘** alpha，各分量为 f32，范围 0..=1。
///
/// 这不是线性空间，也不应该在进入渲染器前转成线性：TekesUI 渲染器沿用
/// AppKit / CoreAnimation 的做法，直接在 sRGB 编码空间（gamma 空间）里混合，
/// token 的数值原样传给着色器，不做 sRGB 转换。详见 `tekes-ui-render` 的
/// 模块文档。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Rgba {
        Rgba { r, g, b, a }
    }

    /// 由 0..=255 的整数分量构造，不透明。
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Rgba {
        Rgba {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// 灰度，不透明。
    pub const fn gray(v: f32) -> Rgba {
        Rgba::new(v, v, v, 1.0)
    }

    /// 替换 alpha。alpha 表示最终值，不与原有 alpha 相乘。
    pub const fn with_alpha(self, a: f32) -> Rgba {
        Rgba { a, ..self }
    }

    /// 转成线性空间的 RGB，alpha 不变，仍然非预乘。
    ///
    /// 只用于需要线性光的颜色运算（例如按亮度做插值或对比度计算），
    /// **不在 UI 混合路径上使用**：渲染器的混合发生在 sRGB 编码空间。
    pub fn to_linear(self) -> [f32; 4] {
        fn lin(c: f32) -> f32 {
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        [lin(self.r), lin(self.g), lin(self.b), self.a]
    }

    /// 各分量是否都在 0..=1 内。
    pub fn in_range(self) -> bool {
        [self.r, self.g, self.b, self.a]
            .iter()
            .all(|c| (0.0..=1.0).contains(c))
    }
}

const BLACK: Rgba = Rgba::gray(0.0);
const WHITE: Rgba = Rgba::gray(1.0);

/// 颜色角色，共 27 个。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // 第 1 组：完全中立
    Scrim,
    WarningText,
    WarningForeground,
    // 第 2 组：半中立
    RowSelection,
    ButtonHover,
    TranscriptInput,
    Border,
    FloatingPanelBorder,
    FloatingPanelDivider,
    BorderFocused,
    // 第 3 组：系统依赖
    PrimaryText,
    SecondaryText,
    TertiaryText,
    SelectedText,
    DocumentCanvas,
    TextSelection,
    Accent,
    StatusRunning,
    Warning,
    StatusFailed,
    StatusSucceeded,
    DiffAddedMarker,
    DiffRemovedMarker,
    HoverWash,
    Separator,
    DiffAddedSurface,
    DiffRemovedSurface,
}

impl ColorRole {
    pub const ALL: [ColorRole; 27] = [
        ColorRole::Scrim,
        ColorRole::WarningText,
        ColorRole::WarningForeground,
        ColorRole::RowSelection,
        ColorRole::ButtonHover,
        ColorRole::TranscriptInput,
        ColorRole::Border,
        ColorRole::FloatingPanelBorder,
        ColorRole::FloatingPanelDivider,
        ColorRole::BorderFocused,
        ColorRole::PrimaryText,
        ColorRole::SecondaryText,
        ColorRole::TertiaryText,
        ColorRole::SelectedText,
        ColorRole::DocumentCanvas,
        ColorRole::TextSelection,
        ColorRole::Accent,
        ColorRole::StatusRunning,
        ColorRole::Warning,
        ColorRole::StatusFailed,
        ColorRole::StatusSucceeded,
        ColorRole::DiffAddedMarker,
        ColorRole::DiffRemovedMarker,
        ColorRole::HoverWash,
        ColorRole::Separator,
        ColorRole::DiffAddedSurface,
        ColorRole::DiffRemovedSurface,
    ];

    /// 在给定主题下解析为 sRGB 颜色。
    pub fn resolve(self, theme: Theme) -> Rgba {
        let dark = theme.is_dark();
        let ic = theme.is_increased();
        // 按 (light, dark) 选值的小助手。
        let pick = |l: Rgba, d: Rgba| if dark { d } else { l };
        match self {
            ColorRole::Scrim => BLACK,
            ColorRole::WarningText => BLACK,
            ColorRole::WarningForeground => {
                pick(Rgba::rgb8(0xA0, 0x3C, 0x00), Rgba::rgb8(0xFF, 0xA8, 0x5C))
            }

            ColorRole::RowSelection => {
                // light 来自 Tekes；dark、IC 为占位。
                if ic {
                    pick(Rgba::gray(0.88), Rgba::gray(0.30)) // 占位
                } else {
                    pick(Rgba::new(0.935, 0.941, 0.941, 1.0), Rgba::gray(0.22)) // dark 占位
                }
            }
            ColorRole::ButtonHover => {
                if ic {
                    pick(Rgba::gray(0.84), Rgba::gray(0.34)) // 占位
                } else {
                    pick(Rgba::gray(0.90), Rgba::gray(0.26)) // dark 占位
                }
            }
            // 占位
            ColorRole::TranscriptInput => match (dark, ic) {
                (false, false) => Rgba::gray(0.97),
                (false, true) => Rgba::gray(1.0),
                (true, false) => Rgba::gray(0.17),
                (true, true) => Rgba::gray(0.10),
            },
            // 占位
            ColorRole::Border => match (dark, ic) {
                (false, false) => BLACK.with_alpha(0.12),
                (false, true) => BLACK.with_alpha(0.45),
                (true, false) => WHITE.with_alpha(0.15),
                (true, true) => WHITE.with_alpha(0.55),
            },
            // 占位
            ColorRole::FloatingPanelBorder => match (dark, ic) {
                (false, false) => BLACK.with_alpha(0.16),
                (false, true) => BLACK.with_alpha(0.50),
                (true, false) => WHITE.with_alpha(0.20),
                (true, true) => WHITE.with_alpha(0.60),
            },
            // 占位
            ColorRole::FloatingPanelDivider => match (dark, ic) {
                (false, false) => BLACK.with_alpha(0.08),
                (false, true) => BLACK.with_alpha(0.35),
                (true, false) => WHITE.with_alpha(0.10),
                (true, true) => WHITE.with_alpha(0.45),
            },
            // 占位：比 border 更强的中性轮廓。
            ColorRole::BorderFocused => match (dark, ic) {
                (false, false) => BLACK.with_alpha(0.35),
                (false, true) => BLACK.with_alpha(0.75),
                (true, false) => WHITE.with_alpha(0.40),
                (true, true) => WHITE.with_alpha(0.80),
            },

            // 占位：系统 label 色系的近似。
            ColorRole::PrimaryText => pick(BLACK, WHITE).with_alpha(0.85),
            ColorRole::SecondaryText => pick(BLACK, WHITE).with_alpha(if ic { 0.70 } else { 0.50 }),
            ColorRole::TertiaryText => pick(BLACK, WHITE).with_alpha(if ic { 0.50 } else { 0.26 }),
            // 占位：选中控件文本色。
            ColorRole::SelectedText => WHITE,
            // 占位：系统文本背景色。
            ColorRole::DocumentCanvas => pick(WHITE, Rgba::rgb8(0x1E, 0x1E, 0x1E)),
            // 占位：accent 的浅色版本；IC 下 alpha 为 .75（来自 Tekes）。
            ColorRole::TextSelection => {
                let base = pick(Rgba::rgb8(0xB3, 0xD7, 0xFF), Rgba::rgb8(0x3F, 0x63, 0x8B));
                if ic {
                    base.with_alpha(TEXT_SELECTION_INCREASED)
                } else {
                    base
                }
            }
            // 占位：systemBlue。
            ColorRole::Accent => pick(Rgba::rgb8(0x00, 0x7A, 0xFF), Rgba::rgb8(0x0A, 0x84, 0xFF)),
            // 占位：systemOrange。
            ColorRole::StatusRunning => {
                pick(Rgba::rgb8(0xFF, 0x95, 0x00), Rgba::rgb8(0xFF, 0x9F, 0x0A))
            }
            // 占位：systemYellow。
            ColorRole::Warning => pick(Rgba::rgb8(0xFF, 0xCC, 0x00), Rgba::rgb8(0xFF, 0xD6, 0x0A)),
            // 占位：systemRed。
            ColorRole::StatusFailed => {
                pick(Rgba::rgb8(0xFF, 0x3B, 0x30), Rgba::rgb8(0xFF, 0x45, 0x3A))
            }
            // 占位：systemGreen。
            ColorRole::StatusSucceeded => {
                pick(Rgba::rgb8(0x34, 0xC7, 0x59), Rgba::rgb8(0x30, 0xD1, 0x58))
            }
            ColorRole::DiffAddedMarker => ColorRole::StatusSucceeded.resolve(theme),
            ColorRole::DiffRemovedMarker => ColorRole::StatusFailed.resolve(theme),

            // 以下为派生色，规则来自 Tekes；faint 在 IC 下提升为 soft。
            ColorRole::HoverWash => ColorRole::PrimaryText
                .resolve(theme)
                .with_alpha(Alpha::Faint.resolve(theme.contrast)),
            ColorRole::Separator => ColorRole::SecondaryText
                .resolve(theme)
                .with_alpha(Alpha::Soft.resolve(theme.contrast)),
            ColorRole::DiffAddedSurface => ColorRole::DiffAddedMarker
                .resolve(theme)
                .with_alpha(Alpha::Faint.resolve(theme.contrast)),
            ColorRole::DiffRemovedSurface => ColorRole::DiffRemovedMarker
                .resolve(theme)
                .with_alpha(Alpha::Faint.resolve(theme.contrast)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_has_27_distinct_roles() {
        assert_eq!(ColorRole::ALL.len(), 27);
        for (i, a) in ColorRole::ALL.iter().enumerate() {
            for b in &ColorRole::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn every_value_in_range_for_all_themes() {
        for theme in Theme::ALL {
            for role in ColorRole::ALL {
                let c = role.resolve(theme);
                assert!(c.in_range(), "{role:?} {theme:?} => {c:?}");
            }
        }
    }

    #[test]
    fn fixed_values_from_brief() {
        let c = ColorRole::WarningForeground.resolve(Theme::LIGHT);
        assert_eq!(c, Rgba::rgb8(0xA0, 0x3C, 0x00));
        let c = ColorRole::WarningForeground.resolve(Theme::DARK);
        assert_eq!(c, Rgba::rgb8(0xFF, 0xA8, 0x5C));
        assert_eq!(
            ColorRole::RowSelection.resolve(Theme::LIGHT),
            Rgba::new(0.935, 0.941, 0.941, 1.0)
        );
        assert_eq!(
            ColorRole::ButtonHover.resolve(Theme::LIGHT),
            Rgba::gray(0.90)
        );
        assert_eq!(ColorRole::Scrim.resolve(Theme::DARK), BLACK);
    }

    #[test]
    fn derived_hover_wash_follows_ic_rule() {
        assert_eq!(ColorRole::HoverWash.resolve(Theme::LIGHT).a, 0.07);
        assert_eq!(ColorRole::HoverWash.resolve(Theme::LIGHT_INCREASED).a, 0.14);
    }

    #[test]
    fn linear_conversion() {
        let l = Rgba::new(1.0, 0.0, 0.5, 0.3).to_linear();
        assert!((l[0] - 1.0).abs() < 1e-6);
        assert_eq!(l[1], 0.0);
        assert!((l[2] - 0.2140).abs() < 1e-3);
        assert_eq!(l[3], 0.3);
    }
}
