//! 验证工作区消息响应提取状态标题时对空值和功能开关的处理。
//! 该解析只消费已提供数据，不触发网络请求或修改工作区状态。

use super::*;
use codex_app_server_protocol::WorkspaceMessage;
use pretty_assertions::assert_eq;

#[test]
// 多条消息中应选择首个非空标题，并忽略仅含空白的候选项。
fn workspace_headline_from_response_uses_first_non_empty_headline() {
    let response = GetWorkspaceMessagesResponse {
        feature_enabled: true,
        messages: vec![
            WorkspaceMessage {
                message_id: "announcement-id".to_string(),
                message_type: WorkspaceMessageType::Announcement,
                message_body: "Announcement body".to_string(),
                created_at: None,
                archived_at: None,
            },
            WorkspaceMessage {
                message_id: "empty-headline-id".to_string(),
                message_type: WorkspaceMessageType::Headline,
                message_body: "   ".to_string(),
                created_at: None,
                archived_at: None,
            },
            WorkspaceMessage {
                message_id: "headline-id".to_string(),
                message_type: WorkspaceMessageType::Headline,
                message_body: " Workspace headline ".to_string(),
                created_at: None,
                archived_at: None,
            },
        ],
    };

    assert_eq!(
        workspace_headline_from_response(response),
        WorkspaceHeadlineFetchResult::Available(Some("Workspace headline".to_string()))
    );
}

#[test]
// 服务端明确关闭功能时应返回专用状态，而不是误报“没有标题”。
fn workspace_headline_from_response_reports_feature_disabled() {
    let response = GetWorkspaceMessagesResponse {
        feature_enabled: false,
        messages: Vec::new(),
    };

    assert_eq!(
        workspace_headline_from_response(response),
        WorkspaceHeadlineFetchResult::FeatureDisabled
    );
}
