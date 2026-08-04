use super::ViewMessage;
use crate::app::{App, Message, set_config_field};
use iced::Task;

const WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE: &str = r#"[
  {
    "name": "session_id",
    "domain": "example.com",
    "days": 365,
    "url_filter": "https://example.com/"
  }
]"#;

pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::OpenWebUrl(_url) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let candidates = crate::app::webview_runtime::webview_helper_candidates();

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
                    let _ = open::that(_url);
                }
            }
            Task::none()
        }
        ViewMessage::OpenWebUrlWithTitle(_url, _title) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let candidates = crate::app::webview_runtime::webview_helper_candidates();

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
                    let _ = open::that(_url);
                }
            }
            Task::none()
        }
        ViewMessage::OpenWebUrlWithTitleAndSize(_url, _title, _w, _h) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let url = _url.trim().trim_matches('`').trim().to_string();
                let candidates = crate::app::webview_runtime::webview_helper_candidates();

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
                    let _ = open::that(url);
                }
            }
            Task::none()
        }
        ViewMessage::OpenUrlExternal(_url) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = open::that(_url);
            }
            Task::none()
        }
        ViewMessage::ToggleWebLinksMenu => {
            app.show_web_links_menu = !app.show_web_links_menu;
            Task::none()
        }
        ViewMessage::WebBookmarkTitleChanged(v) => {
            app.web_bookmark_title_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkUrlChanged(v) => {
            app.web_bookmark_url_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkWidthChanged(v) => {
            app.web_bookmark_width_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkHeightChanged(v) => {
            app.web_bookmark_height_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkAddSave => {
            let title = app.web_bookmark_title_input.trim().to_string();
            let url = app.web_bookmark_url_input.trim().to_string();
            if !title.is_empty() && !url.is_empty() {
                let width = app.web_bookmark_width_input.trim().parse::<i32>().ok();
                let height = app.web_bookmark_height_input.trim().parse::<i32>().ok();
                let bm = crate::app::WebBookmark {
                    title,
                    url: url.clone(),
                    width,
                    height,
                    cookie_configs: None,
                };
                app.web_bookmarks.push(bm);
                let arr = serde_json::Value::Array(
                    app.web_bookmarks
                        .iter()
                        .map(|b| {
                            let mut obj = serde_json::Map::new();
                            obj.insert(
                                "title".to_string(),
                                serde_json::Value::String(b.title.clone()),
                            );
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
                set_config_field("web_bookmarks", arr);
                app.web_bookmark_title_input.clear();
                app.web_bookmark_url_input.clear();
                app.web_bookmark_width_input.clear();
                app.web_bookmark_height_input.clear();
            }
            Task::none()
        }
        ViewMessage::WebBookmarkAddCancel => {
            app.show_web_links_menu = false;
            app.web_bookmark_title_input.clear();
            app.web_bookmark_url_input.clear();
            app.web_bookmark_width_input.clear();
            app.web_bookmark_height_input.clear();
            Task::none()
        }
        ViewMessage::WebBookmarkEditStart(idx) => {
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
            Task::none()
        }
        ViewMessage::WebBookmarkEditTitleChanged(v) => {
            app.edit_web_bookmark_title_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkEditUrlChanged(v) => {
            app.edit_web_bookmark_url_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkEditWidthChanged(v) => {
            app.edit_web_bookmark_width_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkEditHeightChanged(v) => {
            app.edit_web_bookmark_height_input = v;
            Task::none()
        }
        ViewMessage::WebBookmarkEditCookieConfigsChanged(action) => {
            app.edit_web_bookmark_cookie_configs_editor.perform(action);
            Task::none()
        }
        ViewMessage::WebBookmarkEditCookieConfigsInsertExample => {
            app.edit_web_bookmark_cookie_configs_editor =
                iced::widget::text_editor::Content::with_text(WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE);
            Task::none()
        }
        ViewMessage::WebBookmarkEditSave => {
            if let Some(idx) = app.editing_web_bookmark {
                let title = app.edit_web_bookmark_title_input.trim().to_string();
                let url = app.edit_web_bookmark_url_input.trim().to_string();
                let cookie_configs_text = app.edit_web_bookmark_cookie_configs_editor.text();
                let cookie_configs: Option<Vec<crate::app::CookieConfig>> =
                    serde_json::from_str(&cookie_configs_text).ok();

                if !url.is_empty() {
                    if let Some(bm) = app.web_bookmarks.get_mut(idx) {
                        bm.title = if title.is_empty() { url.clone() } else { title };
                        bm.url = url.clone();
                        bm.width = app.edit_web_bookmark_width_input.trim().parse::<i32>().ok();
                        bm.height = app.edit_web_bookmark_height_input.trim().parse::<i32>().ok();
                        bm.cookie_configs = cookie_configs;
                    }
                    let arr = serde_json::Value::Array(
                        app.web_bookmarks
                            .iter()
                            .map(|b| {
                                let mut obj = serde_json::Map::new();
                                obj.insert(
                                    "title".to_string(),
                                    serde_json::Value::String(b.title.clone()),
                                );
                                obj.insert(
                                    "url".to_string(),
                                    serde_json::Value::String(b.url.clone()),
                                );
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
                    set_config_field("web_bookmarks", arr);
                }
            }
            app.editing_web_bookmark = None;
            app.edit_web_bookmark_title_input.clear();
            app.edit_web_bookmark_url_input.clear();
            app.edit_web_bookmark_width_input.clear();
            app.edit_web_bookmark_height_input.clear();
            app.edit_web_bookmark_cookie_configs_editor = iced::widget::text_editor::Content::new();
            Task::none()
        }
        ViewMessage::WebBookmarkEditCancel => {
            app.editing_web_bookmark = None;
            app.edit_web_bookmark_title_input.clear();
            app.edit_web_bookmark_url_input.clear();
            app.edit_web_bookmark_width_input.clear();
            app.edit_web_bookmark_height_input.clear();
            app.edit_web_bookmark_cookie_configs_editor = iced::widget::text_editor::Content::new();
            Task::none()
        }
        ViewMessage::WebBookmarkRemove(idx) => {
            if idx < app.web_bookmarks.len() {
                app.web_bookmarks.remove(idx);
                let arr = serde_json::Value::Array(
                    app.web_bookmarks
                        .iter()
                        .map(|b| {
                            let mut obj = serde_json::Map::new();
                            obj.insert(
                                "title".to_string(),
                                serde_json::Value::String(b.title.clone()),
                            );
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
                set_config_field("web_bookmarks", arr);
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
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;
