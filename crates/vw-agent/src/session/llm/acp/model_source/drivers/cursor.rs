use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "ACP cursor session_model launch has not been verified: CLI third-party Provider endpoint/key injection has no verified upstream contract"
    )
}

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod tests;
