# Changelog

All notable changes to QiyasHash Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial protocol implementation
- Core cryptographic library (qiyashash-crypto)
  - X25519 key exchange
  - Ed25519 signatures
  - AES-256-GCM and ChaCha20-Poly1305 encryption
  - Double Ratchet implementation
  - X3DH key agreement
- Protocol layer (qiyashash-protocol)
  - Secure session management
  - Message encryption/decryption
  - Group messaging support
- DHT layer (qiyashash-dht)
  - Kademlia-based distributed storage
  - Message distribution
  - Peer discovery
- Relay service (qiyashash-relay)
  - Offline message storage
  - QUIC transport
- Chain state management (qiyashash-chain)
  - Message ordering
  - Hash chain integrity
- Anonymity layer (qiyashash-anonymity)
  - Tor integration
  - I2P support
  - Traffic obfuscation
- Identity Service
  - User registration
  - Key management
  - Identity verification
- Encryption Service
  - Session management
  - Key exchange endpoints
- DHT Peer Service
  - libp2p integration
  - Message storage
- Relay Coordination Service
  - Node registration
  - Load balancing
- Metadata Nullification Service
  - Message padding
  - Timing protection
- Chain State Service
  - Chain management API
  - Integrity verification
- CLI Client
  - Full messaging capabilities
  - Identity management
- Desktop Client (Tauri)
  - Cross-platform support
  - Native notifications
- Web Client (React)
  - Modern UI
  - WebCrypto integration
- Mobile Core Library
  - UniFFI bindings
  - iOS/Android support
- Docker deployment
  - Multi-service compose
  - Tor/I2P profiles
- Kubernetes deployment
  - Helm charts
  - Auto-scaling

### Security
- Constant-time cryptographic operations
- Memory zeroization for sensitive data
- No custom cryptographic primitives

## [0.1.0] - 2024-XX-XX

### Added
- Initial release

[Unreleased]: https://github.com/qiyascc/qiyashashchat/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/qiyascc/qiyashashchat/releases/tag/v0.1.0
