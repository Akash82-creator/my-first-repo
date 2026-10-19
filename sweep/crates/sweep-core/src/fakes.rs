//! In-memory fakes of the read-only OS traits, with Windows semantics:
//! case-insensitive lookup, junctions as reparse entries, per-path
//! access-denied flags. Paths are always Windows-style strings.

use crate::api::{DirEntry, EntryMeta, FsError, FsRead, KnownFolders, RegError, RegRead};
use crate::model::{RegRoot, RegValue, RegView};
use crate::winpath::{fold, WinPath};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

fn wp(s: &str) -> WinPath {
    WinPath::parse_external(s).expect("fake path")
}

#[derive(Default)]
pub struct FakeFs {
    nodes: BTreeMap<String, (WinPath, EntryMeta)>,
    denied: BTreeSet<String>,
    /// Hook run on every read_dir (used to cancel mid-scan in tests).
    pub on_read_dir: Mutex<Option<Box<dyn FnMut(&WinPath) + Send>>>,
}

impl FakeFs {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_parents(&mut self, p: &WinPath) {
        let mut cur = p.parent();
        while let Some(d) = cur {
            if d.is_root() {
                break;
            }
            self.nodes.entry(d.key()).or_insert_with(|| {
                (d.clone(), EntryMeta { is_dir: true, is_reparse: false, size: 0, modified: Some(0) })
            });
            cur = d.parent();
        }
    }

    pub fn dir(&mut self, path: &str) -> &mut Self {
        let p = wp(path);
        self.ensure_parents(&p);
        self.nodes.insert(p.key(), (p, EntryMeta { is_dir: true, is_reparse: false, size: 0, modified: Some(0) }));
        self
    }

    pub fn file(&mut self, path: &str, size: u64, modified: i64) -> &mut Self {
        let p = wp(path);
        self.ensure_parents(&p);
        self.nodes.insert(p.key(), (p, EntryMeta { is_dir: false, is_reparse: false, size, modified: Some(modified) }));
        self
    }

    /// A junction: listed as a reparse point; its target is never visible.
    pub fn junction(&mut self, path: &str) -> &mut Self {
        let p = wp(path);
        self.ensure_parents(&p);
        self.nodes.insert(p.key(), (p, EntryMeta { is_dir: true, is_reparse: true, size: 0, modified: Some(0) }));
        self
    }

    pub fn deny(&mut self, path: &str) -> &mut Self {
        self.denied.insert(wp(path).key());
        self
    }
}

impl FsRead for FakeFs {
    fn metadata(&self, path: &WinPath) -> Result<EntryMeta, FsError> {
        if path.is_root() {
            return Ok(EntryMeta { is_dir: true, is_reparse: false, size: 0, modified: None });
        }
        self.nodes.get(&path.key()).map(|(_, m)| m.clone()).ok_or(FsError::NotFound)
    }

    fn read_dir(&self, path: &WinPath) -> Result<Vec<DirEntry>, FsError> {
        if let Some(h) = self.on_read_dir.lock().unwrap().as_mut() {
            h(path);
        }
        if self.denied.contains(&path.key()) {
            return Err(FsError::AccessDenied);
        }
        let meta = self.metadata(path)?;
        if !meta.is_dir {
            return Err(FsError::Other("not a directory".into()));
        }
        if meta.is_reparse {
            return Err(FsError::Other("refusing to list through a reparse point".into()));
        }
        Ok(self
            .nodes
            .values()
            .filter(|(p, _)| p.parent().as_ref() == Some(path))
            .map(|(p, m)| DirEntry { name: p.file_name().unwrap().to_string(), meta: m.clone() })
            .collect())
    }
}

type RegKey = (RegRoot, RegView, String);

#[derive(Default)]
pub struct FakeReg {
    keys: BTreeMap<RegKey, (String, BTreeMap<String, RegValue>)>,
    denied: BTreeSet<RegKey>,
}

fn rk(root: RegRoot, view: RegView, path: &str) -> RegKey {
    (root, view, fold(path))
}

impl FakeReg {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key(&mut self, root: RegRoot, view: RegView, path: &str, values: &[(&str, RegValue)]) -> &mut Self {
        let mut parts: Vec<&str> = path.split('\\').collect();
        while parts.len() > 1 {
            parts.pop();
            let parent = parts.join("\\");
            self.keys.entry(rk(root, view, &parent)).or_insert_with(|| (parent.clone(), BTreeMap::new()));
        }
        let entry = self.keys.entry(rk(root, view, path)).or_insert_with(|| (path.to_string(), BTreeMap::new()));
        for (n, v) in values {
            entry.1.insert(n.to_string(), v.clone());
        }
        self
    }

    pub fn deny(&mut self, root: RegRoot, view: RegView, path: &str) -> &mut Self {
        self.denied.insert(rk(root, view, path));
        self
    }
}

impl RegRead for FakeReg {
    fn subkeys(&self, root: RegRoot, view: RegView, path: &str) -> Result<Vec<String>, RegError> {
        let k = rk(root, view, path);
        if self.denied.contains(&k) {
            return Err(RegError::AccessDenied);
        }
        if !self.keys.contains_key(&k) {
            return Err(RegError::NotFound);
        }
        let prefix = format!("{}\\", fold(path));
        Ok(self
            .keys
            .iter()
            .filter(|((r, v, p), _)| {
                *r == root && *v == view && p.starts_with(&prefix) && !p[prefix.len()..].contains('\\')
            })
            .map(|(_, (orig, _))| orig.rsplit('\\').next().unwrap().to_string())
            .collect())
    }

    fn values(&self, root: RegRoot, view: RegView, path: &str) -> Result<BTreeMap<String, RegValue>, RegError> {
        let k = rk(root, view, path);
        if self.denied.contains(&k) {
            return Err(RegError::AccessDenied);
        }
        self.keys.get(&k).map(|(_, v)| v.clone()).ok_or(RegError::NotFound)
    }
}

/// A typical Known Folders layout for user `alice` on drive C:.
pub fn known_folders() -> KnownFolders {
    let o = |s: &str| Some(wp(s));
    KnownFolders {
        windows: o("C:\\Windows"),
        system32: o("C:\\Windows\\System32"),
        syswow64: o("C:\\Windows\\SysWOW64"),
        program_files: o("C:\\Program Files"),
        program_files_x86: o("C:\\Program Files (x86)"),
        program_files_common: o("C:\\Program Files\\Common Files"),
        program_files_common_x86: o("C:\\Program Files (x86)\\Common Files"),
        program_data: o("C:\\ProgramData"),
        users: o("C:\\Users"),
        user_profile: o("C:\\Users\\alice"),
        public: o("C:\\Users\\Public"),
        roaming_app_data: o("C:\\Users\\alice\\AppData\\Roaming"),
        local_app_data: o("C:\\Users\\alice\\AppData\\Local"),
        local_programs: o("C:\\Users\\alice\\AppData\\Local\\Programs"),
        start_menu: o("C:\\Users\\alice\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu"),
        common_start_menu: o("C:\\ProgramData\\Microsoft\\Windows\\Start Menu"),
        start_menu_programs: o("C:\\Users\\alice\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs"),
        common_start_menu_programs: o("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs"),
        desktop: o("C:\\Users\\alice\\Desktop"),
        common_desktop: o("C:\\Users\\Public\\Desktop"),
        // Redirected folder on another volume.
        documents: o("D:\\Users\\alice\\Documents"),
        pictures: o("C:\\Users\\alice\\Pictures"),
        music: o("C:\\Users\\alice\\Music"),
        videos: o("C:\\Users\\alice\\Videos"),
        downloads: o("C:\\Users\\alice\\Downloads"),
        one_drive: o("C:\\Users\\alice\\OneDrive"),
        temp: o("C:\\Users\\alice\\AppData\\Local\\Temp"),
        sweep_dirs: vec![wp("C:\\Program Files\\Sweep")],
    }
}
