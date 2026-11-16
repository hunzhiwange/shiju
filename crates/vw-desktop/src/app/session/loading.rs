//! 会话加载的桌面状态重置与消息映射。
use super::{App, ChatMessage, ChatSession, ChatSessionStep, PreparedChatUiChunk, TokenUsage};
pub use desktop_session::session::loading::{
    SharedChatMessages, build_session_previews, prepare_timeline_page, session_content_signature,
    session_usage, shared_chat_messages,
};
pub type PreparedSessionOpen =
    desktop_session::session::loading::PreparedSessionOpen<PreparedChatUiChunk>;
fn prepare_base(messages: &[ChatMessage], show_reasoning_summary: bool) -> PreparedChatUiChunk {
    #[cfg(test)]
    desktop_chat_view::performance::record_initial_load(
        messages.len(),
        messages.iter().map(|m| m.content.len()).sum(),
    );
    desktop_chat_view::chat_ui::prepare_initial_session_chunk(messages, show_reasoning_summary)
}
pub fn prepare_session_open(
    session_id: String,
    messages: Vec<ChatMessage>,
    message_ids: Vec<Option<String>>,
    usage: TokenUsage,
    steps: Vec<ChatSessionStep>,
    calls: Vec<serde_json::Value>,
    updated_ms: u64,
    session_model: Option<String>,
    show_reasoning_summary: bool,
) -> PreparedSessionOpen {
    desktop_session::session::loading::prepare_session_open(
        session_id,
        messages,
        message_ids,
        usage,
        steps,
        calls,
        updated_ms,
        session_model,
        |messages| prepare_base(messages, show_reasoning_summary),
    )
}
pub fn prepare_bounded_session_open(
    summary: vw_api_session::GatewaySessionUiSummary,
    page: vw_api_session::GatewaySessionMessagePage<session_ui_types::SessionMessageRecord>,
    show_reasoning_summary: bool,
) -> PreparedSessionOpen {
    desktop_session::session::loading::prepare_bounded_session_open(summary, page, |messages| {
        prepare_base(messages, show_reasoning_summary)
    })
}
pub fn prepare_session_open_from_ui_store(
    session: ChatSession,
    show_reasoning_summary: bool,
) -> PreparedSessionOpen {
    desktop_session::session::loading::prepare_session_open_from_ui_store(session, |messages| {
        prepare_base(messages, show_reasoning_summary)
    })
}
impl App {
    pub fn reload_sessions_for_project(
        &mut self,
        path: Option<String>,
    ) -> iced::Task<crate::app::Message> {
        self.session_previews.clear();
        self.archived_session_ids.clear();
        self.sessions.clear();
        self.input_popovers.recommended_prompt = false;
        self.cache_active_session_chat();
        self.active_session_id = None;
        self.current_session_runtime_mut().draft = None;
        self.chat.clear();
        self.chat_message_ids.clear();
        self.usage = TokenUsage::default();
        self.invalidate_chat_ui_state();

        if let Some(project_path) = path.as_ref() {
            self.project_sessions_loading.insert(project_path.clone());
        }
        let path_clone = path.clone();
        let project_id = self.project_id.clone();
        let endpoint = crate::app::config::gateway_client_endpoint();

        iced::Task::perform(
            async move {
                desktop_session::session::loading::load_project_sessions(
                    endpoint, path_clone, project_id,
                )
                .await
            },
            move |res: Result<
                (
                    Vec<session_info::Info>,
                    std::collections::HashMap<String, String>,
                    std::collections::HashSet<String>,
                ),
                anyhow::Error,
            >| match res {
                Ok((sessions, previews, archived_session_ids)) => crate::app::Message::Project(
                    crate::app::message::project::ProjectMessage::SessionBootstrapLoaded {
                        project_path: path.clone(),
                        result: Ok(sessions),
                        previews,
                        archived_session_ids,
                    },
                ),
                Err(err) => crate::app::Message::Project(
                    crate::app::message::project::ProjectMessage::SessionBootstrapLoaded {
                        project_path: path,
                        result: Err(err.to_string()),
                        previews: std::collections::HashMap::new(),
                        archived_session_ids: std::collections::HashSet::new(),
                    },
                ),
            },
        )
    }

    pub fn rebuild_session_previews(&mut self) -> iced::Task<crate::app::Message> {
        #[cfg(target_arch = "wasm32")]
        {
            return iced::Task::perform(
                async move {
                    let scope =
                        crate::app::session_gateway::gateway_current_session_scope_async().await?;
                    crate::app::session_gateway::gateway_load_sessions_scoped_async(
                        scope.as_deref(),
                    )
                    .await
                },
                crate::app::Message::SessionPreviewsLoaded,
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let scope = crate::app::session_gateway::gateway_current_session_scope();
            self.session_previews = build_session_previews(
                crate::app::session_gateway::gateway_load_sessions_scoped(scope.as_deref()),
            );
            iced::Task::none()
        }
    }
}

#[cfg(test)]
#[path = "loading_tests.rs"]
mod loading_tests;
