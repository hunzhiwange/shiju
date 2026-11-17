// Confirmation and progress for server-owned aggregation publication.
use super::Selection;
use iced::Task;
use gateway_aggregation::{AggregationApiError, GatewayAggregationClient};
use vw_gateway_client::GatewayClient;
use vw_api_app_aggregation::*;
use vw_api_app_aggregation::operations::ProviderOperationProgress;
use vw_api_task::TaskStatus;

#[derive(Clone, Default)]
pub struct Session {
    pub preview: Option<AggregationPreview>,
    pub exit_state: Option<AggregationState>,
    pub operation: Option<ProviderOperationProgress>,
    pub error: Option<&'static str>,
    pub busy: bool,
}
impl Session {
    pub fn running(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|p| matches!(p.status, TaskStatus::Queued | TaskStatus::Running))
    }
    pub fn can_confirm(&self) -> bool {
        !self.busy
            && self.operation.is_none()
            && if let Some(state) = &self.exit_state {
                self.error.is_none()
                    && !state.revision.is_empty()
                    && state.mode != AggregationMode::Direct
            } else {
                self.preview.as_ref().is_some_and(|p| {
                    !p.revision.is_empty()
                        && p.preview_token.as_ref().is_some_and(|t| !t.is_empty())
                        && p.default_provider_id.is_some()
                        && !p.issues.iter().any(|i| i.blocking)
                })
            }
    }
    pub fn begin(&mut self, action: Action) -> bool {
        if self.busy {
            return false;
        }
        let allowed = match action {
            Action::Preview => {
                self.exit_state.is_none() && self.preview.is_none() && self.operation.is_none()
            }
            Action::Confirm => self.can_confirm(),
            Action::Poll => self.operation.is_some(),
            Action::Cancel => {
                self.running() && self.operation.as_ref().is_some_and(|p| !p.cancellation_requested)
            }
        };
        if allowed {
            self.busy = true;
            self.error = None;
        }
        allowed
    }
    pub fn finish(&mut self, result: Result<Reply, AggregationApiError>) {
        self.busy = false;
        match result {
            Ok(Reply::Preview(p)) => self.preview = Some(p),
            Ok(Reply::Operation(p)) => {
                self.preview = None;
                self.error = match &p.aggregation_outcome {
                    Some(AggregationOutcome::Exited { state })
                        if state.mode != AggregationMode::Direct
                            || state.attached
                            || matches!(
                                state.recovery,
                                RecoveryState::Pending | RecoveryState::Failed
                            ) =>
                    {
                        Some("直连恢复尚未成功，请刷新实际状态并检查恢复情况")
                    }

                    Some(AggregationOutcome::Failed { .. }) => {
                        Some("操作失败，请刷新实际状态并检查恢复情况")
                    }
                    _ if p.status == TaskStatus::Failed => Some("操作失败，请刷新实际状态"),
                    _ => None,
                };
                if self.exit_state.is_some()
                    && p.status == TaskStatus::Completed
                    && !matches!(&p.aggregation_outcome, Some(AggregationOutcome::Exited { .. }))
                {
                    self.error = Some("恢复结果不完整，请刷新核对实际状态");
                }
                self.operation = Some(p);
            }
            Err(e) => {
                self.preview = None;
                self.error = Some(if self.exit_state.is_some() {
                    exit_error_label(&e)
                } else {
                    error_label(&e)
                });
            }
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Preview,
    Confirm,
    Poll,
    Cancel,
}
#[derive(Clone)]
pub enum Reply {
    Preview(AggregationPreview),
    Operation(ProviderOperationProgress),
}
#[derive(Clone)]
pub enum Event {
    Open,
    OpenExit,
    RetryExit,
    Dismiss,
    Act(Action),
    Finished { selection: Selection, generation: u64, result: Result<Reply, AggregationApiError> },
}
fn message(event: Event) -> super::list::Event { super::list::Event::Publish(event) }
fn exit_error_label(e: &AggregationApiError) -> &'static str {
    match e {
        AggregationApiError::Http { status: 401 | 403, .. } => "无权恢复直连，请检查项目和主机授权",
        AggregationApiError::Http { status: 409, .. } => "恢复版本冲突，请刷新实际状态后重新确认",
        AggregationApiError::Http { status: 422 | 501, .. } => "此应用或执行目标不支持直连恢复",
        AggregationApiError::Http { status: 502, .. } => {
            "目标恢复失败，请刷新实际状态并检查恢复情况"
        }
        AggregationApiError::Http { status: 503, .. } => "执行主机不可用，请恢复连接后刷新实际状态",
        _ => "恢复结果未知，请刷新实际状态，勿重复提交",
    }
}
fn error_label(e: &AggregationApiError) -> &'static str {
    match e {
        AggregationApiError::Http { status: 401 | 403, .. } => "无权发布聚合，请检查项目和主机授权",
        AggregationApiError::Http { status: 409, .. } => {
            "预览版本或操作冲突，请关闭并刷新目录后重新预览"
        }
        AggregationApiError::Http { status: 422 | 501, .. } => {
            "此配置、应用或执行目标不支持聚合发布"
        }
        AggregationApiError::Http { status: 502, .. } => {
            "目标发布失败，请刷新实际状态并检查恢复情况"
        }
        AggregationApiError::Http { status: 503, .. } => "执行主机不可用，请恢复连接后刷新实际状态",
        AggregationApiError::Http { status: 504, .. } => {
            "目标超时，结果未知，请刷新实际状态，勿重复提交"
        }
        _ => "发布请求结果未知，请刷新实际状态，勿重复提交",
    }
}
pub async fn execute(
    client: GatewayClient,
    selection: Selection,
    token: String,
    session: Session,
    action: Action,
    default: Option<String>,
) -> Result<Reply, AggregationApiError> {
    let endpoint = client.endpoint().clone();
    let transport = gateway_http::GatewayTransport::new(endpoint).map_err(AggregationApiError::Transport)?;
    let client = GatewayAggregationClient::new(transport.clone());
    let query =
        AggregationQuery { target_host_id: selection.scope.host, project_id: selection.project };
    match action {
        Action::Preview => client
            .aggregation_preview(
                selection.scope.app,
                &AggregationPreviewRequest {
                    target_host_id: query.target_host_id,
                    project_id: query.project_id,
                    default_provider_id: default,
                    catalog_policy: CatalogPolicy::Preserve,
                },
            )
            .await
            .map(Reply::Preview),
        Action::Confirm => {
            // The UI marks the in-flight session busy, but the captured intent must still
            // satisfy every confirmation guard before reaching the SDK.
            if !session.can_confirm() {
                return Err(AggregationApiError::Transport("valid confirmation required".into()));
            }
            if let Some(state) = &session.exit_state {
                return client
                    .aggregation_exit(
                        selection.scope.app,
                        &AggregationExitRequest {
                            target_host_id: query.target_host_id,
                            project_id: query.project_id,
                            expected_revision: state.revision.clone(),
                            request_id: format!("aggregation-exit-{:032x}", rand::random::<u128>()),
                            confirmed: true,
                        },
                    )
                    .await
                    .map(Reply::Operation);
            }
            let p = session.preview.as_ref().unwrap();
            client
                .aggregation_publish(
                    selection.scope.app,
                    &AggregationPublishRequest {
                        target_host_id: query.target_host_id,
                        project_id: query.project_id,
                        expected_revision: p.revision.clone(),
                        preview_token: p.preview_token.clone().unwrap(),
                        request_id: format!("aggregation-{:032x}", rand::random::<u128>()),
                        confirmed: true,
                    },
                )
                .await
                .map(Reply::Operation)
        }
        Action::Poll | Action::Cancel => {
            let operation = session
                .operation
                .as_ref()
                .ok_or_else(|| AggregationApiError::Transport("missing operation".into()))?;
            let result = if matches!(action, Action::Poll) {
                gateway_app_provider::GatewayAppProviderClient::new(transport.clone()).provider_aggregation_operation_get(&operation.operation_id.0, &query).await
            } else {
                gateway_app_provider::GatewayAppProviderClient::new(transport.clone()).provider_aggregation_operation_cancel(&operation.operation_id.0, &query)
                    .await
            };
            result.map(Reply::Operation).map_err(|e| match e {
                gateway_http::AgentAppsReadError::Http { status, body } => {
                    AggregationApiError::Http { status, body }
                }
                gateway_http::AgentAppsReadError::Decode(s) => AggregationApiError::Decode(s),
                gateway_http::AgentAppsReadError::Transport(s) => {
                    AggregationApiError::Transport(s)
                }
            })
        }
    }
}
pub fn update(
    aggregation: &mut Option<super::list::Form>,
    counter: &mut u64,
    loading: bool,
    token: &str,
    current: Option<(GatewayClient, Selection)>,
    notice: &mut Option<super::Notice>,
    event: Event,
) -> Task<super::list::Event> {
    let Some((client, selection)) = current else {
        if let Some(f) = &mut (*aggregation) {
            f.publish = None;
            f.snapshot = None;
        }
        return Task::none();
    };
    let Some(form) = &mut (*aggregation) else {
        return Task::none();
    };
    if form.selection != selection {
        form.invalidate_selection(&selection);
        form.publish = None;
        return Task::none();
    }
    if let Event::Finished { selection: completed, generation, result } = event {
        if completed != selection || !form.finish_publication(&completed, generation, result) {
            return Task::none();
        }
        if let Some(error) = form.publish.as_ref().and_then(|s| s.error) {
            return super::error(notice, error);
        }
        return Task::none();
    }
    if matches!(event, Event::RetryExit) {
        if form.publish.as_ref().is_none_or(|s| s.exit_state.is_none() || s.busy || s.running()) {
            return Task::none();
        }
        form.publish = None;
        return super::list::update(aggregation, counter, loading, token, Some((client, selection)), notice, super::list::Event::Refresh);
    }
    if matches!(event, Event::OpenExit) {
        if form.loading || form.publish.is_some() {
            return Task::none();
        }
        let Some(snapshot) = &form.snapshot else {
            return Task::none();
        };
        if !super::supported(selection.scope.app) || snapshot.state.mode == AggregationMode::Direct
        {
            return Task::none();
        }
        form.publish =
            Some(Session { exit_state: Some(snapshot.state.clone()), ..Session::default() });
        return Task::none();
    }
    if matches!(event, Event::Dismiss) {
        // Dismissing a confirmation sends no write. A submitted operation stays visible
        // until terminal, so a pending cancel cannot be mistaken for cancellation.
        if form.publish.as_ref().is_some_and(|s| s.busy || s.running()) {
            return Task::none();
        }
        let refresh_exit =
            form.publish.as_ref().is_some_and(|s| s.exit_state.is_some() && s.operation.is_some());
        form.publish = None;
        if refresh_exit {
            return super::list::update(aggregation, counter, loading, token, Some((client, selection)), notice, super::list::Event::Refresh);
        }
        return Task::none();
    }
    let action = match event {
        Event::Open if !form.loading && form.publish.is_none() && form.snapshot.is_some() => {
            form.publish = Some(Session::default());
            Action::Preview
        }
        Event::Act(action) => action,
        _ => return Task::none(),
    };
    let Some(session) = &mut form.publish else {
        return Task::none();
    };
    let intent = session.clone();
    if !session.begin(action) {
        return Task::none();
    }
    let default = form.snapshot.as_ref().and_then(|s| s.state.default_provider_id.clone());
    // A write can have reached the target even when the response is lost. Require an
    // explicit state refresh before permitting any further membership changes.
    if matches!(action, Action::Confirm) {
        form.snapshot = None;
    }
    *counter = counter.wrapping_add(1);
    let generation = *counter;
    form.generation = generation;
    let token = token.to_owned();
    Task::perform(
        execute(client, selection.clone(), token, intent, action, default),
        move |result| message(Event::Finished { selection: selection.clone(), generation, result }),
    )
}
impl super::Form {
    pub fn finish_publication(
        &mut self,
        selection: &Selection,
        generation: u64,
        result: Result<Reply, AggregationApiError>,
    ) -> bool {
        if &self.selection != selection || self.generation != generation {
            return false;
        }
        let Some(session) = &mut self.publish else {
            return false;
        };
        if !session.busy {
            return false;
        }
        session.finish(result);
        true
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AggregationPublishSession([redacted])")
    }
}
impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AggregationPublishReply([redacted])")
    }
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => f.write_str("PublishOpen"),
            Self::OpenExit => f.write_str("ExitOpen"),
            Self::RetryExit => f.write_str("ExitRetry"),
            Self::Dismiss => f.write_str("PublishDismiss"),
            Self::Act(action) => f.debug_tuple("PublishAct").field(action).finish(),
            Self::Finished { .. } => f.write_str("PublishFinished([redacted])"),
        }
    }
}


#[cfg(test)]
#[path = "publish_tests.rs"]
mod publish_tests;

#[cfg(test)]
#[path = "exit_tests.rs"]
mod exit_tests;
