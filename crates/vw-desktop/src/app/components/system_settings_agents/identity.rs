//! 系统设置中智能体配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::shared::{agent_tab_button, editor_style};
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style,
    settings_checkbox_style, settings_field_label, settings_group_card,
    settings_group_heading_with_help, settings_hover_help_label, settings_muted_text_style,
    settings_tab_row,
};
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{
    AGENT_PROMPT_SYSTEM_TAB, DelegateAgentSettingsEntry, WORKSPACE_IDENTITY_FILES,
};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text, text_editor};
use iced::{Alignment, Element, Length};

pub(super) fn default_workspace_identity_root(agent_key: &str) -> String {
    if agent_key == "main" {
        vw_config_types::paths::tilde_config_path("workspace")
    } else {
        vw_config_types::paths::tilde_config_path(&format!("workspace-{agent_key}"))
    }
}

pub(super) fn workspace_identity_hint(
    locale: vw_config_types::ui::DesktopLocale,
    root: Option<&str>,
    agent_key: &str,
    file_name: &str,
) -> String {
    let root =
        root.map(ToOwned::to_owned).unwrap_or_else(|| default_workspace_identity_root(agent_key));
    let target_path = format!("{root}/{file_name}");
    crate::__!(
        crate::app::i18n::I18n::new(locale),
        "编辑身份文件 `%s`，保存会直接写回 `%s`。",
        file_name,
        target_path
    )
    .expect("valid identity path template")
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
    agent_tab_button(
        label,
        key == active_key,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::PromptTabSelected(
            key.to_string(),
        ))),
    )
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
    let i18n = I18n::new(app.desktop_locale);
    let system_prompt_title = tr(app.desktop_locale, "系统提示词");
    let system_prompt_editor = text_editor(&entry.system_prompt_editor)
        .placeholder(tr(app.desktop_locale, "为该代理定义专用系统提示词 ..."))
        .on_action({
            let key = entry.key.clone();
            move |action| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::SystemPromptAction(
                    key.clone(),
                    action,
                )))
            }
        })
        .font(iced::Font::DEFAULT)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .height(Length::Fixed(260.0))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(editor_style);

    let prompt_tabs = settings_tab_row(
        std::iter::once(prompt_tab_button(
            tr(app.desktop_locale, "系统提示词"),
            AGENT_PROMPT_SYSTEM_TAB,
            &settings.active_prompt_tab,
        ))
        .chain(WORKSPACE_IDENTITY_FILES.iter().map(|(file_name, label)| {
            prompt_tab_button(
                crate::app::i18n::I18n::new(app.desktop_locale).__(label),
                file_name,
                &settings.active_prompt_tab,
            )
        })),
    )
    .wrap();

    let prompt_panel: Element<'_, Message> =
        if settings.active_prompt_tab == AGENT_PROMPT_SYSTEM_TAB {
            column![
                settings_hover_help_label(
                    i18n,
                    &app.app_theme,
                    system_prompt_title,
                    settings_field_label(system_prompt_title),
                    tr(app.desktop_locale, "多行系统提示，将作为该 Agent 的专用角色提示。"),
                )
                .wrap(),
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
                .placeholder(tr(app.desktop_locale, "当前文件为空，可直接在这里编写内容 ..."))
                .on_action({
                    let file_name = file_state.file_name.clone();
                    move |action| {
                        Message::Settings(SettingsMessage::Agents(
                            AgentsMessage::WorkspaceIdentityEditorAction(file_name.clone(), action),
                        ))
                    }
                })
                .font(iced::Font::DEFAULT)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .height(Length::Fixed(320.0))
                .padding(SETTINGS_CONTROL_PADDING)
                .style(editor_style);

            let workspace_hint = workspace_identity_hint(
                app.desktop_locale,
                settings.workspace_identity_root_path.as_deref(),
                &settings.selected_agent,
                &file_state.file_name,
            );
            let workspace_meta = workspace_identity_meta_line(
                app.desktop_locale,
                file_state.size_bytes,
                file_state.modified_at_ms,
            );
            let identity_load_checkbox =
                checkbox(identity_file_is_enabled(entry, &file_state.file_name))
                    .label(tr(app.desktop_locale, "启用后加载到提示词"))
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
            let restore_default_button = button(text(tr(app.desktop_locale, "恢复默认")).size(12))
                .padding([8, 14])
                .on_press(Message::Settings(SettingsMessage::Agents(
                    AgentsMessage::WorkspaceIdentityRestoreDefaultRequested(
                        file_state.file_name.clone(),
                    ),
                )))
                .style(rounded_action_btn_style);

            column![
                row![
                    settings_field_label(&file_state.file_name),
                    identity_load_checkbox,
                    restore_default_button
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .wrap(),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__(&file_state.label))
                    .size(12)
                    .style(settings_muted_text_style),
                workspace_meta.map(|meta| text(meta).size(12).style(settings_muted_text_style)),
                text(workspace_hint)
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(settings_muted_text_style),
                container(workspace_editor).width(Length::Fill),
            ]
            .spacing(8)
            .into()
        } else {
            text(tr(app.desktop_locale, "未找到对应的身份文件页签"))
                .size(12)
                .style(settings_muted_text_style)
                .into()
        };

    column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            tr(app.desktop_locale, "身份与提示词"),
            tr(app.desktop_locale, "管理当前 Agent 的系统提示词，以及独立工作区身份文件。"),
        ),
        prompt_tabs,
        settings_group_card(prompt_panel).padding([14, 16]),
    ]
    .spacing(14)
    .into()
}

pub(super) fn workspace_identity_meta_line(
    locale: vw_config_types::ui::DesktopLocale,
    size: Option<u64>,
    modified: Option<u64>,
) -> Option<String> {
    let relative = modified.map(|timestamp| {
        let (bucket, value) = crate::app::components::chat_panel::utils::relative_time_bucket(
            timestamp,
            crate::app::time::now_ms(),
        );
        let source = match bucket {
            1 => "%u 分钟前",
            2 => "%u 小时前",
            3 => "%u 天前",
            4 => "%u 个月前",
            5 => "%u 年前",
            _ => return tr(locale, "刚刚").to_owned(),
        };
        crate::__!(crate::app::i18n::I18n::new(locale), source, value)
            .expect("valid relative time template")
    });
    match (format_file_size(size), relative) {
        (Some(size), Some(time)) => Some(format!("{size} · {time}")),
        (Some(size), None) => Some(size),
        (None, time) => time,
    }
}
