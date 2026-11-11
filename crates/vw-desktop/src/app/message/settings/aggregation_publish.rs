//! Confirmation and progress for server-owned aggregation publication.
use super::{Selection, current, message as list_message};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    AggregationApiError, GatewayAuth, GatewayClient,
    vw_api_types::{
        agent_apps::aggregation::*, app_providers::operations::ProviderOperationProgress,
        task::TaskStatus,
    },
};

#[derive(Clone, Default)]
pub(crate) struct Session {
    pub preview: Option<AggregationPreview>,
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
            && self.preview.as_ref().is_some_and(|p| {
                !p.revision.is_empty()
                    && p.preview_token.as_ref().is_some_and(|t| !t.is_empty())
                    && p.default_provider_id.is_some()
                    && !p.issues.iter().any(|i| i.blocking)
            })
    }
    pub fn begin(&mut self, action: Action) -> bool {
        if self.busy {
            return false;
        }
        let allowed = match action {
            Action::Preview => self.preview.is_none() && self.operation.is_none(),
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
                    Some(AggregationOutcome::Failed { .. }) => {
                        Some("聚合发布失败，请刷新实际状态并检查恢复情况")
                    }
                    _ if p.status == TaskStatus::Failed => Some("聚合发布失败，请刷新实际状态"),
                    _ => None,
                };
                self.operation = Some(p);
            }
            Err(e) => {
                self.preview = None;
                self.error = Some(error_label(&e));
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
    Dismiss,
    Act(Action),
    Finished { selection: Selection, generation: u64, result: Result<Reply, AggregationApiError> },
}
pub(crate) fn message(event: Event) -> Message {
    list_message(super::Event::Publish(event))
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
pub(crate) async fn execute(
    client: GatewayClient,
    selection: Selection,
    token: String,
    session: Session,
    action: Action,
    default: Option<String>,
) -> Result<Reply, AggregationApiError> {
    let endpoint = client.endpoint().clone().with_auth(GatewayAuth { skey: Some(token) });
    let client = GatewayClient::new(endpoint).map_err(AggregationApiError::Transport)?;
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
                client.provider_aggregation_operation_get(&operation.operation_id.0, &query).await
            } else {
                client
                    .provider_aggregation_operation_cancel(&operation.operation_id.0, &query)
                    .await
            };
            result.map(Reply::Operation).map_err(|e| match e {
                vw_gateway_client::AgentAppsReadError::Http { status, body } => {
                    AggregationApiError::Http { status, body }
                }
                vw_gateway_client::AgentAppsReadError::Decode(s) => AggregationApiError::Decode(s),
                vw_gateway_client::AgentAppsReadError::Transport(s) => {
                    AggregationApiError::Transport(s)
                }
            })
        }
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some((client, selection)) = current(app) else {
        if let Some(f) = &mut app.app_providers.aggregation {
            f.publish = None;
            f.snapshot = None;
        }
        return Task::none();
    };
    let Some(form) = &mut app.app_providers.aggregation else {
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
            return app.show_error_toast(error);
        }
        return Task::none();
    }
    if matches!(event, Event::Dismiss) {
        // Dismissing a confirmation sends no write. A submitted operation stays visible
        // until terminal, so a pending cancel cannot be mistaken for cancellation.
        if form.publish.as_ref().is_some_and(|s| s.busy || s.running()) {
            return Task::none();
        }
        form.publish = None;
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
    app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
    let generation = app.app_providers.generation;
    form.generation = generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(
        execute(client, selection.clone(), token, intent, action, default),
        move |result| message(Event::Finished { selection: selection.clone(), generation, result }),
    )
}
impl super::Form {
    pub(crate) fn finish_publication(
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
#[cfg(test)]
#[path = "aggregation_publish_tests.rs"]
mod tests;

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
            Self::Dismiss => f.write_str("PublishDismiss"),
            Self::Act(action) => f.debug_tuple("PublishAct").field(action).finish(),
            Self::Finished { .. } => f.write_str("PublishFinished([redacted])"),
        }
    }
}
