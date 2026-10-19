//! Stage A Windows integration tests (routine CI, fixture-based).
//!
//! Status: PLANNED. Written but not executed in the Linux development
//! container. Run on Windows with:
//!   cargo build -p sweep-fixtures
//!   set SWEEP_FIXTURE=%CD%\target\debug\sweep-fixture.exe
//!   cargo test -p sweep-win --test stage_a_windows -- --ignored --test-threads=1
//!
//! The fixtures only create per-user artifacts (HKCU, %LOCALAPPDATA%,
//! %APPDATA%), so no elevation is needed and no UAC prompt can block CI.
#![cfg(windows)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use sweep_core::evidence::{take_snapshot, Ownership, SnapshotEnv, Verdict};
use sweep_core::inventory::{build_inventory, read_entries, Inventory};
use sweep_core::launch::{plan_launch, Verb};
use sweep_core::outcome::{Outcome, UninstallState, UnknownReason};
use sweep_core::scan::{CancelToken, CoverageStatus, ScanOptions, ScanReport, Scanner};
use sweep_win::flow::run_vendor_uninstall;
use sweep_win::fs::WinFs;
use sweep_win::known::known_folders;
use sweep_win::registry::WinReg;

fn fixture() -> PathBuf {
    PathBuf::from(std::env::var("SWEEP_FIXTURE").expect("set SWEEP_FIXTURE to target\\debug\\sweep-fixture.exe"))
}

fn unique(tag: &str) -> String {
    format!("SweepT{}{}", tag, std::process::id())
}

struct Installed {
    name: String,
    publisher: String,
}

impl Installed {
    fn new(name: &str, publisher: &str, mode: &str, extra: &[&str]) -> Self {
        let st = Command::new(fixture())
            .args(["install", "--name", name, "--publisher", publisher, "--scope", "user", "--mode", mode])
            .args(extra)
            .status()
            .unwrap();
        assert!(st.success());
        Installed { name: name.into(), publisher: publisher.into() }
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        // Test-fixture cleanup only (not Sweep functionality).
        let local = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("Programs").join(&self.publisher);
        let roaming = PathBuf::from(std::env::var("APPDATA").unwrap()).join(&self.publisher);
        let _ = std::fs::remove_dir_all(local.join(&self.name));
        let _ = std::fs::remove_dir_all(roaming.join(&self.name));
        let _ = Command::new("reg")
            .args(["delete", &format!("HKCU\\Software\\{}\\{}", self.publisher, self.name), "/f"])
            .output();
        let _ = Command::new("reg")
            .args([
                "delete",
                &format!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SweepFixture_{}", self.name),
                "/f",
            ])
            .output();
    }
}

fn inventory() -> Inventory {
    let (entries, unreadable) = read_entries(&WinReg);
    build_inventory(entries, unreadable)
}

fn uninstall_and_scan(name: &str) -> (UninstallState, ScanReport) {
    let inv = inventory();
    let prog = inv.programs.iter().find(|p| p.display_name == name).expect("fixture in inventory");
    let known = known_folders();
    let env = |n: &str| std::env::var(n).ok();
    let snap = take_snapshot(prog, &inv, &SnapshotEnv { known: &known, fs: &WinFs, env: &env });
    let cmd = snap.command.clone().expect("fixture command parses");
    // CI runners (and Wine) run with an elevated token, which would make the
    // app refuse per-user uninstallers. That refusal is unit-tested in
    // sweep-core; here we exercise the launch/track/verify flow itself.
    let plan = plan_launch(&cmd, prog.scope, false, None, known.system32.as_ref()).unwrap();
    assert_eq!(plan.verb, Verb::Open, "per-user fixture never uses runas");
    let state = run_vendor_uninstall(&snap, &plan, &AtomicBool::new(false), |_| {});
    let scanner = Scanner {
        snap: &snap,
        known: &known,
        fs: &WinFs,
        reg: &WinReg,
        opts: ScanOptions::default(),
        cancel: CancelToken::default(),
    };
    (state, scanner.run(&mut |_| {}))
}

fn outcome(s: &UninstallState) -> Outcome {
    match s {
        UninstallState::Finished { outcome, .. } => outcome.clone(),
        other => panic!("not finished: {other:?}"),
    }
}

#[test]
#[ignore = "Windows CI: requires SWEEP_FIXTURE"]
fn inventory_lists_fixture_once_with_user_scope() {
    let name = unique("Inv");
    let _g = Installed::new(&name, "SweepPubInv", "ok", &[]);
    let inv = inventory();
    let found: Vec<_> = inv.programs.iter().filter(|p| p.display_name == name).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].scope, sweep_core::model::Scope::User);
}

#[test]
#[ignore = "Windows CI: requires SWEEP_FIXTURE"]
fn successful_uninstall_then_readonly_scan() {
    let name = unique("Ok");
    let publisher = unique("Pub");
    let _g = Installed::new(&name, &publisher, "ok", &[]);
    let (state, report) = uninstall_and_scan(&name);
    assert_eq!(outcome(&state), Outcome::Succeeded);
    let dir = report
        .candidates
        .iter()
        .find(|c| c.ownership.contains(&Ownership::IsInstallDir))
        .expect("leftover install dir");
    // uninstall.exe remains (fixture cannot delete itself): strong ownership, no user data.
    assert_eq!(dir.verdict, Verdict::Likely);
    // Name-only finds are Review.
    assert!(report.candidates.iter().filter(|c| c.ownership.is_empty()).all(|c| c.verdict != Verdict::Likely));
    assert!(report.candidates.iter().any(|c| c.target.display().contains(&format!("Software\\{publisher}"))));
}

#[test]
#[ignore = "Windows CI: requires SWEEP_FIXTURE"]
fn failure_and_unknown_outcomes() {
    let n1 = unique("Fail");
    let _a = Installed::new(&n1, "SweepPubF", "fail", &[]);
    let (s, _) = uninstall_and_scan(&n1);
    assert_eq!(outcome(&s), Outcome::Failed { code: 3, retryable: false });
    assert!(!outcome(&s).auto_scan_allowed());

    let n2 = unique("Leave");
    let _b = Installed::new(&n2, "SweepPubL", "leave-entry", &[]);
    let (s, _) = uninstall_and_scan(&n2);
    assert_eq!(outcome(&s), Outcome::Unknown(UnknownReason::EntryStillPresent));
}

#[test]
#[ignore = "Windows CI: requires SWEEP_FIXTURE"]
fn spawn_and_exit_uninstaller_is_tracked() {
    let name = unique("Spawn");
    let _g = Installed::new(&name, "SweepPubS", "spawn-child", &[]);
    let (s, _) = uninstall_and_scan(&name);
    // The parent exits immediately; the child removes the entry ~2 s later.
    // Only descendant tracking makes this Succeeded rather than Unknown.
    assert_eq!(outcome(&s), Outcome::Succeeded);
}

#[test]
#[ignore = "Windows CI: requires SWEEP_FIXTURE"]
fn shared_publisher_two_products() {
    let publisher = unique("Shared");
    let a = unique("ProdA");
    let b = unique("ProdB");
    let _ga = Installed::new(&a, &publisher, "ok", &[]);
    let _gb = Installed::new(&b, &publisher, "ok", &[]);
    let (_, report) = uninstall_and_scan(&a);
    for c in &report.candidates {
        assert!(!c.target.display().contains(&b), "B surfaced: {}", c.target.display());
        if c.target.display().ends_with(&publisher) {
            assert_ne!(c.verdict, Verdict::Likely, "publisher folder never Likely");
        }
    }
}

#[test]
#[ignore = "Windows CI: requires SWEEP_FIXTURE"]
fn junction_in_installdir_not_followed() {
    let sentinel = std::env::temp_dir().join(unique("Sentinel"));
    std::fs::create_dir_all(&sentinel).unwrap();
    std::fs::write(sentinel.join("precious.docx"), b"keep").unwrap();
    let name = unique("Junction");
    let _g = Installed::new(&name, "SweepPubJ", "ok", &["--junction", sentinel.to_str().unwrap()]);
    let (_, report) = uninstall_and_scan(&name);
    assert!(report.coverage.iter().any(|c| c.status == CoverageStatus::ReparsePointNotFollowed));
    let dir = report.candidates.iter().find(|c| c.ownership.contains(&Ownership::IsInstallDir)).unwrap();
    assert_eq!(dir.verdict, Verdict::Review);
    assert_eq!(dir.stats.as_ref().unwrap().document_like, 0, "did not descend into the junction");
    assert_eq!(std::fs::read(sentinel.join("precious.docx")).unwrap(), b"keep");
    let _ = std::fs::remove_dir_all(&sentinel);
}
