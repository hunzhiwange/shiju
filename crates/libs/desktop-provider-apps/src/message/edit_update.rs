use super::*;
use crate::message::provider_update::{State, Effect as Message};
use crate::state::{claude::Draft,edit::Mode};
use iced::Task;
use vw_api_types::agent_apps::AppId;
fn membership(snapshot: Option<&crate::state::list::Snapshot>, id: &str) -> vw_api_app_provider_edit::catalog::ProviderMembership {
 snapshot.and_then(|s|s.catalog.rows.iter().find(|r|r.provider.id==id)).map_or(vw_api_app_provider_edit::catalog::ProviderMembership::Unknown, |r|r.native_member)
}
enum SavedProvider { Agy, Claude, Codex, Gemini, Grok, Hermes, Pi, Kimi, Mcode, Dsh, Omp, OpenClaw, OpenCode }
fn finish_provider(state: &mut State, provider: SavedProvider, success: bool) -> bool {
 let Some(form)=state.form.as_mut().filter(|f|f.saving) else {return false};
 let present=match provider {
 SavedProvider::Agy => form.agy.is_some(),
 SavedProvider::Claude => form.claude_editable(),
 SavedProvider::Codex => form.codex.is_some(),
 SavedProvider::Gemini => form.gemini_editable(),
 SavedProvider::Grok => form.grok.is_some(),
 SavedProvider::Hermes => form.hermes.is_some(),
 SavedProvider::Pi => form.pi.is_some(),
 SavedProvider::Kimi => form.kimi.is_some(),
 SavedProvider::Mcode => form.mcode.is_some(),
 SavedProvider::Dsh => form.dsh.is_some(),
 SavedProvider::Omp => form.omp.is_some(),
 SavedProvider::OpenClaw => form.openclaw.is_some(),
 SavedProvider::OpenCode => form.opencode.is_some(),
 };
 if !present {return false}; form.saving=false; if success {state.form=None}; true
}
pub fn update(state: &mut State, current: Option<(GatewayClient, Scope)>, token: &str, snapshot: Option<&crate::state::list::Snapshot>, event: EditMessage) -> Task<Message> {
    match event {
        EditMessage::Create => {
            if state.loading || state.form.is_some() {
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
                state.new_draft.clone(),
            );
            if state.scope.as_ref().is_some_and(|s| s.app == AppId::Agy) {
                form.agy = Some(crate::state::agy::Draft::new());
            } else if claude_scope(state) {
                form.claude = Some(Draft::new());
            } else if state.scope.as_ref().is_some_and(|s| s.app == AppId::Codex) {
                form.codex = Some(crate::state::codex::Draft::new());
            } else if state
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::Gemini)
            {
                form.gemini = Some(crate::state::gemini::Draft::new());
            } else if state
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::GrokBuild)
            {
                form.grok = Some(crate::state::grok::Draft::new());
            } else if state
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::OpenCode)
            {
                form.opencode = Some(crate::state::opencode::Draft::new());
            } else if state
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::Hermes)
            {
                form.hermes = Some(crate::state::hermes::Draft::new());
            } else if state.scope.as_ref().is_some_and(|scope| scope.app == AppId::Pi) {
                form.pi = Some(crate::state::pi::Draft::new());
            } else if state
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::Mcode)
            {
                form.mcode = Some(crate::state::mcode::Draft::new());
            } else if state.scope.as_ref().is_some_and(|scope| scope.app == AppId::Dsh)
            {
                form.dsh = Some(crate::state::dsh::Draft::new());
            } else if state.scope.as_ref().is_some_and(|scope| scope.app == AppId::Kimi)
            {
                form.kimi = Some(crate::state::kimi::Draft::new());
            } else if state.scope.as_ref().is_some_and(|scope| scope.app == AppId::Omp)
            {
                form.omp = Some(crate::state::omp::Draft::new());
            } else if state
                .scope
                .as_ref()
                .is_some_and(|scope| scope.app == AppId::OpenClaw)
            {
                form.openclaw = Some(crate::state::openclaw::Draft::new());
            }
            state.form = Some(form);
        }
        EditMessage::Open { id, copy, delete } => {
            let Some(scope) = state.scope.clone() else { return Task::none() };
            if state.loading || state.form.is_some() {
                return Task::none();
            }
            let Ok((client, now)) = current.clone().ok_or(()) else { return Task::none() };
            if scope != now {
                return Task::none();
            }
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            state.loading = true;
            return Task::done(Message::Load { client, scope, generation, id, copy, delete, claude: claude_scope(state), token: token.to_owned() });
        }
        EditMessage::Loaded { scope, generation, id, copy, delete, result } => {
            if !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
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
                    state.form = Some(EditForm::new(mode, target, fields, None));
                }
                Err(e) => return crate::message::provider_update::error(e),
            }
        }
        EditMessage::ClaudeLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.claude = Some(Draft::from_view(view.config, view.model));
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::CodexLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.codex =
                        view.config.map(crate::state::codex::Draft::from_view);
                    form.codex_config_unavailable = form.codex.is_none();
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        form.codex_can_bind = view.can_bind;
                    }
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::GrokLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.grok =
                        view.config.map(crate::state::grok::Draft::from_view);
                    form.grok_config_unavailable = form.grok.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::GeminiLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.gemini = Some(crate::state::gemini::Draft::from_view(
                        view.config,
                        view.model,
                    ));
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::HermesLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        membership(snapshot, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.hermes = view.config.map(|config| {
                        crate::state::hermes::Draft::from_view(config, member)
                    });
                    form.hermes_config_unavailable = form.hermes.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::PiLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        membership(snapshot, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.pi = view.config.map(|config| {
                        crate::state::pi::Draft::from_view(config, member)
                    });
                    form.pi_config_unavailable = form.pi.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::KimiLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        membership(snapshot, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.kimi = view.config.map(|config| {
                        crate::state::kimi::Draft::from_view(config, member)
                    });
                    form.kimi_config_unavailable = form.kimi.is_none();
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        form.kimi_auth_region = view.auth_region;
                    }
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::McodeLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        membership(snapshot, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.mcode = view.config.map(|config| {
                        crate::state::mcode::Draft::from_view(config, member)
                    });
                    form.mcode_config_unavailable = form.mcode.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::AgyLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let draft = crate::state::agy::Draft::from_view(&view);
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.agy = Some(draft);
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::AgySaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Agy, result.is_ok())
            {
                return Task::none();
            }
            if let Err(error) = result {
                return crate::message::provider_update::error(error);
            }
            return Task::batch([
                crate::message::provider_update::success("已保存档案，下次授权终端启动时生效。"),
                crate::message::provider_update::refresh(state),
            ]);
        }
        EditMessage::DshLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        membership(snapshot, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.dsh = view.config.map(|config| {
                        crate::state::dsh::Draft::from_view(config, member)
                    });
                    form.dsh_config_unavailable = form.dsh.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::OmpLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member =
                        membership(snapshot, &id) == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.omp = view.config.map(|config| {
                        crate::state::omp::Draft::from_view(config, member)
                    });
                    form.omp_config_unavailable = form.omp.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::OpenClawLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member = membership(snapshot, &id)
                        == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.openclaw = view.config.map(|config| {
                        crate::state::openclaw::Draft::from_view(config, member)
                    });
                    form.openclaw_config_unavailable = form.openclaw.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::OpenCodeLoaded { scope, generation, id, result } => {
            if !state.loading || !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(view) => {
                    let member = membership(snapshot, &id)
                        == ProviderMembership::Present;
                    let mut form = EditForm::new(Mode::Update, id, view.fields, None);
                    form.opencode = view.config.map(|config| {
                        crate::state::opencode::Draft::from_view(config, member)
                    });
                    form.opencode_config_unavailable = form.opencode.is_none();
                    state.form = Some(form);
                }
                Err(error) => return crate::message::provider_update::error(error),
            }
        }
        EditMessage::Cancel => {
            if state.form.as_ref().is_some_and(|f| !f.saving) {
                state.form = None;
            }
        }
        EditMessage::Save => {
            let Some(scope) = state.scope.clone() else { return Task::none() };
            let Ok((client, now)) = current.clone().ok_or(()) else { return Task::none() };
            if scope != now {
                return crate::message::provider_update::error("执行主机或工作区已切换，请重新打开");
            }
            let Some(form) = state.form.as_mut() else { return Task::none() };
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
            let kimi = form.kimi.is_some();
            let mcode = form.mcode.is_some();
            let agy = form.agy.is_some();
            let dsh = form.dsh.is_some();
            let omp = form.omp.is_some();
            let openclaw = form.openclaw.is_some();
            if claude
                || gemini
                || codex
                || grok
                || opencode
                || openclaw
                || hermes
                || pi
                || omp
                || kimi
                || mcode
                || dsh
                || agy
            {
                if !form.identified() {
                    return crate::message::provider_update::error("请填写标识与名称");
                }
            } else if !matches!(form.mode, Mode::Delete) {
                if let Err(e) = form.request() {
                    return crate::message::provider_update::error(e);
                }
            }
            form.saving = true;
            let form = form.clone();
            let token = token.to_owned();
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            return Task::done(Message::Save { client, scope, generation, form, token });
        }
        EditMessage::ClaudeSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Claude, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(ClaudeLiveResult::NotAttempted) => {
                    crate::message::provider_update::success("已保存到供应商目录。")
                }
                Ok(ClaudeLiveResult::Applied) => {
                    crate::message::provider_update::success("已保存并应用 Claude 配置。")
                }
                Ok(ClaudeLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Claude 配置失败。")
                }
                Ok(ClaudeLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::CodexSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Codex, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(CodexLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(CodexLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 Codex 配置。"),
                Ok(CodexLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Codex 配置失败。")
                }
                Ok(CodexLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::GeminiSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Gemini, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(GeminiLiveResult::NotAttempted) => {
                    crate::message::provider_update::success("已保存到供应商目录。")
                }
                Ok(GeminiLiveResult::Applied) => {
                    crate::message::provider_update::success("已保存并应用 Gemini 配置。")
                }
                Ok(GeminiLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Gemini 配置失败。")
                }
                Ok(GeminiLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::GrokSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Grok, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(GrokLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(GrokLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 Grok 配置。"),
                Ok(GrokLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Grok 配置失败。")
                }
                Ok(GrokLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::HermesSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Hermes, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(HermesLiveResult::NotAttempted) => {
                    crate::message::provider_update::success("已保存到供应商目录。")
                }
                Ok(HermesLiveResult::Applied) => {
                    crate::message::provider_update::success("已保存并应用 Hermes 配置。")
                }
                Ok(HermesLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Hermes 配置失败。")
                }
                Ok(HermesLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::PiSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Pi, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(PiLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(PiLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 Pi 配置。"),
                Ok(PiLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Pi 配置失败。")
                }
                Ok(PiLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::KimiSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Kimi, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(KimiLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(KimiLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 Kimi 配置。"),
                Ok(KimiLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Kimi 配置失败。")
                }
                Ok(KimiLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::McodeSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Mcode, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(McodeLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(McodeLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 Mcode 配置。"),
                Ok(McodeLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Mcode 配置失败。")
                }
                Ok(McodeLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::DshSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Dsh, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(DshLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(DshLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 Dsh 配置。"),
                Ok(DshLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 Dsh 配置失败。")
                }
                Ok(DshLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::OmpSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::Omp, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(OmpLiveResult::NotAttempted) => crate::message::provider_update::success("已保存到供应商目录。"),
                Ok(OmpLiveResult::Applied) => crate::message::provider_update::success("已保存并应用 OMP 配置。"),
                Ok(OmpLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 OMP 配置失败。")
                }
                Ok(OmpLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::OpenClawSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::OpenClaw, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(OpenClawLiveResult::NotAttempted) => {
                    crate::message::provider_update::success("已保存到供应商目录。")
                }
                Ok(OpenClawLiveResult::Applied) => {
                    crate::message::provider_update::success("已保存并应用 OpenClaw 配置。")
                }
                Ok(OpenClawLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 OpenClaw 配置失败。")
                }
                Ok(OpenClawLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::OpenCodeSaved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation)
                || !finish_provider(state, SavedProvider::OpenCode, result.is_ok())
            {
                return Task::none();
            }
            let toast = match result {
                Err(error) => return crate::message::provider_update::error(error),
                Ok(OpenCodeLiveResult::NotAttempted) => {
                    crate::message::provider_update::success("已保存到供应商目录。")
                }
                Ok(OpenCodeLiveResult::Applied) => {
                    crate::message::provider_update::success("已保存并应用 OpenCode 配置。")
                }
                Ok(OpenCodeLiveResult::Failed) => {
                    crate::message::provider_update::warning("目录已保存，但应用 OpenCode 配置失败。")
                }
                Ok(OpenCodeLiveResult::RecoveryRequired) => {
                    crate::message::provider_update::warning("目录已保存，执行端配置需要恢复。")
                }
            };
            return Task::batch([toast, crate::message::provider_update::refresh(state)]);
        }
        EditMessage::Saved { scope, generation, result } => {
            if !matches_scope(state, &current, &scope, generation) {
                return Task::none();
            }
            let Some(form) = state.form.as_mut() else { return Task::none() };
            if !form.saving {
                return Task::none();
            }
            if form.finish(&result) {
                state.form = None;
                return crate::message::provider_update::refresh(state);
            }
            if let Err(e) = result {
                return crate::message::provider_update::error(e);
            }
        }
        field => {
            let Some(form) = state.form.as_mut().filter(|f| !f.saving) else {
                return Task::none();
            };
            let optional = |v: String| (!v.is_empty()).then_some(v);
            match field {
                EditMessage::Id(v) if matches!(form.mode, Mode::Create | Mode::Copy { .. }) => {
                    form.id = v
                }
                field @ EditMessage::Claude(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Codex(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Gemini(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Grok(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Hermes(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Pi(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Kimi(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Mcode(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Agy(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Dsh(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::Omp(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::OpenClaw(_) => return Task::done(Message::Field(field)),
                field @ EditMessage::OpenCode(_) => return Task::done(Message::Field(field)),
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
fn matches_scope(state: &State, current: &Option<(GatewayClient, Scope)>, scope: &Scope, generation: u64) -> bool {
    state.scope.as_ref() == Some(scope)
        && state.generation == generation
        && current.clone().map(|(_, s)| s).as_ref() == Some(scope)
}

// The basic editor's isolated fixture (and composed drafts) supply their own complete profile.
fn claude_scope(state: &State) -> bool {
    state.scope.as_ref().is_some_and(|scope| scope.app == AppId::Claude)
        && state.new_draft.is_none()
}
