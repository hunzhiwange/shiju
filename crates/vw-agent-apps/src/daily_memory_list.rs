//! Daily memory metadata on the authorized executor, never the desktop HOME.
use std::{io::Read, path::Path, time::UNIX_EPOCH};

use crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory};
use vw_api_types::workspace_memory::{DailyMemoryFileInfo, DailyMemoryFilename};

/// The caller must authorize content reading: previews contain memory content.
/// Missing directories are empty; actual enumeration/read errors are visible.
pub fn list_daily_memory_files(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    workspace_root: &Path,
) -> Result<Vec<DailyMemoryFileInfo>, PathError> {
    let Some(directory) = WorkspaceDirectory::open(
        authorized_host,
        executor_host,
        home,
        workspace_root,
        WorkspaceArea::OpenClawMemory,
        false,
    )?
    else {
        return Ok(Vec::new());
    };
    let mut files = Vec::new();
    for name in directory.filenames()? {
        let Ok(filename) = name.parse::<DailyMemoryFilename>() else { continue };
        let Some(mut file) = directory.regular_file(&name)? else { continue };
        let metadata = file.metadata().map_err(|_| PathError::Io)?;
        let modified_at = metadata
            .modified()
            .map_err(|_| PathError::Io)?
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|_| PathError::Io)?;
        files.push(DailyMemoryFileInfo {
            date: filename.date().to_owned(),
            filename,
            size_bytes: metadata.len(),
            modified_at,
            preview: content.chars().take(200).collect(),
        });
    }
    files.sort_by(|a, b| b.filename.as_str().cmp(a.filename.as_str()));
    Ok(files)
}
