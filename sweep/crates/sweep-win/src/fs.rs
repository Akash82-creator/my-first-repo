//! Read-only filesystem access. Never follows reparse points: metadata comes
//! from `symlink_metadata` / directory enumeration (FindNextFile data), and
//! reparse entries are reported, not descended.

use std::os::windows::fs::MetadataExt;
use std::time::UNIX_EPOCH;
use sweep_core::api::{DirEntry, EntryMeta, FsError, FsRead};
use sweep_core::winpath::WinPath;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

pub struct WinFs;

fn map_err(e: std::io::Error) -> FsError {
    match e.raw_os_error() {
        Some(2) | Some(3) => FsError::NotFound,
        Some(5) => FsError::AccessDenied,
        _ => FsError::Other(e.to_string()),
    }
}

fn meta(m: &std::fs::Metadata) -> EntryMeta {
    let attrs = m.file_attributes();
    EntryMeta {
        is_dir: attrs & FILE_ATTRIBUTE_DIRECTORY != 0,
        is_reparse: attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0,
        size: m.file_size(),
        modified: m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64),
    }
}

impl FsRead for WinFs {
    fn metadata(&self, path: &WinPath) -> Result<EntryMeta, FsError> {
        std::fs::symlink_metadata(crate::wide::extended(path)).map(|m| meta(&m)).map_err(map_err)
    }

    fn read_dir(&self, path: &WinPath) -> Result<Vec<DirEntry>, FsError> {
        let m = self.metadata(path)?;
        if m.is_reparse {
            return Err(FsError::Other("refusing to list through a reparse point".into()));
        }
        let mut out = Vec::new();
        for e in std::fs::read_dir(crate::wide::extended(path)).map_err(map_err)? {
            let e = e.map_err(map_err)?;
            // DirEntry::metadata on Windows does not traverse reparse points.
            let md = e.metadata().map_err(map_err)?;
            out.push(DirEntry { name: e.file_name().to_string_lossy().into_owned(), meta: meta(&md) });
        }
        Ok(out)
    }
}
