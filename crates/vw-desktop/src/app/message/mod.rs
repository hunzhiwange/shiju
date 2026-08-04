//! # 消息模块
//!
//! 本模块定义了应用程序中所有消息类型的集合。
//!
//! ## 模块概述
//!
//! 消息是 Iced 框架中应用状态更新的核心机制。本模块将消息按功能域进行组织，
//! 每个子模块对应一个特定的功能区域（如聊天、编辑器、Git 操作等）。
//!
//! ## 子模块
//!
//! - `chat`: 聊天功能相关消息
//! - `editor`: 编辑器操作消息
//! - `git`: Git 版本控制消息
//! - `notification`: 通知消息
//! - `preview`: 预览功能消息
//! - `project`: 项目管理消息
//! - `search`: 搜索功能消息
//! - `settings`: 设置相关消息
//! - `task_board`: 任务看板消息
//! - `terminal`: 终端操作消息
//! - `view`: 视图切换消息

#[cfg(not(target_arch = "wasm32"))]
pub mod account;
pub mod chat;
pub mod editor;
pub mod gateway_health;
pub mod git;
pub mod knowledge_tool;
pub mod notification;
pub mod preview;
pub mod project;
pub mod search;
pub mod settings;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod shiju_apps;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod skill_hub;
pub mod task_board;
pub mod terminal;
pub mod view;

/// 创建一个延迟发送消息的 Iced 任务。
///
/// 该函数用于在指定的延迟时间后发送一条消息。
/// 这在需要延迟执行某些操作（如动画延迟、定时更新等）时非常有用。
///
/// # 参数
///
/// * `duration` - 延迟的时间长度，使用 `std::time::Duration` 表示
/// * `message` - 延迟后要发送的消息
///
/// # 返回值
///
/// 返回一个 `iced::Task`，该任务会在延迟完成后发送指定的消息。
///
/// # 示例
///
/// ```rust,ignore
/// use std::time::Duration;
/// use crate::app::Message;
///
/// // 在 1 秒后发送一条消息
/// let task = after(Duration::from_secs(1), Message::SomeAction);
/// ```
pub fn after(
    duration: std::time::Duration,
    message: crate::app::Message,
) -> iced::Task<crate::app::Message> {
    iced::Task::perform(sleep(duration), move |_| message)
}

/// 在阻塞上下文中执行一个可能返回 `None` 的函数。
///
/// 该函数提供了跨平台的阻塞任务执行能力。在非 WASM 平台上，
/// 它使用 Tokio 的 `spawn_blocking` 在专用的阻塞线程池中执行任务；
/// 在 WASM 平台上，由于不支持多线程，直接同步执行。
///
/// # 类型参数
///
/// * `F` - 要执行的闭包类型，必须实现 `FnOnce() -> Option<T> + Send + 'static`
/// * `T` - 返回值的类型，必须实现 `Send + 'static`
///
/// # 参数
///
/// * `f` - 要执行的闭包函数
///
/// # 返回值
///
/// 返回 `Option<T>`：
/// - `Some(T)` - 当闭包成功执行并返回 `Some(T)` 时
/// - `None` - 当闭包返回 `None`，或在非 WASM 平台上任务被取消时
///
/// # 平台差异
///
/// - **非 WASM 平台**：使用 `tokio::task::spawn_blocking` 异步执行，
///   如果任务被取消或发生 panic，返回 `None`
/// - **WASM 平台**：直接同步执行闭包，因为 WASM 当前不支持真正的多线程
///
/// # 示例
///
/// ```rust,ignore
/// // 执行一个可能失败的阻塞计算
/// let result = spawn_blocking_opt(|| {
///     // 一些阻塞操作
///     std::fs::read_to_string("config.txt").ok()
/// }).await;
///
/// match result {
///     Some(content) => println!("文件内容: {}", content),
///     None => println!("读取失败或文件不存在"),
/// }
/// ```
pub async fn spawn_blocking_opt<F, T>(f: F) -> Option<T>
where
    F: FnOnce() -> Option<T> + Send + 'static,
    T: Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        f()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(f).await.unwrap_or(None)
    }
}

/// 异步睡眠函数，支持跨平台延迟执行。
///
/// 该函数提供了一个跨平台的异步睡眠实现。在非 WASM 平台上使用 Tokio 的睡眠功能，
/// 在 WASM 平台上使用浏览器的 `setTimeout` API 实现。
///
/// # 参数
///
/// * `duration` - 睡眠的持续时间
///
/// # 平台实现细节
///
/// ## WASM 平台
///
/// 在 WASM 环境中，由于标准库的异步运行时不可用，该函数使用以下策略：
///
/// 1. 获取浏览器窗口对象 (`web_sys::window`)
/// 2. 创建一个 JavaScript Promise，在其内部设置 `setTimeout` 定时器
/// 3. 使用 `Closure::once` 创建一个一次性回调，在定时器触发时 resolve Promise
/// 4. 使用 `wasm_bindgen_futures::JsFuture` 将 Promise 转换为 Rust 的 Future
///
/// ### 注意事项
///
/// - 使用 `Closure::once` 和 `forget()` 来避免生命周期问题，
///   回调在执行后会被 JavaScript 垃圾回收
/// - 如果无法获取窗口对象，函数会立即返回而不等待
/// - 延迟时间会被限制在 `i32::MAX` 毫秒内（约 24.8 天）
///
/// ## 非 WASM 平台
///
/// 直接使用 `tokio::time::sleep` 进行异步睡眠。
async fn sleep(duration: std::time::Duration) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue, closure::Closure};

        // 将持续时间转换为毫秒，并限制在 i32 范围内
        let millis = duration.as_millis().min(i32::MAX as u128) as i32;

        // 获取浏览器窗口对象，如果失败则直接返回
        let Some(window) = web_sys::window() else {
            return;
        };

        // 创建一个 Promise，在其内部设置 setTimeout
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            // 将 resolve 参数转换为 JavaScript 函数
            let Ok(resolve) = resolve.dyn_into::<js_sys::Function>() else {
                return;
            };

            // 创建一次性回调，在定时器触发时 resolve Promise
            let cb = Closure::once(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });

            // 设置定时器，使用回调作为定时器完成时的处理函数
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                millis,
            );

            // 使用 forget() 让回调在 JavaScript 侧保持存活直到被执行
            // 这是必要的，因为 Rust 无法知道 JavaScript 何时会调用这个回调
            cb.forget();
        });

        // 等待 Promise 完成（即定时器触发）
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(duration).await;
    }
}

// ============================================================================
// 公共类型重导出
// ============================================================================
//
// 以下是从各个子模块重导出的主要消息类型。
// 这些重导出允许外部代码直接从 `message` 模块导入消息类型，
// 而不需要知道具体的子模块路径。
//
// 例如：使用 `use crate::app::message::ChatMessage;`
// 而不是 `use crate::app::message::chat::ChatMessage;`

pub use crate::app::views::workflow_manager::WorkflowManagerMessage;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use account::AccountMessage;
pub use chat::ChatMessage;
pub use editor::EditorMessage;
pub use git::GitMessage;
pub use knowledge_tool::KnowledgeToolMessage;
pub use notification::NotificationMessage;
pub use preview::PreviewMessage;
pub use project::ProjectMessage;
pub use search::SearchMessage;
pub(crate) use settings::{GeneralSettingsMessage, SettingsMessage};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use shiju_apps::ShijuAppsMessage;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use skill_hub::SkillHubMessage;
pub use task_board::TaskBoardMessage;
pub use terminal::TerminalMessage;
pub use view::ViewMessage;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;

#[cfg(test)]
#[path = "gateway_health_tests.rs"]
mod gateway_health_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "skill_hub_tests.rs"]
mod skill_hub_tests;
