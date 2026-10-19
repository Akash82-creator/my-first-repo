//! Pre-uninstall evidence snapshot, name matching and candidate verdicts.
//!
//! Names only *discover* candidates. A candidate becomes `Likely` only with
//! strong ownership evidence and no shared-use, user-data or coverage
//! concerns; otherwise it is `Review`, and protected targets are `Blocked`.

use crate::api::{FsRead, KnownFolders};
use crate::cmdline::{parse_uninstall_command, ParseContext, UninstallCommand, Unsupported};
use crate::inventory::Inventory;
use crate::model::{InstalledProgram, RegKeyRef};
use crate::protect::{validate_dir_basic, validate_install_location, LocationRejection, Protection, ProtectionSet};
use crate::winpath::{fold, WinPath};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Name matching
// ---------------------------------------------------------------------------

const STOP_WORDS: &[&str] = &[
    "inc",
    "ltd",
    "llc",
    "gmbh",
    "corp",
    "corporation",
    "company",
    "co",
    "the",
    "software",
    "technologies",
    "technology",
    "limited",
    "edition",
    "setup",
    "installer",
    "x64",
    "x86",
    "64bit",
    "32bit",
    "bit",
    "and",
    "for",
    "version",
    "update",
    "srl",
    "s.a",
    "ag",
    "plc",
    "pty",
    "foundation",
    "project",
];

fn is_version_like(t: &str) -> bool {
    let t = t.trim_start_matches('v');
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Lowercase alphanumeric tokens with stop words and version numbers removed.
pub fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric() && c != '.')
        .flat_map(|t| t.split('.').collect::<Vec<_>>().into_iter().map(str::to_string).collect::<Vec<_>>())
        .map(|t| t.to_lowercase())
        .filter(|t| !t.is_empty() && !is_version_like(t) && !STOP_WORDS.contains(&t.as_str()))
        .collect()
}

/// Compact product name with parenthesised parts and dotted version
/// numbers removed: "7-Zip 23.01 (x64 edition)" → "7zip".
pub fn core_compact(s: &str) -> String {
    let mut depth = 0usize;
    let mut no_parens = String::new();
    for c in s.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => no_parens.push(c),
            _ => {}
        }
    }
    no_parens
        .split_whitespace()
        .filter(|w| {
            let v = w.trim_start_matches(['v', 'V']);
            !(v.contains('.')
                && v.chars().any(|c| c.is_ascii_digit())
                && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
        })
        .map(compact)
        .collect::<Vec<_>>()
        .concat()
}

/// Lowercase alphanumeric-only form.
pub fn compact(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NameMatch {
    /// Name equals the product name (compact form).
    Product,
    /// All (≥2) significant product tokens are present.
    ProductTokens,
    /// Name equals the product name minus publisher words; only valid
    /// inside a publisher folder/key.
    ProductUnderPublisher,
    /// Name equals the publisher.
    Publisher,
}

impl NameMatch {
    pub fn describe(self) -> &'static str {
        match self {
            NameMatch::Product => "name equals the product name",
            NameMatch::ProductTokens => "name contains all product name words",
            NameMatch::ProductUnderPublisher => "product name inside the publisher's folder/key",
            NameMatch::Publisher => "name equals the publisher",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameMatcher {
    pub product_forms: Vec<String>,
    pub product_tokens: Vec<String>,
    pub product_sans_publisher: Option<String>,
    pub publisher_forms: Vec<String>,
}

const MIN_COMPACT: usize = 3;

impl NameMatcher {
    pub fn new(display_name: &str, publisher: Option<&str>) -> Self {
        let pt = tokens(display_name);
        let pubt = publisher.map(tokens).unwrap_or_default();
        let mut product_forms = vec![core_compact(display_name), compact(display_name)];
        product_forms.retain(|f| f.chars().count() >= MIN_COMPACT);
        product_forms.dedup();
        let sans: String = pt.iter().filter(|t| !pubt.contains(t)).cloned().collect::<Vec<_>>().concat();
        let mut publisher_forms = publisher.map(|p| vec![core_compact(p), pubt.concat()]).unwrap_or_default();
        publisher_forms.retain(|f| f.chars().count() >= MIN_COMPACT);
        publisher_forms.dedup();
        // A publisher name identical to the product name is treated as product only.
        publisher_forms.retain(|f| !product_forms.contains(f));
        let product_tokens: Vec<String> = pt.into_iter().filter(|t| t.chars().count() >= MIN_COMPACT).collect();
        NameMatcher {
            product_forms,
            product_tokens,
            product_sans_publisher: Some(sans).filter(|s| s.chars().count() >= MIN_COMPACT),
            publisher_forms,
        }
    }

    fn stem(name: &str) -> &str {
        for ext in [".lnk", ".url"] {
            if name.len() > ext.len()
                && name.is_char_boundary(name.len() - ext.len())
                && crate::winpath::eq_ignore_case(&name[name.len() - ext.len()..], ext)
            {
                return &name[..name.len() - ext.len()];
            }
        }
        name
    }

    /// Match at a top-level location (not inside a publisher folder).
    pub fn match_top(&self, name: &str) -> Option<NameMatch> {
        let stem = Self::stem(name);
        let c = compact(stem);
        let t = tokens(stem);
        if self.product_forms.contains(&c) || self.product_forms.contains(&core_compact(stem)) {
            return Some(NameMatch::Product);
        }
        if self.publisher_forms.contains(&c) || (!t.is_empty() && self.publisher_forms.contains(&t.concat())) {
            return Some(NameMatch::Publisher);
        }
        if self.product_tokens.len() >= 2 && self.product_tokens.iter().all(|pt| t.contains(pt)) {
            return Some(NameMatch::ProductTokens);
        }
        None
    }

    /// Match a child of a publisher folder/key.
    pub fn match_under_publisher(&self, name: &str) -> Option<NameMatch> {
        match self.match_top(name) {
            Some(NameMatch::Publisher) => None,
            Some(m) => Some(m),
            None => {
                let stem = Self::stem(name);
                let c = compact(stem);
                let t = tokens(stem).concat();
                match &self.product_sans_publisher {
                    Some(s) if *s == c || *s == t => Some(NameMatch::ProductUnderPublisher),
                    _ => None,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// Evidence recorded before the vendor uninstaller runs. The leftover scan
/// uses this, because the uninstaller may remove the registry entry.
///
/// Stage A: taken by the unelevated app. Stage B will have the elevated
/// helper take its own snapshot instead of trusting this one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSnapshot {
    pub program_id: String,
    pub display_name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_day: Option<i64>,
    pub product_code: Option<String>,
    pub windows_installer: bool,
    pub uninstall_keys: Vec<RegKeyRef>,
    pub install_dir: Result<WinPath, LocationRejection>,
    pub icon_dir: Option<WinPath>,
    pub uninstaller_dir: Option<WinPath>,
    pub command: Result<UninstallCommand, Unsupported>,
    pub matcher: NameMatcher,
    /// Validated directories of every *other* installed program.
    pub other_program_dirs: Vec<(WinPath, String)>,
    /// How many other installed programs share this publisher.
    pub publisher_shared_with: usize,
    pub protection: ProtectionSet,
}

/// Strip a DisplayIcon value like `"C:\x\app.exe",0` to the path part.
pub fn icon_path(raw: &str) -> &str {
    let t = raw.trim();
    let t = match t.rfind(',') {
        Some(i) if t[i + 1..].trim().trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) => &t[..i],
        _ => t,
    };
    t.trim().trim_matches('"')
}

pub struct SnapshotEnv<'a> {
    pub known: &'a KnownFolders,
    pub fs: &'a dyn FsRead,
    pub env: &'a dyn Fn(&str) -> Option<String>,
}

pub fn take_snapshot(program: &InstalledProgram, inventory: &Inventory, senv: &SnapshotEnv) -> EvidenceSnapshot {
    let base = ProtectionSet::from_known_folders(senv.known, &[]);
    let mut others = Vec::new();
    for q in inventory.programs.iter().filter(|q| q.id != program.id) {
        if let Ok(d) = validate_dir_basic(q.install_location.as_deref(), &base) {
            others.push((d, q.display_name.clone()));
        }
    }
    let install_dir = validate_install_location(program.install_location.as_deref(), &base, &others);
    let icon_dir = program
        .display_icon
        .as_deref()
        .map(icon_path)
        .and_then(|p| WinPath::parse_external(p).ok())
        .and_then(|p| p.parent())
        .and_then(|d| validate_install_location(Some(&d.to_string()), &base, &others).ok());

    let mut anchors = Vec::new();
    if let Ok(d) = &install_dir {
        anchors.push(d.clone());
    }
    if let Some(d) = &icon_dir {
        anchors.push(d.clone());
    }
    let exists = |p: &WinPath| senv.fs.is_file(p);
    let ctx = ParseContext {
        env: senv.env,
        file_exists: &exists,
        anchors: &anchors,
        system_root: senv.known.windows.as_ref(),
    };
    let command = match program.uninstall_string.as_deref() {
        Some(raw) => parse_uninstall_command(raw, program.windows_installer, &program.entry.key_name, &ctx),
        None if program.windows_installer && program.product_code.is_some() => {
            Ok(UninstallCommand::Msi { product_code: program.product_code.clone().unwrap() })
        }
        None => Err(Unsupported::Empty),
    };
    let uninstaller_dir = match &command {
        Ok(UninstallCommand::Exe { exe, .. }) => {
            exe.parent().and_then(|d| validate_install_location(Some(&d.to_string()), &base, &others).ok())
        }
        _ => None,
    };
    let pub_key = program.publisher.as_deref().map(compact).filter(|s| !s.is_empty());
    let publisher_shared_with = match &pub_key {
        Some(k) => inventory
            .programs
            .iter()
            .filter(|q| q.id != program.id && q.publisher.as_deref().map(compact).as_ref() == Some(k))
            .count(),
        None => 0,
    };
    let protection = ProtectionSet::from_known_folders(senv.known, &others);
    let uninstall_keys = program
        .sources
        .iter()
        .map(|s| {
            crate::model::UninstallEntry {
                scope: s.scope,
                view: s.view,
                key_name: s.key_name.clone(),
                values: Default::default(),
            }
            .key_ref()
        })
        .collect();
    EvidenceSnapshot {
        program_id: program.id.clone(),
        display_name: program.display_name.clone(),
        publisher: program.publisher.clone(),
        version: program.version.clone(),
        install_day: program.install_day(),
        product_code: program.product_code.clone(),
        windows_installer: program.windows_installer,
        uninstall_keys,
        install_dir,
        icon_dir,
        uninstaller_dir,
        command,
        matcher: NameMatcher::new(&program.display_name, program.publisher.as_deref()),
        other_program_dirs: others,
        publisher_shared_with,
        protection,
    }
}

// ---------------------------------------------------------------------------
// Candidates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Target {
    Path { path: WinPath, is_dir: bool },
    RegKey { key: RegKeyRef },
}

impl Target {
    pub fn key(&self) -> String {
        match self {
            Target::Path { path, .. } => format!("f:{}", path.key()),
            Target::RegKey { key } => format!("r:{:?}:{:?}:{}", key.root, key.view, fold(&key.path)),
        }
    }
    pub fn display(&self) -> String {
        match self {
            Target::Path { path, .. } => path.to_string(),
            Target::RegKey { key } => key.to_string(),
        }
    }
}

/// Strong ownership evidence. Any one of these may support `Likely`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Ownership {
    IsInstallDir,
    InsideInstallDir,
    UninstallerDir,
    IconDir,
    UninstallEntry,
    AppPathPointsInsideInstallDir(String),
}

impl Ownership {
    pub fn describe(&self) -> String {
        match self {
            Ownership::IsInstallDir => "is the program's validated InstallLocation".into(),
            Ownership::InsideInstallDir => "inside the program's validated InstallLocation".into(),
            Ownership::UninstallerDir => "folder of the program's own uninstaller".into(),
            Ownership::IconDir => "folder of the program's registered icon".into(),
            Ownership::UninstallEntry => "the program's own Uninstall registry entry".into(),
            Ownership::AppPathPointsInsideInstallDir(p) => {
                format!("App Paths entry points inside install folder ({p})")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Verdict {
    /// Strong ownership, no concerns. Would be preselected in Stage B.
    Likely,
    /// Needs human judgement; never preselected.
    Review,
    /// Protected; cannot be selected.
    Blocked,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeStats {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub modified_after_install: u64,
    pub document_like: u64,
    pub reparse_points: Vec<String>,
    pub unreadable: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub target: Target,
    pub discovered_by: Vec<String>,
    pub ownership: Vec<Ownership>,
    pub shared: Vec<String>,
    pub user_data: Vec<String>,
    /// Coverage concerns inside this candidate (reparse points, unreadable…).
    pub concerns: Vec<String>,
    pub protection: Protection,
    pub stats: Option<TreeStats>,
    pub verdict: Verdict,
}

impl Candidate {
    pub fn new(target: Target) -> Self {
        Candidate {
            id: String::new(),
            target,
            discovered_by: vec![],
            ownership: vec![],
            shared: vec![],
            user_data: vec![],
            concerns: vec![],
            protection: Protection::None,
            stats: None,
            verdict: Verdict::Review,
        }
    }

    pub fn merge(&mut self, other: Candidate) {
        for d in other.discovered_by {
            if !self.discovered_by.contains(&d) {
                self.discovered_by.push(d);
            }
        }
        for o in other.ownership {
            if !self.ownership.contains(&o) {
                self.ownership.push(o);
            }
        }
        for s in other.shared {
            if !self.shared.contains(&s) {
                self.shared.push(s);
            }
        }
    }
}

pub const DOCUMENT_EXTS: &[&str] = &[
    "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "pdf", "txt", "rtf", "md", "csv", "db", "sqlite",
    "sqlite3", "mdb", "accdb", "psd", "jpg", "jpeg", "png", "gif", "mp3", "mp4", "wav", "sav", "kdbx", "pst", "ost",
    "zip", "7z",
];

/// Apply ownership (path-based), protection, user-data and verdict rules.
pub fn evaluate(c: &mut Candidate, snap: &EvidenceSnapshot, known: &KnownFolders) {
    if let Target::Path { path, is_dir } = &c.target {
        if let Ok(dir) = &snap.install_dir {
            if path == dir {
                push_unique(&mut c.ownership, Ownership::IsInstallDir);
            } else if path.is_strictly_within(dir) {
                push_unique(&mut c.ownership, Ownership::InsideInstallDir);
            }
        }
        if let Some(d) = &snap.uninstaller_dir {
            if path == d {
                push_unique(&mut c.ownership, Ownership::UninstallerDir);
            }
        }
        if let Some(d) = &snap.icon_dir {
            if path == d {
                push_unique(&mut c.ownership, Ownership::IconDir);
            }
        }
        c.protection = snap.protection.classify(path);
        let is_shortcut = path.extension_is("lnk") || path.extension_is("url");
        if !is_shortcut {
            for r in known.user_data_roots() {
                if path.is_within(&r) {
                    c.user_data.push(format!("located in a user-data folder ({r})"));
                    break;
                }
            }
        }
        if let Some(pf) = [&known.program_files_common, &known.program_files_common_x86]
            .into_iter()
            .flatten()
            .find(|cf| path.is_strictly_within(cf))
        {
            c.shared.push(format!("inside Common Files ({pf}), which is shared by design"));
        }
        if let Some(st) = &c.stats {
            if st.modified_after_install > 0 {
                c.user_data.push(format!("{} file(s) modified after the install date", st.modified_after_install));
            }
            if st.document_like > 0 {
                c.user_data.push(format!("{} document/database/media file(s)", st.document_like));
            }
            if !st.reparse_points.is_empty() {
                c.concerns.push(format!("{} junction/symlink(s) inside; not followed", st.reparse_points.len()));
            }
            if !st.unreadable.is_empty() {
                c.concerns.push(format!("{} unreadable location(s) inside", st.unreadable.len()));
            }
            if st.truncated {
                c.concerns.push("tree too large or deep; not fully examined".into());
            }
        }
        let _ = is_dir;
    }
    c.verdict = verdict(c);
}

fn push_unique<T: PartialEq>(v: &mut Vec<T>, x: T) {
    if !v.contains(&x) {
        v.push(x);
    }
}

pub fn verdict(c: &Candidate) -> Verdict {
    if c.protection.is_blocked() {
        Verdict::Blocked
    } else if !c.ownership.is_empty() && c.shared.is_empty() && c.user_data.is_empty() && c.concerns.is_empty() {
        Verdict::Likely
    } else {
        Verdict::Review
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenization() {
        assert_eq!(tokens("Acme Notes 2.3.1 (x64)"), vec!["acme", "notes"]);
        assert_eq!(tokens("7-Zip 23.01 (x64 edition)"), vec!["zip"]);
        assert_eq!(core_compact("7-Zip 23.01 (x64 edition)"), "7zip");
        assert_eq!(core_compact("Python 3.11.4 (64-bit)"), "python");
        assert_eq!(core_compact("Office 2019"), "office2019");
        assert_eq!(compact("Notepad++"), "notepad");
        assert_eq!(compact("7-Zip"), "7zip");
    }

    #[test]
    fn matcher_rules() {
        let m = NameMatcher::new("Acme Notes 2.3 (x64)", Some("Acme Corporation"));
        assert_eq!(m.match_top("Acme Notes"), Some(NameMatch::Product));
        assert_eq!(m.match_top("AcmeNotes"), Some(NameMatch::Product));
        assert_eq!(m.match_top("Acme"), Some(NameMatch::Publisher));
        assert_eq!(m.match_top("Acme Corporation"), Some(NameMatch::Publisher));
        assert_eq!(m.match_top("Notes"), None, "a bare word is not the product at top level");
        assert_eq!(m.match_top("Acme Notes.lnk"), Some(NameMatch::Product));
        assert_eq!(m.match_top("Acme Notes Backup Tool"), Some(NameMatch::ProductTokens));
        assert_eq!(m.match_under_publisher("Notes"), Some(NameMatch::ProductUnderPublisher));
        assert_eq!(m.match_under_publisher("Paint"), None);
        // Very short names never match.
        let short = NameMatcher::new("Go", Some("Go"));
        assert_eq!(short.match_top("Go"), None);
    }

    #[test]
    fn icon_path_parsing() {
        assert_eq!(icon_path("\"C:\\a b\\x.exe\",0"), "C:\\a b\\x.exe");
        assert_eq!(icon_path("C:\\x\\y.ico"), "C:\\x\\y.ico");
        assert_eq!(icon_path("C:\\x\\y.exe,-101"), "C:\\x\\y.exe");
    }
}
