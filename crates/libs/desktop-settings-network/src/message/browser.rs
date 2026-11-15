//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::state::BrowserSettingsState;
use iced::widget::text_editor;

use config_tools::BrowserComputerUseConfig;

use crate::messages::BrowserMessage;

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

pub fn browser_config(s: &BrowserSettingsState) -> Result<config_tools::BrowserConfig, String> {
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

    let browser = config_tools::BrowserConfig {
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

    Ok(browser)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回局部效果，由宿主继续执行保存、安装或反馈。
pub fn update(app: &mut BrowserSettingsState, message: BrowserMessage) -> Effect {

    match message {
        BrowserMessage::Install => {
            if app.installing {
                return Effect::None;
            }
            let backend = app.backend.clone();
            if !matches!(backend.as_str(), "agent_browser" | "auto") {
                return Effect::Error("该后端需要手工准备服务，请查看安装帮助".into());
            }
            app.installing = true;
            return Effect::Install(backend);
        }
        BrowserMessage::InstallCompleted(result) => {
            app.installing = false;
            return match result {
                Ok(status) => Effect::Success(status),
                Err(error) => Effect::Error(error),
            };
        }
        _ => {}
    }

    if matches!(message, BrowserMessage::Refresh) {
        app.save_error = None;
        app.save_error_translation = None;
        return Effect::None;
    }

    match message {
        BrowserMessage::EnabledToggled(value) => app.enabled = value,
        BrowserMessage::AllowedDomainsChanged(value) => {
            app.allowed_domains_editor = text_editor::Content::with_text(&value);
            app.allowed_domains_input = value
        }
        BrowserMessage::AllowedDomainsEditorAction(action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            app.allowed_domains_editor.perform(action);
            app.allowed_domains_input =
                app.allowed_domains_editor.text();
            if !should_persist {
                return Effect::None;
            }
        }
        BrowserMessage::BrowserOpenChanged(value) => {
            app.browser_open = normalize_browser_open(&value)
        }
        BrowserMessage::SessionNameChanged(value) => {
            app.session_name_input = value
        }
        BrowserMessage::BackendChanged(value) => {
            app.backend = normalize_backend(&value)
        }
        BrowserMessage::NativeHeadlessToggled(value) => {
            app.native_headless = value
        }
        BrowserMessage::NativeWebdriverUrlChanged(value) => {
            app.native_webdriver_url = value
        }
        BrowserMessage::NativeChromePathChanged(value) => {
            app.native_chrome_path_input = value
        }
        BrowserMessage::ComputerUseEndpointChanged(value) => {
            app.computer_use_endpoint = value
        }
        BrowserMessage::ComputerUseApiKeyChanged(value) => {
            app.computer_use_api_key_input = value
        }
        BrowserMessage::ComputerUseTimeoutMsChanged(value) => {
            app.computer_use_timeout_ms_input = value
        }
        BrowserMessage::ComputerUseAllowRemoteEndpointToggled(value) => {
            app.computer_use_allow_remote_endpoint = value
        }
        BrowserMessage::ComputerUseWindowAllowlistChanged(value) => {
            app.computer_use_window_allowlist_input = value
        }
        BrowserMessage::ComputerUseMaxCoordinateXChanged(value) => {
            app.computer_use_max_coordinate_x_input = value
        }
        BrowserMessage::ComputerUseMaxCoordinateYChanged(value) => {
            app.computer_use_max_coordinate_y_input = value
        }
        BrowserMessage::Refresh | BrowserMessage::Install | BrowserMessage::InstallCompleted(_) => {
            unreachable!()
        }
    }

    match browser_config(app) {
        Ok(task) => {
            app.save_error = None;
            app.save_error_translation = None;
            Effect::Persist(task)
        }
        Err(err) => {
            app.save_error_translation = browser_validation_message(app);
            app.save_error = Some(err);
            Effect::None
        }
    }
}

fn browser_validation_message(app: &BrowserSettingsState) -> Option<desktop_i18n::format::Message> {
    let s = app;
    if parse_timeout_ms(&s.computer_use_timeout_ms_input).is_err() {
        return Some(desktop_i18n::format::Message::new(
            if s.computer_use_timeout_ms_input.trim() == "0" {
                "浏览器超时（computer_use.timeout_ms）必须大于 0"
            } else {
                "浏览器超时（computer_use.timeout_ms）必须是正整数"
            },
            vec![],
        ));
    }
    for (input, field) in [
        (&s.computer_use_max_coordinate_x_input, "computer_use.max_coordinate_x"),
        (&s.computer_use_max_coordinate_y_input, "computer_use.max_coordinate_y"),
    ] {
        if parse_optional_i64(input, field).is_err() {
            return Some(desktop_i18n::format::Message::new(
                "%s 必须是整数",
                vec![field.into()],
            ));
        }
    }
    None
}

#[derive(Debug)]
pub enum Effect { None, Install(String), Success(String), Error(String), Persist(config_tools::BrowserConfig) }
