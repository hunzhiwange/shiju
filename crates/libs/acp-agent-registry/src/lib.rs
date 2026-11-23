//! ACP 代理注册、受审运行时验证与安装。

pub mod agent_registry;
#[cfg(not(target_arch = "wasm32"))]
mod install;
mod managed_runtime;

pub use agent_registry::{
    AgentCommandSpec, DEFAULT_AGENT_NAME, EMPLOYEE_ACP_BACKEND, EMPLOYEE_ACP_PROFILE,
    EMPLOYEE_ACP_CREDENTIAL_ENV, built_in_agent_registry, built_in_agent_specs,
    list_built_in_agents, merge_agent_registry, merge_agent_specs, normalize_agent_name,
    resolve_agent_command, resolve_agent_spec, resolve_agent_spec_with_overrides,
};
#[cfg(not(target_arch = "wasm32"))]
pub use install::{InstallLog, InstallProgress, install_builtin_agent, install_builtin_agent_with_log, manual_install_commands};
pub use managed_runtime::{linux_native_only, verify_fixed_linux_cli_launch, reviewed_agent_command, verify_auggie_restricted_executable, verify_droid_restricted_executable, verify_openclaw_mcp_executable, verify_openclaw_restricted_executable, verify_reviewed_agent_executable, verify_vtcode_mcp_executable, verify_vtcode_skill_executable};

/// Resolve the native CLI that serves the built-in Employee ACP stdio protocol.
pub fn employee_acp_executable() -> std::io::Result<std::path::PathBuf> {
    #[cfg(all(unix, debug_assertions))]
    if std::env::var_os("SHIJU_RELAX_GATEWAY").as_deref() == Some(std::ffi::OsStr::new("1")) {
        use std::{io, os::unix::fs::PermissionsExt, path::PathBuf};

        // Relax runs the gateway inside its loader. Use the installed native CLI
        // so the ACP child can start directly on stdio with its restricted environment.
        let executable = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|home| home.is_absolute())
            .map(|home| home.join(".shiju-harness/bin/shiju"))
            .filter(|path| {
                path.symlink_metadata().is_ok_and(|metadata| {
                    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
                })
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Install the ShiJu CLI at ~/.shiju-harness/bin/shiju to use Employee ACP with the Relax gateway",
                )
            })?;
        return Ok(executable);
    }
    std::env::current_exe()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
