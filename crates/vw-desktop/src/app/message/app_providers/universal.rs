use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{list::Scope, universal::Form},
};
use iced::Task;
use vw_gateway_client::{
    GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{
            universal::UniversalProviderSummary,
            universal_sync::{UniversalCatalogResult, UniversalSyncRequest, UniversalSyncResult},
        },
    },
};

pub(crate) async fn project(
    client: GatewayClient,
    id: String,
    apps: Vec<AppId>,
) -> Result<Vec<UniversalSyncResult>, String> {
    client.universal_provider_sync(&id, &UniversalSyncRequest { apps }).await
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Listed(Vec<UniversalProviderSummary>),
    Projected(Vec<UniversalSyncResult>),
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    RetryList,
    Select(String),
    Toggle(AppId, bool),
    Submit,
    Completed { scope: Scope, generation: u64, result: Result<Outcome, String> },
}

pub(crate) fn finish(
    form: &mut Form,
    current: (&Scope, u64),
    completed: (&Scope, u64),
    result: Result<Outcome, String>,
) -> bool {
    if current != completed || !form.busy {
        return false;
    }
    form.busy = false;
    match result {
        Ok(Outcome::Listed(records)) => form.records = Some(records),
        Ok(Outcome::Projected(results)) => {
            form.refresh_catalog_on_close |= results.iter().any(|item| {
                item.app_id == current.0.app && item.catalog == UniversalCatalogResult::Updated
            });
            form.results = results;
        }
        Err(error) => form.error = Some(error),
    }
    true
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Universal(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.app_providers;
    match event {
        Event::Open => {
            if state.loading
                || state.editor.is_some()
                || state.form.is_some()
                || state.endpoints.is_some()
                || state.desktop.is_some()
                || state.icons.is_some()
                || state.universal.is_some()
            {
                return Task::none();
            }
            state.universal = Some(Form::default());
            return update(app, Event::RetryList);
        }
        Event::Close => {
            if state.universal.as_ref().is_some_and(|form| !form.busy)
                && let Some(form) = state.universal.take()
            {
                state.generation = state.generation.wrapping_add(1);
                if form.refresh_catalog_on_close {
                    return list::update(app, ListMessage::Refresh);
                }
            }
            return Task::none();
        }
        _ => {}
    }

    let Some(scope) = state.scope.clone() else { return Task::none() };
    let generation = state.generation;
    let Some(form) = state.universal.as_mut() else { return Task::none() };
    if let Event::Completed { scope: completed_scope, generation: completed_generation, result } =
        event
    {
        if !list::current(app, scope.app).is_ok_and(|(_, now)| now == scope) {
            return Task::none();
        }
        let Some(form) = app.app_providers.universal.as_mut() else { return Task::none() };
        if finish(form, (&scope, generation), (&completed_scope, completed_generation), result)
            && let Some(error) = form.error.clone()
        {
            return app.show_error_toast(error);
        }
        return Task::none();
    }
    if form.busy {
        return Task::none();
    }

    let projection = match event {
        Event::RetryList if form.records.is_none() => None,
        Event::Select(id) => {
            if form.selected.as_deref() != Some(id.as_str())
                && form
                    .records
                    .as_ref()
                    .is_some_and(|records| records.iter().any(|row| row.id == id))
            {
                form.selected = Some(id);
                form.apps.clear();
                form.results.clear();
                form.error = None;
            }
            return Task::none();
        }
        Event::Toggle(id, checked) if form.selected.is_some() => {
            if checked && !form.apps.contains(&id) {
                form.apps.push(id);
            } else if !checked {
                form.apps.retain(|app| *app != id);
            }
            form.results.clear();
            form.error = None;
            return Task::none();
        }
        Event::Submit if !form.apps.is_empty() => {
            let Some(record) = form.selected_record() else { return Task::none() };
            Some((record.id.clone(), form.apps.clone()))
        }
        _ => return Task::none(),
    };
    let (client, now) = match list::current(app, scope.app) {
        Ok(value) => value,
        Err(error) => return app.show_error_toast(error),
    };
    if now != scope {
        return list::update(app, ListMessage::Open(scope.app));
    }

    let state = &mut app.app_providers;
    let form = state.universal.as_mut().expect("projection form remains open");
    form.busy = true;
    form.error = None;
    form.results.clear();
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Task::perform(
        async move {
            match projection {
                Some((id, apps)) => project(client, id, apps).await.map(Outcome::Projected),
                None => client.universal_providers_list().await.map(Outcome::Listed),
            }
        },
        move |result| message(Event::Completed { scope: scope.clone(), generation, result }),
    )
}
