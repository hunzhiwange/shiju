use desktop_provider_apps::state::desktop::Form;
// Desktop scoped form orchestration. Native configuration and rules remain SDK-owned.
use iced::Task;
use gateway_aggregation_desktop::{AgentAppsSettingsError, GatewayAggregationDesktopClient};
use gateway_http::GatewayTransport;
use vw_api_app_aggregation::*;
use vw_api_app_aggregation::operations::ProviderOperationProgress;

pub use desktop_provider_apps::aggregation_desktop_types::{Binding, Session, Action, Reply, Event};
fn message(event: Event) -> Event { event }
pub fn error(e: &AgentAppsSettingsError) -> &'static str {
    match e {
        AgentAppsSettingsError::Http { status: 401 | 403, .. } => {
            "无权访问此项目或主机的 Desktop 配置。"
        }
        AgentAppsSettingsError::Http { status: 409, .. } => {
            "配置修订或操作冲突，请刷新实际状态后重新预览。"
        }
        AgentAppsSettingsError::Http { status: 404, .. } => "供应商或操作不存在，请刷新。",
        AgentAppsSettingsError::Http { status: 400 | 422, .. } => {
            "配置不受支持或校验失败，草稿已保留。"
        }
        AgentAppsSettingsError::Http { status: 501, .. } => "执行主机不支持 Desktop。",
        AgentAppsSettingsError::Http { status: 502, .. } => "目标执行失败，请刷新实际状态。",
        AgentAppsSettingsError::Http { status: 503, .. } => "目标主机不可用，请恢复连接后刷新。",
        AgentAppsSettingsError::Http { status: 504, .. } => {
            "目标超时，结果未知；请刷新，勿重复提交。"
        }
        _ => "Desktop 请求结果未知，请刷新实际状态，勿重复提交。",
    }
}
pub async fn execute(
    transport: GatewayTransport,
    form: Form,
    action: Action,
) -> Result<Reply, AgentAppsSettingsError> {
    let session = form
        .aggregation
        .as_ref()
        .ok_or_else(|| AgentAppsSettingsError::Transport("missing Desktop scope".into()))?;
    let query = AggregationQuery {
        target_host_id: session.binding.scope.host.clone(),
        project_id: session.binding.project.clone(),
    };
    let client = GatewayAggregationDesktopClient::new(transport.clone());
    match action {
        Action::Read => {
            let status = client.aggregation_desktop_status(&query).await?;
            let view = client.aggregation_desktop_mapping(&form.id, &query).await?;
            Ok(Reply::Read(status, view))
        }
        Action::Save => client
            .aggregation_desktop_mapping_save(
                &form.id,
                &DesktopMappingChange {
                    target_host_id: query.target_host_id,
                    project_id: query.project_id,
                    expected_revision: session.revision.clone(),
                    mapping: form.mapping,
                },
            )
            .await
            .map(Reply::Saved),
        Action::Preview => client
            .aggregation_desktop_preview(&AggregationPreviewRequest {
                target_host_id: query.target_host_id,
                project_id: query.project_id,
                default_provider_id: Some(form.id),
                catalog_policy: CatalogPolicy::Preserve,
            })
            .await
            .map(Reply::Preview),
        Action::Publish => {
            let p =
                session.preview.as_ref().filter(|_| session.can_publish()).ok_or_else(|| {
                    AgentAppsSettingsError::Transport("missing valid preview".into())
                })?;
            client
                .aggregation_desktop_publish(&AggregationPublishRequest {
                    target_host_id: query.target_host_id,
                    project_id: query.project_id,
                    expected_revision: p.revision.clone(),
                    preview_token: p.preview_token.clone().unwrap_or_default(),
                    request_id: format!("desktop-{:032x}", rand::random::<u128>()),
                    confirmed: true,
                })
                .await
                .map(Reply::Operation)
        }
        Action::Restore => client
            .aggregation_desktop_exit(&AggregationExitRequest {
                target_host_id: query.target_host_id,
                project_id: query.project_id,
                expected_revision: session
                    .status
                    .as_ref()
                    .map(|s| s.revision.clone())
                    .unwrap_or_default(),
                request_id: format!("desktop-{:032x}", rand::random::<u128>()),
                confirmed: true,
            })
            .await
            .map(Reply::Operation),
        Action::Poll | Action::Cancel => {
            let id = session
                .operation
                .as_ref()
                .ok_or_else(|| AgentAppsSettingsError::Transport("missing operation".into()))?
                .operation_id
                .0
                .as_str();
            if matches!(action, Action::Poll) {
                gateway_app_provider::GatewayAppProviderClient::new(transport.clone()).provider_aggregation_operation_get(id, &query).await
            } else {
                gateway_app_provider::GatewayAppProviderClient::new(transport.clone()).provider_aggregation_operation_cancel(id, &query).await
            }
            .map(Reply::Operation)
            .map_err(|e| match e {
                gateway_app_provider::AgentAppsReadError::Http { status, body } => {
                    AgentAppsSettingsError::Http { status, body }
                }
                gateway_app_provider::AgentAppsReadError::Transport(message) => {
                    AgentAppsSettingsError::Transport(message)
                }
                gateway_app_provider::AgentAppsReadError::Decode(message) => {
                    AgentAppsSettingsError::Decode(message)
                }
            })
        }
    }
}
pub fn finish(form: &mut Form, result: Result<Reply, AgentAppsSettingsError>) {
    form.busy = false;
    let Some(s) = form.aggregation.as_mut() else { return };
    s.confirmation = None;
    match result {
        Ok(Reply::Read(status, view)) => {
            let binding = s.binding.clone();
            let revision = view.revision;
            form.load(view.form);
            form.aggregation =
                Some(Session { status: Some(status), revision, ..Session::new(binding) });
            form.notice = None;
        }
        Ok(Reply::Saved(view)) => {
            s.revision = view.revision;
            s.dirty = false;
            s.invalidate();
            s.status = None;
            form.mapping = view.form.mapping;
            form.mapping_editable = view.form.mapping_editable;
            form.notice = Some("映射已保存；请刷新状态后重新预览。".into());
        }
        Ok(Reply::Preview(p)) => {
            let blocked = p.issues.iter().any(|i| i.blocking);
            s.preview = Some(p);
            form.notice = Some(
                if blocked {
                    "预览被阻止，请检查下方问题。"
                } else {
                    "预览已返回，请核对后确认应用。"
                }
                .into(),
            );
        }
        Ok(Reply::Operation(o)) => {
            form.notice = Some(match &o.aggregation_outcome {
                Some(AggregationOutcome::Failed { status, code, recovery }) => {
                    format!("执行失败：{status} / {code:?}；恢复状态：{recovery:?}。请刷新后处理。")
                }
                Some(AggregationOutcome::Published { .. })
                    if o.status == vw_api_task::TaskStatus::Completed =>
                {
                    "聚合已发布，请完全退出并重新启动 Desktop。".into()
                }
                Some(AggregationOutcome::Exited { .. })
                    if o.status == vw_api_task::TaskStatus::Completed =>
                {
                    "直连配置已恢复，请完全退出并重新启动 Desktop。".into()
                }
                _ if matches!(
                    o.status,
                    vw_api_task::TaskStatus::Queued
                        | vw_api_task::TaskStatus::Running
                ) =>
                {
                    "操作进行中，请查询进度；取消请求不代表已经取消。".into()
                }
                _ => "操作已结束，请刷新核对实际状态。".into(),
            });
            s.invalidate();
            s.status = None;
            s.revision.clear();
            s.operation = Some(o);
        }
        Err(e) => {
            s.invalidate();
            s.status = None;
            s.revision.clear();
            form.notice = Some(error(&e).into());
        }
    }
}
pub fn update(
    desktop: &mut Option<Form>,
    counter: &mut u64,
    loading: bool,
    current: Result<(GatewayTransport, Binding), String>,
    notice: &mut Option<super::Notice>,
    event: Event,
) -> Task<Event> {
    let (client, binding) = match current.clone() {
        Ok(v) => v,
        Err(_) if matches!(event, Event::Finished { .. }) => return Task::none(),
        Err(e) => return super::error(notice, e),
    };
    if let Event::Open(id) = event {
        if loading || (*desktop).is_some() {
            return Task::none();
        }
        let mut form = Form::new(id);
        form.aggregation = Some(Session::new(binding.clone()));
        (*desktop) = Some(form);
        return update(desktop, counter, loading, current, notice, Event::Refresh);
    }
    let Some(form) = (*desktop).as_mut() else { return Task::none() };
    if form.aggregation.as_ref().is_none_or(|s| s.binding != binding) {
        (*desktop) = None;
        return Task::none();
    }
    if let Event::Finished { binding: old, generation, result } = event {
        if old != binding || generation != *counter || !form.busy {
            return Task::none();
        }
        let failed = matches!(
            &result,
            Err(_)
                | Ok(Reply::Operation(ProviderOperationProgress {
                    aggregation_outcome: Some(AggregationOutcome::Failed { .. }),
                    ..
                }))
        );
        let blocked =
            matches!(&result, Ok(Reply::Preview(p)) if p.issues.iter().any(|i| i.blocking));
        finish(form, result);
        let notice = form.notice.clone();
        return match notice {
            Some(n) if failed => super::error(notice, n),
            Some(n) if blocked => super::warning(notice, n),
            Some(n) => super::info(notice, n),
            None => Task::none(),
        };
    }
    if form.busy {
        return Task::none();
    }
    let s = form.aggregation.as_mut().unwrap();
    let action = match event {
        Event::Dismiss => {
            s.confirmation = None;
            return Task::none();
        }
        Event::Review(restore)
            if !s.running() && if restore { s.status.is_some() } else { s.can_publish() } =>
        {
            s.confirmation = Some(restore);
            return Task::none();
        }
        Event::Confirm if s.confirmation == Some(true) && s.status.is_some() && !s.running() => {
            Action::Restore
        }
        Event::Confirm if s.confirmation == Some(false) && s.can_publish() && !s.running() => {
            Action::Publish
        }
        Event::Refresh if s.operation.is_some() => Action::Poll,
        Event::Refresh => {
            s.invalidate();
            s.status = None;
            s.revision.clear();
            Action::Read
        }
        Event::Save if form.mapping_editable && !s.revision.is_empty() && !s.running() => {
            Action::Save
        }
        Event::Preview if !s.dirty && !s.revision.is_empty() && !s.running() => {
            s.invalidate();
            Action::Preview
        }
        Event::Poll if s.operation.is_some() => Action::Poll,
        Event::Cancel if s.running() => Action::Cancel,
        _ => return Task::none(),
    };
    // A terminal operation can be explicitly discarded only after its result was shown.
    if matches!(action, Action::Poll)
        && s.operation.as_ref().is_some_and(|o| {
            !matches!(
                o.status,
                vw_api_task::TaskStatus::Queued
                    | vw_api_task::TaskStatus::Running
            )
        })
    {
        s.operation = None;
        return update(desktop, counter, loading, current, notice, Event::Refresh);
    }
    form.busy = true;
    *counter = counter.wrapping_add(1);
    let generation = *counter;
    Task::perform(execute(client, form.clone(), action), move |result| {
        message(Event::Finished { binding: binding.clone(), generation, result })
    })
}


#[cfg(test)]
#[path = "desktop_tests.rs"]
mod tests;
