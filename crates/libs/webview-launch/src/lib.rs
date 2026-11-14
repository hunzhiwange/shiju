//! WebView 启动选项与日志脱敏。

#![cfg(not(target_arch = "wasm32"))]

pub const DEFAULT_TARGET: &str = "about:blank";
const DEFAULT_TITLE: &str = "Vibe Window WebView";

#[derive(Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    pub mode: String,
    pub window_title: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
    pub cookies_json: String,
    pub shiju_app_set_cookie_stdin: bool,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            mode: "window".to_string(),
            window_title: DEFAULT_TITLE.to_string(),
            width: None,
            height: None,
            pos_x: None,
            pos_y: None,
            cookies_json: "[]".to_string(),
            shiju_app_set_cookie_stdin: false,
        }
    }
}

pub fn parse_launch_options(args: &[String]) -> LaunchOptions {
    let mut options = LaunchOptions::default();
    for a in args {
        if let Some(v) = a.strip_prefix("--mode=") {
            options.mode = v.to_string();
        } else if let Some(v) = a.strip_prefix("--cookies=") {
            options.cookies_json = v.to_string();
        } else if let Some(v) = a.strip_prefix("--width=") {
            options.width = v.parse::<i32>().ok();
        } else if let Some(v) = a.strip_prefix("--height=") {
            options.height = v.parse::<i32>().ok();
        } else if let Some(v) = a.strip_prefix("--x=") {
            options.pos_x = v.parse::<i32>().ok();
        } else if let Some(v) = a.strip_prefix("--y=") {
            options.pos_y = v.parse::<i32>().ok();
        } else if let Some(v) = a.strip_prefix("--title=") {
            options.window_title = v.to_string();
        } else if a == "--shiju-app-set-cookie-stdin" {
            options.shiju_app_set_cookie_stdin = true;
        }
    }
    options
}

fn redacted_launch_target(target: &str) -> String {
    let Some((scheme, remainder)) = target.split_once("://") else {
        return "<local-or-invalid>".to_string();
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return "<local-or-invalid>".to_string();
    }

    let authority =
        remainder.split_once(['/', '?', '#']).map_or(remainder, |(authority, _)| authority);
    let host = authority.rsplit('@').next().unwrap_or_default();
    if host.is_empty() {
        return "<invalid>".to_string();
    }

    format!("{}://{host}", scheme.to_ascii_lowercase())
}

pub fn launch_log_summary(target: &str, options: &LaunchOptions) -> String {
    format!(
        "Launch target={} width={} height={}",
        redacted_launch_target(target),
        options.width.unwrap_or(1280),
        options.height.unwrap_or(800)
    )
}

#[cfg(test)]
mod launch_tests;
