//! Gateway 本地 skills/plugins 管理命令。

use std::io::Write;

use anyhow::{Context, Result};
use clap::{ArgAction, Subcommand};
use serde_json::json;

use super::context::ApiContext;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum ExtensionCommands {
    Skills,
    SkillDetail {
        #[arg(value_parser = required)]
        skill_id: String,
    },
    SkillCreate,
    SkillInstall {
        #[arg(value_parser = required)]
        skill_id: String,
    },
    SkillEnable {
        #[arg(value_parser = required)]
        skill_id: String,
        #[arg(long, action = ArgAction::Set)]
        enabled: bool,
    },
    SkillDelete {
        #[arg(value_parser = required)]
        skill_id: String,
    },
    Plugins,
    PluginDetail {
        #[arg(value_parser = required)]
        plugin_id: String,
    },
    PluginEnable {
        #[arg(value_parser = required)]
        plugin_id: String,
        #[arg(long, action = ArgAction::Set)]
        enabled: bool,
    },
    PluginDelete {
        #[arg(value_parser = required)]
        plugin_id: String,
    },
}

impl ExtensionCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::SkillCreate => Some("create local skill scaffold".to_string()),
            Self::SkillInstall { skill_id } => Some(format!("install built-in skill '{skill_id}'")),
            Self::SkillEnable { skill_id, enabled } => {
                Some(format!("set skill '{skill_id}' enabled={enabled}"))
            }
            Self::SkillDelete { skill_id } => Some(format!("delete skill '{skill_id}'")),
            Self::PluginEnable { plugin_id, enabled } => {
                Some(format!("set plugin '{plugin_id}' enabled={enabled}"))
            }
            Self::PluginDelete { plugin_id } => Some(format!("delete plugin '{plugin_id}'")),
            Self::Skills | Self::SkillDetail { .. } | Self::Plugins | Self::PluginDetail { .. } => {
                None
            }
        }
    }
}

pub(super) async fn run(
    command: ExtensionCommands,
    context: &ApiContext,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let project_path = context.directory();
    let response = match command {
        ExtensionCommands::Skills => {
            json!(client.skills_get(project_path).await.map_err(anyhow::Error::msg)?)
        }
        ExtensionCommands::SkillDetail { skill_id } => json!(
            client.skill_detail_get(project_path, &skill_id).await.map_err(anyhow::Error::msg)?
        ),
        ExtensionCommands::SkillCreate => {
            let project_path =
                project_path.context("api extensions skill-create requires --directory")?;
            json!({"path":client.skill_create(project_path).await.map_err(anyhow::Error::msg)?})
        }
        ExtensionCommands::SkillInstall { skill_id } => {
            let project_path =
                project_path.context("api extensions skill-install requires --directory")?;
            json!({"path":client.skill_install_builtin(project_path, &skill_id).await.map_err(anyhow::Error::msg)?})
        }
        ExtensionCommands::SkillEnable { skill_id, enabled } => {
            json!({"path":client.skill_set_enabled(project_path, &skill_id, enabled).await.map_err(anyhow::Error::msg)?})
        }
        ExtensionCommands::SkillDelete { skill_id } => {
            json!({"path":client.skill_delete(project_path, &skill_id).await.map_err(anyhow::Error::msg)?})
        }
        ExtensionCommands::Plugins => {
            json!(client.plugins_get().await.map_err(anyhow::Error::msg)?)
        }
        ExtensionCommands::PluginDetail { plugin_id } => {
            json!(client.plugin_detail_get(&plugin_id).await.map_err(anyhow::Error::msg)?)
        }
        ExtensionCommands::PluginEnable { plugin_id, enabled } => {
            json!({"path":client.plugin_set_enabled(&plugin_id, enabled).await.map_err(anyhow::Error::msg)?})
        }
        ExtensionCommands::PluginDelete { plugin_id } => {
            json!({"path":client.plugin_delete(&plugin_id).await.map_err(anyhow::Error::msg)?})
        }
    };
    output::write_json(writer, &response, context.output_format())
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}
