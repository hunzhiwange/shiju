use super::*;

#[test]
fn queued_input_remains_fifo() {
    let mut queue = InputQueueState::default();
    queue.push(UserMessage::new("first", Vec::new()));
    queue.push(UserMessage::new("second", Vec::new()));

    assert_eq!(queue.pop().map(|message| message.text), Some("first".into()));
    assert_eq!(queue.pop().map(|message| message.text), Some("second".into()));
}
