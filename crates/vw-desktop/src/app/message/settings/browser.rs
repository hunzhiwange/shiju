//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::{patch_full_agent_config_async, spawn_gateway_task};
use crate::app::{App, Message};
use iced::Task;
use iced::widget::text_editor;
use serde_json::json;
use vw_config_types::tools::BrowserComputerUseConfig;

use super::messages::{BrowserMessage, SettingsMessage};

fn parse_csv_lines(input: &str) -> Vec<String> {
    input
        .split([',', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod browser_tests;

fn normalize_browser_open(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "new_window" => "new_window".to_string(),
        "new_tab" => "new_tab".to_string(),
        _ => "new_tab".to_string(),
    }
}

fn normalize_backend(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "rust_native" | "native" => "native".to_string(),
        "computer_use" => "computer_use".to_string(),
        "auto" => "auto".to_string(),
        _ => "agent_browser".to_string(),
    }
}

fn parse_timeout_ms(input: &str) -> Result<u64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(15_000);
    }

    let value =
        trimmed.parse::<u64>().map_err(|_| "computer_use.timeout_ms 必须是正整数".to_string())?;
    if value == 0 {
        return Err("computer_use.timeout_ms 必须大于 0".to_string());
    }
    Ok(value)
}

fn parse_optional_i64(input: &str, field: &str) -> Result<Option<i64>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed.parse::<i64>().map(Some).map_err(|_| format!("{field} 必须是整数"))
}

fn persist_browser_settings(app: &mut App) -> Result<Task<Message>, String> {
    let s = &app.browser_settings;
    let enabled = s.enabled;
    let native_headless = s.native_headless;
    let computer_use_allow_remote_endpoint = s.computer_use_allow_remote_endpoint;
    let allowed_domains = parse_csv_lines(&s.allowed_domains_input);
    let session_name = s.session_name_input.trim().to_string();
    let native_webdriver_url = s.native_webdriver_url.trim().to_string();
    let native_chrome_path = s.native_chrome_path_input.trim().to_string();
    let computer_use_endpoint = s.computer_use_endpoint.trim().to_string();
    let computer_use_api_key = s.computer_use_api_key_input.trim().to_string();
    let timeout_ms = parse_timeout_ms(&s.computer_use_timeout_ms_input)?;
    let max_coordinate_x = parse_optional_i64(
        &s.computer_use_max_coordinate_x_input,
        "computer_use.max_coordinate_x",
    )?;
    let max_coordinate_y = parse_optional_i64(
        &s.computer_use_max_coordinate_y_input,
        "computer_use.max_coordinate_y",
    )?;
    let window_allowlist = parse_csv_lines(&s.computer_use_window_allowlist_input);
    let browser_open = normalize_browser_open(&s.browser_open);
    let backend = normalize_backend(&s.backend);

    let browser = vw_config_types::tools::BrowserConfig {
        enabled,
        allowed_domains,
        browser_open,
        session_name: if session_name.is_empty() { None } else { Some(session_name) },
        backend,
        native_headless,
        native_webdriver_url: if native_webdriver_url.is_empty() {
            "http://127.0.0.1:9515".to_string()
        } else {
            native_webdriver_url
        },
        native_chrome_path: if native_chrome_path.is_empty() {
            None
        } else {
            Some(native_chrome_path)
        },
        computer_use: BrowserComputerUseConfig {
            endpoint: if computer_use_endpoint.is_empty() {
                "http://127.0.0.1:8787/v1/actions".to_string()
            } else {
                computer_use_endpoint
            },
            api_key: if computer_use_api_key.is_empty() {
                None
            } else {
                Some(computer_use_api_key)
            },
            timeout_ms,
            allow_remote_endpoint: computer_use_allow_remote_endpoint,
            window_allowlist,
            max_coordinate_x,
            max_coordinate_y,
        },
    };

    Ok(spawn_gateway_task("browser", async move {
        patch_full_agent_config_async(json!({ "browser": browser })).await
    }))
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Browser(message) = message else {
        return Task::none();
    };

    if matches!(message, BrowserMessage::Refresh) {
        app.browser_settings.save_error = None;
        return Task::none();
    }

    match message {
        BrowserMessage::EnabledToggled(value) => app.browser_settings.enabled = value,
        BrowserMessage::AllowedDomainsChanged(value) => {
            app.browser_settings.allowed_domains_editor = text_editor::Content::with_text(&value);
            app.browser_settings.allowed_domains_input = value
        }
        BrowserMessage::AllowedDomainsEditorAction(action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            app.browser_settings.allowed_domains_editor.perform(action);
            app.browser_settings.allowed_domains_input =
                app.browser_settings.allowed_domains_editor.text();
            if !should_persist {
                return Task::none();
            }
        }
        BrowserMessage::BrowserOpenChanged(value) => {
            app.browser_settings.browser_open = normalize_browser_open(&value)
        }
        BrowserMessage::SessionNameChanged(value) => {
            app.browser_settings.session_name_input = value
        }
        BrowserMessage::BackendChanged(value) => {
            app.browser_settings.backend = normalize_backend(&value)
        }
        BrowserMessage::NativeHeadlessToggled(value) => {
            app.browser_settings.native_headless = value
        }
        BrowserMessage::NativeWebdriverUrlChanged(value) => {
            app.browser_settings.native_webdriver_url = value
        }
        BrowserMessage::NativeChromePathChanged(value) => {
            app.browser_settings.native_chrome_path_input = value
        }
        BrowserMessage::ComputerUseEndpointChanged(value) => {
            app.browser_settings.computer_use_endpoint = value
        }
        BrowserMessage::ComputerUseApiKeyChanged(value) => {
            app.browser_settings.computer_use_api_key_input = value
        }
        BrowserMessage::ComputerUseTimeoutMsChanged(value) => {
            app.browser_settings.computer_use_timeout_ms_input = value
        }
        BrowserMessage::ComputerUseAllowRemoteEndpointToggled(value) => {
            app.browser_settings.computer_use_allow_remote_endpoint = value
        }
        BrowserMessage::ComputerUseWindowAllowlistChanged(value) => {
            app.browser_settings.computer_use_window_allowlist_input = value
        }
        BrowserMessage::ComputerUseMaxCoordinateXChanged(value) => {
            app.browser_settings.computer_use_max_coordinate_x_input = value
        }
        BrowserMessage::ComputerUseMaxCoordinateYChanged(value) => {
            app.browser_settings.computer_use_max_coordinate_y_input = value
        }
        BrowserMessage::Refresh => unreachable!(),
    }

    match persist_browser_settings(app) {
        Ok(task) => {
            app.browser_settings.save_error = None;
            task
        }
        Err(err) => {
            app.browser_settings.save_error = Some(err);
            Task::none()
        }
    }
}
