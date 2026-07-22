//! CLI 工具自动发现模块
//!
//! 本模块提供 CLI 工具的自动发现功能，通过扫描系统 PATH 环境变量来检测已安装的命令行工具。
//! 完全使用 Rust 标准库实现，零外部依赖，通过 `std::process::Command` 和 `std::env` 实现。
//!
//! # 主要功能
//!
//! - 自动扫描并发现系统中已安装的常见 CLI 工具
//! - 获取工具的可执行文件路径
//! - 尝试获取工具的版本信息
//! - 支持用户自定义额外的工具和排除特定工具
//! - 按类别对工具进行分类（版本控制、语言运行时、包管理器、容器、构建工具、云平台）
//!
//! # 示例
//!
//! ```rust,ignore
//! use vibe_agent::tools::cli_discovery::{discover_cli_tools, DiscoveredCli};
//!
//! // 发现默认的工具列表
//! let tools = discover_cli_tools(&[], &[]);
//!
//! // 发现额外的自定义工具，并排除某些工具
//! let additional = vec!["mytool".to_string()];
//! let excluded = vec!["git".to_string()];
//! let tools = discover_cli_tools(&additional, &excluded);
//!
//! for tool in tools {
//!     println!("{}: {:?} (v{:?})", tool.name, tool.path, tool.version);
//! }
//! ```

use std::path::PathBuf;

use crate::app::agent::shell::std_command;

/// 已发现的 CLI 工具分类枚举
///
/// 该枚举定义了 CLI 工具的不同类别，用于组织和展示发现结果。
/// 每个类别代表一类功能相似的工具。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum CliCategory {
    /// 版本控制工具（如 git）
    VersionControl,
    /// 编程语言运行时（如 python、node、rustc）
    Language,
    /// 包管理器（如 npm、pip、cargo）
    PackageManager,
    /// 容器工具（如 docker）
    Container,
    /// 构建工具（如 make、cargo）
    Build,
    /// 云平台工具（如 kubectl）
    Cloud,
}

impl std::fmt::Display for CliCategory {
    /// 格式化输出分类名称
    ///
    /// # 参数
    ///
    /// - `f`: 格式化器
    ///
    /// # 返回值
    ///
    /// 返回格式化结果，将枚举变体转换为可读的英文字符串
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionControl => write!(f, "Version Control"),
            Self::Language => write!(f, "Language"),
            Self::PackageManager => write!(f, "Package Manager"),
            Self::Container => write!(f, "Container"),
            Self::Build => write!(f, "Build"),
            Self::Cloud => write!(f, "Cloud"),
        }
    }
}

/// 已发现的 CLI 工具元数据结构
///
/// 该结构体存储单个 CLI 工具的完整信息，包括工具名称、可执行文件路径、
/// 版本信息和分类类别。
///
/// # 字段
///
/// - `name`: CLI 工具的名称（如 "git"、"node"）
/// - `path`: 可执行文件的绝对路径
/// - `version`: 工具的版本字符串，如果无法获取则为 None
/// - `category`: 工具所属的分类
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredCli {
    /// CLI 工具的名称
    pub name: String,
    /// 可执行文件的完整路径
    pub path: PathBuf,
    /// 版本信息（如果能获取到）
    pub version: Option<String>,
    /// 工具所属的分类
    pub category: CliCategory,
}

/// 已知的 CLI 工具定义结构
///
/// 该内部结构体用于定义要扫描的已知 CLI 工具的元数据，
/// 包括工具名称、获取版本所需的参数以及工具的分类。
///
/// # 字段
///
/// - `name`: 工具的可执行文件名
/// - `version_args`: 执行该工具获取版本信息所需的命令行参数
/// - `category`: 该工具所属的分类
struct KnownCli {
    /// CLI 工具的可执行文件名称
    name: &'static str,
    /// 获取版本信息所需的命令行参数
    version_args: &'static [&'static str],
    /// 工具的分类
    category: CliCategory,
}

/// 已知 CLI 工具的静态列表
///
/// 该常量定义了本模块默认会扫描的所有 CLI 工具。
/// 涵盖了开发过程中常用的各类工具：
///
/// - 版本控制：git
/// - 编程语言：python、python3、node、rustc
/// - 包管理器：npm、pip、pip3
/// - 容器：docker
/// - 构建工具：cargo、make
/// - 云平台：kubectl
///
/// # 注意
///
/// 该列表是静态的，在编译时确定。如需扫描额外的工具，
/// 请使用 `discover_cli_tools` 函数的 `additional` 参数。
const KNOWN_CLIS: &[KnownCli] = &[
    KnownCli { name: "git", version_args: &["--version"], category: CliCategory::VersionControl },
    KnownCli { name: "python", version_args: &["--version"], category: CliCategory::Language },
    KnownCli { name: "python3", version_args: &["--version"], category: CliCategory::Language },
    KnownCli { name: "node", version_args: &["--version"], category: CliCategory::Language },
    KnownCli { name: "npm", version_args: &["--version"], category: CliCategory::PackageManager },
    KnownCli { name: "pip", version_args: &["--version"], category: CliCategory::PackageManager },
    KnownCli { name: "pip3", version_args: &["--version"], category: CliCategory::PackageManager },
    KnownCli { name: "docker", version_args: &["--version"], category: CliCategory::Container },
    KnownCli { name: "cargo", version_args: &["--version"], category: CliCategory::Build },
    KnownCli { name: "make", version_args: &["--version"], category: CliCategory::Build },
    KnownCli {
        name: "kubectl",
        version_args: &["version", "--client", "--short"],
        category: CliCategory::Cloud,
    },
    KnownCli { name: "rustc", version_args: &["--version"], category: CliCategory::Language },
];

/// 发现系统中可用的 CLI 工具
///
/// 该函数扫描系统 PATH 环境变量，查找已知的 CLI 工具，并为每个找到的工具
/// 收集元数据（名称、路径、版本信息、分类）。
///
/// # 参数
///
/// - `additional`: 额外要扫描的 CLI 工具名称列表，这些工具不在默认列表中
/// - `excluded`: 要从结果中排除的 CLI 工具名称列表
///
/// # 返回值
///
/// 返回一个 `DiscoveredCli` 向量，包含所有找到且未被排除的 CLI 工具的元数据。
///
/// # 算法流程
///
/// 1. 遍历 `KNOWN_CLIS` 中的每个已知工具
/// 2. 跳过在 `excluded` 列表中的工具
/// 3. 尝试查找并探测该工具（获取路径和版本）
/// 4. 如果找到，添加到结果列表
/// 5. 对 `additional` 列表中的工具重复上述过程（跳过已发现的）
///
/// # 示例
///
/// ```rust,ignore
/// // 仅发现默认工具
/// let tools = discover_cli_tools(&[], &[]);
///
/// // 发现默认工具 + 自定义工具，并排除 git
/// let additional = vec!["mytool".to_string()];
/// let excluded = vec!["git".to_string()];
/// let tools = discover_cli_tools(&additional, &excluded);
/// ```
pub fn discover_cli_tools(additional: &[String], excluded: &[String]) -> Vec<DiscoveredCli> {
    let mut results = Vec::new();

    // 遍历所有已知的 CLI 工具定义
    for known in KNOWN_CLIS {
        // 如果该工具在排除列表中，跳过
        if excluded.iter().any(|e| e == known.name) {
            continue;
        }
        // 尝试探测该工具，如果成功则添加到结果列表
        if let Some(cli) = probe_cli(known.name, known.version_args, known.category.clone()) {
            results.push(cli);
        }
    }

    // 探测用户指定的额外工具
    for tool_name in additional {
        // 如果该工具在排除列表中，跳过
        if excluded.iter().any(|e| e == tool_name) {
            continue;
        }
        // 跳过已经发现的工具，避免重复
        if results.iter().any(|r| r.name == *tool_name) {
            continue;
        }
        // 对额外的工具，默认使用 --version 参数尝试获取版本，分类设为 Build
        if let Some(cli) = probe_cli(tool_name, &["--version"], CliCategory::Build) {
            results.push(cli);
        }
    }

    results
}

/// 探测单个 CLI 工具
///
/// 该内部函数尝试查找指定的 CLI 工具并收集其元数据。
///
/// # 参数
///
/// - `name`: CLI 工具的名称
/// - `version_args`: 用于获取版本信息的命令行参数
/// - `category`: 该工具所属的分类
///
/// # 返回值
///
/// 如果工具存在，返回 `Some(DiscoveredCli)` 包含工具的完整元数据；
/// 如果工具不存在或查找失败，返回 `None`。
///
/// # 实现细节
///
/// 1. 首先通过 `which`（Unix）或 `where`（Windows）命令查找可执行文件路径
/// 2. 然后尝试执行工具获取版本信息
/// 3. 将所有信息封装到 `DiscoveredCli` 结构中返回
fn probe_cli(name: &str, version_args: &[&str], category: CliCategory) -> Option<DiscoveredCli> {
    // 使用 which (Unix) 或 where (Windows) 尝试查找工具的可执行文件路径
    let path = find_executable(name)?;

    // 尝试获取版本信息
    let version = get_version(name, version_args);

    Some(DiscoveredCli { name: name.to_string(), path, version, category })
}

/// 在系统 PATH 中查找可执行文件
///
/// 该函数通过调用系统的 `which`（Unix/Linux/macOS）或 `where`（Windows）命令
/// 来查找指定可执行文件的完整路径。
///
/// # 参数
///
/// - `name`: 要查找的可执行文件名称
///
/// # 返回值
///
/// 如果找到，返回 `Some(PathBuf)` 包含可执行文件的完整路径；
/// 如果未找到或查找失败，返回 `None`。
///
/// # 平台差异
///
/// - Unix/Linux/macOS: 使用 `which` 命令
/// - Windows: 使用 `where` 命令
///
/// # 注意
///
/// 该函数仅返回找到的第一个匹配路径（如果有多个）。
fn find_executable(name: &str) -> Option<PathBuf> {
    if let Some(path) = crate::app::agent::shell::resolve_executable(name) {
        return Some(path);
    }

    // 根据操作系统选择不同的查找命令
    #[cfg(target_os = "windows")]
    let which_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";

    // 执行查找命令，捕获标准输出，忽略标准错误
    let output = std_command(which_cmd)
        .arg(name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    // 如果命令执行失败（非零退出码），返回 None
    if !output.status.success() {
        return None;
    }

    // 将输出转换为字符串
    let path_str = String::from_utf8_lossy(&output.stdout);
    // 获取输出的第一行（可能有多个路径，只取第一个）
    let first_line = path_str.lines().next()?.trim();
    // 如果第一行为空，返回 None
    if first_line.is_empty() {
        return None;
    }
    // 将字符串转换为 PathBuf 并返回
    Some(PathBuf::from(first_line))
}

/// 获取 CLI 工具的版本字符串
///
/// 该函数通过执行指定的 CLI 工具并传入版本参数来获取其版本信息。
/// 某些工具会将版本信息输出到标准错误（stderr），本函数会同时检查
/// 标准输出和标准错误。
///
/// # 参数
///
/// - `name`: CLI 工具的名称
/// - `args`: 用于获取版本的命令行参数（如 `["--version"]`）
///
/// # 返回值
///
/// 如果成功获取版本信息，返回 `Some(String)` 包含版本字符串（仅第一行）；
/// 如果执行失败或无法解析版本，返回 `None`。
///
/// # 实现细节
///
/// 1. 执行工具并传入版本参数
/// 2. 同时捕获标准输出和标准错误
/// 3. 优先使用标准输出，如果为空则使用标准错误
///    （某些工具如 pip 会将版本输出到 stderr）
/// 4. 仅返回输出的第一行（通常包含版本号）
fn get_version(name: &str, args: &[&str]) -> Option<String> {
    // 执行工具并传入版本参数，捕获标准输出和标准错误
    let output = std_command(name)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;

    // 将标准输出和标准错误转换为字符串
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 某些工具会将版本输出到 stderr（例如 pip）
    // 优先使用 stdout，如果为空则使用 stderr
    let version_text = if stdout.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        stdout.trim().to_string()
    };

    // 仅提取输出的第一行作为版本字符串
    let first_line = version_text.lines().next()?.trim().to_string();
    // 如果第一行为空，返回 None，否则返回该行
    if first_line.is_empty() { None } else { Some(first_line) }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod shell_env_tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn find_executable_uses_effective_path() {
        let found = find_executable("sh").expect("shell should be found from effective PATH");
        assert!(found.is_file());
        assert_eq!(found.file_name().and_then(|name| name.to_str()), Some("sh"));
    }
}
