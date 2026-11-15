//! Droid uses transient ACP approvals; explicit prompt bypass requires current authorization.
use acp_types::AcpError;

pub fn validate_launch(args: &[String], full_access: bool) -> Result<(), AcpError> {
    let bypass = args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| arg.split('=').next().unwrap_or_default() == "--skip-permissions-unsafe");
    if bypass && !full_access {
        return Err(AcpError::Initialize(
            "Droid skip-permissions-unsafe requires the current host full access grant; remove it when closing access".into(),
        ));
    }
    Ok(())
}
