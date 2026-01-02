# Contributing to Talos

Thank you for your interest in contributing to **Talos**! We welcome contributions of all kinds, including bug fixes, feature enhancements, documentation improvements, and more.

## Getting Started
1. **Fork the repository**: Create a personal fork of the repository on GitHub.
2. **Clone the fork**: Clone your fork locally to your machine.
    ```bash
    git clone https://github.com/yourusername/talos.git
    cd talos
    ```
3. **Create a branch**: Create a new branch for your changes.
    ```bash
    git checkout -b my-new-feature
    ```
4. **Make changes**: Write your code or documentation changes.
5. **Run tests**: Ensure that your changes pass all tests.
    ```bash
    cargo test
    ```
6. **Commit changes**: Commit your changes with a clear and descriptive commit message.
    ```bash
    git commit -m "Add new feature: XYZ"
    ```
7. **Push your branch**: Push your branch to your forked repository.
    ```bash
    git push origin my-new-feature
    ```
8. **Create a Pull Request (PR)**: Submit a PR to the original repository on GitHub.

## Code Style
- Ensure your code follows Rust best practices.
- Run `cargo fmt` before committing to format your code.
- Run `cargo clippy` to catch potential issues.

## Reporting Bugs
If you find a bug, please create an issue on GitHub with the following information:
- A clear and descriptive title.
- Steps to reproduce the issue.
- Expected behavior.
- Actual behavior.
- Any relevant logs or error messages.

## Feature Requests
We welcome feature requests! If you have an idea for a new feature, please open an issue on GitHub with a description of your proposed feature.

## License
By contributing to Talos, you agree that your contributions will be licensed under the [MIT License](LICENSE).
