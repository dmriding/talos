# Talos - Keryx Integration Specification

## Overview

This document specifies the features and API endpoints that need to be added to Talos to support the **Keryx** licensing system. Keryx is a file transfer application with paid tiers that require license validation, feature gating, and device management.

**Integration Model:**

- Django Admin Portal manages billing, users, and business logic
- Talos is the **license authority** - issues, validates, and revokes licenses
- Keryx CLI validates directly with Talos (no Django in the validation path)
- Django and Talos share the same PostgreSQL database

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Same Server / Container                       │
│                                                                  │
│  ┌──────────────────────┐      ┌──────────────────────┐        │
│  │   Django (8000)      │      │   Talos (8080)       │        │
│  │   - Admin Portal     │─────►│   - License CRUD     │        │
│  │   - Customer Portal  │ JWT  │   - Validation       │        │
│  │   - Stripe Webhooks  │      │   - Feature gating   │        │
│  └──────────────────────┘      └──────────────────────┘        │
│             │                              │                    │
│             └──────────┬──────────────────┘                    │
│                        ▼                                        │
│              ┌─────────────────┐                               │
│              │   PostgreSQL    │                               │
│              │   (shared)      │                               │
│              └─────────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   ┌─────────┐     ┌─────────┐     ┌─────────┐
   │ Keryx   │     │ Keryx   │     │ Keryx   │
   │ CLI     │────►│ CLI     │────►│ CLI     │
   └─────────┘     └─────────┘     └─────────┘
        │               │               │
        └───────────────┴───────────────┘
                        │
                        ▼
                   Talos :8080
                   /api/v1/validate
```

---

## Current State vs. Required

| Feature                                 | Current Talos       | Required for Keryx                     | Priority |
| --------------------------------------- | ------------------- | -------------------------------------- | -------- |
| Activate/Deactivate                     | Yes                 | Yes                                    | -        |
| Validate (binary)                       | Yes                 | Yes                                    | -        |
| Heartbeat                               | Yes                 | Yes                                    | -        |
| Feature list storage                    | Yes (`Vec<String>`) | Yes                                    | -        |
| **Admin API (CRUD)**                    | No                  | Yes                                    | P0       |
| **License key generation**              | No                  | Yes (`KERYX-XXXX-XXXX-XXXX`)           | P0       |
| **Feature validation endpoint**         | No                  | Yes                                    | P0       |
| **Hardware binding (1 key = 1 device)** | Yes                 | Yes (enhanced with bind/release)       | P0       |
| **Bind/Release workflow**               | No                  | Yes (release key to re-bind elsewhere) | P0       |
| **Multi-license per org**               | No                  | Yes (org buys N keys)                  | P0       |
| **JWT service authentication**          | No                  | Yes                                    | P0       |
| **Revoke with grace period**            | No                  | Yes                                    | P1       |
| **Extend expiry**                       | No                  | Yes                                    | P1       |
| **Usage/limits tracking**               | No                  | Yes                                    | P1       |
| **Blacklist/ban**                       | No                  | Yes                                    | P2       |
| **Org-based grouping**                  | No                  | Yes (licenses belong to org)           | P0       |
| **Metadata storage**                    | No                  | Yes (Stripe IDs)                       | P1       |

---

## Licensing Model

### Key Concepts

1. **Organization** - A billing entity (company/team) that purchases licenses
2. **License Key** - A single `KERYX-XXXX-XXXX-XXXX` key that grants access
3. **Hardware Binding** - Each license key can only be active on ONE device at a time
4. **Bind/Release** - Users can release a key from current hardware to re-bind it elsewhere

### Purchase Flow

```
Organization "Acme Corp" purchases 5 Pro licenses
    ↓
Django calls Talos 5 times (or batch endpoint)
    ↓
Talos creates 5 license keys:
  - KERYX-A1B2-C3D4-E5F6-G7H8 (unbound)
  - KERYX-H8G7-F6E5-D4C3-B2A1 (unbound)
  - KERYX-X1Y2-Z3W4-V5U6-T7S8 (unbound)
  - KERYX-S8T7-U6V5-W4Z3-Y2X1 (unbound)
  - KERYX-M1N2-O3P4-Q5R6-S7T8 (unbound)
    ↓
Django stores all 5 keys, emails them to org admin
```

### Bind/Release Flow

```
┌─────────────┐         ┌─────────────┐
│   Keryx     │         │   Talos     │
│   CLI       │         │             │
└──────┬──────┘         └──────┬──────┘
       │                       │
       │ POST /api/v1/client/  │
       │ bind                  │
       │ { license_key,        │
       │   hardware_id }       │
       │──────────────────────►│
       │                       │
       │                       │ Check: is key already bound?
       │                       │   No  → bind to this hardware
       │                       │   Yes → reject (already bound)
       │                       │
       │   { success: true,    │
       │     bound_to: hw_id } │
       │◄──────────────────────│
       │                       │

--- Later, user wants to move license to new machine ---

       │                       │
       │ POST /api/v1/client/  │
       │ release               │
       │ { license_key,        │
       │   hardware_id }       │  (must match current binding)
       │──────────────────────►│
       │                       │
       │                       │ Check: does hw_id match binding?
       │                       │   Yes → unbind, key is now free
       │                       │   No  → reject (wrong device)
       │                       │
       │   { success: true,    │
       │     status: unbound } │
       │◄──────────────────────│
       │                       │

--- User activates on new machine ---

       │                       │
       │ POST /api/v1/client/  │
       │ bind                  │
       │ { license_key,        │
       │   new_hardware_id }   │
       │──────────────────────►│
       │                       │
       │   { success: true }   │
       │◄──────────────────────│
```

### Validation with Hardware Check

```
POST /api/v1/client/validate
{ license_key, hardware_id }
    ↓
Talos checks:
  1. Does license exist? → No → LICENSE_NOT_FOUND
  2. Is license expired? → Yes → LICENSE_EXPIRED
  3. Is license revoked? → Yes → LICENSE_REVOKED
  4. Is key bound? → No → LICENSE_NOT_BOUND (must bind first)
  5. Does hardware_id match binding? → No → HARDWARE_MISMATCH
  6. All checks pass → { valid: true, features: [...] }
```

---

## Database Schema Changes

### Modified `licenses` Table

Each row represents ONE license key that can be bound to ONE device.

```sql
-- Drop old schema and recreate (or migrate)
CREATE TABLE licenses (
    -- Primary identifiers
    license_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_key TEXT UNIQUE NOT NULL,              -- KERYX-XXXX-XXXX-XXXX (user-facing)

    -- Organization (for grouping/billing - org can have many licenses)
    org_id UUID NOT NULL,                          -- Links to Django organizations table
    org_name TEXT,                                 -- Denormalized for display

    -- License configuration
    tier TEXT NOT NULL DEFAULT 'free',             -- 'free', 'starter', 'pro', 'team', 'enterprise'
    features JSONB NOT NULL DEFAULT '[]',          -- ['relay', 'priority_support', 'dedicated_relay']

    -- Hardware binding (1 license = 1 device at a time)
    hardware_id TEXT,                              -- SHA-256 fingerprint, NULL = unbound
    device_name TEXT,                              -- User-provided: "David's Laptop"
    device_info JSONB DEFAULT '{}',                -- {os: 'windows', version: '11', ...}
    bound_at TIMESTAMPTZ,                          -- When hardware was bound
    last_seen_at TIMESTAMPTZ,                      -- Last heartbeat/validation

    -- Status and lifecycle
    status TEXT NOT NULL DEFAULT 'active',         -- 'active', 'suspended', 'revoked', 'expired'
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,                        -- NULL = never expires

    -- Suspension/revocation
    suspended_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    revoke_reason TEXT,                            -- 'payment_failed', 'subscription_canceled', 'terms_violation'
    grace_period_ends_at TIMESTAMPTZ,              -- Service continues until this date
    suspension_message TEXT,                       -- Shown to user on validation failure

    -- Blacklist
    is_blacklisted BOOLEAN NOT NULL DEFAULT FALSE,
    blacklisted_at TIMESTAMPTZ,
    blacklist_reason TEXT,

    -- External references
    metadata JSONB DEFAULT '{}',                   -- {stripe_customer_id, stripe_subscription_id, ...}

    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_licenses_org_id ON licenses(org_id);
CREATE INDEX idx_licenses_license_key ON licenses(license_key);
CREATE INDEX idx_licenses_hardware_id ON licenses(hardware_id) WHERE hardware_id IS NOT NULL;
CREATE INDEX idx_licenses_status ON licenses(status);
CREATE INDEX idx_licenses_expires_at ON licenses(expires_at);
```

### New `license_binding_history` Table (Optional)

Tracks bind/release history for audit purposes.

```sql
CREATE TABLE license_binding_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_id UUID NOT NULL REFERENCES licenses(license_id) ON DELETE CASCADE,

    -- Action
    action TEXT NOT NULL,                          -- 'bind', 'release', 'admin_release'
    hardware_id TEXT NOT NULL,                     -- Device that was bound/released
    device_name TEXT,
    device_info JSONB DEFAULT '{}',

    -- Audit
    performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    performed_by TEXT,                             -- 'client', 'admin', 'system'
    reason TEXT                                    -- For admin releases: "user requested", etc.
);

-- Index for looking up history by license
CREATE INDEX idx_binding_history_license_id ON license_binding_history(license_id);
```

### New `api_tokens` Table

For Django service authentication.

```sql
CREATE TABLE api_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,                            -- 'django-service', 'admin-cli'
    token_hash TEXT NOT NULL,                      -- SHA-256 of the actual token
    scopes JSONB NOT NULL DEFAULT '["*"]',         -- ['licenses:read', 'licenses:write', ...]
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    expires_at TIMESTAMPTZ,                        -- NULL = never expires
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## API Specification

### Authentication

**Admin endpoints** require JWT Bearer token:

```
Authorization: Bearer <jwt-token>
```

JWT payload:

```json
{
  "sub": "django-service",
  "iat": 1704067200,
  "exp": 1704153600,
  "scopes": ["licenses:*"]
}
```

**Client endpoints** (validation, heartbeat) are unauthenticated but rate-limited.

---

## Admin API Endpoints

Base path: `/api/v1`

### Endpoint Summary

| Method | Endpoint                                  | Purpose                  |
| ------ | ----------------------------------------- | ------------------------ |
| POST   | `/api/v1/licenses`                        | Create single license    |
| POST   | `/api/v1/licenses/batch`                  | Create multiple licenses |
| GET    | `/api/v1/licenses?org_id={id}`            | List org's licenses      |
| GET    | `/api/v1/licenses/{license_id}`           | Get license details      |
| PATCH  | `/api/v1/licenses/{license_id}`           | Update tier/features     |
| POST   | `/api/v1/licenses/{license_id}/revoke`    | Suspend/revoke           |
| POST   | `/api/v1/licenses/{license_id}/reinstate` | Reinstate                |
| POST   | `/api/v1/licenses/{license_id}/extend`    | Extend expiry            |
| POST   | `/api/v1/licenses/{license_id}/release`   | Admin force unbind       |
| POST   | `/api/v1/licenses/{license_id}/blacklist` | Permanent ban            |

---

### Create License

**POST** `/api/v1/licenses`

Creates a single new license. Each license is hardware-bound (1 key = 1 device).

**Request:**

```json
{
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "features": ["relay", "priority_support"],
  "expires_at": "2025-02-01T00:00:00Z",
  "metadata": {
    "stripe_customer_id": "cus_xxxxxxxxxxxxx",
    "stripe_subscription_id": "sub_xxxxxxxxxxxxx"
  }
}
```

**Response (201 Created):**

```json
{
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "tier": "pro",
  "status": "active",
  "features": ["relay", "priority_support"],
  "is_bound": false,
  "issued_at": "2025-01-01T00:00:00Z",
  "expires_at": "2025-02-01T00:00:00Z"
}
```

**License Key Format:**

- Pattern: `KERYX-XXXX-XXXX-XXXX-XXXX`
- Characters: Uppercase alphanumeric (excluding ambiguous: 0, O, I, L)
- Generated using cryptographically secure random

---

### Get License

**GET** `/api/v1/licenses/{license_id}`

**Response (200 OK):**

```json
{
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "status": "active",
  "features": ["relay", "priority_support"],
  "limits": {
    "bandwidth_gb": 500,
    "max_users": 5
  },
  "max_devices": 5,
  "active_devices": 2,
  "issued_at": "2025-01-01T00:00:00Z",
  "expires_at": "2025-02-01T00:00:00Z",
  "bandwidth_used_bytes": 107374182400,
  "bandwidth_limit_bytes": 536870912000,
  "quota_exceeded": false,
  "metadata": {
    "stripe_customer_id": "cus_xxxxxxxxxxxxx",
    "stripe_subscription_id": "sub_xxxxxxxxxxxxx"
  }
}
```

---

### Update License

**PATCH** `/api/v1/licenses/{license_id}`

Updates tier, features, or expiry. Used for upgrades/downgrades.

**Note:** `limits`, `max_devices`, and `bandwidth_limit` are NOT passed by Django - Talos derives these from the tier configuration internally.

**Request:**

```json
{
  "tier": "team",
  "features": ["relay", "priority_support", "dedicated_relay"],
  "expires_at": "2025-02-01T00:00:00Z"
}
```

**Response (200 OK):** Full license object with updated fields (including tier-derived limits).

---

### Revoke License

**POST** `/api/v1/licenses/{license_id}/revoke`

Suspends or revokes a license. Used for payment failures, cancellations.

**Request:**

```json
{
  "reason": "payment_failed",
  "grace_period_days": 7,
  "message": "Payment failed. Please update your payment method within 7 days."
}
```

**Response (200 OK):**

```json
{
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "status": "suspended",
  "revoke_reason": "payment_failed",
  "grace_period_ends_at": "2025-01-08T00:00:00Z",
  "suspension_message": "Payment failed. Please update your payment method within 7 days."
}
```

**Behavior:**

- `grace_period_days: 0` = immediate revocation, status becomes `revoked`
- `grace_period_days: N` = status becomes `suspended`, validation succeeds until grace period ends
- When grace period ends, background job changes status to `revoked`

---

### Reinstate License

**POST** `/api/v1/licenses/{license_id}/reinstate`

Reinstates a suspended/revoked license (e.g., after payment retry succeeds).

**Request:**

```json
{
  "new_expires_at": "2025-02-01T00:00:00Z",
  "reset_bandwidth": true
}
```

**Response (200 OK):** Full license object with `status: "active"`.

---

### Extend License

**POST** `/api/v1/licenses/{license_id}/extend`

Extends expiry date (e.g., when monthly invoice is paid).

**Request:**

```json
{
  "new_expires_at": "2025-03-01T00:00:00Z",
  "reset_bandwidth": true
}
```

**Response (200 OK):** Full license object with updated `expires_at`.

---

### Update Usage/Limits

**PATCH** `/api/v1/licenses/{license_id}/usage`

Updates bandwidth usage. Django calls this periodically to sync usage from Redis.

**Request:**

```json
{
  "bandwidth_used_bytes": 450000000000,
  "bandwidth_limit_bytes": 536870912000
}
```

**Response (200 OK):**

```json
{
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "bandwidth_used_bytes": 450000000000,
  "bandwidth_limit_bytes": 536870912000,
  "quota_exceeded": false,
  "quota_restricted_features": []
}
```

**Behavior:**

- If `bandwidth_used_bytes >= bandwidth_limit_bytes`:
  - Sets `quota_exceeded: true`
  - Sets `quota_restricted_features: ["relay"]`
- Validation endpoint will deny access to restricted features

---

### Blacklist License

**POST** `/api/v1/licenses/{license_id}/blacklist`

Permanently bans an organization. Validation always fails.

**Request:**

```json
{
  "reason": "terms_violation",
  "message": "Account suspended for Terms of Service violation. Contact support."
}
```

**Response (200 OK):**

```json
{
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "status": "revoked",
  "is_blacklisted": true,
  "blacklist_reason": "terms_violation"
}
```

---

### Admin Release (Force Unbind)

**POST** `/api/v1/licenses/{license_id}/release`

Admin forcibly unbinds a license from its current hardware. Used when user loses access to device.

**Request:**

```json
{
  "reason": "user_request"
}
```

**Response (200 OK):**

```json
{
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "previous_hardware_id": "sha256:abc123...",
  "previous_device_name": "David's Laptop",
  "status": "unbound",
  "message": "License unbound by administrator."
}
```

---

### List Organization Licenses

**GET** `/api/v1/licenses?org_id={org_id}`

Lists all licenses for an organization.

**Response (200 OK):**

```json
{
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "org_name": "Acme Corporation",
  "total_licenses": 5,
  "bound_licenses": 3,
  "licenses": [
    {
      "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
      "tier": "pro",
      "status": "active",
      "is_bound": true,
      "device_name": "David's Laptop",
      "bound_at": "2025-01-01T10:00:00Z",
      "last_seen_at": "2025-01-15T14:30:00Z",
      "expires_at": "2025-02-01T00:00:00Z"
    },
    {
      "license_id": "8d0f7780-8536-51e5-b827-557766551111",
      "license_key": "KERYX-H8G7-F6E5-D4C3-B2A1",
      "tier": "pro",
      "status": "active",
      "is_bound": false,
      "device_name": null,
      "bound_at": null,
      "last_seen_at": null,
      "expires_at": "2025-02-01T00:00:00Z"
    }
  ]
}
```

---

### Batch Create Licenses

**POST** `/api/v1/licenses/batch`

Creates multiple licenses at once (for bulk purchases).

**Request:**

```json
{
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "features": ["relay", "priority_support"],
  "expires_at": "2025-02-01T00:00:00Z",
  "count": 5,
  "metadata": {
    "stripe_customer_id": "cus_xxxxxxxxxxxxx",
    "stripe_subscription_id": "sub_xxxxxxxxxxxxx"
  }
}
```

**Response (201 Created):**

```json
{
  "created": 5,
  "licenses": [
    {
      "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8"
    },
    {
      "license_id": "8d0f7780-8536-51e5-b827-557766551111",
      "license_key": "KERYX-H8G7-F6E5-D4C3-B2A1"
    },
    {
      "license_id": "9e1g8891-9647-62f6-c938-668877662222",
      "license_key": "KERYX-X1Y2-Z3W4-V5U6-T7S8"
    },
    {
      "license_id": "af2h9902-a758-73g7-da49-779988773333",
      "license_key": "KERYX-S8T7-U6V5-W4Z3-Y2X1"
    },
    {
      "license_id": "bg3i0013-b869-84h8-eb50-880099884444",
      "license_key": "KERYX-M1N2-O3P4-Q5R6-S7T8"
    }
  ]
}
```

---

## Client API Endpoints

These endpoints are called by Keryx CLI. No authentication required but rate-limited.

### Bind License

**POST** `/api/v1/client/bind`

Binds a license key to the current hardware. Must be called before validation.

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456...",
  "device_name": "David's Laptop",
  "device_info": {
    "os": "windows",
    "version": "11"
  }
}
```

**Response (200 OK) - Success:**

```json
{
  "success": true,
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "bound_to": "sha256:abc123def456...",
  "tier": "pro",
  "features": ["relay", "priority_support"],
  "expires_at": "2025-02-01T00:00:00Z"
}
```

**Response (200 OK) - Already Bound:**

```json
{
  "success": false,
  "error_code": "ALREADY_BOUND",
  "message": "This license is already bound to another device. Release it first to rebind.",
  "bound_to_device": "Work Desktop"
}
```

**Error Codes:**
| Code | Description |
|------|-------------|
| `LICENSE_NOT_FOUND` | License key doesn't exist |
| `LICENSE_EXPIRED` | License has expired |
| `LICENSE_REVOKED` | License revoked |
| `ALREADY_BOUND` | License is bound to a different device |

---

### Release License

**POST** `/api/v1/client/release`

Releases a license from current hardware, making it available for rebinding.

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456..."
}
```

**Response (200 OK) - Success:**

```json
{
  "success": true,
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "status": "unbound",
  "message": "License released. You can now bind it to another device."
}
```

**Response (200 OK) - Wrong Device:**

```json
{
  "success": false,
  "error_code": "HARDWARE_MISMATCH",
  "message": "This license is not bound to this device."
}
```

**Error Codes:**
| Code | Description |
|------|-------------|
| `LICENSE_NOT_FOUND` | License key doesn't exist |
| `NOT_BOUND` | License is not currently bound |
| `HARDWARE_MISMATCH` | License is bound to a different device |

---

### Validate License

**POST** `/api/v1/client/validate`

Validates a license. License must be bound to the requesting hardware.

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456..."
}
```

**Response (200 OK) - Valid:**

```json
{
  "valid": true,
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "features": ["relay", "priority_support"],
  "expires_at": "2025-02-01T00:00:00Z",
  "message": null
}
```

**Response (200 OK) - Invalid:**

```json
{
  "valid": false,
  "error_code": "LICENSE_EXPIRED",
  "message": "License expired on 2025-01-01. Please renew your subscription."
}
```

**Error Codes:**
| Code | Description |
|------|-------------|
| `LICENSE_NOT_FOUND` | License key doesn't exist |
| `LICENSE_EXPIRED` | License has expired |
| `LICENSE_SUSPENDED` | License suspended (in grace period shows remaining days) |
| `LICENSE_REVOKED` | License revoked (no grace period) |
| `LICENSE_BLACKLISTED` | Organization is banned |
| `NOT_BOUND` | License is not bound to any device |
| `HARDWARE_MISMATCH` | License is bound to a different device |

---

### Validate with Auto-Bind (Convenience)

**POST** `/api/v1/client/validate-or-bind`

Validates if already bound, otherwise attempts to bind first. Convenience endpoint.

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456...",
  "device_name": "David's Laptop",
  "device_info": {
    "os": "windows",
    "version": "11"
  }
}
```

**Behavior:**

1. If license is bound to this `hardware_id` → validate and return
2. If license is unbound → bind to this hardware, then validate
3. If license is bound to different hardware → return `ALREADY_BOUND` error

**Response:** Same as `/validate` endpoint

---

### Validate Feature

**POST** `/api/v1/client/validate-feature`

Checks if a specific feature is available for this license.

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456...",
  "feature": "relay"
}
```

**Response (200 OK) - Allowed:**

```json
{
  "allowed": true,
  "feature": "relay",
  "license_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7"
}
```

**Response (200 OK) - Denied:**

```json
{
  "allowed": false,
  "feature": "relay",
  "error_code": "FEATURE_NOT_INCLUDED",
  "message": "The 'relay' feature is not included in your plan. Upgrade to Pro to access relay servers."
}
```

**Response (200 OK) - Quota Restricted:**

```json
{
  "allowed": false,
  "feature": "relay",
  "error_code": "QUOTA_EXCEEDED",
  "message": "Bandwidth quota exceeded. Relay access disabled until next billing cycle."
}
```

---

### Heartbeat

**POST** `/api/v1/client/heartbeat`

Sends liveness ping. Updates `last_seen_at` on the device activation.

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456..."
}
```

**Response (200 OK):**

```json
{
  "success": true,
  "server_time": "2025-01-15T14:30:00Z"
}
```

---

### Deactivate Device (Client)

**POST** `/api/v1/client/deactivate`

User voluntarily deactivates their device (frees up a slot).

**Request:**

```json
{
  "license_key": "KERYX-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "sha256:abc123def456..."
}
```

**Response (200 OK):**

```json
{
  "success": true,
  "message": "Device deactivated successfully."
}
```

---

## License Key Generation

### Format

```
KERYX-XXXX-XXXX-XXXX-XXXX
```

### Character Set

Uppercase alphanumeric excluding ambiguous characters:

```
ABCDEFGHJKMNPQRSTUVWXYZ23456789
```

(Excludes: 0, O, I, L, 1)

### Generation Algorithm

```rust
use rand::Rng;

const CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

fn generate_license_key() -> String {
    let mut rng = rand::thread_rng();
    let segments: Vec<String> = (0..4)
        .map(|_| {
            (0..4)
                .map(|_| {
                    let idx = rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect()
        })
        .collect();

    format!("KERYX-{}", segments.join("-"))
}
```

---

## Tier Configuration

Talos owns the tier-to-limits mapping. When Django sends `tier: "pro"`, Talos looks up the limits internally:

```rust
// Built-in tier configuration (in Talos)
const TIER_LIMITS: &[TierConfig] = &[
    TierConfig {
        name: "free",
        bandwidth_gb: 0,
        max_devices: 1,
        features: &[],
    },
    TierConfig {
        name: "starter",
        bandwidth_gb: 50,
        max_devices: 2,
        features: &["relay"],
    },
    TierConfig {
        name: "pro",
        bandwidth_gb: 500,
        max_devices: 5,
        features: &["relay", "priority_support"],
    },
    TierConfig {
        name: "team",
        bandwidth_gb: 2000,
        max_devices: 25,
        features: &["relay", "priority_support", "dedicated_relay"],
    },
    TierConfig {
        name: "enterprise",
        bandwidth_gb: 0,  // Custom (set via update_license)
        max_devices: 100,
        features: &["relay", "priority_support", "dedicated_relay", "sla"],
    },
];
```

**Note:** For `enterprise` tier, bandwidth limits can be customized via the `/usage` endpoint.

---

## Configuration

### Environment Variables

```env
# Server
TALOS_HOST=0.0.0.0
TALOS_PORT=8080
TALOS_LOG_LEVEL=info

# Database
DATABASE_URL=postgres://talos:password@localhost/keryx

# JWT Authentication
TALOS_JWT_SECRET=<256-bit-secret>
TALOS_JWT_ISSUER=talos.netviper.cloud
TALOS_JWT_AUDIENCE=talos-api

# Rate Limiting
TALOS_RATE_LIMIT_VALIDATE=100/minute
TALOS_RATE_LIMIT_HEARTBEAT=60/minute
```

### config.toml

```toml
[server]
host = "0.0.0.0"
port = 8080
log_level = "info"

[database]
url = "postgres://talos:password@localhost/keryx"
max_connections = 10

[jwt]
secret = "env:TALOS_JWT_SECRET"
issuer = "talos.netviper.cloud"
audience = "talos-api"
expiry_hours = 24

[rate_limit]
validate_per_minute = 100
heartbeat_per_minute = 60

[license]
key_prefix = "KERYX"
default_grace_period_days = 7
```

---

## Background Jobs

### Grace Period Expiration

Runs every hour. Finds suspended licenses where `grace_period_ends_at < NOW()` and updates status to `revoked`.

```rust
async fn expire_grace_periods(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        UPDATE licenses
        SET status = 'revoked',
            revoked_at = NOW(),
            updated_at = NOW()
        WHERE status = 'suspended'
          AND grace_period_ends_at < NOW()
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
```

### License Expiration

Runs every hour. Finds active licenses where `expires_at < NOW()` and updates status to `expired`.

```rust
async fn expire_licenses(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        UPDATE licenses
        SET status = 'expired',
            updated_at = NOW()
        WHERE status = 'active'
          AND expires_at IS NOT NULL
          AND expires_at < NOW()
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
```

### Stale Device Cleanup (Optional)

Runs daily. Deactivates devices that haven't sent a heartbeat in 90 days.

```rust
async fn cleanup_stale_devices(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        UPDATE license_activations
        SET is_active = FALSE,
            deactivated_at = NOW()
        WHERE is_active = TRUE
          AND last_seen_at < NOW() - INTERVAL '90 days'
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable description",
    "details": {}
  }
}
```

### HTTP Status Codes

| Status | Usage                                  |
| ------ | -------------------------------------- |
| 200    | Success                                |
| 201    | Created                                |
| 204    | No Content (successful delete)         |
| 400    | Bad Request (validation error)         |
| 401    | Unauthorized (missing/invalid JWT)     |
| 403    | Forbidden (insufficient scopes)        |
| 404    | Not Found                              |
| 409    | Conflict (e.g., license key collision) |
| 429    | Too Many Requests (rate limited)       |
| 500    | Internal Server Error                  |

---

## Implementation Phases

### Phase 1: Core Admin API (P0)

- [ ] Database schema migration
- [ ] License CRUD endpoints
- [ ] License key generation
- [ ] JWT authentication middleware
- [ ] Basic validation endpoint update

### Phase 2: Device Management (P0)

- [ ] Device activation table
- [ ] Auto-activation on validate
- [ ] Device limit enforcement
- [ ] List/deactivate devices

### Phase 3: Feature Gating (P0)

- [ ] Feature validation endpoint
- [ ] Quota exceeded feature restriction

### Phase 4: Lifecycle Management (P1)

- [ ] Revoke with grace period
- [ ] Reinstate license
- [ ] Extend expiry
- [ ] Usage tracking endpoint

### Phase 5: Background Jobs (P1)

- [ ] Grace period expiration job
- [ ] License expiration job
- [ ] Stale device cleanup job

### Phase 6: Blacklist & Polish (P2)

- [ ] Blacklist endpoint
- [ ] Rate limiting
- [ ] Comprehensive error handling
- [ ] API documentation (OpenAPI)

---

## Testing Requirements

### Unit Tests

- License key generation uniqueness
- JWT validation
- Feature permission logic
- Device limit enforcement

### Integration Tests

- Full license lifecycle (create → validate → update → revoke)
- Device activation flow
- Grace period expiration
- Quota exceeded behavior

### Load Tests

- Validation endpoint: 1000 req/s target
- Heartbeat endpoint: 500 req/s target

---

## Security Considerations

1. **JWT tokens** should have short expiry (24h) with refresh capability
2. **License keys** should be cryptographically random (128+ bits of entropy)
3. **Hardware IDs** are hashed client-side (SHA-256) before transmission
4. **Rate limiting** prevents brute-force license key guessing
5. **Database** should use TLS connections in production
6. **Admin endpoints** must never be exposed without authentication
