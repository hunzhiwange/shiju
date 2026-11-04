//! 默认配置草稿、候选与保存校验；实际权限上限仍由网关执行。
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::{App, Message, state::DelegateAgentSettingsEntry};
use iced::{
    Element,
    widget::{checkbox, column, pick_list, text},
};
use vw_config_types::agent::{AgentDefault, AgentDefaults, AgentModelReference, AgentRuntime};

pub(crate) fn patch(defaults: &AgentDefaults) -> serde_json::Value {
    let mut value = serde_json::to_value(defaults).expect("typed defaults serialize");
    // Merge Patch must remove a previous fixed payload when selecting inheritance.
    for field in value.as_object_mut().expect("defaults object").values_mut() {
        if field["mode"] == "inherit" {
            field["value"] = serde_json::Value::Null;
        }
    }
    value
}

pub(crate) fn validate(
    runtime: &AgentRuntime,
    defaults: &AgentDefaults,
    models: &[AgentModelReference],
    skills: &[String],
    mcp: &[String],
) -> Result<(), String> {
    if let AgentDefault::Fixed(model) = &defaults.model {
        if !models.contains(model) {
            return Err("模型不可用，或当前引擎尚未提供此模型来源能力，请重新选择或继承。".into());
        }
    }
    if let AgentDefault::Fixed(permission) = &defaults.permission {
        let supported = if matches!(runtime, AgentRuntime::Acp { .. }) {
            permission == &serde_json::json!({"*":"deny"})
        } else {
            permission.as_object().is_some_and(|rules| {
                rules.values().all(|value| {
                    matches!(value.as_str(), Some("allow" | "ask" | "deny"))
                        || value.as_object().is_some_and(|patterns| {
                            patterns.values().all(|action| {
                                matches!(action.as_str(), Some("allow" | "ask" | "deny"))
                            })
                        })
                })
            })
        };
        if !supported {
            return Err("当前引擎不支持此固定权限；ACP 仅支持全部拒绝。请修正或选择继承。".into());
        }
    }
    for (selection, available, label) in
        [(&defaults.skills, skills, "技能"), (&defaults.mcp, mcp, "MCP")]
    {
        if let AgentDefault::Fixed(ids) = selection {
            if let Some(id) = ids.iter().find(|id| !available.contains(id)) {
                return Err(format!(
                    "{label} {id} 未知、已禁用或不在当前作用域，请移除或选择继承。"
                ));
            }
        }
    }
    Ok(())
}

fn catalogs(
    app: &App,
    runtime: &AgentRuntime,
) -> (Vec<AgentModelReference>, Vec<String>, Vec<String>) {
    let model_source_available = match runtime {
        AgentRuntime::Native => true,
        AgentRuntime::Acp { backend } => app.session_runtime_states.values().any(|state|
            state.acp_agent.as_ref() == Some(backend) && state.acp_options.connected &&
            state.acp_options.model_source_capabilities.iter().any(|capability| matches!(capability,
                vw_gateway_client::vw_api_types::acp_model_source::AcpModelSourceCapability::Supported {
                    source: vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource::SessionModel
                }))),
    };
    let models = app
        .agents_settings
        .provider_models
        .iter()
        .filter(|provider| {
            model_source_available
                && app.agents_settings.providers.iter().any(|p| p.id == provider.id && p.connected)
        })
        .flat_map(|provider| {
            provider.models.iter().filter(|model| model.enabled).map(|model| AgentModelReference {
                provider: provider.id.clone(),
                model: model.id.clone(),
            })
        })
        .collect();
    let skills = app
        .skills_settings
        .catalog
        .iter()
        .filter(|skill| skill.enabled && app.agents_settings.project_path == app.project_path)
        .map(|skill| skill.id.clone())
        .collect();
    let mcp = app
        .mcp_settings
        .inspect
        .as_ref()
        .map(|inspect| {
            inspect
                .servers
                .iter()
                .filter(|server| server.enabled)
                .map(|server| server.name.clone())
                .collect()
        })
        .unwrap_or_default();
    (models, skills, mcp)
}

pub(crate) fn validation_error(app: &App, draft: &DelegateAgentSettingsEntry) -> Option<String> {
    if draft.force_model_selection {
        if let AgentDefault::Fixed(model) = &draft.defaults.model {
            if model.provider != draft.provider || model.model != draft.model {
                return Some("默认模型与高级设置中的强制模型绑定冲突，请修正默认模型。".into());
            }
        }
    }
    let (models, skills, mcp) = catalogs(app, &draft.runtime);
    validate(&draft.runtime, &draft.defaults, &models, &skills, &mcp).err()
}

fn changed(defaults: AgentDefaults) -> Message {
    Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileDefaultsChanged(defaults)))
}

pub(super) fn view<'a>(
    app: &'a App,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let (models, skills, mcp) = catalogs(app, &draft.runtime);
    let defaults = &draft.defaults;
    let mut body = column![
        text("默认配置").size(16),
        text("固定空列表表示不选择任何条目；继承使用当前环境默认。权限始终受安全上限约束。")
            .style(settings_muted_text_style)
    ]
    .spacing(10);
    let mut model_options = vec!["继承".to_string()];
    model_options.extend(models.iter().map(|model| format!("{}/{}", model.provider, model.model)));
    let selected = match &defaults.model {
        AgentDefault::Inherit => "继承".into(),
        AgentDefault::Fixed(model) => format!("{}/{}", model.provider, model.model),
    };
    let original = defaults.clone();
    body = body.push(text("默认模型")).push(
        pick_list(model_options, Some(selected), move |value| {
            let mut next = original.clone();
            next.model = models
                .iter()
                .find(|model| format!("{}/{}", model.provider, model.model) == value)
                .cloned()
                .map(AgentDefault::Fixed)
                .unwrap_or_default();
            changed(next)
        })
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
    );
    if matches!(draft.runtime, AgentRuntime::Acp { .. }) {
        body = body.push(
            text("ACP 模型仅在该引擎已声明会话模型来源能力时可选；未连接时请使用继承。")
                .style(settings_muted_text_style),
        );
    }
    let mut policies = vec!["继承", "全部拒绝"];
    if matches!(draft.runtime, AgentRuntime::Native) {
        policies.extend(["逐项询问", "允许（受安全上限约束）"]);
    }
    let selected = match &defaults.permission {
        AgentDefault::Inherit => "继承",
        AgentDefault::Fixed(value) if value == &serde_json::json!({"*":"deny"}) => "全部拒绝",
        AgentDefault::Fixed(value) if value == &serde_json::json!({"*":"ask"}) => "逐项询问",
        AgentDefault::Fixed(value) if value == &serde_json::json!({"*":"allow"}) => {
            "允许（受安全上限约束）"
        }
        _ => "已有自定义策略",
    };
    let original = defaults.clone();
    body = body.push(text("默认权限")).push(
        pick_list(policies, Some(selected), move |value| {
            let mut next = original.clone();
            next.permission = match value {
                "全部拒绝" => AgentDefault::Fixed(serde_json::json!({"*":"deny"})),
                "逐项询问" => AgentDefault::Fixed(serde_json::json!({"*":"ask"})),
                "允许（受安全上限约束）" => {
                    AgentDefault::Fixed(serde_json::json!({"*":"allow"}))
                }
                _ => AgentDefault::Inherit,
            };
            changed(next)
        })
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
    );
    for (is_skill, title, selection, available) in
        [(true, "默认技能", &defaults.skills, skills), (false, "默认 MCP", &defaults.mcp, mcp)]
    {
        let fixed = matches!(selection, AgentDefault::Fixed(_));
        let original = defaults.clone();
        body = body.push(text(title)).push(
            pick_list(
                vec!["继承", "固定选择"],
                Some(if fixed { "固定选择" } else { "继承" }),
                move |mode| {
                    let mut next = original.clone();
                    let value = if mode == "固定选择" {
                        AgentDefault::Fixed(vec![])
                    } else {
                        AgentDefault::Inherit
                    };
                    if is_skill {
                        next.skills = value;
                    } else {
                        next.mcp = value;
                    }
                    changed(next)
                },
            )
            .padding(SETTINGS_CONTROL_PADDING)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style),
        );
        if let AgentDefault::Fixed(selected) = selection {
            if available.is_empty() {
                body = body.push(text("尚无可用候选；可保存固定空列表，或先加载当前作用域目录。"));
            }
            let mut ids = available.clone();
            ids.extend(selected.iter().filter(|id| !available.contains(id)).cloned());
            for id in ids {
                let enabled = available.contains(&id);
                let label = if enabled {
                    id.clone()
                } else {
                    format!("{id}（不可用，取消选择后可保存）")
                };
                let checked = selected.contains(&id);
                let original = defaults.clone();
                body = body.push(
                    checkbox(checked)
                        .label(label)
                        .on_toggle(move |checked| {
                            let mut next = original.clone();
                            let target = if is_skill { &mut next.skills } else { &mut next.mcp };
                            if let AgentDefault::Fixed(ids) = target {
                                ids.retain(|value| value != &id);
                                if checked && enabled {
                                    ids.push(id.clone());
                                }
                            }
                            changed(next)
                        })
                        .style(settings_checkbox_style),
                );
            }
        }
    }
    if let Some(error) = validation_error(app, draft) {
        body = body.push(settings_error_banner(error));
    }
    body.into()
}
