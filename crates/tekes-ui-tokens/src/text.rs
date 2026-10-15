//! Text metrics generated from design/tokens.json; system fonts are a platform strategy.

pub use crate::generated::TextRole;

/// 文字样式。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// 字号（pt）。
    pub size: f32,
    /// 行高（pt）。
    pub line_height: f32,
    /// 字重，100..=900。
    pub weight: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_is_13_16_regular() {
        let s = TextRole::Body.style();
        assert_eq!((s.size, s.line_height, s.weight), (13.0, 16.0, 400));
    }

    #[test]
    fn all_roles_have_positive_metrics() {
        for r in TextRole::ALL {
            let s = r.style();
            assert!(s.size > 0.0 && s.line_height >= s.size);
        }
    }
}
