use super::system_settings_common::*;
use crate::app::{
    Message,
    message::settings::aggregation_list::{Action, Event, Form, message},
};
use iced::{
    Element,
    widget::{button, column, container, row, text},
};

pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = message(Event::Close);
    let header =
        settings_modal_header(text("聚合目录").size(20), settings_close_button(close.clone()));
    let mut body = column![].spacing(12);
    if form.loading {
        body = body.push(settings_panel(text("正在加载聚合目录…")));
    }
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(snapshot) = &form.snapshot {
        let state = &snapshot.state;
        use vw_gateway_client::vw_api_types::agent_apps::aggregation::{
            AggregationMode, RecoveryState,
        };
        let mode = match state.mode {
            AggregationMode::Direct => "直连",
            AggregationMode::Routing => "路由",
            AggregationMode::Aggregation => "聚合",
        };
        let recovery = match state.recovery {
            RecoveryState::None => "无需恢复",
            RecoveryState::Pending => "等待恢复",
            RecoveryState::Restored => "已恢复",
            RecoveryState::Failed => "恢复失败",
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
            .unwrap_or("未设置");
        body = body.push(settings_panel(
            column![
                text(format!("默认供应商：{default}")),
                text(format!(
                    "模式：{mode} · {} · {recovery}",
                    if state.attached { "目录已发布" } else { "目录未发布" }
                ))
            ]
            .spacing(8),
        ));
        body = body.push(settings_field_label("已添加"));
        if state.members.is_empty() {
            body = body.push(text("尚无聚合成员").style(settings_muted_text_style));
        }
        for member in &state.members {
            let role = if state.default_provider_id.as_deref() == Some(&member.provider_id) {
                "默认 · 已添加"
            } else {
                "已添加"
            };
            let count = if state.attached {
                format!("{} 个模型", member.models.len())
            } else {
                "模型数待发布后确认".into()
            };
            body = body.push(settings_panel(
                column![
                    text(&member.name).size(17),
                    text(format!("{role} · {count}")),
                    row![
                        button(text("设为默认"))
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
                        button(text("移除"))
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
                    .spacing(12),
                ]
                .spacing(6),
            ));
        }
        body = body.push(settings_field_label("可添加候选"));
        body = body.push(
            text("以下为未加入的已保存供应商；资格与模型数待网关校验。")
                .size(13)
                .style(settings_muted_text_style),
        );
        if snapshot.candidates.is_empty() {
            body = body.push(text("没有可添加候选").style(settings_muted_text_style));
        }
        for provider in &snapshot.candidates {
            body = body.push(settings_panel(
                column![
                    text(&provider.name),
                    row![
                        button(text("添加"))
                            .padding([8, 14])
                            .style(primary_action_btn_style)
                            .on_press_maybe((!form.loading).then(|| message(Event::Act(
                                Action::Member { provider_id: provider.id.clone(), enabled: true }
                            )))),
                        button(text("设为默认"))
                            .padding([8, 14])
                            .style(rounded_action_btn_style)
                            .on_press_maybe((!form.loading).then(|| message(Event::Act(
                                Action::Default { provider_id: provider.id.clone() }
                            )))),
                    ]
                    .spacing(12),
                ]
                .spacing(6),
            ));
        }
        for notice in &state.notices {
            body = body.push(text(notice_label(*notice)).style(settings_muted_text_style));
        }
    }
    let footer = row![
        button(text("刷新"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!form.loading).then(|| message(Event::Refresh))),
        button(text("关闭"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(close.clone())
    ]
    .spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(700).height(600);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

#[cfg(test)]
#[path = "aggregation_list_tests.rs"]
mod tests;

fn notice_label(
    notice: vw_gateway_client::vw_api_types::agent_apps::aggregation::AggregationNotice,
) -> &'static str {
    use vw_gateway_client::vw_api_types::agent_apps::aggregation::AggregationNotice::*;
    match notice {
        ModelReselect => "请重新选择模型",
        NewSession => "请新建会话以使用新目录",
        RestartDesktop => "请重启桌面客户端以刷新目录",
        RestartEditor => "请重启编辑器以刷新目录",
        CodexRouteOwnsCatalog => "当前路由使用供应商自有目录",
        CodexOfficialCatalogBundled => "正在使用内置官方目录",
        CodexOfficialCatalogUnavailable => "官方目录不可用",
        CodexOfficialCatalogOutdated => "官方目录需要更新",
        EncryptedTaskCrossProvider => "加密任务跨供应商继续执行可能受限",
        MultiAgentV2Forced => "当前配置要求启用多代理模式",
    }
}
