//! 默认配置草稿、候选与保存校验；实际权限上限仍由网关执行。
use crate::app::components::system_settings_common::*;
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::{App, Message, state::DelegateAgentSettingsEntry};
use iced::{
    Element, Length,
    widget::{checkbox, column, responsive, text},
};
use std::fmt;
use vw_config_types::agent::{AgentDefault, AgentDefaults, AgentModelReference, AgentRuntime};
use vw_config_types::ui::DesktopLocale;
use vw_gateway_client::vw_api_types::acp_model_source::{AcpModelSource, AcpModelSourceCapability};

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

fn supports_fixed_model(app: &App, runtime: &AgentRuntime) -> bool {
    match runtime {
        AgentRuntime::Native => true,
        AgentRuntime::Acp { backend } => app.session_runtime_states.values().any(|state| {
            state.acp_agent.as_ref() == Some(backend)
                && state.acp_options.model_source_capabilities.contains(
                    &AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel },
                )
        }),
    }
}

fn catalogs(
    app: &App,
    model_source_available: bool,
) -> (Vec<AgentModelReference>, Vec<String>, Vec<String>) {
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
    let (models, skills, mcp) = catalogs(app, supports_fixed_model(app, &draft.runtime));
    validate(&draft.runtime, &draft.defaults, &models, &skills, &mcp).err()
}

fn changed(defaults: AgentDefaults) -> Message {
    Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileDefaultsChanged(defaults)))
}

pub(super) fn selection_mode(
    locale: DesktopLocale,
    fixed: bool,
    on_selected: impl Fn(bool) -> Message + 'static,
) -> Element<'static, Message> {
    let inherit = tr(locale, "继承");
    let explicit = tr(locale, "固定选择");
    settings_select(
        [inherit, explicit],
        Some(if fixed { explicit } else { inherit }),
        move |value| on_selected(value == explicit),
    )
    .width(Length::Fill)
    .into()
}

#[derive(Clone, PartialEq)]
enum ModelChoice {
    Inherit(&'static str),
    Fixed(AgentModelReference),
}

impl fmt::Display for ModelChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inherit(label) => f.write_str(label),
            Self::Fixed(model) => write!(f, "{}/{}", model.provider, model.model),
        }
    }
}

pub(super) fn view<'a>(
    app: &'a App,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let model_source_available = supports_fixed_model(app, &draft.runtime);
    let (models, _, _) = catalogs(app, model_source_available);
    let i18n = I18n::new(app.desktop_locale);
    let theme = &app.app_theme;
    let defaults = &draft.defaults;
    let label = |source| tr(app.desktop_locale, source);
    let inherit = ModelChoice::Inherit(label("继承会话模型"));
    let selected = match &defaults.model {
        AgentDefault::Inherit => inherit.clone(),
        AgentDefault::Fixed(model) => ModelChoice::Fixed(model.clone()),
    };
    let model_label = label("模型");
    let model_control = move || {
        let options =
            std::iter::once(inherit.clone()).chain(models.iter().cloned().map(ModelChoice::Fixed));
        let select = settings_select(options, Some(selected.clone()), |value| {
            let model = match value {
                ModelChoice::Inherit(_) => AgentDefault::Inherit,
                ModelChoice::Fixed(model) => AgentDefault::Fixed(model),
            };
            Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileModelChanged(model)))
        })
        .width(Length::Fill);
        let force_model = checkbox(draft.force_model_selection)
            .label(label("强制使用所选模型"))
            .on_toggle_maybe(matches!(defaults.model, AgentDefault::Fixed(_)).then(|| {
                let key = draft.key.clone();
                move |value| {
                    Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::ForceModelSelectionToggled(key.clone(), value),
                    ))
                }
            }))
            .style(settings_checkbox_style);
        column![
            select,
            force_model,
            (!model_source_available).then(|| {
                text(label("仅已声明会话模型能力的引擎可固定模型。"))
                    .size(12)
                    .style(settings_muted_text_style)
            }),
        ]
        .spacing(8)
        .width(Length::Fill)
    };
    let model = if matches!(draft.runtime, AgentRuntime::Acp { .. }) {
        let description = label(
            "会话模型来源下，显式选择优先于智能体默认；继承会话模型在无选择时使用环境默认。CLI 本地配置是独立来源，由会话单独选择。",
        );
        responsive(move |size| {
            if size.width >= 600.0 {
                settings_flexible_row_with_help(
                    i18n,
                    theme,
                    model_label,
                    description,
                    460.0,
                    model_control(),
                )
                .padding([6, 0])
                .into()
            } else {
                column![
                    settings_hover_help_label(
                        i18n,
                        theme,
                        model_label,
                        settings_field_label(model_label),
                        description,
                    )
                    .wrap(),
                    model_control(),
                ]
                .spacing(6)
                .width(Length::Fill)
                .into()
            }
        })
        .height(Length::Shrink)
        .into()
    } else {
        settings_responsive_row(model_label, "", model_control)
    };
    let permission = settings_responsive_row(label("权限"), "", move || {
        checkbox(defaults.full_access)
            .label(label("完全访问权限"))
            .on_toggle(move |enabled| {
                let mut next = defaults.clone();
                next.full_access = enabled;
                changed(next)
            })
            .style(settings_checkbox_style)
    });
    column![
        column![
            settings_group_heading_with_help(
                i18n,
                theme,
                label("默认配置"),
                label("固定空列表表示不选择任何条目。"),
            ),
            text(label("权限始终受安全上限约束。")).size(13).style(settings_muted_text_style),
        ]
        .spacing(6),
        settings_group_card(column![model, settings_divider(), permission].spacing(12))
            .padding([14, 16]),
        validation_error(app, draft).map(settings_error_banner),
    ]
    .spacing(8)
    .into()
}
