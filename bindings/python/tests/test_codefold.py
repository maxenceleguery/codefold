"""Tests for the codefold Python bindings."""

from __future__ import annotations

from pathlib import Path

import pytest

import codefold

FIXTURES = (
    Path(__file__).resolve().parent.parent.parent.parent
    / "crates"
    / "codefold-core"
    / "tests"
    / "fixtures"
)


def fixture(name: str) -> str:
    return str(FIXTURES / name)


def test_version_exposed():
    assert codefold.__version__
    # Major.minor.patch
    assert codefold.__version__.count(".") == 2


def test_full_level_python():
    r = codefold.read(fixture("python/auth.py"), level="full")
    assert r.language == "python"
    assert r.tokens_est > 0
    assert "user = next(" in r.content
    assert r.hidden_ranges == []


def test_signatures_level_python_hides_bodies():
    r = codefold.read(fixture("python/auth.py"), level="signatures")
    assert "def login" in r.content
    assert "user = next(" not in r.content
    assert len(r.symbols) > 0


def test_public_level_python_filters_underscore():
    r = codefold.read(fixture("python/auth.py"), level="public")
    assert "def login" in r.content
    assert "def _hash_password" not in r.content
    assert "_PEPPER" not in r.content


def test_bodies_level_python():
    r = codefold.read(fixture("python/auth.py"), level="bodies")
    assert "user = next(" in r.content
    # Nested function body collapsed
    assert "u.email == email" not in r.content


def test_focus_parameter_python():
    r = codefold.read(
        fixture("python/auth.py"),
        level="signatures",
        focus=["login"],
    )
    assert "user = next(" in r.content
    assert "secrets.compare_digest" not in r.content


def test_focus_with_class_name_python():
    r = codefold.read(fixture("python/auth.py"), level="signatures", focus=["User"])
    # User's method bodies should be visible
    assert "secrets.compare_digest" in r.content
    # TokenStore methods stay hidden
    assert "secrets.token_urlsafe" not in r.content


def test_typescript():
    r = codefold.read(fixture("typescript/auth.ts"), level="signatures")
    assert r.language == "typescript"
    assert "class TokenStore" in r.content


def test_rust():
    r = codefold.read(fixture("rust/auth.rs"), level="public")
    assert r.language == "rust"
    assert "pub fn login(" in r.content
    assert "fn hash_password(" not in r.content


def test_symbol_fields():
    r = codefold.read(fixture("python/auth.py"), level="signatures")
    login = next(s for s in r.symbols if s.name == "login")
    assert login.kind == "function"
    assert login.line_start > 0
    assert login.line_end >= login.line_start

    user = next(s for s in r.symbols if s.name == "User")
    assert user.kind == "class"


def test_repr():
    r = codefold.read(fixture("python/auth.py"), level="full")
    assert "FoldResult" in repr(r)
    assert "python" in repr(r)


def test_unsupported_extension_raises_value_error(tmp_path):
    p = tmp_path / "unknown.xyz"
    p.write_text("hello")
    with pytest.raises(ValueError, match="unsupported language"):
        codefold.read(str(p), level="signatures")


def test_invalid_level_raises_value_error():
    with pytest.raises(ValueError, match="unknown level"):
        codefold.read(fixture("python/auth.py"), level="quantum")


def test_missing_file_raises_file_not_found():
    with pytest.raises(FileNotFoundError):
        codefold.read("/nonexistent/path.py", level="signatures")


def test_default_level_is_signatures():
    r = codefold.read(fixture("python/auth.py"))
    assert "def login" in r.content
    assert "user = next(" not in r.content


def test_level_aliases():
    r1 = codefold.read(fixture("python/auth.py"), level="sig")
    r2 = codefold.read(fixture("python/auth.py"), level="signatures")
    assert r1.content == r2.content

    r3 = codefold.read(fixture("python/auth.py"), level="pub")
    r4 = codefold.read(fixture("python/auth.py"), level="public")
    assert r3.content == r4.content
