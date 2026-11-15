//! 外部目录访问控制
//!
//! 管理工作区外部目录的访问权限，确保安全策略得到执行。
//!
//! # 功能概述
//!
//! 本模块提供对工作区外部目录的访问权限检查机制，防止未经授权的文件系统访问。
//! 主要用于工具执行时验证目标路径是否在允许的范围内。
//!
//! # 安全策略
//!
//! 访问权限检查遵循以下优先级：
//! 1. 如果启用了绕过模式（`bypass`），直接允许访问
//! 2. 检查路径是否在安全策略允许的工作区内
//! 3. 检查路径是否在工具输出目录内
//! 4. 检查路径是否在技能目录内
//!
//! # 使用示例
//!
//! ```ignore
//! use tool_files::external_directory::{assert_external_directory, Options, Kind};
//!
//! async fn check_access<H: tool_files::FileToolHost>(security: &SecurityPolicy, path: &str) -> Result<(), String> {
//!     let options = Options {
//!         bypass: false,
//!         kind: Kind::File,
//!     };
//!     assert_external_directory::<H>(security, Some(path), Some(options)).await
//! }
//! ```

use agent_paths as global;
use agent_security::SecurityPolicy;
use crate::FileToolHost;
use agent_util::filesystem;
use std::path::{Path, PathBuf};

/// 路径类型枚举
///
/// 用于指定访问检查的目标是文件还是目录，
/// 影响权限验证时的路径处理逻辑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 文件类型
    ///
    /// 当目标是文件时，权限检查将针对文件所在的父目录进行。
    File,

    /// 目录类型
    ///
    /// 当目标是目录时，权限检查直接针对该目录进行。
    Directory,
}

/// 外部目录访问选项
///
/// 配置外部目录访问检查的行为参数。
#[derive(Debug, Clone)]
pub struct Options {
    /// 是否绕过安全检查
    ///
    /// 当设置为 `true` 时，跳过所有访问权限验证。
    /// 此选项应谨慎使用，仅用于已确认为安全的场景。
    pub bypass: bool,

    /// 路径类型
    ///
    /// 指定要检查的目标是文件还是目录，
    /// 影响权限验证时的路径处理方式。
    pub kind: Kind,
}

impl Default for Options {
    /// 返回默认的访问选项
    ///
    /// 默认配置：
    /// - `bypass`: `false` - 不绕过安全检查
    /// - `kind`: `Kind::File` - 按文件类型处理
    fn default() -> Self {
        Self { bypass: false, kind: Kind::File }
    }
}

/// 解析并构建完整路径
///
/// 根据安全策略中的工作区目录，将目标路径解析为完整的绝对路径。
///
/// # 参数
///
/// - `security`: 安全策略引用，包含工作区目录配置
/// - `target`: 目标路径字符串，可以是绝对路径或相对路径
///
/// # 返回值
///
/// 返回解析后的完整路径 `PathBuf`：
/// - 如果 `target` 是绝对路径，直接返回
/// - 如果 `target` 是相对路径，将其拼接在工作区目录之后
///
/// # 示例
///
/// ```ignore
/// // 绝对路径：直接返回
/// resolve_full_path(&security, "/etc/config") // -> /etc/config
///
/// // 相对路径：拼接工作区目录
/// resolve_full_path(&security, "data/file.txt") // -> /workspace/data/file.txt
/// ```
pub fn resolve_full_path(security: &SecurityPolicy, target: &str) -> PathBuf {
    let target = target.trim();
    if Path::new(target).is_absolute() {
        PathBuf::from(target)
    } else {
        security.workspace_dir.join(target)
    }
}

/// 规范化路径
///
/// 处理路径中的 `.`（当前目录）和 `..`（父目录）组件，
/// 返回规范化的绝对路径形式。
///
/// # 参数
///
/// - `path`: 需要规范化的路径引用
///
/// # 返回值
///
/// 返回规范化后的 `PathBuf`，其中：
/// - `.` 组件被移除
/// - `..` 组件会导致前一个路径组件被移除
/// - 保留路径前缀（如 Windows 盘符）和根目录
///
/// # 处理逻辑
///
/// - `Component::Prefix`: 保留路径前缀（Windows 特有）
/// - `Component::RootDir`: 保留根目录分隔符
/// - `Component::CurDir`: 忽略当前目录标记
/// - `Component::ParentDir`: 移除前一个组件（回溯到父目录）
/// - `Component::Normal`: 保留普通路径组件
pub fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(segment) => out.push(segment),
        }
    }
    out
}

/// 检查路径是否在基础允许范围内
///
/// 验证给定路径是否属于安全策略允许的工作区，
/// 或系统工具输出目录。
///
/// # 参数
///
/// - `security`: 安全策略引用
/// - `full`: 待检查的完整路径
///
/// # 返回值
///
/// - `true`: 路径在允许范围内
/// - `false`: 路径不在基础允许范围内（可能仍在技能目录内）
///
/// # 检查范围
///
/// 1. 安全策略中配置的工作区目录
/// 2. 全局数据目录下的 `tool-output` 子目录
fn within_allowed_basics(security: &SecurityPolicy, full: &Path) -> bool {
    // 检查是否在安全策略允许的工作区路径内
    if security.is_resolved_path_allowed(full) {
        return true;
    }

    // 检查是否在工具输出目录内
    let tool_output_dir = global::paths().data.join("tool-output");
    if filesystem::contains(&tool_output_dir, full) {
        return true;
    }

    false
}

/// 断言外部目录访问权限
///
/// 验证目标路径是否在允许的访问范围内，
/// 如果不在允许范围内则返回错误信息。
///
/// # 参数
///
/// - `security`: 安全策略引用，包含工作区配置和权限规则
/// - `target`: 目标路径字符串（可选），为空时直接通过检查
/// - `options`: 访问选项（可选），配置检查行为
///
/// # 返回值
///
/// - `Ok(())`: 访问权限验证通过
/// - `Err(String)`: 访问被拒绝，包含违规信息描述
///
/// # 验证流程
///
/// 1. 如果目标路径为空，直接允许
/// 2. 如果启用了 `bypass` 选项，直接允许
/// 3. 解析并规范化完整路径
/// 4. 检查是否在基础允许范围（工作区或工具输出目录）
/// 5. 检查是否在技能目录内
/// 6. 根据路径类型（文件/目录）确定最终检查路径
/// 7. 返回验证结果或错误信息
///
/// # 示例
///
/// ```ignore
/// // 检查文件访问权限
/// let result = assert_external_directory(
///     &security,
///     Some("/workspace/data/file.txt"),
///     Some(Options { bypass: false, kind: Kind::File }),
/// ).await;
///
/// // 绕过安全检查
/// let result = assert_external_directory(
///     &security,
///     Some("/any/path"),
///     Some(Options { bypass: true, kind: Kind::Directory }),
/// ).await;
/// ```
pub async fn assert_external_directory<H: FileToolHost>(
    security: &SecurityPolicy,
    target: Option<&str>,
    options: Option<Options>,
) -> Result<(), String> {
    // 目标路径为空或仅包含空白字符时，无需检查
    let Some(target) = target.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };

    // 应用默认选项
    let options = options.unwrap_or_default();

    // 如果启用了绕过模式，直接允许访问
    if options.bypass {
        return Ok(());
    }

    // 解析并规范化完整路径
    let full = normalize_path(&resolve_full_path(security, target));

    // 检查是否在基础允许范围内
    if within_allowed_basics(security, &full) {
        return Ok(());
    }

    // 检查是否在技能目录内
    let skill_dirs = H::skill_directories().await;
    for dir in skill_dirs {
        let path = PathBuf::from(dir);
        // 跳过空路径
        if !path.as_os_str().is_empty() && filesystem::contains(&path, &full) {
            return Ok(());
        }
    }

    // 根据路径类型确定最终检查路径：
    // - 目录类型：直接使用原路径
    // - 文件类型：使用其父目录进行权限检查
    let checked_path = match options.kind {
        Kind::Directory => full,
        Kind::File => full.parent().unwrap_or_else(|| full.as_path()).to_path_buf(),
    };

    // 返回安全策略违规信息
    Err(security.resolved_path_violation_message(&checked_path))
}
