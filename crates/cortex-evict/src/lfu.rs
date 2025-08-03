use cortex_api::EvictionPolicy;

/// Least-frequently-used eviction policy.
pub struct Lfu;

impl EvictionPolicy for Lfu {}
