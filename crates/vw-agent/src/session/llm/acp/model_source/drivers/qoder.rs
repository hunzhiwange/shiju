use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "ACP qoder session_model launch has not been verified: Custom Provider requires an authorized Qoder personal-account login and entitlement"
    )
}

#[cfg(test)]
#[path = "qoder_tests.rs"]
mod tests;
