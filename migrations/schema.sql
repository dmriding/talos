-- migrations/schema.sql
CREATE TABLE IF NOT EXISTS licenses (
    license_id TEXT PRIMARY KEY,          -- Unique license key
    client_id TEXT,                       -- Client ID associated with the license
    status TEXT NOT NULL,                 -- License status: active, inactive, expired
    features TEXT,                        -- Comma-separated list of licensed features/modules
    issued_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,  -- Timestamp when the license was issued
    expires_at TIMESTAMP,                -- Optional expiration date
    hardware_id TEXT,                     -- Optional hardware binding (CPU/Motherboard ID)
    signature TEXT                        -- Cryptographic signature for license validation
);