//! Read an executable's embedded manifest without running it
//! (LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE).

use sweep_core::launch::ExecutionLevel;
use sweep_core::winpath::WinPath;
use windows::core::PCWSTR;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::RT_MANIFEST;

pub fn execution_level(exe: &WinPath) -> Option<ExecutionLevel> {
    let path = crate::wide::wide(&crate::wide::extended(exe));
    unsafe {
        let module =
            LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE)
                .ok()?;
        let mut level = None;
        for id in [1usize, 2] {
            let res = FindResourceW(module, PCWSTR(id as *const u16), RT_MANIFEST);
            if res.is_invalid() {
                continue;
            }
            let size = SizeofResource(module, res) as usize;
            let Ok(handle) = LoadResource(module, res) else { continue };
            let ptr = LockResource(handle) as *const u8;
            if ptr.is_null() || size == 0 {
                continue;
            }
            let bytes = std::slice::from_raw_parts(ptr, size);
            level = ExecutionLevel::from_manifest(&String::from_utf8_lossy(bytes));
            if level.is_some() {
                break;
            }
        }
        let _ = FreeLibrary(module);
        level
    }
}
