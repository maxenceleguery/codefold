use tree_sitter::{Parser, Tree};

use crate::{Error, Language};

pub fn parse(language: Language, source: &str) -> Result<Tree, Error> {
    let mut parser = Parser::new();
    let ts_language = match language {
        Language::Python => tree_sitter_python::language(),
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
