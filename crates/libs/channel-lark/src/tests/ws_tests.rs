//! Lark WebSocket 通道的轻量回归测试。
//!
//! 这里验证通道名称和心跳看门狗的刷新规则，确保只有真正代表连接活跃的
//! WebSocket 帧会延长接收时间，避免普通文本或关闭帧掩盖断线状态。

use super::*;

#[test]
fn lark_channel_name() {
    let ch = make_channel();
    assert_eq!(ch.name(), "lark");
}

#[test]
fn lark_ws_activity_refreshes_heartbeat_watchdog() {
    // 二进制与 ping/pong 帧都说明底层连接仍有活动，心跳看门狗应据此续期。
    assert!(should_refresh_last_recv(&WsMsg::Binary(vec![1, 2, 3].into())));
    assert!(should_refresh_last_recv(&WsMsg::Ping(vec![9, 9].into())));
    assert!(should_refresh_last_recv(&WsMsg::Pong(vec![8, 8].into())));
}

#[test]
fn lark_ws_non_activity_frames_do_not_refresh_heartbeat_watchdog() {
    // 业务文本和关闭帧不能作为存活证据，否则会延迟断线恢复。
    assert!(!should_refresh_last_recv(&WsMsg::Text("hello".into())));
    assert!(!should_refresh_last_recv(&WsMsg::Close(None)));
}
