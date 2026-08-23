// 验证消息延时任务与阻塞任务包装器的成功和失败语义。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use std::time::Duration;

use iced_runtime::Action;
use iced_runtime::futures::futures::StreamExt;

use crate::app::Message;

async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };

    while let Some(action) = stream.next().await {
        if let Action::Output(messages) = action {
            return messages;
        }
    }

    Vec::new()
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[tokio::test(flavor = "multi_thread")]
async fn after_emits_the_original_message_after_sleeping() {
    let messages =
        collect_task_messages(super::after(Duration::ZERO, Message::CopyFeedbackExpired(42))).await;

    assert!(matches!(messages.as_slice(), [Message::CopyFeedbackExpired(42)]));
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[tokio::test(flavor = "multi_thread")]
async fn spawn_blocking_opt_returns_closure_value() {
    let result = super::spawn_blocking_opt(|| Some("completed")).await;

    assert_eq!(result, Some("completed"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test(flavor = "multi_thread")]
async fn spawn_blocking_opt_preserves_explicit_none() {
    let result = super::spawn_blocking_opt(|| Option::<usize>::None).await;

    assert_eq!(result, None);
}

#[cfg(not(target_arch = "wasm32"))]
// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test(flavor = "multi_thread")]
async fn spawn_blocking_opt_maps_panicking_task_to_none() {
    let result = super::spawn_blocking_opt(|| -> Option<()> {
        panic!("intentional panic from blocking test task");
    })
    .await;

    assert_eq!(result, None);
}
