pub(crate) use acp_agent_registry::agent_registry::built_in_agent_display_name;
pub use acp_agent_registry::{EMPLOYEE_ACP_BACKEND, EMPLOYEE_ACP_PROFILE};
use config_aggregate::AcpAgentConfig;
use config_ui::DesktopLocale;
use desktop_i18n::I18n;

use crate::acp_state::{AcpInstallFilter, AcpSettingsState};

pub(crate) const ACP_ORDER: &[&str] = &[
    EMPLOYEE_ACP_BACKEND,
    "claude",
    "gemini",
    "opencode",
    "codex",
    "openclaw",
    "copilot",
    "pi",
    "auggie",
    "cursor",
    "droid",
    "iflow",
    "kilocode",
    "qwen",
    "kimi",
    "kiro",
    "qoder",
    "trae",
    "codebuddy",
    "goose",
    "vibe",
    "hermes",
    "snow",
];

pub(crate) const EMPLOYEE_ACP_HINT: &str =
    "网关内置，无需安装。发起会话时请选择私有或共享员工。会话使用智能体原生模型。";

pub fn acp_title(agent: &str) -> &'static str {
    built_in_agent_display_name(agent).unwrap_or("自定义 ACP")
}

pub fn acp_description(agent: &str) -> &'static str {
    match agent {
        "codex" => "OpenAI Codex ACP 适配器，适合把 Codex CLI 作为外部编码后端。",
        "claude" => "Claude Code ACP 适配器，适合连接本机 Claude Code 工作流。",
        "gemini" => "Gemini CLI 的实验性 ACP 模式，适合 Google Gemini 本地代理流程。",
        "copilot" => "GitHub Copilot CLI 的 ACP/stdio 模式，依赖本机 Copilot 登录状态。",
        "openclaw" => "OpenClaw ACP 后端，适合沿用 OpenClaw 本地代理生态。",
        "pi" => "Pi ACP 适配器，适合轻量对话与辅助代理实验。",
        "auggie" => "Augment Code Auggie CLI 的 ACP 模式。",
        "cursor" => "Cursor Agent 的 ACP 入口，依赖本机 cursor-agent 命令。",
        "droid" => "Factory droid 的 ACP 输出模式。",
        "iflow" => "iFlow 的实验性 ACP 模式。",
        "kilocode" => "Kilocode ACP 后端。",
        "opencode" => "OpenCode ACP 后端。",
        "qwen" => "Qwen Code ACP 后端。",
        "kimi" => "Kimi Code CLI 的 ACP 后端。",
        "kiro" => "Kiro 官方 CLI 的 ACP 后端。",
        "qoder" => "Qoder CLI 的 ACP 后端。",
        "trae" => "TRAE CLI 的 ACP 服务。",
        "codebuddy" => "腾讯 CodeBuddy 原生 ACP 后端。",
        "goose" => "Goose 原生 ACP 后端。",
        "vibe" => "Mistral Vibe 的 vibe-acp 后端。",
        "hermes" => "Nous Research Hermes Agent 的 ACP 后端。",
        "snow" => "Snow CLI 的 ACP 模式。",
        _ if built_in_agent_display_name(agent).is_some() => "通过 ACP 连接此代理。",
        _ => "自定义 ACP 后端，会按全局配置中的 command / args 启动。",
    }
}

pub fn setup_hint(agent: &str) -> &'static str {
    match agent {
        "codex" => "初始化：确保 Node.js 可用；首次运行按 Codex CLI 提示完成登录或令牌配置。",
        "claude" => "初始化：安装或登录 Claude Code；非交互运行建议提前完成认证。",
        "gemini" => "初始化：配置 Gemini CLI 的 API Key 或 OAuth；实验模式需要 CLI 版本支持 ACP。",
        "copilot" => "初始化：先完成 GitHub Copilot CLI 登录，再启用 ACP/stdio。",
        "openclaw" => "初始化：确保 openclaw 命令可用，并提前完成它需要的本地认证。",
        "pi" => "初始化：确保 Node.js 可用；npx 会按需拉取 pi-acp 适配器。",
        _ => "初始化：确认命令在 PATH 中可执行，并提前完成该代理需要的认证。",
    }
}

pub fn command_line(config: &AcpAgentConfig) -> String {
    std::iter::once(config.command.as_str())
        .chain(config.args.iter().map(String::as_str))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn ordered_agents(catalog: &std::collections::BTreeMap<String, AcpAgentConfig>) -> Vec<String> {
    let mut names = catalog.keys().cloned().collect::<Vec<_>>();
    names.sort_by(|left, right| {
        let left_rank =
            ACP_ORDER.iter().position(|candidate| candidate == left).unwrap_or(ACP_ORDER.len());
        let right_rank =
            ACP_ORDER.iter().position(|candidate| candidate == right).unwrap_or(ACP_ORDER.len());
        (left_rank, left.as_str()).cmp(&(right_rank, right.as_str()))
    });
    names
}

pub(crate) fn filtered_agents(state: &AcpSettingsState, locale: DesktopLocale) -> Vec<String> {
    let query = state.catalog_query.trim().to_lowercase();
    let localized = I18n::new(locale);
    let english = I18n::new(DesktopLocale::English);
    let mut agents = ordered_agents(&state.catalog);
    agents.retain(|agent| {
        let installed = if agent == EMPLOYEE_ACP_BACKEND {
            Some(true)
        } else {
            state.installed.get(agent).copied()
        };
        let matches_filter = match state.install_filter {
            AcpInstallFilter::All => true,
            AcpInstallFilter::Installed => installed == Some(true),
            AcpInstallFilter::NotInstalled => installed == Some(false),
        };
        matches_filter
            && (query.is_empty()
                || agent.to_lowercase().contains(&query)
                || built_in_agent_display_name(agent).is_some_and(|name| {
                    [name, localized.__(name), english.__(name)]
                        .iter()
                        .any(|label| label.to_lowercase().contains(&query))
                }))
    });
    agents
}

#[cfg(test)]
mod tests;

pub fn sort_acp_agents(
    acp_cfg: &std::collections::HashMap<String, config_aggregate::AcpAgentConfig>,
) -> Vec<String> {
    let mut acp_agents = acp_cfg.keys().cloned().collect::<Vec<_>>();
    acp_agents.sort_by(|left, right| {
        let left_key = (left != "codex", left.as_str());
        let right_key = (right != "codex", right.as_str());
        left_key.cmp(&right_key)
    });
    acp_agents
}


#[cfg(test)]
#[path = "acp_init_tests.rs"]
mod init_tests;
