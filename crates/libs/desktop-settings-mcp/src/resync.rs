//! Resynchronization progress and confirmation presentation.
use crate::toolbar_button;
use gateway_mcp_distribution::McpResyncRequest;
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, widget::{checkbox, column, row, text}};
use vw_api_app_identity::AppId;
use vw_api_app_operations::AppOperationPhase;
use vw_api_task::TaskStatus;
use vw_api_mcp_management::*;
use crate::distribution::{terminal, failed, successful};
#[derive(Debug, Clone)]
pub enum ResyncMessage { Open, RetryFailed, Refresh, Cancel, Close, Select(McpAppTarget, bool), Account(String), Start }
type Message = ResyncMessage;
fn message(value: ResyncMessage) -> Message { value }
#[derive(Clone, Copy)]
pub struct ResyncView<'a> {
    pub locked: bool, pub has_operation: bool, pub busy: bool, pub cancelling: bool,
    pub status: &'a Option<McpResyncOperationStatus>, pub request: &'a Option<McpResyncRequest>,
    pub observed: &'a [(McpAppTarget, Option<McpNativeStatus>)], pub error: &'a Option<String>,
    pub selected: &'a [McpAppTarget], pub account: &'a String,
}
pub fn panel(i18n: desktop_i18n::I18n, state: Option<ResyncView<'_>>) -> Element<'_, Message> {
    let mut body = column![text(i18n.__("应用配置重写进度")).size(16), text(i18n.__("同步所选应用的全部受管绑定；未选择目录项时使用租户目录，选择后使用其共同作用域。取消保留已完成事务。Claude Desktop、OpenClaw 不支持。")).size(12)].spacing(8);
    let locked = state.is_some_and(|s| s.locked);
    let retry = state.is_some_and(|s| {
        !locked && s.status.as_ref().is_some_and(|r| terminal(r) && r.targets.iter().any(failed))
    });
    body = body.push(
        row![
            toolbar_button(
                i18n.__("按当前开关重写应用配置"),
                (!locked).then(|| message(ResyncMessage::Open))
            ),
            toolbar_button(
                i18n.__("仅重试失败目标"),
                retry.then(|| message(ResyncMessage::RetryFailed))
            ),
            toolbar_button(
                i18n.__("刷新进度"),
                state
                    .filter(|s| s.has_operation && !s.busy)
                    .map(|_| message(ResyncMessage::Refresh))
            ),
            toolbar_button(
                i18n.__("取消同步"),
                state
                    .filter(|s| s.has_operation
                        && !s.cancelling
                        && !s.status.as_ref().is_some_and(terminal))
                    .map(|_| message(ResyncMessage::Cancel))
            )
        ]
        .spacing(8)
        .wrap(),
    );
    if let Some(state) = state {
        if state.busy {
            body = body.push(text(i18n.__("正在请求执行端，等待作业进度…")));
        }
        if let Some(error) = &state.error {
            body = body.push(settings_error_banner(desktop_i18n::settings_extensions::feedback(
                i18n,
                &error.clone(),
            )));
        }
        for (target, status) in state.observed {
            let label = match status {
                Some(McpNativeStatus::Present) => i18n.__("存在"),
                Some(McpNativeStatus::Missing) => i18n.__("缺失"),
                Some(McpNativeStatus::Unreadable) => i18n.__("无法读取"),
                Some(McpNativeStatus::NotVerified) => i18n.__("未验证"),
                Some(McpNativeStatus::Unsupported) => i18n.__("不支持"),
                None => i18n.__("读取失败，请刷新核实"),
            };
            body = body.push(text(
                desktop_i18n::__!(
                    i18n,
                    "%s · 刷新原生状态：%s",
                    (format!("{:?}", target.app_id)).to_string(),
                    (label).to_string()
                )
                .expect("valid extension display template"),
            ));
        }
        if let Some(status) = &state.status {
            body = body.push(text(
                desktop_i18n::__!(
                    i18n,
                    "阶段 %s · 状态 %s · %s/%s 个目标已有结果 · 总体 %s",
                    (match status.operation.phase {
                        AppOperationPhase::Queued => i18n.__("排队"),
                        AppOperationPhase::Preflight => i18n.__("预检"),
                        AppOperationPhase::Executing => i18n.__("执行"),
                        AppOperationPhase::Postflight => i18n.__("核实"),
                        AppOperationPhase::Terminal => i18n.__("结束"),
                    })
                    .to_string(),
                    (match status.operation.status {
                        TaskStatus::Queued => i18n.__("排队"),
                        TaskStatus::Running => i18n.__("运行中"),
                        TaskStatus::Completed => i18n.__("已完成"),
                        TaskStatus::Failed => i18n.__("存在失败"),
                        TaskStatus::Cancelled => i18n.__("已取消"),
                    })
                    .to_string(),
                    (status.targets.len()).to_string(),
                    (state.request.as_ref().map_or(0, |r| r.targets.len())).to_string(),
                    (if status.outcome == Some(McpBatchOutcome::Succeeded) && !successful(status) {
                        i18n.__("未全部验证")
                    } else {
                        status
                            .outcome
                            .map(|outcome| i18n.__(batch_label(outcome)))
                            .unwrap_or(i18n.__("等待结果"))
                    })
                    .to_string()
                )
                .expect("valid extension display template"),
            ));
            for target in &status.targets {
                let label = target
                    .code
                    .map(|code| {
                        desktop_i18n::__!(i18n, " · 失败：%s", (format!("{:?}", code)).to_string())
                            .expect("valid extension display template")
                    })
                    .unwrap_or_default();
                body = body.push(text(format!("{:?}{label}", target.app_id)));
                if let Some(d) = &target.distribution {
                    body = body.push(text(
                        desktop_i18n::__!(
                            i18n,
                            "目标结果：%s",
                            (if d.items.is_empty() {
                                i18n.__(batch_label(d.outcome))
                            } else {
                                i18n.__(crate::distribution::summary(d))
                            })
                            .to_string()
                        )
                        .expect("valid extension display template"),
                    ));
                    for item in &d.items {
                        body = body.push(text(crate::distribution::item_label(i18n, item)));
                    }
                }
            }
        }
    }
    settings_section_content(body).into()
}

fn batch_label(outcome: McpBatchOutcome) -> &'static str {
    match outcome {
        McpBatchOutcome::Succeeded => "全部成功",
        McpBatchOutcome::PartiallySucceeded => "部分成功",
        McpBatchOutcome::Failed => "失败",
        McpBatchOutcome::Cancelled => "已取消",
    }
}

/// Action options use the common modal with fixed controls and a 4px content scrollbar.
pub fn card<'a>(
    i18n: desktop_i18n::I18n,
    state: ResyncView<'a>,
    scope: String,
    targets: Vec<McpAppTarget>,
) -> Element<'a, Message> {
    use iced::{Length, widget::button};
    let locked = state.locked;
    let mut body = column![text(i18n.__("选择可信 macOS 应用实例；每种应用最多一个实例。按已保存开关重写当前作用域的全部受管绑定，隐藏应用列不会改变已保存期望；保留无关条目。运行时和 ACP 选择保持独立。"))].spacing(10).width(Length::Fill);
    body = body.push(text(
        desktop_i18n::__!(i18n, "同步作用域：%s", (scope).to_string())
            .expect("valid extension display template"),
    ));
    for target in targets {
        let selected = state.selected.contains(&target);
        let label = format!("{:?} · {}", target.app_id, target.installation_id.0);
        body = body.push(
            checkbox(selected).label(label).style(settings_checkbox_style).on_toggle_maybe(
                (!locked).then_some(move |checked| {
                    message(ResyncMessage::Select(target.clone(), checked))
                }),
            ),
        );
    }
    if state.selected.iter().any(|t| t.app_id == AppId::Claude) {
        body = body.push(text(i18n.__("受管 Claude 官方账号 ID（仅引用，由执行端解析）")).size(12));
        let account = state.account.as_str();
        let input = settings_text_input(i18n.__("官方账号 ID（非密钥）"), account);
        body = body.push(if locked {
            input
        } else {
            input.on_input(|v| message(ResyncMessage::Account(v)))
        });
    }

    let header = row![
        text(i18n.__("重写应用配置")).size(18),
        iced::widget::Space::new().width(Length::Fill),
        settings_close_button(message(ResyncMessage::Close))
    ]
    .align_y(iced::Alignment::Center);
    let footer = row![
        toolbar_button(i18n.__("取消"), (!locked).then(|| message(ResyncMessage::Close))),
        button(text(i18n.__("确认重写应用配置")))
            .padding([7, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (!locked && !state.selected.is_empty()).then(|| message(ResyncMessage::Start))
            )
    ]
    .spacing(8);
    settings_modal_layout(header, body, Some(footer.into())).width(680).height(540).into()
}
