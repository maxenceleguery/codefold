use std::collections::HashSet;

use tree_sitter::Node;

use crate::result::{Symbol, SymbolKind};
use crate::Level;

pub fn render(
    source: &str,
    tree: &tree_sitter::Tree,
    level: Level,
    focus: &[String],
) -> RenderOutput {
    let base_mode = match level {
        Level::Signatures => Mode::Signatures,
        Level::Bodies => Mode::Bodies,
        Level::Full => Mode::Bodies, // unreachable for non-Full callers; Full handled upstream
    };
    let mut r = Renderer::new(source, base_mode, focus.iter().cloned().collect());
    r.render_module(tree.root_node());
    RenderOutput {
        content: r.out,
        symbols: r.symbols,
        hidden_ranges: r.hidden,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Signatures,
    Bodies,
}

pub struct RenderOutput {
    pub content: String,
    pub symbols: Vec<Symbol>,
    pub hidden_ranges: Vec<(usize, usize)>,
}

struct Renderer<'a> {
    source: &'a str,
    base_mode: Mode,
    focus: HashSet<String>,
    in_focused_class: bool,
    out: String,
    symbols: Vec<Symbol>,
    hidden: Vec<(usize, usize)>,
}

impl<'a> Renderer<'a> {
    fn new(source: &'a str, base_mode: Mode, focus: HashSet<String>) -> Self {
        Self {
            source,
            base_mode,
            focus,
            in_focused_class: false,
            out: String::new(),
            symbols: Vec::new(),
            hidden: Vec::new(),
        }
    }

    /// Effective mode for a symbol named `name`. Focus elevates Signatures → Bodies.
    fn mode_for(&self, name: &str) -> Mode {
        if self.in_focused_class || self.focus.contains(name) {
            Mode::Bodies
        } else {
            self.base_mode
        }
    }

    fn emit_slice(&mut self, start: usize, end: usize) {
        if let Some(s) = self.source.get(start..end) {
            self.out.push_str(s);
        }
    }

    fn hide(&mut self, start: usize, end: usize) {
        if end > start {
            self.hidden.push((start, end));
        }
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        self.source.get(start..end).unwrap_or("")
    }

    fn render_module(&mut self, root: Node<'a>) {
        let mut cursor = root.walk();
        let children: Vec<Node<'a>> = root.children(&mut cursor).collect();

        let mut prev_end = 0usize;
        for child in &children {
            if child.start_byte() > prev_end {
                self.emit_slice(prev_end, child.start_byte());
            }
            self.render_top_level(child);
            prev_end = child.end_byte();
        }

        if prev_end < self.source.len() {
            self.emit_slice(prev_end, self.source.len());
        }
    }

    fn render_top_level(&mut self, node: &Node<'a>) {
        match node.kind() {
            "import_statement" | "import_from_statement" | "future_import_statement" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "expression_statement" => {
                // Module-level expression (docstrings, etc.) — keep verbatim.
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "assignment" | "augmented_assignment" | "type_alias_statement" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "comment" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "function_definition" => {
                self.render_function_def(node, SymbolKind::Function);
            }
            "decorated_definition" => {
                self.render_decorated(node, SymbolKind::Function);
            }
            "class_definition" => {
                self.render_class_def(node);
            }
            _ => {
                self.hide(node.start_byte(), node.end_byte());
            }
        }
    }

    fn render_decorated(&mut self, node: &Node<'a>, inner_default_kind: SymbolKind) {
        let mut cursor = node.walk();
        let children: Vec<Node<'a>> = node.children(&mut cursor).collect();

        let mut prev_end = node.start_byte();
        for child in &children {
            if child.start_byte() > prev_end {
                self.emit_slice(prev_end, child.start_byte());
            }
            match child.kind() {
                "decorator" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
                "function_definition" => {
                    self.render_function_def(child, inner_default_kind);
                }
                "class_definition" => {
                    self.render_class_def(child);
                }
                _ => {}
            }
            prev_end = child.end_byte();
        }
    }

    fn render_function_def(&mut self, node: &Node<'a>, kind: SymbolKind) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
            .unwrap_or_default();

        self.symbols.push(Symbol {
            name,
            kind,
            byte_start: node.start_byte(),
            byte_end: node.end_byte(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
        });

        let body = node.child_by_field_name("body");
        let Some(body) = body else {
            self.emit_slice(node.start_byte(), node.end_byte());
            return;
        };

        // Emit everything before the body verbatim (decorators, def line, params, return type, colon).
        self.emit_slice(node.start_byte(), body.start_byte());

        let effective_mode = self.mode_for(
            &node
                .child_by_field_name("name")
                .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
                .unwrap_or_default(),
        );

        if effective_mode == Mode::Bodies {
            self.emit_body_with_nested_collapsed(body);
            return;
        }

        let body_indent = indent_of_first_line(self.source, &body);
        let pad = " ".repeat(body_indent);

        if let Some(ds) = leading_docstring(&body) {
            // Keep docstring (skip leading whitespace inside the body), summarized.
            self.emit_slice(body.start_byte(), ds.start_byte());
            self.emit_docstring_summary(&ds);
            self.hide(ds.end_byte(), body.end_byte());
            self.out.push('\n');
            self.out.push_str(&pad);
            self.out.push_str("...");
        } else {
            // No docstring: replace whole body with `<indent>...`
            self.out.push('\n');
            self.out.push_str(&pad);
            self.out.push_str("...");
            self.hide(body.start_byte(), body.end_byte());
        }
    }

    /// Emit a function body verbatim from source, but for any function defined
    /// *inside* the body, collapse its body to `<indent>...`.
    fn emit_body_with_nested_collapsed(&mut self, body: Node<'a>) {
        let mut nested: Vec<Node<'a>> = Vec::new();
        collect_outermost_nested_fn_bodies(body, &mut nested);
        nested.sort_by_key(|n| n.start_byte());

        let mut cur = body.start_byte();
        for inner_body in nested {
            // Emit source up to the start of the inner body (so the inner def's signature is kept).
            self.emit_slice(cur, inner_body.start_byte());
            // Collapse the inner body.
            let indent = indent_of_first_line(self.source, &inner_body);
            self.out.push('\n');
            self.out.push_str(&" ".repeat(indent));
            self.out.push_str("...");
            self.hide(inner_body.start_byte(), inner_body.end_byte());
            cur = inner_body.end_byte();
        }
        self.emit_slice(cur, body.end_byte());
    }

    fn emit_docstring_summary(&mut self, ds: &Node<'a>) {
        let raw_len = ds.end_byte() - ds.start_byte();
        let summary = summarize_docstring(self.slice(ds.start_byte(), ds.end_byte()));
        if summary.len() < raw_len {
            self.out.push_str(&summary);
            // Whole original docstring range is elided; the summary in `out` is rendered content.
            self.hide(ds.start_byte(), ds.end_byte());
        } else {
            self.emit_slice(ds.start_byte(), ds.end_byte());
        }
    }

    fn render_class_def(&mut self, node: &Node<'a>) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
            .unwrap_or_default();

        self.symbols.push(Symbol {
            name: name.clone(),
            kind: SymbolKind::Class,
            byte_start: node.start_byte(),
            byte_end: node.end_byte(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
        });

        let body = node.child_by_field_name("body");
        let Some(body) = body else {
            self.emit_slice(node.start_byte(), node.end_byte());
            return;
        };

        // Emit `class Name(bases):` (everything before body).
        self.emit_slice(node.start_byte(), body.start_byte());

        // If the class itself is focused, all methods get Bodies mode.
        let was_focused = self.in_focused_class;
        if self.focus.contains(&name) {
            self.in_focused_class = true;
        }

        let mut cursor = body.walk();
        let children: Vec<Node<'a>> = body.children(&mut cursor).collect();

        let mut prev_end = body.start_byte();
        let mut emitted_any_member = false;

        for child in &children {
            if child.start_byte() > prev_end {
                self.emit_slice(prev_end, child.start_byte());
            }
            match child.kind() {
                "expression_statement" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                    emitted_any_member = true;
                }
                "assignment" | "augmented_assignment" | "type_alias_statement" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                    emitted_any_member = true;
                }
                "comment" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
                "function_definition" => {
                    self.render_function_def(child, SymbolKind::Method);
                    emitted_any_member = true;
                }
                "decorated_definition" => {
                    self.render_decorated(child, SymbolKind::Method);
                    emitted_any_member = true;
                }
                _ => {
                    self.hide(child.start_byte(), child.end_byte());
                }
            }
            prev_end = child.end_byte();
        }

        if prev_end < body.end_byte() {
            self.emit_slice(prev_end, body.end_byte());
        }

        if !emitted_any_member {
            // Empty class body in the rendered view — emit a placeholder.
            let pad = " ".repeat(indent_of_first_line(self.source, &body));
            self.out.push('\n');
            self.out.push_str(&pad);
            self.out.push_str("...");
        }

        self.in_focused_class = was_focused;
    }
}

/// Walk `node`'s descendants and collect the `body` field of every
/// `function_definition` found, but do not recurse into a function once it's
/// added (its inner defs are already inside the collapsed range).
fn collect_outermost_nested_fn_bodies<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(body) = child.child_by_field_name("body") {
                    out.push(body);
                }
                // Do not recurse — body is fully collapsed.
            }
            "decorated_definition" => {
                let mut found = false;
                let mut c = child.walk();
                for grand in child.children(&mut c) {
                    if grand.kind() == "function_definition" {
                        if let Some(body) = grand.child_by_field_name("body") {
                            out.push(body);
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    collect_outermost_nested_fn_bodies(child, out);
                }
            }
            _ => collect_outermost_nested_fn_bodies(child, out),
        }
    }
}

fn indent_of_first_line(_source: &str, body: &Node) -> usize {
    // tree-sitter `block` nodes start at the first non-whitespace character of
    // the first statement. Its column is the body's indent.
    body.start_position().column
}

/// Compress a Python docstring node's raw text to its first non-empty line,
/// keeping the original triple-quote style. Returns the original string if it's
/// already a single line or shorter than the summarized form.
fn summarize_docstring(raw: &str) -> String {
    let (quote, inner) = strip_quotes(raw);
    let trimmed = inner.trim_start_matches(['\n', '\r']);
    let first_line = trimmed.lines().next().unwrap_or("").trim_end();
    if first_line.is_empty() {
        return raw.to_string();
    }
    let summary = format!("{quote}{first_line}{quote}");
    if summary.len() >= raw.len() {
        raw.to_string()
    } else {
        summary
    }
}

/// Extract the triple- or single-quote delimiter and the inner content of a Python
/// string literal. Falls back to returning the whole input as inner if no known
/// delimiter is found.
fn strip_quotes(raw: &str) -> (&'static str, &str) {
    for delim in ["\"\"\"", "'''", "\"", "'"] {
        if raw.starts_with(delim) && raw.ends_with(delim) && raw.len() >= 2 * delim.len() {
            let inner = &raw[delim.len()..raw.len() - delim.len()];
            // Return a 'static str by matching on the known delimiters.
            let d: &'static str = match delim {
                "\"\"\"" => "\"\"\"",
                "'''" => "'''",
                "\"" => "\"",
                "'" => "'",
                _ => unreachable!(),
            };
            return (d, inner);
        }
    }
    ("\"\"\"", raw)
}

fn leading_docstring<'a>(body: &Node<'a>) -> Option<Node<'a>> {
    let first = body.named_child(0)?;
    if first.kind() != "expression_statement" {
        return None;
    }
    let inner = first.named_child(0)?;
    if inner.kind() == "string" {
        Some(first)
    } else {
        None
    }
}
