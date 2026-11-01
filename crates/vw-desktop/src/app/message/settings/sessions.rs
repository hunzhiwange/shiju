//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::{App, Message};
use iced::Task;
use vw_shared::message::types as message;
use vw_shared::session::info;

use super::messages::SettingsMessage;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::SessionDelete(id) => {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(runtime) = app.session_runtime_states.get(&id) {
                for panel in &runtime.mcp_app_panels {
                    panel.cancel();
                }
            }
            app.session_runtime_states.remove(&id);

            if let Some(pos) = app.sessions.iter().position(|s| s.id == id) {
                app.sessions.remove(pos);
            }

            let mut switch_task = Task::none();
            if app.active_session_id.as_ref() == Some(&id) {
                let next = app.sessions.first().map(|s| s.id.clone());
                if let Some(next_id) = next {
                    app.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    switch_task = Task::perform(
                        async move {
                            let client = crate::app::gateway_client().map_err(|e| e.to_string())?;
                            let msgs = client
                                .session_messages::<Vec<message::WithParts>>(&next_id, None)
                                .await
                                .map_err(|e| e.to_string())?;
                            let mut usage = crate::app::models::TokenUsage::default();
                            for m in &msgs {
                                if let message::Info::Assistant(a) = &m.info {
                                    usage.input_tokens += a.tokens.input;
                                    usage.output_tokens += a.tokens.output;
                                    usage.cached_tokens +=
                                        a.tokens.cache.read + a.tokens.cache.write;
                                    usage.reasoning_tokens += a.tokens.reasoning;
                                }
                            }
                            Ok((next_id, msgs, usage))
                        },
                        |res| {
                            Message::Project(
                                crate::app::message::ProjectMessage::SessionMessagesLoaded(res),
                            )
                        },
                    );
                } else {
                    app.active_session_id = None;
                    app.chat.clear();
                    app.usage = crate::app::models::TokenUsage::default();
                    app.invalidate_chat_ui_state();
                }
            }

            Task::batch(vec![
                Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|e| e.to_string())?;
                        client.session_delete(&id, None).await
                    },
                    |_| Message::None,
                ),
                switch_task,
            ])
        }
        SettingsMessage::SessionCopy(id) => Task::perform(
            async move {
                let client = crate::app::gateway_client().map_err(|e| e.to_string())?;
                client.session_fork::<info::Info>(&id, None, &None).await.map_err(|e| e.to_string())
            },
            |res| match res {
                Ok(info) => {
                    Message::Project(crate::app::message::ProjectMessage::SessionCreated(info))
                }
                Err(e) => {
                    eprintln!("Failed to fork session: {}", e);
                    Message::None
                }
            },
        ),
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "sessions_tests.rs"]
mod sessions_tests;
