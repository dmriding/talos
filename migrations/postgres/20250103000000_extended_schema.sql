-- Extended schema for Talos Admin API (PostgreSQL)
-- This migration adds fields needed for the full licensing system

-- ============================================================================
-- Alter licenses table with new columns
-- ============================================================================

-- Organization fields (nullable for simple use cases)
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS org_id TEXT;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS org_name TEXT;

-- Human-readable license key (PREFIX-XXXX-XXXX-XXXX format)
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS license_key TEXT UNIQUE;

-- Tier system (optional)
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS tier TEXT;

-- Extended hardware binding fields
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS device_name TEXT;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS device_info TEXT;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS bound_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMP;

-- Status lifecycle fields
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS revoke_reason TEXT;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS grace_period_ends_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS suspension_message TEXT;

-- Blacklist fields
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS is_blacklisted BOOLEAN DEFAULT FALSE;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS blacklisted_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS blacklist_reason TEXT;

-- Arbitrary metadata (JSONB for efficient querying)
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS metadata JSONB;

-- ============================================================================
-- Create indexes for common queries
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_licenses_org_id ON licenses(org_id);
CREATE INDEX IF NOT EXISTS idx_licenses_license_key ON licenses(license_key);
CREATE INDEX IF NOT EXISTS idx_licenses_hardware_id ON licenses(hardware_id);
CREATE INDEX IF NOT EXISTS idx_licenses_status ON licenses(status);
CREATE INDEX IF NOT EXISTS idx_licenses_expires_at ON licenses(expires_at);
CREATE INDEX IF NOT EXISTS idx_licenses_tier ON licenses(tier);

-- ============================================================================
-- License binding history table (audit trail)
-- ============================================================================

CREATE TABLE IF NOT EXISTS license_binding_history (
    id SERIAL PRIMARY KEY,
    license_id TEXT NOT NULL REFERENCES licenses(license_id),
    action TEXT NOT NULL,              -- 'bind', 'release', 'admin_release'
    hardware_id TEXT,
    device_name TEXT,
    device_info TEXT,
    performed_by TEXT,                 -- 'client', 'admin', 'system'
    reason TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_binding_history_license_id ON license_binding_history(license_id);
CREATE INDEX IF NOT EXISTS idx_binding_history_created_at ON license_binding_history(created_at);
