//! # Ring
//!
//! Safe, fast, small crypto using Rust & C.
//!
//! This is a wrapper crate that automatically selects the appropriate ring
//! implementation based on the target platform:
//! - For non-redox targets: uses the upstream ring crate
//! - For redox targets: uses the redox fork

#![no_std]

// For non-redox targets, re-export from the upstream ring crate
#[cfg(not(target_os = "redox"))]
pub use ring_upstream::*;

// For redox targets, re-export from the ring crate (which is the redox fork)
#[cfg(target_os = "redox")]
pub use ring_redox::*;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
pub use std::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_wrapper() {
        // Test that basic ring functionality works through our wrapper
        #[cfg(not(target_os = "redox"))]
        {
            // Test that we can use ring functionality
            let algorithm = &digest::SHA256;
            let data = b"hello world";
            let _hash = digest::digest(algorithm, data);
        }

        #[cfg(target_os = "redox")]
        {
            // Test that we can use ring functionality on redox
            let algorithm = &digest::SHA256;
            let data = b"hello world";
            let _hash = digest::digest(algorithm, data);
        }
    }
}
