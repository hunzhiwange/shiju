//! 本地执行与工作区认领。
pub mod local;
pub mod workspace_claim;
#[cfg(test)]
mod workspace_claim_tests;
pub mod gateway_request;
pub mod gateway_executor;
pub mod worktree_owner;
