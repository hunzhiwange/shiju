//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{App, Arc, ChatMessage, ChatSession, ChatSessionMeta, TokenUsage};

/// 为 SharedChatMessages 领域数据提供更清晰的类型名称。
pub type SharedChatMessages = Arc<[ChatMessage]>;

/// 执行 shared_chat_messages 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn shared_chat_messages(chat: Vec<ChatMessage>) -> SharedChatMessages {
    Arc::from(chat)
}

/// 执行 build_session_previews 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn build_session_previews(
    previews: Vec<ChatSessionMeta>,
) -> std::collections::HashMap<String, String> {
    previews
        .into_iter()
        .filter_map(|meta| {
            let content = meta.last_content?;
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((
                    meta.id,
                    crate::app::views::project::utils::truncate_display_width(
                        &trimmed.replace('\n', " "),
                        120,
                    ),
                ))
            }
        })
        .collect()
}

/// 执行 session_usage 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn session_usage(session: &ChatSession) -> TokenUsage {
    let mut usage = TokenUsage::default();
    for step in &session.steps {
        usage.input_tokens += step.usage.input_tokens;
        usage.output_tokens += step.usage.output_tokens;
        usage.cached_tokens += step.usage.cached_tokens;
        usage.reasoning_tokens += step.usage.reasoning_tokens;
    }
    usage
}

impl App {
    /// 执行 reload_sessions_for_project 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn reload_sessions_for_project(
        &mut self,
        path: Option<String>,
    ) -> iced::Task<crate::app::Message> {
        self.session_previews.clear();
        self.archived_session_ids.clear();
        self.sessions.clear();
        self.active_session_id = None;
        self.chat.clear();
        self.usage = TokenUsage::default();
        self.invalidate_chat_ui_state();

        if let Some(project_path) = path.as_ref() {
            self.project_sessions_loading.insert(project_path.clone());
        }
        let path_clone = path.clone();
        let project_id = self.project_id.clone();

        iced::Task::perform(
            async move {
                use crate::app::views::project::utils::truncate_display_width;

                let client = crate::app::gateway_client().map_err(anyhow::Error::msg)?;
                let scope_id = crate::app::session_gateway::gateway_resolve_session_scope_id_async(
                    path_clone.as_deref(),
                    project_id.as_deref(),
                )
                .await
                .map_err(anyhow::Error::msg)?;
                crate::app::session_gateway::gateway_set_session_scope_async(scope_id.as_deref())
                    .await
                    .map_err(anyhow::Error::msg)?;

                let previews = crate::app::session_gateway::gateway_load_sessions_scoped_async(
                    scope_id.as_deref(),
                )
                .await
                .map_err(anyhow::Error::msg)?;
                let archived_session_ids =
                    crate::app::session_gateway::gateway_load_archived_session_ids_async(
                        scope_id.as_deref(),
                    )
                    .await
                    .map_err(anyhow::Error::msg)?;

                let preview_map = previews
                    .into_iter()
                    .filter_map(|meta| {
                        let content = meta.last_content?;
                        let trimmed = content.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some((
                                meta.id,
                                truncate_display_width(&trimmed.replace('\n', " "), 120),
                            ))
                        }
                    })
                    .collect::<std::collections::HashMap<_, _>>();

                let sessions = if let Some(p) = path_clone {
                    client
                        .session_list::<Vec<vw_shared::session::info::Info>>(Some(&p))
                        .await
                        .map_err(anyhow::Error::msg)?
                } else {
                    client
                        .session_list::<Vec<vw_shared::session::info::Info>>(None)
                        .await
                        .map_err(anyhow::Error::msg)?
                };

                Ok((sessions, preview_map, archived_session_ids))
            },
            move |res: Result<
                (
                    Vec<vw_shared::session::info::Info>,
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

    /// 执行 rebuild_session_previews 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
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
