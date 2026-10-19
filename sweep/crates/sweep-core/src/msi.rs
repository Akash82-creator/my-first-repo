//! Windows Installer helpers: product codes and exit-code semantics.

use serde::{Deserialize, Serialize};

/// Normalize a product code to `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`
/// (uppercase). Accepts the braces-less form too.
pub fn parse_product_code(s: &str) -> Option<String> {
    let t = s.trim();
    let inner = t.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or(t);
    let groups: Vec<&str> = inner.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    if groups.len() != 5 {
        return None;
    }
    for (g, l) in groups.iter().zip(lens) {
        if g.len() != l || !g.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
    }
    Some(format!("{{{}}}", inner.to_ascii_uppercase()))
}

/// Find the first `{GUID}` in a string.
pub fn find_product_code(s: &str) -> Option<String> {
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let after = &rest[start..];
        let end = after.find('}')?;
        if let Some(code) = parse_product_code(&after[..=end]) {
            return Some(code);
        }
        rest = &after[1..];
    }
    None
}

/// Result of `MsiQueryProductStateW` after an uninstall attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MsiProductState {
    /// INSTALLSTATE_UNKNOWN: not installed for this context.
    NotInstalled,
    /// INSTALLSTATE_DEFAULT / LOCAL / SOURCE etc.
    Installed,
    /// INSTALLSTATE_ADVERTISED.
    Advertised,
    /// Any other / error value.
    Other(i32),
}

impl MsiProductState {
    pub fn from_raw(v: i32) -> Self {
        match v {
            -1 => MsiProductState::NotInstalled,
            1 => MsiProductState::Advertised,
            3..=5 => MsiProductState::Installed,
            other => MsiProductState::Other(other),
        }
    }
}

// Documented msiexec / Windows Installer exit codes.
pub const ERROR_SUCCESS: u32 = 0;
pub const ERROR_INSTALL_USEREXIT: u32 = 1602;
pub const ERROR_UNKNOWN_PRODUCT: u32 = 1605;
pub const ERROR_INSTALL_ALREADY_RUNNING: u32 = 1618;
pub const ERROR_SUCCESS_REBOOT_INITIATED: u32 = 1641;
pub const ERROR_SUCCESS_REBOOT_REQUIRED: u32 = 3010;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_codes() {
        let c = "{23170F69-40C1-2702-2301-000001000000}";
        assert_eq!(parse_product_code(c).as_deref(), Some(c));
        assert_eq!(parse_product_code("23170f69-40c1-2702-2301-000001000000").as_deref(), Some(c));
        assert_eq!(parse_product_code("{23170F69-40C1-2702-2301}"), None);
        assert_eq!(find_product_code("MsiExec.exe /X{23170f69-40c1-2702-2301-000001000000}").as_deref(), Some(c));
        assert_eq!(find_product_code("x {bad} y {23170F69-40C1-2702-2301-000001000000}").as_deref(), Some(c));
        assert_eq!(find_product_code("nothing"), None);
    }
}
