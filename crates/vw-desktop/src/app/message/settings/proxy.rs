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

use crate::app::{App, Message};
use iced::Task;

use super::messages::SettingsMessage;
use super::util::parse_comma_or_newline_list;

/// 将当前应用的代理设置持久化到全局配置中
///
/// 此函数从应用状态中提取代理设置数据，去除首尾空白字符，
/// 解析逗号或换行分隔的列表，然后更新全局代理配置。
///
/// # 参数
///
/// * `app` - 可变引用的应用实例，从中读取代理设置
///
/// # 处理逻辑
///
/// 1. 提取并清理各个代理地址字段（去除首尾空白）
/// 2. 解析 no_proxy 列表（逗号或换行分隔）
/// 3. 解析 services 列表（逗号或换行分隔）
/// 4. 调用全局配置更新函数，将设置写入持久化存储
fn persist_proxy_settings(app: &mut App) -> Task<Message> {
    let s = &app.proxy_settings;
    let enabled = s.enabled;
    let scope = s.scope;
    let http_proxy = s.http_proxy.trim().to_string();
    let https_proxy = s.https_proxy.trim().to_string();
    let all_proxy = s.all_proxy.trim().to_string();
    let no_proxy = parse_comma_or_newline_list(&s.no_proxy_input);
    let services = parse_comma_or_newline_list(&s.services_input);

    // 更新全局代理配置
    // 空字符串转换为 None，非空字符串包装为 Some
    crate::app::update_proxy_config_async(move |proxy| {
        proxy.enabled = enabled;
        proxy.scope = scope;
        proxy.http_proxy = if http_proxy.is_empty() { None } else { Some(http_proxy) };
        proxy.https_proxy = if https_proxy.is_empty() { None } else { Some(https_proxy) };
        proxy.all_proxy = if all_proxy.is_empty() { None } else { Some(all_proxy) };
        proxy.no_proxy = no_proxy;
        proxy.services = services;
    })
}

/// 处理代理设置相关的消息并更新应用状态
///
/// 此函数是代理设置消息的主分发器，根据不同的消息类型
/// 执行相应的状态更新和持久化操作。
///
/// # 参数
///
/// * `app` - 可变引用的应用实例，用于读取和更新状态
/// * `message` - 设置消息枚举，指定要执行的操作类型
///
/// # 返回值
///
/// 返回 `Task<Message>`，通常为 `Task::none()`，因为代理设置变更
/// 不需要触发额外的异步操作。
///
/// # 支持的消息类型
///
/// - `ProxyEnabledToggled` - 切换代理启用状态
/// - `ProxyScopeTextChanged` - 更改代理作用域（environment/services/vibewindow）
/// - `ProxyHttpChanged` - 更新 HTTP 代理地址
/// - `ProxyHttpsChanged` - 更新 HTTPS 代理地址
/// - `ProxyAllChanged` - 更新通用代理地址
/// - `ProxyNoProxyChanged` - 更新代理排除列表
/// - `ProxyServicesChanged` - 更新服务列表
/// - `ProxyHelpOpen` - 打开帮助模态框
/// - `ProxyHelpClose` - 关闭帮助模态框
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        // 处理代理启用状态切换
        SettingsMessage::ProxyEnabledToggled(v) => {
            app.proxy_settings.enabled = v;
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理代理作用域文本变更
        // 支持三种作用域：environment（环境变量）、services（指定服务）、vibewindow（内置）
        SettingsMessage::ProxyScopeTextChanged(v) => {
            // 将输入文本标准化为小写并去除空白，然后匹配对应的作用域枚举
            app.proxy_settings.scope = match v.trim().to_ascii_lowercase().as_str() {
                "environment" | "env" | "系统环境" => {
                    config_proxy::ProxyScope::Environment
                }
                "services" | "service" | "指定服务" => {
                    config_proxy::ProxyScope::Services
                }
                _ => config_proxy::ProxyScope::Vibewindow,
            };
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理 HTTP 代理地址变更
        SettingsMessage::ProxyHttpChanged(v) => {
            app.proxy_settings.http_proxy = v;
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理 HTTPS 代理地址变更
        SettingsMessage::ProxyHttpsChanged(v) => {
            app.proxy_settings.https_proxy = v;
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理通用代理地址变更
        SettingsMessage::ProxyAllChanged(v) => {
            app.proxy_settings.all_proxy = v;
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理代理排除列表变更
        // no_proxy 列表中的地址将绕过代理直接连接
        SettingsMessage::ProxyNoProxyChanged(v) => {
            app.proxy_settings.no_proxy_input = v;
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 处理服务列表变更
        // 当作用域为 Services 时，仅这些服务使用代理
        SettingsMessage::ProxyServicesChanged(v) => {
            app.proxy_settings.services_input = v;
            app.proxy_settings.save_error = None;
            persist_proxy_settings(app)
        }
        // 打开代理帮助模态框
        SettingsMessage::ProxyHelpOpen => {
            app.proxy_settings.show_help_modal = true;
            Task::none()
        }
        // 关闭代理帮助模态框
        SettingsMessage::ProxyHelpClose => {
            app.proxy_settings.show_help_modal = false;
            Task::none()
        }
        // 其他消息类型不在此模块处理，返回空任务
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "proxy_tests.rs"]
mod proxy_tests;
