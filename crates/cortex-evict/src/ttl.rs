use cortex_api::EvictionPolicy;

/// Time-to-live eviction policy.
pub struct Ttl;

impl EvictionPolicy for Ttl {}
