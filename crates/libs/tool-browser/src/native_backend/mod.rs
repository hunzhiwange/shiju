//! 原生浏览器后端模块
//!
//! 本模块提供基于 WebDriver 协议的浏览器自动化功能，使用 fantoccini 作为 WebDriver 客户端。
//! 支持多种浏览器操作，包括页面导航、元素交互、截图、等待等。
//!
//! # 主要功能
//!
//! - **会话管理**: 自动创建和管理浏览器会话
//! - **页面操作**: 打开 URL、获取标题/URL、截图
//! - **元素交互**: 点击、填充、输入、悬停、滚动
//! - **元素查询**: 获取文本、检查可见性、快照
//! - **等待机制**: 等待选择器、文本、超时
//! - **表单操作**: 复选框选中、键盘按键
//!
//! # 架构
//!
//! ```text
//! native_backend/
//! ├── mod.rs          (本文件) - 主要操作实现
//! ├── actions.rs      - 元素交互操作
//! ├── session.rs      - 浏览器会话管理
//! ├── snapshot.rs     - DOM 快照脚本
//! └── webdriver.rs    - WebDriver 工具函数
//! ```
//!
//! # 示例
//!
//! ```ignore
//! use crate::native_backend::NativeBrowserState;
//! use crate::BrowserAction;
//!
//! let mut state = NativeBrowserState::default();
//! let result = state.execute_action(
//!     BrowserAction::Open { url: "https://example.com".into() },
//!     true,
//!     "http://localhost:4444",
//!     None,
//! ).await?;
//! ```

use super::actions::BrowserAction;
use anyhow::{Context, Result};
use base64::Engine;
use fantoccini::Locator;
use serde_json::{Value, json};
use std::time::Duration;

mod actions;
mod session;
mod snapshot;
mod webdriver;

use actions::{
    click_with_recovery, element_checked, fill_with_recovery, find_element, hover_element,
    prepare_interactable_element, type_with_recovery, wait_for_selector,
};
use snapshot::snapshot_script;
use webdriver::{
    selector_for_find, webdriver_endpoint_reachable, webdriver_key, xpath_contains_text,
};

pub use session::NativeBrowserState;

impl NativeBrowserState {
    /// 检查浏览器后端是否可用
    ///
    /// 通过尝试连接 WebDriver 端点来判断浏览器服务是否已启动并可用。
    ///
    /// # 参数
    ///
    /// - `_headless`: 无头模式标志（当前未使用，预留用于未来扩展）
    /// - `webdriver_url`: WebDriver 服务地址，例如 `http://localhost:4444`
    /// - `_chrome_path`: Chrome 浏览器路径（当前未使用，预留用于自定义浏览器路径）
    ///
    /// # 返回值
    ///
    /// 如果 WebDriver 端点可达则返回 `true`，否则返回 `false`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let available = NativeBrowserState::is_available(true, "http://localhost:4444", None);
    /// if available {
    ///     println!("WebDriver 服务可用");
    /// }
    /// ```
    pub fn is_available(_headless: bool, webdriver_url: &str, _chrome_path: Option<&str>) -> bool {
        webdriver_endpoint_reachable(webdriver_url, Duration::from_millis(500))
    }

    /// 执行浏览器操作
    ///
    /// 根据传入的 `BrowserAction` 枚举执行相应的浏览器自动化操作。
    /// 这是原生浏览器后端的核心入口方法。
    ///
    /// # 参数
    ///
    /// - `action`: 要执行的浏览器操作，参见 [`BrowserAction`] 枚举
    /// - `headless`: 是否使用无头模式运行浏览器
    /// - `webdriver_url`: WebDriver 服务地址
    /// - `chrome_path`: 可选的 Chrome 浏览器可执行文件路径
    ///
    /// # 返回值
    ///
    /// 返回包含操作结果的 JSON 对象，通常包含：
    /// - `backend`: 后端类型标识（固定为 `"rust_native"`）
    /// - `action`: 操作类型名称
    /// - 其他操作特定的字段
    ///
    /// # 错误
    ///
    /// 当操作失败时返回错误，可能的原因包括：
    /// - WebDriver 连接失败
    /// - 元素未找到
    /// - 超时
    /// - 页面导航失败
    ///
    /// # 支持的操作
    ///
    /// | 操作 | 说明 |
    /// |------|------|
    /// | `Open` | 打开指定 URL |
    /// | `Snapshot` | 获取页面 DOM 快照 |
    /// | `Click` | 点击元素 |
    /// | `Fill` | 填充输入框（清空后输入） |
    /// | `Type` | 向元素输入文本（不清空） |
    /// | `GetText` | 获取元素文本内容 |
    /// | `GetTitle` | 获取页面标题 |
    /// | `GetUrl` | 获取当前页面 URL |
    /// | `Screenshot` | 截取页面截图 |
    /// | `Wait` | 等待条件满足 |
    /// | `Press` | 发送按键 |
    /// | `Hover` | 鼠标悬停 |
    /// | `Scroll` | 滚动页面 |
    /// | `IsVisible` | 检查元素可见性 |
    /// | `Close` | 关闭浏览器会话 |
    /// | `Find` | 查找并操作元素 |
    #[allow(clippy::too_many_lines)]
    pub async fn execute_action(
        &mut self,
        action: BrowserAction,
        headless: bool,
        webdriver_url: &str,
        chrome_path: Option<&str>,
    ) -> Result<Value> {
        match action {
            // 打开指定 URL 的页面
            // 先确保有活动的浏览器会话，然后导航到目标 URL
            BrowserAction::Open { url } => {
                self.ensure_session(headless, webdriver_url, chrome_path).await?;
                let client = self.active_client()?;
                client.goto(&url).await.with_context(|| format!("Failed to open URL: {url}"))?;

                // 获取导航后的当前 URL（可能被重定向）
                let current_url = client
                    .current_url()
                    .await
                    .context("Failed to read current URL after navigation")?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "open",
                    "url": current_url.as_str(),
                }))
            }

            // 获取页面 DOM 快照
            // 通过注入 JavaScript 脚本来提取页面结构信息
            BrowserAction::Snapshot { interactive_only, compact, depth } => {
                let client = self.active_client()?;
                let snapshot = client
                    .execute(
                        &snapshot_script(interactive_only, compact, depth.map(i64::from)),
                        vec![],
                    )
                    .await
                    .context("Failed to evaluate snapshot script")?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "snapshot",
                    "data": snapshot,
                }))
            }

            // 点击元素
            // 使用带恢复机制的点击，可处理元素被遮挡等情况
            BrowserAction::Click { selector } => {
                let client = self.active_client()?;
                click_with_recovery(client, &selector).await?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "click",
                    "selector": selector,
                }))
            }

            // 填充输入框
            // 先清空现有内容，再输入新值
            BrowserAction::Fill { selector, value } => {
                let client = self.active_client()?;
                fill_with_recovery(client, &selector, &value).await?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "fill",
                    "selector": selector,
                }))
            }

            // 向元素输入文本
            // 模拟逐字符输入，不清空现有内容
            BrowserAction::Type { selector, text } => {
                let client = self.active_client()?;
                type_with_recovery(client, &selector, &text).await?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "type",
                    "selector": selector,
                    "typed": text.len(),
                }))
            }

            // 获取元素的文本内容
            BrowserAction::GetText { selector } => {
                let client = self.active_client()?;
                let text: String = find_element(client, &selector).await?.text().await?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "get_text",
                    "selector": selector,
                    "text": text,
                }))
            }

            // 获取页面标题
            BrowserAction::GetTitle => {
                let client = self.active_client()?;
                let title = client.title().await.context("Failed to read page title")?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "get_title",
                    "title": title,
                }))
            }

            // 获取当前页面 URL
            BrowserAction::GetUrl => {
                let client = self.active_client()?;
                let url = client.current_url().await.context("Failed to read current URL")?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "get_url",
                    "url": url.as_str(),
                }))
            }

            // 截取页面截图
            // 可以保存到文件或返回 base64 编码
            BrowserAction::Screenshot { path, full_page } => {
                let client = self.active_client()?;
                let png = client.screenshot().await.context("Failed to capture screenshot")?;

                // 构建基础响应载荷
                let mut payload = json!({
                    "backend": "rust_native",
                    "action": "screenshot",
                    "full_page": full_page,
                    "bytes": png.len(),
                });

                if let Some(path_str) = path {
                    // 如果指定了路径，将截图保存到文件
                    tokio::fs::write(&path_str, &png)
                        .await
                        .with_context(|| format!("Failed to write screenshot to {path_str}"))?;
                    payload["path"] = Value::String(path_str);
                } else {
                    // 否则返回 base64 编码的图片数据
                    payload["png_base64"] =
                        Value::String(base64::engine::general_purpose::STANDARD.encode(&png));
                }

                Ok(payload)
            }

            // 等待条件满足
            // 支持三种等待方式：选择器出现、固定时间、文本出现
            BrowserAction::Wait { selector, ms, text } => {
                let client = self.active_client()?;

                if let Some(sel) = selector.as_ref() {
                    // 方式1: 等待指定选择器的元素出现
                    wait_for_selector(client, sel).await?;
                    Ok(json!({
                        "backend": "rust_native",
                        "action": "wait",
                        "selector": sel,
                    }))
                } else if let Some(duration_ms) = ms {
                    // 方式2: 等待指定的毫秒数
                    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
                    Ok(json!({
                        "backend": "rust_native",
                        "action": "wait",
                        "ms": duration_ms,
                    }))
                } else if let Some(needle) = text.as_ref() {
                    // 方式3: 等待包含指定文本的元素出现
                    let xpath = xpath_contains_text(needle);
                    client.wait().for_element(Locator::XPath(&xpath)).await.with_context(|| {
                        format!("Timed out waiting for text to appear: {needle}")
                    })?;
                    Ok(json!({
                        "backend": "rust_native",
                        "action": "wait",
                        "text": needle,
                    }))
                } else {
                    // 默认: 等待 250 毫秒
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    Ok(json!({
                        "backend": "rust_native",
                        "action": "wait",
                        "ms": 250,
                    }))
                }
            }

            // 发送键盘按键
            // 将按键名称转换为 WebDriver 键码并发送到当前焦点元素
            BrowserAction::Press { key } => {
                let client = self.active_client()?;
                let key_input = webdriver_key(&key);

                // 尝试获取当前活动元素（焦点元素）
                match client.active_element().await {
                    Ok(element) => {
                        element.send_keys(&key_input).await?;
                    }
                    Err(_) => {
                        // 如果无法获取活动元素，则发送到 body 元素
                        find_element(client, "body").await?.send_keys(&key_input).await?;
                    }
                }

                Ok(json!({
                    "backend": "rust_native",
                    "action": "press",
                    "key": key,
                }))
            }

            // 鼠标悬停到元素上
            BrowserAction::Hover { selector } => {
                let client = self.active_client()?;
                let element = find_element(client, &selector).await?;
                hover_element(client, &element).await?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "hover",
                    "selector": selector,
                }))
            }

            // 滚动页面
            // 支持四个方向的滚动
            BrowserAction::Scroll { direction, pixels } => {
                let client = self.active_client()?;
                let amount = i64::from(pixels.unwrap_or(600));

                // 根据方向计算滚动偏移量
                let (dx, dy) = match direction.as_str() {
                    "up" => (0, -amount),
                    "down" => (0, amount),
                    "left" => (-amount, 0),
                    "right" => (amount, 0),
                    _ => anyhow::bail!(
                        "Unsupported scroll direction '{direction}'. Use up/down/left/right"
                    ),
                };

                // 执行滚动并获取滚动后的位置
                let position = client
                    .execute(
                        "window.scrollBy(arguments[0], arguments[1]); return { x: window.scrollX, y: window.scrollY };",
                        vec![json!(dx), json!(dy)],
                    )
                    .await
                    .context("Failed to execute scroll script")?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "scroll",
                    "position": position,
                }))
            }

            // 检查元素是否可见
            BrowserAction::IsVisible { selector } => {
                let client = self.active_client()?;
                let visible: bool = find_element(client, &selector).await?.is_displayed().await?;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "is_visible",
                    "selector": selector,
                    "visible": visible,
                }))
            }

            // 关闭浏览器会话
            // 重置会话状态，释放资源
            BrowserAction::Close => {
                self.reset_session().await;

                Ok(json!({
                    "backend": "rust_native",
                    "action": "close",
                    "closed": true,
                }))
            }

            // 查找元素并执行操作
            // 支持多种查找方式和后续操作
            BrowserAction::Find { by, value, action, fill_value } => {
                let client = self.active_client()?;
                let selector = selector_for_find(&by, &value);

                let payload = match action.as_str() {
                    // 点击找到的元素
                    "click" => {
                        click_with_recovery(client, &selector).await?;
                        json!({"result": "clicked"})
                    }
                    // 填充输入框
                    "fill" => {
                        let fill = fill_value.ok_or_else(|| {
                            anyhow::anyhow!("find_action='fill' requires fill_value")
                        })?;
                        fill_with_recovery(client, &selector, &fill).await?;
                        json!({"result": "filled", "typed": fill.len()})
                    }
                    // 获取元素文本
                    "text" => {
                        let element: fantoccini::elements::Element =
                            find_element(client, &selector).await?;
                        let text = element.text().await?;
                        json!({"result": "text", "text": text})
                    }
                    // 鼠标悬停
                    "hover" => {
                        let element = prepare_interactable_element(client, &selector).await?;
                        hover_element(client, &element).await?;
                        json!({"result": "hovered"})
                    }
                    // 选中复选框/单选框
                    "check" => {
                        let element = prepare_interactable_element(client, &selector).await?;
                        let checked_before = element_checked(&element).await?;

                        // 只有在未选中时才点击
                        if !checked_before {
                            click_with_recovery(client, &selector).await?;
                        }

                        // 重新获取元素并检查状态（DOM 可能已更新）
                        let refreshed = find_element(client, &selector).await?;
                        let checked_after = element_checked(&refreshed).await?;

                        json!({
                            "result": "checked",
                            "checked_before": checked_before,
                            "checked_after": checked_after,
                        })
                    }
                    _ => anyhow::bail!(
                        "Unsupported find_action '{action}'. Use click/fill/text/hover/check"
                    ),
                };

                Ok(json!({
                    "backend": "rust_native",
                    "action": "find",
                    "by": by,
                    "value": value,
                    "selector": selector,
                    "data": payload,
                }))
            }
        }
    }
}
#[cfg(test)]
mod tests;
