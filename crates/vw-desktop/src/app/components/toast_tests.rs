//! 验证应用提示状态与缓存重试动作到共享控件的接线。

use super::toast::{cache_sync_retry_message, view};
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::SkillCacheSyncState;
use crate::app::state::{Toast, ToastKind};
use crate::app::{App, Message};
use iced::Element;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn cache_sync_pending_retry_exposes_retry_action_in_toast() {
    let mut app = test_app();
    app.expert_hub.cache_sync =
        SkillCacheSyncState::PendingRetry { skill_id: 7, refresh_detail: false };
    app.active_toast = Some(Toast {
        localized_message: None,
        id: 7,
        kind: ToastKind::Warning,
        message: "服务端已更新，本地缓存待同步".to_string(),
    });

    assert!(matches!(
        cache_sync_retry_message(&app),
        Some(Message::SkillHub(SkillHubMessage::RetrySkillCacheSync))
    ));
    keep_element(view(&app));
}
