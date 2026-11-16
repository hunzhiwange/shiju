use vw_api_project::project::ProjectDto;
use vw_api_file::file::{FileNodeDto, FileNodeKind};
/// 网关刷新同时发布纯文件索引和保留空目录的树模型。
#[derive(Debug, Clone)]
pub struct FileTreeIndexSnapshot {
    pub files: Vec<String>,
    pub tree: desktop_file_tree::model::FileTreeNode,
}

impl FileTreeIndexSnapshot {
    pub fn from_dto(base_directory: &str, root: &FileNodeDto) -> Self {
        let mut files = Vec::new();
        collect_gateway_file_paths(base_directory, root, &mut files);
        files.sort();
        let tree = desktop_file_tree::model::build_file_tree_model_from_dto(base_directory, root);
        Self { files, tree }
    }
}

fn collect_gateway_file_paths(base_directory: &str, node: &FileNodeDto, output: &mut Vec<String>) {
    match node.kind {
        FileNodeKind::File => {
            output.push(resolve_absolute_gateway_path(base_directory, &node.path));
        }
        FileNodeKind::Directory => {
            if let Some(children) = node.children.as_ref() {
                for child in children {
                    collect_gateway_file_paths(base_directory, child, output);
                }
            }
        }
    }
}
fn resolve_absolute_gateway_path(base_directory: &str, relative_path: &str) -> String {
    let relative = relative_path.trim();
    if relative.is_empty() || relative == "." {
        return base_directory.to_string();
    }
    std::path::PathBuf::from(base_directory).join(relative).to_string_lossy().to_string()
}
pub(crate) fn normalize_path(value: &str) -> String {
    value.replace('\\', "/").trim_end_matches('/').to_string()
}
fn map_timestamp(value: i64) -> u64 {
    value.max(0) as u64
}
pub fn map_gateway_project(project: ProjectDto) -> project_types::Info {
    project_types::Info {
        id: project.id.0,
        worktree: project.directory.clone(),
        vcs: project.git.is_repo.then_some(project_types::Vcs::Git),
        name: Some(project.name),
        icon: None,
        commands: None,
        time: project_types::TimeInfo {
            created: map_timestamp(project.created_at_ms.0),
            updated: map_timestamp(project.updated_at_ms.0),
            initialized: None,
        },
        sandboxes: vec![project.directory],
    }
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;

pub fn map_loaded_project_info(
    project_path: String,
    project: vw_api_project::ProjectDto,
) -> super::message::LoadedProjectInfo {
    let current_branch = project.current_branch.clone().filter(|value| !value.trim().is_empty());
    super::message::LoadedProjectInfo {
        project_path,
        info: map_gateway_project(project),
        current_branch,
    }
}

pub fn find_session_project_path(
    sessions: &std::collections::HashMap<String, Vec<session_info::Info>>,
    active_project: Option<&str>,
    session_id: &str,
) -> Option<String> {
    sessions
        .iter()
        .find_map(|(path, sessions)| {
            sessions.iter().any(|s| s.id == session_id).then(|| path.clone())
        })
        .or_else(|| active_project.map(str::to_owned))
}
