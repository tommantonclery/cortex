#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Core components of the Cortex cache.

mod entry;
mod index;
mod shard;
mod handle;
mod slab;

pub use entry::Entry;
pub use index::Index;
pub use shard::Shard;
pub use slab::Slab;
pub(crate) use handle::Handle;
