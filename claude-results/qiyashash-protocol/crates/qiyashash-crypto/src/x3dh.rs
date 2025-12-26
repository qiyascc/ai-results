//! X3DH (Extended Triple Diffie-Hellman) Key Agreement
//!
//! Implements the X3DH protocol for asynchronous session establishment.
//! Based on the Signal Protocol's X3DH specification.
//!
//! # Protocol Overview
//!
//! Alice (initiator) and Bob (responder) establish a shared secret:
//!
//! 1. Bob publishes his identity key (IK), signed pre-key (SPK), and
//!    optional one-time pre-keys (OPK) to a server
//! 2. Alice fetches Bob's pre-key bundle
//! 3. Alice generates an ephemeral key (EK) and computes:
//!    - DH1 = DH(IK_A, SPK_B)
//!    - DH2 = DH(EK_A, IK_B)  
//!    - DH3 = DH(EK_A, SPK_B)
//!    - DH4 = DH(EK_A, OPK_B) (if OPK present)
//! 4. Shared secret = KDF(DH1 || DH2 || DH3 || DH4)

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CryptoError, Result};
use crate::identity::{IdentityKeyPair, IdentityPublicKey};
use crate::kdf::{domain, KeyDerivationContext};
use crate::keys::{EphemeralKeyPair, PublicKeyBytes, SharedSecret, SignedPreKey, OneTimePreKey, PreKeyBundle};

/// X3DH shared secret
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct X3DHSharedSecret {
    /// The derived shared secret
    secret: [u8; 32],
    /// Associated data for the first message
    ad: Vec<u8>,
}

impl X3DHSharedSecret {
    /// Get the shared secret bytes
    pub fn secret(&self) -> &[u8; 32] {
        &self.secret
    }

    /// Get the associated data
    pub fn associated_data(&self) -> &[u8] {
        &self.ad
    }
}

/// Pre-key manager for generating and storing pre-keys
pub struct PreKeyManager {
    /// Identity key pair
    identity: IdentityKeyPair,
    /// Signed pre-key (rotated periodically)
    signed_prekey: SignedPreKeyPair,
    /// One-time pre-keys (each used once)
    one_time_prekeys: Vec<OneTimePreKeyPair>,
    /// Counter for one-time pre-key IDs
    opk_counter: u32,
}

/// Signed pre-key pair (private + public)
#[derive(ZeroizeOnDrop)]
struct SignedPreKeyPair {
    id: u32,
    secret: X25519StaticSecret,
    #[zeroize(skip)]
    public: X25519PublicKey,
    signature: [u8; 64],
    timestamp: i64,
}

/// One-time pre-key pair
#[derive(ZeroizeOnDrop)]
struct OneTimePreKeyPair {
    id: u32,
    secret: X25519StaticSecret,
    #[zeroize(skip)]
    public: X25519PublicKey,
}

impl PreKeyManager {
    /// Create a new pre-key manager
    pub fn new(identity: IdentityKeyPair) -> Self {
        let signed_prekey = Self::generate_signed_prekey(&identity, 1);
        
        Self {
            identity,
            signed_prekey,
            one_time_prekeys: Vec::new(),
            opk_counter: 0,
        }
    }

    fn generate_signed_prekey(identity: &IdentityKeyPair, id: u32) -> SignedPreKeyPair {
        let secret = X25519StaticSecret::random_from_rng(OsRng);
        let public = X25519PublicKey::from(&secret);
        let timestamp = chrono::Utc::now().timestamp();
        
        // Sign the public key
        let signature = identity.sign(public.as_bytes());
        
        SignedPreKeyPair {
            id,
            secret,
            public,
            signature,
            timestamp,
        }
    }

    /// Generate one-time pre-keys
    pub fn generate_one_time_prekeys(&mut self, count: usize) {
        for _ in 0..count {
            self.opk_counter += 1;
            let secret = X25519StaticSecret::random_from_rng(OsRng);
            let public = X25519PublicKey::from(&secret);
            
            self.one_time_prekeys.push(OneTimePreKeyPair {
                id: self.opk_counter,
                secret,
                public,
            });
        }
    }

    /// Get the pre-key bundle for publishing
    pub fn get_bundle(&self) -> PreKeyBundle {
        let opk = self.one_time_prekeys.first().map(|opk| OneTimePreKey {
            id: opk.id,
            public_key: PublicKeyBytes::from_x25519(&opk.public),
        });
        
        PreKeyBundle {
            identity_key: self.identity.public_key().signing_key_bytes(),
            signed_prekey: SignedPreKey {
                id: self.signed_prekey.id,
                public_key: PublicKeyBytes::from_x25519(&self.signed_prekey.public),
                signature: self.signed_prekey.signature,
                timestamp: self.signed_prekey.timestamp,
            },
            one_time_prekey: opk,
        }
    }

    /// Consume a one-time pre-key (called when receiving initial message)
    pub fn consume_one_time_prekey(&mut self, id: u32) -> Option<X25519StaticSecret> {
        if let Some(pos) = self.one_time_prekeys.iter().position(|k| k.id == id) {
            let opk = self.one_time_prekeys.remove(pos);
            Some(opk.secret)
        } else {
            None
        }
    }

    /// Get the signed pre-key secret
    pub fn signed_prekey_secret(&self) -> &X25519StaticSecret {
        &self.signed_prekey.secret
    }

    /// Rotate signed pre-key
    pub fn rotate_signed_prekey(&mut self) {
        let new_id = self.signed_prekey.id + 1;
        self.signed_prekey = Self::generate_signed_prekey(&self.identity, new_id);
    }

    /// Get identity key pair reference
    pub fn identity(&self) -> &IdentityKeyPair {
        &self.identity
    }
}

/// X3DH key agreement
pub struct X3DHKeyAgreement;

impl X3DHKeyAgreement {
    /// Initiator (Alice) computes shared secret from Bob's pre-key bundle
    ///
    /// # Arguments
    /// * `our_identity` - Alice's identity key pair
    /// * `their_bundle` - Bob's published pre-key bundle
    ///
    /// # Returns
    /// (shared_secret, ephemeral_public_key, used_one_time_prekey_id)
    pub fn initiate(
        our_identity: &IdentityKeyPair,
        their_bundle: &PreKeyBundle,
    ) -> Result<(X3DHSharedSecret, PublicKeyBytes, Option<u32>)> {
        // Verify signed pre-key signature
        let their_identity = IdentityPublicKey::from_bytes(&their_bundle.identity_key)?;
        their_bundle.signed_prekey.verify(&their_identity.signing_key)?;
        
        // Generate ephemeral key
        let ephemeral = EphemeralKeyPair::generate();
        
        // Perform DH computations
        let spk_public = their_bundle.signed_prekey.public_key.to_x25519();
        
        // DH1 = DH(IK_A, SPK_B)
        let dh1 = our_identity.diffie_hellman(&spk_public);
        
        // DH2 = DH(EK_A, IK_B)
        let dh2 = ephemeral.diffie_hellman(&their_identity.dh_key);
        
        // DH3 = DH(EK_A, SPK_B)
        let dh3 = ephemeral.diffie_hellman(&spk_public);
        
        // DH4 = DH(EK_A, OPK_B) if OPK present
        let (dh4, opk_id) = if let Some(ref opk) = their_bundle.one_time_prekey {
            let opk_public = opk.public_key.to_x25519();
            (Some(ephemeral.diffie_hellman(&opk_public)), Some(opk.id))
        } else {
            (None, None)
        };
        
        // Derive shared secret
        let shared_secret = Self::derive_shared_secret(
            &dh1, &dh2, &dh3, dh4.as_ref(),
            &our_identity.public_key(),
            &their_identity,
        )?;
        
        Ok((
            shared_secret,
            PublicKeyBytes::from_x25519(ephemeral.public_key()),
            opk_id,
        ))
    }

    /// Responder (Bob) computes shared secret from Alice's initial message
    ///
    /// # Arguments
    /// * `our_prekeys` - Bob's pre-key manager
    /// * `their_identity` - Alice's identity public key
    /// * `their_ephemeral` - Alice's ephemeral public key
    /// * `used_opk_id` - ID of the one-time pre-key Alice used (if any)
    pub fn respond(
        our_prekeys: &mut PreKeyManager,
        their_identity: &IdentityPublicKey,
        their_ephemeral: &PublicKeyBytes,
        used_opk_id: Option<u32>,
    ) -> Result<X3DHSharedSecret> {
        let ephemeral_public = their_ephemeral.to_x25519();
        
        // DH1 = DH(SPK_B, IK_A)
        let dh1 = {
            let shared = our_prekeys.signed_prekey_secret().diffie_hellman(&their_identity.dh_key);
            SharedSecret(*shared.as_bytes())
        };
        
        // DH2 = DH(IK_B, EK_A)
        let dh2 = our_prekeys.identity().diffie_hellman(&ephemeral_public);
        
        // DH3 = DH(SPK_B, EK_A)
        let dh3 = {
            let shared = our_prekeys.signed_prekey_secret().diffie_hellman(&ephemeral_public);
            SharedSecret(*shared.as_bytes())
        };
        
        // DH4 = DH(OPK_B, EK_A) if OPK was used
        let dh4 = if let Some(opk_id) = used_opk_id {
            let opk_secret = our_prekeys.consume_one_time_prekey(opk_id)
                .ok_or_else(|| CryptoError::PrekeyNotFound(format!("OPK {}", opk_id)))?;
            let shared = opk_secret.diffie_hellman(&ephemeral_public);
            Some(SharedSecret(*shared.as_bytes()))
        } else {
            None
        };
        
        // Derive shared secret
        Self::derive_shared_secret(
            &dh1, &dh2, &dh3, dh4.as_ref(),
            their_identity,
            &our_prekeys.identity().public_key(),
        )
    }

    /// Derive the final shared secret using HKDF
    fn derive_shared_secret(
        dh1: &SharedSecret,
        dh2: &SharedSecret,
        dh3: &SharedSecret,
        dh4: Option<&SharedSecret>,
        initiator_identity: &IdentityPublicKey,
        responder_identity: &IdentityPublicKey,
    ) -> Result<X3DHSharedSecret> {
        // Concatenate DH outputs
        let mut dh_concat = Vec::with_capacity(128);
        
        // Add 32 bytes of 0xFF as a domain separator (Signal protocol convention)
        dh_concat.extend_from_slice(&[0xFF; 32]);
        dh_concat.extend_from_slice(dh1.as_bytes());
        dh_concat.extend_from_slice(dh2.as_bytes());
        dh_concat.extend_from_slice(dh3.as_bytes());
        
        if let Some(dh4) = dh4 {
            dh_concat.extend_from_slice(dh4.as_bytes());
        }
        
        // Derive shared secret
        let kdf = KeyDerivationContext::new(None, &dh_concat);
        let secret = kdf.derive::<32>(domain::ROOT_KEY)?;
        
        // Create associated data: initiator_identity || responder_identity
        let mut ad = Vec::with_capacity(128);
        ad.extend_from_slice(&initiator_identity.to_bytes());
        ad.extend_from_slice(&responder_identity.to_bytes());
        
        Ok(X3DHSharedSecret {
            secret: secret.into_bytes(),
            ad,
        })
    }
}

/// Initial message header for X3DH
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct X3DHHeader {
    /// Sender's identity public key
    pub identity_key: PublicKeyBytes,
    /// Ephemeral public key used in X3DH
    pub ephemeral_key: PublicKeyBytes,
    /// ID of one-time pre-key used (if any)
    pub one_time_prekey_id: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x3dh_key_agreement() {
        // Setup Alice and Bob
        let alice_identity = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();
        
        let mut bob_prekeys = PreKeyManager::new(bob_identity);
        bob_prekeys.generate_one_time_prekeys(10);
        
        // Alice initiates
        let bob_bundle = bob_prekeys.get_bundle();
        let (alice_secret, ephemeral, opk_id) = 
            X3DHKeyAgreement::initiate(&alice_identity, &bob_bundle).unwrap();
        
        // Bob responds
        let alice_public = alice_identity.public_key();
        let bob_secret = X3DHKeyAgreement::respond(
            &mut bob_prekeys,
            &alice_public,
            &ephemeral,
            opk_id,
        ).unwrap();
        
        // Both should derive the same secret
        assert_eq!(alice_secret.secret(), bob_secret.secret());
        
        // Associated data should match
        assert_eq!(alice_secret.associated_data(), bob_secret.associated_data());
    }

    #[test]
    fn test_x3dh_without_one_time_prekey() {
        let alice_identity = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();
        
        // Bob has no one-time prekeys
        let mut bob_prekeys = PreKeyManager::new(bob_identity);
        
        let bob_bundle = bob_prekeys.get_bundle();
        assert!(bob_bundle.one_time_prekey.is_none());
        
        let (alice_secret, ephemeral, opk_id) = 
            X3DHKeyAgreement::initiate(&alice_identity, &bob_bundle).unwrap();
        
        assert!(opk_id.is_none());
        
        let alice_public = alice_identity.public_key();
        let bob_secret = X3DHKeyAgreement::respond(
            &mut bob_prekeys,
            &alice_public,
            &ephemeral,
            opk_id,
        ).unwrap();
        
        assert_eq!(alice_secret.secret(), bob_secret.secret());
    }

    #[test]
    fn test_prekey_rotation() {
        let identity = IdentityKeyPair::generate();
        let mut prekeys = PreKeyManager::new(identity);
        
        let bundle1 = prekeys.get_bundle();
        let spk_id1 = bundle1.signed_prekey.id;
        
        prekeys.rotate_signed_prekey();
        
        let bundle2 = prekeys.get_bundle();
        let spk_id2 = bundle2.signed_prekey.id;
        
        assert_eq!(spk_id2, spk_id1 + 1);
        assert_ne!(bundle1.signed_prekey.public_key, bundle2.signed_prekey.public_key);
    }

    #[test]
    fn test_one_time_prekey_consumption() {
        let identity = IdentityKeyPair::generate();
        let mut prekeys = PreKeyManager::new(identity);
        
        prekeys.generate_one_time_prekeys(5);
        assert_eq!(prekeys.one_time_prekeys.len(), 5);
        
        let bundle = prekeys.get_bundle();
        let opk_id = bundle.one_time_prekey.unwrap().id;
        
        // Consume the OPK
        let secret = prekeys.consume_one_time_prekey(opk_id);
        assert!(secret.is_some());
        
        // Trying to consume again should fail
        let secret2 = prekeys.consume_one_time_prekey(opk_id);
        assert!(secret2.is_none());
        
        // Should have one less OPK now
        assert_eq!(prekeys.one_time_prekeys.len(), 4);
    }

    #[test]
    fn test_invalid_signed_prekey_signature() {
        let alice_identity = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();
        
        let bob_prekeys = PreKeyManager::new(bob_identity);
        let mut bob_bundle = bob_prekeys.get_bundle();
        
        // Tamper with signature
        bob_bundle.signed_prekey.signature[0] ^= 0xFF;
        
        let result = X3DHKeyAgreement::initiate(&alice_identity, &bob_bundle);
        assert!(result.is_err());
    }
}
