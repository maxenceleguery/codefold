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
        // Use named_children: tree-sitter-go exposes statement terminators
        // (`\n` / `;`) as anonymous children of source_file. Including them
        // here would zero out the gap between real declarations and merge
        // them onto a single line.
        let mut cursor = root.walk();
        let children: Vec<Node<'a>> = root.named_children(&mut cursor).collect();
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
            "package_clause" | "import_declaration" | "comment" => {
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "const_declaration" | "var_declaration" => {
                // Single-spec declarations: filter by name. Grouped (parenthesized) declarations:
                // keep verbatim — they may mix public and private specs.
                let single_name = single_spec_name(node, self.source);
                if let Some(name) = single_name {
                    if self.public_only && !is_public_name(&name) {
                        self.hide(node.start_byte(), node.end_byte());
                    } else {
                        self.emit_slice(node.start_byte(), node.end_byte());
                    }
                } else {
                    self.emit_slice(node.start_byte(), node.end_byte());
                }
            }
            "type_declaration" => {
                // Always emit type decls verbatim. Push a symbol for each named type.
                self.push_type_symbols(node);
                self.emit_slice(node.start_byte(), node.end_byte());
            }
            "function_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
                    .unwrap_or_default();
                if self.public_only && !is_public_name(&name) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.render_function_like(node, &name, SymbolKind::Function);
                }
            }
            "method_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.slice(n.start_byte(), n.end_byte()).to_string())
                    .unwrap_or_default();
                if self.public_only && !is_public_name(&name) {
                    self.hide(node.start_byte(), node.end_byte());
                } else {
                    self.render_function_like(node, &name, SymbolKind::Method);
                }
            }
            _ => {
                self.hide(node.start_byte(), node.end_byte());
            }
        }
    }

    fn render_function_like(&mut self, node: &Node<'a>, name: &str, kind: SymbolKind) {
        self.symbols.push(Symbol {
            name: name.to_string(),
            kind,
            byte_start: node.start_byte(),
            byte_end: node.end_byte(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
        });

        let body = node.child_by_field_name("body");
        let Some(body) = body else {
            // Abstract function — interface methods don't go through function_declaration,
            // but be defensive: emit verbatim if there is no body.
            self.emit_slice(node.start_byte(), node.end_byte());
            return;
        };

        self.emit_slice(node.start_byte(), body.start_byte());

        let mode = self.mode_for(name);
        if mode == Mode::Bodies {
            self.emit_body_with_nested_collapsed(body);
        } else {
            self.out.push_str("{ /* ... */ }");
            self.hide(body.start_byte(), body.end_byte());
        }
    }

    fn push_type_symbols(&mut self, node: &Node<'a>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = self
                        .slice(name_node.start_byte(), name_node.end_byte())
                        .to_string();
                    self.symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Class,
                        byte_start: child.start_byte(),
                        byte_end: child.end_byte(),
                        line_start: child.start_position().row + 1,
                        line_end: child.end_position().row + 1,
                    });
                }
            }
        }
    }

    /// Emit a function body verbatim, collapsing the bodies of any nested
    /// function declarations / literals found inside. Go uses anonymous
    /// function literals (`func() { ... }`) far more than named nested funcs;
    /// we leave those alone here and only collapse named declarations.
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

fn is_public_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

/// If `node` is a const/var declaration with exactly one spec, return that
/// spec's first name. Returns None for grouped (parenthesized) declarations
/// or unexpected shapes.
fn single_spec_name(node: &Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let specs: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| matches!(c.kind(), "const_spec" | "var_spec"))
        .collect();
    if specs.len() != 1 {
        return None;
    }
    let name_node = specs[0].child_by_field_name("name")?;
    Some(
        source
            .get(name_node.start_byte()..name_node.end_byte())?
            .to_string(),
    )
}

fn collect_outermost_nested_fn_bodies<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "function_declaration" | "method_declaration") {
            if let Some(body) = child.child_by_field_name("body") {
                out.push(body);
            }
            // Do not recurse — body is collapsed.
        } else {
            collect_outermost_nested_fn_bodies(child, out);
        }
    }
}
