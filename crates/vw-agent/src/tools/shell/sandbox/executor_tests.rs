//! sandbox executor 的后端探测与命令包装测试。
//!
//! 测试不假设本机一定安装 sandbox 后端，只验证探测不会 panic，并根据后端实际状态检查
//! 构建结果，避免 CI 平台差异造成误报。

use std::any::Any;
use std::path::{Path, PathBuf};

use crate::app::agent::runtime::{NativeRuntime, RuntimeAdapter};
#[cfg(target_os = "macos")]
use crate::tools::shell::sandbox::NetworkPolicy;
use crate::tools::shell::sandbox::SandboxConfig;

use super::SandboxExecutor;

struct FailingRuntime;

impl RuntimeAdapter for FailingRuntime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        "failing"
    }

    fn has_shell_access(&self) -> bool {
        true
    }

    fn has_filesystem_access(&self) -> bool {
        true
    }

    fn storage_path(&self) -> PathBuf {
        PathBuf::from("/tmp")
    }

    fn supports_long_running(&self) -> bool {
        false
    }

    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        anyhow::bail!("runtime unavailable")
    }
}

#[test]
fn sandbox_backend_detection_does_not_panic() {
    let _ = SandboxExecutor::backend_available();
}

#[test]
fn sandbox_executor_builds_runtime_command() {
    let executor = SandboxExecutor::new(SandboxConfig::for_workspace(PathBuf::from(".")));
    let runtime: Box<dyn RuntimeAdapter> = Box::new(NativeRuntime::new());

    let result =
        executor.build_command(runtime.as_ref(), "echo hello", PathBuf::from(".").as_path());

    if SandboxExecutor::backend_available() {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[test]
fn sandbox_executor_adds_workdir_context_to_runtime_errors() {
    let executor = SandboxExecutor::new(SandboxConfig::for_workspace(PathBuf::from("/workspace")));

    let error = match executor.build_command(&FailingRuntime, "echo hello", Path::new("/workspace"))
    {
        Ok(_) => panic!("runtime command construction should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("failed to build runtime command for /workspace"));
}

#[cfg(target_os = "macos")]
#[test]
fn seatbelt_profile_maps_filesystem_and_network_policy() {
    let mut config = SandboxConfig::for_workspace(PathBuf::from("/workspace"));
    config.filesystem.read_paths = vec![PathBuf::from("/read-only")];
    config.filesystem.write_paths = vec![PathBuf::from("/writable")];
    config.filesystem.execute_paths = vec![PathBuf::from("/executables")];
    config.network = NetworkPolicy::AllowHosts(vec!["example.com".into()]);
    let profile = SandboxExecutor::new(config).generate_seatbelt_profile();

    assert!(profile.contains("(allow file-read* (subpath \"/read-only\"))"));
    assert!(profile.contains("(allow file-read* file-write* (subpath \"/writable\"))"));
    assert!(profile.contains("(allow process-exec (subpath \"/executables\"))"));
    assert!(profile.contains("(allow network*)"));
    assert!(!profile.contains("(deny network*)"));
}
