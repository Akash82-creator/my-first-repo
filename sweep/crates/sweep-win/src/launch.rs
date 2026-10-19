//! Launch a vendor uninstaller with ShellExecuteExW and track it and its
//! descendants until they all exit. Sweep itself never elevates: `runas`
//! elevates only the launched uninstaller.

use std::collections::HashMap;
use std::mem::size_of;
use std::time::Duration;
use sweep_core::launch::{LaunchPlan, Verb};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, FILETIME, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const STILL_ACTIVE: u32 = 259;

#[derive(Debug, Clone)]
pub enum TrackEvent {
    Started { pid: u32 },
    Tracked { pids: Vec<u32>, incomplete: bool },
}

#[derive(Debug, Clone)]
pub struct RunResult {
    /// Exit code of the launched process; None when no process handle was
    /// returned or the code could not be read.
    pub exit_code: Option<u32>,
    pub tracking_incomplete: bool,
    /// Exit codes of tracked descendants (for display).
    pub descendant_exits: Vec<(u32, Option<u32>)>,
}

#[derive(Debug, Clone)]
pub enum LaunchError {
    /// GetLastError from ShellExecuteExW (1223 = UAC cancelled).
    Win32(u32),
}

struct Tracked {
    handle: HANDLE,
    can_wait: bool,
    created: u64,
    done: bool,
    exit: Option<u32>,
}

fn ft(f: FILETIME) -> u64 {
    ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
}

fn creation_time(h: HANDLE) -> Option<u64> {
    let (mut c, mut e, mut k, mut u) =
        (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u).ok().map(|_| ft(c)) }
}

fn open(pid: u32) -> Option<(HANDLE, bool)> {
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid) {
            return Some((h, true));
        }
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok().map(|h| (h, false))
    }
}

fn has_exited(t: &mut Tracked) -> bool {
    if t.done {
        return true;
    }
    unsafe {
        let exited = if t.can_wait {
            WaitForSingleObject(t.handle, 0) == WAIT_OBJECT_0
        } else {
            let mut code = 0u32;
            GetExitCodeProcess(t.handle, &mut code).is_ok() && code != STILL_ACTIVE
        };
        if exited {
            let mut code = 0u32;
            t.exit = GetExitCodeProcess(t.handle, &mut code).ok().map(|_| code);
            t.done = true;
        }
        exited
    }
}

fn process_parents() -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut e = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                out.push((e.th32ProcessID, e.th32ParentProcessID));
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

/// Launch and block until the uninstaller and all tracked descendants exit.
///
/// `stop_waiting` lets the user stop waiting (e.g. the uninstaller opened a
/// browser that stays open). Tracking is then reported as incomplete.
pub fn run_uninstaller(
    plan: &LaunchPlan,
    stop_waiting: &std::sync::atomic::AtomicBool,
    mut on_event: impl FnMut(TrackEvent),
) -> Result<RunResult, LaunchError> {
    let verb = crate::wide::wide(match plan.verb {
        Verb::Open => "open",
        Verb::RunAs => "runas",
    });
    // Plain path (not \\?\): ShellExecute does not accept the extended prefix.
    let file = crate::wide::wide(&plan.file.to_string());
    let params = crate::wide::wide(&plan.params);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe {
        if ShellExecuteExW(&mut info).is_err() {
            return Err(LaunchError::Win32(GetLastError().0));
        }
    }
    let root = info.hProcess;
    if root.is_invalid() {
        // No process handle (e.g. handed off via DDE): completion is unknowable.
        return Ok(RunResult { exit_code: None, tracking_incomplete: true, descendant_exits: vec![] });
    }
    let root_pid = unsafe { GetProcessId(root) };
    on_event(TrackEvent::Started { pid: root_pid });

    let mut tracked: HashMap<u32, Tracked> = HashMap::new();
    tracked.insert(
        root_pid,
        Tracked { handle: root, can_wait: true, created: creation_time(root).unwrap_or(0), done: false, exit: None },
    );
    let mut incomplete = false;
    let mut last_reported: Vec<u32> = vec![root_pid];
    loop {
        // Discover new descendants: parent is tracked and child was created after it
        // (guards against PID reuse).
        for (pid, ppid) in process_parents() {
            if tracked.contains_key(&pid) {
                continue;
            }
            let Some(parent_created) = tracked.get(&ppid).map(|t| t.created) else { continue };
            match open(pid) {
                Some((h, can_wait)) => match creation_time(h) {
                    Some(c) if c >= parent_created => {
                        tracked.insert(pid, Tracked { handle: h, can_wait, created: c, done: false, exit: None });
                    }
                    _ => unsafe {
                        let _ = CloseHandle(h);
                    },
                },
                None => incomplete = true,
            }
        }
        let mut alive = Vec::new();
        for (pid, t) in tracked.iter_mut() {
            if !has_exited(t) {
                alive.push(*pid);
            }
        }
        alive.sort_unstable();
        if alive != last_reported {
            on_event(TrackEvent::Tracked { pids: alive.clone(), incomplete });
            last_reported = alive.clone();
        }
        if alive.is_empty() {
            break;
        }
        if stop_waiting.load(std::sync::atomic::Ordering::SeqCst) {
            incomplete = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let exit_code = tracked.get(&root_pid).and_then(|t| t.exit);
    let descendant_exits = tracked.iter().filter(|(p, _)| **p != root_pid).map(|(p, t)| (*p, t.exit)).collect();
    for t in tracked.values() {
        unsafe {
            let _ = CloseHandle(t.handle);
        }
    }
    Ok(RunResult { exit_code, tracking_incomplete: incomplete, descendant_exits })
}
