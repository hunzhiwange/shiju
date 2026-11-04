use super::AcpLaunchProfile;

pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "ACP auggie session_model launch has not been verified: fixed upstream rejects extension Provider registration; arbitrary model injection is unavailable"
    )
}

#[cfg(test)]
#[path = "auggie_tests.rs"]
mod tests;
