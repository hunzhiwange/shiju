//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::shared::{editor_style, section_card};
use crate::app::components::chat_panel::utils::relative_time_label;
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_checkbox_style, settings_muted_text_style,
    settings_segment_button_style,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{
    AGENT_PROMPT_SYSTEM_TAB, DelegateAgentSettingsEntry, WORKSPACE_IDENTITY_FILES,
};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text_editor};
use iced::{Alignment, Element, Length, Theme};

pub(super) fn default_workspace_identity_root(agent_key: &str) -> String {
    if agent_key == "main" {
        vw_config_types::paths::tilde_config_path("workspace")
    } else {
        vw_config_types::paths::tilde_config_path(&format!("workspace-{agent_key}"))
    }
}

pub(super) fn workspace_identity_hint(
    root: Option<&str>,
    agent_key: &str,
    file_name: &str,
) -> String {
    let root =
        root.map(ToOwned::to_owned).unwrap_or_else(|| default_workspace_identity_root(agent_key));
    let target_path = format!("{root}/{file_name}");
    format!("编辑身份文件 `{file_name}`，保存会直接写回 `{target_path}`。")
}

pub(super) fn format_file_size(size_bytes: Option<u64>) -> Option<String> {
    let size = size_bytes?;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;

    if size < 1024 {
        Some(format!("{size} B"))
    } else if (size as f64) < MB {
        Some(format!("{:.1} KB", size as f64 / KB))
    } else {
        Some(format!("{:.1} MB", size as f64 / MB))
    }
}

pub(super) fn workspace_identity_meta_line(
    size_bytes: Option<u64>,
    modified_at_ms: Option<u64>,
) -> Option<String> {
    match (format_file_size(size_bytes), modified_at_ms) {
        (Some(size), Some(modified_at_ms)) => {
            Some(format!("{size} · {}", relative_time_label(modified_at_ms)))
        }
        (Some(size), None) => Some(size),
        (None, Some(modified_at_ms)) => Some(relative_time_label(modified_at_ms)),
        (None, None) => None,
    }
}

pub(super) fn identity_file_is_enabled(
    entry: &DelegateAgentSettingsEntry,
    file_name: &str,
) -> bool {
    entry.enabled_identity_files.iter().any(|enabled| enabled == file_name)
}

fn prompt_tab_button<'a>(
    label: &'a str,
    key: &'a str,
    active_key: &'a str,
) -> Element<'a, Message> {
    let is_active = key == active_key;
    button(text(label).size(12))
        .padding([8, 12])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::PromptTabSelected(
            key.to_string(),
        ))))
        .into()
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn view<'a>(
    app: &'a App,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let settings = &app.agents_settings;
    let system_prompt_editor = text_editor(&entry.system_prompt_editor)
        .placeholder("为该代理定义专用系统提示词 ...")
        .on_action({
            let key = entry.key.clone();
            move |action| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::SystemPromptAction(
                    key.clone(),
                    action,
                )))
            }
        })
        .height(Length::Fixed(260.0))
        .padding(10)
        .style(editor_style);

    let prompt_tabs =
        row(std::iter::once(prompt_tab_button(
            "系统提示词",
            AGENT_PROMPT_SYSTEM_TAB,
            &settings.active_prompt_tab,
        ))
        .chain(WORKSPACE_IDENTITY_FILES.iter().map(|(file_name, label)| {
            prompt_tab_button(label, file_name, &settings.active_prompt_tab)
        }))
        .collect::<Vec<Element<'a, Message>>>())
        .spacing(8)
        .wrap();

    let prompt_panel: Element<'_, Message> =
        if settings.active_prompt_tab == AGENT_PROMPT_SYSTEM_TAB {
            column![
                text("系统提示词"),
                text("多行系统提示，将作为该 Agent 的专用角色提示。")
                    .size(12)
                    .style(settings_muted_text_style),
                container(system_prompt_editor).width(Length::Fill),
            ]
            .spacing(8)
            .into()
        } else if let Some(file_state) = settings
            .workspace_identity_files
            .iter()
            .find(|file| file.file_name == settings.active_prompt_tab)
        {
            let workspace_editor = text_editor(&file_state.editor)
                .placeholder("当前文件为空，可直接在这里编写内容 ...")
                .on_action({
                    let file_name = file_state.file_name.clone();
                    move |action| {
                        Message::Settings(SettingsMessage::Agents(
                            AgentsMessage::WorkspaceIdentityEditorAction(file_name.clone(), action),
                        ))
                    }
                })
                .height(Length::Fixed(320.0))
                .padding(10)
                .style(editor_style);

            let workspace_hint = workspace_identity_hint(
                settings.workspace_identity_root_path.as_deref(),
                &settings.selected_agent,
                &file_state.file_name,
            );
            let workspace_meta =
                workspace_identity_meta_line(file_state.size_bytes, file_state.modified_at_ms);
            let identity_load_checkbox =
                checkbox(identity_file_is_enabled(entry, &file_state.file_name))
                    .label("启用后加载到提示词")
                    .on_toggle({
                        let agent_key = entry.key.clone();
                        let file_name = file_state.file_name.clone();
                        move |enabled| {
                            Message::Settings(SettingsMessage::Agents(
                                AgentsMessage::WorkspaceIdentityEnabledToggled {
                                    agent_key: agent_key.clone(),
                                    file_name: file_name.clone(),
                                    enabled,
                                },
                            ))
                        }
                    })
                    .style(settings_checkbox_style);
            let restore_default_button = button(text("恢复默认").size(12))
                .padding([6, 10])
                .on_press(Message::Settings(SettingsMessage::Agents(
                    AgentsMessage::WorkspaceIdentityRestoreDefaultRequested(
                        file_state.file_name.clone(),
                    ),
                )))
                .style(rounded_action_btn_style);

            column![
                row![
                    text(file_state.file_name.clone()),
                    identity_load_checkbox,
                    restore_default_button
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text(file_state.label.clone()).size(12).style(settings_muted_text_style),
                workspace_meta.map(|meta| text(meta).size(12).style(settings_muted_text_style)),
                text(workspace_hint).size(12).style(settings_muted_text_style),
                container(workspace_editor).width(Length::Fill),
            ]
            .spacing(8)
            .into()
        } else {
            container(text("未找到对应的身份文件页签")).into()
        };

    column![
        section_card("身份与提示词", "管理当前 Agent 的系统提示词，以及独立工作区身份文件。",),
        prompt_tabs,
        prompt_panel,
    ]
    .spacing(14)
    .into()
}
