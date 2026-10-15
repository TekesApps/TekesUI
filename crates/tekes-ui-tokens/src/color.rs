//! Encoded, non-premultiplied sRGB tokens generated from design/tokens.json.

pub use crate::generated::ColorRole;
#[cfg(test)]
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

#[cfg(test)]
const BLACK: Rgba = Rgba::gray(0.0);

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
    fn existing_roles_consume_adopted_proposal_colors() {
        for (role, id) in [
            (ColorRole::WarningForeground, "warning-foreground"),
            (ColorRole::RowSelection, "row-selection"),
            (ColorRole::ButtonHover, "button-hover"),
            (ColorRole::Separator, "separator"),
            (ColorRole::DiffAddedSurface, "diff-added-surface"),
        ] {
            for theme in Theme::ALL {
                assert_eq!(
                    role.resolve(theme),
                    crate::proposal::color(id, theme).unwrap()
                );
            }
        }
        assert_eq!(
            ColorRole::Scrim.resolve(Theme::DARK),
            BLACK.with_alpha(crate::Alpha::Wash.value())
        );
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
