//! Copilot uses transient client approvals; CLI-wide grants require current authorization.
use crate::AcpError;

pub(crate) fn validate_launch(args: &[String], full_access: bool) -> Result<(), AcpError> {
    let bypass = args.iter().any(|arg| {
        matches!(
            arg.split('=').next().unwrap_or_default(),
            "--allow-all"
                | "--allow-all-tools"
                | "--allow-all-paths"
                | "--allow-all-urls"
                | "--yolo"
        )
    });
    if bypass && !full_access {
        return Err(AcpError::Initialize(
            "Copilot allow-all arguments require the current host full access grant; remove them when closing access".into(),
        ));
    }
    Ok(())
}
