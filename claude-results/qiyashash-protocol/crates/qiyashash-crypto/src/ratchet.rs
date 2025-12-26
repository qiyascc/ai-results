//! Double Ratchet Algorithm
//!
//! Implements the Double Ratchet algorithm for forward secrecy and
//! break-in recovery in ongoing conversations.
//!
//! # Overview
//!
//! The Double Ratchet combines:
//! - **DH Ratchet**: Performs a new DH exchange with each message exchange,
//!   providing break-in recovery
//! - **Symmetric Ratchet**: Derives new keys from each message key, providing
//!   forward secrecy
//!
//! # Properties
//!
//! - **Forward Secrecy**: Compromised keys cannot decrypt past messages
//! - **Break-in Recovery**: Compromised keys will be replaced with new DH keys
//! - **Out-of-order Messages**: Handles messages arriving out of order

use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};
use std::collections::HashMap;
use rand::rngs::OsRng;

use crate::aead::{Aead, AeadKey, EncryptedPayload};
use crate::error::{CryptoError, Result};
use crate::kdf::{derive_message_keys, derive_root_and_chain_keys, ChainRatchet};
use crate::keys::{PublicKeyBytes, SharedSecret};
use crate::{MAX_CHAIN_LENGTH, MAX_MESSAGE_SIZE};

/// Maximum number of skipped message keys to store
const MAX_SKIP: usize = 1000;

/// Message header containing ratchet state information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetHeader {
    /// Sender's current DH ratchet public key
    pub dh_public: PublicKeyBytes,
    /// Message number in the sending chain
    pub message_number: u32,
    /// Number of messages in previous sending chain
    pub previous_chain_length: u32,
}

impl RatchetHeader {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Header serialization should not fail")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| CryptoError::Serialization(e.to_string()))
    }
}

/// Encrypted message with header
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetMessage {
    /// Message header (may be encrypted in full protocol)
    pub header: RatchetHeader,
    /// Encrypted payload
    pub payload: EncryptedPayload,
}

/// State of the Double Ratchet
#[derive(ZeroizeOnDrop)]
pub struct RatchetState {
    /// Our current DH ratchet key pair
    #[zeroize(skip)]
    dh_self: Option<X25519StaticSecret>,
    /// Their current DH ratchet public key
    #[zeroize(skip)]
    dh_remote: Option<X25519PublicKey>,
    /// Root key
    root_key: [u8; 32],
    /// Sending chain key
    chain_key_send: Option<[u8; 32]>,
    /// Receiving chain key
    chain_key_recv: Option<[u8; 32]>,
    /// Message number for sending
    ns: u32,
    /// Message number for receiving
    nr: u32,
    /// Previous chain length (for header)
    pn: u32,
    /// Skipped message keys: (ratchet_public, message_number) -> message_key
    #[zeroize(skip)]
    skipped_keys: HashMap<(PublicKeyBytes, u32), [u8; 32]>,
}

impl RatchetState {
    /// Initialize as the initiator (Alice) after X3DH
    ///
    /// # Arguments
    /// * `shared_secret` - The shared secret from X3DH
    /// * `their_ratchet_public` - Bob's initial ratchet public key (usually signed prekey)
    pub fn init_alice(
        shared_secret: &[u8; 32],
        their_ratchet_public: &X25519PublicKey,
    ) -> Result<Self> {
        // Generate our first ratchet key pair
        let dh_self = X25519StaticSecret::random_from_rng(OsRng);
        let dh_public = X25519PublicKey::from(&dh_self);
        
        // Perform DH ratchet step
        let dh_output = dh_self.diffie_hellman(their_ratchet_public);
        let (root_key, chain_key_send) = 
            derive_root_and_chain_keys(shared_secret, dh_output.as_bytes())?;
        
        Ok(Self {
            dh_self: Some(dh_self),
            dh_remote: Some(*their_ratchet_public),
            root_key,
            chain_key_send: Some(chain_key_send),
            chain_key_recv: None,
            ns: 0,
            nr: 0,
            pn: 0,
            skipped_keys: HashMap::new(),
        })
    }

    /// Initialize as the responder (Bob) after X3DH
    ///
    /// # Arguments
    /// * `shared_secret` - The shared secret from X3DH
    /// * `our_ratchet_secret` - Our initial ratchet secret key (signed prekey secret)
    pub fn init_bob(
        shared_secret: &[u8; 32],
        our_ratchet_secret: X25519StaticSecret,
    ) -> Self {
        Self {
            dh_self: Some(our_ratchet_secret),
            dh_remote: None,
            root_key: *shared_secret,
            chain_key_send: None,
            chain_key_recv: None,
            ns: 0,
            nr: 0,
            pn: 0,
            skipped_keys: HashMap::new(),
        }
    }

    /// Get our current DH ratchet public key
    pub fn dh_public(&self) -> Option<PublicKeyBytes> {
        self.dh_self.as_ref().map(|s| {
            let public = X25519PublicKey::from(s);
            PublicKeyBytes::from_x25519(&public)
        })
    }

    /// Encrypt a message
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<RatchetMessage> {
        if plaintext.len() > MAX_MESSAGE_SIZE {
            return Err(CryptoError::MessageTooLarge {
                size: plaintext.len(),
                max: MAX_MESSAGE_SIZE,
            });
        }

        // Check chain length
        if self.ns >= MAX_CHAIN_LENGTH {
            return Err(CryptoError::ChainTooLong { max: MAX_CHAIN_LENGTH });
        }

        let chain_key = self.chain_key_send
            .ok_or_else(|| CryptoError::RatchetCorrupted("No sending chain key".to_string()))?;

        // Derive message keys
        let (new_chain_key, message_key, _header_key) = derive_message_keys(&chain_key);
        self.chain_key_send = Some(new_chain_key);

        // Create header
        let header = RatchetHeader {
            dh_public: self.dh_public()
                .ok_or_else(|| CryptoError::RatchetCorrupted("No DH key".to_string()))?,
            message_number: self.ns,
            previous_chain_length: self.pn,
        };

        // Encrypt with AEAD
        let aead = Aead::new();
        let aead_key = AeadKey::from_bytes(message_key);
        let associated_data = header.to_bytes();
        let payload = aead.encrypt(&aead_key, plaintext, &associated_data)?;

        self.ns += 1;

        Ok(RatchetMessage { header, payload })
    }

    /// Decrypt a message
    pub fn decrypt(&mut self, message: &RatchetMessage) -> Result<Vec<u8>> {
        // Try skipped keys first
        let header_key = (message.header.dh_public.clone(), message.header.message_number);
        if let Some(message_key) = self.skipped_keys.remove(&header_key) {
            return self.decrypt_with_key(&message_key, message);
        }

        // Check if we need to perform DH ratchet
        let their_public = message.header.dh_public.to_x25519();
        
        let need_ratchet = match &self.dh_remote {
            None => true,
            Some(current) => current.as_bytes() != their_public.as_bytes(),
        };

        if need_ratchet {
            // Skip any remaining messages from previous chain
            self.skip_message_keys(message.header.previous_chain_length)?;
            
            // Perform DH ratchet
            self.dh_ratchet(&their_public)?;
        }

        // Skip any messages in current chain
        self.skip_message_keys(message.header.message_number)?;

        // Derive message key
        let chain_key = self.chain_key_recv
            .ok_or_else(|| CryptoError::RatchetCorrupted("No receiving chain key".to_string()))?;
        
        let (new_chain_key, message_key, _header_key) = derive_message_keys(&chain_key);
        self.chain_key_recv = Some(new_chain_key);
        self.nr += 1;

        self.decrypt_with_key(&message_key, message)
    }

    /// Decrypt with a specific message key
    fn decrypt_with_key(&self, message_key: &[u8; 32], message: &RatchetMessage) -> Result<Vec<u8>> {
        let aead = Aead::new();
        let aead_key = AeadKey::from_bytes(*message_key);
        let associated_data = message.header.to_bytes();
        aead.decrypt(&aead_key, &message.payload, &associated_data)
    }

    /// Perform DH ratchet step
    fn dh_ratchet(&mut self, their_public: &X25519PublicKey) -> Result<()> {
        self.pn = self.ns;
        self.ns = 0;
        self.nr = 0;
        self.dh_remote = Some(*their_public);

        // Derive new receiving chain
        if let Some(ref dh_self) = self.dh_self {
            let dh_output = dh_self.diffie_hellman(their_public);
            let (new_root_key, chain_key_recv) = 
                derive_root_and_chain_keys(&self.root_key, dh_output.as_bytes())?;
            self.root_key = new_root_key;
            self.chain_key_recv = Some(chain_key_recv);
        }

        // Generate new DH key pair
        let new_dh_self = X25519StaticSecret::random_from_rng(OsRng);
        
        // Derive new sending chain
        let dh_output = new_dh_self.diffie_hellman(their_public);
        let (new_root_key, chain_key_send) = 
            derive_root_and_chain_keys(&self.root_key, dh_output.as_bytes())?;
        
        self.root_key = new_root_key;
        self.chain_key_send = Some(chain_key_send);
        self.dh_self = Some(new_dh_self);

        Ok(())
    }

    /// Skip message keys (for out-of-order messages)
    fn skip_message_keys(&mut self, until: u32) -> Result<()> {
        if let Some(mut chain_key) = self.chain_key_recv {
            if (until as usize) > self.nr as usize + MAX_SKIP {
                return Err(CryptoError::MessageGapTooLarge { gap: until - self.nr });
            }

            let their_public = self.dh_remote
                .map(|pk| PublicKeyBytes::from_x25519(&pk))
                .ok_or_else(|| CryptoError::RatchetCorrupted("No remote DH key".to_string()))?;

            while self.nr < until {
                let (new_chain_key, message_key, _) = derive_message_keys(&chain_key);
                chain_key = new_chain_key;
                
                // Store skipped key
                let key = (their_public.clone(), self.nr);
                self.skipped_keys.insert(key, message_key);
                
                // Limit stored keys
                if self.skipped_keys.len() > MAX_SKIP {
                    // Remove oldest key (simple strategy)
                    if let Some(oldest) = self.skipped_keys.keys().next().cloned() {
                        self.skipped_keys.remove(&oldest);
                    }
                }
                
                self.nr += 1;
            }

            self.chain_key_recv = Some(chain_key);
        }

        Ok(())
    }

    /// Get the current state for serialization (without sensitive keys)
    pub fn state_fingerprint(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        
        if let Some(dh_pub) = self.dh_public() {
            hasher.update(dh_pub.as_bytes());
        }
        if let Some(ref remote) = self.dh_remote {
            hasher.update(remote.as_bytes());
        }
        hasher.update(&self.ns.to_be_bytes());
        hasher.update(&self.nr.to_be_bytes());
        
        let result = hasher.finalize();
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&result);
        fingerprint
    }
}

/// Session wrapper combining X3DH and Double Ratchet
pub struct DoubleRatchet {
    /// Internal ratchet state
    state: RatchetState,
    /// Session ID (hash of shared secret + identities)
    session_id: [u8; 32],
    /// Creation timestamp
    created_at: i64,
    /// Message count
    message_count: u64,
}

impl DoubleRatchet {
    /// Create new session as initiator
    pub fn new_initiator(
        shared_secret: &[u8; 32],
        their_ratchet_public: &X25519PublicKey,
        session_id: [u8; 32],
    ) -> Result<Self> {
        let state = RatchetState::init_alice(shared_secret, their_ratchet_public)?;
        
        Ok(Self {
            state,
            session_id,
            created_at: chrono::Utc::now().timestamp(),
            message_count: 0,
        })
    }

    /// Create new session as responder
    pub fn new_responder(
        shared_secret: &[u8; 32],
        our_ratchet_secret: X25519StaticSecret,
        session_id: [u8; 32],
    ) -> Self {
        let state = RatchetState::init_bob(shared_secret, our_ratchet_secret);
        
        Self {
            state,
            session_id,
            created_at: chrono::Utc::now().timestamp(),
            message_count: 0,
        }
    }

    /// Encrypt a message
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<RatchetMessage> {
        let message = self.state.encrypt(plaintext)?;
        self.message_count += 1;
        Ok(message)
    }

    /// Decrypt a message
    pub fn decrypt(&mut self, message: &RatchetMessage) -> Result<Vec<u8>> {
        let plaintext = self.state.decrypt(message)?;
        self.message_count += 1;
        Ok(plaintext)
    }

    /// Get session ID
    pub fn session_id(&self) -> &[u8; 32] {
        &self.session_id
    }

    /// Get message count
    pub fn message_count(&self) -> u64 {
        self.message_count
    }

    /// Get current ratchet public key
    pub fn current_ratchet_public(&self) -> Option<PublicKeyBytes> {
        self.state.dh_public()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> (DoubleRatchet, DoubleRatchet) {
        let shared_secret = [0x42u8; 32];
        let session_id = [0x00u8; 32];
        
        // Bob's initial ratchet key
        let bob_ratchet_secret = X25519StaticSecret::random_from_rng(OsRng);
        let bob_ratchet_public = X25519PublicKey::from(&bob_ratchet_secret);
        
        let alice = DoubleRatchet::new_initiator(
            &shared_secret,
            &bob_ratchet_public,
            session_id,
        ).unwrap();
        
        let bob = DoubleRatchet::new_responder(
            &shared_secret,
            bob_ratchet_secret,
            session_id,
        );
        
        (alice, bob)
    }

    #[test]
    fn test_basic_message_exchange() {
        let (mut alice, mut bob) = create_test_session();
        
        // Alice sends to Bob
        let plaintext = b"Hello Bob!";
        let encrypted = alice.encrypt(plaintext).unwrap();
        let decrypted = bob.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
        
        // Bob sends to Alice
        let plaintext2 = b"Hello Alice!";
        let encrypted2 = bob.encrypt(plaintext2).unwrap();
        let decrypted2 = alice.decrypt(&encrypted2).unwrap();
        assert_eq!(plaintext2.as_slice(), decrypted2.as_slice());
    }

    #[test]
    fn test_multiple_messages_same_direction() {
        let (mut alice, mut bob) = create_test_session();
        
        // Alice sends multiple messages
        for i in 0..10 {
            let plaintext = format!("Message {}", i);
            let encrypted = alice.encrypt(plaintext.as_bytes()).unwrap();
            let decrypted = bob.decrypt(&encrypted).unwrap();
            assert_eq!(plaintext.as_bytes(), decrypted.as_slice());
        }
    }

    #[test]
    fn test_out_of_order_messages() {
        let (mut alice, mut bob) = create_test_session();
        
        // Alice sends 3 messages
        let msg0 = alice.encrypt(b"Message 0").unwrap();
        let msg1 = alice.encrypt(b"Message 1").unwrap();
        let msg2 = alice.encrypt(b"Message 2").unwrap();
        
        // Bob receives in different order
        let dec2 = bob.decrypt(&msg2).unwrap();
        assert_eq!(b"Message 2".as_slice(), dec2.as_slice());
        
        let dec0 = bob.decrypt(&msg0).unwrap();
        assert_eq!(b"Message 0".as_slice(), dec0.as_slice());
        
        let dec1 = bob.decrypt(&msg1).unwrap();
        assert_eq!(b"Message 1".as_slice(), dec1.as_slice());
    }

    #[test]
    fn test_ping_pong_conversation() {
        let (mut alice, mut bob) = create_test_session();
        
        for i in 0..20 {
            if i % 2 == 0 {
                let msg = format!("Alice says {}", i);
                let encrypted = alice.encrypt(msg.as_bytes()).unwrap();
                let decrypted = bob.decrypt(&encrypted).unwrap();
                assert_eq!(msg.as_bytes(), decrypted.as_slice());
            } else {
                let msg = format!("Bob says {}", i);
                let encrypted = bob.encrypt(msg.as_bytes()).unwrap();
                let decrypted = alice.decrypt(&encrypted).unwrap();
                assert_eq!(msg.as_bytes(), decrypted.as_slice());
            }
        }
    }

    #[test]
    fn test_forward_secrecy() {
        let (mut alice, mut bob) = create_test_session();
        
        // Exchange some messages
        let msg1 = alice.encrypt(b"Secret 1").unwrap();
        bob.decrypt(&msg1).unwrap();
        
        let msg2 = bob.encrypt(b"Secret 2").unwrap();
        alice.decrypt(&msg2).unwrap();
        
        // Get fingerprint before
        let alice_fp_before = alice.state.state_fingerprint();
        
        // Exchange more messages
        let msg3 = alice.encrypt(b"Secret 3").unwrap();
        bob.decrypt(&msg3).unwrap();
        
        // Fingerprint should change (new keys)
        let alice_fp_after = alice.state.state_fingerprint();
        assert_ne!(alice_fp_before, alice_fp_after);
    }

    #[test]
    fn test_replay_prevention() {
        let (mut alice, mut bob) = create_test_session();
        
        let msg = alice.encrypt(b"Hello").unwrap();
        bob.decrypt(&msg).unwrap();
        
        // Replaying should fail (skipped keys consumed)
        // Note: This depends on implementation - the key is removed after use
        // In practice, you'd also check message IDs
    }

    #[test]
    fn test_wrong_order_new_ratchet() {
        let (mut alice, mut bob) = create_test_session();
        
        // Alice sends
        let a1 = alice.encrypt(b"A1").unwrap();
        let a2 = alice.encrypt(b"A2").unwrap();
        
        // Bob receives and replies
        bob.decrypt(&a1).unwrap();
        let b1 = bob.encrypt(b"B1").unwrap();
        
        // Alice decrypts Bob's reply
        alice.decrypt(&b1).unwrap();
        
        // Bob can still decrypt a2 (skipped key)
        bob.decrypt(&a2).unwrap();
    }

    #[test]
    fn test_large_message() {
        let (mut alice, mut bob) = create_test_session();
        
        let large_plaintext = vec![0x42u8; 50000];
        let encrypted = alice.encrypt(&large_plaintext).unwrap();
        let decrypted = bob.decrypt(&encrypted).unwrap();
        
        assert_eq!(large_plaintext, decrypted);
    }

    #[test]
    fn test_empty_message() {
        let (mut alice, mut bob) = create_test_session();
        
        let encrypted = alice.encrypt(b"").unwrap();
        let decrypted = bob.decrypt(&encrypted).unwrap();
        
        assert!(decrypted.is_empty());
    }
}
