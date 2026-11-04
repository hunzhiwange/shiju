use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!("ACP trae session_model launch has not been verified")
}

#[cfg(test)]
#[path = "trae_tests.rs"]
mod tests;
