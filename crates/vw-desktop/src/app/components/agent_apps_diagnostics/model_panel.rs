//! Reachability is deliberately separate from billable model and streaming checks.
use crate::app::{
    Message,
    components::system_settings_common::*,
    message::agent_apps_diagnostics::{
        model::{self, Event},
        model_state::*,
    },
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::agent_apps_diagnostics::*;

fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    let primary = matches!(&event, Event::Start(_));
    let control = button(text(label)).padding([8, 14]);
    let control = if primary {
        control.style(primary_action_btn_style)
    } else {
        control.style(rounded_action_btn_style)
    };
    control.on_press_maybe(enabled.then(|| model::message(event))).into()
}
pub(crate) fn overlay<'a>(
    state: &'a ModelState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(context) = &state.open else {
        return base;
    };
    let close = model::message(Event::Close);
    let header = settings_modal_header(
        text("模型诊断").size(20),
        Some(settings_close_button(close.clone())),
    );
    let mut body = column![
        text(format!("执行主机：{} · 应用：{:?} · 供应商：{}", context.scope.host, context.scope.app, context.provider.as_ref())).size(13),
        text("网络可达仅表示收到 HTTP 响应，不代表鉴权、模型或流式正确。"),
        action("检查网络可达性", Event::Start(DiagnosticProbeKind::Reachability), true),
        settings_field_label("已保存的模型 ID"),
        settings_text_input("输入执行主机已保存的模型 ID", &state.model).on_input(|v| model::message(Event::Model(v))),
        text("模型与流式检查将使用执行主机的真实授权发送最小请求，可能产生费用。点击下方按钮即开始检查。"),
        row![action("发送最小模型请求", Event::Start(DiagnosticProbeKind::Model), !state.model.trim().is_empty()),
            action("发送最小流式请求", Event::Start(DiagnosticProbeKind::Streaming), !state.model.trim().is_empty())].spacing(8).wrap(),
    ].spacing(12);
    for (index, run) in state.runs.iter().enumerate().filter(|(_, r)| &r.context == context) {
        let mut result = column![
            text(format!("{} · {}", kind_label(run.kind), run.model)).size(16),
            text(status(run))
        ]
        .spacing(6);
        if let Some(job) = &run.job {
            result = result.push(
                text(format!("任务：{} · 关联：{}", job.task_id.as_ref(), job.correlation_id))
                    .size(12),
            );
        }
        if let Some(snapshot) = &run.snapshot {
            result = result.push(text(format!(
                "进度：{} / {}",
                snapshot.progress.completed,
                snapshot.progress.total.map(|v| v.to_string()).unwrap_or_else(|| "未知".into())
            )));
            for evidence in snapshot.results.iter().flatten() {
                result = result.push(text(evidence_text(evidence)).size(13));
            }
            if snapshot.results.as_ref().is_none_or(|r| r.is_empty()) {
                result = result.push(text("详细证据不可用，未验证模型或流式。"));
            }
        }
        if let Some(error) = &run.error {
            result = result.push(settings_error_banner(error));
        }
        result = result.push(action(
            "取消此任务",
            Event::Cancel(index),
            run.job.is_some() && !run.terminal() && (!run.cancelling || run.error.is_some()),
        ));
        body = body.push(settings_panel(result));
    }
    let footer =
        row![action("刷新保留任务", Event::Refresh, true), action("关闭", Event::Close, true)]
            .spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
fn kind_label(kind: DiagnosticProbeKind) -> &'static str {
    match kind {
        DiagnosticProbeKind::Reachability => "网络可达性",
        DiagnosticProbeKind::Model => "模型",
        DiagnosticProbeKind::Streaming => "流式",
        _ => "其他",
    }
}
fn status(run: &ModelRun) -> String {
    if run.starting {
        return "正在提交，尚未取得任务 ID".into();
    }
    if run.cancelling && !run.terminal() {
        return "正在取消，等待执行主机确认清理".into();
    }
    match run.snapshot.as_ref().map(|s| s.state) {
        None => "等待查询，尚未验证".into(),
        Some(DiagnosticJobState::Queued {}) => "排队中".into(),
        Some(DiagnosticJobState::Running {}) => "检查中".into(),
        Some(DiagnosticJobState::Cancelling {}) => "正在取消，等待清理".into(),
        Some(DiagnosticJobState::Cancelled {}) => "已取消，资源已清理".into(),
        Some(DiagnosticJobState::Succeeded {}) => "任务完成，请查看下方证据".into(),
        Some(DiagnosticJobState::Failed { error }) => {
            format!("任务失败：{:?} / {:?}", error.phase, error.code)
        }
        Some(DiagnosticJobState::Interrupted {}) => "任务中断，未验证".into(),
    }
}
fn report_text<E>(report: &DiagnosticReport<E>, success: impl FnOnce(&E) -> String) -> String {
    let source = match report.source {
        DiagnosticEvidenceSource::RealService => "真实服务",
        DiagnosticEvidenceSource::ControlledFixture => "受控夹具（非真实供应商）",
    };
    let outcome = match &report.outcome {
        DiagnosticOutcome::Succeeded { evidence } => success(evidence),
        DiagnosticOutcome::Failed { error, http_status } => {
            format!("失败：{:?} / {:?}，HTTP {:?}", error.phase, error.code, http_status)
        }
        DiagnosticOutcome::Cancelled {} => "已取消".into(),
    };
    let timings = report
        .timings
        .iter()
        .map(|t| format!("{:?} {}ms", t.phase, t.elapsed_ms))
        .collect::<Vec<_>>()
        .join("，");
    format!("来源：{source} · {outcome}\n{timings}")
}
pub(crate) fn evidence_text(result: &DiagnosticResult) -> String {
    match result {
        DiagnosticResult::Reachability { report, .. } => report_text(report, |e| {
            format!(
                "网络可达，HTTP {}；鉴权/模型/流式未验证{}",
                e.http_status,
                if e.slow { "；响应较慢" } else { "" }
            )
        }),
        DiagnosticResult::Model { report, may_incur_cost, .. } => format!(
            "{}\n可能计费：{}",
            report_text(report, |e| format!(
                "模型响应有效，HTTP {}，实际模型：{}",
                e.http_status,
                e.actual_model_id.as_ref()
            )),
            may_incur_cost
        ),
        DiagnosticResult::Streaming { report, may_incur_cost, .. } => format!(
            "{}\n可能计费：{}",
            report_text(report, |e| format!(
                "流式增量与结束有效，HTTP {}，实际模型：{}，首事件 {}ms，最大间隔 {:?}ms",
                e.http_status,
                e.actual_model_id.as_ref(),
                e.first_event_ms,
                e.max_event_gap_ms
            )),
            may_incur_cost
        ),
        _ => "证据类型不匹配，未验证".into(),
    }
}
