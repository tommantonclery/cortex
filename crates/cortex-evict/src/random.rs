use cortex_api::EvictionPolicy;

/// Random eviction policy.
pub struct Random;

impl EvictionPolicy for Random {}
