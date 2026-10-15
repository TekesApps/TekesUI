//! 文字样式：排版和光栅化需要知道的字体选择。颜色不是排版输入，不在这里。

/// 字体家族。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    /// 系统界面字体（macOS 上是 `CTFontCreateUIFontForLanguage(kCTFontUIFontSystem, size)`）。
    SystemUi,
    /// 按名字指定的字体。CoreText 接受 PostScript 名或全名，找不到时由系统选一个近似的。
    Named(String),
}

/// 文字样式。
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub family: FontFamily,
    /// 字号，单位 pt。
    pub size: f32,
    /// 字重，100..=900，400 是常规。
    pub weight: u16,
}

impl TextStyle {
    /// 参照实现里多行排版的默认字体字号。
    pub const DEFAULT_SIZE: f32 = 14.0;

    pub fn system(size: f32) -> TextStyle {
        TextStyle {
            family: FontFamily::SystemUi,
            size,
            weight: 400,
        }
    }

    pub fn named(name: impl Into<String>, size: f32) -> TextStyle {
        TextStyle {
            family: FontFamily::Named(name.into()),
            size,
            weight: 400,
        }
    }

    pub fn with_weight(mut self, weight: u16) -> TextStyle {
        self.weight = weight;
        self
    }
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle::system(TextStyle::DEFAULT_SIZE)
    }
}
