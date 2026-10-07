use codefold_core::{read_source, Language, Level, Options};

#[test]
fn parses_language_names_and_extensions() {
    for (s, lang) in [
        ("python", Language::Python),
        ("py", Language::Python),
        ("TypeScript", Language::TypeScript),
        ("ts", Language::TypeScript),
        ("tsx", Language::TypeScriptTsx),
        ("jsx", Language::TypeScriptTsx),
        ("rust", Language::Rust),
        ("rs", Language::Rust),
        ("go", Language::Go),
        ("markdown", Language::Markdown),
        ("md", Language::Markdown),
    ] {
        assert_eq!(s.parse::<Language>().unwrap(), lang, "{s}");
    }
}

#[test]
fn unknown_language_name_errors() {
    let err = "cobol".parse::<Language>().unwrap_err();
    assert!(err.to_string().contains("cobol"));
}

#[test]
fn name_round_trips_through_from_str() {
    for lang in [
        Language::Python,
        Language::TypeScript,
        Language::TypeScriptTsx,
        Language::Rust,
        Language::Go,
        Language::Markdown,
    ] {
        assert_eq!(lang.name().parse::<Language>().unwrap(), lang);
    }
}

#[test]
fn read_source_with_parsed_language() {
    let lang: Language = "python".parse().unwrap();
    let r = read_source(
        "def f(x):\n    return x + 1\n",
        lang,
        Options::new(Level::Signatures),
    )
    .unwrap();
    assert!(r.content.contains("def f(x):"));
    assert!(!r.content.contains("return x + 1"));
}
