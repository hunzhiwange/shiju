//! 基础参数中的资料字段；描述编辑缓冲不参与持久化。
use crate::app::components::system_settings_common::*;
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
    state::DelegateAgentSettingsEntry,
};
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Alignment, Element, Length};
use vw_config_types::agent::{AgentRuntime, builtin_agent_spec};

#[cfg(test)]
#[path = "basic_profile_ui_refresh_15_tests.rs"]
mod ui_refresh_15_tests;

fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}
pub(super) fn view<'a>(
    app: &'a App,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let label = |source| tr(app.desktop_locale, source);
    let fixed = builtin_agent_spec(&draft.key).is_some()
        || draft.key.starts_with("acp-")
        || app.agents_settings.entries.iter().any(|entry| entry.key == draft.key);
    let input = |placeholder, value, callback: fn(String) -> AgentsMessage| {
        settings_text_input(placeholder, value).on_input(move |value| message(callback(value)))
    };
    let name = settings_responsive_row(label("名称"), "", move || {
        input(label("智能体名称"), &draft.label, AgentsMessage::ProfileNameChanged)
            .width(Length::Fill)
    });
    let description = column![
        settings_field_label(label("描述")),
        settings_text_editor(&draft.description_editor)
            .on_action(|action| message(AgentsMessage::ProfileDescriptionAction(action)))
            .font(iced::Font::DEFAULT)
            .height(120),
    ]
    .spacing(6);
    let mut prompts =
        column![settings_field_label(label("推荐提示词"))].spacing(10).width(Length::Fill);
    let prompt_count = draft.recommended_prompts.len();
    let locale = app.desktop_locale;
    for (index, prompt) in draft.recommended_prompts.iter().enumerate() {
        prompts = prompts.push(
            responsive(move |size| {
                let input = settings_text_input(tr(locale, "推荐提示词"), prompt)
                    .on_input(move |value| {
                        message(AgentsMessage::ProfilePromptChanged(index, value))
                    })
                    .width(Length::Fill);
                let actions = row![
                    action(
                        tr(locale, "上移"),
                        (index > 0).then_some(AgentsMessage::ProfilePromptMove(
                            index,
                            index.saturating_sub(1)
                        ))
                    ),
                    action(
                        tr(locale, "下移"),
                        (index + 1 < prompt_count)
                            .then_some(AgentsMessage::ProfilePromptMove(index, index + 1))
                    ),
                    action(tr(locale, "移除"), Some(AgentsMessage::ProfilePromptRemove(index))),
                ]
                .spacing(6);
                if size.width < 440.0 {
                    column![input, actions.wrap()].spacing(6).into()
                } else {
                    row![input, actions].spacing(6).align_y(Alignment::Center).into()
                }
            })
            .height(Length::Shrink),
        );
    }
    prompts = prompts.push(action(label("添加提示词"), Some(AgentsMessage::ProfilePromptAdd)));
    let runtime = match &draft.runtime {
        AgentRuntime::Native => "native".to_string(),
        AgentRuntime::Acp { backend } => backend.clone(),
    };
    let runtime = settings_responsive_row(
        label("运行智能体"),
        label("首次保存成功后，运行引擎及 ACP backend 不可修改。"),
        move || -> Element<'_, Message> {
            if fixed {
                text(
                    crate::__!(I18n::new(app.desktop_locale), "固定运行引擎：%s", runtime.as_str())
                        .expect("valid fixed agent runtime template"),
                )
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .wrapping(text::Wrapping::WordOrGlyph)
                .into()
            } else {
                let mut options = vec!["native".to_string()];
                options.extend(app.acp_agents.clone());
                let busy = app.agents_settings.profile_saving;
                let picker = settings_select(options, Some(runtime.clone()), |value| {
                    message(AgentsMessage::ProfileRuntimeChanged(value))
                })
                .width(Length::Fill);
                let mut controls = column![picker].spacing(8);
                if app.acp_agents.is_empty() {
                    controls = controls.push(
                        text(label(if app.acp_settings.loading {
                            "正在加载 ACP 引擎目录…"
                        } else {
                            "暂无可用 ACP 引擎，可刷新目录或在 ACP 设置中启用。"
                        }))
                        .size(12)
                        .style(settings_muted_text_style),
                    );
                }
                if let Some(error) = &app.acp_settings.save_error {
                    controls = controls.push(settings_error_banner(error.clone()));
                }
                controls = controls.push(
                    button(
                        text(label(if app.acp_settings.loading {
                            "加载中…"
                        } else {
                            "刷新引擎目录"
                        }))
                        .size(SETTINGS_CONTROL_TEXT_SIZE),
                    )
                    .on_press_maybe((!busy && !app.acp_settings.loading).then_some(
                        Message::Settings(SettingsMessage::Acp(
                            crate::app::message::settings::AcpMessage::Refresh,
                        )),
                    ))
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
                );
                controls.into()
            }
        },
    );
    settings_group_card(
        container(column![name, description, prompts, settings_divider(), runtime].spacing(14))
            .padding([14, 0]),
    )
    .padding([0, 16])
    .into()
}
fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Message> {
    button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
        .on_press_maybe(value.map(message))
        .padding([8, 10])
        .style(rounded_action_btn_style)
        .into()
}
