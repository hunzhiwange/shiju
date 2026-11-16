//! 代理设置消息处理模块
//!
//! 本模块负责处理与代理配置相关的所有设置消息，包括：
//! - 代理开关切换
//! - 代理作用域配置
//! - HTTP/HTTPS/ALL 代理地址设置
//! - 代理排除列表（no_proxy）管理
//! - 服务列表配置
//!
//! 所有设置变更都会立即持久化到全局代理配置中。
use crate::Message as SettingsMessage;
use crate::app_init::state::ProxySettingsState;
use super::parse_comma_or_newline_list;






/// 从领域状态构造待持久化的代理字段。
fn persist_proxy_settings(app: &mut ProxySettingsState) -> Effect {
    let s = &app;
    let enabled = s.enabled;
    let scope = s.scope;
    let http_proxy = s.http_proxy.trim().to_string();
    let https_proxy = s.https_proxy.trim().to_string();
    let all_proxy = s.all_proxy.trim().to_string();
    let no_proxy = parse_comma_or_newline_list(&s.no_proxy_input);
    let services = parse_comma_or_newline_list(&s.services_input);

    // 更新全局代理配置
    // 空字符串转换为 None，非空字符串包装为 Some
    {
        let mut proxy = config_proxy::ProxyConfig::default();
        proxy.enabled = enabled;
        proxy.scope = scope;
        proxy.http_proxy = if http_proxy.is_empty() { None } else { Some(http_proxy) };
        proxy.https_proxy = if https_proxy.is_empty() { None } else { Some(https_proxy) };
        proxy.all_proxy = if all_proxy.is_empty() { None } else { Some(all_proxy) };
        proxy.no_proxy = no_proxy;
        proxy.services = services;

        Effect::Persist(proxy)
    }
}

/// 更新代理领域状态并返回必要的持久化效果。
pub fn update(app: &mut ProxySettingsState, message: SettingsMessage) -> Effect {
    match message {
        // 处理代理启用状态切换
        SettingsMessage::ProxyEnabledToggled(v) => {
            app.enabled = v;
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理代理作用域文本变更
        // 支持三种作用域：environment（环境变量）、services（指定服务）、vibewindow（内置）
        SettingsMessage::ProxyScopeTextChanged(v) => {
            // 将输入文本标准化为小写并去除空白，然后匹配对应的作用域枚举
            app.scope = match v.trim().to_ascii_lowercase().as_str() {
                "environment" | "env" | "系统环境" => {
                    config_proxy::ProxyScope::Environment
                }
                "services" | "service" | "指定服务" => {
                    config_proxy::ProxyScope::Services
                }
                _ => config_proxy::ProxyScope::Vibewindow,
            };
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理 HTTP 代理地址变更
        SettingsMessage::ProxyHttpChanged(v) => {
            app.http_proxy = v;
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理 HTTPS 代理地址变更
        SettingsMessage::ProxyHttpsChanged(v) => {
            app.https_proxy = v;
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理通用代理地址变更
        SettingsMessage::ProxyAllChanged(v) => {
            app.all_proxy = v;
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理代理排除列表变更
        // no_proxy 列表中的地址将绕过代理直接连接
        SettingsMessage::ProxyNoProxyChanged(v) => {
            app.no_proxy_input = v;
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理服务列表变更
        // 当作用域为 Services 时，仅这些服务使用代理
        SettingsMessage::ProxyServicesChanged(v) => {
            app.services_input = v;
            app.save_error = None;
            persist_proxy_settings(app)
        }
        // 打开代理帮助模态框
        SettingsMessage::ProxyHelpOpen => {
            app.show_help_modal = true;
            Effect::None
        }
        // 关闭代理帮助模态框
        SettingsMessage::ProxyHelpClose => {
            app.show_help_modal = false;
            Effect::None
        }
        // 其他消息类型不在此模块处理，返回空任务
        _ => Effect::None,
    }
}
#[cfg(test)]
#[path = "proxy_tests.rs"]
mod proxy_tests;

pub enum Effect { None, Persist(config_proxy::ProxyConfig) }

/// Apply only fields owned by this settings page.
pub fn apply_config(target: &mut config_proxy::ProxyConfig, value: config_proxy::ProxyConfig) {
        target.enabled = value.enabled;
        target.scope = value.scope;
        target.http_proxy = value.http_proxy;
        target.https_proxy = value.https_proxy;
        target.all_proxy = value.all_proxy;
        target.no_proxy = value.no_proxy;
        target.services = value.services;
}
