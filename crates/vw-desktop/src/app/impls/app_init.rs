//! 应用程序初始化模块
//!
//! 本模块负责应用程序启动时的初始化工作，包括：
//! - 加载和解析各种配置文件
//! - 初始化应用程序状态
//! - 检测外部应用程序的可用性
//! - 设置 UI 组件的初始状态
//!
//! 该模块是应用程序启动流程的核心，所有状态初始化都在此完成。

use std::collections::HashMap;

pub(super) use super::components;
pub(super) use super::config;
pub(super) use super::models;
pub(super) use super::state;
pub(super) use super::state::{ConventionalCommitType, ExternalOpenApp};
pub(super) use super::{
    App, AppTab, CookieConfig, DiffTheme, FocusArea, Message, RecentProjectMeta, Screen,
    SettingsTab, Shell, TerminalState, TerminalTheme, load_recent_projects,
    load_recent_projects_meta,
};
#[cfg(not(target_arch = "wasm32"))]
pub(super) use crate::app::lsp::LspServiceManager;

mod external_apps;
mod new;
mod settings_builders;
mod startup;

pub(super) fn sort_acp_agents(
    acp_cfg: &HashMap<String, vw_config_types::config::AcpAgentConfig>,
) -> Vec<String> {
    let mut acp_agents = acp_cfg.keys().cloned().collect::<Vec<_>>();
    acp_agents.sort_by(|left, right| {
        let left_key = (left != "codex", left.as_str());
        let right_key = (right != "codex", right.as_str());
        left_key.cmp(&right_key)
    });
    acp_agents
}

pub(super) fn display_name_for_path(meta: &[RecentProjectMeta], path: &str) -> String {
    if let Some(item) = meta.iter().find(|item| item.path == path) {
        return item.name.clone();
    }

    std::path::Path::new(path)
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or(path)
        .to_string()
}

pub(super) fn load_web_bookmarks(cfg: &serde_json::Value) -> Vec<super::WebBookmark> {
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

                    Some(super::WebBookmark {
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
            vec![super::WebBookmark {
                title: "测试网站".to_string(),
                url: "https://example.com/".to_string(),
                width: None,
                height: None,
                cookie_configs: None,
            }]
        })
}

#[cfg(test)]
#[path = "app_init_tests.rs"]
mod app_init_tests;
