//! 技能安全审计模块
//!
//! 本模块提供技能文件系统的静态安全审计功能，用于检测潜在的安全风险。
//! 主要功能包括：
//! - 检查技能目录结构的合规性
//! - 扫描 Markdown 和 TOML 文件中的危险模式
//! - 验证文件链接的安全性和有效性
//! - 检测高风险命令和提示注入模式
//!
//! 审计 finding 由显式配置逐项启用；路径与文件系统操作错误始终返回错误。

use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use vw_config_types::skills::SkillSecurityConfig;

/// 文本文件的最大允许字节数（512 KB）
///
/// 超过此大小的 Markdown 和 TOML 文件将被标记为过大而无法审计，
/// 以防止资源耗尽和性能问题。
const MAX_TEXT_FILE_BYTES: u64 = 512 * 1024;

/// 技能审计报告
///
/// 包含对技能目录进行安全审计后生成的所有发现和统计信息。
/// 报告记录了扫描的文件数量以及发现的所有安全问题。
///
/// # 字段说明
/// - `files_scanned`: 已扫描的文件总数
/// - `findings`: 发现的所有问题的文本描述列表
///
/// # 示例
/// ```
/// use vibe_agent::skill::audit::SkillAuditReport;
///
/// let report = SkillAuditReport {
///     files_scanned: 10,
///     findings: vec!["发现潜在风险".to_string()],
/// };
///
/// if !report.is_clean() {
///     println!("审计发现问题: {}", report.summary());
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SkillAuditReport {
    /// 已扫描的文件数量
    pub files_scanned: usize,
    /// 发现的所有安全问题列表
    pub findings: Vec<String>,
}

impl SkillAuditReport {
    /// 检查审计报告是否干净（未发现任何问题）
    ///
    /// # 返回值
    /// - `true`: 未发现任何安全问题
    /// - `false`: 至少发现一个安全问题
    ///
    /// # 示例
    /// ```
    /// let clean_report = SkillAuditReport::default();
    /// assert!(clean_report.is_clean());
    /// ```
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// 生成审计报告的摘要文本
    ///
    /// 将所有发现的问题用分号连接成一个字符串。
    ///
    /// # 返回值
    /// 返回所有问题的连接字符串，如果没有任何发现则返回空字符串。
    ///
    /// # 示例
    /// ```
    /// let report = SkillAuditReport {
    ///     files_scanned: 5,
    ///     findings: vec!["问题1".to_string(), "问题2".to_string()],
    /// };
    /// assert_eq!(report.summary(), "问题1; 问题2");
    /// ```
    pub fn summary(&self) -> String {
        self.findings.join("; ")
    }
}

/// 审计技能目录的安全合规性
///
/// 使用默认关闭策略扫描指定的技能目录。需要启用规则时，调用
/// [`audit_skill_directory_with_security`]。
///
/// # 参数
/// - `skill_dir`: 技能目录的路径，必须是存在的目录
///
/// # 返回值
/// - `Ok(SkillAuditReport)`: 包含扫描结果和安全发现的审计报告
/// - `Err`: 如果技能目录不存在、不是目录或发生 I/O 错误
///
/// # 错误
/// - 技能源目录不存在
/// - 技能源不是目录
/// - 无法规范化路径
/// - 无法读取目录内容
///
/// # 示例
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::skill::audit::audit_skill_directory;
///
/// let skill_path = Path::new("/path/to/skill");
/// match audit_skill_directory(skill_path) {
///     Ok(report) => {
///         if report.is_clean() {
///             println!("技能通过安全审计");
///         } else {
///             println!("发现问题: {}", report.summary());
///         }
///     }
///     Err(e) => eprintln!("审计失败: {}", e),
/// }
/// ```
pub fn audit_skill_directory(skill_dir: &Path) -> Result<SkillAuditReport> {
    audit_skill_directory_with_security(skill_dir, &SkillSecurityConfig::default())
}

/// 使用显式安全策略审计技能目录。
pub fn audit_skill_directory_with_security(
    skill_dir: &Path,
    security: &SkillSecurityConfig,
) -> Result<SkillAuditReport> {
    // 验证技能目录是否存在
    if !skill_dir.exists() {
        bail!("Skill source does not exist: {}", skill_dir.display());
    }

    // 验证路径是否为目录
    if !skill_dir.is_dir() {
        bail!("Skill source must be a directory: {}", skill_dir.display());
    }

    // 获取规范化路径，解析所有符号链接和相对路径
    let canonical_root = skill_dir
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", skill_dir.display()))?;
    let mut report = SkillAuditReport::default();

    // 检查技能清单文件是否存在（至少需要 SKILL.md 或 SKILL.toml 之一）
    let has_manifest =
        canonical_root.join("SKILL.md").is_file() || canonical_root.join("SKILL.toml").is_file();
    if security.require_manifest && !has_manifest {
        report.findings.push(
            "Skill root must include SKILL.md or SKILL.toml for deterministic auditing."
                .to_string(),
        );
    }

    // 使用深度优先遍历收集所有路径，并对每个路径进行审计
    for path in collect_paths_depth_first(&canonical_root)? {
        report.files_scanned += 1;
        audit_path(&canonical_root, &path, security, &mut report)?;
    }

    Ok(report)
}

/// 审计开放技能的 Markdown 文件
///
/// 使用默认关闭策略审计仓库中的单个 Markdown 文件。该函数仍会无条件验证
/// 文件路径位于仓库根目录内；需要启用内容规则时，调用
/// [`audit_open_skill_markdown_with_security`]。
///
/// # 参数
/// - `path`: Markdown 文件的路径，必须是存在的文件
/// - `repo_root`: 仓库根目录的路径，用于确保文件不会逃逸仓库边界
///
/// # 返回值
/// - `Ok(SkillAuditReport)`: 包含扫描结果和安全发现的审计报告
/// - `Err`: 如果文件不存在、路径逃逸仓库根目录或发生 I/O 错误
///
/// # 错误
/// - Markdown 文件不存在
/// - 文件路径逃逸仓库根目录
/// - 无法规范化路径
///
/// # 安全性
/// 该函数确保被审计的文件位于仓库边界内，防止路径遍历攻击。
///
/// # 示例
/// ```no_run
/// use std::path::Path;
/// use vibe_agent::skill::audit::audit_open_skill_markdown;
///
/// let repo = Path::new("/path/to/repo");
/// let skill_md = repo.join("docs/skills/example.md");
/// match audit_open_skill_markdown(&skill_md, repo) {
///     Ok(report) => println!("扫描了 {} 个文件", report.files_scanned),
///     Err(e) => eprintln!("审计失败: {}", e),
/// }
/// ```
pub fn audit_open_skill_markdown(path: &Path, repo_root: &Path) -> Result<SkillAuditReport> {
    audit_open_skill_markdown_with_security(path, repo_root, &SkillSecurityConfig::default())
}

/// 使用显式安全策略审计开放式技能 Markdown 文件。
pub fn audit_open_skill_markdown_with_security(
    path: &Path,
    repo_root: &Path,
    security: &SkillSecurityConfig,
) -> Result<SkillAuditReport> {
    // 验证 Markdown 文件是否存在
    if !path.exists() {
        bail!("Open-skill markdown not found: {}", path.display());
    }

    // 获取仓库和文件的规范化路径
    let canonical_repo = repo_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", repo_root.display()))?;
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;

    // 安全检查：确保文件路径不逃逸仓库根目录
    if !canonical_path.starts_with(&canonical_repo) {
        bail!("Open-skill markdown escapes repository root: {}", path.display());
    }

    // 创建审计报告并扫描 Markdown 文件
    let mut report = SkillAuditReport { files_scanned: 1, findings: Vec::new() };
    audit_markdown_file(&canonical_repo, &canonical_path, security, &mut report)?;
    Ok(report)
}

/// 使用深度优先遍历收集目录下的所有路径
///
/// 该函数递归遍历目录树，收集所有文件和目录的路径。
/// 使用栈实现的非递归深度优先遍历，避免递归调用栈溢出。
///
/// # 参数
/// - `root`: 要遍历的根目录路径
///
/// # 返回值
/// - `Ok(Vec<PathBuf>)`: 所有路径的列表，按深度优先顺序排列
/// - `Err`: 如果无法读取目录内容
///
/// # 实现细节
/// - 使用栈实现非递归的深度优先遍历
/// - 子目录按排序顺序处理，确保遍历顺序的一致性
/// - 子项逆序入栈，保证按正序出栈
fn collect_paths_depth_first(root: &Path) -> Result<Vec<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    let mut out = Vec::new();

    while let Some(current) = stack.pop() {
        // 将当前路径加入输出列表
        out.push(current.clone());

        // 如果不是目录，跳过子项处理
        if !current.is_dir() {
            continue;
        }

        // 收集当前目录下的所有子项
        let mut children = Vec::new();
        for entry in fs::read_dir(&current)
            .with_context(|| format!("failed to read directory {}", current.display()))?
        {
            let entry = entry?;
            children.push(entry.path());
        }

        // 排序后逆序入栈，确保按正序出栈
        children.sort();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }

    Ok(out)
}

/// 审计单个路径的安全性
///
/// 对文件或目录路径进行安全检查，包括：
/// - 检测符号链接（在技能中不允许）
/// - 检查文件大小限制
/// - 审计 Markdown 和 TOML 文件的内容
///
/// # 参数
/// - `root`: 技能的根目录路径，用于计算相对路径
/// - `path`: 要审计的文件或目录路径
/// - `report`: 用于记录发现问题的审计报告
///
/// # 返回值
/// - `Ok(())`: 审计完成（可能在报告中记录了问题）
/// - `Err`: 如果无法读取文件元数据或内容
fn audit_path(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    // 获取文件元数据（使用 symlink_metadata 避免跟随符号链接）
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    let rel = relative_display(root, path);

    // 检查是否为符号链接（技能中不允许符号链接）
    if metadata.file_type().is_symlink() {
        if security.reject_symlinks {
            report.findings.push(format!("{rel}: symlinks are not allowed in installed skills."));
        }
        return Ok(());
    }

    // 目录不需要进一步检查
    if metadata.is_dir() {
        return Ok(());
    }

    // 检查是否为不支持的脚本文件
    if security.reject_script_files && is_unsupported_script_file(path) {
        report
            .findings
            .push(format!("{rel}: script-like files are blocked by skill security policy."));
    }

    // 检查文本文件大小是否超过限制
    if security.reject_oversized_text_files
        && metadata.len() > MAX_TEXT_FILE_BYTES
        && (is_markdown_file(path) || is_toml_file(path))
    {
        report.findings.push(format!(
            "{rel}: file is too large for static audit (>{MAX_TEXT_FILE_BYTES} bytes)."
        ));
        return Ok(());
    }

    // 根据文件类型进行相应的审计
    if is_markdown_file(path) {
        audit_markdown_file(root, path, security, report)?;
    } else if is_toml_file(path) {
        audit_manifest_file(root, path, security, report)?;
    }

    Ok(())
}

/// 审计 Markdown 文件的安全性
///
/// 扫描 Markdown 文件内容，检测：
/// - 高风险命令模式（如提示注入、危险命令等）
/// - 不安全的链接（远程链接、脚本文件链接等）
///
/// # 参数
/// - `root`: 技能的根目录路径
/// - `path`: Markdown 文件路径
/// - `report`: 用于记录发现问题的审计报告
///
/// # 返回值
/// - `Ok(())`: 审计完成
/// - `Err`: 如果无法读取文件内容
fn audit_markdown_file(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    // 读取 Markdown 文件内容
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read markdown file {}", path.display()))?;
    let rel = relative_display(root, path);

    // 检测高风险代码片段
    if security.detect_high_risk_content {
        if let Some(pattern) = detect_high_risk_snippet(&content) {
            report.findings.push(format!("{rel}: detected high-risk command pattern ({pattern})."));
        }
    }

    // 提取并审计所有 Markdown 链接
    if security.reject_script_files || security.validate_markdown_links {
        for raw_target in extract_markdown_links(&content) {
            audit_markdown_link_target(root, path, &raw_target, security, report);
        }
    }

    Ok(())
}

/// 审计 TOML 清单文件的安全性
///
/// 解析 TOML 清单文件，验证：
/// - TOML 语法的有效性
/// - 工具命令中是否包含危险的 shell 链接操作符
/// - 工具命令是否匹配高风险模式
/// - 提示内容是否包含高风险模式
///
/// # 参数
/// - `root`: 技能的根目录路径
/// - `path`: TOML 清单文件路径
/// - `report`: 用于记录发现问题的审计报告
///
/// # 返回值
/// - `Ok(())`: 审计完成
/// - `Err`: 如果无法读取文件内容
fn audit_manifest_file(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    // 读取 TOML 文件内容
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read TOML manifest {}", path.display()))?;
    let rel = relative_display(root, path);

    // 解析 TOML 文件
    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            if security.validate_manifest {
                report.findings.push(format!("{rel}: invalid TOML manifest ({err})."));
            }
            return Ok(());
        }
    };

    // 审计工具定义
    if let Some(tools) = parsed.get("tools").and_then(toml::Value::as_array) {
        for (idx, tool) in tools.iter().enumerate() {
            let command = tool.get("command").and_then(toml::Value::as_str);
            let kind = tool.get("kind").and_then(toml::Value::as_str).unwrap_or("unknown");

            // 检查命令字段
            if let Some(command) = command {
                // 检查是否包含 shell 链接操作符
                if security.detect_shell_chaining && contains_shell_chaining(command) {
                    report.findings.push(format!(
                        "{rel}: tools[{idx}].command uses shell chaining operators, which are blocked."
                    ));
                }
                // 检查是否匹配高风险模式
                if security.detect_high_risk_content {
                    if let Some(pattern) = detect_high_risk_snippet(command) {
                        report.findings.push(format!(
                            "{rel}: tools[{idx}].command matches high-risk pattern ({pattern})."
                        ));
                    }
                }
            } else if security.validate_manifest {
                report.findings.push(format!("{rel}: tools[{idx}] is missing a command field."));
            }

            // 检查 script/shell 类型工具的命令是否为空
            if security.validate_manifest
                && (kind.eq_ignore_ascii_case("script") || kind.eq_ignore_ascii_case("shell"))
                && command.is_some_and(|value| value.trim().is_empty())
            {
                report.findings.push(format!("{rel}: tools[{idx}] has an empty {kind} command."));
            }
        }
    }

    // 审计提示定义
    if let Some(prompts) = parsed.get("prompts").and_then(toml::Value::as_array) {
        for (idx, prompt) in prompts.iter().enumerate() {
            if security.detect_high_risk_content {
                if let Some(prompt) = prompt.as_str() {
                    // 检查提示内容是否包含高风险模式
                    if let Some(pattern) = detect_high_risk_snippet(prompt) {
                        report.findings.push(format!(
                            "{rel}: prompts[{idx}] contains high-risk pattern ({pattern})."
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

/// 审计 Markdown 链接的目标
///
/// 验证 Markdown 文件中的链接是否符合安全策略：
/// - 拒绝远程 Markdown 链接（http/https 链接到 .md 文件）
/// - 拒绝不支持的 URL 协议
/// - 拒绝绝对路径链接
/// - 拒绝指向脚本文件的链接
/// - 验证本地链接目标是否存在且在技能根目录内
///
/// # 参数
/// - `root`: 技能的根目录路径
/// - `source`: 包含链接的 Markdown 文件路径
/// - `raw`: 原始链接目标字符串
/// - `report`: 用于记录发现问题的审计报告
fn audit_markdown_link_target(
    root: &Path,
    source: &Path,
    raw: &str,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) {
    // 规范化链接目标
    let normalized = normalize_markdown_target(raw);

    // 跳过空链接和锚点链接
    if normalized.is_empty() || normalized.starts_with('#') {
        return;
    }

    let rel = relative_display(root, source);

    // 检查 URL 协议
    if let Some(scheme) = url_scheme(normalized) {
        // 允许 http/https/mailto 协议，但阻止指向 Markdown 的远程链接
        if matches!(scheme, "http" | "https" | "mailto") {
            if security.validate_markdown_links && has_markdown_suffix(normalized) {
                report.findings.push(format!(
                    "{rel}: remote markdown links are blocked by skill security audit ({normalized})."
                ));
            }
            return;
        }

        // 阻止不支持的 URL 协议
        if security.validate_markdown_links {
            report
                .findings
                .push(format!("{rel}: unsupported URL scheme in markdown link ({normalized})."));
        }
        return;
    }

    // 移除查询字符串和片段标识符
    let stripped = strip_query_and_fragment(normalized);
    if stripped.is_empty() {
        return;
    }

    // 检查是否为绝对路径
    if looks_like_absolute_path(stripped) {
        if security.validate_markdown_links {
            report
                .findings
                .push(format!("{rel}: absolute markdown link paths are not allowed ({normalized})."));
        }
        return;
    }

    // 检查是否指向脚本文件
    if security.reject_script_files && has_script_suffix(stripped) {
        report
            .findings
            .push(format!("{rel}: markdown links to script files are blocked ({normalized})."));
    }

    // 只对 Markdown 文件进行存在性检查
    if !security.validate_markdown_links || !has_markdown_suffix(stripped) {
        return;
    }

    // 获取源文件的父目录
    let Some(base_dir) = source.parent() else {
        report.findings.push(format!(
            "{rel}: failed to resolve parent directory for markdown link ({normalized})."
        ));
        return;
    };

    // 构建链接目标的完整路径
    let linked_path = base_dir.join(stripped);

    // 验证链接目标
    match linked_path.canonicalize() {
        Ok(canonical_target) => {
            // 检查链接目标是否在技能根目录内
            if !canonical_target.starts_with(root) {
                report
                    .findings
                    .push(format!("{rel}: markdown link escapes skill root ({normalized})."));
                return;
            }
            // 检查链接目标是否为文件
            if !canonical_target.is_file() {
                report
                    .findings
                    .push(format!("{rel}: markdown link must point to a file ({normalized})."));
            }
        }
        Err(_) => {
            // 检查是否为跨技能引用（指向当前技能目录外的链接）
            // 跨技能引用允许指向缺失的文件，因为被引用的技能可能未安装。
            // 这在开放技能中很常见，技能之间相互引用但不一定所有技能都存在。
            if is_cross_skill_reference(stripped) {
                // 允许缺失的跨技能引用 - 这对开放技能是有效的
                return;
            }
            report
                .findings
                .push(format!("{rel}: markdown link points to a missing file ({normalized})."));
        }
    }
}

/// 检查链接目标是否为跨技能引用
///
/// 跨技能引用可以采用多种形式：
/// 1. 父目录遍历：`../other-skill/SKILL.md`
/// 2. 裸技能文件名：`other-skill.md`（引用另一个技能的 markdown）
/// 3. 显式相对路径：`./other-skill.md`
///
/// # 参数
/// - `target`: 链接目标字符串
///
/// # 返回值
/// - `true`: 目标看起来像是跨技能引用
/// - `false`: 目标不是跨技能引用
///
/// # 示例
/// ```
/// use vibe_agent::skill::audit::is_cross_skill_reference;
///
/// assert!(is_cross_skill_reference("../other-skill/SKILL.md"));
/// assert!(is_cross_skill_reference("other-skill.md"));
/// assert!(is_cross_skill_reference("./other-skill.md"));
/// assert!(!is_cross_skill_reference("subdirectory/file.md"));
/// ```
pub fn is_cross_skill_reference(target: &str) -> bool {
    let path = Path::new(target);

    // 情况 1：使用父目录遍历（..）
    if path.components().any(|component| component == Component::ParentDir) {
        return true;
    }

    // 情况 2 和 3：裸文件名或 ./filename 形式的技能引用
    // 技能引用通常是一个裸的 markdown 文件名，如 "skill-name.md"
    // 不包含目录分隔符（或仅有 "./" 前缀）
    let stripped = target.strip_prefix("./").unwrap_or(target);

    // 如果只是文件名（无路径分隔符）且有 .md 扩展名，
    // 则很可能是跨技能引用
    !stripped.contains('/') && !stripped.contains('\\') && has_markdown_suffix(stripped)
}

/// 生成路径的相对显示字符串
///
/// 计算给定路径相对于根目录的相对路径字符串。
/// 如果路径不在根目录下，则返回完整路径。
///
/// # 参数
/// - `root`: 根目录路径
/// - `path`: 要计算相对路径的路径
///
/// # 返回值
/// 返回相对路径的字符串表示，如果路径就是根目录则返回 "."
fn relative_display(root: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(root) {
        if rel.as_os_str().is_empty() {
            return ".".to_string();
        }
        return rel.display().to_string();
    }
    path.display().to_string()
}

/// 检查文件是否为 Markdown 文件
///
/// 通过检查文件扩展名判断是否为 Markdown 文件。
/// 支持的扩展名：.md、.markdown（不区分大小写）
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回值
/// - `true`: 文件是 Markdown 文件
/// - `false`: 文件不是 Markdown 文件
fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
}

/// 检查文件是否为 TOML 文件
///
/// 通过检查文件扩展名判断是否为 TOML 文件。
///
/// # 参数
/// - `path`: 文件路径
///
/// # 返回值
/// - `true`: 文件是 TOML 文件
/// - `false`: 文件不是 TOML 文件
fn is_toml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
}

/// 检查文件是否为不支持的脚本文件。
fn is_unsupported_script_file(path: &Path) -> bool {
    has_script_suffix(path.to_string_lossy().as_ref()) || has_shell_shebang(path)
}

/// 检查字符串是否具有脚本文件后缀。
fn has_script_suffix(raw: &str) -> bool {
    let lowered = raw.to_ascii_lowercase();
    let script_suffixes = [".sh", ".bash", ".zsh", ".ksh", ".fish", ".ps1", ".bat", ".cmd"];
    script_suffixes.iter().any(|suffix| lowered.ends_with(suffix))
}

/// 检查文件是否包含 shell shebang。
fn has_shell_shebang(path: &Path) -> bool {
    let Ok(content) = fs::read(path) else {
        return false;
    };
    let prefix = &content[..content.len().min(128)];
    let shebang = String::from_utf8_lossy(prefix).to_ascii_lowercase();
    shebang.starts_with("#!")
        && (shebang.contains("sh")
            || shebang.contains("bash")
            || shebang.contains("zsh")
            || shebang.contains("pwsh")
            || shebang.contains("powershell"))
}

/// 从 Markdown 内容中提取所有链接
///
/// 使用正则表达式匹配 Markdown 链接语法 `[text](target)`，
/// 并提取所有链接目标。
///
/// # 参数
/// - `content`: Markdown 文件内容
///
/// # 返回值
/// 返回所有链接目标字符串的列表
///
/// # 实现细节
/// - 使用 OnceLock 缓存编译后的正则表达式
/// - 匹配格式：`[任意文本](链接目标)`
fn extract_markdown_links(content: &str) -> Vec<String> {
    static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();
    let regex = MARKDOWN_LINK_RE.get_or_init(|| {
        Regex::new(r#"\[[^\]]*\]\(([^)]+)\)"#).expect("markdown link regex must compile")
    });

    regex
        .captures_iter(content)
        .filter_map(|capture| capture.get(1))
        .map(|target| target.as_str().trim().to_string())
        .collect()
}

/// 规范化 Markdown 链接目标
///
/// 处理链接目标字符串：
/// - 去除前后空白
/// - 去除可选的尖括号包围（<>）
/// - 只取第一个空白前的内容（处理标题部分）
///
/// # 参数
/// - `raw_target`: 原始链接目标字符串
///
/// # 返回值
/// 返回规范化后的链接目标字符串切片
fn normalize_markdown_target(raw_target: &str) -> &str {
    let trimmed = raw_target.trim();
    let trimmed = trimmed.strip_prefix('<').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('>').unwrap_or(trimmed);
    trimmed.split_whitespace().next().unwrap_or_default()
}

/// 移除字符串中的查询字符串和片段标识符
///
/// 从 URL 或路径中移除 ? 后的查询字符串和 # 后的片段标识符。
///
/// # 参数
/// - `input`: 输入字符串
///
/// # 返回值
/// 返回移除查询字符串和片段后的字符串切片
///
/// # 示例
/// ```
/// use vibe_agent::skill::audit::strip_query_and_fragment;
///
/// assert_eq!(strip_query_and_fragment("file.md?query=1"), "file.md");
/// assert_eq!(strip_query_and_fragment("file.md#section"), "file.md");
/// assert_eq!(strip_query_and_fragment("file.md?query=1#section"), "file.md");
/// ```
fn strip_query_and_fragment(input: &str) -> &str {
    let mut end = input.len();
    if let Some(idx) = input.find('#') {
        end = end.min(idx);
    }
    if let Some(idx) = input.find('?') {
        end = end.min(idx);
    }
    &input[..end]
}

/// 提取 URL 协议部分
///
/// 从字符串中提取 URL 协议（scheme）部分。
/// 协议必须符合 URI 规范：字母数字、加号、减号、点号。
///
/// # 参数
/// - `target`: 可能包含 URL 的字符串
///
/// # 返回值
/// - `Some(&str)`: 提取到的协议名称
/// - `None`: 字符串不包含有效的 URL 协议
///
/// # 示例
/// ```
/// use vibe_agent::skill::audit::url_scheme;
///
/// assert_eq!(url_scheme("https://example.com"), Some("https"));
/// assert_eq!(url_scheme("file.md"), None);
/// ```
fn url_scheme(target: &str) -> Option<&str> {
    let (scheme, rest) = target.split_once(':')?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    // 验证协议格式是否合法
    if !scheme.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return None;
    }
    Some(scheme)
}

/// 检查字符串是否看起来像绝对路径
///
/// 判断路径字符串是否为绝对路径，包括：
/// - Unix 绝对路径（以 / 开头）
/// - Windows 绝对路径（如 C:\foo）
/// - 用户主目录路径（以 ~/ 开头）
///
/// # 参数
/// - `target`: 路径字符串
///
/// # 返回值
/// - `true`: 字符串看起来像绝对路径
/// - `false`: 字符串不是绝对路径
///
/// # 注意
/// 该函数故意不拒绝以 ".." 开头的路径。
/// 包含父目录引用的相对路径（如 "../other-skill/SKILL.md"）
/// 会被传递给后续的规范化检查，以正确验证它们是否解析到技能根目录内。
/// 这允许开放技能中的跨技能引用，同时仍然保持安全性。
fn looks_like_absolute_path(target: &str) -> bool {
    let path = Path::new(target);
    if path.is_absolute() {
        return true;
    }

    // 检测 Windows 绝对路径前缀，如 C:\foo
    let bytes = target.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }

    // 拒绝以 ~/" 开头的路径，因为它们会绕过工作区边界
    if target.starts_with("~/") {
        return true;
    }

    false
}

/// 检查字符串是否具有 Markdown 文件后缀
///
/// 检查字符串是否以 .md 或 .markdown 结尾（不区分大小写）。
///
/// # 参数
/// - `target`: 要检查的字符串
///
/// # 返回值
/// - `true`: 字符串具有 Markdown 后缀
/// - `false`: 字符串没有 Markdown 后缀
fn has_markdown_suffix(target: &str) -> bool {
    let lowered = target.to_ascii_lowercase();
    lowered.ends_with(".md") || lowered.ends_with(".markdown")
}

/// 检查命令字符串是否包含 shell 链接操作符
///
/// 检测危险的 shell 命令链接操作符，这些操作符可以用于：
/// - 链接多个命令（&&、||、;）
/// - 命令替换（`、$()）
/// - 换行执行（\n、\r）
///
/// # 参数
/// - `command`: 命令字符串
///
/// # 返回值
/// - `true`: 命令包含链式操作符
/// - `false`: 命令不包含链式操作符
fn contains_shell_chaining(command: &str) -> bool {
    ["&&", "||", ";", "\n", "\r", "`", "$("].iter().any(|needle| command.contains(needle))
}

/// 检测内容中的高风险模式
///
/// 使用预定义的正则表达式模式匹配内容中的高风险代码片段。
/// 检测的模式类型包括：
/// - 提示注入攻击（覆盖/泄露系统指令）
/// - 凭证钓鱼（密码、API 密钥等）
/// - 远程代码执行（curl/wget 管道到 shell）
/// - 混淆执行（base64 解码执行）
/// - 破坏性命令（rm -rf /、dd、mkfs）
/// - 反弹 shell（netcat 远程执行）
/// - fork 炸弹
///
/// # 参数
/// - `content`: 要检查的内容字符串
///
/// # 返回值
/// - `Some(&'static str)`: 匹配到的高风险模式标签
/// - `None`: 未检测到高风险模式
///
/// # 实现细节
/// - 使用 OnceLock 缓存编译后的正则表达式，避免重复编译
/// - 所有模式匹配不区分大小写（(?i) 标志）
fn detect_high_risk_snippet(content: &str) -> Option<&'static str> {
    static HIGH_RISK_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    let patterns = HIGH_RISK_PATTERNS.get_or_init(|| {
        vec![
            // 检测提示注入：覆盖/绕过系统指令
            (
                Regex::new(
                    r"(?im)\b(?:ignore|disregard|override|bypass)\b[^\n]{0,140}\b(?:previous|earlier|system|safety|security)\s+instructions?\b",
                )
                .expect("regex"),
                "prompt-injection-override",
            ),
            // 检测提示注入：泄露系统提示词
            (
                Regex::new(
                    r"(?im)\b(?:reveal|show|exfiltrate|leak)\b[^\n]{0,140}\b(?:system prompt|developer instructions|hidden prompt|secret instructions)\b",
                )
                .expect("regex"),
                "prompt-injection-exfiltration",
            ),
            // 检测钓鱼：凭证收集
            (
                Regex::new(
                    r"(?im)\b(?:ask|request|collect|harvest|obtain)\b[^\n]{0,120}\b(?:password|api[_ -]?key|private[_ -]?key|seed phrase|recovery phrase|otp|2fa)\b",
                )
                .expect("regex"),
                "phishing-credential-harvest",
            ),
            // 检测远程代码执行：curl 管道到 shell
            (
                Regex::new(r"(?im)\bcurl\b[^\n|]{0,200}\|\s*(?:sh|bash|zsh)\b").expect("regex"),
                "curl-pipe-shell",
            ),
            // 检测远程代码执行：wget 管道到 shell
            (
                Regex::new(r"(?im)\bwget\b[^\n|]{0,200}\|\s*(?:sh|bash|zsh)\b").expect("regex"),
                "wget-pipe-shell",
            ),
            // 检测 PowerShell 远程执行
            (
                Regex::new(r"(?im)\b(?:invoke-expression|iex)\b").expect("regex"),
                "powershell-iex",
            ),
            // 检测破坏性命令：删除根目录
            (
                Regex::new(r"(?im)\brm\s+-rf\s+/").expect("regex"),
                "destructive-rm-rf-root",
            ),
            // 检测反弹 shell：netcat 远程执行
            (
                Regex::new(r"(?im)\bnc(?:at)?\b[^\n]{0,120}\s-e\b").expect("regex"),
                "netcat-remote-exec",
            ),
            // 检测混淆执行：base64 解码后执行
            (
                Regex::new(r"(?im)\bbase64\s+-d\b[^\n|]{0,220}\|\s*(?:sh|bash|zsh)\b")
                    .expect("regex"),
                "obfuscated-base64-exec",
            ),
            // 检测磁盘覆写：dd 命令
            (
                Regex::new(r"(?im)\bdd\s+if=").expect("regex"),
                "disk-overwrite-dd",
            ),
            // 检测文件系统格式化
            (
                Regex::new(r"(?im)\bmkfs(?:\.[a-z0-9]+)?\b").expect("regex"),
                "filesystem-format",
            ),
            // 检测 fork 炸弹
            (
                Regex::new(r"(?im):\(\)\s*\{\s*:\|\:&\s*\};:").expect("regex"),
                "fork-bomb",
            ),
        ]
    });

    patterns.iter().find_map(|(regex, label)| regex.is_match(content).then_some(*label))
}
#[cfg(test)]
#[path = "audit_tests.rs"]
mod audit_tests;
