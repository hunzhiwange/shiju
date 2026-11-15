//! Lark/飞书平台标识与配置模块
//!
//! 本模块提供 Lark（国际版）与 Feishu（飞书，中国版）的平台区分支持。
//! 两个平台虽然功能相似，但使用不同的 API 端点和区域设置。
//!
//! # 平台差异
//!
//! | 特性 | Lark | Feishu |
//! |------|------|--------|
//! | 服务区域 | 国际 | 中国大陆 |
//! | API 端点 | `larksuite.com` | `feishu.cn` |
//! | 语言设置 | 英语 (`en`) | 中文 (`zh`) |
//!
//! # 使用示例
//!
//! ```ignore
//! use channel_lark::platform::LarkPlatform;
//!
//! let platform = LarkPlatform::Lark;
//! let api_url = platform.api_base();  // 返回 Lark API 地址
//! let locale = platform.locale_header();  // 返回 "en"
//! ```

use super::constants::{FEISHU_BASE_URL, FEISHU_WS_BASE_URL, LARK_BASE_URL, LARK_WS_BASE_URL};

/// Lark/飞书平台枚举
///
/// 用于区分 Lark 国际版和飞书中国版，两个平台共享相同的功能接口，
/// 但使用不同的服务端点和区域配置。
///
/// # 变体
///
/// - `Lark` - Lark 国际版，服务区域为全球
/// - `Feishu` - 飞书中国版，服务区域为中国大陆
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LarkPlatform {
    /// Lark 国际版
    Lark,
    /// 飞书中国版
    Feishu,
}

impl LarkPlatform {
    /// 获取当前平台的 API 基础 URL
    ///
    /// # 返回值
    ///
    /// 返回对应平台的 HTTP API 端点地址：
    /// - `Lark` -> `https://open.larksuite.com/open-apis`
    /// - `Feishu` -> `https://open.feishu.cn/open-apis`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let platform = LarkPlatform::Feishu;
    /// assert!(platform.api_base().contains("feishu.cn"));
    /// ```
    pub(crate) fn api_base(self) -> &'static str {
        match self {
            Self::Lark => LARK_BASE_URL,
            Self::Feishu => FEISHU_BASE_URL,
        }
    }

    /// 获取当前平台的 WebSocket 基础 URL
    ///
    /// # 返回值
    ///
    /// 返回对应平台的 WebSocket 连接端点地址：
    /// - `Lark` -> `wss://open.larksuite.com/ws`
    /// - `Feishu` -> `wss://open.feishu.cn/ws`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let platform = LarkPlatform::Lark;
    /// assert!(platform.ws_base().contains("larksuite.com"));
    /// ```
    pub(crate) fn ws_base(self) -> &'static str {
        match self {
            Self::Lark => LARK_WS_BASE_URL,
            Self::Feishu => FEISHU_WS_BASE_URL,
        }
    }

    /// 获取当前平台的区域语言标识
    ///
    /// # 返回值
    ///
    /// 返回用于 HTTP 请求头的语言标识：
    /// - `Lark` -> `"en"`（英语）
    /// - `Feishu` -> `"zh"`（中文）
    ///
    /// # 用途
    ///
    /// 此值通常用于设置 API 请求的 `Accept-Language` 或
    /// 飞书特定的 `X-Lark-Locale` 请求头。
    pub(crate) fn locale_header(self) -> &'static str {
        match self {
            Self::Lark => "en",
            Self::Feishu => "zh",
        }
    }

    /// 获取当前平台的代理服务标识键
    ///
    /// # 返回值
    ///
    /// 返回用于代理配置或服务路由的标识键：
    /// - `Lark` -> `"channel.lark"`
    /// - `Feishu` -> `"channel.feishu"`
    ///
    /// # 用途
    ///
    /// 此键用于在多通道代理系统中标识和路由不同平台的请求。
    pub(crate) fn proxy_service_key(self) -> &'static str {
        match self {
            Self::Lark => "channel.lark",
            Self::Feishu => "channel.feishu",
        }
    }

    /// 获取当前平台的通道名称
    ///
    /// # 返回值
    ///
    /// 返回平台的简短标识名称：
    /// - `Lark` -> `"lark"`
    /// - `Feishu` -> `"feishu"`
    ///
    /// # 用途
    ///
    /// 用于日志记录、配置识别和用户界面显示。
    pub(crate) fn channel_name(self) -> &'static str {
        match self {
            Self::Lark => "lark",
            Self::Feishu => "feishu",
        }
    }
}

#[cfg(test)]
#[path = "platform_tests.rs"]
mod platform_tests;
