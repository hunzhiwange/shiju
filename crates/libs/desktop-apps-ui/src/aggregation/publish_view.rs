use super::list_view::notice_label;
use desktop_ui_controls::system_settings_common::*;
use super::publish::{Action, Event, Session};
type Message = Event;
fn message(event: Event) -> Event { event }
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_api_app_aggregation::*;
use vw_api_task::TaskStatus;

pub fn overlay<'a>(i18n: desktop_i18n::I18n,
    session: &'a Session,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let exiting = session.exit_state.is_some();
    let close = message(Event::Dismiss);
    let header = column![
        settings_modal_header(
            text(if exiting { i18n.__("回到直连") } else { i18n.__("切换到聚合模式") }).size(20),
            settings_close_button(close.clone()),
        ),
        settings_divider(),
    ]
    .spacing(12);
    let mut body =
        column![text(i18n.__("请核对默认供应商、发布模型和限制。聚合不支持故障转移。"))].spacing(12);
    if let Some(state) = &session.exit_state {
        body = column![text(i18n.__("恢复已保存的直连配置，保留聚合名单与稳定标识。请在恢复后按提示重选模型或重启客户端。"))].spacing(12);
        body = body.push(text(i18n.sprintf("聚合名单保留：%u 项", &[(state.members.len()).into()])));
    }
    if session.busy {
        body = body.push(text(i18n.__("正在加载…")));
    }
    if let Some(error) = session.error {
        body = body.push(settings_error_banner(i18n.__(error)));
    }
    if let Some(preview) = &session.preview {
        let default = preview.default_provider_id.as_deref().unwrap_or(i18n.__("未设置"));
        let default_name = preview
            .members
            .iter()
            .find(|m| m.provider_id == default)
            .map(|m| m.name.as_str())
            .unwrap_or(default);
        body = body.push(settings_panel(
            column![settings_field_label(i18n.__("默认供应商")), text(default_name.to_owned()).size(15),]
                .spacing(6),
        ));
        for member in &preview.members {
            let mut models =
                column![text(i18n.sprintf("%s · %u 个模型", &[format!("{}", member.name).into(), (member.models.len()).into()]))]
                    .spacing(6);
            for model in &member.models {
                models = models.push(text(format!(
                    "{} · {}{}",
                    model.label,
                    model.id,
                    if model.supports_1m { i18n.__(" · 支持 1M") } else { "" }
                )));
            }
            body = body.push(settings_panel(models));
        }
        for issue in &preview.issues {
            let label = match issue.code {
                PreviewIssueCode::ModelCollision => i18n.__("模型标识冲突"),
                PreviewIssueCode::InvalidMember => i18n.__("成员不可用"),
                PreviewIssueCode::MissingDefault => i18n.__("缺少默认供应商"),
                PreviewIssueCode::InvalidDefault => i18n.__("默认供应商无效"),
                PreviewIssueCode::FailoverEnabled => i18n.__("请先关闭故障转移"),
                PreviewIssueCode::UnrestorableDirect => i18n.__("无法安全恢复直连配置"),
            };
            body = body.push(text(format!(
                "{}：{label} · {} · {}",
                if issue.blocking { i18n.__("阻止发布") } else { i18n.__("提示") },
                issue.provider_id.as_deref().unwrap_or("—"),
                issue.model_id.as_deref().unwrap_or("—")
            )));
        }
        for notice in &preview.notices {
            body = body.push(text(notice_label(i18n, *notice)).style(settings_muted_text_style));
        }
    }
    if let Some(op) = &session.operation {
        let status = match (&op.status, &op.aggregation_outcome) {
            (TaskStatus::Completed, Some(AggregationOutcome::Exited { state }))
                if exiting
                    && state.mode == AggregationMode::Direct
                    && !state.attached
                    && matches!(state.recovery, RecoveryState::Restored | RecoveryState::None) =>
            {
                i18n.__("已回到直连，恢复成功")
            }
            (_, Some(AggregationOutcome::Exited { .. })) => i18n.__("直连恢复尚未成功，请刷新核对实际状态"),
            (TaskStatus::Completed, Some(AggregationOutcome::Published { .. })) if !exiting => {
                i18n.__("聚合已发布，请按提示刷新客户端")
            }
            (TaskStatus::Cancelled, _) => i18n.__("操作已取消，请刷新实际状态"),
            (_, Some(AggregationOutcome::Failed { .. })) | (TaskStatus::Failed, _) => {
                i18n.__("操作失败，请检查恢复状态")
            }
            (TaskStatus::Running | TaskStatus::Queued, _) => {
                if exiting {
                    i18n.__("直连恢复进行中")
                } else {
                    i18n.__("发布进行中")
                }
            }
            _ => i18n.__("操作已结束，请刷新核对实际状态"),
        };
        body = body.push(settings_panel(
            column![
                text(status),
                text(i18n.sprintf("进度：%u / %s · 阶段：%s", &[(op.completed).into(), format!("{}", op.total).into(), format!("{:?}", op.phase).into()]))
            ]
            .spacing(6),
        ));
        if op.cancellation_requested {
            body = body.push(text(i18n.__("已请求取消，等待确认；发布阶段可能无法取消。")));
        }
        match &op.aggregation_outcome {
            Some(AggregationOutcome::Published { state })
            | Some(AggregationOutcome::Exited { state }) => {
                for notice in &state.notices {
                    body = body.push(text(notice_label(i18n, *notice)));
                }
                body = body.push(text(i18n.sprintf("恢复状态：%s", &[format!("{}", recovery_label(i18n, state.recovery)).into()])));
                if exiting {
                    body = body.push(text(i18n.sprintf("保留聚合名单：%u 项", &[(state.members.len()).into()])));
                }
            }
            Some(AggregationOutcome::Failed { status, code, recovery }) => {
                body = body.push(text(i18n.sprintf("目标错误：%s / %s · 恢复状态：%s", &[format!("{}", status).into(), format!("{:?}", code).into(), format!("{}", recovery_label(i18n, *recovery)).into()])));
            }
            _ => {}
        }
    }
    if let Some(state) = &session.exit_state {
        for member in &state.members {
            body = body.push(text(i18n.sprintf("保留成员：%s", &[format!("{}", member.name).into()])));
        }
    }
    let mut footer = row![
        button(text(if session.operation.is_some() { i18n.__("返回目录") } else { i18n.__("取消") }))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!session.busy && !session.running()).then_some(close.clone()))
    ]
    .spacing(12);
    if session.operation.is_some() {
        footer = footer.push(
            button(text(i18n.__("查询进度")))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe((!session.busy).then(|| message(Event::Act(Action::Poll)))),
        );
        footer = footer.push(
            button(text(if exiting { i18n.__("取消恢复") } else { i18n.__("取消发布") }))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (!session.busy
                        && session.running()
                        && session.operation.as_ref().is_some_and(|p| !p.cancellation_requested))
                    .then(|| message(Event::Act(Action::Cancel))),
                ),
        );
    } else {
        footer = footer.push(
            button(text(if exiting { i18n.__("确认回到直连") } else { i18n.__("发布聚合目录") }))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    session.can_confirm().then(|| message(Event::Act(Action::Confirm))),
                ),
        );
    }
    if exiting && !session.busy && !session.running() && session.error.is_some() {
        footer = footer.push(
            button(text(i18n.__("刷新后重试")))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(message(Event::RetryExit)),
        );
    }
    let footer =
        column![settings_divider(), container(footer.wrap()).align_right(Length::Fill)].spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(620).height(620);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

fn recovery_label(i18n: desktop_i18n::I18n, recovery: RecoveryState) -> &'static str {
    match recovery {
        RecoveryState::None => i18n.__("无需恢复"),
        RecoveryState::Pending => i18n.__("等待恢复"),
        RecoveryState::Restored => i18n.__("已恢复"),
        RecoveryState::Failed => i18n.__("恢复失败"),
    }
}

#[cfg(test)]
#[path = "aggregation_publish_tests.rs"]
mod tests;
