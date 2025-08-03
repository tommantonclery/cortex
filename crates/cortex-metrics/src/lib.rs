#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Metrics instrumentation for the Cortex cache.

/// Counter metrics.
pub mod counter;
/// Histogram metrics.
pub mod histogram;
/// Tracing utilities.
pub mod trace;

pub use counter::Counter;
pub use histogram::Histogram;
pub use trace::Tracer;
