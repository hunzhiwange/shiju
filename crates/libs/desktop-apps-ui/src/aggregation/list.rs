use super::{clients, publish};
// Scoped aggregation directory. All native observations come through the SDK.
use iced::Task;
use gateway_aggregation::{AggregationApiError, GatewayAggregationClient};
use vw_gateway_client::GatewayClient;
use vw_api_app_identity::AppId;
use vw_api_app_aggregation::*;
use vw_api_app_provider_base::AppProviderSummary;

pub use crate::aggregation::clients::Selection;
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub state: AggregationState,
    pub candidates: Vec<AppProviderSummary>,
}
#[derive(Clone, Debug)]
pub struct Form {
    pub selection: Selection,
    pub generation: u64,
    pub loading: bool,
    pub snapshot: Option<Snapshot>,
    pub error: Option<&'static str>,
    pub publish: Option<publish::Session>,
    pub clients: Option<clients::Session>,
}
impl Form {
    pub fn new(selection: Selection, generation: u64) -> Self {
        Self {
            selection,
            generation,
            loading: true,
            snapshot: None,
            error: None,
            publish: None,
            clients: None,
        }
    }
    pub fn invalidate_selection(&mut self, current: &Selection) {
        if &self.selection != current {
            self.loading = false;
            self.publish = None;
            self.clients = None;
            self.snapshot = None;
            self.error = Some("项目或执行主机上下文已改变，请刷新目录");
        }
    }
    pub fn finish(
        &mut self,
        selection: &Selection,
        generation: u64,
        result: Result<Snapshot, AggregationApiError>,
    ) -> bool {
        if &self.selection != selection || self.generation != generation || !self.loading {
            return false;
        }
        self.loading = false;
        match result {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.error = None;
            }
            Err(error) => {
                self.snapshot = None;
                self.error = Some(error_label(&error));
            }
        }
        true
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Publish(publish::Event),
    Clients(clients::Event),
    Refresh,
    Close,
    Act(Action),
    Acted { selection: Selection, generation: u64, outcome: MutationOutcome },
    Loaded { selection: Selection, generation: u64, result: Result<Snapshot, AggregationApiError> },
}
fn message(event: Event) -> Event { event }
pub fn supported(app: AppId) -> bool {
    cfg!(target_os = "macos") && matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop)
}
pub fn error_label(error: &AggregationApiError) -> &'static str {
    match error {
        AggregationApiError::Http { status: 401 | 403, .. } => "无权读取聚合目录，请检查授权",
        AggregationApiError::Http { status: 409, .. } => "目录版本冲突或恢复未完成，请刷新",
        AggregationApiError::Http { status: 422, body }
            if matches!(
                body.pointer("/error/code").and_then(serde_json::Value::as_str),
                Some("unsupported_app" | "unsupported_target")
            ) =>
        {
            "此应用或执行目标不支持聚合目录"
        }
        AggregationApiError::Http { status: 422, .. } => "聚合目录配置无效，请检查执行目标后重试",
        AggregationApiError::Http { status: 501, .. } => "此应用或执行目标不支持聚合目录",
        AggregationApiError::Http { status: 502, .. } => "执行目标读取失败，请重试",
        AggregationApiError::Http { status: 503, .. } => "执行主机离线或不可用",
        AggregationApiError::Http { status: 504, .. } => "执行目标读取超时，请重试",
        AggregationApiError::Decode(_) => "目录响应无效，请刷新",
        _ => "无法加载聚合目录，请重试",
    }
}
pub fn update(
    form: &mut Option<super::list::Form>,
    counter: &mut u64,
    loading: bool,
    token: &str,
    current: Option<(GatewayClient, Selection)>,
    notice: &mut Option<super::Notice>,
    event: Event,
) -> Task<Event> {
    if let Event::Clients(event) = event {
        return clients::update(form, counter, loading, token, current, notice, event).map(Event::Clients);
    }
    if (*form).as_ref().is_some_and(|f| f.clients.is_some()) {
        return Task::none();
    }
    if let Event::Publish(event) = event {
        return publish::update(form, counter, loading, token, current, notice, event);
    }
    if matches!(event, Event::Close) {
        (*form) = None;
        return Task::none();
    }
    let Some((client, selection)) = current else {
        (*form) = None;
        return super::error(notice, "网关不可用，请刷新后重试");
    };
    if let Event::Act(action) = event {
        let Some(form) = &mut (*form) else {
            return Task::none();
        };
        let Some(revision) = form.begin_action(&selection, &action) else {
            return Task::none();
        };
        *counter = counter.wrapping_add(1);
        let generation = *counter;
        form.generation = generation;
        let token = token.to_owned();
        return Task::perform(
            mutate(client, selection.clone(), token, action, revision),
            move |outcome| {
                message(Event::Acted { selection: selection.clone(), generation, outcome })
            },
        );
    }
    if let Event::Acted { selection: completed, generation, outcome } = event {
        if completed != selection {
            if let Some(form) = &mut (*form) {
                form.invalidate_selection(&selection);
            }
            return Task::none();
        }
        if let Some(form) = &mut (*form)
            && form.finish_mutation(&completed, generation, outcome)
            && let Some(error) = form.error
        {
            return super::error(notice, error);
        }
        return Task::none();
    }
    if let Event::Loaded { selection: completed, generation, result } = event {
        if completed != selection {
            if let Some(form) = &mut (*form) {
                form.invalidate_selection(&selection);
            }
            return Task::none();
        }
        if let Some(form) = &mut (*form)
            && form.finish(&completed, generation, result)
            && let Some(error) = form.error
        {
            return super::error(notice, error);
        }
        return Task::none();
    }
    if matches!(event, Event::Open)
        && (loading || (*form).is_some())
    {
        return Task::none();
    }
    if matches!(event, Event::Refresh)
        && (*form).as_ref().is_none_or(|f| f.loading || f.publish.is_some())
    {
        return Task::none();
    }
    *counter = counter.wrapping_add(1);
    let generation = *counter;
    let mut draft = Form::new(selection.clone(), generation);
    if !supported(selection.scope.app) {
        draft.loading = false;
        draft.error = Some("此应用或平台不支持聚合目录");
        (*form) = Some(draft);
        return Task::none();
    }
    (*form) = Some(draft);
    let token = token.to_owned();
    Task::perform(load(client, selection.clone(), token), move |result| {
        message(Event::Loaded { selection: selection.clone(), generation, result })
    })
}
pub async fn load(
    client: GatewayClient,
    selection: Selection,
    token: String,
) -> Result<Snapshot, AggregationApiError> {
    // Aggregation SDK consumes endpoint bearer; catalog SDK consumes the same explicit grant.
    let endpoint = client.endpoint().clone();
    let aggregation_client =
        GatewayAggregationClient::new(gateway_http::GatewayTransport::new(endpoint).map_err(AggregationApiError::Transport)?);
    let state = aggregation_client
        .aggregation_state(
            selection.scope.app,
            &AggregationQuery {
                target_host_id: selection.scope.host.clone(),
                project_id: selection.project,
            },
        )
        .await?;
    let catalog = gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_catalog(selection.scope.app, &selection.scope.host, &token)
        .await
        .map_err(|e| match e {
            gateway_http::AgentAppsReadError::Http { status, body } => {
                AggregationApiError::Http { status, body }
            }
            _ => AggregationApiError::Decode("catalog unavailable".into()),
        })?;
    let candidates = catalog
        .rows
        .into_iter()
        .map(|r| r.provider)
        .filter(|p| !state.members.iter().any(|m| m.provider_id == p.id))
        .collect();
    Ok(Snapshot { state, candidates })
}

#[derive(Clone, Debug)]
pub enum Action {
    Member { provider_id: String, enabled: bool },
    Default { provider_id: String },
}
impl Form {
    pub fn begin_action(
        &mut self,
        selection: &Selection,
        action: &Action,
    ) -> Option<String> {
        self.invalidate_selection(selection);
        if self.loading
            || self.publish.is_some()
            || &self.selection != selection
            || !supported(selection.scope.app)
        {
            return None;
        }
        let snapshot = self.snapshot.as_ref()?;
        let member = |id: &str| snapshot.state.members.iter().any(|m| m.provider_id == id);
        let candidate = |id: &str| snapshot.candidates.iter().any(|p| p.id == id);
        let default = snapshot.state.default_provider_id.as_deref();
        let allowed = match action {
            Action::Member { provider_id, enabled: false } => {
                member(provider_id) && default != Some(provider_id.as_str())
            }
            Action::Member { provider_id, enabled: true } => candidate(provider_id),
            Action::Default { provider_id } => {
                default != Some(provider_id.as_str())
                    && (member(provider_id) || candidate(provider_id))
            }
        };
        if !allowed {
            return None;
        }
        let revision = snapshot.state.revision.clone();
        self.loading = true;
        self.error = None;
        Some(revision)
    }
    pub fn finish_mutation(
        &mut self,
        selection: &Selection,
        generation: u64,
        outcome: MutationOutcome,
    ) -> bool {
        if !self.finish(selection, generation, outcome.result) {
            return false;
        }
        if let Some(error) = outcome.write_error {
            self.error = Some(error_label(&error));
        }
        true
    }
}

#[derive(Clone, Debug)]
pub struct MutationOutcome {
    pub result: Result<Snapshot, AggregationApiError>,
    pub write_error: Option<AggregationApiError>,
}
pub async fn mutate(
    client: GatewayClient,
    selection: Selection,
    token: String,
    action: Action,
    revision: String,
) -> MutationOutcome {
    let endpoint = client.endpoint().clone();
    let writer = match gateway_http::GatewayTransport::new(endpoint) {
        Ok(transport) => GatewayAggregationClient::new(transport),
        Err(error) => {
            return MutationOutcome {
                result: Err(AggregationApiError::Transport(error)),
                write_error: None,
            };
        }
    };
    let target_host_id = selection.scope.host.clone();
    let project_id = selection.project.clone();
    let write = match action {
        Action::Member { provider_id, enabled } => {
            writer
                .aggregation_member_set(
                    selection.scope.app,
                    &provider_id,
                    &AggregationMemberRequest {
                        target_host_id,
                        project_id,
                        expected_revision: revision,
                        enabled,
                    },
                )
                .await
        }
        Action::Default { provider_id } => {
            writer
                .aggregation_default_set(
                    selection.scope.app,
                    &AggregationDefaultRequest {
                        target_host_id,
                        project_id,
                        expected_revision: revision,
                        provider_id,
                    },
                )
                .await
        }
    };
    match write {
        Ok(_) => {
            MutationOutcome { result: load(client, selection, token).await, write_error: None }
        }
        Err(error @ AggregationApiError::Http { status: 409, .. }) => MutationOutcome {
            result: load(client, selection, token).await,
            write_error: Some(error),
        },
        Err(error) => MutationOutcome { result: Err(error), write_error: None },
    }
}


#[cfg(test)]
#[path = "list_tests.rs"]
mod list_tests;

#[cfg(test)]
#[path = "members_tests.rs"]
mod members_tests;
