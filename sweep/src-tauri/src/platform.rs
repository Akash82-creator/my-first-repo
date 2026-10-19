//! OS access for the app. Windows uses `sweep-win`; other platforms get an
//! inert stub so the UI can be developed, but nothing can be uninstalled.

use std::sync::atomic::AtomicBool;
use sweep_core::api::{FsRead, KnownFolders, RegRead};
use sweep_core::cmdline::UninstallCommand;
use sweep_core::evidence::EvidenceSnapshot;
use sweep_core::launch::{ExecutionLevel, LaunchPlan};
use sweep_core::outcome::UninstallState;
#[cfg(not(windows))]
use {
    std::collections::BTreeMap,
    sweep_core::api::{DirEntry, EntryMeta, FsError, RegError},
    sweep_core::model::{RegRoot, RegValue, RegView},
    sweep_core::winpath::WinPath,
};

pub struct Platform {
    pub reg: Box<dyn RegRead + Send>,
    pub fs: Box<dyn FsRead + Send>,
    pub known: KnownFolders,
    pub elevated: bool,
    pub supported: bool,
}

#[cfg(windows)]
impl Platform {
    pub fn new() -> Self {
        Platform {
            reg: Box::new(sweep_win::registry::WinReg),
            fs: Box::new(sweep_win::fs::WinFs),
            known: sweep_win::known::known_folders(),
            elevated: sweep_win::system::is_elevated(),
            supported: true,
        }
    }

    pub fn manifest_level(&self, cmd: &UninstallCommand) -> Option<ExecutionLevel> {
        match cmd {
            UninstallCommand::Exe { exe, .. } => sweep_win::manifest::execution_level(exe),
            UninstallCommand::Msi { .. } => None,
        }
    }

    pub fn run_vendor_uninstall(
        &self,
        snap: &EvidenceSnapshot,
        plan: &LaunchPlan,
        stop: &AtomicBool,
        on_state: impl FnMut(&UninstallState),
    ) -> UninstallState {
        sweep_win::flow::run_vendor_uninstall(snap, plan, stop, on_state)
    }
}

/// Inert stand-ins for non-Windows builds.
#[cfg(not(windows))]
struct Nothing;

#[cfg(not(windows))]
impl RegRead for Nothing {
    fn subkeys(&self, _: RegRoot, _: RegView, _: &str) -> Result<Vec<String>, RegError> {
        Err(RegError::NotFound)
    }
    fn values(&self, _: RegRoot, _: RegView, _: &str) -> Result<BTreeMap<String, RegValue>, RegError> {
        Err(RegError::NotFound)
    }
}

#[cfg(not(windows))]
impl FsRead for Nothing {
    fn metadata(&self, _: &WinPath) -> Result<EntryMeta, FsError> {
        Err(FsError::NotFound)
    }
    fn read_dir(&self, _: &WinPath) -> Result<Vec<DirEntry>, FsError> {
        Err(FsError::NotFound)
    }
}

#[cfg(not(windows))]
impl Platform {
    pub fn new() -> Self {
        Platform {
            reg: Box::new(Nothing),
            fs: Box::new(Nothing),
            known: KnownFolders::default(),
            elevated: false,
            supported: false,
        }
    }

    pub fn manifest_level(&self, _: &UninstallCommand) -> Option<ExecutionLevel> {
        None
    }

    pub fn run_vendor_uninstall(
        &self,
        _: &EvidenceSnapshot,
        _: &LaunchPlan,
        _: &AtomicBool,
        _: impl FnMut(&UninstallState),
    ) -> UninstallState {
        unreachable!("start_uninstall refuses on unsupported platforms")
    }
}
