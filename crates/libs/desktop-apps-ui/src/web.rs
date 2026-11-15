use crate::bookmark::{WebBookmark, CookieConfig};
use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub enum Message {
    OpenUrlExternal(String),
    OpenWebUrl(String),
    OpenWebUrlWithTitle(String, String),
    OpenWebUrlWithTitleAndSize(String, String, Option<i32>, Option<i32>),
    ToggleWebLinksMenu,
    WebBookmarkTitleChanged(String),
    WebBookmarkUrlChanged(String),
    WebBookmarkWidthChanged(String),
    WebBookmarkHeightChanged(String),
    WebBookmarkAddSave,
    WebBookmarkAddCancel,
    WebBookmarkEditStart(usize),
    WebBookmarkEditTitleChanged(String),
    WebBookmarkEditUrlChanged(String),
    WebBookmarkEditWidthChanged(String),
    WebBookmarkEditHeightChanged(String),
    WebBookmarkEditCookieConfigsChanged(iced::widget::text_editor::Action),
    WebBookmarkEditCookieConfigsInsertExample,
    WebBookmarkEditSave,
    WebBookmarkEditCancel,
    WebBookmarkRemove(usize),
}
#[derive(Default)]
pub struct State {
    pub web_bookmarks: Vec<WebBookmark>,
    #[cfg(not(target_arch = "wasm32"))]
    pub independent_webview_children: Vec<std::process::Child>,
    pub show_web_links_menu: bool,
    pub web_bookmark_title_input: String,
    pub web_bookmark_url_input: String,
    pub web_bookmark_width_input: String,
    pub web_bookmark_height_input: String,
    pub editing_web_bookmark: Option<usize>,
    pub edit_web_bookmark_title_input: String,
    pub edit_web_bookmark_url_input: String,
    pub edit_web_bookmark_width_input: String,
    pub edit_web_bookmark_height_input: String,
    pub edit_web_bookmark_cookie_configs_editor: text_editor::Content,
}
pub enum Effect { Save(serde_json::Value), Error(&'static str) }
const WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE: &str = r#"[
  {
    "name": "session_id",
    "domain": "example.com",
    "days": 365,
    "url_filter": "https://example.com/"
  }
]"#;

fn save_bookmarks(app: &mut State) -> Option<Effect> {
    let bookmarks = serde_json::Value::Array(
        app.web_bookmarks
            .iter()
            .map(|b| {
                let mut obj = serde_json::Map::new();
                obj.insert("title".to_string(), serde_json::Value::String(b.title.clone()));
                obj.insert("url".to_string(), serde_json::Value::String(b.url.clone()));
                if let Some(w) = b.width
                    && let Some(nn) = serde_json::Number::from_f64(w as f64)
                {
                    obj.insert("width".to_string(), serde_json::Value::Number(nn));
                }
                if let Some(h) = b.height
                    && let Some(nn) = serde_json::Number::from_f64(h as f64)
                {
                    obj.insert("height".to_string(), serde_json::Value::Number(nn));
                }
                if let Some(configs) = &b.cookie_configs
                    && let Ok(v) = serde_json::to_value(configs)
                {
                    obj.insert("cookie_configs".to_string(), v);
                }
                serde_json::Value::Object(obj)
            })
            .collect(),
    );
    Some(Effect::Save(bookmarks))
}

/// Only accepts URLs already validated by `update`; do not expose OS error payloads or URLs.
#[cfg(not(target_arch = "wasm32"))]
fn open_in_browser(_app: &mut State, url: &str) -> Option<Effect> {
    match open::that(url) {
        Ok(()) => None,
        Err(_) => Some(Effect::Error("无法打开浏览器，请检查系统默认浏览器设置后重试")),
    }
}

/// 处理网页打开与书签 Cookie 配置；独立 WebView 不可用时回退系统浏览器。
pub fn update(app: &mut State, mut message: Message) -> Option<Effect> {
    // Validate before either the WebView helper or the system URL handler sees input.
    let target = match &mut message {
        Message::OpenWebUrl(url)
        | Message::OpenWebUrlWithTitle(url, _)
        | Message::OpenWebUrlWithTitleAndSize(url, _, _, _)
        | Message::OpenUrlExternal(url) => Some(url),
        _ => None,
    };
    if let Some(target) = target {
        let parsed = url::Url::parse(target.trim());
        match parsed {
            Ok(url) if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() => {
                *target = url.into();
            }
            _ => {
                return Some(Effect::Error("无法打开网页：仅允许有效的 HTTP 或 HTTPS 地址"));
            }
        }
    }
    match message {
        Message::OpenWebUrl(_url) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let candidates = desktop_web_runtime::webview_runtime::webview_helper_candidates();

                // Only use bookmark-specific cookie configs
                let mut configs = Vec::new();
                if let Some(bm) = app.web_bookmarks.iter().find(|b| b.url == _url)
                    && let Some(bm_configs) = &bm.cookie_configs
                {
                    configs.extend(bm_configs.clone());
                }
                let cookies_json = serde_json::to_string(&configs).unwrap_or_default();

                let mut spawned = false;
                for cand in candidates {
                    if let Ok(child) = std::process::Command::new(&cand)
                        .arg(_url.clone())
                        .arg(format!("--cookies={}", cookies_json))
                        .spawn()
                    {
                        app.independent_webview_children.push(child);
                        spawned = true;
                        break;
                    }
                }
                if !spawned {
                    return open_in_browser(app, &_url);
                }
            }
            None
        }
        Message::OpenWebUrlWithTitle(_url, _title) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let candidates = desktop_web_runtime::webview_runtime::webview_helper_candidates();

                // Only use bookmark-specific cookie configs
                let mut configs = Vec::new();
                if let Some(bm) = app.web_bookmarks.iter().find(|b| b.url == _url)
                    && let Some(bm_configs) = &bm.cookie_configs
                {
                    configs.extend(bm_configs.clone());
                }
                let cookies_json = serde_json::to_string(&configs).unwrap_or_default();

                let mut spawned = false;
                for cand in candidates {
                    if let Ok(child) = std::process::Command::new(&cand)
                        .arg(_url.clone())
                        .arg(format!("--title={}", _title))
                        .arg(format!("--cookies={}", cookies_json))
                        .spawn()
                    {
                        app.independent_webview_children.push(child);
                        spawned = true;
                        break;
                    }
                }
                if !spawned {
                    return open_in_browser(app, &_url);
                }
            }
            None
        }
        Message::OpenWebUrlWithTitleAndSize(_url, _title, _w, _h) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let url = _url.trim().trim_matches('`').trim().to_string();
                let candidates = desktop_web_runtime::webview_runtime::webview_helper_candidates();

                // Only use bookmark-specific cookie configs
                let mut configs = Vec::new();
                if let Some(bm) = app.web_bookmarks.iter().find(|b| b.url == url)
                    && let Some(bm_configs) = &bm.cookie_configs
                {
                    configs.extend(bm_configs.clone());
                }
                let cookies_json = serde_json::to_string(&configs).unwrap_or_default();

                let mut spawned = false;
                for cand in candidates {
                    let mut cmd = std::process::Command::new(&cand);
                    cmd.arg(url.clone()).arg(format!("--title={}", _title));
                    cmd.arg(format!("--cookies={}", cookies_json));
                    if let Some(ww) = _w {
                        cmd.arg(format!("--width={}", ww));
                    }
                    if let Some(hh) = _h {
                        cmd.arg(format!("--height={}", hh));
                    }
                    if let Ok(mut child) = cmd.spawn() {
                        if cfg!(windows) {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            if let Ok(Some(_)) = child.try_wait() {
                                let _ = rfd::MessageDialog::new()
                                    .set_title("VibeWindow WebView")
                                    .set_description(
                                        "无法启动内置 WebView。\n\n请安装 Microsoft Edge WebView2 Runtime（Evergreen x64）。\n\n即将使用系统默认浏览器打开。",
                                    )
                                    .set_level(rfd::MessageLevel::Error)
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .show();
                                continue;
                            }
                        }
                        app.independent_webview_children.push(child);
                        spawned = true;
                        break;
                    }
                }
                if !spawned {
                    return open_in_browser(app, &url);
                }
            }
            None
        }
        Message::OpenUrlExternal(_url) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                open_in_browser(app, &_url)
            }
            #[cfg(target_arch = "wasm32")]
            None
        }
        Message::ToggleWebLinksMenu => {
            app.show_web_links_menu = !app.show_web_links_menu;
            None
        }
        Message::WebBookmarkTitleChanged(v) => {
            app.web_bookmark_title_input = v;
            None
        }
        Message::WebBookmarkUrlChanged(v) => {
            app.web_bookmark_url_input = v;
            None
        }
        Message::WebBookmarkWidthChanged(v) => {
            app.web_bookmark_width_input = v;
            None
        }
        Message::WebBookmarkHeightChanged(v) => {
            app.web_bookmark_height_input = v;
            None
        }
        Message::WebBookmarkAddSave => {
            let title = app.web_bookmark_title_input.trim().to_string();
            let url = app.web_bookmark_url_input.trim().to_string();
            if !title.is_empty() && !url.is_empty() {
                let width = app.web_bookmark_width_input.trim().parse::<i32>().ok();
                let height = app.web_bookmark_height_input.trim().parse::<i32>().ok();
                let bm = WebBookmark {
                    title,
                    url: url.clone(),
                    width,
                    height,
                    cookie_configs: None,
                };
                app.web_bookmarks.push(bm);
                app.web_bookmark_title_input.clear();
                app.web_bookmark_url_input.clear();
                app.web_bookmark_width_input.clear();
                app.web_bookmark_height_input.clear();
                return save_bookmarks(app);
            }
            None
        }
        Message::WebBookmarkAddCancel => {
            app.show_web_links_menu = false;
            app.web_bookmark_title_input.clear();
            app.web_bookmark_url_input.clear();
            app.web_bookmark_width_input.clear();
            app.web_bookmark_height_input.clear();
            None
        }
        Message::WebBookmarkEditStart(idx) => {
            if let Some(bm) = app.web_bookmarks.get(idx).cloned() {
                app.editing_web_bookmark = Some(idx);
                app.edit_web_bookmark_title_input = bm.title;
                app.edit_web_bookmark_url_input = bm.url;
                app.edit_web_bookmark_width_input =
                    bm.width.map(|v| v.to_string()).unwrap_or_default();
                app.edit_web_bookmark_height_input =
                    bm.height.map(|v| v.to_string()).unwrap_or_default();
                app.edit_web_bookmark_cookie_configs_editor =
                    iced::widget::text_editor::Content::with_text(
                        &serde_json::to_string_pretty(&bm.cookie_configs.unwrap_or_default())
                            .unwrap_or_default(),
                    );
            }
            None
        }
        Message::WebBookmarkEditTitleChanged(v) => {
            app.edit_web_bookmark_title_input = v;
            None
        }
        Message::WebBookmarkEditUrlChanged(v) => {
            app.edit_web_bookmark_url_input = v;
            None
        }
        Message::WebBookmarkEditWidthChanged(v) => {
            app.edit_web_bookmark_width_input = v;
            None
        }
        Message::WebBookmarkEditHeightChanged(v) => {
            app.edit_web_bookmark_height_input = v;
            None
        }
        Message::WebBookmarkEditCookieConfigsChanged(action) => {
            app.edit_web_bookmark_cookie_configs_editor.perform(action);
            None
        }
        Message::WebBookmarkEditCookieConfigsInsertExample => {
            app.edit_web_bookmark_cookie_configs_editor =
                iced::widget::text_editor::Content::with_text(WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE);
            None
        }
        Message::WebBookmarkEditSave => {
            let mut save = None;
            if let Some(idx) = app.editing_web_bookmark {
                let title = app.edit_web_bookmark_title_input.trim().to_string();
                let url = app.edit_web_bookmark_url_input.trim().to_string();
                let cookie_configs_text = app.edit_web_bookmark_cookie_configs_editor.text();
                let cookie_configs: Option<Vec<CookieConfig>> =
                    serde_json::from_str(&cookie_configs_text).ok();

                if !url.is_empty() {
                    if let Some(bm) = app.web_bookmarks.get_mut(idx) {
                        bm.title = if title.is_empty() { url.clone() } else { title };
                        bm.url = url.clone();
                        bm.width = app.edit_web_bookmark_width_input.trim().parse::<i32>().ok();
                        bm.height = app.edit_web_bookmark_height_input.trim().parse::<i32>().ok();
                        bm.cookie_configs = cookie_configs;
                    }
                    save = save_bookmarks(app);
                }
            }
            app.editing_web_bookmark = None;
            app.edit_web_bookmark_title_input.clear();
            app.edit_web_bookmark_url_input.clear();
            app.edit_web_bookmark_width_input.clear();
            app.edit_web_bookmark_height_input.clear();
            app.edit_web_bookmark_cookie_configs_editor = iced::widget::text_editor::Content::new();
            save
        }
        Message::WebBookmarkEditCancel => {
            app.editing_web_bookmark = None;
            app.edit_web_bookmark_title_input.clear();
            app.edit_web_bookmark_url_input.clear();
            app.edit_web_bookmark_width_input.clear();
            app.edit_web_bookmark_height_input.clear();
            app.edit_web_bookmark_cookie_configs_editor = iced::widget::text_editor::Content::new();
            None
        }
        Message::WebBookmarkRemove(idx) => {
            let mut save = None;
            if idx < app.web_bookmarks.len() {
                app.web_bookmarks.remove(idx);
                save = save_bookmarks(app);
            }
            if app.editing_web_bookmark == Some(idx) {
                app.editing_web_bookmark = None;
                app.edit_web_bookmark_title_input.clear();
                app.edit_web_bookmark_url_input.clear();
                app.edit_web_bookmark_width_input.clear();
                app.edit_web_bookmark_height_input.clear();
                app.edit_web_bookmark_cookie_configs_editor =
                    iced::widget::text_editor::Content::new();
            }
            save
        }
    }
}


#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;
