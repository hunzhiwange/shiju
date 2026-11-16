//! ACP 客户端执行引擎、后台 actor 与代理进程管理。

#![allow(clippy::result_large_err)]

use acp_types::*;
use acp_access_policy::{access_policy, session_mode_preference};
use acp_access_policy::{AccessPolicy, AccessPolicyReason, FullAccessSupport};
use acp_agent_registry::{agent_registry, verify_reviewed_agent_executable};
use acp_host_io::{filesystem, permissions, spawn_command_options, terminal};
use acp_host_io::permissions::PermissionConfirmation;
#[cfg(test)]
use acp_host_io::permissions::{PermissionDecision, classify_permission_decision};
#[cfg(test)]
use acp_lifecycle::{AgentLifecycleExit, AgentLifecycleSnapshot};
use acp_storage::session_persistence;

mod client;
pub use client::{AcpClient, ConnectionDiagnostic, SkillMountLease};

#[cfg(test)]
use acp_agent_registry::{
    AgentCommandSpec, built_in_agent_specs, resolve_agent_spec, resolve_agent_spec_with_overrides,
};

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "client/claude_fallback_acceptance.rs"]
mod claude_fallback_acceptance;
