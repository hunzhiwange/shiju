//! 知识库工作台的状态、消息与更新。
pub mod state;
pub mod message {
    pub mod knowledge_tool;
    pub mod skill_hub;
    pub mod connectors;
    pub mod knowledge;
    pub mod public;
    mod public_gateway;
    #[cfg(test)]
    mod connectors_tests;
    mod skill_boundary;
    pub use skill_boundary::*;
}
pub type GatewayFactory = fn() -> Result<gateway_http::GatewayTransport, String>;

/// 一次领域更新实际使用的输入及同步反馈；不包含桌面全局状态。
pub struct KnowledgeUpdate<'a> {
    pub knowledge: &'a mut state::KnowledgeUiState,
    pub gateway: GatewayFactory,
    pub dropped_paths: Vec<String>,
    pub feedback: Option<(bool, String, desktop_i18n::format::Message)>,
}
#[derive(Debug, Clone)]
pub enum Event { Knowledge(message::knowledge_tool::KnowledgeToolMessage) }
fn after(duration: std::time::Duration, message: Event) -> iced::Task<Event> {
    iced::Task::perform(sleep(duration), move |_| message)
}
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

pub mod skill_operations;

pub mod views;

#[cfg(test)]
mod desktop_split_13_tests;
