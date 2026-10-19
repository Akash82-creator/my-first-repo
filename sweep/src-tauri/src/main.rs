#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;
mod views;

use platform::Platform;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use sweep_core::evidence::{take_snapshot, EvidenceSnapshot, SnapshotEnv};
use sweep_core::inventory::{build_inventory, read_entries, Inventory};
use sweep_core::launch::{plan_launch, LaunchPlan};
use sweep_core::outcome::UninstallState;
use sweep_core::scan::{CancelToken, Progress, ScanOptions, ScanReport, Scanner};
use tauri::{AppHandle, Emitter, Manager, State};

struct Op {
    program_name: String,
    snapshot: Arc<EvidenceSnapshot>,
    state: UninstallState,
    stop: Arc<AtomicBool>,
}

struct ScanSession {
    cancel: CancelToken,
    progress: Progress,
    report: Option<ScanReport>,
    uncertain: bool,
}

#[derive(Default)]
struct Inner {
    inventory: Inventory,
    ops: HashMap<String, Op>,
    scans: HashMap<String, ScanSession>,
}

struct AppState {
    platform: Arc<Platform>,
    inner: Mutex<Inner>,
    next_id: AtomicU64,
}

impl AppState {
    fn id(&self, prefix: &str) -> String {
        format!("{prefix}-{}", self.next_id.fetch_add(1, Ordering::SeqCst))
    }
}

type Res<T> = Result<T, String>;

fn snapshot_for(
    platform: &Platform,
    inv: &Inventory,
    program_id: &str,
) -> Res<(EvidenceSnapshot, sweep_core::model::InstalledProgram)> {
    let p = inv.get(program_id).ok_or("unknown program; refresh the list")?.clone();
    let env = |n: &str| std::env::var(n).ok();
    let snap = take_snapshot(&p, inv, &SnapshotEnv { known: &platform.known, fs: platform.fs.as_ref(), env: &env });
    Ok((snap, p))
}

fn plan_for(platform: &Platform, snap: &EvidenceSnapshot, scope: sweep_core::model::Scope) -> Res<LaunchPlan> {
    let cmd = snap.command.clone().map_err(|e| format!("Uninstall command not supported: {e}"))?;
    let level = platform.manifest_level(&cmd);
    plan_launch(&cmd, scope, platform.elevated, level, platform.known.system32.as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn env_status(state: State<AppState>) -> views::EnvStatus {
    let inner = state.inner.lock().unwrap();
    views::EnvStatus {
        stage: sweep_core::BUILD_STAGE,
        cleanup_enabled: sweep_core::CLEANUP_ENABLED,
        restore_point: "unavailable",
        elevated: state.platform.elevated,
        supported_platform: state.platform.supported,
        inventory_unreadable: inner.inventory.unreadable.clone(),
    }
}

#[tauri::command]
fn list_programs(state: State<AppState>) -> Res<Vec<views::ProgramView>> {
    let (entries, unreadable) = read_entries(state.platform.reg.as_ref());
    let inv = build_inventory(entries, unreadable);
    let out = inv.programs.iter().map(views::program).collect();
    state.inner.lock().unwrap().inventory = inv;
    Ok(out)
}

#[tauri::command]
fn preview_uninstall(state: State<AppState>, program_id: String) -> Res<views::PreviewView> {
    let inner = state.inner.lock().unwrap();
    let (snap, p) = snapshot_for(&state.platform, &inner.inventory, &program_id)?;
    let plan = plan_for(&state.platform, &snap, p.scope);
    Ok(views::preview(&p, &snap, plan))
}

#[tauri::command]
fn start_uninstall(app: AppHandle, state: State<AppState>, program_id: String) -> Res<String> {
    if !state.platform.supported {
        return Err("Uninstalling is only supported on Windows.".into());
    }
    let mut inner = state.inner.lock().unwrap();
    if inner.ops.values().any(|o| !o.state.is_finished()) {
        return Err("Another uninstaller is still running.".into());
    }
    // Pre-uninstall evidence snapshot: the scan uses this even if the
    // uninstaller removes the registry entry.
    let (snap, p) = snapshot_for(&state.platform, &inner.inventory, &program_id)?;
    let plan = plan_for(&state.platform, &snap, p.scope)?;
    let op_id = state.id("op");
    let snap = Arc::new(snap);
    let stop = Arc::new(AtomicBool::new(false));
    inner.ops.insert(
        op_id.clone(),
        Op {
            program_name: p.display_name.clone(),
            snapshot: snap.clone(),
            state: UninstallState::Ready,
            stop: stop.clone(),
        },
    );
    drop(inner);
    let platform = state.platform.clone();
    let id = op_id.clone();
    let name = p.display_name.clone();
    std::thread::spawn(move || {
        let st = app.state::<AppState>();
        let update = |s: &UninstallState| {
            if let Some(op) = st.inner.lock().unwrap().ops.get_mut(&id) {
                op.state = s.clone();
            }
            let _ = app.emit("uninstall-update", views::op(&id, &name, s));
        };
        platform.run_vendor_uninstall(&snap, &plan, &stop, update);
    });
    Ok(op_id)
}

#[tauri::command]
fn stop_waiting(state: State<AppState>, op_id: String) -> Res<()> {
    let inner = state.inner.lock().unwrap();
    let op = inner.ops.get(&op_id).ok_or("unknown operation")?;
    op.stop.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_operation(state: State<AppState>, op_id: String) -> Res<views::OpView> {
    let inner = state.inner.lock().unwrap();
    let op = inner.ops.get(&op_id).ok_or("unknown operation")?;
    Ok(views::op(&op_id, &op.program_name, &op.state))
}

/// Start a read-only leftover scan, either for a finished uninstall
/// operation or for a program that is still installed.
#[tauri::command]
fn start_scan(
    app: AppHandle,
    state: State<AppState>,
    op_id: Option<String>,
    program_id: Option<String>,
    acknowledge_uncertain: bool,
) -> Res<String> {
    let mut inner = state.inner.lock().unwrap();
    let (snap, uncertain) = match (op_id, program_id) {
        (Some(op_id), None) => {
            let op = inner.ops.get(&op_id).ok_or("unknown operation")?;
            let UninstallState::Finished { outcome, .. } = &op.state else {
                return Err("The uninstaller has not finished yet.".into());
            };
            let uncertain = !outcome.auto_scan_allowed();
            (op.snapshot.clone(), uncertain)
        }
        (None, Some(pid)) => {
            let (snap, _) = snapshot_for(&state.platform, &inner.inventory, &pid)?;
            (Arc::new(snap), true)
        }
        _ => return Err("specify exactly one of op_id or program_id".into()),
    };
    if uncertain && !acknowledge_uncertain {
        return Err("The uninstall outcome is uncertain; confirm to run a read-only scan anyway.".into());
    }
    let scan_id = state.id("scan");
    let cancel = CancelToken::default();
    inner.scans.insert(
        scan_id.clone(),
        ScanSession { cancel: cancel.clone(), progress: Progress::default(), report: None, uncertain },
    );
    drop(inner);
    let platform = state.platform.clone();
    let id = scan_id.clone();
    std::thread::spawn(move || {
        let st = app.state::<AppState>();
        let scanner = Scanner {
            snap: &snap,
            known: &platform.known,
            fs: platform.fs.as_ref(),
            reg: platform.reg.as_ref(),
            opts: ScanOptions::default(),
            cancel,
        };
        let report = scanner.run(&mut |p: &Progress| {
            if let Some(s) = st.inner.lock().unwrap().scans.get_mut(&id) {
                s.progress = p.clone();
            }
            let _ = app.emit("scan-progress", views::scan(&id, p, None, uncertain));
        });
        let view = {
            let mut inner = st.inner.lock().unwrap();
            let s = inner.scans.get_mut(&id).expect("scan session");
            s.report = Some(report);
            views::scan(&id, &s.progress, s.report.as_ref(), s.uncertain)
        };
        let _ = app.emit("scan-done", view);
    });
    Ok(scan_id)
}

#[tauri::command]
fn cancel_scan(state: State<AppState>, scan_id: String) -> Res<()> {
    let inner = state.inner.lock().unwrap();
    inner.scans.get(&scan_id).ok_or("unknown scan")?.cancel.cancel();
    Ok(())
}

#[tauri::command]
fn get_scan(state: State<AppState>, scan_id: String) -> Res<views::ScanView> {
    let inner = state.inner.lock().unwrap();
    let s = inner.scans.get(&scan_id).ok_or("unknown scan")?;
    Ok(views::scan(&scan_id, &s.progress, s.report.as_ref(), s.uncertain))
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            platform: Arc::new(Platform::new()),
            inner: Mutex::new(Inner::default()),
            next_id: AtomicU64::new(1),
        })
        .invoke_handler(tauri::generate_handler![
            env_status,
            list_programs,
            preview_uninstall,
            start_uninstall,
            stop_waiting,
            get_operation,
            start_scan,
            cancel_scan,
            get_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sweep");
}
