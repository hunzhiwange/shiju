//! Actual host ports for the profile acceptance suite.
use gateway_session::profile_acceptance as suite;
use super::message_ops::Host;
impl gateway_session::profile_acceptance_host::ProfileAcceptanceHost for Host {
    type Fixture = crate::session::llm::recovery_test_support::RecoveryConfig;
    type Admission = crate::session::agent_binding::AdmissionHost;
    async fn persist_turn(session: &str, query: &str, text: &str, agent: &str, error: Option<&str>) -> Result<(String, String), crate::gateway::ApiError> { super::stream::persist_child_agent_turn(session, query, text, agent, error).await }
    fn env_lock() -> &'static tokio::sync::Mutex<()> { crate::test_support::global_async_env_lock() }
    async fn recovery(api: &str, key: Option<&str>) -> Self::Fixture { Self::Fixture::new(api, key).await }
    async fn config() -> anyhow::Result<agent_config::runtime::Config> { Ok(crate::config::get_current().await?) }
    async fn invalidate_cache() { crate::provider::provider::invalidate_cache().await; }
    fn catalog_router() -> axum::Router { super::super::misc::router::<()>() }
    fn config_router() -> axum::Router { super::super::config::router::<()>() }
    async fn chat(query: axum::extract::Query<gateway_instance::host::InstanceQuery>, headers: axum::http::HeaderMap, body: axum::Json<vw_api_chat::chat::GatewayChatStreamRequest>) -> Result<axum::response::Response, crate::gateway::ApiError> {
        use axum::response::IntoResponse;
        super::stream::chat_stream(axum::extract::Query(crate::gateway::instance::InstanceQuery { directory: query.0.directory }), headers, body).await.map(IntoResponse::into_response)
    }
}

#[test]
fn agent_profile_acceptance_runs_selected_profile_end_to_end() { suite::agent_profile_acceptance_runs_selected_profile_end_to_end::<Host>(); }

#[test]
fn acp_onboarding_autohand_builtin_profile_persists_and_runs() { suite::acp_onboarding_autohand_builtin_profile_persists_and_runs::<Host>(); }

#[test]
#[ignore = "requires reviewed DeepAgents runtime and controlled loopback model"]
fn acp_onboarding_deepagents_builtin_profile_persists_and_runs() { suite::acp_onboarding_deepagents_builtin_profile_persists_and_runs::<Host>(); }

#[test]
fn acp_onboarding_dimcode_builtin_profile_persists_and_runs() { suite::acp_onboarding_dimcode_builtin_profile_persists_and_runs::<Host>(); }

#[test]
fn acp_onboarding_dirac_builtin_profile_persists_and_runs() { suite::acp_onboarding_dirac_builtin_profile_persists_and_runs::<Host>(); }

#[test]
fn acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs() { suite::acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs::<Host>(); }

#[test]
fn acp_onboarding_grok_builtin_profile_persists_and_runs() { suite::acp_onboarding_grok_builtin_profile_persists_and_runs::<Host>(); }

#[test]
fn acp_onboarding_nova_builtin_profile_persists_and_runs() { suite::acp_onboarding_nova_builtin_profile_persists_and_runs::<Host>(); }

#[test]
#[ignore = "requires the official Nova package and authorized isolated local consent"]
fn acp_onboarding_nova_real_builtin_profile_persists_and_runs() { suite::acp_onboarding_nova_real_builtin_profile_persists_and_runs::<Host>(); }

#[test]
fn acp_onboarding_sigit_builtin_profile_persists_and_runs() { suite::acp_onboarding_sigit_builtin_profile_persists_and_runs::<Host>(); }

#[test]
#[ignore = "requires the reviewed siGit build and an owned loopback model endpoint"]
fn acp_onboarding_sigit_real_builtin_profile_persists_and_runs() { suite::acp_onboarding_sigit_real_builtin_profile_persists_and_runs::<Host>(); }

#[test]
#[ignore = "requires the reviewed Amp runtime and an owned loopback model fixture"]
fn acp_onboarding_amp_acp_builtin_profile_persists_and_runs() { suite::acp_onboarding_amp_acp_builtin_profile_persists_and_runs::<Host>(); }

#[tokio::test]
async fn agent_profile_acceptance_rejects_revoked_capabilities() { suite::agent_profile_acceptance_rejects_revoked_capabilities::<Host>().await; }

#[tokio::test]
async fn agent_profile_acceptance_catalog_uses_request_tenant() { suite::agent_profile_acceptance_catalog_uses_request_tenant::<Host>().await; }

#[tokio::test]
async fn agent_config_cleanup_global_profile_roundtrips_to_new_session() { suite::agent_config_cleanup_global_profile_roundtrips_to_new_session::<Host>().await; }
