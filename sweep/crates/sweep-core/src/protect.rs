//! Protection rules.
//!
//! * **Protected containers** (volume roots, Program Files, ProgramData,
//!   Users, profile roots, Known Folder roots…) can never be a target, but
//!   their descendants are evaluated normally.
//! * **Protected subtrees** (Windows, Sweep's own dirs, other installed
//!   programs' validated InstallLocations) block the root and every
//!   descendant.
//! * Any target that *contains* a protected container or subtree is blocked.

use crate::api::KnownFolders;
use crate::winpath::WinPath;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "label")]
pub enum Protection {
    None,
    /// Target is itself a protected container.
    Container(String),
    /// Target is inside (or equal to) a protected subtree.
    InSubtree(String),
    /// Target contains a protected container or subtree.
    ContainsProtected(String),
}

impl Protection {
    pub fn is_blocked(&self) -> bool {
        !matches!(self, Protection::None)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProtectionSet {
    pub containers: Vec<(WinPath, String)>,
    pub subtrees: Vec<(WinPath, String)>,
}

fn drive_root(p: &WinPath) -> WinPath {
    let mut r = p.clone();
    while let Some(parent) = r.parent() {
        r = parent;
    }
    r
}

impl ProtectionSet {
    /// Build from Known Folders. `other_program_dirs` are validated
    /// InstallLocations of *other* installed programs.
    pub fn from_known_folders(kf: &KnownFolders, other_program_dirs: &[(WinPath, String)]) -> Self {
        let mut s = ProtectionSet::default();
        let containers: [(&Option<WinPath>, &str); 23] = [
            (&kf.program_files, "Program Files"),
            (&kf.program_files_x86, "Program Files (x86)"),
            (&kf.program_files_common, "Common Files"),
            (&kf.program_files_common_x86, "Common Files (x86)"),
            (&kf.program_data, "ProgramData"),
            (&kf.users, "Users"),
            (&kf.user_profile, "user profile"),
            (&kf.public, "Public profile"),
            (&kf.roaming_app_data, "AppData\\Roaming"),
            (&kf.local_app_data, "AppData\\Local"),
            (&kf.local_programs, "AppData\\Local\\Programs"),
            (&kf.start_menu, "Start Menu"),
            (&kf.common_start_menu, "Common Start Menu"),
            (&kf.start_menu_programs, "Start Menu\\Programs"),
            (&kf.common_start_menu_programs, "Common Start Menu\\Programs"),
            (&kf.desktop, "Desktop"),
            (&kf.common_desktop, "Public Desktop"),
            (&kf.documents, "Documents"),
            (&kf.pictures, "Pictures"),
            (&kf.music, "Music"),
            (&kf.videos, "Videos"),
            (&kf.downloads, "Downloads"),
            (&kf.one_drive, "OneDrive"),
        ];
        for (p, label) in containers {
            if let Some(p) = p {
                s.containers.push((p.clone(), label.to_string()));
                let root = drive_root(p);
                if !s.containers.iter().any(|(c, _)| *c == root) {
                    s.containers.push((root, "volume root".into()));
                }
            }
        }
        if let Some(t) = &kf.temp {
            s.containers.push((t.clone(), "Temp".into()));
        }
        if let Some(w) = &kf.windows {
            s.subtrees.push((w.clone(), "Windows".into()));
            if let Ok(t) = w.join("Temp") {
                // Windows\Temp is already inside the Windows subtree; listed as a
                // container too so the label is precise if rules change.
                s.containers.push((t, "Windows\\Temp".into()));
            }
            let root = drive_root(w);
            if !s.containers.iter().any(|(c, _)| *c == root) {
                s.containers.push((root, "volume root".into()));
            }
        }
        for (p, label) in [(&kf.system32, "System32"), (&kf.syswow64, "SysWOW64")] {
            if let Some(p) = p {
                s.subtrees.push((p.clone(), label.into()));
            }
        }
        for d in &kf.sweep_dirs {
            s.subtrees.push((d.clone(), "Sweep".into()));
        }
        for (d, name) in other_program_dirs {
            s.subtrees.push((d.clone(), format!("installed program: {name}")));
        }
        s
    }

    pub fn classify(&self, target: &WinPath) -> Protection {
        if target.is_root() {
            return Protection::Container("volume root".into());
        }
        for (p, label) in &self.subtrees {
            if target.is_within(p) {
                return Protection::InSubtree(label.clone());
            }
        }
        for (p, label) in &self.containers {
            if target == p {
                return Protection::Container(label.clone());
            }
        }
        for (p, label) in self.subtrees.iter().chain(&self.containers) {
            if p.is_strictly_within(target) {
                return Protection::ContainsProtected(label.clone());
            }
        }
        Protection::None
    }

    pub fn is_container(&self, p: &WinPath) -> bool {
        p.is_root() || self.containers.iter().any(|(c, _)| c == p)
    }

    /// Strictly below some protected container.
    pub fn below_container(&self, p: &WinPath) -> bool {
        self.containers.iter().any(|(c, _)| p.is_strictly_within(c))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", content = "detail")]
pub enum LocationRejection {
    Missing,
    BadSyntax(String),
    IsContainer,
    NotBelowContainer,
    Protected(String),
    SharedWithOtherProgram(String),
}

/// Basic validity of an InstallLocation-like directory, ignoring other programs.
pub fn validate_dir_basic(raw: Option<&str>, set: &ProtectionSet) -> Result<WinPath, LocationRejection> {
    let raw = raw.ok_or(LocationRejection::Missing)?;
    let p = WinPath::parse_external(raw).map_err(|e| LocationRejection::BadSyntax(e.to_string()))?;
    if set.is_container(&p) {
        return Err(LocationRejection::IsContainer);
    }
    if !set.below_container(&p) {
        return Err(LocationRejection::NotBelowContainer);
    }
    match set.classify(&p) {
        Protection::None => Ok(p),
        other => Err(LocationRejection::Protected(format!("{other:?}"))),
    }
}

/// Full validation including overlap with other installed programs.
/// `set` must NOT already contain this program's own directory as a subtree.
pub fn validate_install_location(
    raw: Option<&str>,
    set: &ProtectionSet,
    others: &[(WinPath, String)],
) -> Result<WinPath, LocationRejection> {
    let p = validate_dir_basic(raw, set)?;
    for (o, name) in others {
        if p.is_within(o) || o.is_within(&p) {
            return Err(LocationRejection::SharedWithOtherProgram(name.clone()));
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::known_folders;

    fn p(s: &str) -> WinPath {
        WinPath::parse_external(s).unwrap()
    }

    #[test]
    fn containers_vs_subtrees() {
        let other = vec![(p("C:\\Program Files\\Other"), "Other".to_string())];
        let set = ProtectionSet::from_known_folders(&known_folders(), &other);
        assert!(matches!(set.classify(&p("C:\\Program Files")), Protection::Container(_)));
        assert_eq!(
            set.classify(&p("C:\\Program Files\\Acme\\App")),
            Protection::None,
            "descendant of container is evaluated"
        );
        assert!(matches!(set.classify(&p("C:\\Windows\\System32\\drivers")), Protection::InSubtree(_)));
        assert!(matches!(set.classify(&p("C:\\Program Files\\Other\\bin")), Protection::InSubtree(_)));
        assert!(matches!(set.classify(&p("C:\\Users\\alice")), Protection::Container(_)));
        assert!(matches!(set.classify(&p("C:\\")), Protection::Container(_)));
        assert!(matches!(set.classify(&p("D:\\")), Protection::Container(_)), "any volume root");
        // A folder that contains a protected container is blocked.
        assert!(matches!(set.classify(&p("C:\\Users\\alice\\AppData")), Protection::ContainsProtected(_)));
        // Case-insensitive.
        assert!(matches!(set.classify(&p("c:\\WINDOWS\\x")), Protection::InSubtree(_)));
    }

    #[test]
    fn install_location_validation() {
        let set = ProtectionSet::from_known_folders(&known_folders(), &[]);
        let others = vec![(p("C:\\Program Files\\Acme\\B"), "Acme B".to_string())];
        assert_eq!(
            validate_install_location(Some("C:\\Program Files\\Acme\\A\\"), &set, &others),
            Ok(p("C:\\Program Files\\Acme\\A"))
        );
        assert_eq!(
            validate_install_location(Some("C:\\Program Files"), &set, &others),
            Err(LocationRejection::IsContainer)
        );
        // Directly below a volume root is allowed (e.g. C:\\Python311).
        assert_eq!(validate_install_location(Some("C:\\Tools\\x"), &set, &others), Ok(p("C:\\Tools\\x")));
        assert!(matches!(
            validate_install_location(Some("C:\\Program Files\\Acme"), &set, &others),
            Err(LocationRejection::SharedWithOtherProgram(_))
        ));
        assert!(matches!(
            validate_install_location(Some("C:\\Windows\\App"), &set, &others),
            Err(LocationRejection::Protected(_))
        ));
        assert!(matches!(
            validate_install_location(Some("\\\\srv\\share\\x"), &set, &others),
            Err(LocationRejection::BadSyntax(_))
        ));
        assert_eq!(validate_install_location(None, &set, &others), Err(LocationRejection::Missing));
    }
}
