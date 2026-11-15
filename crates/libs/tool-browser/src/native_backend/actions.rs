//! 原生浏览器后端的基础 DOM 操作。
//!
//! 这里封装 fantoccini 的常用选择器查找、点击、输入和 hover 操作，并统一补充
//! 错误上下文，便于上层浏览器工具把 WebDriver 失败转换为可诊断的工具错误。

use anyhow::{Context, Result};
use fantoccini::elements::Element;
use fantoccini::{Client, Locator};
use serde_json;

/// 按 CSS 选择器查找元素。
///
/// # 错误
///
/// 找不到元素或 WebDriver 请求失败时返回错误。
pub async fn find_element(client: &Client, selector: &str) -> Result<Element> {
    client.find(Locator::Css(selector)).await.context("Failed to find element")
}

/// 等待 CSS 选择器对应元素出现。
///
/// # 错误
///
/// 等待超时或 WebDriver 请求失败时返回错误。
pub async fn wait_for_selector(client: &Client, selector: &str) -> Result<Element> {
    client.wait().for_element(Locator::Css(selector)).await.context("Failed to wait for selector")
}

/// 查找并点击元素。
///
/// # 错误
///
/// 查找失败或点击失败时返回错误。
pub async fn click_with_recovery(client: &Client, selector: &str) -> Result<()> {
    let el = find_element(client, selector).await?;
    el.click().await.context("Failed to click element")
}

/// 清空输入框后填入文本。
///
/// # 错误
///
/// 查找、清空或发送按键失败时返回错误。
pub async fn fill_with_recovery(client: &Client, selector: &str, text: &str) -> Result<()> {
    let el = find_element(client, selector).await?;
    el.clear().await?;
    el.send_keys(text).await.context("Failed to send keys")
}

/// 在元素上追加输入文本。
///
/// # 错误
///
/// 查找或发送按键失败时返回错误。
pub async fn type_with_recovery(client: &Client, selector: &str, text: &str) -> Result<()> {
    let el = find_element(client, selector).await?;
    el.send_keys(text).await.context("Failed to send keys")
}

/// 读取复选框或单选元素是否被选中。
pub async fn element_checked(element: &Element) -> Result<bool> {
    element.is_selected().await.context("Failed to check if element is selected")
}

/// 准备用于交互的元素。
///
/// 当前实现只查找元素，保留这个窄封装方便未来在同一边界内加入滚动或可见性
/// 校验。
pub async fn prepare_interactable_element(client: &Client, selector: &str) -> Result<Element> {
    let el = find_element(client, selector).await?;
    Ok(el)
}

/// 通过 JavaScript 派发 hover 事件。
///
/// # 错误
///
/// 元素序列化或脚本执行失败时返回错误。
pub async fn hover_element(client: &Client, element: &Element) -> Result<()> {
    let script = r#"
        var element = arguments[0];
        var mouseEvent = new MouseEvent('mouseover', {
            view: window,
            bubbles: true,
            cancelable: true
        });
        element.dispatchEvent(mouseEvent);
    "#;
    client
        .execute(script, vec![serde_json::to_value(element)?])
        .await
        .context("Failed to hover element via JS")?;
    Ok(())
}
#[cfg(test)]
#[path = "actions_tests.rs"]
mod actions_tests;
