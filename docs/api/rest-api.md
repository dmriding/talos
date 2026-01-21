# Talos REST API Reference

This document provides a complete reference for all Talos REST API endpoints. For interactive exploration, see the [Swagger UI](#interactive-documentation).

## Base URL

```
https://your-license-server.com
```

## Authentication

Admin endpoints require Bearer token authentication:

```http
Authorization: Bearer <your-api-token>
```

Client endpoints (bind, validate, etc.) authenticate via license key in the request body.

---

## Table of Contents

- [System Endpoints](#system-endpoints)
- [Client API](#client-api)
- [Admin API](#admin-api)
- [Token Management](#token-management)
- [Legacy Endpoints](#legacy-endpoints)
- [Error Responses](#error-responses)
- [Schema Reference](#schema-reference)

---

## System Endpoints

### Health Check

Check server health and database connectivity.

```http
GET /health
```

**Response** `200 OK`

```json
{
  "status": "healthy",
  "database": "connected",
  "database_type": "postgresql",
  "version": "0.2.2"
}
```

---

## Client API

These endpoints are used by client applications to manage license bindings and validation.

### Bind License

Bind a license to hardware. This activates the license for the current machine.

```http
POST /api/v1/client/bind
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `license_key` | string | Yes | License key (format: `LIC-XXXX-XXXX-XXXX-XXXX`) |
| `hardware_id` | string | Yes | SHA-256 hardware fingerprint (64 hex chars) |
| `device_name` | string | No | Human-readable device name |
| `device_info` | object | No | Additional device metadata |

**Example Request**

```json
{
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd",
  "device_name": "John's Laptop",
  "device_info": {
    "os": "Windows 11",
    "hostname": "DESKTOP-ABC123"
  }
}
```

**Response** `200 OK`

```json
{
  "license_id": "550e8400-e29b-41d4-a716-446655440000",
  "features": ["basic", "export", "api_access"],
  "tier": "professional",
  "expires_at": "2026-12-31T23:59:59Z",
  "grace_period_ends_at": "2026-01-15T00:00:00Z"
}
```

**Errors**
- `400` - Invalid request (missing/invalid fields)
- `404` - License not found
- `409` - License already bound to another device

---

### Release License

Release a license from the current hardware binding.

```http
POST /api/v1/client/release
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `license_key` | string | Yes | License key |
| `hardware_id` | string | Yes | Current hardware fingerprint |

**Example Request**

```json
{
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"
}
```

**Response** `200 OK`

```json
{
  "released": true,
  "message": "License successfully released"
}
```

**Errors**
- `400` - Invalid request
- `404` - License not found
- `409` - Hardware mismatch (not bound to this device)

---

### Validate License

Validate a bound license. Returns current license status and features.

```http
POST /api/v1/client/validate
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `license_key` | string | Yes | License key |
| `hardware_id` | string | Yes | Hardware fingerprint |

**Example Request**

```json
{
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"
}
```

**Response** `200 OK`

```json
{
  "valid": true,
  "license_id": "550e8400-e29b-41d4-a716-446655440000",
  "features": ["basic", "export", "api_access"],
  "tier": "professional",
  "expires_at": "2026-12-31T23:59:59Z",
  "grace_period_ends_at": "2026-01-15T00:00:00Z",
  "warning": null,
  "org_id": "org-123456",
  "org_name": "Acme Corp",
  "bandwidth_used_bytes": 1073741824,
  "bandwidth_limit_bytes": 5368709120
}
```

| Field | Type | Description |
|-------|------|-------------|
| `valid` | boolean | Whether the license is valid |
| `license_id` | string | The license ID |
| `features` | array | List of enabled features |
| `tier` | string | License tier name |
| `expires_at` | string | Expiration date (RFC3339) |
| `grace_period_ends_at` | string | Grace period end date (if suspended) |
| `warning` | string | Warning message (e.g., nearing expiration) |
| `org_id` | string | Organization ID (falls back to license_id if not set) |
| `org_name` | string | Organization name (falls back to org_id if not set) |
| `bandwidth_used_bytes` | integer | Bandwidth used this billing period (bytes) |
| `bandwidth_limit_bytes` | integer | Bandwidth limit (bytes), null means unlimited |

**Note:** Fields with null values are omitted from the response.

**Errors**
- `400` - Invalid request
- `401` - License expired, revoked, or blacklisted
- `404` - License not found
- `409` - Hardware mismatch / Not bound

---

### Validate or Bind

Validate if already bound, otherwise bind automatically. Useful for first-run scenarios.

```http
POST /api/v1/client/validate-or-bind
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `license_key` | string | Yes | License key |
| `hardware_id` | string | Yes | Hardware fingerprint |
| `device_name` | string | No | Device name (used if binding) |
| `device_info` | object | No | Device metadata (used if binding) |

**Example Request**

```json
{
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd",
  "device_name": "John's Laptop"
}
```

**Response** `200 OK`

```json
{
  "valid": true,
  "was_bound": false,
  "license_id": "550e8400-e29b-41d4-a716-446655440000",
  "features": ["basic", "export"],
  "tier": "professional",
  "expires_at": "2026-12-31T23:59:59Z",
  "grace_period_ends_at": null,
  "warning": null,
  "org_id": "org-123456",
  "org_name": "Acme Corp",
  "bandwidth_used_bytes": 0,
  "bandwidth_limit_bytes": 5368709120
}
```

The `was_bound` field indicates whether this request performed a new binding. See the [Validate License](#validate-license) response for field descriptions.

---

### Heartbeat

Send a heartbeat to maintain license validity and update grace period.

```http
POST /api/v1/client/heartbeat
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `license_key` | string | Yes | License key |
| `hardware_id` | string | Yes | Hardware fingerprint |

**Example Request**

```json
{
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"
}
```

**Response** `200 OK`

```json
{
  "acknowledged": true,
  "server_time": "2026-01-05T12:00:00Z",
  "grace_period_ends_at": "2026-01-15T00:00:00Z"
}
```

---

### Validate Feature

Check if a specific feature is available for the license.

```http
POST /api/v1/client/validate-feature
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `license_key` | string | Yes | License key |
| `hardware_id` | string | Yes | Hardware fingerprint |
| `feature` | string | Yes | Feature name to check |

**Example Request**

```json
{
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "hardware_id": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd",
  "feature": "premium_export"
}
```

**Response** `200 OK`

```json
{
  "allowed": true,
  "feature": "premium_export",
  "tier": "professional",
  "message": "Feature is included in your license tier"
}
```

**Response** `403 Forbidden` (feature not included)

```json
{
  "allowed": false,
  "feature": "premium_export",
  "tier": "basic",
  "message": "Upgrade to Professional tier to access this feature"
}
```

---

## Admin API

These endpoints require Bearer token authentication and are used to manage licenses.

### Create License

Create a new license.

```http
POST /api/v1/licenses
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `org_id` | string | Yes | Organization ID (UUID) |
| `tier` | string | No | License tier (default: "basic") |
| `features` | array | No | List of feature names |
| `expires_at` | string | No | Expiration date (RFC3339) |
| `max_devices` | integer | No | Maximum concurrent devices |
| `metadata` | object | No | Custom metadata |

**Example Request**

```json
{
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "tier": "professional",
  "features": ["basic", "export", "api_access"],
  "expires_at": "2027-01-01T00:00:00Z",
  "max_devices": 5
}
```

**Response** `201 Created`

```json
{
  "license_id": "660e8400-e29b-41d4-a716-446655440001",
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "tier": "professional",
  "features": ["basic", "export", "api_access"],
  "status": "active",
  "expires_at": "2027-01-01T00:00:00Z",
  "created_at": "2026-01-05T12:00:00Z"
}
```

---

### Batch Create Licenses

Create multiple licenses in a single request.

```http
POST /api/v1/licenses/batch
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `licenses` | array | Yes | Array of license creation objects |

**Example Request**

```json
{
  "licenses": [
    {
      "org_id": "550e8400-e29b-41d4-a716-446655440000",
      "tier": "basic"
    },
    {
      "org_id": "550e8400-e29b-41d4-a716-446655440000",
      "tier": "professional",
      "features": ["export"]
    }
  ]
}
```

**Response** `201 Created`

```json
{
  "created": 2,
  "licenses": [
    {
      "license_id": "...",
      "license_key": "LIC-XXXX-XXXX-XXXX-XXXX"
    },
    {
      "license_id": "...",
      "license_key": "LIC-YYYY-YYYY-YYYY-YYYY"
    }
  ]
}
```

---

### Get License

Retrieve a specific license by ID.

```http
GET /api/v1/licenses/{license_id}
Authorization: Bearer <token>
```

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `license_id` | UUID | License ID |

**Response** `200 OK`

```json
{
  "license_id": "660e8400-e29b-41d4-a716-446655440001",
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "org_id": "550e8400-e29b-41d4-a716-446655440000",
  "tier": "professional",
  "features": ["basic", "export", "api_access"],
  "status": "active",
  "is_bound": true,
  "hardware_id": "a1b2c3d4...",
  "device_name": "John's Laptop",
  "bound_at": "2026-01-05T10:00:00Z",
  "expires_at": "2027-01-01T00:00:00Z",
  "created_at": "2026-01-05T12:00:00Z",
  "last_validated_at": "2026-01-05T14:00:00Z",
  "last_heartbeat_at": "2026-01-05T14:30:00Z"
}
```

---

### List Licenses

List licenses for an organization.

```http
GET /api/v1/licenses?org_id={org_id}
Authorization: Bearer <token>
```

**Query Parameters**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `org_id` | UUID | Yes | Organization ID |
| `status` | string | No | Filter by status |
| `limit` | integer | No | Max results (default: 100) |
| `offset` | integer | No | Pagination offset |

**Response** `200 OK`

```json
{
  "licenses": [
    {
      "license_id": "...",
      "license_key": "LIC-...",
      "tier": "professional",
      "status": "active",
      "is_bound": true
    }
  ],
  "total": 42,
  "limit": 100,
  "offset": 0
}
```

---

### Update License

Update license properties.

```http
PATCH /api/v1/licenses/{license_id}
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Description |
|-------|------|-------------|
| `tier` | string | New tier |
| `features` | array | New features list |
| `expires_at` | string | New expiration date |
| `metadata` | object | Updated metadata |

**Example Request**

```json
{
  "tier": "enterprise",
  "features": ["basic", "export", "api_access", "priority_support"]
}
```

**Response** `200 OK`

Returns the updated license object.

---

### Admin Release

Force-release a license from its hardware binding.

```http
POST /api/v1/licenses/{license_id}/release
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `reason` | string | No | Reason for force release |

**Example Request**

```json
{
  "reason": "User purchased new machine"
}
```

**Response** `200 OK`

```json
{
  "released": true,
  "previous_device": "John's Laptop",
  "message": "License released successfully"
}
```

---

### Revoke License

Permanently revoke a license.

```http
POST /api/v1/licenses/{license_id}/revoke
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `reason` | string | Yes | Reason for revocation |

**Example Request**

```json
{
  "reason": "Payment failed - account cancelled"
}
```

**Response** `200 OK`

```json
{
  "revoked": true,
  "revoked_at": "2026-01-05T15:00:00Z",
  "reason": "Payment failed - account cancelled"
}
```

---

### Reinstate License

Restore a suspended or revoked license to active status.

```http
POST /api/v1/licenses/{license_id}/reinstate
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `new_expires_at` | string | No | New expiration date |
| `reset_bandwidth` | boolean | No | Reset usage counters |

**Example Request**

```json
{
  "new_expires_at": "2027-06-01T00:00:00Z",
  "reset_bandwidth": true
}
```

**Response** `200 OK`

```json
{
  "reinstated": true,
  "new_status": "active",
  "expires_at": "2027-06-01T00:00:00Z"
}
```

**Note:** Blacklisted licenses cannot be reinstated.

---

### Extend License

Extend a license's expiration date.

```http
POST /api/v1/licenses/{license_id}/extend
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `new_expires_at` | string | Yes | New expiration date (RFC3339) |
| `reset_bandwidth` | boolean | No | Reset usage counters |

**Example Request**

```json
{
  "new_expires_at": "2028-01-01T00:00:00Z"
}
```

**Response** `200 OK`

```json
{
  "extended": true,
  "old_expires_at": "2027-01-01T00:00:00Z",
  "new_expires_at": "2028-01-01T00:00:00Z"
}
```

---

### Update Usage

Update bandwidth/usage tracking for a license.

```http
PATCH /api/v1/licenses/{license_id}/usage
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `bandwidth_used` | integer | No | Current bandwidth used (bytes) |
| `bandwidth_limit` | integer | No | Bandwidth limit (bytes) |

**Example Request**

```json
{
  "bandwidth_used": 1073741824,
  "bandwidth_limit": 10737418240
}
```

**Response** `200 OK`

```json
{
  "updated": true,
  "bandwidth_used": 1073741824,
  "bandwidth_limit": 10737418240,
  "usage_percentage": 10.0,
  "quota_exceeded": false
}
```

---

### Blacklist License

Permanently blacklist a license for abuse or fraud.

```http
POST /api/v1/licenses/{license_id}/blacklist
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `reason` | string | Yes | Reason for blacklisting |

**Example Request**

```json
{
  "reason": "License key sharing detected"
}
```

**Response** `200 OK`

```json
{
  "blacklisted": true,
  "blacklisted_at": "2026-01-05T16:00:00Z",
  "reason": "License key sharing detected"
}
```

**Note:** Blacklisted licenses cannot be reinstated through normal means.

---

## Token Management

Manage API tokens for admin authentication.

### Create Token

Create a new API token.

```http
POST /api/v1/tokens
Authorization: Bearer <token>
```

**Request Body**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Token name/description |
| `scopes` | array | No | Permission scopes |
| `expires_at` | string | No | Token expiration |

**Example Request**

```json
{
  "name": "CI/CD Pipeline Token",
  "scopes": ["licenses:read", "licenses:create"],
  "expires_at": "2027-01-01T00:00:00Z"
}
```

**Response** `201 Created`

```json
{
  "token_id": "tok_abc123...",
  "token": "talos_sk_live_xxxxxxxxxxxx",
  "name": "CI/CD Pipeline Token",
  "scopes": ["licenses:read", "licenses:create"],
  "created_at": "2026-01-05T12:00:00Z",
  "expires_at": "2027-01-01T00:00:00Z"
}
```

**Important:** The `token` value is only shown once. Store it securely.

---

### List Tokens

List all API tokens.

```http
GET /api/v1/tokens
Authorization: Bearer <token>
```

**Response** `200 OK`

```json
{
  "tokens": [
    {
      "token_id": "tok_abc123...",
      "name": "CI/CD Pipeline Token",
      "scopes": ["licenses:read", "licenses:create"],
      "created_at": "2026-01-05T12:00:00Z",
      "last_used_at": "2026-01-05T14:00:00Z"
    }
  ]
}
```

---

### Get Token

Get details of a specific token.

```http
GET /api/v1/tokens/{token_id}
Authorization: Bearer <token>
```

**Response** `200 OK`

```json
{
  "token_id": "tok_abc123...",
  "name": "CI/CD Pipeline Token",
  "scopes": ["licenses:read", "licenses:create"],
  "created_at": "2026-01-05T12:00:00Z",
  "last_used_at": "2026-01-05T14:00:00Z"
}
```

---

### Revoke Token

Revoke (delete) an API token.

```http
DELETE /api/v1/tokens/{token_id}
Authorization: Bearer <token>
```

**Response** `200 OK`

```json
{
  "revoked": true,
  "token_id": "tok_abc123..."
}
```

---

## Legacy Endpoints

These endpoints are maintained for backwards compatibility. New integrations should use the Client API.

### Activate (Legacy)

```http
POST /activate
```

Equivalent to `/api/v1/client/bind`.

### Validate (Legacy)

```http
POST /validate
```

Equivalent to `/api/v1/client/validate`.

### Deactivate (Legacy)

```http
POST /deactivate
```

Equivalent to `/api/v1/client/release`.

### Heartbeat (Legacy)

```http
POST /heartbeat
```

Equivalent to `/api/v1/client/heartbeat`.

---

## Error Responses

All errors follow a consistent format:

```json
{
  "error": {
    "code": "LICENSE_NOT_FOUND",
    "message": "The requested license does not exist",
    "details": null
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `LICENSE_NOT_FOUND` | 404 | License does not exist |
| `LICENSE_EXPIRED` | 401 | License has expired |
| `LICENSE_REVOKED` | 401 | License has been revoked |
| `LICENSE_SUSPENDED` | 401 | License is suspended |
| `LICENSE_BLACKLISTED` | 401 | License is permanently blacklisted |
| `LICENSE_INACTIVE` | 401 | License is not active |
| `ALREADY_BOUND` | 409 | License already bound to another device |
| `NOT_BOUND` | 409 | License not bound to any device |
| `HARDWARE_MISMATCH` | 409 | Hardware ID doesn't match bound device |
| `FEATURE_NOT_INCLUDED` | 403 | Feature not available in license tier |
| `QUOTA_EXCEEDED` | 403 | Usage quota exceeded |
| `INVALID_REQUEST` | 400 | Request format invalid |
| `MISSING_FIELD` | 400 | Required field missing |
| `INVALID_FIELD` | 400 | Field value invalid |
| `MISSING_TOKEN` | 401 | No authorization token provided |
| `INVALID_TOKEN` | 401 | Token is invalid or malformed |
| `TOKEN_EXPIRED` | 401 | Token has expired |
| `INSUFFICIENT_SCOPE` | 403 | Token lacks required permissions |
| `DATABASE_ERROR` | 500 | Database operation failed |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

---

## Schema Reference

### License Status Values

| Status | Description |
|--------|-------------|
| `active` | License is valid and usable |
| `expired` | License has passed its expiration date |
| `suspended` | Temporarily suspended (within grace period) |
| `revoked` | Permanently revoked |

### License Tiers

Tiers are customizable, but common values include:

| Tier | Description |
|------|-------------|
| `basic` | Entry-level features |
| `professional` | Standard features |
| `enterprise` | All features + priority support |

### Feature Names

Features are arbitrary strings defined by your application. Examples:
- `basic`
- `export`
- `api_access`
- `priority_support`
- `white_label`

---

## Interactive Documentation

When running the Talos server with the `openapi` feature enabled, interactive Swagger UI is available at:

```
http://localhost:8080/swagger-ui
```

The raw OpenAPI specification is available at:

```
http://localhost:8080/api-docs/openapi.json
```

### Enabling OpenAPI

Start the server with the openapi feature:

**Mac/Linux:**
```bash
cargo run --bin talos_server --features "admin-api,openapi"
```

**Windows PowerShell:**
```powershell
cargo run --bin talos_server --features "admin-api,openapi"
```

---

## Rate Limiting

When rate limiting is enabled (`rate-limiting` feature), endpoints have default limits:

| Endpoint Type | Default Limit |
|--------------|---------------|
| Validate | 200/minute |
| Heartbeat | 120/minute |
| Bind/Release | 20/minute |
| Admin endpoints | 100/minute |

Exceeded limits return HTTP 429 with a `Retry-After` header.

---

## Versioning

The API uses URL path versioning:
- Current version: `/api/v1/`
- Legacy endpoints (no version): `/activate`, `/validate`, etc.

Breaking changes will be released under new version paths (e.g., `/api/v2/`).
