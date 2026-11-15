use std::{collections::HashSet, path::{Component, Path}};

pub fn is_directory_path(path: &str, files: &[String]) -> bool {
    let prefix = format!("{path}/");
    if files.iter().any(|item| item.starts_with(&prefix)) {
        return true;
    }
    !files.iter().any(|item| item == path)
}

pub fn remove_local_file_tree_path(files: &mut Vec<String>, path: &str, recursive: bool) {
    let descendant_prefix = format!("{path}/");
    files.retain(|item| item != path && (!recursive || !item.starts_with(&descendant_prefix)));
}

pub fn unique_name_for_target(
    dst_dir: &std::path::{Component, Path},
    name: &str,
    existing: &HashSet<String>,
) -> String {
    let original = dst_dir.join(name).to_string_lossy().to_string();
    if !existing.contains(&original) {
        return original;
    }
    let path = std::path::{Component, Path}::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|s| s.to_str());
    for i in 1..=999 {
        let suffix = if i == 1 { "copy".to_string() } else { format!("copy {}", i) };
        let candidate_name = if let Some(ext) = ext {
            format!("{stem} {suffix}.{ext}")
        } else {
            format!("{stem} {suffix}")
        };
        let candidate = dst_dir.join(candidate_name).to_string_lossy().to_string();
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    original
}

pub fn replace_path_prefix(target: &str, old_path: &str, new_path: &str) -> Option<String> {
    if target == old_path {
        return Some(new_path.to_string());
    }
    let old_prefix = format!("{old_path}/");
    target.strip_prefix(&old_prefix).map(|rest| format!("{new_path}/{}", rest))
}

pub fn path_display_name(path: &str) -> String {
    std::path::{Component, Path}::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub fn top_level_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .iter()
        .enumerate()
        .filter(|(index, path)| {
            !paths.iter().enumerate().any(|(other_index, parent)| {
                other_index != *index
                    && Path::new(path).starts_with(parent)
                    && (parent != *path || other_index < *index)
            })
        })
        .map(|(_, path)| path.clone())
        .collect()
}

pub fn outcome(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n")) }
}

pub fn paste_targets(files: &[String], destination: &str, sources: &[String]) -> Vec<(String, String)> {
    let directory = if is_directory_path(destination, files) {
        Path::new(destination)
    } else {
        Path::new(destination).parent().unwrap_or(Path::new(destination))
    };
    let mut existing = files.iter().cloned().collect::<HashSet<_>>();
    let mut targets = Vec::new();
    for source in sources {
        let name = Path::new(source).file_name().and_then(|name| name.to_str()).unwrap_or("");
        let target = unique_name_for_target(directory, name, &existing);
        existing.insert(target.clone());
        targets.push((source.clone(), target));
    }
    targets
}

#[cfg(test)]
#[path = "file_tree_tests.rs"]
mod file_tree_tests;

/// Validate a frozen selection without accepting parent traversal or self moves.
pub fn plan_move(root: &str, directory: &str, files: &[String], sources: Vec<String>) -> Option<Vec<(String, String)>> {
    let directory_path = Path::new(directory);
    let root_path = Path::new(root);
    if !directory_path.starts_with(root_path)
        || directory_path.components().any(|part| part == Component::ParentDir)
        || files.iter().any(|path| path == directory)
        || sources.iter().any(|source| {
            let path = Path::new(source);
            !path.starts_with(root_path)
                || path == root_path
                || directory_path.starts_with(path)
                || path.components().any(|part| part == Component::ParentDir)
        })
    {
        return None;
    }
    let moves = sources.into_iter()
        .filter_map(|source| {
            let path = Path::new(&source);
            if path.parent() == Some(directory_path) {
                return None;
            }
            let target = directory_path.join(path.file_name()?).to_string_lossy().into_owned();
            Some((source, target))
        })
        .collect::<Vec<_>>();
    Some(moves)
}

/// Native imports may only target a directory inside the receiving project snapshot.
pub fn valid_external_drop(root: &str, source: &str, directory: &str, files: &[String]) -> bool {
    Path::new(source).is_absolute()
        && Path::new(directory).starts_with(root)
        && !Path::new(directory).components().any(|part| part == std::path::Component::ParentDir)
        && !files.iter().any(|path| path == directory)
}

#[derive(Clone, Copy)]
pub enum FeedbackKind { Success, Error }

pub fn import_feedback(source: &str, result: Result<vw_api_file::file::LocalImportResponse, String>) -> (FeedbackKind, String) {
    let name = path_display_name(&source);
    let (kind, detail) = match result {
        Ok(response) if response.items.len() == 1 && response.items[0].source_path == source => {
            let item = &response.items[0];
            if item.ok {
                (FeedbackKind::Success, format!("已导入: {name}"))
            } else {
                (
                    FeedbackKind::Error,
                    format!("导入失败: {name}: {}", item.error.as_deref().unwrap_or("未知错误")),
                )
            }
        }
        // Transport failure can occur after publication. Never retry automatically.
        Err(error) => (
            FeedbackKind::Error,
            format!("导入失败或结果未知，请刷新确认: {name}: {error}"),
        ),
        Ok(_) => (FeedbackKind::Error, format!("导入结果未知，请刷新确认: {name}")),
    };
    (kind, detail)
}
