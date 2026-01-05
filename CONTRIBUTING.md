# Contributing to Talos

Thank you for your interest in contributing to **Talos**! We welcome contributions of all kinds, including bug fixes, feature enhancements, documentation improvements, and more.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

1. **Fork the repository**: Create a personal fork of the repository on GitHub.
2. **Clone the fork**: Clone your fork locally to your machine.
   ```bash
   git clone https://github.com/yourusername/talos.git
   cd talos
   ```
3. **Set up the development environment**:
   ```bash
   cp config.toml.example config.toml
   cargo build
   cargo test
   ```
4. **Create a branch**: Create a new branch for your changes.
   ```bash
   git checkout -b feature/my-new-feature
   ```

## Development Workflow

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation changes
- `refactor/description` - Code refactoring

### Before Submitting a PR

**All PRs must include tests for new functionality. PRs without tests will be rejected.**

1. **Run the FULL test suite with ALL features** (REQUIRED):

   ```bash
   cargo test --all-features
   ```

   > **Why `--all-features`?** Many modules are gated behind feature flags. Running `cargo test` alone will NOT test feature-gated code. CI runs with `--all-features` - your local tests must match.

2. **Run the linter with all features**:

   ```bash
   cargo clippy --all-features -- -D warnings
   ```

3. **Format your code**:

   ```bash
   cargo fmt
   ```

4. **Check with different feature combinations** (if applicable):
   ```bash
   cargo build --no-default-features
   cargo build --features sqlite
   cargo build --features postgres
   ```

### Signed Commits (Required)

All commits must be signed. Unsigned commits will be rejected by the repository.

**Setting up commit signing:**

1. **Generate a GPG key** (if you don't have one):

   ```bash
   gpg --full-generate-key
   ```

2. **Get your GPG key ID**:

   ```bash
   gpg --list-secret-keys --keyid-format=long
   ```

3. **Configure Git to use your key**:

   ```bash
   git config --global user.signingkey YOUR_KEY_ID
   git config --global commit.gpgsign true
   ```

4. **Add your GPG key to GitHub**: Go to Settings > SSH and GPG keys > New GPG key

For more details, see [GitHub's guide on signing commits](https://docs.github.com/en/authentication/managing-commit-signature-verification).

### Commit Messages

Write clear, concise commit messages:

- Use the imperative mood ("Add feature" not "Added feature")
- Keep the first line under 72 characters
- Reference issues when applicable ("Fix #123")

## Pull Request Guidelines

1. **One feature per PR** - Keep PRs focused and reviewable
2. **Update documentation** - If your change affects the API or configuration
3. **Tests are REQUIRED** - PRs without tests for new code will be rejected (see Test Requirements below)
4. **Update CHANGELOG.md** - For user-facing changes

## Test Requirements (Non-Negotiable)

**Code without tests is considered incomplete and will not be merged.**

### What Requires Tests

| Change Type | Test Requirement |
|-------------|------------------|
| New endpoint | Integration test in `tests/` directory |
| New client method | Unit test + integration test |
| Bug fix | Test that reproduces the bug (fails before fix, passes after) |
| New error type | Unit test for error handling |
| New feature flag | Tests that run with that feature enabled |
| Refactoring | Existing tests must still pass; add tests if coverage was missing |

### Test Commands (All Must Pass)

```bash
# Run ALL tests with ALL features (REQUIRED before every PR)
cargo test --all-features

# Check formatting
cargo fmt --check

# Run clippy with no warnings
cargo clippy --all-features -- -D warnings
```

### Why This Policy Exists

- Untested code is unreliable code
- Feature-gated modules are only tested when `--all-features` is used
- CI enforces these requirements - save time by running them locally first
- Tests document expected behavior and prevent regressions

## Project Structure

```
talos/
├── src/
│   ├── lib.rs           # Library entry point
│   ├── config.rs        # Configuration system
│   ├── errors.rs        # Error types
│   ├── encryption.rs    # Cryptographic utilities
│   ├── hardware.rs      # Hardware fingerprinting
│   ├── client/          # Client-side code
│   └── server/          # Server-side code (feature-gated)
├── tests/               # Integration tests
├── docs/                # Documentation
└── .github/             # GitHub workflows and templates
```

## Feature Flags

Talos uses Cargo feature flags for optional functionality:

| Feature | Description |
|---------|-------------|
| `server` | Server components (handlers, database) |
| `sqlite` | SQLite database backend |
| `postgres` | PostgreSQL database backend |
| `jwt-auth` | JWT authentication middleware |
| `admin-api` | Admin CRUD API endpoints |
| `rate-limiting` | Rate limiting middleware |
| `background-jobs` | Scheduled background jobs |
| `openapi` | OpenAPI 3.0 spec and Swagger UI |

**Default features:** `server`, `sqlite`

When adding new features, consider whether they should be feature-gated.

## AI-Assisted Development

We welcome contributions that use AI assistants (Claude, GPT, Copilot, etc.) **as tools to implement your ideas**, not as replacements for human thinking and decision-making.

### Philosophy: AI as Implementation Partner

AI should help you:

- **Implement** your design decisions faster
- **Explore** different approaches you've considered
- **Debug** issues you've identified
- **Write** boilerplate code for patterns you've chosen
- **Document** code you understand

AI should **NOT**:

- Make architectural decisions for you
- Choose libraries or frameworks without your evaluation
- Determine security policies
- Replace code review by humans
- Be trusted blindly for correctness

**You are responsible for every line of code you commit.** If you can't explain why the code works and why it's the right approach, don't commit it.

### Requirements for AI-Assisted Contributions

1. **All new code MUST have tests** - No exceptions. PRs without tests will be rejected.

2. **All tests must pass with all features**

   ```bash
   # Run the full test suite with ALL features (REQUIRED)
   cargo test --all-features
   ```

3. **Code must be formatted and lint-free**

   ```bash
   cargo fmt
   cargo clippy --all-features -- -D warnings
   ```

4. **Security review is mandatory**

   - Review all AI-generated code for security vulnerabilities
   - Never commit hardcoded secrets, API keys, or credentials
   - Validate all user inputs at API boundaries
   - Use parameterized queries (SQLx handles this)
   - Follow OWASP guidelines for web security

5. **Understand what you're committing**

   - Read and understand all generated code
   - Don't blindly accept AI suggestions
   - Test edge cases manually
   - Be able to explain the code in a review

6. **Human oversight is non-negotiable**
   - Design decisions must be made by humans
   - Security-critical code must be human-reviewed
   - Test plans should be human-designed (AI can help implement)

### AI Context Files

The `.claude/` directory contains context files to help AI assistants understand the project:

- `.claude/README.md` - Project overview, architecture, and guidelines

When using an AI assistant, you can reference these files to provide context.

### Disclosure

You are **not required** to disclose AI assistance in your commits or PRs. However, if you choose to, you can add something like this example dependent on your AI assistant:

```
Co-Authored-By: Claude / etc
```

### Security Considerations for AI-Generated Code

- **Prompt injection**: Be cautious of user input that could affect AI behavior
- **Sensitive data**: Never include real credentials in prompts
- **Code review**: AI-generated code should receive the same (or more rigorous) review as human code
- **License compliance**: Ensure AI suggestions don't violate open-source licenses
- **Trust but verify**: AI can make subtle mistakes that look correct at first glance

## Testing

**Tests are mandatory for all new code. See "Test Requirements" section above.**

- **Unit tests**: Place in the same file using `#[cfg(test)]` modules
- **Integration tests**: Place in the `tests/` directory
- **Test naming**: Use descriptive names like `test_license_activation_with_valid_key`
- **Always run with `--all-features`**: `cargo test --all-features`

## Reporting Bugs

Please use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml) when creating issues.

## Feature Requests

Please use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.yml) and check the [ROADMAP](docs/public/ROADMAP.md) first.

## Security Vulnerabilities

**Do not open public issues for security vulnerabilities.** Please see [SECURITY.md](SECURITY.md) for reporting instructions.

## Questions?

Feel free to open a [Discussion](https://github.com/dmriding/talos/discussions) on GitHub if you have questions about contributing.

## License

By contributing to Talos, you agree that your contributions will be licensed under the [MIT License](LICENSE).
