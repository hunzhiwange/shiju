use codex_app_server_protocol::GetWorkspaceMessagesResponse;
use codex_app_server_protocol::WorkspaceMessageType;
use std::time::Duration;

pub(crate) const WORKSPACE_HEADLINE_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug, PartialEq, Eq)]
/// 标题获取结果保留“功能关闭”这一独立状态，避免被误当成空消息。
pub(crate) enum WorkspaceHeadlineFetchResult {
    Available(Option<String>),
    FeatureDisabled,
}

pub(crate) fn workspace_headline_from_response(
    response: GetWorkspaceMessagesResponse,
) -> WorkspaceHeadlineFetchResult {
    // 服务端功能开关为最终依据，关闭时不继续解释消息集合。
    if !response.feature_enabled {
        return WorkspaceHeadlineFetchResult::FeatureDisabled;
    }

    // 只采用首条非空标题消息，并在比较后去除展示文本首尾空白。
    WorkspaceHeadlineFetchResult::Available(response.messages.into_iter().find_map(|message| {
        (message.message_type == WorkspaceMessageType::Headline)
            .then(|| message.message_body.trim().to_string())
            .filter(|headline| !headline.is_empty())
    }))
}

#[cfg(test)]
#[path = "workspace_messages_tests.rs"]
mod tests;
//! 从工作区消息响应中提取状态行标题，并区分功能关闭与无标题两种状态。
