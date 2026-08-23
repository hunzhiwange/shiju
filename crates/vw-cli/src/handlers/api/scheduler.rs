//! Agent Scheduler 项目管理命令。

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentSchedulerBulkCommandRequest, AgentSchedulerCommandRequest,
    AgentSchedulerCreateTaskRequest, AgentSchedulerLegacyImportRequest,
    AgentSchedulerOutboxDrainRequest, AgentSchedulerScopeDto,
    AgentSchedulerTaskMetadataUpdateRequest, AgentSchedulerUpsertTeamRequest,
    AgentSchedulerUpsertWorkerRequest, AgentSchedulerWakeRequest,
};
use vw_gateway_client::vw_api_types::id::{ProjectId, ProjectScope};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

/// Agent Scheduler 的非流式用户与运维操作。
#[derive(Subcommand, Debug)]
pub(super) enum SchedulerCommands {
    /// 读取项目 workflow 快照。
    Workflow { #[arg(value_parser = required)] project_id: String },
    /// 读取项目任务快照。
    Tasks { #[arg(value_parser = required)] project_id: String },
    /// 从 JSON 输入创建任务。
    Create {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 从 JSON 输入发送单条任务命令。
    Command {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 从 JSON 输入批量发送任务命令。
    Bulk {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 从 JSON 输入更新任务元数据。
    Metadata {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        task_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 从 JSON 输入导入旧任务。
    Import {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取 Worker 快照。
    Workers { #[arg(value_parser = required)] project_id: String },
    /// 从 JSON 输入创建或更新 Worker。
    WorkerUpsert {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取 Team 快照。
    Teams { #[arg(value_parser = required)] project_id: String },
    /// 从 JSON 输入创建或更新 Team。
    TeamUpsert {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取任务日志。
    Logs {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        task_id: String,
    },
    /// 读取任务产物。
    Artifacts {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        task_id: String,
    },
    /// 按 cursor 拉取事件；实时流由统一实时协议命令提供。
    Events {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long)]
        after_cursor: Option<i64>,
    },
    /// 从 JSON 输入唤醒调度器。
    Wake {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 从 JSON 输入领取 outbox 消息。
    Drain {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// Worker run event 仅供已认证的内部 Worker 回调。
    WorkerEvents {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        task_id: String,
        #[arg(value_parser = required)]
        run_id: String,
    },
}

impl SchedulerCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Create { project_id, .. } => {
                Some(format!("create scheduler task in project '{project_id}'"))
            }
            Self::Command { project_id, .. } => {
                Some(format!("send scheduler command in project '{project_id}'"))
            }
            Self::Bulk { project_id, .. } => {
                Some(format!("send scheduler bulk commands in project '{project_id}'"))
            }
            Self::Metadata { project_id, task_id, .. } => {
                Some(format!("update scheduler task '{task_id}' in project '{project_id}'"))
            }
            Self::Import { project_id, .. } => {
                Some(format!("import scheduler tasks in project '{project_id}'"))
            }
            Self::WorkerUpsert { project_id, .. } => {
                Some(format!("upsert scheduler worker in project '{project_id}'"))
            }
            Self::TeamUpsert { project_id, .. } => {
                Some(format!("upsert scheduler team in project '{project_id}'"))
            }
            Self::Wake { project_id, .. } => {
                Some(format!("wake scheduler in project '{project_id}'"))
            }
            Self::Drain { project_id, .. } => {
                Some(format!("drain scheduler outbox in project '{project_id}'"))
            }
            Self::Workflow { .. }
            | Self::Tasks { .. }
            | Self::Workers { .. }
            | Self::Teams { .. }
            | Self::Logs { .. }
            | Self::Artifacts { .. }
            | Self::Events { .. }
            | Self::WorkerEvents { .. } => None,
        }
    }
}

pub(super) async fn run(
    command: SchedulerCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        SchedulerCommands::Workflow { project_id } => {
            json!(client.agent_scheduler_workflow(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Tasks { project_id } => {
            json!(client.agent_scheduler_status(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Create { project_id, input } => {
            let request = parse::<AgentSchedulerCreateTaskRequest>(&input, stdin, "create")?;
            json!(client.agent_scheduler_create_task(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Command { project_id, input } => {
            let request = parse::<AgentSchedulerCommandRequest>(&input, stdin, "command")?;
            json!(client.agent_scheduler_command(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Bulk { project_id, input } => {
            let request = parse::<AgentSchedulerBulkCommandRequest>(&input, stdin, "bulk command")?;
            json!(client.agent_scheduler_bulk_command(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Metadata { project_id, task_id, input } => {
            let request = parse::<AgentSchedulerTaskMetadataUpdateRequest>(&input, stdin, "metadata")?;
            json!(client.agent_scheduler_update_task_metadata(&scope(context, project_id)?, &task_id, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Import { project_id, input } => {
            let request = parse::<AgentSchedulerLegacyImportRequest>(&input, stdin, "legacy import")?;
            json!(client.agent_scheduler_import_legacy_tasks(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Workers { project_id } => {
            json!(client.agent_scheduler_workers(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::WorkerUpsert { project_id, input } => {
            let request = parse::<AgentSchedulerUpsertWorkerRequest>(&input, stdin, "worker upsert")?;
            json!(client.agent_scheduler_upsert_worker(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Teams { project_id } => {
            json!(client.agent_scheduler_teams(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::TeamUpsert { project_id, input } => {
            let request = parse::<AgentSchedulerUpsertTeamRequest>(&input, stdin, "team upsert")?;
            json!(client.agent_scheduler_upsert_team(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Logs { project_id, task_id } => {
            json!(client.agent_scheduler_logs(&scope(context, project_id)?, &task_id).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Artifacts { project_id, task_id } => {
            json!(client.agent_scheduler_artifacts(&scope(context, project_id)?, &task_id).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Events { project_id, after_cursor } => {
            json!(client.agent_scheduler_events(&scope(context, project_id)?, after_cursor).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Wake { project_id, input } => {
            let request = parse::<AgentSchedulerWakeRequest>(&input, stdin, "wake")?;
            json!(client.agent_scheduler_wake(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::Drain { project_id, input } => {
            let request = parse::<AgentSchedulerOutboxDrainRequest>(&input, stdin, "outbox drain")?;
            json!(client.agent_scheduler_drain_outbox(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        SchedulerCommands::WorkerEvents { project_id, task_id, run_id } => {
            let _ = (project_id, task_id, run_id);
            bail!("scheduler worker-events is an internal worker callback and is not supported by shiju api")
        }
    };
    output::write_json(writer, &response, context.output_format())
}

fn scope(context: &ApiContext, project_id: String) -> Result<AgentSchedulerScopeDto> {
    let project_scope = ProjectScope::new(
        context.client().endpoint().tenant_scope().clone(),
        ProjectId::from(project_id),
    )
    .context("invalid scheduler project scope")?;
    Ok(AgentSchedulerScopeDto::new(project_scope))
}

fn parse<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid scheduler {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
