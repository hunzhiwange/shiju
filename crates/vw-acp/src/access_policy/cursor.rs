//! Cursor uses transient ACP approvals; explicit CLI grants require current authorization.
use crate::AcpError;

pub(crate) fn validate_launch(args: &[String], full_access: bool) -> Result<(), AcpError> {
    let bypass = args.iter().take_while(|arg| arg.as_str() != "--").any(|arg| {
        matches!(arg.split('=').next().unwrap_or_default(), "-f" | "--force" | "--yolo")
    });
    if bypass && !full_access {
        return Err(AcpError::Initialize(
            "Cursor force/yolo arguments require the current host full access grant; remove them when closing access".into(),
        ));
    }
    Ok(())
}
