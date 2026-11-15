//! 远程工作区文件、搜索与 Git 命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use serde_json::json;
use vw_gateway_client::vw_api_types::file::{CopyFileRequest, DeleteFileRequest, MoveFileRequest};
use vw_gateway_client::vw_api_types::git::{GitCommandRequest, GitCommitRequest, GitMergeRequest};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

/// 工作区文件、搜索与 Git 接口族。
#[derive(Subcommand, Debug)]
pub(super) enum WorkspaceCommands {
    /// 在文件内容中搜索文本模式。
    FindText {
        /// 搜索模式；按原样转发给 Gateway。
        #[arg(value_parser = parse_required_value)]
        pattern: String,
    },
    /// 按名称搜索文件或目录。
    FindFile {
        /// 文件或目录名称查询。
        #[arg(value_parser = parse_required_value)]
        query: String,
        /// 是否包含目录。
        #[arg(long)]
        dirs: Option<bool>,
        /// 结果类型：`file` 或 `directory`；由 Gateway 校验。
        #[arg(long = "type", value_parser = parse_required_value)]
        entry_type: Option<String>,
        /// 最大结果数；由 Gateway 限制到安全范围。
        #[arg(long)]
        limit: Option<usize>,
    },
    /// 列出指定相对目录。
    List {
        /// 相对目录路径；按原样转发。
        path: String,
    },
    /// 读取指定文件的完整内容。
    Content {
        /// 相对文件路径；按原样转发。
        #[arg(value_parser = parse_required_value)]
        path: String,
    },
    /// 查询工作区 Git 文件状态。
    Status,
    /// 从 JSON 输入读取相对路径文件。
    Read {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入写入相对路径文件。
    Write {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入移动文件或目录。
    Move {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入复制文件或目录。
    Copy {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入删除文件或目录。
    Delete {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 执行受控 Git 操作。
    Git {
        /// Git 操作。
        #[command(subcommand)]
        command: WorkspaceGitCommands,
    },
}

/// 工作区 Git 副作用操作。
#[derive(Subcommand, Debug)]
pub(super) enum WorkspaceGitCommands {
    /// 从 JSON 输入创建 Git 提交。
    Commit {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入执行服务端受限 Git 命令。
    Command {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入合并 Git 分支。
    Merge {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
}

#[derive(Debug, Deserialize)]
struct DirectoryReadInput {
    #[serde(default)]
    directory: Option<String>,
    #[serde(default)]
    agent_key: Option<String>,
    path: String,
}

#[derive(Debug, Deserialize)]
struct DirectoryWriteInput {
    #[serde(default)]
    directory: Option<String>,
    #[serde(default)]
    agent_key: Option<String>,
    path: String,
    content: String,
    #[serde(default)]
    create_if_missing: bool,
}

pub(super) async fn run<R: Read, W: Write>(
    command: WorkspaceCommands,
    context: &ApiContext,
    yes: bool,
    stdin: &mut R,
    writer: &mut W,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        WorkspaceCommands::FindText { pattern } => client
            .file_find_text_in_directory(context.directory(), &pattern)
            .await
            .map_err(anyhow::Error::msg)?,
        WorkspaceCommands::FindFile { query, dirs, entry_type, limit } => client
            .file_find_in_directory(context.directory(), &query, dirs, entry_type.as_deref(), limit)
            .await
            .map_err(anyhow::Error::msg)?,
        WorkspaceCommands::List { path } => client
            .directory_file_list(context.directory(), &path)
            .await
            .map_err(anyhow::Error::msg)?,
        WorkspaceCommands::Content { path } => client
            .file_content_in_directory(context.directory(), &path)
            .await
            .map_err(anyhow::Error::msg)?,
        WorkspaceCommands::Status => client
            .file_status_in_directory(context.directory())
            .await
            .map_err(anyhow::Error::msg)?,
        WorkspaceCommands::Read { input } => {
            let request = parse_input::<DirectoryReadInput>(&input, stdin, "workspace read")?;
            let directory = request.directory.as_deref().or(context.directory());
            json!(
                client
                    .file_read_in_directory(directory, request.agent_key.as_deref(), &request.path)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        WorkspaceCommands::Write { input } => {
            let request = parse_input::<DirectoryWriteInput>(&input, stdin, "workspace write")?;
            let directory = request.directory.as_deref().or(context.directory());
            super::ensure_confirmed(
                yes,
                &format!("write file '{}' in workspace '{}'", request.path, workspace(directory)),
            )?;
            json!(
                client
                    .file_write_in_directory(
                        directory,
                        request.agent_key.as_deref(),
                        &request.path,
                        &request.content,
                        request.create_if_missing,
                    )
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        WorkspaceCommands::Move { input } => {
            let request = parse_input::<MoveFileRequest>(&input, stdin, "workspace move")?;
            super::ensure_confirmed(
                yes,
                &format!(
                    "move '{}' to '{}' in {}",
                    request.from_path,
                    request.to_path,
                    project_workspace(
                        &request.project_id.0,
                        request.worktree_id.as_ref().map(|worktree_id| worktree_id.0.as_str())
                    )
                ),
            )?;
            json!(client.file_move(&request).await.map_err(anyhow::Error::msg)?)
        }
        WorkspaceCommands::Copy { input } => {
            let request = parse_input::<CopyFileRequest>(&input, stdin, "workspace copy")?;
            super::ensure_confirmed(
                yes,
                &format!(
                    "copy '{}' to '{}' in {}",
                    request.from_path,
                    request.to_path,
                    project_workspace(
                        &request.project_id.0,
                        request.worktree_id.as_ref().map(|worktree_id| worktree_id.0.as_str())
                    )
                ),
            )?;
            json!(client.file_copy(&request).await.map_err(anyhow::Error::msg)?)
        }
        WorkspaceCommands::Delete { input } => {
            let request = parse_input::<DeleteFileRequest>(&input, stdin, "workspace delete")?;
            super::ensure_confirmed(
                yes,
                &format!(
                    "delete '{}' in {}",
                    request.path,
                    project_workspace(
                        &request.project_id.0,
                        request.worktree_id.as_ref().map(|worktree_id| worktree_id.0.as_str())
                    )
                ),
            )?;
            json!(client.file_delete(&request).await.map_err(anyhow::Error::msg)?)
        }
        WorkspaceCommands::Git { command } => match command {
            WorkspaceGitCommands::Commit { input } => {
                let request = parse_input::<GitCommitRequest>(&input, stdin, "git commit")?;
                super::ensure_confirmed(
                    yes,
                    &format!(
                        "commit changes in {}",
                        project_workspace(
                            &request.project_id.0,
                            request.worktree_id.as_ref().map(|worktree_id| worktree_id.0.as_str())
                        )
                    ),
                )?;
                json!(client.git_commit(&request).await.map_err(anyhow::Error::msg)?)
            }
            WorkspaceGitCommands::Command { input } => {
                let request = parse_input::<GitCommandRequest>(&input, stdin, "git command")?;
                super::ensure_confirmed(
                    yes,
                    &format!(
                        "run git command '{}' in workspace '{}'",
                        request.args.join(" "),
                        request.directory
                    ),
                )?;
                json!(client.git_command(&request).await.map_err(anyhow::Error::msg)?)
            }
            WorkspaceGitCommands::Merge { input } => {
                let request = parse_input::<GitMergeRequest>(&input, stdin, "git merge")?;
                super::ensure_confirmed(
                    yes,
                    &format!(
                        "merge '{}' into '{}' in project '{}'",
                        request.source_branch, request.target_branch, request.project_id.0
                    ),
                )?;
                json!(client.git_merge(&request).await.map_err(anyhow::Error::msg)?)
            }
        },
    };

    output::write_json(writer, &response, context.output_format())
}

fn parse_input<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid {operation} JSON input"))
}

fn workspace(directory: Option<&str>) -> &str {
    directory.unwrap_or("Gateway default")
}

fn project_workspace(project_id: &str, worktree_id: Option<&str>) -> String {
    match worktree_id {
        Some(worktree_id) => format!("project '{project_id}' worktree '{worktree_id}'"),
        None => format!("project '{project_id}'"),
    }
}

fn parse_required_value(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
