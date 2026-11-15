//! Transient ACP approvals leave no cached host grants in supported suppliers.
use agent_client_protocol::{PermissionOptionKind, RequestPermissionRequest};

pub fn transient_request(request: &RequestPermissionRequest) -> RequestPermissionRequest {
    let mut request = request.clone();
    // Suppliers may map allow_always to session permission rules. Never create those
    // rules from a host grant that can later be withdrawn or restored differently.
    request.options.retain(|option| option.kind != PermissionOptionKind::AllowAlways);
    request
}
