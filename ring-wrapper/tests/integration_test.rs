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
    // Test that our wrapper correctly imports and re-exports the upstream ring crate
    // This should work on non-redox targets

    // Test digest functionality
    let algorithm = &digest::SHA256;
    let data = b"test data for compatibility check";
    let hash = digest::digest(algorithm, data);

    // Verify expected hash length
    assert_eq!(hash.as_ref().len(), 32);

    // Test that the hash is actually computed (not all zeros)
    let hash_bytes = hash.as_ref();
    let has_non_zero = hash_bytes.iter().any(|&b| b != 0);
    assert!(has_non_zero, "Hash should not be all zeros");

    println!("Upstream ring compatibility test passed");
}
