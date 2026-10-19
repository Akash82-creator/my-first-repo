//! Process token, Windows Installer state and pending-rename queries.

use crate::registry::WinReg;
use sweep_core::api::RegRead;
use sweep_core::model::{RegRoot, RegValue, RegView};
use sweep_core::msi::MsiProductState;
use sweep_core::winpath::WinPath;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::ApplicationInstallationAndServicing::MsiQueryProductStateW;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Whether this process runs with an elevated token.
pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut el = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut el as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && el.TokenIsElevated != 0
    }
}

pub fn msi_product_state(product_code: &str) -> MsiProductState {
    let w = crate::wide::wide(product_code);
    let state = unsafe { MsiQueryProductStateW(PCWSTR(w.as_ptr())) };
    MsiProductState::from_raw(state.0)
}

/// Destination-or-source paths in PendingFileRenameOperations.
pub fn pending_renames() -> Vec<String> {
    let path = "SYSTEM\\CurrentControlSet\\Control\\Session Manager";
    match WinReg.values(RegRoot::Hklm, RegView::Reg64, path) {
        Ok(v) => match v.get("PendingFileRenameOperations") {
            Some(RegValue::MultiSz(items)) => items.clone(),
            _ => vec![],
        },
        Err(_) => vec![],
    }
}

/// Whether `after` has entries not in `before` that point inside `dir`.
pub fn new_renames_under(before: &[String], after: &[String], dir: Option<&WinPath>) -> bool {
    let Some(dir) = dir else { return !after.iter().all(|a| before.contains(a)) };
    after.iter().filter(|a| !before.contains(a)).any(|a| {
        let s = a.trim_start_matches('!').trim_start_matches("\\??\\");
        WinPath::from_api(s).map(|p| p.is_within(dir)).unwrap_or(false)
    })
}
