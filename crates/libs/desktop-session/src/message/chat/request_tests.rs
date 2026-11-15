use super::{AcpHistoryReplayMode, ChatSendBehavior};

#[test]
fn acp_history_replay_modes_expose_stable_keys() {
    let cases = [
        (AcpHistoryReplayMode::Discard, "discard"),
        (AcpHistoryReplayMode::Full, "full"),
        (AcpHistoryReplayMode::Summary, "summary"),
        (AcpHistoryReplayMode::Recent, "recent"),
    ];

    for (mode, key) in cases {
        assert_eq!(mode.as_str(), key);
    }
}

#[test]
fn acp_history_replay_mode_parsing_normalizes_input_and_rejects_unknown_values() {
    assert_eq!(AcpHistoryReplayMode::from_str(" DISCARD "), AcpHistoryReplayMode::Discard);
    assert_eq!(AcpHistoryReplayMode::from_str("Full"), AcpHistoryReplayMode::Full);
    assert_eq!(AcpHistoryReplayMode::from_str("summary"), AcpHistoryReplayMode::Summary);
    assert_eq!(AcpHistoryReplayMode::from_str("RECENT"), AcpHistoryReplayMode::Recent);
    assert_eq!(AcpHistoryReplayMode::from_str("unsupported"), AcpHistoryReplayMode::Discard);
    assert_eq!(AcpHistoryReplayMode::from_str("   "), AcpHistoryReplayMode::Discard);
}

#[test]
fn chat_send_behaviors_expose_stable_metadata() {
    let cases = [
        (ChatSendBehavior::Queue, "queue", "添加到队列", "发送到队列，保持当前排队逻辑。"),
        (
            ChatSendBehavior::StopAndSend,
            "stop_send",
            "停止并发送",
            "立刻停止当前请求，并开始发送。",
        ),
        (ChatSendBehavior::Guide, "guide", "通过消息引导", "等待当前一轮工具完成，下一轮请求前。"),
    ];

    for (behavior, key, label, description) in cases {
        assert_eq!(behavior.as_str(), key);
        assert_eq!(behavior.label(), label);
        assert_eq!(behavior.description(), description);
    }
}

#[test]
fn chat_send_behavior_parsing_accepts_stop_aliases_and_defaults_to_queue() {
    for value in ["stop_send", "stop-and-send", "stop_and_send", "stop", " STOP "] {
        assert_eq!(ChatSendBehavior::from_str(value), ChatSendBehavior::StopAndSend);
    }
    assert_eq!(ChatSendBehavior::from_str(" GUIDE "), ChatSendBehavior::Guide);
    assert_eq!(ChatSendBehavior::from_str("queue"), ChatSendBehavior::Queue);
    assert_eq!(ChatSendBehavior::from_str("unsupported"), ChatSendBehavior::Queue);
    assert_eq!(ChatSendBehavior::from_str(""), ChatSendBehavior::Queue);
}
