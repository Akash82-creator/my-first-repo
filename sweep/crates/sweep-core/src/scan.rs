//! Read-only leftover scanner with explicit coverage, progress and
//! cancellation. A location that could not be examined is reported as a
//! coverage gap; it is never presented as "nothing found".

use crate::api::{FsError, FsRead, KnownFolders, RegError, RegRead};
use crate::evidence::{evaluate, Candidate, EvidenceSnapshot, NameMatch, Ownership, Target, TreeStats, DOCUMENT_EXTS};
use crate::model::{RegKeyRef, RegRoot, RegView, APP_PATHS_PATH};
use crate::winpath::WinPath;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "detail")]
pub enum CoverageStatus {
    Scanned,
    /// The location does not exist (not a gap).
    NotPresent,
    AccessDenied,
    ReparsePointNotFollowed,
    LimitReached,
    Error(String),
    Cancelled,
}

impl CoverageStatus {
    pub fn is_gap(&self) -> bool {
        !matches!(self, CoverageStatus::Scanned | CoverageStatus::NotPresent)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageEntry {
    pub location: String,
    pub status: CoverageStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub phase: String,
    pub items_visited: u64,
    pub candidates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub candidates: Vec<Candidate>,
    pub coverage: Vec<CoverageEntry>,
    /// True only if every planned location was examined without gaps.
    pub complete: bool,
    pub cancelled: bool,
    pub items_visited: u64,
}

impl ScanReport {
    pub fn gaps(&self) -> impl Iterator<Item = &CoverageEntry> {
        self.coverage.iter().filter(|c| c.status.is_gap())
    }
}

#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_depth: usize,
    pub max_entries_per_tree: u64,
    pub progress_every_items: u64,
    pub progress_every: Duration,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            max_depth: 32,
            max_entries_per_tree: 200_000,
            progress_every_items: 100,
            progress_every: Duration::from_millis(200),
        }
    }
}

pub struct Scanner<'a> {
    pub snap: &'a EvidenceSnapshot,
    pub known: &'a KnownFolders,
    pub fs: &'a dyn FsRead,
    pub reg: &'a dyn RegRead,
    pub opts: ScanOptions,
    pub cancel: CancelToken,
}

struct Run<'a, 'b> {
    s: &'b Scanner<'a>,
    found: BTreeMap<String, Candidate>,
    coverage: Vec<CoverageEntry>,
    visited: u64,
    phase: String,
    last_emit: Instant,
    last_emit_items: u64,
    on_progress: &'b mut dyn FnMut(&Progress),
}

fn fs_status(e: &FsError) -> CoverageStatus {
    match e {
        FsError::NotFound => CoverageStatus::NotPresent,
        FsError::AccessDenied => CoverageStatus::AccessDenied,
        FsError::Other(m) => CoverageStatus::Error(m.clone()),
    }
}

fn reg_status(e: &RegError) -> CoverageStatus {
    match e {
        RegError::NotFound => CoverageStatus::NotPresent,
        RegError::AccessDenied => CoverageStatus::AccessDenied,
        RegError::Other(m) => CoverageStatus::Error(m.clone()),
    }
}

impl<'a, 'b> Run<'a, 'b> {
    fn cancelled(&self) -> bool {
        self.s.cancel.is_cancelled()
    }

    fn tick(&mut self, n: u64) {
        self.visited += n;
        let due_items = self.visited - self.last_emit_items >= self.s.opts.progress_every_items;
        if due_items || self.last_emit.elapsed() >= self.s.opts.progress_every {
            self.emit();
        }
    }

    fn emit(&mut self) {
        self.last_emit = Instant::now();
        self.last_emit_items = self.visited;
        let p = Progress { phase: self.phase.clone(), items_visited: self.visited, candidates: self.found.len() };
        (self.on_progress)(&p);
    }

    fn cover(&mut self, location: impl Into<String>, status: CoverageStatus) {
        self.coverage.push(CoverageEntry { location: location.into(), status });
    }

    fn add(&mut self, c: Candidate) {
        let k = c.target.key();
        match self.found.get_mut(&k) {
            Some(existing) => existing.merge(c),
            None => {
                self.found.insert(k, c);
            }
        }
    }

    /// Walk a directory tree without following reparse points.
    fn tree_stats(&mut self, root: &WinPath) -> TreeStats {
        let mut st = TreeStats::default();
        let after = self.s.snap.install_day.map(|d| (d + 1) * 86_400);
        let mut stack = vec![(root.clone(), 0usize)];
        let mut entries = 0u64;
        while let Some((dir, depth)) = stack.pop() {
            if self.cancelled() {
                st.truncated = true;
                break;
            }
            let list = match self.s.fs.read_dir(&dir) {
                Ok(l) => l,
                Err(FsError::NotFound) => continue,
                Err(e) => {
                    st.unreadable.push(dir.to_string());
                    self.cover(dir.to_string(), fs_status(&e));
                    continue;
                }
            };
            st.dirs += 1;
            for e in list {
                entries += 1;
                self.tick(1);
                let Ok(child) = dir.join(&e.name) else {
                    st.unreadable.push(format!("{dir}\\{}", e.name));
                    continue;
                };
                if e.meta.is_reparse {
                    st.reparse_points.push(child.to_string());
                    self.cover(child.to_string(), CoverageStatus::ReparsePointNotFollowed);
                    continue;
                }
                if e.meta.is_dir {
                    if depth + 1 >= self.s.opts.max_depth {
                        st.truncated = true;
                        self.cover(child.to_string(), CoverageStatus::LimitReached);
                    } else {
                        stack.push((child, depth + 1));
                    }
                } else {
                    st.files += 1;
                    st.bytes += e.meta.size;
                    if let (Some(a), Some(m)) = (after, e.meta.modified) {
                        if m > a {
                            st.modified_after_install += 1;
                        }
                    }
                    if DOCUMENT_EXTS.iter().any(|x| child.extension_is(x)) {
                        st.document_like += 1;
                    }
                }
                if entries >= self.s.opts.max_entries_per_tree {
                    st.truncated = true;
                    self.cover(root.to_string(), CoverageStatus::LimitReached);
                    return st;
                }
            }
        }
        st
    }

    fn path_candidate(&mut self, path: &WinPath, discovered: &str, ownership: Option<Ownership>) {
        let meta = match self.s.fs.metadata(path) {
            Ok(m) => m,
            Err(e) => {
                self.cover(path.to_string(), fs_status(&e));
                return;
            }
        };
        let mut c = Candidate::new(Target::Path { path: path.clone(), is_dir: meta.is_dir && !meta.is_reparse });
        c.discovered_by.push(discovered.to_string());
        if let Some(o) = ownership {
            c.ownership.push(o);
        }
        if meta.is_reparse {
            c.concerns.push("target itself is a junction/symlink; only the link would be considered".into());
            self.cover(path.to_string(), CoverageStatus::ReparsePointNotFollowed);
        } else if meta.is_dir {
            if self.found.contains_key(&c.target.key()) {
                // Already walked; just merge the new discovery reason.
            } else {
                c.stats = Some(self.tree_stats(path));
            }
        } else {
            c.stats = Some(TreeStats {
                files: 1,
                bytes: meta.size,
                modified_after_install: match (self.s.snap.install_day, meta.modified) {
                    (Some(d), Some(m)) if m > (d + 1) * 86_400 => 1,
                    _ => 0,
                },
                ..Default::default()
            });
        }
        self.add(c);
    }

    fn scan_root(&mut self, root: &WinPath, label: &str) {
        if self.cancelled() {
            self.cover(format!("{label}: {root}"), CoverageStatus::Cancelled);
            return;
        }
        self.phase = format!("Scanning {label}");
        self.emit();
        let list = match self.s.fs.read_dir(root) {
            Ok(l) => l,
            Err(e) => {
                self.cover(format!("{label}: {root}"), fs_status(&e));
                return;
            }
        };
        self.cover(format!("{label}: {root}"), CoverageStatus::Scanned);
        let m = self.s.snap.matcher.clone();
        for e in list {
            self.tick(1);
            if self.cancelled() {
                self.cover(format!("{label}: {root} (remaining entries)"), CoverageStatus::Cancelled);
                break;
            }
            let Ok(child) = root.join(&e.name) else { continue };
            match m.match_top(&e.name) {
                Some(NameMatch::Publisher) if e.meta.is_dir && !e.meta.is_reparse => {
                    self.publisher_folder(&child, label);
                }
                Some(NameMatch::Publisher) => {}
                Some(k) => {
                    let why = format!("{} ({label})", k.describe());
                    self.path_candidate(&child, &why, None);
                }
                None => {}
            }
        }
    }

    fn publisher_folder(&mut self, dir: &WinPath, label: &str) {
        let list = match self.s.fs.read_dir(dir) {
            Ok(l) => l,
            Err(e) => {
                self.cover(dir.to_string(), fs_status(&e));
                return;
            }
        };
        self.cover(dir.to_string(), CoverageStatus::Scanned);
        let m = self.s.snap.matcher.clone();
        let mut matched = 0usize;
        for e in &list {
            self.tick(1);
            if let Some(k) = m.match_under_publisher(&e.name) {
                if let Ok(child) = dir.join(&e.name) {
                    matched += 1;
                    let why = format!("{} ({label})", k.describe());
                    self.path_candidate(&child, &why, None);
                }
            }
        }
        // The publisher folder itself: always Review, never recursive.
        let mut c = Candidate::new(Target::Path { path: dir.clone(), is_dir: true });
        c.discovered_by.push(format!("{} ({label})", NameMatch::Publisher.describe()));
        let siblings = list.len().saturating_sub(matched);
        c.shared.push("publisher folder: would only ever be removed if empty, never recursively".into());
        if siblings > 0 {
            c.shared.push(format!("contains {siblings} other item(s) not attributed to this product"));
        }
        if self.s.snap.publisher_shared_with > 0 {
            c.shared.push(format!(
                "publisher shared with {} other installed program(s)",
                self.s.snap.publisher_shared_with
            ));
        }
        self.add(c);
    }

    fn reg_software(&mut self, root: RegRoot, view: RegView) {
        let base = RegKeyRef { root, view, path: "Software".into() };
        if self.cancelled() {
            self.cover(base.to_string(), CoverageStatus::Cancelled);
            return;
        }
        self.phase = format!("Scanning registry {base}");
        self.emit();
        let subs = match self.s.reg.subkeys(root, view, "Software") {
            Ok(s) => s,
            Err(e) => {
                self.cover(base.to_string(), reg_status(&e));
                return;
            }
        };
        self.cover(base.to_string(), CoverageStatus::Scanned);
        let m = self.s.snap.matcher.clone();
        for name in subs {
            self.tick(1);
            let path = format!("Software\\{name}");
            match m.match_top(&name) {
                Some(NameMatch::Publisher) => {
                    let key = RegKeyRef { root, view, path: path.clone() };
                    let children = match self.s.reg.subkeys(root, view, &path) {
                        Ok(c) => c,
                        Err(e) => {
                            self.cover(key.to_string(), reg_status(&e));
                            continue;
                        }
                    };
                    let mut matched = 0;
                    for child in &children {
                        if let Some(k) = m.match_under_publisher(child) {
                            matched += 1;
                            let mut c = Candidate::new(Target::RegKey {
                                key: RegKeyRef { root, view, path: format!("{path}\\{child}") },
                            });
                            c.discovered_by.push(k.describe().into());
                            self.add(c);
                        }
                    }
                    let mut c = Candidate::new(Target::RegKey { key });
                    c.discovered_by.push(NameMatch::Publisher.describe().into());
                    c.shared.push("publisher key: would only ever be removed if empty".into());
                    if children.len() > matched {
                        c.shared.push(format!("contains {} other subkey(s)", children.len() - matched));
                    }
                    if self.s.snap.publisher_shared_with > 0 {
                        c.shared.push(format!(
                            "publisher shared with {} other installed program(s)",
                            self.s.snap.publisher_shared_with
                        ));
                    }
                    self.add(c);
                }
                Some(k) => {
                    let mut c = Candidate::new(Target::RegKey { key: RegKeyRef { root, view, path } });
                    c.discovered_by.push(k.describe().into());
                    self.add(c);
                }
                None => {}
            }
        }
    }

    fn uninstall_entries(&mut self) {
        for key in self.s.snap.uninstall_keys.clone() {
            match self.s.reg.values(key.root, key.view, &key.path) {
                Ok(_) => {
                    let mut c = Candidate::new(Target::RegKey { key: key.clone() });
                    c.discovered_by.push("program's Uninstall entry still present".into());
                    c.ownership.push(Ownership::UninstallEntry);
                    self.add(c);
                    self.cover(key.to_string(), CoverageStatus::Scanned);
                }
                Err(e) => self.cover(key.to_string(), reg_status(&e)),
            }
        }
    }

    fn app_paths(&mut self) {
        let Ok(dir) = self.s.snap.install_dir.clone() else { return };
        for (root, view) in
            [(RegRoot::Hklm, RegView::Reg64), (RegRoot::Hklm, RegView::Reg32), (RegRoot::Hkcu, RegView::Shared)]
        {
            let base = RegKeyRef { root, view, path: APP_PATHS_PATH.into() };
            if self.cancelled() {
                self.cover(base.to_string(), CoverageStatus::Cancelled);
                continue;
            }
            let subs = match self.s.reg.subkeys(root, view, APP_PATHS_PATH) {
                Ok(s) => s,
                Err(e) => {
                    self.cover(base.to_string(), reg_status(&e));
                    continue;
                }
            };
            self.cover(base.to_string(), CoverageStatus::Scanned);
            for name in subs {
                self.tick(1);
                let path = format!("{APP_PATHS_PATH}\\{name}");
                let Ok(vals) = self.s.reg.values(root, view, &path) else { continue };
                let target = vals.get("").and_then(|v| v.as_str()).map(crate::evidence::icon_path);
                if let Some(Ok(p)) = target.map(WinPath::parse_external) {
                    if p.is_within(&dir) {
                        let mut c = Candidate::new(Target::RegKey { key: RegKeyRef { root, view, path } });
                        c.discovered_by.push("App Paths registration".into());
                        c.ownership.push(Ownership::AppPathPointsInsideInstallDir(p.to_string()));
                        self.add(c);
                    }
                }
            }
        }
    }
}

impl<'a> Scanner<'a> {
    pub fn run(&self, on_progress: &mut dyn FnMut(&Progress)) -> ScanReport {
        let mut r = Run {
            s: self,
            found: BTreeMap::new(),
            coverage: vec![],
            visited: 0,
            phase: "Starting".into(),
            last_emit: Instant::now(),
            last_emit_items: 0,
            on_progress,
        };
        let snap = self.snap;

        // 1. Directories with direct ownership evidence.
        match &snap.install_dir {
            Ok(d) => r.path_candidate(d, "program's InstallLocation", Some(Ownership::IsInstallDir)),
            Err(why) => r.cover("InstallLocation", CoverageStatus::Error(format!("not used: {why:?}"))),
        }
        for (d, own, why) in [
            (&snap.uninstaller_dir, Ownership::UninstallerDir, "folder of the uninstaller"),
            (&snap.icon_dir, Ownership::IconDir, "folder of the program icon"),
        ] {
            if let Some(d) = d {
                if !matches!(&snap.install_dir, Ok(i) if d.is_within(i)) {
                    r.path_candidate(d, why, Some(own));
                }
            }
        }

        // 2. Name-based discovery under Known Folder roots.
        let k = self.known;
        let roots: Vec<(&Option<WinPath>, &str)> = vec![
            (&k.program_files, "Program Files"),
            (&k.program_files_x86, "Program Files (x86)"),
            (&k.program_files_common, "Common Files"),
            (&k.program_files_common_x86, "Common Files (x86)"),
            (&k.program_data, "ProgramData"),
            (&k.roaming_app_data, "AppData\\Roaming"),
            (&k.local_app_data, "AppData\\Local"),
            (&k.local_programs, "AppData\\Local\\Programs"),
            (&k.start_menu_programs, "Start Menu"),
            (&k.common_start_menu_programs, "Common Start Menu"),
            (&k.desktop, "Desktop"),
            (&k.common_desktop, "Public Desktop"),
        ];
        let mut seen: Vec<WinPath> = vec![];
        for (root, label) in roots {
            if let Some(root) = root {
                if seen.contains(root) {
                    continue;
                }
                seen.push(root.clone());
                r.scan_root(root, label);
            }
        }

        // 3. Registry.
        r.uninstall_entries();
        r.reg_software(RegRoot::Hkcu, RegView::Shared);
        r.reg_software(RegRoot::Hklm, RegView::Reg64);
        r.reg_software(RegRoot::Hklm, RegView::Reg32);
        r.app_paths();

        let cancelled = self.cancel.is_cancelled();
        let mut candidates: Vec<Candidate> = r.found.into_values().collect();
        for (i, c) in candidates.iter_mut().enumerate() {
            c.id = format!("c{i}");
            evaluate(c, snap, self.known);
        }
        candidates.sort_by_key(|a| (a.verdict, a.target.display()));
        let complete = !cancelled && !r.coverage.iter().any(|c| c.status.is_gap());
        let visited = r.visited;
        r.phase = if cancelled { "Cancelled".into() } else { "Done".into() };
        let report = ScanReport { candidates, coverage: r.coverage, complete, cancelled, items_visited: visited };
        (r.on_progress)(&Progress {
            phase: r.phase.clone(),
            items_visited: visited,
            candidates: report.candidates.len(),
        });
        report
    }
}
