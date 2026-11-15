//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。
use crate::Message as SettingsMessage;
use crate::app_init::state::WebSearchSettingsState;


use config_tools::WebSearchConfig;

use crate::messages::{WebSearchMessage};

fn normalize_provider(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ddg" | "duckduckgo" => "duckduckgo".to_string(),
        "brave" => "brave".to_string(),
        "serper" => "serper".to_string(),
        "google" => "google".to_string(),
        "bing" => "bing".to_string(),
        _ => "duckduckgo".to_string(),
    }
}

fn trim_to_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn parse_max_results(input: &str) -> Result<usize, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(WebSearchConfig::default().max_results);
    }

    let value = trimmed.parse::<usize>().map_err(|_| "max_results 必须是整数".to_string())?;
    if value == 0 {
        return Err("max_results 必须大于 0".to_string());
    }
    Ok(value.clamp(1, 10))
}

fn parse_timeout_secs(input: &str) -> Result<u64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(WebSearchConfig::default().timeout_secs);
    }

    let value = trimmed.parse::<u64>().map_err(|_| "timeout_secs 必须是整数秒".to_string())?;
    if value == 0 {
        return Err("timeout_secs 必须大于 0".to_string());
    }
    Ok(value)
}

fn persist_web_search_settings(app: &mut WebSearchSettingsState) -> Result<Effect, String> {
    let s = &app;
    let enabled = s.enabled;
    let provider = normalize_provider(&s.provider);
    let api_key = trim_to_option(&s.api_key_input);
    let api_url = trim_to_option(&s.api_url_input);
    let brave_api_key = trim_to_option(&s.brave_api_key_input);
    let max_results = parse_max_results(&s.max_results_input)?;
    let timeout_secs = parse_timeout_secs(&s.timeout_secs_input)?;
    let user_agent = {
        let value = s.user_agent.trim();
        if value.is_empty() { WebSearchConfig::default().user_agent } else { value.to_string() }
    };

    Ok({
        let mut web_search = config_tools::WebSearchConfig::default();
        web_search.enabled = enabled;
        web_search.provider = provider;
        web_search.api_key = api_key;
        web_search.api_url = api_url;
        web_search.brave_api_key = brave_api_key;
        web_search.max_results = max_results;
        web_search.timeout_secs = timeout_secs;
        web_search.user_agent = user_agent;

        Effect::Persist(web_search)
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回领域效果，由宿主执行持久化或平台动作。
pub fn update(app: &mut WebSearchSettingsState, message: SettingsMessage) -> Effect {
    let SettingsMessage::WebSearch(message) = message else {
        return Effect::None;
    };

    match message {
        WebSearchMessage::Refresh => {
            app.save_error = None;
            return Effect::None;
        }
        WebSearchMessage::EnabledToggled(value) => app.enabled = value,
        WebSearchMessage::ProviderChanged(value) => {
            app.provider = normalize_provider(&value)
        }
        WebSearchMessage::ApiKeyChanged(value) => app.api_key_input = value,
        WebSearchMessage::ApiUrlChanged(value) => app.api_url_input = value,
        WebSearchMessage::BraveApiKeyChanged(value) => {
            app.brave_api_key_input = value
        }
        WebSearchMessage::MaxResultsChanged(value) => {
            app.max_results_input = value;
        }
        WebSearchMessage::TimeoutSecsChanged(value) => {
            app.timeout_secs_input = value;
        }
        WebSearchMessage::UserAgentChanged(value) => app.user_agent = value,
        WebSearchMessage::HelpOpen => {
            app.show_help_modal = true;
            return Effect::None;
        }
        WebSearchMessage::HelpClose => {
            app.show_help_modal = false;
            return Effect::None;
        }
    }

    match persist_web_search_settings(app) {
        Ok(task) => {
            app.save_error = None;
            task
        }
        Err(err) => {
            app.save_error = Some(err);
            Effect::None
        }
    }
}
#[cfg(test)]
#[path = "web_search_tests.rs"]
mod web_search_tests;

pub enum Effect { None, Persist(config_tools::WebSearchConfig) }

/// Apply only fields owned by this settings page.
pub fn apply_config(target: &mut config_tools::WebSearchConfig, value: config_tools::WebSearchConfig) {
        target.enabled = value.enabled;
        target.provider = value.provider;
        target.api_key = value.api_key;
        target.api_url = value.api_url;
        target.brave_api_key = value.brave_api_key;
        target.max_results = value.max_results;
        target.timeout_secs = value.timeout_secs;
        target.user_agent = value.user_agent;
}
