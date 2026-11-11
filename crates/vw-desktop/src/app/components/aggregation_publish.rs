use super::notice_label;
use crate::app::components::system_settings_common::*;
use crate::app::{
    Message,
    message::settings::aggregation_list::publish::{Action, Event, Session, message},
};
use iced::{
    Element,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::{agent_apps::aggregation::*, task::TaskStatus};

pub(super) fn overlay<'a>(
    session: &'a Session,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let close = message(Event::Dismiss);
    let header =
        settings_modal_header(text("确认聚合发布").size(20), settings_close_button(close.clone()));
    let mut body =
        column![text("请核对默认供应商、发布模型和限制。聚合不支持故障转移。")].spacing(12);
    if session.busy {
        body = body.push(text("正在请求执行主机…"));
    }
    if let Some(error) = session.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(preview) = &session.preview {
        let default = preview.default_provider_id.as_deref().unwrap_or("未设置");
        let default_name = preview
            .members
            .iter()
            .find(|m| m.provider_id == default)
            .map(|m| m.name.as_str())
            .unwrap_or(default);
        body = body.push(settings_panel(text(format!("默认供应商：{default_name}"))));
        for member in &preview.members {
            let mut models =
                column![text(format!("{} · {} 个模型", member.name, member.models.len()))]
                    .spacing(6);
            for model in &member.models {
                models = models.push(text(format!(
                    "{} · {}{}",
                    model.label,
                    model.id,
                    if model.supports_1m { " · 支持 1M" } else { "" }
                )));
            }
            body = body.push(settings_panel(models));
        }
        for issue in &preview.issues {
            let label = match issue.code {
                PreviewIssueCode::ModelCollision => "模型标识冲突",
                PreviewIssueCode::InvalidMember => "成员不可用",
                PreviewIssueCode::MissingDefault => "缺少默认供应商",
                PreviewIssueCode::InvalidDefault => "默认供应商无效",
                PreviewIssueCode::FailoverEnabled => "请先关闭故障转移",
                PreviewIssueCode::UnrestorableDirect => "无法安全恢复直连配置",
            };
            body = body.push(text(format!(
                "{}：{label} · {} · {}",
                if issue.blocking { "阻止发布" } else { "提示" },
                issue.provider_id.as_deref().unwrap_or("—"),
                issue.model_id.as_deref().unwrap_or("—")
            )));
        }
        for notice in &preview.notices {
            body = body.push(text(notice_label(*notice)).style(settings_muted_text_style));
        }
    }
    if let Some(op) = &session.operation {
        let status = match (&op.status, &op.aggregation_outcome) {
            (TaskStatus::Completed, Some(AggregationOutcome::Published { .. })) => {
                "聚合已发布，请按提示刷新客户端"
            }
            (TaskStatus::Cancelled, _) => "操作已取消，请刷新实际状态",
            (_, Some(AggregationOutcome::Failed { .. })) | (TaskStatus::Failed, _) => {
                "发布失败，请检查恢复状态"
            }
            (TaskStatus::Running | TaskStatus::Queued, _) => "发布进行中",
            _ => "操作已结束，请刷新核对实际状态",
        };
        body = body.push(settings_panel(
            column![
                text(status),
                text(format!("进度：{} / {} · 阶段：{:?}", op.completed, op.total, op.phase))
            ]
            .spacing(6),
        ));
        if op.cancellation_requested {
            body = body.push(text("已请求取消，等待执行主机确认；发布阶段可能无法取消。"));
        }
        match &op.aggregation_outcome {
            Some(AggregationOutcome::Published { state }) => {
                for notice in &state.notices {
                    body = body.push(text(notice_label(*notice)));
                }
                body = body.push(text(format!("恢复状态：{:?}", state.recovery)));
            }
            Some(AggregationOutcome::Failed { status, code, recovery }) => {
                body = body
                    .push(text(format!("目标错误：{status} / {code:?} · 恢复状态：{recovery:?}")));
            }
            _ => {}
        }
    }
    let mut footer = row![
        button(text(if session.operation.is_some() { "返回目录" } else { "取消" }))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!session.busy && !session.running()).then_some(close.clone()))
    ]
    .spacing(12);
    if session.operation.is_some() {
        footer = footer.push(
            button(text("查询进度"))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press_maybe((!session.busy).then(|| message(Event::Act(Action::Poll)))),
        );
        footer = footer.push(
            button(text("取消发布"))
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
            button(text("确认发布"))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    session.can_confirm().then(|| message(Event::Act(Action::Confirm))),
                ),
        );
    }
    let card = settings_modal_layout(header, body, Some(footer.into())).width(720).height(620);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

#[cfg(test)]
#[path = "aggregation_publish_tests.rs"]
mod tests;
