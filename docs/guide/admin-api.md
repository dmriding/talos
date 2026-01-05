# Admin API Guide

The Admin API provides programmatic access to manage licenses, organizations, and system configuration. This guide covers authentication, all available endpoints, and security best practices.

> **Security Warning:** The Admin API provides full control over your licensing system. Always protect it with:
> - JWT authentication (required in production)
> - IP whitelisting (configure in your reverse proxy)
> - TLS/HTTPS encryption
> - Strong, rotated credentials

## Table of Contents

- [Enabling the Admin API](#enabling-the-admin-api)
- [Authentication](#authentication)
- [Security Best Practices](#security-best-practices)
- [License Management](#license-management)
- [Organization Management](#organization-management)
- [License Lifecycle](#license-lifecycle)
- [Token Management](#token-management)
- [Error Handling](#error-handling)

---

## Enabling the Admin API

The Admin API requires the `admin-api` feature flag:

```toml
# Cargo.toml
[dependencies]
talos = { git = "https://github.com/dmriding/talos", features = ["admin-api"] }
```

For production, also enable JWT authentication:

```toml
talos = { git = "https://github.com/dmriding/talos", features = ["admin-api", "jwt-auth"] }
```

---

## Authentication

### JWT Authentication (Recommended)

When `jwt-auth` is enabled, all admin endpoints require a valid JWT token.

**Configuration:**

```toml
# config.toml
[auth]
enabled = true
jwt_issuer = "talos"
jwt_audience = "talos-api"
token_expiration_secs = 86400  # 24 hours
```

```bash
# .env
TALOS_JWT_SECRET=your-secret-key-at-least-32-characters-long
```

**Creating a Token:**

```bash
# Use the bootstrap token endpoint (first-time setup)
curl -X POST http://localhost:8080/api/v1/tokens \
  -H "Content-Type: application/json" \
  -d '{
    "name": "admin-service",
    "scopes": ["licenses:*", "tokens:*"]
  }'
```

**Using the Token:**

```bash
curl -X GET http://localhost:8080/api/v1/licenses \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

### Scopes

Tokens are scoped to limit access:

| Scope | Description |
|-------|-------------|
| `licenses:read` | Read license information |
| `licenses:write` | Create, update licenses |
| `licenses:delete` | Revoke, blacklist licenses |
| `licenses:*` | All license operations |
| `tokens:read` | Read token information |
| `tokens:write` | Create tokens |
| `tokens:delete` | Revoke tokens |
| `tokens:*` | All token operations |
| `*` | Full access (admin) |

---

## Security Best Practices

### IP Whitelisting

**Critical:** Restrict Admin API access by IP using your reverse proxy.

**nginx:**

```nginx
location /api/v1/ {
    # Only allow from internal network
    allow 10.0.0.0/8;
    allow 192.168.0.0/16;
    allow 127.0.0.1;
    deny all;

    proxy_pass http://talos;
}

# Client endpoints are public
location /api/v1/client/ {
    proxy_pass http://talos;
}
```

**Traefik:**

```yaml
http:
  middlewares:
    admin-whitelist:
      ipWhiteList:
        sourceRange:
          - "10.0.0.0/8"
          - "192.168.0.0/16"
          - "127.0.0.1"

  routers:
    talos-admin:
      rule: "Host(`license.example.com`) && PathPrefix(`/api/v1/`) && !PathPrefix(`/api/v1/client/`)"
      middlewares:
        - admin-whitelist
```

### Additional Security Measures

1. **Use HTTPS** - Never expose Admin API over plain HTTP
2. **Rotate secrets** - Change JWT secret periodically
3. **Audit logs** - Monitor all admin actions
4. **Least privilege** - Create tokens with minimal required scopes
5. **Short-lived tokens** - Use short expiration times for automated systems

---

## License Management

### Create License

Creates a new license with optional organization, features, and tier.

```http
POST /api/v1/licenses
Content-Type: application/json
Authorization: Bearer <token>

{
  "org_id": "acme-corp",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "features": ["export", "api_access"],
  "expires_at": "2025-12-31T23:59:59Z",
  "metadata": {
    "stripe_customer_id": "cus_123",
    "plan": "annual"
  }
}
```

**Response:**

```json
{
  "license_id": "550e8400-e29b-41d4-a716-446655440000",
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "org_id": "acme-corp",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "features": ["export", "api_access"],
  "status": "active",
  "expires_at": "2025-12-31T23:59:59Z",
  "created_at": "2024-01-15T10:30:00Z",
  "is_bound": false
}
```

**Notes:**
- `license_key` is auto-generated using configured prefix
- `features` can be explicit or derived from `tier` configuration
- `metadata` is stored as JSON and returned in responses

### Batch Create Licenses

Create multiple licenses at once (up to 1000).

```http
POST /api/v1/licenses/batch
Content-Type: application/json
Authorization: Bearer <token>

{
  "count": 100,
  "org_id": "acme-corp",
  "tier": "pro",
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Response:**

```json
{
  "created": 100,
  "licenses": [
    {
      "license_id": "...",
      "license_key": "LIC-XXXX-YYYY-ZZZZ-AAAA"
    },
    ...
  ]
}
```

### Get License

Retrieve a license by ID.

```http
GET /api/v1/licenses/{license_id}
Authorization: Bearer <token>
```

**Response:**

```json
{
  "license_id": "550e8400-e29b-41d4-a716-446655440000",
  "license_key": "LIC-A1B2-C3D4-E5F6-G7H8",
  "org_id": "acme-corp",
  "org_name": "Acme Corporation",
  "tier": "pro",
  "features": ["export", "api_access"],
  "status": "active",
  "expires_at": "2025-12-31T23:59:59Z",
  "is_bound": true,
  "hardware_id": "abc123...",
  "device_name": "John's Workstation",
  "bound_at": "2024-01-15T14:00:00Z",
  "last_seen_at": "2024-01-16T09:30:00Z"
}
```

### List Licenses

List licenses with filtering and pagination.

```http
GET /api/v1/licenses?org_id=acme-corp&page=1&per_page=20
Authorization: Bearer <token>
```

**Query Parameters:**

| Parameter | Description | Default |
|-----------|-------------|---------|
| `org_id` | Filter by organization | - |
| `status` | Filter by status (active, suspended, revoked) | - |
| `page` | Page number | 1 |
| `per_page` | Items per page (max 100) | 20 |

**Response:**

```json
{
  "licenses": [...],
  "total": 150,
  "page": 1,
  "per_page": 20,
  "total_pages": 8
}
```

### Update License

Update license properties.

```http
PATCH /api/v1/licenses/{license_id}
Content-Type: application/json
Authorization: Bearer <token>

{
  "tier": "enterprise",
  "features": ["export", "api_access", "sso"],
  "expires_at": "2026-12-31T23:59:59Z",
  "metadata": {
    "upgraded_at": "2024-01-16"
  }
}
```

**Notes:**
- Only specified fields are updated
- Changing `tier` can auto-update features (if tier config exists)

---

## Organization Management

### List Organization Licenses

```http
GET /api/v1/licenses?org_id=acme-corp
Authorization: Bearer <token>
```

### Organization Summary (Custom Implementation)

You can build organization summaries using the list endpoint:

```python
# Example: Python script to get org summary
import requests

response = requests.get(
    "https://license.example.com/api/v1/licenses",
    params={"org_id": "acme-corp", "per_page": 100},
    headers={"Authorization": f"Bearer {token}"}
)

licenses = response.json()["licenses"]
active = sum(1 for l in licenses if l["status"] == "active")
bound = sum(1 for l in licenses if l["is_bound"])

print(f"Organization: acme-corp")
print(f"Total licenses: {len(licenses)}")
print(f"Active: {active}")
print(f"Bound: {bound}")
```

---

## License Lifecycle

### Suspend License

Temporarily suspend a license (with optional grace period).

```http
POST /api/v1/licenses/{license_id}/suspend
Content-Type: application/json
Authorization: Bearer <token>

{
  "reason": "Payment failed",
  "grace_period_days": 7,
  "message": "Please update your payment method"
}
```

**Notes:**
- Suspended licenses can still be used during grace period
- After grace period, license is automatically revoked (if background-jobs enabled)

### Revoke License

Permanently revoke a license.

```http
POST /api/v1/licenses/{license_id}/revoke
Content-Type: application/json
Authorization: Bearer <token>

{
  "reason": "Terms of service violation"
}
```

**Notes:**
- Clears hardware binding
- Cannot be used again without reinstatement

### Reinstate License

Restore a suspended or revoked license.

```http
POST /api/v1/licenses/{license_id}/reinstate
Content-Type: application/json
Authorization: Bearer <token>

{
  "new_expires_at": "2025-12-31T23:59:59Z",
  "reset_bandwidth": true
}
```

**Notes:**
- Cannot reinstate blacklisted licenses
- Optionally set new expiration and reset usage

### Extend License

Extend the expiration date.

```http
POST /api/v1/licenses/{license_id}/extend
Content-Type: application/json
Authorization: Bearer <token>

{
  "new_expires_at": "2026-12-31T23:59:59Z",
  "reset_bandwidth": false
}
```

### Release Hardware Binding

Force-release a license from its current hardware (admin action).

```http
POST /api/v1/licenses/{license_id}/release
Content-Type: application/json
Authorization: Bearer <token>

{
  "reason": "User requested transfer to new machine"
}
```

**Notes:**
- Records admin release in binding history
- License can then be bound to a different machine

### Blacklist License

Permanently ban a license (cannot be reinstated).

```http
POST /api/v1/licenses/{license_id}/blacklist
Content-Type: application/json
Authorization: Bearer <token>

{
  "reason": "License sharing detected"
}
```

**Notes:**
- Sets `is_blacklisted = true`
- Cannot be reinstated through normal means
- Use for fraud, abuse, or policy violations

### Update Usage

Update bandwidth/quota usage (for metered licenses).

```http
PATCH /api/v1/licenses/{license_id}/usage
Content-Type: application/json
Authorization: Bearer <token>

{
  "bandwidth_used_bytes": 1073741824,
  "reset": false
}
```

**Response:**

```json
{
  "license_id": "...",
  "bandwidth_used_bytes": 1073741824,
  "bandwidth_limit_bytes": 107374182400,
  "quota_exceeded": false,
  "usage_percentage": 1.0
}
```

---

## Token Management

### Create Token

Create a new API token.

```http
POST /api/v1/tokens
Content-Type: application/json
Authorization: Bearer <token>

{
  "name": "billing-service",
  "scopes": ["licenses:read", "licenses:write"],
  "expires_in_days": 365
}
```

**Response:**

```json
{
  "token_id": "tok_123",
  "name": "billing-service",
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "scopes": ["licenses:read", "licenses:write"],
  "expires_at": "2025-01-15T10:30:00Z"
}
```

**Important:** The `token` value is only shown once. Store it securely!

### List Tokens

```http
GET /api/v1/tokens
Authorization: Bearer <token>
```

**Response:**

```json
{
  "tokens": [
    {
      "token_id": "tok_123",
      "name": "billing-service",
      "scopes": ["licenses:read", "licenses:write"],
      "created_at": "2024-01-15T10:30:00Z",
      "expires_at": "2025-01-15T10:30:00Z",
      "last_used_at": "2024-01-16T09:00:00Z"
    }
  ]
}
```

### Revoke Token

```http
DELETE /api/v1/tokens/{token_id}
Authorization: Bearer <token>
```

---

## Error Handling

All errors return a standard format:

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
| `LICENSE_NOT_FOUND` | 404 | License doesn't exist |
| `LICENSE_EXPIRED` | 403 | License has expired |
| `LICENSE_REVOKED` | 403 | License has been revoked |
| `LICENSE_SUSPENDED` | 403 | License is suspended |
| `LICENSE_BLACKLISTED` | 403 | License is blacklisted |
| `ALREADY_BOUND` | 409 | Already bound to another device |
| `NOT_BOUND` | 409 | Not bound to any device |
| `INVALID_REQUEST` | 400 | Request validation failed |
| `MISSING_FIELD` | 400 | Required field missing |
| `MISSING_TOKEN` | 401 | No auth token provided |
| `INVALID_TOKEN` | 401 | Token is invalid |
| `TOKEN_EXPIRED` | 401 | Token has expired |
| `INSUFFICIENT_SCOPE` | 403 | Token lacks required scope |
| `DATABASE_ERROR` | 500 | Database operation failed |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

### Example Error Handling

```python
import requests

response = requests.post(
    "https://license.example.com/api/v1/licenses",
    json={"org_id": "acme"},
    headers={"Authorization": f"Bearer {token}"}
)

if not response.ok:
    error = response.json().get("error", {})
    code = error.get("code")
    message = error.get("message")

    if code == "MISSING_TOKEN":
        print("Authentication required")
    elif code == "INSUFFICIENT_SCOPE":
        print("Token doesn't have permission for this action")
    elif code == "INVALID_REQUEST":
        print(f"Invalid request: {message}")
    else:
        print(f"Error: {message}")
```

---

## Next Steps

- **[Server Deployment Guide](server-deployment.md)** - Production deployment
- **[Advanced Topics](advanced.md)** - Background jobs, custom configuration
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions
