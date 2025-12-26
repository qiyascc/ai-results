# QiyasHash Protocol Specification

**Version:** 1.0.0  
**Status:** Draft  
**Last Updated:** 2024

## Abstract

QiyasHash is an end-to-end encrypted messaging protocol designed for decentralized, 
peer-to-peer communication. This specification defines the cryptographic primitives, 
message formats, and protocol flows that enable secure messaging with perfect forward 
secrecy, backward secrecy, and deniability.

## 1. Introduction

### 1.1 Goals

- **Confidentiality**: Only intended recipients can read messages
- **Integrity**: Messages cannot be modified without detection
- **Authenticity**: Recipients can verify the sender
- **Forward Secrecy**: Compromised long-term keys don't reveal past messages
- **Backward Secrecy**: Compromised session keys don't reveal future messages
- **Deniability**: Senders can deny sending specific messages
- **Metadata Protection**: Minimize information leakage about communication patterns

### 1.2 Notation

- `||` - Concatenation
- `HKDF(salt, ikm, info, len)` - HKDF-SHA512
- `HMAC(key, data)` - HMAC-SHA256
- `AEAD_Enc(key, nonce, plaintext, ad)` - XChaCha20-Poly1305 encryption
- `DH(sk, pk)` - X25519 Diffie-Hellman
- `Sign(sk, message)` - Ed25519 signature
- `H(data)` - SHA-256 hash

## 2. Cryptographic Primitives

### 2.1 Key Types

| Key Type | Algorithm | Size | Purpose |
|----------|-----------|------|---------|
| Identity Key (IK) | Ed25519 | 32 bytes | Long-term signing |
| Identity DH Key | X25519 | 32 bytes | Derived from IK for DH |
| Signed Pre-Key (SPK) | X25519 | 32 bytes | Medium-term DH |
| One-Time Pre-Key (OPK) | X25519 | 32 bytes | Single-use DH |
| Ephemeral Key (EK) | X25519 | 32 bytes | Per-session DH |
| Chain Key (CK) | - | 32 bytes | Ratchet chain state |
| Message Key (MK) | - | 32 bytes | Per-message encryption |

### 2.2 Algorithms

- **AEAD**: XChaCha20-Poly1305 (primary), AES-256-GCM (alternative)
- **KDF**: HKDF-SHA512
- **Hash**: SHA-256 (chain), SHA-512 (key derivation)
- **Signature**: Ed25519
- **Key Exchange**: X25519

## 3. Key Agreement (X3DH)

### 3.1 Key Publishing

Bob publishes to DHT:
- Identity Public Key: `IK_B`
- Signed Pre-Key: `SPK_B`
- SPK Signature: `Sign(IK_B, SPK_B)`
- One-Time Pre-Keys: `{OPK_B_1, OPK_B_2, ...}`

### 3.2 Initial Key Agreement

When Alice wants to establish a session with Bob:

1. Fetch Bob's pre-key bundle from DHT
2. Verify SPK signature
3. Generate ephemeral key pair `EK_A`
4. Compute DH outputs:
   ```
   DH1 = DH(IK_A, SPK_B)
   DH2 = DH(EK_A, IK_B)
   DH3 = DH(EK_A, SPK_B)
   DH4 = DH(EK_A, OPK_B)  // if OPK available
   ```
5. Derive shared secret:
   ```
   SK = HKDF(salt=0xFF*32, ikm=DH1||DH2||DH3||DH4, info="QiyasHash_v1_RootKey", len=32)
   ```
6. Associated data:
   ```
   AD = IK_A || IK_B
   ```

### 3.3 Initial Message

Alice sends:
- `IK_A` - Her identity public key
- `EK_A` - Ephemeral public key
- `OPK_id` - ID of one-time pre-key used (if any)
- Encrypted initial message

## 4. Double Ratchet

### 4.1 State Variables

Each party maintains:
- `DHs` - DH ratchet key pair (sending)
- `DHr` - DH ratchet public key (receiving)
- `RK` - Root key (32 bytes)
- `CKs` - Sending chain key
- `CKr` - Receiving chain key
- `Ns` - Message number (sending)
- `Nr` - Message number (receiving)
- `PN` - Previous chain length
- `MKSKIPPED` - Dictionary of skipped message keys

### 4.2 KDF Chains

**Root Key Ratchet:**
```
(RK', CK) = HKDF(RK, DH_output, "QiyasHash_v1_RootKey", 64)
```

**Chain Key Ratchet:**
```
MK = HMAC(CK, 0x01)
CK' = HMAC(CK, 0x02)
```

### 4.3 Ratchet Steps

**DH Ratchet (on receiving new DH key):**
1. `DHr = header.dh_public`
2. `(RK, CKr) = KDF_RK(RK, DH(DHs, DHr))`
3. `DHs = GENERATE_DH()`
4. `(RK, CKs) = KDF_RK(RK, DH(DHs, DHr))`

**Symmetric Ratchet (for each message):**
1. `(CK, MK) = KDF_CK(CK)`
2. Use MK to encrypt/decrypt message

### 4.4 Message Format

```
Header:
  dh_public:             [u8; 32]  - Current DH ratchet public key
  message_number:        u32       - Message number in chain
  previous_chain_length: u32       - Previous sending chain length

Encrypted Payload:
  algorithm:    AeadAlgorithm     - XChaCha20-Poly1305 or AES-256-GCM
  nonce:        [u8; 24]          - Random nonce
  ciphertext:   Vec<u8>           - Encrypted content + auth tag
```

## 5. Chain State

### 5.1 Chain Link

Each message creates a chain link:

```
ChainLink:
  link_type:    LinkType    - Message, Deletion, Rotation, etc.
  state:        [u8; 32]    - Current chain state hash
  message_hash: [u8; 32]    - Hash of message content
  timestamp:    u64         - Unix timestamp
  sequence:     u64         - Sequence number
```

### 5.2 State Transition

```
new_state = SHA256(
  prev_state ||
  message_hash ||
  timestamp.to_be_bytes() ||
  sequence.to_be_bytes()
)
```

### 5.3 Chain Proof

For external verification:
```
proof = SHA512(
  for each link in chain:
    link.state ||
    link.message_hash ||
    link.timestamp.to_be_bytes()
)
```

## 6. Message Distribution

### 6.1 Fragment Encoding

Messages are split using Reed-Solomon:
- Default: 3 data shards + 2 parity shards
- Any 3 of 5 fragments sufficient for reconstruction
- Each fragment stored independently in DHT

### 6.2 Fragment Format

```
Fragment:
  id:           FragmentId   - SHA256(message_id || index)
  message_id:   String       - Parent message identifier
  index:        usize        - Fragment index (0..total)
  total:        usize        - Total fragment count
  data:         Vec<u8>      - Fragment data
  is_parity:    bool         - True for parity shards
  shard_size:   usize        - Size of each shard
  message_size: usize        - Original message size
  expiry:       u64          - Expiration timestamp
  created_at:   u64          - Creation timestamp
```

### 6.3 DHT Storage

Fragments stored using Kademlia DHT:
- Key: Fragment ID (32 bytes)
- Value: Serialized Fragment
- Replication factor: 3
- Expiry: 30 days default

## 7. Message Envelope

### 7.1 Wire Format

```
MessageEnvelope:
  version:              u32         - Protocol version
  sender_identity_key:  [u8; 32]    - Sender's identity key
  ephemeral_key:        Option<[u8; 32]>  - For initial message
  one_time_prekey_id:   Option<u32> - OPK ID if used
  ratchet_header:       RatchetHeader
  ciphertext:           Vec<u8>     - Encrypted payload
  chain_proof:          [u8; 32]    - Chain state proof
  timestamp_hash:       [u8; 32]    - Hashed timestamp + noise
```

### 7.2 Timestamp Protection

To prevent timing correlation:
```
timestamp_hash = SHA256(
  "QiyasHash_Timestamp_v1" ||
  timestamp.to_be_bytes() ||
  random_noise[16]
)
```

## 8. Identity Management

### 8.1 Identity Rotation

When rotating identity keys:

1. Generate new identity key pair
2. Create rotation proof:
   ```
   message = old_public || new_public || timestamp
   old_sig = Sign(old_sk, message)
   new_sig = Sign(new_sk, message)
   commitment = SHA256(message || old_sig || new_sig)
   ```
3. Announce rotation to network
4. Update sessions to use new key

### 8.2 Trust Model

- Trust on First Use (TOFU) by default
- Safety numbers for verification:
  ```
  safety_number = SHA256(
    min(fingerprint_a, fingerprint_b) ||
    max(fingerprint_a, fingerprint_b)
  )
  ```

## 9. Security Considerations

### 9.1 Key Compromise

| Compromised Key | Impact | Mitigation |
|-----------------|--------|------------|
| Identity Key | Future sessions compromised | Rotate immediately |
| Signed Pre-Key | Some sessions affected | Regular rotation (weekly) |
| One-Time Pre-Key | Single session affected | Consumed after use |
| Chain Key | Future messages in chain | DH ratchet recovers |
| Message Key | Single message | Not stored after use |

### 9.2 Replay Prevention

- Message numbers in headers
- Chain state verification
- Timestamp validation (within window)
- One-time pre-key consumption

### 9.3 Denial of Service

- Rate limiting on pre-key requests
- Proof of work for initial messages (optional)
- Fragment expiration
- DHT replication for availability

## 10. Appendix

### 10.1 Test Vectors

**X25519 DH:**
```
alice_sk: 77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a
alice_pk: 8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a
bob_sk:   5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb
bob_pk:   de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f
shared:   4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742
```

**HKDF-SHA512:**
```
salt:   000102030405060708090a0b0c
ikm:    0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b
info:   f0f1f2f3f4f5f6f7f8f9
length: 42
okm:    832390086cda71fb47625bb5ceb168e4c8e26a1a16ed34d9...
```

### 10.2 References

1. Signal Protocol Specification
2. The Double Ratchet Algorithm (Marlinspike, Perrin)
3. The X3DH Key Agreement Protocol (Marlinspike, Perrin)
4. RFC 7748 - Elliptic Curves for Security
5. RFC 8439 - ChaCha20 and Poly1305
6. RFC 5869 - HKDF
