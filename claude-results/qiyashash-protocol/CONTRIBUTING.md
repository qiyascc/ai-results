# Contributing to QiyasHash

Thank you for your interest in contributing to QiyasHash! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We're building security-critical software, and thoughtful collaboration is essential.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Node.js 20+ (for web client)
- Docker & Docker Compose (for testing)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/qiyascc/qiyashashchat.git
cd qiyashashchat

# Build all crates
cargo build

# Run tests
cargo test

# Build web client
cd clients/web
npm install
npm run dev
```

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template
3. Include:
   - Rust/Node.js version
   - Operating system
   - Steps to reproduce
   - Expected vs actual behavior
   - Relevant logs

### Security Vulnerabilities

**Do NOT open public issues for security vulnerabilities.**

Please email security concerns to: security@qiyashash.dev

See [SECURITY.md](SECURITY.md) for our security policy.

### Feature Requests

1. Check the roadmap and existing issues
2. Use the feature request template
3. Describe the use case clearly

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test`
6. Run clippy: `cargo clippy -- -D warnings`
7. Format code: `cargo fmt`
8. Commit with clear messages
9. Push and create a Pull Request

## Code Style

### Rust

- Follow Rust idioms and best practices
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document public APIs with doc comments
- Write unit tests for new code

### TypeScript (Web Client)

- Use TypeScript strict mode
- Follow ESLint configuration
- Use Prettier for formatting

## Architecture Guidelines

### Cryptography

- Never implement custom cryptographic primitives
- Use audited libraries (ring, dalek, etc.)
- Always use constant-time comparisons for secrets
- Zeroize sensitive data after use

### Security

- Assume all input is malicious
- Validate and sanitize all data
- Use strong typing to prevent errors
- Log security-relevant events (without secrets)

## Testing

### Unit Tests

Each module should have comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // Test code
    }
}
```

### Integration Tests

Integration tests go in `tests/integration/`.

### Security Tests

Security-focused tests go in `tests/security/`.

### Benchmarks

Performance benchmarks go in `tests/performance/`.

## Documentation

- Update README.md for user-facing changes
- Update PROTOCOL_SPEC.md for protocol changes
- Add inline documentation for complex code
- Update deployment docs as needed

## Release Process

1. Update version in Cargo.toml files
2. Update CHANGELOG.md
3. Create a release branch
4. Run full test suite
5. Create GitHub release
6. Build and publish Docker images

## Questions?

- Open a GitHub Discussion for general questions
- Join our community chat (coming soon)

Thank you for contributing to secure messaging! 🔐
