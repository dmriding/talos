# Keryx Django Admin Portal Specification

## Overview

The Django Admin Portal manages the backend infrastructure for Keryx's paid tiers, including relay server health monitoring, billing, and customer self-service. **Licensing is handled by the Talos License Server** - Django communicates with Talos to issue, update, and revoke licenses based on payment status and usage.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Django Admin Portal                                    │
│                                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐         │
│  │ Django Admin │  │ Customer     │  │ Health       │  │ Stripe      │         │
│  │ (internal)   │  │ Portal       │  │ Poller       │  │ Webhooks    │         │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬──────┘         │
│         │                 │                 │                  │                │
│         └─────────────────┼─────────────────┼──────────────────┘                │
│                           │                 │                                   │
└───────────────────────────┼─────────────────┼───────────────────────────────────┘
                            │                 │
           ┌────────────────┼─────────────────┼────────────────┐
           │                │                 │                │
           ▼                ▼                 ▼                ▼
    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
    │   Talos     │  │ PostgreSQL  │  │    Redis    │  │   Relay     │
    │  License    │  │  (shared)   │  │ (real-time) │  │  Servers    │
    │   Server    │  └──────▲──────┘  └──────▲──────┘  │ (3 regions) │
    └──────▲──────┘         │                │         └──────▲──────┘
           │                │                │                │
           │ validates      │ reads config   │ writes health  │
           │ licenses       │                │                │
           │                │                │                │
    ┌──────┴──────┐  ┌──────┴──────┐        │         ┌──────┴──────┐
    │   Keryx     │  │    Rust     │        │         │   Keryx     │
    │   CLI       │  │   Signal    │────────┘         │   CLI       │
    │  (client)   │  │   Server    │                  │  (client)   │
    └─────────────┘  └─────────────┘                  └─────────────┘
```

### Communication Flow

```
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│   Stripe    │         │   Django    │         │   Talos     │
│             │         │             │         │  License    │
└──────┬──────┘         └──────┬──────┘         └──────┬──────┘
       │                       │                       │
       │ webhook: payment      │                       │
       │ succeeded             │                       │
       │──────────────────────►│                       │
       │                       │                       │
       │                       │ POST /api/v1/licenses │
       │                       │ { org_id, tier,       │
       │                       │   features, ... }     │
       │                       │──────────────────────►│
       │                       │                       │
       │                       │   { license_key,      │
       │                       │     license_id }      │
       │                       │◄──────────────────────│
       │                       │                       │
       │                       │ store in DB           │
       │                       │ email to customer     │
       │                       │                       │
```

```
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│   Keryx     │         │   Talos     │         │   Django    │
│   CLI       │         │  License    │         │             │
└──────┬──────┘         └──────┬──────┘         └──────┬──────┘
       │                       │                       │
       │ POST /api/v1/client/  │                       │
       │ validate              │                       │
       │ { license_key,        │                       │
       │   hardware_id }       │                       │
       │──────────────────────►│                       │
       │                       │                       │
       │                       │ (Talos checks its DB) │
       │                       │                       │
       │   { valid: true,      │                       │
       │     features: [...],  │                       │
       │     tier: "pro" }     │                       │
       │◄──────────────────────│                       │
       │                       │                       │
```

---

## Service Responsibilities

### Django Admin Portal
- **Payment processing** (Stripe webhooks)
- **Customer portal** (self-service dashboard)
- **Relay routing** (which relays org can access)
- **Usage tracking** (read from Redis, sync to Talos)
- **Talos communication** (issue/update/revoke licenses via HTTP)
- **Admin UI** (manage orgs, users, relays)

**Note:** Relay health polling is handled by the Rust signal server (Phase 7d), not Django.

### Talos License Server
- **License issuance** (Django calls Talos API to create licenses)
- **License validation** (CLI validates directly with Talos)
- **Device management** (track activated devices per license)
- **Feature gating** (validate specific features)
- **Grace period handling** (suspended licenses with countdown)
- **License storage** (Talos maintains license database)

### Rust Signal Server
- **WebSocket signaling** (real-time P2P coordination)
- **Room management** (create/join rooms)
- **Relay token generation** (JWT for relay auth)
- **Relay health polling** (QUIC ping every 30s, cached in memory)
- **Exposes `/health/relays`** (Django/admin can read this for status)

---

## Talos API Contract

Django communicates with Talos via HTTP. All admin endpoints require JWT authentication.

### Authentication

```python
# JWT token generation (Django side)
import jwt
from datetime import datetime, timedelta

def get_talos_jwt():
    payload = {
        "sub": "django-service",
        "iat": datetime.utcnow(),
        "exp": datetime.utcnow() + timedelta(hours=24),
        "scopes": ["licenses:*"]
    }
    return jwt.encode(payload, settings.TALOS_JWT_SECRET, algorithm="HS256")
```

### Licensing Model

- **Organization** can purchase **multiple license keys**
- Each **license key** is bound to **ONE device** at a time
- Users can **release** a key from hardware to **re-bind** elsewhere
- Django creates licenses via Talos when subscriptions are purchased

### Admin Endpoints (called by Django)

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/v1/licenses` | Create single license |
| POST | `/api/v1/licenses/batch` | Create multiple licenses |
| GET | `/api/v1/licenses?org_id={id}` | List org's licenses |
| GET | `/api/v1/licenses/{license_id}` | Get license details |
| PATCH | `/api/v1/licenses/{license_id}` | Update tier/features |
| POST | `/api/v1/licenses/{license_id}/revoke` | Suspend/revoke license |
| POST | `/api/v1/licenses/{license_id}/reinstate` | Reinstate suspended license |
| POST | `/api/v1/licenses/{license_id}/extend` | Extend expiry date |
| POST | `/api/v1/licenses/{license_id}/release` | Admin force unbind from hardware |
| POST | `/api/v1/licenses/{license_id}/blacklist` | Permanently ban |

### Client Endpoints (called by Keryx CLI)

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/v1/client/bind` | Bind license to hardware |
| POST | `/api/v1/client/release` | Release license from hardware |
| POST | `/api/v1/client/validate` | Validate license (must be bound) |
| POST | `/api/v1/client/validate-or-bind` | Validate or auto-bind if unbound |
| POST | `/api/v1/client/validate-feature` | Check specific feature access |
| POST | `/api/v1/client/heartbeat` | Liveness ping |

---

## TalosClient Service

### Implementation with Stubs

```python
# services/talos.py

import jwt
import logging
from datetime import datetime, timedelta
from typing import Optional
from dataclasses import dataclass
from enum import Enum

import requests
from django.conf import settings

logger = logging.getLogger(__name__)


class TalosError(Exception):
    """Base exception for Talos API errors."""
    def __init__(self, code: str, message: str, status_code: int = 500):
        self.code = code
        self.message = message
        self.status_code = status_code
        super().__init__(f"{code}: {message}")


class LicenseStatus(Enum):
    ACTIVE = "active"
    SUSPENDED = "suspended"
    REVOKED = "revoked"
    EXPIRED = "expired"


@dataclass
class License:
    license_id: str
    license_key: str
    org_id: str
    org_name: str
    tier: str
    status: str
    features: list[str]
    limits: dict
    max_devices: int
    active_devices: int
    issued_at: datetime
    expires_at: Optional[datetime]
    bandwidth_used_bytes: int
    bandwidth_limit_bytes: int
    quota_exceeded: bool


class TalosClient:
    """
    Client for communicating with Talos License Server.

    Set TALOS_USE_STUBS=True in settings to use stub responses for development.
    """

    def __init__(self, base_url: str = None, jwt_secret: str = None):
        self.base_url = base_url or settings.TALOS_URL
        self.jwt_secret = jwt_secret or settings.TALOS_JWT_SECRET
        self.use_stubs = getattr(settings, 'TALOS_USE_STUBS', False)
        self._session = requests.Session()

    def _get_auth_headers(self) -> dict:
        """Generate JWT authorization header."""
        payload = {
            "sub": "django-service",
            "iat": datetime.utcnow(),
            "exp": datetime.utcnow() + timedelta(hours=24),
            "scopes": ["licenses:*"]
        }
        token = jwt.encode(payload, self.jwt_secret, algorithm="HS256")
        return {"Authorization": f"Bearer {token}"}

    def _request(self, method: str, endpoint: str, **kwargs) -> dict:
        """Make authenticated request to Talos API."""
        url = f"{self.base_url}{endpoint}"
        headers = self._get_auth_headers()
        headers.update(kwargs.pop("headers", {}))

        try:
            response = self._session.request(
                method, url, headers=headers, timeout=30, **kwargs
            )
            response.raise_for_status()
            return response.json() if response.content else {}
        except requests.exceptions.HTTPError as e:
            error_data = e.response.json().get("error", {})
            raise TalosError(
                code=error_data.get("code", "UNKNOWN_ERROR"),
                message=error_data.get("message", str(e)),
                status_code=e.response.status_code
            )
        except requests.exceptions.RequestException as e:
            logger.error(f"Talos request failed: {e}")
            raise TalosError(
                code="CONNECTION_ERROR",
                message=f"Failed to connect to Talos: {e}"
            )

    # =========================================================================
    # LICENSE CRUD
    # =========================================================================

    def create_license(
        self,
        org_id: str,
        org_name: str,
        tier: str,
        features: list[str],
        expires_at: datetime,
        metadata: dict = None
    ) -> dict:
        """
        Create a single new license. Each license can be bound to one device.

        Note: limits and max_devices are NOT passed - Talos derives these from tier.

        Args:
            org_id: Organization UUID
            org_name: Organization display name
            tier: 'starter', 'pro', 'team', 'enterprise'
            features: List of enabled features ['relay', 'priority_support']
            expires_at: License expiry datetime
            metadata: Optional dict with stripe_customer_id, stripe_subscription_id

        Returns:
            dict with license_id, license_key, issued_at, expires_at
        """
        if self.use_stubs:
            return self._stub_create_license(org_id, org_name, tier, expires_at)

        return self._request("POST", "/api/v1/licenses", json={
            "org_id": str(org_id),
            "org_name": org_name,
            "tier": tier,
            "features": features,
            "expires_at": expires_at.isoformat() if expires_at else None,
            "metadata": metadata or {}
        })

    def create_licenses_batch(
        self,
        org_id: str,
        org_name: str,
        tier: str,
        features: list[str],
        expires_at: datetime,
        count: int,
        metadata: dict = None
    ) -> dict:
        """
        Create multiple licenses at once (for bulk purchases).

        Args:
            org_id: Organization UUID
            org_name: Organization display name
            tier: License tier
            features: List of enabled features
            expires_at: License expiry datetime
            count: Number of licenses to create
            metadata: Optional dict with stripe IDs

        Returns:
            dict with created count and list of license_id/license_key pairs
        """
        if self.use_stubs:
            return self._stub_create_licenses_batch(org_id, org_name, tier, expires_at, count)

        return self._request("POST", "/api/v1/licenses/batch", json={
            "org_id": str(org_id),
            "org_name": org_name,
            "tier": tier,
            "features": features,
            "expires_at": expires_at.isoformat() if expires_at else None,
            "count": count,
            "metadata": metadata or {}
        })

    def list_org_licenses(self, org_id: str) -> dict:
        """List all licenses for an organization."""
        if self.use_stubs:
            return self._stub_list_org_licenses(org_id)

        return self._request("GET", f"/api/v1/licenses?org_id={org_id}")

    def get_license(self, license_id: str) -> dict:
        """Get full license details by ID."""
        if self.use_stubs:
            return self._stub_get_license(license_id)

        return self._request("GET", f"/api/v1/licenses/{license_id}")

    def update_license(
        self,
        license_id: str,
        tier: str = None,
        features: list[str] = None,
        expires_at: datetime = None
    ) -> dict:
        """
        Update license tier, features, or expiry.
        Used for subscription upgrades/downgrades.

        Note: limits and max_devices are NOT passed - Talos derives these from tier.
        """
        if self.use_stubs:
            return self._stub_update_license(license_id, tier)

        payload = {}
        if tier is not None:
            payload["tier"] = tier
        if features is not None:
            payload["features"] = features
        if expires_at is not None:
            payload["expires_at"] = expires_at.isoformat()

        return self._request("PATCH", f"/api/v1/licenses/{license_id}", json=payload)

    # =========================================================================
    # LICENSE LIFECYCLE
    # =========================================================================

    def revoke_license(
        self,
        license_id: str,
        reason: str,
        grace_period_days: int = 7,
        message: str = None
    ) -> dict:
        """
        Suspend or revoke a license.

        Args:
            license_id: License UUID
            reason: 'payment_failed', 'subscription_canceled', 'terms_violation'
            grace_period_days: Days before full revocation (0 = immediate)
            message: Message shown to user on validation failure
        """
        if self.use_stubs:
            return self._stub_revoke_license(license_id, reason, grace_period_days)

        return self._request("POST", f"/api/v1/licenses/{license_id}/revoke", json={
            "reason": reason,
            "grace_period_days": grace_period_days,
            "message": message
        })

    def reinstate_license(
        self,
        license_id: str,
        new_expires_at: datetime = None,
        reset_bandwidth: bool = True
    ) -> dict:
        """
        Reinstate a suspended/revoked license.
        Used when payment retry succeeds.
        """
        if self.use_stubs:
            return self._stub_reinstate_license(license_id)

        payload = {"reset_bandwidth": reset_bandwidth}
        if new_expires_at:
            payload["new_expires_at"] = new_expires_at.isoformat()

        return self._request("POST", f"/api/v1/licenses/{license_id}/reinstate", json=payload)

    def extend_license(
        self,
        license_id: str,
        new_expires_at: datetime,
        reset_bandwidth: bool = True
    ) -> dict:
        """
        Extend license expiry date.
        Called when monthly invoice is paid.
        """
        if self.use_stubs:
            return self._stub_extend_license(license_id, new_expires_at)

        return self._request("POST", f"/api/v1/licenses/{license_id}/extend", json={
            "new_expires_at": new_expires_at.isoformat(),
            "reset_bandwidth": reset_bandwidth
        })

    def update_usage(
        self,
        license_id: str,
        bandwidth_used_bytes: int,
        bandwidth_limit_bytes: int
    ) -> dict:
        """
        Update bandwidth usage stats.
        Django calls this periodically to sync usage from Redis.
        """
        if self.use_stubs:
            return self._stub_update_usage(license_id, bandwidth_used_bytes, bandwidth_limit_bytes)

        return self._request("PATCH", f"/api/v1/licenses/{license_id}/usage", json={
            "bandwidth_used_bytes": bandwidth_used_bytes,
            "bandwidth_limit_bytes": bandwidth_limit_bytes
        })

    def blacklist_license(
        self,
        license_id: str,
        reason: str,
        message: str = None
    ) -> dict:
        """
        Permanently ban an organization.
        Validation always fails after this.
        """
        if self.use_stubs:
            return self._stub_blacklist_license(license_id, reason)

        return self._request("POST", f"/api/v1/licenses/{license_id}/blacklist", json={
            "reason": reason,
            "message": message
        })

    # =========================================================================
    # HARDWARE BINDING (ADMIN)
    # =========================================================================

    def admin_release(self, license_id: str, reason: str = "admin_action") -> dict:
        """
        Admin forcibly unbinds a license from its current hardware.
        Used when user loses access to device and needs to rebind.
        """
        if self.use_stubs:
            return self._stub_admin_release(license_id, reason)

        return self._request("POST", f"/api/v1/licenses/{license_id}/release", json={
            "reason": reason
        })

    # =========================================================================
    # STUB IMPLEMENTATIONS (for development without Talos running)
    # =========================================================================

    def _generate_license_key(self):
        """Generate a fake license key for stubs."""
        import random
        chars = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789'
        segments = [''.join(random.choices(chars, k=4)) for _ in range(4)]
        return f"KERYX-{'-'.join(segments)}"

    def _stub_create_license(self, org_id, org_name, tier, expires_at):
        import uuid

        license_key = self._generate_license_key()
        logger.info(f"[STUB] Created license for org {org_id}: {license_key}")

        return {
            "license_id": str(uuid.uuid4()),
            "license_key": license_key,
            "org_id": str(org_id),
            "org_name": org_name,
            "tier": tier,
            "status": "active",
            "features": ["relay", "priority_support"],
            "is_bound": False,
            "issued_at": datetime.utcnow().isoformat(),
            "expires_at": expires_at.isoformat() if expires_at else None
        }

    def _stub_create_licenses_batch(self, org_id, org_name, tier, expires_at, count):
        import uuid

        licenses = []
        for _ in range(count):
            licenses.append({
                "license_id": str(uuid.uuid4()),
                "license_key": self._generate_license_key()
            })

        logger.info(f"[STUB] Created {count} licenses for org {org_id}")

        return {
            "created": count,
            "licenses": licenses
        }

    def _stub_list_org_licenses(self, org_id):
        import uuid

        logger.info(f"[STUB] List licenses for org {org_id}")
        return {
            "org_id": org_id,
            "org_name": "Stub Organization",
            "total_licenses": 2,
            "bound_licenses": 1,
            "licenses": [
                {
                    "license_id": str(uuid.uuid4()),
                    "license_key": "KERYX-STUB-AAAA-BBBB-CCCC",
                    "tier": "pro",
                    "status": "active",
                    "is_bound": True,
                    "device_name": "Stub Device",
                    "bound_at": datetime.utcnow().isoformat(),
                    "last_seen_at": datetime.utcnow().isoformat(),
                    "expires_at": (datetime.utcnow() + timedelta(days=30)).isoformat()
                },
                {
                    "license_id": str(uuid.uuid4()),
                    "license_key": "KERYX-STUB-DDDD-EEEE-FFFF",
                    "tier": "pro",
                    "status": "active",
                    "is_bound": False,
                    "device_name": None,
                    "bound_at": None,
                    "last_seen_at": None,
                    "expires_at": (datetime.utcnow() + timedelta(days=30)).isoformat()
                }
            ]
        }

    def _stub_get_license(self, license_id):
        logger.info(f"[STUB] Get license {license_id}")
        return {
            "license_id": license_id,
            "license_key": "KERYX-STUB-TEST-XXXX-YYYY",
            "org_id": "stub-org-id",
            "org_name": "Stub Organization",
            "tier": "pro",
            "status": "active",
            "features": ["relay", "priority_support"],
            "is_bound": True,
            "hardware_id": "sha256:stub123...",
            "device_name": "Stub Device",
            "bound_at": datetime.utcnow().isoformat(),
            "last_seen_at": datetime.utcnow().isoformat(),
            "issued_at": datetime.utcnow().isoformat(),
            "expires_at": (datetime.utcnow() + timedelta(days=30)).isoformat(),
            "metadata": {}
        }

    def _stub_update_license(self, license_id, tier):
        logger.info(f"[STUB] Updated license {license_id} to tier {tier}")
        return self._stub_get_license(license_id)

    def _stub_revoke_license(self, license_id, reason, grace_period_days):
        logger.info(f"[STUB] Revoked license {license_id}: {reason} (grace: {grace_period_days} days)")
        grace_ends = datetime.utcnow() + timedelta(days=grace_period_days) if grace_period_days > 0 else None
        return {
            "license_id": license_id,
            "status": "suspended" if grace_period_days > 0 else "revoked",
            "revoke_reason": reason,
            "grace_period_ends_at": grace_ends.isoformat() if grace_ends else None
        }

    def _stub_reinstate_license(self, license_id):
        logger.info(f"[STUB] Reinstated license {license_id}")
        result = self._stub_get_license(license_id)
        result["status"] = "active"
        return result

    def _stub_extend_license(self, license_id, new_expires_at):
        logger.info(f"[STUB] Extended license {license_id} to {new_expires_at}")
        result = self._stub_get_license(license_id)
        result["expires_at"] = new_expires_at.isoformat()
        return result

    def _stub_update_usage(self, license_id, used, limit):
        logger.info(f"[STUB] Updated usage for {license_id}: {used}/{limit} bytes")
        exceeded = used >= limit
        return {
            "license_id": license_id,
            "bandwidth_used_bytes": used,
            "bandwidth_limit_bytes": limit,
            "quota_exceeded": exceeded,
            "quota_restricted_features": ["relay"] if exceeded else []
        }

    def _stub_blacklist_license(self, license_id, reason):
        logger.info(f"[STUB] Blacklisted license {license_id}: {reason}")
        return {
            "license_id": license_id,
            "status": "revoked",
            "is_blacklisted": True,
            "blacklist_reason": reason
        }

    def _stub_admin_release(self, license_id, reason):
        logger.info(f"[STUB] Admin released license {license_id}: {reason}")
        return {
            "license_id": license_id,
            "previous_hardware_id": "sha256:stub123...",
            "previous_device_name": "Stub Device",
            "status": "unbound",
            "message": "License unbound by administrator."
        }


# Singleton instance
_talos_client = None


def get_talos_client() -> TalosClient:
    """Get or create singleton TalosClient instance."""
    global _talos_client
    if _talos_client is None:
        _talos_client = TalosClient()
    return _talos_client
```

---

## Tier Configuration

```python
# config/tiers.py

from dataclasses import dataclass
from typing import Optional


@dataclass
class TierConfig:
    name: str
    display_name: str
    features: list[str]
    bandwidth_gb: int
    max_users: int
    max_devices: int
    price_monthly: Optional[int]  # cents, None = custom pricing
    stripe_price_id: Optional[str]


TIERS = {
    "free": TierConfig(
        name="free",
        display_name="Free",
        features=[],
        bandwidth_gb=0,
        max_users=1,
        max_devices=1,
        price_monthly=0,
        stripe_price_id=None
    ),
    "starter": TierConfig(
        name="starter",
        display_name="Starter",
        features=["relay"],
        bandwidth_gb=50,
        max_users=1,
        max_devices=2,
        price_monthly=None,  # TBD
        stripe_price_id=None  # settings.STRIPE_PRICE_STARTER
    ),
    "pro": TierConfig(
        name="pro",
        display_name="Pro",
        features=["relay", "priority_support"],
        bandwidth_gb=500,
        max_users=5,
        max_devices=5,
        price_monthly=None,  # TBD
        stripe_price_id=None  # settings.STRIPE_PRICE_PRO
    ),
    "team": TierConfig(
        name="team",
        display_name="Team",
        features=["relay", "priority_support", "dedicated_relay"],
        bandwidth_gb=2000,
        max_users=25,
        max_devices=25,
        price_monthly=None,  # TBD
        stripe_price_id=None  # settings.STRIPE_PRICE_TEAM
    ),
    "enterprise": TierConfig(
        name="enterprise",
        display_name="Enterprise",
        features=["relay", "priority_support", "dedicated_relay", "sla", "custom_integration"],
        bandwidth_gb=0,  # Custom
        max_users=0,  # Custom
        max_devices=100,
        price_monthly=None,  # Custom
        stripe_price_id=None
    ),
}


def get_tier_config(tier_name: str) -> TierConfig:
    """Get tier configuration by name."""
    return TIERS.get(tier_name, TIERS["free"])


def get_tier_limits(tier_name: str) -> dict:
    """Get limits dict for Talos API."""
    config = get_tier_config(tier_name)
    return {
        "bandwidth_gb": config.bandwidth_gb,
        "max_users": config.max_users
    }
```

---

## Existing Database Schema

Django will manage these **existing tables** (already deployed to production):

| Table | Purpose |
|-------|---------|
| `organizations` | Billing entities with tier, bandwidth limits, Stripe IDs |
| `users` | User accounts linked to orgs, password hash, JWT token versioning |
| `sessions` | Active WebSocket connections (peer tracking) |
| `rooms` | P2P room codes with 23hr expiry |
| `usage_records` | Transfer logs (bytes, speed, connection type) for billing |
| `relay_servers` | Relay server registry (public/private, region, health status) |
| `relay_sessions` | Relay usage tracking for bandwidth billing |

### Database Additions

```sql
-- Stripe integration on organizations
ALTER TABLE organizations ADD COLUMN stripe_customer_id TEXT;
ALTER TABLE organizations ADD COLUMN stripe_subscription_id TEXT;
ALTER TABLE organizations ADD COLUMN subscription_status TEXT;  -- 'active', 'past_due', 'canceled'

-- New table: organization_licenses (many licenses per org)
-- This is a Django-side tracking table - Talos is the source of truth
CREATE TABLE organization_licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- Talos reference (Talos owns the license, we just track it)
    talos_license_id UUID NOT NULL UNIQUE,
    license_key TEXT NOT NULL,              -- KERYX-XXXX-XXXX-XXXX (cached from Talos)

    -- Cached status (synced from Talos periodically)
    status TEXT NOT NULL DEFAULT 'active',  -- 'active', 'suspended', 'revoked', 'expired'
    is_bound BOOLEAN NOT NULL DEFAULT FALSE,
    device_name TEXT,                       -- Cached from Talos for display

    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_org_licenses_org_id ON organization_licenses(org_id);
CREATE INDEX idx_org_licenses_talos_id ON organization_licenses(talos_license_id);
```

---

## Priority 1: Infrastructure Foundation

### 1.1 Relay Server Health (Read from Signal Server)

**Note:** Relay health polling is already implemented in the Rust signal server (Phase 7d).
The signal server polls relays via QUIC ping every 30 seconds and exposes the results via `GET /health/relays`.

Django should **read** relay health from the signal server, not poll relays directly.

```python
# services/relay_health.py

import requests
import logging
from django.conf import settings

logger = logging.getLogger(__name__)


def get_relay_health() -> dict:
    """
    Fetch relay health from signal server.
    Signal server does the actual QUIC probing.
    """
    try:
        response = requests.get(
            f"{settings.SIGNAL_SERVER_URL}/health/relays",
            timeout=5
        )
        response.raise_for_status()
        return response.json()
    except Exception as e:
        logger.error(f"Failed to fetch relay health: {e}")
        return {}


def get_healthy_relays() -> list:
    """Get list of healthy relay URLs."""
    health = get_relay_health()
    return [
        relay for relay in health.get("relays", [])
        if relay.get("status") == "healthy"
    ]
```

For admin display, Django can fetch and display relay health on demand from the signal server.

### 1.2 Django Admin Models

```python
# apps/organizations/admin.py

from django.contrib import admin
from unfold.admin import ModelAdmin
from .models import Organization


@admin.register(Organization)
class OrganizationAdmin(ModelAdmin):
    list_display = [
        'name', 'tier', 'license_status', 'bandwidth_display',
        'subscription_status', 'created_at'
    ]
    list_filter = ['tier', 'license_status', 'subscription_status']
    search_fields = ['name', 'talos_license_key', 'stripe_customer_id']
    readonly_fields = ['talos_license_id', 'talos_license_key', 'created_at', 'updated_at']

    fieldsets = (
        ('Organization', {
            'fields': ('name', 'tier')
        }),
        ('License', {
            'fields': ('talos_license_id', 'talos_license_key', 'license_status')
        }),
        ('Bandwidth', {
            'fields': ('relay_bandwidth_used_bytes', 'relay_bandwidth_limit_bytes')
        }),
        ('Stripe', {
            'fields': ('stripe_customer_id', 'stripe_subscription_id', 'subscription_status')
        }),
        ('Metadata', {
            'fields': ('created_at', 'updated_at', 'notes'),
            'classes': ('collapse',)
        }),
    )

    actions = ['suspend_license', 'reinstate_license']

    def bandwidth_display(self, obj):
        used_gb = obj.relay_bandwidth_used_bytes / (1024 ** 3)
        limit_gb = obj.relay_bandwidth_limit_bytes / (1024 ** 3)
        return f"{used_gb:.1f} / {limit_gb:.0f} GB"
    bandwidth_display.short_description = 'Bandwidth'

    @admin.action(description='Suspend selected licenses')
    def suspend_license(self, request, queryset):
        from services.talos import get_talos_client
        talos = get_talos_client()
        for org in queryset.exclude(talos_license_id__isnull=True):
            talos.revoke_license(
                str(org.talos_license_id),
                reason="admin_action",
                grace_period_days=0,
                message="License suspended by administrator."
            )
            org.license_status = 'suspended'
            org.save()

    @admin.action(description='Reinstate selected licenses')
    def reinstate_license(self, request, queryset):
        from services.talos import get_talos_client
        talos = get_talos_client()
        for org in queryset.filter(license_status='suspended'):
            talos.reinstate_license(str(org.talos_license_id))
            org.license_status = 'active'
            org.save()
```

```python
# apps/relays/admin.py

from django.contrib import admin
from unfold.admin import ModelAdmin
from .models import RelayServer


@admin.register(RelayServer)
class RelayServerAdmin(ModelAdmin):
    list_display = [
        'name', 'region', 'url', 'is_active', 'is_healthy',
        'connections_display', 'last_health_check'
    ]
    list_filter = ['region', 'is_active', 'is_healthy', 'is_public']
    search_fields = ['name', 'url']

    fieldsets = (
        ('Server', {
            'fields': ('name', 'region', 'url')
        }),
        ('Status', {
            'fields': ('is_active', 'is_public', 'is_healthy', 'current_connections', 'max_connections')
        }),
        ('Owner', {
            'fields': ('owner_org',),
            'classes': ('collapse',)
        }),
        ('Notes', {
            'fields': ('notes',),
            'classes': ('collapse',)
        }),
    )

    actions = ['test_health', 'enable_relays', 'disable_relays']

    def connections_display(self, obj):
        return f"{obj.current_connections or 0} / {obj.max_connections}"
    connections_display.short_description = 'Connections'

    @admin.action(description='Test health of selected relays')
    def test_health(self, request, queryset):
        from tasks.relay_health import poll_relay_health
        poll_relay_health.delay()
        self.message_user(request, "Health check triggered.")

    @admin.action(description='Enable selected relays')
    def enable_relays(self, request, queryset):
        queryset.update(is_active=True)

    @admin.action(description='Disable selected relays')
    def disable_relays(self, request, queryset):
        queryset.update(is_active=False)
```

---

## Priority 2: Talos Integration

### 2.1 Usage Sync Background Task

```python
# tasks/usage_sync.py

from celery import shared_task
import redis
from django.conf import settings

from apps.organizations.models import Organization
from services.talos import get_talos_client

redis_client = redis.from_url(settings.REDIS_URL)


@shared_task
def sync_usage_to_talos():
    """
    Sync bandwidth usage from Redis to Talos.
    Runs every 5 minutes via Celery Beat.
    """
    talos = get_talos_client()

    orgs_with_licenses = Organization.objects.exclude(
        talos_license_id__isnull=True
    ).exclude(license_status='none')

    for org in orgs_with_licenses:
        # Get usage from Redis
        usage_key = f"relay:usage:{org.id}:bytes"
        usage_bytes = int(redis_client.get(usage_key) or 0)

        # Update org in database
        org.relay_bandwidth_used_bytes = usage_bytes
        org.save(update_fields=['relay_bandwidth_used_bytes'])

        # Update Talos
        try:
            result = talos.update_usage(
                license_id=str(org.talos_license_id),
                bandwidth_used_bytes=usage_bytes,
                bandwidth_limit_bytes=org.relay_bandwidth_limit_bytes
            )

            # Check if quota was exceeded
            if result.get('quota_exceeded') and not org.quota_exceeded_notified:
                send_quota_exceeded_email(org)
                org.quota_exceeded_notified = True
                org.save(update_fields=['quota_exceeded_notified'])

        except Exception as e:
            # Log but don't fail - Talos might be temporarily unavailable
            import logging
            logging.error(f"Failed to sync usage to Talos for org {org.id}: {e}")


def send_quota_exceeded_email(org):
    """Send email notification when bandwidth quota is exceeded."""
    # TODO: Implement email sending
    pass
```

---

## Priority 3: Payment Processing (Stripe)

### 3.1 Stripe Webhook Handler

```python
# views/webhooks.py

import stripe
import logging
from datetime import datetime, timedelta

from django.conf import settings
from django.http import HttpResponse
from django.views.decorators.csrf import csrf_exempt
from django.views.decorators.http import require_POST

from apps.organizations.models import Organization
from config.tiers import get_tier_config, get_tier_limits, TIERS
from services.talos import get_talos_client, TalosError
from services.email import send_license_email, send_payment_failed_email

logger = logging.getLogger(__name__)

stripe.api_key = settings.STRIPE_SECRET_KEY


@csrf_exempt
@require_POST
def stripe_webhook(request):
    payload = request.body
    sig_header = request.META.get('HTTP_STRIPE_SIGNATURE')

    try:
        event = stripe.Webhook.construct_event(
            payload, sig_header, settings.STRIPE_WEBHOOK_SECRET
        )
    except ValueError:
        return HttpResponse(status=400)
    except stripe.error.SignatureVerificationError:
        return HttpResponse(status=400)

    talos = get_talos_client()

    handlers = {
        'checkout.session.completed': handle_checkout_completed,
        'customer.subscription.updated': handle_subscription_updated,
        'customer.subscription.deleted': handle_subscription_deleted,
        'invoice.paid': handle_invoice_paid,
        'invoice.payment_failed': handle_payment_failed,
    }

    handler = handlers.get(event['type'])
    if handler:
        try:
            handler(event['data']['object'], talos)
        except Exception as e:
            logger.exception(f"Webhook handler failed for {event['type']}: {e}")
            # Return 200 to prevent Stripe retries for handled events
            # Log for manual investigation

    return HttpResponse(status=200)


def handle_checkout_completed(session, talos):
    """
    New subscription created - issue license(s) via Talos.
    Creates multiple licenses for tiers that include multiple seats.
    """
    from apps.organizations.models import Organization, OrganizationLicense

    # Get or create organization
    org = get_or_create_org_from_session(session)
    tier = get_tier_from_session(session)
    tier_config = get_tier_config(tier)

    # Calculate expiry (1 month + buffer)
    expires_at = datetime.utcnow() + timedelta(days=32)

    # Determine how many licenses to create based on tier
    # Individual tiers get 1 license, team tiers get multiple
    license_count = get_license_count_for_tier(tier, session)

    metadata = {
        "stripe_customer_id": session['customer'],
        "stripe_subscription_id": session['subscription']
    }

    try:
        if license_count == 1:
            # Single license
            result = talos.create_license(
                org_id=str(org.id),
                org_name=org.name,
                tier=tier,
                features=tier_config.features,
                expires_at=expires_at,
                metadata=metadata
            )
            license_keys = [result['license_key']]
            license_ids = [(result['license_id'], result['license_key'])]
        else:
            # Batch create for team/enterprise
            result = talos.create_licenses_batch(
                org_id=str(org.id),
                org_name=org.name,
                tier=tier,
                features=tier_config.features,
                expires_at=expires_at,
                count=license_count,
                metadata=metadata
            )
            license_keys = [lic['license_key'] for lic in result['licenses']]
            license_ids = [(lic['license_id'], lic['license_key']) for lic in result['licenses']]

        # Create OrganizationLicense records for each license
        for talos_license_id, license_key in license_ids:
            OrganizationLicense.objects.create(
                org=org,
                talos_license_id=talos_license_id,
                license_key=license_key,
                status='active'
            )

        # Update organization
        org.tier = tier
        org.relay_bandwidth_limit_bytes = tier_config.bandwidth_gb * (1024 ** 3)
        org.stripe_customer_id = session['customer']
        org.stripe_subscription_id = session['subscription']
        org.subscription_status = 'active'
        org.save()

        # Email license key(s) to customer
        send_license_email(org, license_keys)

        logger.info(f"Issued {len(license_keys)} license(s) for org {org.id}")

    except TalosError as e:
        logger.error(f"Failed to issue license for org {org.id}: {e}")
        raise


def handle_subscription_updated(subscription, talos):
    """
    Subscription changed (upgrade/downgrade) - update all org licenses.
    """
    from apps.organizations.models import Organization, OrganizationLicense

    try:
        org = Organization.objects.get(stripe_subscription_id=subscription['id'])
    except Organization.DoesNotExist:
        logger.warning(f"No org found for subscription {subscription['id']}")
        return

    new_tier = get_tier_from_subscription(subscription)
    tier_config = get_tier_config(new_tier)

    # Update all licenses for this org
    org_licenses = OrganizationLicense.objects.filter(org=org)
    for org_license in org_licenses:
        try:
            talos.update_license(
                license_id=str(org_license.talos_license_id),
                tier=new_tier,
                features=tier_config.features
            )
            logger.info(f"License {org_license.license_key} updated to tier={new_tier}")

        except TalosError as e:
            logger.error(f"Failed to update license {org_license.talos_license_id}: {e}")
            # Continue updating other licenses even if one fails

    org.tier = new_tier
    org.relay_bandwidth_limit_bytes = tier_config.bandwidth_gb * (1024 ** 3)
    org.save()

    logger.info(f"Updated {org_licenses.count()} license(s) for org {org.id}: tier={new_tier}")


def handle_subscription_deleted(subscription, talos):
    """
    Subscription canceled - revoke all org licenses.
    """
    from apps.organizations.models import Organization, OrganizationLicense

    try:
        org = Organization.objects.get(stripe_subscription_id=subscription['id'])
    except Organization.DoesNotExist:
        return

    # Revoke all licenses for this org
    org_licenses = OrganizationLicense.objects.filter(org=org)
    for org_license in org_licenses:
        try:
            talos.revoke_license(
                license_id=str(org_license.talos_license_id),
                reason="subscription_canceled",
                grace_period_days=0,
                message="Your subscription has been canceled."
            )
            org_license.status = 'revoked'
            org_license.save()

        except TalosError as e:
            logger.error(f"Failed to revoke license {org_license.talos_license_id}: {e}")

    org.tier = 'free'
    org.subscription_status = 'canceled'
    org.relay_bandwidth_limit_bytes = 0
    org.save()

    logger.info(f"Revoked {org_licenses.count()} license(s) for org {org.id}: subscription canceled")


def handle_invoice_paid(invoice, talos):
    """
    Monthly invoice paid - extend all org licenses and reset bandwidth.
    """
    from apps.organizations.models import Organization, OrganizationLicense

    try:
        org = Organization.objects.get(stripe_customer_id=invoice['customer'])
    except Organization.DoesNotExist:
        return

    new_expires = datetime.utcnow() + timedelta(days=32)

    # Extend all active licenses for this org
    org_licenses = OrganizationLicense.objects.filter(org=org, status='active')
    for org_license in org_licenses:
        try:
            talos.extend_license(
                license_id=str(org_license.talos_license_id),
                new_expires_at=new_expires,
                reset_bandwidth=True
            )

        except TalosError as e:
            logger.error(f"Failed to extend license {org_license.talos_license_id}: {e}")

    # Reset local bandwidth tracking
    org.relay_bandwidth_used_bytes = 0
    org.billing_cycle_start = datetime.utcnow()
    org.quota_exceeded_notified = False
    org.save()

    # Reset Redis counter
    import redis
    redis_client = redis.from_url(settings.REDIS_URL)
    redis_client.set(f"relay:usage:{org.id}:bytes", 0)

    logger.info(f"Extended {org_licenses.count()} license(s) for org {org.id}: expires={new_expires}")


def handle_payment_failed(invoice, talos):
    """
    Payment failed - suspend all org licenses with grace period.
    """
    from apps.organizations.models import Organization, OrganizationLicense

    try:
        org = Organization.objects.get(stripe_customer_id=invoice['customer'])
    except Organization.DoesNotExist:
        return

    # Suspend all licenses for this org
    org_licenses = OrganizationLicense.objects.filter(org=org, status='active')
    for org_license in org_licenses:
        try:
            talos.revoke_license(
                license_id=str(org_license.talos_license_id),
                reason="payment_failed",
                grace_period_days=7,
                message="Payment failed. Please update your payment method within 7 days."
            )
            org_license.status = 'suspended'
            org_license.save()

        except TalosError as e:
            logger.error(f"Failed to suspend license {org_license.talos_license_id}: {e}")

    org.subscription_status = 'past_due'
    org.save()

    send_payment_failed_email(org)

    logger.info(f"Suspended {org_licenses.count()} license(s) for org {org.id}: payment failed")


# Helper functions

def get_license_count_for_tier(tier: str, session: dict) -> int:
    """
    Determine how many licenses to create based on tier.
    For team tiers, this may come from the checkout session quantity.
    """
    # Default license counts per tier
    tier_defaults = {
        'starter': 1,
        'pro': 1,
        'team': 5,       # Team includes 5 licenses by default
        'enterprise': 10  # Enterprise default, usually customized
    }

    # Check if quantity was specified in checkout (for per-seat pricing)
    line_items = session.get('line_items', {}).get('data', [])
    if line_items:
        quantity = line_items[0].get('quantity', 1)
        if quantity > 1:
            return quantity

    return tier_defaults.get(tier, 1)


def get_or_create_org_from_session(session):
    """Get or create organization from Stripe checkout session."""
    customer_id = session['customer']
    customer_email = session.get('customer_email') or session.get('customer_details', {}).get('email')

    org, created = Organization.objects.get_or_create(
        stripe_customer_id=customer_id,
        defaults={
            'name': customer_email.split('@')[0] if customer_email else 'Unknown',
            'tier': 'free'
        }
    )
    return org


def get_tier_from_session(session):
    """Extract tier from checkout session line items."""
    # Map Stripe price IDs to tiers
    price_to_tier = {
        settings.STRIPE_PRICE_STARTER: 'starter',
        settings.STRIPE_PRICE_PRO: 'pro',
        settings.STRIPE_PRICE_TEAM: 'team',
    }

    # Get price ID from line items
    line_items = session.get('line_items', {}).get('data', [])
    if line_items:
        price_id = line_items[0].get('price', {}).get('id')
        return price_to_tier.get(price_id, 'starter')

    return 'starter'


def get_tier_from_subscription(subscription):
    """Extract tier from subscription object."""
    price_to_tier = {
        settings.STRIPE_PRICE_STARTER: 'starter',
        settings.STRIPE_PRICE_PRO: 'pro',
        settings.STRIPE_PRICE_TEAM: 'team',
    }

    items = subscription.get('items', {}).get('data', [])
    if items:
        price_id = items[0].get('price', {}).get('id')
        return price_to_tier.get(price_id, 'starter')

    return 'starter'
```

---

## Priority 4: Customer Portal

### 4.1 Dashboard Views

```python
# apps/portal/views.py

from django.shortcuts import render, redirect
from django.contrib.auth.decorators import login_required
from django.http import JsonResponse

from services.talos import get_talos_client


@login_required
def dashboard(request):
    """Customer dashboard overview."""
    org = request.user.organization

    context = {
        'org': org,
        'tier_display': org.get_tier_display(),
        'bandwidth_used_gb': org.relay_bandwidth_used_bytes / (1024 ** 3),
        'bandwidth_limit_gb': org.relay_bandwidth_limit_bytes / (1024 ** 3),
        'bandwidth_percent': (
            (org.relay_bandwidth_used_bytes / org.relay_bandwidth_limit_bytes * 100)
            if org.relay_bandwidth_limit_bytes > 0 else 0
        ),
        'license_key': org.talos_license_key,
        'license_status': org.license_status,
    }

    return render(request, 'portal/dashboard.html', context)


@login_required
def licenses(request):
    """List and manage organization licenses."""
    org = request.user.organization

    talos = get_talos_client()
    licenses_data = talos.list_org_licenses(str(org.id))

    context = {
        'licenses': licenses_data.get('licenses', []),
        'total_licenses': licenses_data.get('total_licenses', 0),
        'bound_licenses': licenses_data.get('bound_licenses', 0),
    }

    return render(request, 'portal/licenses.html', context)


@login_required
def release_license(request, license_id):
    """Admin releases a license from its bound hardware."""
    if request.method != 'POST':
        return JsonResponse({'error': 'Method not allowed'}, status=405)

    org = request.user.organization

    # Verify license belongs to this org (fetch license first)
    talos = get_talos_client()
    license_data = talos.get_license(license_id)

    if license_data.get('org_id') != str(org.id):
        return JsonResponse({'error': 'Unauthorized'}, status=403)

    result = talos.admin_release(license_id, reason="user_portal_request")

    return JsonResponse({
        'success': True,
        'message': 'License released. It can now be bound to a new device.'
    })


@login_required
def billing(request):
    """Billing page with Stripe portal link."""
    import stripe
    from django.conf import settings

    org = request.user.organization

    # Create Stripe billing portal session
    portal_url = None
    if org.stripe_customer_id:
        stripe.api_key = settings.STRIPE_SECRET_KEY
        session = stripe.billing_portal.Session.create(
            customer=org.stripe_customer_id,
            return_url=request.build_absolute_uri('/portal/billing/')
        )
        portal_url = session.url

    context = {
        'org': org,
        'portal_url': portal_url,
    }

    return render(request, 'portal/billing.html', context)
```

---

## Priority 5: Relay Routing

### 5.1 Relay Selection API

```python
# apps/api/views.py

import json
import jwt
import redis
from functools import wraps
from django.conf import settings
from django.http import JsonResponse
from django.views.decorators.http import require_GET

from apps.organizations.models import Organization

redis_client = redis.from_url(settings.REDIS_URL)


def require_service_auth(view_func):
    """
    Decorator requiring JWT service authentication.
    Used for internal APIs called by Rust signal server.
    """
    @wraps(view_func)
    def wrapper(request, *args, **kwargs):
        auth_header = request.META.get('HTTP_AUTHORIZATION', '')
        if not auth_header.startswith('Bearer '):
            return JsonResponse({'error': 'missing_auth'}, status=401)

        token = auth_header[7:]
        try:
            payload = jwt.decode(
                token,
                settings.SERVICE_JWT_SECRET,
                algorithms=['HS256'],
                audience='keryx-internal'
            )
            # Verify it's a service token (not user token)
            if payload.get('type') != 'service':
                return JsonResponse({'error': 'invalid_token_type'}, status=403)

        except jwt.ExpiredSignatureError:
            return JsonResponse({'error': 'token_expired'}, status=401)
        except jwt.InvalidTokenError:
            return JsonResponse({'error': 'invalid_token'}, status=401)

        return view_func(request, *args, **kwargs)
    return wrapper


@require_GET
@require_service_auth
def available_relays(request):
    """
    Get available relays for an organization.
    Called by Rust signal server (requires service JWT).
    """
    org_id = request.GET.get('org_id')
    if not org_id:
        return JsonResponse({'error': 'org_id required'}, status=400)

    try:
        org = Organization.objects.get(id=org_id)
    except Organization.DoesNotExist:
        return JsonResponse({'error': 'org_not_found'}, status=404)

    # Check license status
    if org.license_status != 'active':
        return JsonResponse({
            'error': 'license_inactive',
            'message': 'License is not active',
            'relays': []
        })

    # Check if org has relay access
    if org.tier == 'free':
        return JsonResponse({
            'error': 'relay_not_available',
            'message': 'Upgrade to access relay servers',
            'relays': []
        })

    # Check bandwidth quota
    if org.relay_bandwidth_used_bytes >= org.relay_bandwidth_limit_bytes:
        return JsonResponse({
            'error': 'bandwidth_exceeded',
            'message': 'Bandwidth quota exceeded',
            'relays': []
        })

    # Get available relays
    from apps.relays.models import RelayServer
    from django.db.models import Q

    relays = RelayServer.objects.filter(is_active=True).filter(
        Q(is_public=True) | Q(owner_org_id=org_id)
    )

    healthy_relays = []
    for relay in relays:
        health_key = f"relay:health:{relay.region}"
        health_data = redis_client.get(health_key)

        if health_data:
            health = json.loads(health_data)
            if health.get('healthy'):
                healthy_relays.append({
                    'url': relay.url,
                    'region': relay.region,
                    'latency_ms': health.get('latency_ms'),
                    'load_percent': (
                        health.get('connections', 0) / relay.max_connections * 100
                        if relay.max_connections > 0 else 0
                    )
                })

    # Sort by latency
    healthy_relays.sort(key=lambda r: r.get('latency_ms', 9999))

    return JsonResponse({
        'relays': healthy_relays,
        'bandwidth_remaining_bytes': org.relay_bandwidth_limit_bytes - org.relay_bandwidth_used_bytes
    })
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Framework | Django 5.x |
| Admin UI | Django Admin + django-unfold (modern styling) |
| API | Django REST Framework |
| Background Tasks | Celery + Redis (or Django-Q) |
| Payments | stripe-python |
| Cache | redis-py |
| Database | psycopg (PostgreSQL 16) |
| Talos Client | requests + PyJWT |

---

## Environment Configuration

```env
# Database (same instance as Rust signal server)
DATABASE_URL=postgres://keryx:xxx@localhost/keryx_signal

# Redis
REDIS_URL=redis://localhost:6379

# Talos License Server
TALOS_URL=http://localhost:8080
TALOS_JWT_SECRET=<shared-secret-with-talos>
TALOS_USE_STUBS=True  # Set to False when Talos is running

# Stripe
STRIPE_SECRET_KEY=sk_live_xxx
STRIPE_WEBHOOK_SECRET=whsec_xxx
STRIPE_PRICE_STARTER=price_xxx
STRIPE_PRICE_PRO=price_xxx
STRIPE_PRICE_TEAM=price_xxx

# Signal Server (for relay health API)
SIGNAL_SERVER_URL=https://signal.netviper.cloud

# JWT Secrets
RELAY_JWT_SECRET=<shared-with-relay-servers>
SERVICE_JWT_SECRET=<shared-with-signal-server>  # For internal API auth

# Django
SECRET_KEY=<django-secret>
DEBUG=False
ALLOWED_HOSTS=admin.netviper.cloud
```

---

## Redis Key Schema

```
# Relay health (written by Django poller, read by Rust signal server)
relay:health:{region}           -> JSON { url, latency_ms, connections, healthy, ... }
                                   TTL: 60 seconds

# Usage tracking (written by relay servers, read by Django for billing)
relay:usage:{org_id}:bytes      -> Integer (INCRBY for cumulative bytes)
relay:usage:{org_id}:month      -> String (current month for reset detection)

# Active sessions (written by relay servers, read by Django for stats)
relay:sessions:{region}         -> Hash { room_code -> session_json }
```

---

## Celery Beat Schedule

```python
# config/celery.py

from celery.schedules import crontab

CELERY_BEAT_SCHEDULE = {
    # Note: Relay health polling is done by the Rust signal server, not Django
    'sync-usage-to-talos': {
        'task': 'tasks.usage_sync.sync_usage_to_talos',
        'schedule': 300.0,  # Every 5 minutes
    },
}
```

---

## Checklist

### Priority 1: Infrastructure
- [ ] Django project setup with PostgreSQL connection
- [ ] Django Admin models for all existing tables
- [ ] Celery/Django-Q setup for background tasks
- [ ] Redis integration for usage tracking
- [ ] Signal server URL config for reading relay health

### Priority 2: Talos Integration
- [ ] TalosClient service class with stubs
- [ ] Tier configuration module
- [ ] Create license(s) on checkout complete (batch for multi-seat)
- [ ] List org licenses
- [ ] Update licenses on tier change
- [ ] Revoke licenses on subscription cancel
- [ ] Suspend licenses on payment failure
- [ ] Extend licenses on invoice paid
- [ ] Admin release (force unbind) functionality
- [ ] Blacklist/ban functionality

### Priority 3: Payments
- [ ] Stripe webhook endpoint
- [ ] Handle checkout.session.completed -> Talos issue
- [ ] Handle subscription.updated -> Talos update
- [ ] Handle subscription.deleted -> Talos revoke
- [ ] Handle invoice.paid -> Talos extend
- [ ] Handle invoice.payment_failed -> Talos suspend

### Priority 4: Customer Portal
- [ ] User authentication (login/logout)
- [ ] Dashboard overview page
- [ ] Licenses management page (list, view binding status, release)
- [ ] Usage statistics page
- [ ] Billing/upgrade page (Stripe portal, add/remove licenses)

### Priority 5: Relay Routing
- [ ] Available relays API endpoint
- [ ] Relay token generation endpoint
- [ ] Bandwidth quota enforcement

---

## Notes

- **Talos is the license authority** - Django never validates licenses directly, only tells Talos what to do
- **Keryx CLI validates with Talos** - clients talk to Talos for license validation, not Django
- **Django manages the business logic** - payments, usage tracking, relay routing decisions
- **TALOS_USE_STUBS=True** - Start development with stubs, switch to real Talos when ready
- **All tables already exist in PostgreSQL** - Django connects to existing schema
- **Services communicate via**: PostgreSQL (shared data), Redis (real-time), HTTP APIs (Talos)