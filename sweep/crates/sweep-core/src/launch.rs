//! How a vendor uninstaller is launched. Sweep's UI never elevates; when an
//! uninstaller needs admin rights, Windows asks for approval for that
//! process only (its own manifest, or the `runas` verb).

use crate::cmdline::UninstallCommand;
use crate::model::Scope;
use crate::winpath::WinPath;
use serde::{Deserialize, Serialize};

/// `requestedExecutionLevel` from the executable's embedded manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionLevel {
    AsInvoker,
    HighestAvailable,
    RequireAdministrator,
}

impl ExecutionLevel {
    /// Parse from manifest XML text (no full XML parser needed for one attribute).
    pub fn from_manifest(xml: &str) -> Option<Self> {
        let lower = xml.to_ascii_lowercase();
        let at = lower.find("requestedexecutionlevel")?;
        let rest = &lower[at + "requestedexecutionlevel".len()..];
        let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
        // Find an attribute named exactly `level` (preceded by whitespace).
        let mut val = None;
        let mut search = 0;
        while let Some(i) = tag[search..].find("level") {
            let i = search + i;
            search = i + 5;
            if !tag[..i].ends_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let Some(after) = tag[i + 5..].trim_start().strip_prefix('=') else { continue };
            let after = after.trim_start();
            let q = after.chars().next()?;
            if q != '"' && q != '\'' {
                return None;
            }
            val = Some(&after[1..after[1..].find(q)? + 1]);
            break;
        }
        let val = val?;
        match val.trim() {
            "asinvoker" => Some(ExecutionLevel::AsInvoker),
            "highestavailable" => Some(ExecutionLevel::HighestAvailable),
            "requireadministrator" => Some(ExecutionLevel::RequireAdministrator),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verb {
    /// ShellExecuteEx "open": the target's manifest decides elevation.
    Open,
    /// ShellExecuteEx "runas": UAC prompt for this process only.
    RunAs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchPlan {
    pub file: WinPath,
    pub params: String,
    pub verb: Verb,
    pub is_msi: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum LaunchRefusal {
    #[error(
        "Sweep is running elevated; a per-user uninstaller would run with the wrong account. Restart Sweep normally."
    )]
    ElevatedForPerUserProgram,
    #[error("the Windows directory is unknown, so msiexec cannot be located safely")]
    NoSystemDirectory,
}

pub fn plan_launch(
    cmd: &UninstallCommand,
    scope: Scope,
    sweep_elevated: bool,
    manifest_level: Option<ExecutionLevel>,
    system32: Option<&WinPath>,
) -> Result<LaunchPlan, LaunchRefusal> {
    if scope == Scope::User && sweep_elevated {
        return Err(LaunchRefusal::ElevatedForPerUserProgram);
    }
    match cmd {
        UninstallCommand::Msi { product_code } => {
            let sys = system32.ok_or(LaunchRefusal::NoSystemDirectory)?;
            let file = sys.join("msiexec.exe").map_err(|_| LaunchRefusal::NoSystemDirectory)?;
            Ok(LaunchPlan {
                file,
                params: format!("/x {product_code} /norestart"),
                verb: Verb::Open,
                is_msi: true,
                reason: "Windows Installer handles elevation for per-machine products itself.".into(),
            })
        }
        UninstallCommand::Exe { exe, args } => {
            let (verb, reason) = match (scope, manifest_level, sweep_elevated) {
                (Scope::User, Some(ExecutionLevel::RequireAdministrator), _) => (
                    Verb::Open,
                    "Per-user program whose uninstaller requests administrator rights; Windows will prompt.".to_string(),
                ),
                (Scope::User, _, _) => (Verb::Open, "Per-user program: runs as you, without elevation.".to_string()),
                (Scope::Machine, Some(ExecutionLevel::RequireAdministrator | ExecutionLevel::HighestAvailable), _) => (
                    Verb::Open,
                    "The uninstaller's manifest requests elevation; Windows will ask for approval.".to_string(),
                ),
                (Scope::Machine, _, true) => (
                    Verb::Open,
                    "Sweep is already elevated (not recommended); the uninstaller inherits that.".to_string(),
                ),
                (Scope::Machine, level, false) => (
                    Verb::RunAs,
                    format!(
                        "Per-machine program and the uninstaller {}; Windows will ask for administrator approval for the uninstaller only.",
                        match level {
                            Some(ExecutionLevel::AsInvoker) => "declares asInvoker",
                            _ => "has no readable elevation manifest",
                        }
                    ),
                ),
            };
            Ok(LaunchPlan { file: exe.clone(), params: args.clone(), verb, is_msi: false, reason })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wp(s: &str) -> WinPath {
        WinPath::parse_external(s).unwrap()
    }

    #[test]
    fn manifest_level_parsing() {
        let m = r#"<trustInfo><security><requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/></requestedPrivileges>"#;
        assert_eq!(ExecutionLevel::from_manifest(m), Some(ExecutionLevel::RequireAdministrator));
        assert_eq!(
            ExecutionLevel::from_manifest("<requestedExecutionLevel uiAccess='false' level = 'asInvoker' />"),
            Some(ExecutionLevel::AsInvoker)
        );
        assert_eq!(ExecutionLevel::from_manifest("<assembly/>"), None);
    }

    #[test]
    fn launch_matrix() {
        let exe = UninstallCommand::Exe { exe: wp("C:\\P\\u.exe"), args: "/S".into() };
        let sys = wp("C:\\Windows\\System32");
        let p = plan_launch(&exe, Scope::Machine, false, None, Some(&sys)).unwrap();
        assert_eq!(p.verb, Verb::RunAs);
        let p = plan_launch(&exe, Scope::Machine, false, Some(ExecutionLevel::AsInvoker), Some(&sys)).unwrap();
        assert_eq!(p.verb, Verb::RunAs);
        let p =
            plan_launch(&exe, Scope::Machine, false, Some(ExecutionLevel::RequireAdministrator), Some(&sys)).unwrap();
        assert_eq!(p.verb, Verb::Open);
        let p = plan_launch(&exe, Scope::User, false, None, Some(&sys)).unwrap();
        assert_eq!(p.verb, Verb::Open);
        assert_eq!(p.params, "/S");
        assert_eq!(
            plan_launch(&exe, Scope::User, true, None, Some(&sys)),
            Err(LaunchRefusal::ElevatedForPerUserProgram)
        );

        let msi = UninstallCommand::Msi { product_code: "{A}".into() };
        let p = plan_launch(&msi, Scope::Machine, false, None, Some(&sys)).unwrap();
        assert_eq!(p.file, wp("C:\\Windows\\System32\\msiexec.exe"));
        assert_eq!(p.params, "/x {A} /norestart");
        assert_eq!(plan_launch(&msi, Scope::Machine, false, None, None), Err(LaunchRefusal::NoSystemDirectory));
    }
}
