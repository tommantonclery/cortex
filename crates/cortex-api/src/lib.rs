#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Public interfaces for the Cortex cache.

mod cache;
mod eviction;
mod filter;

pub use cache::Cache;
pub use eviction::EvictionPolicy;
pub use filter::ValueFilter;
