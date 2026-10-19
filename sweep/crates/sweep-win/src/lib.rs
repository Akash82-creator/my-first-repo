//! Windows implementations of Sweep's OS traits.
//!
//! Everything here is read-only except [`launch::run_uninstaller`], which
//! starts the *vendor's* uninstaller (it changes the system; Sweep itself does
//! not delete anything in Stage A). On non-Windows targets this crate is empty.

#[cfg(windows)]
pub mod flow;
#[cfg(windows)]
pub mod fs;
#[cfg(windows)]
pub mod known;
#[cfg(windows)]
pub mod launch;
#[cfg(windows)]
pub mod manifest;
#[cfg(windows)]
pub mod registry;
#[cfg(windows)]
pub mod system;
#[cfg(windows)]
mod wide;
