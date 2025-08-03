//! Internal handle referencing a slab block.
//!
//! Exposed within the crate for coordinating slab access without leaking
//! indices outside the module boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Handle(pub(crate) usize);
