//! 验证工作区沙箱对网络、读写路径与基础可执行路径采用安全默认策略。

use super::*;
use std::path::PathBuf;

#[test]
fn workspace_policy_defaults_to_deny_network_and_workspace_io() {
    let workspace = PathBuf::from("/tmp/project");
    let config = SandboxConfig::for_workspace(workspace.clone());
    assert!(config.enabled);
    assert!(!config.allow_override);
    assert_eq!(config.network, NetworkPolicy::DenyAll);
    assert_eq!(config.filesystem.read_paths, vec![workspace.clone()]);
    assert_eq!(config.filesystem.write_paths, vec![workspace]);
}

#[test]
fn filesystem_policy_keeps_basic_execute_paths() {
    let policy = FilesystemPolicy::for_workspace(PathBuf::from("/tmp/project"));
    assert!(policy.execute_paths.contains(&PathBuf::from("/bin")));
    assert!(policy.execute_paths.contains(&PathBuf::from("/usr/bin")));
}
