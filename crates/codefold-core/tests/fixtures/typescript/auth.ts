/**
 * Authentication helpers for the demo app.
 *
 * Covers login, token verification, and password reset flows.
 */

import { createHash, randomBytes, timingSafeEqual } from "node:crypto";

export const SESSION_TTL_SECONDS = 3600;
const _PEPPER = "do-not-commit-me";

export interface User {
  id: number;
  email: string;
  passwordHash: string;
}

export class TokenStore {
  /** In-memory token store. Replace with Redis in production. */
  private tokens: Map<string, number> = new Map();

  /** Mint a fresh session token for the given user id. */
  issue(userId: number): string {
    const token = randomBytes(32).toString("base64url");
    this.tokens.set(token, userId);
    return token;
  }

  /** Return the user id if the token is known, else null. */
  verify(token: string): number | null {
    return this.tokens.get(token) ?? null;
  }

  private _rotate(): void {
    this.tokens.clear();
  }
}

/** Constant-time check against the stored hash. */
export function checkPassword(plaintext: string, expected: string): boolean {
  const candidate = hashPassword(plaintext);
  if (candidate.length !== expected.length) return false;
  return timingSafeEqual(Buffer.from(candidate), Buffer.from(expected));
}

/**
 * Attempt to log a user in. Returns a session token on success.
 *
 * The lookup is intentionally linear; in a real app this would hit the DB.
 */
export function login(
  email: string,
  password: string,
  users: User[],
  store: TokenStore
): string | null {
  function matches(u: User): boolean {
    return u.email === email && checkPassword(password, u.passwordHash);
  }

  const user = users.find(matches);
  if (user === undefined) return null;
  return store.issue(user.id);
}

/** Validate a session token. */
export function verifyToken(token: string, store: TokenStore): number | null {
  return store.verify(token);
}

function hashPassword(plaintext: string): string {
  return createHash("sha256")
    .update(plaintext + _PEPPER, "utf-8")
    .digest("hex");
}
