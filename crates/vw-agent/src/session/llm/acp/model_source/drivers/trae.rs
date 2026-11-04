use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "ACP trae session_model launch has not been verified: fixed 0.121.3 requires authorized platform authentication and lacks a verified dedicated runtime root"
    )
}

#[cfg(test)]
#[path = "trae_tests.rs"]
mod tests;
