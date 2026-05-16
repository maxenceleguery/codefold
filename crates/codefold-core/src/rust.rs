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
    r.render_source_file(tree.root_node());
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
    out: String,
    symbols: Vec<Symbol>,
    hidden: Vec<(usize, usize)>,
}

impl<'a> Renderer<'a> {
    fn new(source: &'a str, base_mode: Mode, public_only: bool, focus: HashSet<String>) -> Self {
        Self {
            source,
            base_mode,
            public_only,
            focus,
            out: String::new(),
            symbols: Vec::new(),
            hidden: Vec::new(),
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

    fn mode_for(&self, name: &str) -> Mode {
        if self.focus.contains(name) {
            Mode::Bodies
        } else {
            self.base_mode
        }
    }

    fn render_source_file(&mut self, root: Node<'a>) {
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
            "use_declaration"
            | "extern_crate_declaration"
            | "attribute_item"
            | "inner_attribute_item"
            | "line_comment"
            | "block_comment" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "const_item" | "static_item" | "type_item" => {
                if self.public_only && !has_pub_visibility(node) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.emit_slice(node.start_byte(), node.end_byte());
                }
            }
            "struct_item" | "enum_item" | "union_item" => {
                if self.public_only && !has_pub_visibility(node) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.emit_item_with_symbol(node, SymbolKind::Class);
                }
            }
            "trait_item" => {
                if self.public_only && !has_pub_visibility(node) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.render_trait_or_impl(
                        node, /* is_trait */ true, /* filter_inner */ false,
                    );
                }
            }
            "impl_item" => {
                // For trait impls (`impl Trait for Type`) inner methods are public-by-the-trait,
                // so don't filter them. For inherent impls, filter by `pub` in Public mode.
                let is_trait_impl = node.child_by_field_name("trait").is_some();
                let filter_inner = self.public_only && !is_trait_impl;
                self.render_trait_or_impl(node, /* is_trait */ false, filter_inner);
            }
            "function_item" => {
                if self.public_only && !has_pub_visibility(node) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.render_function_item(node, SymbolKind::Function);
                }
            }
            "mod_item" => {
                if self.public_only && !has_pub_visibility(node) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.emit_slice(node.start_byte(), node.end_byte());
                }
            }
            "macro_definition" | "macro_invocation" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            _ => {
                self.hide(node.start_byte(), node.end_byte());
            }
        }
    }

    fn emit_item_with_symbol(&mut self, node: &Node<'a>, kind: SymbolKind) {
        if let Some(name) = node
            .child_by_field_name("name")
            .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
        {
            self.symbols.push(Symbol {
                name,
                kind,
                byte_start: node.start_byte(),
                byte_end: node.end_byte(),
                line_start: node.start_position().row + 1,
                line_end: node.end_position().row + 1,
            });
        }
        self.emit_slice(node.start_byte(), node.end_byte());
    }

    fn render_function_item(&mut self, node: &Node<'a>, kind: SymbolKind) {
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

        // Trait function items may have no body (abstract).
        let body = node.child_by_field_name("body");
        let Some(body) = body else {
            self.emit_slice(node.start_byte(), node.end_byte());
            return;
        };

        self.emit_slice(node.start_byte(), body.start_byte());

        let mode = self.mode_for(&name);
        if mode == Mode::Bodies {
            self.emit_body_with_nested_collapsed(body);
        } else {
            self.out.push_str("{ /* ... */ }");
            self.hide(body.start_byte(), body.end_byte());
        }
    }

    fn render_trait_or_impl(&mut self, node: &Node<'a>, is_trait: bool, filter_inner: bool) {
        // For traits, push a symbol with the trait name (kind Class — close enough).
        if is_trait {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = self
                    .slice(name_node.start_byte(), name_node.end_byte())
                    .to_string();
                self.symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Class,
                    byte_start: node.start_byte(),
                    byte_end: node.end_byte(),
                    line_start: node.start_position().row + 1,
                    line_end: node.end_position().row + 1,
                });
            }
        }

        let body = node.child_by_field_name("body");
        let Some(body) = body else {
            self.emit_slice(node.start_byte(), node.end_byte());
            return;
        };

        self.emit_slice(node.start_byte(), body.start_byte());

        let mut cursor = body.walk();
        let children: Vec<Node<'a>> = body.children(&mut cursor).collect();
        let mut prev_end = body.start_byte();

        for child in &children {
            if child.start_byte() > prev_end {
                self.emit_slice(prev_end, child.start_byte());
            }
            match child.kind() {
                "function_item" => {
                    if filter_inner && !has_pub_visibility(child) {
                        self.hide(child.start_byte(), child.end_byte());
                    } else {
                        self.render_function_item(child, SymbolKind::Method);
                    }
                }
                "const_item" | "type_item" | "associated_type" => {
                    if filter_inner && !has_pub_visibility(child) {
                        self.hide(child.start_byte(), child.end_byte());
                    } else {
                        self.emit_slice(child.start_byte(), child.end_byte());
                    }
                }
                "attribute_item" | "inner_attribute_item" | "line_comment" | "block_comment" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
                "{" | "}" => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
                _ => {
                    self.emit_slice(child.start_byte(), child.end_byte());
                }
            }
            prev_end = child.end_byte();
        }

        if prev_end < body.end_byte() {
            self.emit_slice(prev_end, body.end_byte());
        }
    }

    /// Emit a function body verbatim, collapsing the bodies of any nested
    /// function items inside.
    fn emit_body_with_nested_collapsed(&mut self, body: Node<'a>) {
        let mut nested: Vec<Node<'a>> = Vec::new();
        collect_outermost_nested_fn_bodies(body, &mut nested);
        nested.sort_by_key(|n| n.start_byte());

        let mut cur = body.start_byte();
        for inner_body in nested {
            self.emit_slice(cur, inner_body.start_byte());
            self.out.push_str("{ /* ... */ }");
            self.hide(inner_body.start_byte(), inner_body.end_byte());
            cur = inner_body.end_byte();
        }
        self.emit_slice(cur, body.end_byte());
    }
}

fn has_pub_visibility(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return true;
        }
        // Stop scanning once we get past the modifier zone.
        if matches!(
            child.kind(),
            "fn" | "struct"
                | "enum"
                | "trait"
                | "impl"
                | "const"
                | "static"
                | "type"
                | "mod"
                | "union"
        ) {
            return false;
        }
    }
    false
}

fn collect_outermost_nested_fn_bodies<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_item" {
            if let Some(body) = child.child_by_field_name("body") {
                out.push(body);
            }
            // Do not recurse — body is collapsed.
        } else {
            collect_outermost_nested_fn_bodies(child, out);
        }
    }
}
