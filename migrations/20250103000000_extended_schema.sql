-- Extended schema for Talos Admin API
-- This migration adds fields needed for the full licensing system

-- ============================================================================
-- Alter licenses table with new columns
-- ============================================================================

-- Organization fields (nullable for simple use cases)
ALTER TABLE licenses ADD COLUMN org_id TEXT;
ALTER TABLE licenses ADD COLUMN org_name TEXT;

-- Human-readable license key (PREFIX-XXXX-XXXX-XXXX format)
ALTER TABLE licenses ADD COLUMN license_key TEXT UNIQUE;

-- Tier system (optional)
ALTER TABLE licenses ADD COLUMN tier TEXT;

-- Extended hardware binding fields
ALTER TABLE licenses ADD COLUMN device_name TEXT;
ALTER TABLE licenses ADD COLUMN device_info TEXT;
ALTER TABLE licenses ADD COLUMN bound_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN last_seen_at TIMESTAMP;

-- Status lifecycle fields
ALTER TABLE licenses ADD COLUMN suspended_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN revoked_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN revoke_reason TEXT;
ALTER TABLE licenses ADD COLUMN grace_period_ends_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN suspension_message TEXT;

-- Blacklist fields
ALTER TABLE licenses ADD COLUMN is_blacklisted INTEGER DEFAULT 0;
ALTER TABLE licenses ADD COLUMN blacklisted_at TIMESTAMP;
ALTER TABLE licenses ADD COLUMN blacklist_reason TEXT;

-- Arbitrary metadata (JSON string)
ALTER TABLE licenses ADD COLUMN metadata TEXT;

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
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    license_id TEXT NOT NULL,
    action TEXT NOT NULL,              -- 'bind', 'release', 'admin_release'
    hardware_id TEXT,
    device_name TEXT,
    device_info TEXT,
    performed_by TEXT,                 -- 'client', 'admin', 'system'
    reason TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (license_id) REFERENCES licenses(license_id)
);

CREATE INDEX IF NOT EXISTS idx_binding_history_license_id ON license_binding_history(license_id);
CREATE INDEX IF NOT EXISTS idx_binding_history_created_at ON license_binding_history(created_at);
