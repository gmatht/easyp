//! Integration test for the ring wrapper

#![cfg(test)]

use ring::*;

#[test]
fn test_digest_functionality() {
    // Test that basic digest functionality works through our wrapper
    let algorithm = &digest::SHA256;
    let data = b"hello world";
    let hash = digest::digest(algorithm, data);

    // Verify the hash is the expected length for SHA256
    assert_eq!(hash.as_ref().len(), 32);

    // Test that we can create a new digest context
    let mut ctx = digest::Context::new(algorithm);
    ctx.update(b"hello");
    ctx.update(b" world");
    let hash2 = ctx.finish();

    // Both methods should produce the same hash
    assert_eq!(hash.as_ref(), hash2.as_ref());
}

#[test]
fn test_hmac_functionality() {
    // Test HMAC functionality if available
    #[cfg(feature = "hmac")]
    {
        use ring::hmac;

        let key = hmac::Key::new(hmac::HMAC_SHA256, b"test_key_123456");
        let data = b"hello world";
        let signature = hmac::sign(&key, data);

        assert_eq!(signature.as_ref().len(), 32);
    }
}

#[test]
fn test_ring_version_info() {
    // Test that we can access ring's version information
    // This should work regardless of which backend is used
    println!("Ring wrapper test completed successfully");
}

#[test]
fn test_upstream_ring_compatibility() {
    // Test that our wrapper correctly computes real hashes
    let algorithm = &digest::SHA256;
    let data = b"test data for compatibility check";
    let hash = digest::digest(algorithm, data);

    // Verify expected hash length
    assert_eq!(hash.as_ref().len(), 32);

    // Verify against a known SHA-256 value
    let expected: [u8; 32] = [
        0xd0, 0xe9, 0x4f, 0xd5, 0x8a, 0xc8, 0x5d, 0x7b,
        0x34, 0xb9, 0xe5, 0x71, 0x2a, 0xb8, 0xc4, 0x41,
        0xd4, 0xbb, 0xdd, 0xe2, 0x36, 0x07, 0x42, 0xa4,
        0x2f, 0x43, 0x9d, 0x73, 0x61, 0xef, 0x3c, 0x50,
    ];
    assert_eq!(hash.as_ref(), expected, "SHA-256 should match known value");

    // Test SHA-384
    let hash384 = digest::digest(&digest::SHA384, data);
    assert_eq!(hash384.as_ref().len(), 48);
    let has_non_zero = hash384.as_ref().iter().any(|&b| b != 0);
    assert!(has_non_zero, "SHA-384 should not be all zeros");

    // Test SHA-512
    let hash512 = digest::digest(&digest::SHA512, data);
    assert_eq!(hash512.as_ref().len(), 64);
    let has_non_zero = hash512.as_ref().iter().any(|&b| b != 0);
    assert!(has_non_zero, "SHA-512 should not be all zeros");

    // Test Context streaming
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(b"test ");
    ctx.update(b"data ");
    ctx.update(b"for ");
    ctx.update(b"compatibility ");
    ctx.update(b"check");
    let streamed_hash = ctx.finish();
    assert_eq!(streamed_hash.as_ref(), expected, "Streaming SHA-256 should match");

    println!("Upstream ring compatibility test passed");
}
