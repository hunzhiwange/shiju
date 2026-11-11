//! 通道“工作项目”下的智能体与模型选择。
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style, settings_responsive_row, settings_select,
};
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, column, text};
use iced::{Element, Length};
use vw_config_types::agent::AgentModelReference;

#[derive(Clone, PartialEq, Eq)]
struct NamedOption {
    id: Option<String>,
    label: String,
}
impl std::fmt::Display for NamedOption {
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
    let Some(choices) = state.execution_choices.get(&instance) else {
        let failed = state.execution_error.contains_key(&instance);
        let status = tr(app.desktop_locale, if failed { "加载失败" } else { "加载中…" });
        let mut content = column![].spacing(8);
        for label in ["智能体", "模型提供商", "模型"] {
            content = content.push(settings_responsive_row(
                tr(app.desktop_locale, label),
                "",
                move || text(status).size(13).style(settings_muted_text_style),
            ));
        }
        if failed {
            content = content.push(
                button(text(tr(app.desktop_locale, "重试")).size(13))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(Message::Settings(SettingsMessage::Channels(
                        ChannelsMessage::ExecutionChoicesRequested(instance),
                    ))),
            );
        }
        return content.into();
    };
    let mut agents =
        vec![NamedOption { id: None, label: tr(app.desktop_locale, "默认智能体").into() }];
    agents.extend(
        choices
            .agents
            .iter()
            .map(|(id, label)| NamedOption { id: Some(id.clone()), label: label.clone() }),
    );
    let selected_agent = binding.and_then(|binding| binding.agent.as_ref());
    let selected_agent =
        agents.iter().find(|option| option.id.as_ref() == selected_agent).cloned().unwrap_or_else(
            || NamedOption {
                id: selected_agent.cloned(),
                label: format!(
                    "{}：{}",
                    tr(app.desktop_locale, "已绑定"),
                    selected_agent.map(String::as_str).unwrap_or_default()
                ),
            },
        );
    let selected_provider_id = state
        .execution_provider
        .get(&instance)
        .or_else(|| binding.and_then(|binding| binding.model.as_ref()).map(|model| &model.provider))
        .cloned();
    let mut providers = vec![NamedOption {
        id: None,
        label: tr(app.desktop_locale, "跟随智能体默认模型").into(),
    }];
    let provider_ids = choices
        .models
        .iter()
        .map(|model| model.provider.clone())
        .collect::<std::collections::BTreeSet<_>>();
    providers
        .extend(provider_ids.into_iter().map(|id| NamedOption { label: id.clone(), id: Some(id) }));
    let selected_provider =
        providers.iter().find(|option| option.id == selected_provider_id).cloned().unwrap_or_else(
            || NamedOption {
                id: selected_provider_id.clone(),
                label: selected_provider_id.clone().unwrap_or_default(),
            },
        );
    let mut models = vec![ModelOption {
        model: None,
        label: tr(app.desktop_locale, "跟随智能体默认模型").into(),
    }];
    models.extend(
        choices
            .models
            .iter()
            .filter(|model| Some(&model.provider) == selected_provider_id.as_ref())
            .map(|model| ModelOption { model: Some(model.clone()), label: model.model.clone() }),
    );
    let selected_model = binding.and_then(|binding| binding.model.as_ref());
    let selected_model = models
        .iter()
        .find(|option| option.model.as_ref() == selected_model)
        .cloned()
        .unwrap_or_else(|| ModelOption {
            model: selected_model.cloned(),
            label: selected_model.map(|model| model.model.clone()).unwrap_or_default(),
        });
    let agent_instance = instance.clone();
    let provider_instance = instance.clone();
    let model_instance = instance.clone();
    let agent = settings_responsive_row(
        tr(app.desktop_locale, "智能体"),
        tr(app.desktop_locale, "默认使用 Main，套用智能体配置中的模型和参数。"),
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
        },
    );
    let provider = settings_responsive_row(
        tr(app.desktop_locale, "模型提供商"),
        tr(app.desktop_locale, "与智能体配置中的模型选择一致。"),
        move || {
            let key = provider_instance.clone();
            settings_select(providers.clone(), Some(selected_provider.clone()), move |option| {
                Message::Settings(SettingsMessage::Channels(
                    ChannelsMessage::ExecutionProviderSelected {
                        instance: key.clone(),
                        provider: option.id,
                    },
                ))
            })
            .width(Length::Fill)
        },
    );
    let model = settings_responsive_row(
        tr(app.desktop_locale, "模型"),
        tr(app.desktop_locale, "从当前提供商的模型列表选择，留空使用智能体默认模型。"),
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
            .disabled(selected_provider_id.is_none())
        },
    );
    let content = column![
        agent,
        provider,
        model,
        text(tr(app.desktop_locale, "更换智能体或模型后，请在通道中新建会话（/new）。"))
            .size(12)
            .style(settings_muted_text_style)
    ]
    .spacing(8);
    content.into()
}
