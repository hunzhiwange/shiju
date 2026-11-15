//! shell sandbox 的策略数据结构。
//!
//! 该文件只描述沙箱开关、文件系统范围和网络范围，不负责执行平台适配。

use std::path::PathBuf;

/// shell 沙箱的完整配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxConfig {
    /// 是否启用 shell 沙箱。
    pub enabled: bool,
    /// 当前执行上下文是否允许显式关闭沙箱。
    pub allow_override: bool,
    /// 当前上下文是否已经请求关闭沙箱。
    pub override_enabled: bool,
    /// 不使用沙箱的命令名列表。
    pub excluded_commands: Vec<String>,
    /// 文件系统读写执行范围。
    pub filesystem: FilesystemPolicy,
    /// 网络访问策略。
    pub network: NetworkPolicy,
}

impl SandboxConfig {
    /// 为工作区构造默认沙箱配置。
    ///
    /// 参数：
    /// - `workspace_dir`：允许读写的工作区根目录。
    ///
    /// 返回值：默认启用、禁止网络、只允许工作区读写和系统基础执行路径的配置。
    /// 错误处理：该函数不返回错误；调用方需要保证传入路径是期望的工作区边界。
    pub fn for_workspace(workspace_dir: PathBuf) -> Self {
        Self {
            enabled: true,
            allow_override: false,
            override_enabled: false,
            excluded_commands: Vec::new(),
            filesystem: FilesystemPolicy::for_workspace(workspace_dir),
            network: NetworkPolicy::DenyAll,
        }
    }
}

/// 文件系统能力边界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemPolicy {
    /// 允许读取的路径集合。
    pub read_paths: Vec<PathBuf>,
    /// 允许写入的路径集合。
    pub write_paths: Vec<PathBuf>,
    /// 允许执行程序的路径集合。
    pub execute_paths: Vec<PathBuf>,
    /// 工作区根目录。
    pub workspace_dir: PathBuf,
}

impl FilesystemPolicy {
    /// 为单个工作区构造默认文件系统策略。
    ///
    /// 参数：
    /// - `workspace_dir`：工作区根目录。
    ///
    /// 返回值：允许工作区读写，并允许 `/bin`、`/usr/bin` 下基础命令执行的策略。
    /// 错误处理：该函数不返回错误；路径存在性由具体执行阶段处理。
    pub fn for_workspace(workspace_dir: PathBuf) -> Self {
        Self {
            read_paths: vec![workspace_dir.clone()],
            write_paths: vec![workspace_dir.clone()],
            execute_paths: vec![PathBuf::from("/bin"), PathBuf::from("/usr/bin")],
            workspace_dir,
        }
    }
}

/// shell 沙箱网络访问策略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkPolicy {
    /// 拒绝全部网络访问。
    DenyAll,
    /// 预留的按主机允许策略。
    AllowHosts(Vec<String>),
    /// 允许全部网络访问。
    AllowAll,
}
#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
