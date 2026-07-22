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

#[tokio::test(flavor = "multi_thread")]
async fn after_emits_the_original_message_after_sleeping() {
    let messages =
        collect_task_messages(super::after(Duration::ZERO, Message::CopyFeedbackExpired(42))).await;

    assert!(matches!(messages.as_slice(), [Message::CopyFeedbackExpired(42)]));
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_blocking_opt_returns_closure_value() {
    let result = super::spawn_blocking_opt(|| Some("completed")).await;

    assert_eq!(result, Some("completed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_blocking_opt_preserves_explicit_none() {
    let result = super::spawn_blocking_opt(|| Option::<usize>::None).await;

    assert_eq!(result, None);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn spawn_blocking_opt_maps_panicking_task_to_none() {
    let result = super::spawn_blocking_opt(|| -> Option<()> {
        panic!("intentional panic from blocking test task");
    })
    .await;

    assert_eq!(result, None);
}
