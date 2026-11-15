//! Resume-plan orchestration; terminal execution is owned by app-session-terminal.
use crate::resume::ResumePlan;
use std::process::Command;
use vw_api_external_session::ExternalSessionIdentity;

/// Execute on the authorized host with a freshly rebuilt plan; retain its guard
/// until the native CLI and PTY resources are reaped.
pub fn launch_resume(
    plan: &ResumePlan,
    resolve_agy: impl FnOnce(&ExternalSessionIdentity, &mut Command) -> Result<(), String>,
) -> Result<(), String> {
    app_session_terminal::otty::launch_command(plan.command_with_credentials(resolve_agy)?)
}
