//! Authentication helpers for the demo app.
//!
//! Covers login, token verification, and password reset flows.

use std::collections::HashMap;

pub const SESSION_TTL_SECONDS: u64 = 3600;
const PEPPER: &str = "do-not-commit-me";

/// A registered user.
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub email: String,
    pub password_hash: String,
}

impl User {
    /// Constant-time check against the stored hash.
    pub fn check_password(&self, plaintext: &str) -> bool {
        let candidate = hash_password(plaintext);
        constant_time_eq(candidate.as_bytes(), self.password_hash.as_bytes())
    }
}

/// In-memory token store. Replace with Redis in production.
pub struct TokenStore {
    tokens: HashMap<String, u64>,
}

impl TokenStore {
    /// Build a fresh, empty store.
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Mint a fresh session token for ``user_id``.
    pub fn issue(&mut self, user_id: u64) -> String {
        let token = mint_token();
        self.tokens.insert(token.clone(), user_id);
        token
    }

    /// Return the user id if the token is known, else None.
    pub fn verify(&self, token: &str) -> Option<u64> {
        self.tokens.get(token).copied()
    }

    fn rotate(&mut self) {
        self.tokens.clear();
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Attempt to log a user in. Returns a session token on success.
pub fn login(
    email: &str,
    password: &str,
    users: &[User],
    store: &mut TokenStore,
) -> Option<String> {
    fn matches(u: &User, email: &str, password: &str) -> bool {
        u.email == email && u.check_password(password)
    }

    let user = users.iter().find(|u| matches(u, email, password))?;
    Some(store.issue(user.id))
}

/// Validate a session token.
pub fn verify_token(token: &str, store: &TokenStore) -> Option<u64> {
    store.verify(token)
}

fn hash_password(plaintext: &str) -> String {
    let mut s = String::new();
    s.push_str(plaintext);
    s.push_str(PEPPER);
    format!("{:x}", simple_hash(s.as_bytes()))
}

fn mint_token() -> String {
    let n: u64 = 0xdead_beef_cafe_babe;
    format!("{n:016x}")
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn simple_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}
