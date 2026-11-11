//! Read-only aggregation directory. All native observations come through the SDK.
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
    state::app_providers::list::Scope,
};
use iced::Task;
use vw_gateway_client::{
    AggregationApiError, GatewayClient,
    vw_api_types::{
        agent_apps::{AppId, aggregation::*},
        app_providers::AppProviderSummary,
        id::ProjectId,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub scope: Scope,
    pub project: ProjectId,
    pub catalog_generation: u64,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub state: AggregationState,
    pub candidates: Vec<AppProviderSummary>,
}
#[derive(Clone, Debug)]
pub(crate) struct Form {
    pub selection: Selection,
    pub generation: u64,
    pub loading: bool,
    pub snapshot: Option<Snapshot>,
    pub error: Option<&'static str>,
}
impl Form {
    pub fn new(selection: Selection, generation: u64) -> Self {
        Self { selection, generation, loading: true, snapshot: None, error: None }
    }
    pub fn invalidate_selection(&mut self, current: &Selection) {
        if &self.selection != current {
            self.loading = false;
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
            Ok(snapshot) => self.snapshot = Some(snapshot),
            Err(error) => self.error = Some(error_label(&error)),
        }
        true
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Refresh,
    Close,
    Loaded { selection: Selection, generation: u64, result: Result<Snapshot, AggregationApiError> },
}
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Aggregation(event))
}
pub(crate) fn supported(app: AppId) -> bool {
    cfg!(target_os = "macos") && matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop)
}
pub(crate) fn error_label(error: &AggregationApiError) -> &'static str {
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
fn current(app: &App) -> Option<(GatewayClient, Selection)> {
    let scope = app.app_providers.scope.as_ref()?;
    let (client, now) = list::current(app, scope.app).ok()?;
    if &now != scope {
        return None;
    }
    let project = app.project_id.as_deref().filter(|s| !s.trim().is_empty())?.into();
    Some((
        client,
        Selection { scope: now, project, catalog_generation: app.apps_catalog.generation },
    ))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.app_providers.aggregation = None;
        return Task::none();
    }
    let Some((client, selection)) = current(app) else {
        app.app_providers.aggregation = None;
        return app.show_error_toast("请先选择项目并连接执行主机");
    };
    if let Event::Loaded { selection: completed, generation, result } = event {
        if completed != selection {
            if let Some(form) = &mut app.app_providers.aggregation {
                form.invalidate_selection(&selection);
            }
            return Task::none();
        }
        if let Some(form) = &mut app.app_providers.aggregation
            && form.finish(&completed, generation, result)
            && let Some(error) = form.error
        {
            return app.show_error_toast(error);
        }
        return Task::none();
    }
    if matches!(event, Event::Open)
        && (app.app_providers.loading || app.app_providers.aggregation.is_some())
    {
        return Task::none();
    }
    if matches!(event, Event::Refresh)
        && app.app_providers.aggregation.as_ref().is_none_or(|f| f.loading)
    {
        return Task::none();
    }
    app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
    let generation = app.app_providers.generation;
    let mut form = Form::new(selection.clone(), generation);
    if !supported(selection.scope.app) {
        form.loading = false;
        form.error = Some("此应用或平台不支持聚合目录");
        app.app_providers.aggregation = Some(form);
        return Task::none();
    }
    app.app_providers.aggregation = Some(form);
    let token = app.apps_catalog.token.clone();
    Task::perform(load(client, selection.clone(), token), move |result| {
        message(Event::Loaded { selection: selection.clone(), generation, result })
    })
}
pub(crate) async fn load(
    client: GatewayClient,
    selection: Selection,
    token: String,
) -> Result<Snapshot, AggregationApiError> {
    // Aggregation SDK consumes endpoint bearer; catalog SDK consumes the same explicit grant.
    let endpoint = client
        .endpoint()
        .clone()
        .with_auth(vw_gateway_client::GatewayAuth { skey: Some(token.clone()) });
    let aggregation_client =
        GatewayClient::new(endpoint).map_err(AggregationApiError::Transport)?;
    let state = aggregation_client
        .aggregation_state(
            selection.scope.app,
            &AggregationQuery {
                target_host_id: selection.scope.host.clone(),
                project_id: selection.project,
            },
        )
        .await?;
    let catalog = client
        .app_provider_catalog(selection.scope.app, &selection.scope.host, &token)
        .await
        .map_err(|e| match e {
            vw_gateway_client::AgentAppsReadError::Http { status, body } => {
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
#[cfg(test)]
#[path = "aggregation_list_tests.rs"]
mod tests;
