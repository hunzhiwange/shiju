//! 基础参数中的资料字段；描述编辑缓冲不参与持久化。
use crate::app::components::system_settings_common::*;
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
    state::DelegateAgentSettingsEntry,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, pick_list, row, text, text_editor, text_input},
};
use vw_config_types::agent::{AgentRuntime, builtin_agent_spec};
fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}
pub(super) fn view<'a>(
    app: &'a App,
    draft: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let fixed = builtin_agent_spec(&draft.key).is_some()
        || draft.key.starts_with("acp-")
        || app.agents_settings.entries.iter().any(|entry| entry.key == draft.key);
    let input = |placeholder, value, callback: fn(String) -> AgentsMessage| {
        text_input(placeholder, value)
            .on_input(move |value| message(callback(value)))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
    };
    let name = row![
        text("名称").width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        input("智能体名称", &draft.label, AgentsMessage::ProfileNameChanged).width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center);
    let description = row![
        text("描述").width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text_editor(&draft.description_editor)
            .on_action(|action| message(AgentsMessage::ProfileDescriptionAction(action)))
            .font(iced::Font::DEFAULT)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .height(120)
            .style(settings_text_editor_style),
    ]
    .spacing(16)
    .align_y(Alignment::Start);
    let mut prompts = column![].spacing(10).width(Length::Fill);
    for (index, prompt) in draft.recommended_prompts.iter().enumerate() {
        prompts = prompts.push(
            row![
                text_input("推荐提示词", prompt)
                    .on_input(move |value| message(AgentsMessage::ProfilePromptChanged(
                        index, value
                    )))
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style),
                action(
                    "上移",
                    (index > 0).then_some(AgentsMessage::ProfilePromptMove(
                        index,
                        index.saturating_sub(1)
                    ))
                ),
                action(
                    "下移",
                    (index + 1 < draft.recommended_prompts.len())
                        .then_some(AgentsMessage::ProfilePromptMove(index, index + 1))
                ),
                action("移除", Some(AgentsMessage::ProfilePromptRemove(index))),
            ]
            .spacing(8),
        );
    }
    prompts = prompts.push(action("添加提示词", Some(AgentsMessage::ProfilePromptAdd)));
    let prompts = row![text("推荐提示词").width(Length::Fixed(SETTINGS_LABEL_WIDTH)), prompts]
        .spacing(16)
        .align_y(Alignment::Start);
    let runtime = match &draft.runtime {
        AgentRuntime::Native => "native".to_string(),
        AgentRuntime::Acp { backend } => backend.clone(),
    };
    let runtime_control: Element<'_, Message> = if fixed {
        text(format!("固定运行引擎：{runtime}")).into()
    } else {
        let mut options = vec!["native".to_string()];
        options.extend(app.acp_agents.clone());
        let busy = app.agents_settings.profile_saving;
        let picker = pick_list(options, Some(runtime), move |value| {
            message(AgentsMessage::ProfileRuntimeChanged(value))
        })
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill);
        let mut controls = column![picker].spacing(8);
        if app.acp_agents.is_empty() {
            controls = controls.push(
                text(if app.acp_settings.loading {
                    "正在加载 ACP 引擎目录…"
                } else {
                    "暂无可用 ACP 引擎，可刷新目录或在 ACP 设置中启用。"
                })
                .size(12)
                .style(settings_muted_text_style),
            );
        }
        if let Some(error) = &app.acp_settings.save_error {
            controls = controls.push(settings_error_banner(error.clone()));
        }
        controls = controls.push(
            button(text(if app.acp_settings.loading {
                "加载中…"
            } else {
                "刷新引擎目录"
            }))
            .on_press_maybe((!busy && !app.acp_settings.loading).then_some(Message::Settings(
                SettingsMessage::Acp(crate::app::message::settings::AcpMessage::Refresh),
            )))
            .padding([8, 14])
            .style(rounded_action_btn_style),
        );
        controls.into()
    };
    let runtime = row![
        text("运行智能体").width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        settings_control_with_hint(
            runtime_control,
            "首次保存成功后，运行引擎及 ACP backend 不可修改。"
        ),
    ]
    .spacing(16)
    .align_y(Alignment::Start);
    column![name, description, prompts, runtime].spacing(14).into()
}
fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Message> {
    button(text(label))
        .on_press_maybe(value.map(message))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}
