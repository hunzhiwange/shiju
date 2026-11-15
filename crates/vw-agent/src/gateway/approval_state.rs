//! 宿主租户上下文与网关审批缓存组合。
pub(crate) use agent_gateway::approval_state::{ACP_APPROVAL_CHANNEL, clear_approval_manager_for_directory};

pub(crate) async fn approval_manager_for_current_instance() -> std::sync::Arc<agent_approval::ApprovalManager> {
    agent_gateway::approval_state::approval_manager_for_current_instance(
        crate::session::ui_store::current_tenant_scope(),
    ).await
}
