//! Docker 运行时模块测试
//!
//! 本模块包含 `DockerRuntime` 及其相关功能的单元测试。
//! 测试范围覆盖：
//! - 运行时名称与内存预算的基础功能
//! - Shell 命令构建（包含 Docker 运行时标志）
//! - 工作区路径白名单访问控制
//! - 网络隔离与只读文件系统标志
//! - 根目录挂载拒绝等安全边界验证
//!
//! 部分测试仅在 Unix 平台执行（如根目录挂载拒绝测试）。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 Docker 运行时的名称返回
    ///
    /// 验证 `DockerRuntime::name()` 方法返回正确的运行时标识符 "docker"。
    #[test]
    fn docker_runtime_name() {
        let runtime = DockerRuntime::new(DockerRuntimeConfig::default());
        assert_eq!(runtime.name(), "docker");
    }

    /// 测试 Docker 运行时的内存预算计算
    ///
    /// 验证 `DockerRuntime::memory_budget()` 方法正确地将 MB 单位转换为字节。
    /// 配置 256 MB 应返回 256 * 1024 * 1024 字节。
    #[test]
    fn docker_runtime_memory_budget() {
        let mut cfg = DockerRuntimeConfig::default();
        cfg.memory_limit_mb = Some(256);
        let runtime = DockerRuntime::new(cfg);
        // 内存限制从 MB 转换为字节
        assert_eq!(runtime.memory_budget(), 256 * 1024 * 1024);
    }

    #[test]
    fn docker_runtime_capabilities_follow_mount_workspace() {
        let runtime = DockerRuntime::new(DockerRuntimeConfig {
            mount_workspace: true,
            ..DockerRuntimeConfig::default()
        });
        assert!(runtime.has_shell_access());
        assert!(runtime.has_filesystem_access());
        assert!(!runtime.supports_long_running());
        assert_eq!(
            runtime.storage_path(),
            PathBuf::from("/workspace").join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        );

        let runtime = DockerRuntime::new(DockerRuntimeConfig {
            mount_workspace: false,
            ..DockerRuntimeConfig::default()
        });
        assert!(!runtime.has_filesystem_access());
        assert_eq!(
            runtime.storage_path(),
            PathBuf::from("/tmp").join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        );
    }

    #[test]
    fn docker_runtime_memory_budget_saturates() {
        let runtime = DockerRuntime::new(DockerRuntimeConfig {
            memory_limit_mb: Some(u64::MAX),
            ..DockerRuntimeConfig::default()
        });
        assert_eq!(runtime.memory_budget(), u64::MAX);
    }

    /// 测试 Shell 命令构建是否包含所有运行时标志
    ///
    /// 验证 `build_shell_command` 方法在构建 Docker 命令时：
    /// - 包含 `--memory` 标志及配置的内存限制（128m）
    /// - 包含 `--cpus` 标志及配置的 CPU 限制（1.5）
    /// - 包含 `--workdir` 标志指定工作目录
    /// - 原始命令（"echo hello"）被正确嵌入
    ///
    /// 此测试确保资源限制参数正确传递到 Docker 命令行。
    #[test]
    fn docker_build_shell_command_includes_runtime_flags() {
        // 构造具有完整资源配置的 Docker 运行时配置
        let cfg = DockerRuntimeConfig {
            image: "alpine:3.20".into(),
            network: "none".into(),
            memory_limit_mb: Some(128),
            cpu_limit: Some(1.5),
            read_only_rootfs: true,
            mount_workspace: true,
            allowed_workspace_roots: Vec::new(),
        };
        let runtime = DockerRuntime::new(cfg);

        // 使用临时目录作为工作区
        let workspace = std::env::temp_dir();
        let command = runtime.build_shell_command("echo hello", &workspace).unwrap();
        let debug = format!("{command:?}");

        // 验证所有关键标志存在
        assert!(debug.contains("docker"));
        assert!(debug.contains("--memory"));
        assert!(debug.contains("128m"));
        assert!(debug.contains("--cpus"));
        assert!(debug.contains("1.5"));
        assert!(debug.contains("--workdir"));
        assert!(debug.contains("echo hello"));
    }

    /// 测试工作区白名单对路径的访问控制
    ///
    /// 验证当 `allowed_workspace_roots` 配置后，只有白名单内的路径被允许。
    /// 尝试使用不在白名单中的路径（"/tmp/blocked_workspace"）应返回错误。
    ///
    /// 此测试确保了 AGENTS.md §3.6 中定义的"默认拒绝"安全原则：
    /// 非授权路径的访问被明确拒绝。
    #[test]
    fn docker_workspace_allowlist_blocks_outside_paths() {
        // 配置仅允许 /tmp/allowed 作为工作区根目录
        let cfg = DockerRuntimeConfig {
            allowed_workspace_roots: vec!["/tmp/allowed".into()],
            ..DockerRuntimeConfig::default()
        };
        let runtime = DockerRuntime::new(cfg);

        // 尝试使用不在白名单中的工作区路径
        let outside = PathBuf::from("/tmp/blocked_workspace");
        let result = runtime.build_shell_command("echo test", &outside);

        // 应返回错误，拒绝访问
        assert!(result.is_err());
    }

    #[test]
    fn docker_workspace_mount_rejects_relative_path() {
        let runtime = DockerRuntime::new(DockerRuntimeConfig {
            mount_workspace: true,
            ..DockerRuntimeConfig::default()
        });

        let error = runtime
            .build_shell_command("echo test", Path::new("relative/workspace"))
            .expect_err("relative workspace path must be rejected")
            .to_string();

        assert!(error.contains("Failed to validate workspace mount path"));
    }

    #[test]
    fn docker_workspace_allowlist_allows_child_paths() {
        let workspace =
            std::env::temp_dir().join(format!("vw-agent-docker-runtime-{}", std::process::id()));
        std::fs::create_dir_all(workspace.join("child")).unwrap();

        let cfg = DockerRuntimeConfig {
            mount_workspace: true,
            allowed_workspace_roots: vec![workspace.display().to_string()],
            ..DockerRuntimeConfig::default()
        };
        let runtime = DockerRuntime::new(cfg);
        let command = runtime.build_shell_command("pwd", &workspace.join("child")).unwrap();
        let debug = format!("{command:?}");

        assert!(debug.contains("--volume"));
        assert!(debug.contains("/workspace:rw"));

        let _ = std::fs::remove_dir_all(workspace);
    }

    // ── §3.3 / §3.4 Docker 挂载与网络隔离测试 ──

    /// 测试 Shell 命令是否包含网络隔离标志
    ///
    /// 验证当配置 `network: "none"` 时，生成的 Docker 命令包含
    /// `--network none` 标志，确保容器被完全隔离，无法访问外部网络。
    ///
    /// 这对应 AGENTS.md 中安全关键面的网络边界控制要求。
    #[test]
    fn docker_build_shell_command_includes_network_flag() {
        // 配置网络隔离
        let cfg = DockerRuntimeConfig { network: "none".into(), ..DockerRuntimeConfig::default() };
        let runtime = DockerRuntime::new(cfg);
        let workspace = std::env::temp_dir();
        let cmd = runtime.build_shell_command("echo hello", &workspace).unwrap();
        let debug = format!("{cmd:?}");
        assert!(
            debug.contains("--network") && debug.contains("none"),
            "must include --network none for isolation"
        );
    }

    /// 测试 Shell 命令是否包含只读文件系统标志
    ///
    /// 验证当配置 `read_only_rootfs: true` 时，生成的 Docker 命令包含
    /// `--read-only` 标志，确保容器内的根文件系统不可被修改。
    ///
    /// 这是安全加固的重要措施，防止恶意代码持久化或篡改系统文件。
    #[test]
    fn docker_build_shell_command_includes_read_only_flag() {
        // 配置只读根文件系统
        let cfg = DockerRuntimeConfig { read_only_rootfs: true, ..DockerRuntimeConfig::default() };
        let runtime = DockerRuntime::new(cfg);
        let workspace = std::env::temp_dir();
        let cmd = runtime.build_shell_command("echo hello", &workspace).unwrap();
        let debug = format!("{cmd:?}");
        assert!(
            debug.contains("--read-only"),
            "must include --read-only flag when read_only_rootfs is set"
        );
    }

    /// 测试拒绝挂载根目录
    ///
    /// 验证当尝试将工作区设置为根目录（"/"）时，命令构建应失败并返回错误。
    /// 挂载根目录会导致容器获得对宿主机完整文件系统的访问权限，这是严重的安全风险。
    ///
    /// 此测试仅在 Unix 平台执行。
    ///
    /// 错误链中应包含 "root" 关键字以明确错误原因。
    #[cfg(unix)]
    #[test]
    fn docker_refuses_root_mount() {
        // 启用工作区挂载功能
        let cfg = DockerRuntimeConfig { mount_workspace: true, ..DockerRuntimeConfig::default() };
        let runtime = DockerRuntime::new(cfg);
        // 尝试使用根目录作为工作区
        let result = runtime.build_shell_command("echo test", Path::new("/"));
        assert!(result.is_err(), "mounting filesystem root (/) must be refused");
        let error_chain = format!("{:#}", result.unwrap_err());
        assert!(
            error_chain.contains("root"),
            "expected root-mount error chain, got: {error_chain}"
        );
    }

    /// 测试未配置内存限制时不生成内存标志
    ///
    /// 验证当 `memory_limit_mb` 为 `None` 时，生成的 Docker 命令
    /// 不应包含 `--memory` 标志，允许容器使用默认内存配置。
    ///
    /// 这确保了配置的可选性：未显式配置的资源限制不会产生默认约束。
    #[test]
    fn docker_no_memory_flag_when_not_configured() {
        // 显式不配置内存限制
        let cfg = DockerRuntimeConfig { memory_limit_mb: None, ..DockerRuntimeConfig::default() };
        let runtime = DockerRuntime::new(cfg);
        let workspace = std::env::temp_dir();
        let cmd = runtime.build_shell_command("echo hello", &workspace).unwrap();
        let debug = format!("{cmd:?}");
        assert!(!debug.contains("--memory"), "should not include --memory when not configured");
    }

    #[test]
    fn docker_omits_blank_or_zero_valued_optional_flags() {
        let cfg = DockerRuntimeConfig {
            network: "   ".into(),
            memory_limit_mb: Some(0),
            cpu_limit: Some(0.0),
            mount_workspace: false,
            ..DockerRuntimeConfig::default()
        };
        let runtime = DockerRuntime::new(cfg);
        let cmd = runtime.build_shell_command("true", Path::new("/tmp")).unwrap();
        let debug = format!("{cmd:?}");

        assert!(!debug.contains("--network"));
        assert!(!debug.contains("--memory"));
        assert!(!debug.contains("--cpus"));
        assert!(!debug.contains("--workdir"));
        assert!(debug.contains("sh"));
        assert!(debug.contains("-c"));
        assert!(debug.contains("true"));
    }
}
