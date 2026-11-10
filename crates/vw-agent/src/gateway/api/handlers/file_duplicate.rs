//! Duplicate a workspace entry by staging a complete copy before publishing it.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use axum::{Extension, Json};
use vw_api_types::common::OperationAck;
use vw_api_types::file::DuplicateFileRequest;
use vw_api_types::id::TenantScope;

use super::file_import::{copy_entry, io_error, publish_no_replace, target_directory};
use super::{ApiError, resolve_file_root};

pub(super) async fn duplicate(
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<DuplicateFileRequest>,
) -> Result<Json<OperationAck>, ApiError> {
    let root = resolve_file_root(
        &tenant,
        &body.project_id.0,
        body.worktree_id.as_ref().map(|id| id.0.as_str()),
    )
    .await?;
    tokio::task::spawn_blocking(move || {
        let root = PathBuf::from(root).canonicalize().map_err(io_error)?;
        duplicate_entry(&root, Path::new(&body.path))
    })
    .await
    .map_err(|_| ApiError::internal("duplicate task failed; refresh before retrying"))??;
    Ok(Json(OperationAck { ok: true, message: None }))
}

fn duplicate_entry(root: &Path, relative: &Path) -> Result<(), ApiError> {
    let invalid_path = || ApiError::bad_request("source must be a relative workspace entry");
    let name = relative.file_name().and_then(|name| name.to_str()).ok_or_else(invalid_path)?;
    let parent = relative.parent().and_then(Path::to_str).ok_or_else(invalid_path)?;
    let parent = target_directory(root, parent)?;
    let source = parent.join(name);
    let metadata = std::fs::symlink_metadata(&source).map_err(io_error)?;
    let name = Path::new(name);
    let (stem, extension) = if metadata.is_file() {
        (
            name.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default(),
            name.extension().and_then(|extension| extension.to_str()),
        )
    } else {
        (name.to_str().unwrap_or_default(), None)
    };

    let staging = tempfile::Builder::new()
        .prefix(".shiju-duplicate-")
        .tempdir_in(&parent)
        .map_err(io_error)?;
    let staged = staging.path().join("content");
    copy_entry(&source, &staged, true)?;
    for index in 1..=999 {
        let mut candidate = format!("{stem} copy");
        if index > 1 {
            candidate.push_str(&format!(" {index}"));
        }
        if let Some(extension) = extension {
            candidate.push('.');
            candidate.push_str(extension);
        }
        let destination = parent.join(candidate);
        match publish_no_replace(&staged, &destination) {
            Ok(()) => return staging.close().map_err(io_error),
            Err(error)
                if error.kind() == ErrorKind::AlreadyExists
                    || destination.symlink_metadata().is_ok() => {}
            Err(error) => return Err(io_error(error)),
        }
    }
    Err(ApiError::conflict("no available name for the duplicate"))
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos", windows)))]
#[path = "file_duplicate_tests.rs"]
mod tests;
