//! Immutable, source-mapped projection for the bounded public rich-document models.
//!
//! The immutable source is retained exactly; display line endings follow TextInput
//! normalization (`CRLF` and lone `CR` become one `LF`). Markdown syntax that is
//! not rendered (heading/fence delimiters and link targets) is omitted from `text()`. Bullets gain a display `• ` prefix. Blocks are joined
//! with a display LF; that synthetic separator maps to the preceding block's source
//! end. Paragraph source line breaks collapsed to spaces and code line breaks
//! normalized to LF map to their original source interval. Inline links support
//! only simple `[label](target)` forms; nested, escaped, inline-code, malformed, or
//! whitespace-target forms remain literal. Targets are reported as data only.
//!
//! Range APIs use UTF-16 offsets. Endpoints must be Unicode grapheme boundaries.
//! For a non-empty range, starts map toward visible text on the right and ends
//! toward visible text on the left. Thus a selection spanning omitted syntax maps
//! to the displayed content around it. A zero-length caret uses the right-side
//! policy. If omitted syntax would reverse a source range's mapped endpoints, the
//! result collapses at the right-side start. A display caret at document end maps
//! to the last visible source end (before trailing omitted syntax); a source caret
//! at source end maps to display end.
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::extended::{CodeBlock, DiffKind, DiffView, MarkdownBlock};
use tekes_ui_tokens::proposal::TextRole;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RichBlockKind {
    Heading { level: u8 },
    Paragraph,
    Bullet,
    Code,
    Diff { kind: DiffKind },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichBlock {
    pub(super) display: Range<usize>,
    pub(super) source: Range<usize>,
    pub(super) role: TextRole,
    pub(super) kind: RichBlockKind,
    pub(super) language: Option<String>,
}
impl RichBlock {
    pub fn display(&self) -> Range<usize> {
        self.display.clone()
    }
    pub fn source(&self) -> Range<usize> {
        self.source.clone()
    }
    pub fn role(&self) -> TextRole {
        self.role
    }
    pub fn kind(&self) -> RichBlockKind {
        self.kind
    }
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichLink {
    pub(super) display: Range<usize>,
    pub(super) source: Range<usize>,
    pub(super) target: String,
}
impl RichLink {
    pub fn display(&self) -> Range<usize> {
        self.display.clone()
    }
    pub fn source(&self) -> Range<usize> {
        self.source.clone()
    }
    pub fn target(&self) -> &str {
        &self.target
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MapSpan {
    source: Range<usize>,
    display: Range<usize>,
}

/// Complete original source and one flattened, selectable display projection.
/// Construction is bounded to existing Markdown/CodeBlock/DiffView models.
#[derive(Clone, Debug, PartialEq)]
pub struct RichDocument {
    pub(super) source: String,
    pub(super) text: String,
    pub(super) blocks: Vec<RichBlock>,
    pub(super) links: Vec<RichLink>,
    map: Vec<MapSpan>,
}

#[derive(Clone, Debug)]
struct Fragment {
    text: String,
    source_bytes: Range<usize>,
}
#[derive(Clone, Debug)]
struct PendingBlock {
    fragments: Vec<Fragment>,
    source_bytes: Range<usize>,
    role: TextRole,
    kind: RichBlockKind,
    language: Option<String>,
    legacy: Option<MarkdownBlock>,
}
#[derive(Clone, Debug)]
struct PendingLink {
    display_bytes: Range<usize>,
    source_bytes: Range<usize>,
    target: String,
}

impl RichDocument {
    pub fn from_markdown(source: &str) -> Self {
        let pending = parse_markdown(source);
        Self::build(source, pending)
    }

    pub(crate) fn markdown_blocks(source: &str) -> Vec<MarkdownBlock> {
        parse_markdown(source)
            .into_iter()
            .filter_map(|block| block.legacy)
            .collect()
    }

    pub fn from_code(code: &CodeBlock) -> Self {
        let source = code.source.clone();
        let fragments = normalized_fragments(&source, 0);
        let pending = vec![PendingBlock {
            fragments,
            source_bytes: 0..source.len(),
            role: TextRole::Code,
            kind: RichBlockKind::Code,
            language: Some(code.language.clone()),
            legacy: None,
        }];
        Self::build(&source, pending)
    }

    pub fn from_diff(diff: &DiffView) -> Self {
        let mut source = String::new();
        let mut pending = Vec::with_capacity(diff.lines.len());
        for (index, line) in diff.lines.iter().enumerate() {
            if index > 0 {
                source.push('\n');
            }
            let start = source.len();
            source.push_str(&line.text);
            let end = source.len();
            pending.push(PendingBlock {
                fragments: normalized_fragments(&line.text, start),
                source_bytes: start..end,
                role: TextRole::Code,
                kind: RichBlockKind::Diff { kind: line.kind },
                language: None,
                legacy: None,
            });
        }
        Self::build(&source, pending)
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn blocks(&self) -> &[RichBlock] {
        &self.blocks
    }
    pub fn links(&self) -> &[RichLink] {
        &self.links
    }

    pub fn display_to_source(&self, range: Range<usize>) -> Option<Range<usize>> {
        if range.start > range.end
            || !grapheme_boundary(&self.text, range.start)
            || !grapheme_boundary(&self.text, range.end)
        {
            return None;
        }
        if range.is_empty() {
            let at = self.display_start_to_source(range.start)?;
            return Some(at..at);
        }
        let start = self.display_start_to_source(range.start)?;
        let end = self.display_end_to_source(range.end)?;
        if start <= end {
            Some(start..end)
        } else {
            Some(start..start)
        }
    }

    pub fn source_to_display(&self, range: Range<usize>) -> Option<Range<usize>> {
        if range.start > range.end
            || !grapheme_boundary(&self.source, range.start)
            || !grapheme_boundary(&self.source, range.end)
        {
            return None;
        }
        if range.is_empty() {
            let at = self.source_start_to_display(range.start)?;
            return Some(at..at);
        }
        let start = self.source_start_to_display(range.start)?;
        let end = self.source_end_to_display(range.end)?;
        if start <= end {
            Some(start..end)
        } else {
            Some(start..start)
        }
    }

    fn display_start_to_source(&self, offset: usize) -> Option<usize> {
        if offset == self.text.encode_utf16().count() {
            return self.blocks.last().map(|b| b.source.end).or(Some(0));
        }
        self.map
            .iter()
            .find(|s| s.display.start == offset)
            .map(|s| s.source.start)
    }
    fn display_end_to_source(&self, offset: usize) -> Option<usize> {
        if offset == 0 {
            return self.map.first().map(|s| s.source.start).or(Some(0));
        }
        self.map
            .iter()
            .find(|s| s.display.end == offset)
            .map(|s| s.source.end)
    }
    fn source_start_to_display(&self, offset: usize) -> Option<usize> {
        let total = self.source.encode_utf16().count();
        if offset == total {
            return Some(self.text.encode_utf16().count());
        }
        self.map
            .iter()
            .find(|s| !s.source.is_empty() && s.source.start >= offset)
            .map(|s| s.display.start)
            .or_else(|| {
                self.map
                    .iter()
                    .find(|s| s.source.start <= offset && offset < s.source.end)
                    .map(|s| s.display.start)
            })
    }
    fn source_end_to_display(&self, offset: usize) -> Option<usize> {
        if offset == 0 {
            return Some(0);
        }
        self.map
            .iter()
            .rev()
            .find(|s| s.source.end <= offset && !s.source.is_empty())
            .map(|s| s.display.end)
            .or_else(|| {
                self.map
                    .iter()
                    .find(|s| s.source.start >= offset)
                    .map(|s| s.display.start)
            })
            .or_else(|| {
                self.map
                    .iter()
                    .find(|s| s.source.start < offset && offset <= s.source.end)
                    .map(|s| s.display.end)
            })
    }

    fn build(source: &str, pending: Vec<PendingBlock>) -> Self {
        let mut text = String::new();
        let mut blocks = Vec::with_capacity(pending.len());
        let mut links = Vec::new();
        let mut map = Vec::new();
        for block in pending {
            if !blocks.is_empty() {
                let previous = blocks.last().map_or(0, |b: &RichBlock| b.source.end);
                let next = byte_to_utf16(source, block.source_bytes.start);
                let gap = utf16_to_byte(source, previous)..block.source_bytes.start;
                let source_range = if source.get(gap.clone()) == Some("\n") {
                    previous..next
                } else {
                    previous..previous
                };
                let display_start = text.encode_utf16().count();
                text.push('\n');
                map.push(MapSpan {
                    source: source_range,
                    display: display_start..display_start + 1,
                });
            }
            let display_start = text.encode_utf16().count();
            let display_block_byte_start = text.len();
            let fragments = normalize_fragment_line_endings(source, block.fragments);
            let (fragments, mut block_links) =
                if matches!(block.kind, RichBlockKind::Code | RichBlockKind::Diff { .. }) {
                    (fragments, Vec::new())
                } else {
                    linkify(fragments)
                };
            for fragment in fragments {
                append_fragment(source, &mut text, fragment, &mut map);
            }
            let display_end = text.encode_utf16().count();
            for link in block_links.drain(..) {
                let display = display_block_byte_start + link.display_bytes.start
                    ..display_block_byte_start + link.display_bytes.end;
                let source_range = byte_to_utf16(source, link.source_bytes.start)
                    ..byte_to_utf16(source, link.source_bytes.end);
                links.push(RichLink {
                    display: byte_to_utf16(&text, display.start)..byte_to_utf16(&text, display.end),
                    source: source_range,
                    target: link.target,
                });
            }
            blocks.push(RichBlock {
                display: display_start..display_end,
                source: byte_to_utf16(source, block.source_bytes.start)
                    ..byte_to_utf16(source, block.source_bytes.end),
                role: block.role,
                kind: block.kind,
                language: block.language,
            });
        }
        Self {
            source: source.to_owned(),
            text,
            blocks,
            links,
            map,
        }
    }
}

fn normalize_fragment_line_endings(source: &str, fragments: Vec<Fragment>) -> Vec<Fragment> {
    fragments
        .into_iter()
        .flat_map(|fragment| {
            if source.get(fragment.source_bytes.clone()) == Some(fragment.text.as_str()) {
                normalized_fragments(&fragment.text, fragment.source_bytes.start)
            } else {
                vec![fragment]
            }
        })
        .collect()
}

fn append_fragment(source: &str, text: &mut String, fragment: Fragment, map: &mut Vec<MapSpan>) {
    if fragment.text.is_empty() {
        return;
    }
    let display_utf16_start = text.encode_utf16().count();
    text.push_str(&fragment.text);
    if source.get(fragment.source_bytes.clone()) == Some(fragment.text.as_str()) {
        let mut source_utf16 = byte_to_utf16(source, fragment.source_bytes.start);
        let mut display_utf16 = display_utf16_start;
        for (_, grapheme) in fragment.text.grapheme_indices(true) {
            let length = grapheme.encode_utf16().count();
            map.push(MapSpan {
                source: source_utf16..source_utf16 + length,
                display: display_utf16..display_utf16 + length,
            });
            source_utf16 += length;
            display_utf16 += length;
        }
    } else {
        let source_start = fragment.source_bytes.start;
        let source_end = fragment.source_bytes.end;
        let source_range = byte_to_utf16(source, source_start)..byte_to_utf16(source, source_end);
        let mut display_offset = display_utf16_start;
        for (index, grapheme) in fragment.text.graphemes(true).enumerate() {
            let length = grapheme.encode_utf16().count();
            let mapped_source = if index == 0 {
                source_range.clone()
            } else {
                source_range.end..source_range.end
            };
            map.push(MapSpan {
                source: mapped_source,
                display: display_offset..display_offset + length,
            });
            display_offset += length;
        }
    }
}

fn utf16_to_byte(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (byte, grapheme) in text.grapheme_indices(true) {
        if units == offset {
            return byte;
        }
        units += grapheme.encode_utf16().count();
    }
    assert_eq!(
        units, offset,
        "internal UTF-16 offset must be a grapheme boundary"
    );
    text.len()
}
fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text.get(..byte)
        .expect("internal source/display map uses a UTF-8 boundary")
        .encode_utf16()
        .count()
}
fn grapheme_boundary(text: &str, offset: usize) -> bool {
    let mut units = 0;
    for grapheme in text.graphemes(true) {
        if units == offset {
            return true;
        }
        units += grapheme.encode_utf16().count();
    }
    units == offset
}
fn normalized_fragments(source: &str, base: usize) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    let mut chunk_start = 0;
    let bytes = source.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'\r' && bytes[cursor] != b'\n' {
            cursor += source[cursor..]
                .chars()
                .next()
                .expect("valid source UTF-8")
                .len_utf8();
            continue;
        }
        if chunk_start < cursor {
            fragments.push(Fragment {
                text: source[chunk_start..cursor].to_owned(),
                source_bytes: base + chunk_start..base + cursor,
            });
        }
        let newline_start = cursor;
        cursor += 1;
        if bytes[newline_start] == b'\r' && cursor < bytes.len() && bytes[cursor] == b'\n' {
            cursor += 1;
        }
        fragments.push(Fragment {
            text: "\n".into(),
            source_bytes: base + newline_start..base + cursor,
        });
        chunk_start = cursor;
    }
    if chunk_start < source.len() {
        fragments.push(Fragment {
            text: source[chunk_start..].to_owned(),
            source_bytes: base + chunk_start..base + source.len(),
        });
    }
    fragments
}

#[derive(Clone, Debug)]
struct SourceLine {
    content: Range<usize>,
}
fn source_lines(source: &str) -> Vec<SourceLine> {
    let mut result = Vec::new();
    let mut start = 0;
    for raw in source.split_inclusive('\n') {
        let end = start + raw.len();
        let mut content_end = end;
        let terminated = content_end > start && source.as_bytes()[content_end - 1] == b'\n';
        if terminated {
            content_end -= 1;
        }
        if terminated && content_end > start && source.as_bytes()[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        result.push(SourceLine {
            content: start..content_end,
        });
        start = end;
    }
    if start < source.len() {
        result.push(SourceLine {
            content: start..source.len(),
        });
    }
    if source.is_empty() {
        return Vec::new();
    }
    result
}
type SourceLines = Vec<(String, Range<usize>)>;
fn parse_markdown(source: &str) -> Vec<PendingBlock> {
    let lines = source_lines(source);
    let mut out = Vec::new();
    let mut paragraph: SourceLines = Vec::new();
    let mut code: Option<(String, SourceLines)> = None;
    for line in lines {
        let raw = &source[line.content.clone()];
        let trimmed = raw.trim();
        if let Some((language, contents)) = &mut code {
            if trimmed.starts_with("```") {
                let mut block =
                    make_lines_block(source, contents, TextRole::Code, RichBlockKind::Code, true)
                        .unwrap_or_else(|| empty_code_block(language.clone(), line.content.start));
                block.language = Some(language.clone());
                if let Some(MarkdownBlock::Code { language: old, .. }) = &mut block.legacy {
                    *old = language.clone();
                }
                out.push(block);
                code = None;
            } else {
                contents.push((raw.to_owned(), line.content));
            }
            continue;
        }
        if let Some(fence) = trimmed.strip_prefix("```") {
            flush_paragraph(source, &mut out, &mut paragraph);
            code = Some((fence.trim().to_owned(), Vec::new()));
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph(source, &mut out, &mut paragraph);
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed.as_bytes().get(hashes) == Some(&b' ') {
            flush_paragraph(source, &mut out, &mut paragraph);
            let body = trimmed[hashes..].trim();
            let line_start = line.content.start + raw.find(trimmed).unwrap_or(0);
            let body_start = line_start + trimmed.find(body).unwrap_or(trimmed.len());
            let mut fragments = vec![Fragment {
                text: body.to_owned(),
                source_bytes: body_start..body_start + body.len(),
            }];
            out.push(PendingBlock {
                fragments: std::mem::take(&mut fragments),
                source_bytes: line_start..body_start + body.len(),
                role: TextRole::UiHeading,
                kind: RichBlockKind::Heading {
                    level: hashes as u8,
                },
                language: None,
                legacy: Some(MarkdownBlock::Heading {
                    level: hashes as u8,
                    text: body.to_owned(),
                }),
            });
        } else if let Some(text) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(source, &mut out, &mut paragraph);
            let line_start = line.content.start + raw.find(trimmed).unwrap_or(0);
            let body_start = line_start + 2;
            let source_range = body_start..body_start + text.len();
            let fragments = vec![
                Fragment {
                    text: "• ".into(),
                    source_bytes: line_start..body_start,
                },
                Fragment {
                    text: text.to_owned(),
                    source_bytes: source_range.clone(),
                },
            ];
            out.push(PendingBlock {
                fragments,
                source_bytes: line_start..source_range.end,
                role: TextRole::UiBody,
                kind: RichBlockKind::Bullet,
                language: None,
                legacy: Some(MarkdownBlock::Bullet(text.to_owned())),
            });
        } else {
            let start = line.content.start + raw.find(trimmed).unwrap_or(0);
            paragraph.push((trimmed.to_owned(), start..start + trimmed.len()));
        }
    }
    if let Some((language, contents)) = code {
        let mut block =
            make_lines_block(source, &contents, TextRole::Code, RichBlockKind::Code, true)
                .unwrap_or_else(|| empty_code_block(language.clone(), source.len()));
        block.language = Some(language.clone());
        if let Some(MarkdownBlock::Code { language: old, .. }) = &mut block.legacy {
            *old = language;
        }
        out.push(block);
    }
    flush_paragraph(source, &mut out, &mut paragraph);
    out
}
fn empty_code_block(language: String, at: usize) -> PendingBlock {
    PendingBlock {
        fragments: Vec::new(),
        source_bytes: at..at,
        role: TextRole::Code,
        kind: RichBlockKind::Code,
        language: Some(language.clone()),
        legacy: Some(MarkdownBlock::Code {
            language,
            source: String::new(),
        }),
    }
}
fn flush_paragraph(
    source: &str,
    out: &mut Vec<PendingBlock>,
    lines: &mut Vec<(String, Range<usize>)>,
) {
    if let Some(block) = make_lines_block(
        source,
        lines,
        TextRole::UiBody,
        RichBlockKind::Paragraph,
        false,
    ) {
        out.push(block);
    }
    lines.clear();
}
fn make_lines_block(
    _source: &str,
    lines: &[(String, Range<usize>)],
    role: TextRole,
    kind: RichBlockKind,
    preserve_newlines: bool,
) -> Option<PendingBlock> {
    if lines.is_empty() {
        return None;
    }
    let mut fragments = Vec::new();
    for (i, (text, range)) in lines.iter().enumerate() {
        if i > 0 {
            let previous = &lines[i - 1].1;
            fragments.push(Fragment {
                text: if preserve_newlines { "\n" } else { " " }.into(),
                source_bytes: previous.end..range.start,
            });
        }
        fragments.push(Fragment {
            text: text.clone(),
            source_bytes: range.clone(),
        });
    }
    Some(PendingBlock {
        fragments,
        source_bytes: lines.first().unwrap().1.start..lines.last().unwrap().1.end,
        role,
        kind,
        language: None,
        legacy: Some(match kind {
            RichBlockKind::Code => MarkdownBlock::Code {
                language: String::new(),
                source: lines
                    .iter()
                    .map(|x| x.0.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            },
            RichBlockKind::Paragraph => MarkdownBlock::Paragraph(
                lines
                    .iter()
                    .map(|x| x.0.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => unreachable!(),
        }),
    })
}

fn linkify(fragments: Vec<Fragment>) -> (Vec<Fragment>, Vec<PendingLink>) {
    let mut output = Vec::new();
    let mut links = Vec::new();
    let mut display_byte = 0;
    for fragment in fragments {
        let direct = fragment
            .source_bytes
            .end
            .saturating_sub(fragment.source_bytes.start)
            == fragment.text.len();
        if !direct || fragment.text.contains('`') {
            display_byte += fragment.text.len();
            output.push(fragment);
            continue;
        }
        let value = fragment.text.as_str();
        let mut scan = 0;
        let mut plain_start = 0;
        while let Some(relative) = value[scan..].find('[') {
            let open = scan + relative;
            let Some(close_rel) = value[open + 1..].find("](") else {
                break;
            };
            let close = open + 1 + close_rel;
            let target_start = close + 2;
            let Some(end_rel) = value[target_start..].find(')') else {
                break;
            };
            let end = target_start + end_rel;
            let label = &value[open + 1..close];
            let target = &value[target_start..end];
            let mut slashes = 0;
            let mut before = open;
            while before > 0 && value.as_bytes()[before - 1] == b'\\' {
                slashes += 1;
                before -= 1;
            }
            let supported = slashes % 2 == 0
                && !label.is_empty()
                && !target.is_empty()
                && !label.chars().any(|c| matches!(c, '[' | ']' | '\\'))
                && !target
                    .chars()
                    .any(|c| matches!(c, '(' | ')' | '[' | ']' | '\\'))
                && !label.contains('`')
                && !target.chars().any(char::is_whitespace);
            if !supported {
                scan = end + 1;
                continue;
            }
            if plain_start < open {
                output.push(Fragment {
                    text: value[plain_start..open].to_owned(),
                    source_bytes: fragment.source_bytes.start + plain_start
                        ..fragment.source_bytes.start + open,
                });
                display_byte += open - plain_start;
            }
            let shown_start = display_byte;
            output.push(Fragment {
                text: label.to_owned(),
                source_bytes: fragment.source_bytes.start + open + 1
                    ..fragment.source_bytes.start + close,
            });
            display_byte += label.len();
            links.push(PendingLink {
                display_bytes: shown_start..display_byte,
                source_bytes: fragment.source_bytes.start + open
                    ..fragment.source_bytes.start + end + 1,
                target: target.to_owned(),
            });
            plain_start = end + 1;
            scan = plain_start;
        }
        if plain_start < value.len() {
            output.push(Fragment {
                text: value[plain_start..].to_owned(),
                source_bytes: fragment.source_bytes.start + plain_start..fragment.source_bytes.end,
            });
            display_byte += value.len() - plain_start;
        }
    }
    (output, links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extended::{DiffKind, DiffLine, Markdown};

    #[test]
    fn markdown_preserves_the_public_parser_projection_and_maps_unicode() {
        let source =
            "# 标题👋\n\n中文 e\u{301} text\nnext\n\n- item\n\n```rust\nlet x = \"🦀\";\n```";
        let old = Markdown::parse(source);
        let rich = RichDocument::from_markdown(source);
        assert_eq!(
            rich.text(),
            "标题👋\n中文 e\u{301} text next\n• item\nlet x = \"🦀\";"
        );
        assert_eq!(old.blocks.len(), rich.blocks().len());
        assert_eq!(
            old.blocks[0],
            MarkdownBlock::Heading {
                level: 1,
                text: "标题👋".into()
            }
        );
        assert_eq!(
            old.blocks[1],
            MarkdownBlock::Paragraph("中文 e\u{301} text next".into())
        );
        assert_eq!(old.blocks[2], MarkdownBlock::Bullet("item".into()));
        assert_eq!(
            old.blocks[3],
            MarkdownBlock::Code {
                language: "rust".into(),
                source: "let x = \"🦀\";".into()
            }
        );
        for (old, new) in old.blocks.iter().zip(rich.blocks()) {
            assert_eq!(
                new.kind(),
                match old {
                    MarkdownBlock::Heading { level, .. } =>
                        RichBlockKind::Heading { level: *level },
                    MarkdownBlock::Paragraph(_) => RichBlockKind::Paragraph,
                    MarkdownBlock::Bullet(_) => RichBlockKind::Bullet,
                    MarkdownBlock::Code { .. } => RichBlockKind::Code,
                }
            );
        }
        let end = rich.text().encode_utf16().count();
        let final_visible_source_end = rich.blocks().last().unwrap().source().end;
        assert_eq!(
            rich.display_to_source(end..end),
            Some(final_visible_source_end..final_visible_source_end)
        );
        assert!(final_visible_source_end < rich.source().encode_utf16().count());
        assert_eq!(
            rich.source_to_display(
                rich.source().encode_utf16().count()..rich.source().encode_utf16().count()
            ),
            Some(end..end)
        );
        let display_emoji = rich.text().find('👋').unwrap();
        let d = byte_to_utf16(rich.text(), display_emoji);
        assert_eq!(
            rich.display_to_source(d..d + 2),
            Some("# 标题👋".encode_utf16().count() - 2.."# 标题👋".encode_utf16().count())
        );
        assert_eq!(rich.display_to_source(d + 1..d + 2), None);
        let source_emoji = source.find('👋').unwrap();
        let s = byte_to_utf16(source, source_emoji);
        assert!(rich.source_to_display(s..s + 2).is_some());
        assert_eq!(rich.source_to_display(s + 1..s + 2), None);
    }

    #[test]
    fn interleaved_markdown_blocks_flush_once_and_match_the_legacy_projection() {
        let source = "first\n\n# Head\n\nsecond\nthird\n\n- item\n\n```txt\ncode\n```\n\nlast";
        let parsed = Markdown::parse(source);
        assert_eq!(
            parsed.blocks,
            vec![
                MarkdownBlock::Paragraph("first".into()),
                MarkdownBlock::Heading {
                    level: 1,
                    text: "Head".into()
                },
                MarkdownBlock::Paragraph("second third".into()),
                MarkdownBlock::Bullet("item".into()),
                MarkdownBlock::Code {
                    language: "txt".into(),
                    source: "code".into()
                },
                MarkdownBlock::Paragraph("last".into()),
            ]
        );
        let rich = RichDocument::from_markdown(source);
        assert_eq!(rich.blocks().len(), parsed.blocks.len());
        assert_eq!(rich.text(), "first\nHead\nsecond third\n• item\ncode\nlast");
    }

    #[test]
    fn missing_fence_is_retained_as_code_and_link_targets_are_host_data() {
        let source = "[中文👩🏽‍💻](https://example.invalid)\n\n```rs\nhello";
        let rich = RichDocument::from_markdown(source);
        assert_eq!(
            Markdown::parse(source).blocks.last(),
            Some(&MarkdownBlock::Code {
                language: "rs".into(),
                source: "hello".into()
            })
        );
        assert!(rich.text().starts_with("中文👩🏽‍💻"));
        assert_eq!(rich.links().len(), 1);
        assert_eq!(rich.links()[0].target(), "https://example.invalid");
        assert_eq!(
            rich.links()[0].display(),
            0.."中文👩🏽‍💻".encode_utf16().count()
        );
        assert_eq!(
            rich.links()[0].source(),
            0.."[中文👩🏽‍💻](https://example.invalid)".encode_utf16().count()
        );
        let label_end = "[中文👩🏽‍💻".encode_utf16().count();
        let link_end = "[中文👩🏽‍💻](https://example.invalid)".encode_utf16().count();
        let collapsed = rich.source_to_display(label_end..link_end).unwrap();
        assert_eq!(
            collapsed,
            rich.blocks().last().unwrap().display().start
                ..rich.blocks().last().unwrap().display().start
        );
        assert_eq!(rich.source_to_display(0..1), Some(0..0));
        let code = rich.blocks().last().unwrap();
        assert_eq!(code.kind(), RichBlockKind::Code);
        let start = utf16_to_byte(rich.text(), code.display().start);
        let end = utf16_to_byte(rich.text(), code.display().end);
        assert!(rich.text()[start..end].contains("hello"));
        let display_end = rich.text().encode_utf16().count();
        let final_visible_end = rich.blocks().last().unwrap().source().end;
        assert_eq!(
            rich.display_to_source(display_end..display_end),
            Some(final_visible_end..final_visible_end)
        );
        assert_eq!(
            rich.source_to_display(
                rich.source().encode_utf16().count()..rich.source().encode_utf16().count()
            ),
            Some(display_end..display_end)
        );
        let emoji = source.find('👩').unwrap();
        let emoji_offset = byte_to_utf16(source, emoji);
        assert_eq!(
            rich.source_to_display(emoji_offset + 1..emoji_offset + 2),
            None
        );
    }

    #[test]
    fn unsupported_nested_escaped_and_inline_code_links_remain_literal() {
        let source = r"[nested [label](inner)](outer) \[escaped](target) `[code](target)`";
        let rich = RichDocument::from_markdown(source);
        assert!(rich.links().is_empty());
        assert!(rich.text().contains("[nested [label](inner)](outer)"));
        assert!(rich.text().contains(r"\[escaped](target)"));
        assert!(rich.text().contains("`[code](target)`"));
    }

    #[test]
    fn code_normalizes_line_endings_and_maps_to_exact_original_source() {
        let code = RichDocument::from_code(&CodeBlock {
            language: "txt".into(),
            source: "a\r\nb\rc".into(),
        });
        assert_eq!(code.source(), "a\r\nb\rc");
        assert_eq!(code.text(), "a\nb\nc");
        assert_eq!(code.source_to_display(1..3), Some(1..2));
        assert_eq!(code.display_to_source(1..2), Some(1..3));
    }

    #[test]
    fn markdown_normalizes_lone_cr_inside_paragraph_and_code_fragments_with_unicode_maps() {
        let source = "左😀\r右\n\n```txt\n碼😀\r尾\n```";
        let legacy = Markdown::parse(source);
        assert_eq!(
            legacy.blocks,
            vec![
                MarkdownBlock::Paragraph("左😀\r右".into()),
                MarkdownBlock::Code {
                    language: "txt".into(),
                    source: "碼😀\r尾".into(),
                },
            ]
        );

        let rich = RichDocument::from_markdown(source);
        assert_eq!(rich.text(), "左😀\n右\n碼😀\n尾");
        let paragraph_cr = "左😀".encode_utf16().count();
        let code_source_start = byte_to_utf16(source, source.find("碼").unwrap());
        let code_display_start = byte_to_utf16(rich.text(), rich.text().find("碼").unwrap());
        let code_cr = code_display_start + "碼😀".encode_utf16().count();
        assert_eq!(
            rich.display_to_source(paragraph_cr..paragraph_cr + 1),
            Some(paragraph_cr..paragraph_cr + 1)
        );
        assert_eq!(
            rich.source_to_display(paragraph_cr..paragraph_cr + 1),
            Some(paragraph_cr..paragraph_cr + 1)
        );
        assert_eq!(
            rich.display_to_source(code_cr..code_cr + 1),
            Some(
                code_source_start + "碼😀".encode_utf16().count()
                    ..code_source_start + "碼😀".encode_utf16().count() + 1
            )
        );
        assert_eq!(
            rich.source_to_display(
                code_source_start + "碼😀".encode_utf16().count()
                    ..code_source_start + "碼😀".encode_utf16().count() + 1
            ),
            Some(code_cr..code_cr + 1)
        );
    }

    #[test]
    fn code_and_diff_copies_exclude_diff_decorations_and_keep_kinds() {
        let code = RichDocument::from_code(&CodeBlock {
            language: "rs".into(),
            source: "a🦀\nb".into(),
        });
        assert_eq!(code.text(), "a🦀\nb");
        assert_eq!(code.source_to_display(1..3), Some(1..3));
        let diff = DiffView {
            lines: vec![
                DiffLine {
                    kind: DiffKind::Removed,
                    text: "old🦀".into(),
                },
                DiffLine {
                    kind: DiffKind::Added,
                    text: "new".into(),
                },
            ],
        };
        let rich = RichDocument::from_diff(&diff);
        assert_eq!(rich.text(), "old🦀\nnew");
        assert_eq!(
            rich.blocks()[0].kind(),
            RichBlockKind::Diff {
                kind: DiffKind::Removed
            }
        );
        assert_eq!(
            rich.blocks()[1].kind(),
            RichBlockKind::Diff {
                kind: DiffKind::Added
            }
        );
    }

    #[test]
    fn range_endpoints_reject_invalid_order_and_map_carets_rightward() {
        let rich = RichDocument::from_markdown("- a\n- b");
        let bullet_source = "- a".encode_utf16().count();
        assert_eq!(rich.source_to_display(2..2), Some(2..2));
        assert_eq!(rich.display_to_source(1..1), Some(2..2));
        assert_eq!(rich.source_to_display(Range { start: 3, end: 1 }), None);
        assert_eq!(rich.display_to_source(0..2), Some(0..2));
        assert!(bullet_source > 0);
    }
}
