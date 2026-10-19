//! Data model shared by inventory, uninstall and scan.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Scope {
    /// HKLM: installed for all users.
    Machine,
    /// HKCU: installed for the current user only.
    User,
}

/// Registry view. HKLM has distinct 64-bit and 32-bit (WOW6432Node) views on
/// 64-bit Windows; HKCU\Software is shared, represented as `Shared`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RegView {
    Reg64,
    Reg32,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RegRoot {
    Hklm,
    Hkcu,
}

/// A registry value with its type preserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RegValue {
    Sz(String),
    ExpandSz(String),
    MultiSz(Vec<String>),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
    Other { ty: u32, raw: Vec<u8> },
}

impl RegValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RegValue::Sz(s) | RegValue::ExpandSz(s) => Some(s.as_str()),
            _ => None,
        }
    }
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            RegValue::Dword(v) => Some(*v),
            RegValue::Qword(v) => u32::try_from(*v).ok(),
            RegValue::Sz(s) => s.trim().parse().ok(),
            _ => None,
        }
    }
}

/// Fully qualified registry key reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RegKeyRef {
    pub root: RegRoot,
    pub view: RegView,
    pub path: String,
}

impl std::fmt::Display for RegKeyRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let root = match self.root {
            RegRoot::Hklm => "HKLM",
            RegRoot::Hkcu => "HKCU",
        };
        let view = match self.view {
            RegView::Reg64 => " (64-bit)",
            RegView::Reg32 => " (32-bit)",
            RegView::Shared => "",
        };
        write!(f, "{root}\\{}{view}", self.path)
    }
}

pub const UNINSTALL_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
pub const APP_PATHS_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths";

/// Raw contents of one `...\Uninstall\<key>` registry key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallEntry {
    pub scope: Scope,
    pub view: RegView,
    pub key_name: String,
    pub values: BTreeMap<String, RegValue>,
}

impl UninstallEntry {
    pub fn key_ref(&self) -> RegKeyRef {
        RegKeyRef {
            root: match self.scope {
                Scope::Machine => RegRoot::Hklm,
                Scope::User => RegRoot::Hkcu,
            },
            view: self.view,
            path: format!("{UNINSTALL_PATH}\\{}", self.key_name),
        }
    }

    /// String value lookup, case-insensitive on the value name.
    pub fn str(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| crate::winpath::eq_ignore_case(k, name))
            .and_then(|(_, v)| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    pub fn u32(&self, name: &str) -> Option<u32> {
        self.values.iter().find(|(k, _)| crate::winpath::eq_ignore_case(k, name)).and_then(|(_, v)| v.as_u32())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrySource {
    pub scope: Scope,
    pub view: RegView,
    pub key_name: String,
}

/// A program as shown in the inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledProgram {
    /// Opaque identifier; only meaningful to the backend session.
    pub id: String,
    pub display_name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub display_icon: Option<String>,
    /// `InstallDate` as found (normally `YYYYMMDD`).
    pub install_date: Option<String>,
    pub estimated_size_kb: Option<u32>,
    pub windows_installer: bool,
    pub product_code: Option<String>,
    pub scope: Scope,
    /// All registry locations that describe this program (after de-duplication).
    pub sources: Vec<EntrySource>,
    /// The primary entry the program was built from.
    pub entry: UninstallEntry,
}

impl InstalledProgram {
    /// Install date as days since 1970-01-01, if parseable.
    pub fn install_day(&self) -> Option<i64> {
        self.install_date.as_deref().and_then(parse_yyyymmdd)
    }
}

/// Parse `YYYYMMDD` to days since the Unix epoch.
pub fn parse_yyyymmdd(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: i64 = s[4..6].parse().ok()?;
    let d: i64 = s[6..8].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1980 {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// Howard Hinnant's days-from-civil algorithm.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_parsing() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(parse_yyyymmdd("19700101"), None, "implausible install year");
        assert_eq!(parse_yyyymmdd("20240229"), Some(19782));
        assert_eq!(parse_yyyymmdd("2024-02-29"), None);
        assert_eq!(parse_yyyymmdd("20241301"), None);
    }
}
