-- Talos PostgreSQL Schema
-- Run this on a fresh database or use with docker-compose
--
-- Manual setup:
--   psql -U postgres -c "CREATE DATABASE talos;"
--   psql -U postgres -d talos -f migrations/init_postgres.sql

-- =============================================================================
-- Licenses Table
-- =============================================================================
CREATE TABLE IF NOT EXISTS licenses (
    license_id      TEXT PRIMARY KEY,
    client_id       TEXT,
    status          TEXT NOT NULL,
    features        TEXT,
    issued_at       TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at      TIMESTAMP WITH TIME ZONE,
    hardware_id     TEXT,
    signature       TEXT,
    last_heartbeat  TIMESTAMP WITH TIME ZONE,

    -- Organization fields
    org_id          TEXT,
    org_name        TEXT,

    -- Human-readable license key
    license_key     TEXT UNIQUE,

    -- Tier system
    tier            TEXT,

    -- Extended hardware binding
    device_name     TEXT,
    device_info     TEXT,
    bound_at        TIMESTAMP WITH TIME ZONE,
    last_seen_at    TIMESTAMP WITH TIME ZONE,

    -- Status lifecycle
    suspended_at    TIMESTAMP WITH TIME ZONE,
    revoked_at      TIMESTAMP WITH TIME ZONE,
    revoke_reason   TEXT,
    grace_period_ends_at TIMESTAMP WITH TIME ZONE,
    suspension_message TEXT,

    -- Blacklist
    is_blacklisted  BOOLEAN DEFAULT FALSE,
    blacklisted_at  TIMESTAMP WITH TIME ZONE,
    blacklist_reason TEXT,

    -- Metadata (JSON)
    metadata        TEXT,

    -- Quota/Usage tracking
    bandwidth_used_bytes BIGINT DEFAULT 0,
    bandwidth_limit_bytes BIGINT,
    quota_exceeded  BOOLEAN DEFAULT FALSE
);

-- Indexes for licenses
CREATE INDEX IF NOT EXISTS idx_licenses_org_id ON licenses(org_id);
CREATE INDEX IF NOT EXISTS idx_licenses_license_key ON licenses(license_key);
CREATE INDEX IF NOT EXISTS idx_licenses_hardware_id ON licenses(hardware_id);
CREATE INDEX IF NOT EXISTS idx_licenses_status ON licenses(status);
CREATE INDEX IF NOT EXISTS idx_licenses_expires_at ON licenses(expires_at);
CREATE INDEX IF NOT EXISTS idx_licenses_tier ON licenses(tier);
CREATE INDEX IF NOT EXISTS idx_licenses_quota_exceeded ON licenses(quota_exceeded);

-- =============================================================================
-- License Binding History Table
-- =============================================================================
CREATE TABLE IF NOT EXISTS license_binding_history (
    id              SERIAL PRIMARY KEY,
    license_id      TEXT NOT NULL REFERENCES licenses(license_id),
    action          TEXT NOT NULL,
    hardware_id     TEXT,
    device_name     TEXT,
    device_info     TEXT,
    performed_by    TEXT,
    reason          TEXT,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_binding_history_license_id ON license_binding_history(license_id);
CREATE INDEX IF NOT EXISTS idx_binding_history_created_at ON license_binding_history(created_at);

-- =============================================================================
-- API Tokens Table
-- =============================================================================
CREATE TABLE IF NOT EXISTS api_tokens (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    token_hash      TEXT NOT NULL UNIQUE,
    scopes          TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    expires_at      TEXT,
    last_used_at    TEXT,
    revoked_at      TEXT,
    created_by      TEXT
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_api_tokens_revoked ON api_tokens(revoked_at);

-- =============================================================================
-- Grant privileges (for non-superuser connections)
-- =============================================================================
-- If using a dedicated talos user, run these as superuser:
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO talos;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO talos;
