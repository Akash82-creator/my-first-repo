//! Stage A acceptance tests (Linux, fakes with Windows semantics).
//! Windows-runtime counterparts live in `crates/sweep-win/tests/` (planned
//! CI; not executed in the development container).

use crate::api::KnownFolders;
use crate::evidence::{take_snapshot, Candidate, EvidenceSnapshot, Ownership, SnapshotEnv, Target, Verdict};
use crate::fakes::{known_folders, FakeFs, FakeReg};
use crate::inventory::{build_inventory, Inventory};
use crate::model::{
    days_from_civil, RegRoot, RegValue, RegView, Scope, UninstallEntry, APP_PATHS_PATH, UNINSTALL_PATH,
};
use crate::protect::LocationRejection;
use crate::scan::{CancelToken, CoverageStatus, ScanOptions, ScanReport, Scanner};
use crate::winpath::WinPath;

const BEFORE_INSTALL: i64 = 19_000 * 86_400; // 2022
fn after_install() -> i64 {
    (days_from_civil(2024, 6, 1)) * 86_400
}

fn sz(s: &str) -> RegValue {
    RegValue::Sz(s.into())
}

fn entry(key: &str, vals: &[(&str, &str)]) -> UninstallEntry {
    UninstallEntry {
        scope: Scope::Machine,
        view: RegView::Reg64,
        key_name: key.into(),
        values: vals.iter().map(|(k, v)| (k.to_string(), sz(v))).collect(),
    }
}

struct World {
    fs: FakeFs,
    reg: FakeReg,
    known: KnownFolders,
    inv: Inventory,
}

/// Two products from one publisher, plus unrelated and user data.
fn world() -> World {
    let known = known_folders();
    let inv = build_inventory(
        vec![
            entry(
                "AcmeNotes",
                &[
                    ("DisplayName", "Acme Notes 2.3 (x64)"),
                    ("Publisher", "Acme Corporation"),
                    ("InstallLocation", "C:\\Program Files\\Acme\\Notes\\"),
                    ("UninstallString", "\"C:\\Program Files\\Acme\\Notes\\unins000.exe\" /SILENT"),
                    ("InstallDate", "20240101"),
                ],
            ),
            entry(
                "AcmePaint",
                &[
                    ("DisplayName", "Acme Paint"),
                    ("Publisher", "Acme Corporation"),
                    ("InstallLocation", "C:\\Program Files\\Acme\\Paint"),
                    ("UninstallString", "C:\\Program Files\\Acme\\Paint\\uninstall.exe"),
                ],
            ),
        ],
        vec![],
    );
    let mut fs = FakeFs::new();
    for d in [
        "C:\\Program Files",
        "C:\\Program Files (x86)",
        "C:\\Program Files\\Common Files",
        "C:\\ProgramData",
        "C:\\Users\\alice\\AppData\\Roaming",
        "C:\\Users\\alice\\AppData\\Local\\Programs",
        "C:\\Users\\alice\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs",
        "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs",
        "C:\\Users\\alice\\Desktop",
        "C:\\Users\\Public\\Desktop",
    ] {
        fs.dir(d);
    }
    fs.file("C:\\Program Files\\Acme\\Notes\\unins000.exe", 100, BEFORE_INSTALL)
        .file("C:\\Program Files\\Acme\\Notes\\notes.dll", 100, BEFORE_INSTALL)
        .file("C:\\Program Files\\Acme\\Paint\\paint.exe", 100, BEFORE_INSTALL)
        .file("C:\\Program Files\\Acme\\Paint\\uninstall.exe", 100, BEFORE_INSTALL)
        .file("C:\\Users\\alice\\AppData\\Roaming\\Acme\\Notes\\settings.json", 10, BEFORE_INSTALL)
        .file("C:\\Users\\alice\\AppData\\Roaming\\Acme\\Paint\\settings.json", 10, BEFORE_INSTALL)
        // Unrelated folders whose names resemble the product.
        .file("C:\\ProgramData\\Acme Notes Backup\\data.bin", 10, BEFORE_INSTALL)
        .file("C:\\Program Files (x86)\\AcmeNotes\\other.exe", 10, BEFORE_INSTALL)
        .file("C:\\Users\\alice\\Desktop\\Acme Notes.lnk", 1, BEFORE_INSTALL);
    let mut reg = FakeReg::new();
    for (root, view) in
        [(RegRoot::Hkcu, RegView::Shared), (RegRoot::Hklm, RegView::Reg64), (RegRoot::Hklm, RegView::Reg32)]
    {
        reg.key(root, view, "Software\\Microsoft", &[]);
        reg.key(root, view, APP_PATHS_PATH, &[]);
    }
    reg.key(RegRoot::Hkcu, RegView::Shared, "Software\\Acme\\Notes", &[("Theme", sz("dark"))])
        .key(RegRoot::Hkcu, RegView::Shared, "Software\\Acme\\Paint", &[])
        .key(
            RegRoot::Hklm,
            RegView::Reg64,
            &format!("{UNINSTALL_PATH}\\AcmeNotes"),
            &[("DisplayName", sz("Acme Notes"))],
        )
        .key(
            RegRoot::Hklm,
            RegView::Reg64,
            &format!("{APP_PATHS_PATH}\\notes.exe"),
            &[("", sz("C:\\Program Files\\Acme\\Notes\\notes.exe"))],
        )
        .key(
            RegRoot::Hklm,
            RegView::Reg64,
            &format!("{APP_PATHS_PATH}\\paint.exe"),
            &[("", sz("C:\\Program Files\\Acme\\Paint\\paint.exe"))],
        );
    World { fs, reg, known, inv }
}

fn snapshot(w: &World, name: &str) -> EvidenceSnapshot {
    let prog = w.inv.programs.iter().find(|p| p.display_name.starts_with(name)).unwrap();
    let env = |_: &str| None;
    take_snapshot(prog, &w.inv, &SnapshotEnv { known: &w.known, fs: &w.fs, env: &env })
}

fn scan_with(w: &World, snap: &EvidenceSnapshot, cancel: CancelToken) -> ScanReport {
    let s = Scanner { snap, known: &w.known, fs: &w.fs, reg: &w.reg, opts: ScanOptions::default(), cancel };
    s.run(&mut |_| {})
}

fn scan(w: &World, name: &str) -> ScanReport {
    let snap = snapshot(w, name);
    scan_with(w, &snap, CancelToken::default())
}

fn find<'a>(r: &'a ScanReport, display: &str) -> Option<&'a Candidate> {
    let want = display.to_lowercase();
    r.candidates.iter().find(|c| c.target.display().to_lowercase() == want)
}

fn wp(s: &str) -> WinPath {
    WinPath::parse_external(s).unwrap()
}

#[test]
fn shared_publisher_two_products() {
    let w = world();
    let r = scan(&w, "Acme Notes");
    assert!(r.complete, "gaps: {:?}", r.gaps().collect::<Vec<_>>());

    // Nothing belonging to Acme Paint is a candidate at all, let alone Likely.
    for c in &r.candidates {
        let d = c.target.display().to_lowercase();
        assert!(!d.contains("paint"), "Paint artifact surfaced: {d}");
    }
    // Publisher folder containing Paint's install dir is Blocked.
    let pf = find(&r, "C:\\Program Files\\Acme").expect("publisher folder listed");
    assert_eq!(pf.verdict, Verdict::Blocked);
    // Publisher folder in AppData is Review with shared signals, never Likely.
    let ad = find(&r, "C:\\Users\\alice\\AppData\\Roaming\\Acme").expect("appdata publisher folder");
    assert_eq!(ad.verdict, Verdict::Review);
    assert!(ad.shared.iter().any(|s| s.contains("never recursively")));
    assert!(ad.shared.iter().any(|s| s.contains("1 other installed program")));
    // Publisher registry key: Review.
    let rk = find(&r, "HKCU\\Software\\Acme").expect("publisher key");
    assert_eq!(rk.verdict, Verdict::Review);
    // The product's own install dir is Likely (strong ownership, no concerns).
    let own = find(&r, "C:\\Program Files\\Acme\\Notes").unwrap();
    assert_eq!(own.verdict, Verdict::Likely);
    assert!(own.ownership.contains(&Ownership::IsInstallDir));
    // Product subfolder of a publisher folder, found by name only: Review.
    let cfg = find(&r, "C:\\Users\\alice\\AppData\\Roaming\\Acme\\Notes").unwrap();
    assert_eq!(cfg.verdict, Verdict::Review);
    assert!(cfg.ownership.is_empty());
    // Uninstall entry and App Paths entry pointing inside: strong.
    let u = find(&r, &format!("HKLM\\{UNINSTALL_PATH}\\AcmeNotes (64-bit)")).unwrap();
    assert_eq!(u.verdict, Verdict::Likely);
    let ap = find(&r, &format!("HKLM\\{APP_PATHS_PATH}\\notes.exe (64-bit)")).unwrap();
    assert_eq!(ap.verdict, Verdict::Likely);
    assert!(find(&r, &format!("HKLM\\{APP_PATHS_PATH}\\paint.exe (64-bit)")).is_none());
}

#[test]
fn unrelated_name_match_not_preselected() {
    let w = world();
    let r = scan(&w, "Acme Notes");
    for path in [
        "C:\\ProgramData\\Acme Notes Backup",
        "C:\\Program Files (x86)\\AcmeNotes",
        "C:\\Users\\alice\\Desktop\\Acme Notes.lnk",
    ] {
        let c = find(&r, path).unwrap_or_else(|| panic!("{path} should be discovered"));
        assert!(!c.discovered_by.is_empty());
        assert!(c.ownership.is_empty(), "{path}: name is not ownership");
        assert_eq!(c.verdict, Verdict::Review, "{path}");
    }
    // Every Likely candidate has ownership evidence.
    assert!(r.candidates.iter().filter(|c| c.verdict == Verdict::Likely).all(|c| !c.ownership.is_empty()));
}

#[test]
fn user_data_inside_installdir_review_only() {
    let mut w = world();
    w.fs.file("C:\\Program Files\\Acme\\Notes\\notebooks\\my-notes.txt", 10, after_install());
    let r = scan(&w, "Acme Notes");
    let c = find(&r, "C:\\Program Files\\Acme\\Notes").unwrap();
    assert!(c.ownership.contains(&Ownership::IsInstallDir));
    assert_eq!(c.verdict, Verdict::Review, "user data blocks Likely");
    assert!(c.user_data.iter().any(|s| s.contains("modified after the install date")));
    assert!(c.user_data.iter().any(|s| s.contains("document")));
}

#[test]
fn junction_in_installdir_not_followed() {
    let mut w = world();
    w.fs.junction("C:\\Program Files\\Acme\\Notes\\shared-data");
    // Content "behind" the junction must never be visited.
    w.fs.file("C:\\Program Files\\Acme\\Notes\\shared-data\\precious.docx", 10, BEFORE_INSTALL);
    let r = scan(&w, "Acme Notes");
    let c = find(&r, "C:\\Program Files\\Acme\\Notes").unwrap();
    let st = c.stats.as_ref().unwrap();
    assert_eq!(st.reparse_points.len(), 1);
    assert_eq!(st.document_like, 0, "did not descend through the junction");
    assert_eq!(c.verdict, Verdict::Review);
    assert!(r.coverage.iter().any(|e| e.status == CoverageStatus::ReparsePointNotFollowed));
    assert!(!r.complete);
}

#[test]
fn access_denied_reported_as_coverage_gap() {
    let mut w = world();
    w.fs.deny("C:\\ProgramData");
    w.reg.deny(RegRoot::Hklm, RegView::Reg32, "Software");
    let r = scan(&w, "Acme Notes");
    assert!(!r.complete);
    let gaps: Vec<_> = r.gaps().map(|g| (g.location.clone(), g.status.clone())).collect();
    assert!(gaps.iter().any(|(l, s)| l.contains("ProgramData") && *s == CoverageStatus::AccessDenied), "{gaps:?}");
    assert!(
        gaps.iter().any(|(l, s)| l.contains("HKLM\\Software (32-bit)") && *s == CoverageStatus::AccessDenied),
        "{gaps:?}"
    );
    // The unrelated folder under ProgramData was not seen, and that is visible as a gap.
    assert!(find(&r, "C:\\ProgramData\\Acme Notes Backup").is_none());
}

#[test]
fn scan_cancellation_partial_report() {
    let w = world();
    let snap = snapshot(&w, "Acme Notes");
    let token = CancelToken::default();
    let t2 = token.clone();
    *w.fs.on_read_dir.lock().unwrap() = Some(Box::new(move |p: &WinPath| {
        if *p == WinPath::parse_external("C:\\ProgramData").unwrap() {
            t2.cancel();
        }
    }));
    let r = scan_with(&w, &snap, token);
    assert!(r.cancelled);
    assert!(!r.complete);
    assert!(r.coverage.iter().any(|e| e.status == CoverageStatus::Cancelled));
    // Locations after the cancel point were not scanned.
    assert!(!r.coverage.iter().any(|e| e.location.contains("Desktop") && e.status == CoverageStatus::Scanned));
}

#[test]
fn buggy_install_location_is_not_trusted() {
    let mut w = world();
    let mut e = w.inv.programs[0].clone();
    e.install_location = Some("C:\\Program Files".into());
    e.display_icon = Some("C:\\Windows\\Installer\\{X}\\icon.exe,0".into());
    w.inv.programs[0] = e;
    let snap = snapshot(&w, "Acme Notes");
    assert_eq!(snap.install_dir, Err(LocationRejection::IsContainer));
    assert!(snap.icon_dir.is_none(), "icon under Windows is not an anchor");
    let r = scan_with(&w, &snap, CancelToken::default());
    assert!(find(&r, "C:\\Program Files").is_none());
    // No candidate claims InstallLocation ownership. The Notes folder is still
    // supported by independent evidence: it holds the registered uninstaller.
    assert!(r.candidates.iter().all(
        |c| !c.ownership.contains(&Ownership::IsInstallDir) && !c.ownership.contains(&Ownership::InsideInstallDir)
    ));
    let notes = find(&r, "C:\\Program Files\\Acme\\Notes").unwrap();
    assert_eq!(notes.ownership, vec![Ownership::UninstallerDir]);
    for c in &r.candidates {
        if let Target::Path { path, .. } = &c.target {
            if path.is_within(&wp("C:\\Program Files")) && *path != wp("C:\\Program Files\\Acme\\Notes") {
                assert_ne!(c.verdict, Verdict::Likely, "{path}");
            }
        }
    }
}

#[test]
fn snapshot_survives_entry_removal() {
    // The vendor uninstaller removes the Uninstall key; the scan still works
    // from the pre-uninstall snapshot and reports the key as not present.
    let w = world();
    let snap = snapshot(&w, "Acme Notes");
    let mut reg = FakeReg::new();
    reg.key(RegRoot::Hkcu, RegView::Shared, "Software\\Acme\\Notes", &[]);
    let s = Scanner {
        snap: &snap,
        known: &w.known,
        fs: &w.fs,
        reg: &reg,
        opts: ScanOptions::default(),
        cancel: CancelToken::default(),
    };
    let r = s.run(&mut |_| {});
    assert!(find(&r, "C:\\Program Files\\Acme\\Notes").is_some());
    assert!(r
        .coverage
        .iter()
        .any(|e| e.location.contains("Uninstall\\AcmeNotes") && e.status == CoverageStatus::NotPresent));
}

#[test]
fn progress_is_reported() {
    let w = world();
    let snap = snapshot(&w, "Acme Notes");
    let s = Scanner {
        snap: &snap,
        known: &w.known,
        fs: &w.fs,
        reg: &w.reg,
        opts: ScanOptions::default(),
        cancel: CancelToken::default(),
    };
    let mut phases = vec![];
    let r = s.run(&mut |p| phases.push(p.phase.clone()));
    assert!(phases.iter().any(|p| p.contains("Program Files")));
    assert_eq!(phases.last().unwrap(), "Done");
    assert!(r.items_visited > 0);
}
