-- API Tokens table for service authentication (PostgreSQL version)
-- Phase 6.5.3: Token Management System

CREATE TABLE IF NOT EXISTS api_tokens (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    scopes TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_by TEXT
);

-- Index for fast token lookup by hash
CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash);

-- Index for listing active (non-revoked) tokens
CREATE INDEX IF NOT EXISTS idx_api_tokens_revoked ON api_tokens(revoked_at);
