use crate::{Scope,state::list::{Snapshot,Editor}};
use super::provider_update::{State,Effect as Message};
use vw_gateway_client::GatewayClient;
use vw_api_app_provider_edit::presentation::AppProviderPresentation;
use iced::Task;
pub async fn load(
    client: GatewayClient,
    scope: Scope,
    token: String,
) -> Result<Snapshot, String> {
    let (catalog, presentation) = gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_presented_catalog(scope.app, &scope.host, &token)
        .await
        .map_err(|_| "无法读取供应商列表与排序".to_string())?;
    Ok(Snapshot { catalog, presentation })
}
pub async fn save(
    client: GatewayClient,
    scope: Scope,
    id: String,
    value: AppProviderPresentation,
) -> Result<AppProviderPresentation, String> {
    gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_presentation_set(scope.app, &id, &value).await
}
#[derive(Clone, Debug)]
pub enum Event {
Edit(String), Sort(String), Category(String), Notes(String), Cancel, Save,
Saved {scope:Scope,generation:u64,result:Result<AppProviderPresentation,String>},
}
pub fn update(state:&mut State,current:Result<(GatewayClient,Scope),String>,snapshot:Option<&Snapshot>,event:Event)->Task<Message>{
 match event {
        Event::Edit(id) => {
            if state.loading || state.editor.is_some() {
                return Task::none();
            }
            if !snapshot
                .is_some_and(|s| s.catalog.rows.iter().any(|r| r.provider.id == id))
            {
                return Task::none();
            }
            let p = snapshot.and_then(|s|s.presentation.iter().find(|e|e.id==id)).map(|e|e.presentation.clone()).unwrap_or_default();
            state.editor = Some(Editor {
                id,
                sort: p.sort_index.map(|v| v.to_string()).unwrap_or_default(),
                category: p.category.unwrap_or_default(),
                notes: p.notes.unwrap_or_default(),
                saving: false,
            });
        }
        Event::Sort(v) => {
            if let Some(f) = state.editor.as_mut().filter(|f| !f.saving) {
                f.sort = v;
            }
        }
        Event::Category(v) => {
            if let Some(f) = state.editor.as_mut().filter(|f| !f.saving) {
                f.category = v;
            }
        }
        Event::Notes(v) => {
            if let Some(f) = state.editor.as_mut().filter(|f| !f.saving) {
                f.notes = v;
            }
        }
        Event::Cancel => {
            if state.editor.as_ref().is_some_and(|f| !f.saving) {
                state.editor = None;
            }
        }
        Event::Save => {
            let Some(scope) = state.scope.clone() else { return Task::none() };
            let (client, now) = match current.clone() {
                Ok(v) => v,
                Err(e) => return super::provider_update::error(e),
            };
            if now != scope {
                return Task::done(Message::OpenList(scope.app));
            }
            let Some(form) = state.editor.as_mut() else { return Task::none() };
            if form.saving {
                return Task::none();
            }
            let sort_index = if form.sort.trim().is_empty() {
                None
            } else {
                match form.sort.trim().parse::<u32>() {
                    Ok(v) => Some(v),
                    Err(_) => return super::provider_update::error("排序须为非负整数"),
                }
            };
            let value = AppProviderPresentation {
                sort_index,
                category: (!form.category.trim().is_empty())
                    .then(|| form.category.trim().to_string()),
                notes: (!form.notes.trim().is_empty()).then(|| form.notes.trim().to_string()),
            };
            form.saving = true;
            let id = form.id.clone();
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            return Task::perform(save(client, scope.clone(), id, value), move |result| {
                Message::List(Event::Saved { scope: scope.clone(), generation, result })
            });
        }
        Event::Saved { scope, generation, result } => {
            if state.scope.as_ref() != Some(&scope)
                || state.generation != generation
                || current.clone().ok().map(|(_, s)| s).as_ref() != Some(&scope)
            {
                return Task::none();
            }
            match result {
                Ok(_) => {
                    state.editor = None;
                    return super::provider_update::refresh(state);
                }
                Err(e) => {
                    if let Some(f) = state.editor.as_mut() {
                        f.saving = false;
                    }
                    return super::provider_update::error(e);
                }
            }
        }
 }
 Task::none()
}

#[cfg(test)]
#[path="list_tests.rs"]
mod tests;
