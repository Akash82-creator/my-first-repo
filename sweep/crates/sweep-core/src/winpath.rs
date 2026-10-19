//! Windows path model.
//!
//! Only local, drive-absolute paths are representable (`C:\a\b`). Two parsers
//! exist because the inputs have different trust and syntax:
//!
//! * [`WinPath::parse_external`] — strings from the registry, uninstall
//!   commands or config. Strict: UNC, device paths, `\\?\` prefixes,
//!   alternate data streams, drive-relative (`C:foo`), rooted-relative
//!   (`\foo`), relative and `..` paths are rejected.
//! * [`WinPath::from_api`] — paths returned by Windows APIs such as
//!   `GetFinalPathNameByHandle` or `SHGetKnownFolderPath`. Accepts the `\\?\`
//!   local prefix, rejects `\\?\UNC\` (network, unsupported in Stage A).
//!
//! Comparison is ordinal and case-insensitive using a *simple* (one char to
//! one char) uppercase fold, approximating `CompareStringOrdinal(..., TRUE)`.
//! Characters whose uppercase form expands to several chars (e.g. `ß`) are
//! left unchanged, which matches Windows' simple-case behaviour.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum PathError {
    #[error("empty path")]
    Empty,
    #[error("UNC, network or device paths are not supported")]
    UncOrDevice,
    #[error("drive-relative path (e.g. C:foo)")]
    DriveRelative,
    #[error("rooted path without a drive (e.g. \\foo)")]
    RootedRelative,
    #[error("relative path")]
    Relative,
    #[error("path contains '..'")]
    DotDot,
    #[error("alternate data stream or stray ':' in component")]
    AlternateDataStream,
    #[error("invalid character in component")]
    InvalidChar,
    #[error("reserved device name component")]
    ReservedName,
    #[error("component is empty after trimming trailing dots/spaces")]
    InvalidComponent,
}

/// A normalized, local, drive-absolute Windows path.
#[derive(Clone, Serialize, Deserialize)]
pub struct WinPath {
    drive: char, // uppercase ASCII letter
    components: Vec<String>,
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CONIN$", "CONOUT$",
];

/// Simple uppercase fold of a single char (one-to-one only).
pub fn fold_char(c: char) -> char {
    let mut up = c.to_uppercase();
    match (up.next(), up.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}

/// Ordinal case-insensitive key for a string.
pub fn fold(s: &str) -> String {
    s.chars().map(fold_char).collect()
}

pub fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.chars().count() == b.chars().count() && a.chars().zip(b.chars()).all(|(x, y)| fold_char(x) == fold_char(y))
}

fn validate_component(raw: &str) -> Result<Option<String>, PathError> {
    if raw.is_empty() || raw == "." {
        return Ok(None);
    }
    if raw == ".." {
        return Err(PathError::DotDot);
    }
    if raw.contains(':') {
        return Err(PathError::AlternateDataStream);
    }
    if raw.chars().any(|c| (c as u32) < 0x20 || matches!(c, '<' | '>' | '"' | '|' | '?' | '*')) {
        return Err(PathError::InvalidChar);
    }
    // Win32 strips trailing dots and spaces from each component.
    let trimmed = raw.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return Err(PathError::InvalidComponent);
    }
    let stem = trimmed.split('.').next().unwrap_or(trimmed).trim_end();
    if RESERVED.iter().any(|r| eq_ignore_case(r, stem)) {
        return Err(PathError::ReservedName);
    }
    Ok(Some(trimmed.to_string()))
}

impl WinPath {
    /// Strict parser for untrusted strings (registry values, commands).
    pub fn parse_external(input: &str) -> Result<WinPath, PathError> {
        let mut s = input.trim();
        // One enclosing pair of quotes is common in InstallLocation values.
        if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
            s = s[1..s.len() - 1].trim();
        }
        if s.is_empty() {
            return Err(PathError::Empty);
        }
        let s = s.replace('/', "\\");
        if s.starts_with("\\\\") {
            return Err(PathError::UncOrDevice);
        }
        Self::parse_drive_absolute(&s)
    }

    /// Parser for paths produced by Windows APIs.
    pub fn from_api(input: &str) -> Result<WinPath, PathError> {
        let s = input.replace('/', "\\");
        let rest = if let Some(r) = s.strip_prefix("\\\\?\\") {
            if r.len() >= 4 && eq_ignore_case(&r[..4], "UNC\\") {
                return Err(PathError::UncOrDevice);
            }
            r.to_string()
        } else if s.starts_with("\\\\") {
            return Err(PathError::UncOrDevice);
        } else {
            s
        };
        if rest.is_empty() {
            return Err(PathError::Empty);
        }
        Self::parse_drive_absolute(&rest)
    }

    fn parse_drive_absolute(s: &str) -> Result<WinPath, PathError> {
        let mut chars = s.chars();
        let first = chars.next().ok_or(PathError::Empty)?;
        let second = chars.next();
        let third = chars.next();
        if s.starts_with('\\') {
            return Err(PathError::RootedRelative);
        }
        match (first.is_ascii_alphabetic(), second, third) {
            (true, Some(':'), Some('\\')) => {}
            (true, Some(':'), None) => return Err(PathError::DriveRelative),
            (true, Some(':'), Some(_)) => return Err(PathError::DriveRelative),
            _ => return Err(PathError::Relative),
        }
        let mut components = Vec::new();
        for raw in s[3..].split('\\') {
            if let Some(c) = validate_component(raw)? {
                components.push(c);
            }
        }
        Ok(WinPath { drive: first.to_ascii_uppercase(), components })
    }

    pub fn drive(&self) -> char {
        self.drive
    }

    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// Number of components below the drive root (`C:\` is 0).
    pub fn depth(&self) -> usize {
        self.components.len()
    }

    pub fn is_root(&self) -> bool {
        self.components.is_empty()
    }

    pub fn file_name(&self) -> Option<&str> {
        self.components.last().map(String::as_str)
    }

    pub fn parent(&self) -> Option<WinPath> {
        if self.components.is_empty() {
            None
        } else {
            Some(WinPath { drive: self.drive, components: self.components[..self.components.len() - 1].to_vec() })
        }
    }

    /// Append one component (a single name from a directory listing).
    pub fn join(&self, name: &str) -> Result<WinPath, PathError> {
        if name.contains('\\') || name.contains('/') {
            return Err(PathError::InvalidChar);
        }
        match validate_component(name)? {
            Some(c) => {
                let mut components = self.components.clone();
                components.push(c);
                Ok(WinPath { drive: self.drive, components })
            }
            None => Err(PathError::InvalidComponent),
        }
    }

    /// Case-insensitive key suitable for maps and equality.
    pub fn key(&self) -> String {
        let mut k = String::new();
        k.push(self.drive);
        k.push(':');
        for c in &self.components {
            k.push('\\');
            k.push_str(&fold(c));
        }
        k
    }

    /// `self` equals `other` or lies beneath it.
    pub fn is_within(&self, other: &WinPath) -> bool {
        self.drive == other.drive
            && self.components.len() >= other.components.len()
            && self.components.iter().zip(&other.components).all(|(a, b)| eq_ignore_case(a, b))
    }

    /// `self` lies strictly beneath `other`.
    pub fn is_strictly_within(&self, other: &WinPath) -> bool {
        self.components.len() > other.components.len() && self.is_within(other)
    }

    pub fn extension_is(&self, ext: &str) -> bool {
        self.file_name().and_then(|n| n.rsplit_once('.')).map(|(_, e)| eq_ignore_case(e, ext)).unwrap_or(false)
    }
}

impl PartialEq for WinPath {
    fn eq(&self, other: &Self) -> bool {
        self.drive == other.drive && self.components.len() == other.components.len() && self.is_within(other)
    }
}
impl Eq for WinPath {}

impl std::hash::Hash for WinPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

impl fmt::Display for WinPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:\\{}", self.drive, self.components.join("\\"))
    }
}

impl fmt::Debug for WinPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WinPath({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> WinPath {
        WinPath::parse_external(s).unwrap()
    }

    #[test]
    fn drive_absolute_variants() {
        assert_eq!(p("c:\\Program Files\\X\\").to_string(), "C:\\Program Files\\X");
        assert_eq!(p("C:/a//b/./c"), p("C:\\A\\B\\C"));
        assert_eq!(p("\"C:\\Program Files\\Acme\""), p("C:\\program files\\acme"));
        assert!(p("C:\\").is_root());
    }

    #[test]
    fn rejects_unsafe_external_syntax() {
        use PathError::*;
        let cases = [
            ("", Empty),
            ("\\\\server\\share\\x", UncOrDevice),
            ("\\\\?\\C:\\x", UncOrDevice),
            ("\\\\.\\PhysicalDrive0", UncOrDevice),
            ("C:foo", DriveRelative),
            ("C:", DriveRelative),
            ("\\Windows", RootedRelative),
            ("Program Files\\x", Relative),
            ("C:\\a\\..\\Windows", DotDot),
            ("C:\\a\\file.txt:stream", AlternateDataStream),
            ("C:\\a\\b?c", InvalidChar),
            ("C:\\a\\NUL", ReservedName),
            ("C:\\a\\com1.txt", ReservedName),
            ("C:\\a\\...", InvalidComponent),
        ];
        for (input, err) in cases {
            assert_eq!(WinPath::parse_external(input), Err(err), "input {input:?}");
        }
    }

    #[test]
    fn api_paths_accept_local_extended_prefix_only() {
        assert_eq!(WinPath::from_api("\\\\?\\C:\\Windows").unwrap(), p("C:\\windows"));
        assert_eq!(WinPath::from_api("\\\\?\\UNC\\srv\\share"), Err(PathError::UncOrDevice));
        assert_eq!(WinPath::from_api("\\\\srv\\share"), Err(PathError::UncOrDevice));
    }

    #[test]
    fn trailing_dots_and_spaces_are_stripped_like_win32() {
        assert_eq!(p("C:\\Acme. \\App"), p("C:\\Acme\\App"));
    }

    #[test]
    fn case_insensitive_ancestry() {
        let root = p("C:\\Program Files");
        let app = p("c:\\PROGRAM FILES\\Acme\\App");
        assert!(app.is_within(&root));
        assert!(app.is_strictly_within(&root));
        assert!(!root.is_strictly_within(&root));
        assert!(root.is_within(&root));
        // Prefix of a component is not ancestry.
        assert!(!p("C:\\Program Files Extra\\x").is_within(&root));
        assert!(!p("D:\\Program Files\\x").is_within(&root));
    }

    #[test]
    fn non_ascii_simple_fold() {
        assert_eq!(p("C:\\ÄPFEL"), p("C:\\äpfel"));
        // ß has no single-char uppercase; it only equals itself.
        assert_ne!(p("C:\\straße"), p("C:\\STRASSE"));
    }

    #[test]
    fn join_rejects_separators_and_ads() {
        let base = p("C:\\a");
        assert!(base.join("b\\c").is_err());
        assert!(base.join("b:c").is_err());
        assert_eq!(base.join("B").unwrap(), p("C:\\a\\b"));
    }

    proptest::proptest! {
        #[test]
        fn parse_never_panics_and_roundtrips(s in "\\PC{0,40}") {
            if let Ok(w) = WinPath::parse_external(&s) {
                let again = WinPath::parse_external(&w.to_string()).unwrap();
                proptest::prop_assert_eq!(&again, &w);
                proptest::prop_assert_eq!(again.key(), w.key());
            }
        }
    }
}
