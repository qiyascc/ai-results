//! Benchmarks for QiyasHash cryptographic operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use qiyashash_crypto::{
    aead::{Aead, AeadAlgorithm, AeadKey},
    identity::{Identity, IdentityKeyPair},
    keys::EphemeralKeyPair,
    kdf::{ChainRatchet, KeyDerivationContext, derive_message_keys},
    ratchet::DoubleRatchet,
    x3dh::{PreKeyManager, X3DHKeyAgreement},
};
use rand::rngs::OsRng;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

fn bench_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Key Generation");

    group.bench_function("identity_keypair", |b| {
        b.iter(|| black_box(IdentityKeyPair::generate()))
    });

    group.bench_function("ephemeral_keypair", |b| {
        b.iter(|| black_box(EphemeralKeyPair::generate()))
    });

    group.bench_function("identity_full", |b| {
        b.iter(|| black_box(Identity::new()))
    });

    group.finish();
}

fn bench_diffie_hellman(c: &mut Criterion) {
    let alice = EphemeralKeyPair::generate();
    let bob = EphemeralKeyPair::generate();

    c.bench_function("x25519_dh", |b| {
        b.iter(|| black_box(alice.diffie_hellman(bob.public_key())))
    });
}

fn bench_kdf(c: &mut Criterion) {
    let mut group = c.benchmark_group("KDF");

    let ikm = [0x42u8; 32];
    let salt = [0x00u8; 32];

    group.bench_function("hkdf_derive_32", |b| {
        let kdf = KeyDerivationContext::new(Some(&salt), &ikm);
        b.iter(|| {
            let key: [u8; 32] = kdf.derive::<32>(b"context").unwrap().into_bytes();
            black_box(key)
        })
    });

    group.bench_function("chain_ratchet", |b| {
        b.iter(|| {
            let mut ratchet = ChainRatchet::new([0x42u8; 32]);
            black_box(ratchet.ratchet())
        })
    });

    group.bench_function("derive_message_keys", |b| {
        let chain_key = [0x42u8; 32];
        b.iter(|| black_box(derive_message_keys(&chain_key)))
    });

    group.finish();
}

fn bench_aead(c: &mut Criterion) {
    let mut group = c.benchmark_group("AEAD");

    let key = AeadKey::from_bytes([0x42; 32]);
    let aad = b"associated data";

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let plaintext = vec![0x42u8; *size];

        group.bench_with_input(
            BenchmarkId::new("xchacha_encrypt", size),
            size,
            |b, _| {
                let cipher = Aead::new();
                b.iter(|| black_box(cipher.encrypt(&key, &plaintext, aad).unwrap()))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("aes_gcm_encrypt", size),
            size,
            |b, _| {
                let cipher = Aead::with_algorithm(AeadAlgorithm::Aes256Gcm);
                b.iter(|| black_box(cipher.encrypt(&key, &plaintext, aad).unwrap()))
            },
        );
    }

    // Decryption benchmarks
    let plaintext = vec![0x42u8; 1024];
    let cipher_xchacha = Aead::new();
    let cipher_aes = Aead::with_algorithm(AeadAlgorithm::Aes256Gcm);
    
    let encrypted_xchacha = cipher_xchacha.encrypt(&key, &plaintext, aad).unwrap();
    let encrypted_aes = cipher_aes.encrypt(&key, &plaintext, aad).unwrap();

    group.bench_function("xchacha_decrypt_1kb", |b| {
        b.iter(|| black_box(cipher_xchacha.decrypt(&key, &encrypted_xchacha, aad).unwrap()))
    });

    group.bench_function("aes_gcm_decrypt_1kb", |b| {
        b.iter(|| black_box(cipher_aes.decrypt(&key, &encrypted_aes, aad).unwrap()))
    });

    group.finish();
}

fn bench_x3dh(c: &mut Criterion) {
    let mut group = c.benchmark_group("X3DH");

    group.bench_function("initiate", |b| {
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();
        let mut bob_prekeys = PreKeyManager::new(bob);
        bob_prekeys.generate_one_time_prekeys(10);
        let bundle = bob_prekeys.get_bundle();

        b.iter(|| black_box(X3DHKeyAgreement::initiate(&alice, &bundle).unwrap()))
    });

    group.bench_function("respond", |b| {
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();
        let mut bob_prekeys = PreKeyManager::new(bob);
        bob_prekeys.generate_one_time_prekeys(100);
        let bundle = bob_prekeys.get_bundle();

        let (_, ephemeral, opk_id) = X3DHKeyAgreement::initiate(&alice, &bundle).unwrap();
        let alice_public = alice.public_key();

        b.iter(|| {
            // Need fresh prekeys for each iteration since OPK is consumed
            let mut fresh_bob = IdentityKeyPair::generate();
            let mut fresh_prekeys = PreKeyManager::new(fresh_bob);
            fresh_prekeys.generate_one_time_prekeys(1);
            
            black_box(
                X3DHKeyAgreement::respond(
                    &mut fresh_prekeys,
                    &alice_public,
                    &ephemeral,
                    None, // Skip OPK to avoid consumption issues
                )
                .unwrap(),
            )
        })
    });

    group.finish();
}

fn bench_double_ratchet(c: &mut Criterion) {
    let mut group = c.benchmark_group("Double Ratchet");

    // Setup
    let shared_secret = [0x42u8; 32];
    let session_id = [0x00u8; 32];
    let bob_secret = X25519StaticSecret::random_from_rng(OsRng);
    let bob_public = X25519PublicKey::from(&bob_secret);

    group.bench_function("encrypt", |b| {
        let mut alice = DoubleRatchet::new_initiator(&shared_secret, &bob_public, session_id).unwrap();
        let plaintext = b"Hello, QiyasHash!";

        b.iter(|| black_box(alice.encrypt(plaintext).unwrap()))
    });

    group.bench_function("decrypt", |b| {
        let mut alice = DoubleRatchet::new_initiator(&shared_secret, &bob_public, session_id).unwrap();
        let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_secret.clone(), session_id);
        
        let plaintext = b"Hello, QiyasHash!";
        let encrypted = alice.encrypt(plaintext).unwrap();

        b.iter(|| {
            // Create fresh Bob for each iteration
            let mut fresh_bob = DoubleRatchet::new_responder(
                &shared_secret,
                X25519StaticSecret::from(bob_secret.to_bytes()),
                session_id,
            );
            black_box(fresh_bob.decrypt(&encrypted).unwrap())
        })
    });

    group.bench_function("roundtrip", |b| {
        b.iter(|| {
            let bob_secret = X25519StaticSecret::random_from_rng(OsRng);
            let bob_public = X25519PublicKey::from(&bob_secret);
            
            let mut alice = DoubleRatchet::new_initiator(&shared_secret, &bob_public, session_id).unwrap();
            let mut bob = DoubleRatchet::new_responder(&shared_secret, bob_secret, session_id);
            
            let encrypted = alice.encrypt(b"Hello").unwrap();
            let decrypted = bob.decrypt(&encrypted).unwrap();
            
            let response = bob.encrypt(b"World").unwrap();
            let response_decrypted = alice.decrypt(&response).unwrap();
            
            black_box((decrypted, response_decrypted))
        })
    });

    group.finish();
}

fn bench_signing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Signing");

    let identity = Identity::new();
    let message = b"Hello, QiyasHash! This is a test message for signing benchmarks.";

    group.bench_function("sign", |b| {
        b.iter(|| black_box(identity.key_pair.sign(message)))
    });

    let signature = identity.key_pair.sign(message);
    let public_key = identity.key_pair.public_key();

    group.bench_function("verify", |b| {
        b.iter(|| black_box(public_key.verify(message, &signature).unwrap()))
    });

    group.finish();
}

fn bench_identity_rotation(c: &mut Criterion) {
    c.bench_function("identity_rotation", |b| {
        let identity = Identity::new();
        b.iter(|| black_box(identity.rotate()))
    });
}

criterion_group!(
    benches,
    bench_key_generation,
    bench_diffie_hellman,
    bench_kdf,
    bench_aead,
    bench_x3dh,
    bench_double_ratchet,
    bench_signing,
    bench_identity_rotation,
);

criterion_main!(benches);
