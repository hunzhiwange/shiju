//! 智能体资料草稿弹窗；持久化成功前不修改列表快照。
use crate::app::components::system_settings_common::*;
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_editor, text_input,
};
use iced::{Element, Length};
use vw_config_types::agent::{AgentRuntime, builtin_agent_spec};

fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = app.agents_settings.profile_modal.as_ref() else {
        return base;
    };
    let busy = app.agents_settings.profile_saving;
    let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
    let input = |placeholder, value, callback: fn(String) -> AgentsMessage| {
        text_input(placeholder, value)
            .on_input(move |value| message(callback(value)))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
    };
    let mut fields = column![
        text("资料").size(16),
        text("名称"),
        input("智能体名称", &draft.label, AgentsMessage::ProfileNameChanged),
        text("描述"),
        input("描述", &draft.description, AgentsMessage::ProfileDescriptionChanged),
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
    fields = fields
        .push(text("引擎和规则修改只影响新会话。").style(settings_muted_text_style))
        .push(text("规则").size(16))
        .push(
            text_editor(&draft.system_prompt_editor)
                .on_action(|action| message(AgentsMessage::ProfileRulesAction(action)))
                .font(iced::Font::DEFAULT)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .height(180)
                .style(settings_text_editor_style),
        );
    let body = scrollable(container(fields).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    let mut footer =
        row![action("取消", (!busy).then_some(AgentsMessage::ProfileCancel))].spacing(10);
    if !fixed {
        footer = footer.push(action("删除智能体", (!busy).then_some(AgentsMessage::ProfileDelete)));
    }
    footer = footer.push(
        button(text(if busy { "保存中…" } else { "保存资料" }))
            .on_press_maybe((!busy).then_some(message(AgentsMessage::ProfileSave)))
            .padding([8, 14])
            .style(primary_action_btn_style),
    );
    let card = settings_modal_card(
        column![
            row![
                text("智能体资料").size(20),
                settings_close_button(message(AgentsMessage::ProfileCancel))
            ]
            .spacing(20),
            body,
            footer,
        ]
        .spacing(16),
    )
    .width(Length::Fixed(760.0))
    .height(Length::Fill)
    .max_height(820);
    settings_modal_overlay(Some(base), message(AgentsMessage::ProfileCancel), card)
}

fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Message> {
    button(text(label))
        .on_press_maybe(value.map(message))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}
