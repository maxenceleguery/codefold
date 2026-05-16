"""Authentication helpers for the demo app.

This module covers the login, token verification, and password reset flows.
"""

from __future__ import annotations

import hashlib
import secrets
from dataclasses import dataclass
from typing import Optional


SESSION_TTL_SECONDS = 3600
_PEPPER = "do-not-commit-me"


@dataclass
class User:
    """A registered user."""

    id: int
    email: str
    password_hash: str

    def check_password(self, plaintext: str) -> bool:
        """Constant-time check against the stored hash."""
        candidate = _hash_password(plaintext)
        return secrets.compare_digest(candidate, self.password_hash)


class TokenStore:
    """In-memory token store. Replace with Redis in production."""

    def __init__(self) -> None:
        self._tokens: dict[str, int] = {}

    def issue(self, user_id: int) -> str:
        """Mint a fresh session token for ``user_id``."""
        token = secrets.token_urlsafe(32)
        self._tokens[token] = user_id
        return token

    def verify(self, token: str) -> Optional[int]:
        """Return the user id if the token is known, else None."""
        return self._tokens.get(token)

    def _rotate(self) -> None:
        # Internal helper; not part of the public API.
        self._tokens.clear()


def login(email: str, password: str, users: list[User], store: TokenStore) -> Optional[str]:
    """Attempt to log a user in. Returns a session token on success.

    The lookup is intentionally linear; in a real app this would hit the DB.
    """

    def matches(u: User) -> bool:
        return u.email == email and u.check_password(password)

    user = next((u for u in users if matches(u)), None)
    if user is None:
        return None
    return store.issue(user.id)


def verify_token(token: str, store: TokenStore) -> Optional[int]:
    """Validate a session token."""
    return store.verify(token)


def _hash_password(plaintext: str) -> str:
    salted = (plaintext + _PEPPER).encode("utf-8")
    return hashlib.sha256(salted).hexdigest()
