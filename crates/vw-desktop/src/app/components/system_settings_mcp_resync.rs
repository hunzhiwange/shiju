use super::toolbar_button;
use crate::app::{
    App, Message,
    message::settings::{
        mcp_management_bindings as bindings, mcp_management_import as import,
        mcp_management_resync::*,
    },
};
use iced::{
    Element,
    widget::{checkbox, column, row, text},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::{AppId, AppOperationPhase},
    mcp_management::{McpBatchOutcome, McpNativeStatus},
    task::TaskStatus,
};
pub(crate) fn panel(app: &App) -> Element<'_, Message> {
    use crate::app::components::system_settings_common::*;
    let state = visible(app);
    let mut body = column![text("重新同步进度").size(16), text("同步所选应用的全部受管绑定；未选择目录项时使用租户目录，选择后使用其共同作用域。取消保留已完成事务。Claude Desktop、OpenClaw 不支持。").size(12)].spacing(8);
    let locked = state.is_some_and(locked);
    let retry = state.is_some_and(|s| {
        !locked && s.status.as_ref().is_some_and(|r| terminal(r) && r.targets.iter().any(failed))
    });
    body = body.push(
        row![
            toolbar_button("重新同步", (!locked).then(|| message(ResyncMessage::Open))),
            toolbar_button("仅重试失败目标", retry.then(|| message(ResyncMessage::RetryFailed))),
            toolbar_button(
                "刷新进度",
                state
                    .filter(|s| s.operation.is_some() && !s.busy)
                    .map(|_| message(ResyncMessage::Refresh))
            ),
            toolbar_button(
                "取消同步",
                state
                    .filter(|s| s.operation.is_some()
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
            body = body.push(text("正在请求执行端，等待作业进度…"));
        }
        if let Some(error) = &state.error {
            body = body.push(settings_error_banner(error.clone()));
        }
        for (target, status) in &state.observed {
            let label = match status {
                Some(McpNativeStatus::Present) => "存在",
                Some(McpNativeStatus::Missing) => "缺失",
                Some(McpNativeStatus::Unreadable) => "无法读取",
                Some(McpNativeStatus::NotVerified) => "未验证",
                Some(McpNativeStatus::Unsupported) => "不支持",
                None => "读取失败，请刷新核实",
            };
            body = body.push(text(format!("{:?} · 刷新原生状态：{label}", target.app_id)));
        }
        if let Some(status) = &state.status {
            body = body.push(text(format!(
                "阶段 {} · 状态 {} · {}/{} 个目标已有结果 · 总体 {}",
                match status.operation.phase {
                    AppOperationPhase::Queued => "排队",
                    AppOperationPhase::Preflight => "预检",
                    AppOperationPhase::Executing => "执行",
                    AppOperationPhase::Postflight => "核实",
                    AppOperationPhase::Terminal => "结束",
                },
                match status.operation.status {
                    TaskStatus::Queued => "排队",
                    TaskStatus::Running => "运行中",
                    TaskStatus::Completed => "已完成",
                    TaskStatus::Failed => "存在失败",
                    TaskStatus::Cancelled => "已取消",
                },
                status.targets.len(),
                state.request.as_ref().map_or(0, |r| r.targets.len()),
                if status.outcome == Some(McpBatchOutcome::Succeeded) && !successful(status) {
                    "未全部验证"
                } else {
                    status.outcome.map(batch_label).unwrap_or("等待结果")
                }
            )));
            for target in &status.targets {
                let label =
                    target.code.map(|code| format!(" · 失败：{code:?}")).unwrap_or_default();
                body = body.push(text(format!("{:?}{label}", target.app_id)));
                if let Some(d) = &target.distribution {
                    body = body.push(text(format!(
                        "目标结果：{}",
                        if d.items.is_empty() {
                            batch_label(d.outcome)
                        } else {
                            bindings::summary(d)
                        }
                    )));
                    for item in &d.items {
                        body = body.push(text(bindings::item_label(item)));
                    }
                }
            }
        }
    }
    settings_panel(body).into()
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
pub(crate) fn overlay<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    use iced::{Length, widget::button};
    let Some(state) = visible(app).filter(|s| s.open) else {
        return dialog;
    };
    let locked = locked(state);
    let mut body = column![text("选择可信 macOS 应用实例；每种应用最多一个实例。同步的是当前作用域的全部受管绑定，不只包含勾选的目录项。运行时和 ACP 选择保持独立。")].spacing(10).width(Length::Fill);
    let scope = if app.mcp_settings.bindings.selected.is_empty() {
        "当前租户".to_owned()
    } else {
        match crate::app::message::settings::mcp_management_bindings::project(
            &app.mcp_settings.bindings.selected,
        ) {
            Ok(Some(project)) => format!("项目 {}", project.0),
            Ok(None) => "当前租户".to_owned(),
            Err(error) => format!("无效：{error}"),
        }
    };
    body = body.push(text(format!("同步作用域：{scope}")));
    for target in import::targets(app) {
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
        body = body.push(text("受管 Claude 官方账号 ID（仅引用，由执行端解析）").size(12));
        let account = state.account.as_str();
        let input = settings_text_input("官方账号 ID（非密钥）", account);
        body = body.push(if locked {
            input
        } else {
            input.on_input(|v| message(ResyncMessage::Account(v)))
        });
    }

    let header = row![
        text("重新同步应用").size(18),
        iced::widget::Space::new().width(Length::Fill),
        settings_close_button(message(ResyncMessage::Close))
    ]
    .align_y(iced::Alignment::Center);
    let footer = row![
        toolbar_button("取消", (!locked).then(|| message(ResyncMessage::Close))),
        button(text("确认重新同步"))
            .padding([7, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (!locked && !state.selected.is_empty()).then(|| message(ResyncMessage::Start))
            )
    ]
    .spacing(8);
    settings_modal_overlay(
        Some(dialog),
        message(ResyncMessage::Close),
        settings_modal_layout(header, body, Some(footer.into())).width(680).height(540),
    )
}
