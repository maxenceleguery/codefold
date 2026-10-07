use tree_sitter::{Parser, Tree};

use crate::{Error, Language};

pub fn parse(language: Language, source: &str) -> Result<Tree, Error> {
    let mut parser = Parser::new();
    let ts_language = match language {
        Language::Python => tree_sitter_python::language(),
        Language::TypeScript => tree_sitter_typescript::language_typescript(),
        Language::TypeScriptTsx => tree_sitter_typescript::language_tsx(),
        Language::Rust => tree_sitter_rust::language(),
        Language::Go => tree_sitter_go::language(),
        // Markdown is handled outside tree-sitter (pulldown_cmark in markdown.rs).
        // We never reach this arm because `read_opts` dispatches to markdown
        // before invoking the tree-sitter parser. Return Python's language as
        // a harmless placeholder so the match stays exhaustive.
        Language::Markdown => {
            return Err(Error::Parse {
                path: Default::default(),
            })
        }
    };
    parser
        .set_language(&ts_language)
        .map_err(|_| Error::Parse {
            path: Default::default(),
        })?;
    parser.parse(source, None).ok_or(Error::Parse {
        path: Default::default(),
    })
}
