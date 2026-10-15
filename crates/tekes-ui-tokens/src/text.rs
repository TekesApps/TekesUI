//! **临时占位：数值取自 Tekes 现有 Spacing / CornerRadius / Stroke / Motion / ControlSize / Alpha / ColorRole / TextRole，等 Design 交付的 tokens.json 生成后替换。**
//!
//! 文字角色。这里只有数据，文字渲染以后再做。

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

/// 文字角色。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextRole {
    Body,
    BodyEmphasized,
    RowMeta,
    SectionHeader,
    IdentityBadge,
    Brand,
}

impl TextRole {
    pub const ALL: [TextRole; 6] = [
        TextRole::Body,
        TextRole::BodyEmphasized,
        TextRole::RowMeta,
        TextRole::SectionHeader,
        TextRole::IdentityBadge,
        TextRole::Brand,
    ];

    pub const fn style(self) -> TextStyle {
        match self {
            TextRole::Body => TextStyle {
                size: 13.0,
                line_height: 16.0,
                weight: 400,
            },
            TextRole::BodyEmphasized => TextStyle {
                size: 13.0,
                line_height: 16.0,
                weight: 600,
            },
            TextRole::RowMeta => TextStyle {
                size: 11.0,
                line_height: 16.0,
                weight: 400,
            },
            TextRole::SectionHeader => TextStyle {
                size: 11.0,
                line_height: 16.0,
                weight: 700,
            },
            TextRole::IdentityBadge => TextStyle {
                size: 10.0,
                line_height: 16.0,
                weight: 600,
            },
            // 占位：现状是随系统的 title2，字号待核实。
            TextRole::Brand => TextStyle {
                size: 17.0,
                line_height: 22.0,
                weight: 700,
            },
        }
    }
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
