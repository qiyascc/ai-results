# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please do NOT report security vulnerabilities through public GitHub issues.**

### How to Report

1. **Email**: Send details to security@qiyashash.dev
2. **Subject**: Include "SECURITY" in the subject line
3. **Encryption**: Use our PGP key for sensitive details (available upon request)

### What to Include

- Type of vulnerability (e.g., cryptographic flaw, injection, etc.)
- Full paths of affected source files
- Step-by-step reproduction instructions
- Proof-of-concept or exploit code (if available)
- Impact assessment

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution Target**: Within 90 days (critical issues faster)

### What to Expect

1. Acknowledgment of your report
2. Assessment of the vulnerability
3. Regular updates on progress
4. Credit in security advisories (if desired)

## Security Measures

### Cryptographic Standards

- **Key Exchange**: X25519 (Curve25519)
- **Signatures**: Ed25519
- **Symmetric Encryption**: AES-256-GCM, ChaCha20-Poly1305
- **Key Derivation**: HKDF-SHA256
- **Hashing**: SHA-256, SHA-3

### Protocol Security

- **Forward Secrecy**: Double Ratchet with ephemeral keys
- **Post-Compromise Security**: Automatic key rotation
- **Replay Protection**: Unique message IDs and chain states
- **Metadata Protection**: Traffic obfuscation and padding

### Implementation Security

- Memory-safe Rust implementation
- Constant-time cryptographic operations
- Zeroization of sensitive data
- No custom cryptographic primitives

## Security Audits

We welcome security audits. Contact us for cooperation.

## Bug Bounty

We're working on a formal bug bounty program. Stay tuned!

## Acknowledgments

We thank security researchers who responsibly disclose vulnerabilities.
