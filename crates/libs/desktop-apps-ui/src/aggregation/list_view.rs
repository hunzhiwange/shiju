use desktop_ui_controls::system_settings_common::*;
use super::list::{Action, Event, Form};
type Message = Event;
fn message(event: Event) -> Event { event }
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text},
};

pub fn overlay<'a>(i18n: desktop_i18n::I18n, form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    if form.clients.is_some() || form.publish.is_some() {
        return iced::widget::stack![settings_inactive_view(base), dialogs(i18n, form)].into();
    }
    let close = message(Event::Close);
    let header = column![
        settings_modal_header(text(i18n.__("聚合目录")).size(20), settings_close_button(close.clone())),
        settings_divider(),
    ]
    .spacing(12);
    let footer =
        column![settings_divider(), container(actions(i18n, form)).align_right(Length::Fill)].spacing(12);
    let card = settings_modal_layout(header, sections(i18n, form, "", false), Some(footer.into()))
        .width(760)
        .height(620);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

pub fn dialogs(i18n: desktop_i18n::I18n, form: &Form) -> Element<'_, Event> {
    if let Some(session) = &form.clients {
        return super::clients_view::overlay(i18n, session, iced::widget::Space::new().into())
            .map(Event::Clients);
    }
    if let Some(session) = &form.publish {
        return super::publish_view::overlay(i18n, session, iced::widget::Space::new().into())
            .map(Event::Publish);
    }
    iced::widget::Space::new().into()
}

pub fn page<'a>(i18n: desktop_i18n::I18n, form: &'a Form, query: &str) -> Element<'a, Event> {
    sections(i18n, form, query, true)
}

fn actions(i18n: desktop_i18n::I18n, form: &Form) -> Element<'_, Event> {
    row![
        button(text(i18n.__("预览发布"))).padding([8, 14]).style(primary_action_btn_style).on_press_maybe(
            (!form.loading && form.snapshot.is_some())
                .then(|| message(Event::Publish(super::publish::Event::Open)))
        ),
        button(text(i18n.__("回到直连"))).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe(
            (!form.loading
                && form.snapshot.as_ref().is_some_and(
                    |s| s.state.mode != vw_api_app_aggregation::AggregationMode::Direct
                ))
            .then(|| message(Event::Publish(super::publish::Event::OpenExit)))
        ),
        button(text(i18n.__("刷新")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!form.loading).then(|| message(Event::Refresh))),
    ]
    .spacing(8)
    .wrap()
    .into()
}

fn sections<'a>(i18n: desktop_i18n::I18n, form: &'a Form, query: &str, inline: bool) -> Element<'a, Event> {
    let query = query.trim().to_lowercase();
    let matches = |name: &str, id: &str| {
        query.is_empty()
            || name.to_lowercase().contains(&query)
            || id.to_lowercase().contains(&query)
    };
    let mut body = column![].spacing(12);
    if form.selection.scope.app == vw_api_app_identity::AppId::Codex {
        body = body.push(
            button(text(i18n.__("Codex 客户端状态")))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe((!form.loading).then(|| {
                    message(Event::Clients(
                        super::clients::Event::Open,
                    ))
                })),
        );
    }
    if form.loading {
        body = body.push(settings_panel(text(i18n.__("正在加载聚合目录…"))));
    }
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(i18n.__(error)));
    }
    if let Some(snapshot) = &form.snapshot {
        let state = &snapshot.state;
        use vw_api_app_aggregation::AggregationMode;
use vw_api_app_aggregation::RecoveryState;
        let mode = match state.mode {
            AggregationMode::Direct => i18n.__("直连"),
            AggregationMode::Routing => i18n.__("路由"),
            AggregationMode::Aggregation => i18n.__("聚合"),
        };
        let recovery = match state.recovery {
            RecoveryState::None => i18n.__("无需恢复"),
            RecoveryState::Pending => i18n.__("等待恢复"),
            RecoveryState::Restored => i18n.__("已恢复"),
            RecoveryState::Failed => i18n.__("恢复失败"),
        };
        let default = state
            .default_provider_id
            .as_deref()
            .map(|id| {
                state
                    .members
                    .iter()
                    .find(|m| m.provider_id == id)
                    .map(|m| m.name.as_str())
                    .unwrap_or(id)
            })
            .unwrap_or(i18n.__("未设置"));
        let summary = column![
            text(i18n.sprintf("当前连接 · %s", &[format!("{}", mode).into()])).size(15),
            text(format!(
                "{} · {recovery}",
                if state.attached { i18n.__("目录已发布") } else { i18n.__("目录未发布") }
            ))
            .size(12)
            .style(settings_muted_text_style),
        ]
        .spacing(5)
        .width(Length::Fill);
        let mut status = row![summary].spacing(10).align_y(Alignment::Center);
        if inline {
            status = status.push(actions(i18n, form));
        }
        body = body.push(settings_panel(status));
        body = body.push(settings_panel(
            column![settings_field_label(i18n.__("默认供应商")), text(default).size(15),].spacing(5),
        ));
        let added = state
            .members
            .iter()
            .filter(|member| {
                state.default_provider_id.as_deref() != Some(member.provider_id.as_str())
                    && matches(&member.name, &member.provider_id)
            })
            .collect::<Vec<_>>();
        body = body.push(text(i18n.sprintf("已添加 · %u", &[(added.len()).into()])).size(14));
        if added.is_empty() {
            body = body.push(settings_panel(
                text(if query.is_empty() {
                    i18n.__("尚无其他聚合成员")
                } else {
                    i18n.__("没有匹配的聚合成员")
                })
                .size(13)
                .style(settings_muted_text_style),
            ));
        }
        for member in added {
            let role = if state.default_provider_id.as_deref() == Some(&member.provider_id) {
                i18n.__("默认 · 已添加")
            } else {
                i18n.__("已添加")
            };
            let count = if state.attached {
                i18n.sprintf("%u 个模型", &[(member.models.len()).into()])
            } else {
                i18n.__("模型数待发布后确认").into()
            };
            body = body.push(settings_panel(
                row![
                    column![
                        text(&member.name).size(15),
                        text(format!("{role} · {count}")).size(12).style(settings_muted_text_style),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    row![
                        button(text(i18n.__("设为默认")))
                            .padding([8, 14])
                            .style(primary_action_btn_style)
                            .on_press_maybe(
                                (!form.loading
                                    && state.default_provider_id.as_deref()
                                        != Some(&member.provider_id))
                                .then(|| message(Event::Act(Action::Default {
                                    provider_id: member.provider_id.clone()
                                })))
                            ),
                        button(text(i18n.__("移除")))
                            .padding([8, 14])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!form.loading
                                    && state.default_provider_id.as_deref()
                                        != Some(&member.provider_id))
                                .then(|| message(Event::Act(Action::Member {
                                    provider_id: member.provider_id.clone(),
                                    enabled: false
                                })))
                            ),
                    ]
                    .spacing(8)
                    .wrap(),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ));
        }
        let candidates = snapshot
            .candidates
            .iter()
            .filter(|provider| matches(&provider.name, &provider.id))
            .collect::<Vec<_>>();
        body = body.push(text(i18n.sprintf("可添加 · %u", &[(candidates.len()).into()])).size(14));
        if candidates.is_empty() {
            body = body.push(settings_panel(
                text(if query.is_empty() {
                    i18n.__("没有可添加的供应商")
                } else {
                    i18n.__("没有匹配的供应商")
                })
                .size(13)
                .style(settings_muted_text_style),
            ));
        }
        for provider in candidates {
            body = body.push(settings_panel(
                row![
                    text(&provider.name).size(15).width(Length::Fill),
                    row![
                        button(text(i18n.__("添加")))
                            .padding([8, 14])
                            .style(primary_action_btn_style)
                            .on_press_maybe((!form.loading).then(|| message(Event::Act(
                                Action::Member { provider_id: provider.id.clone(), enabled: true }
                            )))),
                        button(text(i18n.__("设为默认")))
                            .padding([8, 14])
                            .style(rounded_action_btn_style)
                            .on_press_maybe((!form.loading).then(|| message(Event::Act(
                                Action::Default { provider_id: provider.id.clone() }
                            )))),
                    ]
                    .spacing(8)
                    .wrap(),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ));
        }
        for notice in &state.notices {
            body = body.push(text(notice_label(i18n, *notice)).style(settings_muted_text_style));
        }
    } else if inline && !form.loading {
        body = body.push(actions(i18n, form));
    }
    body.into()
}

pub(super) fn notice_label(i18n: desktop_i18n::I18n,
    notice: vw_api_app_aggregation::AggregationNotice,
) -> &'static str {
    use vw_api_app_aggregation::AggregationNotice::*;
    match notice {
        ModelReselect => i18n.__("请重新选择模型"),
        NewSession => i18n.__("请新建会话以使用新目录"),
        RestartDesktop => i18n.__("请重启桌面客户端以刷新目录"),
        RestartEditor => i18n.__("请重启编辑器以刷新目录"),
        CodexRouteOwnsCatalog => i18n.__("当前路由使用供应商自有目录"),
        CodexOfficialCatalogBundled => i18n.__("正在使用内置官方目录"),
        CodexOfficialCatalogUnavailable => i18n.__("官方目录不可用"),
        CodexOfficialCatalogOutdated => i18n.__("官方目录需要更新"),
        EncryptedTaskCrossProvider => i18n.__("加密任务跨供应商继续执行可能受限"),
        MultiAgentV2Forced => i18n.__("当前配置要求启用多代理模式"),
    }
}


#[cfg(test)]
#[path = "aggregation_list_tests.rs"]
mod tests;
