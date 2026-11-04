//! 基础参数中的资料字段；描述编辑缓冲不参与持久化。
use crate::app::components::system_settings_common::*;
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
    state::DelegateAgentSettingsEntry,
};
use iced::{
    Element,
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
    let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
    let input = |placeholder, value, callback: fn(String) -> AgentsMessage| {
        text_input(placeholder, value)
            .on_input(move |value| message(callback(value)))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
    };
    let mut fields = column![
        text("名称"),
        input("智能体名称", &draft.label, AgentsMessage::ProfileNameChanged),
        text("描述"),
        text_editor(&draft.description_editor)
            .on_action(|action| message(AgentsMessage::ProfileDescriptionAction(action)))
            .font(iced::Font::DEFAULT)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .height(120)
            .style(settings_text_editor_style),
        text("推荐提示词").size(16),
    ]
    .spacing(10);
    for (index, prompt) in draft.recommended_prompts.iter().enumerate() {
        fields = fields.push(
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
    fields = fields
        .push(action("添加提示词", Some(AgentsMessage::ProfilePromptAdd)))
        .push(text("运行智能体").size(16));
    let runtime = match &draft.runtime {
        AgentRuntime::Native => "native".to_string(),
        AgentRuntime::Acp { backend } => backend.clone(),
    };
    if fixed {
        fields = fields.push(text(format!("固定运行引擎：{runtime}")));
    } else {
        let mut options = vec!["native".to_string()];
        options.extend(app.acp_agents.clone());
        fields = fields.push(
            pick_list(options, Some(runtime), |value| {
                message(AgentsMessage::ProfileRuntimeChanged(value))
            })
            .padding(SETTINGS_CONTROL_PADDING)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style),
        );
    }
    fields.push(text("引擎和系统提示词修改只影响新会话。").style(settings_muted_text_style)).into()
}
fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Message> {
    button(text(label))
        .on_press_maybe(value.map(message))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}
