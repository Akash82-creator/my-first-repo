const COMMANDS: &[&str] = &[
    "env_status",
    "list_programs",
    "preview_uninstall",
    "start_uninstall",
    "stop_waiting",
    "get_operation",
    "start_scan",
    "cancel_scan",
    "get_scan",
];

fn main() {
    // Declaring the commands generates one `allow-<command>` permission each;
    // capabilities/main.json grants exactly these and nothing else.
    let attrs = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS))
        .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(include_str!("sweep.exe.manifest")));
    tauri_build::try_build(attrs).expect("tauri-build failed");
}
