-- Add quota/usage tracking fields to licenses table
-- Phase 6.5.1: Quota Persistence

-- Bandwidth usage tracking
ALTER TABLE licenses ADD COLUMN bandwidth_used_bytes INTEGER DEFAULT 0;
ALTER TABLE licenses ADD COLUMN bandwidth_limit_bytes INTEGER;
ALTER TABLE licenses ADD COLUMN quota_exceeded INTEGER DEFAULT 0;

-- Index for quota queries (find licenses over quota)
CREATE INDEX IF NOT EXISTS idx_licenses_quota_exceeded ON licenses(quota_exceeded);
