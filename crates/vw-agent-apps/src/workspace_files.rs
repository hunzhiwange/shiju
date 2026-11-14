//! OpenClaw's nine workspace files, on the already-authorized execution host.
//! Callers supply the native resolver's workspace root and authorize each action
//! before calling these blocking IO functions. No HOME or local-path fallback.
use std::path::Path;

use crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory};
use vw_api_types::workspace_memory::WorkspaceFilename;

/// Missing directories/files return None; empty files remain Some("").
pub fn read_workspace_file(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    workspace_root: &Path,
    filename: WorkspaceFilename,
) -> Result<Option<String>, PathError> {
    let Some(directory) = WorkspaceDirectory::open(
        authorized_host,
        executor_host,
        home,
        workspace_root,
        WorkspaceArea::OpenClawWorkspace,
        false,
    )?
    else {
        return Ok(None);
    };
    directory.read(filename.as_str())
}

/// Creates missing parents within the authorized HOME, then atomically replaces
/// one private file under the shared physical write lock. Preparation/rename
/// failures preserve the destination. Content is never included in errors.
pub fn write_workspace_file(
    authorized_host: &str,
    executor_host: &str,
    home: &Path,
    workspace_root: &Path,
    filename: WorkspaceFilename,
    content: &str,
) -> Result<(), PathError> {
    let directory = WorkspaceDirectory::open(
        authorized_host,
        executor_host,
        home,
        workspace_root,
        WorkspaceArea::OpenClawWorkspace,
        true,
    )?
    .ok_or(PathError::Io)?;
    directory.write(filename.as_str(), content)
}
