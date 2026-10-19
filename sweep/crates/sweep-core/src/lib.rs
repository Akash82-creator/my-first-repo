//! Platform-neutral core of Sweep.
//!
//! Nothing in this crate touches the operating system directly; Windows access
//! goes through the traits in [`api`], implemented by `sweep-win` and by the
//! in-memory fakes in [`fakes`]. Windows paths are modelled by [`winpath`] and
//! never by `std::path`, so behaviour is identical when tested on Linux.
//!
//! Stage A is read-only: this crate contains no code that deletes, moves or
//! writes anything.

pub mod api;
pub mod cmdline;
pub mod evidence;
pub mod inventory;
pub mod launch;
pub mod model;
pub mod msi;
pub mod outcome;
pub mod protect;
pub mod scan;
pub mod winpath;

#[cfg(any(test, feature = "fakes"))]
pub mod fakes;

/// Build-time statement of which destructive features exist. Stage A ships with
/// Sweep's own cleanup disabled; there is no code path that performs it.
pub const CLEANUP_ENABLED: bool = false;
pub const BUILD_STAGE: &str = "A";

#[cfg(test)]
mod acceptance_tests;
