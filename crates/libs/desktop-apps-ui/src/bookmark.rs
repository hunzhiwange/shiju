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
#[path = "bookmark_tests.rs"]
mod bookmark_tests;
