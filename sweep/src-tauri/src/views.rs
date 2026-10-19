//! The frontend contract. The UI only ever sends back opaque IDs.

use serde::Serialize;
use sweep_core::cmdline::UninstallCommand;
use sweep_core::evidence::{Candidate, EvidenceSnapshot, Target, Verdict};
use sweep_core::launch::{LaunchPlan, Verb};
use sweep_core::model::{InstalledProgram, RegView, Scope};
use sweep_core::outcome::{Outcome, UninstallState, UnknownReason};
use sweep_core::protect::Protection;
use sweep_core::scan::{CoverageStatus, Progress, ScanReport};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub stage: &'static str,
    pub cleanup_enabled: bool,
    pub restore_point: &'static str,
    pub elevated: bool,
    pub supported_platform: bool,
    pub inventory_unreadable: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgramView {
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub scope: &'static str,
    pub views: Vec<&'static str>,
    pub msi: bool,
    pub size_kb: Option<u32>,
    pub install_date: Option<String>,
    pub has_uninstaller: bool,
}

pub fn scope_label(s: Scope) -> &'static str {
    match s {
        Scope::Machine => "All users",
        Scope::User => "Current user",
    }
}

pub fn program(p: &InstalledProgram) -> ProgramView {
    ProgramView {
        id: p.id.clone(),
        name: p.display_name.clone(),
        publisher: p.publisher.clone(),
        version: p.version.clone(),
        scope: scope_label(p.scope),
        views: p
            .sources
            .iter()
            .map(|s| match s.view {
                RegView::Reg64 => "64-bit",
                RegView::Reg32 => "32-bit",
                RegView::Shared => "per-user",
            })
            .collect(),
        msi: p.windows_installer,
        size_kb: p.estimated_size_kb,
        install_date: p.install_date.clone(),
        has_uninstaller: p.uninstall_string.is_some() || p.windows_installer,
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub verb: &'static str,
    pub file: String,
    pub params: String,
    pub reason: String,
    pub elevation: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PreviewView {
    pub program: ProgramView,
    pub install_dir: Option<String>,
    pub install_dir_rejected: Option<String>,
    pub command_summary: Option<String>,
    pub command_error: Option<String>,
    pub plan: Option<PlanView>,
    pub refusal: Option<String>,
    pub restore_point: &'static str,
    pub restore_point_note: &'static str,
    pub can_launch: bool,
}

pub const RESTORE_POINT_NOTE: &str =
    "Unavailable in this build: creating a restore point needs the privileged helper planned for Stage B. \
     You can create one yourself in System Protection before continuing.";

pub fn preview(p: &InstalledProgram, snap: &EvidenceSnapshot, plan: Result<LaunchPlan, String>) -> PreviewView {
    let (command_summary, command_error) = match &snap.command {
        Ok(UninstallCommand::Msi { product_code }) => (Some(format!("Windows Installer product {product_code}")), None),
        Ok(UninstallCommand::Exe { exe, args }) => (Some(format!("\"{exe}\" {args}").trim().to_string()), None),
        Err(e) => (None, Some(e.to_string())),
    };
    let (plan, refusal) = match plan {
        Ok(pl) => (
            Some(PlanView {
                verb: match pl.verb {
                    Verb::Open => "open",
                    Verb::RunAs => "runas",
                },
                elevation: match (pl.verb, pl.is_msi) {
                    (Verb::RunAs, _) => "Windows will ask for administrator approval (for the uninstaller only)",
                    (Verb::Open, true) => "Windows Installer decides; it may prompt",
                    (Verb::Open, false) => "Decided by the uninstaller's own manifest",
                },
                file: pl.file.to_string(),
                params: pl.params.clone(),
                reason: pl.reason.clone(),
            }),
            None,
        ),
        Err(e) => (None, Some(e)),
    };
    let can_launch = plan.is_some();
    PreviewView {
        program: program(p),
        install_dir: snap.install_dir.as_ref().ok().map(|d| d.to_string()),
        install_dir_rejected: snap.install_dir.as_ref().err().map(|e| format!("{e:?}")),
        command_summary,
        command_error,
        plan,
        refusal,
        restore_point: "unavailable",
        restore_point_note: RESTORE_POINT_NOTE,
        can_launch,
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeView {
    pub kind: &'static str,
    pub label: String,
    pub detail: String,
    pub auto_scan_allowed: bool,
}

pub fn outcome(o: &Outcome) -> OutcomeView {
    let (kind, label, detail) = match o {
        Outcome::Succeeded => {
            ("succeeded", "Uninstaller finished".into(), "The program's registration is gone.".into())
        }
        Outcome::SucceededRebootRequired => (
            "succeeded",
            "Uninstaller finished; restart required".into(),
            "Some files will be removed by Windows at the next restart.".into(),
        ),
        Outcome::RebootInitiated => ("succeeded", "Uninstaller started a restart".into(), String::new()),
        Outcome::Cancelled => {
            ("cancelled", "Uninstall was cancelled".into(), "Nothing indicates the program was removed.".into())
        }
        Outcome::ElevationCancelled => {
            ("cancelled", "Administrator approval was declined".into(), "The uninstaller did not start.".into())
        }
        Outcome::Failed { code, retryable } => (
            "failed",
            format!("Uninstaller failed (exit code {code})"),
            if *retryable { "Another installation is in progress; try again later.".into() } else { String::new() },
        ),
        Outcome::LaunchFailed { code } => {
            ("failed", format!("Could not start the uninstaller (error {code})"), String::new())
        }
        Outcome::Unknown(r) => (
            "unknown",
            "Outcome uncertain".into(),
            match r {
                UnknownReason::EntryStillPresent => {
                    "The uninstaller exited normally but the program is still registered."
                }
                UnknownReason::NotInstalled => "Windows Installer reports the product was not installed.",
                UnknownReason::ProductStillInstalled => "Windows Installer still reports the product as installed.",
                UnknownReason::ExitCodeUnavailable => "The uninstaller's result could not be read.",
            }
            .into(),
        ),
    };
    OutcomeView { kind, label, detail, auto_scan_allowed: o.auto_scan_allowed() }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpView {
    pub op_id: String,
    pub program_name: String,
    pub state: &'static str,
    pub pids: Vec<u32>,
    pub tracking_incomplete: bool,
    pub outcome: Option<OutcomeView>,
}

pub fn op(op_id: &str, program_name: &str, st: &UninstallState) -> OpView {
    let (state, pids, inc, out) = match st {
        UninstallState::Ready => ("ready", vec![], false, None),
        UninstallState::Launching => ("launching", vec![], false, None),
        UninstallState::Running { pids, tracking_incomplete } => ("running", pids.clone(), *tracking_incomplete, None),
        UninstallState::Verifying { tracking_incomplete } => ("verifying", vec![], *tracking_incomplete, None),
        UninstallState::Finished { outcome: o, tracking_incomplete } => {
            ("finished", vec![], *tracking_incomplete, Some(outcome(o)))
        }
    };
    OpView {
        op_id: op_id.into(),
        program_name: program_name.into(),
        state,
        pids,
        tracking_incomplete: inc,
        outcome: out,
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatsView {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CandidateView {
    pub id: String,
    pub target: String,
    pub kind: &'static str,
    pub verdict: &'static str,
    pub discovered_by: Vec<String>,
    pub ownership: Vec<String>,
    pub shared: Vec<String>,
    pub user_data: Vec<String>,
    pub concerns: Vec<String>,
    pub protection: Option<String>,
    pub stats: Option<StatsView>,
}

fn candidate(c: &Candidate) -> CandidateView {
    CandidateView {
        id: c.id.clone(),
        target: c.target.display(),
        kind: match &c.target {
            Target::Path { is_dir: true, .. } => "folder",
            Target::Path { .. } => "file",
            Target::RegKey { .. } => "registry",
        },
        verdict: match c.verdict {
            Verdict::Likely => "likely",
            Verdict::Review => "review",
            Verdict::Blocked => "blocked",
        },
        discovered_by: c.discovered_by.clone(),
        ownership: c.ownership.iter().map(|o| o.describe()).collect(),
        shared: c.shared.clone(),
        user_data: c.user_data.clone(),
        concerns: c.concerns.clone(),
        protection: match &c.protection {
            Protection::None => None,
            Protection::Container(l) => Some(format!("protected location: {l}")),
            Protection::InSubtree(l) => Some(format!("inside protected area: {l}")),
            Protection::ContainsProtected(l) => Some(format!("contains protected area: {l}")),
        },
        stats: c.stats.as_ref().map(|s| StatsView { files: s.files, dirs: s.dirs, bytes: s.bytes }),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoverageView {
    pub location: String,
    pub status: &'static str,
    pub detail: Option<String>,
    pub gap: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReportView {
    pub candidates: Vec<CandidateView>,
    pub coverage: Vec<CoverageView>,
    pub complete: bool,
    pub cancelled: bool,
    pub uncertain: bool,
    pub cleanup_enabled: bool,
    pub items_visited: u64,
}

pub fn report(r: &ScanReport, uncertain: bool) -> ReportView {
    ReportView {
        candidates: r.candidates.iter().map(candidate).collect(),
        coverage: r
            .coverage
            .iter()
            .map(|c| {
                let (status, detail) = match &c.status {
                    CoverageStatus::Scanned => ("scanned", None),
                    CoverageStatus::NotPresent => ("not present", None),
                    CoverageStatus::AccessDenied => ("access denied", None),
                    CoverageStatus::ReparsePointNotFollowed => ("junction/symlink not followed", None),
                    CoverageStatus::LimitReached => ("size/depth limit reached", None),
                    CoverageStatus::Error(e) => ("error", Some(e.clone())),
                    CoverageStatus::Cancelled => ("cancelled", None),
                };
                CoverageView { location: c.location.clone(), status, detail, gap: c.status.is_gap() }
            })
            .collect(),
        complete: r.complete,
        cancelled: r.cancelled,
        uncertain,
        cleanup_enabled: sweep_core::CLEANUP_ENABLED,
        items_visited: r.items_visited,
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanView {
    pub scan_id: String,
    pub phase: String,
    pub items_visited: u64,
    pub candidate_count: usize,
    pub done: bool,
    pub report: Option<ReportView>,
}

pub fn scan(scan_id: &str, p: &Progress, r: Option<&ScanReport>, uncertain: bool) -> ScanView {
    ScanView {
        scan_id: scan_id.into(),
        phase: p.phase.clone(),
        items_visited: p.items_visited,
        candidate_count: p.candidates,
        done: r.is_some(),
        report: r.map(|r| report(r, uncertain)),
    }
}
