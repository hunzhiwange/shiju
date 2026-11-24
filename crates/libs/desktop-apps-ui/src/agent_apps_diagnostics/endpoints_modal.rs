use desktop_ui_controls::system_settings_common::*;
type Message = Event;
#[derive(Clone, Debug)]
pub enum Event {
    Close,
    Start,
    Cancel,
    Preset(String),
    Reload,
    ManageCustom,
    Select(DiagnosticEndpointId),
    Apply,
}
pub struct State<'a> {
    pub loading: bool,
    pub active: bool,
    pub cancelling: bool,
    pub closing: bool,
    pub candidates: Vec<&'a DiagnosticEndpointCandidate>,
    pub reports: Vec<(&'a DiagnosticEndpointId, &'a DiagnosticReport<SpeedEvidence>)>,
    pub preset_name: &'a str,
    pub snapshot: &'a Option<DiagnosticJobSnapshot>,
    pub selected: &'a Option<DiagnosticEndpointId>,
    pub error: &'a Option<String>,
}
fn message(event: Event) -> Event {
    event
}
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_api_app_diagnostics::*;
fn action(i18n: desktop_i18n::I18n,
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    let b = button(text(i18n.__(label))).padding([8, 14]);
    let b =
        if primary { b.style(primary_action_btn_style) } else { b.style(rounded_action_btn_style) };
    b.on_press_maybe(enabled.then(|| message(event))).into()
}
pub fn overlay<'a>(i18n: desktop_i18n::I18n, state: State<'a>, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("端点测速与选择")).size(20),
        Some(settings_close_button(close.clone())),
    );
    let mut body = column![text(i18n.__("测速为热身后 GET 响应头耗时，不代表模型速度或鉴权成功。")),
        text(i18n.__("合并当前草稿、已保存配置、预设和自定义地址簿。选择仅回填草稿，供应商表单保存后才写入端点。自定义地址通过「端点与请求选项」维护。")),
        row![action(i18n, i18n.__("开始测速"), Event::Start, !state.loading && !state.active && !state.candidates.is_empty(), true),
            action(i18n, i18n.__("取消测速"), Event::Cancel, state.active && (!state.cancelling || state.error.is_some()), false)].spacing(8)].spacing(12);
    body = body
        .push(settings_field_label(i18n.__("预设名称（可选）")))
        .push(
            settings_text_input(i18n.__("例如 PackyCode；留空不合并预设"), &state.preset_name)
                .on_input_maybe(
                    (!state.active && !state.loading).then_some(|v| message(Event::Preset(v))),
                ),
        )
        .push(action(i18n, i18n.__("刷新候选"), Event::Reload, !state.active && !state.loading, false));
    body = body.push(action(i18n,
        i18n.__("管理自定义端点"),
        Event::ManageCustom,
        !state.active && !state.loading,
        false,
    ));
    if state.loading {
        body = body.push(text(i18n.__("正在读取候选端点…")));
    }
    if let Some(snapshot) = state.snapshot {
        body = body.push(text(i18n.sprintf("进度：%u / %s · %s", &[(snapshot.progress.completed).into(), format!("{}", snapshot.progress.total.map(|n| n.to_string()).unwrap_or_else(|| i18n.__("未知").into())).into(), format!("{:?}", snapshot.state).into()])));
    }
    if state.cancelling {
        body = body.push(text(i18n.__("正在取消测速，等待清理完成…")));
    }
    for candidate in state.candidates.iter().copied() {
        let sources = candidate
            .sources
            .iter()
            .map(|s| match s {
                DiagnosticEndpointSource::Draft => i18n.__("草稿"),
                DiagnosticEndpointSource::SavedConfig => i18n.__("已保存配置"),
                DiagnosticEndpointSource::Preset => i18n.__("预设"),
                DiagnosticEndpointSource::Custom => i18n.__("自定义地址簿"),
            })
            .collect::<Vec<_>>()
            .join("、");
        let evidence = state
            .reports
            .iter()
            .find(|(id, _)| **id == candidate.id)
            .map(|(_, report)| *report)
            .map(|report| report_text(i18n, report))
            .unwrap_or_else(|| {
                if state
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| matches!(s.state, DiagnosticJobState::Cancelled {}))
                {
                    i18n.__("已取消，未取得测速结果").into()
                } else {
                    i18n.__("等待测速，尚未验证").into()
                }
            });
        body = body.push(settings_panel(
            column![
                text(&candidate.display_url).size(14),
                text(i18n.sprintf("来源：%s", &[format!("{}", sources).into()])),
                text(evidence),
                action(i18n,
                    if state.selected.as_ref() == Some(&candidate.id) {
                        i18n.__("已选中")
                    } else {
                        i18n.__("选择")
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
        body = body.push(text(i18n.__("没有可测速候选，请在端点地址簿中添加地址后重试。")));
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    let footer = row![
        action(i18n, i18n.__("取消编辑"), Event::Close, true, false),
        action(i18n,
            i18n.__("使用所选端点"),
            Event::Apply,
            state.selected.is_some() && !state.active && !state.closing,
            true
        )
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub fn report_text(i18n: desktop_i18n::I18n, report: &DiagnosticReport<SpeedEvidence>) -> String {
    let source = match report.source {
        DiagnosticEvidenceSource::RealService => i18n.__("真实服务"),
        DiagnosticEvidenceSource::ControlledFixture => i18n.__("受控夹具（非真实供应商）"),
    };
    let result = match &report.outcome {
        DiagnosticOutcome::Succeeded { evidence } => {
            format!("{}ms · HTTP {}", evidence.latency_ms, evidence.http_status)
        }
        DiagnosticOutcome::Failed { error, http_status } => {
            i18n.sprintf("失败：%s / %s · HTTP %s", &[format!("{:?}", error.phase).into(), format!("{:?}", error.code).into(), format!("{:?}", http_status).into()])
        }
        DiagnosticOutcome::Cancelled {} => i18n.__("已取消").into(),
    };
    let timings = report
        .timings
        .iter()
        .map(|t| format!("{:?} {}ms", t.phase, t.elapsed_ms))
        .collect::<Vec<_>>()
        .join("，");
    format!("{source} · {result}\n{timings}")
}
