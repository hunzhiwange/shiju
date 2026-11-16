use crate::Scope;
use super::{agy,claude,codex,dsh,gemini,grok,hermes,kimi,mcode,omp,openclaw,opencode,pi};
use vw_api_app_provider_base::basic::ProviderBasic;
use vw_api_app_provider_edit::catalog::ProviderMembership;
use vw_api_app_provider_base::claude::ClaudeFormView;
use vw_api_app_provider_base::claude::ClaudeLiveResult;
use vw_api_app_provider_profiles::codex_catalog::CodexFormView;
use vw_api_app_provider_profiles::codex_catalog::CodexLiveResult;
use vw_api_app_provider_terminal::dsh::DshFormView;
use vw_api_app_provider_terminal::dsh::DshLiveResult;
use vw_api_app_provider_profiles::gemini::GeminiFormView;
use vw_api_app_provider_profiles::gemini::GeminiLiveResult;
use vw_api_app_provider_extended::grok::GrokFormView;
use vw_api_app_provider_extended::grok::GrokLiveResult;
use vw_api_app_provider_extended::hermes::HermesFormView;
use vw_api_app_provider_extended::hermes::HermesLiveResult;
use vw_api_app_provider_terminal::kimi::KimiFormView;
use vw_api_app_provider_terminal::kimi::KimiLiveResult;
use vw_api_app_provider_terminal::mcode::McodeFormView;
use vw_api_app_provider_terminal::mcode::McodeLiveResult;
use vw_api_app_provider_terminal::omp::OmpFormView;
use vw_api_app_provider_terminal::omp::OmpLiveResult;
use vw_api_app_provider_extended::openclaw::OpenClawFormView;
use vw_api_app_provider_extended::openclaw::OpenClawLiveResult;
use vw_api_app_provider_extended::opencode::OpenCodeFormView;
use vw_api_app_provider_extended::opencode::OpenCodeLiveResult;
use vw_api_app_provider_terminal::pi::PiFormView;
use vw_api_app_provider_terminal::pi::PiLiveResult;
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
    KimiLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<KimiFormView, String>,
    },
    McodeLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<McodeFormView, String>,
    },
    AgyLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<vw_api_app_provider_profiles::agy::AgyFormView, String>,
    },
    Agy(agy::Field),
    AgySaved {
        scope: Scope,
        generation: u64,
        result: Result<(), String>,
    },
    DshLoaded {
        scope: Scope,
        generation: u64,
        id: String,
        result: Result<DshFormView, String>,
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
    Kimi(kimi::Field),
    Mcode(mcode::Field),
    Dsh(dsh::Field),
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
    KimiSaved {
        scope: Scope,
        generation: u64,
        result: Result<KimiLiveResult, String>,
    },
    McodeSaved {
        scope: Scope,
        generation: u64,
        result: Result<McodeLiveResult, String>,
    },
    DshSaved {
        scope: Scope,
        generation: u64,
        result: Result<DshLiveResult, String>,
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

pub fn message(value: EditMessage) -> crate::Message { crate::Message::Edit(value) }

use crate::state::edit::EditForm;

use vw_api_app_provider_edit::delete::AppProviderDelete;
pub async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<(), String> {
    if matches!(form.mode, crate::state::edit::Mode::Delete) {
        let deleted = if scope.app == vw_api_app_identity::AppId::Agy {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
                .agy_form_delete(
                    &form.id,
                    &vw_api_app_provider_profiles::agy::AgyTarget {
                        target_host_id: scope.host,
                    },
                    &token,
                )
                .await
                .map_err(|_| "Agy 档案未删除：请先关闭运行中的终端并检查引用。".to_string())?
        } else {
            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_delete(
                    scope.app,
                    &form.id,
                    &AppProviderDelete { target_host_id: scope.host },
                    &token,
                )
                .await?
        };
        if !deleted.deleted {
            return Err("供应商未删除".into());
        }
    } else {
        gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_basic_write(scope.app, &form.id, &form.request()?).await?;
    }
    Ok(())
}

use vw_gateway_client::GatewayClient;
#[path = "edit_update.rs"]
mod update_impl;
pub use update_impl::update;

/// Apply a provider-specific field without executing host effects.
pub fn change(state: &mut super::provider_update::State, field: &EditMessage) -> bool {
    if !matches!(
        field,
        EditMessage::Claude(_)
            | EditMessage::Codex(_)
            | EditMessage::Gemini(_)
            | EditMessage::Grok(_)
            | EditMessage::Hermes(_)
            | EditMessage::Pi(_)
            | EditMessage::Kimi(_)
            | EditMessage::Mcode(_)
            | EditMessage::Agy(_)
            | EditMessage::Dsh(_)
            | EditMessage::Omp(_)
            | EditMessage::OpenClaw(_)
            | EditMessage::OpenCode(_)
    ) {
        return false;
    }
    let Some(form) = state.form.as_mut().filter(|f| !f.saving) else { return true };
    use EditMessage as E;
    match field.clone() {
        E::Claude(field) => super::claude::change(form, field),
        E::Codex(field) => super::codex::change(form, field),
        E::Gemini(field) => super::gemini::change(form, field),
        E::Grok(field) => super::grok::change(form, field),
        E::Hermes(field) => super::hermes::change(form, field),
        E::Pi(field) => super::pi::change(form, field),
        E::Kimi(field) => super::kimi::change(form, field),
        E::Mcode(field) => super::mcode::change(form, field),
        E::Agy(field) => super::agy::change(form, field),
        E::Dsh(field) => super::dsh::change(form, field),
        E::Omp(field) => super::omp::change(form, field),
        E::OpenClaw(field) => super::openclaw::change(form, field),
        E::OpenCode(field) => super::opencode::change(form, field),
        _ => {}
    }

    true
}
