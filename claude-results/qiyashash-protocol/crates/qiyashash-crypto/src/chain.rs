//! Chain State Management
//!
//! Provides cryptographic chain state for message ordering and integrity.
//! Each message creates a new chain link that proves message order without
//! revealing content.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CryptoError, Result};
use crate::kdf::domain;

/// Chain key for deriving message keys
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct ChainKey(pub [u8; 32]);

impl ChainKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get key bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Message key derived from chain key
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MessageKey(pub [u8; 32]);

impl MessageKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get key bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A link in the message chain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainLink {
    /// Link type
    pub link_type: ChainLinkType,
    /// Current chain state hash
    #[serde(with = "hex::serde")]
    pub state: [u8; 32],
    /// Hash of the message (for message links)
    #[serde(with = "hex::serde")]
    pub message_hash: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// Sequence number
    pub sequence: u64,
}

/// Type of chain link
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainLinkType {
    /// Regular message
    Message,
    /// Message deletion
    Deletion,
    /// Identity rotation
    IdentityRotation,
    /// Session re-key
    ReKey,
    /// Chain initialization
    Init,
}

/// Chain state manager
pub struct ChainState {
    /// Current state hash
    state: [u8; 32],
    /// Chain history (limited)
    history: Vec<ChainLink>,
    /// Current sequence number
    sequence: u64,
    /// Maximum history length
    max_history: usize,
}

impl ChainState {
    /// Create a new chain state
    pub fn new() -> Self {
        let initial_state = Self::compute_initial_state();
        
        let init_link = ChainLink {
            link_type: ChainLinkType::Init,
            state: initial_state,
            message_hash: [0u8; 32],
            timestamp: Self::current_timestamp(),
            sequence: 0,
        };
        
        Self {
            state: initial_state,
            history: vec![init_link],
            sequence: 0,
            max_history: 1000,
        }
    }

    /// Create with specific initial state (e.g., from shared secret)
    pub fn from_shared_secret(shared_secret: &[u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(domain::CHAIN_PROOF);
        hasher.update(shared_secret);
        let result = hasher.finalize();
        let mut initial_state = [0u8; 32];
        initial_state.copy_from_slice(&result);
        
        let init_link = ChainLink {
            link_type: ChainLinkType::Init,
            state: initial_state,
            message_hash: [0u8; 32],
            timestamp: Self::current_timestamp(),
            sequence: 0,
        };
        
        Self {
            state: initial_state,
            history: vec![init_link],
            sequence: 0,
            max_history: 1000,
        }
    }

    fn compute_initial_state() -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(domain::CHAIN_PROOF);
        hasher.update(b"QiyasHash_ChainInit_v1");
        let result = hasher.finalize();
        let mut state = [0u8; 32];
        state.copy_from_slice(&result);
        state
    }

    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get current state
    pub fn current_state(&self) -> &[u8; 32] {
        &self.state
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Add a message to the chain
    pub fn add_message(&mut self, message_hash: &[u8; 32]) -> ChainLink {
        self.sequence += 1;
        let timestamp = Self::current_timestamp();
        
        // Compute new state
        let new_state = self.compute_new_state(message_hash, timestamp);
        self.state = new_state;
        
        let link = ChainLink {
            link_type: ChainLinkType::Message,
            state: new_state,
            message_hash: *message_hash,
            timestamp,
            sequence: self.sequence,
        };
        
        self.add_to_history(link.clone());
        link
    }

    /// Record a message deletion
    pub fn add_deletion(&mut self, message_hash: &[u8; 32]) -> ChainLink {
        self.sequence += 1;
        let timestamp = Self::current_timestamp();
        
        let new_state = self.compute_new_state(message_hash, timestamp);
        self.state = new_state;
        
        let link = ChainLink {
            link_type: ChainLinkType::Deletion,
            state: new_state,
            message_hash: *message_hash,
            timestamp,
            sequence: self.sequence,
        };
        
        self.add_to_history(link.clone());
        link
    }

    /// Record an identity rotation
    pub fn add_identity_rotation(&mut self, proof_hash: &[u8; 32]) -> ChainLink {
        self.sequence += 1;
        let timestamp = Self::current_timestamp();
        
        let new_state = self.compute_new_state(proof_hash, timestamp);
        self.state = new_state;
        
        let link = ChainLink {
            link_type: ChainLinkType::IdentityRotation,
            state: new_state,
            message_hash: *proof_hash,
            timestamp,
            sequence: self.sequence,
        };
        
        self.add_to_history(link.clone());
        link
    }

    /// Record a re-key event
    pub fn add_rekey(&mut self, rekey_proof: &[u8; 32]) -> ChainLink {
        self.sequence += 1;
        let timestamp = Self::current_timestamp();
        
        let new_state = self.compute_new_state(rekey_proof, timestamp);
        self.state = new_state;
        
        let link = ChainLink {
            link_type: ChainLinkType::ReKey,
            state: new_state,
            message_hash: *rekey_proof,
            timestamp,
            sequence: self.sequence,
        };
        
        self.add_to_history(link.clone());
        link
    }

    fn compute_new_state(&self, input: &[u8; 32], timestamp: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.state);
        hasher.update(input);
        hasher.update(&timestamp.to_be_bytes());
        hasher.update(&self.sequence.to_be_bytes());
        let result = hasher.finalize();
        let mut new_state = [0u8; 32];
        new_state.copy_from_slice(&result);
        new_state
    }

    fn add_to_history(&mut self, link: ChainLink) {
        self.history.push(link);
        
        // Trim history if too long
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Verify chain integrity
    pub fn verify_integrity(&self) -> Result<()> {
        if self.history.is_empty() {
            return Err(CryptoError::InvalidChainState("Empty chain".to_string()));
        }

        // Verify each link
        for i in 1..self.history.len() {
            let prev = &self.history[i - 1];
            let curr = &self.history[i];
            
            if !self.verify_link_transition(prev, curr) {
                return Err(CryptoError::InvalidChainState(format!(
                    "Invalid transition at sequence {}",
                    curr.sequence
                )));
            }
        }

        // Verify current state matches last link
        if let Some(last) = self.history.last() {
            if last.state != self.state {
                return Err(CryptoError::InvalidChainState(
                    "Current state doesn't match history".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn verify_link_transition(&self, prev: &ChainLink, curr: &ChainLink) -> bool {
        // Sequence should increase by 1
        if curr.sequence != prev.sequence + 1 {
            return false;
        }

        // Timestamp should not decrease
        if curr.timestamp < prev.timestamp {
            return false;
        }

        // Recompute expected state
        let mut hasher = Sha256::new();
        hasher.update(&prev.state);
        hasher.update(&curr.message_hash);
        hasher.update(&curr.timestamp.to_be_bytes());
        hasher.update(&(prev.sequence + 1).to_be_bytes());
        let result = hasher.finalize();
        let mut expected_state = [0u8; 32];
        expected_state.copy_from_slice(&result);

        expected_state == curr.state
    }

    /// Generate a chain proof for external verification
    pub fn generate_proof(&self) -> ChainProof {
        let mut hasher = Sha512::new();
        
        for link in &self.history {
            hasher.update(&link.state);
            hasher.update(&link.message_hash);
            hasher.update(&link.timestamp.to_be_bytes());
        }
        
        let result = hasher.finalize();
        let mut proof = [0u8; 64];
        proof.copy_from_slice(&result);
        
        ChainProof {
            current_state: self.state,
            sequence: self.sequence,
            proof,
            link_count: self.history.len() as u64,
        }
    }

    /// Get history (for debugging/verification)
    pub fn history(&self) -> &[ChainLink] {
        &self.history
    }

    /// Get a specific link by sequence number
    pub fn get_link(&self, sequence: u64) -> Option<&ChainLink> {
        self.history.iter().find(|l| l.sequence == sequence)
    }
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

/// Proof of chain state for external verification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainProof {
    /// Current chain state
    #[serde(with = "hex::serde")]
    pub current_state: [u8; 32],
    /// Current sequence number
    pub sequence: u64,
    /// Merkle-like proof over history
    #[serde(with = "hex::serde")]
    pub proof: [u8; 64],
    /// Number of links in chain
    pub link_count: u64,
}

/// Verifier for chain proofs
pub struct ChainVerifier;

impl ChainVerifier {
    /// Verify a chain of links
    pub fn verify_chain(links: &[ChainLink]) -> Result<()> {
        if links.is_empty() {
            return Err(CryptoError::InvalidChainState("Empty chain".to_string()));
        }

        // First link should be init
        if links[0].link_type != ChainLinkType::Init {
            return Err(CryptoError::InvalidChainState(
                "Chain must start with Init link".to_string(),
            ));
        }

        for i in 1..links.len() {
            let prev = &links[i - 1];
            let curr = &links[i];

            // Verify sequence
            if curr.sequence != prev.sequence + 1 {
                return Err(CryptoError::InvalidChainState(format!(
                    "Sequence gap at {}",
                    curr.sequence
                )));
            }

            // Verify state transition
            let mut hasher = Sha256::new();
            hasher.update(&prev.state);
            hasher.update(&curr.message_hash);
            hasher.update(&curr.timestamp.to_be_bytes());
            hasher.update(&curr.sequence.to_be_bytes());
            let result = hasher.finalize();
            let mut expected = [0u8; 32];
            expected.copy_from_slice(&result);

            if expected != curr.state {
                return Err(CryptoError::InvalidChainState(format!(
                    "Invalid state at sequence {}",
                    curr.sequence
                )));
            }
        }

        Ok(())
    }

    /// Verify that a link belongs to a chain with given state
    pub fn verify_link_membership(
        link: &ChainLink,
        expected_subsequent_state: &[u8; 32],
        subsequent_link: &ChainLink,
    ) -> bool {
        // Verify the subsequent link derives from this link's state
        let mut hasher = Sha256::new();
        hasher.update(&link.state);
        hasher.update(&subsequent_link.message_hash);
        hasher.update(&subsequent_link.timestamp.to_be_bytes());
        hasher.update(&subsequent_link.sequence.to_be_bytes());
        let result = hasher.finalize();
        let mut computed = [0u8; 32];
        computed.copy_from_slice(&result);

        computed == *expected_subsequent_state && subsequent_link.state == *expected_subsequent_state
    }
}

/// Compute message hash for chain
pub fn compute_message_hash(ciphertext: &[u8], header: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ciphertext);
    hasher.update(header);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_creation() {
        let chain = ChainState::new();
        assert_eq!(chain.sequence(), 0);
        assert!(!chain.history().is_empty());
    }

    #[test]
    fn test_add_messages() {
        let mut chain = ChainState::new();
        
        let msg_hash1 = [0x01u8; 32];
        let link1 = chain.add_message(&msg_hash1);
        assert_eq!(link1.sequence, 1);
        assert_eq!(link1.link_type, ChainLinkType::Message);
        
        let msg_hash2 = [0x02u8; 32];
        let link2 = chain.add_message(&msg_hash2);
        assert_eq!(link2.sequence, 2);
        
        // States should be different
        assert_ne!(link1.state, link2.state);
    }

    #[test]
    fn test_chain_integrity() {
        let mut chain = ChainState::new();
        
        for i in 0..10 {
            let hash = [i as u8; 32];
            chain.add_message(&hash);
        }
        
        assert!(chain.verify_integrity().is_ok());
    }

    #[test]
    fn test_chain_verifier() {
        let mut chain = ChainState::new();
        
        for i in 0..5 {
            let hash = [i as u8; 32];
            chain.add_message(&hash);
        }
        
        let links = chain.history().to_vec();
        assert!(ChainVerifier::verify_chain(&links).is_ok());
    }

    #[test]
    fn test_tampered_chain() {
        let mut chain = ChainState::new();
        
        for i in 0..5 {
            let hash = [i as u8; 32];
            chain.add_message(&hash);
        }
        
        let mut links = chain.history().to_vec();
        // Tamper with a link
        links[2].message_hash[0] ^= 0xFF;
        
        assert!(ChainVerifier::verify_chain(&links).is_err());
    }

    #[test]
    fn test_deletion_link() {
        let mut chain = ChainState::new();
        
        let msg_hash = [0x01u8; 32];
        chain.add_message(&msg_hash);
        
        let del_link = chain.add_deletion(&msg_hash);
        assert_eq!(del_link.link_type, ChainLinkType::Deletion);
        
        assert!(chain.verify_integrity().is_ok());
    }

    #[test]
    fn test_chain_proof() {
        let mut chain = ChainState::new();
        
        for i in 0..10 {
            let hash = [i as u8; 32];
            chain.add_message(&hash);
        }
        
        let proof = chain.generate_proof();
        assert_eq!(proof.sequence, 10);
        assert_eq!(proof.current_state, *chain.current_state());
    }

    #[test]
    fn test_from_shared_secret() {
        let secret = [0x42u8; 32];
        let chain1 = ChainState::from_shared_secret(&secret);
        let chain2 = ChainState::from_shared_secret(&secret);
        
        // Same secret should produce same initial state
        assert_eq!(chain1.current_state(), chain2.current_state());
        
        // Different secret should produce different state
        let other_secret = [0x43u8; 32];
        let chain3 = ChainState::from_shared_secret(&other_secret);
        assert_ne!(chain1.current_state(), chain3.current_state());
    }
}
