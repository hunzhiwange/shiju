//! 原生浏览器会话管理模块
//!
//! 本模块提供基于 WebDriver 协议的原生浏览器会话管理功能，主要用于：
//! - 管理浏览器客户端连接的生命周期
//! - 配置和初始化浏览器会话（支持无头模式、自定义浏览器路径等）
//! - 提供统一的会话状态管理接口
//!
//! ## 架构说明
//!
//! 该模块是 `native_backend` 子系统的核心组件，通过 `fantoccini` crate 与 WebDriver
//! (如 chromedriver、geckodriver) 通信，实现对真实浏览器的控制。
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use crate::app::agent::tools::browser::native_backend::session::NativeBrowserState;
//!
//! let mut state = NativeBrowserState::default();
//!
//! // 初始化会话（连接到本地 WebDriver）
//! state.ensure_session(true, "http://localhost:4444", None).await?;
//!
//! // 获取活跃客户端执行操作
//! let client = state.active_client()?;
//!
//! // 重置会话
//! state.reset_session().await;
//! ```

use anyhow::{Context, Result};
use fantoccini::{Client, ClientBuilder};
use serde_json::{Map, Value};

const AUTOMATION_EVASION_ARG: &str = "--disable-blink-features=AutomationControlled";

/// 原生浏览器状态管理器
///
/// 负责管理与 WebDriver 客户端的连接状态，提供会话的创建、访问和重置功能。
/// 该结构体设计为可变状态容器，支持在多次操作间复用同一个浏览器会话。
///
/// # 线程安全
///
/// 该类型不是线程安全的。如需跨线程共享，请使用 `Arc<Mutex<NativeBrowserState>>`。
///
/// # 生命周期
///
/// - 创建时处于未初始化状态（`client` 为 `None`）
/// - 调用 `ensure_session` 后建立与 WebDriver 的连接
/// - 调用 `reset_session` 后关闭连接并重置为未初始化状态
#[derive(Default)]
pub struct NativeBrowserState {
    /// WebDriver 客户端实例
    ///
    /// - `Some(client)`: 表示存在活跃的浏览器会话
    /// - `None`: 表示会话未初始化或已被重置
    client: Option<Client>,
}

impl NativeBrowserState {
    /// 重置浏览器会话
    ///
    /// 关闭当前活跃的浏览器会话（如果存在），并将状态重置为未初始化。
    /// 该方法会忽略关闭过程中的任何错误，确保状态总能被重置。
    ///
    /// # 异步特性
    ///
    /// 该方法是异步的，因为关闭 WebDriver 连接需要进行网络 I/O。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut state = NativeBrowserState::default();
    /// state.ensure_session(true, "http://localhost:4444", None).await?;
    ///
    /// // 关闭并重置会话
    /// state.reset_session().await;
    /// assert!(state.active_client().is_err());
    /// ```
    pub async fn reset_session(&mut self) {
        // 取出客户端所有权，关闭连接并忽略任何错误
        // 使用 take() 确保即使 close() 失败，client 字段也会被置为 None
        if let Some(client) = self.client.take() {
            let _ = client.close().await;
        }
    }

    /// 获取当前活跃的 WebDriver 客户端引用
    ///
    /// 返回一个引用，调用者可以用于执行各种浏览器操作（导航、元素查找、截图等）。
    ///
    /// # 返回值
    ///
    /// - `Ok(&Client)`: 成功返回客户端引用
    /// - `Err`: 如果会话未初始化，返回错误提示
    ///
    /// # 错误
    ///
    /// 如果在调用 `ensure_session` 之前或 `reset_session` 之后调用此方法，
    /// 将返回包含友好错误消息的错误。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut state = NativeBrowserState::default();
    ///
    /// // 未初始化时调用会报错
    /// assert!(state.active_client().is_err());
    ///
    /// // 初始化后可正常获取
    /// state.ensure_session(true, "http://localhost:4444", None).await?;
    /// let client = state.active_client()?;
    /// ```
    pub fn active_client(&self) -> Result<&Client> {
        self.client.as_ref().ok_or_else(|| {
            anyhow::anyhow!("No active native browser session. Run browser action='open' first")
        })
    }

    /// 确保浏览器会话已初始化
    ///
    /// 如果会话尚未建立，则创建一个新的 WebDriver 连接。如果会话已存在，
    /// 则直接返回成功（幂等操作）。
    ///
    /// # 参数
    ///
    /// - `headless`: 是否以无头模式运行浏览器
    ///   - `true`: 浏览器将在后台运行，不显示任何窗口界面（适合 CI/CD 环境）
    ///   - `false`: 浏览器将以正常模式运行，显示窗口界面
    ///
    /// - `webdriver_url`: WebDriver 服务的 URL 地址
    ///   - 通常是 `http://localhost:4444`（chromedriver 默认端口）
    ///   - 或 `http://localhost:4444`（geckodriver 默认端口）
    ///
    /// - `chrome_path`: 可选的自定义 Chrome 可执行文件路径
    ///   - `Some(path)`: 使用指定路径的浏览器
    ///   - `None`: 使用系统默认的 Chrome 浏览器
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 会话初始化成功或已存在
    /// - `Err`: 连接 WebDriver 失败（服务未启动、URL 错误等）
    ///
    /// # 配置能力 (Capabilities)
    ///
    /// 该方法会自动配置以下 Chrome 选项：
    /// - 所有模式下添加 `--disable-blink-features=AutomationControlled` 参数
    /// - 无头模式下添加 `--headless=new` 和 `--disable-gpu` 参数
    /// - 支持自定义浏览器二进制文件路径
    ///
    /// # 错误处理
    ///
    /// 如果连接 WebDriver 失败，错误消息会提示用户先启动 chromedriver/geckodriver。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut state = NativeBrowserState::default();
    ///
    /// // 以无头模式连接本地 WebDriver
    /// state.ensure_session(true, "http://localhost:4444", None).await?;
    ///
    /// // 使用自定义 Chrome 路径
    /// state.ensure_session(
    ///     false,
    ///     "http://localhost:4444",
    ///     Some("/usr/bin/google-chrome")
    /// ).await?;
    /// ```
    pub async fn ensure_session(
        &mut self,
        headless: bool,
        webdriver_url: &str,
        chrome_path: Option<&str>,
    ) -> Result<()> {
        // 幂等性检查：如果客户端已存在，直接返回成功
        if self.client.is_some() {
            return Ok(());
        }

        // 构建 WebDriver 能力配置
        // capabilities 是符合 W3C WebDriver 标准的配置对象
        let mut capabilities: Map<String, Value> = Map::new();
        let mut chrome_options: Map<String, Value> = Map::new();
        let mut args = vec![Value::String(AUTOMATION_EVASION_ARG.to_string())];

        // 配置无头模式参数
        // --headless=new 使用 Chrome 新版无头模式（比旧版更稳定）
        // --disable-gpu 在无头环境下避免 GPU 相关问题
        if headless {
            args.push(Value::String("--headless=new".to_string()));
            args.push(Value::String("--disable-gpu".to_string()));
        }

        // 仅在有参数时才添加到 chrome_options
        if !args.is_empty() {
            chrome_options.insert("args".to_string(), Value::Array(args));
        }

        // 配置自定义浏览器路径（如果提供）
        // 允许使用非系统默认的 Chrome/Chromium 版本
        if let Some(path) = chrome_path {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                chrome_options.insert("binary".to_string(), Value::String(trimmed.to_string()));
            }
        }

        // 将 Chrome 选项合并到能力配置中
        // goog:chromeOptions 是 ChromeDriver 识别的专有能力键
        if !chrome_options.is_empty() {
            capabilities.insert("goog:chromeOptions".to_string(), Value::Object(chrome_options));
        }

        // 创建并配置 WebDriver 客户端构建器
        // browser-native 会同时带入 Ring 与 AWS-LC，需在 Fantoccini 创建 Rustls 配置前
        // 显式选择项目统一使用的 Ring provider。
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let mut builder =
            ClientBuilder::rustls().context("Failed to initialize WebDriver client builder")?;
        if !capabilities.is_empty() {
            builder.capabilities(capabilities);
        }

        // 连接到 WebDriver 服务
        // 这里的 connect 是异步操作，需要 WebDriver 服务已经启动并监听指定 URL
        let client = builder
            .connect(webdriver_url)
            .await
            .with_context(|| {
                format!(
                    "Failed to connect to WebDriver at {webdriver_url}. Start chromedriver/geckodriver first"
                )
            })?;

        // 保存客户端实例以供后续使用
        self.client = Some(client);
        Ok(())
    }
}
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
