#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Core components of the Cortex cache.

mod shard;
mod slab;
mod entry;
mod index;

pub use shard::Shard;
pub use slab::Slab;
pub use entry::Entry;
pub use index::Index;
