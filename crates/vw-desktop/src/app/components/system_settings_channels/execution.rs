//! 通道“工作项目”下的智能体与模型选择。
use crate::app::components::system_settings_common::{
    settings_muted_text_style, settings_responsive_row, settings_select,
};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{column, text};
use iced::{Element, Length};
use vw_config_types::agent::AgentModelReference;

#[derive(Clone, PartialEq, Eq)]
struct AgentOption {
    id: Option<String>,
    label: String,
}
impl std::fmt::Display for AgentOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
#[derive(Clone, PartialEq, Eq)]
struct ModelOption {
    model: Option<AgentModelReference>,
    label: String,
}
impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub(super) fn selectors(app: &App, instance: String) -> Element<'_, Message> {
    let state = &app.channels_settings;
    let binding = state.instance_execution.get(&instance);
    let choices = state
        .execution_choices
        .as_ref()
        .filter(|(key, _)| key == &instance)
        .map(|(_, choices)| choices);
    let mut agents =
        vec![AgentOption { id: None, label: tr(app.desktop_locale, "默认智能体").into() }];
    if let Some(choices) = choices {
        agents.extend(
            choices
                .agents
                .iter()
                .map(|(id, label)| AgentOption { id: Some(id.clone()), label: label.clone() }),
        );
    }
    let selected_agent = binding.and_then(|binding| binding.agent.as_ref());
    let selected_agent =
        agents.iter().find(|option| option.id.as_ref() == selected_agent).cloned().unwrap_or_else(
            || AgentOption {
                id: selected_agent.cloned(),
                label: format!(
                    "{}：{}",
                    tr(app.desktop_locale, "已绑定"),
                    selected_agent.map(String::as_str).unwrap_or_default()
                ),
            },
        );
    let mut models = vec![ModelOption {
        model: None,
        label: tr(app.desktop_locale, "跟随智能体默认模型").into(),
    }];
    if let Some(choices) = choices {
        models.extend(choices.models.iter().map(|model| ModelOption {
            model: Some(model.clone()),
            label: format!("{}/{}", model.provider, model.model),
        }));
    }
    let selected_model = binding.and_then(|binding| binding.model.as_ref());
    let selected_model = models
        .iter()
        .find(|option| option.model.as_ref() == selected_model)
        .cloned()
        .unwrap_or_else(|| ModelOption {
            model: selected_model.cloned(),
            label: selected_model
                .map(|model| format!("{}/{}", model.provider, model.model))
                .unwrap_or_default(),
        });
    let agent_instance = instance.clone();
    let model_instance = instance.clone();
    let agent = settings_responsive_row(
        tr(app.desktop_locale, "智能体"),
        tr(app.desktop_locale, "选择处理此通道任务的智能体，可使用不同的 ACP。"),
        move || {
            let key = agent_instance.clone();
            settings_select(agents.clone(), Some(selected_agent.clone()), move |option| {
                Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::ExecutionAgentSelected {
                        instance: key.clone(),
                        agent: option.id,
                    },
                ))
            })
            .width(Length::Fill)
            .reload_on_open(
                format!("channel-agent:{agent_instance}"),
                Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::ExecutionChoicesRequested(agent_instance.clone()),
                )),
            )
        },
    );
    let model = settings_responsive_row(
        tr(app.desktop_locale, "对话模型"),
        tr(app.desktop_locale, "用于此通道的对话；ACP 未指定模型时使用智能体默认模型。"),
        move || {
            let key = model_instance.clone();
            settings_select(models.clone(), Some(selected_model.clone()), move |option| {
                Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::ExecutionModelSelected {
                        instance: key.clone(),
                        model: option.model,
                    },
                ))
            })
            .width(Length::Fill)
            .reload_on_open(
                format!("channel-model:{model_instance}"),
                Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::ExecutionChoicesRequested(model_instance.clone()),
                )),
            )
        },
    );
    let mut content = column![
        agent,
        model,
        text(tr(app.desktop_locale, "更换智能体或模型后，请在通道中新建会话（/new）。"))
            .size(12)
            .style(settings_muted_text_style)
    ]
    .spacing(8);
    if state.execution_request.as_ref().is_some_and(|(_, key, _)| key == &instance)
        && state.execution_error.is_some()
    {
        content = content.push(
            text(tr(app.desktop_locale, "执行候选加载失败，请重新打开下拉框重试。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    content.into()
}
