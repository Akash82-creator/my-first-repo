use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

/// NUL-terminated UTF-16 for Win32 calls.
pub fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Extended-length form of a validated local path.
pub fn extended(p: &sweep_core::winpath::WinPath) -> String {
    format!("\\\\?\\{p}")
}
