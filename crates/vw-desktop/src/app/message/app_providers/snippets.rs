use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{
        list::Scope,
        snippets::{Form, Operation, available},
    },
};
use iced::{
    Task,
    widget::text_editor::{Action, Content},
};
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::app_providers::snippets::{
        ConfigSnippet, SnippetExtract, SnippetRead, SnippetSource, SnippetWrite,
    },
};

#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    RetryRead,
    Edit(Action),
    SelectProvider(String),
    Extract,
    AdoptExtracted,
    Preview,
    Save,
    Completed { scope: Scope, generation: u64, result: Result<ConfigSnippet, String> },
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SnippetEvent([redacted])")
    }
}

fn error_label(error: AgentAppsReadError) -> String {
    let AgentAppsReadError::Http { status, .. } = error else {
        return "无法读取公共片段服务的响应，请重试。".into();
    };
    let hint = match status {
        400 => "请检查草稿格式和片段内容。",
        401 | 403 => "请检查当前执行主机的配置权限。",
        404 => "找不到选中的供应商记录。",
        _ => "公共片段服务请求失败，请重试。",
    };
    format!("{hint}（HTTP {status}）")
}

pub(crate) async fn save(
    client: &GatewayClient,
    scope: &Scope,
    token: &str,
    content: String,
) -> Result<ConfigSnippet, String> {
    client
        .app_provider_snippet_save(
            scope.app,
            &SnippetWrite { target_host_id: scope.host.clone(), content },
            token,
        )
        .await
        .map_err(error_label)
}

pub(crate) fn finish(
    form: &mut Form,
    current: (&Scope, u64),
    completed: (&Scope, u64),
    result: Result<ConfigSnippet, String>,
) -> bool {
    if current != completed {
        return false;
    }
    let Some(operation) = form.pending.take() else { return false };
    match result {
        Ok(value) => match operation {
            Operation::Read | Operation::Save => {
                form.draft = Content::with_text(value.content.as_deref().unwrap_or_default());
                form.loaded = true;
            }
            Operation::Preview | Operation::Extract => form.preview = Some((operation, value)),
        },
        Err(error) => form.error = Some(error),
    }
    true
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Snippets(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.app_providers;
    let Some(scope) = state.scope.clone() else { return Task::none() };
    match event {
        Event::Open => {
            if !available(scope.app)
                || state.loading
                || state.editor.is_some()
                || state.form.is_some()
                || state.endpoints.is_some()
                || state.desktop.is_some()
                || state.icons.is_some()
                || state.universal.is_some()
                || state.snippets.is_some()
            {
                return Task::none();
            }
            state.snippets = Some(Form::default());
            return update(app, Event::RetryRead);
        }
        Event::Close => {
            if state.snippets.as_ref().is_some_and(|form| form.pending != Some(Operation::Save)) {
                state.snippets = None;
                state.generation = state.generation.wrapping_add(1);
            }
            return Task::none();
        }
        _ => {}
    }

    let generation = state.generation;
    let Some(form) = state.snippets.as_mut() else { return Task::none() };
    if let Event::Completed { scope: completed_scope, generation: completed_generation, result } =
        event
    {
        if !list::current(app, scope.app).is_ok_and(|(_, now)| now == scope) {
            return Task::none();
        }
        let Some(form) = app.app_providers.snippets.as_mut() else { return Task::none() };
        let was_saving = form.pending == Some(Operation::Save);
        if finish(form, (&scope, generation), (&completed_scope, completed_generation), result) {
            if let Some(error) = form.error.clone() {
                return app.show_error_toast(error);
            }
            if was_saving {
                return app.show_success_toast("公共配置片段已保存");
            }
        }
        return Task::none();
    }
    if form.pending.is_some() {
        return Task::none();
    }
    let operation = match event {
        Event::RetryRead if !form.loaded => Operation::Read,
        Event::Edit(action) if form.loaded => {
            if action.is_edit() {
                form.error = None;
                form.preview = None;
            }
            form.draft.perform(action);
            return Task::none();
        }
        Event::SelectProvider(id)
            if form.loaded
                && state.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.catalog.rows.iter().any(|row| row.provider.id == id)
                }) =>
        {
            form.provider_id = Some(id);
            form.error = None;
            form.preview = None;
            return Task::none();
        }
        Event::AdoptExtracted => {
            if let Some((Operation::Extract, value)) = &form.preview {
                form.draft = Content::with_text(value.content.as_deref().unwrap_or_default());
                form.preview = None;
            }
            return Task::none();
        }
        Event::Extract if form.loaded && form.provider_id.is_some() => Operation::Extract,
        Event::Preview if form.loaded => Operation::Preview,
        Event::Save if form.loaded => Operation::Save,
        _ => return Task::none(),
    };
    let (client, now) = match list::current(app, scope.app) {
        Ok(value) => value,
        Err(error) => return app.show_error_toast(error),
    };
    if now != scope {
        return list::update(app, ListMessage::Open(scope.app));
    }

    let token = app.apps_catalog.token.clone();
    let state = &mut app.app_providers;
    let form = state.snippets.as_mut().expect("snippet form remains open");
    form.pending = Some(operation);
    form.error = None;
    form.preview = None;
    let content = form.draft.text();
    let provider_id = form.provider_id.clone();
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let completed_scope = scope.clone();
    Task::perform(
        async move {
            match operation {
                Operation::Read => client
                    .app_provider_snippet_get(
                        scope.app,
                        &SnippetRead { target_host_id: scope.host },
                        &token,
                    )
                    .await
                    .map_err(error_label),
                Operation::Extract => client
                    .app_provider_snippet_extract(
                        scope.app,
                        &SnippetExtract {
                            target_host_id: scope.host,
                            source: SnippetSource::Saved {
                                provider_id: provider_id.expect("selected saved provider"),
                            },
                        },
                        &token,
                    )
                    .await
                    .map_err(error_label),
                Operation::Preview => client
                    .app_provider_snippet_preview(
                        scope.app,
                        &SnippetWrite { target_host_id: scope.host, content },
                        &token,
                    )
                    .await
                    .map_err(error_label),
                Operation::Save => save(&client, &scope, &token, content).await,
            }
        },
        move |result| {
            message(Event::Completed { scope: completed_scope.clone(), generation, result })
        },
    )
}
