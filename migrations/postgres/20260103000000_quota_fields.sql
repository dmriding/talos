-- Add quota/usage tracking fields to licenses table (PostgreSQL version)
-- Phase 6.5.1: Quota Persistence

-- Bandwidth usage tracking
ALTER TABLE licenses ADD COLUMN bandwidth_used_bytes BIGINT DEFAULT 0;
ALTER TABLE licenses ADD COLUMN bandwidth_limit_bytes BIGINT;
ALTER TABLE licenses ADD COLUMN quota_exceeded BOOLEAN DEFAULT FALSE;

-- Index for quota queries (find licenses over quota)
CREATE INDEX IF NOT EXISTS idx_licenses_quota_exceeded ON licenses(quota_exceeded);
