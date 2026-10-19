//! `sweep-fixture`: a controlled fake application for Windows tests.
//!
//! install   --name N --publisher P --scope user|machine [--mode M] [--junction SENTINEL_DIR]
//! uninstall --name N --publisher P --scope user|machine --mode ok|fail|leave-entry|spawn-child
//!
//! Known artifacts written by `install` (user scope shown):
//!   %LOCALAPPDATA%\Programs\<P>\<N>\{app.dll, uninstall.exe}
//!   %APPDATA%\<P>\<N>\settings.ini
//!   HKCU\Software\<P>\<N>
//!   HKCU\...\Uninstall\SweepFixture_<N>
//! Machine scope uses %ProgramFiles% and HKLM (64-bit view) instead.
//!
//! `uninstall --mode ok` removes everything except the running uninstall.exe
//! and the settings file, leaving deterministic leftovers for the scanner.

#[cfg(not(windows))]
fn main() {
    eprintln!("sweep-fixture only runs on Windows");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(win::main(&args));
}

#[cfg(windows)]
mod win {
    use std::path::PathBuf;
    use winreg::enums::*;
    use winreg::RegKey;

    const UNINSTALL: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

    struct Opts {
        name: String,
        publisher: String,
        machine: bool,
        mode: String,
        junction: Option<String>,
    }

    fn arg(args: &[String], k: &str) -> Option<String> {
        args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned()
    }

    fn opts(args: &[String]) -> Opts {
        Opts {
            name: arg(args, "--name").expect("--name"),
            publisher: arg(args, "--publisher").expect("--publisher"),
            machine: arg(args, "--scope").as_deref() == Some("machine"),
            mode: arg(args, "--mode").unwrap_or_else(|| "ok".into()),
            junction: arg(args, "--junction"),
        }
    }

    fn base(machine: bool, var: &str) -> PathBuf {
        PathBuf::from(std::env::var(if machine { "ProgramFiles" } else { var }).expect("env"))
    }

    fn install_dir(o: &Opts) -> PathBuf {
        let root = if o.machine { base(true, "") } else { base(false, "LOCALAPPDATA").join("Programs") };
        root.join(&o.publisher).join(&o.name)
    }

    fn data_dir(o: &Opts) -> PathBuf {
        base(false, "APPDATA").join(&o.publisher).join(&o.name)
    }

    fn hive(o: &Opts) -> (RegKey, u32) {
        if o.machine {
            (RegKey::predef(HKEY_LOCAL_MACHINE), KEY_WOW64_64KEY)
        } else {
            (RegKey::predef(HKEY_CURRENT_USER), 0)
        }
    }

    fn entry_path(o: &Opts) -> String {
        format!("{UNINSTALL}\\SweepFixture_{}", o.name)
    }

    pub fn main(args: &[String]) -> i32 {
        match args.get(1).map(String::as_str) {
            Some("install") => install(&opts(args)),
            Some("uninstall") => uninstall(&opts(args), args),
            _ => {
                eprintln!("usage: sweep-fixture install|uninstall ...");
                2
            }
        }
    }

    fn install(o: &Opts) -> i32 {
        let dir = install_dir(o);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.dll"), b"fixture").unwrap();
        let unins = dir.join("uninstall.exe");
        std::fs::copy(std::env::current_exe().unwrap(), &unins).unwrap();
        let data = data_dir(o);
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("settings.ini"), b"[x]\n").unwrap();
        if let Some(sentinel) = &o.junction {
            let link = dir.join("linked");
            let st = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J", link.to_str().unwrap(), sentinel])
                .status()
                .unwrap();
            assert!(st.success(), "mklink /J failed");
        }
        let (h, flag) = hive(o);
        let (k, _) = h.create_subkey_with_flags(entry_path(o), KEY_WRITE | flag).unwrap();
        let scope = if o.machine { "machine" } else { "user" };
        k.set_value("DisplayName", &o.name).unwrap();
        k.set_value("Publisher", &o.publisher).unwrap();
        k.set_value("InstallLocation", &dir.to_string_lossy().to_string()).unwrap();
        k.set_value("DisplayIcon", &format!("{},0", unins.display())).unwrap();
        k.set_value("InstallDate", &today()).unwrap();
        k.set_value(
            "UninstallString",
            &format!(
                "\"{}\" uninstall --name \"{}\" --publisher \"{}\" --scope {scope} --mode {}",
                unins.display(),
                o.name,
                o.publisher,
                o.mode
            ),
        )
        .unwrap();
        let (sk, _) =
            RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!("Software\\{}\\{}", o.publisher, o.name)).unwrap();
        sk.set_value("Fixture", &1u32).unwrap();
        0
    }

    /// Today's date as YYYYMMDD (UTC), like real installers write.
    fn today() -> String {
        let days =
            (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 86_400) as i64;
        // Inverse of days-from-civil (Howard Hinnant).
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        format!("{y:04}{m:02}{d:02}")
    }

    fn remove_installed(o: &Opts) {
        let dir = install_dir(o);
        let _ = std::fs::remove_file(dir.join("app.dll"));
        let (h, flag) = hive(o);
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(format!("Software\\{}\\{}", o.publisher, o.name));
        let _ = h
            .open_subkey_with_flags(UNINSTALL, KEY_ALL_ACCESS | flag)
            .and_then(|u| u.delete_subkey_all(format!("SweepFixture_{}", o.name)));
    }

    fn uninstall(o: &Opts, args: &[String]) -> i32 {
        match o.mode.as_str() {
            "ok" => {
                remove_installed(o);
                // Leave the Software key for the scanner to find.
                let _ =
                    RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!("Software\\{}\\{}", o.publisher, o.name));
                0
            }
            "fail" => 3,
            "leave-entry" => {
                let _ = std::fs::remove_file(install_dir(o).join("app.dll"));
                0
            }
            "spawn-child" => {
                // Inno/NSIS style: copy to %TEMP%, start the copy, exit at once.
                let tmp = std::env::temp_dir().join(format!("sweep-fixture-{}.exe", std::process::id()));
                std::fs::copy(std::env::current_exe().unwrap(), &tmp).unwrap();
                let mut child_args: Vec<String> = args[1..].to_vec();
                if let Some(i) = child_args.iter().position(|a| a == "--mode") {
                    child_args[i + 1] = "slow-ok".into();
                }
                std::process::Command::new(&tmp).args(&child_args).spawn().unwrap();
                0
            }
            "slow-ok" => {
                std::thread::sleep(std::time::Duration::from_secs(2));
                remove_installed(o);
                0
            }
            _ => 2,
        }
    }
}
