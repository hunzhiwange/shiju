//! Skill 下载来源策略的加载、持久化与域名信任校验。
//!
//! 策略文件记录 alias、可信域名和显式阻止域名。下载 skill 前必须通过
//! 本模块确认来源域名可信；非交互环境默认拒绝未知域名，避免自动化流程
//! 静默扩大网络获取能力。

use crate::source::parse_skills_sh_source;
use crate::types::{SkillDownloadPolicy, default_preloaded_skill_aliases};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::Path;

const SKILL_DOWNLOAD_POLICY_FILE: &str = ".download-policy.toml";

/// 加载或初始化 skill 下载策略。
///
/// # 参数
///
/// - `skills_path`: skill 数据目录。
///
/// # 返回值
///
/// 返回已规范化并补齐默认 alias 的下载策略。
///
/// # 错误
///
/// 策略文件读取、写入或序列化失败时返回错误。无法解析的策略文件会
/// 回退为默认策略，再由后续保存路径修复。
pub fn load_or_init_skill_download_policy(
    skills_path: &Path,
) -> Result<SkillDownloadPolicy> {
    let path = download_policy_path(skills_path);
    if !path.exists() {
        let policy = SkillDownloadPolicy::default();
        save_skill_download_policy(skills_path, &policy)?;
        return Ok(policy);
    }

    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read skill download policy {}", path.display()))?;
    let mut policy: SkillDownloadPolicy = toml::from_str(&raw).unwrap_or_default();
    let mut policy_changed = false;
    for (alias, source) in default_preloaded_skill_aliases() {
        if let std::collections::btree_map::Entry::Vacant(entry) = policy.aliases.entry(alias) {
            entry.insert(source);
            policy_changed = true;
        }
    }
    let before_trusted = policy.trusted_domains.clone();
    let before_blocked = policy.blocked_domains.clone();
    normalize_domain_list(&mut policy.trusted_domains);
    normalize_domain_list(&mut policy.blocked_domains);
    if before_trusted != policy.trusted_domains || before_blocked != policy.blocked_domains {
        policy_changed = true;
    }
    if policy_changed {
        save_skill_download_policy(skills_path, &policy)?;
    }
    Ok(policy)
}

/// 保存 skill 下载策略。
///
/// 保存前会规范化可信/阻止域名列表，确保持久化内容稳定。
///
/// # 错误
///
/// TOML 序列化或文件写入失败时返回错误。
pub fn save_skill_download_policy(
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

fn download_policy_path(skills_path: &Path) -> std::path::PathBuf {
    skills_path.join(SKILL_DOWNLOAD_POLICY_FILE)
}

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

fn host_matches_trusted_domain(host: &str, trusted_domain: &str) -> bool {
    let host = normalize_domain_entry(host);
    let trusted = normalize_domain_entry(trusted_domain);
    if host.is_empty() || trusted.is_empty() {
        return false;
    }
    host == trusted || host.ends_with(&format!(".{trusted}"))
}

fn host_matches_any_domain(host: &str, entries: &[String]) -> bool {
    entries.iter().any(|entry| host_matches_trusted_domain(host, entry))
}

fn extract_link_host(url: &str) -> Option<String> {
    let trimmed = url.strip_prefix("zip:").unwrap_or(url);
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("ssh://"))
        .or_else(|| trimmed.strip_prefix("git://"))?;
    let host_part = rest.split(&['/', '?', '#'][..]).next().unwrap_or("");
    let host_part = host_part.rsplit('@').next().unwrap_or(host_part);
    let host = host_part.split(':').next().unwrap_or("");
    let normalized = normalize_domain_entry(host);
    if normalized.is_empty() { None } else { Some(normalized) }
}

fn source_urls_for_trust_check(source: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();
    let mut push_unique = |url: String| {
        if seen.insert(url.clone()) {
            urls.push(url);
        }
    };

    if source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("ssh://")
        || source.starts_with("git://")
    {
        push_unique(source.to_string());
    }

    if let Some(skills_source) = parse_skills_sh_source(source) {
        push_unique(skills_source.github_repo_url());
    }

    urls
}

/// 解析 skill 来源 alias。
///
/// # 参数
///
/// - `source`: 用户输入的来源或 alias。
/// - `policy`: 当前下载策略。
///
/// # 返回值
///
/// 若 alias 存在则返回映射后的真实来源，否则原样返回输入字符串。
pub fn resolve_skill_source_alias(source: &str, policy: &SkillDownloadPolicy) -> String {
    policy.aliases.get(source.trim()).cloned().unwrap_or_else(|| source.to_string())
}

/// 确保 skill 来源域名已被信任。
///
/// # 参数
///
/// - `source`: 真实下载来源，支持普通 URL 与 skills.sh 风格来源。
/// - `policy`: 可变下载策略，交互确认后会写入可信或阻止列表。
/// - `skills_path`: 策略文件所在目录。
///
/// # 返回值
///
/// 域名已可信、来源不需要域名校验或用户确认信任时返回 `Ok(())`。
///
/// # 错误
///
/// 域名被显式阻止、非交互模式遇到未知域名、用户拒绝信任、确认读取失败
/// 或策略保存失败时返回错误。
///
/// # 安全说明
///
/// 未知域名在非交互环境中失败关闭，防止 CI/后台任务在没有人工确认的
/// 情况下下载新来源；用户拒绝时会把域名写入阻止列表，后续重复尝试也会
/// 被显式拒绝。
pub fn ensure_source_domain_trust(
    source: &str,
    policy: &mut SkillDownloadPolicy,
    skills_path: &Path,
) -> Result<()> {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    ensure_source_domain_trust_with_interactivity(source, policy, skills_path, interactive)
}

fn ensure_source_domain_trust_with_interactivity(
    source: &str,
    policy: &mut SkillDownloadPolicy,
    skills_path: &Path,
    interactive: bool,
) -> Result<()> {
    let urls = source_urls_for_trust_check(source);
    if urls.is_empty() {
        return Ok(());
    }

    let mut policy_changed = false;

    for url in urls {
        let Some(host) = extract_link_host(&url) else {
            continue;
        };

        if host_matches_any_domain(&host, &policy.blocked_domains) {
            anyhow::bail!(
                "Domain '{host}' is explicitly blocked for skill downloads. \
                 Remove it from {}/{} to allow download.",
                skills_path.display(),
                SKILL_DOWNLOAD_POLICY_FILE
            );
        }
        if host_matches_any_domain(&host, &policy.trusted_domains) {
            continue;
        }

        if !interactive {
            // 后台或管道运行时没有可靠的人机确认通道，因此未知域名必须
            // 显式配置后才能下载。
            anyhow::bail!(
                "Refusing to download skill from untrusted domain '{host}' in non-interactive mode. \
                 Re-run interactively to approve, or add the domain to trusted_domains in {}/{}.",
                skills_path.display(),
                SKILL_DOWNLOAD_POLICY_FILE
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let trust = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "First time downloading a skill from '{host}'. Trust this domain for future downloads?"
                ))
                .default(false)
                .interact()
                .context("failed to read domain trust confirmation")?;

            if trust {
                policy.trusted_domains.push(host);
                policy_changed = true;
                continue;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!(
                "Cannot confirm trust interactively in WASM. Add domain to trusted_domains manually."
            );
        }

        policy.blocked_domains.push(host);
        save_skill_download_policy(skills_path, policy)?;
        anyhow::bail!("Skill download canceled because the source domain was not trusted.");
    }

    if policy_changed {
        save_skill_download_policy(skills_path, policy)?;
    }

    Ok(())
}
#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
