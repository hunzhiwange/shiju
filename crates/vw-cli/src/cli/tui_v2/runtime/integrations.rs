use super::gateway::{GatewayUiRuntime, block_on_gateway};
use super::super::commands::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandSource, CommandSpec,
};
pub(crate) use super::super::model::integration::{
    IntegrationAction, IntegrationItem, IntegrationKind, IntegrationPanel,
};
use vw_config_types::skills::{
    SkillsDirectoryProvider, parse_skills_directory_provider,
};
#[cfg(test)]
pub(crate) use super::super::model::integration::IntegrationLoadState;

pub(crate) trait IntegrationRuntime {
    fn load(&mut self, kind: IntegrationKind) -> Result<Vec<IntegrationItem>, String>;

    fn set_enabled(
        &mut self,
        _kind: IntegrationKind,
        _id: &str,
        _enabled: bool,
    ) -> Result<(), String> {
        Err("this integration does not support enable/disable".to_string())
    }

    fn delete(&mut self, _kind: IntegrationKind, _id: &str) -> Result<(), String> {
        Err("this integration does not support deletion".to_string())
    }

    fn skill_provider(&mut self) -> Result<SkillsDirectoryProvider, String> {
        Ok(SkillsDirectoryProvider::default())
    }

    fn set_skill_provider(
        &mut self,
        _provider: SkillsDirectoryProvider,
    ) -> Result<(), String> {
        Err("this integration does not support skill platform switching".to_string())
    }

    fn load_skills(
        &mut self,
        _provider: SkillsDirectoryProvider,
    ) -> Result<Vec<IntegrationItem>, String> {
        self.load(IntegrationKind::Skill)
    }
}

pub(crate) fn refresh_integrations(
    panel: &mut IntegrationPanel,
    runtime: &mut dyn IntegrationRuntime,
) {
    panel.begin_refresh();
    if panel.kind == IntegrationKind::Skill {
        match runtime.skill_provider() {
            Ok(provider) => {
                panel.skill_provider = provider;
                panel.can_toggle = provider == SkillsDirectoryProvider::Vibewindow;
            }
            Err(error) => {
                panel.finish_refresh(Err(error));
                return;
            }
        }
    }
    let result = if panel.kind == IntegrationKind::Skill {
        runtime.load_skills(panel.skill_provider)
    } else {
        runtime.load(panel.kind)
    };
    panel.finish_refresh(result);
}

pub(crate) fn switch_skill_provider(
    panel: &mut IntegrationPanel,
    runtime: &mut dyn IntegrationRuntime,
    provider: SkillsDirectoryProvider,
) -> Result<(), String> {
    if panel.kind != IntegrationKind::Skill {
        return Err("skill platform switching requires the Skill panel".to_string());
    }
    let previous = panel.skill_provider;
    runtime.set_skill_provider(provider)?;
    let items = match runtime.load_skills(provider) {
        Ok(items) => items,
        Err(error) => {
            return match runtime.set_skill_provider(previous) {
                Ok(()) => Err(format!(
                    "技能平台目录加载失败，已恢复 {}：{error}",
                    previous.label()
                )),
                Err(rollback_error) => Err(format!(
                    "技能平台目录加载失败且恢复配置失败：{error}；{rollback_error}"
                )),
            };
        }
    };
    panel.set_skill_provider(provider);
    panel.finish_refresh(Ok(items));
    Ok(())
}

pub(crate) fn toggle_selected_integration(
    panel: &IntegrationPanel,
    runtime: &mut dyn IntegrationRuntime,
) -> Result<bool, String> {
    if panel.kind == IntegrationKind::Skill && !panel.can_toggle {
        return Ok(false);
    }
    let Some(item) = panel.items.get(panel.selected) else {
        return Ok(false);
    };
    if !item.actions.iter().any(|action| action.enabled) {
        return Ok(false);
    }
    runtime.set_enabled(panel.kind, item.id.as_str(), !item.enabled)?;
    Ok(true)
}

pub(crate) fn integration_command_specs(
    kind: IntegrationKind,
    items: &[IntegrationItem],
) -> Vec<CommandSpec> {
    if kind == IntegrationKind::Mcp {
        return items
            .iter()
            .flat_map(|server| {
                server.actions.iter().filter_map(|action| {
                    let prompt = action.label.strip_prefix("prompt:")?;
                    let server_name = normalize_command_name(server.id.as_str());
                    let prompt_name = normalize_command_name(prompt);
                    (!server_name.is_empty() && !prompt_name.is_empty()).then(|| CommandSpec {
                        name: format!("mcp__{server_name}__{prompt_name}"),
                        aliases: Vec::new(),
                        summary: format!("MCP prompt: {prompt}"),
                        takes_argument: true,
                        argument_hint: Some("[arguments]".to_string()),
                        source: CommandSource::Mcp,
                        availability: CommandAvailability::Unavailable {
                            reason: action.disabled_reason.clone().unwrap_or_else(|| {
                                "Gateway 仅暴露 prompt 元数据，尚无稳定调用契约".to_string()
                            }),
                        },
                        execution_policy: CommandExecutionPolicy::Queued,
                        action: CommandAction::Gateway(format!(
                            "mcp_prompt:{}/{}",
                            server.id, prompt
                        )),
                    })
                })
            })
            .collect();
    }
    if kind != IntegrationKind::Skill {
        return Vec::new();
    }
    items
        .iter()
        .filter(|item| item.enabled)
        .filter_map(|item| {
            let name = normalize_command_name(item.id.as_str());
            (!name.is_empty()).then(|| CommandSpec {
                name: format!("skill:{name}"),
                aliases: Vec::new(),
                summary: item.detail.clone(),
                takes_argument: true,
                argument_hint: Some("[input]".to_string()),
                source: CommandSource::Skill,
                availability: CommandAvailability::Available,
                execution_policy: CommandExecutionPolicy::Queued,
                action: CommandAction::PromptExpansion(format!("${}", item.id)),
            })
        })
        .collect()
}

fn normalize_command_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

impl IntegrationRuntime for GatewayUiRuntime {
    fn load(&mut self, kind: IntegrationKind) -> Result<Vec<IntegrationItem>, String> {
        if kind == IntegrationKind::Skill {
            let provider = self.skill_provider()?;
            return self.load_skills(provider);
        }
        block_on_gateway(async {
            match kind {
                IntegrationKind::Mcp => self.client().mcp_inspect().await.map(|response| {
                    response
                        .servers
                        .into_iter()
                        .map(|server| {
                            let actions = server
                                .prompts
                                .iter()
                                .filter_map(|prompt| {
                                    prompt.get("name").and_then(serde_json::Value::as_str)
                                })
                                .map(|name| IntegrationAction {
                                    label: format!("prompt:{name}"),
                                    enabled: false,
                                    disabled_reason: Some(
                                        "Gateway 仅暴露 prompt 元数据，尚无稳定调用契约"
                                            .to_string(),
                                    ),
                                })
                                .collect();
                            IntegrationItem {
                                id: server.name.clone(),
                                title: server.name,
                                detail: format!(
                                    "transport={} status={:?} tools={} resources={} prompts={}",
                                    server.transport,
                                    server.connection_status,
                                    server.tool_count.unwrap_or(server.tools.len()),
                                    server.resource_count.unwrap_or(server.resources.len()),
                                    server.prompt_count.unwrap_or(server.prompts.len())
                                ),
                                enabled: server.enabled,
                                actions,
                            }
                        })
                        .collect()
                }),
                IntegrationKind::Skill => unreachable!("skill loading is handled above"),
                IntegrationKind::Plugin | IntegrationKind::Hook => {
                    self.client().plugins_get().await.map(|plugins| {
                        plugins
                            .into_iter()
                            .map(|plugin| IntegrationItem {
                                id: plugin.id,
                                title: plugin.display_name,
                                detail: format!(
                                    "{} · version={} components={}",
                                    plugin.description, plugin.version, plugin.component_count
                                ),
                                enabled: plugin.enabled,
                                actions: vec![IntegrationAction {
                                    label: "启用/停用".to_string(),
                                    enabled: true,
                                    disabled_reason: None,
                                }],
                            })
                            .collect()
                    })
                }
                IntegrationKind::Doctor => {
                    let mcp = self.client().mcp_inspect().await?;
                    let directory = self.directory_value();
                    let skills = self.client().skills_get(directory.as_deref()).await?;
                    let plugins = self.client().plugins_get().await?;
                    Ok(vec![
                        IntegrationItem {
                            id: "gateway".to_string(),
                            title: "Gateway 集成诊断".to_string(),
                            detail: format!(
                                "mcp_servers={} skills={} plugins={}",
                                mcp.servers.len(),
                                skills.len(),
                                plugins.len()
                            ),
                            enabled: true,
                            actions: Vec::new(),
                        },
                    ])
                }
                IntegrationKind::Memory => Ok(Vec::new()),
            }
        })
    }

    fn set_enabled(
        &mut self,
        kind: IntegrationKind,
        id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let directory = self.directory_value();
        block_on_gateway(async {
            match kind {
                IntegrationKind::Skill => {
                    let config = self.client().config_get(directory.as_deref()).await?;
                    let provider = skill_provider_from_config(&config)?;
                    if provider != SkillsDirectoryProvider::Vibewindow {
                        return Err(format!(
                            "{} 技能平台不支持启用/停用",
                            provider.label()
                        ));
                    }
                    self.client()
                        .skill_set_enabled(directory.as_deref(), id, enabled)
                        .await
                        .map(|_| ())
                }
                IntegrationKind::Plugin => {
                    self.client().plugin_set_enabled(id, enabled).await.map(|_| ())
                }
                _ => Err("this integration does not support enable/disable".to_string()),
            }
        })
    }

    fn delete(&mut self, kind: IntegrationKind, id: &str) -> Result<(), String> {
        let directory = self.directory_value();
        block_on_gateway(async {
            match kind {
                IntegrationKind::Skill => self
                    .client()
                    .skill_delete(directory.as_deref(), id)
                    .await
                    .map(|_| ()),
                IntegrationKind::Plugin => self.client().plugin_delete(id).await.map(|_| ()),
                _ => Err("this integration does not support deletion".to_string()),
            }
        })
    }

    fn skill_provider(&mut self) -> Result<SkillsDirectoryProvider, String> {
        let directory = self.directory_value();
        block_on_gateway(async {
            let config = self.client().config_get(directory.as_deref()).await?;
            skill_provider_from_config(&config)
        })
    }

    fn set_skill_provider(
        &mut self,
        provider: SkillsDirectoryProvider,
    ) -> Result<(), String> {
        block_on_gateway(self.client().global_config_patch(&serde_json::json!({
            "skills": {
                "directory_provider": provider
            }
        })))
    }

    fn load_skills(
        &mut self,
        provider: SkillsDirectoryProvider,
    ) -> Result<Vec<IntegrationItem>, String> {
        let directory = self.directory_value();
        block_on_gateway(async {
            self.client().skills_get(directory.as_deref()).await.map(|skills| {
                let can_toggle = provider == SkillsDirectoryProvider::Vibewindow;
                skills
                    .into_iter()
                    .filter(|skill| skill.installed)
                    .map(|skill| IntegrationItem {
                        id: skill.id,
                        title: skill.title,
                        detail: skill.description,
                        enabled: if can_toggle { skill.enabled } else { true },
                        actions: if can_toggle {
                            vec![IntegrationAction {
                                label: "启用/停用".to_string(),
                                enabled: true,
                                disabled_reason: None,
                            }]
                        } else {
                            Vec::new()
                        },
                    })
                    .collect()
            })
        })
    }
}

fn skill_provider_from_config(
    config: &serde_json::Value,
) -> Result<SkillsDirectoryProvider, String> {
    let Some(value) =
        config.get("skills").and_then(|skills| skills.get("directory_provider"))
    else {
        return Ok(SkillsDirectoryProvider::default());
    };
    let raw =
        value.as_str().ok_or_else(|| "skills.directory_provider must be a string".to_string())?;
    parse_skills_directory_provider(raw)
        .ok_or_else(|| format!("unsupported skills.directory_provider: {}", raw.trim()))
}
