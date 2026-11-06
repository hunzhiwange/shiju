use super::{PreferencesMessage, event};
use crate::app::components::system_settings_common::*;
use crate::app::{Message, state::AppsCatalogState};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, container, pick_list, row, scrollable, text, text_input},
};
use vw_gateway_client::vw_api_types::agent_apps::*;

pub(crate) fn overlay<'a>(
    catalog: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &catalog.preferences;
    if !state.open {
        return base;
    }
    let close = event(PreferencesMessage::Close);
    let mut body = column![text(format!("执行主机：{}", catalog.host)).size(13)].spacing(12);
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    if state.busy {
        body = body.push(text("正在等待网关确认…"));
    }
    if let Some(draft) = &state.draft {
        body = body.push(text("可见应用与顺序（用户偏好）").size(17));
        for (index, id) in draft.user.app_order.iter().enumerate() {
            if let Some(app) = catalog.rows.iter().find(|r| {
                serde_json::to_value(r.entry.id)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .as_ref()
                    == Some(id)
            }) {
                let app_id = app.entry.id;
                body =
                    body.push(
                        row![
                            checkbox(draft.user.visible_apps.contains(&app_id))
                                .label(app.entry.name.clone())
                                .style(settings_checkbox_style)
                                .on_toggle_maybe((!state.busy).then_some(move |v| event(
                                    PreferencesMessage::Visible(app_id, v)
                                ))),
                            container(text("")).width(Length::Fill),
                            button(text("上移"))
                                .padding([6, 12])
                                .style(rounded_action_btn_style)
                                .on_press_maybe(
                                    (!state.busy && index > 0)
                                        .then_some(event(PreferencesMessage::MoveUp(id.clone())))
                                ),
                        ]
                        .spacing(8),
                    );
            }
        }
        body = body.push(text("主机终端设置").size(17));
        if let Some(capabilities) = &draft.capabilities {
            let options = std::iter::once(String::new())
                .chain(capabilities.terminals.clone())
                .collect::<Vec<_>>();
            body = body.push(text("首选终端（空白使用主机默认值）").size(13));
            body = body.push(
                pick_list(
                    options,
                    Some(draft.host.preferred_terminal.clone().unwrap_or_default()),
                    |v| event(PreferencesMessage::Terminal(v)),
                )
                .style(settings_pick_list_style)
                .menu_style(settings_pick_list_menu_style)
                .padding(SETTINGS_CONTROL_PADDING)
                .text_size(SETTINGS_CONTROL_TEXT_SIZE),
            );
            if capabilities.wsl_preferences {
                body = body.push(text("WSL：发行版由所选安装实例决定；此处仅保存各应用的 shell 与 flag。空白表示自动。").size(12));
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
                    body = body.push(
                        text(format!(
                            "{} · 发行版：{}",
                            app.entry.name,
                            if distributions.is_empty() {
                                "尚未发现".into()
                            } else {
                                distributions.join("、")
                            }
                        ))
                        .size(13),
                    );
                    let entry = draft.host.wsl_shells.iter().find(|e| e.app_id == id);
                    let shell = entry.and_then(|e| e.shell.as_deref()).unwrap_or("");
                    let flag = entry.and_then(|e| e.flag.as_deref()).unwrap_or("");
                    body = body.push(
                        text_input("shell（自动或主机支持的 shell）", shell)
                            .on_input_maybe(
                                (!state.busy)
                                    .then_some(move |v| event(PreferencesMessage::Shell(id, v))),
                            )
                            .style(settings_text_input_style)
                            .padding(SETTINGS_CONTROL_PADDING)
                            .size(SETTINGS_CONTROL_TEXT_SIZE),
                    );
                    body = body.push(
                        text_input("flag（自动或 -c / -ic / -lc / -lic）", flag)
                            .on_input_maybe(
                                (!state.busy)
                                    .then_some(move |v| event(PreferencesMessage::Flag(id, v))),
                            )
                            .style(settings_text_input_style)
                            .padding(SETTINGS_CONTROL_PADDING)
                            .size(SETTINGS_CONTROL_TEXT_SIZE),
                    );
                }
            }
        } else {
            body = body.push(text("网关未提供主机能力，终端与 WSL 设置暂不可编辑。").size(12));
        }
        if state.saved.as_ref().is_some_and(|v| v.restart_required) {
            body = body.push(text(format!("已保存。请手动重启执行主机「{}」上使用新设置的应用终端会话；现有会话不会自动重启。", catalog.host)).size(13));
        }
    }
    let actions = row![
        button(text("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(close.clone())),
        button(text("保存可见性/顺序"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (!state.busy && state.draft.is_some())
                    .then_some(event(PreferencesMessage::SaveUser))
            ),
        button(text("保存主机设置"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(state.host_editable().then_some(event(PreferencesMessage::SaveHost))),
    ]
    .spacing(8);
    let card = settings_modal_card(
        column![
            row![
                container(text("应用设置").size(22)).width(Length::Fill),
                settings_close_button(close.clone())
            ],
            scrollable(container(body).padding([0, 7]).width(Length::Fill))
                .id("apps-preferences-body")
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .height(Length::Fill),
            actions,
        ]
        .spacing(16)
        .height(Length::Fill),
    )
    .width(Length::Fixed(560.0))
    .height(Length::Fixed(560.0));
    settings_modal_overlay(Some(base), close, card)
}
