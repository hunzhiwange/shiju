//! 技能安装与初始化模块
//!
//! 本模块提供技能（Skill）的安装、初始化和管理功能，是 VibeWindow 技能生态的核心入口点。
//!
//! # 主要功能
//!
//! - **技能安装**：支持从多种来源安装技能，包括本地路径、Git 仓库、skills.sh 平台
//! - **安全审计**：所有安装的技能都必须通过安全审计，防止恶意代码注入
//! - **目录初始化**：初始化技能工作目录，创建必要的配置文件和内置技能
//! - **CLI 命令处理**：处理 `vibewindow skills` 相关的命令行指令
//!
//! # 安装来源类型
//!
//! 1. **本地路径**：从本地文件系统复制技能目录
//! 2. **Git 仓库**：通过 `git clone` 从远程仓库安装
//! 3. **skills.sh**：从 skills.sh 平台安装社区技能
//!
//! # 安全机制
//!
//! - 禁止符号链接以防止路径遍历攻击
//! - 安装前后双重安全审计
//! - 路径规范化防止目录逃逸
//! - 自动清理失败的安装
//!
//! # 示例
//!
//! ```bash
//! # 安装技能
//! vibewindow skills install https://github.com/user/skill-repo
//!
//! # 列出已安装技能
//! vibewindow skills list
//!
//! # 删除技能
//! vibewindow skills remove my-skill
//! ```

use crate::app::agent::skill::audit;
use crate::app::agent::skill::constants::BUILTIN_PRELOADED_SKILLS;
use crate::app::agent::skill::loader::load_skills_full_with_config;
use crate::app::agent::skill::policy::{
    ensure_source_domain_trust, load_or_init_skill_download_policy, resolve_skill_source_alias,
};
use crate::app::agent::skill::prompt::skills_dir;
use crate::app::agent::skill::source::{
    is_skills_sh_source, normalize_skills_sh_dir_name, parse_skills_sh_source,
};
use crate::app::agent::skill::types::{SkillCommands, SkillRuntimeConfig};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vw_config_types::skills::SkillSecurityConfig;

/// 快照技能目录的子目录列表
///
/// 在执行可能创建新目录的操作（如 git clone）之前调用，
/// 用于后续检测新创建的目录。
///
/// # 参数
///
/// * `skills_path` - 技能根目录路径
///
/// # 返回值
///
/// 返回包含所有子目录路径的 HashSet，用于后续比较
///
/// # 错误
///
/// 如果读取目录失败，返回 IO 错误
///
/// # 示例
///
/// ```ignore
/// let before = snapshot_skill_children(&skills_path)?;
/// // 执行 git clone...
/// let new_dir = detect_newly_installed_directory(&skills_path, &before)?;
/// ```
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
/// 通过比较快照前后目录列表的差异，识别新创建的目录。
/// 这主要用于 git clone 操作后定位克隆的技能目录。
///
/// # 参数
///
/// * `skills_path` - 技能根目录路径
/// * `before` - 操作前的目录快照
///
/// # 返回值
///
/// - 成功：返回新创建的目录路径
/// - 失败：如果没有新目录或有多个新目录，返回错误
///
/// # 错误
///
/// - 未找到新目录
/// - 发现多个新目录（无法确定是哪一个）
#[cfg(not(target_arch = "wasm32"))]
fn detect_newly_installed_directory(
    skills_path: &Path,
    before: &HashSet<PathBuf>,
) -> Result<PathBuf> {
    let mut created = Vec::new();
    for entry in std::fs::read_dir(skills_path)? {
        let entry = entry?;
        let path = entry.path();
        if !before.contains(&path) && path.is_dir() {
            created.push(path);
        }
    }

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
/// 对指定目录进行安全扫描，检查是否存在潜在的安全风险，
/// 如敏感文件泄露、恶意脚本等。
///
/// # 参数
///
/// * `skill_path` - 要审计的技能目录路径
///
/// # 返回值
///
/// - 成功：返回审计报告（如果审计通过）
/// - 失败：如果审计发现问题，返回错误
///
/// # 安全考虑
///
/// 此函数是技能安装过程中的关键安全检查点，
/// 确保只有通过审计的技能才能被使用。
#[cfg(not(target_arch = "wasm32"))]
fn enforce_skill_security_audit(
    skill_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<audit::SkillAuditReport> {
    let report = audit::audit_skill_directory_with_security(skill_path, security)?;
    if report.is_clean() {
        return Ok(report);
    }

    anyhow::bail!("Skill security audit failed: {}", report.summary());
}

/// 移除 Git 元数据目录
///
/// 删除技能目录中的 `.git` 目录，避免将 Git 历史信息
/// 保留在安装的技能中。这有助于：
/// - 减小技能体积
/// - 避免潜在的敏感信息泄露
/// - 使技能成为独立副本
///
/// # 参数
///
/// * `skill_path` - 技能目录路径
///
/// # 返回值
///
/// 成功返回 Ok(())，失败返回 IO 错误
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
/// 将源目录及其所有内容复制到目标位置，同时执行安全检查：
/// - 拒绝复制符号链接（防止路径遍历攻击）
/// - 验证源路径是目录而非文件
/// - 递归处理子目录
///
/// # 参数
///
/// * `src` - 源目录路径
/// * `dest` - 目标目录路径
///
/// # 返回值
///
/// 成功返回 Ok(())，失败返回详细的错误信息
///
/// # 安全机制
///
/// 1. **符号链接检查**：如果源路径或任何子项是符号链接，立即拒绝
/// 2. **类型验证**：确保源路径是目录
/// 3. **原子性**：如果复制失败，调用者负责清理目标目录
///
/// # 错误
///
/// - 源路径是符号链接
/// - 源路径不是目录
/// - IO 操作失败（创建目录、复制文件等）
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
/// 将本地文件系统中的技能目录复制到技能工作目录。
/// 执行完整的安全审计流程，确保技能安全性。
///
/// # 参数
///
/// * `source` - 源路径（本地文件系统路径）
/// * `skills_path` - 技能根目录路径
///
/// # 返回值
///
/// 成功返回元组 `(安装路径, 扫描文件数)`
///
/// # 安装流程
///
/// 1. 验证源路径存在
/// 2. 规范化路径（解析符号链接）
/// 3. 安装前安全审计
/// 4. 复制目录内容
/// 5. 安装后安全审计
/// 6. 失败时自动清理
///
/// # 错误
///
/// - 源路径不存在
/// - 目标技能已存在
/// - 安全审计失败
/// - IO 操作失败
#[cfg(not(target_arch = "wasm32"))]
fn install_local_skill_source(
    source: &str,
    skills_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<(PathBuf, usize)> {
    // 验证源路径存在
    let source_path = PathBuf::from(source);
    if !source_path.exists() {
        anyhow::bail!("Source path does not exist: {source}");
    }

    // 规范化路径（解析所有符号链接和相对路径）
    let source_path = source_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize source path {source}"))?;

    // 安装前安全审计
    let _ = enforce_skill_security_audit(&source_path, security)?;

    // 提取目录名并构建目标路径
    let name = source_path.file_name().context("Source path must include a directory name")?;
    let dest = skills_path.join(name);

    // 检查目标是否已存在
    if dest.exists() {
        anyhow::bail!("Destination skill already exists: {}", dest.display());
    }

    // 尝试复制目录，失败时清理
    if let Err(err) = copy_dir_recursive_secure(&source_path, &dest) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(err);
    }

    // 安装后安全审计
    match enforce_skill_security_audit(&dest, security) {
        Ok(report) => Ok((dest, report.files_scanned)),
        Err(err) => {
            // 审计失败，清理已复制的目录
            let _ = std::fs::remove_dir_all(&dest);
            Err(err)
        }
    }
}

/// 从 Git 仓库安装技能
///
/// 通过 `git clone` 命令从远程 Git 仓库克隆技能。
/// 使用浅克隆（--depth 1）提高效率。
///
/// # 参数
///
/// * `source` - Git 仓库 URL（支持 https、http、ssh、git 协议）
/// * `skills_path` - 技能根目录路径
///
/// # 返回值
///
/// 成功返回元组 `(安装路径, 扫描文件数)`
///
/// # 安装流程
///
/// 1. 快照当前目录状态
/// 2. 执行浅克隆（git clone --depth 1）
/// 3. 检测新创建的目录
/// 4. 移除 .git 元数据目录
/// 5. 执行安全审计
/// 6. 失败时自动清理
///
/// # 错误
///
/// - Git clone 失败
/// - 无法确定克隆的目录
/// - 安全审计失败
#[cfg(not(target_arch = "wasm32"))]
fn install_git_skill_source(
    source: &str,
    skills_path: &Path,
    security: &SkillSecurityConfig,
) -> Result<(PathBuf, usize)> {
    // 快照当前状态，用于后续检测新目录
    let before = snapshot_skill_children(skills_path)?;

    // 执行浅克隆
    let output = git_std_command()
        .args(["clone", "--depth", "1", source])
        .current_dir(skills_path)
        .output()?;

    // 检查克隆是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {stderr}");
    }

    // 检测新创建的目录
    let installed_dir = detect_newly_installed_directory(skills_path, &before)?;

    // 移除 Git 元数据
    remove_git_metadata(&installed_dir)?;

    // 执行安全审计
    match enforce_skill_security_audit(&installed_dir, security) {
        Ok(report) => Ok((installed_dir, report.files_scanned)),
        Err(err) => {
            // 审计失败，清理已克隆的目录
            let _ = std::fs::remove_dir_all(&installed_dir);
            Err(err)
        }
    }
}

/// 从 skills.sh 平台安装技能
///
/// skills.sh 是一个社区技能分享平台。此函数从 skills.sh URL
/// 解析出对应的 GitHub 仓库，克隆后提取指定技能。
///
/// # 参数
///
/// * `source` - skills.sh URL（格式：https://skills.sh/<owner>/<repo>/<skill>）
/// * `skills_path` - 技能根目录路径
///
/// # 返回值
///
/// 成功返回元组 `(安装路径, 扫描文件数)`
///
/// # 安装流程
///
/// 1. 解析 skills.sh URL
/// 2. 克隆对应的 GitHub 仓库到临时目录
/// 3. 在仓库中查找技能目录（检查 skills/<skill>/ 或 <skill>/）
/// 4. 复制技能文件到目标位置
/// 5. 写入元数据文件（_meta.json）
/// 6. 执行安全审计
/// 7. 失败时自动清理
///
/// # 技能定位策略
///
/// 函数会依次检查：
/// - `<repo>/skills/<skill>/SKILL.md` 或 `SKILL.toml`
/// - `<repo>/<skill>/SKILL.md` 或 `SKILL.toml`
///
/// # 错误
///
/// - URL 格式无效
/// - Git clone 失败
/// - 在仓库中未找到技能
/// - 安全审计失败
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

    // 克隆仓库
    let output =
        git_std_command().args(["clone", "--depth", "1", &repo_url]).arg(&checkout_dir).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("failed to clone skills.sh repository {repo_url}: {stderr}");
    }

    // 尝试在仓库中定位技能目录
    // 检查两个可能的位置：skills/<skill>/ 和 <skill>/
    let candidate_paths =
        [checkout_dir.join("skills").join(&parsed.skill), checkout_dir.join(&parsed.skill)];
    let source_dir = candidate_paths
        .iter()
        .find(|candidate| {
            let candidate = candidate.as_path();
            // 检查是否存在 SKILL.md 或 SKILL.toml
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
    let dest = skills_path.join(&normalized_name);

    // 检查目标是否已存在
    if dest.exists() {
        anyhow::bail!("Destination skill already exists: {}", dest.display());
    }

    // 复制技能文件
    if let Err(err) = copy_dir_recursive_secure(&source_dir, &dest) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(err);
    }

    // 写入 skills.sh 元数据
    let meta = serde_json::json!({
        "slug": format!("{}/{}", parsed.owner, parsed.skill),
        "version": "skills.sh",
        "ownerId": parsed.owner,
        "source": source,
    });
    if let Err(err) = std::fs::write(
        dest.join("_meta.json"),
        serde_json::to_vec_pretty(&meta).context("failed to serialize skills.sh metadata")?,
    ) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(err).context("failed to persist skills.sh metadata");
    }

    // 执行安全审计
    match enforce_skill_security_audit(&dest, security) {
        Ok(report) => Ok((dest, report.files_scanned)),
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dest);
            Err(err)
        }
    }
}

/// 确保内置预加载技能存在
///
/// 为所有内置技能创建目录和必要的文件。
/// 如果技能目录已存在，则跳过。
///
/// # 参数
///
/// * `skills_path` - 技能根目录路径
///
/// # 返回值
///
/// 成功返回 Ok(())
///
/// # 内置技能
///
/// 内置技能在 `BUILTIN_PRELOADED_SKILLS` 常量中定义，
/// 每个技能包含：
/// - `dir_name`: 目录名称
/// - `markdown`: SKILL.md 内容
/// - `source_url`: 来源 URL
///
/// # 创建的文件
///
/// 对于每个内置技能，会创建：
/// - `SKILL.md`: 技能说明文档
/// - `_meta.json`: 元数据文件
#[cfg(not(target_arch = "wasm32"))]
fn ensure_builtin_preloaded_skills(skills_path: &Path) -> Result<()> {
    for builtin in BUILTIN_PRELOADED_SKILLS {
        let skill_dir = skills_path.join(builtin.dir_name);

        // 如果已存在则跳过
        if skill_dir.exists() {
            continue;
        }

        // 创建技能目录
        std::fs::create_dir_all(&skill_dir)
            .with_context(|| format!("failed to create {}", skill_dir.display()))?;

        // 写入 SKILL.md 文件
        std::fs::write(skill_dir.join("SKILL.md"), builtin.markdown).with_context(|| {
            format!("failed to write preloaded skill {}", skill_dir.join("SKILL.md").display())
        })?;

        // 写入元数据文件
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

/// 初始化技能目录
///
/// 创建技能工作目录并生成 README 文件。
/// 同时确保所有内置预加载技能已创建。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录
///
/// # 返回值
///
/// 成功返回 Ok(())
///
/// # 创建的内容
///
/// 1. 技能目录（如果不存在）
/// 2. README.md（如果不存在），包含：
///    - 技能目录结构说明
///    - SKILL.toml 格式示例
///    - SKILL.md 格式说明
///    - 安装社区技能的命令示例
/// 3. 内置预加载技能
/// 4. 下载策略配置
///
/// # 示例
///
/// ```ignore
/// init_skills_dir(&workspace_dir)?;
/// // 技能目录已准备就绪
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn init_skills_dir(workspace_dir: &Path) -> Result<()> {
    let dir = skills_dir(workspace_dir);
    std::fs::create_dir_all(&dir)?;

    // 创建 README.md（如果不存在）
    let readme = dir.join("README.md");
    if !readme.exists() {
        std::fs::write(
            &readme,
            "# VibeWindow Skills\n\n\
             Each subdirectory is a skill. Create a `SKILL.toml` or `SKILL.md` file inside.\n\n\
             ## SKILL.toml format\n\n\
             ```toml\n\
             [skill]\n\
             name = \"my-skill\"\n\
             description = \"What this skill does\"\n\
             version = \"0.1.0\"\n\
             author = \"your-name\"\n\
             tags = [\"productivity\", \"automation\"]\n\n\
             [[tools]]\n\
             name = \"my_tool\"\n\
             description = \"What this tool does\"\n\
             kind = \"shell\"\n\
             command = \"echo hello\"\n\
             ```\n\n\
             ## SKILL.md format (simpler)\n\n\
             Just write a markdown file with instructions for the agent.\n\
             The agent will read it and follow the instructions.\n\n\
             ## Installing community skills\n\n\
             ```bash\n\
             vibewindow skills install <source>\n\
             vibewindow skills list\n\
             ```\n",
        )?;
    }

    // 确保内置技能存在
    ensure_builtin_preloaded_skills(&dir)?;

    // 初始化下载策略
    let _ = load_or_init_skill_download_policy(&dir)?;

    Ok(())
}

/// WASM 平台的技能目录初始化（空操作）
///
/// 在 WebAssembly 环境中，技能目录初始化不支持或不需要。
#[cfg(target_arch = "wasm32")]
pub fn init_skills_dir(_workspace_dir: &Path) -> Result<()> {
    Ok(())
}

/// 判断源字符串是否为 Git 仓库 URL
///
/// 检查源字符串是否匹配任何 Git 协议格式。
///
/// # 参数
///
/// * `source` - 源字符串
///
/// # 返回值
///
/// 如果是 Git 仓库 URL 返回 true
///
/// # 支持的协议
///
/// - HTTPS: `https://...`
/// - HTTP: `http://...`
/// - SSH: `ssh://...`
/// - Git: `git://...`
/// - SCP 风格: `user@host:path`
fn is_git_source(source: &str) -> bool {
    (is_git_scheme_source(source, "https://") && source.ends_with(".git"))
        || (is_git_scheme_source(source, "http://") && source.ends_with(".git"))
        || is_git_scheme_source(source, "ssh://")
        || is_git_scheme_source(source, "git://")
        || is_git_scp_source(source)
}

/// 检查是否为指定协议的 Git URL
///
/// 验证源字符串是否以指定协议开头，并包含有效的主机名。
///
/// # 参数
///
/// * `source` - 源字符串
/// * `scheme` - 协议前缀（如 "https://"）
///
/// # 返回值
///
/// 如果匹配协议且包含有效主机名返回 true
///
/// # 验证逻辑
///
/// 1. 检查是否以协议前缀开头
/// 2. 协议后不能为空或以 / 开头
/// 3. 主机名不能为空
fn is_git_scheme_source(source: &str, scheme: &str) -> bool {
    let Some(rest) = source.strip_prefix(scheme) else {
        return false;
    };
    if rest.is_empty() || rest.starts_with('/') {
        return false;
    }

    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !host.is_empty()
}

/// 检查是否为 SCP 风格的 Git URL
///
/// SCP 风格使用 `user@host:path` 格式，例如：
/// - `git@github.com:user/repo.git`
///
/// # 参数
///
/// * `source` - 源字符串
///
/// # 返回值
///
/// 如果是有效的 SCP 风格 Git URL 返回 true
///
/// # 验证逻辑
///
/// 1. 必须包含 `:` 分隔符
/// 2. 冒号后不能为空
/// 3. 不能包含 `://`（那是标准 URL 格式）
/// 4. 必须包含 `@` 分隔用户和主机
/// 5. 用户名和主机名不能为空
/// 6. 用户名和主机名不能包含路径分隔符
fn is_git_scp_source(source: &str) -> bool {
    let Some((user_host, remote_path)) = source.split_once(':') else {
        return false;
    };
    if remote_path.is_empty() {
        return false;
    }
    if source.contains("://") {
        return false;
    }

    let Some((user, host)) = user_host.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && !host.is_empty()
        && !user.contains('/')
        && !user.contains('\\')
        && !host.contains('/')
        && !host.contains('\\')
}

/// 处理技能相关的 CLI 命令
///
/// 根据命令类型执行相应的技能操作，包括列出、审计、安装和删除技能。
///
/// # 参数
///
/// * `command` - 技能命令枚举
/// * `config` - 技能运行时配置
///
/// # 返回值
///
/// 成功返回 Ok(())，失败返回错误
///
/// # 支持的命令
///
/// ## List - 列出已安装的技能
///
/// 显示所有已安装技能的名称、版本、描述、工具和标签。
///
/// ## Audit - 审计技能
///
/// 对指定技能目录执行安全审计，检查潜在的安全风险。
/// 参数可以是本地路径或已安装技能的名称。
///
/// ## Install - 安装技能
///
/// 从指定来源安装技能：
/// - 本地路径
/// - Git 仓库 URL
/// - skills.sh URL
///
/// 安装过程包括：
/// 1. 解析源别名
/// 2. 验证域名信任
/// 3. 根据源类型执行安装
/// 4. 执行安全审计
///
/// ## Remove - 删除技能
///
/// 删除指定的技能。包含路径遍历防护：
/// - 拒绝包含 `..` 的名称
/// - 拒绝包含路径分隔符的名称
/// - 验证最终路径在技能目录内
///
/// # 错误
///
/// - 技能未找到
/// - 安全审计失败
/// - 安装/删除失败
/// - 路径遍历尝试
#[allow(clippy::too_many_lines)]
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_command(command: SkillCommands, config: &SkillRuntimeConfig) -> Result<()> {
    let workspace_dir = &config.workspace_dir;
    let security = SkillSecurityConfig::default();
    match command {
        // 列出已安装的技能
        SkillCommands::List => {
            let skills = load_skills_full_with_config(workspace_dir, config);
            if skills.is_empty() {
                println!("No skills installed.");
                println!();
                println!(
                    "  Create one: mkdir -p {}",
                    workspace_dir.join("skills").join("my-skill").display()
                );
                println!(
                    "              echo '# My Skill' > {}",
                    workspace_dir.join("skills").join("my-skill").join("SKILL.md").display()
                );
                println!();
                println!("  Or install: vibewindow skills install <source>");
            } else {
                println!("Installed skills ({}):", skills.len());
                println!();
                for skill in &skills {
                    println!(
                        "  {} {} — {}",
                        skill.name,
                        format!("v{}", skill.version),
                        skill.description
                    );
                    if !skill.tools.is_empty() {
                        println!(
                            "    Tools: {}",
                            skill
                                .tools
                                .iter()
                                .map(|t| t.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    if !skill.tags.is_empty() {
                        println!("    Tags:  {}", skill.tags.join(", "));
                    }
                }
            }
            println!();
            Ok(())
        }

        // 审计技能
        SkillCommands::Audit { source } => {
            // 确定审计目标：本地路径或已安装技能
            let source_path = PathBuf::from(&source);
            let target = if source_path.exists() {
                source_path
            } else {
                skills_dir(workspace_dir).join(&source)
            };

            if !target.exists() {
                anyhow::bail!("Skill source or installed skill not found: {source}");
            }

            // 执行审计
            let report = audit::audit_skill_directory_with_security(&target, &security)?;
            if report.is_clean() {
                println!(
                    "  {} Skill audit passed for {} ({} files scanned).",
                    "✓",
                    target.display(),
                    report.files_scanned
                );
                return Ok(());
            }

            // 审计失败，显示发现的问题
            println!("  {} Skill audit failed for {}", "✗", target.display());
            for finding in report.findings {
                println!("    - {finding}");
            }
            anyhow::bail!("Skill audit failed.");
        }

        // 安装技能
        SkillCommands::Install { source } => {
            println!("Installing skill from: {source}");

            // 确保技能目录已初始化
            init_skills_dir(workspace_dir)?;
            let skills_path = skills_dir(workspace_dir);

            // 加载下载策略
            let mut download_policy = load_or_init_skill_download_policy(&skills_path)?;
            let source = source.trim().to_string();

            // 解析源别名
            let resolved_source = resolve_skill_source_alias(&source, &download_policy);
            if resolved_source != source {
                println!("  Using configured alias '{source}' -> {resolved_source}");
            }

            // 验证域名信任
            ensure_source_domain_trust(&resolved_source, &mut download_policy, &skills_path)?;

            // 根据源类型执行安装
            if is_skills_sh_source(&resolved_source) {
                // skills.sh 平台源
                let (installed_dir, files_scanned) =
                    install_skills_sh_source(&resolved_source, &skills_path, &security).with_context(
                        || format!("failed to install skills.sh skill: {resolved_source}"),
                    )?;
                println!(
                    "  {} Skill installed from skills.sh: {} ({} files scanned)",
                    "✓",
                    installed_dir.display(),
                    files_scanned
                );
            } else if is_git_source(&resolved_source) {
                // Git 仓库源
                let (installed_dir, files_scanned) =
                    install_git_skill_source(&resolved_source, &skills_path, &security).with_context(
                        || format!("failed to install git skill source: {resolved_source}"),
                    )?;
                println!(
                    "  {} Skill installed and audited: {} ({} files scanned)",
                    "✓",
                    installed_dir.display(),
                    files_scanned
                );
            } else {
                // 本地路径源
                let (dest, files_scanned) =
                    install_local_skill_source(&resolved_source, &skills_path, &security).with_context(
                        || format!("failed to install local skill source: {resolved_source}"),
                    )?;
                println!(
                    "  {} Skill installed and audited: {} ({} files scanned)",
                    "✓",
                    dest.display(),
                    files_scanned
                );
            }

            println!("  Security audit completed successfully.");
            Ok(())
        }

        // 删除技能
        SkillCommands::Remove { name } => {
            // 路径遍历防护：拒绝可疑字符
            if name.contains("..") || name.contains('/') || name.contains('\\') {
                anyhow::bail!("Invalid skill name: {name}");
            }

            let skill_path = skills_dir(workspace_dir).join(&name);

            // 验证最终路径在技能目录内（防止符号链接逃逸）
            let canonical_skills = skills_dir(workspace_dir)
                .canonicalize()
                .unwrap_or_else(|_| skills_dir(workspace_dir));
            if let Ok(canonical_skill) = skill_path.canonicalize() {
                if !canonical_skill.starts_with(&canonical_skills) {
                    anyhow::bail!("Skill path escapes skills directory: {name}");
                }
            }

            // 检查技能是否存在
            if !skill_path.exists() {
                anyhow::bail!("Skill not found: {name}");
            }

            // 删除技能目录
            std::fs::remove_dir_all(&skill_path)?;
            println!("  {} Skill '{}' removed.", "✓", name);
            Ok(())
        }
    }
}

/// WASM 平台的命令处理（不支持）
///
/// 在 WebAssembly 环境中，技能 CLI 命令不可用。
#[cfg(target_arch = "wasm32")]
pub fn handle_command(_command: SkillCommands, _config: &SkillRuntimeConfig) -> Result<()> {
    anyhow::bail!("Skill CLI commands are not supported on Web/WASM");
}
use crate::app::agent::shell::git_std_command;
#[cfg(test)]
#[path = "install_tests.rs"]
mod install_tests;
