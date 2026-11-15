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

pub fn load_web_bookmarks(cfg: &serde_json::Value) -> Vec<WebBookmark> {
    cfg.get("web_bookmarks")
        .and_then(|value: &serde_json::Value| value.as_array())
        .map(|items: &Vec<serde_json::Value>| {
            items
                .iter()
                .filter_map(|item: &serde_json::Value| {
                    let title = item.get("title").and_then(|value| value.as_str())?;
                    let url = item.get("url").and_then(|value| value.as_str())?;
                    let width = item
                        .get("width")
                        .and_then(|value| value.as_i64())
                        .map(|value| value as i32);
                    let height = item
                        .get("height")
                        .and_then(|value| value.as_i64())
                        .map(|value| value as i32);
                    let cookie_configs = item
                        .get("cookie_configs")
                        .and_then(|value| value.as_array())
                        .map(|configs: &Vec<serde_json::Value>| {
                            configs
                                .iter()
                                .filter_map(|config_item: &serde_json::Value| {
                                    let name =
                                        config_item.get("name").and_then(|value| value.as_str())?;
                                    let domain = config_item
                                        .get("domain")
                                        .and_then(|value| value.as_str())
                                        .map(ToString::to_string);
                                    let days =
                                        config_item.get("days").and_then(|value| value.as_i64());
                                    let url_filter = config_item
                                        .get("url_filter")
                                        .and_then(|value| value.as_str())
                                        .map(ToString::to_string);

                                    Some(CookieConfig {
                                        name: name.to_string(),
                                        domain,
                                        days,
                                        url_filter,
                                    })
                                })
                                .collect()
                        });

                    Some(WebBookmark {
                        title: title.to_string(),
                        url: url.to_string(),
                        width,
                        height,
                        cookie_configs,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![WebBookmark {
                title: "测试网站".to_string(),
                url: "https://example.com/".to_string(),
                width: None,
                height: None,
                cookie_configs: None,
            }]
        })
}


#[cfg(test)]
#[path = "bookmark_init_tests.rs"]
mod init_tests;
