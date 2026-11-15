use crate::help::{settings_hover_help_label, settings_group_heading_with_help, settings_flexible_row_with_help};
pub fn patch(defaults: &AgentDefaults) -> serde_json::Value {
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

pub fn validate(
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

use crate::message::{Event, AgentsMessage};
use crate::state::*;
// 默认配置草稿、候选与保存校验；实际权限上限仍由网关执行。
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::{I18n, settings_extensions::text as tr};
use iced::{
    Element, Length,
    widget::{checkbox, column, responsive, text},
};
use std::fmt;
use config_agent_profile::{AgentDefault, AgentDefaults, AgentModelReference, AgentRuntime};
use config_ui::DesktopLocale;

pub fn validation_error(input: &Input<'_>, draft: &DelegateAgentSettingsEntry) -> Option<String> {
    if draft.skill_policy == config_agent::DelegateCapabilityPolicy::Strict
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
        && (!input.mcp_ready)
    {
        return Some("MCP 服务目录尚未就绪或加载失败，请刷新后再保存已有选择。".into());
    }
    validate(&draft.runtime, &draft.defaults, &input.models, &input.skills, &input.mcp).err()
}

fn changed(defaults: AgentDefaults) -> Event {
    Event::Agents(AgentsMessage::ProfileDefaultsChanged(defaults))
}

pub fn selection_mode<Message: Clone + 'static>(
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

pub fn view<'a>(
    input: Input<'a>,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Event> {
    let error = validation_error(&input, draft);
    let model_source_available = input.model_source_available;
    let models = input.models;
    let i18n = I18n::new(input.locale);
    let theme = input.theme;
    let defaults = &draft.defaults;
    let locale = input.locale;
    let label = move |source| tr(locale, source);
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
            Event::Agents(AgentsMessage::ProfileModelChanged(model))
        })
        .width(Length::Fill);
        let force_model = checkbox(draft.force_model_selection)
            .label(label("强制使用所选模型"))
            .on_toggle_maybe(matches!(defaults.model, AgentDefault::Fixed(_)).then(|| {
                let key = draft.key.clone();
                move |value| {
                    Event::Agents(
                        AgentsMessage::ForceModelSelectionToggled(key.clone(), value),
                    )
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
                settings_flexible_row_with_help(input.help,
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
                    settings_hover_help_label(input.help,
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
            settings_group_heading_with_help(input.help,
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
        error.map(settings_error_banner),
    ]
    .spacing(8)
    .into()
}

/// Borrowed projections keep provider and skill ownership in their existing domains.
pub fn catalogs<'a>(
    model_source_available: bool,
    models: impl IntoIterator<Item = (&'a str, &'a str, bool, bool)>,
    skills: impl IntoIterator<Item = (&'a str, bool, bool)>,
    inspect: Option<&vw_api_mcp_inspect::McpInspectResponse>,
) -> (Vec<AgentModelReference>, Vec<String>, Vec<String>) {
    let models = models.into_iter().filter(|(_, _, enabled, connected)|
        model_source_available && *enabled && *connected
    ).map(|(provider, model, _, _)| AgentModelReference {
        provider: provider.to_owned(), model: model.to_owned(),
    }).collect();
    let skills = skills.into_iter().filter(|(_, installed, enabled)| *installed && *enabled)
        .map(|(id, _, _)| id.to_owned()).collect();
    let mcp = inspect.map(|inspect| inspect.servers.iter().filter(|server| server.enabled)
        .map(|server| server.name.clone()).collect()).unwrap_or_default();
    (models, skills, mcp)
}
pub struct Input<'a> {
    pub help: crate::help::Renderer,
    pub locale: DesktopLocale,
    pub theme: &'a iced::Theme,
    pub model_source_available: bool,
    pub mcp_ready: bool,
    pub models: Vec<AgentModelReference>,
    pub skills: Vec<String>,
    pub mcp: Vec<String>,
}

#[cfg(test)] mod tests;

pub fn supports_fixed_model<'a>(
    runtime: &AgentRuntime,
    states: impl IntoIterator<Item = (Option<&'a str>, &'a [vw_api_acp_model::acp_model_source::AcpModelSourceCapability])>,
) -> bool {
    use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpModelSourceCapability};
    match runtime {
        AgentRuntime::Native => true,
        AgentRuntime::Acp { backend } => states.into_iter().any(|(agent, capabilities)| {
            agent == Some(backend.as_str()) && capabilities.contains(
                &AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel },
            )
        }),
    }
}
