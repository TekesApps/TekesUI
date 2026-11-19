//! Lossless, line-preserving Live-Preview projection between canonical Markdown
//! source and a flat editable display string.
//!
//! Canonical `source` is normalized to LF (`\n`). Unfocused editors project all
//! blocks collapsed; when focused, blocks intersecting `expanded_source` reveal
//! raw Markdown delimiters inline while preserving block/inline styling roles.
//! Range APIs use UTF-16 code units on Unicode grapheme boundaries.
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use super::rich_document::{
    byte_to_utf16, find_backtick_span, grapheme_boundary, is_fence_close, parse_fence_open,
    try_parse_simple_link, utf16_to_byte,
};
use super::{RichBlockKind, RichLink};
use tekes_ui_tokens::proposal::TextRole;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownInlineKind {
    Code,
    Strong,
    Link,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownInlineSpan {
    pub(super) display: Range<usize>,
    pub(super) source: Range<usize>,
    pub(super) kind: MarkdownInlineKind,
    pub(super) target: Option<String>,
}

impl MarkdownInlineSpan {
    pub fn display(&self) -> Range<usize> {
        self.display.clone()
    }
    pub fn source(&self) -> Range<usize> {
        self.source.clone()
    }
    pub fn kind(&self) -> MarkdownInlineKind {
        self.kind
    }
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownProjectedBlock {
    pub(super) display: Range<usize>,
    pub(super) body_display: Range<usize>,
    pub(super) source: Range<usize>,
    pub(super) role: TextRole,
    pub(super) kind: RichBlockKind,
    pub(super) language: Option<String>,
    pub(super) expanded: bool,
    pub(super) fence_display: Vec<Range<usize>>,
}

impl MarkdownProjectedBlock {
    pub fn display(&self) -> Range<usize> {
        self.display.clone()
    }
    pub fn body_display(&self) -> Range<usize> {
        self.body_display.clone()
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
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
    pub(super) fn fence_display(&self) -> &[Range<usize>] {
        &self.fence_display
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MapSpan {
    source: Range<usize>,
    display: Range<usize>,
}

/// Complete canonical Markdown source and its Live-Preview display projection.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownProjection {
    source: String,
    source_utf16_len: usize,
    text: String,
    text_utf16_len: usize,
    blocks: Vec<MarkdownProjectedBlock>,
    links: Vec<RichLink>,
    inlines: Vec<MarkdownInlineSpan>,
    map: Vec<MapSpan>,
}

pub(super) fn normalize_markdown_source(raw: &str) -> String {
    if !raw.contains('\r') {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push('\n');
            i += if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                2
            } else {
                1
            };
        } else {
            let len = raw[i..].chars().next().expect("valid UTF-8").len_utf8();
            out.push_str(&raw[i..i + len]);
            i += len;
        }
    }
    out
}

pub(super) fn clamp_grapheme_utf16(text: &str, target: usize) -> usize {
    let mut best = 0usize;
    let mut acc = 0usize;
    for grapheme in text.graphemes(true) {
        let len = grapheme.encode_utf16().count();
        if acc + len <= target {
            acc += len;
            best = acc;
        } else {
            break;
        }
    }
    best
}

enum RawBlock {
    Heading {
        level: u8,
        line: Range<usize>,
        prefix_end: usize,
    },
    Bullet {
        line: Range<usize>,
        prefix_end: usize,
    },
    Code {
        language: String,
        open_line: Range<usize>,
        body_lines: Vec<Range<usize>>,
        close_line: Option<Range<usize>>,
        source_bytes: Range<usize>,
    },
    Paragraph {
        lines: Vec<Range<usize>>,
    },
    Blank {
        line: Range<usize>,
        has_newline: bool,
    },
}

fn split_lf_lines(source: &str) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (idx, b) in source.bytes().enumerate() {
        if b == b'\n' {
            lines.push(start..idx);
            start = idx + 1;
        }
    }
    lines.push(start..source.len());
    lines
}

fn parse_raw_blocks(source: &str) -> Vec<RawBlock> {
    if source.is_empty() {
        return Vec::new();
    }
    let lines = split_lf_lines(source);
    let mut blocks = Vec::new();
    let mut paragraph: Vec<Range<usize>> = Vec::new();
    let mut i = 0usize;

    let flush_paragraph = |blocks: &mut Vec<RawBlock>, paragraph: &mut Vec<Range<usize>>| {
        if !paragraph.is_empty() {
            blocks.push(RawBlock::Paragraph {
                lines: std::mem::take(paragraph),
            });
        }
    };

    while i < lines.len() {
        let line = lines[i].clone();
        let raw = &source[line.clone()];
        let trimmed = raw.trim();
        let leading = raw.len() - raw.trim_start().len();

        if let Some((open_ticks, lang)) = parse_fence_open(raw) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let open_line = line;
            let mut body_lines = Vec::new();
            let mut close_line = None;
            let mut j = i + 1;
            while j < lines.len() {
                let candidate = lines[j].clone();
                if is_fence_close(&source[candidate.clone()], open_ticks) {
                    close_line = Some(candidate);
                    break;
                }
                body_lines.push(candidate);
                j += 1;
            }
            let source_end = close_line
                .as_ref()
                .map(|c| c.end)
                .or_else(|| body_lines.last().map(|l| l.end))
                .unwrap_or(open_line.end);
            blocks.push(RawBlock::Code {
                language: lang.to_owned(),
                source_bytes: open_line.start..source_end,
                open_line,
                body_lines,
                close_line,
            });
            i = if j < lines.len() { j + 1 } else { lines.len() };
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
            // Trailing empty slice after final '\n' has no separate '\n' after it;
            // whitespace-only lines must remain 1:1 in display/map.
            let has_newline = i + 1 < lines.len();
            if has_newline || !line.is_empty() {
                blocks.push(RawBlock::Blank { line, has_newline });
            }
            i += 1;
            continue;
        }

        let leading_raw = &raw[leading..];
        let hashes = leading_raw.bytes().take_while(|&b| b == b'#').count();
        if leading <= 3
            && (1..=6).contains(&hashes)
            && leading_raw.as_bytes().get(hashes) == Some(&b' ')
        {
            flush_paragraph(&mut blocks, &mut paragraph);
            let prefix_end = line.start + leading + hashes + 1;
            blocks.push(RawBlock::Heading {
                level: hashes as u8,
                line,
                prefix_end,
            });
            i += 1;
            continue;
        }

        if leading <= 3 && (leading_raw.starts_with("- ") || leading_raw.starts_with("* ")) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let prefix_end = line.start + leading + 2;
            blocks.push(RawBlock::Bullet { line, prefix_end });
            i += 1;
            continue;
        }

        paragraph.push(line);
        i += 1;
    }

    flush_paragraph(&mut blocks, &mut paragraph);
    blocks
}

fn is_block_expanded(block_src: &Range<usize>, expanded: Option<&Range<usize>>) -> bool {
    let Some(range) = expanded else {
        return false;
    };
    if range.is_empty() {
        block_src.start <= range.start && range.start <= block_src.end
    } else {
        range.start < block_src.end && range.end > block_src.start
    }
}

struct EmitContext<'a> {
    source: &'a str,
    text: String,
    text_utf16: usize,
    blocks: Vec<MarkdownProjectedBlock>,
    links: Vec<RichLink>,
    inlines: Vec<MarkdownInlineSpan>,
    map: Vec<MapSpan>,
}

impl<'a> EmitContext<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            text: String::with_capacity(source.len()),
            text_utf16: 0,
            blocks: Vec::new(),
            links: Vec::new(),
            inlines: Vec::new(),
            map: Vec::new(),
        }
    }

    fn emit_verbatim(&mut self, byte_range: Range<usize>) {
        if byte_range.is_empty() {
            return;
        }
        let slice = &self.source[byte_range.clone()];
        let mut src_u16 = byte_to_utf16(self.source, byte_range.start);
        for grapheme in slice.graphemes(true) {
            let len = grapheme.encode_utf16().count();
            self.text.push_str(grapheme);
            self.map.push(MapSpan {
                source: src_u16..src_u16 + len,
                display: self.text_utf16..self.text_utf16 + len,
            });
            src_u16 += len;
            self.text_utf16 += len;
        }
    }

    fn emit_mapped_unit(&mut self, display_piece: &str, src_byte_range: Range<usize>) {
        let src_start = byte_to_utf16(self.source, src_byte_range.start);
        let src_end = byte_to_utf16(self.source, src_byte_range.end);
        let disp_start = self.text_utf16;
        let len = display_piece.encode_utf16().count();
        self.text.push_str(display_piece);
        self.map.push(MapSpan {
            source: src_start..src_end,
            display: disp_start..disp_start + len,
        });
        self.text_utf16 += len;
    }

    fn emit_newline_after(&mut self, end_byte: usize) {
        if self.source.as_bytes().get(end_byte) == Some(&b'\n') {
            self.emit_verbatim(end_byte..end_byte + 1);
        }
    }

    fn emit_inline_line(&mut self, seg_range: Range<usize>, expanded: bool) {
        let line = &self.source[seg_range.clone()];
        let bytes = line.as_bytes();
        let mut scan = 0usize;
        let mut plain_start = 0usize;

        while scan < bytes.len() {
            let b = bytes[scan];
            if b == b'`' {
                match find_backtick_span(line, scan) {
                    Ok((ticks, close_pos, end_after)) if close_pos > scan + ticks => {
                        let inner = &line[scan + ticks..close_pos];
                        let prev = &line[plain_start..scan];
                        let next = &line[end_after..];
                        let clean_inner = clean_splice(prev, inner, next)
                            && grapheme_boundary(line, byte_to_utf16(line, scan))
                            && grapheme_boundary(line, byte_to_utf16(line, scan + ticks))
                            && grapheme_boundary(line, byte_to_utf16(line, close_pos))
                            && grapheme_boundary(line, byte_to_utf16(line, end_after));
                        if clean_inner {
                            self.emit_verbatim(
                                seg_range.start + plain_start..seg_range.start + scan,
                            );
                            let span_src = byte_to_utf16(self.source, seg_range.start + scan)
                                ..byte_to_utf16(self.source, seg_range.start + end_after);
                            let disp_start = self.text_utf16;
                            if expanded {
                                self.emit_verbatim(
                                    seg_range.start + scan..seg_range.start + end_after,
                                );
                            } else {
                                self.emit_verbatim(
                                    seg_range.start + scan + ticks..seg_range.start + close_pos,
                                );
                            }
                            let disp_end = self.text_utf16;
                            self.inlines.push(MarkdownInlineSpan {
                                display: disp_start..disp_end,
                                source: span_src,
                                kind: MarkdownInlineKind::Code,
                                target: None,
                            });
                            plain_start = end_after;
                            scan = end_after;
                            continue;
                        }
                        scan = end_after;
                        continue;
                    }
                    Ok((_, _, next)) | Err(next) => {
                        scan = next;
                        continue;
                    }
                }
            }

            if b == b'[' {
                let prev = &line[plain_start..scan];
                if let Ok((close, _label, target, end)) = try_parse_simple_link(line, scan, prev) {
                    self.emit_verbatim(seg_range.start + plain_start..seg_range.start + scan);
                    let src_range = byte_to_utf16(self.source, seg_range.start + scan)
                        ..byte_to_utf16(self.source, seg_range.start + end + 1);
                    if expanded {
                        self.emit_verbatim(seg_range.start + scan..seg_range.start + scan + 1);
                    }
                    let label_disp_start = self.text_utf16;
                    self.emit_verbatim(seg_range.start + scan + 1..seg_range.start + close);
                    let label_disp_end = self.text_utf16;
                    if expanded {
                        self.emit_verbatim(seg_range.start + close..seg_range.start + end + 1);
                    }
                    self.links.push(RichLink {
                        display: label_disp_start..label_disp_end,
                        source: src_range.clone(),
                        target: target.to_owned(),
                    });
                    self.inlines.push(MarkdownInlineSpan {
                        display: label_disp_start..label_disp_end,
                        source: src_range,
                        kind: MarkdownInlineKind::Link,
                        target: Some(target.to_owned()),
                    });
                    plain_start = end + 1;
                    scan = plain_start;
                    continue;
                }
            }

            if line[scan..].starts_with("**") && is_unescaped(bytes, scan) {
                if let Some(end_after) = find_strong_end(line, scan) {
                    let close_pos = end_after - 2;
                    let inner = &line[scan + 2..close_pos];
                    let prev = &line[plain_start..scan];
                    let next = &line[end_after..];
                    let valid = !inner.is_empty()
                        && !inner.contains('`')
                        && !inner.contains('\n')
                        && clean_splice(prev, inner, next)
                        && grapheme_boundary(line, byte_to_utf16(line, scan))
                        && grapheme_boundary(line, byte_to_utf16(line, scan + 2))
                        && grapheme_boundary(line, byte_to_utf16(line, close_pos))
                        && grapheme_boundary(line, byte_to_utf16(line, end_after));
                    if valid {
                        self.emit_verbatim(seg_range.start + plain_start..seg_range.start + scan);
                        let span_src = byte_to_utf16(self.source, seg_range.start + scan)
                            ..byte_to_utf16(self.source, seg_range.start + end_after);
                        let disp_start = self.text_utf16;
                        if expanded {
                            self.emit_verbatim(seg_range.start + scan..seg_range.start + end_after);
                        } else {
                            self.emit_verbatim(
                                seg_range.start + scan + 2..seg_range.start + close_pos,
                            );
                        }
                        let disp_end = self.text_utf16;
                        self.inlines.push(MarkdownInlineSpan {
                            display: disp_start..disp_end,
                            source: span_src,
                            kind: MarkdownInlineKind::Strong,
                            target: None,
                        });
                        plain_start = end_after;
                        scan = end_after;
                        continue;
                    }
                }
                scan += 1;
                continue;
            }

            scan += line[scan..].chars().next().expect("valid UTF-8").len_utf8();
        }

        if plain_start < line.len() {
            self.emit_verbatim(seg_range.start + plain_start..seg_range.end);
        }
    }
}

fn is_unescaped(bytes: &[u8], pos: usize) -> bool {
    let mut slashes = 0usize;
    let mut cur = pos;
    while cur > 0 && bytes[cur - 1] == b'\\' {
        slashes += 1;
        cur -= 1;
    }
    slashes.is_multiple_of(2)
}

fn find_strong_end(line: &str, open: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = open + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' {
            i += 1;
            if i < bytes.len() {
                i += line[i..].chars().next().expect("valid UTF-8").len_utf8();
            }
            continue;
        }
        if bytes[i] == b'*' && bytes[i + 1] == b'*' {
            return Some(i + 2);
        }
        i += line[i..].chars().next().expect("valid UTF-8").len_utf8();
    }
    None
}

fn clean_splice(prev: &str, inner: &str, next: &str) -> bool {
    (prev.is_empty() || {
        let combined = format!("{prev}{inner}");
        grapheme_boundary(&combined, byte_to_utf16(prev, prev.len()))
    }) && (next.is_empty() || {
        let combined = format!("{inner}{next}");
        grapheme_boundary(&combined, byte_to_utf16(inner, inner.len()))
    })
}

impl MarkdownProjection {
    pub fn from_markdown(raw_source: &str, expanded_source: Option<Range<usize>>) -> Self {
        let source = normalize_markdown_source(raw_source);
        let source_utf16_len = source.encode_utf16().count();
        let raw_blocks = parse_raw_blocks(&source);
        let mut ctx = EmitContext::new(&source);

        for raw in raw_blocks {
            match raw {
                RawBlock::Blank { line, has_newline } => {
                    let end = line.end;
                    ctx.emit_verbatim(line);
                    if has_newline {
                        ctx.emit_newline_after(end);
                    }
                }
                RawBlock::Heading {
                    level,
                    line,
                    prefix_end,
                } => {
                    let src_range =
                        byte_to_utf16(&source, line.start)..byte_to_utf16(&source, line.end);
                    let expanded = is_block_expanded(&src_range, expanded_source.as_ref());
                    let disp_start = ctx.text_utf16;
                    if expanded {
                        ctx.emit_verbatim(line.start..prefix_end);
                    }
                    ctx.emit_inline_line(prefix_end..line.end, expanded);
                    let disp_end = ctx.text_utf16;
                    ctx.blocks.push(MarkdownProjectedBlock {
                        display: disp_start..disp_end,
                        body_display: disp_start..disp_end,
                        source: src_range,
                        role: TextRole::UiHeading,
                        kind: RichBlockKind::Heading { level },
                        language: None,
                        expanded,
                        fence_display: Vec::new(),
                    });
                    ctx.emit_newline_after(line.end);
                }
                RawBlock::Bullet { line, prefix_end } => {
                    let src_range =
                        byte_to_utf16(&source, line.start)..byte_to_utf16(&source, line.end);
                    let expanded = is_block_expanded(&src_range, expanded_source.as_ref());
                    let disp_start = ctx.text_utf16;
                    if expanded {
                        ctx.emit_verbatim(line.start..prefix_end);
                    } else {
                        ctx.emit_mapped_unit("•", line.start..prefix_end - 1);
                        ctx.emit_mapped_unit(" ", prefix_end - 1..prefix_end);
                    }
                    ctx.emit_inline_line(prefix_end..line.end, expanded);
                    let disp_end = ctx.text_utf16;
                    ctx.blocks.push(MarkdownProjectedBlock {
                        display: disp_start..disp_end,
                        body_display: disp_start..disp_end,
                        source: src_range,
                        role: TextRole::UiBody,
                        kind: RichBlockKind::Bullet,
                        language: None,
                        expanded,
                        fence_display: Vec::new(),
                    });
                    ctx.emit_newline_after(line.end);
                }
                RawBlock::Code {
                    language,
                    open_line,
                    body_lines,
                    close_line,
                    source_bytes,
                } => {
                    let src_range = byte_to_utf16(&source, source_bytes.start)
                        ..byte_to_utf16(&source, source_bytes.end);
                    let expanded = is_block_expanded(&src_range, expanded_source.as_ref());
                    let disp_start = ctx.text_utf16;
                    let mut fence_display = Vec::new();
                    let body_display;
                    if expanded {
                        let open_disp_start = ctx.text_utf16;
                        ctx.emit_verbatim(open_line.clone());
                        let open_disp_end = ctx.text_utf16;
                        fence_display.push(open_disp_start..open_disp_end);
                        if !body_lines.is_empty() {
                            ctx.emit_newline_after(open_line.end);
                            let body_start = body_lines.first().unwrap().start;
                            let body_end = body_lines.last().unwrap().end;
                            let body_disp_start = ctx.text_utf16;
                            ctx.emit_verbatim(body_start..body_end);
                            let body_disp_end = ctx.text_utf16;
                            body_display = body_disp_start..body_disp_end;
                            if let Some(close) = &close_line {
                                ctx.emit_newline_after(body_end);
                                let close_disp_start = ctx.text_utf16;
                                ctx.emit_verbatim(close.clone());
                                let close_disp_end = ctx.text_utf16;
                                fence_display.push(close_disp_start..close_disp_end);
                            }
                        } else if let Some(close) = &close_line {
                            ctx.emit_newline_after(open_line.end);
                            let body_at = ctx.text_utf16;
                            body_display = body_at..body_at;
                            let close_disp_start = ctx.text_utf16;
                            ctx.emit_verbatim(close.clone());
                            let close_disp_end = ctx.text_utf16;
                            fence_display.push(close_disp_start..close_disp_end);
                        } else {
                            let body_at = ctx.text_utf16;
                            body_display = body_at..body_at;
                        }
                    } else if !body_lines.is_empty() {
                        let body_start = body_lines.first().unwrap().start;
                        let body_end = body_lines.last().unwrap().end;
                        let body_disp_start = ctx.text_utf16;
                        ctx.emit_verbatim(body_start..body_end);
                        let body_disp_end = ctx.text_utf16;
                        body_display = body_disp_start..body_disp_end;
                    } else {
                        body_display = disp_start..disp_start;
                    }
                    let disp_end = ctx.text_utf16;
                    ctx.blocks.push(MarkdownProjectedBlock {
                        display: disp_start..disp_end,
                        body_display,
                        source: src_range,
                        role: TextRole::Code,
                        kind: RichBlockKind::Code,
                        language: Some(language),
                        expanded,
                        fence_display,
                    });
                    let end_byte = close_line
                        .as_ref()
                        .map(|c| c.end)
                        .or_else(|| body_lines.last().map(|l| l.end))
                        .unwrap_or(open_line.end);
                    ctx.emit_newline_after(end_byte);
                }
                RawBlock::Paragraph { lines } => {
                    let first = lines.first().expect("nonempty paragraph");
                    let last = lines.last().expect("nonempty paragraph");
                    let src_range =
                        byte_to_utf16(&source, first.start)..byte_to_utf16(&source, last.end);
                    let expanded = is_block_expanded(&src_range, expanded_source.as_ref());
                    let disp_start = ctx.text_utf16;
                    for (idx, line) in lines.iter().enumerate() {
                        if idx > 0 {
                            ctx.emit_newline_after(lines[idx - 1].end);
                        }
                        ctx.emit_inline_line(line.clone(), expanded);
                    }
                    let disp_end = ctx.text_utf16;
                    ctx.blocks.push(MarkdownProjectedBlock {
                        display: disp_start..disp_end,
                        body_display: disp_start..disp_end,
                        source: src_range,
                        role: TextRole::UiBody,
                        kind: RichBlockKind::Paragraph,
                        language: None,
                        expanded,
                        fence_display: Vec::new(),
                    });
                    ctx.emit_newline_after(last.end);
                }
            }
        }

        let EmitContext {
            text,
            text_utf16,
            blocks,
            links,
            inlines,
            map,
            ..
        } = ctx;

        Self {
            source,
            source_utf16_len,
            text,
            text_utf16_len: text_utf16,
            blocks,
            links,
            inlines,
            map,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn blocks(&self) -> &[MarkdownProjectedBlock] {
        &self.blocks
    }
    pub fn links(&self) -> &[RichLink] {
        &self.links
    }
    pub fn inlines(&self) -> &[MarkdownInlineSpan] {
        &self.inlines
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

    /// Reconcile an edited display string and UTF-16 selection back into the
    /// canonical LF source, preserving collapsed syntax outside the edited span.
    pub(super) fn apply_display_state(
        &self,
        new_display_raw: &str,
        new_anchor: usize,
        new_caret: usize,
    ) -> (String, usize, usize) {
        let new_display = normalize_markdown_source(new_display_raw);
        let clamped_anchor = clamp_grapheme_utf16(&new_display, new_anchor);
        let clamped_caret = clamp_grapheme_utf16(&new_display, new_caret);

        if new_display == self.text {
            if clamped_anchor == clamped_caret {
                let at = self
                    .display_to_source(clamped_anchor..clamped_anchor)
                    .map_or(0, |r| r.start);
                return (self.source.clone(), at, at);
            }
            let min = clamped_anchor.min(clamped_caret);
            let max = clamped_anchor.max(clamped_caret);
            let (mut src_start, mut src_end) = if min == 0 && max == self.text_utf16_len {
                (0, self.source_utf16_len)
            } else if let Some(r) = self.display_to_source(min..max) {
                (r.start, r.end)
            } else {
                let a = self.display_to_source(min..min).map_or(0, |r| r.start);
                let b = self.display_to_source(max..max).map_or(a, |r| r.start);
                (a.min(b), a.max(b))
            };
            for block in &self.blocks {
                if !block.expanded {
                    if block.display.start == min && max > min {
                        src_start = src_start.min(block.source.start);
                    }
                    if block.display.end == max && min < max {
                        src_end = src_end.max(block.source.end);
                    }
                }
            }
            let (a, c) = if clamped_anchor <= clamped_caret {
                (src_start, src_end)
            } else {
                (src_end, src_start)
            };
            return (self.source.clone(), a, c);
        }

        let old_g: Vec<&str> = self.text.graphemes(true).collect();
        let new_g: Vec<&str> = new_display.graphemes(true).collect();
        let mut g_prefix = 0usize;
        while g_prefix < old_g.len() && g_prefix < new_g.len() && old_g[g_prefix] == new_g[g_prefix]
        {
            g_prefix += 1;
        }
        let mut g_suffix = 0usize;
        while g_prefix + g_suffix < old_g.len()
            && g_prefix + g_suffix < new_g.len()
            && old_g[old_g.len() - 1 - g_suffix] == new_g[new_g.len() - 1 - g_suffix]
        {
            g_suffix += 1;
        }

        let old_prefix_u16: usize = old_g[..g_prefix]
            .iter()
            .map(|g| g.encode_utf16().count())
            .sum();
        let old_suffix_u16: usize = old_g[old_g.len() - g_suffix..]
            .iter()
            .map(|g| g.encode_utf16().count())
            .sum();
        let old_end_u16 = self.text_utf16_len - old_suffix_u16;

        let new_prefix_u16: usize = new_g[..g_prefix]
            .iter()
            .map(|g| g.encode_utf16().count())
            .sum();
        let new_suffix_u16: usize = new_g[new_g.len() - g_suffix..]
            .iter()
            .map(|g| g.encode_utf16().count())
            .sum();
        let new_display_u16 = new_display.encode_utf16().count();
        let new_end_u16 = new_display_u16 - new_suffix_u16;

        let new_start_byte: usize = new_g[..g_prefix].iter().map(|g| g.len()).sum();
        let new_suffix_byte: usize = new_g[new_g.len() - g_suffix..]
            .iter()
            .map(|g| g.len())
            .sum();
        let inserted = &new_display[new_start_byte..new_display.len() - new_suffix_byte];

        let src_edit = if old_prefix_u16 == 0
            && old_end_u16 == self.text_utf16_len
            && self.text_utf16_len == 0
        {
            0..self.source_utf16_len
        } else {
            self.display_to_source(old_prefix_u16..old_end_u16)
                .unwrap_or(0..self.source_utf16_len)
        };
        let src_start_byte = utf16_to_byte(&self.source, src_edit.start);
        let src_end_byte = utf16_to_byte(&self.source, src_edit.end);

        let mut new_source = String::with_capacity(self.source.len() + inserted.len());
        new_source.push_str(&self.source[..src_start_byte]);
        new_source.push_str(inserted);
        new_source.push_str(&self.source[src_end_byte..]);

        let inserted_u16 = new_end_u16 - new_prefix_u16;
        let deleted_src_u16 = src_edit.end - src_edit.start;
        let map_post_edit = |pos: usize| -> usize {
            let mapped = if pos <= new_prefix_u16 {
                self.display_to_source(pos..pos)
                    .map_or(src_edit.start, |r| r.start)
            } else if pos >= new_end_u16 {
                let old_pos = old_end_u16 + (pos - new_end_u16);
                let old_src = self
                    .display_to_source(old_pos..old_pos)
                    .map_or(src_edit.end, |r| r.start);
                let delta = inserted_u16 as isize - deleted_src_u16 as isize;
                (old_src as isize + delta).max(0) as usize
            } else {
                src_edit.start + (pos - new_prefix_u16)
            };
            clamp_grapheme_utf16(&new_source, mapped)
        };

        let next_anchor = map_post_edit(clamped_anchor);
        let next_caret = map_post_edit(clamped_caret);
        (new_source, next_anchor, next_caret)
    }

    fn display_start_to_source(&self, offset: usize) -> Option<usize> {
        if offset == self.text_utf16_len {
            return Some(
                self.map
                    .last()
                    .map_or(self.source_utf16_len, |span| span.source.end),
            );
        }
        self.map
            .iter()
            .find(|s| s.display.start == offset)
            .map(|s| s.source.start)
    }

    fn display_end_to_source(&self, offset: usize) -> Option<usize> {
        if offset == 0 {
            return Some(self.map.first().map_or(0, |span| span.source.start));
        }
        self.map
            .iter()
            .find(|s| s.display.end == offset)
            .map(|s| s.source.end)
    }

    fn source_start_to_display(&self, offset: usize) -> Option<usize> {
        if offset >= self.source_utf16_len {
            return Some(self.text_utf16_len);
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
            .or(Some(self.text_utf16_len))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_slice(text: &str, range: Range<usize>) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[range]).unwrap()
    }

    #[test]
    fn live_preview_expands_only_active_blocks_and_preserves_newlines() {
        let source = "\n# 标题👋\n\n- 第一项 `code` **加粗** [手册](diff-view.md)\n\n```rust\nlet x = 1;\n```";

        // Unfocused: every block is collapsed while blank newlines stay 1:1.
        let unfocused = MarkdownProjection::from_markdown(source, None);
        assert_eq!(
            unfocused.text(),
            "\n标题👋\n\n• 第一项 code 加粗 手册\n\nlet x = 1;"
        );
        assert!(unfocused.blocks().iter().all(|b| !b.is_expanded()));
        assert_eq!(unfocused.links().len(), 1);
        assert_eq!(unfocused.links()[0].target(), "diff-view.md");

        // Focusing inside bullet line expands ONLY the bullet, leaving heading & code collapsed.
        let bullet_start = source[..source.find("- ").unwrap()].encode_utf16().count();
        let bullet_focused =
            MarkdownProjection::from_markdown(source, Some(bullet_start..bullet_start));
        assert_eq!(
            bullet_focused.text(),
            "\n标题👋\n\n- 第一项 `code` **加粗** [手册](diff-view.md)\n\nlet x = 1;"
        );
        assert!(!bullet_focused.blocks()[0].is_expanded());
        assert!(bullet_focused.blocks()[1].is_expanded());
        assert!(!bullet_focused.blocks()[2].is_expanded());

        // Code fence focused reveals opening/closing fence lines separately from body lines.
        let code_pos = source[..source.find("let x").unwrap()]
            .encode_utf16()
            .count();
        let code_focused = MarkdownProjection::from_markdown(source, Some(code_pos..code_pos));
        let code_block = &code_focused.blocks()[2];
        assert!(code_block.is_expanded());
        assert_eq!(
            utf16_slice(code_focused.text(), code_block.body_display()),
            "let x = 1;"
        );
        assert_eq!(code_block.fence_display().len(), 2);
        assert_eq!(
            utf16_slice(code_focused.text(), code_block.fence_display()[0].clone()),
            "```rust"
        );
        assert_eq!(
            utf16_slice(code_focused.text(), code_block.fence_display()[1].clone()),
            "```"
        );

        // SelectAll expands every block so copy/cut operates on lossless canonical source.
        let total_u16 = source.encode_utf16().count();
        let all_selected = MarkdownProjection::from_markdown(source, Some(0..total_u16));
        assert_eq!(all_selected.text(), source);
        assert_eq!(
            all_selected.display_to_source(0..total_u16),
            Some(0..total_u16)
        );
    }

    #[test]
    fn handles_unclosed_code_fence_empty_code_block_and_trailing_blank_lines() {
        let empty_code = "```rust\n```\n";
        let collapsed = MarkdownProjection::from_markdown(empty_code, None);
        assert_eq!(collapsed.text(), "\n");
        assert_eq!(collapsed.blocks().len(), 1);
        assert_eq!(collapsed.blocks()[0].kind(), RichBlockKind::Code);
        assert!(collapsed.blocks()[0].display().is_empty());

        let unclosed = "```rust\nlet a = 42;";
        let p = MarkdownProjection::from_markdown(unclosed, Some(0..0));
        assert_eq!(p.text(), unclosed);
        assert_eq!(p.blocks()[0].fence_display().len(), 1);
        assert_eq!(
            utf16_slice(p.text(), p.blocks()[0].body_display()),
            "let a = 42;"
        );

        let p_collapsed = MarkdownProjection::from_markdown(unclosed, None);
        assert_eq!(p_collapsed.text(), "let a = 42;");
    }

    #[test]
    fn unicode_zwj_crlf_and_display_edit_reconciliation_preserve_collapsed_syntax() {
        let raw = "# Heading\r\n👩\u{200d}🔬 [Docs](guide.md) end\r\n";
        let normalized = normalize_markdown_source(raw);
        let h_pos = 4usize; // inside the heading text ("# He|ading")
        let proj = MarkdownProjection::from_markdown(&normalized, Some(h_pos..h_pos));
        assert_eq!(proj.text(), "# Heading\n👩\u{200d}🔬 Docs end\n");

        // Mid-grapheme UTF-16 offset must be rejected.
        let zwj_start = proj.text()[..proj.text().find('👩').unwrap()]
            .encode_utf16()
            .count();
        assert!(
            proj.display_to_source(zwj_start + 1..zwj_start + 1)
                .is_none()
        );

        // Edit Heading while paragraph link stays collapsed -> hidden (guide.md) survives intact!
        let edited_display = "# Heading!\n👩\u{200d}🔬 Docs end\n";
        let (next_src, next_a, next_c) = proj.apply_display_state(edited_display, 10, 10);
        assert_eq!(next_src, "# Heading!\n👩\u{200d}🔬 [Docs](guide.md) end\n");
        assert_eq!((next_a, next_c), (10, 10));

        // Clicking collapsed link label lands inside source paragraph and expands it.
        let doc_click = proj.text()[..proj.text().find("Docs").unwrap()]
            .encode_utf16()
            .count();
        let (_, a, c) = proj.apply_display_state(proj.text(), doc_click, doc_click);
        let expanded_para = MarkdownProjection::from_markdown(&normalized, Some(a..c));
        assert_eq!(
            expanded_para.text(),
            "Heading\n👩\u{200d}🔬 [Docs](guide.md) end\n"
        );
    }
}
