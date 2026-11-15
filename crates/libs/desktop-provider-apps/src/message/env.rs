use crate::Scope;
use crate::state::env::Action;
use gateway_http::AgentAppsReadError;
use vw_api_app_provider_environment::{env_diagnose::EnvDiagnosis,env_clean::EnvCleanResult,env_restore::EnvRestoreResult};
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Toggle(usize, bool),
    BackupId(String),
    Review(Action),
    Cancel,
    Confirm,
    Completed { scope: Scope, generation: u64, result: Result<Outcome, AgentAppsReadError> },
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderEnvironmentEvent([redacted])")
    }
}

#[derive(Clone)]
pub enum Outcome {
    Diagnosis(EnvDiagnosis),
    Cleaned(EnvCleanResult),
    Restored(EnvRestoreResult),
}

pub fn message(value: Event) -> crate::Message { crate::Message::Env(value) }
