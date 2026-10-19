//! The connected Stage A uninstall flow: launch → track → verify.

use crate::launch::{run_uninstaller, LaunchError, TrackEvent};
use crate::registry::WinReg;
use crate::system::{msi_product_state, new_renames_under, pending_renames};
use std::sync::atomic::AtomicBool;
use sweep_core::api::RegRead;
use sweep_core::evidence::EvidenceSnapshot;
use sweep_core::launch::LaunchPlan;
use sweep_core::outcome::{classify_exe, classify_msi, Event, Postconditions, UninstallState};

fn step(st: &mut UninstallState, ev: Event, on_state: &mut impl FnMut(&UninstallState)) {
    if let Ok(next) = st.clone().next(ev) {
        *st = next;
        on_state(st);
    }
}

pub fn postconditions(
    snap: &EvidenceSnapshot,
    plan: &LaunchPlan,
    before: &[String],
    tracking_incomplete: bool,
) -> Postconditions {
    let after = pending_renames();
    Postconditions {
        entry_still_present: snap.uninstall_keys.iter().any(|k| WinReg.key_exists(k.root, k.view, &k.path)),
        msi_state: if plan.is_msi { snap.product_code.as_deref().map(msi_product_state) } else { None },
        new_pending_renames: new_renames_under(before, &after, snap.install_dir.as_ref().ok()),
        tracking_incomplete,
    }
}

/// Run the vendor uninstaller described by `plan` and return the final state.
/// Sweep performs no cleanup here or anywhere else in Stage A.
pub fn run_vendor_uninstall(
    snap: &EvidenceSnapshot,
    plan: &LaunchPlan,
    stop_waiting: &AtomicBool,
    mut on_state: impl FnMut(&UninstallState),
) -> UninstallState {
    let mut st = UninstallState::Ready;
    step(&mut st, Event::Launch, &mut on_state);
    let before = pending_renames();
    let result = {
        let st_ref = &mut st;
        let on = &mut on_state;
        run_uninstaller(plan, stop_waiting, |ev| match ev {
            TrackEvent::Started { pid } => step(st_ref, Event::Started { pid }, on),
            TrackEvent::Tracked { pids, incomplete } => step(st_ref, Event::Tracked { pids, incomplete }, on),
        })
    };
    match result {
        Err(LaunchError::Win32(code)) => {
            step(&mut st, Event::LaunchError { code }, &mut on_state);
        }
        Ok(res) => {
            if matches!(st, UninstallState::Launching) {
                // No process handle was returned.
                step(&mut st, Event::Started { pid: 0 }, &mut on_state);
            }
            if res.tracking_incomplete {
                step(&mut st, Event::Tracked { pids: vec![], incomplete: true }, &mut on_state);
            }
            step(&mut st, Event::Exited, &mut on_state);
            let post = postconditions(snap, plan, &before, res.tracking_incomplete);
            let outcome =
                if plan.is_msi { classify_msi(res.exit_code, &post) } else { classify_exe(res.exit_code, &post) };
            step(&mut st, Event::Verified(outcome), &mut on_state);
        }
    }
    st
}
