//! Validated, bounded foreground attributes for one complete readonly label.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_SEMANTIC_LABEL_UTF16: usize = 16_384;
pub const MAX_SEMANTIC_LABEL_RUNS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LabelSourceVersion {
    pub source: u64,
    pub revision: u64,
}

/// Generic foreground intent, resolved through the current SDK Palette at paint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelForeground {
    #[default]
    Inherited,
    Secondary,
    Positive,
    Negative,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelForegroundRun {
    pub utf16: Range<usize>,
    pub foreground: LabelForeground,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticLabelError {
    InvalidIdentity,
    Capacity,
    InvalidRange(usize),
    UnsupportedComponent,
    SourceMismatch,
    StaleUpdate,
}
impl std::fmt::Display for SemanticLabelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "semantic label contract violation: {self:?}")
    }
}
impl std::error::Error for SemanticLabelError {}

/// One immutable source/version and sorted, disjoint UTF-16 attributes. Gaps
/// inherit the original component foreground. Construction validates once;
/// callers cannot mutate ranges or source behind the retained layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticLabel {
    version: LabelSourceVersion,
    text: String,
    runs: Vec<LabelForegroundRun>,
}
impl SemanticLabel {
    pub fn new(
        version: LabelSourceVersion,
        text: impl Into<String>,
        runs: Vec<LabelForegroundRun>,
    ) -> Result<Self, SemanticLabelError> {
        if version.source == 0 || version.revision == 0 {
            return Err(SemanticLabelError::InvalidIdentity);
        }
        let text = text.into();
        if runs.len() > MAX_SEMANTIC_LABEL_RUNS
            || text
                .encode_utf16()
                .take(MAX_SEMANTIC_LABEL_UTF16 + 1)
                .count()
                > MAX_SEMANTIC_LABEL_UTF16
        {
            return Err(SemanticLabelError::Capacity);
        }
        let mut boundaries = vec![0];
        let mut length = 0;
        for grapheme in text.graphemes(true) {
            length += grapheme.encode_utf16().count();
            boundaries.push(length);
        }
        let mut previous_end = 0;
        for (index, run) in runs.iter().enumerate() {
            if run.utf16.start >= run.utf16.end
                || run.utf16.end > length
                || run.utf16.start < previous_end
                || boundaries.binary_search(&run.utf16.start).is_err()
                || boundaries.binary_search(&run.utf16.end).is_err()
            {
                return Err(SemanticLabelError::InvalidRange(index));
            }
            previous_end = run.utf16.end;
        }
        Ok(Self {
            version,
            text,
            runs,
        })
    }
    pub fn version(&self) -> LabelSourceVersion {
        self.version
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn runs(&self) -> &[LabelForegroundRun] {
        &self.runs
    }
    pub fn foreground_at(&self, utf16: usize) -> LabelForeground {
        let index = self.runs.partition_point(|run| run.utf16.end <= utf16);
        self.runs
            .get(index)
            .filter(|run| run.utf16.start <= utf16)
            .map_or(LabelForeground::Inherited, |run| run.foreground)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "工具👩🏽‍💻 · 中文 e\u{301}    +12 −3";
    #[test]
    fn validates_original_utf16_roles_without_splitting_unicode_or_graphemes() {
        let version = LabelSourceVersion {
            source: 25,
            revision: 1,
        };
        let make = |range| {
            SemanticLabel::new(
                version,
                SOURCE,
                vec![LabelForegroundRun {
                    utf16: range,
                    foreground: LabelForeground::Positive,
                }],
            )
        };
        for range in [9..17, 21..24, 25..27, 2..9] {
            assert!(make(range).is_ok());
        }
        for range in [
            3..9,
            2..4,
            15..16,
            16..17,
            27..28,
            21..21,
            Range { start: 9, end: 2 },
        ] {
            assert_eq!(
                make(range).unwrap_err(),
                SemanticLabelError::InvalidRange(0)
            );
        }
        let valid = make(21..24).unwrap();
        assert_eq!(valid.foreground_at(20), LabelForeground::Inherited);
        assert_eq!(valid.foreground_at(21), LabelForeground::Positive);
        assert_eq!(valid.foreground_at(24), LabelForeground::Inherited);
    }
    #[test]
    fn bounds_identity_source_and_sorted_disjoint_ranges() {
        let version = LabelSourceVersion {
            source: 25,
            revision: 1,
        };
        assert_eq!(
            SemanticLabel::new(
                LabelSourceVersion {
                    source: 0,
                    ..version
                },
                "",
                vec![]
            )
            .unwrap_err(),
            SemanticLabelError::InvalidIdentity
        );
        assert_eq!(
            SemanticLabel::new(version, "x".repeat(MAX_SEMANTIC_LABEL_UTF16 + 1), vec![])
                .unwrap_err(),
            SemanticLabelError::Capacity
        );
        let run = LabelForegroundRun {
            utf16: 0..1,
            foreground: LabelForeground::Secondary,
        };
        assert_eq!(
            SemanticLabel::new(version, "xy", vec![run.clone(), run]).unwrap_err(),
            SemanticLabelError::InvalidRange(1)
        );
        assert!(SemanticLabel::new(version, "", vec![]).is_ok());
    }
}
