use std::collections::HashSet;

use tree_sitter::Node;

use crate::result::{Symbol, SymbolKind};
use crate::Level;

pub struct RenderOutput {
    pub content: String,
    pub symbols: Vec<Symbol>,
    pub hidden_ranges: Vec<(usize, usize)>,
}

pub fn render(
    source: &str,
    tree: &tree_sitter::Tree,
    level: Level,
    focus: &[String],
) -> RenderOutput {
    let (base_mode, public_only) = match level {
        Level::Signatures => (Mode::Signatures, false),
        Level::Public => (Mode::Signatures, true),
        Level::Bodies => (Mode::Bodies, false),
        Level::Full => (Mode::Bodies, false),
    };
    let mut r = Renderer::new(
        source,
        base_mode,
        public_only,
        focus.iter().cloned().collect(),
    );
    r.render_program(tree.root_node());
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

struct Renderer<'a> {
    source: &'a str,
    base_mode: Mode,
    public_only: bool,
    focus: HashSet<String>,
    in_focused_class: bool,
    in_export: bool,
    out: String,
    symbols: Vec<Symbol>,
    hidden: Vec<(usize, usize)>,
}

impl<'a> Renderer<'a> {
    fn new(
        source: &'a str,
        base_mode: Mode,
        public_only: bool,
        focus: HashSet<String>,
    ) -> Self {
        Self {
            source,
            base_mode,
            public_only,
            focus,
            in_focused_class: false,
            in_export: false,
            out: String::new(),
            symbols: Vec::new(),
            hidden: Vec::new(),
        }
    }

    fn mode_for(&self, name: &str) -> Mode {
        if self.in_focused_class || self.focus.contains(name) {
            Mode::Bodies
        } else {
            self.base_mode
        }
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        self.source.get(start..end).unwrap_or("")
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

    fn render_program(&mut self, root: Node<'a>) {
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
            "import_statement" | "import_alias" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "comment" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "ambient_declaration" => {
                // Type-only declarations. Keep when exported or not Public-filtered.
                if self.public_only && !self.in_export {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.emit_slice(node.start_byte(), node.end_byte());
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                if self.public_only && !self.in_export {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.emit_slice(node.start_byte(), node.end_byte());
                }
            }
            "expression_statement" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "function_declaration" => {
                if self.public_only && !self.in_export {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.render_function_decl(node, SymbolKind::Function);
                }
            }
            "class_declaration" | "abstract_class_declaration" => {
                if self.public_only && !self.in_export {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.render_class_decl(node);
                }
            }
            "export_statement" => {
                self.render_export_statement(node);
            }
            _ => {
                self.hide(node.start_byte(), node.end_byte());
            }
        }
    }

    fn render_export_statement(&mut self, node: &Node<'a>) {
        // Find the wrapped declaration (or value clause).
        let mut cursor = node.walk();
        let children: Vec<Node<'a>> = node.children(&mut cursor).collect();

        let mut wrapped: Option<Node<'a>> = None;
        for child in &children {
            match child.kind() {
                "function_declaration"
                | "class_declaration"
                | "abstract_class_declaration"
                | "interface_declaration"
                | "type_alias_declaration"
                | "enum_declaration"
                | "lexical_declaration"
                | "variable_declaration" => {
                    wrapped = Some(*child);
                    break;
                }
                _ => {}
            }
        }

        match wrapped {
            Some(inner) => {
                // Emit `export ` prefix (and any modifiers like `default`).
                self.emit_slice(node.start_byte(), inner.start_byte());
                let was_in_export = self.in_export;
                self.in_export = true;
                self.render_top_level(&inner);
                self.in_export = was_in_export;
                // Emit anything after the inner declaration (e.g., trailing semicolons).
                if inner.end_byte() < node.end_byte() {
                    self.emit_slice(inner.end_byte(), node.end_byte());
                }
            }
            None => {
                // `export { X, Y }` / `export * from "..."` etc. — keep verbatim.
                self.emit_slice(node.start_byte(), node.end_byte());
            }
        }
    }

    fn render_function_decl(&mut self, node: &Node<'a>, kind: SymbolKind) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
            .unwrap_or_default();

        self.symbols.push(Symbol {
            name: name.clone(),
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

        self.emit_slice(node.start_byte(), body.start_byte());

        let effective_mode = self.mode_for(&name);
        if effective_mode == Mode::Bodies {
            self.emit_body_with_nested_collapsed(body);
            return;
        }

        // Signatures: replace `{ ... }` with `{ ... }` placeholder.
        self.out.push_str("{ /* ... */ }");
        self.hide(body.start_byte(), body.end_byte());
    }

    fn render_method_def(&mut self, node: &Node<'a>) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
            .unwrap_or_default();

        self.symbols.push(Symbol {
            name: name.clone(),
            kind: SymbolKind::Method,
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

        self.emit_slice(node.start_byte(), body.start_byte());

        let effective_mode = self.mode_for(&name);
        if effective_mode == Mode::Bodies {
            self.emit_body_with_nested_collapsed(body);
            return;
        }

        self.out.push_str("{ /* ... */ }");
        self.hide(body.start_byte(), body.end_byte());
    }

    fn render_class_decl(&mut self, node: &Node<'a>) {
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

        self.emit_slice(node.start_byte(), body.start_byte());

        let was_focused = self.in_focused_class;
        if self.focus.contains(&name) {
            self.in_focused_class = true;
        }

        // body is class_body: `{ members... }`
        let mut cursor = body.walk();
        let children: Vec<Node<'a>> = body.children(&mut cursor).collect();

        let mut prev_end = body.start_byte();
        for child in &children {
            if child.start_byte() > prev_end {
                self.emit_slice(prev_end, child.start_byte());
            }
            match child.kind() {
                "method_definition" => {
                    if self.public_only && is_private_member(child, self.source) {
                        self.hide(child.start_byte(), child.end_byte());
                    } else {
                        self.render_method_def(child);
                    }
                }
                "public_field_definition" | "field_definition" => {
                    if self.public_only && is_private_member(child, self.source) {
                        self.hide(child.start_byte(), child.end_byte());
                    } else {
                        self.emit_slice(child.start_byte(), child.end_byte());
                    }
                }
                "comment" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
                "{" | "}" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
                _ => {
                    // Keep other class-body items verbatim (decorators, semis, etc.).
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
            }
            prev_end = child.end_byte();
        }

        if prev_end < body.end_byte() {
            self.emit_slice(prev_end, body.end_byte());
        }

        self.in_focused_class = was_focused;
    }

    /// Emit a body statement_block verbatim, but for any nested function inside,
    /// collapse its body.
    fn emit_body_with_nested_collapsed(&mut self, body: Node<'a>) {
        let mut nested_bodies: Vec<Node<'a>> = Vec::new();
        collect_outermost_nested_fn_bodies(body, &mut nested_bodies);
        nested_bodies.sort_by_key(|n| n.start_byte());

        let mut cur = body.start_byte();
        for inner_body in nested_bodies {
            self.emit_slice(cur, inner_body.start_byte());
            self.out.push_str("{ /* ... */ }");
            self.hide(inner_body.start_byte(), inner_body.end_byte());
            cur = inner_body.end_byte();
        }
        self.emit_slice(cur, body.end_byte());
    }
}

/// True if a class member has an `accessibility_modifier` child marked
/// `private`. TypeScript also has `protected`, which we treat as private for
/// the Public level (caller asked for public surface only).
fn is_private_member(node: &Node, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "accessibility_modifier" {
            let text = source.get(child.start_byte()..child.end_byte()).unwrap_or("");
            if text == "private" || text == "protected" {
                return true;
            }
        }
    }
    false
}

fn collect_outermost_nested_fn_bodies<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function" => {
                if let Some(body) = child.child_by_field_name("body") {
                    if body.kind() == "statement_block" {
                        out.push(body);
                    }
                }
                // Do not recurse — body is collapsed.
            }
            _ => collect_outermost_nested_fn_bodies(child, out),
        }
    }
}
