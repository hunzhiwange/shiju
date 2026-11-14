//! Operation receipts are correlated with the original submission, never the latest group result.
use super::{Identity, Event, message, apply::Event as ApplyEvent};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{GatewayClient, project_profile_api::ProjectProfileApiScope, vw_api_types::project_profiles::*};

/// Required executor envelope. Until this is supplied, fail closed with an unknown outcome.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    operation_id: String,
    profile_id: ProjectProfileId,
    group: ProjectProfileScope,
    result: Option<ProjectProfileApplyResult>,
}
fn owner(a: &Identity, b: &Identity) -> bool {
    a.context == b.context && a.project == b.project && a.host == b.host
}
fn valid(result: &ProjectProfileApplyResult, id: &ProjectProfileId, scope: ProjectProfileScope) -> bool {
    result.profile_id == *id && result.group == scope &&
        (result.status != ProjectProfileApplyStatus::Applied ||
            (result.selected && result.items.iter().all(|i| i.status == ProjectProfileItemStatus::Succeeded)))
}
pub(super) fn admitted(app: &mut App, scope: ProjectProfileScope, response: Identity,
    value: Result<serde_json::Value, ()>, current: Identity) -> Task<Message> {
    let Some(form) = app.apps_catalog.project_profiles.application.as_mut()
        .filter(|f| f.scope == scope && f.submitted && owner(&f.owner, &response) && owner(&response, &current)) else { return Task::none(); };
    form.loading = false;
    if let Ok(value) = value {
        if let Ok(receipt) = serde_json::from_value::<Receipt>(value.clone()) {
            if receipt.profile_id == form.id && receipt.group == scope && !receipt.operation_id.trim().is_empty() {
                form.operation_id = Some(receipt.operation_id.clone());
                return accept(app, scope, receipt);
            }
        }
        if let Ok(result) = serde_json::from_value::<ProjectProfileApplyResult>(value) {
            if valid(&result, &form.id, scope) { return finish(app, scope, result); }
        }
    }
    form.error = Some("应用结果未知，未取得操作 ID；禁止重发。请在执行主机核对。 ");
    app.show_error_toast("应用结果未知；原请求不会重发")
}
pub(super) fn update(app: &mut App, scope: ProjectProfileScope, event: ApplyEvent,
    identity: Identity, client: GatewayClient, query: ProjectProfileApiScope) -> Task<Message> {
    let Some(form) = app.apps_catalog.project_profiles.application.as_mut()
        .filter(|f| f.scope == scope && f.submitted && f.result.is_none() && owner(&f.owner, &identity)) else { return Task::none(); };
    match event {
        ApplyEvent::Recover | ApplyEvent::Cancel => {
            if form.loading { return Task::none(); }
            let Some(operation) = form.operation_id.clone() else { return Task::none(); };
            let cancel = matches!(event, ApplyEvent::Cancel);
            if cancel && form.cancelling { return Task::none(); }
            form.loading = true;
            form.cancelling |= cancel;
            Task::perform(async move {
                let result = if cancel { client.project_profile_cancel(&query, &operation).await }
                    else { client.project_profile_progress(&query, &operation).await };
                message(Event::Apply(scope, ApplyEvent::Progress(identity, operation, result.map_err(|_| ()))))
            }, |m| m)
        }
        ApplyEvent::Progress(response, operation, value) => {
            if !owner(&response, &identity) || form.operation_id.as_ref() != Some(&operation) { return Task::none(); }
            form.loading = false;
            let receipt = value.ok().and_then(|v| serde_json::from_value::<Receipt>(v).ok())
                .filter(|r| r.operation_id == operation && r.profile_id == form.id && r.group == scope);
            if let Some(receipt) = receipt { return accept(app, scope, receipt); }
            form.error = Some("进度或取消未获执行主机确认；可查询原操作，不会重发应用。 ");
            app.show_error_toast("进度或取消未确认，请查询原操作结果")
        }
        _ => Task::none(),
    }
}
fn accept(app: &mut App, scope: ProjectProfileScope, receipt: Receipt) -> Task<Message> {
    let Some(form) = app.apps_catalog.project_profiles.application.as_mut() else { return Task::none(); };
    if let Some(result) = receipt.result {
        if valid(&result, &form.id, scope) { return finish(app, scope, result); }
        form.error = Some("操作结果与目标不符，无法确认；请查询原操作。 ");
        return Task::none();
    }
    form.error = None;
    Task::perform(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        message(Event::Apply(scope, ApplyEvent::Recover))
    }, |m| m)
}
fn finish(app: &mut App, scope: ProjectProfileScope, result: ProjectProfileApplyResult) -> Task<Message> {
    let status = result.status;
    let Some(form) = app.apps_catalog.project_profiles.application.as_mut() else { return Task::none(); };
    form.loading = false;
    form.error = None;
    form.result = Some(result);
    let toast = match status {
        ProjectProfileApplyStatus::Cancelled => app.show_info_toast("执行主机确认已取消；已完成的局部写入请查看逐项结果"),
        ProjectProfileApplyStatus::Partial => app.show_error_toast("组合部分应用失败，请查看逐项结果"),
        ProjectProfileApplyStatus::Failed => app.show_error_toast("组合应用失败"),
        ProjectProfileApplyStatus::Uncaptured => app.show_info_toast("此组未拍快照，仅更新选择"),
        ProjectProfileApplyStatus::Applied => app.show_info_toast("执行主机确认组合已应用"),
    };
    Task::batch([super::update(app, Event::Refresh(scope)), toast])
}
