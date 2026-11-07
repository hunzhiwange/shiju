use crate::app::state::app_providers::{edit::EditForm, list::Scope};
use vw_gateway_client::{GatewayClient, vw_api_types::app_providers::delete::AppProviderDelete};
pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<(), String> {
    if matches!(form.mode, crate::app::state::app_providers::edit::Mode::Delete) {
        let deleted = client
            .app_provider_delete(
                scope.app,
                &form.id,
                &AppProviderDelete { target_host_id: scope.host },
                &token,
            )
            .await?;
        if !deleted.deleted {
            return Err("供应商未删除".into());
        }
    } else {
        client.app_provider_basic_write(scope.app, &form.id, &form.request()?).await?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;

use super::{
    claude, gemini,
    list::{ListMessage, current},
};
use crate::app::state::app_providers::claude::Draft;
use crate::app::state::app_providers::edit::Mode;
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        basic::ProviderBasic,
        claude::{ClaudeFormView, ClaudeLiveResult},
        gemini::{GeminiFormView, GeminiLiveResult},
    },
};
#[derive(Clone)]
pub enum EditMessage {
    Create,
    Open {
        id: String,
        copy: bool,
        delete: bool,
    },
    Loaded {
        scope: Scope,
        generation: u64,
        id: String,
        copy: bool,
        delete: bool,
        result: Result<ProviderBasic, String>,
    },
    ClaudeLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<ClaudeFormView, String>,
    },
    Claude(claude::Field),
    GeminiLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<GeminiFormView, String>,
    },
    Gemini(gemini::Field),
    Id(String),
    Name(String),
    Category(String),
    Notes(String),
    Website(String),
    Key(String),
    Replace(bool),
    Clear(bool),
    Cancel,
    Save,
    ClaudeSaved {
        scope: Scope,
        generation: u64,
        result: Result<ClaudeLiveResult, String>,
    },
    GeminiSaved {
        scope: Scope,
        generation: u64,
        result: Result<GeminiLiveResult, String>,
    },
    Saved {
        scope: Scope,
        generation: u64,
        result: Result<(), String>,
    },
}
impl std::fmt::Debug for EditMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditMessage([redacted])")
    }
}
pub(crate) fn message(value: EditMessage) -> Message {
    Message::Settings(SettingsMessage::AppProviderEdit(value))
}
pub(crate) fn update(app: &mut App, event: EditMessage) -> Task<Message> {
    match event {
        EditMessage::Create => {
            if app.app_providers.loading || app.app_providers.form.is_some() {
                return Task::none();
            }
            let mut form = EditForm::new(
                Mode::Create,
                String::new(),
                ProviderBasic {
                    name: String::new(),
                    category: None,
                    notes: None,
                    website_url: None,
                },
                app.app_providers.new_draft.clone(),
            );
            if claude_scope(app) {
                form.claude = Some(Draft::new());
            } else if app
                .app_providers
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::Gemini)
            {
                form.gemini = Some(crate::app::state::app_providers::gemini::Draft::new());
            }
            app.app_providers.form = Some(form);
        }
        EditMessage::Open { id, copy, delete } => {
            let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
            if app.app_providers.loading || app.app_providers.form.is_some() {
                return Task::none();
            }
            let Ok((client, now)) = current(app, scope.app) else { return Task::none() };
            if scope != now {
                return Task::none();
            }
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            let generation = app.app_providers.generation;
            app.app_providers.loading = true;
            let source = id.clone();
            if claude_scope(app) && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { claude::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::ClaudeLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Gemini && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { gemini::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::GeminiLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            return Task::perform(
                async move { client.app_provider_basic(scope.app, &source).await },
                move |result| {
                    message(EditMessage::Loaded {
                        scope: scope.clone(),
                        generation,
                        id: id.clone(),
                        copy,
                        delete,
                        result,
                    })
                },
            );
        }
        EditMessage::Loaded { scope, generation, id, copy, delete, result } => {
            if !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(mut fields) => {
                    let mode = if delete {
                        Mode::Delete
                    } else if copy {
                        fields.name.push_str(" 副本");
                        Mode::Copy { source_id: id.clone() }
                    } else {
                        Mode::Update
                    };
                    let target = if copy { String::new() } else { id };
                    app.app_providers.form = Some(EditForm::new(mode, target, fields, None));
                }
                Err(e) => return app.show_error_toast(e),
            }
        }
        EditMessage::ClaudeLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.claude = Some(Draft::from_view(view.config, view.model));
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::GeminiLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.gemini = Some(crate::app::state::app_providers::gemini::Draft::from_view(
                        view.config,
                        view.model,
                    ));
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::Cancel => {
            if app.app_providers.form.as_ref().is_some_and(|f| !f.saving) {
                app.app_providers.form = None;
            }
        }
        EditMessage::Save => {
            let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
            let Ok((client, now)) = current(app, scope.app) else { return Task::none() };
            if scope != now {
                return app.show_error_toast("执行主机或工作区已切换，请重新打开");
            }
            let Some(form) = app.app_providers.form.as_mut() else { return Task::none() };
            if form.saving {
                return Task::none();
            }
            let claude = form.claude_editable();
            let gemini = form.gemini_editable();
            if claude || gemini {
                if !form.identified() {
                    return app.show_error_toast("请填写标识与名称");
                }
            } else if !matches!(form.mode, Mode::Delete) {
                if let Err(e) = form.request() {
                    return app.show_error_toast(e);
                }
            }
            form.saving = true;
            let form = form.clone();
            let token = app.apps_catalog.token.clone();
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            let generation = app.app_providers.generation;
            if claude {
                return Task::perform(
                    claude::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::ClaudeSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if gemini {
                return Task::perform(
                    gemini::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::GeminiSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            return Task::perform(save(client, scope.clone(), form, token), move |result| {
                message(EditMessage::Saved { scope: scope.clone(), generation, result })
            });
        }
        EditMessage::ClaudeSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !claude::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(ClaudeLiveResult::NotAttempted) => {
                    app.show_success_toast("已保存到供应商目录。")
                }
                Ok(ClaudeLiveResult::Applied) => {
                    app.show_success_toast("已保存并应用 Claude 配置。")
                }
                Ok(ClaudeLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 Claude 配置失败。")
                }
                Ok(ClaudeLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::GeminiSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !gemini::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(GeminiLiveResult::NotAttempted) => {
                    app.show_success_toast("已保存到供应商目录。")
                }
                Ok(GeminiLiveResult::Applied) => {
                    app.show_success_toast("已保存并应用 Gemini 配置。")
                }
                Ok(GeminiLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 Gemini 配置失败。")
                }
                Ok(GeminiLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::Saved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            let Some(form) = app.app_providers.form.as_mut() else { return Task::none() };
            if !form.saving {
                return Task::none();
            }
            if form.finish(&result) {
                app.app_providers.form = None;
                return super::list::update(app, ListMessage::Refresh);
            }
            if let Err(e) = result {
                return app.show_error_toast(e);
            }
        }
        field => {
            let Some(form) = app.app_providers.form.as_mut().filter(|f| !f.saving) else {
                return Task::none();
            };
            let optional = |v: String| (!v.is_empty()).then_some(v);
            match field {
                EditMessage::Id(v) if matches!(form.mode, Mode::Create | Mode::Copy { .. }) => {
                    form.id = v
                }
                EditMessage::Claude(field) => claude::change(form, field),
                EditMessage::Gemini(field) => gemini::change(form, field),
                EditMessage::Name(v) => form.fields.name = v,
                EditMessage::Category(v) => form.fields.category = optional(v),
                EditMessage::Notes(v) => form.fields.notes = optional(v),
                EditMessage::Website(v) => form.fields.website_url = optional(v),
                EditMessage::Key(v) => form.key = v,
                EditMessage::Replace(v) => {
                    form.replace_key = v;
                    form.clear_key = false;
                    form.key.clear();
                }
                EditMessage::Clear(v) => {
                    form.clear_key = v;
                    form.replace_key = false;
                    form.key.clear();
                }
                _ => {}
            }
        }
    }
    Task::none()
}
fn matches_scope(app: &App, scope: &Scope, generation: u64) -> bool {
    app.app_providers.scope.as_ref() == Some(scope)
        && app.app_providers.generation == generation
        && current(app, scope.app).ok().map(|(_, s)| s).as_ref() == Some(scope)
}

// The basic editor's isolated fixture (and composed drafts) supply their own complete profile.
fn claude_scope(app: &App) -> bool {
    app.app_providers.scope.as_ref().is_some_and(|scope| scope.app == AppId::Claude)
        && app.app_providers.new_draft.is_none()
}
