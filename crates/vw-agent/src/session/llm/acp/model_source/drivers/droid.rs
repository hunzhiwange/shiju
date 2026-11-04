use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "ACP droid session_model launch has not been verified: fixed ACP bidirectional transport and isolated native configuration root are not verified"
    )
}

#[cfg(test)]
#[path = "droid_tests.rs"]
mod tests;
