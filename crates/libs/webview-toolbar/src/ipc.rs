//! 工具栏 IPC 与安全 URL 归一化。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolbarAction {
    Back,
    Forward,
    Navigate(String),
    Reload,
    OpenExternal(String),
    Clear,
}

pub fn parse_toolbar_ipc_message(message: &str) -> Option<ToolbarAction> {
    let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
    match value.get("type").and_then(serde_json::Value::as_str)? {
        "back" => Some(ToolbarAction::Back),
        "forward" => Some(ToolbarAction::Forward),
        "navigate" => toolbar_url(&value).map(ToolbarAction::Navigate),
        "reload" => Some(ToolbarAction::Reload),
        "open_external" => toolbar_url(&value).map(ToolbarAction::OpenExternal),
        "close" => Some(ToolbarAction::Clear),
        _ => None,
    }
}

fn toolbar_url(value: &serde_json::Value) -> Option<String> {
    normalize_toolbar_url(value.get("url")?.as_str()?)
}

fn normalize_toolbar_url(input: &str) -> Option<String> {
    if input.chars().any(char::is_control) {
        return None;
    }
    let trimmed = input.trim().trim_matches(['`', '"', '\'']).trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    if webview_core::navigation::is_safe_navigation_url(trimmed) {
        return Some(trimmed.to_owned());
    }

    // Preserve host:port shorthand while treating actual URI schemes as
    // explicit. A rejected scheme must never be rewritten as an HTTPS host.
    let host_port = trimmed.split_once(':').is_some_and(|(host, rest)| {
        let port = rest.split(['/', '?', '#']).next().unwrap_or_default();
        (host == "localhost" || host.contains('.')) && port.parse::<u16>().is_ok()
    });
    if trimmed.contains(':') && !host_port && !trimmed.starts_with('[') {
        return webview_core::navigation::ExternalTarget::parse(trimmed)
            .map(|_| trimmed.to_owned());
    }
    let target = format!("https://{trimmed}");
    url::Url::parse(&target).ok()?.has_host().then_some(target)
}
