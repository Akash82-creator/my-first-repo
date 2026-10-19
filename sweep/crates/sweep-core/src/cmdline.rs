//! Parse `UninstallString` values into something Sweep can launch safely.
//!
//! Rules (see docs/SPEC.md):
//! * `msiexec /x|/i {GUID}` → [`UninstallCommand::Msi`].
//! * Quoted executable: inside the quotes is the exe; the remainder (leading
//!   whitespace removed) is the exact argument tail.
//! * Unquoted, first token ends in `.exe`: that token is the exe.
//! * Unquoted with spaces in the exe path: candidates are prefixes at space
//!   boundaries that *end in `.exe`*. Exactly one must exist **and** sit in an
//!   independently validated anchor directory (InstallLocation or the
//!   DisplayIcon directory). Otherwise the command is rejected as ambiguous.
//!   This never resolves `C:\Program Files\X\u.exe` to `C:\Program.exe`.
//! * Bare `rundll32` maps to `%SystemRoot%\System32\rundll32.exe`; other bare
//!   or relative names are rejected.
//! * Shells and script hosts are rejected and never run.

use crate::msi;
use crate::winpath::{eq_ignore_case, PathError, WinPath};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UninstallCommand {
    Msi { product_code: String },
    Exe { exe: WinPath, args: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "reason", content = "detail")]
pub enum Unsupported {
    #[error("no uninstall command is registered")]
    Empty,
    #[error("unterminated quote in command")]
    UnterminatedQuote,
    #[error("environment variable %{0}% could not be resolved")]
    UnresolvedVariable(String),
    #[error("msiexec command without a product code")]
    MsiWithoutProductCode,
    #[error("ambiguous unquoted path; candidates: {0:?}")]
    Ambiguous(Vec<String>),
    #[error("relative or bare program name: {0}")]
    Relative(String),
    #[error("shell or script host is not run by Sweep: {0}")]
    ScriptOrShell(String),
    #[error("target is not an .exe: {0}")]
    NotExe(String),
    #[error("executable not found: {0}")]
    NotFound(String),
    #[error("invalid path: {0}")]
    BadPath(PathError),
}

pub struct ParseContext<'a> {
    /// Environment lookup (case-insensitive names on Windows).
    pub env: &'a dyn Fn(&str) -> Option<String>,
    /// True when `path` is an existing regular file (not a reparse point).
    pub file_exists: &'a dyn Fn(&WinPath) -> bool,
    /// Independently validated directories that may disambiguate unquoted paths.
    pub anchors: &'a [WinPath],
    /// `%SystemRoot%`, from Known Folders.
    pub system_root: Option<&'a WinPath>,
}

const SHELLS: &[&str] = &[
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "wscript.exe",
    "cscript.exe",
    "mshta.exe",
    "bash.exe",
    "wsl.exe",
    "conhost.exe",
    "explorer.exe",
];
const SCRIPT_EXTS: &[&str] = &["bat", "cmd", "ps1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "lnk", "msc"];

/// Expand `%VAR%` references. `%%` and unmatched `%` are kept literally.
pub fn expand_env(s: &str, env: &dyn Fn(&str) -> Option<String>) -> Result<String, Unsupported> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(0) => {
                out.push('%');
                rest = &after[1..];
            }
            Some(end) => {
                let name = &after[..end];
                if name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')' | ' ' | '-')) {
                    match env(name) {
                        Some(v) => out.push_str(&v),
                        None => return Err(Unsupported::UnresolvedVariable(name.to_string())),
                    }
                    rest = &after[end + 1..];
                } else {
                    out.push('%');
                    rest = after;
                }
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

fn file_name_of(token: &str) -> &str {
    token.rsplit(['\\', '/']).next().unwrap_or(token)
}

fn is_msiexec(token: &str) -> bool {
    let n = file_name_of(token.trim_matches('"'));
    eq_ignore_case(n, "msiexec") || eq_ignore_case(n, "msiexec.exe")
}

fn ends_with_exe(s: &str) -> bool {
    s.len() >= 4 && s.is_char_boundary(s.len() - 4) && eq_ignore_case(&s[s.len() - 4..], ".exe")
}

fn check_exe(exe: WinPath, ctx: &ParseContext) -> Result<WinPath, Unsupported> {
    let name = exe.file_name().unwrap_or_default().to_string();
    if SHELLS.iter().any(|s| eq_ignore_case(s, &name)) {
        return Err(Unsupported::ScriptOrShell(exe.to_string()));
    }
    if SCRIPT_EXTS.iter().any(|e| exe.extension_is(e)) {
        return Err(Unsupported::ScriptOrShell(exe.to_string()));
    }
    if !exe.extension_is("exe") {
        return Err(Unsupported::NotExe(exe.to_string()));
    }
    if !(ctx.file_exists)(&exe) {
        return Err(Unsupported::NotFound(exe.to_string()));
    }
    Ok(exe)
}

fn resolve_token(token: &str, ctx: &ParseContext) -> Result<WinPath, Unsupported> {
    let looks_bare = !token.contains('\\') && !token.contains('/') && !token.contains(':');
    if looks_bare {
        let n = token;
        if eq_ignore_case(n, "rundll32") || eq_ignore_case(n, "rundll32.exe") {
            let root = ctx.system_root.ok_or_else(|| Unsupported::Relative(token.to_string()))?;
            let p = root.join("System32").and_then(|p| p.join("rundll32.exe")).map_err(Unsupported::BadPath)?;
            return check_exe(p, ctx);
        }
        if SHELLS.iter().any(|s| eq_ignore_case(s, n) || eq_ignore_case(&s[..s.len() - 4], n)) {
            return Err(Unsupported::ScriptOrShell(token.to_string()));
        }
        return Err(Unsupported::Relative(token.to_string()));
    }
    let p = WinPath::parse_external(token).map_err(|e| match e {
        PathError::Relative | PathError::DriveRelative | PathError::RootedRelative => {
            Unsupported::Relative(token.to_string())
        }
        other => Unsupported::BadPath(other),
    })?;
    check_exe(p, ctx)
}

pub fn parse_uninstall_command(
    raw: &str,
    windows_installer: bool,
    key_name: &str,
    ctx: &ParseContext,
) -> Result<UninstallCommand, Unsupported> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Unsupported::Empty);
    }
    let s = expand_env(trimmed, ctx.env)?;
    let s = s.trim();

    // First token, honouring a leading quote.
    let (first, tail, quoted) = if let Some(r) = s.strip_prefix('"') {
        let end = r.find('"').ok_or(Unsupported::UnterminatedQuote)?;
        (&r[..end], r[end + 1..].trim_start(), true)
    } else {
        match s.find(char::is_whitespace) {
            Some(i) => (&s[..i], s[i..].trim_start(), false),
            None => (s, "", false),
        }
    };

    if is_msiexec(first) {
        let code = msi::find_product_code(tail)
            .or_else(|| if windows_installer { msi::parse_product_code(key_name) } else { None })
            .ok_or(Unsupported::MsiWithoutProductCode)?;
        return Ok(UninstallCommand::Msi { product_code: code });
    }

    let bare = !first.contains(['\\', '/', ':']);
    if quoted || bare || ends_with_exe(first) || tail.is_empty() {
        let exe = resolve_token(first, ctx)?;
        return Ok(UninstallCommand::Exe { exe, args: tail.to_string() });
    }

    // Unquoted path containing spaces: only resolve with anchor validation.
    let mut candidates: Vec<(WinPath, String)> = Vec::new();
    let mut listed: Vec<String> = Vec::new();
    for (i, c) in s.char_indices() {
        let boundary = c.is_whitespace();
        let prefix = if boundary {
            &s[..i]
        } else if i + c.len_utf8() == s.len() {
            s
        } else {
            continue;
        };
        if !ends_with_exe(prefix) {
            continue;
        }
        listed.push(prefix.to_string());
        if let Ok(p) = WinPath::parse_external(prefix) {
            let anchored = p.parent().map(|d| ctx.anchors.contains(&d)).unwrap_or(false);
            if anchored && (ctx.file_exists)(&p) {
                candidates.push((p, s[prefix.len()..].trim_start().to_string()));
            }
        }
    }
    if listed.is_empty() {
        return Err(Unsupported::Relative(first.to_string()));
    }
    if candidates.len() != 1 {
        return Err(Unsupported::Ambiguous(listed));
    }
    let (exe, args) = candidates.pop().unwrap();
    let exe = check_exe(exe, ctx)?;
    Ok(UninstallCommand::Exe { exe, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> WinPath {
        WinPath::parse_external(s).unwrap()
    }

    struct Env {
        files: Vec<WinPath>,
        anchors: Vec<WinPath>,
        root: WinPath,
    }

    impl Env {
        fn new(files: &[&str], anchors: &[&str]) -> Self {
            Env {
                files: files.iter().map(|f| p(f)).collect(),
                anchors: anchors.iter().map(|f| p(f)).collect(),
                root: p("C:\\Windows"),
            }
        }
        fn parse(&self, raw: &str) -> Result<UninstallCommand, Unsupported> {
            let env = |n: &str| {
                if eq_ignore_case(n, "ProgramFiles") {
                    Some("C:\\Program Files".to_string())
                } else {
                    None
                }
            };
            let exists = |w: &WinPath| self.files.contains(w);
            let ctx =
                ParseContext { env: &env, file_exists: &exists, anchors: &self.anchors, system_root: Some(&self.root) };
            parse_uninstall_command(raw, false, "Key", &ctx)
        }
    }

    fn exe(path: &str, args: &str) -> UninstallCommand {
        UninstallCommand::Exe { exe: p(path), args: args.into() }
    }

    #[test]
    fn quoted_exe_preserves_argument_tail() {
        let e = Env::new(&["C:\\Program Files\\Acme App\\unins000.exe"], &[]);
        assert_eq!(
            e.parse("\"C:\\Program Files\\Acme App\\unins000.exe\"   /SILENT  \"/LOG=C:\\a b.txt\""),
            Ok(exe("C:\\Program Files\\Acme App\\unins000.exe", "/SILENT  \"/LOG=C:\\a b.txt\""))
        );
    }

    #[test]
    fn program_exe_prefix_rejected() {
        // C:\Program.exe exists, but "C:\Program" does not end in .exe so it is
        // never a candidate. Without a validated anchor the real path is
        // ambiguous and rejected.
        let e = Env::new(&["C:\\Program.exe", "C:\\Program Files\\Vendor\\uninstall.exe"], &[]);
        let r = e.parse("C:\\Program Files\\Vendor\\uninstall.exe /S");
        assert_eq!(r, Err(Unsupported::Ambiguous(vec!["C:\\Program Files\\Vendor\\uninstall.exe".into()])));
        // With the InstallLocation anchor it resolves correctly, tail intact.
        let e =
            Env::new(&["C:\\Program.exe", "C:\\Program Files\\Vendor\\uninstall.exe"], &["C:\\Program Files\\Vendor"]);
        assert_eq!(
            e.parse("C:\\Program Files\\Vendor\\uninstall.exe /S /x=\"a b\""),
            Ok(exe("C:\\Program Files\\Vendor\\uninstall.exe", "/S /x=\"a b\""))
        );
    }

    #[test]
    fn two_anchored_candidates_are_ambiguous() {
        let e = Env::new(
            &["C:\\Apps\\My Tool.exe", "C:\\Apps\\My Tool.exe x\\u.exe"],
            &["C:\\Apps", "C:\\Apps\\My Tool.exe x"],
        );
        assert!(matches!(e.parse("C:\\Apps\\My Tool.exe x\\u.exe /q"), Err(Unsupported::Ambiguous(_))));
    }

    #[test]
    fn unquoted_without_spaces() {
        let e = Env::new(&["C:\\Tools\\uninst.exe"], &[]);
        assert_eq!(e.parse("C:\\Tools\\uninst.exe /S"), Ok(exe("C:\\Tools\\uninst.exe", "/S")));
        assert_eq!(e.parse("C:\\Tools\\uninst.exe"), Ok(exe("C:\\Tools\\uninst.exe", "")));
    }

    #[test]
    fn msiexec_forms() {
        let e = Env::new(&[], &[]);
        let code = "{23170F69-40C1-2702-2301-000001000000}";
        for raw in [
            "MsiExec.exe /X{23170F69-40C1-2702-2301-000001000000}",
            "msiexec /i {23170f69-40c1-2702-2301-000001000000}",
            "\"C:\\Windows\\System32\\msiexec.exe\" /x{23170F69-40C1-2702-2301-000001000000} /qb",
        ] {
            assert_eq!(e.parse(raw), Ok(UninstallCommand::Msi { product_code: code.into() }), "{raw}");
        }
        assert_eq!(e.parse("MsiExec.exe /X"), Err(Unsupported::MsiWithoutProductCode));
    }

    #[test]
    fn env_vars_expand_or_reject() {
        let e = Env::new(&["C:\\Program Files\\Acme\\u.exe"], &[]);
        assert_eq!(e.parse("\"%ProgramFiles%\\Acme\\u.exe\" /q"), Ok(exe("C:\\Program Files\\Acme\\u.exe", "/q")));
        assert_eq!(e.parse("%NOPE%\\u.exe"), Err(Unsupported::UnresolvedVariable("NOPE".into())));
        assert_eq!(expand_env("100%% done", &|_| None).unwrap(), "100% done");
    }

    #[test]
    fn shells_scripts_and_relative_rejected() {
        let e = Env::new(&["C:\\Windows\\System32\\cmd.exe", "C:\\x\\remove.bat"], &[]);
        assert!(matches!(e.parse("cmd /c del C:\\x"), Err(Unsupported::ScriptOrShell(_))));
        assert!(matches!(e.parse("\"C:\\Windows\\System32\\cmd.exe\" /c x"), Err(Unsupported::ScriptOrShell(_))));
        assert!(matches!(e.parse("\"C:\\x\\remove.bat\""), Err(Unsupported::ScriptOrShell(_))));
        assert!(matches!(e.parse("powershell -File x.ps1"), Err(Unsupported::ScriptOrShell(_))));
        assert!(matches!(e.parse("uninstall.exe /S"), Err(Unsupported::Relative(_))));
        assert!(matches!(e.parse("\"C:\\x\\missing.exe\""), Err(Unsupported::NotFound(_))));
        assert!(matches!(e.parse("\"\\\\server\\share\\u.exe\""), Err(Unsupported::BadPath(PathError::UncOrDevice))));
        assert_eq!(e.parse("   "), Err(Unsupported::Empty));
        assert_eq!(e.parse("\"C:\\x\\u.exe"), Err(Unsupported::UnterminatedQuote));
    }

    #[test]
    fn rundll32_maps_to_system32() {
        let e = Env::new(&["C:\\Windows\\System32\\rundll32.exe"], &[]);
        assert_eq!(
            e.parse("RunDll32 C:\\PROGRA~1\\X\\setup.dll,Remove"),
            Ok(exe("C:\\Windows\\System32\\rundll32.exe", "C:\\PROGRA~1\\X\\setup.dll,Remove"))
        );
    }

    proptest::proptest! {
        #[test]
        fn parser_never_panics(s in "\\PC{0,60}") {
            let e = Env::new(&["C:\\a b\\c.exe"], &["C:\\a b"]);
            let _ = e.parse(&s);
        }
    }
}
