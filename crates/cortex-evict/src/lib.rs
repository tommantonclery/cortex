#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Built-in eviction policies for the Cortex cache.

/// Random eviction policy implementation.
pub mod random;
/// Time-to-live eviction policy.
pub mod ttl;
/// Least-frequently-used eviction policy.
pub mod lfu;

pub use random::Random;
pub use ttl::Ttl;
pub use lfu::Lfu;
