use super::{PreferencesMessage, event};
use crate::app::components::system_settings_common::*;
use crate::app::{Message, state::AppsCatalogState};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, container, row, text},
};
use vw_gateway_client::vw_api_types::agent_apps::*;

const ACTION_PADDING: [u16; 2] = [8, 12];

#[derive(Clone, PartialEq)]
struct TerminalChoice(String);

impl std::fmt::Display for TerminalChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() { f.write_str("主机默认终端") } else { f.write_str(&self.0) }
    }
}

pub(crate) fn overlay<'a>(
    catalog: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &catalog.preferences;
    if !state.open {
        return base;
    }
    let close = event(PreferencesMessage::Close);
    let editable = !state.busy;
    let mut body = column![text(format!("执行主机：{}", catalog.host)).size(13)].spacing(14);
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    if state.busy {
        body = body.push(text("正在等待网关确认…").size(13));
    }
    if let Some(draft) = &state.draft {
        let mut visibility = column![settings_group_heading(
            "可见应用与顺序",
            "显示或隐藏应用，并调整它们在目录中的顺序。",
        )]
        .spacing(10);
        for (index, id) in draft.user.app_order.iter().enumerate() {
            if let Some(app) = catalog.rows.iter().find(|r| {
                serde_json::to_value(r.entry.id)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .as_ref()
                    == Some(id)
            }) {
                let app_id = app.entry.id;
                visibility =
                    visibility.push(
                        row![
                            checkbox(draft.user.visible_apps.contains(&app_id))
                                .label(app.entry.name.clone())
                                .size(16)
                                .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                                .width(Length::Fill)
                                .style(settings_checkbox_style)
                                .on_toggle_maybe(editable.then_some(move |v| event(
                                    PreferencesMessage::Visible(app_id, v)
                                ))),
                            button(text("上移").size(SETTINGS_CONTROL_TEXT_SIZE))
                                .padding(ACTION_PADDING)
                                .style(rounded_action_btn_style)
                                .on_press_maybe(
                                    (editable && index > 0)
                                        .then_some(event(PreferencesMessage::MoveUp(id.clone())))
                                ),
                        ]
                        .align_y(Alignment::Center)
                        .spacing(8),
                    );
            }
        }
        body = body.push(visibility).push(settings_divider());

        let mut host =
            column![settings_group_heading("主机终端设置", "这些偏好仅用于所选执行主机。",)]
                .spacing(10);
        if let Some(capabilities) = &draft.capabilities {
            let options = std::iter::once(String::new())
                .chain(capabilities.terminals.iter().cloned())
                .map(TerminalChoice);
            let selected =
                TerminalChoice(draft.host.preferred_terminal.clone().unwrap_or_default());
            host = host.push(
                column![
                    settings_field_label("首选终端"),
                    settings_select(options, Some(selected), |v| event(
                        PreferencesMessage::Terminal(v.0)
                    ))
                    .disabled(!state.host_editable())
                    .width(Length::Fill),
                ]
                .spacing(6),
            );
            if capabilities.wsl_preferences {
                host = host.push(settings_group_heading(
                    "WSL",
                    "发行版由所选安装实例决定；只保存各应用的 shell 与 flag。留空表示自动。",
                ));
                for app in &catalog.rows {
                    if app.entry.id == AppId::ClaudeDesktop {
                        continue;
                    }
                    let id = app.entry.id;
                    let distributions: Vec<_> = app
                        .observation
                        .as_ref()
                        .ok()
                        .into_iter()
                        .flat_map(|o| &o.installations)
                        .filter_map(|i| match &i.environment {
                            AppInstallationEnvironment::Wsl { distribution } => {
                                Some(distribution.as_str())
                            }
                            _ => None,
                        })
                        .collect();
                    let entry = draft.host.wsl_shells.iter().find(|e| e.app_id == id);
                    let shell = entry.and_then(|e| e.shell.as_deref()).unwrap_or("");
                    let flag = entry.and_then(|e| e.flag.as_deref()).unwrap_or("");
                    let description = if distributions.is_empty() {
                        "发行版：尚未发现".to_owned()
                    } else {
                        format!("发行版：{}", distributions.join("、"))
                    };
                    host = host.push(settings_panel(
                        column![
                            column![
                                settings_field_label(&app.entry.name),
                                text(description).size(12).style(settings_muted_text_style),
                            ]
                            .spacing(4),
                            column![
                                settings_field_label("Shell"),
                                settings_text_input("自动或主机支持的 shell", shell)
                                    .on_input_maybe(editable.then_some(move |v| {
                                        event(PreferencesMessage::Shell(id, v))
                                    })),
                            ]
                            .spacing(6),
                            column![
                                settings_field_label("Flag"),
                                settings_text_input("自动或 -c / -ic / -lc / -lic", flag)
                                    .on_input_maybe(editable.then_some(move |v| {
                                        event(PreferencesMessage::Flag(id, v))
                                    })),
                            ]
                            .spacing(6),
                        ]
                        .spacing(10),
                    ));
                }
            }
        } else {
            host = host.push(
                text("网关未提供主机能力，终端与 WSL 设置暂不可编辑。")
                    .size(12)
                    .style(settings_muted_text_style),
            );
        }
        body = body.push(host);
        if state.saved.as_ref().is_some_and(|v| v.restart_required) {
            body = body.push(settings_success_banner(format!(
                "已保存。请手动重启执行主机「{}」上使用新设置的应用终端会话；现有会话不会自动重启。",
                catalog.host
            )));
        }
    }
    let header = settings_modal_header(
        settings_page_heading("应用设置", "分别保存用户偏好和主机设置。"),
        settings_close_button(close.clone()),
    );
    let actions = row![
        Space::new().width(Length::Fill),
        button(text("取消").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(ACTION_PADDING)
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(close.clone())),
        button(text("保存可见性/顺序").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(ACTION_PADDING)
            .style(primary_action_btn_style)
            .on_press_maybe(
                (editable && state.draft.is_some()).then_some(event(PreferencesMessage::SaveUser))
            ),
        button(text("保存主机设置").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(ACTION_PADDING)
            .style(primary_action_btn_style)
            .on_press_maybe(state.host_editable().then_some(event(PreferencesMessage::SaveHost))),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(actions.into()),
        Some("apps-preferences-body"),
    )
    .width(Length::Fill)
    .max_width(560.0)
    .max_height(560.0);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
