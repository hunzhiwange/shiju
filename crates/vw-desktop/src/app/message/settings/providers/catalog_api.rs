//! Search and add models to a saved application's exact deployment through the SDK.

use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
    state::app_providers::{
        list::Scope,
        model_catalog::{CatalogContext, CatalogForm},
    },
};
use iced::Task;
use std::collections::HashSet;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        common::PaginationRequest,
        model_catalog::{
            ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogIdentity, ModelCatalogPreset,
            ModelCatalogProject, ModelCatalogQuery, ScopedModelCatalogIdentity,
        },
    },
};

#[derive(Clone)]
pub enum Event {
    Open(String),
    Close,
    Query(String),
    Search,
    ChooseEndpoint { generation: u64, preset: usize },
    Toggle { generation: u64, preset: usize, model: usize, checked: bool },
    Endpoint(String),
    ManualId(String),
    Save,
    Searched { generation: u64, result: Result<Vec<ModelCatalogPreset>, String> },
    Saved { generation: u64, result: Result<usize, String> },
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ModelCatalogEvent")
    }
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::ModelCatalog(event))
}

fn current(app: &App, scope: &Scope) -> Result<(GatewayClient, CatalogContext), String> {
    let (client, actual_scope) = list::current(app, scope.app)?;
    if &actual_scope != scope || app.apps_catalog.token.is_empty() {
        return Err("应用执行主机已切换，请重新打开模型目录".into());
    }
    let project = match (&app.project_id, &scope.context.project) {
        (Some(id), Some(directory)) if !id.is_empty() => {
            ModelCatalogProject { project_id: id.as_str().into(), directory: directory.clone() }
        }
        _ => return Err("请先打开已注册的项目，再使用模型目录".into()),
    };
    Ok((
        client,
        CatalogContext {
            scope: actual_scope,
            project,
            access_generation: app.apps_catalog.generation,
        },
    ))
}

fn catalog_error(error: AgentAppsReadError) -> String {
    match error {
        AgentAppsReadError::Http { status: 409, .. } => "目录已被修改，请确认所选条目后重试".into(),
        AgentAppsReadError::Http { status, .. } => format!("模型目录请求失败（HTTP {status}）"),
        _ => "模型目录请求失败，请检查网关连接后重试".into(),
    }
}

async fn search(
    client: &GatewayClient,
    context: &CatalogContext,
    query: &str,
    token: &str,
) -> Result<Vec<ModelCatalogPreset>, String> {
    let mut items = vec![];
    let mut page = PaginationRequest { limit: Some(100), cursor: None };
    let mut cursors = HashSet::new();
    loop {
        let response = client
            .model_catalog_presets(
                &context.project,
                &context.scope.host,
                context.scope.app,
                Some(query),
                Some(&page),
                token,
            )
            .await
            .map_err(catalog_error)?;
        items.extend(response.items);
        let Some(cursor) = response.next_cursor else { return Ok(items) };
        if !cursors.insert(cursor.clone()) {
            return Err("模型目录返回了重复的分页游标".into());
        }
        page.cursor = Some(cursor);
    }
}

async fn save(
    client: &GatewayClient,
    context: &CatalogContext,
    deployment: ModelCatalogIdentity,
    selected: Vec<ScopedModelCatalogIdentity>,
    token: &str,
) -> Result<usize, String> {
    let mut query = ModelCatalogQuery {
        deployment: deployment.clone(),
        search: None,
        page: Some(PaginationRequest { limit: Some(100), cursor: None }),
    };
    let first =
        client.model_catalog(&context.project, &query, token).await.map_err(catalog_error)?;
    let expected_revision = first.revision.clone();
    let mut response = first;
    let mut existing = HashSet::new();
    let mut cursors = HashSet::new();
    loop {
        if response.revision != expected_revision {
            return Err("目录已被修改，请确认所选条目后重试".into());
        }
        for row in response.items {
            if row.identity.scope != context.scope.context.tenant
                || row.identity.deployment != deployment
            {
                return Err("服务返回了其他部署的模型目录".into());
            }
            existing.insert(row.identity);
        }
        let Some(cursor) = response.next_cursor else { break };
        if !cursors.insert(cursor.clone()) {
            return Err("模型目录返回了重复的分页游标".into());
        }
        query.page = Some(PaginationRequest { limit: Some(100), cursor: Some(cursor) });
        response =
            client.model_catalog(&context.project, &query, token).await.map_err(catalog_error)?;
    }
    let edits: Vec<_> = selected
        .into_iter()
        .filter(|row| existing.insert(row.clone()))
        .map(|row| ModelCatalogEdit::Upsert {
            model_id: row.model_id,
            display_alias: None,
            metadata: Box::default(),
        })
        .collect();
    let count = edits.len();
    if count != 0 {
        client
            .edit_model_catalog(
                &context.project,
                &ModelCatalogEditRequest {
                    deployment,
                    expected_revision,
                    edits,
                    default_change: None,
                },
                token,
            )
            .await
            .map_err(catalog_error)?;
    }
    Ok(count)
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
    if matches!(event, Event::Close) {
        if app.app_providers.model_catalog.as_ref().is_some_and(|form| !form.saving) {
            app.app_providers.model_catalog = None;
        }
        return Task::none();
    }
    let (client, context) = match current(app, &scope) {
        Ok(value) => value,
        Err(error) => {
            app.app_providers.model_catalog = None;
            return app.show_error_toast(error);
        }
    };
    if let Event::Open(provider_id) = event {
        if app.app_providers.model_catalog.is_some()
            || !app.app_providers.snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.catalog.rows.iter().any(|row| row.provider.id == provider_id)
            })
        {
            return Task::none();
        }
        app.app_providers.model_catalog = Some(CatalogForm::new(context, provider_id));
        return update(app, Event::Search);
    }
    let generation = &mut app.app_providers.generation;
    let Some(form) = &mut app.app_providers.model_catalog else { return Task::none() };
    if form.context != context {
        app.app_providers.model_catalog = None;
        return Task::none();
    }
    let token = &app.apps_catalog.token;
    match event {
        Event::Searched { generation: request, result }
            if *generation == request && form.loading =>
        {
            form.loading = false;
            match result {
                Ok(items) => {
                    form.finish_search(&context, items);
                }
                Err(error) => form.notice = Some(error),
            }
        }
        Event::Saved { generation: request, result } if *generation == request && form.saving => {
            form.saving = false;
            match result {
                Ok(count) => {
                    form.selected.clear();
                    form.manual_id.clear();
                    return app.show_success_toast(format!("已添加 {count} 个模型"));
                }
                Err(error) => form.notice = Some(error),
            }
        }
        _ if form.saving => {}
        Event::Query(query) => {
            *generation = generation.wrapping_add(1);
            form.query = query;
            form.items.clear();
            form.selected.clear();
            form.notice = None;
            form.loading = false;
            form.searched = false;
        }
        Event::Search => {
            *generation = generation.wrapping_add(1);
            let request = *generation;
            form.items.clear();
            form.selected.clear();
            form.notice = None;
            form.loading = true;
            form.searched = false;
            let query = form.query.trim().to_owned();
            let token = token.clone();
            return Task::perform(
                async move { search(&client, &context, &query, &token).await },
                move |result| message(Event::Searched { generation: request, result }),
            );
        }
        Event::ChooseEndpoint { generation: request, preset } if *generation == request => {
            if let Some(endpoint) = form.items.get(preset).and_then(|row| row.endpoint.clone()) {
                form.set_endpoint(endpoint);
            }
        }
        Event::Toggle { generation: request, preset, model, checked } if *generation == request => {
            form.toggle(preset, model, checked);
        }
        Event::Endpoint(value) => form.set_endpoint(value),
        Event::ManualId(value) => form.manual_id = value,
        Event::Save => {
            let deployment = form.deployment();
            let mut selected = form.selected.clone();
            let manual_id = form.manual_id.trim();
            if !manual_id.is_empty() {
                selected.push(ScopedModelCatalogIdentity {
                    scope: scope.context.tenant,
                    deployment: deployment.clone(),
                    model_id: manual_id.into(),
                });
            }
            if deployment.endpoint.is_empty() || selected.is_empty() {
                return Task::none();
            }
            *generation = generation.wrapping_add(1);
            let request = *generation;
            form.loading = false;
            form.saving = true;
            form.notice = None;
            let token = token.clone();
            return Task::perform(
                async move { save(&client, &context, deployment, selected, &token).await },
                move |result| message(Event::Saved { generation: request, result }),
            );
        }
        _ => {}
    }
    Task::none()
}
