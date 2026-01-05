-- Combined SQLite schema for Talos
-- Run this on a fresh database: sqlite3 talos_dev.db < migrations/init_sqlite.sql

CREATE TABLE IF NOT EXISTS licenses (
    license_id TEXT PRIMARY KEY,
    client_id TEXT,
    status TEXT NOT NULL,
    features TEXT,
    issued_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    hardware_id TEXT,
    signature TEXT,
    last_heartbeat TIMESTAMP,

    -- Organization fields
    org_id TEXT,
    org_name TEXT,

    -- Human-readable license key
    license_key TEXT UNIQUE,

    -- Tier system
    tier TEXT,

    -- Extended hardware binding
    device_name TEXT,
    device_info TEXT,
    bound_at TIMESTAMP,
    last_seen_at TIMESTAMP,

    -- Status lifecycle
    suspended_at TIMESTAMP,
    revoked_at TIMESTAMP,
    revoke_reason TEXT,
    grace_period_ends_at TIMESTAMP,
    suspension_message TEXT,

    -- Blacklist
    is_blacklisted INTEGER DEFAULT 0,
    blacklisted_at TIMESTAMP,
    blacklist_reason TEXT,

    -- Metadata
    metadata TEXT,

    -- Quota/Usage tracking
    bandwidth_used_bytes INTEGER DEFAULT 0,
    bandwidth_limit_bytes INTEGER,
    quota_exceeded INTEGER DEFAULT 0
);

-- Indexes for licenses
CREATE INDEX IF NOT EXISTS idx_licenses_org_id ON licenses(org_id);
CREATE INDEX IF NOT EXISTS idx_licenses_license_key ON licenses(license_key);
CREATE INDEX IF NOT EXISTS idx_licenses_hardware_id ON licenses(hardware_id);
CREATE INDEX IF NOT EXISTS idx_licenses_status ON licenses(status);
CREATE INDEX IF NOT EXISTS idx_licenses_expires_at ON licenses(expires_at);
CREATE INDEX IF NOT EXISTS idx_licenses_tier ON licenses(tier);
CREATE INDEX IF NOT EXISTS idx_licenses_quota_exceeded ON licenses(quota_exceeded);

-- License binding history table
CREATE TABLE IF NOT EXISTS license_binding_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    license_id TEXT NOT NULL,
    action TEXT NOT NULL,
    hardware_id TEXT,
    device_name TEXT,
    device_info TEXT,
    performed_by TEXT,
    reason TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (license_id) REFERENCES licenses(license_id)
);

CREATE INDEX IF NOT EXISTS idx_binding_history_license_id ON license_binding_history(license_id);
CREATE INDEX IF NOT EXISTS idx_binding_history_created_at ON license_binding_history(created_at);

-- API Tokens table
CREATE TABLE IF NOT EXISTS api_tokens (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    scopes TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT,
    last_used_at TEXT,
    revoked_at TEXT,
    created_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_api_tokens_revoked ON api_tokens(revoked_at);