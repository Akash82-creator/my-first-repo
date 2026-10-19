//! Known Folders via SHGetKnownFolderPath for the current user.

use sweep_core::api::KnownFolders;
use sweep_core::winpath::WinPath;
use windows::core::GUID;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::*;

fn get(id: &GUID) -> Option<WinPath> {
    unsafe {
        let p = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        // Network-redirected folders (UNC) are unsupported and yield None.
        WinPath::from_api(&s?).ok()
    }
}

pub fn known_folders() -> KnownFolders {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        .and_then(|s| WinPath::from_api(&s).ok());
    KnownFolders {
        windows: get(&FOLDERID_Windows),
        system32: get(&FOLDERID_System),
        syswow64: get(&FOLDERID_SystemX86),
        program_files: get(&FOLDERID_ProgramFilesX64).or_else(|| get(&FOLDERID_ProgramFiles)),
        program_files_x86: get(&FOLDERID_ProgramFilesX86),
        program_files_common: get(&FOLDERID_ProgramFilesCommonX64).or_else(|| get(&FOLDERID_ProgramFilesCommon)),
        program_files_common_x86: get(&FOLDERID_ProgramFilesCommonX86),
        program_data: get(&FOLDERID_ProgramData),
        users: get(&FOLDERID_UserProfiles),
        user_profile: get(&FOLDERID_Profile),
        public: get(&FOLDERID_Public),
        roaming_app_data: get(&FOLDERID_RoamingAppData),
        local_app_data: get(&FOLDERID_LocalAppData),
        local_programs: get(&FOLDERID_UserProgramFiles),
        start_menu: get(&FOLDERID_StartMenu),
        common_start_menu: get(&FOLDERID_CommonStartMenu),
        start_menu_programs: get(&FOLDERID_Programs),
        common_start_menu_programs: get(&FOLDERID_CommonPrograms),
        desktop: get(&FOLDERID_Desktop),
        common_desktop: get(&FOLDERID_PublicDesktop),
        documents: get(&FOLDERID_Documents),
        pictures: get(&FOLDERID_Pictures),
        music: get(&FOLDERID_Music),
        videos: get(&FOLDERID_Videos),
        downloads: get(&FOLDERID_Downloads),
        one_drive: get(&FOLDERID_SkyDrive),
        temp: WinPath::from_api(&std::env::temp_dir().to_string_lossy()).ok(),
        sweep_dirs: exe_dir.into_iter().collect(),
    }
}
