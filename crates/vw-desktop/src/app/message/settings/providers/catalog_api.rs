//! Read and edit models in a saved application's exact deployment through the SDK.

use super::{discovery, metadata_fill};
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
    state::app_providers::{
        list::Scope,
        model_catalog::{CatalogContext, CatalogForm, FillState},
    },
};
use iced::Task;
use std::collections::HashSet;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        common::PaginationRequest,
        id::ModelId,
        model_catalog::{
            ModelCatalogDefaultChange, ModelCatalogDefaultRequest, ModelCatalogEdit,
            ModelCatalogEditRequest, ModelCatalogIdentity, ModelCatalogPreset, ModelCatalogProject,
            ModelCatalogQuery, ModelCatalogReadResponse,
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
    Alias(String),
    MakeDefault(bool),
    EditModel { generation: u64, model_id: ModelId },
    CancelEdit,
    LoadCatalog,
    CatalogLoaded { generation: u64, result: Result<ModelCatalogReadResponse, String> },
    Save,
    Fill(metadata_fill::Event),
    Discovery(discovery::Event),
    Searched { generation: u64, result: Result<Vec<ModelCatalogPreset>, String> },
    Saved { generation: u64, result: Result<(), String> },
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
        AgentAppsReadError::Http { status: 409, .. } => {
            "目录已被修改，请刷新已保存模型，确认草稿后重试".into()
        }
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

pub(super) async fn read_catalog(
    client: &GatewayClient,
    context: &CatalogContext,
    deployment: ModelCatalogIdentity,
    token: &str,
) -> Result<ModelCatalogReadResponse, String> {
    let mut query = ModelCatalogQuery {
        deployment,
        search: None,
        page: Some(PaginationRequest { limit: Some(100), cursor: None }),
    };
    let mut catalog =
        client.model_catalog(&context.project, &query, token).await.map_err(catalog_error)?;
    let mut cursors = HashSet::new();
    while let Some(cursor) = catalog.next_cursor.take() {
        if !cursors.insert(cursor.clone()) {
            return Err("模型目录返回了重复的分页游标".into());
        }
        query.page = Some(PaginationRequest { limit: Some(100), cursor: Some(cursor) });
        let page =
            client.model_catalog(&context.project, &query, token).await.map_err(catalog_error)?;
        if page.revision != catalog.revision {
            return Err("目录已被修改，请刷新已保存模型，确认草稿后重试".into());
        }
        catalog.items.extend(page.items);
        catalog.next_cursor = page.next_cursor;
    }
    if catalog.items.iter().any(|row| {
        row.identity.scope != context.scope.context.tenant
            || row.identity.deployment != query.deployment
    }) {
        return Err("服务返回了其他部署的模型目录".into());
    }
    Ok(catalog)
}

struct SaveDraft {
    deployment: ModelCatalogIdentity,
    catalog: Option<ModelCatalogReadResponse>,
    selected: Vec<ModelId>,
    manual_id: Option<ModelId>,
    edit_id: Option<ModelId>,
    alias: Option<String>,
    default_override: Option<bool>,
}

async fn save(
    client: &GatewayClient,
    context: &CatalogContext,
    draft: SaveDraft,
    token: &str,
) -> Result<(), String> {
    let SaveDraft { deployment, catalog, selected, manual_id, edit_id, alias, default_override } =
        draft;
    let catalog = match catalog {
        Some(catalog) => catalog,
        None => read_catalog(client, context, deployment.clone(), token).await?,
    };
    let mut existing: HashSet<_> =
        catalog.items.iter().map(|row| row.identity.model_id.clone()).collect();
    let mut edits = vec![];
    let mut default_change = None;

    if let Some(id) = edit_id.or(manual_id.clone()) {
        let row = catalog.items.iter().find(|row| row.identity.model_id == id);
        let adding = manual_id.is_some();
        if !adding && row.is_none() {
            return Err("模型已从此部署移除，请取消编辑后重试".into());
        }
        let is_default = catalog.default_model_id.as_ref() == Some(&id);
        if default_override == Some(true) && !is_default {
            default_change = Some(ModelCatalogDefaultChange::Set { model_id: id.clone() });
        } else if !adding && default_override == Some(false) && is_default {
            default_change = Some(ModelCatalogDefaultChange::Clear);
        }

        let old_alias = row.and_then(|row| row.display_alias.as_deref()).unwrap_or_default();
        // A blank manual alias preserves an existing entry. In edit mode, clearing it is explicit.
        let display_alias =
            alias.filter(|alias| (!adding || !alias.is_empty()) && old_alias != alias);
        if existing.insert(id.clone()) || display_alias.is_some() {
            edits.push(ModelCatalogEdit::Upsert {
                model_id: id,
                display_alias,
                metadata: Box::default(),
            });
        }
    }

    edits.extend(selected.into_iter().filter(|id| existing.insert(id.clone())).map(|model_id| {
        ModelCatalogEdit::Upsert { model_id, display_alias: None, metadata: Box::default() }
    }));

    if !edits.is_empty() {
        client
            .edit_model_catalog(
                &context.project,
                &ModelCatalogEditRequest {
                    deployment,
                    expected_revision: catalog.revision,
                    edits,
                    default_change,
                },
                token,
            )
            .await
            .map_err(catalog_error)?;
    } else if let Some(change) = default_change {
        client
            .set_model_catalog_default(
                &context.project,
                &ModelCatalogDefaultRequest {
                    deployment,
                    expected_revision: catalog.revision,
                    change,
                },
                token,
            )
            .await
            .map_err(catalog_error)?;
    }
    Ok(())
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
            if matches!(&event, Event::Fill(event) if event.is_response())
                || matches!(&event, Event::Discovery(event) if event.is_response())
            {
                return Task::none();
            }
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
                Ok(()) => {
                    form.selected.clear();
                    form.clear_model_draft();
                    form.catalog = None;
                    let reload = update(app, Event::LoadCatalog);
                    return Task::batch([reload, app.show_success_toast("已保存模型目录")]);
                }
                Err(error) => {
                    form.notice = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
        }
        Event::CatalogLoaded { generation: request, result }
            if *generation == request && form.catalog_loading =>
        {
            form.catalog_loading = false;
            match result {
                Ok(catalog) => form.apply_catalog(catalog),
                Err(error) => {
                    form.notice = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
        }
        Event::Fill(event) => return metadata_fill::update(app, client, event),
        Event::Discovery(event) => return discovery::update(app, client, event),
        _ if form.busy() => {}
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
                return update(app, Event::LoadCatalog);
            }
        }
        Event::Toggle { generation: request, preset, model, checked } if *generation == request => {
            form.toggle(preset, model, checked);
        }
        Event::Endpoint(value) => form.set_endpoint(value),
        Event::ManualId(value) if form.edit_id.is_none() => form.manual_id = value,
        Event::Alias(alias) => {
            form.alias = alias;
            form.alias_dirty = true;
        }
        Event::MakeDefault(value) => form.default_override = Some(value),
        Event::EditModel { generation: request, model_id } if *generation == request => {
            form.edit_model(model_id);
            return metadata_fill::update(app, client, metadata_fill::Event::Preview);
        }
        Event::CancelEdit => form.clear_model_draft(),
        Event::LoadCatalog => {
            let deployment = form.deployment();
            if deployment.endpoint.is_empty() {
                return Task::none();
            }
            *generation = generation.wrapping_add(1);
            let request = *generation;
            form.loading = false;
            form.catalog_loading = true;
            form.fill = FillState::Idle;
            form.notice = None;
            let token = token.clone();
            return Task::perform(
                async move { read_catalog(&client, &context, deployment, &token).await },
                move |result| message(Event::CatalogLoaded { generation: request, result }),
            );
        }
        Event::Save => {
            let deployment = form.deployment();
            let manual_id = form.manual_id.trim();
            let manual_id = (!manual_id.is_empty()).then(|| ModelId::from(manual_id));
            if deployment.endpoint.is_empty()
                || (form.selected.is_empty() && manual_id.is_none() && form.edit_id.is_none())
            {
                return Task::none();
            }
            let draft = SaveDraft {
                deployment,
                catalog: form.catalog.clone(),
                selected: form.selected.iter().map(|row| row.model_id.clone()).collect(),
                manual_id,
                edit_id: form.edit_id.clone(),
                alias: form.alias_dirty.then(|| form.alias.clone()),
                default_override: form.default_override,
            };
            *generation = generation.wrapping_add(1);
            let request = *generation;
            form.loading = false;
            form.saving = true;
            form.fill = FillState::Idle;
            form.notice = None;
            let token = token.clone();
            return Task::perform(
                async move { save(&client, &context, draft, &token).await },
                move |result| message(Event::Saved { generation: request, result }),
            );
        }
        _ => {}
    }
    Task::none()
}
