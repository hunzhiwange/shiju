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
    claude, codex, gemini, grok, hermes,
    list::{ListMessage, current},
    omp, openclaw, opencode, pi,
};
use crate::app::state::app_providers::claude::Draft;
use crate::app::state::app_providers::edit::Mode;
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        basic::ProviderBasic,
        catalog::ProviderMembership,
        claude::{ClaudeFormView, ClaudeLiveResult},
        codex_catalog::{CodexFormView, CodexLiveResult},
        gemini::{GeminiFormView, GeminiLiveResult},
        grok::{GrokFormView, GrokLiveResult},
        hermes::{HermesFormView, HermesLiveResult},
        omp::{OmpFormView, OmpLiveResult},
        openclaw::{OpenClawFormView, OpenClawLiveResult},
        opencode::{OpenCodeFormView, OpenCodeLiveResult},
        pi::{PiFormView, PiLiveResult},
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
    CodexLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<CodexFormView, String>,
    },
    Codex(codex::Field),
    GeminiLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<GeminiFormView, String>,
    },
    Gemini(gemini::Field),
    GrokLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<GrokFormView, String>,
    },
    Grok(grok::Field),
    HermesLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<HermesFormView, String>,
    },
    PiLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<PiFormView, String>,
    },
    OmpLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<OmpFormView, String>,
    },
    OpenClawLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<OpenClawFormView, String>,
    },
    OpenCodeLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<OpenCodeFormView, String>,
    },
    OpenCode(opencode::Field),
    Hermes(hermes::Field),
    Pi(pi::Field),
    Omp(omp::Field),
    OpenClaw(openclaw::Field),
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
    CodexSaved {
        scope: Scope,
        generation: u64,
        result: Result<CodexLiveResult, String>,
    },
    GeminiSaved {
        scope: Scope,
        generation: u64,
        result: Result<GeminiLiveResult, String>,
    },
    GrokSaved {
        scope: Scope,
        generation: u64,
        result: Result<GrokLiveResult, String>,
    },
    HermesSaved {
        scope: Scope,
        generation: u64,
        result: Result<HermesLiveResult, String>,
    },
    PiSaved {
        scope: Scope,
        generation: u64,
        result: Result<PiLiveResult, String>,
    },
    OmpSaved {
        scope: Scope,
        generation: u64,
        result: Result<OmpLiveResult, String>,
    },
    OpenClawSaved {
        scope: Scope,
        generation: u64,
        result: Result<OpenClawLiveResult, String>,
    },
    OpenCodeSaved {
        scope: Scope,
        generation: u64,
        result: Result<OpenCodeLiveResult, String>,
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
            } else if app.app_providers.scope.as_ref().is_some_and(|s| s.app == AppId::Codex) {
                form.codex = Some(crate::app::state::app_providers::codex::Draft::new());
            } else if app
                .app_providers
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::Gemini)
            {
                form.gemini = Some(crate::app::state::app_providers::gemini::Draft::new());
            } else if app
                .app_providers
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::GrokBuild)
            {
                form.grok = Some(crate::app::state::app_providers::grok::Draft::new());
            } else if app
                .app_providers
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::OpenCode)
            {
                form.opencode = Some(crate::app::state::app_providers::opencode::Draft::new());
            } else if app
                .app_providers
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::Hermes)
            {
                form.hermes = Some(crate::app::state::app_providers::hermes::Draft::new());
            } else if app.app_providers.scope.as_ref().is_some_and(|scope| scope.app == AppId::Pi) {
                form.pi = Some(crate::app::state::app_providers::pi::Draft::new());
            } else if app.app_providers.scope.as_ref().is_some_and(|scope| scope.app == AppId::Omp)
            {
                form.omp = Some(crate::app::state::app_providers::omp::Draft::new());
            } else if app
                .app_providers
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::OpenClaw)
            {
                form.openclaw = Some(crate::app::state::app_providers::openclaw::Draft::new());
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
            if scope.app == AppId::Codex && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { codex::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::CodexLoaded {
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
            if scope.app == AppId::GrokBuild && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { grok::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::GrokLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Hermes && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { hermes::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::HermesLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Pi && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { pi::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::PiLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::Omp && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { omp::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::OmpLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::OpenClaw && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { openclaw::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::OpenClawLoaded {
                            scope: scope.clone(),
                            generation,
                            id: id.clone(),
                            result,
                        })
                    },
                );
            }
            if scope.app == AppId::OpenCode && !copy && !delete {
                let token = app.apps_catalog.token.clone();
                let request_scope = scope.clone();
                return Task::perform(
                    async move { opencode::load(client, request_scope, &source, &token).await },
                    move |result| {
                        message(EditMessage::OpenCodeLoaded {
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
        EditMessage::CodexLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.codex =
                        view.config.map(crate::app::state::app_providers::codex::Draft::from_view);
                    form.codex_config_unavailable = form.codex.is_none();
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::GrokLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.grok =
                        view.config.map(crate::app::state::app_providers::grok::Draft::from_view);
                    form.grok_config_unavailable = form.grok.is_none();
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
        EditMessage::HermesLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        hermes::membership(&app.app_providers, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.hermes = view.config.map(|config| {
                        crate::app::state::app_providers::hermes::Draft::from_view(config, member)
                    });
                    form.hermes_config_unavailable = form.hermes.is_none();
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::PiLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        pi::membership(&app.app_providers, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.pi = view.config.map(|config| {
                        crate::app::state::app_providers::pi::Draft::from_view(config, member)
                    });
                    form.pi_config_unavailable = form.pi.is_none();
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::OmpLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        omp::membership(&app.app_providers, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.omp = view.config.map(|config| {
                        crate::app::state::app_providers::omp::Draft::from_view(config, member)
                    });
                    form.omp_config_unavailable = form.omp.is_none();
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::OpenClawLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let member = openclaw::membership(&app.app_providers, &id)
                        == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.openclaw = view.config.map(|config| {
                        crate::app::state::app_providers::openclaw::Draft::from_view(config, member)
                    });
                    form.openclaw_config_unavailable = form.openclaw.is_none();
                    app.app_providers.form = Some(form);
                }
                Err(error) => return app.show_error_toast(error),
            }
        }
        EditMessage::OpenCodeLoaded { scope, generation, id, result } => {
            if !app.app_providers.loading || !matches_scope(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.loading = false;
            match result {
                Ok(view) => {
                    let member = opencode::membership(&app.app_providers, &id)
                        == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.opencode = view.config.map(|config| {
                        crate::app::state::app_providers::opencode::Draft::from_view(config, member)
                    });
                    form.opencode_config_unavailable = form.opencode.is_none();
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
            let codex = form.codex.is_some();
            let grok = form.grok.is_some();
            let opencode = form.opencode.is_some();
            let hermes = form.hermes.is_some();
            let pi = form.pi.is_some();
            let omp = form.omp.is_some();
            let openclaw = form.openclaw.is_some();
            if claude || gemini || codex || grok || opencode || openclaw || hermes || pi || omp {
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
            if codex {
                return Task::perform(
                    codex::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::CodexSaved {
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
            if grok {
                return Task::perform(
                    grok::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::GrokSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if hermes {
                return Task::perform(
                    hermes::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::HermesSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if pi {
                return Task::perform(
                    pi::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::PiSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if omp {
                return Task::perform(
                    omp::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::OmpSaved { scope: scope.clone(), generation, result })
                    },
                );
            }
            if openclaw {
                return Task::perform(
                    openclaw::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::OpenClawSaved {
                            scope: scope.clone(),
                            generation,
                            result,
                        })
                    },
                );
            }
            if opencode {
                return Task::perform(
                    opencode::save(client, scope.clone(), form, token),
                    move |result| {
                        message(EditMessage::OpenCodeSaved {
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
        EditMessage::CodexSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !codex::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(CodexLiveResult::NotAttempted) => app.show_success_toast("已保存到供应商目录。"),
                Ok(CodexLiveResult::Applied) => app.show_success_toast("已保存并应用 Codex 配置。"),
                Ok(CodexLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 Codex 配置失败。")
                }
                Ok(CodexLiveResult::RecoveryRequired) => {
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
        EditMessage::GrokSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !grok::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(GrokLiveResult::NotAttempted) => app.show_success_toast("已保存到供应商目录。"),
                Ok(GrokLiveResult::Applied) => app.show_success_toast("已保存并应用 Grok 配置。"),
                Ok(GrokLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 Grok 配置失败。")
                }
                Ok(GrokLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::HermesSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !hermes::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(HermesLiveResult::NotAttempted) => {
                    app.show_success_toast("已保存到供应商目录。")
                }
                Ok(HermesLiveResult::Applied) => {
                    app.show_success_toast("已保存并应用 Hermes 配置。")
                }
                Ok(HermesLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 Hermes 配置失败。")
                }
                Ok(HermesLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::PiSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !pi::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(PiLiveResult::NotAttempted) => app.show_success_toast("已保存到供应商目录。"),
                Ok(PiLiveResult::Applied) => app.show_success_toast("已保存并应用 Pi 配置。"),
                Ok(PiLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 Pi 配置失败。")
                }
                Ok(PiLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::OmpSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !omp::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(OmpLiveResult::NotAttempted) => app.show_success_toast("已保存到供应商目录。"),
                Ok(OmpLiveResult::Applied) => app.show_success_toast("已保存并应用 OMP 配置。"),
                Ok(OmpLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 OMP 配置失败。")
                }
                Ok(OmpLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::OpenClawSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !openclaw::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(OpenClawLiveResult::NotAttempted) => {
                    app.show_success_toast("已保存到供应商目录。")
                }
                Ok(OpenClawLiveResult::Applied) => {
                    app.show_success_toast("已保存并应用 OpenClaw 配置。")
                }
                Ok(OpenClawLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 OpenClaw 配置失败。")
                }
                Ok(OpenClawLiveResult::RecoveryRequired) => {
                    app.show_warning_toast("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, super::list::update(app, ListMessage::Refresh)]);
        }
        EditMessage::OpenCodeSaved { scope, generation, result } => {
            if !matches_scope(app, &scope, generation)
                || !opencode::finish(&mut app.app_providers, &scope, generation, &result)
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return app.show_error_toast(error),
                Ok(OpenCodeLiveResult::NotAttempted) => {
                    app.show_success_toast("已保存到供应商目录。")
                }
                Ok(OpenCodeLiveResult::Applied) => {
                    app.show_success_toast("已保存并应用 OpenCode 配置。")
                }
                Ok(OpenCodeLiveResult::Failed) => {
                    app.show_warning_toast("目录已保存，但应用 OpenCode 配置失败。")
                }
                Ok(OpenCodeLiveResult::RecoveryRequired) => {
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
                EditMessage::Codex(field) => codex::change(form, field),
                EditMessage::Gemini(field) => gemini::change(form, field),
                EditMessage::Grok(field) => grok::change(form, field),
                EditMessage::Hermes(field) => hermes::change(form, field),
                EditMessage::Pi(field) => pi::change(form, field),
                EditMessage::Omp(field) => omp::change(form, field),
                EditMessage::OpenClaw(field) => openclaw::change(form, field),
                EditMessage::OpenCode(field) => opencode::change(form, field),
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
