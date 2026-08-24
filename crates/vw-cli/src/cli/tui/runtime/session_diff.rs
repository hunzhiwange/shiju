//! TUI 会话文件差异的只读 Gateway 适配边界。

use similar::TextDiff;
use vw_gateway_client::vw_api_types::session::GatewaySessionDiffQuery;
use vw_shared::snapshot::{DiffStatus, FileDiff};
use vw_tui::{V3FileChange, V3FileDiff, V3FileDiffEntry};

use super::gateway::{GatewayUiRuntime, block_on_gateway};

pub(crate) fn load_session_diff_blocking(
    runtime: &GatewayUiRuntime,
) -> Result<V3FileDiff, String> {
    block_on_gateway(async {
        let session_id = runtime.resolve_session_id(None)?.to_string();
        let directory = runtime
            .directory()
            .to_str()
            .map(str::trim)
            .filter(|directory| !directory.is_empty())
            .ok_or_else(|| {
                "gateway runtime directory is required and must be valid UTF-8".to_string()
            })?;
        let query = GatewaySessionDiffQuery {
            directory: Some(directory.to_string()),
            message_id: None,
        };
        let files = runtime.client().session_diff::<Vec<FileDiff>>(&session_id, &query).await?;
        Ok(map_session_diff(files))
    })
}

fn map_session_diff(files: Vec<FileDiff>) -> V3FileDiff {
    V3FileDiff { files: files.into_iter().map(map_file_diff).collect() }
}

fn map_file_diff(file: FileDiff) -> V3FileDiffEntry {
    let change = match file.status {
        Some(DiffStatus::Added) => V3FileChange::Add { content: file.after },
        Some(DiffStatus::Deleted) => V3FileChange::Delete { content: file.before },
        None if file.before.is_empty() && !file.after.is_empty() => {
            V3FileChange::Add { content: file.after }
        }
        None if !file.before.is_empty() && file.after.is_empty() => {
            V3FileChange::Delete { content: file.before }
        }
        Some(DiffStatus::Modified) | None => {
            modified_change(&file.file, &file.before, &file.after)
        }
    };
    V3FileDiffEntry { path: file.file, change }
}

fn modified_change(path: &str, before: &str, after: &str) -> V3FileChange {
    let unified_diff = TextDiff::from_lines(before, after)
        .unified_diff()
        .header(path, path)
        .to_string();
    V3FileChange::Update { unified_diff }
}
