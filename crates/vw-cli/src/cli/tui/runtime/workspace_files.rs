//! 当前 Gateway 目录下的只读文件浏览与文本读取边界。

use std::path::{Component, Path};

use serde::Deserialize;
use serde_json::Value;
use vw_tui::V3SelectionItem;

use super::gateway::{GatewayUiRuntime, block_on_gateway};

const MAX_DIRECTORY_ENTRIES: usize = 200;
const MAX_FILE_DISPLAY_CHARS: usize = 16_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceFileListing {
    path: String,
    items: Vec<V3SelectionItem>,
}

impl WorkspaceFileListing {
    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn items(&self) -> &[V3SelectionItem] {
        self.items.as_slice()
    }

    pub(crate) fn into_items(self) -> Vec<V3SelectionItem> {
        self.items
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceFileText {
    path: String,
    content: String,
    truncated: bool,
}

impl WorkspaceFileText {
    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn render_text(&self) -> String {
        let suffix = if self.truncated { "\n\n[内容已截断]" } else { "" };
        format!("{}\n\n{}{}", self.path, self.content, suffix)
    }
}

pub(crate) fn load_workspace_files_blocking(
    runtime: &GatewayUiRuntime,
    path: &str,
) -> Result<WorkspaceFileListing, String> {
    let path = normalize_workspace_relative_path(path)?;
    block_on_gateway(async {
        let value = runtime
            .client()
            .directory_file_list(runtime.directory_value().as_deref(), path.as_str())
            .await?;
        map_workspace_listing(path.as_str(), value)
    })
}

pub(crate) fn read_workspace_file_blocking(
    runtime: &GatewayUiRuntime,
    path: &str,
) -> Result<WorkspaceFileText, String> {
    let path = normalize_workspace_file_path(path)?;
    block_on_gateway(async {
        let response = runtime
            .client()
            .file_read_in_directory(
                runtime.directory_value().as_deref(),
                None,
                path.as_str(),
            )
            .await?;
        workspace_text_from_parts(
            runtime.directory(),
            path.as_str(),
            response.root_directory.as_str(),
            response.path.as_str(),
            response.content,
        )
    })
}

pub(crate) fn normalize_workspace_relative_path(value: &str) -> Result<String, String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        return Ok(".".to_string());
    }
    if normalized.contains('\0') || normalized.starts_with('/') {
        return Err("workspace file path must be relative to the current Gateway directory".to_string());
    }
    if normalized
        .split('/')
        .next()
        .is_some_and(|segment| segment.ends_with(':'))
    {
        return Err("workspace file path must not be absolute".to_string());
    }

    let mut segments = Vec::new();
    for component in Path::new(normalized.as_str()).components() {
        match component {
            Component::Normal(segment) => {
                let segment = segment
                    .to_str()
                    .ok_or_else(|| "workspace file path must be valid UTF-8".to_string())?;
                if segment.chars().any(char::is_control) {
                    return Err("workspace file path contains control characters".to_string());
                }
                segments.push(segment.to_string());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("workspace file path escapes the current Gateway directory".to_string());
            }
        }
    }
    if segments.is_empty() {
        Ok(".".to_string())
    } else {
        Ok(segments.join("/"))
    }
}

fn normalize_workspace_file_path(value: &str) -> Result<String, String> {
    let path = normalize_workspace_relative_path(value)?;
    if path == "." {
        Err("workspace file selection requires a file path".to_string())
    } else {
        Ok(path)
    }
}

pub(super) fn map_workspace_listing(
    requested_path: &str,
    value: Value,
) -> Result<WorkspaceFileListing, String> {
    let requested_path = normalize_workspace_relative_path(requested_path)?;
    let entries = serde_json::from_value::<Vec<WorkspaceEntryWire>>(value)
        .map_err(|error| format!("invalid workspace file listing response: {error}"))?;
    let mut items = Vec::with_capacity(
        entries.len().min(MAX_DIRECTORY_ENTRIES) + usize::from(requested_path != "."),
    );
    if requested_path != "." {
        let parent = workspace_parent(requested_path.as_str());
        items.push(selection_item(WorkspaceEntryKind::Directory, parent.as_str(), ".."));
    }
    for entry in entries.into_iter().take(MAX_DIRECTORY_ENTRIES) {
        let path = normalize_workspace_file_path(entry.path.as_str())?;
        if !is_direct_child(requested_path.as_str(), path.as_str()) {
            return Err(format!(
                "workspace file listing returned an entry outside requested directory: {path}"
            ));
        }
        let label = Path::new(path.as_str())
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "workspace file listing returned an invalid entry name".to_string())?;
        items.push(selection_item(entry.kind, path.as_str(), label));
    }
    Ok(WorkspaceFileListing { path: requested_path, items })
}

pub(super) fn workspace_text_from_parts(
    expected_root: &Path,
    requested_path: &str,
    response_root: &str,
    response_path: &str,
    response_content: String,
) -> Result<WorkspaceFileText, String> {
    let requested_path = normalize_workspace_file_path(requested_path)?;
    let response_path = normalize_workspace_file_path(response_path)?;
    if response_path != requested_path {
        return Err("Gateway file response path does not match the selected file".to_string());
    }
    if Path::new(response_root) != expected_root {
        return Err("Gateway file response root does not match the current directory".to_string());
    }
    if looks_binary(response_content.as_str()) {
        return Err("selected file is not displayable UTF-8 text".to_string());
    }

    let char_count = response_content.chars().count();
    let truncated = char_count > MAX_FILE_DISPLAY_CHARS;
    let content = if truncated {
        response_content.chars().take(MAX_FILE_DISPLAY_CHARS).collect()
    } else {
        response_content
    };
    Ok(WorkspaceFileText { path: requested_path, content, truncated })
}

fn selection_item(kind: WorkspaceEntryKind, path: &str, label: &str) -> V3SelectionItem {
    let (kind_id, description) = match kind {
        WorkspaceEntryKind::Directory => ("directory", "目录"),
        WorkspaceEntryKind::File => ("file", "文件"),
    };
    V3SelectionItem {
        id: format!("{kind_id}:{path}"),
        label: label.to_string(),
        description: Some(description.to_string()),
        search_value: Some(path.to_string()),
        ..V3SelectionItem::default()
    }
}

fn workspace_parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::to_str)
        .filter(|parent| !parent.is_empty())
        .unwrap_or(".")
        .to_string()
}

fn is_direct_child(parent: &str, child: &str) -> bool {
    let child_parent = Path::new(child).parent().and_then(Path::to_str).unwrap_or_default();
    if parent == "." {
        child_parent.is_empty()
    } else {
        child_parent == parent
    }
}

fn looks_binary(content: &str) -> bool {
    content.contains('\u{fffd}')
        || content.chars().any(|character| {
            character.is_control() && !matches!(character, '\n' | '\r' | '\t')
        })
}

#[derive(Debug, Deserialize)]
struct WorkspaceEntryWire {
    path: String,
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "type")]
    kind: WorkspaceEntryKind,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WorkspaceEntryKind {
    File,
    Directory,
}
