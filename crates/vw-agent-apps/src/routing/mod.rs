pub mod compaction_replay;

#[cfg(test)]
mod compaction_replay_tests;

pub mod lifecycle_state;

#[cfg(test)]
mod lifecycle_state_tests;

#[cfg(test)]
mod override_validate_tests;

pub mod private_params;

#[cfg(test)]
mod private_params_tests;

#[cfg(test)]
mod user_agent_tests;

/// Applications that can use the gateway's application routing data plane.
pub fn supports_app(app: vw_api_types::agent_apps::AppId) -> bool {
    use vw_api_types::agent_apps::AppId::{Claude, Codex, Gemini, GrokBuild};
    matches!(app, Claude | Codex | Gemini | GrokBuild)
}

pub mod target_model;
#[cfg(test)]
mod target_model_tests;

pub mod gemini_schema;

pub mod moonshot_schema;

#[cfg(test)]
mod moonshot_schema_tests;

pub mod inline_think;
#[cfg(test)]
mod inline_think_tests;

pub mod thinking_options;
#[cfg(test)]
mod thinking_options_tests;

pub mod classifier_guard;
#[cfg(test)]
mod classifier_guard_tests;

pub mod budget_rectify;

pub mod media_sanitize;
