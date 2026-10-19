//! Build the visible program list from raw Uninstall registry entries.

use crate::model::{EntrySource, InstalledProgram, RegView, Scope, UninstallEntry};
use crate::msi;
use crate::winpath::fold;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HiddenReason {
    SystemComponent,
    UpdateOrHotfix,
    NoDisplayName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenEntry {
    pub source: EntrySource,
    pub reason: HiddenReason,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub programs: Vec<InstalledProgram>,
    pub hidden: Vec<HiddenEntry>,
    /// Registry locations that could not be read (reported, not silently empty).
    pub unreadable: Vec<String>,
}

impl Inventory {
    pub fn get(&self, id: &str) -> Option<&InstalledProgram> {
        self.programs.iter().find(|p| p.id == id)
    }
}

fn program_id(scope: Scope, view: RegView, key: &str) -> String {
    let s = match scope {
        Scope::Machine => "m",
        Scope::User => "u",
    };
    let v = match view {
        RegView::Reg64 => "64",
        RegView::Reg32 => "32",
        RegView::Shared => "s",
    };
    format!("{s}{v}:{key}")
}

fn hidden_reason(e: &UninstallEntry) -> Option<HiddenReason> {
    if e.u32("SystemComponent") == Some(1) {
        return Some(HiddenReason::SystemComponent);
    }
    if e.str("ParentKeyName").is_some() {
        return Some(HiddenReason::UpdateOrHotfix);
    }
    if let Some(rt) = e.str("ReleaseType") {
        let rt = fold(rt);
        if ["SECURITY UPDATE", "UPDATE ROLLUP", "HOTFIX", "UPDATE", "SERVICE PACK"].contains(&rt.as_str()) {
            return Some(HiddenReason::UpdateOrHotfix);
        }
    }
    if e.str("DisplayName").is_none() {
        return Some(HiddenReason::NoDisplayName);
    }
    None
}

fn to_program(e: UninstallEntry) -> InstalledProgram {
    let windows_installer = e.u32("WindowsInstaller") == Some(1);
    let product_code = msi::parse_product_code(&e.key_name).or_else(|| {
        if windows_installer {
            e.str("UninstallString").and_then(msi::find_product_code)
        } else {
            None
        }
    });
    InstalledProgram {
        id: program_id(e.scope, e.view, &e.key_name),
        display_name: e.str("DisplayName").unwrap_or_default().to_string(),
        publisher: e.str("Publisher").map(str::to_string),
        version: e.str("DisplayVersion").map(str::to_string),
        install_location: e.str("InstallLocation").map(str::to_string),
        uninstall_string: e.str("UninstallString").map(str::to_string),
        quiet_uninstall_string: e.str("QuietUninstallString").map(str::to_string),
        display_icon: e.str("DisplayIcon").map(str::to_string),
        install_date: e.str("InstallDate").map(str::to_string),
        estimated_size_kb: e.u32("EstimatedSize"),
        windows_installer,
        product_code,
        scope: e.scope,
        sources: vec![EntrySource { scope: e.scope, view: e.view, key_name: e.key_name.clone() }],
        entry: e,
    }
}

/// Read every Uninstall entry from HKLM (both views) and HKCU. Locations
/// that cannot be read are returned, not silently treated as empty.
pub fn read_entries(reg: &dyn crate::api::RegRead) -> (Vec<UninstallEntry>, Vec<String>) {
    use crate::model::{RegRoot, UNINSTALL_PATH};
    let mut out = Vec::new();
    let mut unreadable = Vec::new();
    for (root, view, scope) in [
        (RegRoot::Hklm, RegView::Reg64, Scope::Machine),
        (RegRoot::Hklm, RegView::Reg32, Scope::Machine),
        (RegRoot::Hkcu, RegView::Shared, Scope::User),
    ] {
        let label = format!("{root:?} {view:?} {UNINSTALL_PATH}");
        let keys = match reg.subkeys(root, view, UNINSTALL_PATH) {
            Ok(k) => k,
            Err(crate::api::RegError::NotFound) => continue,
            Err(e) => {
                unreadable.push(format!("{label}: {e:?}"));
                continue;
            }
        };
        for k in keys {
            match reg.values(root, view, &format!("{UNINSTALL_PATH}\\{k}")) {
                Ok(values) => out.push(UninstallEntry { scope, view, key_name: k, values }),
                Err(e) => unreadable.push(format!("{label}\\{k}: {e:?}")),
            }
        }
    }
    (out, unreadable)
}

/// Filter and de-duplicate raw entries.
///
/// De-duplication: entries with the same scope and key name in both HKLM
/// views are merged (64-bit view preferred), then entries sharing a
/// ProductCode within the same scope are merged.
pub fn build_inventory(entries: Vec<UninstallEntry>, unreadable: Vec<String>) -> Inventory {
    let mut hidden = Vec::new();
    let mut by_key: BTreeMap<(Scope, String), InstalledProgram> = BTreeMap::new();
    let mut sorted = entries;
    // Deterministic preference: 64-bit view first.
    sorted.sort_by_key(|a| (a.scope, a.view, fold(&a.key_name)));
    for e in sorted {
        if let Some(reason) = hidden_reason(&e) {
            hidden.push(HiddenEntry {
                source: EntrySource { scope: e.scope, view: e.view, key_name: e.key_name.clone() },
                reason,
            });
            continue;
        }
        let k = (e.scope, fold(&e.key_name));
        let source = EntrySource { scope: e.scope, view: e.view, key_name: e.key_name.clone() };
        match by_key.get_mut(&k) {
            Some(existing) => existing.sources.push(source),
            None => {
                by_key.insert(k, to_program(e));
            }
        }
    }
    let mut programs: Vec<InstalledProgram> = Vec::new();
    for p in by_key.into_values() {
        let dup = p.product_code.as_ref().and_then(|code| {
            programs.iter().position(|q| q.scope == p.scope && q.product_code.as_deref() == Some(code.as_str()))
        });
        match dup {
            Some(i) => programs[i].sources.extend(p.sources),
            None => programs.push(p),
        }
    }
    programs.sort_by_key(|a| fold(&a.display_name));
    Inventory { programs, hidden, unreadable }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::RegValue;

    pub fn entry(scope: Scope, view: RegView, key: &str, vals: &[(&str, RegValue)]) -> UninstallEntry {
        UninstallEntry {
            scope,
            view,
            key_name: key.into(),
            values: vals.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        }
    }

    fn sz(s: &str) -> RegValue {
        RegValue::Sz(s.into())
    }

    #[test]
    fn each_view_and_scope_listed_once_with_labels() {
        let inv = build_inventory(
            vec![
                entry(Scope::Machine, RegView::Reg64, "App64", &[("DisplayName", sz("App 64"))]),
                entry(Scope::Machine, RegView::Reg32, "App32", &[("DisplayName", sz("App 32"))]),
                entry(Scope::User, RegView::Shared, "AppUser", &[("DisplayName", sz("App User"))]),
                // Same key in both HKLM views: one program, two sources.
                entry(Scope::Machine, RegView::Reg32, "Both", &[("DisplayName", sz("Both"))]),
                entry(Scope::Machine, RegView::Reg64, "Both", &[("DisplayName", sz("Both"))]),
            ],
            vec![],
        );
        let names: Vec<_> = inv.programs.iter().map(|p| p.display_name.as_str()).collect();
        assert_eq!(names, ["App 32", "App 64", "App User", "Both"]);
        let both = &inv.programs[3];
        assert_eq!(both.sources.len(), 2);
        assert_eq!(both.sources[0].view, RegView::Reg64, "64-bit view preferred");
        assert_eq!(inv.programs[2].scope, Scope::User);
    }

    #[test]
    fn hides_components_updates_and_nameless() {
        let inv = build_inventory(
            vec![
                entry(
                    Scope::Machine,
                    RegView::Reg64,
                    "Sys",
                    &[("DisplayName", sz("Sys")), ("SystemComponent", RegValue::Dword(1))],
                ),
                entry(
                    Scope::Machine,
                    RegView::Reg64,
                    "KB1",
                    &[("DisplayName", sz("KB1")), ("ParentKeyName", sz("Office"))],
                ),
                entry(
                    Scope::Machine,
                    RegView::Reg64,
                    "KB2",
                    &[("DisplayName", sz("KB2")), ("ReleaseType", sz("Security Update"))],
                ),
                entry(Scope::Machine, RegView::Reg64, "Blank", &[("UninstallString", sz("C:\\x.exe"))]),
                entry(
                    Scope::Machine,
                    RegView::Reg64,
                    "Real",
                    &[("DisplayName", sz("Real")), ("SystemComponent", RegValue::Dword(0))],
                ),
            ],
            vec![],
        );
        assert_eq!(inv.programs.len(), 1);
        assert_eq!(inv.hidden.len(), 4);
    }

    #[test]
    fn reads_all_views_and_reports_unreadable() {
        use crate::fakes::FakeReg;
        use crate::model::{RegRoot, UNINSTALL_PATH};
        let mut reg = FakeReg::new();
        reg.key(RegRoot::Hklm, RegView::Reg64, &format!("{UNINSTALL_PATH}\\A"), &[("DisplayName", sz("A"))])
            .key(RegRoot::Hklm, RegView::Reg32, &format!("{UNINSTALL_PATH}\\B"), &[("DisplayName", sz("B"))])
            .key(RegRoot::Hkcu, RegView::Shared, &format!("{UNINSTALL_PATH}\\C"), &[("DisplayName", sz("C"))])
            .key(RegRoot::Hkcu, RegView::Shared, &format!("{UNINSTALL_PATH}\\D"), &[])
            .deny(RegRoot::Hkcu, RegView::Shared, &format!("{UNINSTALL_PATH}\\D"));
        let (entries, unreadable) = read_entries(&reg);
        let names: Vec<_> = entries.iter().map(|e| (e.key_name.as_str(), e.scope, e.view)).collect();
        assert_eq!(
            names,
            [
                ("A", Scope::Machine, RegView::Reg64),
                ("B", Scope::Machine, RegView::Reg32),
                ("C", Scope::User, RegView::Shared)
            ]
        );
        assert_eq!(unreadable.len(), 1);
    }

    #[test]
    fn msi_product_code_dedup_within_scope() {
        let code = "{11111111-2222-3333-4444-555555555555}";
        let inv = build_inventory(
            vec![
                entry(
                    Scope::Machine,
                    RegView::Reg64,
                    code,
                    &[("DisplayName", sz("Tool")), ("WindowsInstaller", RegValue::Dword(1))],
                ),
                entry(
                    Scope::Machine,
                    RegView::Reg32,
                    "ToolAlias",
                    &[
                        ("DisplayName", sz("Tool")),
                        ("WindowsInstaller", RegValue::Dword(1)),
                        ("UninstallString", sz(&format!("MsiExec.exe /X{code}"))),
                    ],
                ),
            ],
            vec![],
        );
        assert_eq!(inv.programs.len(), 1);
        assert_eq!(inv.programs[0].product_code.as_deref(), Some(code));
        assert_eq!(inv.programs[0].sources.len(), 2);
    }
}
