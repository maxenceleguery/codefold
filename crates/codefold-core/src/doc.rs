//! Doc-comment summaries for the `signatures`/`public` levels.
//!
//! Each run of line comments (`///`, `//!`, `//`) keeps its first paragraph;
//! each block comment (`/** ... */`) keeps the lines before its first blank
//! or `@tag` line, plus the closing `*/`. Mirrors Python's docstring
//! summaries. Comments documenting a focused symbol stay in full.

use std::collections::HashSet;

use tree_sitter::{Node, Tree};

/// Sorted, non-overlapping byte ranges of doc text to elide.
pub fn tails(source: &str, tree: &Tree, focus: &HashSet<String>) -> Vec<(usize, usize)> {
    let mut comments = Vec::new();
    collect_comments(tree.root_node(), &mut comments);

    let mut out = Vec::new();
    let mut i = 0;
    while i < comments.len() {
        let first = comments[i];
        if !text(source, &first).starts_with("//") {
            if !documents_focus(source, &first, focus) {
                out.extend(block_tail(source, &first));
            }
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < comments.len() && same_run(source, &comments[j - 1], &comments[j]) {
            j += 1;
        }
        let run = &comments[i..j];
        if !documents_focus(source, &run[run.len() - 1], focus) {
            out.extend(line_run_tail(source, run));
        }
        i = j;
    }
    out
}

/// Append `source[start..end]` to `out`, skipping `elide` ranges (recorded in `hidden`).
pub fn emit(
    source: &str,
    elide: &[(usize, usize)],
    start: usize,
    end: usize,
    out: &mut String,
    hidden: &mut Vec<(usize, usize)>,
) {
    let mut pos = start;
    let first = elide.partition_point(|r| r.1 <= start);
    for &(a, b) in elide[first..].iter().take_while(|r| r.0 < end) {
        let (a, b) = (a.max(pos), b.min(end));
        out.push_str(source.get(pos..a).unwrap_or(""));
        match hidden.last_mut() {
            Some(last) if last.1 == a => last.1 = b,
            _ => hidden.push((a, b)),
        }
        pos = b;
    }
    out.push_str(source.get(pos..end).unwrap_or(""));
}

fn collect_comments<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind().ends_with("comment") {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_comments(child, out);
    }
}

fn text<'s>(source: &'s str, node: &Node) -> &'s str {
    &source[node.start_byte()..node.end_byte()]
}

/// Two line comments on consecutive lines with only whitespace between.
fn same_run(source: &str, prev: &Node, next: &Node) -> bool {
    let between = &source[prev.end_byte()..next.start_byte()];
    let newlines = between.matches('\n').count() + text(source, prev).ends_with('\n') as usize;
    text(source, next).starts_with("//") && between.trim().is_empty() && newlines == 1
}

/// From the first blank comment line after the summary to the end of the run.
fn line_run_tail(source: &str, run: &[Node]) -> Option<(usize, usize)> {
    let blank = |n: &Node| {
        text(source, n)
            .trim_start_matches('/')
            .trim_start_matches('!')
            .trim()
            .is_empty()
    };
    let k = (1..run.len()).find(|&k| blank(&run[k]))?;
    run[k + 1..].iter().find(|n| !blank(n))?;
    let start = line_start(source, run[k].start_byte());
    let mut end = run[run.len() - 1].end_byte();
    if !source[..end].ends_with('\n') {
        end += source[end..].find('\n').map_or(0, |p| p + 1);
    }
    Some((start, end))
}

/// From the first blank or `@tag` line after the summary up to the closing `*/` line.
fn block_tail(source: &str, node: &Node) -> Option<(usize, usize)> {
    let body = text(source, node);
    let lines: Vec<&str> = body.split_inclusive('\n').collect();
    let last = lines.len().checked_sub(1)?;
    if lines[last].trim() != "*/" {
        return None;
    }
    let content = |l: &str| {
        l.trim()
            .trim_start_matches("/**")
            .trim_start_matches("/*")
            .trim_start_matches('*')
            .trim()
            .to_string()
    };
    let summary = (0..last).find(|&k| !content(lines[k]).is_empty())?;
    let k = (summary + 1..last).find(|&k| {
        let c = content(lines[k]);
        c.is_empty() || c.starts_with('@')
    })?;
    let offset = |k: usize| node.start_byte() + lines[..k].iter().map(|l| l.len()).sum::<usize>();
    Some((offset(k), offset(last)))
}

fn line_start(source: &str, byte: usize) -> usize {
    source[..byte].rfind('\n').map_or(0, |p| p + 1)
}

/// Whether the declaration right after `comment` is a focused symbol.
fn documents_focus(source: &str, comment: &Node, focus: &HashSet<String>) -> bool {
    if focus.is_empty() {
        return false;
    }
    let mut next = comment.next_named_sibling();
    while let Some(n) = next {
        if n.kind().ends_with("comment") || n.kind() == "attribute_item" {
            next = n.next_named_sibling();
            continue;
        }
        let decl = n.child_by_field_name("declaration").unwrap_or(n);
        return decl
            .child_by_field_name("name")
            .is_some_and(|name| focus.contains(text(source, &name)));
    }
    false
}
