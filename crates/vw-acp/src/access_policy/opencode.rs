//! OpenCode ACP approvals remain per request so closing access leaves no cached grants.
use agent_client_protocol::{
    PermissionOptionKind, RequestPermissionRequest, RequestPermissionResponse,
};

use crate::{NonInteractivePermissionPolicy, PermissionMode};

pub(crate) fn resolve_permission_request(
    request: &RequestPermissionRequest,
    mode: PermissionMode,
    non_interactive_policy: Option<NonInteractivePermissionPolicy>,
) -> Result<RequestPermissionResponse, crate::errors::PermissionPromptUnavailableError> {
    let mut request = request.clone();
    // OpenCode maps allow_always to session permission rules. Never create those
    // rules from a host grant that can later be withdrawn or restored differently.
    request.options.retain(|option| option.kind != PermissionOptionKind::AllowAlways);
    crate::permissions::resolve_permission_request(&request, mode, non_interactive_policy)
}
