//! 两个目录共用网关配置快照；停用的自定义项仍可管理。
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{AgentsSettingsState, DelegateAgentSettingsEntry};
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};
use vw_config_types::agent::AgentRuntime;

pub(super) fn entries<'a>(
    settings: &'a AgentsSettingsState,
    official: bool,
    query: &str,
) -> Vec<&'a DelegateAgentSettingsEntry> {
    let query = query.trim().to_lowercase();
    settings
        .entries
        .iter()
        .filter(|entry| {
            (if official { entry.builtin } else { entry.enabled || !entry.builtin })
                && [&entry.key, &entry.label, &entry.description]
                    .iter()
                    .any(|value| value.to_lowercase().contains(&query))
        })
        .collect()
}

fn message(value: AgentsMessage) -> Message {
    Message::Settings(SettingsMessage::Agents(value))
}

pub(super) fn controls(app: &App) -> Element<'_, Message> {
    let settings = &app.agents_settings;
    let tab = |official, label| {
        let selected = settings.catalog_official == official;
        button(text(label).size(13))
            .padding([8, 12])
            .style(move |theme, status| settings_segment_button_style(theme, status, selected))
            .on_press(message(AgentsMessage::CatalogTabSelected(official)))
    };
    column![
        row![tab(false, "已启用智能体"), tab(true, "官方智能体")].spacing(8),
        text_input("搜索名称、描述或标识", &settings.catalog_query)
            .on_input(|value| message(AgentsMessage::CatalogQueryChanged(value)))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style),
        text("自定义项停用后仍保留在管理区。启用与 CLI 安装状态独立。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(8)
    .into()
}

pub(super) fn cards(app: &App) -> Vec<Element<'_, Message>> {
    let settings = &app.agents_settings;
    let busy = settings.catalog_busy
        || settings.save_in_flight
        || settings.profile_saving
        || settings.dirty;
    entries(settings, settings.catalog_official, &settings.catalog_query)
        .into_iter()
        .map(|entry| {
            let engine = match &entry.runtime {
                AgentRuntime::Native => "识矩原生".to_string(),
                AgentRuntime::Acp { backend } => format!("ACP · {backend}"),
            };
            let status = match &entry.runtime {
                AgentRuntime::Native => "原生运行引擎".to_string(),
                AgentRuntime::Acp { backend } => app
                    .acp_settings
                    .diagnostic_results
                    .get(backend)
                    .map(|status| match status {
                        crate::app::state::AcpDiagnosticStatus::Connected => {
                            "已安装 · 已连接".to_string()
                        }
                        crate::app::state::AcpDiagnosticStatus::AuthenticationRequired => {
                            "已安装 · 需要认证".to_string()
                        }
                        _ => status.description().to_string(),
                    })
                    .unwrap_or_else(|| "网关安装 / 认证状态待检测".into()),
            };
            let action = |label, value| {
                button(text(label).size(12))
                    .padding([8, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!busy).then_some(message(value)))
            };
            let mut actions = row![
                action("编辑资料", AgentsMessage::ProfileOpen(entry.key.clone())),
                action("复制", AgentsMessage::CatalogCopy(entry.key.clone())),
            ]
            .spacing(6);
            if entry.key != "main" {
                actions = actions.push(action(
                    if entry.enabled { "停用" } else { "启用" },
                    AgentsMessage::CatalogToggle(entry.key.clone(), !entry.enabled),
                ));
            }
            if entry.enabled
                && !matches!(entry.kind, crate::app::state::AgentSettingsEntryKind::BuiltinWorker)
            {
                actions =
                    actions.push(action("启动", AgentsMessage::CatalogLaunch(entry.key.clone())));
            }
            let mut body = column![
                button(
                    column![text(&entry.label).size(14), text(&entry.key).size(12)]
                        .spacing(4)
                        .width(Length::Fill)
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(move |theme, status| super::shared::agent_sidebar_button_style(
                    theme,
                    status,
                    settings.selected_agent == entry.key
                ))
                .on_press(message(AgentsMessage::SelectAgent(entry.key.clone()))),
                text(&entry.description).size(12).style(settings_muted_text_style),
                text(format!(
                    "{engine} · {} · {}",
                    super::shared::entry_kind_label(entry),
                    if entry.enabled { "已启用" } else { "已停用" }
                ))
                .size(12),
                text(status).size(12).style(settings_muted_text_style),
                actions.wrap(),
            ]
            .spacing(8);
            if let AgentRuntime::Acp { backend } = &entry.runtime {
                body = body.push(
                    button(text("运行设置 / 安装 / 网关诊断").size(12))
                        .padding([8, 10])
                        .style(rounded_action_btn_style)
                        .on_press(message(AgentsMessage::CatalogRuntimeSettings(backend.clone()))),
                );
            }
            container(body).padding(12).width(Length::Fill).style(settings_panel_style).into()
        })
        .collect()
}
