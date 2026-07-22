//! Docker 沙箱模块测试
//!
//! 本模块包含针对 `DockerSandbox` 实现的单元测试，验证以下核心功能：
//!
//! # 测试覆盖范围
//!
//! - **基本功能**：沙箱名称获取、默认镜像配置、自定义镜像创建
//! - **隔离标志**：网络隔离、资源限制（内存、CPU）
//! - **命令包装**：确保原始命令在容器内正确执行
//! - **配置验证**：验证 Docker 运行时参数的正确性
//!
//! # 安全性测试
//!
//! 重点关注容器隔离参数，包括：
//! - 网络隔离（`--network none`）
//! - 内存限制（`--memory 512m`）
//! - CPU 限制（`--cpus 1.0`）
//! - 自动清理（`--rm`）
//!
//! # 运行测试
//!
//! ```bash
//! cargo test --package vibe-agent --lib security::docker::tests
//! ```

use super::*;

/// Docker 沙箱功能测试模块
///
/// 包含所有针对 `DockerSandbox` 的单元测试用例，验证沙箱的
/// 创建、配置和命令包装功能。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试沙箱名称获取
    ///
    /// 验证 `DockerSandbox` 的 `name()` 方法返回正确的标识符。
    /// 所有沙箱实现必须提供唯一的名称用于标识和日志记录。
    ///
    /// # 验证点
    ///
    /// - 返回值应为 `"docker"`
    /// - 名称应与实现类型匹配
    #[test]
    fn docker_sandbox_name() {
        let sandbox = DockerSandbox::default();
        assert_eq!(sandbox.name(), "docker");
    }

    /// 测试默认镜像配置
    ///
    /// 验证使用 `default()` 创建的沙箱使用正确的默认容器镜像。
    /// 默认镜像选择 `alpine:latest` 以确保：
    ///
    /// - 最小化攻击面（Alpine 镜像体积小）
    /// - 快速启动时间
    /// - 基本的 shell 和工具支持
    ///
    /// # 验证点
    ///
    /// - 默认镜像应为 `"alpine:latest"`
    #[test]
    fn docker_sandbox_default_image() {
        let sandbox = DockerSandbox::default();
        assert_eq!(sandbox.image, "alpine:latest");
    }

    #[test]
    fn docker_sandbox_description_and_availability_are_stable() {
        let sandbox = DockerSandbox::default();
        assert_eq!(sandbox.description(), "Docker container isolation (requires docker)");
        assert_eq!(sandbox.is_available(), DockerSandbox::is_installed());
    }

    /// 测试自定义镜像创建
    ///
    /// 验证使用 `with_image()` 方法可以创建使用自定义镜像的沙箱。
    /// 如果 Docker 未安装，该方法应返回错误而不是创建无效沙箱。
    ///
    /// # 行为说明
    ///
    /// - **成功情况**：Docker 已安装时，返回使用指定镜像的沙箱
    /// - **失败情况**：Docker 未安装时，返回错误
    ///
    /// # 验证点
    ///
    /// - 成功时，沙箱应使用指定的镜像（`ubuntu:latest`）
    /// - 失败时，应确认 Docker 未安装
    #[test]
    fn docker_with_custom_image() {
        let result = DockerSandbox::with_image("ubuntu:latest".to_string());
        match result {
            Ok(sandbox) => assert_eq!(sandbox.image, "ubuntu:latest"),
            Err(_) => assert!(!DockerSandbox::is_installed()),
        }
    }

    #[test]
    fn docker_new_and_probe_follow_installation_probe() {
        let installed = DockerSandbox::is_installed();
        assert_eq!(DockerSandbox::new().is_ok(), installed);
        assert_eq!(DockerSandbox::probe().is_ok(), installed);
    }

    /// §1.1 沙箱隔离标志测试
    ///
    /// 以下测试验证 Docker 命令包装器是否正确注入了安全隔离参数。

    /// 测试命令包装包含必要的隔离标志
    ///
    /// 验证 `wrap_command()` 方法正确地将普通命令转换为 Docker 命令，
    /// 并注入所有必要的安全隔离标志。
    ///
    /// # 转换示例
    ///
    /// 输入：`echo hello`
    /// 输出：`docker run --rm --network none --memory 512m --cpus 1.0 alpine:latest echo hello`
    ///
    /// # 安全验证点
    ///
    /// - **程序名**：应变为 `docker`
    /// - **子命令**：必须包含 `run`
    /// - **自动清理**：必须包含 `--rm`（容器退出后自动删除）
    /// - **网络隔离**：必须包含 `--network none`（禁用网络访问）
    /// - **内存限制**：必须包含 `--memory 512m`（限制为 512MB）
    /// - **CPU 限制**：必须包含 `--cpus 1.0`（限制为 1 个 CPU）
    #[test]
    fn docker_wrap_command_includes_isolation_flags() {
        let sandbox = DockerSandbox::default();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        sandbox.wrap_command(&mut cmd).unwrap();

        // 验证命令程序已更改为 docker
        assert_eq!(
            cmd.get_program().to_string_lossy(),
            "docker",
            "wrapped command should use docker as program"
        );

        // 收集所有参数以便验证
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 验证必要的 Docker 子命令和标志
        assert!(args.contains(&"run".to_string()), "must include 'run' subcommand");
        assert!(args.contains(&"--rm".to_string()), "must include --rm for auto-cleanup");
        assert!(args.contains(&"--network".to_string()), "must include --network flag");
        assert!(args.contains(&"none".to_string()), "network must be set to 'none' for isolation");
        assert!(args.contains(&"--memory".to_string()), "must include --memory limit");
        assert!(args.contains(&"512m".to_string()), "memory limit must be 512m");
        assert!(args.contains(&"--cpus".to_string()), "must include --cpus limit");
        assert!(args.contains(&"1.0".to_string()), "CPU limit must be 1.0");
    }

    /// 测试命令包装保留原始命令
    ///
    /// 验证 `wrap_command()` 方法在包装命令时不会丢失或修改
    /// 原始的程序名和参数。
    ///
    /// # 转换示例
    ///
    /// 输入：`ls -la`
    /// 输出：`docker run ... alpine:latest ls -la`
    ///
    /// # 验证点
    ///
    /// - **镜像名称**：必须包含配置的容器镜像
    /// - **原始程序**：原命令程序（`ls`）必须作为参数保留
    /// - **原始参数**：原命令参数（`-la`）必须完整保留
    #[test]
    fn docker_wrap_command_preserves_original_command() {
        let sandbox = DockerSandbox::default();
        let mut cmd = Command::new("ls");
        cmd.arg("-la");
        sandbox.wrap_command(&mut cmd).unwrap();

        // 收集所有参数
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 验证容器镜像和原始命令都被保留
        assert!(args.contains(&"alpine:latest".to_string()), "must include the container image");
        assert!(args.contains(&"ls".to_string()), "original program must be passed as argument");
        assert!(args.contains(&"-la".to_string()), "original args must be preserved");
    }

    /// 测试命令包装使用自定义镜像
    ///
    /// 验证当沙箱配置了自定义镜像时，`wrap_command()` 方法
    /// 使用正确的镜像名称。
    ///
    /// # 配置说明
    ///
    /// 当使用特定的容器镜像时（如 `ubuntu:22.04`），包装后的
    /// Docker 命令必须使用该镜像而非默认的 `alpine:latest`。
    ///
    /// # 验证点
    ///
    /// - 参数列表中必须包含指定的自定义镜像名称
    #[test]
    fn docker_wrap_command_uses_custom_image() {
        let sandbox = DockerSandbox { image: "ubuntu:22.04".to_string() };
        let mut cmd = Command::new("echo");
        sandbox.wrap_command(&mut cmd).unwrap();

        // 收集所有参数
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 验证使用了自定义镜像
        assert!(args.contains(&"ubuntu:22.04".to_string()), "must use the custom image");
    }

    #[test]
    fn docker_wrap_command_preserves_multiple_arguments_in_order() {
        let sandbox = DockerSandbox { image: "busybox:latest".to_string() };
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo hello"]);
        sandbox.wrap_command(&mut cmd).unwrap();

        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
        let image_pos = args.iter().position(|arg| arg == "busybox:latest").unwrap();
        assert_eq!(&args[image_pos + 1..], ["sh", "-c", "echo hello"]);
    }

    /// 测试 Debug 和 Clone derives
    ///
    /// 验证 `DockerSandbox` 正确实现了 `Debug` 和 `Clone` trait。
    /// 这些派生 trait 用于日志记录和沙箱实例共享。
    #[test]
    fn docker_sandbox_debug_and_clone() {
        let sandbox = DockerSandbox::default();
        let cloned = sandbox.clone();
        assert_eq!(format!("{:?}", sandbox), format!("{:?}", cloned));
        assert_eq!(sandbox.image, cloned.image);
    }

    /// 测试命令包装保留程序路径
    ///
    /// 验证当原始程序带有路径前缀（如 `/usr/bin/env`）时，
    /// 包装后的 Docker 命令仍然正确地在参数中保留程序路径。
    #[test]
    fn docker_wrap_command_preserves_program_with_path() {
        let sandbox = DockerSandbox::default();
        let mut cmd = Command::new("/usr/bin/env");
        cmd.arg("SHELL=/bin/bash");
        sandbox.wrap_command(&mut cmd).unwrap();

        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 验证原始程序路径作为参数保留
        assert!(
            args.contains(&"/usr/bin/env".to_string()),
            "original program path must be preserved as argument"
        );
        assert!(
            args.contains(&"SHELL=/bin/bash".to_string()),
            "original arguments must be preserved"
        );
    }

    /// 测试命令包装无参数的情况
    ///
    /// 验证当原始命令没有任何参数时，包装后的命令仍然正常工作。
    /// 这是 `wrap_command` 的边界值测试。
    #[test]
    fn docker_wrap_command_with_no_args() {
        let sandbox = DockerSandbox::default();
        let mut cmd = Command::new("true");
        sandbox.wrap_command(&mut cmd).unwrap();

        let program = cmd.get_program().to_string_lossy().to_string();
        assert_eq!(program, "docker", "program should be docker");

        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
        assert!(args.contains(&"true".to_string()), "original program must appear as argument");
    }

    /// 测试命令包装完全替换原命令
    ///
    /// 验证 `wrap_command` 后，原 `Command` 对象被完全替换为 Docker 命令，
    /// 原程序不再作为程序（而是作为参数），所有安全标志都正确注入。
    #[test]
    fn docker_wrap_command_fully_replaces_command() {
        let sandbox = DockerSandbox::default();
        let mut cmd = Command::new("original_prog");
        cmd.args(["arg1", "arg2", "arg3"]);
        sandbox.wrap_command(&mut cmd).unwrap();

        let program = cmd.get_program().to_string_lossy().to_string();
        assert_eq!(program, "docker", "program must be fully replaced to docker");

        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 验证安全标志都在
        for flag in &["run", "--rm", "--memory", "512m", "--cpus", "1.0", "--network", "none"] {
            assert!(args.contains(&flag.to_string()), "must contain flag: {}", flag);
        }

        // 验证镜像在参数中
        assert!(args.contains(&"alpine:latest".to_string()), "must contain default image");

        // 验证原始程序变为参数
        assert!(args.contains(&"original_prog".to_string()), "original program must be in args");
        assert!(args.contains(&"arg1".to_string()), "original arg1 must be in args");
        assert!(args.contains(&"arg2".to_string()), "original arg2 must be in args");
        assert!(args.contains(&"arg3".to_string()), "original arg3 must be in args");
    }

    /// 测试自定义镜像空字符串边界值
    ///
    /// 验证使用空字符串作为镜像名称时，`with_image` 的行为。
    /// 当 Docker 安装时，`with_image("")` 会创建空白镜像名的沙箱；
    /// 未安装时返回错误。
    ///
    /// 这是 `with_image` 的边界值测试。
    #[test]
    fn docker_with_empty_image() {
        let result = DockerSandbox::with_image("".to_string());
        match result {
            Ok(sandbox) => assert_eq!(
                sandbox.image, "",
                "empty image string should be accepted when docker is installed"
            ),
            Err(_) => assert!(
                !DockerSandbox::is_installed(),
                "error should only occur when docker is not installed"
            ),
        }
    }

    /// 测试 wrap_command 使用空镜像名
    ///
    /// 验证当沙箱使用空字符串作为镜像名时，wrap_command 仍然能够
    /// 正常构造 Docker 命令。这测试了代码路径中 `self.image` 的表达能力。
    #[test]
    fn docker_wrap_command_with_empty_image() {
        let sandbox = DockerSandbox { image: "".to_string() };
        let mut cmd = Command::new("ls");
        sandbox.wrap_command(&mut cmd).unwrap();

        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
        // 验证空镜像作为空参数存在
        assert!(args.contains(&"".to_string()), "empty image should appear as empty arg");
    }

    /// 测试 with_image 创建后 wrap_command 一致性
    ///
    /// 验证通过 `with_image` 创建的沙箱，其 `wrap_command` 使用正确的镜像名称。
    #[test]
    fn docker_with_image_wrap_consistency() {
        let result = DockerSandbox::with_image("python:3.11-slim".to_string());
        match result {
            Ok(sandbox) => {
                let mut cmd = Command::new("python");
                cmd.arg("-c");
                cmd.arg("print('hello')");
                sandbox.wrap_command(&mut cmd).unwrap();

                let args: Vec<String> =
                    cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
                assert!(args.contains(&"python:3.11-slim".to_string()), "must use custom image");
                assert!(args.contains(&"-c".to_string()), "must preserve -c flag");
                assert!(args.contains(&"print('hello')".to_string()), "must preserve script arg");
            }
            Err(_) => assert!(!DockerSandbox::is_installed()),
        }
    }

    /// 测试 is_available 返回一致结果
    ///
    /// 验证 `is_available` 与 `is_installed` 返回相同的结果。
    /// 确保可用性检查的一致性。
    #[test]
    fn docker_is_available_consistent_with_is_installed() {
        let sandbox = DockerSandbox::default();
        assert_eq!(sandbox.is_available(), DockerSandbox::is_installed());
    }
}
