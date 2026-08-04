//! # 技能安装器模块
//!
//! 本模块提供技能（Skill）的安装与管理功能，支持多种来源的技能安装：
//!
//! - **skills.sh 源**：从 skills.sh 平台安装技能
//! - **Git 源**：从 Git 仓库克隆安装技能
//! - **本地源**：从本地文件系统路径复制安装技能
//!
//! ## 安全机制
//!
//! 所有安装操作都遵循以下安全原则：
//!
//! 1. **域名信任检查**：验证来源域名的可信度
//! 2. **安全审计**：安装前/后执行安全审计
//! 3. **符号链接防护**：拒绝复制符号链接以防止目录遍历攻击
//! 4. **元数据清理**：移除 Git 元数据以减少攻击面
//!
//! ## 使用示例
//!
//! ```ignore
//! use std::path::Path;
//! use crate::app::agent::skills::installer::install_skill_from_source;
//!
//! let skills_path = Path::new("./skills");
//! let result = install_skill_from_source("https://skills.sh/user/repo/skill", skills_path)?;
//! ```

use crate::app::agent::skills::audit;
use crate::app::agent::skills::policy::{
    ensure_source_domain_trust, load_or_init_skill_download_policy, resolve_skill_source_alias,
};
use crate::app::agent::skills::source::{
    is_skills_sh_source, normalize_skills_sh_dir_name, parse_skills_sh_source,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vw_config_types::skills::SkillSecurityConfig;

/// 确保内置预加载技能已安装到指定路径
///
/// 此函数检查并安装系统预置的内置技能。这些技能在首次运行时自动安装，
/// 无需用户手动配置。
///
/// # 参数
///
/// * `skills_path` - 技能安装目录的路径
///
/// # 返回值
///
/// - `Ok(())` - 所有内置技能已成功安装或已存在
/// - `Err(...)` - 安装过程中发生错误（如权限不足、磁盘空间不足等）
///
/// # 内置技能列表
///
/// 当前预加载的技能包括：
/// - `find-skills`：技能发现与搜索功能
/// - `skill-creator`：技能创建与生成辅助工具
///
/// # 平台差异
///
/// - **非 WASM 平台**：执行实际的文件系统操作
/// - **WASM 平台**：直接返回成功（不支持文件系统操作）
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ensure_builtin_preloaded_skills(skills_path: &Path) -> Result<()> {
    /// 内置预加载技能定义数组
    ///
    /// 每个内置技能包含：
    /// - `dir_name`: 安装目录名称
    /// - `source_url`: 来源 URL（用于元数据记录）
    /// - `markdown`: SKILL.md 文件内容（编译时嵌入）
    const BUILTIN_PRELOADED_SKILLS: [BuiltinPreloadedSkill; 2] = [
        BuiltinPreloadedSkill {
            dir_name: "find-skills",
            source_url: "https://skills.sh/vercel-labs/skills/find-skills",
            markdown: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/skills/find-skills/SKILL.md"
            )),
        },
        BuiltinPreloadedSkill {
            dir_name: "skill-creator",
            source_url: "https://skills.sh/anthropics/skills/skill-creator",
            markdown: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/skills/skill-creator/SKILL.md"
            )),
        },
    ];

    // 遍历所有内置技能，逐个检查并安装
    for builtin in BUILTIN_PRELOADED_SKILLS {
        let skill_dir = skills_path.join(builtin.dir_name);

        // 如果技能目录已存在，跳过安装
        if skill_dir.exists() {
            continue;
        }

        // 创建技能目录结构
        std::fs::create_dir_all(&skill_dir)
            .with_context(|| format!("failed to create {}", skill_dir.display()))?;

        // 写入 SKILL.md 文件（技能定义文件）
        std::fs::write(skill_dir.join("SKILL.md"), builtin.markdown).with_context(|| {
            format!("failed to write preloaded skill {}", skill_dir.join("SKILL.md").display())
        })?;

        // 构建并写入元数据文件
        let meta = serde_json::json!({
            "slug": builtin.dir_name,
            "version": "preloaded",
            "source": builtin.source_url
        });
        std::fs::write(skill_dir.join("_meta.json"), serde_json::to_vec_pretty(&meta)?)
            .with_context(|| {
                format!("failed to write {}", skill_dir.join("_meta.json").display())
            })?;
    }
    Ok(())
}

/// WASM 平台的内置技能安装存根
///
/// 在 WebAssembly 环境中，文件系统操作受限或不支持，
/// 因此此函数直接返回成功而不执行任何操作。
#[cfg(target_arch = "wasm32")]
pub(crate) fn ensure_builtin_preloaded_skills(_skills_path: &Path) -> Result<()> {
    // No-op or unsupported on WASM
    Ok(())
}

/// 从指定来源安装技能
///
/// 这是技能安装的主要入口点，根据来源类型自动选择合适的安装策略。
///
/// # 参数
///
/// * `source` - 技能来源字符串，支持以下格式：
///   - `https://skills.sh/<owner>/<repo>/<skill>` - skills.sh 平台技能
///   - `https://github.com/...` - Git 仓库地址
///   - `git@github.com:...` - Git SSH 地址
///   - `/path/to/skill` - 本地目录路径
/// * `skills_path` - 技能安装目标目录
///
/// # 返回值
///
/// 返回 `InstallResult` 枚举，包含：
/// - 安装后的目录路径
/// - 扫描的文件数量（用于安全审计统计）
///
/// # 安装流程
///
/// 1. 加载下载策略配置
/// 2. 解析来源别名（如有配置）
/// 3. 验证来源域名信任
/// 4. 根据来源类型分派到具体安装函数
/// 5. 执行安全审计
///
/// # 错误
///
/// 可能返回的错误包括：
/// - 来源路径不存在
/// - 域名未通过信任检查
/// - Git 克隆失败
/// - 安全审计未通过
/// - 目标目录已存在
///
/// # 示例
///
/// ```ignore
/// let result = install_skill_from_source(
///     "https://skills.sh/user/repo/my-skill",
///     Path::new("./skills")
/// )?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn install_skill_from_source(
    source: &str,
    skills_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<InstallResult> {
    // 加载或初始化技能下载策略配置
    let mut download_policy = load_or_init_skill_download_policy(skills_path)?;

    // 清理来源字符串（移除首尾空白）
    let source = source.trim().to_string();
    if source.is_empty() {
        anyhow::bail!("skill source must not be empty");
    }

    // 解析来源别名（将简短名称映射到完整 URL）
    let resolved_source = resolve_skill_source_alias(&source, &download_policy);
    if resolved_source != source {
        println!("  Using configured alias '{source}' -> {resolved_source}");
    }

    // 验证来源域名的可信度
    ensure_source_domain_trust(&resolved_source, &mut download_policy, skills_path)?;

    // 检测是否为 skills.sh 来源
    if is_skills_sh_source(&resolved_source) {
        let (installed_dir, files_scanned) =
            install_skills_sh_source(&resolved_source, skills_path, security)
                .with_context(|| format!("failed to install skills.sh skill: {resolved_source}"))?;
        return Ok(InstallResult::SkillsSh { installed_dir, files_scanned });
    }

    // 检测是否为 Git 来源
    if is_git_source(&resolved_source) {
        let (installed_dir, files_scanned) =
            install_git_skill_source(&resolved_source, skills_path, security).with_context(|| {
                format!("failed to install git skill source: {resolved_source}")
            })?;
        return Ok(InstallResult::Git { installed_dir, files_scanned });
    }

    // 默认作为本地来源处理
    let (installed_dir, files_scanned) =
        install_local_skill_source(&resolved_source, skills_path, security)
            .with_context(|| format!("failed to install local skill source: {resolved_source}"))?;
    Ok(InstallResult::Local { installed_dir, files_scanned })
}

/// WASM 平台的技能安装存根
///
/// WebAssembly 环境不支持技能安装操作，直接返回错误。
#[cfg(target_arch = "wasm32")]
pub(crate) fn install_skill_from_source(
    _source: &str,
    _skills_path: &Path,
    _security: &SkillSecurityConfig,
) -> Result<InstallResult> {
    anyhow::bail!("Skill installation is not supported on Web/WASM");
}

/// 技能安装结果枚举
///
/// 表示不同来源类型的安装结果，所有变体都包含：
/// - `installed_dir`: 安装后的技能目录路径
/// - `files_scanned`: 安全审计扫描的文件数量
pub(crate) enum InstallResult {
    /// 从 skills.sh 平台安装的结果
    SkillsSh { installed_dir: PathBuf, files_scanned: usize },
    /// 从 Git 仓库安装的结果
    Git { installed_dir: PathBuf, files_scanned: usize },
    /// 从本地路径安装的结果
    Local { installed_dir: PathBuf, files_scanned: usize },
}

/// 内置预加载技能定义结构体
///
/// 用于在编译时定义要预装的内置技能。
#[cfg(not(target_arch = "wasm32"))]
struct BuiltinPreloadedSkill {
    /// 技能目录名称（也是技能的 slug）
    dir_name: &'static str,
    /// 技能来源 URL（用于元数据记录）
    source_url: &'static str,
    /// SKILL.md 文件内容（编译时通过 include_str! 嵌入）
    markdown: &'static str,
}

/// 检测字符串是否为 Git 来源地址
///
/// 支持多种 Git URL 格式的检测：
/// - HTTPS URL: `https://github.com/user/repo.git`
/// - HTTP URL: `http://github.com/user/repo.git`
/// - SSH URL: `ssh://git@github.com/user/repo.git`
/// - Git 协议: `git://github.com/user/repo.git`
/// - SCP 风格: `git@github.com:user/repo.git`
///
/// # 参数
///
/// * `source` - 待检测的来源字符串
///
/// # 返回值
///
/// - `true` - 是有效的 Git 来源
/// - `false` - 不是 Git 来源（可能是本地路径或其他格式）
pub(crate) fn is_git_source(source: &str) -> bool {
    (is_git_scheme_source(source, "https://") && source.ends_with(".git"))
        || (is_git_scheme_source(source, "http://") && source.ends_with(".git"))
        || is_git_scheme_source(source, "ssh://")
        || is_git_scheme_source(source, "git://")
        || is_git_scp_source(source)
}

/// 检测是否为指定协议前缀的 Git URL
///
/// 验证字符串是否以指定的协议开头，并且包含有效的主机名。
///
/// # 参数
///
/// * `source` - 待检测的来源字符串
/// * `scheme` - 协议前缀（如 "https://"、"git://" 等）
///
/// # 返回值
///
/// - `true` - 符合格式要求
/// - `false` - 不符合格式要求
fn is_git_scheme_source(source: &str, scheme: &str) -> bool {
    // 移除协议前缀
    let Some(rest) = source.strip_prefix(scheme) else {
        return false;
    };

    // URL 路径不能为空或以 / 开头（必须有主机名）
    if rest.is_empty() || rest.starts_with('/') {
        return false;
    }

    // 提取主机名部分（在第一个 / ? # 之前）
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !host.is_empty()
}

/// 检测是否为 SCP 风格的 Git 地址
///
/// SCP 风格的地址格式为 `user@host:path`，例如 `git@github.com:user/repo.git`。
///
/// 此函数采用严格的检测规则以避免将本地路径误判为 Git 远程地址。
///
/// # 参数
///
/// * `source` - 待检测的来源字符串
///
/// # 返回值
///
/// - `true` - 符合 SCP 风格的 Git 地址格式
/// - `false` - 不符合格式
///
/// # 检测规则
///
/// 1. 必须包含 `:` 分隔符
/// 2. 不能包含 `://`（排除普通 URL）
/// 3. 用户名和主机名之间必须有 `@`
/// 4. 用户名和主机名都不能为空
/// 5. 用户名和主机名都不能包含路径分隔符（`/` 或 `\`）
fn is_git_scp_source(source: &str) -> bool {
    // SCP 风格的 Git 地址格式：git@host:owner/repo.git
    // 保持足够的严格性，避免将本地路径误判为 Git 远程地址

    // 必须包含冒号分隔符
    let Some((user_host, remote_path)) = source.split_once(':') else {
        return false;
    };

    // 远程路径不能为空
    if remote_path.is_empty() {
        return false;
    }

    // 不能包含 ://（排除普通 URL 格式）
    if source.contains("://") {
        return false;
    }

    // 必须包含 @ 分隔用户名和主机名
    let Some((user, host)) = user_host.split_once('@') else {
        return false;
    };

    // 验证用户名和主机名的有效性
    !user.is_empty()
        && !host.is_empty()
        && !user.contains('/')
        && !user.contains('\\')
        && !host.contains('/')
        && !host.contains('\\')
}

/// 创建技能目录的快照
///
/// 记录指定目录下所有子目录/文件的路径，用于后续比较检测新增内容。
///
/// # 参数
///
/// * `skills_path` - 要快照的技能目录路径
///
/// # 返回值
///
/// 返回包含所有现有路径的 HashSet
#[cfg(not(target_arch = "wasm32"))]
fn snapshot_skill_children(skills_path: &Path) -> Result<HashSet<PathBuf>> {
    let mut paths = HashSet::new();
    for entry in std::fs::read_dir(skills_path)? {
        let entry = entry?;
        paths.insert(entry.path());
    }
    Ok(paths)
}

/// 检测新安装的技能目录
///
/// 通过比较安装前后的目录快照，确定新创建的目录。
///
/// # 参数
///
/// * `skills_path` - 技能安装目录
/// * `before` - 安装前的目录快照
///
/// # 返回值
///
/// - `Ok(PathBuf)` - 唯一新增的目录路径
/// - `Err(...)` - 没有新增目录或新增了多个目录
///
/// # 错误情况
///
/// - 未发现新目录：可能是克隆失败或目录权限问题
/// - 发现多个新目录：无法确定哪个是技能目录
#[cfg(not(target_arch = "wasm32"))]
fn detect_newly_installed_directory(
    skills_path: &Path,
    before: &HashSet<PathBuf>,
) -> Result<PathBuf> {
    let mut created = Vec::new();

    // 遍历当前目录，找出所有新增的路径
    for entry in std::fs::read_dir(skills_path)? {
        let entry = entry?;
        let path = entry.path();
        if !before.contains(&path) && path.is_dir() {
            created.push(path);
        }
    }

    // 根据新增目录数量决定返回结果
    match created.len() {
        1 => Ok(created.remove(0)),
        0 => anyhow::bail!(
            "Unable to determine installed skill directory after clone (no new directory found)"
        ),
        _ => anyhow::bail!(
            "Unable to determine installed skill directory after clone (multiple new directories found)"
        ),
    }
}

/// 执行技能安全审计
///
/// 对指定目录进行安全审计，检查是否存在潜在的安全风险。
///
/// # 参数
///
/// * `skill_path` - 要审计的技能目录路径
///
/// # 返回值
///
/// - `Ok(SkillAuditReport)` - 审计通过，返回审计报告
/// - `Err(...)` - 审计未通过，包含详细的失败原因
#[cfg(not(target_arch = "wasm32"))]
fn enforce_skill_security_audit(
    skill_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<audit::SkillAuditReport> {
    let report = audit::audit_skill_directory_with_security(skill_path, security)?;

    // 如果审计结果干净，直接返回报告
    if report.is_clean() {
        return Ok(report);
    }

    // 审计未通过，返回错误
    anyhow::bail!("Skill security audit failed: {}", report.summary());
}

/// 移除技能目录中的 Git 元数据
///
/// 删除 `.git` 目录以减少攻击面和存储空间占用。
/// 安装完成后，技能不再需要 Git 版本控制信息。
///
/// # 参数
///
/// * `skill_path` - 技能目录路径
///
/// # 返回值
///
/// - `Ok(())` - 成功移除或 `.git` 目录不存在
/// - `Err(...)` - 移除失败（权限问题等）
#[cfg(not(target_arch = "wasm32"))]
fn remove_git_metadata(skill_path: &Path) -> Result<()> {
    let git_dir = skill_path.join(".git");
    if git_dir.exists() {
        std::fs::remove_dir_all(&git_dir)
            .with_context(|| format!("failed to remove {}", git_dir.display()))?;
    }
    Ok(())
}

/// 安全地递归复制目录
///
/// 复制源目录及其所有内容到目标位置，但拒绝处理符号链接
/// 以防止目录遍历攻击和其他安全风险。
///
/// # 参数
///
/// * `src` - 源目录路径
/// * `dest` - 目标目录路径
///
/// # 返回值
///
/// - `Ok(())` - 复制成功
/// - `Err(...)` - 复制失败
///
/// # 安全检查
///
/// 1. 源路径本身不能是符号链接
/// 2. 源目录中的任何条目不能是符号链接
/// 3. 源路径必须是目录
///
/// # 错误情况
///
/// - 源路径是符号链接：拒绝复制
/// - 源路径不是目录：拒绝复制
/// - 目录中的条目是符号链接：拒绝复制
/// - 文件操作失败（权限、磁盘空间等）
#[cfg(not(target_arch = "wasm32"))]
fn copy_dir_recursive_secure(src: &Path, dest: &Path) -> Result<()> {
    // 获取源路径的元数据（不跟随符号链接）
    let src_meta = std::fs::symlink_metadata(src)
        .with_context(|| format!("failed to read metadata for {}", src.display()))?;

    // 安全检查：拒绝符号链接
    if src_meta.file_type().is_symlink() {
        anyhow::bail!("Refusing to copy symlinked skill source path: {}", src.display());
    }

    // 类型检查：必须是目录
    if !src_meta.is_dir() {
        anyhow::bail!("Skill source must be a directory: {}", src.display());
    }

    // 创建目标目录
    std::fs::create_dir_all(dest)
        .with_context(|| format!("failed to create destination {}", dest.display()))?;

    // 遍历源目录中的所有条目
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&src_path)
            .with_context(|| format!("failed to read metadata for {}", src_path.display()))?;

        // 安全检查：拒绝符号链接
        if metadata.file_type().is_symlink() {
            anyhow::bail!("Refusing to copy symlink within skill source: {}", src_path.display());
        }

        // 根据类型递归处理
        if metadata.is_dir() {
            // 递归复制子目录
            copy_dir_recursive_secure(&src_path, &dest_path)?;
        } else if metadata.is_file() {
            // 复制文件
            std::fs::copy(&src_path, &dest_path).with_context(|| {
                format!(
                    "failed to copy skill file from {} to {}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;
        }
    }

    Ok(())
}

/// 从本地路径安装技能
///
/// 将本地文件系统中的技能目录复制到技能安装目录。
///
/// # 参数
///
/// * `source` - 本地技能源路径
/// * `skills_path` - 技能安装目标目录
///
/// # 返回值
///
/// 返回元组 `(安装后的路径, 扫描的文件数)`
///
/// # 安装流程
///
/// 1. 验证源路径存在
/// 2. 规范化路径（解析相对路径、符号链接等）
/// 3. 执行安全审计（安装前）
/// 4. 检查目标目录不存在
/// 5. 安全复制目录
/// 6. 执行安全审计（安装后）
///
/// # 错误处理
///
/// 如果复制过程中或安装后审计失败，会自动清理已创建的目标目录。
#[cfg(not(target_arch = "wasm32"))]
fn install_local_skill_source(
    source: &str,
    skills_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<(PathBuf, usize)> {
    let source_path = PathBuf::from(source);

    // 验证源路径存在
    if !source_path.exists() {
        anyhow::bail!("Source path does not exist: {source}");
    }

    // 规范化路径（解析 . 和 .. 等）
    let source_path = source_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize source path {source}"))?;

    // 安装前安全审计
    let _ = enforce_skill_security_audit(&source_path, security)?;

    // 确定目标路径
    let name = source_path.file_name().context("Source path must include a directory name")?;
    let dest = skills_path.join(name);

    // 检查目标不存在
    if dest.exists() {
        anyhow::bail!("Destination skill already exists: {}", dest.display());
    }

    // 执行安全复制，失败时自动清理
    if let Err(err) = copy_dir_recursive_secure(&source_path, &dest) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(err);
    }

    // 安装后安全审计，失败时自动清理
    match enforce_skill_security_audit(&dest, security) {
        Ok(report) => Ok((dest, report.files_scanned)),
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dest);
            Err(err)
        }
    }
}

/// 从 Git 仓库安装技能
///
/// 克隆 Git 仓库到技能目录，并执行必要的安全检查和清理。
///
/// # 参数
///
/// * `source` - Git 仓库地址
/// * `skills_path` - 技能安装目标目录
///
/// # 返回值
///
/// 返回元组 `(安装后的路径, 扫描的文件数)`
///
/// # 安装流程
///
/// 1. 记录安装前的目录状态快照
/// 2. 执行浅克隆（depth=1，只获取最新提交）
/// 3. 检测新创建的目录
/// 4. 移除 `.git` 元数据目录
/// 5. 执行安全审计
///
/// # 错误处理
///
/// - Git 克隆失败：返回错误信息
/// - 审计失败：自动清理已克隆的目录
#[cfg(not(target_arch = "wasm32"))]
fn install_git_skill_source(
    source: &str,
    skills_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<(PathBuf, usize)> {
    // 记录安装前的目录快照
    let before = snapshot_skill_children(skills_path)?;

    // 执行 Git 浅克隆（只获取最新版本，节省空间和时间）
    let output = git_std_command()
        .args(["clone", "--depth", "1", source])
        .current_dir(skills_path)
        .output()?;

    // 检查克隆是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {stderr}");
    }

    // 检测新安装的目录
    let installed_dir = detect_newly_installed_directory(skills_path, &before)?;

    // 移除 Git 元数据
    remove_git_metadata(&installed_dir)?;

    // 执行安全审计
    match enforce_skill_security_audit(&installed_dir, security) {
        Ok(report) => Ok((installed_dir, report.files_scanned)),
        Err(err) => {
            // 审计失败时清理目录
            let _ = std::fs::remove_dir_all(&installed_dir);
            Err(err)
        }
    }
}

/// 从 skills.sh 平台安装技能
///
/// skills.sh 是一个技能分享平台。此函数处理从该平台安装技能的特定逻辑：
/// 克隆对应的 GitHub 仓库，定位技能目录，然后复制到本地。
///
/// # 参数
///
/// * `source` - skills.sh 风格的 URL，格式为 `https://skills.sh/<owner>/<repo>/<skill>`
/// * `skills_path` - 技能安装目标目录
///
/// # 返回值
///
/// 返回元组 `(安装后的路径, 扫描的文件数)`
///
/// # 安装流程
///
/// 1. 解析 skills.sh URL，提取 owner、repo、skill 信息
/// 2. 构建 GitHub 仓库 URL
/// 3. 克隆仓库到临时目录
/// 4. 在仓库中定位技能目录（检查 `skills/<skill>/` 或 `<skill>/`）
/// 5. 安全复制技能目录到目标位置
/// 6. 写入 `_meta.json` 元数据文件
/// 7. 执行安全审计
///
/// # 目录查找逻辑
///
/// 技能在仓库中可能位于两个位置：
/// - `skills/<skill-name>/` - 推荐的标准位置
/// - `<skill-name>/` - 仓库根目录下的技能
///
/// # 错误处理
///
/// 任何步骤失败都会清理已创建的目录和临时文件。
#[cfg(not(target_arch = "wasm32"))]
fn install_skills_sh_source(
    source: &str,
    skills_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<(PathBuf, usize)> {
    // 解析 skills.sh URL
    let parsed = parse_skills_sh_source(source).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid skills.sh source '{source}': expected https://skills.sh/<owner>/<repo>/<skill>"
        )
    })?;

    // 获取对应的 GitHub 仓库 URL
    let repo_url = parsed.github_repo_url();

    // 创建临时目录用于克隆
    let checkout_root = tempfile::tempdir().context("failed to create temporary checkout dir")?;
    let checkout_dir = checkout_root.path().join("repo");

    // 克隆 GitHub 仓库
    let output =
        git_std_command().args(["clone", "--depth", "1", &repo_url]).arg(&checkout_dir).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("failed to clone skills.sh repository {repo_url}: {stderr}");
    }

    // 构建可能的技能目录路径候选
    let candidate_paths =
        [checkout_dir.join("skills").join(&parsed.skill), checkout_dir.join(&parsed.skill)];

    // 查找包含 SKILL.md 或 SKILL.toml 的目录
    let source_dir = candidate_paths
        .iter()
        .find(|candidate| {
            let candidate = candidate.as_path();
            candidate.join("SKILL.md").exists() || candidate.join("SKILL.toml").exists()
        })
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not locate skill '{}' in repository {} (checked skills/{}/ and {}/)",
                parsed.skill,
                repo_url,
                parsed.skill,
                parsed.skill
            )
        })?;

    // 规范化技能目录名称
    let normalized_name = normalize_skills_sh_dir_name(&parsed.skill);
    if normalized_name.is_empty() {
        anyhow::bail!("invalid skill name '{}' derived from skills.sh URL: {source}", parsed.skill);
    }

    // 确定目标路径
    let dest = skills_path.join(&normalized_name);
    if dest.exists() {
        anyhow::bail!("Destination skill already exists: {}", dest.display());
    }

    // 安全复制技能目录
    if let Err(err) = copy_dir_recursive_secure(&source_dir, &dest) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(err);
    }

    // 构建 skills.sh 特定的元数据
    let meta = serde_json::json!({
        "slug": format!("{}/{}", parsed.owner, parsed.skill),
        "version": "skills.sh",
        "ownerId": parsed.owner,
        "source": source,
    });

    // 写入元数据文件
    if let Err(err) = std::fs::write(
        dest.join("_meta.json"),
        serde_json::to_vec_pretty(&meta).context("failed to serialize skills.sh metadata")?,
    ) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(err).context("failed to persist skills.sh metadata");
    }

    // 执行最终安全审计
    match enforce_skill_security_audit(&dest, security) {
        Ok(report) => Ok((dest, report.files_scanned)),
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dest);
            Err(err)
        }
    }
}
use crate::app::agent::shell::git_std_command;
#[cfg(test)]
#[path = "installer_tests.rs"]
mod installer_tests;
