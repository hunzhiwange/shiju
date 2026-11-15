//! 用量视图的数据整理层，将原始会话与模型用量转换为界面可直接消费的结构。

use session_ui_types as models;

/// UsageData 数据结构，承载当前模块对外传递的显式状态。
pub struct UsageData {
    /// total tokens 字段，保存渲染或状态更新所需的输入数据。
    pub total_tokens: i64,
    /// session 字段，保存渲染或状态更新所需的输入数据。
    pub session: Option<models::ChatSession>,
    /// session id 字段，保存渲染或状态更新所需的输入数据。
    #[allow(dead_code)]
    pub session_id: String,
    /// message count 字段，保存渲染或状态更新所需的输入数据。
    pub message_count: usize,
    /// call count 字段，保存渲染或状态更新所需的输入数据。
    pub call_count: usize,
    /// session title 字段，保存渲染或状态更新所需的输入数据。
    pub session_title: String,
    /// last step input tokens 字段，保存渲染或状态更新所需的输入数据。
    pub last_step_input_tokens: i64,
    /// last step output tokens 字段，保存渲染或状态更新所需的输入数据。
    pub last_step_output_tokens: i64,
    /// last step cached tokens 字段，保存渲染或状态更新所需的输入数据。
    pub last_step_cached_tokens: i64,
    /// last step reasoning tokens 字段，保存渲染或状态更新所需的输入数据。
    pub last_step_reasoning_tokens: i64,
    /// last step total tokens 字段，保存渲染或状态更新所需的输入数据。
    pub last_step_total_tokens: i64,
    /// user msgs 字段，保存渲染或状态更新所需的输入数据。
    pub user_msgs: usize,
    /// assistant msgs 字段，保存渲染或状态更新所需的输入数据。
    pub assistant_msgs: usize,
    /// system msgs 字段，保存渲染或状态更新所需的输入数据。
    pub system_msgs: usize,
    /// tool msgs 字段，保存渲染或状态更新所需的输入数据。
    pub tool_msgs: usize,
}

impl UsageData {
    /// 构建或更新 from app 相关行为。
    ///
    /// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
    pub fn from_parts(
        usage: &models::TokenUsage, active_session_id: Option<&String>,
        chat: &[models::ChatMessage], session_info: Option<&session_info::Info>,
        calls: &[serde_json::Value], steps: &[models::ChatSessionStep], updated_ms: u64,
        locale: config_ui::DesktopLocale,
    ) -> Self {
        let i18n = desktop_i18n::I18n::new(locale);
        let total_tokens = usage.input_tokens
            + usage.output_tokens
            + usage.cached_tokens
            + usage.reasoning_tokens;
        let session_id =
            active_session_id.cloned().unwrap_or_else(|| i18n.__("暂无").to_string());
        let message_count = chat.len();

        let session_title =
            session_info.map(|info| info.title.as_str()).unwrap_or(i18n.__("暂无")).to_string();
        let session = active_session_id.map(|id| models::ChatSession {
            id: id.clone(),
            title: session_title.clone(),
            messages: Vec::new(),
            message_ids: Vec::new(),
            calls: calls.to_vec(),
            steps: steps.to_vec(),
            created_ms: session_info.map(|info| info.time.created).unwrap_or_default(),
            updated_ms: session_info
                .map(|info| info.time.updated)
                .unwrap_or(updated_ms),
        });
        let call_count = session.as_ref().map(|session| session.calls.len()).unwrap_or_default();

        let last_step_usage = session
            .as_ref()
            .and_then(|session| {
                latest_reported_usage(&session.steps)
            })
            .cloned();
        let last_step_input_tokens = last_step_usage.as_ref().map(|u| u.input_tokens).unwrap_or(0);
        let last_step_output_tokens =
            last_step_usage.as_ref().map(|u| u.output_tokens).unwrap_or(0);
        let last_step_cached_tokens =
            last_step_usage.as_ref().map(|u| u.cached_tokens).unwrap_or(0);
        let last_step_reasoning_tokens =
            last_step_usage.as_ref().map(|u| u.reasoning_tokens).unwrap_or(0);
        let last_step_total_tokens = last_step_input_tokens
            + last_step_output_tokens
            + last_step_cached_tokens
            + last_step_reasoning_tokens;

        let (user_msgs, assistant_msgs, system_msgs, tool_msgs) = {
            let mut u = 0usize;
            let mut a = 0usize;
            let mut sys = 0usize;
            let mut tool = 0usize;
            for m in chat {
                match m.role {
                    models::ChatRole::User => u += 1,
                    models::ChatRole::Assistant => a += 1,
                    models::ChatRole::System | models::ChatRole::Developer => sys += 1,
                    models::ChatRole::Tool => tool += 1,
                }
            }
            (u, a, sys, tool)
        };

        Self {
            total_tokens,
            session,
            session_id,
            message_count,
            call_count,
            session_title,
            last_step_input_tokens,
            last_step_output_tokens,
            last_step_cached_tokens,
            last_step_reasoning_tokens,
            last_step_total_tokens,
            user_msgs,
            assistant_msgs,
            system_msgs,
            tool_msgs,
        }
    }
}

fn latest_reported_usage(steps: &[models::ChatSessionStep]) -> Option<&models::TokenUsage> {
    steps.iter().rev().find(|step| step.usage.input_tokens > 0).map(|step| &step.usage)
}
