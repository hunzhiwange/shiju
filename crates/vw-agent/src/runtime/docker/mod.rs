//! Docker 运行时适配器模块
//!
//! 本模块提供基于 Docker 容器的运行时隔离实现。通过将代理任务在独立的 Docker 容器中执行，
//! 实现轻量级的进程隔离、资源限制和安全边界。
//!
//! # 主要特性
//!
//! - **容器隔离**：每个 Shell 命令在独立的 Docker 容器中运行
//! - **资源限制**：支持内存、CPU 和文件系统的限制配置
//! - **工作区挂载**：可选地将宿主机工作目录挂载到容器内
//! - **网络安全**：可配置容器网络模式
//! - **只读根文件系统**：支持只读模式以增强安全性
//!
//! # 安全考虑
//!
//! - 禁止挂载根文件系统（/）到容器
//! - 支持配置允许的工作区根路径白名单
//! - 默认使用 `--init` 和 `--rm` 标志确保容器清理

use super::traits::RuntimeAdapter;
use crate::app::agent::config::DockerRuntimeConfig;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Docker 运行时适配器
///
/// 提供基于 Docker 容器的轻量级隔离运行时环境。通过将命令执行封装在 Docker 容器中，
/// 实现进程级别的资源隔离和安全边界。
///
/// # 示例
///
/// ```rust,ignore
/// use vibe_agent::runtime::docker::DockerRuntime;
/// use vibe_agent::config::DockerRuntimeConfig;
///
/// let config = DockerRuntimeConfig {
///     image: "alpine:latest".to_string(),
///     mount_workspace: true,
///     memory_limit_mb: Some(512),
///     ..Default::default()
/// };
///
/// let runtime = DockerRuntime::new(config);
/// ```
#[derive(Debug, Clone)]
pub struct DockerRuntime {
    /// Docker 运行时配置
    config: DockerRuntimeConfig,
}

impl DockerRuntime {
    /// 创建新的 Docker 运行时实例
    ///
    /// # 参数
    ///
    /// - `config`: Docker 运行时配置，包含镜像名称、资源限制、挂载选项等
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `DockerRuntime` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let runtime = DockerRuntime::new(config);
    /// ```
    pub fn new(config: DockerRuntimeConfig) -> Self {
        Self { config }
    }

    /// 解析并验证工作区挂载路径
    ///
    /// 执行以下验证步骤：
    /// 1. 将路径解析为绝对路径
    /// 2. 确保路径不是根文件系统（/）
    /// 3. 检查路径是否在允许的工作区根路径白名单内
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 需要挂载的工作区目录路径
    ///
    /// # 返回值
    ///
    /// - `Ok(PathBuf)`: 验证通过的绝对路径
    /// - `Err`: 如果路径验证失败（非绝对路径、根文件系统或不在白名单内）
    ///
    /// # 错误
    ///
    /// - 如果路径不是绝对路径
    /// - 如果路径是根文件系统（/）
    /// - 如果路径不在 `allowed_workspace_roots` 白名单内（当白名单非空时）
    fn workspace_mount_path(&self, workspace_dir: &Path) -> Result<PathBuf> {
        // 尝试将路径解析为规范化的绝对路径
        // 如果解析失败（例如路径不存在），则使用原始路径
        let resolved = workspace_dir.canonicalize().unwrap_or_else(|_| workspace_dir.to_path_buf());

        // 验证路径必须是绝对路径
        if !resolved.is_absolute() {
            anyhow::bail!(
                "Docker runtime requires an absolute workspace path, got: {}",
                resolved.display()
            );
        }

        // 安全检查：禁止挂载根文件系统
        // 这是防止容器逃逸的重要安全措施
        if resolved == Path::new("/") {
            anyhow::bail!("Refusing to mount filesystem root (/) into docker runtime");
        }

        // 如果未配置白名单，则允许所有路径（向后兼容）
        if self.config.allowed_workspace_roots.is_empty() {
            return Ok(resolved);
        }

        // 检查路径是否在允许的白名单内
        // 使用 starts_with 检查以支持子目录
        let allowed = self.config.allowed_workspace_roots.iter().any(|root| {
            let root_path = Path::new(root).canonicalize().unwrap_or_else(|_| PathBuf::from(root));
            resolved.starts_with(root_path)
        });

        if !allowed {
            anyhow::bail!(
                "Workspace path {} is not in runtime.docker.allowed_workspace_roots",
                resolved.display()
            );
        }

        Ok(resolved)
    }
}

/// RuntimeAdapter trait 实现
///
/// 为 Docker 运行时提供标准化的运行时适配器接口实现。
impl RuntimeAdapter for DockerRuntime {
    /// 返回类型擦除的自我引用
    ///
    /// 用于运行时类型检查和向下转型
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// 返回运行时名称
    ///
    /// # 返回值
    ///
    /// 总是返回 `"docker"`
    fn name(&self) -> &str {
        "docker"
    }

    /// 检查运行时是否支持 Shell 访问
    ///
    /// # 返回值
    ///
    /// Docker 运行时总是返回 `true`，因为它通过容器执行 Shell 命令
    fn has_shell_access(&self) -> bool {
        true
    }

    /// 检查运行时是否支持文件系统访问
    ///
    /// # 返回值
    ///
    /// - `true`: 如果配置了 `mount_workspace`
    /// - `false`: 如果工作区未挂载到容器
    fn has_filesystem_access(&self) -> bool {
        self.config.mount_workspace
    }

    /// 获取运行时存储路径
    ///
    /// # 返回值
    ///
    /// - 如果挂载了工作区：`/workspace/<mode-config-dir>`（容器内路径）
    /// - 如果未挂载工作区：`/tmp/<mode-config-dir>`（容器内临时路径）
    ///
    /// # 说明
    ///
    /// 该路径是容器内部的路径，用于存储运行时状态和临时文件
    fn storage_path(&self) -> PathBuf {
        if self.config.mount_workspace {
            PathBuf::from("/workspace").join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        } else {
            PathBuf::from("/tmp").join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        }
    }

    /// 检查运行时是否支持长时间运行的任务
    ///
    /// # 返回值
    ///
    /// Docker 运行时返回 `false`，因为每个命令都在独立的容器中执行，
    /// 容器在命令完成后即被销毁（`--rm` 标志）
    fn supports_long_running(&self) -> bool {
        false
    }

    /// 获取内存预算（以字节为单位）
    ///
    /// # 返回值
    ///
    /// - 如果配置了 `memory_limit_mb`：返回配置的内存限制（转换为字节）
    /// - 如果未配置：返回 `0`（表示无限制）
    ///
    /// # 说明
    ///
    /// 使用 `saturating_mul` 防止内存限制值溢出
    fn memory_budget(&self) -> u64 {
        self.config.memory_limit_mb.map_or(0, |mb| mb.saturating_mul(1024 * 1024))
    }

    /// 构建 Shell 命令的 Docker 执行命令
    ///
    /// 根据配置构建一个 `docker run` 命令，用于在容器中执行指定的 Shell 命令。
    ///
    /// # 参数
    ///
    /// - `command`: 要在容器中执行的 Shell 命令
    /// - `workspace_dir`: 宿主机上的工作区目录路径
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `tokio::process::Command`，可直接用于异步执行
    ///
    /// # 错误
    ///
    /// 如果工作区路径验证失败（非绝对路径、根文件系统或不在白名单内），返回错误
    ///
    /// # Docker 标志说明
    ///
    /// - `--rm`: 容器退出后自动删除
    /// - `--init`: 在容器内运行 init 进程，正确处理信号和僵尸进程
    /// - `--interactive`: 保持 STDIN 打开
    /// - `--network`: 配置网络模式（如 `host`、`bridge`、`none`）
    /// - `--memory`: 内存限制（例如 `512m`）
    /// - `--cpus`: CPU 限制（例如 `1.5`）
    /// - `--read-only`: 将容器根文件系统挂载为只读
    /// - `--volume`: 挂载宿主机目录到容器
    /// - `--workdir`: 设置容器内工作目录
    ///
    /// # 示例
    ///
    /// 生成的命令类似：
    /// ```bash
    /// docker run --rm --init --interactive \
    ///   --network bridge \
    ///   --memory 512m \
    ///   --volume /host/workspace:/workspace:rw \
    ///   --workdir /workspace \
    ///   alpine:latest \
    ///   sh -c "ls -la"
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    fn build_shell_command(
        &self,
        command: &str,
        workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        // 创建基础的 docker run 命令
        let mut process = tokio::process::Command::new("docker");
        process.arg("run").arg("--rm").arg("--init").arg("--interactive");

        // 配置网络模式（如果指定）
        let network = self.config.network.trim();
        if !network.is_empty() {
            process.arg("--network").arg(network);
        }

        // 配置内存限制（如果指定且大于 0）
        if let Some(memory_limit_mb) = self.config.memory_limit_mb.filter(|mb| *mb > 0) {
            process.arg("--memory").arg(format!("{memory_limit_mb}m"));
        }

        // 配置 CPU 限制（如果指定且大于 0）
        if let Some(cpu_limit) = self.config.cpu_limit.filter(|cpus| *cpus > 0.0) {
            process.arg("--cpus").arg(cpu_limit.to_string());
        }

        // 配置只读根文件系统（增强安全性）
        if self.config.read_only_rootfs {
            process.arg("--read-only");
        }

        // 配置工作区挂载（如果启用）
        if self.config.mount_workspace {
            // 验证并获取宿主机工作区路径
            let host_workspace = self.workspace_mount_path(workspace_dir).with_context(|| {
                format!("Failed to validate workspace mount path {}", workspace_dir.display())
            })?;

            // 挂载工作区到容器的 /workspace 目录，权限为读写
            process
                .arg("--volume")
                .arg(format!("{}:/workspace:rw", host_workspace.display()))
                .arg("--workdir")
                .arg("/workspace");
        }

        // 设置容器镜像和要执行的命令
        process.arg(self.config.image.trim()).arg("sh").arg("-c").arg(command);

        Ok(process)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
