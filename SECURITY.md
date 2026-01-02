# Security Policy

## Supported Versions

We actively support the latest stable release of this repository.

| Version | Supported |
| ------- | --------- |
| latest  | ✅        |
| older   | ❌        |

## Reporting a Vulnerability

If you believe you’ve found a security vulnerability, **do not open a public GitHub Issue**.

Instead, please report it privately using one of the following methods:

### Preferred: GitHub Private Vulnerability Reporting

Use GitHub’s built-in reporting feature (if enabled):

- Go to the repository
- Click **Security**
- Click **Report a vulnerability**

### Alternative: Email

If private reporting is not available, email:
**david@netviper.gr**

Please include:

- A detailed description of the vulnerability
- Steps to reproduce (proof-of-concept if possible)
- Potential impact
- Affected version/commit hash
- Any suggested fix (if you have one)

## Response Timeline

We aim to respond within:

- **48 hours** for acknowledgement
- **7 days** for an initial assessment
- **30 days** for a fix or mitigation (depending on severity)

## Coordinated Disclosure

We support coordinated vulnerability disclosure.

Please allow us reasonable time to investigate and fix the issue before public disclosure.
We will credit reporters who request attribution.

## Scope

This policy applies to:

- The code in this repository
- Published releases
- Build pipelines and default configurations

This policy does **not** cover:

- User-specific deployments or third-party integrations
- Issues caused by modified forks
- Dependency vulnerabilities without an exploitable path in this repo

## Security Best Practices

If you use this code in production, we strongly recommend:

- Keeping dependencies up to date
- Restricting access to secrets/keys
- Using environment variables for configuration
- Enabling CI security checks where available
