# QiyasHash E2EE Messaging Protocol

<div align="center">

![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
[![GitHub](https://img.shields.io/github/stars/qiyascc/qiyashashchat?style=social)](https://github.com/qiyascc/qiyashashchat)

**A decentralized, peer-to-peer encrypted messaging system with state-of-the-art security**

[Protocol Spec](#protocol-specification) • [Quick Start](#quick-start) • [Architecture](#architecture) • [Deployment](#deployment) • [API Reference](#api-reference)

</div>

---

## Overview

QiyasHash is a production-grade end-to-end encrypted messaging protocol that provides:

- 🔐 **Perfect Forward Secrecy** - Compromised keys cannot decrypt past messages
- 🔄 **Backward Secrecy** - Key ratcheting ensures future messages remain secure
- 🕵️ **Deniability** - Senders can plausibly deny sending messages
- 🌐 **Decentralized** - No central server, fully peer-to-peer via DHT
- 📊 **Metadata Protection** - Zero metadata leakage to any party
- 🔗 **Chain Integrity** - Cryptographic proof of message ordering
- 🧅 **Anonymous Routing** - Optional Tor and I2P integration
- 📡 **Traffic Obfuscation** - Cover traffic and timing randomization

## Quick Start

### Prerequisites

- Rust 1.75 or later
- OpenSSL development libraries
- For DHT: libp2p-compatible network

### Installation

```bash
# Clone the repository
git clone https://github.com/qiyascc/qiyashashchat.git
cd qiyashashchat

# Build all crates
cargo build --release

# Run tests
cargo test --all

# Install CLI
cargo install --path clients/cli
```

### Basic Usage

```bash
# Initialize identity
qiyashash init --name "My Device"

# Show identity
qiyashash identity --fingerprint

# Send a message
qiyashash send --to <user-id> --message "Hello!"

# Receive messages
qiyashash receive

# Verify a contact
qiyashash verify <user-id>
```

## Architecture

### Crates

```
qiyashashchat/
├── crates/
│   ├── qiyashash-crypto/     # Core cryptographic primitives
│   ├── qiyashash-core/       # Core types and storage traits
│   ├── qiyashash-protocol/   # Protocol implementation
│   ├── qiyashash-dht/        # Distributed hash table
│   ├── qiyashash-relay/      # Relay node implementation
│   ├── qiyashash-chain/      # Chain state management
│   └── qiyashash-anonymity/  # Tor/I2P and traffic obfuscation
├── services/
│   ├── identity-service/     # Identity management API
│   ├── encryption-service/   # Encryption/decryption API
│   ├── dht-peer-service/     # DHT node service
│   └── ...
├── clients/
│   ├── cli/                  # Command-line client
│   ├── desktop/              # Desktop application (Tauri)
│   ├── web/                  # Web client (React + TypeScript)
│   └── mobile-core/          # Mobile FFI library
├── deploy/                   # Docker & Kubernetes configs
└── docs/                     # Documentation
```

### Protocol Stack

```
┌─────────────────────────────────────────────┐
│              Application Layer              │
│         (Messages, Attachments, etc)        │
├─────────────────────────────────────────────┤
│              Protocol Layer                 │
│    (Session Management, Message Routing)    │
├─────────────────────────────────────────────┤
│            Encryption Layer                 │
│     (Double Ratchet, X3DH, AEAD)           │
├─────────────────────────────────────────────┤
│            Anonymity Layer                  │
│    (Tor, I2P, Traffic Obfuscation)         │
├─────────────────────────────────────────────┤
│             Transport Layer                 │
│        (DHT, Relay Nodes, Gossipsub)       │
└─────────────────────────────────────────────┘
```

## Deployment

### Docker Quick Start

```bash
cd deploy/docker
docker compose up -d
```

### With Tor/I2P Anonymity

```bash
docker compose --profile anonymity up -d
```

See [Deployment Guide](deploy/DEPLOYMENT.md) for full documentation including:
- Kubernetes deployment
- Manual installation
- Security hardening
- Monitoring setup

## Protocol Specification

### Key Exchange (X3DH)

QiyasHash uses Extended Triple Diffie-Hellman (X3DH) for asynchronous session establishment:

1. **Identity Keys (IK)** - Long-term Ed25519 keys, converted to X25519 for DH
2. **Signed Pre-Keys (SPK)** - Medium-term X25519 keys, rotated weekly
3. **One-Time Pre-Keys (OPK)** - Single-use X25519 keys for additional forward secrecy

```
Alice                                 Bob
  │                                    │
  │  ──── Fetch PreKey Bundle ────►   │
  │                                    │
  │  Generate Ephemeral Key (EK)       │
  │                                    │
  │  DH1 = DH(IK_A, SPK_B)            │
  │  DH2 = DH(EK_A, IK_B)             │
  │  DH3 = DH(EK_A, SPK_B)            │
  │  DH4 = DH(EK_A, OPK_B)            │
  │                                    │
  │  SK = KDF(DH1 || DH2 || DH3 || DH4)│
  │                                    │
  │  ──── Initial Message ──────────► │
  │                                    │
```

### Double Ratchet

After X3DH, the Double Ratchet algorithm provides:

- **DH Ratchet**: New key pair with each message exchange direction change
- **Symmetric Ratchet**: KDF chain for each message in same direction

```rust
// Simplified ratchet step
fn ratchet_encrypt(&mut self, plaintext: &[u8]) -> RatchetMessage {
    let (chain_key, message_key) = kdf_chain(&self.sending_chain);
    self.sending_chain = chain_key;
    
    let ciphertext = aead_encrypt(message_key, plaintext);
    
    RatchetMessage {
        header: RatchetHeader {
            dh_public: self.dh_keypair.public(),
            message_number: self.send_count,
            previous_chain_length: self.previous_chain_length,
        },
        ciphertext,
    }
}
```

### Chain State

Messages are linked in a cryptographic chain for ordering verification:

```
State₀ ──► State₁ ──► State₂ ──► State₃
  │          │          │          │
  ▼          ▼          ▼          ▼
 Init      Msg₁       Msg₂      Msg₃
```

Each state transition: `State_{n+1} = SHA256(State_n || msg_hash || timestamp)`

### Fragment Distribution

Messages are split using Reed-Solomon erasure coding:

```
Original Message
       │
       ▼
┌──────────────┐
│ Reed-Solomon │
│   Encoder    │
└──────────────┘
       │
       ▼
┌───┬───┬───┬───┬───┐
│ D1│ D2│ D3│ P1│ P2│  (3 data + 2 parity shards)
└───┴───┴───┴───┴───┘
       │
       ▼
  Distribute to DHT
  (Any 3 of 5 sufficient for reconstruction)
```

## Security Properties

| Property | Implementation |
|----------|----------------|
| Forward Secrecy | DH Ratchet with ephemeral keys |
| Backward Secrecy | KDF chain ratcheting |
| Deniability | HMAC authentication (not signatures) |
| Metadata Protection | DHT distribution, timing obfuscation |
| Replay Prevention | Chain state + message counters |
| Quantum Resistance | SHA-512 foundation, upgradable to ML-KEM |

## API Reference

### Identity Service

```
POST /api/v1/identity/generate
POST /api/v1/identity/rotate
POST /api/v1/identity/verify
GET  /api/v1/identity/prekeys
POST /api/v1/identity/prekeys
GET  /api/v1/identity/bundle/{user_id}
```

### Encryption Service

```
POST /api/v1/encrypt/message
POST /api/v1/encrypt/establish-session
POST /api/v1/decrypt/message
GET  /api/v1/session/{session_id}
```

### DHT Service

```
POST /api/v1/dht/store
GET  /api/v1/dht/retrieve/{fragment_id}
POST /api/v1/dht/announce
GET  /api/v1/dht/peers
```

## Performance

Benchmarks on Apple M1:

| Operation | Time |
|-----------|------|
| Key Generation | 0.5 ms |
| X25519 DH | 0.05 ms |
| Message Encrypt (1KB) | 0.8 ms |
| Message Decrypt (1KB) | 0.8 ms |
| X3DH Initiate | 1.2 ms |
| Chain Verification (1000 links) | 45 ms |

## Development

### Running Tests

```bash
# All tests
cargo test --all

# Crypto tests only
cargo test -p qiyashash-crypto

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

### Running Benchmarks

```bash
cargo bench -p qiyashash-crypto
```

### Code Coverage

```bash
cargo tarpaulin --all --out Html
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure:
- All tests pass
- Code is formatted with `cargo fmt`
- No clippy warnings (`cargo clippy`)
- Documentation is updated

## Security

### Reporting Vulnerabilities

Please report security vulnerabilities to security@qiyashash.dev

Do NOT create public issues for security vulnerabilities.

### Audit Status

This implementation has not yet undergone a formal security audit. Use in production at your own risk.

## License

Dual licensed under:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

## Acknowledgments

- Signal Protocol for inspiration on Double Ratchet and X3DH
- libp2p for DHT implementation
- The Rust cryptography ecosystem

---

<div align="center">
Made with 🔐 by the QiyasHash Team
</div>
