//! Reachability is deliberately separate from billable model and streaming checks.
use desktop_ui_controls::system_settings_common::*;
use super::{model::{self, Event}, model_state::*};
type Message = Event;
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_api_app_diagnostics::*;

fn action(i18n: desktop_i18n::I18n, label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    let primary = matches!(&event, Event::Start(_));
    let control = button(text(i18n.__(label))).padding([8, 14]);
    let control = if primary {
        control.style(primary_action_btn_style)
    } else {
        control.style(rounded_action_btn_style)
    };
    control.on_press_maybe(enabled.then(|| model::message(event))).into()
}
pub fn overlay<'a>(i18n: desktop_i18n::I18n,
    state: &'a ModelState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(context) = &state.open else {
        return base;
    };
    let close = model::message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("模型诊断")).size(20),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(i18n.sprintf("应用：%s · 供应商：%s", &[format!("{:?}", context.scope.app).into(), format!("{}", context.provider.as_ref()).into()])).size(13),
        text(i18n.__("网络可达仅表示收到 HTTP 响应，不代表鉴权、模型或流式正确。")),
        action(i18n, i18n.__("检查网络可达性"), Event::Start(DiagnosticProbeKind::Reachability), true),
        settings_field_label(i18n.__("已保存的模型 ID")),
        settings_text_input(i18n.__("输入已保存的模型 ID"), &state.model).on_input(|v| model::message(Event::Model(v))),
        text(i18n.__("模型与流式检查会发送实际请求，可能产生费用。")),
        row![action(i18n, i18n.__("发送最小模型请求"), Event::Start(DiagnosticProbeKind::Model), !state.model.trim().is_empty()),
            action(i18n, i18n.__("发送最小流式请求"), Event::Start(DiagnosticProbeKind::Streaming), !state.model.trim().is_empty())].spacing(8).wrap(),
    ].spacing(12);
    for (index, run) in state.runs.iter().enumerate().filter(|(_, r)| &r.context == context) {
        let mut result = column![
            text(format!("{} · {}", kind_label(i18n, run.kind), run.model)).size(16),
            text(status(i18n, run))
        ]
        .spacing(6);
        if let Some(job) = &run.job {
            result = result.push(
                text(job_label(i18n, job.task_id.as_ref(), &job.correlation_id))
                    .size(12),
            );
        }
        if let Some(snapshot) = &run.snapshot {
            result = result.push(text(i18n.sprintf("进度：%u / %s", &[(snapshot.progress.completed).into(), format!("{}", snapshot.progress.total.map(|v| v.to_string()).unwrap_or_else(|| i18n.__("未知").into())).into()])));
            for evidence in snapshot.results.iter().flatten() {
                result = result.push(text(evidence_text(i18n, evidence)).size(13));
            }
            if snapshot.results.as_ref().is_none_or(|r| r.is_empty()) {
                result = result.push(text(i18n.__("详细证据不可用，未验证模型或流式。")));
            }
        }
        if let Some(error) = &run.error {
            result = result.push(settings_error_banner(i18n.__(error).to_owned()));
        }
        result = result.push(action(i18n,
            i18n.__("取消此任务"),
            Event::Cancel(index),
            run.job.is_some() && !run.terminal() && (!run.cancelling || run.error.is_some()),
        ));
        body = body.push(settings_panel(result));
    }
    let footer =
        row![action(i18n, i18n.__("刷新保留任务"), Event::Refresh, true), action(i18n, i18n.__("关闭"), Event::Close, true)]
            .spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
fn kind_label(i18n: desktop_i18n::I18n, kind: DiagnosticProbeKind) -> &'static str {
    match kind {
        DiagnosticProbeKind::Reachability => i18n.__("网络可达性"),
        DiagnosticProbeKind::Model => i18n.__("模型"),
        DiagnosticProbeKind::Streaming => i18n.__("流式"),
        _ => i18n.__("其他"),
    }
}
fn status(i18n: desktop_i18n::I18n, run: &ModelRun) -> String {
    if run.starting {
        return i18n.__("正在提交，尚未取得任务 ID").into();
    }
    if run.cancelling && !run.terminal() {
        return i18n.__("正在取消，等待清理完成").into();
    }
    match run.snapshot.as_ref().map(|s| s.state) {
        None => i18n.__("等待查询，尚未验证").into(),
        Some(DiagnosticJobState::Queued {}) => i18n.__("排队中").into(),
        Some(DiagnosticJobState::Running {}) => i18n.__("检查中").into(),
        Some(DiagnosticJobState::Cancelling {}) => i18n.__("正在取消，等待清理").into(),
        Some(DiagnosticJobState::Cancelled {}) => i18n.__("已取消，资源已清理").into(),
        Some(DiagnosticJobState::Succeeded {}) => i18n.__("任务完成，请查看下方证据").into(),
        Some(DiagnosticJobState::Failed { error }) => {
            i18n.sprintf("任务失败：%s / %s", &[format!("{:?}", error.phase).into(), format!("{:?}", error.code).into()])
        }
        Some(DiagnosticJobState::Interrupted {}) => i18n.__("任务中断，未验证").into(),
    }
}
fn report_text<E>(i18n: desktop_i18n::I18n, report: &DiagnosticReport<E>, success: impl FnOnce(&E) -> String) -> String {
    let source = match report.source {
        DiagnosticEvidenceSource::RealService => i18n.__("真实服务"),
        DiagnosticEvidenceSource::ControlledFixture => i18n.__("受控夹具（非真实供应商）"),
    };
    let outcome = match &report.outcome {
        DiagnosticOutcome::Succeeded { evidence } => success(evidence),
        DiagnosticOutcome::Failed { error, http_status } => {
            i18n.sprintf("失败：%s / %s，HTTP %s", &[format!("{:?}", error.phase).into(), format!("{:?}", error.code).into(), format!("{:?}", http_status).into()])
        }
        DiagnosticOutcome::Cancelled {} => i18n.__("已取消").into(),
    };
    let timings = report
        .timings
        .iter()
        .map(|t| format!("{:?} {}ms", t.phase, t.elapsed_ms))
        .collect::<Vec<_>>()
        .join("，");
    i18n.sprintf("来源：%s · %s\n%s", &[format!("{}", source).into(), format!("{}", outcome).into(), format!("{}", timings).into()])
}
pub fn evidence_text(i18n: desktop_i18n::I18n, result: &DiagnosticResult) -> String {
    match result {
        DiagnosticResult::Reachability { report, .. } => report_text(i18n, report, |e| {
            i18n.sprintf("网络可达，HTTP %s；鉴权/模型/流式未验证%s", &[format!("{}", e.http_status).into(), format!("{}", if e.slow { i18n.__("；响应较慢") } else { "" }).into()])
        }),
        DiagnosticResult::Model { report, may_incur_cost, .. } => i18n.sprintf("%s\n可能计费：%s", &[format!("{}", report_text(i18n, report, |e| i18n.sprintf("模型响应有效，HTTP %s，实际模型：%s", &[format!("{}", e.http_status).into(), format!("{}", e.actual_model_id.as_ref()).into()]))).into(), format!("{}", may_incur_cost).into()]),
        DiagnosticResult::Streaming { report, may_incur_cost, .. } => i18n.sprintf("%s\n可能计费：%s", &[format!("{}", report_text(i18n, report, |e| i18n.sprintf("流式增量与结束有效，HTTP %s，实际模型：%s，首事件 %sms，最大间隔 %sms", &[format!("{}", e.http_status).into(), format!("{}", e.actual_model_id.as_ref()).into(), format!("{}", e.first_event_ms).into(), format!("{:?}", e.max_event_gap_ms).into()]))).into(), format!("{}", may_incur_cost).into()]),
        _ => i18n.__("证据类型不匹配，未验证").into(),
    }
}

pub(crate) fn job_label(i18n: desktop_i18n::I18n, task: &str, correlation: &str) -> String {
    i18n.sprintf("任务：%s · 关联：%s", &[task.into(), correlation.into()])
}
