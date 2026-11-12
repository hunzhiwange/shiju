use crate::app::{
    Message,
    components::system_settings_common::*,
    message::agent_apps_diagnostics::{
        endpoint_state::State,
        endpoints::{self, Event},
    },
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::agent_apps_diagnostics::*;
fn action(
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    let b = button(text(label)).padding([8, 14]);
    let b =
        if primary { b.style(primary_action_btn_style) } else { b.style(rounded_action_btn_style) };
    b.on_press_maybe(enabled.then(|| endpoints::message(event))).into()
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = endpoints::message(Event::Close);
    let header = settings_modal_header(
        text("端点测速与选择").size(20),
        Some(settings_close_button(close.clone())),
    );
    let mut body = column![text("测速为热身后 GET 响应头耗时，不代表模型速度或鉴权成功。"),
        text("合并当前草稿、已保存配置、预设和自定义地址簿。选择仅回填草稿，供应商表单保存后才写入端点。自定义地址通过「端点与请求选项」维护。"),
        row![action("开始测速", Event::Start, !state.loading && !state.active() && !state.candidates.is_empty(), true),
            action("取消测速", Event::Cancel, state.active() && (!state.cancelling || state.error.is_some()), false)].spacing(8)].spacing(12);
    body = body
        .push(settings_field_label("预设名称（可选）"))
        .push(
            settings_text_input("例如 PackyCode；留空不合并预设", &state.preset_name)
                .on_input_maybe(
                    (!state.active() && !state.loading)
                        .then_some(|v| endpoints::message(Event::Preset(v))),
                ),
        )
        .push(action("刷新候选", Event::Reload, !state.active() && !state.loading, false));
    body = body.push(action(
        "管理自定义端点",
        Event::ManageCustom,
        !state.active() && !state.loading,
        false,
    ));
    if state.loading {
        body = body.push(text("正在读取候选端点…"));
    }
    if let Some(snapshot) = &state.snapshot {
        body = body.push(text(format!(
            "进度：{} / {} · {:?}",
            snapshot.progress.completed,
            snapshot.progress.total.map(|n| n.to_string()).unwrap_or_else(|| "未知".into()),
            snapshot.state
        )));
    }
    if state.cancelling {
        body = body.push(text("正在取消测速，等待执行主机确认清理…"));
    }
    for candidate in state.sorted() {
        let sources = candidate
            .sources
            .iter()
            .map(|s| match s {
                DiagnosticEndpointSource::Draft => "草稿",
                DiagnosticEndpointSource::SavedConfig => "已保存配置",
                DiagnosticEndpointSource::Preset => "预设",
                DiagnosticEndpointSource::Custom => "自定义地址簿",
            })
            .collect::<Vec<_>>()
            .join("、");
        let evidence = state.report(&candidate.id).map(report_text).unwrap_or_else(|| {
            if state
                .snapshot
                .as_ref()
                .is_some_and(|s| matches!(s.state, DiagnosticJobState::Cancelled {}))
            {
                "已取消，未取得测速结果".into()
            } else {
                "等待测速，尚未验证".into()
            }
        });
        body = body.push(settings_panel(
            column![
                text(&candidate.display_url).size(14),
                text(format!("来源：{sources}")),
                text(evidence),
                action(
                    if state.selected.as_ref() == Some(&candidate.id) {
                        "已选中"
                    } else {
                        "选择"
                    },
                    Event::Select(candidate.id.clone()),
                    !state.closing,
                    false
                )
            ]
            .spacing(8),
        ));
    }
    if state.candidates.is_empty() && !state.loading {
        body = body.push(text("没有可测速候选，请在端点地址簿中添加地址后重试。"));
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error));
    }
    let footer = row![
        action("取消编辑", Event::Close, true, false),
        action(
            "使用所选端点",
            Event::Apply,
            state.selected.is_some() && !state.active() && !state.closing,
            true
        )
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn report_text(report: &DiagnosticReport<SpeedEvidence>) -> String {
    let source = match report.source {
        DiagnosticEvidenceSource::RealService => "真实服务",
        DiagnosticEvidenceSource::ControlledFixture => "受控夹具（非真实供应商）",
    };
    let result = match &report.outcome {
        DiagnosticOutcome::Succeeded { evidence } => {
            format!("{}ms · HTTP {}", evidence.latency_ms, evidence.http_status)
        }
        DiagnosticOutcome::Failed { error, http_status } => {
            format!("失败：{:?} / {:?} · HTTP {:?}", error.phase, error.code, http_status)
        }
        DiagnosticOutcome::Cancelled {} => "已取消".into(),
    };
    let timings = report
        .timings
        .iter()
        .map(|t| format!("{:?} {}ms", t.phase, t.elapsed_ms))
        .collect::<Vec<_>>()
        .join("，");
    format!("{source} · {result}\n{timings}")
}
