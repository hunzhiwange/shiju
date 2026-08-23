//! 项目与 worktree 远程命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use serde_json::json;
use vw_gateway_client::vw_api_types::id::{ProjectId, ProjectScope};
use vw_gateway_client::vw_api_types::project::{
    ListProjectsRequest, ProjectStatus, ResolveProjectRequest, UpdateProjectRequest,
};
use vw_gateway_client::vw_api_types::worktree::{
    CreateWorktreeRequest, DeleteWorktreeRequest, ResetWorktreeRequest,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

/// 项目与 worktree 接口族。
#[derive(Subcommand, Debug)]
pub(super) enum ProjectCommands {
    /// 分页列出项目。
    List {
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, value_enum)]
        status: Option<ProjectStatusArg>,
    },
    /// 从 JSON 输入按目录解析项目。
    Resolve {
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 读取项目详情。
    Detail {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
    },
    /// 从 JSON 输入更新项目。
    Update {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 读取项目在当前目录的变更记录。
    ChangeRecords {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
    },
    /// 管理项目 worktree。
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommands,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum ProjectStatusArg {
    Ready,
    Indexing,
    Error,
}

impl From<ProjectStatusArg> for ProjectStatus {
    fn from(status: ProjectStatusArg) -> Self {
        match status {
            ProjectStatusArg::Ready => Self::Ready,
            ProjectStatusArg::Indexing => Self::Indexing,
            ProjectStatusArg::Error => Self::Error,
        }
    }
}

/// Worktree 生命周期操作。
#[derive(Subcommand, Debug)]
pub(super) enum WorktreeCommands {
    /// 列出项目 worktree。
    List {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
    },
    /// 从 JSON 输入创建 worktree。
    Create {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 读取 worktree 详情。
    Detail {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
        #[arg(value_parser = parse_required_value)]
        worktree_id: String,
    },
    /// 从 JSON 输入删除 worktree。
    Delete {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
        #[arg(value_parser = parse_required_value)]
        worktree_id: String,
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入重置 worktree。
    Reset {
        #[arg(value_parser = parse_required_value)]
        project_id: String,
        #[arg(value_parser = parse_required_value)]
        worktree_id: String,
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
}

impl ProjectCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Update { project_id, .. } => Some(format!("update project '{project_id}'")),
            Self::Worktree { command: WorktreeCommands::Create { project_id, .. } } => {
                Some(format!("create worktree in project '{project_id}'"))
            }
            Self::Worktree {
                command: WorktreeCommands::Delete { project_id, worktree_id, .. },
            } => Some(format!("delete worktree '{worktree_id}' in project '{project_id}'")),
            Self::Worktree {
                command: WorktreeCommands::Reset { project_id, worktree_id, .. },
            } => Some(format!("reset worktree '{worktree_id}' in project '{project_id}'")),
            Self::List { .. }
            | Self::Resolve { .. }
            | Self::Detail { .. }
            | Self::ChangeRecords { .. }
            | Self::Worktree {
                command: WorktreeCommands::List { .. } | WorktreeCommands::Detail { .. },
            } => None,
        }
    }
}

pub(super) async fn run(
    command: ProjectCommands,
    context: &ApiContext,
    yes: bool,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let tenant = client.endpoint().tenant_scope();
    let response = match command {
        ProjectCommands::List { cursor, limit, query, status } => json!(
            client
                .project_list(
                    tenant,
                    &ListProjectsRequest {
                        cursor,
                        limit,
                        query,
                        status: status.map(Into::into),
                    },
                )
                .await
                .map_err(anyhow::Error::msg)?
        ),
        ProjectCommands::Resolve { input } => {
            let request = parse_input::<ResolveProjectRequest>(&input, stdin, "project resolve")?;
            if request.create_if_missing {
                super::ensure_confirmed(
                    yes,
                    &format!("resolve and create project for '{}'", request.directory),
                )?;
            }
            json!(client.project_resolve(tenant, &request).await.map_err(anyhow::Error::msg)?)
        }
        ProjectCommands::Detail { project_id } => json!(
            client
                .project_get(&project_scope(context, &project_id)?)
                .await
                .map_err(anyhow::Error::msg)?
        ),
        ProjectCommands::Update { project_id, input } => {
            let request = parse_input::<UpdateProjectRequest>(&input, stdin, "project update")?;
            json!(
                client
                    .project_update(&project_scope(context, &project_id)?, &request)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        ProjectCommands::ChangeRecords { project_id } => {
            let directory = context
                .directory()
                .filter(|value| !value.trim().is_empty())
                .context("project change-records requires --directory")?;
            json!(
                client
                    .project_change_records(&project_scope(context, &project_id)?, directory)
                    .await
                    .map_err(anyhow::Error::msg)?
            )
        }
        ProjectCommands::Worktree { command } => match command {
            WorktreeCommands::List { project_id } => json!(
                client
                    .project_worktrees(&project_scope(context, &project_id)?)
                    .await
                    .map_err(anyhow::Error::msg)?
            ),
            WorktreeCommands::Create { project_id, input } => {
                let request =
                    parse_input::<CreateWorktreeRequest>(&input, stdin, "worktree create")?;
                json!(
                    client
                        .project_worktree_create(&project_scope(context, &project_id)?, &request)
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
            WorktreeCommands::Detail { project_id, worktree_id } => json!(
                client
                    .worktree_get(&project_scope(context, &project_id)?, &worktree_id)
                    .await
                    .map_err(anyhow::Error::msg)?
            ),
            WorktreeCommands::Delete { project_id, worktree_id, input } => {
                let request =
                    parse_input::<DeleteWorktreeRequest>(&input, stdin, "worktree delete")?;
                json!(
                    client
                        .worktree_delete(
                            &project_scope(context, &project_id)?,
                            &worktree_id,
                            &request,
                        )
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
            WorktreeCommands::Reset { project_id, worktree_id, input } => {
                let request =
                    parse_input::<ResetWorktreeRequest>(&input, stdin, "worktree reset")?;
                json!(
                    client
                        .worktree_reset(
                            &project_scope(context, &project_id)?,
                            &worktree_id,
                            &request,
                        )
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
        },
    };
    output::write_json(writer, &response, context.output_format())
}

fn project_scope(context: &ApiContext, project_id: &str) -> Result<ProjectScope> {
    ProjectScope::new(
        context.client().endpoint().tenant_scope().clone(),
        ProjectId::from(project_id),
    )
    .context("invalid project scope")
}

fn parse_input<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid {operation} JSON input"))
}

fn parse_required_value(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
