//! CLI 控制命令先按目录与 Agent 定位名称，再保留显式 ID 的原有解析语义。

use crate::{
    FindSessionByDirectoryWalkOptions, SessionRecord, SessionRepositoryError,
    find_git_repository_root, find_session_by_directory_walk, resolve_session_record,
};

pub async fn resolve_session_target(
    agent_command: &str,
    cwd: &str,
    selector: &str,
) -> Result<SessionRecord, SessionRepositoryError> {
    let boundary = find_git_repository_root(cwd)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string());
    if let Some(record) = find_session_by_directory_walk(&FindSessionByDirectoryWalkOptions {
        agent_command: agent_command.to_string(),
        cwd: cwd.to_string(),
        name: Some(selector.to_string()),
        boundary: Some(boundary),
    })
    .await?
    {
        return Ok(record);
    }
    resolve_session_record(selector).await
}
