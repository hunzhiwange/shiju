pub(super) fn configure(profile: &mut super::AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    llm_runtime::acp::model_source::drivers::openclaw::configure(
        profile, endpoint, &crate::project::instance::directory(),
    )
}
#[cfg(test)]
#[allow(unused_imports)]
use super::AcpLaunchProfile;
#[cfg(test)]
#[allow(unused_imports)]
use serde_json::json;

#[cfg(test)]
#[path = "openclaw_tests.rs"]
mod tests;
