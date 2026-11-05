//! 默认配置草稿、候选与保存校验；实际权限上限仍由网关执行。
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::{App, Message, state::DelegateAgentSettingsEntry};
use iced::{
    Alignment, Element, Length,
    widget::{checkbox, column, pick_list, row, text},
};
use vw_config_types::agent::{AgentDefault, AgentDefaults, AgentModelReference, AgentRuntime};

pub(crate) fn patch(defaults: &AgentDefaults) -> serde_json::Value {
    let mut value = serde_json::to_value(defaults).expect("typed defaults serialize");
    // Merge Patch must remove a previous fixed payload when selecting inheritance.
    for key in ["model", "permission", "skills", "mcp"] {
        let field = &mut value[key];
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
        .agents_settings
        .skill_catalog
        .iter()
        .filter(|skill| skill.installed && skill.enabled)
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
    if draft.skill_policy == vw_config_types::agent::DelegateCapabilityPolicy::Strict
        && let AgentDefault::Fixed(ids) = &draft.defaults.skills
        && ids.iter().any(|id| !draft.allowed_skills.contains(id))
    {
        return Some("默认技能超出严格访问范围，请移除越界选择。".into());
    }
    if draft.force_model_selection
        && !matches!(&draft.defaults.model,
        AgentDefault::Fixed(model) if model.provider == draft.provider && model.model == draft.model)
    {
        return Some("默认模型与强制模型绑定冲突，请在模型控件中重新选择。".into());
    }
    if let AgentDefault::Fixed(ids) = &draft.defaults.mcp
        && !ids.is_empty()
        && (app.mcp_settings.inspect_loading
            || app.mcp_settings.inspect_error.is_some()
            || app.mcp_settings.inspect.is_none())
    {
        return Some("MCP 服务目录尚未就绪或加载失败，请刷新后再保存已有选择。".into());
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
    let (models, _, _) = catalogs(app, &draft.runtime);
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
    let mut model_controls = column![
        pick_list(model_options, Some(selected), move |value| {
            let model = models
                .iter()
                .find(|model| format!("{}/{}", model.provider, model.model) == value)
                .cloned()
                .map(AgentDefault::Fixed)
                .unwrap_or_default();
            Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileModelChanged(model)))
        })
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill),
    ]
    .spacing(10)
    .width(Length::Fill);
    model_controls = model_controls.push(
        checkbox(draft.force_model_selection)
            .label("强制使用所选模型")
            .on_toggle_maybe(matches!(defaults.model, AgentDefault::Fixed(_)).then(|| {
                let key = draft.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::ForceModelSelectionToggled(key.clone(), value),
                    ))
                }
            }))
            .style(settings_checkbox_style),
    );
    if matches!(draft.runtime, AgentRuntime::Acp { .. }) {
        model_controls = model_controls.push(
            text("ACP 模型仅在该引擎已声明会话模型来源能力时可选；未连接时请使用继承。")
                .style(settings_muted_text_style),
        );
    }
    body = body.push(
        row![text("模型").width(Length::Fixed(SETTINGS_LABEL_WIDTH)), model_controls]
            .spacing(16)
            .align_y(Alignment::Start),
    );
    let original = defaults.clone();
    body = body.push(
        row![
            text("权限").width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            checkbox(defaults.full_access)
                .label("完全访问权限")
                .on_toggle(move |enabled| {
                    let mut next = original.clone();
                    next.full_access = enabled;
                    changed(next)
                })
                .style(settings_checkbox_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    );
    if let Some(error) = validation_error(app, draft) {
        body = body.push(settings_error_banner(error));
    }
    body.into()
}
