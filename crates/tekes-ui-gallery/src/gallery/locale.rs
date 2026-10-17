//! Gallery copy lives in JSON. User documents never pass through this translator.
use std::{borrow::Cow, collections::HashMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Locale {
    #[default]
    En,
    ZhCn,
    /// Exact original copy for immutable historical image comparisons only.
    Reference,
}
struct Dictionary {
    en: HashMap<String, String>,
    zh: HashMap<String, String>,
    fragments: Vec<(String, String)>,
}
fn dictionary() -> &'static Dictionary {
    static VALUE: OnceLock<Dictionary> = OnceLock::new();
    VALUE.get_or_init(|| {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../../../design/gallery/localization.json"))
                .expect("Gallery localization JSON");
        assert_eq!(source["default_locale"], "en");
        let en: HashMap<String, String> =
            serde_json::from_value(source["en"].clone()).expect("English copy");
        let zh: HashMap<String, String> =
            serde_json::from_value(source["zh-CN"].clone()).expect("Chinese copy");
        let mut fragments: Vec<_> = en
            .iter()
            .filter(|(key, _)| has_cjk(key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        fragments.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
        Dictionary { en, zh, fragments }
    })
}
pub fn has_cjk(value: &str) -> bool {
    value
        .chars()
        .any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
}
impl Locale {
    pub const ALL: [Self; 2] = [Self::En, Self::ZhCn];
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" | "en-US" => Some(Self::En),
            "zh" | "zh-CN" => Some(Self::ZhCn),
            _ => None,
        }
    }
    pub fn text<'a>(self, source: &'a str) -> Cow<'a, str> {
        if self == Self::Reference {
            return Cow::Borrowed(source);
        }
        let d = dictionary();
        let map = if self == Self::En { &d.en } else { &d.zh };
        if let Some(value) = map.get(source) {
            return Cow::Borrowed(value);
        }
        if self == Self::En && has_cjk(source) {
            // Formatted counts/status labels combine JSON-owned copy fragments.
            // Translation runs only on a paragraph-cache miss, never per glyph.
            let mut result = source.to_owned();
            for (key, value) in &d.fragments {
                if result.contains(key) {
                    result = result.replace(key, value);
                }
            }
            return Cow::Owned(result);
        }
        Cow::Borrowed(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_english_and_formatted_copy_is_translated() {
        assert_eq!(Locale::default(), Locale::En);
        assert_eq!(Locale::En.text("基础组件"), "Foundations");
        assert_eq!(Locale::ZhCn.text("Linear rotation"), "匀速旋转");
        assert_eq!(Locale::Reference.text("基础组件"), "基础组件");
        assert!(!has_cjk(
            &Locale::En.text("186 / 8 icons · 名称包含匹配 · 12 / 16 / 24 pt")
        ));
        assert_eq!(Locale::from_code("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("unknown"), None);
    }
}
