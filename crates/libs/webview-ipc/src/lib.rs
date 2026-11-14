//! Pure encoding and decoding for WebView process control messages.

#[cfg(not(target_arch = "wasm32"))]
use webview_core::{events::UserEvent, navigation::is_safe_navigation_url};

#[cfg(not(target_arch = "wasm32"))]
pub fn navigation_event_line(url: &str) -> Option<String> {
    if !is_safe_navigation_url(url) {
        return None;
    }
    let encoded = serde_json::to_string(url).ok()?;
    Some(format!("navigation {encoded}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_navigation_ipc_message(message: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
    if value.get("type").and_then(serde_json::Value::as_str) != Some("navigation") {
        return None;
    }
    let url = value.get("url").and_then(serde_json::Value::as_str)?;
    is_safe_navigation_url(url).then(|| url.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_stdin_event(line: &str) -> Option<UserEvent> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    match parts.as_slice() {
        ["resize", w, h, ..] => Some(UserEvent::Resize(w.parse().ok()?, h.parse().ok()?)),
        ["move", x, y, ..] => Some(UserEvent::Move(x.parse().ok()?, y.parse().ok()?)),
        ["navigate", url] if is_safe_navigation_url(url) => {
            Some(UserEvent::OpenUrl((*url).to_string()))
        }
        ["back"] => Some(UserEvent::Back),
        ["forward"] => Some(UserEvent::Forward),
        ["reload"] => Some(UserEvent::Reload),
        ["raise"] => Some(UserEvent::SetAlwaysOnTop(true)),
        ["lower"] => Some(UserEvent::SetAlwaysOnTop(false)),
        ["show"] => Some(UserEvent::Show),
        ["hide"] => Some(UserEvent::Hide),
        ["close"] => Some(UserEvent::Close),
        _ => None,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod protocol_tests;
