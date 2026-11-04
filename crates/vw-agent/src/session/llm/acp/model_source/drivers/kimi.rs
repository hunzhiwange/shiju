use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!("ACP kimi session_model launch has not been verified")
}

#[cfg(test)]
#[path = "kimi_tests.rs"]
mod tests;
