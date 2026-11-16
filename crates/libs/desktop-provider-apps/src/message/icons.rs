use crate::Scope;
use vw_api_types::agent_apps::CustomIconEntry;
use vw_api_app_provider_edit::icons::AppProviderIcon;
#[derive(Clone)]
pub enum Outcome {
    Refreshed {
        rows: Result<Vec<CustomIconEntry>, String>,
        current: Result<AppProviderIcon, String>,
        deleted: Option<String>,
    },
    Uploaded(CustomIconEntry),
    Renamed(CustomIconEntry),
    Saved(AppProviderIcon),
    FilePicked(Option<(String, String)>),
}

pub const PNG_PREFIX: &str = "data:image/png;base64,";
pub const MAX_ICON_BYTES: usize = 64 * 1024;
pub const SCROLL_ID: &str = "cc04-provider-icons-scroll";

#[derive(Clone)]
pub enum Event {
    Open(String),
    Close,
    Refresh,
    Pick,
    Rename(String),
    Delete(String),
    DraftName(String),
    CancelDraft,
    CommitDraft,
    Select(String),
    Clear,
    Save,
    Completed { scope: Scope, generation: u64, result: Result<Outcome, String> },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderIconEvent([redacted])")
    }
}


// The desktop edits a selection draft; the SDK owns every persistent operation.
use super::provider_update::{State,Effect as Message};
use crate::{state::icons::{Draft,Form}};
use std::io::Read as _;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::Task;
use vw_gateway_client::GatewayClient;
use vw_api_types::agent_apps::AddCustomIconRequest;
use vw_api_types::agent_apps::AppId;
use vw_api_types::agent_apps::RenameCustomIconRequest;
use vw_api_app_provider_edit::icons::AppProviderIconChange as Change;



pub enum Operation {
    Refresh,
    Upload(AddCustomIconRequest),
    Rename { id: String, request: RenameCustomIconRequest },
    Delete(String),
    Save(Change),
}



pub async fn execute(
    client: GatewayClient,
    app: AppId,
    provider: String,
    operation: Operation,
) -> Result<Outcome, String> {
    let icons = gateway_app_read::GatewayAppReadClient::new(client.transport().clone());
    let deleted = match operation {
        Operation::Refresh => None,
        Operation::Delete(id) => {
            icons.custom_icon_delete(&id).await?;
            Some(id)
        }
        Operation::Upload(request) => {
            return icons
                .custom_icon_add(&request)
                .await
                .map(|result| Outcome::Uploaded(result.icon));
        }
        Operation::Rename { id, request } => {
            return icons.custom_icon_rename(&id, &request).await.map(Outcome::Renamed);
        }
        Operation::Save(request) => {
            return gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_icon_change(app, &provider, &request)
                .await
                .map(Outcome::Saved);
        }
    };
    // Both reads matter after deletion: the row is gone, but saved provider images are copies.
    let providers = gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone());
    let (rows, current) =
        tokio::join!(icons.custom_icons_list(), providers.app_provider_icon(app, &provider));
    Ok(Outcome::Refreshed { rows, current, deleted })
}

pub fn finish(form: &mut Form, result: Result<Outcome, String>) {
    form.busy = false;
    form.notice = None;
    match result {
        Ok(Outcome::Refreshed { rows, current, deleted }) => {
            if deleted.is_some() {
                form.draft = None;
            }
            let error = rows.as_ref().err().or_else(|| current.as_ref().err());
            if let Some(error) = error {
                form.notice = Some(if deleted.is_some() {
                    format!("库条目已删除，刷新失败：{error}")
                } else {
                    error.clone()
                });
            }
            // Failed reads must not discard the last usable list or saved icon.
            if let Ok(rows) = rows {
                form.rows = Some(rows);
            } else if let Some(deleted) = &deleted {
                // The delete already succeeded even when the follow-up download failed.
                if let Some(rows) = &mut form.rows {
                    rows.retain(|row| &row.id != deleted);
                }
            }
            if let Ok(current) = current {
                form.current = Some(current);
            }
            if let Some(Change::Select { icon_id }) = &form.selection
                && (deleted.as_ref() == Some(icon_id)
                    || form
                        .rows
                        .as_ref()
                        .is_some_and(|rows| !rows.iter().any(|row| &row.id == icon_id)))
            {
                form.selection = None;
            }
        }
        Ok(Outcome::Uploaded(icon)) => {
            let rows = form.rows.get_or_insert_with(Vec::new);
            if let Some(row) = rows.iter_mut().find(|row| row.id == icon.id) {
                *row = icon.clone();
            } else {
                rows.insert(0, icon.clone());
            }
            form.selection = Some(Change::Select { icon_id: icon.id });
            form.draft = None;
        }
        Ok(Outcome::Renamed(icon)) => {
            if let Some(row) =
                form.rows.as_mut().and_then(|rows| rows.iter_mut().find(|row| row.id == icon.id))
            {
                *row = icon;
            }
            form.draft = None;
        }
        Ok(Outcome::Saved(icon)) => {
            form.current = Some(icon);
            form.selection = None;
        }
        Ok(Outcome::FilePicked(Some((filename, data)))) => {
            let name = match form.draft.take() {
                Some(Draft::Upload { name, .. }) => name,
                _ => filename,
            };
            form.draft = Some(Draft::Upload { name, data });
        }
        Ok(Outcome::FilePicked(None)) => {}
        Err(error) => form.notice = Some(error),
    }
}



fn effect_message(event: Event) -> Message {
    Message::Icons(event)
}

async fn pick() -> Result<Outcome, String> {
    let Some(file) = rfd::AsyncFileDialog::new().add_filter("PNG", &["png"]).pick_file().await
    else {
        return Ok(Outcome::FilePicked(None));
    };
    let name = file.file_name();
    let name = name.strip_suffix(".png").unwrap_or(&name).to_owned();
    #[cfg(target_arch = "wasm32")]
    {
        if file.inner().size() > MAX_ICON_BYTES as f64 {
            return Err("所选文件超过 64 KiB。".into());
        }
        let bytes = file.read().await;
        if bytes.len() > MAX_ICON_BYTES {
            return Err("所选文件超过 64 KiB。".into());
        }
        Ok(Outcome::FilePicked(Some((name, format!("{PNG_PREFIX}{}", STANDARD.encode(bytes))))))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = file.path().to_owned();
        tokio::task::spawn_blocking(move || {
            // Cap local reads before encoding. The service decides which submitted PNGs it accepts.
            let mut bytes = Vec::new();
            std::fs::File::open(path)
                .and_then(|file| file.take((MAX_ICON_BYTES + 1) as u64).read_to_end(&mut bytes))
                .map_err(|_| "无法读取所选文件。")?;
            if bytes.len() > MAX_ICON_BYTES {
                return Err("所选文件超过 64 KiB。".into());
            }
            Ok(Outcome::FilePicked(Some((name, format!("{PNG_PREFIX}{}", STANDARD.encode(bytes))))))
        })
        .await
        .map_err(|_| "无法读取所选文件。")?
    }
}

pub fn update(state: &mut State, current: Result<(GatewayClient, Scope), String>, snapshot:Option<&crate::state::list::Snapshot>, event: Event) -> Task<Message> {
    match event {
        Event::Open(id) => {

            if state.loading
                || state.editor.is_some()
                || state.form.is_some()
                || state.endpoints.is_some()
                || state.desktop.is_some()
                || state.icons.is_some()
            {
                return Task::none();
            }
            let Some(provider) = snapshot
                .and_then(|s| s.catalog.rows.iter().find(|row| row.provider.id == id))
            else {
                return Task::none();
            };
            state.icons =
                Some(Form { id, name: provider.provider.name.clone(), ..Form::default() });
            return Task::batch([
                update(state, current.clone(), snapshot, Event::Refresh),
                iced::widget::operation::snap_to(
                    iced::widget::Id::new(SCROLL_ID),
                    iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
                )
                .map(|_: ()| Message::None),
            ]);
        }
        Event::Close => {
            if state.icons.as_ref().is_some_and(|form| !form.busy) {
                state.icons = None;
                state.generation = state.generation.wrapping_add(1);
            }
            return Task::none();
        }
        Event::Completed { scope, generation, result } => {
            if state.scope.as_ref() != Some(&scope)
                || state.generation != generation
                || !current.clone().is_ok_and(|(_, now)| now == scope)
            {
                return Task::none();
            }
            let Some(form) = state.icons.as_mut().filter(|form| form.busy) else {
                return Task::none();
            };
            let saved = matches!(result, Ok(Outcome::Saved(_)));
            finish(form, result);
            if let Some(error) = form.notice.clone() {
                return super::provider_update::error(error);
            }
            if saved {
                return super::provider_update::info("供应商图标已保存");
            }
            return Task::none();
        }
        _ => {}
    }
    let Some(scope) = state.scope.clone() else { return Task::none() };
    let Some(form) = state.icons.as_mut().filter(|form| !form.busy) else {
        return Task::none();
    };
    let renaming = matches!(event, Event::Rename(_));
    let operation = match event {
        Event::Refresh if form.draft.is_none() => Some(Operation::Refresh),
        Event::Pick
            if form.rows.is_some() && matches!(form.draft, None | Some(Draft::Upload { .. })) =>
        {
            None
        }
        Event::Rename(id) | Event::Delete(id) if form.draft.is_none() => {
            let Some(entry) =
                form.rows.as_ref().and_then(|rows| rows.iter().find(|row| row.id == id))
            else {
                return Task::none();
            };
            let name = entry.name.clone();
            form.draft = Some(if renaming {
                Draft::Rename { id, name }
            } else {
                Draft::Delete { id, name }
            });
            form.notice = None;
            return iced::widget::operation::snap_to(
                iced::widget::Id::new(SCROLL_ID),
                iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
            )
            .map(|_: ()| Message::None);
        }
        Event::DraftName(value) => {
            if let Some(Draft::Upload { name, .. } | Draft::Rename { name, .. }) = &mut form.draft {
                *name = value;
                form.notice = None;
            }
            return Task::none();
        }
        Event::CancelDraft => {
            form.draft = None;
            form.notice = None;
            return Task::none();
        }
        Event::CommitDraft => match &form.draft {
            Some(Draft::Upload { name, data }) => Some(Operation::Upload(AddCustomIconRequest {
                name: name.clone(),
                data: data.clone(),
            })),
            Some(Draft::Rename { id, name }) => Some(Operation::Rename {
                id: id.clone(),
                request: RenameCustomIconRequest { name: name.clone() },
            }),
            Some(Draft::Delete { id, .. }) => Some(Operation::Delete(id.clone())),
            None => return Task::none(),
        },
        Event::Select(id) if form.draft.is_none() => {
            if form.rows.as_ref().is_some_and(|rows| rows.iter().any(|row| row.id == id)) {
                form.selection = Some(Change::Select { icon_id: id });
                form.notice = None;
            }
            return Task::none();
        }
        Event::Clear if form.draft.is_none() => {
            form.selection = Some(Change::Clear);
            form.notice = None;
            return Task::none();
        }
        Event::Save if form.draft.is_none() => match &form.selection {
            Some(change @ Change::Select { icon_id })
                if form
                    .rows
                    .as_ref()
                    .is_some_and(|rows| rows.iter().any(|row| &row.id == icon_id)) =>
            {
                Some(Operation::Save(change.clone()))
            }
            Some(Change::Clear) => Some(Operation::Save(Change::Clear)),
            _ => return Task::none(),
        },
        _ => return Task::none(),
    };
    let id = form.id.clone();
    let (client, now) = match current.clone() {
        Ok(value) => value,
        Err(error) => return super::provider_update::error(error),
    };
    if now != scope {
        return Task::done(Message::OpenList(scope.app));
    }
    let form = state.icons.as_mut().unwrap();
    form.busy = true;
    form.notice = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let target = scope.app;
    Task::perform(
        async move {
            match operation {
                Some(operation) => execute(client, target, id, operation).await,
                None => pick().await,
            }
        },
        move |result| effect_message(Event::Completed { scope: scope.clone(), generation, result }),
    )
}

pub fn message(value:Event)->crate::Message {crate::Message::Icons(value)}
