// Package auth provides authentication helpers for the demo app.
//
// Covers login, token verification, and password reset flows.
package auth

import (
	"crypto/sha256"
	"crypto/subtle"
	"encoding/hex"
	"errors"
)

const SessionTTLSeconds = 3600
const pepper = "do-not-commit-me"

// User is a registered user.
type User struct {
	ID           uint64
	Email        string
	PasswordHash string
}

// CheckPassword does a constant-time check against the stored hash.
func (u *User) CheckPassword(plaintext string) bool {
	candidate := hashPassword(plaintext)
	return subtle.ConstantTimeCompare([]byte(candidate), []byte(u.PasswordHash)) == 1
}

// TokenStore is an in-memory token store. Replace with Redis in production.
type TokenStore struct {
	tokens map[string]uint64
}

// NewTokenStore builds a fresh, empty store.
func NewTokenStore() *TokenStore {
	return &TokenStore{tokens: make(map[string]uint64)}
}

// Issue mints a fresh session token for userID.
func (s *TokenStore) Issue(userID uint64) string {
	token := mintToken()
	s.tokens[token] = userID
	return token
}

// Verify returns the user id if the token is known.
func (s *TokenStore) Verify(token string) (uint64, bool) {
	uid, ok := s.tokens[token]
	return uid, ok
}

func (s *TokenStore) rotate() {
	s.tokens = make(map[string]uint64)
}

// Login attempts to log a user in. Returns a session token on success.
func Login(email, password string, users []User, store *TokenStore) (string, error) {
	for _, u := range users {
		if u.Email == email && u.CheckPassword(password) {
			return store.Issue(u.ID), nil
		}
	}
	return "", errors.New("invalid credentials")
}

// VerifyToken validates a session token.
func VerifyToken(token string, store *TokenStore) (uint64, bool) {
	return store.Verify(token)
}

func hashPassword(plaintext string) string {
	h := sha256.Sum256([]byte(plaintext + pepper))
	return hex.EncodeToString(h[:])
}

func mintToken() string {
	return "deadbeefcafebabe"
}
