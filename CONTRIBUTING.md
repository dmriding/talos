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

1. **Run the test suite**:
   ```bash
   cargo test
   ```

2. **Run the linter**:
   ```bash
   cargo clippy -- -D warnings
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

### Commit Messages

Write clear, concise commit messages:
- Use the imperative mood ("Add feature" not "Added feature")
- Keep the first line under 72 characters
- Reference issues when applicable ("Fix #123")

## Pull Request Guidelines

1. **One feature per PR** - Keep PRs focused and reviewable
2. **Update documentation** - If your change affects the API or configuration
3. **Add tests** - For new features and bug fixes
4. **Update CHANGELOG.md** - For user-facing changes

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

- `server` - Server components (handlers, database)
- `sqlite` - SQLite database backend
- `postgres` - PostgreSQL database backend

When adding new features, consider whether they should be feature-gated.

## Testing

- **Unit tests**: Place in the same file using `#[cfg(test)]` modules
- **Integration tests**: Place in the `tests/` directory
- **Test naming**: Use descriptive names like `test_license_activation_with_valid_key`

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
