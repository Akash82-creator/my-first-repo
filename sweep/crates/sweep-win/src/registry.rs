//! Read-only registry access with an explicit WOW64 view on every open.

use std::collections::BTreeMap;
use sweep_core::api::{RegError, RegRead};
use sweep_core::model::{RegRoot, RegValue, RegView};
use winreg::enums::*;
use winreg::RegKey;

pub struct WinReg;

fn map_err(e: std::io::Error) -> RegError {
    match e.kind() {
        std::io::ErrorKind::NotFound => RegError::NotFound,
        std::io::ErrorKind::PermissionDenied => RegError::AccessDenied,
        _ => RegError::Other(e.to_string()),
    }
}

fn open(root: RegRoot, view: RegView, path: &str) -> Result<RegKey, RegError> {
    let hk = RegKey::predef(match root {
        RegRoot::Hklm => HKEY_LOCAL_MACHINE,
        RegRoot::Hkcu => HKEY_CURRENT_USER,
    });
    let flag = match view {
        RegView::Reg64 => KEY_WOW64_64KEY,
        RegView::Reg32 => KEY_WOW64_32KEY,
        RegView::Shared => 0,
    };
    hk.open_subkey_with_flags(path, KEY_READ | flag).map_err(map_err)
}

fn utf16(bytes: &[u8]) -> Vec<u16> {
    bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
}

fn to_string(bytes: &[u8]) -> String {
    let w = utf16(bytes);
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..end])
}

/// Convert raw registry data, preserving the value type.
pub fn convert(ty: u32, bytes: &[u8]) -> RegValue {
    match ty {
        1 => RegValue::Sz(to_string(bytes)),
        2 => RegValue::ExpandSz(to_string(bytes)),
        7 => RegValue::MultiSz(
            String::from_utf16_lossy(&utf16(bytes)).split('\0').filter(|s| !s.is_empty()).map(str::to_string).collect(),
        ),
        4 if bytes.len() >= 4 => RegValue::Dword(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
        11 if bytes.len() >= 8 => RegValue::Qword(u64::from_le_bytes(bytes[..8].try_into().unwrap())),
        3 => RegValue::Binary(bytes.to_vec()),
        other => RegValue::Other { ty: other, raw: bytes.to_vec() },
    }
}

impl RegRead for WinReg {
    fn subkeys(&self, root: RegRoot, view: RegView, path: &str) -> Result<Vec<String>, RegError> {
        let k = open(root, view, path)?;
        k.enum_keys().collect::<Result<Vec<_>, _>>().map_err(map_err)
    }

    fn values(&self, root: RegRoot, view: RegView, path: &str) -> Result<BTreeMap<String, RegValue>, RegError> {
        let k = open(root, view, path)?;
        let mut out = BTreeMap::new();
        for v in k.enum_values() {
            let (name, val) = v.map_err(map_err)?;
            out.insert(name, convert(val.vtype as u32, &val.bytes));
        }
        Ok(out)
    }
}
