use super::*;
use std::time::Duration;

#[tokio::test]
async fn closed_backend_channel_finishes_subscription() {
    let (sender, receiver) = mpsc::channel(1);
    drop(sender);
    let data = TerminalSubscriptionData {
        id: 1,
        event_receiver: Arc::new(Mutex::new(receiver)),
    };
    let mut stream = terminal_subscription_stream(&data);
    let event = tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("closed channel must not spin");
    assert!(event.is_none());
}

#[tokio::test]
async fn exit_is_forwarded_and_finishes_subscription_with_sender_alive() {
    let (sender, receiver) = mpsc::channel(1);
    sender.send(AlacrittyEvent::Exit).await.unwrap();
    let data = TerminalSubscriptionData {
        id: 7,
        event_receiver: Arc::new(Mutex::new(receiver)),
    };
    let mut stream = terminal_subscription_stream(&data);
    let event = tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Exit must be forwarded");
    assert!(matches!(
        event,
        Some(Event::BackendCall(
            7,
            backend::Command::ProcessAlacrittyEvent(AlacrittyEvent::Exit)
        ))
    ));
    assert!(tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("Exit must finish the stream")
        .is_none());
    drop(sender);
}
