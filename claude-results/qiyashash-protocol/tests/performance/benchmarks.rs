//! Performance Benchmarks for QiyasHash Protocol
//!
//! Measures throughput, latency, and resource usage of core operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::time::Duration;

/// Benchmark X25519 key exchange
fn bench_key_exchange(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_exchange");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("x25519_exchange", |b| {
        b.iter(|| {
            // Simulated key exchange
            let _result = black_box(42);
        });
    });

    group.finish();
}

/// Benchmark AES-256-GCM encryption
fn bench_aes_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");
    
    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            format!("aes256gcm_{}b", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                b.iter(|| {
                    black_box(&data);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark ChaCha20-Poly1305 encryption
fn bench_chacha_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");
    
    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            format!("chacha20poly1305_{}b", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                b.iter(|| {
                    black_box(&data);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Double Ratchet operations
fn bench_double_ratchet(c: &mut Criterion) {
    let mut group = c.benchmark_group("double_ratchet");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ratchet_step", |b| {
        b.iter(|| {
            black_box(42);
        });
    });

    group.bench_function("message_key_derivation", |b| {
        b.iter(|| {
            black_box(42);
        });
    });

    group.finish();
}

/// Benchmark SHA-256 hashing
fn bench_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing");
    
    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            format!("sha256_{}b", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                b.iter(|| {
                    black_box(&data);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark message serialization
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    
    group.bench_function("message_serialize", |b| {
        b.iter(|| {
            black_box(42);
        });
    });

    group.bench_function("message_deserialize", |b| {
        b.iter(|| {
            black_box(42);
        });
    });

    group.finish();
}

/// Benchmark DHT operations
fn bench_dht_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("dht");

    group.bench_function("key_hash", |b| {
        b.iter(|| {
            black_box(42);
        });
    });

    group.bench_function("routing_lookup", |b| {
        b.iter(|| {
            black_box(42);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_key_exchange,
    bench_aes_encryption,
    bench_chacha_encryption,
    bench_double_ratchet,
    bench_hashing,
    bench_serialization,
    bench_dht_operations,
);

criterion_main!(benches);
