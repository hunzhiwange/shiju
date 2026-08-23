//! 远程 Gateway 主机的 desktop 专用接口。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::{ArgAction, Subcommand, ValueEnum};
use serde::Deserialize;
use serde_json::json;
use vw_gateway_client::DesktopServiceCommand;

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum DesktopCommands {
    PreferencesGet,
    PreferencesPatch { #[arg(long, value_parser = required)] input: String },
    ToolGet { #[arg(value_parser = required)] tool_type: String },
    ToolPut { #[arg(value_parser = required)] tool_type: String, #[arg(long, value_parser = required)] input: String },
    Skills,
    SkillDetail { #[arg(value_parser = required)] skill_id: String },
    SkillCreate,
    SkillInstall { #[arg(value_parser = required)] skill_id: String },
    SkillEnable { #[arg(value_parser = required)] skill_id: String, #[arg(long, action = ArgAction::Set)] enabled: bool },
    SkillDelete { #[arg(value_parser = required)] skill_id: String },
    ExternalApps,
    Open { #[arg(value_parser = required)] path: String, #[arg(value_parser = required)] target: String },
    Reveal { #[arg(value_parser = required)] path: String },
    Service { #[arg(value_enum)] command: ServiceCommand },
    ProjectPreferencesGet,
    ProjectPreferencesPut { #[arg(long, value_parser = required)] input: String },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum ServiceCommand { Status, Start, Stop, Restart }
impl From<ServiceCommand> for DesktopServiceCommand {
    fn from(value: ServiceCommand) -> Self {
        match value {
            ServiceCommand::Status => Self::Status,
            ServiceCommand::Start => Self::Start,
            ServiceCommand::Stop => Self::Stop,
            ServiceCommand::Restart => Self::Restart,
        }
    }
}

impl DesktopCommands {
    pub(super) fn dangerous_operation(&self, host: &str, port: u16) -> Option<String> {
        let endpoint = format!("http://{host}:{port}");
        let action = match self {
            Self::PreferencesPatch { .. } => "patch desktop preferences".to_string(),
            Self::ToolPut { tool_type, .. } => format!("write desktop tool content '{tool_type}'"),
            Self::SkillCreate => "create desktop skill scaffold".to_string(),
            Self::SkillInstall { skill_id } => format!("install desktop skill '{skill_id}'"),
            Self::SkillEnable { skill_id, enabled } => format!("set desktop skill '{skill_id}' enabled={enabled}"),
            Self::SkillDelete { skill_id } => format!("delete desktop skill '{skill_id}'"),
            Self::Open { path, target } => format!("open '{path}' with '{target}'"),
            Self::Reveal { path } => format!("reveal '{path}'"),
            Self::Service { command } => format!("run desktop service command '{command:?}'"),
            Self::ProjectPreferencesPut { .. } => "write desktop project preferences".to_string(),
            Self::PreferencesGet | Self::ToolGet { .. } | Self::Skills | Self::SkillDetail { .. }
            | Self::ExternalApps | Self::ProjectPreferencesGet => return None,
        };
        Some(format!("{action} on remote Gateway {endpoint}"))
    }
}

pub(super) async fn run(
    command: DesktopCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let project_path = context.directory();
    let response = match command {
        DesktopCommands::PreferencesGet => client.desktop_preferences_get().await.map_err(anyhow::Error::msg)?,
        DesktopCommands::PreferencesPatch { input } => client.desktop_preferences_patch(&read_json_input(&input, stdin)?).await.map_err(anyhow::Error::msg)?,
        DesktopCommands::ToolGet { tool_type } => json!({"content":client.desktop_tool_content_get(&tool_type).await.map_err(anyhow::Error::msg)?}),
        DesktopCommands::ToolPut { tool_type, input } => {
            let body = parse::<ContentInput>(&input, stdin, "tool content")?;
            client.desktop_tool_content_put(&tool_type, &body.content).await.map_err(anyhow::Error::msg)?;
            json!({"saved":true})
        }
        DesktopCommands::Skills => json!(client.desktop_skills_get(project_path).await.map_err(anyhow::Error::msg)?),
        DesktopCommands::SkillDetail { skill_id } => json!(client.desktop_skill_detail_get(project_path, &skill_id).await.map_err(anyhow::Error::msg)?),
        DesktopCommands::SkillCreate => {
            let path = required_project_path(project_path)?;
            json!({"path":client.desktop_skill_create(path).await.map_err(anyhow::Error::msg)?})
        }
        DesktopCommands::SkillInstall { skill_id } => {
            let path = required_project_path(project_path)?;
            json!({"path":client.desktop_skill_install_builtin(path, &skill_id).await.map_err(anyhow::Error::msg)?})
        }
        DesktopCommands::SkillEnable { skill_id, enabled } => json!({"path":client.desktop_skill_set_enabled(project_path, &skill_id, enabled).await.map_err(anyhow::Error::msg)?}),
        DesktopCommands::SkillDelete { skill_id } => json!({"path":client.desktop_skill_delete(project_path, &skill_id).await.map_err(anyhow::Error::msg)?}),
        DesktopCommands::ExternalApps => {
            let state = client.desktop_external_apps_get().await.map_err(anyhow::Error::msg)?;
            json!({"platform":state.platform,"apps":state.apps.into_iter().map(|(id,available)|json!({"id":id,"available":available})).collect::<Vec<_>>()})
        }
        DesktopCommands::Open { path, target } => {
            client.desktop_external_app_open(&path, &target).await.map_err(anyhow::Error::msg)?;
            json!({"opened":true,"path":path,"target":target})
        }
        DesktopCommands::Reveal { path } => {
            client.desktop_external_path_reveal(&path).await.map_err(anyhow::Error::msg)?;
            json!({"revealed":true,"path":path})
        }
        DesktopCommands::Service { command } => {
            let value = client.desktop_service_command_typed(command.into()).await.map_err(anyhow::Error::msg)?;
            json!({"command":value.command,"output":value.output})
        }
        DesktopCommands::ProjectPreferencesGet => {
            let path = required_project_path(project_path)?;
            json!(client.desktop_project_preferences_get(path).await.map_err(anyhow::Error::msg)?)
        }
        DesktopCommands::ProjectPreferencesPut { input } => {
            let path = required_project_path(project_path)?;
            let body = parse::<ProjectPreferencesInput>(&input, stdin, "project preferences")?;
            client.desktop_project_preferences_put(path, &body.model, body.auto_model, body.acp_agent.as_deref()).await.map_err(anyhow::Error::msg)?;
            json!({"saved":true})
        }
    };
    output::write_json(writer, &response, context.output_format())
}

#[derive(Deserialize)]
struct ContentInput { content: String }
#[derive(Deserialize)]
struct ProjectPreferencesInput { model: String, #[serde(default)] auto_model: bool, #[serde(default)] acp_agent: Option<String> }
fn required_project_path(path: Option<&str>) -> Result<&str> { path.context("desktop command requires --directory") }
fn parse<T: serde::de::DeserializeOwned>(source: &str, stdin: &mut impl Read, kind: &str) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?).with_context(|| format!("invalid desktop {kind} JSON input"))
}
fn required(value: &str) -> Result<String, String> {
    let value = value.trim(); if value.is_empty() { return Err("value must not be empty".to_string()); } Ok(value.to_string())
}
