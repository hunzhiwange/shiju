//! Native provider formats and guarded file adapters.
pub use app_provider_validation::validate_identity;
pub use app_provider_core::basic;
pub(crate) use app_provider_projection::opencode_schema;
pub use app_auth::providers::live;
pub mod hermes;

fn uses_retired_bedrock(declaration: &serde_json::Value) -> bool {
    let retired_api = |node: &serde_json::Value| node["api"] == "bedrock-converse-stream";
    retired_api(declaration)
        || matches!(
            declaration["npm"].as_str(),
            Some("@ai-sdk/amazon-bedrock" | "@ai-sdk/amazon-bedrock/mantle")
        )
        || declaration["models"]
            .as_array()
            .is_some_and(|models| models.iter().any(retired_api))
}

#[cfg(test)]
mod hermes_tests;
pub mod openclaw;
#[cfg(test)]
mod openclaw_tests;
pub mod opencode;
#[cfg(test)]
mod opencode_tests;
pub mod pi;
#[cfg(test)]
mod pi_tests;
pub mod omp;
#[cfg(test)]
mod omp_tests;
pub mod mcode;
#[cfg(test)]
mod mcode_tests;
