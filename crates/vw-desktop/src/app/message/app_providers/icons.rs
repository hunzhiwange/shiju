//! The desktop edits a selection draft; the SDK owns every persistent operation.
use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{
        icons::{Draft, Form},
        list::Scope,
    },
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
use vw_gateway_client::{
    GatewayClient,
    vw_api_types::{
        agent_apps::{AddCustomIconRequest, AppId, CustomIconEntry, RenameCustomIconRequest},
        app_providers::icons::{AppProviderIcon, AppProviderIconChange as Change},
    },
};

pub(crate) const PNG_PREFIX: &str = "data:image/png;base64,";
pub(crate) const MAX_ICON_BYTES: usize = 64 * 1024;
pub(crate) const SCROLL_ID: &str = "cc04-provider-icons-scroll";

pub(crate) enum Operation {
    Refresh,
    Upload(AddCustomIconRequest),
    Rename { id: String, request: RenameCustomIconRequest },
    Delete(String),
    Save(Change),
}

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

pub(crate) async fn execute(
    client: GatewayClient,
    app: AppId,
    provider: String,
    operation: Operation,
) -> Result<Outcome, String> {
    let deleted = match operation {
        Operation::Refresh => None,
        Operation::Delete(id) => {
            client.custom_icon_delete(&id).await?;
            Some(id)
        }
        Operation::Upload(request) => {
            return client
                .custom_icon_add(&request)
                .await
                .map(|result| Outcome::Uploaded(result.icon));
        }
        Operation::Rename { id, request } => {
            return client.custom_icon_rename(&id, &request).await.map(Outcome::Renamed);
        }
        Operation::Save(request) => {
            return client
                .app_provider_icon_change(app, &provider, &request)
                .await
                .map(Outcome::Saved);
        }
    };
    // Both reads matter after deletion: the row is gone, but saved provider images are copies.
    let (rows, current) =
        tokio::join!(client.custom_icons_list(), client.app_provider_icon(app, &provider));
    Ok(Outcome::Refreshed { rows, current, deleted })
}

pub(crate) fn finish(form: &mut Form, result: Result<Outcome, String>) {
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
            form.rows = rows.ok();
            form.current = current.ok();
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

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Icons(event))
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

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Open(id) => {
            let state = &mut app.app_providers;
            if state.loading
                || state.editor.is_some()
                || state.form.is_some()
                || state.endpoints.is_some()
                || state.desktop.is_some()
                || state.icons.is_some()
            {
                return Task::none();
            }
            let Some(provider) = state
                .snapshot
                .as_ref()
                .and_then(|s| s.catalog.rows.iter().find(|row| row.provider.id == id))
            else {
                return Task::none();
            };
            state.icons =
                Some(Form { id, name: provider.provider.name.clone(), ..Form::default() });
            return Task::batch([
                update(app, Event::Refresh),
                iced::widget::operation::snap_to(
                    iced::widget::Id::new(SCROLL_ID),
                    iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
                )
                .map(|_: ()| Message::None),
            ]);
        }
        Event::Close => {
            if app.app_providers.icons.as_ref().is_some_and(|form| !form.busy) {
                app.app_providers.icons = None;
                app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            }
            return Task::none();
        }
        Event::Completed { scope, generation, result } => {
            if app.app_providers.scope.as_ref() != Some(&scope)
                || app.app_providers.generation != generation
                || !list::current(app, scope.app).is_ok_and(|(_, now)| now == scope)
            {
                return Task::none();
            }
            let Some(form) = app.app_providers.icons.as_mut().filter(|form| form.busy) else {
                return Task::none();
            };
            let saved = matches!(result, Ok(Outcome::Saved(_)));
            finish(form, result);
            if let Some(error) = form.notice.clone() {
                return app.show_error_toast(error);
            }
            if saved {
                return app.show_info_toast("供应商图标已保存");
            }
            return Task::none();
        }
        _ => {}
    }
    let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
    let Some(form) = app.app_providers.icons.as_mut().filter(|form| !form.busy) else {
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
    let (client, now) = match list::current(app, scope.app) {
        Ok(value) => value,
        Err(error) => return app.show_error_toast(error),
    };
    if now != scope {
        return list::update(app, ListMessage::Open(scope.app));
    }
    let form = app.app_providers.icons.as_mut().unwrap();
    form.busy = true;
    form.notice = None;
    app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
    let generation = app.app_providers.generation;
    let target = scope.app;
    Task::perform(
        async move {
            match operation {
                Some(operation) => execute(client, target, id, operation).await,
                None => pick().await,
            }
        },
        move |result| message(Event::Completed { scope: scope.clone(), generation, result }),
    )
}
