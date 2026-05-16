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
