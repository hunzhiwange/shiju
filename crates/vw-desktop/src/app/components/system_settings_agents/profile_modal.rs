//! 智能体资料草稿弹窗；持久化成功前不修改列表快照。
use crate::app::components::system_settings_common::*;
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};
use vw_config_types::agent::builtin_agent_spec;

fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = app.agents_settings.profile_modal.as_ref() else {
        return base;
    };
    let busy = app.agents_settings.profile_saving;
    let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
    let settings = &app.agents_settings;
    let tabs = row![
        super::shared::detail_tab_button(
            "基础参数",
            crate::app::state::AGENT_DETAIL_BASIC_TAB,
            &settings.active_detail_tab
        ),
        super::shared::detail_tab_button(
            "身份",
            crate::app::state::AGENT_DETAIL_IDENTITY_TAB,
            &settings.active_detail_tab
        ),
        super::shared::detail_tab_button(
            "工具",
            crate::app::state::AGENT_DETAIL_TOOLS_TAB,
            &settings.active_detail_tab
        ),
        super::shared::detail_tab_button(
            "技能",
            crate::app::state::AGENT_DETAIL_SKILLS_TAB,
            &settings.active_detail_tab
        ),
    ]
    .spacing(8)
    .wrap();
    let entry = draft;
    let fields: Element<'_, Message> = match settings.active_detail_tab.as_str() {
        crate::app::state::AGENT_DETAIL_IDENTITY_TAB => super::identity::view(app, entry),
        crate::app::state::AGENT_DETAIL_TOOLS_TAB => super::tools::view(app, entry),
        crate::app::state::AGENT_DETAIL_SKILLS_TAB => super::skills::view(app, entry),
        _ => super::basic::view(app, entry, Vec::new()),
    };
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
            tabs,
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
