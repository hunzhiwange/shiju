//! TUI 文件名查询的 Gateway 适配边界。

use std::path::{Component, Path};

use serde_json::Value;
use vw_tui::V3FileSearchMatch;

use super::gateway::{GatewayUiRuntime, block_on_gateway};

const FILE_SEARCH_LIMIT: usize = 20;

pub(crate) fn search_files_blocking(
    runtime: &GatewayUiRuntime,
    query: &str,
) -> Result<Vec<V3FileSearchMatch>, String> {
    block_on_gateway(async {
        let value = runtime
            .client()
            .file_find_in_directory(
                runtime.directory_value().as_deref(),
                query,
                Some(false),
                Some("file"),
                Some(FILE_SEARCH_LIMIT),
            )
            .await?;
        map_file_search_response(value)
    })
}

pub(super) fn map_file_search_response(value: Value) -> Result<Vec<V3FileSearchMatch>, String> {
    let paths = serde_json::from_value::<Vec<String>>(value)
        .map_err(|error| format!("invalid file search response: {error}"))?;
    paths
        .into_iter()
        .take(FILE_SEARCH_LIMIT)
        .map(|path| {
            let path = safe_relative_path(path.as_str())?;
            Ok(V3FileSearchMatch { id: path.clone(), path, match_indices: Vec::new() })
        })
        .collect()
}

pub(super) fn safe_relative_path(value: &str) -> Result<String, String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty() || normalized.contains('\0') || normalized.starts_with('/') {
        return Err("file search returned an invalid relative path".to_string());
    }
    if normalized
        .split('/')
        .next()
        .is_some_and(|segment| segment.ends_with(':'))
    {
        return Err("file search returned an absolute path".to_string());
    }

    let mut segments = Vec::new();
    for component in Path::new(normalized.as_str()).components() {
        match component {
            Component::Normal(segment) => segments.push(segment.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("file search returned a path outside the workspace".to_string());
            }
        }
    }
    if segments.is_empty() {
        return Err("file search returned an invalid relative path".to_string());
    }
    Ok(segments.join("/"))
}
