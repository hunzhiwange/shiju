use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// 将 scope 转为稳定且可用作路径片段的键名。
pub fn session_scope_key(scope: &str) -> String {
    if !scope.is_empty()
        && scope.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    {
        return scope.to_string();
    }
    let mut hasher = DefaultHasher::new();
    scope.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 返回指定 scope 对应的会话目录。
pub fn sessions_dir_for_scope(data_dir: &Path, scope: Option<&str>) -> Option<PathBuf> {
    let base = data_dir.join("storage").join("session");
    Some(match scope {
        Some(scope) => base.join("scoped").join(session_scope_key(scope)),
        None => base,
    })
}

#[cfg(target_arch = "wasm32")]
/// 在 WASM 场景下返回单个会话 JSON 文件路径。
pub fn session_file_path(data_dir: &Path, id: &str, scope: Option<&str>) -> Option<PathBuf> {
    Some(sessions_dir_for_scope(data_dir, scope)?.join(format!("session-{id}.json")))
}

/// 返回会话步骤快照文件路径。
pub fn session_step_snapshot_file_path(
    data_dir: &Path,
    session_id: &str,
    step_index: u32,
    kind: &str,
    scope: Option<&str>,
) -> Option<PathBuf> {
    Some(
        sessions_dir_for_scope(data_dir, scope)?
            .join("snapshots")
            .join(format!("session-{session_id}-step-{step_index}-{kind}.json")),
    )
}

/// 返回会话步骤原始 LLM 输出文件路径。
pub fn session_step_llm_raw_file_path(
    data_dir: &Path,
    session_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> Option<PathBuf> {
    session_step_llm_raw_file_path_scoped(data_dir, session_id, step_index, scope)
}

/// 返回带 scope 的会话步骤原始 LLM 输出文件路径。
pub fn session_step_llm_raw_file_path_scoped(
    data_dir: &Path,
    session_id: &str,
    step_index: u32,
    scope: Option<&str>,
) -> Option<PathBuf> {
    Some(
        sessions_dir_for_scope(data_dir, scope)?
            .join("snapshots")
            .join(format!("session-{session_id}-step-{step_index}-llm_raw.json")),
    )
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod path_tests;
