//! 补丁文件系统应用模块，负责将解析后的补丁 hunk 应用到指定根目录下的文件。

use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{AffectedPaths, Error, Hunk, derive_new_contents_from_chunks, parse_patch};

fn resolve_path(root: Option<&Path>, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }
    if let Some(root) = root {
        return root.join(path);
    }
    path
}

/// 应用 hunks to files 变更。
///
/// 返回受影响路径或错误，文件系统写入失败时不会被吞掉。
pub fn apply_hunks_to_files(hunks: &[Hunk], root: Option<&Path>) -> Result<AffectedPaths, Error> {
    if hunks.is_empty() {
        return Err(Error::Parse("No files were modified.".to_string()));
    }

    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for hunk in hunks {
        match hunk {
            Hunk::Add { path, contents } => {
                let full = resolve_path(root, path);
                if let Some(parent) = full.parent()
                    && parent != Path::new("")
                {
                    fs::create_dir_all(parent)?;
                }

                fs::write(&full, contents)?;
                added.push(full.to_string_lossy().to_string());
            }
            Hunk::Delete { path } => {
                let full = resolve_path(root, path);
                fs::remove_file(&full)?;
                deleted.push(full.to_string_lossy().to_string());
            }
            Hunk::Update { path, move_path, chunks } => {
                let full = resolve_path(root, path);
                let update = derive_new_contents_from_chunks(&full, chunks)?;

                if let Some(move_path) = move_path {
                    let target = resolve_path(root, move_path);
                    if let Some(parent) = target.parent()
                        && parent != Path::new("")
                    {
                        fs::create_dir_all(parent)?;
                    }

                    fs::write(&target, update.content)?;
                    fs::remove_file(&full)?;
                    modified.push(target.to_string_lossy().to_string());
                } else {
                    fs::write(&full, update.content)?;
                    modified.push(full.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(AffectedPaths { added, modified, deleted })
}

/// 应用 patch 变更。
///
/// 返回受影响路径或错误，文件系统写入失败时不会被吞掉。
pub fn apply_patch(patch_text: &str, root: Option<&Path>) -> Result<AffectedPaths, Error> {
    let parsed = parse_patch(patch_text)?;
    apply_hunks_to_files(&parsed.hunks, root)
}

/// 生成 changes 预览。
///
/// 该函数只计算预览数据，不直接修改文件系统。
pub fn preview_changes(patch_text: &str, root: Option<&Path>) -> Result<serde_json::Value, Error> {
    let parsed = parse_patch(patch_text)?;
    let mut changes = HashMap::<String, serde_json::Value>::new();

    for hunk in &parsed.hunks {
        match hunk {
            Hunk::Add { path, contents } => {
                let full = resolve_path(root, path);
                changes.insert(
                    full.to_string_lossy().to_string(),
                    json!({ "type": "add", "content": contents }),
                );
            }
            Hunk::Delete { path } => {
                let full = resolve_path(root, path);
                let content = fs::read_to_string(&full)?;
                changes.insert(
                    full.to_string_lossy().to_string(),
                    json!({ "type": "delete", "content": content }),
                );
            }
            Hunk::Update { path, move_path, chunks } => {
                let full = resolve_path(root, path);
                let update = derive_new_contents_from_chunks(&full, chunks)?;
                let resolved = if let Some(move_path) = move_path {
                    resolve_path(root, move_path).to_string_lossy().to_string()
                } else {
                    full.to_string_lossy().to_string()
                };

                changes.insert(
                    resolved,
                    json!({
                        "type": "update",
                        "unified_diff": update.unified_diff,
                        "move_path": move_path.as_ref().map(|path| resolve_path(root, path).to_string_lossy().to_string()),
                        "new_content": update.content
                    }),
                );
            }
        }
    }

    Ok(json!({
        "changes": changes,
        "patch": patch_text,
        "cwd": root.map(|path| path.to_string_lossy().to_string())
    }))
}

#[cfg(test)]
#[path = "filesystem_tests.rs"]
mod filesystem_tests;
