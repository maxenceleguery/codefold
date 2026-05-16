//! Markdown rendering via `pulldown_cmark`. We deliberately avoid a
//! tree-sitter grammar here: `tree-sitter-md` ships with the newer
//! `LanguageFn` API that's not source-compatible with our tree-sitter 0.22.
//!
//! Levels mapped to markdown:
//!   - `Full`   : the whole document (handled by `read_opts` upstream).
//!   - `Bodies` : same as Full — markdown has no nested "function bodies".
//!   - `Signatures` / `Public` : keep heading lines only (ATX and Setext),
//!     hide every byte between them. Turns a long README / spec into its
//!     outline.
//!
//! Symbols: one per heading. `name` is the heading text (without `#`
//! markers), `kind` is `Class` (closest existing `SymbolKind` for a section
//! header), `line_start`/`line_end` point at the heading line(s).

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::result::{Symbol, SymbolKind};
use crate::Level;

pub struct RenderOutput {
    pub content: String,
    pub symbols: Vec<Symbol>,
    pub hidden_ranges: Vec<(usize, usize)>,
}

pub fn render(source: &str, level: Level) -> RenderOutput {
    let headings = parse_headings(source);
    let symbols = headings_to_symbols(&headings);

    let keep_only_headings = matches!(level, Level::Signatures | Level::Public);
    if !keep_only_headings {
        // Full or Bodies: emit verbatim.
        return RenderOutput {
            content: source.to_string(),
            symbols,
            hidden_ranges: Vec::new(),
        };
    }

    // Skeleton output: emit each heading verbatim with one blank line between.
    let mut out = String::new();
    let mut hidden = Vec::new();
    let mut cursor_byte = 0usize;
    for (i, h) in headings.iter().enumerate() {
        if h.byte_start > cursor_byte {
            hidden.push((cursor_byte, h.byte_start));
        }
        if i > 0 {
            out.push_str("\n\n");
        }
        if let Some(s) = source.get(h.byte_start..h.byte_end) {
            out.push_str(s.trim_end_matches('\n'));
        }
        cursor_byte = h.byte_end;
    }
    if cursor_byte < source.len() {
        hidden.push((cursor_byte, source.len()));
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }

    RenderOutput {
        content: out,
        symbols,
        hidden_ranges: hidden,
    }
}

struct Heading {
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
    title: String,
    level: HeadingLevel,
}

fn parse_headings(source: &str) -> Vec<Heading> {
    let line_starts = build_line_starts(source);
    let parser = Parser::new_ext(source, Options::empty());
    let iter = parser.into_offset_iter();

    let mut headings = Vec::new();
    let mut current: Option<(HeadingLevel, std::ops::Range<usize>, String)> = None;

    for (event, range) in iter {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((level, range, String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, span, title)) = current.take() {
                    headings.push(Heading {
                        byte_start: span.start,
                        byte_end: span.end,
                        line_start: byte_to_line(&line_starts, span.start),
                        line_end: byte_to_line(&line_starts, span.end.saturating_sub(1)),
                        title: title.trim().to_string(),
                        level,
                    });
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, _, title)) = current.as_mut() {
                    title.push_str(&t);
                }
            }
            _ => {}
        }
    }
    headings.sort_by_key(|h| h.byte_start);
    headings
}

fn headings_to_symbols(headings: &[Heading]) -> Vec<Symbol> {
    headings
        .iter()
        .map(|h| {
            let depth = match h.level {
                HeadingLevel::H1 => 1,
                HeadingLevel::H2 => 2,
                HeadingLevel::H3 => 3,
                HeadingLevel::H4 => 4,
                HeadingLevel::H5 => 5,
                HeadingLevel::H6 => 6,
            };
            // Prefix the depth so readers know which header level this is.
            let name = format!("{} {}", "#".repeat(depth), h.title);
            Symbol {
                name,
                kind: SymbolKind::Class,
                byte_start: h.byte_start,
                byte_end: h.byte_end,
                line_start: h.line_start,
                line_end: h.line_end,
            }
        })
        .collect()
}

fn build_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' && i + 1 < source.len() {
            starts.push(i + 1);
        }
    }
    starts
}

fn byte_to_line(line_starts: &[usize], byte: usize) -> usize {
    // 1-based line numbers. Binary search the largest start ≤ byte.
    match line_starts.binary_search(&byte) {
        Ok(i) => i + 1,
        Err(i) => i, // (i is the insertion point; the line containing `byte` is i-1, 0-indexed → i 1-indexed)
    }
}
