//! Staff 能力目录、编排与投影命令。

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::id::{ProjectId, ProjectScope};
use vw_gateway_client::vw_api_types::staff::{
    StaffAutoSchedulePlanRequest, StaffCommandRequest, StaffProjectCapabilityCatalogRequest,
    StaffProjectOrchestrationRequest, StaffWorkflowOrchestrationRequest,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum StaffCommands {
    /// 读取 Staff 阶段能力注册表。
    StageCapabilities,
    /// 读取项目能力目录。
    ProjectCapabilities { #[arg(value_parser = required)] project_id: String },
    /// 从 JSON 输入保存项目能力目录。
    ProjectCapabilitiesSave {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取项目编排状态。
    ProjectOrchestration { #[arg(value_parser = required)] project_id: String },
    /// 从 JSON 输入保存项目编排状态。
    ProjectOrchestrationSave {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取 workflow 能力目录。
    WorkflowCapabilities {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        workflow_id: String,
    },
    /// 从 JSON 输入保存 workflow 能力目录。
    WorkflowCapabilitiesSave {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        workflow_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取 workflow 编排状态。
    WorkflowOrchestration {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        workflow_id: String,
    },
    /// 从 JSON 输入保存 workflow 编排状态。
    WorkflowOrchestrationSave {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(value_parser = required)]
        workflow_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 读取项目 Staff 投影。
    Projection { #[arg(value_parser = required)] project_id: String },
    /// 从 JSON 输入校验并执行 Staff 命令。
    Command {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// 从 JSON 输入生成自动排程计划。
    AutoPlan {
        #[arg(value_parser = required)]
        project_id: String,
        #[arg(long, value_parser = required)]
        input: String,
    },
    /// Staff IPC 仅供已认证的内部员工进程协议。
    Ipc,
}

impl StaffCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::ProjectCapabilitiesSave { project_id, .. } => {
                Some(format!("save staff capabilities for project '{project_id}'"))
            }
            Self::ProjectOrchestrationSave { project_id, .. } => {
                Some(format!("save staff orchestration for project '{project_id}'"))
            }
            Self::WorkflowCapabilitiesSave { project_id, workflow_id, .. } => Some(format!(
                "save staff capabilities for workflow '{workflow_id}' in project '{project_id}'"
            )),
            Self::WorkflowOrchestrationSave { project_id, workflow_id, .. } => Some(format!(
                "save staff orchestration for workflow '{workflow_id}' in project '{project_id}'"
            )),
            Self::Command { project_id, .. } => {
                Some(format!("execute staff command in project '{project_id}'"))
            }
            Self::AutoPlan { project_id, .. } => {
                Some(format!("generate staff auto-schedule plan in project '{project_id}'"))
            }
            Self::StageCapabilities
            | Self::ProjectCapabilities { .. }
            | Self::ProjectOrchestration { .. }
            | Self::WorkflowCapabilities { .. }
            | Self::WorkflowOrchestration { .. }
            | Self::Projection { .. }
            | Self::Ipc => None,
        }
    }
}

pub(super) async fn run(
    command: StaffCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        StaffCommands::StageCapabilities => {
            json!(client.staff_stage_capabilities().await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::ProjectCapabilities { project_id } => {
            json!(client.staff_project_capabilities(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::ProjectCapabilitiesSave { project_id, input } => {
            let request = parse::<StaffProjectCapabilityCatalogRequest>(&input, stdin, "project capabilities")?;
            json!(client.save_staff_project_capabilities(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::ProjectOrchestration { project_id } => {
            json!(client.staff_project_orchestration(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::ProjectOrchestrationSave { project_id, input } => {
            let request = parse::<StaffProjectOrchestrationRequest>(&input, stdin, "project orchestration")?;
            json!(client.save_staff_project_orchestration(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::WorkflowCapabilities { project_id, workflow_id } => {
            json!(client.staff_workflow_capabilities(&scope(context, project_id)?, &workflow_id).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::WorkflowCapabilitiesSave { project_id, workflow_id, input } => {
            let request = parse::<StaffProjectCapabilityCatalogRequest>(&input, stdin, "workflow capabilities")?;
            json!(client.save_staff_workflow_capabilities(&scope(context, project_id)?, &workflow_id, &request).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::WorkflowOrchestration { project_id, workflow_id } => {
            json!(client.staff_workflow_orchestration(&scope(context, project_id)?, &workflow_id).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::WorkflowOrchestrationSave { project_id, workflow_id, input } => {
            let request = parse::<StaffWorkflowOrchestrationRequest>(&input, stdin, "workflow orchestration")?;
            json!(client.save_staff_workflow_orchestration(&scope(context, project_id)?, &workflow_id, &request).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::Projection { project_id } => {
            json!(client.staff_projection(&scope(context, project_id)?).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::Command { project_id, input } => {
            let request = parse::<StaffCommandRequest>(&input, stdin, "command")?;
            json!(client.staff_command(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::AutoPlan { project_id, input } => {
            let request = parse::<StaffAutoSchedulePlanRequest>(&input, stdin, "auto schedule plan")?;
            json!(client.staff_auto_schedule_plan(&scope(context, project_id)?, &request).await.map_err(anyhow::Error::msg)?)
        }
        StaffCommands::Ipc => {
            bail!("staff IPC is an internal staff process protocol and is not supported by shiju")
        }
    };
    output::write_json(writer, &response, context.output_format())
}

fn scope(context: &ApiContext, project_id: String) -> Result<AgentSchedulerScopeDto> {
    let project_scope = ProjectScope::new(
        context.client().endpoint().tenant_scope().clone(),
        ProjectId::from(project_id),
    )
    .context("invalid staff project scope")?;
    Ok(AgentSchedulerScopeDto::new(project_scope))
}

fn parse<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?)
        .with_context(|| format!("invalid staff {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
