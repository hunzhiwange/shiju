//! 验证应用初始化入口及各初始化子模块的连线。
//!
//! 初始化夹具组合配置、账户、项目和服务状态，检查各子流程汇入同一应用实例。

use super::*;
use serde_json::json;
use vw_config_types::config::AcpAgentConfig;

fn recent_project_meta(path: &str, name: &str) -> RecentProjectMeta {
    RecentProjectMeta {
        path: path.to_string(),
        name: name.to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }
}

#[test]
fn sort_acp_agents_puts_codex_first_then_sorts_remaining_names() {
    let acp_cfg = HashMap::from([
        ("zeta".to_string(), AcpAgentConfig::default()),
        ("codex".to_string(), AcpAgentConfig::default()),
        ("alpha".to_string(), AcpAgentConfig::default()),
    ]);

    assert_eq!(sort_acp_agents(&acp_cfg), ["codex", "alpha", "zeta"]);
}

#[test]
fn sort_acp_agents_sorts_without_codex() {
    let acp_cfg = HashMap::from([
        ("shell".to_string(), AcpAgentConfig::default()),
        ("browser".to_string(), AcpAgentConfig::default()),
    ]);

    assert_eq!(sort_acp_agents(&acp_cfg), ["browser", "shell"]);
}

#[test]
fn sort_acp_agents_returns_empty_list_for_empty_config() {
    let acp_cfg = HashMap::new();

    assert!(sort_acp_agents(&acp_cfg).is_empty());
}

#[test]
fn display_name_for_path_uses_recent_project_meta_name() {
    let meta = vec![recent_project_meta("/workspace/app", "App Workspace")];

    assert_eq!(display_name_for_path(&meta, "/workspace/app"), "App Workspace");
}

#[test]
fn display_name_for_path_uses_first_matching_meta_name() {
    let meta = vec![
        recent_project_meta("/workspace/app", "First Name"),
        recent_project_meta("/workspace/app", "Second Name"),
    ];

    assert_eq!(display_name_for_path(&meta, "/workspace/app"), "First Name");
}

#[test]
fn display_name_for_path_falls_back_to_last_path_segment() {
    let meta = vec![recent_project_meta("/workspace/other", "Other")];

    assert_eq!(display_name_for_path(&meta, "/workspace/app"), "app");
}

#[test]
fn display_name_for_path_returns_original_path_when_segment_is_unavailable() {
    let meta = Vec::new();

    assert_eq!(display_name_for_path(&meta, "/"), "/");
}

#[test]
fn display_name_for_path_handles_relative_and_non_utf8_like_fallbacks() {
    let meta = Vec::new();

    assert_eq!(display_name_for_path(&meta, "workspace/app"), "app");
    assert_eq!(display_name_for_path(&meta, ""), "");
}

#[test]
fn load_web_bookmarks_returns_default_when_config_key_is_missing() {
    let bookmarks = load_web_bookmarks(&json!({}));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "测试网站");
    assert_eq!(bookmarks[0].url, "https://example.com/");
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    assert!(bookmarks[0].cookie_configs.is_none());
}

#[test]
fn load_web_bookmarks_keeps_explicit_empty_array_empty() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": []
    }));

    assert!(bookmarks.is_empty());
}

#[test]
fn load_web_bookmarks_parses_valid_bookmarks_and_cookie_configs() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Console",
                "url": "https://console.example.com/",
                "width": 1280,
                "height": 720,
                "cookie_configs": [
                    {
                        "name": "session",
                        "domain": ".example.com",
                        "days": 7,
                        "url_filter": "console"
                    }
                ]
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Console");
    assert_eq!(bookmarks[0].url, "https://console.example.com/");
    assert_eq!(bookmarks[0].width, Some(1280));
    assert_eq!(bookmarks[0].height, Some(720));

    let cookie_configs =
        bookmarks[0].cookie_configs.as_ref().expect("valid bookmark should keep cookie configs");
    assert_eq!(cookie_configs.len(), 1);
    assert_eq!(cookie_configs[0].name, "session");
    assert_eq!(cookie_configs[0].domain.as_deref(), Some(".example.com"));
    assert_eq!(cookie_configs[0].days, Some(7));
    assert_eq!(cookie_configs[0].url_filter.as_deref(), Some("console"));
}

#[test]
fn load_web_bookmarks_ignores_non_numeric_window_size_and_non_array_cookies() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Tool",
                "url": "https://tool.example.com/",
                "width": "wide",
                "height": false,
                "cookie_configs": "not an array"
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Tool");
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    assert!(bookmarks[0].cookie_configs.is_none());
}

#[test]
fn load_web_bookmarks_filters_invalid_bookmarks_and_cookie_configs() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Missing URL"
            },
            {
                "url": "https://missing-title.example.com/"
            },
            {
                "title": "Valid",
                "url": "https://valid.example.com/",
                "cookie_configs": [
                    {
                        "domain": ".example.com"
                    },
                    {
                        "name": "auth"
                    }
                ]
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Valid");

    let cookie_configs = bookmarks[0]
        .cookie_configs
        .as_ref()
        .expect("valid bookmark should keep filtered cookie configs");
    assert_eq!(cookie_configs.len(), 1);
    assert_eq!(cookie_configs[0].name, "auth");
    assert_eq!(cookie_configs[0].domain, None);
    assert_eq!(cookie_configs[0].days, None);
    assert_eq!(cookie_configs[0].url_filter, None);
}

#[test]
fn load_web_bookmarks_keeps_valid_bookmark_without_cookie_configs() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Docs",
                "url": "https://docs.example.com/"
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Docs");
    assert_eq!(bookmarks[0].url, "https://docs.example.com/");
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    assert!(bookmarks[0].cookie_configs.is_none());
}

#[test]
fn load_web_bookmarks_keeps_empty_cookie_array_and_ignores_invalid_optional_fields() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": [
            {
                "title": "Dashboard",
                "url": "https://dashboard.example.com/",
                "width": null,
                "height": "tall",
                "cookie_configs": []
            }
        ]
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].width, None);
    assert_eq!(bookmarks[0].height, None);
    let cookie_configs = bookmarks[0]
        .cookie_configs
        .as_ref()
        .expect("an explicit cookie array should remain present");
    assert!(cookie_configs.is_empty());
}

#[test]
fn load_web_bookmarks_returns_default_when_config_key_is_not_an_array() {
    let bookmarks = load_web_bookmarks(&json!({
        "web_bookmarks": "not an array"
    }));

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "测试网站");
}
