//! 智能体资料草稿弹窗；持久化成功前不修改列表快照。
use crate::app::components::system_settings_common::*;
use crate::app::i18n::settings_extensions::text as tr;
use crate::app::{
    App, Message,
    message::settings::{AgentsMessage, SettingsMessage},
};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};
use vw_config_types::agent::builtin_agent_spec;

fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(draft) = app.agents_settings.profile_modal.as_ref() else {
        return base;
    };
    let busy = app.agents_settings.profile_saving;
    let label = |source| tr(app.desktop_locale, source);
    let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
    let settings = &app.agents_settings;
    let native = match draft.runtime {
        vw_config_types::agent::AgentRuntime::Native => true,
        vw_config_types::agent::AgentRuntime::Acp { .. } => false,
    };
    let detail_tab =
        |label, key| super::shared::detail_tab_button(label, key, &settings.active_detail_tab);
    let tabs = settings_tab_row(
        [
            Some(detail_tab(label("基础参数"), crate::app::state::AGENT_DETAIL_BASIC_TAB)),
            Some(detail_tab(label("身份"), crate::app::state::AGENT_DETAIL_IDENTITY_TAB)),
            native.then(|| detail_tab(label("工具"), crate::app::state::AGENT_DETAIL_TOOLS_TAB)),
            Some(detail_tab(label("技能"), crate::app::state::AGENT_DETAIL_SKILLS_TAB)),
            Some(detail_tab("MCP", crate::app::state::AGENT_DETAIL_MCP_TAB)),
        ]
        .into_iter()
        .flatten(),
    )
    .wrap();
    let entry = super::shared::selected_entry(app).unwrap_or(draft);
    let fields: Element<'_, Message> = match settings.active_detail_tab.as_str() {
        crate::app::state::AGENT_DETAIL_IDENTITY_TAB => super::identity::view(app, entry),
        crate::app::state::AGENT_DETAIL_TOOLS_TAB if native => super::tools::view(app, entry),
        crate::app::state::AGENT_DETAIL_SKILLS_TAB => super::skills::view(app, entry),
        crate::app::state::AGENT_DETAIL_MCP_TAB => super::mcp::view(app, entry),
        _ => super::basic::view(app, entry, Vec::new()),
    };
    let mut footer =
        row![action(label("取消"), (!busy).then_some(AgentsMessage::ProfileCancel))].spacing(10);
    if !fixed {
        footer = footer
            .push(action(label("删除智能体"), (!busy).then_some(AgentsMessage::ProfileDelete)));
    }
    footer = footer.push(
        button(text(label(if busy { "保存中…" } else { "保存配置" })))
            .on_press_maybe((!busy).then_some(message(AgentsMessage::ProfileSave)))
            .padding([8, 14])
            .style(primary_action_btn_style),
    );
    let header = column![
        settings_modal_header(
            text(label("智能体配置")).size(20),
            settings_close_button(message(AgentsMessage::ProfileCancel)),
        ),
        tabs,
        text(label("身份文件正文即时保存；其余配置点击保存后生效。"))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    let footer = container(footer.align_y(Alignment::Center).wrap()).align_right(Length::Fill);
    let card = settings_modal_layout(header, fields, Some(footer.into()))
        .width(Length::Fixed(1000.0))
        .height(Length::Fill)
        .max_height(820);
    settings_modal_overlay(
        Some(base),
        message(AgentsMessage::ProfileCancel),
        container(card).padding(12),
    )
}

fn action(label: &'static str, value: Option<AgentsMessage>) -> Element<'static, Message> {
    button(text(label))
        .on_press_maybe(value.map(message))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .into()
}
