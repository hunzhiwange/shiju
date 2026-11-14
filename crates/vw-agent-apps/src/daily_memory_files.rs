//! Daily memory IO on an authorized executor. The caller supplies the resolved
//! OpenClaw workspace root after checking content read/write permissions.
use std::path::Path;

use crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory};
use vw_api_types::workspace_memory::DailyMemoryFilename;

/// Missing directories/files return None without creating anything. Invalid
/// dates cannot reach IO: DailyMemoryFilename validates the Gregorian calendar.
pub fn read_daily_memory_file(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    workspace_root: &Path,
    filename: &DailyMemoryFilename,
) -> Result<Option<String>, PathError> {
    let Some(directory) = WorkspaceDirectory::open(
        authorized_host,
        executor_host,
        home,
        workspace_root,
        WorkspaceArea::OpenClawMemory,
        false,
    )?
    else {
        return Ok(None);
    };
    directory.read(filename.as_str())
}

/// Creates the memory directory only after the caller has authorized writing.
/// Uses the shared physical write lock and private atomic replacement; failed
/// preparation or rename leaves the previous content intact.
pub fn write_daily_memory_file(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    workspace_root: &Path,
    filename: &DailyMemoryFilename,
    content: &str,
) -> Result<(), PathError> {
    let directory = WorkspaceDirectory::open(
        authorized_host,
        executor_host,
        home,
        workspace_root,
        WorkspaceArea::OpenClawMemory,
        true,
    )?
    .ok_or(PathError::Io)?;
    directory.write(filename.as_str(), content)
}
