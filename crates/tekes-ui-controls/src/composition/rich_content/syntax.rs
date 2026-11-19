//! Syntax scopes become shared semantic ink; parsing never changes the document.
use super::RichDocument;
use std::ops::Range;

#[derive(Debug)]
pub(super) struct CodeSpan {
    pub display: Range<usize>,
    pub token: &'static str,
}

#[cfg(feature = "syntax-highlighting")]
pub(super) fn parse(document: &RichDocument) -> Result<Vec<CodeSpan>, String> {
    use super::RichBlockKind;
    use std::sync::OnceLock;
    use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    let mut spans = Vec::new();
    let units: Vec<_> = document.text().encode_utf16().collect();
    for block in document
        .blocks()
        .iter()
        .filter(|block| block.kind() == RichBlockKind::Code)
    {
        let Some(language) = block.language().filter(|language| !language.is_empty()) else {
            continue;
        };
        let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
        let Some(syntax) = syntaxes.find_syntax_by_token(language) else {
            continue; // Unknown language is explicitly plain text.
        };
        let text = String::from_utf16(&units[block.display()]).expect("validated block boundaries");
        let mut parser = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut offset = block.display().start;
        for line in text.split_inclusive('\n') {
            let operations = parser
                .parse_line(line, syntaxes)
                .map_err(|error| format!("{language} code at UTF-16 {offset}: {error}"))?;
            let mut previous = 0;
            for (position, operation) in operations {
                append(&mut spans, &stack, &line[previous..position], &mut offset);
                stack
                    .apply(&operation)
                    .map_err(|error| format!("{language} scope at UTF-16 {offset}: {error}"))?;
                previous = position;
            }
            append(&mut spans, &stack, &line[previous..], &mut offset);
        }
    }
    Ok(spans)
}

#[cfg(not(feature = "syntax-highlighting"))]
pub(super) fn parse(document: &RichDocument) -> Result<Vec<CodeSpan>, String> {
    let _ = document;
    Ok(Vec::new())
}

#[cfg(feature = "syntax-highlighting")]
fn append(
    spans: &mut Vec<CodeSpan>,
    stack: &syntect::parsing::ScopeStack,
    text: &str,
    offset: &mut usize,
) {
    let start = *offset;
    *offset += text.encode_utf16().count();
    if text.is_empty() {
        return;
    }
    // Outer comments/strings own embedded punctuation and escape sequences.
    let scopes: Vec<_> = stack
        .scopes
        .iter()
        .map(|scope| scope.build_string())
        .collect();
    let has = |prefix: &str| {
        scopes
            .iter()
            .any(|scope| scope == prefix || scope.starts_with(&format!("{prefix}.")))
    };
    let token = if has("comment") {
        "c-code-comment"
    } else if has("string") {
        "c-code-string"
    } else if has("constant.numeric") {
        "c-code-number"
    } else if has("keyword") || has("storage.type") || has("storage.modifier") {
        "c-code-keyword"
    } else {
        return;
    };
    if let Some(last) = spans
        .last_mut()
        .filter(|last| last.token == token && last.display.end == start)
    {
        last.display.end = *offset;
    } else {
        spans.push(CodeSpan {
            display: start..*offset,
            token,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "syntax-highlighting")]
    #[test]
    fn rust_scopes_preserve_unicode_offsets_and_multiline_comment_state() {
        let source =
            "```rust\nlet title = \"你好🦀\"; /* start\n42 still comment */ let n = 12;\n```";
        let document = RichDocument::from_markdown(source);
        let units: Vec<_> = document.text().encode_utf16().collect();
        let spans = parse(&document).unwrap();
        let texts = |token| {
            spans
                .iter()
                .filter(|span| span.token == token)
                .map(|span| String::from_utf16(&units[span.display.clone()]).unwrap())
                .collect::<Vec<_>>()
                .join("")
        };
        assert!(texts("c-code-keyword").contains("let"));
        assert!(texts("c-code-string").contains("你好🦀"));
        assert!(texts("c-code-number").contains("12"));
        assert!(!texts("c-code-number").contains("42"));
        assert!(texts("c-code-comment").contains("42 still comment"));
        assert_eq!(document.source(), source);
        for span in spans {
            assert!(document.display_to_source(span.display).is_some());
        }
    }

    #[cfg(feature = "syntax-highlighting")]
    #[test]
    fn unknown_or_omitted_languages_are_plain_and_each_fence_resets_parser_state() {
        for language in ["", "not-a-real-language"] {
            let document = RichDocument::from_markdown(&format!("```{language}\nlet n = 12;\n```"));
            assert!(parse(&document).unwrap().is_empty());
        }
        let document =
            RichDocument::from_markdown("```rust\n/* unfinished\n```\n\n```rust\nlet n = 12;\n```");
        let second = document.blocks()[1].display();
        assert!(
            parse(&document)
                .unwrap()
                .iter()
                .any(|span| span.token == "c-code-number" && second.contains(&span.display.start))
        );
    }

    #[cfg(not(feature = "syntax-highlighting"))]
    #[test]
    fn plain_text_fallback_emits_no_syntax_spans_when_feature_disabled() {
        let source = "```rust\nlet n = 12;\n```";
        let document = RichDocument::from_markdown(source);
        assert!(parse(&document).unwrap().is_empty());
        assert_eq!(document.source(), source);
    }
}
