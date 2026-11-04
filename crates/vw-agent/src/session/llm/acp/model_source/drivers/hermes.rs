use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!("ACP hermes session_model launch has not been verified")
}

#[cfg(test)]
#[path = "hermes_tests.rs"]
mod tests;
