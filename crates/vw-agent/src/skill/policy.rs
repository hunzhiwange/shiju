//! # 技能下载策略管理模块
//!
//! 本模块负责管理技能（Skill）下载的安全策略，包括：
//! - **信任域名管理**：维护允许下载技能的受信任域名列表
//! - **阻止域名管理**：维护禁止下载技能的黑名单域名列表
//! - **别名映射**：为技能源地址提供简短的别名映射
//! - **交互式信任确认**：首次遇到新域名时，在交互模式下提示用户确认
//!
//! ## 安全机制
//!
//! 为防止从恶意源下载技能，本模块实现了域名白名单/黑名单机制：
//! 1. 下载前检查源域名是否在阻止列表中，若在则拒绝下载
//! 2. 检查源域名是否在信任列表中，若在则允许下载
//! 3. 首次遇到的域名会在交互模式下询问用户是否信任
//! 4. 非交互模式下遇到未知域名将拒绝下载
//!
//! ## 配置持久化
//!
//! 策略配置以 TOML 格式存储在技能目录下的 `skill_download_policy.toml` 文件中。

use crate::app::agent::skill::constants::SKILL_DOWNLOAD_POLICY_FILE;
use crate::app::agent::skill::source::{extract_link_host, source_urls_for_trust_check};
use crate::app::agent::skill::types::SkillDownloadPolicy;
use anyhow::{Context, Result};
use std::io::IsTerminal;
use std::path::Path;

/// 规范化域名条目
///
/// 将用户输入的域名字符串转换为标准化的域名格式，处理过程包括：
/// 1. 去除首尾空白字符
/// 2. 转换为小写
/// 3. 移除 `https://` 或 `http://` 协议前缀
/// 4. 移除路径、查询参数和片段（只保留主机部分）
/// 5. 移除通配符前缀（如 `*.`）和开头的点
/// 6. 移除端口号
///
/// # 参数
///
/// * `raw` - 原始的域名输入字符串，可能包含协议、路径、端口等
///
/// # 返回值
///
/// 返回规范化后的域名字符串。如果输入为空或仅包含空白字符，返回空字符串。
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_domain_entry("  HTTPS://GitHub.COM/foo/bar  "), "github.com");
/// assert_eq!(normalize_domain_entry("*.example.com"), "example.com");
/// assert_eq!(normalize_domain_entry("example.com:8080"), "example.com");
/// ```
fn normalize_domain_entry(raw: &str) -> String {
    let mut s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return s;
    }
    if let Some(rest) = s.strip_prefix("https://") {
        s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix("http://") {
        s = rest.to_string();
    }
    s = s.split(&['/', '?', '#'][..]).next().unwrap_or("").trim().to_string();
    s = s.trim_start_matches("*.").trim_start_matches('.').to_string();
    if let Some((host, _port)) = s.split_once(':') {
        return host.to_string();
    }
    s
}

/// 规范化域名列表
///
/// 对域名列表进行批量规范化处理，包括：
/// 1. 对每个条目应用 `normalize_domain_entry` 规范化
/// 2. 过滤掉空条目
/// 3. 按字母顺序排序
/// 4. 去除重复项
///
/// # 参数
///
/// * `entries` - 可变引用，待规范化的域名列表
///
/// # 副作用
///
/// 直接修改传入的向量，替换为规范化后的结果。
fn normalize_domain_list(entries: &mut Vec<String>) {
    let mut normalized = entries
        .iter()
        .map(|entry| normalize_domain_entry(entry))
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    *entries = normalized;
}

/// 检查主机名是否匹配受信任的域名
///
/// 判断给定的主机名是否属于受信任域名或其子域名。支持通配符匹配，
/// 即如果信任域名为 `example.com`，则 `sub.example.com` 也会被视为匹配。
///
/// # 参数
///
/// * `host` - 待检查的主机名（URL 中的主机部分）
/// * `trusted_domain` - 受信任的域名模式
///
/// # 返回值
///
/// 如果主机名完全匹配受信任域名，或是受信任域名的子域名，返回 `true`；
/// 否则返回 `false`。如果任一参数规范化后为空，返回 `false`。
///
/// # 示例
///
/// ```ignore
/// assert!(host_matches_trusted_domain("github.com", "github.com"));
/// assert!(host_matches_trusted_domain("api.github.com", "github.com"));
/// assert!(!host_matches_trusted_domain("github.io", "github.com"));
/// ```
pub(crate) fn host_matches_trusted_domain(host: &str, trusted_domain: &str) -> bool {
    let host = normalize_domain_entry(host);
    let trusted = normalize_domain_entry(trusted_domain);
    if host.is_empty() || trusted.is_empty() {
        return false;
    }
    host == trusted || host.ends_with(&format!(".{trusted}"))
}

/// 检查主机名是否匹配任一域名列表中的条目
///
/// 遍历域名列表，检查主机名是否匹配其中任一域名。
///
/// # 参数
///
/// * `host` - 待检查的主机名
/// * `entries` - 域名列表
///
/// # 返回值
///
/// 如果主机名匹配列表中的任一域名（包括子域名匹配），返回 `true`；
/// 否则返回 `false`。
fn host_matches_any_domain(host: &str, entries: &[String]) -> bool {
    entries.iter().any(|entry| host_matches_trusted_domain(host, entry))
}

/// 获取策略配置文件的完整路径
///
/// 根据技能目录路径构建策略配置文件的完整文件系统路径。
///
/// # 参数
///
/// * `skills_path` - 技能存储目录的路径
///
/// # 返回值
///
/// 返回策略配置文件的完整路径（`{skills_path}/{SKILL_DOWNLOAD_POLICY_FILE}`）。
fn download_policy_path(skills_path: &Path) -> std::path::PathBuf {
    skills_path.join(SKILL_DOWNLOAD_POLICY_FILE)
}

/// 加载或初始化技能下载策略
///
/// 从技能目录中加载策略配置文件，如果文件不存在则创建默认配置。
/// 加载后会自动合并系统预置的技能别名，并规范化域名列表。
///
/// # 参数
///
/// * `skills_path` - 技能存储目录的路径
///
/// # 返回值
///
/// 成功时返回 `SkillDownloadPolicy` 实例，失败时返回错误。
///
/// # 处理逻辑
///
/// 1. 如果配置文件不存在：
///    - 创建默认策略配置
///    - 保存到磁盘
///    - 返回默认策略
/// 2. 如果配置文件存在：
///    - 读取并解析 TOML 配置
///    - 合并缺失的预置技能别名
///    - 规范化信任域名和阻止域名列表
///    - 如果有变更则保存更新后的配置
///    - 返回策略实例
///
/// # 错误
///
/// - 文件读取失败
/// - 配置保存失败
///
/// # 示例
///
/// ```ignore
/// let policy = load_or_init_skill_download_policy(Path::new("/path/to/skills"))?;
/// println!("Trusted domains: {:?}", policy.trusted_domains);
/// ```
pub(crate) fn load_or_init_skill_download_policy(
    skills_path: &Path,
) -> Result<SkillDownloadPolicy> {
    let path = download_policy_path(skills_path);
    // 如果配置文件不存在，创建默认策略并保存
    if !path.exists() {
        let policy = SkillDownloadPolicy::default();
        save_skill_download_policy(skills_path, &policy)?;
        return Ok(policy);
    }

    // 读取并解析现有配置文件
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read skill download policy {}", path.display()))?;
    let mut policy: SkillDownloadPolicy = toml::from_str(&raw).unwrap_or_default();
    let mut policy_changed = false;

    // 合并预置的技能别名：如果用户配置中缺少某个预置别名，则添加
    for (alias, source) in crate::app::agent::skill::constants::default_preloaded_skill_aliases() {
        if let std::collections::btree_map::Entry::Vacant(entry) = policy.aliases.entry(alias) {
            entry.insert(source);
            policy_changed = true;
        }
    }

    // 规范化域名列表，并检查是否有变更
    let before_trusted = policy.trusted_domains.clone();
    let before_blocked = policy.blocked_domains.clone();
    normalize_domain_list(&mut policy.trusted_domains);
    normalize_domain_list(&mut policy.blocked_domains);
    if before_trusted != policy.trusted_domains || before_blocked != policy.blocked_domains {
        policy_changed = true;
    }

    // 如果有任何变更，保存更新后的配置
    if policy_changed {
        save_skill_download_policy(skills_path, &policy)?;
    }
    Ok(policy)
}

/// 保存技能下载策略到磁盘
///
/// 将策略配置序列化为 TOML 格式并写入磁盘。保存前会自动规范化
/// 信任域名和阻止域名列表，确保存储格式的一致性。
///
/// # 参数
///
/// * `skills_path` - 技能存储目录的路径
/// * `policy` - 待保存的策略配置引用
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误。
///
/// # 错误
///
/// - TOML 序列化失败
/// - 文件写入失败
///
/// # 注意事项
///
/// 保存时会先克隆策略对象并规范化域名列表，不会修改传入的原始策略对象。
pub(crate) fn save_skill_download_policy(
    skills_path: &Path,
    policy: &SkillDownloadPolicy,
) -> Result<()> {
    let mut to_save = policy.clone();
    normalize_domain_list(&mut to_save.trusted_domains);
    normalize_domain_list(&mut to_save.blocked_domains);
    let serialized =
        toml::to_string_pretty(&to_save).context("failed to serialize skill download policy")?;
    let path = download_policy_path(skills_path);
    std::fs::write(&path, serialized)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// 解析技能源地址别名
///
/// 将可能为别名的源地址解析为实际的源地址。如果传入的字符串在
/// 策略的别名映射中存在，则返回对应的实际地址；否则返回原字符串。
///
/// # 参数
///
/// * `source` - 可能是别名的源地址字符串
/// * `policy` - 当前的策略配置
///
/// # 返回值
///
/// 返回解析后的实际源地址字符串。
///
/// # 示例
///
/// ```ignore
/// // 假设策略中有别名 "core" -> "https://github.com/example/core"
/// let resolved = resolve_skill_source_alias("core", &policy);
/// assert_eq!(resolved, "https://github.com/example/core");
///
/// // 未知别名返回原字符串
/// let resolved = resolve_skill_source_alias("https://other.com/skill", &policy);
/// assert_eq!(resolved, "https://other.com/skill");
/// ```
pub(crate) fn resolve_skill_source_alias(source: &str, policy: &SkillDownloadPolicy) -> String {
    policy.aliases.get(source.trim()).cloned().unwrap_or_else(|| source.to_string())
}

/// 确保技能源域名的信任状态
///
/// 在下载技能前检查源域名的信任状态，执行必要的安全验证。
/// 这是技能下载安全机制的核心函数。
///
/// # 参数
///
/// * `source` - 技能源地址（可能是 URL 或别名）
/// * `policy` - 可变引用，当前的策略配置（可能会被更新）
/// * `skills_path` - 技能存储目录的路径
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，表示可以安全地从该源下载；
/// 失败时返回错误，表示不应下载。
///
/// # 处理逻辑
///
/// 1. 从源地址中提取所有需要检查的 URL
/// 2. 对每个 URL 的主机名执行检查：
///    - 如果在阻止列表中，立即拒绝
///    - 如果在信任列表中，允许继续
///    - 如果是未知域名：
///      - 非交互模式：拒绝下载
///      - 交互模式（非 WASM）：询问用户是否信任
///        - 用户确认信任：添加到信任列表
///        - 用户拒绝：添加到阻止列表并拒绝下载
///      - WASM 模式：拒绝下载（无法交互确认）
/// 3. 如果策略有变更，保存到磁盘
///
/// # 错误
///
/// - 域名在阻止列表中
/// - 非交互模式下遇到未知域名
/// - 用户拒绝信任该域名
/// - WASM 模式下无法交互确认
/// - 交互式确认失败（如 I/O 错误）
/// - 策略保存失败
///
/// # 安全考虑
///
/// 此函数是防止从恶意源下载技能的第一道防线，确保用户明确知晓
/// 并批准从未知来源下载内容。
pub(crate) fn ensure_source_domain_trust(
    source: &str,
    policy: &mut SkillDownloadPolicy,
    skills_path: &Path,
) -> Result<()> {
    // 从源地址中提取所有需要检查信任状态的 URL
    let urls = source_urls_for_trust_check(source);
    if urls.is_empty() {
        return Ok(());
    }

    // 检测是否在交互式终端环境中运行
    let interactive = source_domain_trust_can_prompt_interactively();
    let mut policy_changed = false;

    for url in urls {
        // 提取 URL 的主机名部分
        let Some(host) = extract_link_host(&url) else {
            continue;
        };

        // 检查是否在阻止列表中：如果是，直接拒绝下载
        if host_matches_any_domain(&host, &policy.blocked_domains) {
            anyhow::bail!(
                "Domain '{host}' is explicitly blocked for skill downloads. \
                 Remove it from {}/{} to allow download.",
                skills_path.display(),
                SKILL_DOWNLOAD_POLICY_FILE
            );
        }

        // 检查是否在信任列表中：如果是，允许继续
        if host_matches_any_domain(&host, &policy.trusted_domains) {
            continue;
        }

        // 未知域名处理：非交互模式下直接拒绝
        if !interactive {
            anyhow::bail!(
                "Refusing to download skill from untrusted domain '{host}' in non-interactive mode. \
                 Re-run interactively to approve, or add the domain to trusted_domains in {}/{}.",
                skills_path.display(),
                SKILL_DOWNLOAD_POLICY_FILE
            );
        }

        // 交互模式下询问用户是否信任该域名（非 WASM 平台）
        #[cfg(not(target_arch = "wasm32"))]
        {
            let trust = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "First time downloading a skill from '{host}'. Trust this domain for future downloads?"
                ))
                .default(false)  // 默认不信任，需要用户明确确认
                .interact()
                .context("failed to read domain trust confirmation")?;

            if trust {
                // 用户确认信任：添加到信任列表
                policy.trusted_domains.push(host);
                policy_changed = true;
                continue;
            }
        }

        // WASM 平台无法进行交互式确认
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!(
                "Cannot confirm trust interactively in WASM. Add domain to trusted_domains manually."
            );
        }

        // 用户拒绝信任：添加到阻止列表并拒绝下载
        policy.blocked_domains.push(host);
        save_skill_download_policy(skills_path, policy)?;
        anyhow::bail!("Skill download canceled because the source domain was not trusted.");
    }

    // 如果策略有变更（如新增了信任域名），保存到磁盘
    if policy_changed {
        save_skill_download_policy(skills_path, policy)?;
    }

    Ok(())
}

fn source_domain_trust_can_prompt_interactively() -> bool {
    #[cfg(test)]
    {
        false
    }

    #[cfg(not(test))]
    {
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
    }
}

/// 解析开放技能启用状态的字符串值
///
/// 将环境变量或配置中的字符串值解析为布尔值。支持多种常见的布尔值表示方式。
///
/// # 参数
///
/// * `raw` - 原始字符串值
///
/// # 返回值
///
/// 如果字符串可以被识别为布尔值，返回 `Some(true)` 或 `Some(false)`；
/// 否则返回 `None`。
///
/// # 支持的值
///
/// - **真值**: `"1"`, `"true"`, `"yes"`, `"on"`（不区分大小写）
/// - **假值**: `"0"`, `"false"`, `"no"`, `"off"`（不区分大小写）
fn parse_open_skills_enabled(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// 从多个来源确定开放技能功能是否启用
///
/// 按优先级合并配置文件和环境变量的设置，确定最终的启用状态。
/// 环境变量优先级高于配置文件。
///
/// # 参数
///
/// * `config_open_skills_enabled` - 来自配置文件的设置（可选）
/// * `env_override` - 来自环境变量的设置（可选）
///
/// # 返回值
///
/// 返回最终确定的启用状态。
///
/// # 优先级
///
/// 1. 如果环境变量有效（可解析为布尔值），使用环境变量的值
/// 2. 如果环境变量无效（非空但无法解析），记录警告并忽略
/// 3. 如果环境变量未设置或无效，使用配置文件的值
/// 4. 如果两者都未设置，默认返回 `false`
fn open_skills_enabled_from_sources(
    config_open_skills_enabled: Option<bool>,
    env_override: Option<&str>,
) -> bool {
    // 环境变量优先：如果设置了环境变量，尝试解析
    if let Some(raw) = env_override {
        if let Some(enabled) = parse_open_skills_enabled(raw) {
            return enabled;
        }
        // 环境变量值无效：记录警告但继续处理
        if !raw.trim().is_empty() {
            tracing::warn!(
                "Ignoring invalid VIBEWINDOW_OPEN_SKILLS_ENABLED (valid: 1|0|true|false|yes|no|on|off)"
            );
        }
    }

    // 环境变量未设置或无效：使用配置文件的值，默认为 false
    config_open_skills_enabled.unwrap_or(false)
}

/// 检查开放技能功能是否启用
///
/// 这是检查开放技能功能状态的主入口函数，从环境变量和配置文件中
/// 读取设置并确定最终状态。
///
/// # 参数
///
/// * `config_open_skills_enabled` - 来自配置文件的设置（可选）
///
/// # 返回值
///
/// 返回开放技能功能是否启用。
///
/// # 环境变量
///
/// 函数会检查环境变量 `VIBEWINDOW_OPEN_SKILLS_ENABLED`。
/// 环境变量的值会覆盖配置文件中的设置。
///
/// # 示例
///
/// ```ignore
/// // 在代码中调用
/// if open_skills_enabled(config.open_skills_enabled) {
///     // 允许加载任意来源的技能
/// } else {
///     // 仅允许加载预置或已信任的技能
/// }
/// ```
///
/// # 安全考虑
///
/// 开放技能功能允许加载任意来源的技能，可能带来安全风险。
/// 应谨慎启用此功能，并确保仅在受信任的环境中使用。
pub(crate) fn open_skills_enabled(config_open_skills_enabled: Option<bool>) -> bool {
    // 读取环境变量 VIBEWINDOW_OPEN_SKILLS_ENABLED
    let env_override = std::env::var("VIBEWINDOW_OPEN_SKILLS_ENABLED").ok();
    // 委托给辅助函数处理优先级逻辑
    open_skills_enabled_from_sources(config_open_skills_enabled, env_override.as_deref())
}
#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
