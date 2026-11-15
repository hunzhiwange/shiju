pub use desktop_navigation::components::notification::Notification;

pub use desktop_ui_controls::toast::{Toast, ToastKind};

#[derive(Debug)]
pub(crate) struct WorkBrowserState {
    pub(crate) address: String,
    pub(crate) error: Option<String>,
    pub(crate) window_focused: bool,
    pub(crate) closed: bool,
}

impl Default for WorkBrowserState {
    fn default() -> Self {
        Self { address: String::new(), error: None, window_focused: true, closed: true }
    }
}

pub use desktop_workspace::project_message::edit_meta::{RecentProjectMeta, default_recent_project_session_auto_refresh, default_recent_project_session_refresh_interval_seconds};

/// 网页书签
///
/// 存储嵌入 WebView 的网页书签配置，
/// 包括标题、URL、窗口尺寸和 Cookie 配置。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebBookmark {
    /// 书签标题
    pub title: String,
    /// 网页 URL
    pub url: String,
    /// 窗口宽度（可选）
    pub width: Option<i32>,
    /// 窗口高度（可选）
    pub height: Option<i32>,
    /// Cookie 配置列表（可选）
    pub cookie_configs: Option<Vec<CookieConfig>>,
}

/// Cookie 配置
///
/// 定义 WebView 中预设 Cookie 的配置，
/// 包括名称、域名、有效期和 URL 过滤规则。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CookieConfig {
    /// Cookie 名称
    pub name: String,
    /// Cookie 作用域域名（可选）
    pub domain: Option<String>,
    /// Cookie 有效天数（可选）
    pub days: Option<i64>,
    /// URL 过滤规则（可选）
    pub url_filter: Option<String>,
}

#[cfg(test)]
#[path = "app_aux_tests.rs"]
mod app_aux_tests;
