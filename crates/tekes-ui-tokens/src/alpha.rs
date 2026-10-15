//! **临时占位：数值取自 Tekes 现有 Spacing / CornerRadius / Stroke / Motion / ControlSize / Alpha / ColorRole / TextRole，等 Design 交付的 tokens.json 生成后替换。**
//!
//! 透明度档位。显式 alpha 表示最终 alpha，不相乘。

use crate::theme::Contrast;

/// 透明度档位。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Alpha {
    Faint,
    Soft,
    Wash,
}

impl Alpha {
    /// 标准对比度下的值。
    pub const fn value(self) -> f32 {
        match self {
            Alpha::Faint => 0.07,
            Alpha::Soft => 0.14,
            Alpha::Wash => 0.45,
        }
    }

    /// 按对比度取值。Increase Contrast 下 `Faint` 提升为 `Soft` 的值。
    pub const fn resolve(self, contrast: Contrast) -> f32 {
        match (self, contrast) {
            (Alpha::Faint, Contrast::Increased) => Alpha::Soft.value(),
            _ => self.value(),
        }
    }
}

/// 高对比下文本选区使用的 alpha。
pub const TEXT_SELECTION_INCREASED: f32 = 0.75;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_values() {
        assert_eq!(Alpha::Faint.value(), 0.07);
        assert_eq!(Alpha::Soft.value(), 0.14);
        assert_eq!(Alpha::Wash.value(), 0.45);
    }

    #[test]
    fn increased_contrast_raises_faint_to_soft() {
        assert_eq!(Alpha::Faint.resolve(Contrast::Standard), 0.07);
        assert_eq!(Alpha::Faint.resolve(Contrast::Increased), 0.14);
        assert_eq!(Alpha::Soft.resolve(Contrast::Increased), 0.14);
        assert_eq!(Alpha::Wash.resolve(Contrast::Increased), 0.45);
    }
}
