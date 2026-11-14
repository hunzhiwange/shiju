use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::data_management::recovery_ui::{self, Event, message};
use iced::{Element, widget::{button, column, row, text}};
use vw_gateway_client::DataTargetHealth;

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    button(text("恢复与损坏诊断")).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(crate::app::message::settings::data_management::export_ui::identity(app).is_some().then_some(message(Event::Open))).into()
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if !recovery_ui::visible(app) { return base; }
    let state = &app.apps_catalog.recovery;
    let close = message(Event::Close);
    let header = settings_modal_header(text("恢复与损坏诊断").size(18), settings_close_button(close.clone()));
    let mut body = column![
        text("诊断不修改数据。恢复前由执行主机校验备份并建立回退点；无法安全保存原数据时拒绝恢复。"),
        text("授权主体 ID"),
        settings_text_input("填写当前授权主体 ID", &state.principal)
            .on_input_maybe((!state.busy && !state.submitted).then_some(|v| message(Event::Principal(v)))),
        button(text("诊断 / 刷新备份")).padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((!state.busy && !state.submitted && !state.principal.trim().is_empty()).then_some(message(Event::Diagnose))),
    ].spacing(12);
    if let Some(diagnosis) = &state.diagnosis {
        body = body.push(text(match diagnosis.target_health {
            DataTargetHealth::Healthy => "目标状态：正常",
            DataTargetHealth::Corrupt => "目标状态：损坏；保留原始数据，选择备份进行安全预览",
            DataTargetHealth::VersionUnsupported => "目标版本不受支持：保留数据，请升级兼容版本；禁止降级恢复",
            DataTargetHealth::Unavailable => "目标不可用：可预览备份，恢复仍需执行主机安全校验",
        }));
        if diagnosis.backups.is_empty() { body = body.push(text("当前边界内没有可用备份")); }
        for backup in &diagnosis.backups {
            body = body.push(text(format!("备份：{} · 格式 {} · {} 字节 · {:?}", backup.id, backup.format_version, backup.size_bytes, backup.restore_status)))
                .push(button(text("预览此备份")).padding([8, 14]).style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy && !state.submitted && diagnosis.target_health != DataTargetHealth::VersionUnsupported && backup.restore_status == vw_gateway_client::DataBackupRestoreStatus::RequiresOwnerValidation).then_some(message(Event::Preview(backup.id.clone())))));
        }
    }
    if let Some(preview) = &state.preview {
        body = body.push(text(format!("恢复预览：{}；类别：{:?}", preview.backup.backup_id.0, preview.backup.categories)))
            .push(text("确认后将覆盖所列目标类别。取消不会执行恢复。恢复任务返回后将在此显示回退 ID 和结果警告。"));
    }
    if let Some(progress) = &state.progress {
        body = body.push(text(format!("任务：{}；状态：{:?}", progress.operation_id, progress.status)))
            .push(text(format!("回退 ID：{}", progress.rollback_backup_id.as_ref().map(|id| id.0.as_str()).unwrap_or("尚未返回"))))
            .push(text(format!("结果与警告：{:?}", progress.result)))
            .push(button(text("查询恢复结果")).padding([8, 14]).style(rounded_action_btn_style)
                .on_press_maybe((!state.busy).then_some(message(Event::Refresh))));
    }
    if let Some(error) = &state.error { body = body.push(text(error)); }
    let footer = row![
        button(text(if state.submitted { "关闭" } else { "取消" })).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe((!state.busy).then_some(close.clone())),
        button(text(if state.busy { "处理中…" } else { "确认恢复" })).padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!state.busy && !state.submitted && state.preview.is_some()).then_some(message(Event::Confirm))),
    ].spacing(10);
    settings_modal_overlay(Some(base), close, settings_modal_layout(header, body, Some(footer.into())).width(660).height(640))
}
