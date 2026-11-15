//! 验证队列面板的任务分组、状态展示、操作消息与空态渲染。
//! 队列 fixture 固定发送行为和历史策略，便于单独验证顺序与移动可用性。

use super::queue_panel::queue_panel;
use crate::app::QueueItem;
use crate::app::state::{AcpHistoryReplayMode, ChatSendBehavior};
use iced::Length;

// 固定历史回放与发送策略，使排序和移动测试只受查询及创建时间影响。
fn queue_item(query: &str, created_ms: u64) -> QueueItem {
    QueueItem {
        created_ms,
        query: query.to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }
}

#[test]
fn queue_panel_renders_empty_short_and_long_queues() {
    let empty = queue_panel(
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        Vec::new(),
        false,
    );
    assert_eq!(empty.as_widget().size().width, Length::Fill);

    let mut guided = queue_item("first line\nsecond line", 0);
    guided.send_behavior = ChatSendBehavior::Guide;
    let short = queue_panel(
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        vec![guided],
        true,
    );
    assert_eq!(short.as_widget().size().width, Length::Fill);

    let long_label = "这是一条很长很长很长很长很长很长很长很长很长的任务标题";
    let long = queue_panel(
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        vec![queue_item(long_label, 1_000), queue_item("waiting", 2_000)],
        false,
    );
    assert_eq!(long.as_widget().size().width, Length::Fill);
}
